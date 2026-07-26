#!/usr/bin/env python3
"""Which DS cells does the game WRITE wider than the port reads them?

audit-fixes #271. The alien camera's X was recorded as "genuinely a word" because
`movsx eax, word ptr [0x22ec]` reads sixteen bits there. It is not a word: four
instructions earlier `add dword ptr [0x22ea], eax` writes a 32-bit accumulator
whose HIGH WORD sits at `0x22EC`. The load told me what the caller wanted; only
the store tells me how wide the cell is.

That mistake is mechanical, so this looks for it mechanically. Decoding forward
from known routine entries, it records the WIDEST WRITE to each fixed DS address,
and flags any address a port constant names as `u16` that the game writes as a
dword — or any address that is `base + 2` of such a write, which is the high-word
case that fooled me.

WHAT THIS CANNOT DO, learned from its own first run. It originally also matched
port constants: any `const X: u16 = <addr>` naming a 32-bit cell was flagged. That
found exactly one hit, `SHIP_3D_PLANAR_FRAMEBUFFER_PTR_DS_OFFSET = 0x5219`, and it
is a FALSE POSITIVE — the constant is `u16` because a DS OFFSET is sixteen bits,
while the four bytes at that address are a far pointer (`les di,ptr [0x5219]`).

Every DS constant in this tree is an address, so its type says nothing about the
cell's width and the match is vacuous. The #271 bug was in a STRUCT FIELD
(`x: i16` holding what should have been an accumulator), which no type-matching on
constants could have caught.

So the matching is gone and the WIDTH REPORT stays: knowing which cells the game
writes 32 bits wide is real decode information, and reading it is how #271 was
found in the first place.

Usage:
    python3 re/tools/cell_widths.py <entry_hex> [more...] [--image path]
"""

import os
import re
import sys

# capstone BEFORE this directory joins sys.path: re/tools/dis.py shadows the
# stdlib `dis` that capstone -> inspect imports.
import capstone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def load(path):
    if path:
        return open(path, "rb").read()
    from mzfile import MZ

    return MZ().data


def widest_writes(md, data, entries, limit=4096):
    """address -> widest byte-width the code STORES there."""
    width = {}
    for entry in entries:
        for insn in md.disasm(data[entry : entry + limit], entry):
            # A store has the memory operand FIRST (dst, src).
            if insn.operands and insn.operands[0].type == capstone.x86.X86_OP_MEM:
                mem = insn.operands[0].mem
                if mem.base == 0 and mem.index == 0 and mem.disp:
                    at = mem.disp & 0xFFFF
                    width[at] = max(width.get(at, 0), insn.operands[0].size)
            if insn.mnemonic in ("ret", "retf"):
                break
    return width


def main():
    # Drop flags AND their values: `--image path` used to leave the PATH in the
    # positional list, where it was parsed as a hex address (the same bug
    # `find_imm.py` fixed for `--max`).
    raw = sys.argv[1:]
    args, image, skip = [], None, False
    for i, a in enumerate(raw):
        if skip:
            skip = False
            continue
        if a == "--image":
            image = raw[i + 1] if i + 1 < len(raw) else None
            skip = True
            continue
        if a.startswith("--"):
            continue
        args.append(a)
    if not args:
        print(__doc__)
        return 0

    data = load(image)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True
    entries = [int(a, 16) for a in args]
    width = widest_writes(md, data, entries)

    wide = {at: w for at, w in width.items() if w >= 4}
    print(f"{len(width)} written DS cells; {len(wide)} written 32 bits wide")

    for at, w in sorted(wide.items()):
        print(f"   {at:#06x} written {w} bytes wide; its high word is {at + 2:#06x}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
