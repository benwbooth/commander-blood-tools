#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited A0-A4 control-flow handlers.

The original handler and PRNG instructions run unmodified under Unicorn. The
fixture records semantic inputs and outputs only; it does not export machine
code. Run with ``python3 -P`` so the adjacent dis.py cannot shadow the standard
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
    UC_X86_REG_AX,
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
GLOBALS = 0x30000
SOURCE = 0x40000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800
QUERY = 0x6B83
SELECTED = 0x6B34
ALTERNATE = 0x6B36
RESUME = 0x6B87
SCAN = 0x6B88
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
PRNG_SEED = 0x0CDA
PRNG_MIX_LOW = 0x0CDC
PRNG_MIX_HIGH = 0x0CDD
PRNG_COUNTER = 0x0CDE
DISPATCH_TABLE = 0x16A78

HANDLERS = {
    "begin_guard": (0x6A75, 0x6A8E),
    "end_guard": (0x6A8E, 0x6AA4),
    "random_guard": (0x6AA4, 0x6AB2),
    "concept_guard": (0x6AB2, 0x6AF7),
    "jump": (0x6AF7, 0x6B07),
}
FAILURE_HELPER = (0x697A, 0x6993)
PRNG = (0x3163, 0x31B4)


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def assert_dispatch_table(executable):
    expected = tuple(
        HANDLERS[operation][0] - (HEADER + CODE_BASE)
        for operation in ("begin_guard", "end_guard", "random_guard", "concept_guard", "jump")
    )
    assert struct.unpack_from("<5H", executable, DISPATCH_TABLE) == expected


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def execute(executable, operation, source, before, allowed_writes, observe=None):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)
    cpu.mem_write(SOURCE + SCRIPT, source)
    struct.pack_into("<H", before, STACK, RETURN)
    cpu.mem_write(GLOBALS, bytes(before))
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

    instruction_ranges = [HANDLERS[operation]]
    if operation in ("random_guard", "concept_guard"):
        instruction_ranges.append(FAILURE_HELPER)
    if operation == "random_guard":
        instruction_ranges.append(PRNG)

    def instruction(machine, address, size, _context):
        assert any(in_range(address, size, span) for span in instruction_ranges), hex(address + HEADER)
        if observe is not None:
            observe(machine, address + HEADER)

    stack_writes = [(GLOBALS + STACK - 16, 18)]
    writes = [(GLOBALS + offset, size) for offset, size in allowed_writes] + stack_writes

    def write_hook(_machine, _access, address, size, _value, _context):
        assert any(start <= address and address + size <= start + length
                   for start, length in writes), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    start = HANDLERS[operation][0] - HEADER
    cpu.emu_start(start, CODE_BASE + RETURN, count=200000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + SCRIPT, len(source))) == source
    after = bytes(cpu.mem_read(GLOBALS, len(before)))
    for index, (old, new) in enumerate(zip(before, after)):
        if old != new:
            address = GLOBALS + index
            assert any(start <= address < start + length for start, length in writes), hex(address)
    return cpu, after


def guard_state(depth, query=1):
    before = bytearray(0x10000)
    before[QUERY] = query
    struct.pack_into("<H", before, GUARD_TOP, depth * 2)
    for index in range(depth):
        struct.pack_into("<H", before, GUARD_STACK + index * 2, 0x1000 + index * 0x111)
    return before


def begin_guard_vectors(executable):
    rows = []
    for depth, target, query_before in itertools.product(
        (1, 2, 4), (0, 0x1234, 0xFFFF), (0, 0xFF)
    ):
        before = guard_state(depth, query_before)
        source = struct.pack("<H", target)
        _cpu, after = execute(
            executable,
            "begin_guard",
            source,
            before,
            [(QUERY, 1), (GUARD_TOP, 2), (GUARD_STACK + depth * 2, 2)],
        )
        assert word(after, GUARD_STACK + depth * 2) == target
        assert word(after, GUARD_TOP) == (depth + 1) * 2
        assert after[QUERY] == 1
        assert _cpu.reg_read(UC_X86_REG_SI) == SCRIPT + len(source)
        rows.append({
            "operation": "begin_guard",
            "depth_before": depth,
            "query_before": query_before,
            "target": target,
            "depth_after": word(after, GUARD_TOP) // 2,
            "query_after": after[QUERY],
            "cursor": _cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def end_guard_vectors(executable):
    rows = []
    for depth, query_before in itertools.product((1, 2, 4), (0, 0xFF)):
        before = guard_state(depth, query_before)
        cpu, after = execute(
            executable, "end_guard", b"", before, [(QUERY, 1), (GUARD_TOP, 2)]
        )
        assert word(after, GUARD_TOP) == max(depth - 1, 1) * 2
        assert after[QUERY] == 0
        assert cpu.reg_read(UC_X86_REG_SI) == SCRIPT
        rows.append({
            "operation": "end_guard",
            "depth_before": depth,
            "query_before": query_before,
            "depth_after": word(after, GUARD_TOP) // 2,
            "query_after": after[QUERY],
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def random_guard_vectors(executable):
    rows = []
    states = [
        (0, 0, 0, 0),
        (0x1234, 0x56, 0x78, 0x9A),
        (0xFFFF, 0xFF, 0xFF, 0xFF),
        (0xA55A, 0x81, 0x7E, 0xFE),
    ]
    for modulus, state in itertools.product((0, 1, 2, 3, 7, 256, 32768, 65535), states):
        seed, mix_low, mix_high, counter = state
        before = guard_state(2)
        struct.pack_into("<H", before, GUARD_STACK + 2, 0x2468)
        struct.pack_into("<H", before, PRNG_SEED, seed)
        before[PRNG_MIX_LOW] = mix_low
        before[PRNG_MIX_HIGH] = mix_high
        before[PRNG_COUNTER] = counter
        result = []

        def observe(cpu, file_address):
            if file_address == 0x6AAA:
                result.append(cpu.reg_read(UC_X86_REG_AX))

        cpu, after = execute(
            executable,
            "random_guard",
            struct.pack("<H", modulus),
            before,
            [(QUERY, 1), (GUARD_TOP, 2), (PRNG_MIX_LOW, 3)],
            observe,
        )
        assert len(result) == 1
        branch_taken = result[0] != 0
        assert word(after, GUARD_TOP) == (1 if branch_taken else 2) * 2
        assert after[QUERY] == (0 if branch_taken else 1)
        assert cpu.reg_read(UC_X86_REG_SI) == (0x2468 if branch_taken else SCRIPT + 2)
        rows.append({
            "operation": "random_guard",
            "modulus": modulus,
            "seed": seed,
            "mix_low_before": mix_low,
            "mix_high_before": mix_high,
            "counter_before": counter,
            "result": result[0],
            "mix_low_after": after[PRNG_MIX_LOW],
            "mix_high_after": after[PRNG_MIX_HIGH],
            "counter_after": after[PRNG_COUNTER],
            "branch_taken": branch_taken,
            "depth_after": word(after, GUARD_TOP) // 2,
            "query_after": after[QUERY],
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def concept_guard_vectors(executable):
    rows = []
    for resume, primary, alternate, expected, inverted in itertools.product(
        (0, 2), (0, 32, 64), (0, 32, 64), (32, 64), (False, True)
    ):
        before = guard_state(2)
        before[SCAN] = 0
        before[RESUME] = resume
        struct.pack_into("<HH", before, SELECTED, primary, alternate)
        struct.pack_into("<H", before, GUARD_STACK + 2, 0x2468)
        source = (b"\xA1" if inverted else b"") + struct.pack("<H", expected)
        cpu, after = execute(
            executable,
            "concept_guard",
            source,
            before,
            [(QUERY, 1), (GUARD_TOP, 2)],
        )
        branch_taken = cpu.reg_read(UC_X86_REG_SI) != SCRIPT + len(source)
        active = alternate if resume & 2 else primary
        continues = active != 0 and ((active == expected) != inverted)
        assert branch_taken != continues
        assert word(after, GUARD_TOP) == (1 if branch_taken else 2) * 2
        assert after[QUERY] == (0 if branch_taken else 1)
        assert cpu.reg_read(UC_X86_REG_SI) == (0x2468 if branch_taken else SCRIPT + len(source))
        rows.append({
            "operation": "concept_guard",
            "resume": resume,
            "primary": primary,
            "alternate": alternate,
            "expected": expected,
            "inverted": inverted,
            "branch_taken": branch_taken,
            "depth_after": word(after, GUARD_TOP) // 2,
            "query_after": after[QUERY],
            "cursor": cpu.reg_read(UC_X86_REG_SI),
        })
    return rows


def jump_vectors(executable):
    rows = []
    for target, resume, alternate in itertools.product(
        (0, 1, 0x1234, 0xFFFF), (0, 2, 0xFF), (0, 0x5678)
    ):
        before = guard_state(2)
        before[RESUME] = resume
        struct.pack_into("<H", before, ALTERNATE, alternate)
        cpu, after = execute(
            executable,
            "jump",
            struct.pack("<H", target),
            before,
            [(RESUME, 1), (ALTERNATE, 2)],
        )
        assert after[RESUME] == 0
        assert word(after, ALTERNATE) == 0
        assert cpu.reg_read(UC_X86_REG_SI) == target
        rows.append({
            "operation": "jump",
            "target": target,
            "resume_before": resume,
            "alternate_before": alternate,
            "resume_after": after[RESUME],
            "alternate_after": word(after, ALTERNATE),
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
        begin_guard_vectors,
        end_guard_vectors,
        random_guard_vectors,
        concept_guard_vectors,
        jump_vectors,
    ):
        rows.extend(generate(executable))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows)
    )
    print(f"captured {len(rows)} original A0-A4 control-flow cases")


if __name__ == "__main__":
    main()
