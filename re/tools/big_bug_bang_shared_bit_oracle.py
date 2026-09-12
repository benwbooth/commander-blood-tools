#!/usr/bin/env python3
"""Execute Big Bug Bang's shared AE/B0 masked-bit handler.

The original handler and failure-helper instructions run unmodified under
Unicorn. The generated rows contain semantic state only, not executable bytes.
Run with ``python3 -P`` so the adjacent dis.py cannot shadow the standard
library.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
HANDLER = (0x750A, 0x754E)
FAILURE_HELPER = (0x697A, 0x6993)
GLOBALS = 0x30000
SOURCE = 0x40000
STATE = 0x50000
STACK_MEMORY = 0x60000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
TARGET = 2

OPCODES = (0xAE, 0xB0)
QUERY_VALUES = (0, 1, 2, 3, 0xFF)
FIELD_VALUES = (0, 1, 0xA55A, 0xFFFF)
MASKS = (0, 1, 3, 0x8000, 0xFFFF, 0x5AA5)


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def execute(executable, opcode, query, inverted, field, mask):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_before = bytearray(0x10000)
    globals_before[QUERY] = query
    struct.pack_into("<HH", globals_before, STATE_POINTER, 0, STATE // 16)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    cpu.mem_write(GLOBALS, bytes(globals_before))

    source_before = bytearray(0x10000)
    operand = (b"\xA1" if inverted else b"") + struct.pack("<HH", TARGET, mask)
    source_before[SCRIPT:SCRIPT + len(operand)] = operand
    cpu.mem_write(SOURCE, bytes(source_before))

    state_before = bytearray(0x10000)
    struct.pack_into("<H", state_before, TARGET, field)
    cpu.mem_write(STATE, bytes(state_before))
    cpu.mem_write(STACK_MEMORY + GUARD_STACK, struct.pack("<H", BRANCH_TARGET))
    cpu.mem_write(STACK_MEMORY + STACK, struct.pack("<H", RETURN))

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, SOURCE // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_SS, STACK_MEMORY // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    instruction_ranges = (HANDLER, FAILURE_HELPER)
    allowed_writes = (
        (STATE + TARGET, 2),
        (GLOBALS + QUERY, 1),
        (GLOBALS + GUARD_TOP, 2),
        (STACK_MEMORY + STACK - 16, 18),
    )

    def instruction(_cpu, address, size, _context):
        assert any(in_range(address, size, span) for span in instruction_ranges), hex(
            address + HEADER
        )

    def write_hook(_cpu, _access, address, size, _value, _context):
        assert any(
            first <= address and address + size <= first + length
            for first, length in allowed_writes
        ), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(HANDLER[0] - HEADER, CODE_BASE + RETURN, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + SCRIPT, len(operand))) == operand
    assert cpu.mem_read(STACK_MEMORY + GUARD_STACK, 2) == struct.pack(
        "<H", BRANCH_TARGET
    )

    globals_after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    state_after = bytes(cpu.mem_read(STATE, len(state_before)))
    has_bits = field & mask != 0
    failed = query & 1 != 0 and has_bits == inverted
    expected_field = (
        field
        if query & 1 != 0
        else field & (~mask & 0xFFFF)
        if inverted
        else field | mask
    )
    expected_query = 0 if failed else query
    expected_depth = 0 if failed else 1
    expected_cursor = BRANCH_TARGET if failed else SCRIPT + len(operand)
    assert word(state_after, TARGET) == expected_field
    assert globals_after[QUERY] == expected_query
    assert word(globals_after, GUARD_TOP) // 2 == expected_depth
    actual_cursor = cpu.reg_read(UC_X86_REG_SI)
    assert actual_cursor == expected_cursor, (
        opcode,
        query,
        inverted,
        field,
        mask,
        hex(actual_cursor),
        hex(expected_cursor),
    )
    return {
        "opcode": opcode,
        "query_before": query,
        "query_after": expected_query,
        "inverted": inverted,
        "field_before": field,
        "field_after": expected_field,
        "mask": mask,
        "failed": failed,
        "guard_depth_after": expected_depth,
        "cursor": expected_cursor,
    }


def vectors(executable):
    expected_handler = HANDLER[0] - (HEADER + CODE_BASE)
    for opcode in OPCODES:
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert encoded == expected_handler
    return [
        execute(executable, opcode, query, inverted, field, mask)
        for opcode, query, inverted, field, mask in itertools.product(
            OPCODES, QUERY_VALUES, (False, True), FIELD_VALUES, MASKS
        )
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    rows = vectors(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"captured {len(rows)} original AE/B0 shared-bit cases")


if __name__ == "__main__":
    main()
