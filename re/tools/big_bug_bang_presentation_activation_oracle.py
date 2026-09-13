#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation activation request with Commander Blood."""

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
BUFFER = 0x30000
EXTRA = 0x40000
FS_DATA = 0x60000
GAME = 0x70000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DEFAULT_STORAGE_SEGMENT = 0x4567
ALTERNATE_STORAGE_SEGMENT = 0x5678

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
    activate: int
    default_storage: int
    resource_flags: int
    tail_pointer: int
    active_segment: int
    queued_bytes: int
    alternate_storage: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA20C, 0xA240),
    body_sha256="8c2be585dd7b3567cf6c2e7a3472cdc57ea5657c6c2366f9f2a85681837f2f33",
    activate=0xA552,
    default_storage=0x0ABE,
    resource_flags=0x0D76,
    tail_pointer=0x0D90,
    active_segment=0x0D96,
    queued_bytes=0x0D9A,
    alternate_storage=0x0DA8,
    branches={
        0xA211: (0xA213, 0xA23F),
        0xA218: (0xA21A, 0xA23F),
        0xA225: (0xA227, 0xA22B),
        0xA229: (0xA22B, 0xA23F),
        0xA234: (0xA236, 0xA23A),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB9EF, 0xBA23),
    body_sha256="2f8874819169b2f8dc396e4b45f9b01f5a0cdefaff97ce1f7b184f8f1ed74b17",
    activate=0xBD3C,
    default_storage=0x0CB6,
    resource_flags=0x0FC4,
    tail_pointer=0x0FDE,
    active_segment=0x0FE4,
    queued_bytes=0x0FE8,
    alternate_storage=0x0FF6,
    branches={
        0xB9F4: (0xB9F6, 0xBA22),
        0xB9FB: (0xB9FD, 0xBA22),
        0xBA08: (0xBA0A, 0xBA0E),
        0xBA0C: (0xBA0E, 0xBA22),
        0xBA17: (0xBA19, 0xBA1D),
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

TAILS = {
    "already_active": 0x0100,
    "empty_queue": 0x0200,
    "ordinary_incomplete": 0x0300,
    "ordinary_exact_default_storage": 0x0400,
    "ordinary_extra_alternate_storage": 0x0500,
    "link_marker_bypasses_extent_check": 0x0600,
    "tail_offset_wrap": 0xFFFE,
}


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_circular_word(data: bytearray, offset: int, value: int) -> None:
    encoded = struct.pack("<H", value & 0xFFFF)
    data[offset & 0xFFFF] = encoded[0]
    data[(offset + 1) & 0xFFFF] = encoded[1]


def machine_word(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine_word(machine, STACK + sp)
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    helper_return = routine.image_address(routine.span[0] + 49)
    if read16(result, STACK_POINTER - 2) == helper_return:
        write16(
            result,
            STACK_POINTER - 2,
            COMMANDER.image_address(COMMANDER.span[0] + 49),
        )
    return bytes(result)


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
) -> tuple[dict[str, object], dict[int, int], int, bytes]:
    name = str(vector["name"])
    tail = TAILS[name]
    active_segment = int(vector["active_segment"])
    queued_bytes = int(vector["queued_bytes"])
    entry_extent = int(vector["entry_extent"])
    entry_marker = int(vector["entry_marker"])
    resource_flags = int(vector["resource_flags"])

    data_before = seeded_segment(case_index, 17, 0x31)
    write16(data_before, routine.default_storage, DEFAULT_STORAGE_SEGMENT)
    write16(data_before, routine.resource_flags, resource_flags)
    data_before[routine.tail_pointer : routine.tail_pointer + 4] = struct.pack(
        "<HH", tail, BUFFER // 16
    )
    write16(data_before, routine.active_segment, active_segment)
    write16(data_before, routine.queued_bytes, queued_bytes)
    write16(data_before, routine.alternate_storage, ALTERNATE_STORAGE_SEGMENT)

    buffer_before = seeded_segment(case_index, 11, 0x53)
    write_circular_word(buffer_before, tail, entry_extent)
    write_circular_word(buffer_before, tail + 2, entry_marker)
    extra_before = seeded_segment(case_index, 7, 0x75)
    fs_before = seeded_segment(case_index, 5, 0x97)
    game_before = seeded_segment(case_index, 3, 0xB9)
    for offset, value in (
        (routine.default_storage, 0xDEAD),
        (routine.resource_flags, 0xFFFF),
        (routine.tail_pointer, 0x1111),
        (routine.tail_pointer + 2, 0x2222),
        (routine.active_segment, 0x3333),
        (routine.queued_bytes, 0x4444),
        (routine.alternate_storage, 0xBEEF),
    ):
        write16(game_before, offset, value)

    stack_before = seeded_segment(case_index, 19, 0xDB)
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
        UC_X86_REG_EBP: 0xABCD6789,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(BUFFER, bytes(buffer_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, int | str]] = []
    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.activate:
            previous = None
            assert machine_word(cpu, STACK + cpu.reg_read(UC_X86_REG_SP)) == (
                routine.image_address(routine.span[0] + 49)
            )
            calls.append(
                {
                    "call": "activate",
                    "extent": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "entry_segment": cpu.reg_read(UC_X86_REG_ES),
                    "entry_offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    "storage_segment": cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF,
                }
            )
            near_return(cpu)
            return
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
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(BUFFER, SEGMENT_SIZE)) == bytes(buffer_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 64] == bytes(
        stack_before[: STACK_POINTER - 64]
    )

    expected_calls = vector["calls"]
    assert calls == expected_calls, (routine.name, name, calls, expected_calls)
    row = {
        "name": name,
        "ready": bool(vector["ready"]),
        "active_segment": active_segment,
        "queued_bytes": queued_bytes,
        "entry_extent": entry_extent,
        "entry_marker": entry_marker,
        "resource_flags": resource_flags,
        "calls": calls,
        "result_carry": machine.reg_read(UC_X86_REG_EFLAGS) & 1,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
    }
    assert row == vector, (routine.name, name, row, vector)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    if active_segment == 0:
        expected_registers[UC_X86_REG_ECX] = (
            initial[UC_X86_REG_ECX] & 0xFFFF0000
        ) | queued_bytes
        if queued_bytes != 0:
            expected_registers[UC_X86_REG_EAX] = (
                initial[UC_X86_REG_EAX] & 0xFFFF0000
            ) | int(vector["result_ax"])
            expected_registers[UC_X86_REG_ES] = BUFFER // 16
            expected_registers[UC_X86_REG_ESI] = (
                initial[UC_X86_REG_ESI] & 0xFFFF0000
            ) | ((tail + 2) & 0xFFFF)
            if calls:
                expected_registers[UC_X86_REG_EBP] = (
                    initial[UC_X86_REG_EBP] & 0xFFFF0000
                ) | int(calls[0]["storage_segment"])
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    expected_result_registers = {
        register: expected_registers[register] for register in REGISTERS
    }
    assert registers == expected_result_registers, (
        routine.name,
        name,
        registers,
        expected_result_registers,
    )
    return (
        row,
        registers,
        machine.reg_read(UC_X86_REG_EFLAGS),
        normalized_stack(routine, stack_after),
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
        default=Path(__file__).parent / "oracle_vectors/func_a20c_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    assert [str(vector["name"]) for vector in vectors] == list(TAILS)
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
        f"verified {len(rows)} BBB presentation activation cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
