#!/usr/bin/env python3
"""Capture every descriptor selected by the original sequel's B763 helper.

The helper and original index/data bytes are unmodified. Captures describe initial
templates, not DESCRIPT mutations, scene-start policy, or media playback.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
DATA_FILE = 0xF7F0
GLOBALS = 0x30000
STACK = 0xFF00
RETURN = 0x200
INDEX = 0x2203
DESCRIPTOR_END = 0x2745


def capture(executable, line):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    data = bytearray(0x10000)
    original = executable[DATA_FILE:]
    data[:len(original)] = original
    struct.pack_into("<H", data, STACK, RETURN)
    cpu.mem_write(GLOBALS, bytes(data))
    registers = {UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
                 UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: STACK,
                 UC_X86_REG_AX: line, UC_X86_REG_BX: 0xA55A}
    for register, value in registers.items():
        cpu.reg_write(register, value)

    def instruction(_cpu, address, size, _context):
        assert 0xB763 <= address + HEADER < address + HEADER + size <= 0xB771

    def write(*_args):
        raise AssertionError("the descriptor selector must not write memory")

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0xB763 - HEADER, RETURN, count=32)
    pointer = cpu.reg_read(UC_X86_REG_BX)
    registers[UC_X86_REG_SP] += 2
    del registers[UC_X86_REG_BX]
    for register, value in registers.items():
        assert cpu.reg_read(register) == value
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(GLOBALS, len(data))) == data
    assert INDEX + 45 * 4 <= pointer < DESCRIPTOR_END - 2
    end = struct.unpack_from("<H", data, INDEX + (line + 1) * 4)[0] if line < 44 else DESCRIPTOR_END
    name_field = data[pointer + 2:end]
    terminator = name_field.index(0)
    return dict(line=line, descriptor_offset=pointer, flags=data[pointer], variant=data[pointer + 1],
                name=list(name_field[:terminator]),
                scene_image_offset=struct.unpack_from("<H", data, INDEX + line * 4 + 2)[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    first_descriptor = struct.unpack_from("<H", executable, DATA_FILE + INDEX)[0]
    assert first_descriptor - INDEX == 45 * 4
    result = dict(executable_sha256=SHA256,
                  unclamped_line_ids=list(executable[DATA_FILE + 0x100C:DATA_FILE + 0x1014]),
                  lines=[capture(executable, line) for line in range(45)])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print("captured all 45 original presentation descriptor selections")


if __name__ == "__main__":
    main()
