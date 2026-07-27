#!/usr/bin/env python3
"""The VM opcode -> handler map, read out of the dispatch table (audit-fixes #517).

The table at `DS:0x6EB0` (file 0x142D0) holds NEAR OFFSETS, not addresses: entry
`i` belongs to opcode `0xA0 + i`, and the offset is relative to the segment based
at file 0x53A0 (seg 0x4DA). Anyone reading the raw words and treating them as file
offsets gets numbers in the 0x11xx-0x14xx range that disassemble into plausible
nonsense, which is the trap this tool exists to remove.

The base is CHECKED, not assumed, against four handlers with independent decodes
(0xA0, 0xA6, 0xB7, 0xB8). If those stop matching, the table or the base moved and
every result here is suspect -- so the check is fatal rather than a warning.

Usage:
    python3 re/tools/vm_dispatch.py              # the whole map, grouped
    python3 re/tools/vm_dispatch.py 0xB1 0xC0    # specific opcodes
"""
import struct
import sys
from collections import defaultdict

EXE = "re/bin/BLOODPRG.EXE"
TABLE = 0x142D0        # = DS:0x6EB0
CODE_BASE = 0x53A0     # = 0x600 + 0x4DA * 16
OP_MIN = 0xA0
ENTRIES = 52           # 104 bytes / 2

# handlers whose addresses were established independently of this table
KNOWN = {0xA0: 0x6559, 0xA6: 0x660C, 0xB7: 0x6AA7, 0xB8: 0x6B06}


def load():
    data = open(EXE, "rb").read()
    table = {}
    for i in range(ENTRIES):
        off = struct.unpack_from("<H", data, TABLE + i * 2)[0]
        table[OP_MIN + i] = CODE_BASE + off
    for op, want in KNOWN.items():
        got = table.get(op)
        if got != want:
            raise SystemExit(
                f"BASE CHECK FAILED: opcode {op:#04x} -> {got:#07x}, expected {want:#07x}.\n"
                "The table or the code base moved; do not trust any output."
            )
    return table


def main():
    table = load()
    wanted = [int(a, 16) for a in sys.argv[1:]]
    if wanted:
        for op in wanted:
            print(f"  {op:#04x} -> {table.get(op, 0):#07x}")
        return

    shared = defaultdict(list)
    for op, h in sorted(table.items()):
        shared[h].append(op)

    print(f"{len(table)} opcodes ({OP_MIN:#04x}..{OP_MIN + ENTRIES - 1:#04x}) "
          f"-> {len(shared)} distinct handler(s)\n")
    for h, ops in sorted(shared.items()):
        tag = "  <-- SHARED" if len(ops) > 1 else ""
        print(f"  {h:#07x}  {' '.join(f'{o:#04x}' for o in ops)}{tag}")


if __name__ == "__main__":
    main()
