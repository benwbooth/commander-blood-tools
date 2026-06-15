#!/usr/bin/env python3
"""Dump the script-VM opcode handler jump table.

Master dispatch (executor @0x5613): `call word ptr gs:[bx + 0x6eb0]` where
bx=(opcode-0xa0)*2. So DS:0x6EB0 (file 0x142D0) is a 52-entry table of NEAR
offsets into code segment 0x04DA (file base 0x53A0), for opcodes 0xA0..0xD3.
Immediately followed by the length table at DS:0x6F18.

Usage: python3 re/tools/dump_handler_table.py
"""
import os
import struct
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
sys.path.insert(0, _here)
from mzfile import MZ, load_labels

TABLE_FILE_OFF = 0x142D0      # DS:0x6EB0
SEG_04DA_FILE_BASE = 0x053A0  # segment 0x04DA file base
N = 0x34                       # 52 opcodes 0xA0..0xD3


def main():
    mz = MZ()
    _, lbl_file = load_labels()
    print(f"opcode  near_off  handler_file  label")
    seen = {}
    for i in range(N):
        op = 0xA0 + i
        near = struct.unpack_from("<H", mz.data, TABLE_FILE_OFF + i * 2)[0]
        foff = SEG_04DA_FILE_BASE + near
        lab = lbl_file.get(foff)
        labtxt = f"<{lab[0]}> {lab[1]}" if lab else ""
        # group opcodes that share a handler (common default/no-op)
        seen.setdefault(near, []).append(op)
        print(f"0x{op:02x}    {near:#06x}    {foff:#08x}    {labtxt}")
    print("\n# handlers shared by multiple opcodes (likely default/no-op or family):")
    for near, ops in sorted(seen.items()):
        if len(ops) > 1:
            print(f"  {SEG_04DA_FILE_BASE+near:#08x}: "
                  + " ".join(f"0x{o:02x}" for o in ops))


if __name__ == "__main__":
    main()
