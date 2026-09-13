#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation resource switch with Commander Blood."""

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
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
BUFFER = 0x40000
DTA = 0x60000
EXTRA = 0x70000
FS_DATA = 0x80000
STACK = 0x90000
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
    requested: int
    active: int
    close: int
    list_init: int
    bounds_init: int
    lookup: int
    descriptor_table: int
    variant: int
    resource_flags: int
    archive_size: int
    buffer_segment: int
    reserved_handle: int
    archive_offset: int
    archive_remaining: int
    embedded_source: int
    source_is_banked: int
    file_handle: int
    queue_status: int
    queue_bounds: int
    source_offset: int
    source_remaining: int
    queue_cursor: int
    queue_segment: int
    queue_tail_cursor: int
    queue_tail_segment: int
    queue_limit_cursor: int
    queue_limit_segment: int
    queue_aux_cursor: int
    queue_aux_segment: int
    entry_metric: int
    resource_ready: int
    buffer_end: int
    palette_dirty: int
    live_palette: int
    render_state: int
    render_update_flags: int
    path_call: int
    initial_read_call: int
    body_read_call: int
    palette_apply: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0x9F8E, 0xA0C3),
    body_sha256="a37330896c8ee762160d2f8d7af9867ec9361d370967044452bf92bea1918ca6",
    requested=0x0D80,
    active=0x0D82,
    close=0xA141,
    list_init=0xA757,
    bounds_init=0xA73E,
    lookup=0x9F80,
    descriptor_table=0x1FB5,
    variant=0x1FB1,
    resource_flags=0x0D76,
    archive_size=0x0A52,
    buffer_segment=0x0A7E,
    reserved_handle=0x0A86,
    archive_offset=0x0A8A,
    archive_remaining=0x0A8E,
    embedded_source=0x0AE2,
    source_is_banked=0x0DBC,
    file_handle=0x0D5B,
    queue_status=0x0D5F,
    queue_bounds=0x0D60,
    source_offset=0x0D84,
    source_remaining=0x0D88,
    queue_cursor=0x0D8C,
    queue_segment=0x0D8E,
    queue_tail_cursor=0x0D90,
    queue_tail_segment=0x0D92,
    queue_limit_cursor=0x0D96,
    queue_limit_segment=0x0D98,
    queue_aux_cursor=0x0D9A,
    queue_aux_segment=0x0DA0,
    entry_metric=0x0DAF,
    resource_ready=0x0DB7,
    buffer_end=0x5233,
    palette_dirty=0x5B55,
    live_palette=0x5251,
    render_state=0x5851,
    render_update_flags=0x2751,
    path_call=0x9FCB,
    initial_read_call=0xA021,
    body_read_call=0xA03E,
    palette_apply=0xA0C3,
    branches={
        0x9FC9: (0x9FCB, 0xA00F),
        0x9FE9: (0x9FEB, 0xA019),
        0xA00B: (0xA00F, 0xA0C0),
        0xA024: (0xA026, 0xA041),
        0xA02B: (0xA02D, 0xA033),
        0xA031: (0xA033, 0xA039),
        0xA049: (0xA04B, 0xA0C0),
        0xA053: (0xA055, 0xA05B),
        0xA059: (0xA05B, 0xA05D),
        0xA06B: (0xA066, 0xA06D),
        0xA074: (0xA076, 0xA078),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB771, 0xB8A6),
    body_sha256="69ec05d8cad88b4d6bb351a2a68d0cd6dda271ee7ee3ce74767fdaab4b4c68f9",
    requested=0x0FCE,
    active=0x0FD0,
    close=0xB924,
    list_init=0xBF41,
    bounds_init=0xBF28,
    lookup=0xB763,
    descriptor_table=0x2203,
    variant=0x21FF,
    resource_flags=0x0FC4,
    archive_size=0x0C4A,
    buffer_segment=0x0C76,
    reserved_handle=0x0C7E,
    archive_offset=0x0C82,
    archive_remaining=0x0C86,
    embedded_source=0x0CEB,
    source_is_banked=0x100A,
    file_handle=0x0FA9,
    queue_status=0x0FAD,
    queue_bounds=0x0FAE,
    source_offset=0x0FD2,
    source_remaining=0x0FD6,
    queue_cursor=0x0FDA,
    queue_segment=0x0FDC,
    queue_tail_cursor=0x0FDE,
    queue_tail_segment=0x0FE0,
    queue_limit_cursor=0x0FE4,
    queue_limit_segment=0x0FE6,
    queue_aux_cursor=0x0FE8,
    queue_aux_segment=0x0FEE,
    entry_metric=0x0FFD,
    resource_ready=0x1005,
    buffer_end=0x5603,
    palette_dirty=0x5F25,
    live_palette=0x5621,
    render_state=0x5C21,
    render_update_flags=0x29DF,
    path_call=0xB7AE,
    initial_read_call=0xB804,
    body_read_call=0xB821,
    palette_apply=0xB8A6,
    branches={
        0xB7AC: (0xB7AE, 0xB7F2),
        0xB7CC: (0xB7CE, 0xB7FC),
        0xB7EE: (0xB7F2, 0xB8A3),
        0xB807: (0xB809, 0xB824),
        0xB80E: (0xB810, 0xB816),
        0xB814: (0xB816, 0xB81C),
        0xB82C: (0xB82E, 0xB8A3),
        0xB836: (0xB838, 0xB83E),
        0xB83C: (0xB83E, 0xB840),
        0xB84E: (0xB849, 0xB850),
        0xB857: (0xB859, 0xB85B),
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

CASES: tuple[dict[str, object], ...] = (
    {
        "name": "banked_primary_bounds",
        "mode": "banked",
        "resource_id": 2,
        "record_flags": 0x00,
        "variant": 0x0C,
        "inner_skip": 6,
        "palette_blocks": [],
        "padding": 2,
        "render_copy": False,
        "archive_size": 0x00123456,
        "primary_relative": 0x00000121,
        "alternate_relative": 0x00000341,
        "index_relative": 0x00000561,
        "read_failure": None,
    },
    {
        "name": "embedded_alternate_bounds",
        "mode": "embedded",
        "resource_id": 5,
        "record_flags": 0x04,
        "variant": 0x07,
        "inner_skip": 10,
        "palette_blocks": [(3, bytes([1, 2, 3]))],
        "padding": 4,
        "render_copy": True,
        "archive_offset": 0x12345670,
        "archive_remaining": 0x01020304,
        "path_handle": 0x0042,
        "primary_relative": 0x00001121,
        "alternate_relative": 0x00002231,
        "index_relative": 0x00003341,
        "read_failure": None,
    },
    {
        "name": "external_file_success",
        "mode": "external",
        "resource_id": 8,
        "record_flags": 0x00,
        "variant": 0x09,
        "inner_skip": 14,
        "palette_blocks": [
            (1, bytes([0x3F, 0x20, 0x10, 0x08, 0x04, 0x02])),
            (9, bytes()),
        ],
        "padding": 1,
        "render_copy": False,
        "file_size": 0x00020000,
        "open_handle": 0x0033,
        "old_handle": 0x0022,
        "primary_relative": 0x00000211,
        "alternate_relative": 0x00000421,
        "index_relative": 0x00000631,
        "read_failure": None,
    },
    {
        "name": "wrapped_inner_cursor",
        "mode": "banked",
        "resource_id": 11,
        "record_flags": 0x00,
        "variant": 0x02,
        "inner_skip": 0xFFFF,
        "palette_blocks": [],
        "padding": 3,
        "render_copy": False,
        "archive_size": 0x00018000,
        "primary_relative": 0x00000031,
        "alternate_relative": 0x00000051,
        "index_relative": 0x00000071,
        "read_failure": None,
    },
    {
        "name": "external_open_failure",
        "mode": "external",
        "resource_id": 13,
        "record_flags": 0x04,
        "variant": 0x05,
        "file_size": 0x00009999,
        "open_error": 2,
        "path_handle": 0x1357,
        "archive_offset": 0x11112222,
        "archive_remaining": 0x33334444,
        "read_failure": "open",
    },
    {
        "name": "initial_read_failure",
        "mode": "banked",
        "resource_id": 17,
        "record_flags": 0x00,
        "variant": 0x03,
        "archive_size": 0x00012345,
        "read_failure": "initial",
    },
    {
        "name": "body_read_failure",
        "mode": "embedded",
        "resource_id": 19,
        "record_flags": 0x00,
        "variant": 0x01,
        "archive_offset": 0x01020304,
        "archive_remaining": 0x00054321,
        "path_handle": 0x0055,
        "read_failure": "body",
    },
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def read32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def case_buffer(case: dict[str, object]) -> tuple[bytearray, int, int]:
    palette_stream = bytearray()
    for palette_start, payload in case.get("palette_blocks", []):
        payload = bytes(payload)
        palette_stream.extend((int(palette_start), len(payload) // 3))
        palette_stream.extend(payload)
    palette_stream.extend(b"\xff\xff")
    read_extent = len(palette_stream) + 32
    buffer = seeded_segment(0, 23, 0x35)
    if case["read_failure"] in {"open", "initial", "body"}:
        return buffer, read_extent, 0

    inner_skip = int(case["inner_skip"])
    write16(buffer, 0, inner_skip)
    buffer_end = int(case.get("buffer_end", 0xFFFE))
    palette_offset = 0 if 2 + inner_skip > buffer_end else 2
    buffer[palette_offset : palette_offset + len(palette_stream)] = palette_stream
    metadata_offset = (palette_offset + len(palette_stream)) & 0xFFFF
    padding = int(case["padding"])
    buffer[metadata_offset : metadata_offset + padding] = b"\xff" * padding
    metadata_offset = (metadata_offset + padding) & 0xFFFF
    write32(buffer, metadata_offset, int(case["primary_relative"]))
    write32(buffer, metadata_offset + 0x10, int(case["alternate_relative"]))
    write32(buffer, metadata_offset + 6 * 4, int(case["index_relative"]))
    return buffer, read_extent, metadata_offset


def initialize_data(
    routine: Routine, case: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray, int, int]:
    data = seeded_segment(case_index, 17, 0x31)
    expected_palette = bytearray((index * 7 + 3) & 0x3F for index in range(768))
    initial_render = bytes((index * 11 + 5) & 0xFF for index in range(0x180))
    for palette_start, payload in case.get("palette_blocks", []):
        payload = bytes(payload)
        destination = int(palette_start) * 3
        expected_palette[destination : destination + len(payload)] = payload

    resource_id = int(case["resource_id"])
    record_offset = 0x3000
    table_offset = (routine.descriptor_table + resource_id * 4) & 0xFFFF
    data[table_offset : table_offset + 4] = struct.pack(
        "<HH", record_offset, 0xA000 + resource_id
    )
    data[record_offset : record_offset + 15] = (
        bytes([int(case["record_flags"]), 0xA5]) + b"RESOURCE.DAT\0"
    )

    write32(data, routine.archive_size, int(case.get("archive_size", 0x01010101)))
    write16(data, routine.buffer_segment, BUFFER // 16)
    write16(data, routine.reserved_handle, 0x7FFF)
    write32(data, routine.archive_offset, int(case.get("archive_offset", 0x55667788)))
    write32(
        data,
        routine.archive_remaining,
        int(case.get("archive_remaining", 0x11223344)),
    )
    data[routine.embedded_source] = 0xA5
    data[routine.source_is_banked] = int(case["mode"] == "banked")
    write16(data, routine.file_handle, int(case.get("old_handle", 0)))
    data[routine.queue_status] = 0xA5
    data[routine.queue_bounds : routine.queue_bounds + 8] = struct.pack(
        "<4H", 9, 8, 7, 6
    )
    for offset, size in (
        (routine.resource_flags - 8, 8),
        (routine.resource_flags + 2, 8),
        (routine.source_offset, 8),
        (routine.queue_cursor, 18),
        (routine.queue_aux_segment, 2),
        (routine.entry_metric, 2),
        (routine.resource_ready, 1),
        (routine.requested, 4),
        (routine.resource_flags, 2),
    ):
        data[offset : offset + size] = bytes(size)
    data[routine.variant] = int(case["variant"])
    write16(data, routine.buffer_end, int(case.get("buffer_end", 0xFFFE)))
    data[routine.palette_dirty] = 0x5A
    data[routine.live_palette : routine.live_palette + 768] = bytes(
        (index * 7 + 3) & 0x3F for index in range(768)
    )
    data[routine.render_state : routine.render_state + 0x180] = initial_render
    data[routine.render_update_flags] = 0 if case.get("render_copy") else 1
    return data, expected_palette, record_offset, table_offset


def allowed_data_ranges(routine: Routine, record_offset: int) -> list[tuple[int, int]]:
    fields = [
        (routine.resource_flags - 8, 8),
        (routine.requested, 4),
        (routine.resource_flags, 2),
        (routine.resource_flags + 2, 8),
        (routine.file_handle, 2),
        (routine.queue_status, 9),
        (routine.source_offset, 8),
        (routine.queue_cursor, 18),
        (routine.queue_aux_segment, 2),
        (routine.entry_metric, 2),
        (routine.resource_ready, 1),
        (routine.embedded_source, 1),
        (routine.palette_dirty, 1),
        (routine.live_palette, 768),
        (routine.render_state, 0x180),
        (record_offset + 1, 1),
    ]
    return [(start, start + size) for start, size in fields]


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
) -> None:
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or any(start <= offset < end for start, end in allowed):
            continue
        raise AssertionError(f"{label} changed outside owned state at {offset:#x}")


def canonical_data(routine: Routine, data: bytes, record_offset: int) -> bytes:
    fields = (
        (routine.resource_flags - 8, 8),
        (routine.requested, 4),
        (routine.resource_flags, 2),
        (routine.resource_flags + 2, 8),
        (routine.file_handle, 2),
        (routine.queue_status, 9),
        (routine.source_offset, 8),
        (routine.queue_cursor, 18),
        (routine.entry_metric, 2),
        (routine.resource_ready, 1),
        (routine.embedded_source, 1),
        (routine.palette_dirty, 1),
        (routine.live_palette, 768),
        (routine.render_state, 0x180),
        (record_offset + 1, 1),
    )
    return b"".join(data[start : start + size] for start, size in fields)


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    normalized = bytearray(stack)
    if routine is SEQUEL:
        return_addresses = {
            SEQUEL.image_address(sequel): COMMANDER.image_address(commander)
            for commander, sequel in (
                (0x9F96, 0xB779),
                (0x9F9A, 0xB77D),
                (0x9FA2, 0xB785),
                (0x9FAB, 0xB78E),
                (0xA065, 0xB848),
                (0xA0F1, 0xB8D4),
                (0xA15C, 0xB93F),
            )
        }
        for offset in range(0xFEC0, STACK_POINTER, 2):
            word = read16(normalized, offset)
            if word in return_addresses:
                write16(normalized, offset, return_addresses[word])
    return bytes(normalized)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    return {
        (source - routine.span[0], destination - routine.span[0])
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def canonical_registers(routine: Routine, machine: Uc) -> dict[int, int]:
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    edi = registers[UC_X86_REG_EDI]
    di = edi & 0xFFFF
    for source, target, size in (
        (routine.live_palette, COMMANDER.live_palette, 768),
        (routine.render_state, COMMANDER.render_state, 0x180),
    ):
        if source <= di <= source + size:
            registers[UC_X86_REG_EDI] = (edi & 0xFFFF0000) | (target + di - source)
            break
    return registers


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(case["name"])
    assert name == vector["name"]
    data_before, expected_palette, record_offset, _table_offset = initialize_data(
        routine, case, case_index
    )
    buffer_before, read_extent, metadata_offset = case_buffer(case)
    dta_before = seeded_segment(case_index, 11, 0x53)
    extra_before = bytes(seeded_segment(case_index, 7, 0x75))
    fs_before = bytes(seeded_segment(case_index, 5, 0x97))
    stack_before = seeded_segment(case_index, 3, 0xB9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    resource_id = int(case["resource_id"])
    initial = {
        UC_X86_REG_EAX: 0x89AB0000 | resource_id,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E55555,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: DATA // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xA0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    for call, size in (
        (routine.path_call, 5),
        (routine.initial_read_call, 3),
        (routine.body_read_call, 3),
    ):
        start = routine.image_address(call)
        expected_module[start : start + size] = b"\x90" * size
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(BUFFER, bytes(buffer_before))
    machine.mem_write(DTA, bytes(dta_before))
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, int | str]] = []
    pending_branch: int | None = None
    reached_return = False
    mode = str(case["mode"])
    read_failure = case["read_failure"]
    path_handle = int(case.get("path_handle", 0x2468))
    file_size = int(case.get("file_size", 0x00020000))
    open_handle = int(case.get("open_handle", 0x0033))

    def read_data32(offset: int) -> int:
        return struct.unpack("<I", machine.mem_read(DATA + offset, 4))[0]

    def write_data32(offset: int, value: int) -> None:
        machine.mem_write(DATA + offset, struct.pack("<I", value & 0xFFFFFFFF))

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal pending_branch, reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_offset = address + routine.header_size
        if pending_branch is not None:
            covered_edges.add(
                (pending_branch - routine.span[0], file_offset - routine.span[0])
            )
            pending_branch = None
        if file_offset in routine.branches:
            pending_branch = file_offset
        if file_offset == routine.path_call:
            assert cpu.reg_read(UC_X86_REG_DX) == record_offset + 2
            calls.append({"call": "path", "filename": record_offset + 2})
            cpu.reg_write(UC_X86_REG_BX, path_handle)
            cpu.mem_write(DATA + routine.embedded_source, bytes([mode == "embedded"]))
        elif file_offset == routine.initial_read_call:
            calls.append({"call": "initial_read", "bytes": 2})
            if read_failure == "initial":
                set_carry(cpu, True)
                return
            write_data32(routine.source_offset, read_data32(routine.source_offset) + 2)
            write_data32(
                routine.source_remaining, read_data32(routine.source_remaining) - 2
            )
            cpu.reg_write(UC_X86_REG_AX, read_extent)
            cpu.reg_write(UC_X86_REG_ES, BUFFER // 16)
            cpu.reg_write(UC_X86_REG_SI, int(case.get("initial_si", 0)))
            set_carry(cpu, False)
        elif file_offset == routine.body_read_call:
            byte_count = cpu.reg_read(UC_X86_REG_CX)
            calls.append({"call": "body_read", "bytes": byte_count})
            if read_failure == "body":
                set_carry(cpu, True)
                return
            write_data32(
                routine.source_offset,
                read_data32(routine.source_offset) + byte_count,
            )
            write_data32(
                routine.source_remaining,
                read_data32(routine.source_remaining) - byte_count,
            )
            cpu.reg_write(UC_X86_REG_AX, byte_count)
            set_carry(cpu, False)

    def interrupt(cpu: Uc, number: int, _context) -> None:
        assert number == 0x21, (routine.name, name, number)
        function = cpu.reg_read(UC_X86_REG_AX) >> 8
        if function == 0x3E:
            calls.append({"call": "close", "handle": cpu.reg_read(UC_X86_REG_BX)})
            set_carry(cpu, False)
        elif function == 0x2F:
            calls.append({"call": "get_dta"})
            cpu.reg_write(UC_X86_REG_ES, DTA // 16)
            cpu.reg_write(UC_X86_REG_BX, 0x0200)
        elif function == 0x4E:
            calls.append({"call": "find_first"})
            cpu.mem_write(DTA + 0x0200 + 0x1A, struct.pack("<I", file_size))
            set_carry(cpu, case_index % 2 == 0)
        elif function == 0x3D:
            calls.append({"call": "open"})
            if read_failure == "open":
                cpu.reg_write(UC_X86_REG_AX, int(case.get("open_error", 0)))
                set_carry(cpu, True)
            else:
                cpu.reg_write(UC_X86_REG_AX, open_handle)
                set_carry(cpu, False)
        else:
            raise AssertionError(
                f"{routine.name} {name} invoked unexpected DOS function {function:#x}"
            )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=20000)
    assert reached_return, (routine.name, name)

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    buffer_after = bytes(machine.mem_read(BUFFER, SEGMENT_SIZE))
    dta_after = bytes(machine.mem_read(DTA, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)
    assert buffer_after == bytes(buffer_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        allowed_data_ranges(routine, record_offset),
        f"{routine.name} {name} data",
    )
    assert_unchanged_outside(
        bytes(dta_before),
        dta_after,
        [(0x021A, 0x021E)],
        f"{routine.name} {name} DTA",
    )

    expected_success = read_failure is None
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert bool(flags & 1) is (not expected_success)
    assert machine.reg_read(UC_X86_REG_EAX) == initial[UC_X86_REG_EAX]
    assert machine.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert read16(data_after, routine.requested) == resource_id
    assert read16(data_after, routine.active) == resource_id
    expected_flags = int(case["record_flags"]) | int(case["variant"]) << 8
    assert read16(data_after, routine.resource_flags) == expected_flags
    assert data_after[record_offset + 1] == int(case["variant"])
    assert data_after[routine.queue_status] == 0
    assert struct.unpack_from("<4H", data_after, routine.queue_bounds) == (
        0,
        0,
        0xFFFF,
        0xFFFF,
    )

    if expected_success:
        assert data_after[routine.resource_ready] == 0xFF
        assert data_after[routine.live_palette : routine.live_palette + 768] == bytes(
            expected_palette
        )
        expected_render = (
            bytes(expected_palette[:0x180])
            if case.get("render_copy")
            else bytes((index * 11 + 5) & 0xFF for index in range(0x180))
        )
        assert (
            data_after[routine.render_state : routine.render_state + 0x180]
            == expected_render
        )

    row = {
        "name": name,
        "mode": mode,
        "resource_id": resource_id,
        "record_flags": expected_flags,
        "success": expected_success,
        "calls": calls,
        "source_offset": read32(data_after, routine.source_offset),
        "source_remaining": read32(data_after, routine.source_remaining),
        "file_handle": read16(data_after, routine.file_handle),
        "entry_metric": read16(data_after, routine.entry_metric),
        "metadata_offset": metadata_offset,
        "range_start": read32(data_after, routine.resource_flags - 8),
        "range_remaining": read32(data_after, routine.resource_flags - 4),
        "index_start": read32(data_after, routine.resource_flags + 2),
        "index_remaining": read32(data_after, routine.resource_flags + 6),
        "carry": flags & 1,
        "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
    }
    registers = canonical_registers(routine, machine)
    return (
        row,
        registers,
        flags,
        canonical_data(routine, data_after, record_offset),
        normalized_stack(routine, stack_after),
    )


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
    assert executable[routine.span[1] - 1] == 0xC3
    return executable


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
        default=Path(__file__).parent / "oracle_vectors/func_9f8e_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    assert len(vectors) == len(CASES)
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, (vector, case) in enumerate(zip(vectors, CASES, strict=True)):
        commander_result = execute(
            commander_executable,
            COMMANDER,
            vector,
            case,
            case_index,
            commander_edges,
        )
        sequel_result = execute(
            sequel_executable,
            SEQUEL,
            vector,
            case,
            case_index,
            sequel_edges,
        )
        (
            commander_row,
            commander_registers,
            commander_flags,
            commander_data,
            commander_stack,
        ) = commander_result
        sequel_row, sequel_registers, sequel_flags, sequel_data, sequel_stack = (
            sequel_result
        )
        assert commander_row == vector, (vector["name"], commander_row, vector)
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, (
            vector["name"],
            commander_registers,
            sequel_registers,
        )
        assert sequel_flags == commander_flags, vector["name"]
        assert sequel_data == commander_data, vector["name"]
        assert sequel_stack == commander_stack, (
            vector["name"],
            [
                (offset, old, new)
                for offset, (old, new) in enumerate(
                    zip(commander_stack, sequel_stack, strict=True)
                )
                if old != new
            ][:32],
        )
        rows.append(sequel_row)

    probes = []
    initial_carry = dict(CASES[0])
    initial_carry.update(name="probe_initial_cursor_add_carry", initial_si=0xFFF0)
    probes.append(initial_carry)
    cursor_limit = dict(CASES[0])
    cursor_limit.update(
        name="probe_cursor_limit_resets",
        buffer_end=7,
        inner_skip=8,
        palette_blocks=[(8, bytes())],
    )
    probes.append(cursor_limit)
    for probe_index, case in enumerate(probes, start=len(CASES)):
        vector = {"name": case["name"]}
        commander_result = execute(
            commander_executable,
            COMMANDER,
            vector,
            case,
            probe_index,
            commander_edges,
        )
        sequel_result = execute(
            sequel_executable,
            SEQUEL,
            vector,
            case,
            probe_index,
            sequel_edges,
        )
        assert commander_result == sequel_result, case["name"]

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
        f"verified {len(rows)} BBB resource-switch cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
