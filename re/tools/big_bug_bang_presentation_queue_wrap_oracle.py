#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation queue wrap with Commander Blood."""

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
EXTRA = 0x40000
FS_DATA = 0x50000
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
    head: int
    wrap_limit: int
    pending: int
    wrap_count: int
    buffer_end: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA38E, 0xA3AD),
    body_sha256="84de4c19f0e44213424ba2e92b86fe66d3d582c0c42555fe59031b629b10a323",
    head=0x0D8C,
    wrap_limit=0x0D98,
    pending=0x0DA0,
    wrap_count=0x0D62,
    buffer_end=0x5233,
    branches={
        0xA390: (0xA392, 0xA398),
        0xA396: (0xA398, 0xA3A2),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBB78, 0xBB97),
    body_sha256="a7a399ba07876fd3a16620ac3d1bfe6b5ab81133b64f699e7bcfe9ea5e858357",
    head=0x0FDA,
    wrap_limit=0x0FE6,
    pending=0x0FEE,
    wrap_count=0x0FB0,
    buffer_end=0x5603,
    branches={
        0xBB7A: (0xBB7C, 0xBB82),
        0xBB80: (0xBB82, 0xBB8C),
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
    before: bytes, after: bytes, allowed: tuple[tuple[int, int], ...], label: str
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
    assert executable[routine.span[1] - 1] == 0xC3
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(vector["name"])
    cursor = int(vector["cursor"])
    byte_count = int(vector["byte_count"])
    head = int(vector["head"])
    wrap_limit = int(vector["wrap_limit"])
    wrap_count = int(vector["wrap_count"])
    buffer_end = int(vector["buffer_end"])

    data_before = seeded_segment(case_index, 17, 0x31)
    write16(data_before, routine.head, head)
    write16(data_before, routine.wrap_limit, wrap_limit)
    write16(data_before, routine.pending, 0x5A5A)
    write16(data_before, routine.wrap_count, wrap_count)
    write16(data_before, routine.buffer_end, buffer_end)
    extra_before = seeded_segment(case_index, 11, 0x53)
    fs_before = seeded_segment(case_index, 7, 0x75)
    game_before = seeded_segment(case_index, 5, 0x97)
    for offset, value in (
        (routine.head, 0x1111),
        (routine.wrap_limit, 0x2222),
        (routine.pending, 0x3333),
        (routine.wrap_count, 0x4444),
        (routine.buffer_end, 0x5555),
    ):
        write16(game_before, offset, value)
    stack_before = seeded_segment(case_index, 19, 0xB9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | byte_count,
        UC_X86_REG_EBX: 0xB6B62222,
        UC_X86_REG_ECX: 0xC7C73333,
        UC_X86_REG_EDX: 0xD8D84444,
        UC_X86_REG_ESI: 0xE9E90000 | cursor,
        UC_X86_REG_EDI: 0xFAFA6666,
        UC_X86_REG_EBP: 0xABCD7777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
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
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        (
            (routine.head, routine.head + 2),
            (routine.wrap_limit, routine.wrap_limit + 2),
            (routine.pending, routine.pending + 2),
            (routine.wrap_count, routine.wrap_count + 2),
        ),
        f"{routine.name} {name}",
    )
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after == bytes(stack_before)

    row = {
        "name": name,
        "cursor": cursor,
        "byte_count": byte_count,
        "buffer_end": buffer_end,
        "head": head,
        "wrap_limit": wrap_limit,
        "wrap_count": wrap_count,
        "wrapped": bool(vector["wrapped"]),
        "result_cursor": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        "result_head": read16(data_after, routine.head),
        "result_wrap_limit": read16(data_after, routine.wrap_limit),
        "result_iteration_count": read16(data_after, routine.pending),
        "result_wrap_count": read16(data_after, routine.wrap_count),
        "result_carry": machine.reg_read(UC_X86_REG_EFLAGS) & 1,
    }
    assert row == vector, (routine.name, name, row, vector)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | int(
        vector["result_iteration_count"]
    )
    expected_registers[UC_X86_REG_ECX] = (initial[UC_X86_REG_ECX] & 0xFFFF0000) | (
        head if bool(vector["wrapped"]) else 0x3333
    )
    expected_registers[UC_X86_REG_ESI] = (initial[UC_X86_REG_ESI] & 0xFFFF0000) | int(
        vector["result_cursor"]
    )
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers)

    state = struct.pack(
        "<4H",
        read16(data_after, routine.head),
        read16(data_after, routine.wrap_limit),
        read16(data_after, routine.pending),
        read16(data_after, routine.wrap_count),
    )
    return row, registers, machine.reg_read(UC_X86_REG_EFLAGS), stack_after, state


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
        default=Path(__file__).parent / "oracle_vectors/func_a38e_natural.json",
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
        commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        rows.append(sequel_result[0])

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
        f"verified {len(rows)} BBB presentation queue-wrap cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
