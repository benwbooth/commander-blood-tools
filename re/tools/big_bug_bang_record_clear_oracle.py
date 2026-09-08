#!/usr/bin/env python3
"""Execute BBB's unchanged C9 handler and field resolver on synthetic records."""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_SI,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GLOBALS, STATE, STACK = 0x30000, 0x20000, 0xFF00


def run(executable, profile, pending, actor, enabled):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    globals_before = bytearray(0x10000)
    struct.pack_into("<HH", globals_before, 0x6AEC, 0, STATE // 16)
    struct.pack_into("<Hh", globals_before, 0x6B50, profile, pending)
    globals_before[0x6B7E] = enabled
    globals_before[0x277C] = 1
    globals_before[0x2783] = 9
    globals_before[0x7128:0x7278] = executable[0x16918:0x16A68]
    assert globals_before[0x7128 + 19 * 16 + 1] == 58
    state = bytearray(0x10000)
    struct.pack_into("<H", state, 0x200, 2)
    struct.pack_into("<HHH", state, 0x23A, 0xC4, 0x100, 0x1234)
    struct.pack_into("<HHH", state, 0x400, 0xC4 if actor else 0xC3, 0x200, 0x5678)
    struct.pack_into("<H", state, 0x500, 0x400)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(STATE, bytes(state))
    for register, value in (
        (UC_X86_REG_CS, 0), (UC_X86_REG_DS, STATE // 16),
        (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK), (UC_X86_REG_SI, 0x500),
    ):
        cpu.reg_write(register, value)
    reached = []

    def instruction(machine, address, size, _context):
        if address == 0x7C0D:
            reached.append(address)
            machine.emu_stop()
        else:
            assert (0x7BBF <= address < address + size <= 0x7C0D
                    or 0x6633 <= address < address + size <= 0x6644)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(0x7BBF, 0, count=100)
    assert reached == [0x7C0D]
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    after_state = bytes(cpu.mem_read(STATE, len(state)))
    expected_state = state[:]
    expected_state[0x400:0x406] = bytes(6)
    if actor:
        expected_state[0x23A:0x240] = bytes(6)
    assert after_state == expected_state
    allowed = {0x6B52, 0x6B53, 0x6B7E, 0x277C, 0x2783, *range(STACK - 8, STACK)}
    unexpected = [(hex(offset), a, b) for offset, (a, b) in
                  enumerate(zip(after, globals_before)) if a != b and offset not in allowed]
    assert not unexpected, unexpected
    return dict(profile=profile, pending=pending, actor=actor, enabled=enabled,
                pending_after=struct.unpack_from("<h", after, 0x6B52)[0],
                enabled_after=after[0x6B7E], sequence_after=after[0x277C],
                depth_after=after[0x2783])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = [run(executable, *case) for case in
             itertools.product(range(17), (-1, 0, 4), (False, True), (0, 1))]
    args.output.write_text("[\n" + ",\n".join(json.dumps(case, sort_keys=True)
                                            for case in cases) + "\n]\n")
    print(f"verified {len(cases)} original C9 teardown cases")


if __name__ == "__main__":
    main()
