#!/usr/bin/env python3
"""Execute BBB A3 with both concept slots populated, including empty resume slots."""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS, SOURCE = 0x30000, 0x40000


def run(executable, phase, primary, alternate, expected, inverted):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[0x800:]
    cpu.mem_write(0, module)
    before = bytearray(0x10000)
    before[0x6B87] = phase
    before[0x6B83] = 1
    struct.pack_into("<HH", before, 0x6B34, primary, alternate)
    struct.pack_into("<H", before, 0x6C2C, 2)
    struct.pack_into("<H", before, 0x6C04, 0x1234)
    cpu.mem_write(GLOBALS, bytes(before))
    source = (b"\xA1" if inverted else b"") + struct.pack("<H", expected)
    cpu.mem_write(SOURCE, source)
    for register, value in [(UC_X86_REG_CS, 0x502), (UC_X86_REG_DS, SOURCE // 16),
                            (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                            (UC_X86_REG_SP, 0xFF00), (UC_X86_REG_SI, 0)]:
        cpu.reg_write(register, value)
    ended = []

    def instruction(machine, address, size, _context):
        offset = address + 0x800
        assert any(start <= offset < offset + size <= end for start, end in [(0x6AB2, 0x6AF7), (0x697A, 0x6993)]), hex(offset)
        if offset == 0x6AF6:
            ended.append(True)
            machine.emu_stop()

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x6AB2 - 0x800, 0, count=1000)
    assert ended == [True] and cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    assert all(a == b or i in (0x6B83, 0x6C2C, 0x6C2D) or 0xFEFA <= i < 0xFF00
               for i, (a, b) in enumerate(zip(before, after)))
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE, len(source))) == source
    cursor = cpu.reg_read(UC_X86_REG_SI)
    assert cursor in (len(source), 0x1234)
    continues = cursor == len(source)
    assert struct.unpack_from("<H", after, 0x6C2C)[0] == (2 if continues else 0)
    return dict(phase=phase, primary=primary, alternate=alternate, expected=expected,
                inverted=inverted, continues=continues)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = [run(executable, *case) for case in itertools.product(range(4), (0, 32, 64), (0, 32, 64), (32, 64), (False, True))]
    args.output.write_text(json.dumps(dict(executable_sha256=SHA256, cases=cases), indent=2) + "\n")
    print(f"verified {len(cases)} original two-slot concept guards")


if __name__ == "__main__":
    main()
