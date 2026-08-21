#!/usr/bin/env python3
"""Enumerate far-pointer stores into game data in BLOODPRG's code segments.

Scans the whole load image with capstone (16-bit 386) and reports every
instruction of the shape

    mov word ptr GS:[disp16], imm16      ; one half of a baked far pointer
    mov dword ptr GS:[disp16], imm32     ; a complete seg:off pair
    mov word ptr DS:[disp16], imm16      ; same, DS-owned

plus register stores `mov GS:[disp16], reg16`, because pairs are often built
in two halves. The output is an inventory: which game-data offsets receive
pointer values from code. Anything NOT covered by a recovered candidate
declaration is state initialized only by unrecovered machine code -- exactly
the class that silently diverges after the relink.

Usage: python3 re/tools/audit_far_pointer_stores.py [--output FILE]
"""
from __future__ import annotations

import argparse
import csv
import importlib.util
import sys
from pathlib import Path

import capstone

ROOT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location(
    "re_mzfile", ROOT / "re" / "tools" / "mzfile.py")
_module = importlib.util.module_from_spec(_spec)
sys.modules["re_mzfile"] = _module
_spec.loader.exec_module(_module)
MZ = _module.MZ

SEGMENT_NAMES = {
    0x26: "ES", 0x2E: "CS", 0x36: "SS",
    0x3E: "DS", 0x64: "FS", 0x65: "GS",
}


def segment_prefix(insn) -> str | None:
    """capstone 5 exposes the override as a raw prefix byte."""
    override = insn.prefix[1] if len(insn.prefix) > 1 else 0
    return SEGMENT_NAMES.get(override)


def mem_displacement(insn) -> int | None:
    operand = insn.operands[0]
    if operand.type != capstone.x86_const.X86_OP_MEM:
        return None
    memory = operand.mem
    base = insn.reg_name(memory.base) if memory.base else ""
    index = insn.reg_name(memory.index) if memory.index else ""
    # pure disp16 forms only: [0x1234] or [bp+0x1234] style via base none.
    # bp-relative frames are locals, not globals.
    if base in ("bp", "ebp") or index:
        return None
    if base not in ("",):
        return None
    return memory.disp & 0xFFFF


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", type=Path,
                        default=ROOT / "re/bin/BLOODPRG.EXE")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    mz = MZ()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    rows = []
    for insn in md.disasm(mz.data[mz.header_size:mz.image_total],
                          mz.header_size):
        mnemonic = insn.mnemonic
        if mnemonic not in ("mov", "lea"):
            continue
        seg = segment_prefix(insn)
        if seg is None:
            continue
        displacement = mem_displacement(insn)
        if displacement is None:
            continue
        operands = insn.operands
        if len(operands) < 2:
            continue
        source = operands[1]
        detail = ""
        if source.type == capstone.x86_const.X86_OP_IMM:
            detail = f"imm={source.imm & 0xFFFFFFFF:#x}"
        elif source.type == capstone.x86_const.X86_OP_REG:
            detail = f"reg={insn.reg_name(source.reg)}"
        else:
            continue
        rows.append((
            insn.address,
            seg,
            f"{displacement:#06x}",
            mnemonic,
            detail,
            insn.mnemonic + " " + insn.op_str,
        ))

    stream_args: dict = {}
    stream = open(args.output, "w", newline="", encoding="ascii") \
        if args.output else sys.stdout
    writer = csv.writer(stream, delimiter="\t", **stream_args)
    writer.writerow(("file_offset", "segment", "target", "op", "source",
                     "text"))
    writer.writerows(rows)
    if args.output:
        stream.close()
    print(f"{len(rows)} segment-prefixed global stores")


if __name__ == "__main__":
    main()
