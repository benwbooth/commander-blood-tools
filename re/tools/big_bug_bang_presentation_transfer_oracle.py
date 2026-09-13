#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation byte transfer with Commander Blood."""

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
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
BUFFER = 0x30000
BUFFER_WRAP = 0x40000
PAGE_FRAME = 0x50000
CALLBACK = 0x70000
CALLBACK_OFFSET = 0x0100
EXTRA = 0x80000
FS_DATA = 0x90000
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
    banked: int
    ems_handle: int
    xms_handle: int
    page_frame_segment: int
    xms_callback: int
    xms_descriptor: int
    file_handle: int
    source_offset: int
    source_remaining: int
    head: int
    head_segment: int
    queued: int
    ems_move_call: int
    xms_move_call: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA664, 0xA73E),
    body_sha256="6030932e78a0968b041fc7d343a49c3ed79316b8ae5e9d6f1dfa0a261819085a",
    banked=0x0DBC,
    ems_handle=0x0A58,
    xms_handle=0x0A56,
    page_frame_segment=0x0A66,
    xms_callback=0x0A4A,
    xms_descriptor=0x0A6C,
    file_handle=0x0D5B,
    source_offset=0x0D84,
    source_remaining=0x0D88,
    head=0x0D8C,
    head_segment=0x0D8E,
    queued=0x0D9A,
    ems_move_call=0xA6AA,
    xms_move_call=0xA6F0,
    branches={
        0xA669: (0xA66D, 0xA6FC),
        0xA672: (0xA674, 0xA6B5),
        0xA699: (0xA692, 0xA69B),
        0xA6BA: (0xA6BC, 0xA6FC),
        0xA6D1: (0xA6D3, 0xA6D5),
        0xA703: (0xA705, 0xA73D),
        0xA720: (0xA705, 0xA722),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBE4E, 0xBF28),
    body_sha256="6c1962d62e0b6696003b94fba69725f769f488e042abd71340c7a2d92a9ae469",
    banked=0x100A,
    ems_handle=0x0C50,
    xms_handle=0x0C4E,
    page_frame_segment=0x0C5E,
    xms_callback=0x0C42,
    xms_descriptor=0x0C64,
    file_handle=0x0FA9,
    source_offset=0x0FD2,
    source_remaining=0x0FD6,
    head=0x0FDA,
    head_segment=0x0FDC,
    queued=0x0FE8,
    ems_move_call=0xBE94,
    xms_move_call=0xBEDA,
    branches={
        0xBE53: (0xBE57, 0xBEE6),
        0xBE5C: (0xBE5E, 0xBE9F),
        0xBE83: (0xBE7C, 0xBE85),
        0xBEA4: (0xBEA6, 0xBEE6),
        0xBEBB: (0xBEBD, 0xBEBF),
        0xBEED: (0xBEEF, 0xBF27),
        0xBF0A: (0xBEEF, 0xBF0C),
    },
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
        "name": "direct_no_handle",
        "mode": "direct",
        "byte_count": 4,
        "file_handle": 0,
        "head": 0x0100,
        "queued": 0x0020,
        "source_offset": 0x12345678,
        "source_remaining": 0x01020304,
        "reads": (),
    },
    {
        "name": "direct_zero_count",
        "mode": "direct",
        "byte_count": 0,
        "file_handle": 1,
        "head": 0,
        "queued": 0,
        "source_offset": 0,
        "source_remaining": 0,
        "reads": ((0, False, b""),),
    },
    {
        "name": "direct_short_retry",
        "mode": "direct",
        "byte_count": 4,
        "file_handle": 5,
        "head": 0x0220,
        "queued": 0x0030,
        "source_offset": 0x0000FFFF,
        "source_remaining": 0x00010001,
        "reads": ((2, False, b"no"), (4, False, b"DATA")),
    },
    {
        "name": "direct_oversized_result",
        "mode": "direct",
        "byte_count": 2,
        "file_handle": 0x7FFF,
        "head": 0xFFFE,
        "queued": 0xFFFF,
        "source_offset": 0xFFFFFFFF,
        "source_remaining": 1,
        "reads": ((3, True, b"XYZ"),),
    },
    {
        "name": "banked_ems_cross_page",
        "mode": "ems",
        "byte_count": 9,
        "ems_handle": 0x2345,
        "head": 0x1234,
        "queued": 0x4567,
        "source_offset": 0x0000BFFD,
        "source_remaining": 0x00020000,
        "payload": b"EMS-CROSS",
    },
    {
        "name": "banked_ems_page_wrap_zero_count",
        "mode": "ems",
        "byte_count": 0,
        "ems_handle": 0xFFFE,
        "head": 0xAAAA,
        "queued": 0x5555,
        "source_offset": 0xFFFFC000,
        "source_remaining": 0,
        "payload": b"",
    },
    {
        "name": "banked_xms_even",
        "mode": "xms",
        "byte_count": 6,
        "xms_handle": 0x1357,
        "head": 0x0300,
        "queued": 0x0400,
        "source_offset": 0x10203040,
        "source_remaining": 0x55667788,
        "payload": b"XMS123",
    },
    {
        "name": "banked_xms_odd_rounds_move",
        "mode": "xms",
        "byte_count": 5,
        "xms_handle": 0x2468,
        "head": 0xFFFC,
        "queued": 0xFFFE,
        "source_offset": 0xFFFFFFFE,
        "source_remaining": 3,
        "payload": b"odd5!+",
    },
    {
        "name": "banked_without_memory_falls_back_to_file",
        "mode": "fallback",
        "byte_count": 3,
        "file_handle": 9,
        "head": 0x0800,
        "queued": 0x0900,
        "source_offset": 0xAABBCCDD,
        "source_remaining": 0x11223344,
        "reads": ((3, False, b"DOS"),),
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


def machine16(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def with_low16(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def far_return(machine: Uc) -> None:
    stack_pointer = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine16(machine, STACK + stack_pointer)
    return_cs = machine16(machine, STACK + ((stack_pointer + 2) & 0xFFFF))
    machine.reg_write(UC_X86_REG_SP, (stack_pointer + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_CS, return_cs)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, mode: str, stack: bytes) -> bytes:
    result = bytearray(stack)
    if mode == "ems":
        write16(
            result,
            STACK_POINTER - 12,
            COMMANDER.image_address(COMMANDER.ems_move_call + 5),
        )
    elif mode == "xms":
        write16(
            result,
            STACK_POINTER - 12,
            COMMANDER.image_address(COMMANDER.xms_move_call + 4),
        )
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


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert len(body) == 218 and body[-1] == 0xC3
    return executable


def semantic_state(game: bytes, routine: Routine) -> tuple[object, ...]:
    return (
        game[routine.banked],
        read16(game, routine.ems_handle),
        read16(game, routine.xms_handle),
        read16(game, routine.page_frame_segment),
        read16(game, routine.file_handle),
        read32(game, routine.source_offset),
        read32(game, routine.source_remaining),
        read16(game, routine.head),
        read16(game, routine.head_segment),
        read16(game, routine.queued),
        bytes(game[routine.xms_descriptor : routine.xms_descriptor + 16]),
    )


def execute(
    executable: bytes,
    routine: Routine,
    case: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(case["name"])
    mode = str(case["mode"])
    byte_count = int(case["byte_count"])
    file_handle = int(case.get("file_handle", 0))
    ems_handle = int(case.get("ems_handle", 0xFFFF))
    xms_handle = int(case.get("xms_handle", 0xFFFF))
    head = int(case["head"])
    queued = int(case["queued"])
    source_offset = int(case["source_offset"])
    source_remaining = int(case["source_remaining"])
    reads = list(case.get("reads", ()))
    payload = bytes(case.get("payload", b""))

    game_before = seeded_segment(case_index, 17, 0x13)
    data_before = seeded_segment(case_index, 19, 0x25)
    buffer_before = seeded_segment(case_index, 23, 0x37)
    wrap_before = seeded_segment(case_index, 29, 0x49)
    page_before = seeded_segment(case_index, 31, 0x5B)
    extra_before = seeded_segment(case_index, 37, 0x6D)
    fs_before = seeded_segment(case_index, 41, 0x7F)
    stack_before = seeded_segment(case_index, 43, 0x91)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    game_before[routine.banked] = int(mode in {"ems", "xms", "fallback"})
    write16(game_before, routine.ems_handle, ems_handle)
    write16(game_before, routine.xms_handle, xms_handle)
    write16(game_before, routine.page_frame_segment, PAGE_FRAME // 16)
    write16(game_before, routine.file_handle, file_handle)
    write32(game_before, routine.source_offset, source_offset)
    write32(game_before, routine.source_remaining, source_remaining)
    write16(game_before, routine.head, head)
    write16(game_before, routine.head_segment, BUFFER // 16)
    write16(game_before, routine.queued, queued)
    descriptor_seed = bytes(range(0x80, 0x90))
    game_before[routine.xms_descriptor : routine.xms_descriptor + 16] = descriptor_seed
    struct.pack_into(
        "<HH",
        game_before,
        routine.xms_callback,
        CALLBACK_OFFSET,
        CALLBACK // 16,
    )

    if mode == "ems":
        page_offset = source_offset & 0x3FFF
        page_before[page_offset : page_offset + len(payload)] = payload

    game_expected = bytearray(game_before)
    buffer_expected = bytearray(buffer_before)
    wrap_expected = bytearray(wrap_before)
    calls: list[dict[str, object]] = []
    callback_frames: list[dict[str, int | str]] = []
    read_index = 0

    success = not (mode in {"direct", "fallback"} and file_handle < 1)
    if mode in {"ems", "xms"}:
        transferred = byte_count
    elif success:
        transferred = int(reads[-1][0])
    else:
        transferred = None
    increment = transferred or 0
    if success:
        write32(game_expected, routine.source_offset, source_offset + increment)
        write32(game_expected, routine.source_remaining, source_remaining - increment)
        write16(game_expected, routine.head, head + increment)
        write16(game_expected, routine.queued, queued + increment)

    if mode == "xms":
        rounded_length = byte_count + (byte_count & 1)
        descriptor = struct.pack(
            "<IHIHI",
            rounded_length,
            xms_handle,
            source_offset,
            0,
            (BUFFER // 16) << 16 | head,
        )
        game_expected[
            routine.xms_descriptor : routine.xms_descriptor + len(descriptor)
        ] = descriptor

    def write_destination(offset: int, contents: bytes) -> None:
        first_count = min(len(contents), SEGMENT_SIZE - offset)
        buffer_expected[offset : offset + first_count] = contents[:first_count]
        wrap_expected[: len(contents) - first_count] = contents[first_count:]

    if mode in {"ems", "xms"}:
        write_destination(head, payload)
    elif success:
        for _returned, _failed, response_payload in reads:
            write_destination(head, bytes(response_payload))

    initial_eax = 0xA5A50000 | (0x2000 + case_index)
    initial = {
        UC_X86_REG_EAX: initial_eax,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C30000 | byte_count,
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
    expected_module = bytearray(module)
    ems_call = routine.image_address(routine.ems_move_call)
    assert expected_module[ems_call] == 0x9A
    expected_module[ems_call + 1 : ems_call + 5] = struct.pack(
        "<HH", CALLBACK_OFFSET, CALLBACK // 16
    )
    machine.mem_write(0, bytes(expected_module))
    for base, contents in (
        (DATA, data_before),
        (BUFFER, buffer_before),
        (BUFFER_WRAP, wrap_before),
        (PAGE_FRAME, page_before),
        (EXTRA, extra_before),
        (FS_DATA, fs_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    machine.mem_write(CALLBACK + CALLBACK_OFFSET, b"\xCC")
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    previous: int | None = None

    def interrupt(cpu: Uc, number: int, _context) -> None:
        nonlocal read_index
        if number == 0x67:
            ax = cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF
            assert ax >> 8 == 0x44, (routine.name, name, hex(ax))
            calls.append(
                {
                    "call": "ems_map",
                    "handle": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                    "logical_page": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                    "physical_page": ax & 0xFF,
                }
            )
            cpu.reg_write(UC_X86_REG_EAX, with_low16(cpu.reg_read(UC_X86_REG_EAX), ax & 0xFF))
            return

        assert number == 0x21, (routine.name, name, hex(number))
        function = (cpu.reg_read(UC_X86_REG_EAX) >> 8) & 0xFF
        if function == 0x42:
            calls.append(
                {
                    "call": "seek",
                    "handle": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                    "offset_high": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "offset_low": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                }
            )
            cpu.reg_write(
                UC_X86_REG_EAX,
                with_low16(cpu.reg_read(UC_X86_REG_EAX), source_offset),
            )
            cpu.reg_write(
                UC_X86_REG_EDX,
                with_low16(cpu.reg_read(UC_X86_REG_EDX), source_offset >> 16),
            )
            set_carry(cpu, False)
            return

        assert function == 0x3F and read_index < len(reads), (
            routine.name,
            name,
            hex(function),
        )
        returned, failed, response_payload = reads[read_index]
        read_index += 1
        destination_segment = cpu.reg_read(UC_X86_REG_DS)
        destination_offset = cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF
        calls.append(
            {
                "call": "read",
                "handle": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                "requested": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                "destination_segment": destination_segment,
                "destination_offset": destination_offset,
                "returned": int(returned),
                "carry": bool(failed),
            }
        )
        cpu.mem_write(
            destination_segment * 16 + destination_offset, bytes(response_payload)
        )
        cpu.reg_write(
            UC_X86_REG_EAX,
            with_low16(cpu.reg_read(UC_X86_REG_EAX), int(returned)),
        )
        set_carry(cpu, bool(failed))

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == CALLBACK + CALLBACK_OFFSET:
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            callback_frames.append(
                {
                    "kind": mode,
                    "sp": stack_pointer,
                    "return_offset": machine16(cpu, STACK + stack_pointer)
                    + routine.header_size
                    - routine.span[0],
                    "return_cs": machine16(cpu, STACK + stack_pointer + 2),
                }
            )
            if mode == "ems":
                moved = cpu.reg_read(UC_X86_REG_EAX)
                source_segment = cpu.reg_read(UC_X86_REG_DS)
                source_pointer = cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF
                destination_segment = cpu.reg_read(UC_X86_REG_ES)
                destination_offset = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
                calls.append(
                    {
                        "call": "far_memmove",
                        "byte_count": moved,
                        "source_segment": source_segment,
                        "source_offset": source_pointer,
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                copied = bytes(
                    cpu.mem_read(source_segment * 16 + source_pointer, moved)
                )
                cpu.mem_write(destination_segment * 16 + destination_offset, copied)
                flags = cpu.reg_read(UC_X86_REG_EFLAGS)
                cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~(1 << 10))
            elif mode == "xms":
                descriptor = struct.unpack(
                    "<IHIHI",
                    cpu.mem_read(GAME + routine.xms_descriptor, 16),
                )
                destination_offset = descriptor[4] & 0xFFFF
                destination_segment = descriptor[4] >> 16
                calls.append(
                    {
                        "call": "xms_move",
                        "function": cpu.reg_read(UC_X86_REG_EAX),
                        "length": descriptor[0],
                        "source_handle": descriptor[1],
                        "source_offset": descriptor[2],
                        "destination_handle": descriptor[3],
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                cpu.mem_write(
                    destination_segment * 16 + destination_offset,
                    payload[: descriptor[0]],
                )
                cpu.reg_write(UC_X86_REG_EAX, 1)
                set_carry(cpu, False)
            else:
                raise AssertionError((routine.name, name, mode))
            far_return(cpu)
            previous = None
            return

        file_address = address + routine.header_size
        assert routine.span[0] <= file_address < routine.span[1], (
            routine.name,
            name,
            hex(file_address),
        )
        normalized = file_address - routine.span[0]
        if previous is not None and previous + routine.span[0] in routine.branches:
            covered_edges.add((previous, normalized))
        previous = normalized

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=2000)
    assert reached_return, (routine.name, name)

    assert read_index == len(reads), (routine.name, name, read_index, len(reads))
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    after = {
        base: bytes(machine.mem_read(base, SEGMENT_SIZE))
        for base in (DATA, BUFFER, BUFFER_WRAP, PAGE_FRAME, EXTRA, FS_DATA, GAME, STACK)
    }
    for base, expected, label in (
        (DATA, data_before, "DS decoy"),
        (BUFFER, buffer_expected, "queue buffer"),
        (BUFFER_WRAP, wrap_expected, "queue wrap"),
        (PAGE_FRAME, page_before, "EMS page frame"),
        (EXTRA, extra_before, "ES decoy"),
        (FS_DATA, fs_before, "FS decoy"),
    ):
        assert after[base] == bytes(expected), (routine.name, name, label)
    game_allowed = [
        (routine.source_offset, routine.source_offset + 4),
        (routine.source_remaining, routine.source_remaining + 4),
        (routine.head, routine.head + 2),
        (routine.queued, routine.queued + 2),
        (routine.xms_descriptor, routine.xms_descriptor + 16),
    ]
    assert_unchanged_outside(game_before, after[GAME], game_allowed, routine.name)
    assert after[GAME] == bytes(game_expected), (routine.name, name, "game")
    assert_unchanged_outside(
        stack_before,
        after[STACK],
        [(STACK_POINTER - 16, STACK_POINTER + 2)],
        f"{routine.name} stack",
    )
    assert after[STACK][STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] == (
        STACK_SENTINEL
    )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    if success and mode in {"ems", "xms"}:
        expected_registers[UC_X86_REG_EAX] = byte_count
    elif success:
        expected_registers[UC_X86_REG_EAX] = with_low16(initial_eax, increment)
    if mode == "ems":
        expected_registers[UC_X86_REG_EBX] = with_low16(
            initial[UC_X86_REG_EBX], (source_offset >> 14) + 4
        )
        expected_registers[UC_X86_REG_EDX] = with_low16(
            initial[UC_X86_REG_EDX], ems_handle
        )
    elif mode in {"direct", "fallback"}:
        expected_registers[UC_X86_REG_EBX] = with_low16(
            initial[UC_X86_REG_EBX], file_handle
        )
        if success:
            expected_registers[UC_X86_REG_EDX] = with_low16(
                initial[UC_X86_REG_EDX], head
            )
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags & 1 == int(not success), (routine.name, name, hex(flags))

    result = {
        "source_offset": read32(after[GAME], routine.source_offset),
        "source_remaining": read32(after[GAME], routine.source_remaining),
        "head": read16(after[GAME], routine.head),
        "queued": read16(after[GAME], routine.queued),
    }
    descriptor_after = bytes(
        after[GAME][routine.xms_descriptor : routine.xms_descriptor + 16]
    )
    legacy_row = {
        "name": name,
        "mode": mode,
        "success": success,
        "requested": byte_count,
        "transferred": transferred,
        "calls": calls,
        "result": result,
        "result_carry": flags & 1,
        "xms_descriptor": list(descriptor_after) if mode == "xms" else None,
    }
    row = {
        **legacy_row,
        "callback_frames": callback_frames,
        "result_flags": flags & 0xFFFF,
        "buffer_sha256": hashlib.sha256(
            after[BUFFER] + after[BUFFER_WRAP]
        ).hexdigest(),
    }
    canonical = (
        tuple(json.dumps(call, sort_keys=True) for call in calls),
        tuple(json.dumps(frame, sort_keys=True) for frame in callback_frames),
        semantic_state(after[GAME], routine),
        after[BUFFER],
        after[BUFFER_WRAP],
        after[PAGE_FRAME],
        tuple(registers.items()),
        flags,
        normalized_stack(routine, mode, after[STACK]),
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
        default=Path(__file__).parent / "oracle_vectors/func_a664_natural.json",
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

    assert commander_edges == normalized_edges(COMMANDER)
    assert sequel_edges == normalized_edges(SEQUEL)
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB presentation-transfer cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
