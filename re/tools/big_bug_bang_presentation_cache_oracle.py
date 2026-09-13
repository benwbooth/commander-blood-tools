#!/usr/bin/env python3
"""Compare BBB's presentation resource-cache initializer with Commander Blood."""

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
EXTRA = 0x50000
FS_DATA = 0x60000
GAME = 0x70000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0xF100
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DESCRIPTOR_BASE = 0x3008
DESCRIPTOR_STRIDE = 0x20
DESCRIPTOR_COUNT = 9

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
    lookup: int
    switch: int
    close: int
    descriptor_table: int
    mode_flags: int
    descriptor_count: int
    range_start: int
    branches: dict[int, tuple[int, ...]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA78C, 0xA7E6),
    body_sha256="022e50d254636e58a3a2fc29340abb123ae2a16db323e7fc4040644db6bf1c09",
    lookup=0x9F80,
    switch=0x9F8E,
    close=0xA141,
    descriptor_table=0x1FB5,
    mode_flags=0x0A9B,
    descriptor_count=0x0DAD,
    range_start=0x0D6E,
    branches={
        0xA792: (0xA794,),
        0xA7A1: (0xA797, 0xA7A3),
        0xA7A9: (0xA7AB, 0xA7BA),
        0xA7B8: (0xA7AD, 0xA7BA),
        0xA7C6: (0xA7BD, 0xA7C8),
        0xA7D5: (0xA7D7, 0xA7E5),
        0xA7DA: (0xA7DC, 0xA7E5),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBF76, 0xBFD0),
    body_sha256="263b286ceba6b82cf9e923613850a7d80cc4c1f453e9e9aacc3268e111197230",
    lookup=0xB763,
    switch=0xB771,
    close=0xB924,
    descriptor_table=0x2203,
    mode_flags=0x0C93,
    descriptor_count=0x0FFB,
    range_start=0x0FBC,
    branches={
        0xBF7C: (0xBF7E,),
        0xBF8B: (0xBF81, 0xBF8D),
        0xBF93: (0xBF95, 0xBFA4),
        0xBFA2: (0xBF97, 0xBFA4),
        0xBFB0: (0xBFA7, 0xBFB2),
        0xBFBF: (0xBFC1, 0xBFCF),
        0xBFC4: (0xBFC6, 0xBFCF),
    },
)

CASES = (
    {
        "name": "mode_zero_mixed_cache",
        "mode_flags": 0,
        "descriptor_count": 9,
        "flags": [0xFF, 0xA5, 0x0C, 0x84, 0x08, 0x0C, 0x00, 0x8C, 0x04],
        "success": [True, True, True, True, True, True],
    },
    {
        "name": "low_mode_clears_dynamic_and_keeps_failures",
        "mode_flags": 1,
        "descriptor_count": 4,
        "flags": [0x88, 0x80, 0x8C, 0x0C, 0x88, 0x08, 0x80, 0x0C, 0x84],
        "success": [False, True, True, False, True, True],
    },
    {
        "name": "bit_two_zero_count_do_while",
        "mode_flags": 2,
        "descriptor_count": 0,
        "flags": [0x80, 0x81, 0x08, 0x88, 0x0C, 0x04, 0x89, 0x08, 0x84],
        "success": [True, False, True, True, False, True],
    },
    {
        "name": "both_mode_bits_clear_all_dynamic",
        "mode_flags": 3,
        "descriptor_count": 9,
        "flags": [0xFF, 0x80, 0x8C, 0x88, 0x84, 0x09, 0x8D, 0x04, 0xFC],
        "success": [False, True, True, True, True, False],
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


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def read32(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def descriptor_offset(index: int) -> int:
    return DESCRIPTOR_BASE + index * DESCRIPTOR_STRIDE


def range_for(case_index: int, resource: int) -> tuple[int, int]:
    return (40 + case_index * 48 + resource * 3, 96 + case_index * 17 + resource * 5)


def set_carry(machine: Uc, value: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, (flags | 1) if value else (flags & ~1))


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    target = struct.unpack("<H", machine.mem_read(STACK + sp, 2))[0]
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, target)


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
    assert len(body) == 90 and body[-1] == 0xC3
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(vector["name"])
    mode_flags = int(vector["mode_flags"])
    count = int(vector["descriptor_count"])
    initial_flags = [int(value) for value in vector["flags"]]
    successes = [bool(value) for value in vector["success"]]

    data_before = seeded_segment(case_index, 17, 0x13)
    extra_before = seeded_segment(case_index, 19, 0x25)
    fs_before = seeded_segment(case_index, 23, 0x37)
    game_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    write16(data_before, routine.mode_flags, mode_flags)
    write16(data_before, routine.descriptor_count, count)
    initial_cached = []
    for resource, flags in enumerate(initial_flags):
        descriptor = descriptor_offset(resource)
        cached = (
            0x1000_0000 + case_index * 0x10000 + resource * 0x101,
            0x2000_0000 + resource,
        )
        initial_cached.append(cached)
        write32(data_before, descriptor - 8, cached[0])
        write32(data_before, descriptor - 4, cached[1])
        data_before[descriptor] = flags
        write16(data_before, routine.descriptor_table + resource * 4, descriptor)
    write32(data_before, routine.range_start, 0xDEAD_BEEF)
    write32(data_before, routine.range_start + 4, 0xCAFE_BABE)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    expected_flags = initial_flags.copy()
    for resource in range(2, 9):
        expected_flags[resource] &= ~0x04
    if mode_flags & 3:
        for resource in range(max(1, count)):
            expected_flags[resource] &= ~0x80
    expected_cached = initial_cached.copy()
    calls = []
    for resource, success in zip(range(2, 8), successes, strict=True):
        selected_range = range_for(case_index, resource)
        calls.append(
            {
                "call": "resource_switch",
                "resource": resource,
                "success": success,
                "range": list(selected_range),
            }
        )
        if success and expected_flags[resource] & 0x08:
            expected_cached[resource] = selected_range
    calls.append({"call": "close_owned_source"})

    initial = {
        UC_X86_REG_EAX: 0xA5A51234 + case_index,
        UC_X86_REG_EBX: 0xB6B62345 + case_index,
        UC_X86_REG_ECX: 0xC7C73456 + case_index,
        UC_X86_REG_EDX: 0xD8D84567 + case_index,
        UC_X86_REG_ESI: 0xE9E95678 + case_index,
        UC_X86_REG_EDI: 0xFAFA6789 + case_index,
        UC_X86_REG_EBP: 0xABCD789A + case_index,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    machine.mem_write(0, module)
    for base, contents in (
        (DATA, data_before),
        (EXTRA, extra_before),
        (FS_DATA, fs_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    observed_calls = []
    switch_index = 0
    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal switch_index, reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.switch:
            resource = cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF
            assert resource == switch_index + 2, (routine.name, name, resource)
            assert cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF == descriptor_offset(resource)
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_file = (
                struct.unpack("<H", cpu.mem_read(STACK + sp, 2))[0]
                + routine.header_size
            )
            assert return_file - routine.span[0] == 0x48, (
                routine.name,
                name,
                return_file,
            )
            selected_range = range_for(case_index, resource)
            cpu.mem_write(
                DATA + routine.range_start, struct.pack("<II", *selected_range)
            )
            success = successes[switch_index]
            switch_index += 1
            observed_calls.append(
                {
                    "call": "resource_switch",
                    "resource": resource,
                    "success": success,
                    "range": list(selected_range),
                }
            )
            set_carry(cpu, not success)
            near_return(cpu)
            previous = None
            return
        if file_address == routine.close:
            assert cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF == 8
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_file = (
                struct.unpack("<H", cpu.mem_read(STACK + sp, 2))[0]
                + routine.header_size
            )
            assert return_file - routine.span[0] == 0x3F, (
                routine.name,
                name,
                return_file,
            )
            observed_calls.append({"call": "close_owned_source"})
            near_return(cpu)
            previous = None
            return
        if routine.span[0] <= file_address < routine.span[1]:
            normalized = file_address - routine.span[0]
            if previous is not None:
                branch = previous + routine.span[0]
                if branch in routine.branches:
                    covered_edges.add((previous, normalized))
            previous = normalized
        else:
            previous = None

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=2000)
    assert reached_return, (routine.name, name)
    assert switch_index == len(successes), (routine.name, name, "switch count")
    assert observed_calls == calls, (routine.name, name, observed_calls, calls)
    assert bytes(machine.mem_read(0, len(module))) == module

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    expected_data = bytearray(data_before)
    for resource, flags in enumerate(expected_flags):
        expected_data[descriptor_offset(resource)] = flags
        write32(
            expected_data, descriptor_offset(resource) - 8, expected_cached[resource][0]
        )
        write32(
            expected_data, descriptor_offset(resource) - 4, expected_cached[resource][1]
        )
    final_range = range_for(case_index, 7)
    write32(expected_data, routine.range_start, final_range[0])
    write32(expected_data, routine.range_start + 4, final_range[1])
    assert data_after == bytes(expected_data), (routine.name, name, "data")
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 64] == bytes(
        stack_before[: STACK_POINTER - 64]
    )

    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers[UC_X86_REG_EAX] == (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 8
    expected_pointer = descriptor_offset(7)
    assert (
        registers[UC_X86_REG_EBX]
        == (initial[UC_X86_REG_EBX] & 0xFFFF0000) | expected_pointer
    )
    assert (
        registers[UC_X86_REG_EDI]
        == (initial[UC_X86_REG_EDI] & 0xFFFF0000) | expected_pointer
    )
    copied_any = any(
        success and expected_flags[resource] & 0x08
        for resource, success in zip(range(2, 8), successes, strict=True)
    )
    expected_si = (
        routine.range_start + 8 if copied_any else initial[UC_X86_REG_ESI] & 0xFFFF
    )
    assert (
        registers[UC_X86_REG_ESI]
        == (initial[UC_X86_REG_ESI] & 0xFFFF0000) | expected_si
    )
    assert registers[UC_X86_REG_ES] == (DATA // 16 if copied_any else EXTRA // 16)
    for register in (
        UC_X86_REG_ECX,
        UC_X86_REG_EDX,
        UC_X86_REG_EBP,
        UC_X86_REG_DS,
        UC_X86_REG_FS,
        UC_X86_REG_GS,
        UC_X86_REG_SS,
        UC_X86_REG_CS,
    ):
        assert registers[register] == initial[register], (routine.name, name, register)
    assert registers[UC_X86_REG_SP] == STACK_POINTER + 2

    normalized_registers = dict(registers)
    normalized_registers[UC_X86_REG_ESI] = (
        (registers[UC_X86_REG_ESI] & 0xFFFF0000) | 8
        if copied_any
        else registers[UC_X86_REG_ESI]
    )
    row = {
        "name": name,
        "mode_flags": mode_flags,
        "descriptor_count": count,
        "initial_flags": initial_flags,
        "result_flags": expected_flags,
        "initial_cached_ranges": [list(value) for value in initial_cached],
        "result_cached_ranges": [list(value) for value in expected_cached],
        "switches": calls[:-1],
        "close_called": True,
    }
    canonical = (
        tuple(expected_flags),
        tuple(expected_cached),
        tuple(
            (call["resource"], call["success"], tuple(call["range"]))
            for call in calls[:-1]
        ),
        tuple(normalized_registers.items()),
        machine.reg_read(UC_X86_REG_EFLAGS),
        bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)),
        bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)),
        bytes(machine.mem_read(GAME, SEGMENT_SIZE)),
        stack_after[STACK_POINTER:],
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
    args = parser.parse_args()

    commander = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel = verify_executable(args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL)
    covered_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(CASES):
        commander_row, commander_result = execute(
            commander, COMMANDER, vector, case_index, covered_edges
        )
        sequel_edges: set[tuple[int, int]] = set()
        sequel_row, sequel_result = execute(
            sequel, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        covered_edges.update(sequel_edges)
        rows.append(sequel_row)

    expected_edges = {
        (address - COMMANDER.span[0], target - COMMANDER.span[0])
        for address, targets in COMMANDER.branches.items()
        for target in targets
    }
    assert covered_edges == expected_edges, (
        sorted(covered_edges),
        sorted(expected_edges),
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB presentation resource-cache cases and {len(covered_edges)} edges"
    )


if __name__ == "__main__":
    main()
