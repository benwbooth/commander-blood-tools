#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited CA/CB host-clock guards.

The original handlers and guard-failure helper run unmodified under Unicorn.
Rows contain semantic inputs and outputs only. Run with ``python3 -P`` so the
adjacent dis.py cannot shadow the standard library.
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
HOUR_HANDLER = (0x6A01, 0x6A2C)
DATE_HANDLER = (0x6A2C, 0x6A75)
FAILURE_HELPER = (0x697A, 0x6993)
GLOBALS = 0x30000
SOURCE = 0x40000
STACK_MEMORY = 0x50000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800
BRANCH_TARGET = 0x5AA5

CURRENT_HOUR = 0x0C9E
CURRENT_DAY = 0x0CA0
CURRENT_MONTH = 0x0CA2
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C

HOUR_TAG_WORDS = (0x00F1, 0xA5F1, 0x00F2, 0x5AF2, 0x0000, 0xFFF0)
HOURS = (-32768, -129, -1, 0, 1, 128, 32767)
DATE_TAGS = (0xF1, 0xF2, 0xF0)
DATE_PAIRS = (
    (-128, 127),
    (-1, 0),
    (0, -1),
    (0, 0),
    (0, 1),
    (1, 0),
    (127, -128),
)
ENCODED_YEARS = (0, 0xFFFF)
QUERY_VALUES = (1, 3, 0xFF)


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def execute(executable, handler, operand, query, configure_clock):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    cpu.mem_write(SOURCE + SCRIPT, operand)

    globals_before = bytearray(0x10000)
    globals_before[QUERY] = query
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    configure_clock(globals_before)
    cpu.mem_write(GLOBALS, bytes(globals_before))
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
        (GLOBALS + QUERY, 1),
        (GLOBALS + GUARD_TOP, 2),
        (STACK_MEMORY + STACK - 16, 18),
    )

    def instruction(_cpu, address, size, _context):
        assert any(
            in_range(address, size, span) for span in (handler, FAILURE_HELPER)
        ), hex(address + HEADER)

    def write_hook(_cpu, _access, address, size, _value, _context):
        assert any(
            first <= address and address + size <= first + length
            for first, length in allowed_writes
        ), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(handler[0] - HEADER, CODE_BASE + RETURN, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + SCRIPT, len(operand))) == operand
    assert cpu.mem_read(STACK_MEMORY + GUARD_STACK, 2) == struct.pack(
        "<H", BRANCH_TARGET
    )

    globals_after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    for index, (before, after) in enumerate(zip(globals_before, globals_after)):
        if before != after:
            assert index == QUERY or GUARD_TOP <= index < GUARD_TOP + 2, hex(index)
    return {
        "query_after": globals_after[QUERY],
        "guard_depth_after": word(globals_after, GUARD_TOP) // 2,
        "cursor": cpu.reg_read(UC_X86_REG_SI),
    }


def relation_matches(tag, authored, current):
    if tag == 0xF1:
        return authored > current
    if tag == 0xF2:
        return authored < current
    return authored == current


def hour_vectors(executable):
    rows = []
    for tag_word, authored, current, query in itertools.product(
        HOUR_TAG_WORDS, HOURS, HOURS, QUERY_VALUES
    ):
        operand = struct.pack("<Hh", tag_word, authored)

        def configure_clock(before):
            struct.pack_into("<h", before, CURRENT_HOUR, current)

        observed = execute(
            executable, HOUR_HANDLER, operand, query, configure_clock
        )
        passed = relation_matches(tag_word & 0xFF, authored, current)
        expected_cursor = SCRIPT + len(operand) if passed else BRANCH_TARGET
        assert observed == {
            "query_after": query if passed else 0,
            "guard_depth_after": 1 if passed else 0,
            "cursor": expected_cursor,
        }
        rows.append(
            {
                "operation": "hour",
                "tag_word": tag_word,
                "authored": authored,
                "current": current,
                "query_before": query,
                "failed": not passed,
                **observed,
            }
        )
    return rows


def date_vectors(executable):
    rows = []
    for tag, authored, current, encoded_year, query in itertools.product(
        DATE_TAGS, DATE_PAIRS, DATE_PAIRS, ENCODED_YEARS, QUERY_VALUES
    ):
        authored_month, authored_day = authored
        current_month, current_day = current
        operand = bytes((tag, authored_day & 0xFF, authored_month & 0xFF))
        operand += struct.pack("<H", encoded_year)

        def configure_clock(before):
            before[CURRENT_DAY] = current_day & 0xFF
            before[CURRENT_DAY + 1] = 0xA5
            before[CURRENT_MONTH] = current_month & 0xFF
            before[CURRENT_MONTH + 1] = 0x5A

        observed = execute(
            executable, DATE_HANDLER, operand, query, configure_clock
        )
        passed = relation_matches(tag, authored, current)
        expected_cursor = SCRIPT + len(operand) if passed else BRANCH_TARGET
        assert observed == {
            "query_after": query if passed else 0,
            "guard_depth_after": 1 if passed else 0,
            "cursor": expected_cursor,
        }
        rows.append(
            {
                "operation": "date",
                "tag": tag,
                "authored_day": authored_day,
                "authored_month": authored_month,
                "encoded_year": encoded_year,
                "current_day": current_day,
                "current_month": current_month,
                "query_before": query,
                "failed": not passed,
                **observed,
            }
        )
    return rows


def vectors(executable):
    for opcode, handler in ((0xCA, HOUR_HANDLER), (0xCB, DATE_HANDLER)):
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert encoded == handler[0] - (HEADER + CODE_BASE)
    return hour_vectors(executable) + date_vectors(executable)


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
    print(f"captured {len(rows)} original CA/CB host-clock cases")


if __name__ == "__main__":
    main()
