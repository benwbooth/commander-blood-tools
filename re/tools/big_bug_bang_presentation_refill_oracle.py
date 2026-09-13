#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation queue refill with Commander Blood."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
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
BUFFER = 0x40000
EXTRA = 0x50000
FS_DATA = 0x60000
GAME = 0x70000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0xF100
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DESCRIPTOR_OFFSET = 0x3000

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
    room: int
    read: int
    transfer: int
    wrap: int
    enqueue: int
    bounds_reset: int
    lookup: int
    copy_words: int
    close: int
    malformed_switch: int
    descriptor_table: int
    file_handle: int
    queue_status: int
    wrap_count: int
    read_wrap_limit: int
    secondary_wrap_limit: int
    range_start: int
    range_remaining: int
    flags: int
    requested: int
    active: int
    source_offset: int
    source_remaining: int
    head: int
    head_segment: int
    tail: int
    tail_segment: int
    wrap_limit: int
    byte_count: int
    pending: int
    rollover_state: int
    buffer_end: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA291, 0xA38E),
    body_sha256="4d36349869310c376a92e5443f1730b71d5d93f788e059fdcef6ca6f4954658b",
    room=0xA3AD,
    read=0xA622,
    transfer=0xA664,
    wrap=0xA38E,
    enqueue=0xA734,
    bounds_reset=0xA744,
    lookup=0x9F80,
    copy_words=0xA7E6,
    close=0xA141,
    malformed_switch=0x9FA2,
    descriptor_table=0x1FB5,
    file_handle=0x0D5B,
    queue_status=0x0D5F,
    wrap_count=0x0D62,
    read_wrap_limit=0x0D64,
    secondary_wrap_limit=0x0D66,
    range_start=0x0D6E,
    range_remaining=0x0D72,
    flags=0x0D76,
    requested=0x0D80,
    active=0x0D82,
    source_offset=0x0D84,
    source_remaining=0x0D88,
    head=0x0D8C,
    head_segment=0x0D8E,
    tail=0x0D90,
    tail_segment=0x0D92,
    wrap_limit=0x0D98,
    byte_count=0x0D9A,
    pending=0x0DA0,
    rollover_state=0x0DAC,
    buffer_end=0x5233,
    branches={
        0xA298: (0xA29A, 0xA2F2),
        0xA2A1: (0xA2A3, 0xA2F2),
        0xA2A6: (0xA2A8, 0xA2D5),
        0xA2AF: (0xA2B1, 0xA291),
        0xA2B6: (0xA2B8, 0xA2C9),
        0xA2C5: (0xA2C7, 0xA2C9),
        0xA2CC: (0xA2CE, 0xA2D5),
        0xA2E7: (0xA2E9, 0xA2F1),
        0xA2F7: (0xA2F9, 0xA2DD),
        0xA2FF: (0xA301, 0xA2D5),
        0xA316: (0xA318, 0xA333),
        0xA31E: (0xA320, 0xA2D6),
        0xA324: (0xA326, 0xA2D6),
        0xA350: (0xA352, 0xA38B),
        0xA384: (0xA386, 0xA355),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBA7B, 0xBB78),
    body_sha256="1a863ffa8035a62206b1a2bb78238c71b342088c084249d089cd0130a5dedc95",
    room=0xBB97,
    read=0xBE0C,
    transfer=0xBE4E,
    wrap=0xBB78,
    enqueue=0xBF1E,
    bounds_reset=0xBF2E,
    lookup=0xB763,
    copy_words=0xBFD0,
    close=0xB924,
    malformed_switch=0xB785,
    descriptor_table=0x2203,
    file_handle=0x0FA9,
    queue_status=0x0FAD,
    wrap_count=0x0FB0,
    read_wrap_limit=0x0FB2,
    secondary_wrap_limit=0x0FB4,
    range_start=0x0FBC,
    range_remaining=0x0FC0,
    flags=0x0FC4,
    requested=0x0FCE,
    active=0x0FD0,
    source_offset=0x0FD2,
    source_remaining=0x0FD6,
    head=0x0FDA,
    head_segment=0x0FDC,
    tail=0x0FDE,
    tail_segment=0x0FE0,
    wrap_limit=0x0FE6,
    byte_count=0x0FE8,
    pending=0x0FEE,
    rollover_state=0x0FFA,
    buffer_end=0x5603,
    branches={
        0xBA82: (0xBA84, 0xBADC),
        0xBA8B: (0xBA8D, 0xBADC),
        0xBA90: (0xBA92, 0xBABF),
        0xBA99: (0xBA9B, 0xBA7B),
        0xBAA0: (0xBAA2, 0xBAB3),
        0xBAAF: (0xBAB1, 0xBAB3),
        0xBAB6: (0xBAB8, 0xBABF),
        0xBAD1: (0xBAD3, 0xBADB),
        0xBAE1: (0xBAE3, 0xBAC7),
        0xBAE9: (0xBAEB, 0xBABF),
        0xBB00: (0xBB02, 0xBB1D),
        0xBB08: (0xBB0A, 0xBAC0),
        0xBB0E: (0xBB10, 0xBAC0),
        0xBB3A: (0xBB3C, 0xBB75),
        0xBB6E: (0xBB70, 0xBB3F),
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

CASES = {
    "pending_uncapped_resource": {"room": [True]},
    "pending_capped_at_next_window": {"room": [True]},
    "pending_below_window_cap": {"room": [True]},
    "pending_capacity_failure": {"room": [False]},
    "next_extent_then_body": {
        "wrap_count": 1,
        "secondary_limit": 3,
        "reads": [(True, 0x20)],
        "room": [True],
    },
    "next_extent_read_failure": {
        "wrap_count": 1,
        "secondary_limit": 3,
        "reads": [(False, 0)],
    },
    "finish_with_queued_entries": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "byte_count": 0x30,
        "state": 0x40,
    },
    "finish_empty_closes_source": {
        "wrap_count": 1,
        "secondary_limit": 3,
        "byte_count": 0,
        "state": 0x80,
    },
    "rollover_capacity_failure": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "room": [False],
    },
    "rollover_existing_descriptor_range": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "range_start": 0x00024000,
        "range_remaining": 0x00000100,
        "reads": [(False, 0)],
        "room": [True],
    },
    "rollover_cached_descriptor": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "requested_id": 2,
        "active_id": 7,
        "descriptor_flags": 0x09,
        "descriptor_start": 0x00156000,
        "descriptor_remaining": 0x00000200,
        "reads": [(False, 0)],
        "room": [True],
    },
    "rollover_synthesizes_four_links": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "range_start": 0x00034000,
        "range_remaining": 0x00000400,
        "link_extents": [6, 8, 10, 12],
        "reads": [(False, 0)],
        "room": [True],
    },
    "rollover_invalid_cache_hits_malformed_suffix": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "requested_id": 2,
        "active_id": 7,
        "descriptor_flags": 0x01,
        "descriptor_start": 0x00006000,
        "descriptor_remaining": 0x00000200,
        "room": [True],
    },
    "rollover_zero_segment_cache_hits_malformed_suffix": {
        "wrap_count": 3,
        "secondary_limit": 3,
        "requested_id": 2,
        "active_id": 7,
        "descriptor_flags": 0x09,
        "descriptor_start": 0x00006000,
        "descriptor_remaining": 0x00000200,
        "room": [True],
    },
}

RETURN_OFFSETS = (0x15, 0x3B, 0x4B, 0x60, 0x6E, 0x76, 0x8A, 0xA1, 0xCE, 0xD7, 0xF3)

EXTRA_VECTORS = [
    {
        "name": "rollover_zero_segment_cache_hits_malformed_suffix",
        "initial_pending": 0,
        "initial_flags": 0x1201,
        "initial_source_offset": 0x1000,
        "initial_source_remaining": 0,
        "link_target_offset": 0x500,
        "result_link_target_offset": 0x500,
        "result": {
            "pending": 0,
            "wrap_count": 0,
            "read_wrap_limit": 3,
            "source_offset": 0x1000,
            "source_remaining": 0,
            "head": 0x100,
            "byte_count": 0x20,
            "state": 0x20,
            "rollover_state": 0,
            "requested_id": 7,
            "flags": 0x1201,
        },
        "calls": [
            {
                "call": "queue_d8c_has_room",
                "request": 0x1000,
                "return_ip": 0xA2FF,
                "has_room": True,
            },
            {"call": "list_d8c_wrap_bounds_reset", "return_ip": 0xA307},
            {
                "call": "lookup_table_1fb5",
                "resource_id": 7,
                "return_ip": 0xA31B,
            },
            {
                "call": "resource_switch_suffix_malformed",
                "return_ip": 0xA2DC,
            },
        ],
        "buffer_sha256": "0427d5d23a409ec7777d52ca40017fee1eb3ab2655af56f59bf3c230a0830ba5",
    }
]


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def read32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def far_word(data: bytes, offset: int) -> int:
    return data[offset & 0xFFFF] | (data[(offset + 1) & 0xFFFF] << 8)


def write_far_word(data: bytearray, offset: int, value: int) -> None:
    data[offset & 0xFFFF] = value & 0xFF
    data[(offset + 1) & 0xFFFF] = (value >> 8) & 0xFF


def machine_word(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def write_low16(machine: Uc, register: int, value: int) -> None:
    current = machine.reg_read(register)
    machine.reg_write(register, (current & 0xFFFF0000) | (value & 0xFFFF))


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine_word(machine, STACK + sp)
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def canonical_return_address(routine: Routine, image_address: int) -> int:
    if image_address == RETURN_IP:
        return RETURN_IP
    file_address = image_address + routine.header_size
    return COMMANDER.span[0] + file_address - routine.span[0]


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    return_map = {
        routine.image_address(routine.span[0] + offset): COMMANDER.image_address(
            COMMANDER.span[0] + offset
        )
        for offset in RETURN_OFFSETS
    }
    for offset in range(STACK_POINTER - 128, STACK_POINTER, 2):
        value = read16(result, offset)
        if value in return_map:
            write16(result, offset, return_map[value])
    return bytes(result)


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
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
    body_digest = hashlib.sha256(executable[slice(*routine.span)]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    return executable


def canonical_state(data: bytes, routine: Routine) -> bytes:
    return struct.pack(
        "<B3H2I3H2I7HB",
        data[routine.queue_status],
        read16(data, routine.wrap_count),
        read16(data, routine.read_wrap_limit),
        read16(data, routine.secondary_wrap_limit),
        read32(data, routine.range_start),
        read32(data, routine.range_remaining),
        read16(data, routine.flags),
        read16(data, routine.requested),
        read16(data, routine.active),
        read32(data, routine.source_offset),
        read32(data, routine.source_remaining),
        read16(data, routine.head),
        read16(data, routine.head_segment),
        read16(data, routine.tail),
        read16(data, routine.tail_segment),
        read16(data, routine.wrap_limit),
        read16(data, routine.byte_count),
        read16(data, routine.pending),
        data[routine.rollover_state],
    )


def canonical_registers(machine: Uc, routine: Routine) -> dict[int, int]:
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    edi = registers[UC_X86_REG_EDI]
    if edi & 0xFFFF == (routine.flags + 1) & 0xFFFF:
        registers[UC_X86_REG_EDI] = (edi & 0xFFFF0000) | (COMMANDER.flags + 1)
    return registers


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes, bytes]:
    name = str(vector["name"])
    case = CASES[name]
    pending = int(vector["initial_pending"])
    flags = int(vector["initial_flags"])
    source_offset = int(vector["initial_source_offset"])
    source_remaining = int(vector["initial_source_remaining"])
    link_target = int(vector["link_target_offset"])
    wrap_count = int(case.get("wrap_count", 1))
    read_limit = 0xAAAA
    secondary_limit = int(case.get("secondary_limit", 3))
    range_start = int(case.get("range_start", 0x00022000))
    range_remaining = int(case.get("range_remaining", 0x00000080))
    requested_id = int(case.get("requested_id", 5))
    active_id = int(case.get("active_id", requested_id))
    head = 0x0100
    tail = 0x0040
    byte_count = int(case.get("byte_count", 0x20))
    wrap_limit = 0xE000
    state = int(case.get("state", 0x20))
    rollover_state = 0x5A
    descriptor_flags = int(case.get("descriptor_flags", 0x09))
    descriptor_start = int(case.get("descriptor_start", 0x00156000))
    descriptor_remaining = int(case.get("descriptor_remaining", 0x00000200))

    data_before = bytearray([0xCC]) * SEGMENT_SIZE
    write16(data_before, routine.file_handle, 0x0033)
    data_before[routine.queue_status] = state
    write16(data_before, routine.wrap_count, wrap_count)
    write16(data_before, routine.read_wrap_limit, read_limit)
    write16(data_before, routine.secondary_wrap_limit, secondary_limit)
    write32(data_before, routine.range_start, range_start)
    write32(data_before, routine.range_remaining, range_remaining)
    write16(data_before, routine.flags, flags)
    write16(data_before, routine.requested, requested_id)
    write16(data_before, routine.active, active_id)
    write32(data_before, routine.source_offset, source_offset)
    write32(data_before, routine.source_remaining, source_remaining)
    write16(data_before, routine.head, head)
    write16(data_before, routine.head_segment, BUFFER // 16)
    write16(data_before, routine.tail, tail)
    write16(data_before, routine.tail_segment, BUFFER // 16)
    write16(data_before, routine.wrap_limit, wrap_limit)
    write16(data_before, routine.byte_count, byte_count)
    write16(data_before, routine.pending, pending)
    data_before[routine.rollover_state] = rollover_state
    write16(data_before, routine.buffer_end, 0xF000)

    table_offset = (routine.descriptor_table + active_id * 4) & 0xFFFF
    write16(data_before, table_offset, DESCRIPTOR_OFFSET)
    write16(data_before, table_offset + 2, 0xFFFF)
    write32(data_before, DESCRIPTOR_OFFSET - 8, descriptor_start)
    write32(data_before, DESCRIPTOR_OFFSET - 4, descriptor_remaining)
    data_before[DESCRIPTOR_OFFSET] = descriptor_flags
    data_before[DESCRIPTOR_OFFSET + 1] = 0x44
    data_before[DESCRIPTOR_OFFSET + 2 : DESCRIPTOR_OFFSET + 14] = b"ROLLOVER.DAT"

    buffer_before = bytearray(
        (offset * 17 + case_index * 29 + 0x31) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    cursor = link_target
    for extent in case.get("link_extents", []):
        write_far_word(buffer_before, cursor, int(extent))
        cursor = (cursor + int(extent)) & 0xFFFF
    extra_before = bytes(
        (offset * 7 + case_index * 13 + 0x53) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    fs_before = bytes(
        (offset * 5 + case_index * 11 + 0x75) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    game_before = bytes(data_before)
    stack_before = bytearray(
        (offset * 19 + case_index * 23 + 0x97) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | case_index,
        UC_X86_REG_EBX: 0xB6B61234,
        UC_X86_REG_ECX: 0xC7C72345,
        UC_X86_REG_EDX: 0xD8D83456,
        UC_X86_REG_ESI: 0xE9E94567,
        UC_X86_REG_EDI: 0xFAFA5678,
        UC_X86_REG_EBP: 0xABCD0000 | link_target,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(BUFFER, bytes(buffer_before))
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(GAME, game_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    room_responses = [bool(value) for value in case.get("room", [])]
    read_responses = [
        (bool(success), int(extent)) for success, extent in case.get("reads", [])
    ]
    room_index = 0
    read_index = 0
    calls: list[dict[str, object]] = []
    reached_return = False
    previous: int | None = None

    def return_address(cpu: Uc) -> int:
        sp = cpu.reg_read(UC_X86_REG_SP)
        return canonical_return_address(routine, machine_word(cpu, STACK + sp))

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous, room_index, read_index
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.room:
            previous = None
            assert room_index < len(room_responses), (routine.name, name, "room")
            has_room = room_responses[room_index]
            room_index += 1
            calls.append(
                {
                    "call": "queue_d8c_has_room",
                    "request": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "return_ip": return_address(cpu),
                    "has_room": has_room,
                }
            )
            write_low16(cpu, UC_X86_REG_EAX, 0xA3AD)
            write_low16(cpu, UC_X86_REG_EBX, 0xBEEF)
            set_carry(cpu, not has_room)
            near_return(cpu)
            return
        if file_address == routine.read:
            previous = None
            assert read_index < len(read_responses), (routine.name, name, "read")
            success, extent = read_responses[read_index]
            read_index += 1
            calls.append({"call": "list_d8c_read", "return_ip": return_address(cpu)})
            if success:
                data_address = DATA
                head_now = (machine_word(cpu, data_address + routine.head) + 2) & 0xFFFF
                byte_count_now = machine_word(cpu, data_address + routine.byte_count)
                source_offset_now = struct.unpack(
                    "<I", cpu.mem_read(data_address + routine.source_offset, 4)
                )[0]
                source_remaining_now = struct.unpack(
                    "<I", cpu.mem_read(data_address + routine.source_remaining, 4)
                )[0]
                cpu.mem_write(data_address + routine.head, struct.pack("<H", head_now))
                cpu.mem_write(
                    data_address + routine.byte_count,
                    struct.pack("<H", (byte_count_now + 2) & 0xFFFF),
                )
                cpu.mem_write(
                    data_address + routine.source_offset,
                    struct.pack("<I", (source_offset_now + 2) & 0xFFFFFFFF),
                )
                cpu.mem_write(
                    data_address + routine.source_remaining,
                    struct.pack("<I", (source_remaining_now - 2) & 0xFFFFFFFF),
                )
                write_low16(cpu, UC_X86_REG_EAX, extent)
                cpu.reg_write(UC_X86_REG_ES, BUFFER // 16)
                write_low16(cpu, UC_X86_REG_ESI, head_now)
            set_carry(cpu, not success)
            near_return(cpu)
            return
        if file_address == routine.transfer:
            previous = None
            pending_after = machine_word(cpu, DATA + routine.pending)
            byte_count_now = cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF
            calls.append(
                {
                    "call": "ems_paged_read",
                    "byte_count": byte_count_now,
                    "pending_after": pending_after,
                    "return_ip": return_address(cpu),
                }
            )
            write_low16(cpu, UC_X86_REG_EAX, byte_count_now)
            set_carry(cpu, False)
            near_return(cpu)
            return
        if file_address == routine.close:
            previous = None
            calls.append({"call": "close_file_d5b", "return_ip": return_address(cpu)})
            near_return(cpu)
            return
        if file_address == routine.malformed_switch:
            previous = None
            calls.append(
                {
                    "call": "resource_switch_suffix_malformed",
                    "return_ip": return_address(cpu),
                }
            )
            near_return(cpu)
            return
        if file_address == routine.wrap:
            previous = None
            calls.append(
                {
                    "call": "queue_d8c_wrap",
                    "extent": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "cursor": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    "return_ip": return_address(cpu),
                }
            )
            return
        if file_address == routine.enqueue:
            previous = None
            calls.append(
                {
                    "call": "queue_d8c_enqueue",
                    "byte_count": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "head": machine_word(cpu, DATA + routine.head),
                    "return_ip": return_address(cpu),
                }
            )
            return
        if file_address == routine.bounds_reset:
            previous = None
            calls.append(
                {
                    "call": "list_d8c_wrap_bounds_reset",
                    "return_ip": return_address(cpu),
                }
            )
            return
        if file_address == routine.lookup:
            previous = None
            calls.append(
                {
                    "call": "lookup_table_1fb5",
                    "resource_id": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "return_ip": return_address(cpu),
                }
            )
            return
        if file_address == routine.copy_words:
            previous = None
            destination = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
            assert destination == routine.range_start
            calls.append(
                {
                    "call": "mem_copy_words",
                    "source": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    "destination": COMMANDER.range_start,
                    "return_ip": return_address(cpu),
                }
            )
            return
        if routine.span[0] <= file_address < routine.span[1]:
            normalized = file_address - routine.span[0]
            if previous is not None and previous + routine.span[0] in routine.branches:
                covered_edges.add((previous, normalized))
            previous = normalized
        else:
            previous = None

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(
        routine.image_address(0xA2AB if routine is COMMANDER else 0xBA95), 0, count=4000
    )
    assert reached_return, (routine.name, name)
    assert room_index == len(room_responses), (routine.name, name, "room responses")
    assert read_index == len(read_responses), (routine.name, name, "read responses")
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    buffer_after = bytes(machine.mem_read(BUFFER, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [
            (routine.queue_status, routine.queue_status + 1),
            (routine.wrap_count, routine.secondary_wrap_limit + 2),
            (routine.range_start, routine.flags + 2),
            (routine.requested, routine.requested + 2),
            (routine.source_offset, routine.source_remaining + 4),
            (routine.head, routine.head + 2),
            (routine.wrap_limit, routine.byte_count + 2),
            (routine.pending, routine.pending + 2),
            (routine.rollover_state, routine.rollover_state + 1),
        ],
        f"{routine.name} {name}",
    )
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == game_before
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 128] == bytes(
        stack_before[: STACK_POINTER - 128]
    )

    result = {
        "pending": read16(data_after, routine.pending),
        "wrap_count": read16(data_after, routine.wrap_count),
        "read_wrap_limit": read16(data_after, routine.read_wrap_limit),
        "source_offset": read32(data_after, routine.source_offset),
        "source_remaining": read32(data_after, routine.source_remaining),
        "head": read16(data_after, routine.head),
        "byte_count": read16(data_after, routine.byte_count),
        "state": data_after[routine.queue_status],
        "rollover_state": data_after[routine.rollover_state],
        "requested_id": read16(data_after, routine.requested),
        "flags": read16(data_after, routine.flags),
    }
    cpu_bp = machine.reg_read(UC_X86_REG_EBP) & 0xFFFF
    row = {
        "name": name,
        "initial_pending": pending,
        "initial_flags": flags,
        "initial_source_offset": source_offset,
        "initial_source_remaining": source_remaining,
        "link_target_offset": link_target,
        "result_link_target_offset": cpu_bp,
        "result": result,
        "calls": calls,
        "buffer_sha256": hashlib.sha256(buffer_after).hexdigest(),
    }
    assert row == vector, (routine.name, name, row, vector)
    assert cpu_bp == int(vector["result_link_target_offset"])
    assert machine.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_DS) == DATA // 16
    assert machine.reg_read(UC_X86_REG_GS) == GAME // 16
    assert machine.reg_read(UC_X86_REG_SS) == STACK // 16
    for register in (
        UC_X86_REG_EAX,
        UC_X86_REG_EBX,
        UC_X86_REG_ECX,
        UC_X86_REG_EDX,
        UC_X86_REG_ESI,
        UC_X86_REG_EDI,
        UC_X86_REG_EBP,
    ):
        assert machine.reg_read(register) & 0xFFFF0000 == initial[register] & 0xFFFF0000

    registers = canonical_registers(machine, routine)
    return (
        row,
        registers,
        machine.reg_read(UC_X86_REG_EFLAGS),
        normalized_stack(routine, stack_after),
        canonical_state(data_after, routine),
        buffer_after,
    )


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
        default=Path(__file__).parent / "oracle_vectors/func_a2ab_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text()) + EXTRA_VECTORS
    assert [str(vector["name"]) for vector in vectors] == list(CASES)

    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(vectors):
        commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        rows.append(sequel_result[0])

    assert commander_edges == normalized_edges(COMMANDER), (
        sorted(normalized_edges(COMMANDER) - commander_edges),
        sorted(commander_edges - normalized_edges(COMMANDER)),
    )
    assert sequel_edges == normalized_edges(SEQUEL), (
        sorted(normalized_edges(SEQUEL) - sequel_edges),
        sorted(sequel_edges - normalized_edges(SEQUEL)),
    )
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB presentation refill cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
