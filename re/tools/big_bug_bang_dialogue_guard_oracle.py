#!/usr/bin/env python3
"""Probe the sequel's authored Daddy choice/guard identity mismatch.

Execute the unmodified A3 handler, stopping at its real guard-failure callee
or its normal return. No game instructions or resource bytes are exported.
Run with python -P so the adjacent dis.py cannot shadow the standard library.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP, UC_X86_REG_EFLAGS,
)

HASHES = {
    "executable": "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834",
    "SCRIPT3.COD": "6d137440a7ace2650ff6c5c00c745176e7469c42cdb2008f68b729842a8ebfed",
    "SCRIPT3.DIC": "d458a17c6ba3d39f978edaf4c9985af28b08f33640bac4588ae0b3361b91c721",
}
ENTRY = 0x6AB2
FAIL = 0x697A
RETURN = 0x200
SCRIPT = 0x20000
GLOBALS = 0x30000
STACK = 0xFF00


def run(executable, expected, inverted, selected, alternate):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    operand = (b"\xa1" if inverted else b"") + struct.pack("<H", expected)
    cpu.mem_write(SCRIPT, operand)
    before = bytearray(0x10000)
    # Poison the inactive slot with the opposite match result.
    inactive = 0 if selected == expected else expected
    struct.pack_into("<HH", before, 0x6B34,
                     inactive if alternate else selected,
                     selected if alternate else inactive)
    before[0x6B87] = 2 if alternate else 0
    struct.pack_into("<H", before, STACK, RETURN)
    cpu.mem_write(GLOBALS, bytes(before))
    registers = {
        UC_X86_REG_CS: 0, UC_X86_REG_DS: SCRIPT // 16,
        UC_X86_REG_GS: GLOBALS // 16, UC_X86_REG_SS: GLOBALS // 16,
        UC_X86_REG_SP: STACK, UC_X86_REG_SI: 0, UC_X86_REG_EFLAGS: 2,
    }
    for register, value in registers.items():
        cpu.reg_write(register, value)
    failed = False

    def instruction(machine, address, size, _context):
        nonlocal failed
        if address == FAIL:
            failed = True
            machine.emu_stop()
        else:
            assert ENTRY <= address < address + size <= 0x6AF7, hex(address)

    def write(_machine, _access, address, size, _value, _context):
        assert GLOBALS + STACK - 4 <= address < address + size <= GLOBALS + STACK

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY, RETURN, count=100)
    assert cpu.reg_read(UC_X86_REG_IP) == (FAIL if failed else RETURN)
    assert cpu.reg_read(UC_X86_REG_SI) == len(operand)
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    assert bytes(cpu.mem_read(SCRIPT, len(operand))) == operand
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    assert after[:STACK - 4] == before[:STACK - 4]
    assert after[STACK:] == before[STACK:]
    return dict(expected=expected, inverted=inverted, selected=selected,
                alternate=alternate, continues=not failed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("resources", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    images = {"executable": args.executable.read_bytes()}
    images.update({name: (args.resources / name).read_bytes()
                   for name in ("SCRIPT3.COD", "SCRIPT3.DIC")})
    for name, data in images.items():
        if hashlib.sha256(data).hexdigest() != HASHES[name]:
            raise ValueError(f"unrecognized {name}; refusing fixed-offset probe")
    code, dictionary = images["SCRIPT3.COD"], images["SCRIPT3.DIC"]
    assert struct.unpack_from("<HH", code, 0xD7F) == (0x6F7, 0x700)
    assert code[0xD88:0xD8B] == b"\xa3\x07\x07"
    assert code[0xDAC:0xDB0] == b"\xa3\xa1\x3a\x07"
    assert dictionary[0x6F7:0x6FF] == b"bien-\x87a\0"
    assert dictionary[0x707:0x70F] == b"bien_\x87a\0"
    rows = [run(images["executable"], expected, inverted, selected, alternate)
            for expected, inverted in ((0x707, False), (0x73A, True))
            for selected in (0, 0x6F7, 0x700, 0x707, 0x73A)
            for alternate in (False, True)]
    with args.output.open("x") as output:
        for row in rows:
            output.write(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n")
    print(f"verified {len(rows)} original Daddy guard cases")


if __name__ == "__main__":
    main()
