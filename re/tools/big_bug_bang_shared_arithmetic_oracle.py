#!/usr/bin/env python3
"""Execute BBB's shared-state handler and guard helper, including F8/F9.

Results are observed from original instructions, not calculated by Python.
The dispatch table proves all seven opcode aliases enter this same handler.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER, CODE_BASE = 0x800, 0x5020
GLOBALS, SOURCE, STATE, STACK = 0x30000, 0x40000, 0x50000, 0x60000
ALIASES = [0xB1, 0xB4, 0xB5, 0xB6, 0xBE, 0xBF, 0xC0]


def run(executable, operator, mode, current, rhs, query, alias):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    state = bytearray(range(32))
    struct.pack_into("<HH", state, 2, current, rhs)
    cpu.mem_write(STATE + 0x180, bytes(state))
    operand = (2 if alias else 4) if mode in (0xC0, 0xC2) else rhs
    token = struct.pack("<B H B B H", 0xB1, 2, operator, mode, operand)
    cpu.mem_write(SOURCE + 0x40, token)
    globals_before = bytearray(0x10000)
    struct.pack_into("<HH", globals_before, 0x6AEC, 0x180, STATE // 16)
    struct.pack_into("<H", globals_before, 0x6C2C, 4)
    globals_before[0x6B83] = query
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(STACK + 0x6C06, struct.pack("<H", 0x200))
    cpu.mem_write(STACK + 0xFF00, struct.pack("<H", 0x1800))
    for reg, value in [(UC_X86_REG_CS, CODE_BASE // 16), (UC_X86_REG_DS, SOURCE // 16),
                       (UC_X86_REG_GS, GLOBALS // 16), (UC_X86_REG_SS, STACK // 16),
                       (UC_X86_REG_SP, 0xFF00), (UC_X86_REG_SI, 0x41), (UC_X86_REG_EFLAGS, 2)]:
        cpu.reg_write(reg, value)
    branch_failed = False

    def instruction(_cpu, address, size, _context):
        nonlocal branch_failed
        address += HEADER
        assert any(a <= address and address + size <= b
                   for a, b in [(0x744B, 0x750A), (0x697A, 0x6993)]), hex(address)
        branch_failed |= address == 0x697A

    def write(_cpu, _access, address, size, _value, _context):
        allowed = [(STACK + 0xFEFA, 6)]
        allowed += ([(GLOBALS + 0x6C2C, 2), (GLOBALS + 0x6B83, 1)] if query
                    else [(STATE + 0x182, 2)])
        assert any(a <= address and address + size <= a + n for a, n in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x744B - HEADER, CODE_BASE + 0x1800, count=100)
    assert cpu.reg_read(UC_X86_REG_IP) == 0x1800
    assert cpu.reg_read(UC_X86_REG_SP) == 0xFF02
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + 0x40, len(token))) == token
    return dict(operator=operator, mode=mode, query=query, alias=alias,
                token=list(token), state_before=list(state),
                state_after=list(cpu.mem_read(STATE + 0x180, len(state))),
                branch_failed=branch_failed, cursor=cpu.reg_read(UC_X86_REG_SI),
                query_after=cpu.mem_read(GLOBALS + 0x6B83, 1)[0],
                guard_depth=struct.unpack("<H", cpu.mem_read(GLOBALS + 0x6C2C, 2))[0] // 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    for opcode in ALIASES:
        assert struct.unpack_from("<H", executable, 0x16A78 + (opcode - 0xA0) * 2)[0] + 0x5820 == 0x744B
    cases = [run(executable, operator, mode, current, rhs, query, alias)
             for operator in [*range(0xF0, 0xFA), 0xFA, 0xFF]
             for mode in [0, 1, 0xC0, 0xC1, 0xC2, 0xFF]
             for current, rhs in [(0, 0), (65535, 0), (32768, 2), (65535, 65535), (7, 3), (0, 65535)]
             for query in [0, 1] for alias in [False, True]]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(case, sort_keys=True) + "\n" for case in cases))
    print(f"captured {len(cases)} original shared-state cases for seven dispatch aliases")


if __name__ == "__main__":
    main()
