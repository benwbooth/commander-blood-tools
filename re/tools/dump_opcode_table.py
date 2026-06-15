#!/usr/bin/env python3
"""Dump the script-VM per-opcode length/descriptor table.

Located at DS:0x6f18 (file 0x14338). Indexed by (opcode - 0xa0), 2 bytes each:
  byte0 = token length in mode 0  (gs:[0x67ad]==0)
  byte1 = token length in mode 1  (gs:[0x67ad]==1)
length 0 => variable/special handling (e.g. 0xa6 text: 5 bytes + 0-term words).
High bit set on byte1 marks a control/mode token (0xff/0xfe/0xfd/0xfb).
See decoder at file 0x62b6 (REVERSE.md ## Script VM).
"""
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
sys.path.insert(0, _here)
from mzfile import MZ

TABLE_FILE_OFF = 0x14338
N_OPCODES = 0x60  # 0xa0..0xff


def main():
    mz = MZ()
    base = TABLE_FILE_OFF
    print(f"opcode  b0  b1   mode0_len  mode1_len  note")
    for i in range(N_OPCODES):
        op = 0xA0 + i
        b0 = mz.data[base + i * 2]
        b1 = mz.data[base + i * 2 + 1]
        note = ""
        if b1 & 0x80:
            note = "CONTROL/mode token"
        elif b0 == 0 or b1 == 0:
            note = "variable/special"
        m0 = b0
        m1 = b1 & 0x7f if (b1 & 0x80) else b1
        print(f"0x{op:02x}    {b0:02x}  {b1:02x}   "
              f"{m0:<9} {m1:<9}  {note}")


if __name__ == "__main__":
    main()
