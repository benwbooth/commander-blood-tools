#!/usr/bin/env python3
"""Compare BBB's presentation payload dispatch with Commander Blood."""

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
SOURCE = 0x20000
DESTINATION = 0x38000
GAME = 0x50000
ALTERNATE = 0x68000
STACK = 0x80000
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
CALLBACK_VALUES = (0xCAFE, 0x1357, 0x2468, 0x369C, 0x48AD, 0x5ABE, 0x6BCF)

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
    decode_ab: int
    decode_ad: int
    mode: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    "Commander Blood",
    0x600,
    (0xA82C, 0xA867),
    "75ac345da65c1258183340faaf1505164d8b4b53fc2f2b1288a95d0b31c730ee",
    0xA867,
    0xA914,
    0x0AA0,
    {0xA83C: (0xA839, 0xA83E), 0xA843: (0xA845, 0xA852), 0xA848: (0xA84A, 0xA84F)},
)

SEQUEL = Routine(
    "Big Bug Bang",
    0x800,
    (0xC016, 0xC051),
    "d512c6352cdf40f1801f0eeccb73ca40695b2776898ab1c7b35d423a98601a2d",
    0xC051,
    0xC0FE,
    0x0C98,
    {0xC026: (0xC023, 0xC028), 0xC02D: (0xC02F, 0xC03C), 0xC032: (0xC034, 0xC039)},
)

CASES = (
    ("ordinary_low", [1, 2, 3, 4, 5, 6], 0x0100, 0x03FF, False, 0x0247),
    ("ordinary_below_ab", [0, 0, 0, 0, 0, 0xAA], 0x1111, 0xFEFF, False, 0x0282),
    ("ordinary_above_ab", [0, 0, 0, 0, 0, 0xAC], 0x2222, 0x0200, False, 0x0212),
    ("checksum_ab", [1, 2, 3, 4, 5, 0x9C], 0x3333, 0xA7FF, False, 0x0AD7),
    ("checksum_ad", [1, 2, 3, 4, 5, 0x9E], 0x4444, 0xB6AA, False, 0x0293),
    (
        "checksum_ab_source_wrap",
        [0x80, 1, 2, 3, 4, 0x21],
        0xFFFD,
        0xFFFF,
        False,
        0x0647,
    ),
    (
        "checksum_ad_sum_wrap",
        [0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xBC],
        0x5555,
        0x1357,
        False,
        0x0202,
    ),
    (
        "checksum_ab_reverse_df",
        [0x20, 0x21, 0x22, 0x23, 0x24, 1],
        0x0002,
        0x0246,
        True,
        0x0603,
    ),
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


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_low16(machine: Uc, register: int, value: int) -> None:
    current = machine.reg_read(register)
    machine.reg_write(register, (current & 0xFFFF0000) | (value & 0xFFFF))


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    target = struct.unpack("<H", machine.mem_read(STACK + sp, 2))[0]
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, target)


def subtract8_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFF
    return {
        "cf": left < right,
        "pf": result.bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x80),
        "of": bool(((left ^ right) & (left ^ result)) & 0x80),
    }


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[slice(*routine.span)]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(f"{routine.name} dispatch body changed: {body_digest}")
    return executable


def execute(executable, routine, case, case_index, covered_edges):
    name, header, source_offset, destination_offset, reverse_df, callback_flags = case
    checksum = sum(header) & 0xFF
    path = "ad" if checksum == 0xAD else "ab" if checksum == 0xAB else "none"
    masked_destination = destination_offset & 0xFDFF
    initial_mode = (0x7100 + case_index * 0x101) & 0xFFFF
    source_before = seeded_segment(case_index, 17, 0x13)
    destination_before = seeded_segment(case_index, 23, 0x25)
    game_before = seeded_segment(case_index, 29, 0x37)
    alternate_before = seeded_segment(case_index, 31, 0x49)
    stack_before = seeded_segment(case_index, 37, 0x5B)
    step = -1 if reverse_df else 1
    for index, value in enumerate(header):
        source_before[(source_offset + index * step) & 0xFFFF] = value
    write16(game_before, routine.mode, initial_mode)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E50000 | source_offset,
        UC_X86_REG_EDI: 0xF6F60000 | destination_offset,
        UC_X86_REG_EBP: 0x97970000 | (ALTERNATE // 16),
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: SOURCE // 16,
        UC_X86_REG_ES: DESTINATION // 16,
        UC_X86_REG_FS: 0x1800,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0602 if reverse_df else 0x0202,
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x90000)
    module = executable[routine.header_size :]
    machine.mem_write(0, module)
    for base, contents in (
        (SOURCE, source_before),
        (DESTINATION, destination_before),
        (GAME, game_before),
        (ALTERNATE, alternate_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls = []
    reached_return = False
    previous = None

    def instruction(cpu, address, _size, _context):
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address in (routine.decode_ab, routine.decode_ad):
            kind = "ab" if file_address == routine.decode_ab else "ad"
            assert kind == path and not calls, (routine.name, name, kind)
            sp = cpu.reg_read(UC_X86_REG_SP)
            frame = struct.unpack("<4H", cpu.mem_read(STACK + sp, 8))
            expected_return = 0x21 if kind == "ab" else 0x32
            assert frame == (
                routine.image_address(routine.span[0] + expected_return),
                masked_destination,
                initial[UC_X86_REG_ECX] & 0xFFFF,
                RETURN_IP,
            ), (routine.name, name, frame)
            expected_es = ALTERNATE // 16 if kind == "ad" else DESTINATION // 16
            assert cpu.reg_read(UC_X86_REG_ES) == expected_es
            assert cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF == source_offset
            assert cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF == masked_destination
            expected_mode = 3 if kind == "ad" else initial_mode
            assert (
                struct.unpack("<H", cpu.mem_read(GAME + routine.mode, 2))[0]
                == expected_mode
            )
            calls.append(kind)
            for register, value in zip(
                (
                    UC_X86_REG_EAX,
                    UC_X86_REG_EBX,
                    UC_X86_REG_ECX,
                    UC_X86_REG_EDX,
                    UC_X86_REG_ESI,
                    UC_X86_REG_EDI,
                    UC_X86_REG_EBP,
                ),
                CALLBACK_VALUES,
                strict=True,
            ):
                write_low16(cpu, register, value)
            cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)
            near_return(cpu)
            previous = None
            return
        if routine.span[0] <= file_address < routine.span[1]:
            normalized = file_address - routine.span[0]
            if previous is not None and previous + routine.span[0] in routine.branches:
                covered_edges.add((previous, normalized))
            previous = normalized
        else:
            previous = None

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=200)
    assert reached_return and calls == ([] if path == "none" else [path])
    assert bytes(machine.mem_read(0, len(module))) == module
    assert bytes(machine.mem_read(SOURCE, SEGMENT_SIZE)) == bytes(source_before)
    assert bytes(machine.mem_read(DESTINATION, SEGMENT_SIZE)) == bytes(
        destination_before
    )
    assert bytes(machine.mem_read(ALTERNATE, SEGMENT_SIZE)) == bytes(alternate_before)
    game_after = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    expected_game = bytearray(game_before)
    if path == "ad":
        write16(expected_game, routine.mode, 3)
    assert game_after == bytes(expected_game)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 16] == bytes(
        stack_before[: STACK_POINTER - 16]
    )

    registers = {register: machine.reg_read(register) for register in REGISTERS}
    expected = dict(initial)
    expected[UC_X86_REG_SP] = STACK_POINTER + 2
    expected[UC_X86_REG_EDI] = (
        expected[UC_X86_REG_EDI] & 0xFFFF0000
    ) | masked_destination
    if path == "none":
        expected[UC_X86_REG_EAX] = (
            (expected[UC_X86_REG_EAX] & 0xFFFF0000) | checksum << 8 | header[-1]
        )
    elif path == "ab":
        for register, value in zip(
            (
                UC_X86_REG_EAX,
                UC_X86_REG_EBX,
                UC_X86_REG_ECX,
                UC_X86_REG_EDX,
                UC_X86_REG_ESI,
                UC_X86_REG_EDI,
                UC_X86_REG_EBP,
            ),
            CALLBACK_VALUES,
            strict=True,
        ):
            expected[register] = (expected[register] & 0xFFFF0000) | value
        expected[UC_X86_REG_ECX] = initial[UC_X86_REG_ECX]
        expected[UC_X86_REG_EDI] = (
            initial[UC_X86_REG_EDI] & 0xFFFF0000
        ) | masked_destination
        expected[UC_X86_REG_ESI] = (
            initial[UC_X86_REG_ESI] & 0xFFFF0000
        ) | CALLBACK_VALUES[5]
    else:
        for register, value in zip(
            (
                UC_X86_REG_EAX,
                UC_X86_REG_EBX,
                UC_X86_REG_ECX,
                UC_X86_REG_EDX,
                UC_X86_REG_ESI,
                UC_X86_REG_EDI,
                UC_X86_REG_EBP,
            ),
            CALLBACK_VALUES,
            strict=True,
        ):
            expected[register] = (expected[register] & 0xFFFF0000) | value
        expected[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | (
            ALTERNATE // 16
        )
        expected[UC_X86_REG_ECX] = initial[UC_X86_REG_ECX]
        expected[UC_X86_REG_EDI] = (
            initial[UC_X86_REG_EDI] & 0xFFFF0000
        ) | masked_destination
        expected[UC_X86_REG_ESI] = initial[UC_X86_REG_ESI] & 0xFFFF0000
        expected[UC_X86_REG_DS] = ALTERNATE // 16
        expected[UC_X86_REG_ES] = ALTERNATE // 16
    assert registers == {register: expected[register] for register in REGISTERS}, (
        routine.name,
        name,
    )

    masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    if path == "none":
        defined_flags = subtract8_flags(checksum, 0xAB)
    elif path == "ab":
        defined_flags = {
            flag: bool(callback_flags & mask) for flag, mask in masks.items()
        }
    else:
        defined_flags = {"cf": False, "pf": True, "zf": True, "sf": False, "of": False}
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert {flag: bool(flags & masks[flag]) for flag in defined_flags} == defined_flags

    row = {
        "name": name,
        "header_bytes_in_read_order": header,
        "checksum": checksum,
        "direction": "backward" if reverse_df else "forward",
        "path": path,
        "masked_destination_offset": masked_destination,
        "source_result_offset": 0
        if path == "ad"
        else CALLBACK_VALUES[5]
        if path == "ab"
        else source_offset,
        "source_result_segment": ALTERNATE // 16 if path == "ad" else SOURCE // 16,
        "destination_result_segment": ALTERNATE // 16
        if path == "ad"
        else DESTINATION // 16,
        "destination_result_offset": masked_destination,
        "mode_before": initial_mode,
        "mode_after": 3 if path == "ad" else initial_mode,
        "defined_flags": defined_flags,
    }
    canonical = (
        row["checksum"],
        row["path"],
        row["mode_after"],
        tuple(registers.items()),
        tuple(sorted(defined_flags.items())),
        bytes(machine.mem_read(SOURCE, SEGMENT_SIZE)),
        bytes(machine.mem_read(DESTINATION, SEGMENT_SIZE)),
        bytes(machine.mem_read(ALTERNATE, SEGMENT_SIZE)),
        stack_after[STACK_POINTER:],
    )
    return row, canonical


def main():
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
    covered_edges = set()
    rows = []
    for case_index, case in enumerate(CASES):
        commander_row, commander_result = execute(
            commander, COMMANDER, case, case_index, covered_edges
        )
        sequel_edges = set()
        sequel_row, sequel_result = execute(
            sequel, SEQUEL, case, case_index, sequel_edges
        )
        assert sequel_result == commander_result, case[0]
        assert sequel_row == commander_row, case[0]
        covered_edges.update(sequel_edges)
        rows.append(sequel_row)
    expected_edges = {
        (a - COMMANDER.span[0], t - COMMANDER.span[0])
        for a, targets in COMMANDER.branches.items()
        for t in targets
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
        f"verified {len(rows)} BBB presentation dispatch cases and {len(covered_edges)} edges"
    )


if __name__ == "__main__":
    main()
