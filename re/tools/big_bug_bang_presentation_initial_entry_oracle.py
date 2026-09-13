#!/usr/bin/env python3
"""Compare BBB's initial presentation-entry loader with Commander Blood."""

# ruff: noqa: E402

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16, Uc
from unicorn.x86_const import (
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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
BUFFER = 0x30000
BUFFER_WRAP = 0x40000
EXTRA = 0x70000
FS_DATA = 0x80000
GAME = 0xA0000
STACK = 0xB0000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    read_span: tuple[int, int]
    read_sha256: str
    init_span: tuple[int, int]
    init_sha256: str
    banked: int
    ems_handle: int
    xms_handle: int
    buffer_segment: int
    buffer_end: int
    file_handle: int
    source_offset: int
    source_remaining: int
    head: int
    head_segment: int
    tail: int
    tail_segment: int
    active: int
    wrap_limit: int
    byte_count: int
    iteration_count: int
    branches: dict[int, tuple[int, int]]
    required_edges: tuple[tuple[int, int], ...]
    return_addresses: tuple[int, ...]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA642, 0xA73E),
    body_sha256="ad608c6fb80044aeec02c44132c4bd973b66c4c1e6431349799532debce68133",
    read_span=(0xA622, 0xA634),
    read_sha256="b11b14dd5e323bd7f73ffe721f8a5ddcce7dd7e34e770391077c71deb442a6fe",
    init_span=(0xA757, 0xA778),
    init_sha256="1898e092b06c16b06473284e2f625a6c4aa576718d90f4c44fe81021ad09040f",
    banked=0x0DBC,
    ems_handle=0x0A58,
    xms_handle=0x0A56,
    buffer_segment=0x0A7E,
    buffer_end=0x5233,
    file_handle=0x0D5B,
    source_offset=0x0D84,
    source_remaining=0x0D88,
    head=0x0D8C,
    head_segment=0x0D8E,
    tail=0x0D90,
    tail_segment=0x0D92,
    active=0x0D96,
    wrap_limit=0x0D98,
    byte_count=0x0D9A,
    iteration_count=0x0DA0,
    branches={
        0xA628: (0xA62A, 0xA633),
        0xA649: (0xA64D, 0xA73D),
        0xA669: (0xA66D, 0xA6FC),
        0xA703: (0xA705, 0xA73D),
        0xA720: (0xA705, 0xA722),
    },
    required_edges=(
        (0xA628, 0xA62A),
        (0xA628, 0xA633),
        (0xA649, 0xA64D),
        (0xA649, 0xA73D),
        (0xA669, 0xA6FC),
        (0xA703, 0xA705),
        (0xA703, 0xA73D),
        (0xA720, 0xA705),
        (0xA720, 0xA722),
    ),
    return_addresses=(0xA646, 0xA649, 0xA628),
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBE2C, 0xBF28),
    body_sha256="877fa54af98b02f51ce11892845bfccfdcf6928d8ff00b60f6cf2b3816c986c9",
    read_span=(0xBE0C, 0xBE1E),
    read_sha256="fedab3c41c3b155fe2f0e9a912c37113e3a7b93637c831ae4ccc23470e822df4",
    init_span=(0xBF41, 0xBF62),
    init_sha256="669174423f6374d436eaed9b2d313493ceceee75381b5a14f09226d25c36b2e9",
    banked=0x100A,
    ems_handle=0x0C50,
    xms_handle=0x0C4E,
    buffer_segment=0x0C76,
    buffer_end=0x5603,
    file_handle=0x0FA9,
    source_offset=0x0FD2,
    source_remaining=0x0FD6,
    head=0x0FDA,
    head_segment=0x0FDC,
    tail=0x0FDE,
    tail_segment=0x0FE0,
    active=0x0FE4,
    wrap_limit=0x0FE6,
    byte_count=0x0FE8,
    iteration_count=0x0FEE,
    branches={
        0xBE12: (0xBE14, 0xBE1D),
        0xBE33: (0xBE37, 0xBF27),
        0xBE53: (0xBE57, 0xBEE6),
        0xBEED: (0xBEEF, 0xBF27),
        0xBF0A: (0xBEEF, 0xBF0C),
    },
    required_edges=(
        (0xBE12, 0xBE14),
        (0xBE12, 0xBE1D),
        (0xBE33, 0xBE37),
        (0xBE33, 0xBF27),
        (0xBE53, 0xBEE6),
        (0xBEED, 0xBEEF),
        (0xBEED, 0xBF27),
        (0xBF0A, 0xBEEF),
        (0xBF0A, 0xBF0C),
    ),
    return_addresses=(0xBE30, 0xBE33, 0xBE12),
)

REGISTERS = (
    UC_X86_REG_EAX,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDX,
    UC_X86_REG_ESI,
    UC_X86_REG_EDI,
    UC_X86_REG_EBP,
    UC_X86_REG_SP,
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_ES,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SS,
)

CASES = (
    {
        "name": "initial_no_handle",
        "file_handle": 0,
        "buffer_end": 0x9000,
        "extent": 10,
        "source_offset": 0x12345678,
        "source_remaining": 0x01020304,
        "reads": (),
    },
    {
        "name": "extent_two_empty_body",
        "file_handle": 4,
        "buffer_end": 0x9000,
        "extent": 2,
        "source_offset": 0,
        "source_remaining": 2,
        "reads": (
            ("initial", 2, False, struct.pack("<H", 2), False),
            ("body", 0, False, b"", False),
        ),
    },
    {
        "name": "ordinary_extent",
        "file_handle": 5,
        "buffer_end": 0xA000,
        "extent": 10,
        "source_offset": 0x0000FFFF,
        "source_remaining": 0x00010020,
        "reads": (
            ("initial", 2, False, struct.pack("<H", 10), False),
            ("body", 8, False, b"BODYDATA", False),
        ),
    },
    {
        "name": "short_reads_ignore_carry",
        "file_handle": 6,
        "buffer_end": 0x7F00,
        "extent": 12,
        "source_offset": 0x10203040,
        "source_remaining": 0x55667788,
        "reads": (
            ("initial", 1, False, b"\x0c", False),
            ("initial", 2, True, struct.pack("<H", 12), False),
            ("body", 3, True, b"bad", False),
            ("body", 10, False, b"0123456789", False),
        ),
    },
    {
        "name": "body_handle_removed",
        "file_handle": 7,
        "buffer_end": 0x8800,
        "extent": 6,
        "source_offset": 0xFFFFFFFE,
        "source_remaining": 8,
        "reads": (("initial", 2, False, struct.pack("<H", 6), True),),
    },
    {
        "name": "extent_one_wraps_body_count",
        "file_handle": 8,
        "buffer_end": 0x7000,
        "extent": 1,
        "source_offset": 0x01020304,
        "source_remaining": 0x00020000,
        "reads": (
            ("initial", 2, False, struct.pack("<H", 1), False),
            ("body", 0xFFFF, False, b"", False),
        ),
    },
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def read32(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def with_low16(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def normalize_edge(routine: Routine, edge: tuple[int, int]) -> tuple[int, int]:
    return edge[0] - routine.span[0], edge[1] - routine.span[0]


def required_edges(routine: Routine) -> set[tuple[int, int]]:
    return {normalize_edge(routine, edge) for edge in routine.required_edges}


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    replacements = {
        routine.image_address(address): COMMANDER.image_address(commander_address)
        for address, commander_address in zip(
            routine.return_addresses, COMMANDER.return_addresses, strict=True
        )
    }
    for offset in range(STACK_POINTER - 32, STACK_POINTER + 2, 2):
        value = read16(result, offset)
        if value in replacements:
            write16(result, offset, replacements[value])
    return bytes(result)


def assert_unchanged_outside(
    before: bytes,
    after: bytes,
    allowed: list[tuple[int, int]],
    label: str,
) -> None:
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or any(start <= offset < end for start, end in allowed):
            continue
        raise AssertionError(f"{label} changed outside owned state at {offset:#x}")


def verify_span(
    executable: bytes, span: tuple[int, int], expected_sha256: str, label: str
) -> None:
    digest = hashlib.sha256(executable[slice(*span)]).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"{label} span {span[0]:#x}..{span[1]:#x} changed: {digest}")


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    verify_span(executable, routine.span, routine.body_sha256, routine.name)
    verify_span(
        executable, routine.read_span, routine.read_sha256, f"{routine.name} reader"
    )
    verify_span(
        executable, routine.init_span, routine.init_sha256, f"{routine.name} init"
    )
    assert len(executable[slice(*routine.span)]) == 252
    assert executable[routine.span[1] - 1] == 0xC3
    assert executable[routine.init_span[1] - 1] == 0xCB
    return executable


def state_snapshot(game: bytes, routine: Routine) -> dict[str, int]:
    return {
        "head": read16(game, routine.head),
        "head_segment": read16(game, routine.head_segment),
        "tail": read16(game, routine.tail),
        "tail_segment": read16(game, routine.tail_segment),
        "active": read16(game, routine.active),
        "wrap_limit": read16(game, routine.wrap_limit),
        "byte_count": read16(game, routine.byte_count),
        "iteration_count": read16(game, routine.iteration_count),
        "source_offset": read32(game, routine.source_offset),
        "source_remaining": read32(game, routine.source_remaining),
    }


def execute(
    executable: bytes,
    routine: Routine,
    case: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(case["name"])
    file_handle = int(case["file_handle"])
    buffer_end = int(case["buffer_end"])
    extent = int(case["extent"])
    source_offset = int(case["source_offset"])
    source_remaining = int(case["source_remaining"])
    reads = list(case["reads"])
    body_count = (extent - 2) & 0xFFFF
    entry_start = (buffer_end - extent - 2) & 0xFFFF
    body_start = (entry_start + 2) & 0xFFFF
    initial_failed = file_handle < 1
    body_failed = bool(reads and reads[-1][4])
    success = not initial_failed and not body_failed

    game_before = seeded_segment(case_index, 17, 0x13)
    data_before = seeded_segment(case_index, 19, 0x25)
    buffer_before = seeded_segment(case_index, 23, 0x37)
    wrap_before = seeded_segment(case_index, 29, 0x49)
    extra_before = seeded_segment(case_index, 31, 0x5B)
    fs_before = seeded_segment(case_index, 37, 0x6D)
    stack_before = seeded_segment(case_index, 41, 0x7F)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    game_before[routine.banked] = 0
    write16(game_before, routine.ems_handle, 0xFFFF)
    write16(game_before, routine.xms_handle, 0xFFFF)
    write16(game_before, routine.buffer_segment, BUFFER // 16)
    write16(game_before, routine.buffer_end, buffer_end)
    write16(game_before, routine.file_handle, file_handle)
    write32(game_before, routine.source_offset, source_offset)
    write32(game_before, routine.source_remaining, source_remaining)
    for offset, value in (
        (routine.head, 0x1111),
        (routine.head_segment, 0x2222),
        (routine.tail, 0x3333),
        (routine.tail_segment, 0x4444),
        (routine.active, 0x5555),
        (routine.wrap_limit, 0x6666),
        (routine.byte_count, 0x7777),
        (routine.iteration_count, 0x8888),
    ):
        write16(game_before, offset, value)

    game_expected = bytearray(game_before)
    write16(game_expected, routine.head_segment, BUFFER // 16)
    write16(game_expected, routine.tail_segment, BUFFER // 16)
    write16(game_expected, routine.active, 0)
    write16(game_expected, routine.wrap_limit, buffer_end)
    write16(game_expected, routine.iteration_count, 0)
    if initial_failed:
        transferred = 0
        expected_head = 0
        expected_tail = 0
        expected_count = 0
    elif body_failed:
        transferred = 2
        expected_head = body_start
        expected_tail = entry_start
        expected_count = 2
        write16(game_expected, routine.file_handle, 0)
    else:
        transferred = 2 + body_count
        expected_head = body_start + body_count
        expected_tail = entry_start
        expected_count = 2 + body_count
    write16(game_expected, routine.head, expected_head)
    write16(game_expected, routine.tail, expected_tail)
    write16(game_expected, routine.byte_count, expected_count)
    write32(game_expected, routine.source_offset, source_offset + transferred)
    write32(game_expected, routine.source_remaining, source_remaining - transferred)

    buffer_expected = bytearray(buffer_before)
    wrap_expected = bytearray(wrap_before)

    def write_destination(offset: int, contents: bytes) -> None:
        first_count = min(len(contents), SEGMENT_SIZE - offset)
        buffer_expected[offset : offset + first_count] = contents[:first_count]
        wrap_expected[: len(contents) - first_count] = contents[first_count:]

    if not initial_failed:
        for stage, _returned, _carry, payload, _drop_handle in reads:
            write_destination(0 if stage == "initial" else body_start, bytes(payload))
        write_destination(entry_start, struct.pack("<H", extent))

    initial_eax = 0xA5A50000 | case_index
    initial = {
        UC_X86_REG_EAX: initial_eax,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E55555,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xC0000)
    module = executable[routine.header_size :]
    machine.mem_write(0, module)
    for base, contents in (
        (DATA, data_before),
        (BUFFER, buffer_before),
        (BUFFER_WRAP, wrap_before),
        (EXTRA, extra_before),
        (FS_DATA, fs_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, object]] = []
    read_index = 0
    reached_return = False
    previous: int | None = None

    def interrupt(cpu: Uc, number: int, _context) -> None:
        nonlocal read_index
        assert number == 0x21 and read_index < len(reads), (
            routine.name,
            name,
            hex(number),
            read_index,
        )
        stage, returned, carry, payload, drop_handle = reads[read_index]
        function = (cpu.reg_read(UC_X86_REG_EAX) >> 8) & 0xFF
        expected_offset = (source_offset + (2 if stage == "body" else 0)) & 0xFFFFFFFF
        if function == 0x42:
            calls.append(
                {
                    "call": "seek",
                    "stage": stage,
                    "handle": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                    "offset": (
                        (cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF) << 16
                        | (cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF)
                    ),
                }
            )
            cpu.reg_write(
                UC_X86_REG_EAX,
                with_low16(cpu.reg_read(UC_X86_REG_EAX), expected_offset),
            )
            cpu.reg_write(
                UC_X86_REG_EDX,
                with_low16(cpu.reg_read(UC_X86_REG_EDX), expected_offset >> 16),
            )
            set_carry(cpu, False)
            return

        assert function == 0x3F, (routine.name, name, hex(function))
        requested = cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF
        destination_segment = cpu.reg_read(UC_X86_REG_DS)
        destination_offset = cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF
        assert requested == (2 if stage == "initial" else body_count)
        assert destination_segment == BUFFER // 16
        assert destination_offset == (0 if stage == "initial" else body_start)
        if payload:
            cpu.mem_write(destination_segment * 16 + destination_offset, bytes(payload))
        calls.append(
            {
                "call": "read",
                "stage": stage,
                "handle": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                "requested": requested,
                "destination": destination_offset,
                "returned": returned,
                "carry": carry,
            }
        )
        cpu.reg_write(
            UC_X86_REG_EAX, with_low16(cpu.reg_read(UC_X86_REG_EAX), returned)
        )
        set_carry(cpu, carry)
        if drop_handle:
            cpu.mem_write(GAME + routine.file_handle, struct.pack("<H", 0))
        read_index += 1

    allowed_spans = (routine.span, routine.read_span, routine.init_span)

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        assert any(start <= file_address < end for start, end in allowed_spans), (
            routine.name,
            name,
            hex(file_address),
        )
        if previous in routine.branches:
            covered_edges.add(
                normalize_edge(routine, (previous, file_address))
            )
        previous = file_address

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=5000)
    assert reached_return, (routine.name, name)
    assert read_index == len(reads), (routine.name, name, read_index, len(reads))
    assert bytes(machine.mem_read(0, len(module))) == module

    after = {
        base: bytes(machine.mem_read(base, SEGMENT_SIZE))
        for base in (DATA, BUFFER, BUFFER_WRAP, EXTRA, FS_DATA, GAME, STACK)
    }
    for base, expected, label in (
        (DATA, data_before, "data decoy"),
        (BUFFER, buffer_expected, "queue buffer"),
        (BUFFER_WRAP, wrap_expected, "queue wrap"),
        (EXTRA, extra_before, "ES decoy"),
        (FS_DATA, fs_before, "FS decoy"),
    ):
        assert after[base] == bytes(expected), (routine.name, name, label)
    game_allowed = [
        (routine.file_handle, routine.file_handle + 2),
        (routine.source_offset, routine.source_offset + 4),
        (routine.source_remaining, routine.source_remaining + 4),
        (routine.head, routine.head + 2),
        (routine.head_segment, routine.head_segment + 2),
        (routine.tail, routine.tail + 2),
        (routine.tail_segment, routine.tail_segment + 2),
        (routine.active, routine.active + 2),
        (routine.wrap_limit, routine.wrap_limit + 2),
        (routine.byte_count, routine.byte_count + 2),
        (routine.iteration_count, routine.iteration_count + 2),
    ]
    assert_unchanged_outside(game_before, after[GAME], game_allowed, routine.name)
    assert after[GAME] == bytes(game_expected), (routine.name, name, "game")
    assert_unchanged_outside(
        stack_before,
        after[STACK],
        [(STACK_POINTER - 32, STACK_POINTER + 2)],
        f"{routine.name} stack",
    )
    assert after[STACK][STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] == (
        STACK_SENTINEL
    )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EBX] = with_low16(
        initial[UC_X86_REG_EBX], 0 if body_failed else file_handle
    )
    expected_registers[UC_X86_REG_ECX] = with_low16(
        initial[UC_X86_REG_ECX], 2 if initial_failed else body_count
    )
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    if initial_failed:
        expected_registers[UC_X86_REG_EAX] = with_low16(initial_eax, buffer_end)
    else:
        expected_registers[UC_X86_REG_EAX] = with_low16(
            initial_eax, extent if body_failed else body_count
        )
        expected_registers[UC_X86_REG_EDI] = with_low16(
            initial[UC_X86_REG_EDI], body_start
        )
        expected_registers[UC_X86_REG_ESI] = with_low16(initial[UC_X86_REG_ESI], 2)
        expected_registers[UC_X86_REG_ES] = BUFFER // 16
        expected_registers[UC_X86_REG_EDX] = with_low16(
            initial[UC_X86_REG_EDX], 0 if body_failed else body_start
        )
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags & 1 == int(not success), (routine.name, name, hex(flags))

    result = state_snapshot(after[GAME], routine)
    legacy_row = {
        "name": name,
        "success": success,
        "extent": extent,
        "body_count": None if initial_failed else body_count,
        "entry_start": None if initial_failed else entry_start,
        "calls": calls,
        "result": result,
        "result_carry": flags & 1,
    }
    row = {
        **legacy_row,
        "result_flags": flags & 0xFFFF,
        "buffer_sha256": hashlib.sha256(
            after[BUFFER] + after[BUFFER_WRAP]
        ).hexdigest(),
        "stack_sha256": hashlib.sha256(
            normalized_stack(routine, after[STACK])
        ).hexdigest(),
    }
    canonical = (
        tuple(json.dumps(call, sort_keys=True) for call in calls),
        tuple(result.values()),
        after[BUFFER],
        after[BUFFER_WRAP],
        tuple(registers.items()),
        flags,
        normalized_stack(routine, after[STACK]),
    )
    return row, (legacy_row, canonical)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-executable",
        type=Path,
        default=Path(__file__).parents[1] / "bin/BLOODPRG.EXE",
    )
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_a642_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    commander_vectors = json.loads(args.commander_vectors.read_text())
    assert [case["name"] for case in CASES] == [
        vector["name"] for vector in commander_vectors
    ]

    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, (case, vector) in enumerate(zip(CASES, commander_vectors, strict=True)):
        commander_row, (commander_legacy, commander_result) = execute(
            commander_executable, COMMANDER, case, case_index, commander_edges
        )
        sequel_row, (sequel_legacy, sequel_result) = execute(
            sequel_executable, SEQUEL, case, case_index, sequel_edges
        )
        assert commander_legacy == vector, case["name"]
        assert sequel_legacy == commander_legacy, case["name"]
        assert sequel_result == commander_result, case["name"]
        assert sequel_row == commander_row, case["name"]
        rows.append(sequel_row)

    assert commander_edges == required_edges(COMMANDER)
    assert sequel_edges == required_edges(SEQUEL)
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB initial-entry cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
