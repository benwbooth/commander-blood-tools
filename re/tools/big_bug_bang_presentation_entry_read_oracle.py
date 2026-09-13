#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation entry read with Commander Blood."""

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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
BUFFER = 0x30000
EXTRA = 0x40000
FS_DATA = 0x60000
GAME = 0x70000
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
    transfer: int
    file_handle: int
    source_offset: int
    source_remaining: int
    head: int
    head_segment: int
    byte_count: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA622, 0xA634),
    body_sha256="b11b14dd5e323bd7f73ffe721f8a5ddcce7dd7e34e770391077c71deb442a6fe",
    transfer=0xA664,
    file_handle=0x0D5B,
    source_offset=0x0D84,
    source_remaining=0x0D88,
    head=0x0D8C,
    head_segment=0x0D8E,
    byte_count=0x0D9A,
    branches={0xA628: (0xA62A, 0xA633)},
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBE0C, 0xBE1E),
    body_sha256="fedab3c41c3b155fe2f0e9a912c37113e3a7b93637c831ae4ccc23470e822df4",
    transfer=0xBE4E,
    file_handle=0x0FA9,
    source_offset=0x0FD2,
    source_remaining=0x0FD6,
    head=0x0FDA,
    head_segment=0x0FDC,
    byte_count=0x0FE8,
    branches={0xBE12: (0xBE14, 0xBE1D)},
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


def seeded_region(
    size: int, case_index: int, multiplier: int, shift_multiplier: int
) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * shift_multiplier + case_index * 29)
        & 0xFF
        for offset in range(size)
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
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert len(body) == 18 and body[-1] == 0xC3
    return executable


def state_snapshot(game: bytes | bytearray, routine: Routine) -> tuple[int, ...]:
    return (
        read16(game, routine.file_handle),
        read32(game, routine.source_offset),
        read32(game, routine.source_remaining),
        read16(game, routine.head),
        read16(game, routine.head_segment),
        read16(game, routine.byte_count),
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple]:
    name = str(vector["name"])
    success = bool(vector["success"])
    handle = int(vector["handle"])
    initial_head = int(vector["initial_head"])
    initial_byte_count = int(vector["initial_byte_count"])
    result = dict(vector["result"])
    source_offset = int(result["source_offset"])
    source_remaining = int(result["source_remaining"])
    if success:
        source_offset = (source_offset - 2) & 0xFFFFFFFF
        source_remaining = (source_remaining + 2) & 0xFFFFFFFF

    data_before = seeded_region(SEGMENT_SIZE, case_index, 17, 7)
    game_before = seeded_region(SEGMENT_SIZE, case_index, 23, 11)
    buffer_before = seeded_region(SEGMENT_SIZE, case_index, 31, 13)
    extra_before = seeded_region(SEGMENT_SIZE, case_index, 37, 17)
    fs_before = seeded_region(SEGMENT_SIZE, case_index, 41, 19)
    write16(game_before, routine.file_handle, handle)
    write32(game_before, routine.source_offset, source_offset)
    write32(game_before, routine.source_remaining, source_remaining)
    write16(game_before, routine.head, initial_head)
    write16(game_before, routine.head_segment, BUFFER // 16)
    write16(game_before, routine.byte_count, initial_byte_count)

    game_expected = bytearray(game_before)
    buffer_expected = bytearray(buffer_before)
    extra_expected = bytearray(extra_before)
    if success:
        write32(game_expected, routine.source_offset, int(result["source_offset"]))
        write32(
            game_expected, routine.source_remaining, int(result["source_remaining"])
        )
        write16(game_expected, routine.head, int(result["head"]))
        write16(game_expected, routine.byte_count, int(result["byte_count"]))
        extent = int(vector["extent"])
        encoded_extent = struct.pack("<H", extent)
        buffer_expected[initial_head] = encoded_extent[0]
        if initial_head == 0xFFFF:
            extra_expected[0] = encoded_extent[1]
        else:
            buffer_expected[initial_head + 1] = encoded_extent[1]

    stack_before = seeded_region(SEGMENT_SIZE, case_index, 43, 23)
    stack_before[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    stack_expected = bytearray(stack_before)
    write16(
        stack_expected,
        STACK_POINTER - 2,
        routine.image_address(routine.span[0] + 6),
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | (0x1000 + case_index),
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E55555,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0ED7,
    }
    callback_flags = 0x0AD6 | int(not success)

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xA0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    transfer_entry = routine.image_address(routine.transfer)
    expected_module[transfer_entry] = 0xC3
    machine.mem_write(0, bytes(expected_module))
    for base, contents in (
        (DATA, data_before),
        (BUFFER, buffer_before),
        (EXTRA, extra_before),
        (FS_DATA, fs_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    transfer_calls: list[dict[str, object]] = []
    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == transfer_entry:
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            transfer_calls.append(
                {
                    "requested": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "sp": stack_pointer,
                    "return_offset": machine16(cpu, STACK + stack_pointer)
                    + routine.header_size
                    - routine.span[0],
                    "state": state_snapshot(
                        bytes(cpu.mem_read(GAME, SEGMENT_SIZE)), routine
                    ),
                }
            )
            if success:
                cpu.mem_write(GAME, bytes(game_expected))
                cpu.mem_write(BUFFER, bytes(buffer_expected))
                cpu.mem_write(EXTRA, bytes(extra_expected))
                cpu.reg_write(UC_X86_REG_EAX, with_low16(initial[UC_X86_REG_EAX], 2))
                cpu.reg_write(
                    UC_X86_REG_EDX,
                    with_low16(initial[UC_X86_REG_EDX], initial_head),
                )
            cpu.reg_write(UC_X86_REG_EBX, with_low16(initial[UC_X86_REG_EBX], handle))
            cpu.reg_write(UC_X86_REG_ECX, with_low16(initial[UC_X86_REG_ECX], 2))
            cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)
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
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=100)
    assert reached_return, (routine.name, name)
    assert transfer_calls == [
        {
            "requested": 2,
            "sp": STACK_POINTER - 2,
            "return_offset": 6,
            "state": state_snapshot(game_before, routine),
        }
    ], (routine.name, name, transfer_calls)

    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    for base, expected, size, label in (
        (DATA, data_before, SEGMENT_SIZE, "data"),
        (BUFFER, buffer_expected, SEGMENT_SIZE, "queue buffer"),
        (EXTRA, extra_expected, SEGMENT_SIZE, "ES decoy"),
        (FS_DATA, fs_before, SEGMENT_SIZE, "FS decoy"),
        (GAME, game_expected, SEGMENT_SIZE, "game"),
        (STACK, stack_expected, SEGMENT_SIZE, "stack"),
    ):
        assert bytes(machine.mem_read(base, size)) == bytes(expected), (
            routine.name,
            name,
            label,
        )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EBX] = with_low16(initial[UC_X86_REG_EBX], handle)
    expected_registers[UC_X86_REG_ECX] = with_low16(initial[UC_X86_REG_ECX], 2)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    if success:
        expected_registers[UC_X86_REG_EAX] = with_low16(
            initial[UC_X86_REG_EAX], int(vector["extent"])
        )
        expected_registers[UC_X86_REG_EDX] = with_low16(
            initial[UC_X86_REG_EDX], initial_head
        )
        expected_registers[UC_X86_REG_ESI] = with_low16(
            initial[UC_X86_REG_ESI], int(result["head"])
        )
        expected_registers[UC_X86_REG_ES] = BUFFER // 16
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert result_flags & 1 == int(vector["result_carry"])

    observed_result = {
        "head": read16(game_expected, routine.head),
        "byte_count": read16(game_expected, routine.byte_count),
        "source_offset": read32(game_expected, routine.source_offset),
        "source_remaining": read32(game_expected, routine.source_remaining),
    }
    row = {
        "name": name,
        "success": success,
        "handle": handle,
        "initial_head": initial_head,
        "initial_byte_count": initial_byte_count,
        "extent": vector["extent"],
        "result": observed_result,
        "result_cursor_segment": machine.reg_read(UC_X86_REG_ES),
        "result_cursor_offset": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        "result_carry": result_flags & 1,
        "result_flags": result_flags & 0xFFFF,
        "transfer_call": {
            key: value for key, value in transfer_calls[0].items() if key != "state"
        },
        "buffer_sha256": hashlib.sha256(
            bytes(buffer_expected) + bytes(extra_expected[:1])
        ).hexdigest(),
    }
    for key in (
        "name",
        "success",
        "handle",
        "initial_head",
        "initial_byte_count",
        "extent",
        "result",
        "result_cursor_segment",
        "result_cursor_offset",
        "result_carry",
    ):
        assert row[key] == vector[key], (
            routine.name,
            name,
            key,
            row[key],
            vector[key],
        )

    normalized_call = dict(transfer_calls[0])
    normalized_call["state"] = tuple(normalized_call["state"])
    canonical = (
        tuple(sorted(normalized_call.items())),
        state_snapshot(game_expected, routine),
        bytes(buffer_expected),
        bytes(extra_expected),
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
        default=Path(__file__).parent / "oracle_vectors/func_a622_natural.json",
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
            "success",
            "handle",
            "initial_head",
            "initial_byte_count",
            "extent",
            "result",
            "result_cursor_segment",
            "result_cursor_offset",
            "result_carry",
            "result_flags",
            "buffer_sha256",
        ):
            assert sequel_row[key] == commander_row[key], (vector["name"], key)
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
        f"verified {len(rows)} BBB presentation entry-read cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
