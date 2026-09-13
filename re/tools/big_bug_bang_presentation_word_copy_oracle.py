#!/usr/bin/env python3
"""Compare BBB's fixed four-word copy helper with Commander Blood."""

# ruff: noqa: E402

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import argparse
import hashlib
import json
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
EXTRA = 0x50000
FS_DATA = 0x60000
GAME = 0x70000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0xF100
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
SOURCE_WORDS = (0x1122, 0x3344, 0x5566, 0x7788)

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
BODY_SHA256 = "6aa5c60d59aa4dd835e5df01e31aca24da6cd83fb35b6b96dfb1381bbf9de5b2"


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine("Commander Blood", 0x600, (0xA7E6, 0xA7ED))
SEQUEL = Routine("Big Bug Bang", 0x800, (0xBFD0, 0xBFD7))

CASES = (
    ("disjoint", 0x0100, 0x0200),
    ("same_pointer", 0x0100, 0x0100),
    ("forward_overlap", 0x0100, 0x0102),
    ("backward_overlap", 0x0102, 0x0100),
    ("source_offset_wrap", 0xFFFC, 0x0200),
    ("destination_offset_wrap", 0x0100, 0xFFFC),
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


def read_word(data: bytes | bytearray, offset: int) -> int:
    return data[offset] | data[(offset + 1) & 0xFFFF] << 8


def write_word(data: bytearray, offset: int, value: int) -> None:
    data[offset] = value & 0xFF
    data[(offset + 1) & 0xFFFF] = value >> 8


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert body == bytes.fromhex("1e07a5a5a5a5c3")
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    case: tuple[str, int, int],
    case_index: int,
) -> tuple[dict[str, object], tuple[object, ...]]:
    name, source_offset, destination_offset = case
    data_before = seeded_segment(case_index, 17, 0x13)
    extra_before = seeded_segment(case_index, 19, 0x25)
    fs_before = seeded_segment(case_index, 23, 0x37)
    game_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    for index, value in enumerate(SOURCE_WORDS):
        write_word(data_before, (source_offset + index * 2) & 0xFFFF, value)
    write_word(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    stack_expected = bytearray(stack_before)
    write_word(stack_expected, STACK_POINTER - 2, DATA // 16)

    expected_data = bytearray(data_before)
    copied_words = []
    for index in range(4):
        source = (source_offset + index * 2) & 0xFFFF
        destination = (destination_offset + index * 2) & 0xFFFF
        value = read_word(expected_data, source)
        copied_words.append(value)
        write_word(expected_data, destination, value)

    initial = {
        UC_X86_REG_EAX: 0xA5A51234 + case_index,
        UC_X86_REG_EBX: 0xB6B62345 + case_index,
        UC_X86_REG_ECX: 0xC7C73456 + case_index,
        UC_X86_REG_EDX: 0xD8D84567 + case_index,
        UC_X86_REG_ESI: 0xE9E90000 | source_offset,
        UC_X86_REG_EDI: 0xFAFA0000 | destination_offset,
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

    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=20)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == module
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(expected_data)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert bytes(machine.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_ESI] = (initial[UC_X86_REG_ESI] & 0xFFFF0000) | (
        (source_offset + 8) & 0xFFFF
    )
    expected_registers[UC_X86_REG_EDI] = (initial[UC_X86_REG_EDI] & 0xFFFF0000) | (
        (destination_offset + 8) & 0xFFFF
    )
    expected_registers[UC_X86_REG_ES] = DATA // 16
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags == initial[UC_X86_REG_EFLAGS], (routine.name, name, hex(flags))

    row = {
        "name": name,
        "source_offset": source_offset,
        "destination_offset": destination_offset,
        "copied_words_in_order": copied_words,
        "result_source_offset": registers[UC_X86_REG_ESI] & 0xFFFF,
        "result_destination_offset": registers[UC_X86_REG_EDI] & 0xFFFF,
        "result_es": registers[UC_X86_REG_ES],
        "preserved_flags": flags & 0xFFFF,
    }
    canonical = (
        bytes(machine.mem_read(DATA, SEGMENT_SIZE)),
        tuple(registers.items()),
        flags,
        bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)),
        bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)),
        bytes(machine.mem_read(GAME, SEGMENT_SIZE)),
        bytes(machine.mem_read(STACK, SEGMENT_SIZE)),
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
    rows = []
    for case_index, case in enumerate(CASES):
        commander_row, commander_result = execute(
            commander, COMMANDER, case, case_index
        )
        sequel_row, sequel_result = execute(sequel, SEQUEL, case, case_index)
        assert sequel_result == commander_result, case[0]
        assert sequel_row == commander_row, case[0]
        rows.append(sequel_row)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} BBB presentation four-word copy cases")


if __name__ == "__main__":
    main()
