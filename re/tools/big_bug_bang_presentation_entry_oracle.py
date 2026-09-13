#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation entry parser with Commander Blood."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
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
SOURCE = 0x32000
LINKED = 0x44000
STORAGE = 0x56000
DEFAULT_STORAGE = 0x68000
GAME = 0x7A000
STACK = 0x8C000
FS_DATA = 0x9E000
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
    flag_helper: int
    flag_helper_sha256: str
    decode: int
    consume: int
    reset_word: int
    compressed: int
    data_sound: int
    data_palette: int
    game_sound: int
    game_palette: int
    state_flag: int
    default_storage: int
    buffer_end: int
    active_offset: int
    active_segment: int
    layout: int
    row_mode: int
    back_buffer_mode: int
    skip_present: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA552, 0xA622),
    body_sha256="3783f26b33e432594b2256290f2c67a208dce8cfaedb64e2415705212c3c6d4e",
    flag_helper=0xA634,
    flag_helper_sha256="70ed47cfe0a09240e61c1dda4ba1daf5f24019813d06e2f4d0151a221c046432",
    decode=0xA82C,
    consume=0xA3D0,
    reset_word=0x0AA0,
    compressed=0x0DBA,
    data_sound=0x0D9C,
    data_palette=0x0D9E,
    game_sound=0x0D9C,
    game_palette=0x0D9E,
    state_flag=0x0B17,
    default_storage=0x0ABE,
    buffer_end=0x5233,
    active_offset=0x0D94,
    active_segment=0x0D96,
    layout=0x0DA4,
    row_mode=0x0DA6,
    back_buffer_mode=0x0DB9,
    skip_present=0x0DBB,
    branches={
        0xA56D: (0xA56F, 0xA576),
        0xA574: (0xA576, 0xA578),
        0xA57D: (0xA57F, 0xA592),
        0xA582: (0xA584, 0xA589),
        0xA595: (0xA597, 0xA5A5),
        0xA5A8: (0xA5AA, 0xA5BC),
        0xA5B7: (0xA5B9, 0xA5BC),
        0xA5C5: (0xA5C7, 0xA5CC),
        0xA5E8: (0xA5EA, 0xA610),
        0xA5EE: (0xA5F0, 0xA612),
        0xA5F6: (0xA5F8, 0xA60D),
        0xA5FE: (0xA600, 0xA60D),
        0xA603: (0xA605, 0xA60D),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBD3C, 0xBE0C),
    body_sha256="6de81969ae473043c5ffcc7d6c9a68e37a1ceb75c7e92cddbe48b4bacb828aa0",
    flag_helper=0xBE1E,
    flag_helper_sha256="400200255be1b4a36cabd7f21fc8babb33e4c2b1b56a7da79e725b33a0496020",
    decode=0xC016,
    consume=0xBBBA,
    reset_word=0x0C98,
    compressed=0x1008,
    data_sound=0x0FEA,
    data_palette=0x0FEC,
    game_sound=0x0FEA,
    game_palette=0x0FEC,
    state_flag=0x0D21,
    default_storage=0x0CB6,
    buffer_end=0x5603,
    active_offset=0x0FE2,
    active_segment=0x0FE4,
    layout=0x0FF2,
    row_mode=0x0FF4,
    back_buffer_mode=0x1007,
    skip_present=0x1009,
    branches={
        0xBD57: (0xBD59, 0xBD60),
        0xBD5E: (0xBD60, 0xBD62),
        0xBD67: (0xBD69, 0xBD7C),
        0xBD6C: (0xBD6E, 0xBD73),
        0xBD7F: (0xBD81, 0xBD8F),
        0xBD92: (0xBD94, 0xBDA6),
        0xBDA1: (0xBDA3, 0xBDA6),
        0xBDAF: (0xBDB1, 0xBDB6),
        0xBDD2: (0xBDD4, 0xBDFA),
        0xBDD8: (0xBDDA, 0xBDFC),
        0xBDE0: (0xBDE2, 0xBDF7),
        0xBDE8: (0xBDEA, 0xBDF7),
        0xBDED: (0xBDEF, 0xBDF7),
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


def seeded_segment(
    case_index: int, multiplier: int, shift_multiplier: int, case_multiplier: int
) -> bytearray:
    return bytearray(
        (
            offset * multiplier
            + (offset >> 8) * shift_multiplier
            + case_index * case_multiplier
        )
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes | bytearray, offset: int) -> int:
    return data[offset & 0xFFFF] | (data[(offset + 1) & 0xFFFF] << 8)


def write16(data: bytearray, offset: int, value: int) -> None:
    data[offset & 0xFFFF] = value & 0xFF
    data[(offset + 1) & 0xFFFF] = (value >> 8) & 0xFF


def machine16(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def with_low16(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


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
    helper = executable[routine.flag_helper : routine.flag_helper + 14]
    if hashlib.sha256(helper).hexdigest() != routine.flag_helper_sha256:
        raise SystemExit(f"{routine.name} flag helper changed")
    assert len(executable[slice(*routine.span)]) == 208
    assert executable[routine.span[1] - 1] == 0xC3
    assert helper[-1] == 0xC3
    return executable


def state_snapshot(
    data: bytes | bytearray, game: bytes | bytearray, routine: Routine
) -> tuple[int, ...]:
    return (
        read16(data, routine.data_sound),
        read16(data, routine.data_palette),
        read16(game, routine.reset_word),
        game[routine.compressed],
        read16(game, routine.game_sound),
        read16(game, routine.game_palette),
        read16(game, routine.active_offset),
        read16(game, routine.active_segment),
        read16(game, routine.layout),
        read16(game, routine.row_mode),
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple]:
    name = str(vector["name"])
    kind = str(vector["kind"])
    layout = int(vector["layout"])
    row_mode = int(vector["row_mode"])
    extent = int(vector["entry_extent"])
    source_offset = int(vector["input_source"][1])
    parse_offset = int(vector["parse_source"][1])
    reverse_df = vector["direction"] == "backward"
    direction_step = -2 if reverse_df else 2
    skip_present = int(
        bool(vector.get("skip_present", name == "compressed_dispatch_gate"))
    )
    back_buffer_mode = int(bool(vector.get("back_buffer_mode", False)))
    state_flag = int(bool(vector.get("state_flag", vector["sound_offset"] is not None)))
    buffer_end = int(
        vector.get(
            "buffer_end",
            0x0600
            if name in ("extent_exceeds_buffer_end", "extent_equal_buffer_end")
            else 0xF000,
        )
    )
    linked_offset = 0x2400 + case_index * 0x21
    link_key = 0x7100 + case_index

    source_before = seeded_segment(case_index, 17, 7, 29)
    linked_before = seeded_segment(case_index, 19, 11, 23)
    if kind == "direct":
        write16(source_before, parse_offset, layout)
        write16(source_before, parse_offset + direction_step, row_mode)
    elif kind == "sound_palette":
        for offset, value in (
            (0, 0x6473),
            (2, 8),
            (8, 0x6C70),
            (10, 8),
            (16, layout),
            (18, row_mode),
        ):
            write16(source_before, parse_offset + offset, value)
    elif kind == "sound_two_palettes":
        for offset, value in (
            (0, 0x6473),
            (2, 6),
            (6, 0x6C70),
            (8, 6),
            (12, 0x6C70),
            (14, 8),
            (20, layout),
            (22, row_mode),
        ):
            write16(source_before, parse_offset + offset, value)
    elif kind == "link":
        write16(source_before, parse_offset, 0x6D6D)
        write16(source_before, parse_offset + 2, linked_offset)
        write16(source_before, parse_offset + 4, LINKED // 16)
        write16(source_before, parse_offset + 6, link_key)
        actual_key = link_key if name == "matching_link" else link_key ^ 0x00FF
        write16(linked_before, linked_offset, actual_key)
        write16(linked_before, linked_offset + 2, layout)
        write16(linked_before, linked_offset + 4, row_mode)
    else:
        raise AssertionError(f"{routine.name} {name}: unknown case kind {kind}")

    data_before = seeded_segment(case_index, 23, 13, 31)
    game_before = seeded_segment(case_index, 31, 5, 17)
    storage_before = seeded_segment(case_index, 37, 3, 19)
    default_before = seeded_segment(case_index, 41, 9, 13)
    fs_before = seeded_segment(case_index, 47, 15, 7)
    write16(data_before, routine.data_sound, 0xD100 + case_index)
    write16(data_before, routine.data_palette, 0xD200 + case_index)
    game_before[routine.state_flag] = state_flag
    write16(game_before, routine.reset_word, 0xA100 + case_index)
    write16(game_before, routine.default_storage, DEFAULT_STORAGE // 16)
    write16(game_before, routine.active_offset, 0xD300 + case_index)
    write16(game_before, routine.active_segment, 0xD400 + case_index)
    write16(game_before, routine.layout, 0xD500 + case_index)
    write16(game_before, routine.row_mode, 0xD600 + case_index)
    write16(game_before, routine.game_sound, 0xD700 + case_index)
    write16(game_before, routine.game_palette, 0xD800 + case_index)
    game_before[routine.back_buffer_mode] = back_buffer_mode
    game_before[routine.compressed] = 0x70 + case_index
    game_before[routine.skip_present] = skip_present
    write16(game_before, routine.buffer_end, buffer_end)

    data_expected = bytearray(data_before)
    game_expected = bytearray(game_before)
    storage_expected = bytearray(storage_before)
    default_expected = bytearray(default_before)
    writes: list[tuple[str, int, int, int]] = []

    def expected_write(
        target: bytearray, offset: int, value: int, size: int, name_: str
    ) -> None:
        if size == 1:
            target[offset] = value & 0xFF
        else:
            write16(target, offset, value)
        writes.append((name_, offset, size, value & ((1 << (size * 8)) - 1)))

    expected_write(game_expected, routine.reset_word, 0, 2, "reset_word")
    expected_write(game_expected, routine.compressed, 0, 1, "compressed")
    expected_write(data_expected, routine.data_sound, 0xFFFF, 2, "data_sound")
    expected_write(data_expected, routine.data_palette, 0xFFFF, 2, "data_palette")

    wrapped_end = source_offset + extent
    cursor_segment = SOURCE // 16
    cursor = (
        0
        if wrapped_end > 0xFFFF or (wrapped_end & 0xFFFF) > buffer_end
        else source_offset
    )
    assert cursor == parse_offset, (routine.name, name, cursor, parse_offset)

    def lodsw(memory: bytes | bytearray, offset: int) -> tuple[int, int]:
        return read16(memory, offset), (offset + direction_step) & 0xFFFF

    marker_start = cursor
    active_layout, cursor = lodsw(source_before, cursor)
    sound_offset: int | None = None
    palette_offsets: list[int] = []
    if active_layout == 0x6473:
        sound_offset = cursor if state_flag & 1 else None
        record_extent, _ = lodsw(source_before, cursor)
        cursor = (marker_start + record_extent) & 0xFFFF
        marker_start = cursor
        active_layout, cursor = lodsw(source_before, cursor)
        if sound_offset is not None:
            expected_write(
                game_expected,
                routine.game_sound,
                sound_offset,
                2,
                "game_sound",
            )
    while active_layout == 0x6C70:
        record_extent, after_extent = lodsw(source_before, cursor)
        palette_offsets.append(after_extent)
        expected_write(
            game_expected,
            routine.game_palette,
            after_extent,
            2,
            "game_palette",
        )
        cursor = (marker_start + record_extent) & 0xFFFF
        marker_start = cursor
        active_layout, cursor = lodsw(source_before, cursor)
    assert sound_offset == vector["sound_offset"], (routine.name, name, sound_offset)
    assert palette_offsets == vector["palette_offsets"], (
        routine.name,
        name,
        palette_offsets,
    )

    mismatch_tail = False
    if active_layout == 0x6D6D:
        pointer_offset = read16(source_before, cursor)
        pointer_segment = read16(source_before, cursor + 2)
        expected_key = read16(source_before, cursor + 4)
        assert pointer_segment == LINKED // 16
        cursor_segment = pointer_segment
        cursor = pointer_offset
        actual_key, cursor = lodsw(linked_before, cursor)
        active_layout, cursor = lodsw(linked_before, cursor)
        mismatch_tail = actual_key != expected_key

    selected_segment = (
        DEFAULT_STORAGE // 16 if active_layout & 0x0400 else STORAGE // 16
    )
    selected_expected = (
        default_expected
        if selected_segment == DEFAULT_STORAGE // 16
        else storage_expected
    )
    selected_name = (
        "default_storage"
        if selected_segment == DEFAULT_STORAGE // 16
        else "alternate_storage"
    )
    callback_kind: str | None = None
    destination_offset = 0
    if mismatch_tail:
        result_kind = "consumed_mismatched_link"
        callback_kind = "queue_d8c_consume"
    else:
        expected_write(
            game_expected,
            routine.active_segment,
            selected_segment,
            2,
            "active_segment",
        )
        expected_write(game_expected, routine.active_offset, 0, 2, "active_offset")
        expected_write(game_expected, routine.layout, active_layout, 2, "layout")
        expected_write(
            selected_expected,
            destination_offset,
            active_layout,
            2,
            selected_name,
        )
        destination_offset = (destination_offset + direction_step) & 0xFFFF
        source_memory = (
            source_before if cursor_segment == SOURCE // 16 else linked_before
        )
        active_row_mode, cursor = lodsw(source_memory, cursor)
        expected_write(game_expected, routine.row_mode, active_row_mode, 2, "row_mode")
        expected_write(
            selected_expected,
            destination_offset,
            active_row_mode,
            2,
            selected_name,
        )
        destination_offset = (destination_offset + direction_step) & 0xFFFF
        assert active_row_mode == row_mode

        rows = active_row_mode & 0xFF
        if rows == 0:
            result_kind = "stored_empty"
        elif active_layout & 0x0200 == 0:
            result_kind = "published_source"
            published_offset = (cursor - 4) & 0xFFFF
            expected_write(
                game_expected,
                routine.active_offset,
                published_offset,
                2,
                "active_offset",
            )
            expected_write(
                game_expected,
                routine.active_segment,
                cursor_segment,
                2,
                "active_segment",
            )
        elif (
            skip_present == 0 and back_buffer_mode == 0 and active_row_mode >> 8 == 0xFF
        ):
            result_kind = "deferred_rect"
            expected_write(game_expected, routine.compressed, 1, 1, "compressed")
            expected_write(
                game_expected, routine.active_offset, cursor, 2, "active_offset"
            )
            expected_write(
                game_expected,
                routine.active_segment,
                cursor_segment,
                2,
                "active_segment",
            )
        else:
            result_kind = "decoded_storage"
            callback_kind = "resource_payload_decode_dispatch"

    assert result_kind == vector["result_kind"], (
        routine.name,
        name,
        result_kind,
        vector["result_kind"],
    )
    assert selected_segment == int(vector["selected_storage_segment"])
    expected_active_pointer = [
        read16(game_expected, routine.active_offset),
        read16(game_expected, routine.active_segment),
    ]
    if "active_pointer" in vector:
        assert expected_active_pointer == vector["active_pointer"], (
            routine.name,
            name,
            expected_active_pointer,
            vector["active_pointer"],
        )

    stack_before = seeded_segment(case_index, 43, 0, 11)
    stack_before = bytearray((value + 0x51) & 0xFF for value in stack_before)
    stack_before[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    stack_expected = bytearray(stack_before)
    if kind.startswith("sound"):
        write16(
            stack_expected,
            STACK_POINTER - 2,
            routine.image_address(routine.span[0] + 0x30),
        )
        write16(stack_expected, STACK_POINTER - 4, 0x6473)
        write16(stack_expected, STACK_POINTER - 6, DATA // 16)
    if not mismatch_tail:
        write16(stack_expected, STACK_POINTER - 2, DATA // 16)
        write16(stack_expected, STACK_POINTER - 4, cursor_segment)
        if result_kind == "decoded_storage":
            write16(
                stack_expected,
                STACK_POINTER - 4,
                routine.image_address(routine.span[0] + 0xBE),
            )

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | extent,
        UC_X86_REG_EBX: 0xB2B20000 | (0x2100 + case_index),
        UC_X86_REG_ECX: 0xC3C30000 | (0x3200 + case_index),
        UC_X86_REG_EDX: 0xD4D40000 | (0x4300 + case_index),
        UC_X86_REG_ESI: 0xE5E50000 | source_offset,
        UC_X86_REG_EDI: 0xF6F60000 | (0x5400 + case_index),
        UC_X86_REG_EBP: 0x97970000 | (STORAGE // 16),
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: SOURCE // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0602 if reverse_df else 0x0202,
    }
    callback_registers = {
        UC_X86_REG_EAX: 0xA5A50000 | (0xA500 + case_index),
        UC_X86_REG_EBX: 0xB5B50000 | (0xB500 + case_index),
        UC_X86_REG_ECX: 0xC5C50000 | (0xC500 + case_index),
        UC_X86_REG_EDX: 0xD5D50000 | (0xD500 + case_index),
        UC_X86_REG_ESI: 0xE5E50000 | (0xE500 + case_index),
        UC_X86_REG_EDI: 0xF5F50000 | (0xF500 + case_index),
        UC_X86_REG_EBP: 0x85850000 | (0x8500 + case_index),
        UC_X86_REG_ES: 0x9500 + case_index,
        UC_X86_REG_FS: 0xA600 + case_index,
    }
    callback_flags = 0x0AD7 | (0x0400 if reverse_df else 0)

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    expected_module[routine.image_address(routine.decode)] = 0xC3
    expected_module[routine.image_address(routine.consume)] = 0xC3
    machine.mem_write(0, bytes(expected_module))
    for base, contents in (
        (DATA, data_before),
        (SOURCE, source_before),
        (LINKED, linked_before),
        (STORAGE, storage_before),
        (DEFAULT_STORAGE, default_before),
        (GAME, game_before),
        (STACK, stack_before),
        (FS_DATA, fs_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    observed_writes: list[tuple[str, int, int, int]] = []
    observed_calls: list[dict[str, object]] = []
    flag_calls: list[dict[str, object]] = []
    reached_return = False
    previous: int | None = None
    field_names = {
        DATA + routine.data_sound: "data_sound",
        DATA + routine.data_palette: "data_palette",
        GAME + routine.reset_word: "reset_word",
        GAME + routine.compressed: "compressed",
        GAME + routine.game_sound: "game_sound",
        GAME + routine.game_palette: "game_palette",
        GAME + routine.active_offset: "active_offset",
        GAME + routine.active_segment: "active_segment",
        GAME + routine.layout: "layout",
        GAME + routine.row_mode: "row_mode",
    }

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        flag_entry = routine.image_address(routine.flag_helper)
        flag_end = flag_entry + 14
        if address == flag_entry:
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            flag_calls.append(
                {
                    "source": [
                        cpu.reg_read(UC_X86_REG_ES),
                        cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    ],
                    "marker": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "sp": stack_pointer,
                    "return_offset": machine16(cpu, STACK + stack_pointer)
                    + routine.header_size
                    - routine.span[0],
                    "state_flag": cpu.mem_read(GAME + routine.state_flag, 1)[0],
                }
            )
            previous = None
            return
        if flag_entry < address < flag_end:
            previous = None
            return
        callback_name = {
            routine.image_address(routine.decode): "resource_payload_decode_dispatch",
            routine.image_address(routine.consume): "queue_d8c_consume",
        }.get(address)
        if callback_name is not None:
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            call: dict[str, object] = {
                "call": callback_name,
                "source": [
                    cpu.reg_read(
                        UC_X86_REG_DS
                        if callback_name == "resource_payload_decode_dispatch"
                        else UC_X86_REG_ES
                    ),
                    cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                ],
                "sp": stack_pointer,
                "state": state_snapshot(
                    bytes(cpu.mem_read(DATA, SEGMENT_SIZE)),
                    bytes(cpu.mem_read(GAME, SEGMENT_SIZE)),
                    routine,
                ),
            }
            if callback_name == "resource_payload_decode_dispatch":
                call.update(
                    {
                        "destination": [
                            cpu.reg_read(UC_X86_REG_ES),
                            cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        ],
                        "alternate_segment": cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF,
                        "layout": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "row_mode": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                        "return_offset": machine16(cpu, STACK + stack_pointer)
                        + routine.header_size
                        - routine.span[0],
                    }
                )
                for register, value in callback_registers.items():
                    cpu.reg_write(register, value)
                cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)
            else:
                call["layout"] = cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF
            observed_calls.append(call)
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

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, value: int, _context
    ) -> None:
        name_ = field_names.get(address)
        if DATA <= address < DATA + SEGMENT_SIZE:
            offset = address - DATA
        elif STORAGE <= address < STORAGE + SEGMENT_SIZE:
            name_ = "alternate_storage"
            offset = address - STORAGE
        elif DEFAULT_STORAGE <= address < DEFAULT_STORAGE + SEGMENT_SIZE:
            name_ = "default_storage"
            offset = address - DEFAULT_STORAGE
        elif GAME <= address < GAME + SEGMENT_SIZE:
            offset = address - GAME
        else:
            offset = address & 0xFFFF
        if name_ is not None:
            observed_writes.append(
                (name_, offset, size, value & ((1 << (size * 8)) - 1))
            )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=2000)
    assert reached_return, (routine.name, name)
    assert observed_writes == writes, (routine.name, name, observed_writes, writes)
    assert len(flag_calls) == int(kind.startswith("sound")), (
        routine.name,
        name,
        flag_calls,
    )
    if flag_calls:
        assert flag_calls == [
            {
                "source": [SOURCE // 16, parse_offset + 2],
                "marker": 0x6473,
                "sp": STACK_POINTER - 2,
                "return_offset": 0x30,
                "state_flag": state_flag,
            }
        ], (routine.name, name, flag_calls)
    assert len(observed_calls) == int(callback_kind is not None)
    if callback_kind is not None:
        call = observed_calls[0]
        assert call["call"] == callback_kind
        assert call["state"] == state_snapshot(data_expected, game_expected, routine)
        if callback_kind == "resource_payload_decode_dispatch":
            assert call == {
                "call": callback_kind,
                "source": [cursor_segment, cursor],
                "destination": [selected_segment, destination_offset],
                "alternate_segment": STORAGE // 16,
                "layout": active_layout,
                "row_mode": row_mode,
                "return_offset": 0xBE,
                "sp": STACK_POINTER - 4,
                "state": state_snapshot(data_expected, game_expected, routine),
            }, (routine.name, name, call)
        else:
            assert call == {
                "call": callback_kind,
                "source": [cursor_segment, cursor],
                "layout": active_layout,
                "sp": STACK_POINTER,
                "state": state_snapshot(data_expected, game_expected, routine),
            }, (routine.name, name, call)

    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    for base, expected, label in (
        (DATA, data_expected, "data"),
        (SOURCE, source_before, "source"),
        (LINKED, linked_before, "linked source"),
        (STORAGE, storage_expected, "alternate storage"),
        (DEFAULT_STORAGE, default_expected, "default storage"),
        (GAME, game_expected, "game"),
        (STACK, stack_expected, "stack"),
        (FS_DATA, fs_before, "FS"),
    ):
        assert bytes(machine.mem_read(base, SEGMENT_SIZE)) == bytes(expected), (
            routine.name,
            name,
            label,
        )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    expected_registers[UC_X86_REG_DS] = DATA // 16
    if mismatch_tail:
        expected_registers[UC_X86_REG_EAX] = with_low16(
            initial[UC_X86_REG_EAX], active_layout
        )
        expected_registers[UC_X86_REG_EBX] = with_low16(
            initial[UC_X86_REG_EBX], link_key
        )
        expected_registers[UC_X86_REG_ESI] = with_low16(initial[UC_X86_REG_ESI], cursor)
        expected_registers[UC_X86_REG_ES] = cursor_segment
    elif result_kind == "decoded_storage":
        expected_registers.update(callback_registers)
        expected_registers[UC_X86_REG_DS] = DATA // 16
        expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    else:
        expected_registers[UC_X86_REG_EAX] = with_low16(
            initial[UC_X86_REG_EAX],
            row_mode if result_kind == "stored_empty" else cursor_segment,
        )
        expected_registers[UC_X86_REG_EBX] = with_low16(
            initial[UC_X86_REG_EBX], active_layout
        )
        expected_registers[UC_X86_REG_ECX] = with_low16(
            initial[UC_X86_REG_ECX], row_mode
        )
        expected_registers[UC_X86_REG_ESI] = with_low16(
            initial[UC_X86_REG_ESI],
            published_offset if result_kind == "published_source" else cursor,
        )
        expected_registers[UC_X86_REG_EDI] = with_low16(
            initial[UC_X86_REG_EDI], destination_offset
        )
        expected_registers[UC_X86_REG_ES] = selected_segment
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)

    output_calls = []
    for call in observed_calls:
        output_call = dict(call)
        output_call.pop("state")
        if "return_offset" in output_call:
            output_call["return_ip"] = routine.span[0] + int(
                output_call.pop("return_offset")
            )
        output_calls.append(output_call)
    row = {
        "name": name,
        "kind": kind,
        "entry_extent": extent,
        "input_source": [SOURCE // 16, source_offset],
        "parse_source": [SOURCE // 16, parse_offset],
        "layout": layout,
        "row_mode": row_mode,
        "direction": "backward" if reverse_df else "forward",
        "sound_offset": sound_offset,
        "palette_offsets": palette_offsets,
        "selected_storage_segment": selected_segment,
        "result_kind": result_kind,
        "active_pointer": expected_active_pointer,
        "back_buffer_mode": bool(back_buffer_mode),
        "skip_present": bool(skip_present),
        "calls": output_calls,
        "result_flags": result_flags & 0xFFFF,
        "state_sha256": hashlib.sha256(bytes(game_expected)).hexdigest(),
    }
    for key in (
        "name",
        "kind",
        "entry_extent",
        "input_source",
        "parse_source",
        "layout",
        "row_mode",
        "direction",
        "sound_offset",
        "palette_offsets",
        "selected_storage_segment",
        "result_kind",
    ):
        assert row[key] == vector[key], (
            routine.name,
            name,
            key,
            row[key],
            vector[key],
        )

    canonical_calls = []
    for call in observed_calls:
        normalized = dict(call)
        normalized["state"] = tuple(normalized["state"])
        canonical_calls.append(tuple(sorted(normalized.items())))
    canonical = (
        tuple(canonical_calls),
        tuple((field, size, value) for field, _offset, size, value in observed_writes),
        state_snapshot(data_expected, game_expected, routine),
        bytes(source_before),
        bytes(linked_before),
        bytes(storage_expected),
        bytes(default_expected),
        tuple(registers.items()),
        result_flags,
    )
    return row, canonical


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
        default=Path(__file__).parent / "oracle_vectors/func_a552_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    vectors.append(
        {
            "name": "compressed_dispatch_back_buffer",
            "kind": "direct",
            "entry_extent": 64,
            "input_source": [SOURCE // 16, 0x1E10],
            "parse_source": [SOURCE // 16, 0x1E10],
            "layout": 0x0205,
            "row_mode": 0xFF03,
            "direction": "forward",
            "sound_offset": None,
            "palette_offsets": [],
            "selected_storage_segment": STORAGE // 16,
            "result_kind": "decoded_storage",
            "active_pointer": [0, STORAGE // 16],
            "back_buffer_mode": True,
        }
    )
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(vectors):
        commander_row, commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_row, sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        for key in (
            "name",
            "kind",
            "entry_extent",
            "input_source",
            "parse_source",
            "layout",
            "row_mode",
            "direction",
            "sound_offset",
            "palette_offsets",
            "selected_storage_segment",
            "result_kind",
            "active_pointer",
            "back_buffer_mode",
            "skip_present",
            "result_flags",
        ):
            assert sequel_row[key] == commander_row[key], (vector["name"], key)
        rows.append(sequel_row)

    assert commander_edges == normalized_edges(COMMANDER), (
        commander_edges,
        normalized_edges(COMMANDER),
    )
    assert sequel_edges == normalized_edges(SEQUEL), (
        sequel_edges,
        normalized_edges(SEQUEL),
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
        f"verified {len(rows)} BBB presentation-entry cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
