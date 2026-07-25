#!/usr/bin/env python3
"""Find instructions whose IMMEDIATE or memory DISPLACEMENT equals a value.

Answers "where does the code actually use this number?" — the question a constant
needs answered before it can carry a citation. Several port constants documented
their VALUE and cited no instruction (`MENU_ANGLE_MASK = 0x0FFC` said "0xFFC = a
10-bit angle"), which is a value restated rather than a provenance.

Because x86 is variable-length, an instruction can start at any byte; this decodes
from every offset and keeps hits whose start is confirmed by decoding from several
EARLIER anchors, so a phantom resynchronised mid-instruction is not reported (the
failure mode fixed in audit-fixes #101 and #106).

Usage:
    python3 re/tools/find_imm.py <value_hex> [file] [--max N]

`file` defaults to the main image; pass e.g. output/_tmp_dat/manu3.xdb for an
overlay, whose offsets map 1:1 to runtime cs.
"""

import os
import sys

# capstone BEFORE this directory joins sys.path: re/tools/dis.py shadows the
# stdlib `dis` that capstone -> inspect imports.
import capstone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def load(path):
    if path:
        return open(path, "rb").read(), os.path.basename(path)
    from mzfile import MZ

    return MZ().data, "BLOODPRG.EXE"


def confirmed(md, data, at):
    """Do earlier decode anchors agree `at` starts an instruction?"""
    agree = total = 0
    for back in range(6, 34, 4):
        anchor = max(0, at - back)
        total += 1
        for insn in md.disasm(data[anchor : at + 16], anchor):
            if insn.address == at:
                agree += 1
                break
            if insn.address > at:
                break
    return total and agree * 2 > total


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        print(__doc__)
        return 0
    want = int(args[0], 16)
    path = args[1] if len(args) > 1 else None
    limit = 20
    if "--max" in sys.argv:
        limit = int(sys.argv[sys.argv.index("--max") + 1])

    data, label = load(path)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    hits = {}
    for start in range(len(data) - 8):
        for insn in md.disasm(data[start : start + 8], start):
            for op in insn.operands:
                if op.type == capstone.x86.X86_OP_IMM and op.imm == want:
                    hits.setdefault(start, (insn.mnemonic, insn.op_str))
                # A base address is usually a DISPLACEMENT, not an immediate:
                # `mov ax,[0x2274]` carries it in the memory operand.
                elif op.type == capstone.x86.X86_OP_MEM and op.mem.disp == want:
                    hits.setdefault(start, (insn.mnemonic, insn.op_str))
            break

    real = [(a, v) for a, v in sorted(hits.items()) if confirmed(md, data, a)]
    print(f"{label}: {len(real)} confirmed instruction(s) with immediate {want:#x} "
          f"({len(hits) - len(real)} rejected as mid-instruction phantoms)")
    for a, (m, o) in real[:limit]:
        print(f"  {a:#07x}: {m} {o}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
