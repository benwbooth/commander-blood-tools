#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang clip playback and stream mixing."""

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

import capstone  # noqa: E402
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM  # noqa: E402
from unicorn import (  # noqa: E402
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_AH,
    UC_X86_REG_AL,
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
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
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

ROOT = Path(__file__).resolve().parents[2]
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_b8cd_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xB8CD, 0xBB9D),
        "sha256": "9c7e53a679ab66f26f0621eb7f70b6da0e348778f2b6183da22c1c5b202ab063",
        "fields": {
            "xms_driver": 0x0A4A,
            "xms_handle": 0x0A5A,
            "ems_handle": 0x0A5C,
            "ems_segment": 0x0A66,
            "xms_request": 0x0A6C,
            "graphics_pointer": 0x0ABC,
            "sound": 0x0ADE,
            "first_buffer": 0x0B89,
            "second_buffer": 0x0B91,
            "pending": 0x0BA0,
            "packed": 0x0BA2,
            "scratch": 0x0BAB,
            "bank_pointer": 0x0BB3,
            "stream_pointer": 0x0BB7,
            "resident_table": 0x0BBF,
            "voice_handle": 0x0C47,
            "stream_table": 0x0C57,
            "play_callback": 0x0CDB,
            "position_callback": 0x0CF3,
        },
        "ultrasound": None,
        "ultrasound_helper": None,
        "ultrasound_return": None,
        "stop_helper": 0xBB9D,
        "stop_return": 0xB8F0,
        "play_returns": (0xB917, 0xB97B, 0xB9DB, 0xBA1A),
        "position_return": 0xBB2D,
        "xms_returns": (0xB9C1, 0xBAC5),
        "memmove": (0x01CE, 0x0B93),
        "memmove_return": 0xB964,
    },
    "sequel": {
        "routine": (0xD05D, 0xD33A),
        "sha256": "96fc545f412c7a9528bd09be164ef268017943dc6ace95e7842220c4917026af",
        "fields": {
            "xms_driver": 0x0C42,
            "xms_handle": 0x0C52,
            "ems_handle": 0x0C54,
            "ems_segment": 0x0C5E,
            "xms_request": 0x0C64,
            "graphics_pointer": 0x0CB4,
            "sound": 0x0CE7,
            "first_buffer": 0x0D93,
            "second_buffer": 0x0D9B,
            "pending": 0x0DAA,
            "packed": 0x0DAC,
            "scratch": 0x0DB5,
            "bank_pointer": 0x0DBD,
            "stream_pointer": 0x0DC1,
            "resident_table": 0x0DC9,
            "voice_handle": 0x0E51,
            "stream_table": 0x0E61,
            "play_callback": 0x0F29,
            "position_callback": 0x0F41,
        },
        "ultrasound": 0x0F1F,
        "ultrasound_helper": 0xDCE5,
        "ultrasound_return": 0xD07D,
        "stop_helper": 0xD33A,
        "stop_return": 0xD08D,
        "play_returns": (0xD0B4, 0xD118, 0xD178, 0xD1B7),
        "position_return": 0xD2CA,
        "xms_returns": (0xD15E, 0xD262),
        "memmove": (0x01E6, 0x0B94),
        "memmove_return": 0xD101,
    },
}

MACHINE_SIZE = 0xF0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x2000
BANK_SEGMENT = 0x4000
STREAM_SEGMENT = 0x5000
GRAPHICS_SEGMENT = 0x6000
EMS_SEGMENT = 0x7000
BUFFER_SEGMENTS = (0x8000, 0x9000)
DATA_SEGMENT = 0xA000
EXTRA_SEGMENT = 0xB000
FS_SEGMENT = 0xC000
CALLBACK_SEGMENT = 0xE000
STREAM_OFFSET = 0x1000
GRAPHICS_OFFSET = 0x0200
STAGING_OFFSET = GRAPHICS_OFFSET + 0x7D00
BUFFER_OFFSET = 0x1000
PLAY_OFFSET = 0x0100
POSITION_OFFSET = 0x0200
XMS_OFFSET = 0x0300
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("69965aa5c33c")
VOICE_HANDLE = 0x2468
EMS_HANDLE = 0x1357
XMS_HANDLE = 0x369C
VECTOR_STREAM_SEGMENT = 0x4800
VECTOR_EMS_SEGMENT = 0x6000

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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


CASES = [
    {
        "name": "sound_disabled",
        "sound": 0,
        "pending": 0,
        "clip": 0,
        "backend": "memory",
    },
    {
        "name": "idle_memory",
        "sound": 1,
        "pending": 1,
        "clip": 2,
        "backend": "memory",
        "memory_length": 31,
    },
    {
        "name": "idle_memory_ignores_bank_offset",
        "sound": 1,
        "pending": 0,
        "clip": 1,
        "backend": "memory",
        "memory_length": 23,
        "bank_offset": 0x1234,
    },
    {
        "name": "idle_ems",
        "sound": 1,
        "pending": 0,
        "clip": 0x8003,
        "backend": "ems",
        "start": 0x1234,
        "length": 97,
    },
    {
        "name": "idle_xms_odd",
        "sound": 1,
        "pending": 0,
        "clip": 0x8004,
        "backend": "xms",
        "start": 0x2011,
        "length": 35,
    },
    {
        "name": "idle_file_short_read",
        "sound": 1,
        "pending": 0,
        "clip": 0x8005,
        "backend": "file",
        "start": 0x3456,
        "length": 41,
        "read_return": 37,
    },
    {
        "name": "mix_memory_spans_buffers",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 10,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 8],
        "position": 8,
    },
    {
        "name": "mix_memory_adds_bank_offset",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 10,
        "bank_offset": 0x0100,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 8],
        "position": 8,
    },
    {
        "name": "mix_memory_packed",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 6,
        "packed": 1,
        "states": [3, 0],
        "buffer_lengths": [10, 10],
        "position": 7,
    },
    {
        "name": "mix_second_buffer_selected",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 7,
        "packed": 0,
        "states": [1, 3],
        "buffer_lengths": [9, 11],
        "position": 9,
    },
    {
        "name": "mix_no_active_buffer",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 9,
        "packed": 0,
        "states": [1, 2],
        "buffer_lengths": [12, 12],
        "position": 4,
    },
    {
        "name": "mix_position_unavailable",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 9,
        "packed": 0,
        "states": [3, 1],
        "buffer_lengths": [12, 12],
        "position": 0xFFFF,
    },
    {
        "name": "mix_ems_cross_page",
        "sound": 1,
        "pending": 2,
        "clip": 0x8006,
        "backend": "ems",
        "start": 0x3FF8,
        "length": 24,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [20, 10],
        "position": 20,
    },
    {
        "name": "mix_xms_spans_buffers",
        "sound": 1,
        "pending": 2,
        "clip": 0x8007,
        "backend": "xms",
        "start": 0x4445,
        "length": 25,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [15, 10],
        "position": 15,
    },
    {
        "name": "mix_file_short_read",
        "sound": 1,
        "pending": 2,
        "clip": 0x8008,
        "backend": "file",
        "start": 0x5500,
        "length": 30,
        "read_return": 22,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 10],
        "position": 12,
    },
]

ULTRASOUND_CASES = (
    {"name": "ultrasound_resident_delegate", "clip": 2, "pending": 2},
    {"name": "ultrasound_streamed_delegate", "clip": 0x8005, "pending": 0},
)

COVERAGE_CASES = (
    {
        "name": "coverage_play_xms_even",
        "sound": 1,
        "pending": 0,
        "clip": 0x8004,
        "backend": "xms",
        "start": 0x2011,
        "length": 34,
    },
    {
        "name": "coverage_mix_xms_even",
        "sound": 1,
        "pending": 2,
        "clip": 0x8007,
        "backend": "xms",
        "start": 0x4445,
        "length": 24,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [15, 10],
        "position": 15,
    },
    {
        "name": "coverage_position_beyond_buffer",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 10,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [8, 8],
        "position": 20,
    },
    {
        "name": "coverage_first_mix_one_byte",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 1,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 8],
        "position": 8,
    },
    {
        "name": "coverage_first_mix_zero_bytes",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 0,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 8],
        "position": 8,
    },
    {
        "name": "coverage_first_mix_exact_fill",
        "sound": 1,
        "pending": 2,
        "clip": 1,
        "backend": "memory",
        "memory_length": 8,
        "packed": 0,
        "states": [3, 0],
        "buffer_lengths": [12, 8],
        "position": 8,
    },
)


def image(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 13 + seed * 29 + 0x47) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def signed16(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


def put_pointer(
    memory: bytearray, offset: int, pointer_offset: int, segment: int
) -> None:
    struct.pack_into("<HH", memory, offset, pointer_offset, segment)


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    result = set()
    for instruction in decoder.disasm(body, start):
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
        immediate = next(
            (
                operand.imm
                for operand in instruction.operands
                if operand.type == X86_OP_IMM
            ),
            None,
        )
        assert immediate is not None
        source = instruction.address - start
        result.add((source, int(immediate) - start))
        result.add((source, instruction.address + instruction.size - start))
    return result


def mix_expected(
    before: list[bytearray],
    source_data: bytes,
    source_bytes: int,
    packed: bool,
    states: list[int],
    lengths: list[int],
    position: int,
) -> tuple[list[bytearray], int, list[dict[str, int]]]:
    after = [bytearray(value) for value in before]
    operations: list[dict[str, int]] = []
    selected = 0 if states[0] == 3 else 1
    if states[selected] != 3 or position == 0xFFFF:
        return after, 0, operations
    other = 1 - selected
    position_delta = (position - lengths[selected]) & 0xFFFF
    if signed16(position_delta) < 0:
        position_delta = (-position_delta) & 0xFFFF
    remaining = source_bytes & 0xFFFF
    source_cursor = 6

    def apply_mix(buffer_index: int, destination_index: int, count: int) -> None:
        nonlocal source_cursor
        original_count = count
        while count != 0:
            sample = source_data[source_cursor]
            if not packed or (count & 1) == 0:
                source_cursor += 1
            destination_value = after[buffer_index][destination_index]
            after[buffer_index][destination_index] = (sample + destination_value) >> 1
            destination_index += 1
            count = (count - 1) & 0xFFFF
        operations.append({"buffer": buffer_index, "bytes": original_count})

    if position_delta < lengths[selected]:
        available = (lengths[selected] - position_delta) & 0xFFFF
        remaining = (remaining - available) & 0xFFFF
        mix_count = available if signed16(remaining) >= 0 else source_bytes
        mix_count = (mix_count - 1) & 0xFFFF
        if signed16(mix_count) > 0:
            apply_mix(selected, 6 + position_delta, mix_count)
    if signed16(remaining) > 0:
        mix_count = min(remaining, lengths[other])
        mix_count = (mix_count - 1) & 0xFFFF
        if signed16(mix_count) > 0:
            apply_mix(other, 6, mix_count)
    return after, source_cursor - 6, operations


def in_ranges(address: int, size: int, ranges: list[tuple[int, int]]) -> bool:
    return any(start <= address and address + size <= stop for start, stop in ranges)


def execute_shared(
    executable: bytes,
    branch_name: str,
    case: dict[str, Any],
    expected_row: dict[str, Any] | None,
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    fields = branch["fields"]
    start, stop = branch["routine"]
    backend = str(case["backend"])
    mixing = bool(int(case["pending"]) & 2)
    clip_value = int(case["clip"])
    streamed_index = clip_value & 0x3FFF
    clip_start = int(case.get("start", 0))
    clip_length = int(case.get("length", 0))
    read_return = int(case.get("read_return", clip_length))
    memory_length = int(case.get("memory_length", 0))
    bank_offset = int(case.get("bank_offset", 0))
    memory_offset = 0x0200 + case_index * 0x0040
    data_length = max(clip_length, memory_length + 6, read_return, 64)
    clip_data = bytes(
        (index * 29 + case_index * 37 + 0x31) & 0xFF for index in range(data_length + 8)
    )
    buffer_before = [
        bytearray(
            (index * (11 + buffer_index * 6) + case_index * 19 + 0x55) & 0xFF
            for index in range(64)
        )
        for buffer_index in range(2)
    ]
    states = list(case.get("states", [0, 0]))
    buffer_lengths = list(case.get("buffer_lengths", [16, 16]))
    position = int(case.get("position", 0))
    packed = bool(int(case.get("packed", 0)))
    active = bool(int(case["sound"]) & 1)

    if mixing and active:
        if backend == "memory":
            source_bytes = memory_length
        elif backend == "file":
            source_bytes = (read_return - 6) & 0xFFFF
        else:
            source_bytes = (clip_length - 6) & 0xFFFF
        if packed:
            source_bytes = (source_bytes * 2) & 0xFFFF
        expected_buffers, consumed_source, mix_operations = mix_expected(
            buffer_before,
            clip_data,
            source_bytes,
            packed,
            states,
            buffer_lengths,
            position,
        )
    else:
        source_bytes = 0
        expected_buffers = [bytearray(value) for value in buffer_before]
        consumed_source = 0
        mix_operations = []

    game = image(case_index + 1)
    bank = image(case_index + 17)
    stream = image(case_index + 33)
    graphics = image(case_index + 49)
    ems = image(case_index + 65)
    buffers = [image(case_index + 81), image(case_index + 97)]
    data = image(case_index + 113)
    extra = image(case_index + 129)
    fs_data = image(case_index + 145)
    callbacks = image(case_index + 161)

    put_pointer(game, fields["xms_driver"], XMS_OFFSET, CALLBACK_SEGMENT)
    struct.pack_into(
        "<H", game, fields["xms_handle"], XMS_HANDLE if backend == "xms" else 0xFFFF
    )
    struct.pack_into(
        "<H", game, fields["ems_handle"], EMS_HANDLE if backend == "ems" else 0xFFFF
    )
    struct.pack_into("<H", game, fields["ems_segment"], EMS_SEGMENT)
    put_pointer(game, fields["graphics_pointer"], GRAPHICS_OFFSET, GRAPHICS_SEGMENT)
    game[fields["sound"]] = int(case["sound"])
    if branch["ultrasound"] is not None:
        game[int(branch["ultrasound"])] = 0
    first_descriptor = struct.pack(
        "<HHHBB", BUFFER_OFFSET, BUFFER_SEGMENTS[0], buffer_lengths[0], states[0], 0xA5
    )
    second_descriptor = struct.pack(
        "<HHHBB", BUFFER_OFFSET, BUFFER_SEGMENTS[1], buffer_lengths[1], states[1], 0x5A
    )
    game[fields["first_buffer"] : fields["first_buffer"] + 8] = first_descriptor
    game[fields["second_buffer"] : fields["second_buffer"] + 8] = second_descriptor
    game[fields["pending"]] = int(case["pending"])
    game[fields["packed"]] = int(case.get("packed", 0))
    game[fields["scratch"] : fields["scratch"] + 6] = struct.pack(
        "<HHH", 0xAAAA, 0xBBBB, 0xCCCC
    )
    put_pointer(game, fields["bank_pointer"], bank_offset, BANK_SEGMENT)
    put_pointer(game, fields["stream_pointer"], STREAM_OFFSET, STREAM_SEGMENT)
    struct.pack_into("<H", game, fields["voice_handle"], VOICE_HANDLE)
    put_pointer(game, fields["play_callback"], PLAY_OFFSET, CALLBACK_SEGMENT)
    put_pointer(game, fields["position_callback"], POSITION_OFFSET, CALLBACK_SEGMENT)
    if backend == "memory":
        table = fields["resident_table"] + clip_value * 4
        struct.pack_into("<HH", game, table, memory_offset, memory_length)
    else:
        table = fields["stream_table"] + streamed_index * 4
        struct.pack_into("<II", game, table, clip_start, clip_start + clip_length)

    source_offset = (
        (memory_offset + bank_offset) & 0xFFFF
        if mixing and backend == "memory"
        else memory_offset
    )
    bank[source_offset : source_offset + len(clip_data)] = clip_data
    if backend == "ems":
        page_offset = clip_start & 0x3FFF
        ems[page_offset : page_offset + len(clip_data)] = clip_data
    for index in range(2):
        buffers[index][BUFFER_OFFSET : BUFFER_OFFSET + 64] = buffer_before[index]
    for offset in (PLAY_OFFSET, POSITION_OFFSET, XMS_OFFSET):
        callbacks[offset] = 0xCB

    initial = {
        UC_X86_REG_EAX: ((0xA1A10000 | clip_value) + case_index * 0x10000) & 0xFFFFFFFF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: GAME_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    game[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    stop_helper = int(branch["stop_helper"])
    memmove_segment, memmove_offset = branch["memmove"]
    memmove_address = int(memmove_segment) * 16 + int(memmove_offset)
    patched_executable = bytearray(executable)
    patched_executable[stop_helper] = 0xCB
    patched_executable[memmove_address] = 0xCB
    if branch["ultrasound_helper"] is not None:
        patched_executable[int(branch["ultrasound_helper"])] = 0xC3

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(patched_executable))
    for segment, contents in (
        (GAME_SEGMENT, game),
        (BANK_SEGMENT, bank),
        (STREAM_SEGMENT, stream),
        (GRAPHICS_SEGMENT, graphics),
        (EMS_SEGMENT, ems),
        (BUFFER_SEGMENTS[0], buffers[0]),
        (BUFFER_SEGMENTS[1], buffers[1]),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (CALLBACK_SEGMENT, callbacks),
    ):
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    body = executable[start:stop]
    expected_edges = conditional_edges(body, start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    reached_return = False
    writes: list[tuple[int, int]] = []
    stop_calls: list[dict[str, int]] = []
    play_calls: list[dict[str, Any]] = []
    position_calls: list[dict[str, int]] = []
    xms_calls: list[dict[str, Any]] = []
    memmove_calls: list[dict[str, Any]] = []
    ems_maps: list[dict[str, int]] = []
    seeks: list[dict[str, int]] = []
    reads: list[dict[str, Any]] = []

    def assert_frame(
        cpu: Uc, valid_returns: tuple[int, ...], expected_sp: int = 0xFEEA
    ) -> None:
        assert cpu.reg_read(UC_X86_REG_SP) == expected_sp, (
            case["name"],
            branch_name,
            hex(cpu.reg_read(UC_X86_REG_SP)),
            hex(expected_sp),
        )
        return_ip, return_cs = struct.unpack(
            "<HH", cpu.mem_read(GAME_SEGMENT * 16 + expected_sp, 4)
        )
        assert return_ip in valid_returns and return_cs == 0, (
            case["name"],
            branch_name,
            return_ip,
            return_cs,
        )

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == stop_helper and cpu.reg_read(UC_X86_REG_CS) == 0:
            assert_frame(cpu, (int(branch["stop_return"]),))
            assert cpu.reg_read(UC_X86_REG_DS) == GAME_SEGMENT
            stop_calls.append({"ax": cpu.reg_read(UC_X86_REG_AX), "ds": GAME_SEGMENT})
            cpu.mem_write(GAME_SEGMENT * 16 + fields["pending"], b"\0")
            previous_source = None
            return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear == CALLBACK_SEGMENT * 16 + PLAY_OFFSET:
            assert_frame(cpu, tuple(int(value) for value in branch["play_returns"]))
            descriptor = struct.unpack(
                "<HHH", cpu.mem_read(GAME_SEGMENT * 16 + fields["scratch"], 6)
            )
            play_calls.append(
                {
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "descriptor": descriptor,
                }
            )
            previous_source = None
            return
        if linear == CALLBACK_SEGMENT * 16 + POSITION_OFFSET:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEE8
            return_ip, return_cs, saved_dx = struct.unpack(
                "<HHH", cpu.mem_read(GAME_SEGMENT * 16 + 0xFEE8, 6)
            )
            assert return_ip == int(branch["position_return"]) and return_cs == 0
            selected_buffer = 0 if states[0] == 3 else 1
            expected_saved_dx = (
                fields["second_buffer"]
                if selected_buffer == 0
                else fields["first_buffer"]
            )
            assert saved_dx == expected_saved_dx
            position_calls.append(
                {"ds": cpu.reg_read(UC_X86_REG_DS), "sp": cpu.reg_read(UC_X86_REG_SP)}
            )
            cpu.reg_write(UC_X86_REG_AX, position)
            previous_source = None
            return
        if linear == CALLBACK_SEGMENT * 16 + XMS_OFFSET:
            assert_frame(
                cpu,
                tuple(int(value) for value in branch["xms_returns"]),
                expected_sp=0xFEE8,
            )
            saved_length = struct.unpack(
                "<H", cpu.mem_read(GAME_SEGMENT * 16 + 0xFEEC, 2)
            )[0]
            assert saved_length == clip_length
            request = struct.unpack(
                "<IHIHI", cpu.mem_read(GAME_SEGMENT * 16 + fields["xms_request"], 16)
            )
            xms_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "request": request,
                }
            )
            destination_linear = request[4]
            transfer = clip_data[: request[0]]
            if len(transfer) < request[0]:
                transfer += bytes(request[0] - len(transfer))
            cpu.mem_write(
                (destination_linear >> 16) * 16 + (destination_linear & 0xFFFF),
                transfer,
            )
            previous_source = None
            return
        if linear == memmove_address:
            assert_frame(cpu, (int(branch["memmove_return"]),))
            byte_count = cpu.reg_read(UC_X86_REG_EAX)
            source_segment = cpu.reg_read(UC_X86_REG_DS)
            source = cpu.reg_read(UC_X86_REG_SI)
            destination_segment = cpu.reg_read(UC_X86_REG_ES)
            destination = cpu.reg_read(UC_X86_REG_DI)
            memmove_calls.append(
                {
                    "byte_count": byte_count,
                    "source": [source, source_segment],
                    "destination": [destination, destination_segment],
                }
            )
            transfer = bytes(cpu.mem_read(source_segment * 16 + source, byte_count))
            cpu.mem_write(destination_segment * 16 + destination, transfer)
            previous_source = None
            return
        assert cpu.reg_read(UC_X86_REG_CS) == 0 and start <= address < stop, (
            case["name"],
            branch_name,
            hex(address),
            hex(linear),
        )
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        flags = cpu.reg_read(UC_X86_REG_EFLAGS)
        if number == 0x67:
            ems_maps.append(
                {
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "logical_page": cpu.reg_read(UC_X86_REG_BX),
                    "handle": cpu.reg_read(UC_X86_REG_DX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                }
            )
            cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
            return
        assert number == 0x21, (case["name"], branch_name, number)
        function = cpu.reg_read(UC_X86_REG_AH)
        if function == 0x42:
            offset = cpu.reg_read(UC_X86_REG_CX) << 16 | cpu.reg_read(UC_X86_REG_DX)
            seeks.append(
                {
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "offset": offset,
                    "origin": cpu.reg_read(UC_X86_REG_AL),
                }
            )
            cpu.reg_write(UC_X86_REG_AX, offset & 0xFFFF)
            cpu.reg_write(UC_X86_REG_DX, offset >> 16)
            cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
        elif function == 0x3F:
            destination_segment = cpu.reg_read(UC_X86_REG_DS)
            destination = cpu.reg_read(UC_X86_REG_DX)
            requested = cpu.reg_read(UC_X86_REG_CX)
            reads.append(
                {
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "destination": [destination, destination_segment],
                    "requested": requested,
                }
            )
            cpu.mem_write(
                destination_segment * 16 + destination, clip_data[:read_return]
            )
            cpu.reg_write(UC_X86_REG_AX, read_return)
            cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
        else:
            raise AssertionError((case["name"], branch_name, "INT 21h", function))

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(start, 0, count=100_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (case["name"], branch_name, "did not return")

    expected_stop_count = int(active and not mixing)
    expected_play_count = expected_stop_count
    assert len(stop_calls) == expected_stop_count, (
        case["name"],
        branch_name,
        stop_calls,
    )
    assert len(play_calls) == expected_play_count, (
        case["name"],
        branch_name,
        play_calls,
    )
    expected_descriptor = None
    if expected_play_count:
        if backend == "memory":
            expected_descriptor = (memory_offset, BANK_SEGMENT, memory_length)
        else:
            expected_descriptor = (
                STREAM_OFFSET,
                STREAM_SEGMENT,
                read_return if backend == "file" else clip_length,
            )
        assert play_calls == [
            {
                "ax": 0,
                "ds": GAME_SEGMENT,
                "si": fields["scratch"],
                "descriptor": expected_descriptor,
            }
        ], (case["name"], branch_name, play_calls)

    expected_position_count = int(active and mixing and 3 in states)
    assert len(position_calls) == expected_position_count
    expected_position_ds = {
        "memory": BANK_SEGMENT,
        "ems": EMS_SEGMENT,
        "xms": GRAPHICS_SEGMENT,
        "file": GRAPHICS_SEGMENT,
    }[backend]
    assert (
        position_calls
        == [{"ds": expected_position_ds, "sp": 0xFEE8}] * expected_position_count
    ), (case["name"], branch_name, position_calls)

    if backend == "ems" and active:
        first_page = clip_start >> 14
        expected_maps = [
            {
                "ax": 0x4400 + physical,
                "logical_page": first_page + physical,
                "handle": EMS_HANDLE,
                "ds": EMS_SEGMENT,
            }
            for physical in range(4)
        ]
        assert ems_maps == expected_maps, (case["name"], branch_name, ems_maps)
    else:
        assert not ems_maps, (case["name"], branch_name, ems_maps)

    expected_memmove_count = int(active and not mixing and backend == "ems")
    assert len(memmove_calls) == expected_memmove_count
    if memmove_calls:
        assert memmove_calls == [
            {
                "byte_count": clip_length,
                "source": [clip_start & 0x3FFF, EMS_SEGMENT],
                "destination": [STREAM_OFFSET, STREAM_SEGMENT],
            }
        ]
    expected_xms_count = int(active and backend == "xms")
    assert len(xms_calls) == expected_xms_count
    if xms_calls:
        destination = (
            (GRAPHICS_SEGMENT << 16) | STAGING_OFFSET
            if mixing
            else (STREAM_SEGMENT << 16) | STREAM_OFFSET
        )
        expected_request = (
            clip_length + (clip_length & 1),
            XMS_HANDLE,
            clip_start,
            0,
            destination,
        )
        assert xms_calls[0]["request"] == expected_request
        assert xms_calls[0]["eax"] == 0x0B00

    expected_file_count = int(active and backend == "file")
    assert len(seeks) == len(reads) == expected_file_count
    if seeks:
        assert seeks == [{"handle": VOICE_HANDLE, "offset": clip_start, "origin": 0}]
        expected_destination = (
            [0x7D00, GRAPHICS_SEGMENT] if mixing else [STREAM_OFFSET, STREAM_SEGMENT]
        )
        assert reads == [
            {
                "handle": VOICE_HANDLE,
                "destination": expected_destination,
                "requested": clip_length,
            }
        ]

    for index in range(2):
        actual = bytes(
            machine.mem_read(BUFFER_SEGMENTS[index] * 16 + BUFFER_OFFSET, 64)
        )
        assert actual == bytes(expected_buffers[index]), (
            case["name"],
            branch_name,
            index,
            first_difference(actual, bytes(expected_buffers[index])),
        )
    if expected_play_count and backend in ("ems", "xms", "file"):
        copied_length = read_return if backend == "file" else clip_length
        actual = bytes(
            machine.mem_read(STREAM_SEGMENT * 16 + STREAM_OFFSET, copied_length)
        )
        assert actual == clip_data[:copied_length]
    pending_after = machine.mem_read(GAME_SEGMENT * 16 + fields["pending"], 1)[0]
    assert pending_after == (0 if expected_stop_count else int(case["pending"]))

    allowed_writes = [
        (GAME_SEGMENT * 16 + 0xFEE0, GAME_SEGMENT * 16 + 0x10000),
        (
            GAME_SEGMENT * 16 + fields["pending"],
            GAME_SEGMENT * 16 + fields["pending"] + 1,
        ),
        (
            GAME_SEGMENT * 16 + fields["scratch"],
            GAME_SEGMENT * 16 + fields["scratch"] + 6,
        ),
        (
            GAME_SEGMENT * 16 + fields["xms_request"],
            GAME_SEGMENT * 16 + fields["xms_request"] + 16,
        ),
        (STREAM_SEGMENT * 16, STREAM_SEGMENT * 16 + SEGMENT_SIZE),
        (GRAPHICS_SEGMENT * 16, GRAPHICS_SEGMENT * 16 + SEGMENT_SIZE),
        (BUFFER_SEGMENTS[0] * 16, BUFFER_SEGMENTS[0] * 16 + SEGMENT_SIZE),
        (BUFFER_SEGMENTS[1] * 16, BUFFER_SEGMENTS[1] * 16 + SEGMENT_SIZE),
    ]
    assert all(in_ranges(address, size, allowed_writes) for address, size in writes), (
        case["name"],
        branch_name,
        [item for item in writes if not in_ranges(*item, allowed_writes)],
    )
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched_executable)
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(data)
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(extra)
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(fs_data)
    assert bytes(machine.mem_read(CALLBACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        callbacks
    )
    assert (
        bytes(machine.mem_read(GAME_SEGMENT * 16 + STACK_POINTER + 4, 6))
        == STACK_SENTINEL
    )

    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    for name in (
        "ebx",
        "ecx",
        "edx",
        "esi",
        "edi",
        "ebp",
        "ds",
        "es",
        "fs",
        "gs",
        "ss",
    ):
        assert (
            actual_registers[name]
            == {
                "ebx": initial[UC_X86_REG_EBX],
                "ecx": initial[UC_X86_REG_ECX],
                "edx": initial[UC_X86_REG_EDX],
                "esi": initial[UC_X86_REG_ESI],
                "edi": initial[UC_X86_REG_EDI],
                "ebp": initial[UC_X86_REG_EBP],
                "ds": DATA_SEGMENT,
                "es": EXTRA_SEGMENT,
                "fs": FS_SEGMENT,
                "gs": GAME_SEGMENT,
                "ss": GAME_SEGMENT,
            }[name]
        ), (case["name"], branch_name, name, actual_registers[name])
    assert actual_registers["sp"] == STACK_POINTER + 4
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    flags_after = {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}

    vector_descriptor = (
        list(expected_descriptor) if expected_descriptor is not None else None
    )
    if vector_descriptor is not None and backend != "memory":
        vector_descriptor[1] = VECTOR_STREAM_SEGMENT
    vector_ems_maps = [dict(entry, ds=VECTOR_EMS_SEGMENT) for entry in ems_maps]
    row = {
        "name": case["name"],
        "clip_index": clip_value,
        "mode": "mix" if mixing else "play",
        "backend": backend,
        "bank_offset": bank_offset,
        "descriptor": vector_descriptor,
        "ems_maps": vector_ems_maps,
        "xms_move_count": len(xms_calls),
        "file_read_count": len(reads),
        "position_calls": len(position_calls),
        "mix_operations": mix_operations,
        "source_bytes": source_bytes,
        "source_data_bytes_consumed": consumed_source,
    }
    if expected_row is not None:
        assert row == expected_row, (case["name"], branch_name, row, expected_row)
    row.update(
        {
            "eax_after": actual_registers["eax"],
            "flags_after": flags_after,
            "return": "far",
        }
    )
    return row, covered_edges


def execute_ultrasound(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES["sequel"]
    fields = branch["fields"]
    start, stop = branch["routine"]
    game = image(case_index + 201)
    data = image(case_index + 217)
    extra = image(case_index + 233)
    game[fields["sound"]] = 1
    game[int(branch["ultrasound"])] = 1
    game[fields["pending"]] = int(case["pending"])
    game[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    before_game = bytes(game)

    helper = int(branch["ultrasound_helper"])
    patched = bytearray(executable)
    patched[helper] = 0xC3
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(patched))
    machine.mem_write(GAME_SEGMENT * 16, bytes(game))
    machine.mem_write(DATA_SEGMENT * 16, bytes(data))
    machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra))
    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | int(case["clip"]),
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: GAME_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected_edges = conditional_edges(executable[start:stop], start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    helper_calls: list[int] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == helper:
            assert cpu.reg_read(UC_X86_REG_AX) == int(case["clip"])
            assert cpu.reg_read(UC_X86_REG_DS) == GAME_SEGMENT
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEC
            return_ip = struct.unpack(
                "<H", cpu.mem_read(GAME_SEGMENT * 16 + 0xFEEC, 2)
            )[0]
            assert return_ip == int(branch["ultrasound_return"])
            helper_calls.append(cpu.reg_read(UC_X86_REG_AX))
            previous_source = None
            return
        assert start <= address < stop
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(start, 0, count=200)
    assert reached_return and helper_calls == [int(case["clip"])]
    actual_game = bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
    for offset in range(0xFEE0):
        assert actual_game[offset] == before_game[offset], (case["name"], hex(offset))
    assert actual_game[STACK_POINTER + 4 : STACK_POINTER + 10] == STACK_SENTINEL
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(data)
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(extra)
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)
    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    expected_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": DATA_SEGMENT,
        "es": EXTRA_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": GAME_SEGMENT,
    }
    assert actual_registers == expected_registers
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    flags_after = {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}
    assert flags_after == {name: False for name in FLAG_MASKS}
    return {
        "name": case["name"],
        "clip_index": int(case["clip"]),
        "pending_after": int(case["pending"]),
        "delegate_calls": helper_calls,
        "flags_after": flags_after,
        "return": "far",
    }, covered_edges


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    commander = COMMANDER_PATH.read_bytes()
    sequel = args.sequel_executable.read_bytes()
    for name, executable, expected in (
        ("Commander", commander, COMMANDER_SHA256),
        ("BBB", sequel, SEQUEL_SHA256),
    ):
        digest = hashlib.sha256(executable).hexdigest()
        if digest != expected:
            raise SystemExit(f"unsupported {name} executable SHA-256 {digest}")
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(CASES) == len(expected_rows) == 15
    assert [case["name"] for case in CASES] == [row["name"] for row in expected_rows]

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, (case, expected_row) in enumerate(
        zip(CASES, expected_rows, strict=True)
    ):
        commander_row, commander_edges = execute_shared(
            commander, "commander", case, expected_row, case_index
        )
        sequel_row, sequel_edges = execute_shared(
            sequel, "sequel", case, expected_row, case_index
        )
        assert commander_row == sequel_row, (case["name"], commander_row, sequel_row)
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    for probe_index, case in enumerate(COVERAGE_CASES, start=len(CASES)):
        commander_row, commander_edges = execute_shared(
            commander, "commander", case, None, probe_index
        )
        sequel_row, sequel_edges = execute_shared(
            sequel, "sequel", case, None, probe_index
        )
        assert commander_row == sequel_row, (case["name"], commander_row, sequel_row)
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)

    ultrasound_rows = []
    for case_index, case in enumerate(ULTRASOUND_CASES):
        row, edges = execute_ultrasound(sequel, case, case_index)
        ultrasound_rows.append(row)
        coverage["sequel"].update(edges)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges, (
            branch_name,
            sorted(expected_edges - coverage[branch_name]),
            sorted(coverage[branch_name] - expected_edges),
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual audio-playback cases, "
        f"{len(COVERAGE_CASES)} dual branch probes, "
        f"{len(ultrasound_rows)} BBB ULTRASND delegates, and "
        f"{len(coverage['sequel'])} conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} branch sites"
    )


if __name__ == "__main__":
    main()
