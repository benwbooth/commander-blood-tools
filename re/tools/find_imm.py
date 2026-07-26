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
import collections
import re
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
    """Do earlier decode anchors agree `at` starts an instruction?

    HAS FALSE NEGATIVES, and they are not rare (audit-fixes #334). The rule is a
    MAJORITY of seven back-anchors, and a real instruction whose neighbourhood
    decodes badly loses that vote. `mov byte ptr [0x2737], 1` @0x893C is genuine —
    the bytes are `c6 06 37 27 01`, immediately after an identical store to
    `0x2738` — and this function rejects it.

    So a ZERO RESULT FROM THIS TOOL IS NOT PROOF OF ABSENCE. When the argument
    depends on absence, search the raw ENCODINGS instead (e.g. `3c XX` for
    `cmp al,imm8`, `80 3e .. .. XX` for `cmp byte [imm16],imm8`); that is what
    #327's "the game never compares against 0x5F" now rests on.

    Rejected candidates are listed by `--rejected` rather than silently dropped.
    """
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
    # Drop flags AND their values: `--max 4` used to leave "4" in the positional
    # list, where it was read as a FILENAME and raised FileNotFoundError: '4'.
    raw = sys.argv[1:]
    args, skip = [], False
    for i, a in enumerate(raw):
        if skip:
            skip = False
            continue
        if a.startswith("--"):
            skip = a == "--max"
            continue
        args.append(a)
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

    real, rejected = [], []
    for a, v in sorted(hits.items()):
        (real if confirmed(md, data, a) else rejected).append((a, v))
    print(f"{label}: {len(real)} confirmed instruction(s) with immediate {want:#x} "
          f"({len(hits) - len(real)} rejected as mid-instruction phantoms)")

    # SHAPE FIRST (audit-fixes #309). A list of addresses invites reading the
    # first screenful and generalising; audit-fixes #308 did exactly that on a
    # 66-hit result and published a wrong claim about which bits of a flag byte
    # are written. Aggregating by OPERATION makes the populations visible in a
    # few lines, so a truncated read cannot hide one.
    ops = collections.Counter()
    for _, (m, o) in real:
        # Normalise away the address itself and the segment prefix so that
        # `or byte ptr gs:[0x2793], 4` and `or byte ptr [0x2793], 4` group.
        norm = re.sub(r"\b(?:byte|word|dword) ptr ", "", o)
        norm = norm.replace("gs:", "").replace(f"[{want:#x}]", "FLAG")
        ops[f"{m} {norm}"] += 1
    if len(ops) > 1:
        print("  --- by operation ---")
        for form, n in ops.most_common():
            print(f"  {n:>4}x  {form}")

    if "--rejected" in sys.argv:
        # Not noise by default: #334 found a REAL instruction in here.
        print("  --- rejected as phantoms (MAY CONTAIN REAL INSTRUCTIONS) ---")
        for a, (m, o) in rejected:
            print(f"  {a:#07x}: {m} {o}")

    shown = real[:limit]
    for a, (m, o) in shown:
        print(f"  {a:#07x}: {m} {o}")
    if len(real) > len(shown):
        # SAY SO. This used to truncate silently, which is how #308's read of
        # `| tail -12` saw twelve of sixty-six and never learned it.
        print(f"  ... {len(real) - len(shown)} more NOT SHOWN (--max {len(real)} for all)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
