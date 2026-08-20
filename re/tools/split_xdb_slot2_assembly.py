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
        (0x1B1A, 0x1B5F, "func_001b1a_unreferenced_steering_update.asm",
         "compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference", ()),
    ),
    "croolis": (
        (0x171D, 0x1727, "func_00171d_slot2_restart.asm",
         "internal restart tail reached by fade callback 0x17F2", (0x1727,)),
        (0x1727, 0x178E, "func_001727_slot2_update.asm",
         "callback published by method-table slots 2 and 4", (0x178E, 0x1815, 0x1960)),
        (0x178E, 0x1794, "func_00178e_slot2_common_dispatch.asm",
         "shared control-latch dispatch reached by slot-2 callbacks", (0x1794, 0x17E4)),
        (0x1794, 0x17E4, "func_001794_slot2_motion_update.asm",
         "shared motion tail reached by callbacks 0x1727, 0x17F2, and 0x1960", ()),
        (0x17E4, 0x17F2, "func_0017e4_slot2_begin_fade.asm",
         "internal control-latch transition reached by 0x178E", (0x17F2,)),
        (0x17F2, 0x1815, "func_0017f2_slot2_fade_update.asm",
         "callback published by internal transition 0x17E4", (0x171D, 0x1794)),
        (0x1815, 0x1828, "func_001815_slot2_selection_init.asm",
         "internal selection transition reached by callback 0x1727", (0x1828,)),
        (0x1828, 0x1960, "func_001828_slot2_selection_update.asm",
         "callback published by internal transition 0x1815", (0x1727, 0x1960)),
        (0x1960, 0x1A86, "func_001960_slot2_reset_or_camera.asm",
         "shared reset and camera tail reached by callbacks 0x1727 and 0x1828", (0x178E,)),
        (0x1A86, 0x1ACB, "func_001a86_unreferenced_steering_update.asm",
         "compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference", ()),
    ),
    "scrut": (
        (0x1711, 0x171B, "func_001711_slot2_restart.asm",
         "shared restart tail reached by callbacks 0x17E6 and 0x181B", (0x171B,)),
        (0x171B, 0x1781, "func_00171b_slot2_update.asm",
         "callback published by method-table slots 2 and 4", (0x1781, 0x1802, 0x1A11)),
        (0x1781, 0x1787, "func_001781_slot2_common_dispatch.asm",
         "shared control-latch dispatch reached by slot-2 callbacks", (0x1787,)),
        (0x1787, 0x17E1, "func_001787_slot2_motion_update.asm",
         "shared motion tail reached by callbacks 0x171B, 0x17E6, and 0x1A11", ()),
        (0x17E1, 0x17E6, "func_0017e1_slot2_begin_fade.asm",
         "compiled callback setup with no table or in-overlay pointer reference", (0x17E6,)),
        (0x17E6, 0x1802, "func_0017e6_slot2_fade_update.asm",
         "callback published by unreferenced setup 0x17E1", (0x1711, 0x1787)),
        (0x1802, 0x1810, "func_001802_slot2_selection_init.asm",
         "internal selection transition reached by callback 0x171B", (0x1810,)),
        (0x1810, 0x181B, "func_001810_slot2_selection_restart.asm",
         "callback published by 0x1868 and entered by 0x1802", (0x181B,)),
        (0x181B, 0x1858, "func_00181b_slot2_selection_begin.asm",
         "callback published by internal transition 0x1810", (0x1858, 0x19CF, 0x1A11)),
        (0x1858, 0x1868, "func_001858_slot2_selection_damp.asm",
         "callback published by selection callback 0x181B", (0x1868, 0x18D9)),
        (0x1868, 0x18D9, "func_001868_slot2_selection_approach.asm",
         "callback published by damping callback 0x1858", (0x1810, 0x18D9, 0x1952, 0x19CF)),
        (0x18D9, 0x1952, "func_0018d9_slot2_steering_helper.asm",
         "near carry-return helper called by callbacks 0x1858 and 0x1868", ()),
        (0x1952, 0x1957, "func_001952_slot2_finish_setup.asm",
         "internal callback transition reached by 0x1868", (0x1957,)),
        (0x1957, 0x19CF, "func_001957_slot2_finish_update.asm",
         "callback published by internal transition 0x1952", (0x1802,)),
        (0x19CF, 0x1A03, "func_0019cf_slot2_selection_reset_restart.asm",
         "shared selection-reset tail reached by callbacks 0x181B and 0x1868", (0x1711,)),
        (0x1A03, 0x1A11, "func_001a03_slot2_active_reset_setup.asm",
         "compiled active/reset setup with no table or in-overlay pointer reference", (0x1A11,)),
        (0x1A11, 0x1B3B, "func_001a11_slot2_reset_or_camera.asm",
         "shared reset and camera tail reached by callbacks 0x171B and 0x181B", (0x1781,)),
        (0x1B3B, 0x1B80, "func_001b3b_unreferenced_steering_update.asm",
         "compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference", ()),
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
