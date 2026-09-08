#!/usr/bin/env python3
"""Probe the unchanged BBB F7 handler; no patched instructions or runtime dependency."""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_DI,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS, STATE, STACK = 0x30000, 0x20000, 0xFF00


def run(executable, profile, pending, related, auxiliary):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    before = bytearray(0x10000)
    struct.pack_into("<H", before, 0x6AEE, STATE // 16)
    struct.pack_into("<H", before, 0x6B30, 0x400)
    struct.pack_into("<HHH", before, 0x6B3A, 0xC3, 0x300, auxiliary)
    struct.pack_into("<Hh", before, 0x6B50, profile, pending)
    struct.pack_into("<H", before, 0x6BDC, 345)
    for offset, value in ((0x277C, 1), (0x2781, 7), (0x2783, 9), (0x6B90, 2)):
        before[offset] = value
    state = bytearray(0x10000)
    struct.pack_into("<HHH", state, 0x400, 0xC4, related, 0x1234)
    cpu.mem_write(GLOBALS, bytes(before))
    cpu.mem_write(STATE, bytes(state))
    for register, value in (
        (UC_X86_REG_CS, 0), (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_SS, GLOBALS // 16), (UC_X86_REG_ES, 0x1234),
        (UC_X86_REG_SP, STACK), (UC_X86_REG_DI, 0x5678),
    ):
        cpu.reg_write(register, value)
    reached = []

    def instruction(machine, address, size, _context):
        if address == 0x2513:
            reached.append(address)
            machine.emu_stop()
        else:
            assert 0x24C8 <= address < address + size <= 0x2513

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x24C8, 0, count=100)
    assert reached == [0x2513]
    for register, value in ((UC_X86_REG_SP, STACK), (UC_X86_REG_ES, 0x1234), (UC_X86_REG_DI, 0x5678)):
        assert cpu.reg_read(register) == value
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    assert bytes(cpu.mem_read(STATE, len(state))) == state
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    expected = before[:]
    if related:
        struct.pack_into("<HH", expected, 0x6B3A, 0xC9, related)
    if profile > 1:
        struct.pack_into("<h", expected, 0x6B52, 1)
    struct.pack_into("<H", expected, 0x6BDC, 0)
    for offset, value in ((0x277C, 0), (0x2781, 1), (0x2783, 6), (0x6B90, 0), (0x6B7E, 1)):
        expected[offset] = value
    expected[STACK - 4:STACK] = after[STACK - 4:STACK]
    assert after == expected
    return dict(profile=profile, pending=pending, related=related, auxiliary=auxiliary,
                pending_after=struct.unpack_from("<h", after, 0x6B52)[0],
                deferred_after=list(struct.unpack_from("<HHH", after, 0x6B3A)),
                menu_count_after=struct.unpack_from("<H", after, 0x6BDC)[0],
                sequence_after=after[0x277C], opening_after=after[0x2781],
                depth_after=after[0x2783], phase_after=after[0x6B90], enabled_after=after[0x6B7E])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    table = executable[0x2281:0x2381]
    assert (table[0xC1], table[0x1B], table[0x20]) == (14, 4, 7)
    assert struct.unpack_from("<H", executable, 0x2382 + 14 * 2)[0] + 0xF70 == 0x24C8
    cases = [run(executable, *case) for case in
             itertools.product(range(17), (-1, 0, 16), (0, 0x200), (0, 0xFFFF))]
    args.output.write_text("[\n" + ",\n".join(json.dumps(case, sort_keys=True) for case in cases) + "\n]\n")
    print(f"verified {len(cases)} original F7 cases and Escape/Space/F7 key bindings")


if __name__ == "__main__":
    main()
