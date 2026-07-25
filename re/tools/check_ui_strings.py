#!/usr/bin/env python3
"""Which of the game's UI strings does any code actually reference?

The UI string table (DS:0x100..) holds the labels the game can draw. A string with
NO reference is dead data the port must not implement; a string WITH references
that the port never draws is a MISSING SURFACE. Both are worth knowing, and the
port cannot tell you about a screen it never knew existed -- which is how the
`ARE_YOU_SURE?` dialog was found.

For each string this searches the image for `mov <reg>,imm16` carrying its DS
offset (the form every draw site uses: `mov si,0x17B` then a draw call), plus the
bare little-endian word as a weaker signal.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from mzfile import DEFAULT_BIN  # noqa: E402

DS_BASE = 0xD420
# `mov r16,imm16` opcodes B8..BF: ax bx cx dx sp bp si di
MOV_IMM = {0xB8: "ax", 0xB9: "cx", 0xBA: "dx", 0xBB: "bx", 0xBD: "bp", 0xBE: "si", 0xBF: "di"}


def strings_in(data, start, end):
    out, i = [], start
    while i < end:
        j = i
        while j < end and 32 <= data[j] < 127:
            j += 1
        if j - i >= 2 and j < end and data[j] == 0:
            out.append((i - DS_BASE, data[i:j].decode("latin-1")))
            i = j + 1
        else:
            i += 1
    return out


def main():
    data = open(DEFAULT_BIN, "rb").read()
    table = strings_in(data, 0xD520, 0xD640)
    print(f"{len(table)} UI strings\n")
    for ds, text in table:
        lo, hi = ds & 0xFF, (ds >> 8) & 0xFF
        movs, bare = [], 0
        for off in range(len(data) - 3):
            if data[off + 1] == lo and data[off + 2] == hi:
                if data[off] in MOV_IMM:
                    movs.append((off, MOV_IMM[data[off]]))
        for off in range(len(data) - 2):
            if data[off] == lo and data[off + 1] == hi:
                bare += 1
        mark = "     " if movs else "DEAD "
        sites = ", ".join(f"{o:#07x}/{r}" for o, r in movs[:4])
        print(f"  {mark}DS:{ds:#06x} {text!r:<18} mov-imm refs: {len(movs):<3} {sites}")
    print(
        "\nDEAD = no `mov reg,imm` anywhere carries this offset, so no draw site "
        "loads it.\nThose are shipped-but-unreachable labels; the port must not "
        "implement them."
    )


if __name__ == "__main__":
    raise SystemExit(main())
