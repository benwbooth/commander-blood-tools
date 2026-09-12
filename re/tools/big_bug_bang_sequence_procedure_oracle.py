#!/usr/bin/env python3
"""Execute BBB's inherited A7, A9, AA, AB, and AC VM handlers.

The original instructions run unmodified. The output contains semantic state
only, not executable bytes. Run with ``python3 -P`` so the adjacent dis.py
cannot shadow Python's standard library.
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
GLOBALS = 0x30000
SOURCE = 0x40000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800

PRESENTATION_ACTIVE = 0x6B82
OFFERED_TOPIC = 0x6B42
QUERY = 0x6B83
GUARD_ROOT = 0x6C04
GUARD_TOP = 0x6C2C
YIELD = 0x6B8A

HANDLERS = {
    "topic_offer": (0xA7, 0x6E6E, 0x6E7C),
    "procedure_gate": (0xA9, 0x6EE4, 0x6F00),
    "procedure_activation": (0xAB, 0x6F00, 0x6F09),
    "yield": (0xAA, 0x6F09, 0x6F10),
    "selector_yield": (0xAC, 0x6F10, 0x6F17),
}


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def assert_dispatch_table(executable):
    for opcode, start, _end in HANDLERS.values():
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert HEADER + CODE_BASE + encoded == start


def execute(executable, operation, operand, globals_before, source_writes=()):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    source_before = bytearray(0x10000)
    source_before[SCRIPT:SCRIPT + len(operand)] = operand
    for offset, value in source_writes:
        source_before[offset] = value
    struct.pack_into("<H", globals_before, STACK, RETURN)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(SOURCE, bytes(source_before))
    for register, value in [
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, SOURCE // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT),
        (UC_X86_REG_EFLAGS, 2),
    ]:
        cpu.reg_write(register, value)

    opcode, start, end = HANDLERS[operation]
    allowed_global = {
        "topic_offer": [(OFFERED_TOPIC, 2)],
        "procedure_gate": [(QUERY, 1), (GUARD_ROOT, 2), (GUARD_TOP, 2)],
        "procedure_activation": [],
        "yield": [(YIELD, 1)],
        "selector_yield": [(YIELD, 1)],
    }[operation]
    allowed = [(GLOBALS + offset, size) for offset, size in allowed_global]
    allowed.extend((SOURCE + offset, 1) for offset, _value in source_writes)
    allowed.append((GLOBALS + STACK - 2, 4))

    def instruction(_cpu, address, size, _context):
        file_address = address + HEADER
        assert start <= file_address and file_address + size <= end, hex(file_address)

    def write_hook(_cpu, _access, address, size, _value, _context):
        assert any(first <= address and address + size <= first + length
                   for first, length in allowed), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(start - HEADER, CODE_BASE + RETURN, count=100)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module

    globals_after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    source_after = bytes(cpu.mem_read(SOURCE, len(source_before)))
    ranges = allowed
    for base, before, after in (
        (GLOBALS, globals_before, globals_after),
        (SOURCE, source_before, source_after),
    ):
        for index, (old, new) in enumerate(zip(before, after)):
            if old != new:
                address = base + index
                assert any(first <= address < first + length for first, length in ranges), hex(address)
    return cpu, globals_after, source_after


def topic_offer_vectors(executable):
    rows = []
    for active, operand, offered_before in itertools.product(
        (0, 1, 2, 0xFF), (0, 32, 0xFFFF), (0, 0x1234)
    ):
        before = bytearray(0x10000)
        before[PRESENTATION_ACTIVE] = active
        struct.pack_into("<H", before, OFFERED_TOPIC, offered_before)
        cpu, after, _source = execute(
            executable, "topic_offer", struct.pack("<H", operand), before
        )
        stored = active & 1 != 0
        expected = operand if stored else offered_before
        assert word(after, OFFERED_TOPIC) == expected
        assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT + 2
        rows.append({
            "operation": "topic_offer",
            "active": active,
            "operand": operand,
            "offered_before": offered_before,
            "stored": stored,
            "offered_after": word(after, OFFERED_TOPIC),
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def procedure_gate_vectors(executable):
    rows = []
    for flags, target, query_before in itertools.product(
        (0, 1, 2, 3, 0x80, 0x81, 0xFE, 0xFF),
        (0, 0x1234, 0xFFFF),
        (0, 0xFF),
    ):
        before = bytearray(0x10000)
        before[QUERY] = query_before
        struct.pack_into("<H", before, GUARD_ROOT, 0x5AA5)
        struct.pack_into("<H", before, GUARD_TOP, 2)
        operand = bytes([flags]) + struct.pack("<H", target)
        cpu, after, _source = execute(executable, "procedure_gate", operand, before)
        enabled = flags & 1 != 0
        expected_query = 1 if enabled else query_before
        expected_root = target if enabled else 0x5AA5
        expected_cursor = SCRIPT + 3 if enabled else target
        assert after[QUERY] == expected_query
        assert word(after, GUARD_ROOT) == expected_root
        assert word(after, GUARD_TOP) == 2
        assert cpu.reg_read(UC_X86_REG_SI) == expected_cursor
        rows.append({
            "operation": "procedure_gate",
            "flags": flags,
            "target": target,
            "query_before": query_before,
            "enabled": enabled,
            "query_after": after[QUERY],
            "root_after": word(after, GUARD_ROOT),
            "depth_after": word(after, GUARD_TOP) // 2,
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def procedure_activation_vectors(executable):
    rows = []
    for value, target_before in itertools.product(
        (0, 1, 2, 3, 0x80, 0x81, 0xFE, 0xFF), (0, 0xA5)
    ):
        before = bytearray(0x10000)
        operand = bytes([value]) + struct.pack("<H", 1)
        cpu, _after, source = execute(
            executable,
            "procedure_activation",
            operand,
            before,
            source_writes=((1, target_before),),
        )
        assert source[1] == value
        assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT + 3
        rows.append({
            "operation": "procedure_activation",
            "value": value,
            "target_before": target_before,
            "target_after": source[1],
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def yield_vectors(executable):
    rows = []
    for operation, value in itertools.product(("yield", "selector_yield"), (0, 1, 2, 0xFF)):
        before = bytearray(0x10000)
        before[YIELD] = value
        cpu, after, _source = execute(executable, operation, b"", before)
        assert after[YIELD] == 1
        assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT
        rows.append({
            "operation": operation,
            "yield_before": value,
            "yield_after": after[YIELD],
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    assert_dispatch_table(executable)

    rows = []
    for generate in (
        topic_offer_vectors,
        procedure_gate_vectors,
        procedure_activation_vectors,
        yield_vectors,
    ):
        rows.extend(generate(executable))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows)
    )
    print(f"captured {len(rows)} original A7/A9-AA/AB-AC cases")


if __name__ == "__main__":
    main()
