#!/usr/bin/env python3
"""Regenerate one-routine assembly dumps for alien slot-3 callbacks."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
import sys

_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import argparse
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
ARTIFACT_ROOT = ROOT / "output" / "_tmp_dat"
ASSEMBLY_ROOT = ROOT / "re" / "assembly" / "xdb"

SPECS = {
    "amer": (
        (0x1414, 0x1558, "func_001414_slot3_update.asm",
         "generic callback published by slot-3 initializer"),
        (0x1558, 0x158A, "func_001558_slot3_restart_initial_update.asm",
         "generic slot-3 fallthrough and callback installed by final resume stage"),
        (0x158A, 0x15DB, "func_00158a_slot3_resume_callback.asm",
         "callback installed by resume pair stage"),
        (0x15DB, 0x1614, "func_0015db_slot3_capture_resume_state.asm",
         "tail target of slot-3 update when ring flag bit 1 is set"),
        (0x1614, 0x1648, "func_001614_slot3_ring_zero_callback.asm",
         "callback installed by slot-3 resume callback"),
    ),
    "croolis": (
        (0x146C, 0x15B0, "func_00146c_slot3_update.asm",
         "generic callback published by slot-3 initializer"),
        (0x15B0, 0x15E2, "func_0015b0_slot3_restart_initial_update.asm",
         "generic slot-3 fallthrough and callback installed by final resume stage"),
        (0x15E2, 0x1633, "func_0015e2_slot3_resume_callback.asm",
         "callback installed by resume pair stage"),
        (0x1633, 0x166C, "func_001633_slot3_capture_resume_state.asm",
         "tail target of slot-3 update when ring flag bit 1 is set"),
        (0x166C, 0x16A0, "func_00166c_slot3_ring_zero_callback.asm",
         "callback installed by slot-3 resume callback"),
    ),
    "scrut": (
        (0x145A, 0x159E, "func_00145a_slot3_update.asm",
         "generic callback published by slot-3 initializer"),
        (0x159E, 0x15D0, "func_00159e_slot3_restart_initial_update.asm",
         "generic slot-3 fallthrough and callback installed by final resume stage"),
        (0x15D0, 0x1621, "func_0015d0_slot3_resume_callback.asm",
         "callback installed by resume pair stage"),
        (0x1621, 0x165A, "func_001621_slot3_capture_resume_state.asm",
         "tail target of slot-3 update when ring flag bit 1 is set"),
        (0x165A, 0x168E, "func_00165a_slot3_ring_zero_callback.asm",
         "callback installed by slot-3 resume callback"),
    ),
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def render(objdump: str, module: str, start: int, stop: int, provenance: str) -> str:
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
        for start, stop, filename, provenance in specs:
            output = output_dir / filename
            expected = render(objdump, module, start, stop, provenance)
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
        print("OK: 15 slot-3 assembly dumps match the extracted overlays")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
