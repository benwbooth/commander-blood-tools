#!/usr/bin/env python3
"""Capture BBB's original spoken-number formatter and dictionary cursor behavior."""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS, SOURCE, STATE, DICTIONARY = 0x30000, 0x40000, 0x50000, 0x60000


def run(executable, dictionary, words, state, name):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable[HEADER:])
    source = struct.pack(f"<{len(words)}H", *words)
    cpu.mem_write(SOURCE, source)
    cpu.mem_write(STATE, state)
    cpu.mem_write(DICTIONARY, dictionary)
    cpu.mem_write(GLOBALS + 0x6AEC, struct.pack("<HH", 0, STATE // 16))
    for reg, value in [(UC_X86_REG_CS, 0x502), (UC_X86_REG_DS, SOURCE // 16),
                       (UC_X86_REG_ES, GLOBALS // 16), (UC_X86_REG_GS, GLOBALS // 16),
                       (UC_X86_REG_SS, GLOBALS // 16), (UC_X86_REG_SP, 0xFF00),
                       (UC_X86_REG_BX, DICTIONARY // 16), (UC_X86_REG_DI, 0x1066),
                       (UC_X86_REG_SI, 0), (UC_X86_REG_DX, 0)]:
        cpu.reg_write(reg, value)
    ended = []

    def instruction(machine, address, size, _context):
        offset = address + HEADER
        if offset == 0x6DEA:
            ended.append(True)
            machine.emu_stop()
            return
        assert any(start <= offset and offset + size <= end for start, end in
                   [(0x6D58, 0x6DEA), (0x6E54, 0x6E68), (0x2832, 0x286B)]), hex(offset)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x6D58 - HEADER, 0, count=10000)
    assert ended == [True]
    output = bytes(cpu.mem_read(GLOBALS + 0x1066, cpu.reg_read(UC_X86_REG_DI) - 0x1066))
    assert output[-1:] == b"\0"
    assert bytes(cpu.mem_read(SOURCE, len(source))) == source
    assert bytes(cpu.mem_read(STATE, len(state))) == state
    assert bytes(cpu.mem_read(DICTIONARY, len(dictionary))) == dictionary
    return dict(name=name, dictionary=list(dictionary), words=words, state=list(state),
                output=list(output[:-1]), cursor=cpu.reg_read(UC_X86_REG_SI))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    dictionary = b"\0\0There\0are\0people\0.\0" + b"a" * 38 + b"\0,\0tail\0"
    cases = []
    for value in [0, 1, 9, 10, 99, 100, 32767, 32768, 65535]:
        for operand in [12, 17, 20, 58, 59]:
            state = bytearray(96)
            struct.pack_into("<H", state, operand, value)
            cases.append(run(executable, dictionary, [2, 8, 1, operand, 12, 19, 0],
                             bytes(state), f"value_{value}_operand_{operand}"))
    args.output.write_text(json.dumps(dict(executable_sha256=SHA256, cases=cases), indent=2) + "\n")
    print(f"verified {len(cases)} original spoken-number cases")


if __name__ == "__main__":
    main()
