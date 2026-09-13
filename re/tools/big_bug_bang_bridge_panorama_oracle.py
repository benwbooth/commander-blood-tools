#!/usr/bin/env python3
"""Compare Big Bug Bang's bridge panorama loader directly with Commander Blood."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AH,
    UC_X86_REG_AL,
    UC_X86_REG_AX,
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

HEADER_SIZE = 0x800
SEGMENT_SIZE = 0x10000
DATA = 0x20000
GAME = 0x40000
BUFFER = 0x60000
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
    span: tuple[int, int]
    body_sha256: str
    handle: int
    directory: int
    framebuffer_pointer: int
    station_table: int
    palette_refresh: int
    panorama_palette: int
    live_palette: int
    unpack_entry: int
    unpack_return: int
    branches: dict[int, tuple[int, int]]


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x981B, 0x98B9),
    body_sha256="15f5ca552b2f2c16e3836a20d958494931366820d788599c0d068a5b44c09910",
    handle=0x0AC4,
    directory=0x0AD2,
    framebuffer_pointer=0x5221,
    station_table=0x2A1B,
    palette_refresh=0x5B53,
    panorama_palette=0x5B58,
    live_palette=0x5251,
    unpack_entry=0x01CE * 16 + 0x0A70,
    unpack_return=0x9893,
    branches={0x9875: (0x9865, 0x9877), 0x9899: (0x98AD, 0x989B)},
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xAFBA, 0xB058),
    body_sha256="144a3952b9f63fb1a5db5f00a2e38ff2863a26328da4a3be34cc541203ff265d",
    handle=0x0CBC,
    directory=0x0CCE,
    framebuffer_pointer=0x55F1,
    station_table=0x2CBB,
    palette_refresh=0x5F23,
    panorama_palette=0x5F28,
    live_palette=0x5621,
    unpack_entry=0x01E6 * 16 + 0x0A76,
    unpack_return=0xB032,
    branches={0xB014: (0xB004, 0xB016), 0xB038: (0xB04C, 0xB03A)},
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


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def write_wrapped(data: bytearray, offset: int, payload: bytes) -> None:
    for index, value in enumerate(payload):
        data[(offset + index) & 0xFFFF] = value


def seeded_segment(
    case_index: int,
    multiplier: int,
    high_multiplier: int,
    case_multiplier: int,
    salt: int,
) -> bytearray:
    return bytearray(
        (
            offset * multiplier
            + (offset >> 8) * high_multiplier
            + case_index * case_multiplier
            + salt
        )
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initialize_memory(
    routine: Routine, vector: dict[str, object], case_index: int
) -> tuple[
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bool,
]:
    handle = int(vector["dos_calls"][0]["handle"])
    directory = tuple(int(value) for value in vector["active_directory"])
    chunk_offset = int(vector["chunk_pointer"][1])
    requested = int(vector["chunk_read_count_low_word"])
    selected_box = bytes(vector["selected_box"])
    selected_station = int(vector["selected_station_unchecked"])
    directory_success = bool(vector["directory_read_success"])
    chunk_success = bool(vector["chunk_read_success"])

    data_before = seeded_segment(case_index, 17, 29, 41, 0x07)
    write16(data_before, routine.handle, handle)
    struct.pack_into("<II", data_before, routine.directory, *directory)
    struct.pack_into(
        "<HH", data_before, routine.framebuffer_pointer, chunk_offset, BUFFER // 16
    )
    data_expected = bytearray(data_before)
    if directory_success:
        struct.pack_into("<II", data_expected, routine.directory, *directory)

    buffer_before = seeded_segment(case_index, 23, 11, 31, 0x35)
    write_wrapped(
        buffer_before,
        chunk_offset,
        selected_box + struct.pack("<H", selected_station),
    )
    buffer_expected = bytearray(buffer_before)
    chunk_payload = bytearray(
        (0x80 + case_index * 19 + index * 3) & 0xFF for index in range(requested)
    )
    assert requested >= 10
    chunk_payload[:8] = selected_box
    chunk_payload[8:10] = struct.pack("<H", selected_station)
    if chunk_success:
        write_wrapped(buffer_expected, chunk_offset, bytes(chunk_payload))

    game_before = seeded_segment(case_index, 31, 7, 37, 0x51)
    game_before[routine.palette_refresh] = int(vector["palette_refresh_before"])
    game_before[routine.live_palette : routine.live_palette + 768] = bytes(
        (index * 5 + case_index * 43 + 0x17) & 0xFF for index in range(768)
    )
    game_before[routine.panorama_palette : routine.panorama_palette + 768] = bytes(
        (index * 11 + case_index * 47 + 0x6D) & 0xFF for index in range(768)
    )
    game_at_unpack = bytearray(game_before)
    for station in range(4):
        box_offset = routine.station_table + station * 24 + 12
        game_at_unpack[box_offset : box_offset + 8] = b"\xff" * 8
    selected_offset = routine.station_table + selected_station * 24 + 12
    game_at_unpack[selected_offset : selected_offset + 8] = selected_box

    game_expected = bytearray(game_at_unpack)
    callback_refresh = int(vector["palette_refresh_after_unpack"])
    game_expected[routine.palette_refresh] = callback_refresh
    mutate_palette = callback_refresh != int(vector["palette_refresh_before"])
    if mutate_palette:
        game_expected[routine.panorama_palette : routine.panorama_palette + 768] = (
            bytes((index * 13 + case_index * 53 + 0xA1) & 0xFF for index in range(768))
        )
    if callback_refresh & 1:
        game_expected[routine.live_palette : routine.live_palette + 768] = (
            game_expected[routine.panorama_palette : routine.panorama_palette + 768]
        )

    return (
        data_before,
        data_expected,
        buffer_before,
        buffer_expected,
        game_before,
        game_at_unpack,
        game_expected,
        chunk_payload,
        selected_box,
        mutate_palette,
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], bytes]:
    name = str(vector["name"])
    (
        data_before,
        data_expected,
        buffer_before,
        buffer_expected,
        game_before,
        game_at_unpack,
        game_expected,
        chunk_payload,
        _selected_box,
        mutate_palette,
    ) = initialize_memory(routine, vector, case_index)
    extra_before = bytes(seeded_segment(case_index, 13, 3, 17, 0x73))
    fs_before = bytes(seeded_segment(case_index, 7, 5, 23, 0x97))
    stack_before = seeded_segment(case_index, 5, 19, 43, 0xB1)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    frame = int(vector["frame"])
    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | frame,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E55678 + case_index,
        UC_X86_REG_EDI: 0xF6F66789 + case_index,
        UC_X86_REG_EBP: 0x9797789A + case_index,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    stack_expected = bytearray(stack_before)
    write32(stack_expected, 0xFEFC, initial[UC_X86_REG_EAX])
    write16(stack_expected, 0xFEFA, initial[UC_X86_REG_ESI])
    write16(stack_expected, 0xFEF8, initial[UC_X86_REG_EDI])
    write16(stack_expected, 0xFEF6, initial[UC_X86_REG_ES])
    write16(stack_expected, 0xFEF4, initial[UC_X86_REG_DS])
    write16(stack_expected, 0xFEF2, initial[UC_X86_REG_ECX])
    write16(stack_expected, 0xFEF0, initial[UC_X86_REG_EDX])
    write16(stack_expected, 0xFEEE, initial[UC_X86_REG_EBX])
    write32(stack_expected, 0xFEEA, initial[UC_X86_REG_EBP])
    write16(stack_expected, 0xFEE8, 0)
    write16(stack_expected, 0xFEE6, image_address(routine.unpack_return))

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    expected_module[routine.unpack_entry] = 0xCB
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(BUFFER, bytes(buffer_before))
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, object]] = []
    unpack_calls: list[dict[str, object]] = []
    reached_return = False
    previous_file_offset: int | None = None

    def set_carry(cpu: Uc, carry: bool) -> None:
        flags = cpu.reg_read(UC_X86_REG_EFLAGS)
        cpu.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)

    def interrupt(cpu: Uc, number: int, _context) -> None:
        call_index = len(calls)
        assert number == 0x21 and call_index < 4, (routine.name, name, number)
        expected = vector["dos_calls"][call_index]
        assert cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF == int(expected["handle"])
        function = cpu.reg_read(UC_X86_REG_AH)
        if call_index in (0, 2):
            assert function == 0x42 and cpu.reg_read(UC_X86_REG_AL) == 0
            offset = (cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF) << 16
            offset |= cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF
            assert offset == int(expected["offset"]), (routine.name, name, offset)
            call = dict(expected)
        else:
            assert function == 0x3F
            destination = [
                cpu.reg_read(UC_X86_REG_DS),
                cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
            ]
            actual_destination = list(expected["destination"])
            if call_index == 1:
                actual_destination[1] = routine.directory
            assert destination == actual_destination, (routine.name, name, destination)
            requested = cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF
            assert requested == int(expected["requested"])
            if call_index == 1 and bool(vector["directory_read_success"]):
                cpu.mem_write(
                    DATA + routine.directory,
                    struct.pack("<II", *(int(v) for v in vector["active_directory"])),
                )
            elif call_index == 3 and bool(vector["chunk_read_success"]):
                chunk_offset = int(vector["chunk_pointer"][1])
                for index, value in enumerate(chunk_payload):
                    cpu.mem_write(
                        BUFFER + ((chunk_offset + index) & 0xFFFF), bytes((value,))
                    )
            call = dict(expected)
            call["destination"] = list(expected["destination"])
        calls.append(call)
        cpu.reg_write(UC_X86_REG_AX, int(expected["result_ax_ignored"]))
        set_carry(cpu, bool(expected["carry_ignored"]))

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal previous_file_offset, reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == routine.unpack_entry:
            assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_at_unpack), (
                routine.name,
                name,
            )
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_frame = list(struct.unpack("<HH", cpu.mem_read(STACK + sp, 4)))
            assert return_frame == [image_address(routine.unpack_return), 0]
            unpack_calls.append(
                {
                    "call": "bridge_panorama_frame_unpack",
                    "source": [
                        cpu.reg_read(UC_X86_REG_DS),
                        cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    ],
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "gs": cpu.reg_read(UC_X86_REG_GS),
                    "sp": sp,
                    "return_frame": list(vector["unpack_call"]["return_frame"]),
                }
            )
            cpu.mem_write(
                GAME + routine.palette_refresh,
                bytes((int(vector["palette_refresh_after_unpack"]),)),
            )
            if mutate_palette:
                cpu.mem_write(
                    GAME + routine.panorama_palette,
                    bytes(
                        (index * 13 + case_index * 53 + 0xA1) & 0xFF
                        for index in range(768)
                    ),
                )
            return

        file_offset = address + HEADER_SIZE
        assert routine.span[0] <= file_offset < routine.span[1], (
            routine.name,
            name,
            hex(file_offset),
        )
        if previous_file_offset in routine.branches:
            covered_edges.add(
                (
                    previous_file_offset - routine.span[0],
                    file_offset - routine.span[0],
                )
            )
        previous_file_offset = file_offset

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(image_address(routine.span[0]), 0, count=1024)
    assert reached_return and len(calls) == 4 and len(unpack_calls) == 1
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected)
    assert bytes(machine.mem_read(BUFFER, SEGMENT_SIZE)) == bytes(buffer_expected)
    actual_game = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    assert actual_game == bytes(game_expected), (routine.name, name)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    assert bytes(machine.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = 0xFF02
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        "cf": bool(flags & 1),
        "pf": bool(flags & 4),
        "zf": bool(flags & 0x40),
        "sf": bool(flags & 0x80),
        "of": bool(flags & 0x800),
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, flags)
    assert flags & 0x0400 == 0

    canonical_game = initialize_memory(COMMANDER, vector, case_index)[6]
    row = {
        "name": name,
        "frame": frame,
        "directory_seek_offset": calls[0]["offset"],
        "directory_read_success": bool(vector["directory_read_success"]),
        "active_directory": [int(value) for value in vector["active_directory"]],
        "chunk_read_count_low_word": calls[3]["requested"],
        "chunk_read_success": bool(vector["chunk_read_success"]),
        "chunk_pointer": list(vector["chunk_pointer"]),
        "selected_station_unchecked": int(vector["selected_station_unchecked"]),
        "selected_box": list(vector["selected_box"]),
        "palette_refresh_before": int(vector["palette_refresh_before"]),
        "palette_refresh_after_unpack": int(vector["palette_refresh_after_unpack"]),
        "palette_copied": bool(int(vector["palette_refresh_after_unpack"]) & 1),
        "dos_calls": calls,
        "unpack_call": unpack_calls[0],
        "game_state_sha256": hashlib.sha256(canonical_game).hexdigest(),
        "buffer_state_sha256": hashlib.sha256(buffer_expected).hexdigest(),
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    return row, registers, bytes(machine.mem_read(STACK, SEGMENT_SIZE))


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


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    return {
        (source - routine.span[0], destination - routine.span[0])
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


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
        default=Path(__file__).parent / "oracle_vectors/func_981b_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for index, vector in enumerate(vectors):
        commander_row, commander_registers, _commander_stack = execute(
            commander_executable, COMMANDER, vector, index, commander_edges
        )
        sequel_row, sequel_registers, _sequel_stack = execute(
            sequel_executable, SEQUEL, vector, index, sequel_edges
        )
        assert commander_row == vector, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
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
        f"verified {len(rows)} BBB panorama-loader cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
