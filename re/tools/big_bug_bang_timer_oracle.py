#!/usr/bin/env python3
"""Execute BBB A5 and its guard helper, without replacing original instructions.

The corpus covers the port's 128 owned timer slots. Negative signed indices
address unrelated native state and are not claimed as supported timer slots.
GS globals, SS state, and DS script are deliberately distinct allocations.
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
CODE_BASE = 0x5020
GLOBALS, SOURCE, OWNED = 0x30000, 0x40000, 0x50000
TIMER = 0x6E86
STACK = 0xFF00
RETURN = 0x1800
FAILURE = 0x180


def run(executable, slot, flags, initial):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_before = bytearray(0x10000)
    globals_before[0x6B83] = flags
    struct.pack_into("<H", globals_before, 0x6C2C, 4)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    owned_before = bytearray([0xA5] * 0x10000)
    for index in range(128):
        struct.pack_into("<H", owned_before, TIMER + index * 2, index * 257)
    struct.pack_into("<H", owned_before, TIMER + slot * 2, initial)
    struct.pack_into("<H", owned_before, 0x6C06, FAILURE)
    struct.pack_into("<H", owned_before, STACK, RETURN)
    cpu.mem_write(OWNED, bytes(owned_before))
    operand = initial ^ 0x5AA5
    source = bytes([slot]) + struct.pack("<H", operand)
    cpu.mem_write(SOURCE + 0x40, source)
    for register, value in [(UC_X86_REG_CS, CODE_BASE // 16),
                            (UC_X86_REG_GS, GLOBALS // 16),
                            (UC_X86_REG_DS, SOURCE // 16),
                            (UC_X86_REG_SS, OWNED // 16),
                            (UC_X86_REG_SP, STACK), (UC_X86_REG_SI, 0x40),
                            (UC_X86_REG_EFLAGS, 2)]:
        cpu.reg_write(register, value)
    branch_taken = False
    allowed = [(OWNED + STACK - 4, 4)]
    if flags & 1:
        allowed += [(GLOBALS + 0x6C2C, 2), (GLOBALS + 0x6B83, 1)]
    else:
        allowed += [(OWNED + TIMER + slot * 2, 2)]

    def instruction(_cpu, address, size, _context):
        nonlocal branch_taken
        address += HEADER
        assert any(a <= address and address + size <= b
                   for a, b in [(0x6B07, 0x6B28), (0x697A, 0x6993)]), hex(address)
        branch_taken |= address == 0x697A

    def write(_cpu, _access, address, size, _value, _context):
        assert any(a <= address and address + size <= a + n for a, n in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write)
    cpu.emu_start(0x6B07 - HEADER, CODE_BASE + RETURN, count=100)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + 0x40, len(source))) == source
    final_owned = bytes(cpu.mem_read(OWNED, len(owned_before)))
    final_globals = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    for base, before, after in [(OWNED, owned_before, final_owned),
                                (GLOBALS, globals_before, final_globals)]:
        assert all(a == b or any(start <= base + i < start + size for start, size in allowed)
                   for i, (a, b) in enumerate(zip(before, after)))
    return dict(slot=slot, flags=flags, initial=initial, operand=operand,
                final_state=struct.unpack_from("<H", final_owned, TIMER + slot * 2)[0],
                branch_taken=branch_taken, cursor=cpu.reg_read(UC_X86_REG_SI),
                flags_after=final_globals[0x6B83],
                guard_depth=struct.unpack_from("<H", final_globals, 0x6C2C)[0] // 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    assert hashlib.sha256(executable).hexdigest() == SHA256
    cases = [run(executable, slot, flags, initial)
             for slot in range(128) for flags in [0, 1, 2, 3, 254, 255]
             for initial in [0, 1, 32768, 65535]]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(case, sort_keys=True) + "\n" for case in cases))
    print(f"captured {len(cases)} original A5 timer and guard-helper cases")


if __name__ == "__main__":
    main()
