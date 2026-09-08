#!/usr/bin/env python3
"""Probe BBB's BAS-entry gate, stopping before dialogue execution.

Execute the original field resolver too. Synthetic actor/action records isolate
the gate; these vectors do not claim that a nonzero BAS entry is reachable.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_GS, UC_X86_REG_SS,
    UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_BP, UC_X86_REG_BX,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY, CALL, SKIP = 0x5E0D, 0x5E66, 0x5E69
GLOBALS, STATE, STACK = 0x30000, 0x20000, 0xFF00


def run(executable, flags, target):
    active, gate, choice, locked, blocked, primary, paired = flags
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable)
    globals_before = bytearray(0x10000)
    for offset, value in zip((0x6B82, 0x2200, 0x2A77, 0x6B8D), flags[:4]):
        globals_before[offset] = value
    struct.pack_into("<H", globals_before, 0x6B30, 0x400)
    struct.pack_into("<H", globals_before, 0x6B1E, 0x100)
    globals_before[0x7128:0x7128 + 21 * 16] = executable[0x16918:0x16918 + 21 * 16]
    assert globals_before[0x7128 + 2 * 16 + 1] == 26
    state = bytearray(0x10000)
    struct.pack_into("<HH", state, 0x200, 2, 0x8000 if blocked else 0)
    struct.pack_into("<H", state, 0x200 + 26, target)
    struct.pack_into("<HH", state, 0x23A, 0xC4, 0x100 if paired else 0x102)
    struct.pack_into("<HH", state, 0x400, 0xC4 if primary else 0, 0x200)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(STATE, bytes(state))
    for register, value in (
        (UC_X86_REG_CS, 0), (UC_X86_REG_DS, STATE // 16),
        (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK), (UC_X86_REG_SI, 0x200), (UC_X86_REG_BP, 0x23A),
    ):
        cpu.reg_write(register, value)
    reached = []

    def instruction(machine, address, size, _context):
        if address in (CALL, SKIP):
            reached.append(address)
            machine.emu_stop()
        else:
            assert ENTRY <= address < address + size <= SKIP or 0x6633 <= address < address + size <= 0x6644

    def write(_machine, _access, address, size, _value, _context):
        assert GLOBALS + STACK - 4 <= address < address + size <= GLOBALS + STACK

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(ENTRY, 0, count=100)
    assert len(reached) == 1
    assert bytes(cpu.mem_read(0, len(executable))) == executable
    assert bytes(cpu.mem_read(STATE, len(state))) == state
    after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    assert after[:STACK - 4] == globals_before[:STACK - 4]
    assert after[STACK:] == globals_before[STACK:]
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    if reached[0] == CALL:
        assert cpu.reg_read(UC_X86_REG_BX) == target
    return dict(active=active, gate=gate, choice=choice, locked=locked,
                blocked=blocked, primary=primary, paired=paired, target=target,
                enters_bas=reached[0] == CALL)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    rows = [run(executable, flags, target)
            for flags in itertools.product((0, 1), repeat=7)
            for target in (0, 1, 0x1234, 0xFFFF)]
    args.output.write_text("[\n" + ",\n".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")) for row in rows
    ) + "\n]\n")
    print(f"verified {len(rows)} original BAS-entry gate cases")


if __name__ == "__main__":
    main()
