#!/usr/bin/env python3
"""Capture the original sequel startup loop with every destination present.

The DOS boundary supplies successful directory changes and find-first responses.
The original loop and directory helper execute unchanged; no copy routine is
entered. This verifies ordered visits, not source availability or file copying.
"""

import argparse
import hashlib
import json
from pathlib import Path

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_INTR, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
GLOBALS = 0x30000


def capture(executable):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    data = bytearray(0x10000)
    source = executable[0xF7F0:]
    assert len(source) <= len(data)
    data[:len(source)] = source
    assert data[0xCE9] == 0
    cpu.mem_write(GLOBALS, bytes(data))
    for register, value in {UC_X86_REG_CS: 0, UC_X86_REG_DS: GLOBALS // 16,
                            UC_X86_REG_ES: GLOBALS // 16, UC_X86_REG_GS: GLOBALS // 16,
                            UC_X86_REG_SS: GLOBALS // 16, UC_X86_REG_SP: 0xFF00,
                            UC_X86_REG_EFLAGS: 2}.items():
        cpu.reg_write(register, value)
    names = []
    directories = []
    drives = []
    enters = []
    attributes = []

    def instruction(_cpu, address, size, _context):
        file = address + HEADER
        assert any(lo <= file < file + size <= hi
                   for lo, hi in [(0x190C, 0x191F), (0x1944, 0x194C), (0x2B43, 0x2B69)])
        if file == 0x2B43:
            enters.append(cpu.reg_read(UC_X86_REG_SI))

    def write(_cpu, _access, address, size, _value, _context):
        assert any(GLOBALS + lo <= address < address + size <= GLOBALS + hi
                   for lo, hi in [(0xCE9, 0xCEA), (0xFEF6, 0xFF00)])

    def interrupt(_cpu, number, _context):
        assert number == 0x21
        ah = cpu.reg_read(UC_X86_REG_AH)
        if ah == 0x4E:
            pointer = cpu.reg_read(UC_X86_REG_DX)
            assert pointer == 0x2A0 + len(names) * 16
            assert cpu.reg_read(UC_X86_REG_DS) == GLOBALS // 16
            name = bytes(cpu.mem_read(GLOBALS + pointer, 16)).split(b"\0", 1)[0]
            assert 0 < len(name) < 16
            names.append(list(name))
            attributes.append(cpu.reg_read(UC_X86_REG_CX))
        elif ah == 0x0E:
            drives.append(cpu.reg_read(UC_X86_REG_DL))
        elif ah == 0x3B:
            assert cpu.reg_read(UC_X86_REG_DS) == GLOBALS // 16
            pointer = cpu.reg_read(UC_X86_REG_DX)
            directories.append(list(bytes(cpu.mem_read(GLOBALS + pointer, 32)).split(b"\0", 1)[0]))
        else:
            raise AssertionError(hex(ah))
        cpu.reg_write(UC_X86_REG_EFLAGS, cpu.reg_read(UC_X86_REG_EFLAGS) & ~1)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.hook_add(UC_HOOK_INTR, interrupt)
    cpu.emu_start(0x190C - HEADER, 0x194C - HEADER, count=8192)
    assert cpu.reg_read(UC_X86_REG_CS) == 0
    assert cpu.reg_read(UC_X86_REG_IP) == 0x194C - HEADER
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF00
    assert cpu.reg_read(UC_X86_REG_SI) == 0xC20
    assert len(names) == len(enters) == 152
    assert len(drives) == len(directories) == 1
    assert attributes == [0x18] * 152
    assert bytes(cpu.mem_read(0, len(module))) == module
    after = bytes(cpu.mem_read(GLOBALS, len(data)))
    assert after[0xCE9] == 1
    for offset, (before, result) in enumerate(zip(data, after)):
        assert before == result or offset == 0xCE9 or 0xFEF6 <= offset < 0xFF00
    return dict(executable_sha256=SHA256, names=names, directory_enter_count=len(enters),
                drive_select_count=len(drives), directory_change_count=len(directories),
                find_attributes=0x18)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    result = capture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print("captured 152 original writable-resource visits")


if __name__ == "__main__":
    main()
