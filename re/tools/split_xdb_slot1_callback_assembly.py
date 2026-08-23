#!/usr/bin/env python3
"""Regenerate the missing CROOLIS/SCRUT slot-1 callback assembly dumps."""

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
    "croolis": (
        (0x0B78, 0x0C24, "slot1_wave_update", "state callback selected by method slot 1"),
        (0x0C24, 0x0C3E, "slot1_finish_update", "callback published by slot-1 wave update"),
        (0x0C3E, 0x0CB5, "slot1_state_update", "slot-1 bounds and selection state callback"),
        (0x0CB5, 0x0CD9, "slot1_camera_update", "shared camera-to-motion transition"),
        (0x0CD9, 0x0CF9, "slot1_motion_update", "callback published by slot-1 camera update"),
        (0x0CF9, 0x0D04, "slot1_return_update", "callback published by slot-1 motion update"),
        (0x0D04, 0x0DB3, "slot1_motion_continuation", "out-of-bounds slot-1 motion continuation"),
    ),
    "scrut": (
        (0x0B78, 0x0C18, "slot1_wave_update", "state callback selected by method slot 1"),
        (0x0C18, 0x0C32, "slot1_finish_update", "callback published by slot-1 wave update"),
        (0x0C32, 0x0CA3, "slot1_state_update", "slot-1 bounds and selection state callback"),
        (0x0CA3, 0x0CC7, "slot1_camera_update", "shared camera-to-motion transition"),
        (0x0CC7, 0x0CE7, "slot1_motion_update", "callback published by slot-1 camera update"),
        (0x0CE7, 0x0CF2, "slot1_return_update", "callback published by slot-1 motion update"),
        (0x0CF2, 0x0DA1, "slot1_motion_continuation", "out-of-bounds slot-1 motion continuation"),
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
        "; direct_callees: none\n"
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

    stale: list[Path] = []
    for module, specs in SPECS.items():
        output_dir = ASSEMBLY_ROOT / module / "callback_state_machine"
        output_dir.mkdir(parents=True, exist_ok=True)
        for start, stop, stem, provenance in specs:
            output = output_dir / f"func_{start:06x}_{stem}.asm"
            expected = render(objdump, module, start, stop, provenance)
            if args.check:
                if not output.is_file() or output.read_text(encoding="ascii") != expected:
                    stale.append(output.relative_to(ROOT))
            else:
                output.write_text(expected, encoding="ascii")
                print(f"wrote {output.relative_to(ROOT)}")

    if stale:
        for path in stale:
            print(f"ERROR: stale assembly dump: {path}")
        return 1
    if args.check:
        print("OK: 14 CROOLIS/SCRUT slot-1 callback dumps match extracted overlays")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
