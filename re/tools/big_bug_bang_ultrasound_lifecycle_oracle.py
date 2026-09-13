#!/usr/bin/env python3
"""Exercise Big Bug Bang's Gravis detection, initialization, and shutdown."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EAX,
    UC_X86_REG_EBP,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDI,
    UC_X86_REG_EDX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_ESI,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ROUTINES = {
    "shutdown": (
        0xDE5B,
        0xDEC3,
        50,
        "b6b650cbd52f9a27919694623f8aa1bc8e7759b0feda3d2693b71fdcead84869",
    ),
    "initialize": (
        0xDEC3,
        0xE05E,
        177,
        "0862d3794aef940eaf89ad74c876d13727b58236328b365d1d628d8172198ae0",
    ),
    "detect": (
        0xE05E,
        0xE0DE,
        58,
        "f9b88ea03d4e4f8ba4452e9c4e18070f3ac58ee32a152d2da0aa5896247ae7da",
    ),
}

DELAY_START = 0xE0DE
DELAY_STOP = 0xE0ED
PARSER_SEGMENT = 0x01E6
PARSER_OFFSET = 0x0332
PARSER_LINEAR = PARSER_SEGMENT * 16 + PARSER_OFFSET
DEAD_INITIALIZE_BRANCH = 0xDF8F
GAME_SEGMENT = 0x3000
ENVIRONMENT_SEGMENT = 0x4000
EXTRA_SEGMENT = 0x5000
FS_SEGMENT = 0x6000
STACK_SEGMENT = 0x7000
UNOWNED_SEGMENT = 0x8000
OLD_VECTOR_SEGMENT = 0x9000
OLD_VECTOR_OFFSET = 0x1357
STACK_POINTER = 0xFF00
RETURN_IP = 0x7400
SEGMENT_SIZE = 0x10000
MACHINE_SIZE = 0xB0000
BASE_PORT = 0x0240
DELAY_VALUE = 0x5A
MASTER_MASK = 0xFD
SLAVE_MASK = 0xFB
FLAG_MASK = 0x08D5
INTERRUPT_FLAG = 0x0200

FIELDS = {
    "environment_segment": 0x0CE1,
    "sound_enabled": 0x0CE7,
    "old_vector_offset": 0x0CD6,
    "old_vector_segment": 0x0CD8,
    "saved_master_mask": 0x0CDF,
    "saved_slave_mask": 0x0CE0,
    "isr_offset": 0x0CF6,
    "isr_segment": 0x0CF8,
    "base_port": 0x0EE5,
    "rate_index": 0x0EE7,
    "frequency": 0x0EE9,
    "dma_primary": 0x0EEB,
    "dma_secondary": 0x0EED,
    "irq_secondary": 0x0EEF,
    "irq_primary": 0x0EF1,
    "name": 0x0EF5,
    "irq_map": 0x0EFE,
    "ultrasound": 0x0F1F,
}

REGISTER_IDS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}

SHUTDOWN_CASES = (
    {"name": "disabled", "enabled": False, "irq": 7},
    {"name": "master_irq", "enabled": True, "irq": 7},
    {"name": "slave_irq", "enabled": True, "irq": 11},
)

INITIALIZE_CASES = (
    {
        "name": "disabled",
        "enabled": False,
        "environment": "first",
        "values": [1, 3, 7, 5],
    },
    {
        "name": "first_environment_entry",
        "enabled": True,
        "environment": "first",
        "values": [1, 3, 7, 5],
    },
    {
        "name": "later_environment_entry",
        "enabled": True,
        "environment": "later",
        "values": [3, 5, 11, 7],
    },
    {
        "name": "missing_environment_entry",
        "enabled": True,
        "environment": "missing",
        "values": [2, 4, 7, 5],
    },
)

DETECT_CASES = (
    {
        "name": "first_port_success",
        "responses": {0x220: [0x55, 0xAA]},
        "result": True,
        "detected_port": 0x220,
    },
    {
        "name": "second_probe_fails_then_later_succeeds",
        "responses": {
            0x220: [0x55, 0x00],
            0x230: [0x00],
            0x240: [0x55, 0xAA],
        },
        "result": True,
        "detected_port": 0x240,
    },
    {
        "name": "all_ports_fail",
        "responses": {},
        "result": False,
        "detected_port": None,
    },
)


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (index * 67 + seed * 43 + 11) & 0xFF for index in range(SEGMENT_SIZE)
    )


def write_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def read_word(machine: Uc, segment: int, offset: int) -> int:
    return struct.unpack("<H", machine.mem_read(segment * 16 + offset, 2))[0]


def initial_registers(case_index: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A10F00 | case_index,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x08D7,
    }


def conditional_edges(
    executable: bytes,
    start: int,
    stop: int,
    excluded: set[int] | None = None,
) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    edges: set[tuple[int, int]] = set()
    for instruction in decoder.disasm(executable[start:stop], start):
        loop_control = instruction.mnemonic in {
            "jcxz",
            "jecxz",
            "loop",
            "loope",
            "loopne",
        }
        if capstone.CS_GRP_JUMP not in instruction.groups and not loop_control:
            continue
        if instruction.id in (X86_INS_JMP, X86_INS_LJMP):
            continue
        if excluded and instruction.address in excluded:
            continue
        target = next(
            (
                int(operand.imm)
                for operand in instruction.operands
                if operand.type == X86_OP_IMM
            ),
            None,
        )
        assert target is not None
        edges.add((instruction.address, target))
        edges.add((instruction.address, instruction.address + instruction.size))
    return edges


def expected_edges(executable: bytes, routine: str) -> set[tuple[int, int]]:
    start, stop, _count, _digest = ROUTINES[routine]
    excluded = {DEAD_INITIALIZE_BRANCH} if routine == "initialize" else None
    return conditional_edges(executable, start, stop, excluded)


def edge_observer(
    executable: bytes,
    routine: str,
    covered: set[tuple[int, int]],
):
    start, stop, _count, _digest = ROUTINES[routine]
    sources = {source for source, _target in expected_edges(executable, routine)}
    previous: int | None = None

    def observe(address: int) -> None:
        nonlocal previous
        if previous is not None and start <= address < stop:
            covered.add((previous, address))
        previous = address if address in sources else None

    def reset() -> None:
        nonlocal previous
        previous = None

    return observe, reset


def common_memory(case_index: int) -> tuple[bytearray, ...]:
    game = seeded_segment(case_index + 1)
    environment = seeded_segment(case_index + 17)
    extra = seeded_segment(case_index + 33)
    fs_data = seeded_segment(case_index + 49)
    stack = seeded_segment(case_index + 65)
    unowned = seeded_segment(case_index + 81)
    game[FIELDS["name"] : FIELDS["name"] + 9] = b"ULTRASND\0"
    write_word(game, FIELDS["base_port"], BASE_PORT)
    write_word(game, FIELDS["environment_segment"], ENVIRONMENT_SEGMENT)
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + b"STABLE"
    )
    return game, environment, extra, fs_data, stack, unowned


def map_machine(
    executable: bytes,
    segments: tuple[tuple[int, bytearray], ...],
    initial: dict[int, int],
    patch_parser: bool = False,
) -> tuple[Uc, bytes]:
    expected_code = bytearray(executable)
    if patch_parser:
        expected_code[PARSER_LINEAR] = 0xCB
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(expected_code))
    for segment, contents in segments:
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)
    return machine, bytes(expected_code)


def register_state(machine: Uc) -> dict[str, int]:
    return {name: machine.reg_read(register) for name, register in REGISTER_IDS.items()}


def assert_return(
    machine: Uc,
    initial: dict[int, int],
    changes: dict[str, int] | None = None,
) -> None:
    expected = {name: initial[register] for name, register in REGISTER_IDS.items()}
    expected["sp"] = STACK_POINTER + 4
    expected.update(changes or {})
    actual = register_state(machine)
    assert actual == expected, (actual, expected)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )


def assert_memory(
    machine: Uc,
    expected_code: bytes,
    expected: dict[int, bytes],
    stack_before: bytes,
    minimum_stack: int,
    writes: list[tuple[int, int]],
    allowed: list[tuple[int, int]],
) -> None:
    assert bytes(machine.mem_read(0, len(expected_code))) == expected_code
    for segment, contents in expected.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    unexpected_stack = [
        offset
        for offset in range(minimum_stack)
        if stack_after[offset] != stack_before[offset]
    ]
    assert not unexpected_stack, (minimum_stack, unexpected_stack[:16], writes)
    assert stack_after[STACK_POINTER + 4 :] == stack_before[STACK_POINTER + 4 :]
    unexpected_writes = [
        (address, size)
        for address, size in writes
        if not any(low <= address and address + size <= high for low, high in allowed)
    ]
    assert not unexpected_writes, unexpected_writes


def logic_flags(value: int) -> int:
    result = value & 0xFF
    flags = 0
    if result == 0:
        flags |= 0x40
    if result & 0x80:
        flags |= 0x80
    if result.bit_count() % 2 == 0:
        flags |= 0x04
    return flags


def irq_vector(irq: int) -> int:
    value = irq + 8
    return value if value <= 0x0F else (value + 0x60) & 0xFF


def voice_reset_outputs(base: int) -> list[tuple[int, int, int]]:
    outputs: list[tuple[int, int, int]] = []
    for voice in range(31, -1, -1):
        outputs.extend(
            [
                (base + 0x102, 1, voice),
                (base + 0x103, 1, 0),
                (base + 0x105, 1, 3),
                (base + 0x105, 1, 3),
            ]
        )
    return outputs


def initialize_voice_outputs(base: int) -> list[tuple[int, int, int]]:
    outputs: list[tuple[int, int, int]] = []
    for voice in range(31, -1, -1):
        outputs.extend(
            [
                (base + 0x102, 1, voice),
                (base + 0x103, 1, 0),
                (base + 0x105, 1, 3),
                (base + 0x105, 1, 3),
                (base + 0x103, 1, 0x0D),
                (base + 0x105, 1, 3),
                (base + 0x105, 1, 3),
                (base + 0x103, 1, 9),
                (base + 0x104, 2, 0),
                (base + 0x104, 2, 0),
                (base + 0x103, 1, 0x0C),
                (base + 0x105, 1, 7),
            ]
        )
    return outputs


def memory_write_hook(writes: list[tuple[int, int]]):
    def memory_write(
        _cpu: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address, size))

    return memory_write


def execute_shutdown(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, environment, extra, fs_data, stack, unowned = common_memory(case_index)
    initial = initial_registers(case_index)
    enabled = bool(case["enabled"])
    irq = int(case["irq"])
    game[FIELDS["ultrasound"]] = int(enabled)
    write_word(game, FIELDS["irq_primary"], irq)
    write_word(game, FIELDS["old_vector_offset"], OLD_VECTOR_OFFSET)
    write_word(game, FIELDS["old_vector_segment"], OLD_VECTOR_SEGMENT)
    game[FIELDS["saved_master_mask"]] = 0xD7
    game[FIELDS["saved_slave_mask"]] = 0xEB
    segments = (
        (GAME_SEGMENT, game),
        (ENVIRONMENT_SEGMENT, environment),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    )
    machine, expected_code = map_machine(executable, segments, initial)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "shutdown", covered)
    inputs: list[tuple[int, int, int]] = []
    outputs: list[tuple[int, int, int]] = []
    interrupts: list[dict[str, int]] = []
    writes: list[tuple[int, int]] = []
    reached_return = False

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        if port == BASE_PORT + 6:
            value = 0x31
        elif port == BASE_PORT + 0x105:
            value = 0x42
        else:
            assert port == 0x0300
            value = DELAY_VALUE
        inputs.append((port, size, value))
        return value

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        assert number == 0x21
        vector = irq_vector(irq)
        call = {
            "eax": cpu.reg_read(UC_X86_REG_EAX),
            "ebx": cpu.reg_read(UC_X86_REG_EBX),
            "edx": cpu.reg_read(UC_X86_REG_EDX),
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "sp": cpu.reg_read(UC_X86_REG_SP),
        }
        interrupts.append(call)
        assert call == {
            "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x2500 | vector,
            "ebx": initial[UC_X86_REG_EBX],
            "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | OLD_VECTOR_OFFSET,
            "ds": OLD_VECTOR_SEGMENT,
            "sp": 0xFEF8,
        }
        cpu.reg_write(UC_X86_REG_EFLAGS, 0x0857)

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if ROUTINES["shutdown"][0] <= address < ROUTINES["shutdown"][1]:
            observe(address)
            return
        assert DELAY_START <= address < DELAY_STOP
        reset()

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write_hook(writes))
    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    try:
        machine.emu_start(ROUTINES["shutdown"][0], 0, count=100_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    if enabled:
        expected_inputs = [(BASE_PORT + 6, 1, 0x31), (BASE_PORT + 0x105, 1, 0x42)] + [
            (0x0300, 1, DELAY_VALUE)
        ] * (32 * 7)
        expected_outputs = [
            (BASE_PORT + 0x103, 1, 0x8F),
        ] + voice_reset_outputs(BASE_PORT)
        expected_outputs.extend([(0x21, 1, 0xD7), (0xA1, 1, 0xEB)])
        assert inputs == expected_inputs
        assert outputs == expected_outputs, (outputs, expected_outputs)
        assert len(interrupts) == 1
        assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == 0x0855
        assert machine.reg_read(UC_X86_REG_EFLAGS) & INTERRUPT_FLAG
    else:
        assert not inputs and not outputs and not interrupts
        assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == logic_flags(0)
        assert not machine.reg_read(UC_X86_REG_EFLAGS) & INTERRUPT_FLAG
    assert_return(machine, initial)
    assert_memory(
        machine,
        expected_code,
        {
            GAME_SEGMENT: bytes(game),
            ENVIRONMENT_SEGMENT: bytes(environment),
            EXTRA_SEGMENT: bytes(extra),
            FS_SEGMENT: bytes(fs_data),
            UNOWNED_SEGMENT: bytes(unowned),
        },
        bytes(stack),
        0xFEF4 if enabled else 0xFEFA,
        writes,
        [
            (
                STACK_SEGMENT * 16 + (0xFEF4 if enabled else 0xFEFA),
                STACK_SEGMENT * 16 + STACK_POINTER + 4,
            )
        ],
    )
    return {
        "routine": "shutdown",
        "name": case["name"],
        "active": enabled,
        "irq": irq,
        "vector": irq_vector(irq) if enabled else None,
        "delay_reads": 32 * 7 if enabled else 0,
        "port_reads": len(inputs),
        "port_writes": len(outputs),
    }, covered


def environment_bytes(mode: str, values: list[int]) -> bytes:
    setting = f"ULTRASND=240,{values[0]},{values[1]},{values[2]},{values[3]}"
    if mode == "first":
        text = setting + "\0PATH=X\0\0"
    elif mode == "later":
        text = "PATH=X\0" + setting + "\0HOME=Y\0\0"
    else:
        assert mode == "missing"
        text = "PATH=X\0HOME=Y\0\0"
    return text.encode("ascii")


def initialize_outputs(
    base: int,
    irq_map_value: int,
    rate_index: int,
    irq: int,
) -> list[tuple[int, int, int]]:
    outputs = [
        (base, 1, 0x49),
        (base + 0x0B, 1, irq_map_value | 0x40),
    ]
    outputs.extend(initialize_voice_outputs(base))
    outputs.extend(
        [
            (base + 0x103, 1, 0x0E),
            (base + 0x105, 1, ((rate_index - 1) | 0xC0) & 0xFF),
            (base + 0x103, 1, 0x4C),
            (base + 0x105, 1, 7),
        ]
    )
    if irq > 7:
        slave_bit = 1 << (irq - 8)
        outputs.append((0xA1, 1, SLAVE_MASK & (~slave_bit & 0xFF)))
        master_irq = 2
    else:
        master_irq = irq
    master_bit = 1 << master_irq
    outputs.append((0x21, 1, MASTER_MASK & (~master_bit & 0xFF)))
    return outputs


def execute_initialize(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, environment, extra, fs_data, stack, unowned = common_memory(case_index + 20)
    initial = initial_registers(case_index + 20)
    enabled = bool(case["enabled"])
    mode = str(case["environment"])
    values = [int(value) for value in case["values"]]
    rate_index = 15 + case_index
    initial[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | rate_index
    game[FIELDS["ultrasound"]] = int(enabled)
    environment_data = environment_bytes(mode, values)
    environment[: len(environment_data)] = environment_data
    stored_values = (
        values
        if mode == "missing"
        else [0x1101 + case_index, 0x2202 + case_index, 5, 6]
    )
    for field, value in zip(
        ("dma_primary", "dma_secondary", "irq_primary", "irq_secondary"),
        stored_values,
        strict=True,
    ):
        write_word(game, FIELDS[field], value)
    irq = values[2]
    irq_map_value = 0x12 + case_index
    game[FIELDS["irq_map"] + irq - 2] = irq_map_value
    frequency_byte = 0xA4 + case_index
    game[FIELDS["irq_map"] + rate_index] = frequency_byte
    segments = (
        (GAME_SEGMENT, game),
        (ENVIRONMENT_SEGMENT, environment),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    )
    machine, expected_code = map_machine(
        executable, segments, initial, patch_parser=True
    )
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "initialize", covered)
    inputs: list[tuple[int, int, int]] = []
    outputs: list[tuple[int, int, int]] = []
    interrupts: list[dict[str, int]] = []
    parser_calls: list[dict[str, Any]] = []
    writes: list[tuple[int, int]] = []
    reached_return = False
    hardware_reads = iter((0x31, 0x42))

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        if port == 0x0300:
            value = DELAY_VALUE
        elif port == BASE_PORT + 6:
            value = next(hardware_reads)
        elif port == 0xA1:
            value = SLAVE_MASK
        else:
            assert port == 0x21
            value = MASTER_MASK
        inputs.append((port, size, value))
        return value

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        assert number == 0x21
        function = (cpu.reg_read(UC_X86_REG_AX) >> 8) & 0xFF
        vector = irq_vector(irq)
        call = {
            "function": function,
            "eax": cpu.reg_read(UC_X86_REG_EAX),
            "ebx": cpu.reg_read(UC_X86_REG_EBX),
            "edx": cpu.reg_read(UC_X86_REG_EDX),
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "es": cpu.reg_read(UC_X86_REG_ES),
            "sp": cpu.reg_read(UC_X86_REG_SP),
        }
        interrupts.append(call)
        if function == 0x35:
            assert (
                call["eax"] == (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x3500 | vector
            )
            assert call["ebx"] == initial[UC_X86_REG_EBX]
            assert call["ds"] == GAME_SEGMENT
            assert call["sp"] == 0xFEF0
            cpu.reg_write(UC_X86_REG_BX, OLD_VECTOR_OFFSET)
            cpu.reg_write(UC_X86_REG_ES, OLD_VECTOR_SEGMENT)
        else:
            assert function == 0x25
            assert call == {
                "function": 0x25,
                "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x2500 | vector,
                "ebx": initial[UC_X86_REG_EBX] & 0xFFFF0000,
                "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | 0x11AD,
                "ds": 0,
                "es": OLD_VECTOR_SEGMENT,
                "sp": 0xFEEE,
            }
        cpu.reg_write(UC_X86_REG_EFLAGS, 0x0246)

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear == PARSER_LINEAR and cpu.reg_read(UC_X86_REG_CS) == PARSER_SEGMENT:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEC
            frame = struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEC, 4))
            assert frame[1] == 0
            si = cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF
            di = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
            assert si == di
            assert cpu.reg_read(UC_X86_REG_DS) == ENVIRONMENT_SEGMENT
            assert cpu.reg_read(UC_X86_REG_ES) == ENVIRONMENT_SEGMENT
            raw = bytes(cpu.mem_read(ENVIRONMENT_SEGMENT * 16 + si, 8))
            digits = raw.split(b",", 1)[0].split(b"\0", 1)[0]
            value = int(digits)
            parser_calls.append(
                {
                    "return": frame[0],
                    "si": si,
                    "di": di,
                    "value": value,
                }
            )
            cpu.reg_write(UC_X86_REG_AX, value)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0283)
            reset()
            return
        if ROUTINES["initialize"][0] <= address < ROUTINES["initialize"][1]:
            assert address != DEAD_INITIALIZE_BRANCH
            observe(address)
            return
        assert DELAY_START <= address < DELAY_STOP
        reset()

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write_hook(writes))
    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    try:
        machine.emu_start(ROUTINES["initialize"][0], 0, count=300_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    found = enabled and mode != "missing"
    expected_game = bytearray(game)
    allowed = [
        (
            STACK_SEGMENT * 16 + (0xFEEA if enabled else 0xFEF0),
            STACK_SEGMENT * 16 + STACK_POINTER + 4,
        )
    ]
    if enabled:
        write_word(expected_game, FIELDS["rate_index"], rate_index)
        expected_game[FIELDS["sound_enabled"]] = 1
        write_word(expected_game, FIELDS["isr_offset"], 0x011D)
        write_word(expected_game, FIELDS["isr_segment"], 0)
        if found:
            for field, value in zip(
                ("dma_primary", "dma_secondary", "irq_primary", "irq_secondary"),
                values,
                strict=True,
            ):
                write_word(expected_game, FIELDS[field], value)
        write_word(expected_game, FIELDS["old_vector_offset"], OLD_VECTOR_OFFSET)
        write_word(expected_game, FIELDS["old_vector_segment"], OLD_VECTOR_SEGMENT)
        frequency = frequency_byte | (0xFF00 if frequency_byte & 0x80 else 0)
        write_word(expected_game, FIELDS["frequency"], frequency)
        expected_game[FIELDS["saved_master_mask"]] = MASTER_MASK
        if irq > 7:
            expected_game[FIELDS["saved_slave_mask"]] = SLAVE_MASK
        mutable_fields = (
            (FIELDS["rate_index"], 2),
            (FIELDS["sound_enabled"], 1),
            (FIELDS["isr_offset"], 4),
            (FIELDS["dma_primary"], 8),
            (FIELDS["old_vector_offset"], 4),
            (FIELDS["frequency"], 2),
            (FIELDS["saved_master_mask"], 2),
        )
        allowed.extend(
            (GAME_SEGMENT * 16 + offset, GAME_SEGMENT * 16 + offset + size)
            for offset, size in mutable_fields
        )
        expected_parser_returns = [0xDF14, 0xDF23, 0xDF32, 0xDF41] if found else []
        assert [call["return"] for call in parser_calls] == expected_parser_returns
        if found:
            assert [call["value"] for call in parser_calls] == values
        assert [call["function"] for call in interrupts] == [0x35, 0x25]
        expected_inputs = [(0x0300, 1, DELAY_VALUE)] * (32 * 3 * 7)
        expected_inputs.extend(
            [
                (BASE_PORT + 6, 1, 0x31),
                *([(0x0300, 1, DELAY_VALUE)] * 7),
                (BASE_PORT + 6, 1, 0x42),
            ]
        )
        if irq > 7:
            expected_inputs.append((0xA1, 1, SLAVE_MASK))
        expected_inputs.append((0x21, 1, MASTER_MASK))
        assert inputs == expected_inputs
        assert outputs == initialize_outputs(BASE_PORT, irq_map_value, rate_index, irq)
        final_mask = (
            SLAVE_MASK & (~(1 << (irq - 8)) & 0xFF)
            if irq > 7
            else MASTER_MASK & (~(1 << irq) & 0xFF)
        )
        if irq > 7:
            final_mask = MASTER_MASK & (~(1 << 2) & 0xFF)
        assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == logic_flags(
            final_mask
        )
        assert machine.reg_read(UC_X86_REG_EFLAGS) & INTERRUPT_FLAG
    else:
        assert not parser_calls and not interrupts and not inputs and not outputs
        assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == logic_flags(0)
        assert not machine.reg_read(UC_X86_REG_EFLAGS) & INTERRUPT_FLAG
    assert_return(machine, initial)
    assert_memory(
        machine,
        expected_code,
        {
            GAME_SEGMENT: bytes(expected_game),
            ENVIRONMENT_SEGMENT: bytes(environment),
            EXTRA_SEGMENT: bytes(extra),
            FS_SEGMENT: bytes(fs_data),
            UNOWNED_SEGMENT: bytes(unowned),
        },
        bytes(stack),
        0xFEEA if enabled else 0xFEF0,
        writes,
        allowed,
    )
    return {
        "routine": "initialize",
        "name": case["name"],
        "active": enabled,
        "environment_found": found if enabled else None,
        "parsed_values": values if found else [],
        "irq": irq if enabled else None,
        "vector": irq_vector(irq) if enabled else None,
        "parser_calls": len(parser_calls),
        "delay_reads": 32 * 3 * 7 + 7 if enabled else 0,
        "port_reads": len(inputs),
        "port_writes": len(outputs),
        "dead_equal_irq_branch_reached": False,
    }, covered


def detect_probe_outputs(base: int, first_success: bool) -> list[tuple[int, int, int]]:
    outputs = [
        (base + 0x103, 1, 0x4C),
        (base + 0x105, 1, 0),
        (base + 0x103, 1, 0x4C),
        (base + 0x105, 1, 1),
        (base + 0x103, 1, 0x43),
        (base + 0x104, 2, 0),
        (base + 0x103, 1, 0x44),
        (base + 0x105, 1, 0),
        (base + 0x107, 1, 0xAA),
        (base + 0x103, 1, 0x43),
        (base + 0x104, 2, 1),
        (base + 0x107, 1, 0x55),
    ]
    if first_success:
        outputs.append((base + 0x104, 2, 0))
    return outputs


def execute_detect(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, environment, extra, fs_data, stack, unowned = common_memory(case_index + 60)
    initial = initial_registers(case_index + 60)
    game[FIELDS["ultrasound"]] = 0
    responses = {
        int(port): [int(value) for value in values]
        for port, values in case["responses"].items()
    }
    segments = (
        (GAME_SEGMENT, game),
        (ENVIRONMENT_SEGMENT, environment),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    )
    machine, expected_code = map_machine(executable, segments, initial)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "detect", covered)
    inputs: list[tuple[int, int, int]] = []
    outputs: list[tuple[int, int, int]] = []
    writes: list[tuple[int, int]] = []
    reached_return = False
    probe_indices: dict[int, int] = {}

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        if port == 0x0300:
            value = DELAY_VALUE
        else:
            base = port - 0x107
            assert 0x220 <= base <= 0x260 and base & 0xF == 0
            index = probe_indices.get(base, 0)
            values = responses.get(base, [0x00])
            value = values[index] if index < len(values) else 0x00
            probe_indices[base] = index + 1
        inputs.append((port, size, value))
        return value

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if ROUTINES["detect"][0] <= address < ROUTINES["detect"][1]:
            observe(address)
            return
        assert DELAY_START <= address < DELAY_STOP
        reset()

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write_hook(writes))
    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    try:
        machine.emu_start(ROUTINES["detect"][0], 0, count=100_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    success = bool(case["result"])
    detected_port = case["detected_port"]
    last_probe = int(detected_port) if success else 0x260
    expected_inputs: list[tuple[int, int, int]] = []
    expected_outputs: list[tuple[int, int, int]] = []
    for base in range(0x220, last_probe + 1, 0x10):
        first = responses.get(base, [0])[0]
        first_success = first == 0x55
        expected_inputs.extend([(0x0300, 1, DELAY_VALUE)] * 14)
        expected_outputs.extend(detect_probe_outputs(base, first_success))
        expected_inputs.append((base + 0x107, 1, first))
        if first_success:
            second_values = responses.get(base, [0, 0])
            second = second_values[1] if len(second_values) > 1 else 0
            expected_inputs.append((base + 0x107, 1, second))
            if second == 0xAA:
                break
    assert inputs == expected_inputs
    assert outputs == expected_outputs

    expected_game = bytearray(game)
    final_base = int(detected_port) if success else 0x270
    write_word(expected_game, FIELDS["base_port"], final_base)
    expected_game[FIELDS["ultrasound"]] = int(success)
    final_dx = last_probe + 0x107
    assert_return(
        machine,
        initial,
        {
            "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | int(success),
            "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | final_dx,
        },
    )
    assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == 0x44
    assert not machine.reg_read(UC_X86_REG_EFLAGS) & INTERRUPT_FLAG
    allowed = [
        (
            STACK_SEGMENT * 16 + 0xFEFA,
            STACK_SEGMENT * 16 + STACK_POINTER + 4,
        ),
        (
            GAME_SEGMENT * 16 + FIELDS["base_port"],
            GAME_SEGMENT * 16 + FIELDS["base_port"] + 2,
        ),
        (
            GAME_SEGMENT * 16 + FIELDS["ultrasound"],
            GAME_SEGMENT * 16 + FIELDS["ultrasound"] + 1,
        ),
    ]
    assert_memory(
        machine,
        expected_code,
        {
            GAME_SEGMENT: bytes(expected_game),
            ENVIRONMENT_SEGMENT: bytes(environment),
            EXTRA_SEGMENT: bytes(extra),
            FS_SEGMENT: bytes(fs_data),
            UNOWNED_SEGMENT: bytes(unowned),
        },
        bytes(stack),
        0xFEFA,
        writes,
        allowed,
    )
    return {
        "routine": "detect",
        "name": case["name"],
        "detected": success,
        "detected_port": detected_port,
        "probed_ports": list(range(0x220, last_probe + 1, 0x10)),
        "delay_reads": 14 * len(range(0x220, last_probe + 1, 0x10)),
        "port_reads": len(inputs),
        "port_writes": len(outputs),
    }, covered


def assert_bodies(executable: bytes) -> None:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    for name, (start, stop, expected_count, expected_digest) in ROUTINES.items():
        body = executable[start:stop]
        assert hashlib.sha256(body).hexdigest() == expected_digest, name
        assert len(list(decoder.disasm(body, start))) == expected_count, name


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.sequel_executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != SEQUEL_SHA256:
        raise SystemExit(f"unsupported BBB executable SHA-256 {digest}")
    assert_bodies(executable)

    rows: list[dict[str, Any]] = []
    coverage = {name: set() for name in ROUTINES}
    for index, case in enumerate(SHUTDOWN_CASES):
        row, edges = execute_shutdown(executable, case, index)
        rows.append(row)
        coverage["shutdown"].update(edges)
    for index, case in enumerate(INITIALIZE_CASES):
        row, edges = execute_initialize(executable, case, index)
        rows.append(row)
        coverage["initialize"].update(edges)
    for index, case in enumerate(DETECT_CASES):
        row, edges = execute_detect(executable, case, index)
        rows.append(row)
        coverage["detect"].update(edges)

    total_edges = 0
    for name, covered in coverage.items():
        expected = expected_edges(executable, name)
        missing = expected - covered
        assert not missing, f"{name} missing conditional edges: {sorted(missing)}"
        assert covered == expected
        total_edges += len(expected)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB Ultrasound lifecycle cases across "
        f"{total_edges} reachable conditional edges"
    )


if __name__ == "__main__":
    main()
