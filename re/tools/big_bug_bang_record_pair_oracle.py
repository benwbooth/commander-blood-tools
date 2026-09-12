#!/usr/bin/env python3
"""Execute Big Bug Bang's shared B8/B9/BD adjacent-word handler.

The original pair handler, owner lookup, and guard-failure helper run
unmodified under Unicorn. Rows contain semantic inputs and outputs only. Run
with ``python3 -P`` so the adjacent dis.py cannot shadow the standard library.
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
HANDLER = (0x770C, 0x7752)
OWNER_HELPER = (0x6644, 0x665E)
FAILURE_HELPER = (0x697A, 0x6993)
GLOBALS = 0x30000
SOURCE = 0x40000
STATE = 0x50000
STACK_MEMORY = 0x60000
DIRECTORY = 0x70000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
ACTIVE_REFERENCE_POINTER = 0x6B22
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C

OWNER_OFFSET = 34
OWNER_END = 108
PAIR_OFFSET = OWNER_OFFSET + 24
ACTIVE_REFERENCE = 0x0200
ACTIVE_REFERENCE_FIELD = ACTIVE_REFERENCE + 0x16
DIRECTORY_CURSOR = 0x2000
OPCODES = (0xB8, 0xB9, 0xBD)
QUERY_VALUES = (0, 1, 2, 3, 0xFF)
REQUESTED_PAIRS = ((0, 0), (0x1111, 0x2222), (0xFFFF, 0xFFFF), (0x8000, 7))
REFERENCE_VALUES = {"none": 0, "owner": OWNER_OFFSET, "other": 0x9999}


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def stored_pairs(requested):
    first, second = requested
    return (
        (first, second),
        (first ^ 0xFFFF, second),
        (first, second ^ 0xFFFF),
        (first ^ 0xA5A5, second ^ 0x5A5A),
    )


def execute(executable, opcode, query, requested, stored, reference_name):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)

    globals_before = bytearray(0x10000)
    globals_before[QUERY] = query
    struct.pack_into("<HH", globals_before, STATE_POINTER, 0, STATE // 16)
    struct.pack_into(
        "<HH", globals_before, DIRECTORY_POINTER, DIRECTORY_CURSOR, DIRECTORY // 16
    )
    struct.pack_into("<H", globals_before, ACTIVE_REFERENCE_POINTER, ACTIVE_REFERENCE)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    cpu.mem_write(GLOBALS, bytes(globals_before))

    operand = struct.pack("<HHH", PAIR_OFFSET, *requested)
    cpu.mem_write(SOURCE + SCRIPT, operand)
    state_before = bytearray(0x10000)
    struct.pack_into("<HH", state_before, PAIR_OFFSET, *stored)
    reference_before = REFERENCE_VALUES[reference_name]
    struct.pack_into("<H", state_before, ACTIVE_REFERENCE_FIELD, reference_before)
    cpu.mem_write(STATE, bytes(state_before))

    directory_before = bytearray(0x10000)
    struct.pack_into("<H", directory_before, DIRECTORY_CURSOR - 4, OWNER_OFFSET)
    struct.pack_into("<H", directory_before, DIRECTORY_CURSOR + 0x10, OWNER_END)
    cpu.mem_write(DIRECTORY, bytes(directory_before))
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

    allowed_writes = (
        (STATE + PAIR_OFFSET, 4),
        (STATE + ACTIVE_REFERENCE_FIELD, 2),
        (GLOBALS + QUERY, 1),
        (GLOBALS + GUARD_TOP, 2),
        (STACK_MEMORY + STACK - 16, 18),
    )
    owner_lookup_called = False

    def instruction(_cpu, address, size, _context):
        nonlocal owner_lookup_called
        assert any(
            in_range(address, size, span)
            for span in (HANDLER, OWNER_HELPER, FAILURE_HELPER)
        ), hex(address + HEADER)
        owner_lookup_called |= address + HEADER == OWNER_HELPER[0]

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
    assert bytes(cpu.mem_read(DIRECTORY, len(directory_before))) == directory_before

    globals_after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    state_after = bytes(cpu.mem_read(STATE, len(state_before)))
    query_mode = query & 1 != 0
    failed = query_mode and stored != requested
    expected_pair = stored if query_mode else requested
    expected_reference = (
        0 if not query_mode and reference_name == "owner" else reference_before
    )
    expected_query = 0 if failed else query
    expected_depth = 0 if failed else 1
    expected_cursor = BRANCH_TARGET if failed else SCRIPT + len(operand)
    assert (word(state_after, PAIR_OFFSET), word(state_after, PAIR_OFFSET + 2)) == expected_pair
    assert word(state_after, ACTIVE_REFERENCE_FIELD) == expected_reference
    assert globals_after[QUERY] == expected_query
    assert word(globals_after, GUARD_TOP) // 2 == expected_depth
    assert cpu.reg_read(UC_X86_REG_SI) == expected_cursor
    assert owner_lookup_called == (not query_mode)
    reference_after = (
        "none" if expected_reference == 0 else "owner" if expected_reference == OWNER_OFFSET else "other"
    )
    return {
        "opcode": opcode,
        "query_before": query,
        "query_after": expected_query,
        "requested_pair": list(requested),
        "pair_before": list(stored),
        "pair_after": list(expected_pair),
        "reference_before": reference_name,
        "reference_after": reference_after,
        "owner_lookup_called": owner_lookup_called,
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
    rows = []
    for opcode, query, requested, reference_name in itertools.product(
        OPCODES, QUERY_VALUES, REQUESTED_PAIRS, REFERENCE_VALUES
    ):
        for stored in stored_pairs(requested):
            rows.append(
                execute(
                    executable, opcode, query, requested, stored, reference_name
                )
            )
    return rows


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
    print(f"captured {len(rows)} original B8/B9/BD record-pair cases")


if __name__ == "__main__":
    main()
