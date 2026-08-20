#!/usr/bin/env python3
"""Regenerate one-routine assembly dumps for alien slot-2 callbacks."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

ROOT = Path(__file__).resolve().parents[2]
ARTIFACT_ROOT = ROOT / "output" / "_tmp_dat"
ASSEMBLY_ROOT = ROOT / "re" / "assembly" / "xdb"

SPECS = {
    "amer": (
        (0x1688, 0x1692, "func_001688_slot2_restart.asm",
         "internal transition reached by callback 0x1948", (0x1692,)),
        (0x1692, 0x171D, "func_001692_slot2_update.asm",
         "callback published by method-table slot 2", (0x171D, 0x193E, 0x1A2B)),
        (0x171D, 0x18D3, "func_00171d_slot2_common_update.asm",
         "shared update tail reached by callbacks 0x1692, 0x1948, and 0x19CB", ()),
        (0x193E, 0x1948, "func_00193e_slot2_selection_wait.asm",
         "callback published by finish callback 0x1AA0", (0x1948,)),
        (0x1948, 0x19CB, "func_001948_slot2_selection_update.asm",
         "callback published by selection wait 0x193E", (0x1688, 0x171D, 0x1A2B)),
        (0x19CB, 0x1A2B, "func_0019cb_slot2_selection_late_update.asm",
         "callback published by selection callback 0x1948", (0x171D, 0x193E, 0x1A2B)),
        (0x1A2B, 0x1A5C, "func_001a2b_slot2_reset.asm",
         "shared reset tail reached by four AMER slot-2 callbacks", ()),
    ),
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def render(
    objdump: str,
    module: str,
    start: int,
    stop: int,
    provenance: str,
    direct_callees: tuple[int, ...],
) -> str:
    artifact = ARTIFACT_ROOT / f"{module}.xdb"
    relative_artifact = artifact.relative_to(ROOT)
    blob = artifact.read_bytes()
    process = subprocess.run(
        (
            objdump,
            "-D",
            "-b",
            "binary",
            "-m",
            "i386",
            "-M",
            "addr16,data16",
            f"--start-address={start}",
            f"--stop-address={stop}",
            str(relative_artifact),
        ),
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    callee_text = ", ".join(f"0x{entry:06X}" for entry in direct_callees)
    header = (
        "; Commander Blood recovered routine assembly\n"
        f"; module: xdb_{module}\n"
        f"; artifact: {relative_artifact}\n"
        f"; artifact_sha256: {sha256(blob)}\n"
        f"; overlay_offset: 0x{start:06X}\n"
        f"; byte_count: {stop - start}\n"
        f"; routine_bytes_sha256: {sha256(blob[start:stop])}\n"
        f"; routine_entry: 0x{start:06X}\n"
        "; group: callback_state_machine\n"
        f"; provenance: {provenance}\n"
        f"; direct_callees: {callee_text or 'none'}\n"
        f"; raw stop: 0x{stop:06X}\n\n"
    )
    listing = "\n".join(line.rstrip() for line in process.stdout.splitlines()) + "\n"
    return header + listing


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--objdump", default="objdump", help="GNU objdump path")
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail unless committed dumps match regenerated output",
    )
    args = parser.parse_args()
    objdump = shutil.which(args.objdump)
    if objdump is None:
        raise SystemExit(f"objdump not found: {args.objdump}")

    stale = []
    for module, specs in SPECS.items():
        output_dir = ASSEMBLY_ROOT / module / "callback_state_machine"
        output_dir.mkdir(parents=True, exist_ok=True)
        for start, stop, filename, provenance, direct_callees in specs:
            output = output_dir / filename
            expected = render(
                objdump, module, start, stop, provenance, direct_callees
            )
            if args.check:
                if not output.is_file() or output.read_text(encoding="ascii") != expected:
                    stale.append(output.relative_to(ROOT))
                continue
            output.write_text(expected, encoding="ascii")
            print(f"wrote {output.relative_to(ROOT)}")

    if stale:
        for path in stale:
            print(f"ERROR: stale assembly dump: {path}")
        return 1
    if args.check:
        dump_count = sum(len(specs) for specs in SPECS.values())
        print(
            f"OK: {dump_count} slot-2 assembly dumps match "
            "the extracted overlays"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
