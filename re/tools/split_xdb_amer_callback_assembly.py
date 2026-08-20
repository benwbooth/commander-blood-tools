#!/usr/bin/env python3
"""Regenerate one-routine assembly dumps for bounded AMER callbacks."""

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
ARTIFACT = ROOT / "output" / "_tmp_dat" / "amer.xdb"
OUTPUT_DIR = ROOT / "re" / "assembly" / "xdb" / "amer" / "callback_state_machine"

SPECS = (
    (
        0x0B37,
        0x0BD0,
        "func_000b37_slot1_wave_update.asm",
        "internal callback selected by the AMER slot-1 wave state; alternate path tail-jumps to callback 0x0C5D",
    ),
    (
        0x0BD0,
        0x0BEA,
        "func_000bd0_slot1_finish_update.asm",
        "callback installed by the AMER slot-1 wave update at 0x0B70",
    ),
    (
        0x0BEA,
        0x0C5D,
        "func_000bea_slot1_state_update.asm",
        "slot-1 state callback head; tail-transfers to callbacks 0x0C5D or continuation 0x0CAC",
    ),
    (
        0x0C5D,
        0x0C81,
        "func_000c5d_slot1_camera_update.asm",
        "callback reached from the slot-1 wave path at 0x0BCD and movement path at 0x0C24",
    ),
    (
        0x0C81,
        0x0CA1,
        "func_000c81_slot1_motion_update.asm",
        "callback published by the AMER slot-1 camera callback at 0x0C7B",
    ),
    (
        0x0CA1,
        0x0CAC,
        "func_000ca1_slot1_return_update.asm",
        "callback published by the AMER slot-1 motion callback at 0x0C96",
    ),
    (
        0x0CAC,
        0x0D5B,
        "func_000cac_slot1_motion_continuation.asm",
        "motion continuation reached by the AMER slot-1 state callback at 0x0BEA",
    ),
    (
        0x1AA0,
        0x1B1A,
        "func_001aa0_slot2_finish_update.asm",
        "callback installed by the AMER slot-2 steering callback at 0x1A95",
    ),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def render(objdump: str, start: int, stop: int, provenance: str) -> str:
    relative_artifact = ARTIFACT.relative_to(ROOT)
    blob = ARTIFACT.read_bytes()
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
        "; module: xdb_amer\n"
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
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for start, stop, filename, provenance in SPECS:
        output = OUTPUT_DIR / filename
        expected = render(objdump, start, stop, provenance)
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
        print("OK: 8 bounded AMER callback dumps match the extracted overlay")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
