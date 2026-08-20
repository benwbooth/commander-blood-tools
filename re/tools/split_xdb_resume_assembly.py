#!/usr/bin/env python3
"""Regenerate the one-routine assembly dumps for alien resume state machines."""

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
        (0x1C03, 0x1C34, "func_001c03_resume_apply_object_delta.asm", "near helper called by resume pair and timeout stages"),
        (0x1C34, 0x1C7D, "func_001c34_resume_1c34.asm", "resume callback published by method-table slot 13"),
        (0x1C7D, 0x1CBF, "func_001c7d_resume_stage_pair.asm", "continuation stored at context +0x36 by 0x1C34"),
        (0x1CBF, 0x1CCF, "func_001cbf_resume_stage_timeout.asm", "continuation stored at context +0x36 by 0x1C7D"),
        (0x1CCF, 0x1CFA, "func_001ccf_resume_stage_final.asm", "continuation stored at context +0x36 by 0x1CBF"),
        (0x1CFA, 0x1D79, "func_001cfa_resume_pair_outside.asm", "near helper called by resume pair and final stages"),
    ),
    "croolis": (
        (0x1B5F, 0x1B85, "func_001b5f_resume_apply_object_delta.asm", "near helper called by resume pair and timeout stages"),
        (0x1B85, 0x1BC9, "func_001b85_resume_1b85.asm", "resume callback published by method-table slot 13"),
        (0x1BC9, 0x1C0B, "func_001bc9_resume_stage_pair.asm", "continuation stored at context +0x36 by 0x1B85"),
        (0x1C0B, 0x1C1B, "func_001c0b_resume_stage_timeout.asm", "continuation stored at context +0x36 by 0x1BC9"),
        (0x1C1B, 0x1C46, "func_001c1b_resume_stage_final.asm", "continuation stored at context +0x36 by 0x1C0B"),
        (0x1C46, 0x1CC7, "func_001c46_resume_pair_outside.asm", "near helper called by resume pair and final stages"),
    ),
    "scrut": (
        (0x1C14, 0x1C45, "func_001c14_resume_apply_object_delta.asm", "near helper called by resume pair and timeout stages"),
        (0x1C45, 0x1C89, "func_001c45_resume_1c45.asm", "resume callback published by method-table slot 13"),
        (0x1C89, 0x1CCB, "func_001c89_resume_stage_pair.asm", "continuation stored at context +0x36 by 0x1C45"),
        (0x1CCB, 0x1CDB, "func_001ccb_resume_stage_timeout.asm", "continuation stored at context +0x36 by 0x1C89"),
        (0x1CDB, 0x1D06, "func_001cdb_resume_stage_final.asm", "continuation stored at context +0x36 by 0x1CCB"),
        (0x1D06, 0x1D87, "func_001d06_resume_pair_outside.asm", "near helper called by resume pair and final stages"),
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
        print("OK: 18 resume assembly dumps match the extracted overlays")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
