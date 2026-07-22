#!/usr/bin/env python3
"""Disassemble a .xdb overlay (raw 16-bit code loaded at segment base, offset 0).

The overlays (manu3/amer/croolis/scrut.xdb) are raw 386 real-mode code+data
images loaded verbatim from blood.dat; runtime cs maps 1:1 to file offsets
(verified: live segment 0x166C == manu3.xdb at identical offsets).

Usage: python3 tools/dis_xdb.py <file.xdb> <hexoff> [count]
"""
import sys
from capstone import Cs, CS_ARCH_X86, CS_MODE_16

path, off = sys.argv[1], int(sys.argv[2], 16)
count = int(sys.argv[3]) if len(sys.argv) > 3 else 40
data = open(path, 'rb').read()
md = Cs(CS_ARCH_X86, CS_MODE_16)
md.skipdata = True
for i, insn in enumerate(md.disasm(data[off:off + 0x800], off)):
    if i >= count:
        break
    print(f"{insn.address:#06x}: {insn.bytes.hex():<20} {insn.mnemonic:<8} {insn.op_str}")
