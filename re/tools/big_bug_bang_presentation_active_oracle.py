#!/usr/bin/env python3
"""Compare Big Bug Bang's active presentation coordinator with Commander Blood."""

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
ACTIVE = 0x20000
DATA = 0x32000
STAGING = 0x44000
EXTRA = 0x56000
DISPLAY = 0x68000
BACK_BUFFER = 0x7A000
FS_DATA = 0x8C000
STACK = 0x9E000
GAME = 0xB0000
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
    active_offset: int
    active_segment: int
    retired_segment: int
    requested_id: int
    layout: int
    row_mode: int
    frame_presented: int
    back_buffer_mode: int
    compressed: int
    skip_present: int
    unclamped_rows: int
    vertical_offset: int
    staging_segment: int
    display_pointer: int
    back_buffer_pointer: int
    rect_blit: int
    decode_rect: int
    full_screen: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA41A, 0xA4ED),
    body_sha256="d33eeda97f2b75f7d3446bce71daad29b5c669d95e5958ef7e178d11cf99eeab",
    active_offset=0x0D94,
    active_segment=0x0D96,
    retired_segment=0x0DAA,
    requested_id=0x0D80,
    layout=0x0DA4,
    row_mode=0x0DA6,
    frame_presented=0x0DB8,
    back_buffer_mode=0x0DB9,
    compressed=0x0DBA,
    skip_present=0x0DBB,
    unclamped_rows=0x0DBD,
    vertical_offset=0x1FA7,
    staging_segment=0x0ABE,
    display_pointer=0x5221,
    back_buffer_pointer=0x5229,
    rect_blit=0xA4ED,
    decode_rect=0xAB25,
    full_screen=0x3846,
    branches={
        0xA430: (0xA434, 0xA4E4),
        0xA45B: (0xA45D, 0xA461),
        0xA475: (0xA477, 0xA49B),
        0xA479: (0xA47B, 0xA48B),
        0xA47E: (0xA480, 0xA482),
        0xA4A1: (0xA4A3, 0xA4B1),
        0xA4B7: (0xA4B9, 0xA4D2),
        0xA4BB: (0xA4BD, 0xA4CF),
        0xA4C3: (0xA4C5, 0xA4CC),
        0xA4C8: (0xA4CA, 0xA4CC),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBC04, 0xBCD7),
    body_sha256="75763b21515471444c07b0f9c11603f870101879bbf343a59d037b2d6f2726fb",
    active_offset=0x0FE2,
    active_segment=0x0FE4,
    retired_segment=0x0FF8,
    requested_id=0x0FCE,
    layout=0x0FF2,
    row_mode=0x0FF4,
    frame_presented=0x1006,
    back_buffer_mode=0x1007,
    compressed=0x1008,
    skip_present=0x1009,
    unclamped_rows=0x100B,
    vertical_offset=0x21F5,
    staging_segment=0x0CB6,
    display_pointer=0x55F1,
    back_buffer_pointer=0x55F9,
    rect_blit=0xBCD7,
    decode_rect=0xC30D,
    full_screen=0x3AC3,
    branches={
        0xBC1A: (0xBC1E, 0xBCCE),
        0xBC45: (0xBC47, 0xBC4B),
        0xBC5F: (0xBC61, 0xBC85),
        0xBC63: (0xBC65, 0xBC75),
        0xBC68: (0xBC6A, 0xBC6C),
        0xBC8B: (0xBC8D, 0xBC9B),
        0xBCA1: (0xBCA3, 0xBCBC),
        0xBCA5: (0xBCA7, 0xBCB9),
        0xBCAD: (0xBCAF, 0xBCB6),
        0xBCB2: (0xBCB4, 0xBCB6),
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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: tuple[int, int], label: str
) -> None:
    start, end = allowed
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or start <= offset < end:
            continue
        raise AssertionError(f"{label} changed outside stack envelope at {offset:#x}")


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
    assert executable[routine.span[1] - 1] == 0xCB
    return executable


def expected_calls(vector: dict[str, object]) -> list[dict[str, object]]:
    normalized = []
    for call_value in vector["calls"]:
        call = dict(call_value)
        call["return_offset"] = int(call.pop("return_ip")) - COMMANDER.span[0]
        call["callback_state"] = [0, ACTIVE // 16, 1]
        normalized.append(call)
    return normalized


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple]:
    name = str(vector["name"])
    active = bool(vector["active"])
    header = int(vector["header"])
    row_mode = int(vector["row_mode"])
    coordinates_value = vector["coordinates"]
    coordinates = (
        None
        if coordinates_value is None
        else tuple(int(value) for value in coordinates_value)
    )
    vertical_offset = int(vector["vertical_offset"])
    back_buffer_mode = int(bool(vector["back_buffer_mode"]))
    skip_present = int(bool(vector["skip_present"]))
    compressed = int(bool(vector["compressed"]))
    unclamped_rows = int(bool(vector["unclamped_rows"]))
    reverse_df = vector["direction"] == "backward"
    active_offset = int(vector["source_offset"])
    requested_id = 0x1200 + case_index
    back_buffer_offset = 0x2340 + case_index * 3
    display_offset = 0x4560 + case_index * 5

    active_before = seeded_segment(case_index, 17, 0x29)

    def store_active_word(offset: int, value: int) -> None:
        active_before[offset & 0xFFFF] = value & 0xFF
        active_before[(offset + 1) & 0xFFFF] = value >> 8

    source_cursor = active_offset
    source_step = -2 if reverse_df else 2
    for value in (header, row_mode):
        store_active_word(source_cursor, value)
        source_cursor = (source_cursor + source_step) & 0xFFFF
    if coordinates is not None:
        for value in coordinates:
            store_active_word(source_cursor, value)
            source_cursor = (source_cursor + source_step) & 0xFFFF
    assert source_cursor == int(vector["payload_offset"]), name

    game_before = seeded_segment(case_index, 23, 0x3B)
    write16(game_before, routine.staging_segment, STAGING // 16)
    write16(game_before, routine.requested_id, requested_id)
    write16(game_before, routine.active_offset, active_offset)
    write16(game_before, routine.active_segment, ACTIVE // 16 if active else 0)
    write16(game_before, routine.layout, header)
    write16(game_before, routine.row_mode, row_mode)
    write16(game_before, routine.retired_segment, 0xA500 + case_index)
    game_before[routine.frame_presented] = 0x60 + case_index
    game_before[routine.back_buffer_mode] = back_buffer_mode
    game_before[routine.compressed] = compressed
    game_before[routine.skip_present] = skip_present
    game_before[routine.unclamped_rows] = unclamped_rows
    write16(game_before, routine.vertical_offset, vertical_offset)
    struct.pack_into(
        "<HH", game_before, routine.display_pointer, display_offset, DISPLAY // 16
    )
    struct.pack_into(
        "<HH",
        game_before,
        routine.back_buffer_pointer,
        back_buffer_offset,
        BACK_BUFFER // 16,
    )
    game_expected = bytearray(game_before)
    write16(game_expected, routine.active_segment, 0)
    write16(game_expected, routine.retired_segment, ACTIVE // 16 if active else 0)
    if active:
        game_expected[routine.frame_presented] = 1

    data_before = seeded_segment(case_index, 31, 0x4D)
    staging_before = seeded_segment(case_index, 37, 0x5F)
    extra_before = seeded_segment(case_index, 41, 0x71)
    display_before = seeded_segment(case_index, 43, 0x83)
    back_buffer_before = seeded_segment(case_index, 47, 0x95)
    fs_before = seeded_segment(case_index, 53, 0xA7)
    stack_before = seeded_segment(case_index, 59, 0xB9)
    stack_before[STACK_POINTER : STACK_POINTER + 12] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | (0x3000 + case_index),
        UC_X86_REG_EBX: 0xB2B22340 + case_index,
        UC_X86_REG_ECX: 0xC3C33450 + case_index,
        UC_X86_REG_EDX: 0xD4D44560 + case_index,
        UC_X86_REG_ESI: 0xE5E55670 + case_index,
        UC_X86_REG_EDI: 0xF6F66780 + case_index,
        UC_X86_REG_EBP: 0x97977890 + case_index,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0602 if reverse_df else 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xC1000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    rect_entry = routine.image_address(routine.rect_blit)
    decode_entry = routine.image_address(routine.decode_rect)
    expected_module[RETURN_IP] = 0xCC
    expected_module[rect_entry] = 0xC3
    expected_module[decode_entry] = 0xC3
    expected_module[routine.full_screen] = 0xCB
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(ACTIVE, bytes(active_before))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(STAGING, bytes(staging_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(DISPLAY, bytes(display_before))
    machine.mem_write(BACK_BUFFER, bytes(back_buffer_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(STACK, bytes(stack_before))
    machine.mem_write(GAME, bytes(game_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected = expected_calls(vector)
    calls: list[dict[str, object]] = []
    writes: list[tuple[str, int]] = []
    write_names = {
        routine.active_segment: "active_segment",
        routine.retired_segment: "retired_segment",
        routine.frame_presented: "frame_presented",
    }
    reached_return = False
    previous: int | None = None
    callback_flags = 0x0AD7 | (0x0400 if reverse_df else 0)

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        callback_name = {
            rect_entry: "resource_rect_blit",
            decode_entry: "resource_payload_decode_rect",
            routine.full_screen: "full_screen_blit",
        }.get(address)
        if callback_name is not None:
            if len(calls) >= len(expected):
                raise AssertionError(f"{routine.name} {name}: unexpected callback")
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            return_ip = read16(
                bytes(cpu.mem_read(STACK + stack_pointer, 2)), 0
            ) + routine.header_size
            call: dict[str, object] = {
                "call": callback_name,
                "return_offset": return_ip - routine.span[0],
                "source": [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF],
                "sp": stack_pointer,
                "callback_state": [
                    read16(bytes(cpu.mem_read(GAME + routine.active_segment, 2)), 0),
                    read16(bytes(cpu.mem_read(GAME + routine.retired_segment, 2)), 0),
                    cpu.mem_read(GAME + routine.frame_presented, 1)[0],
                ],
            }
            if callback_name == "full_screen_blit":
                call["return_cs"] = read16(
                    bytes(cpu.mem_read(STACK + stack_pointer + 2, 2)), 0
                )
            elif callback_name == "resource_rect_blit":
                call.update(
                    {
                        "target_segment": cpu.reg_read(UC_X86_REG_ES),
                        "x": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                        "y": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "width": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "row_mode": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    }
                )
            else:
                call["staging_offset"] = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
            assert call == expected[len(calls)], (
                routine.name,
                name,
                call,
                expected[len(calls)],
            )
            assert cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF == requested_id
            assert cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF == ACTIVE // 16
            cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)
            calls.append(call)
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
        if not GAME <= address < GAME + SEGMENT_SIZE:
            return
        offset = address - GAME
        assert offset in write_names, (routine.name, name, hex(offset), size, value)
        writes.append((write_names[offset], value & ((1 << (size * 8)) - 1)))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=1000)
    assert reached_return, (routine.name, name)
    assert calls == expected, (routine.name, name, calls, expected)

    expected_writes = [("active_segment", 0), ("retired_segment", ACTIVE // 16)]
    if not active:
        expected_writes[1] = ("retired_segment", 0)
    else:
        expected_writes.append(("frame_presented", 1))
    assert writes == expected_writes, (routine.name, name, writes, expected_writes)

    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    for base, before, label in (
        (ACTIVE, active_before, "active resource"),
        (DATA, data_before, "data"),
        (STAGING, staging_before, "staging"),
        (EXTRA, extra_before, "extra"),
        (DISPLAY, display_before, "display"),
        (BACK_BUFFER, back_buffer_before, "back buffer"),
        (FS_DATA, fs_before, "FS"),
    ):
        assert bytes(machine.mem_read(base, SEGMENT_SIZE)) == bytes(before), (
            routine.name,
            name,
            label,
        )
    game_after = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    assert game_after == bytes(game_expected), (routine.name, name, "game")
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(stack_before), stack_after, (0xFEE6, 0xFF04), f"{routine.name} {name}"
    )
    assert stack_after[0xFF04 : 0xFF04 + len(STACK_SENTINEL)] == STACK_SENTINEL

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
    if active:
        expected_registers[UC_X86_REG_EAX] = (
            initial[UC_X86_REG_EAX] & 0xFFFF0000
        ) | (0 if compressed and not back_buffer_mode else requested_id)
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers)
    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)

    output_calls = []
    for call in calls:
        output_call = dict(call)
        output_call["return_ip"] = routine.span[0] + int(
            output_call.pop("return_offset")
        )
        del output_call["callback_state"]
        output_calls.append(output_call)
    row = {
        "name": name,
        "active": active,
        "header": header,
        "masked_width": header & 0xF9FF,
        "row_mode": row_mode,
        "coordinates": list(coordinates) if coordinates is not None else None,
        "vertical_offset": vertical_offset,
        "adjusted_y": int(vector["adjusted_y"]),
        "source_offset": active_offset,
        "payload_offset": source_cursor,
        "direction": str(vector["direction"]),
        "back_buffer_mode": bool(back_buffer_mode),
        "skip_present": bool(skip_present),
        "compressed": bool(compressed),
        "unclamped_rows": bool(unclamped_rows),
        "calls": output_calls,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "result_flags": result_flags & 0xFFFF,
        "writes": [[field, value] for field, value in writes],
        "state_sha256": hashlib.sha256(game_after).hexdigest(),
    }
    for key in (
        "name",
        "active",
        "header",
        "masked_width",
        "row_mode",
        "coordinates",
        "vertical_offset",
        "adjusted_y",
        "source_offset",
        "payload_offset",
        "direction",
        "back_buffer_mode",
        "skip_present",
        "compressed",
        "unclamped_rows",
        "result_ax",
    ):
        assert row[key] == vector[key], (routine.name, name, key, row[key], vector[key])

    canonical = (
        tuple(tuple(sorted(call.items())) for call in calls),
        tuple(writes),
        (0, ACTIVE // 16 if active else 0, 1 if active else 0),
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
        default=Path(__file__).parent / "oracle_vectors/func_a41a_natural.json",
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
            "name": "back_buffer_rows_within_clamp",
            "active": True,
            "header": 0x0407,
            "masked_width": 7,
            "row_mode": 0x1202,
            "coordinates": None,
            "vertical_offset": 6,
            "adjusted_y": 6,
            "source_offset": 0x1A00,
            "payload_offset": 0x1A04,
            "direction": "forward",
            "back_buffer_mode": True,
            "skip_present": False,
            "compressed": False,
            "unclamped_rows": False,
            "calls": [
                {
                    "call": "resource_rect_blit",
                    "return_ip": 0xA48B,
                    "source": [ACTIVE // 16, 0x1A04],
                    "target_segment": BACK_BUFFER // 16,
                    "x": 0,
                    "y": 6,
                    "width": 7,
                    "row_mode": 0x1202,
                    "sp": 0xFEEC,
                },
                {
                    "call": "full_screen_blit",
                    "return_ip": 0xA497,
                    "return_cs": 0,
                    "source": [BACK_BUFFER // 16, 0x235E],
                    "sp": 0xFEE6,
                },
            ],
            "result_ax": 0x120A,
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
        assert commander_row["result_ax"] == sequel_row["result_ax"]
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
        f"verified {len(rows)} BBB active-presentation cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
