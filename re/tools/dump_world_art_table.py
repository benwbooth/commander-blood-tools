#!/usr/bin/env python3
"""Join the WORLD-ARTWORK table (DS:0x2BC7) to the resource-name table (file
0xCDF4) and print the result.

The info panel's first zoom frame (`0x9098..0x90C3`) walks 22-byte records at
DS:0x2BC7 comparing each record's name against the selected object's inline name;
on a match it takes `[si+0x10]`, ORs in `0x8000` and loads that resource. So
`+0x10` is a RESOURCE ID, and the id indexes the 16-byte filename records the
port already decodes as `LEVEL_DIRECTORY`. This script checks that every id in
the artwork table resolves to a real filename.
"""

import os
import struct

RE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(RE_ROOT, "bin", "BLOODPRG.EXE")

DS_BASE = 0xD420
ART_TABLE_DS = 0x2BC7
ART_RECORD = 0x16
NAME_TABLE_FILE = 0xCDF4
NAME_RECORD = 16


def main():
    data = open(BIN, "rb").read()
    base = DS_BASE + ART_TABLE_DS
    rows = []
    index = 0
    while True:
        rec = data[base + index * ART_RECORD : base + (index + 1) * ART_RECORD]
        if len(rec) < ART_RECORD or rec[0] == 0:
            break
        name = rec[:16].split(b"\0")[0].decode("latin-1")
        rid, group, extra = struct.unpack("<3H", rec[16:22])
        rows.append((name, rid, group, extra))
        index += 1

    missing = 0
    for name, rid, group, extra in rows:
        off = NAME_TABLE_FILE + rid * NAME_RECORD
        raw = data[off : off + NAME_RECORD].split(b"\0")[0].decode("latin-1")
        if not raw:
            missing += 1
        print(f"{name:<16} id {rid:3d} group {group:2d} extra {extra} -> {raw!r}")

    end = ART_TABLE_DS + len(rows) * ART_RECORD
    print()
    print(f"{len(rows)} entries, {len(rows) * ART_RECORD} bytes, ends at DS:{end:#06x}")
    print(f"{missing} id(s) with no filename record")


if __name__ == "__main__":
    main()
