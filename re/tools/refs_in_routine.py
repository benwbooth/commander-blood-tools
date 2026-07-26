#!/usr/bin/env python3
"""Which DS addresses does a routine touch, decoding forward from its ENTRY?

`find_imm.py` scans the whole image from every byte offset and filters phantoms by
anchor agreement. That is the right tool when you do not know where code is, and
audit-fixes #234 shows its limit: several hits for `0x2736`/`0x2737` sat at file
offsets like `0x010af`, inside the header, where "an instruction" is a decode of
data. A citation whose only support is that scan is a restatement, not evidence —
and the citation guard cannot tell, because it disassembles at the same phantom
address and agrees with itself.

This takes the opposite approach. Given a routine's VERIFIED entry point, it
decodes forward linearly to the terminating `ret`/`retf` and reports every memory
displacement the instructions reference. Every hit is therefore inside real code
at a real instruction boundary, because the decode started somewhere known.

The entry points worth passing are the ones already checked against the image —
`bloodprg.rs`'s segment/offset constants, verified in #232 and #256 to land on a
prologue preceded by a `retf`.

Usage:
    python3 re/tools/refs_in_routine.py <entry_hex> [more...] [--max N]

Example:
    python3 re/tools/refs_in_routine.py 0xB692     # the transition updater
"""

import os
import sys

# capstone BEFORE this directory joins sys.path: re/tools/dis.py shadows the
# stdlib `dis` that capstone -> inspect imports.
import capstone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def load():
    from mzfile import MZ

    return MZ().data


def walk(md, data, entry, limit):
    """(address, mnemonic, op_str, displacement, immediate) per instruction.

    IMMEDIATES matter as much as displacements. A constant naming an ADDRESS shows
    up as `mov ax,[0x2527]`; one naming a VALUE -- a mask, a sentinel, a mode --
    shows up as `mov word [0x524d],0xa`, and reporting only displacements finds
    the address while missing the value beside it.

    BUT IMMEDIATES ARE NOT EVIDENCE THE WAY DISPLACEMENTS ARE, and the difference
    is measured (audit-fixes #263). Across 18 routines there are 230 distinct
    immediates; 137 uncited port constants share a value with one, and 86 of those
    -- nearly two thirds -- appear in MORE THAN ONE routine. A DS address is
    unique to what it names; the number 4 is not.

    So this output supports a citation you already have a reason to believe (the
    transition constants matched `mov byte [0x2531],4` in the routine they are
    about, at the instruction writing the byte they name). It does NOT support
    bulk-citing constants by value, which would manufacture evidence at exactly
    the scale that is hard to notice afterwards.
    """
    out = []
    for insn in md.disasm(data[entry : entry + limit], entry):
        disp = imm = None
        for op in insn.operands:
            if op.type == capstone.x86.X86_OP_MEM and op.mem.disp:
                # A DS-relative reference has no base/index register; anything
                # else ([bx+si], [bp+4]) is a struct field, not a fixed address.
                if op.mem.base == 0 and op.mem.index == 0:
                    disp = op.mem.disp & 0xFFFF
            elif op.type == capstone.x86.X86_OP_IMM:
                imm = op.imm & 0xFFFF
        out.append((insn.address, insn.mnemonic, insn.op_str, disp, imm))
        if insn.mnemonic in ("ret", "retf"):
            break
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = 4096
    if "--max" in sys.argv:
        limit = int(sys.argv[sys.argv.index("--max") + 1])
    if not args:
        print(__doc__)
        return 0

    data = load()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    for arg in args:
        entry = int(arg, 16)
        insns = walk(md, data, entry, limit)
        print(f"\n=== {entry:#07x} ({len(insns)} instructions to ret) ===")
        seen, imms = {}, {}
        for addr, mnem, ops, disp, imm in insns:
            if disp is not None:
                seen.setdefault(disp, (addr, mnem, ops))
            # Skip tiny immediates: 0/1/2 appear in nearly every routine and
            # matching a port constant on them would be coincidence, not evidence.
            if imm is not None and imm > 2:
                imms.setdefault(imm, (addr, mnem, ops))
        for disp, (addr, mnem, ops) in sorted(seen.items()):
            print(f"   DS:{disp:#06x}  first touched at {addr:#07x}: {mnem} {ops}")
        for imm, (addr, mnem, ops) in sorted(imms.items()):
            print(f"   IMM:{imm:#06x} first used at   {addr:#07x}: {mnem} {ops}")
        if not seen and not imms:
            print("   (no fixed DS references or immediates)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
