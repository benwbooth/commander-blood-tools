#!/usr/bin/env python3
"""Execute the original BBB audio hash with numeric-token dictionary offsets."""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS, UC_X86_REG_SP

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS, SOURCE, DICTIONARY = 0x30000, 0x40000, 0x50000


def run(executable, dictionary, number):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_before = bytearray(0x10000)
    globals_before[0xCE7] = 1
    globals_before[0xF47] = 1
    struct.pack_into("<HH", globals_before, 0x6AFC, 0, DICTIONARY // 16)
    struct.pack_into("<HH", globals_before, 0x6B1A, 0, SOURCE // 16)
    # A dictionary word, numeric marker + operand, another word, terminator.
    words = [2, 1, number, 2, 0]
    source = struct.pack("<5H", *words)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(SOURCE, source)
    cpu.mem_write(DICTIONARY, dictionary)
    for register, value in [(UC_X86_REG_CS, 0xC5C), (UC_X86_REG_DS, GLOBALS // 16),
                            (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
                            (UC_X86_REG_SP, 0xFF00)]:
        cpu.reg_write(register, value)
    ended = []

    def instruction(machine, address, size, _context):
        offset = address + HEADER
        assert 0xCF73 <= offset < offset + size <= 0xD05D, hex(offset)
        if offset == 0xD05C:
            ended.append(True)
            machine.emu_stop()

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0xCF73 - HEADER, 0, count=10000)
    assert ended == [True]
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE, len(source))) == source
    assert bytes(cpu.mem_read(DICTIONARY, len(dictionary))) == dictionary
    after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    assert all(a == b or i in (0xF47, 0xF48, 0xE5F, 0xE60) or 0xFEEE <= i < 0xFF00
               for i, (a, b) in enumerate(zip(globals_before, after)))
    assert after[0xF47] == 0 and after[0xF48] == 1
    return dict(number=number, seed=struct.unpack_from("<H", after, 0xE5F)[0])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    dictionary = b"\0\0hello\0signed\x80\xff\0tail\0"
    cases = [run(executable, dictionary, number) for number in [*range(len(dictionary)), 65535]]
    args.output.write_text(json.dumps(dict(executable_sha256=SHA256, dictionary=list(dictionary), cases=cases), indent=2) + "\n")
    print(f"verified {len(cases)} original numeric chatter cases")


if __name__ == "__main__":
    main()
