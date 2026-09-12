#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited CE-D2 environment handler family.

The original activity guards enter their real failure helper, CF clears both
native resume globals, and D2 consumes every possible signed byte operand. Run
with ``python3 -P`` so the adjacent dis.py cannot shadow the standard library.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_GS,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
GLOBALS = 0x30000
SIZE = 0x10000
STACK = 0xFF00
SCRIPT = 0x9000

QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
RESUME_STATE = 0x6B87
RESUME_VALUE = 0x6B36
PROFILE_REQUEST = 0x6B52

FAILURE_HELPER = (0x697A, 0x6993)
GUARDS = {
    0xCE: (0x69AC, 0x69B7, 0x2A33),
    0xD0: (0x69B8, 0x69C3, 0x277C),
    0xD1: (0x69C4, 0x69CF, 0x29DD),
}
CLEAR = (0x69D8, 0x69E5)
REQUEST = (0x69D0, 0x69D7)
TARGETS = (0x5AA5, 0x6BB6)


def in_file_range(address, size, span):
    start, end = span
    return start - HEADER <= address and address + size <= end - HEADER


def machine(executable, noise):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    globals_before = bytearray([noise] * SIZE)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_ES, 0x4000),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT + 1),
        (UC_X86_REG_EFLAGS, 0x0AD7),
    ):
        cpu.reg_write(register, value)
    return cpu, module, globals_before


def execute(cpu, module, globals_before, handler, allowed_writes, extra_ranges=()):
    start, terminal = handler
    reached_terminal = False
    entered_failure = 0
    ranges = ((start, terminal + 1),) + tuple(extra_ranges)

    def instruction(machine, address, size, _context):
        nonlocal reached_terminal, entered_failure
        file_address = address + HEADER
        assert any(in_file_range(address, size, span) for span in ranges), hex(file_address)
        if file_address == FAILURE_HELPER[0]:
            entered_failure += 1
        if file_address == terminal:
            reached_terminal = True
            machine.emu_stop()

    def write_hook(_machine, _access, address, size, _value, _context):
        assert GLOBALS <= address and address + size <= GLOBALS + SIZE, hex(address)
        offset = address - GLOBALS
        assert any(
            start <= offset and offset + size <= start + length
            for start, length in allowed_writes
        ), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(start - HEADER, 0x100000, count=100)
    assert reached_terminal
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert bytes(cpu.mem_read(0, len(module))) == module

    globals_after = bytearray(cpu.mem_read(GLOBALS, SIZE))
    restored = bytearray(globals_after)
    for offset, length in allowed_writes:
        restored[offset : offset + length] = globals_before[offset : offset + length]
    assert restored == globals_before
    return globals_after, entered_failure


def guard_case(executable, opcode, flag_value, query_before, depth, noise):
    cpu, module, before = machine(executable, noise)
    start, terminal, flag_offset = GUARDS[opcode]
    before[SCRIPT] = opcode
    before[flag_offset] = flag_value
    before[QUERY] = query_before
    struct.pack_into("<H", before, GUARD_TOP, depth * 2)
    struct.pack_into("<2H", before, GUARD_STACK, *TARGETS)
    cpu.mem_write(GLOBALS, bytes(before))
    allowed = ((QUERY, 1), (GUARD_TOP, 2), (STACK - 8, 8))
    after, failure_calls = execute(
        cpu,
        module,
        before,
        (start, terminal),
        allowed,
        (FAILURE_HELPER,),
    )
    branch_taken = flag_value & 1 == 0
    cursor_after = cpu.reg_read(UC_X86_REG_SI)
    expected_target = TARGETS[depth - 1]
    assert failure_calls == int(branch_taken)
    assert cursor_after == (expected_target if branch_taken else SCRIPT + 1)
    assert after[QUERY] == (0 if branch_taken else query_before)
    assert struct.unpack_from("<H", after, GUARD_TOP)[0] == (depth - int(branch_taken)) * 2
    return {
        "kind": "guard",
        "opcode": opcode,
        "flag_offset": flag_offset,
        "flag_value": flag_value,
        "query_before": query_before,
        "query_after": after[QUERY],
        "guard_depth_before": depth,
        "guard_depth_after": struct.unpack_from("<H", after, GUARD_TOP)[0] // 2,
        "failure_target": expected_target,
        "branch_taken": branch_taken,
        "failure_calls": failure_calls,
        "cursor_before": SCRIPT + 1,
        "cursor_after": cursor_after,
    }


def clear_case(executable, resume_state, resume_value, noise):
    cpu, module, before = machine(executable, noise)
    before[SCRIPT] = 0xCF
    before[RESUME_STATE] = resume_state
    struct.pack_into("<H", before, RESUME_VALUE, resume_value)
    cpu.mem_write(GLOBALS, bytes(before))
    after, failure_calls = execute(
        cpu,
        module,
        before,
        CLEAR,
        ((RESUME_STATE, 1), (RESUME_VALUE, 2)),
    )
    assert failure_calls == 0
    assert after[RESUME_STATE] == 0
    assert struct.unpack_from("<H", after, RESUME_VALUE)[0] == 0
    assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT + 1
    return {
        "kind": "clear",
        "opcode": 0xCF,
        "resume_state_before": resume_state,
        "resume_value_before": resume_value,
        "resume_state_after": after[RESUME_STATE],
        "resume_value_after": struct.unpack_from("<H", after, RESUME_VALUE)[0],
        "cursor_before": SCRIPT + 1,
        "cursor_after": cpu.reg_read(UC_X86_REG_SI),
    }


def request_case(executable, operand, noise):
    cpu, module, before = machine(executable, noise)
    before[SCRIPT : SCRIPT + 2] = bytes([0xD2, operand])
    struct.pack_into("<H", before, PROFILE_REQUEST, 0xA55A)
    cpu.mem_write(GLOBALS, bytes(before))
    after, failure_calls = execute(
        cpu,
        module,
        before,
        REQUEST,
        ((PROFILE_REQUEST, 2),),
    )
    assert failure_calls == 0
    stored = struct.unpack_from("<H", after, PROFILE_REQUEST)[0]
    expected = ((operand if operand < 0x80 else operand - 0x100) - 1) & 0xFFFF
    assert stored == expected
    assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT + 2
    return {
        "kind": "profile_request",
        "opcode": 0xD2,
        "operand": operand,
        "request_before": 0xA55A,
        "request_after": stored,
        "cursor_before": SCRIPT + 1,
        "cursor_after": cpu.reg_read(UC_X86_REG_SI),
    }


def vectors(executable):
    handlers = {**{opcode: values[0] for opcode, values in GUARDS.items()}, 0xCF: CLEAR[0], 0xD2: REQUEST[0]}
    for opcode, handler in handlers.items():
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert encoded == handler - (HEADER + CODE_BASE), (hex(opcode), hex(encoded))

    rows = []
    for opcode, flag, query, depth in itertools.product(
        GUARDS, (0, 1, 2, 0x80, 0xFF), (1, 3, 0xFF), (1, 2)
    ):
        rows.append(guard_case(executable, opcode, flag, query, depth, query ^ flag))
    for resume_state, resume_value in itertools.product(
        (0, 1, 0x80, 0xFF), (0, 1, 0x1234, 0xFFFF)
    ):
        rows.append(
            clear_case(executable, resume_state, resume_value, (resume_state ^ resume_value) & 0xFF)
        )
    for operand in range(256):
        rows.append(request_case(executable, operand, operand ^ 0xA5))
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
        "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows)
    )
    print(f"captured {len(rows)} original CE-D2 environment cases")


if __name__ == "__main__":
    main()
