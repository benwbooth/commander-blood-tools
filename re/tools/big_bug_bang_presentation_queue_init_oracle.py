#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation queue init with Commander Blood."""

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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
EXTRA = 0x70000
FS_DATA = 0x80000
GAME = 0xA0000
STACK = 0xB0000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
CLEARED_SEMANTIC_OFFSETS = (0x0D8C, 0x0D90, 0x0D96, 0x0D9A, 0x0DA0)
PRESERVED_SEMANTIC_OFFSETS = (0x0D94, 0x0D9C, 0x0D9E)


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    buffer_segment: int
    buffer_end: int
    head: int
    head_segment: int
    tail: int
    tail_segment: int
    preserved_one: int
    active: int
    wrap_limit: int
    byte_count: int
    preserved_two: int
    preserved_three: int
    iteration_count: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA757, 0xA778),
    body_sha256="1898e092b06c16b06473284e2f625a6c4aa576718d90f4c44fe81021ad09040f",
    buffer_segment=0x0A7E,
    buffer_end=0x5233,
    head=0x0D8C,
    head_segment=0x0D8E,
    tail=0x0D90,
    tail_segment=0x0D92,
    preserved_one=0x0D94,
    active=0x0D96,
    wrap_limit=0x0D98,
    byte_count=0x0D9A,
    preserved_two=0x0D9C,
    preserved_three=0x0D9E,
    iteration_count=0x0DA0,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBF41, 0xBF62),
    body_sha256="669174423f6374d436eaed9b2d313493ceceee75381b5a14f09226d25c36b2e9",
    buffer_segment=0x0C76,
    buffer_end=0x5603,
    head=0x0FDA,
    head_segment=0x0FDC,
    tail=0x0FDE,
    tail_segment=0x0FE0,
    preserved_one=0x0FE2,
    active=0x0FE4,
    wrap_limit=0x0FE6,
    byte_count=0x0FE8,
    preserved_two=0x0FEA,
    preserved_three=0x0FEC,
    iteration_count=0x0FEE,
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


def with_low16(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def assert_unchanged_outside(
    before: bytes,
    after: bytes,
    allowed: list[tuple[int, int]],
    label: str,
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
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert len(body) == 33 and body[-1] == 0xCB
    return executable


def semantic_words(game: bytes, routine: Routine) -> tuple[int, ...]:
    return tuple(
        read16(game, offset)
        for offset in (
            routine.head,
            routine.head_segment,
            routine.tail,
            routine.tail_segment,
            routine.preserved_one,
            routine.active,
            routine.wrap_limit,
            routine.byte_count,
            routine.preserved_two,
            routine.preserved_three,
            routine.iteration_count,
        )
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(vector["name"])
    base_segment = int(vector["base_segment"])
    buffer_end = int(vector["buffer_end"])

    game_before = seeded_segment(case_index, 17, 0x13)
    data_before = seeded_segment(case_index, 19, 0x25)
    extra_before = seeded_segment(case_index, 23, 0x37)
    fs_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    write16(stack_before, STACK_POINTER + 2, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    write16(game_before, routine.buffer_segment, base_segment)
    write16(game_before, routine.buffer_end, buffer_end)
    initial_words = (0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888, 0x9999, 0xAAAA, 0xBBBB)
    semantic_offsets = (
        routine.head,
        routine.head_segment,
        routine.tail,
        routine.tail_segment,
        routine.preserved_one,
        routine.active,
        routine.wrap_limit,
        routine.byte_count,
        routine.preserved_two,
        routine.preserved_three,
        routine.iteration_count,
    )
    for offset, value in zip(semantic_offsets, initial_words, strict=True):
        write16(game_before, offset, value)

    game_expected = bytearray(game_before)
    for offset in (
        routine.head,
        routine.tail,
        routine.active,
        routine.byte_count,
        routine.iteration_count,
    ):
        write16(game_expected, offset, 0)
    write16(game_expected, routine.head_segment, base_segment)
    write16(game_expected, routine.tail_segment, base_segment)
    write16(game_expected, routine.wrap_limit, buffer_end)

    initial_eax = 0xA5A51234 + case_index
    initial = {
        UC_X86_REG_EAX: initial_eax,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E55555,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: DATA // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xC0000)
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
            return
        file_address = address + routine.header_size
        assert routine.span[0] <= file_address < routine.span[1], (
            routine.name,
            name,
            hex(file_address),
        )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=100)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == module

    after = {
        base: bytes(machine.mem_read(base, SEGMENT_SIZE))
        for base in (DATA, EXTRA, FS_DATA, GAME, STACK)
    }
    for base, expected, label in (
        (DATA, data_before, "GS decoy"),
        (EXTRA, extra_before, "ES decoy"),
        (FS_DATA, fs_before, "FS decoy"),
        (STACK, stack_before, "stack"),
    ):
        assert after[base] == bytes(expected), (routine.name, name, label)
    written_offsets = (
        routine.head,
        routine.head_segment,
        routine.tail,
        routine.tail_segment,
        routine.active,
        routine.wrap_limit,
        routine.byte_count,
        routine.iteration_count,
    )
    assert_unchanged_outside(
        game_before,
        after[GAME],
        [(offset, offset + 2) for offset in written_offsets],
        routine.name,
    )
    assert after[GAME] == bytes(game_expected), (routine.name, name, "game")

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = with_low16(initial_eax, buffer_end)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    flags = machine.reg_read(UC_X86_REG_EFLAGS)

    row = {
        "name": name,
        "base_segment": base_segment,
        "buffer_end": buffer_end,
        "head_pointer": [0, base_segment],
        "tail_pointer": [0, base_segment],
        "cleared_offsets": list(CLEARED_SEMANTIC_OFFSETS),
        "result_wrap_limit": read16(after[GAME], routine.wrap_limit),
        "preserved_sentinel_offsets": list(PRESERVED_SEMANTIC_OFFSETS),
    }
    enriched_row = {
        **row,
        "result_flags": flags & 0xFFFF,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "return_stack_advance": 4,
    }
    canonical = (
        semantic_words(after[GAME], routine),
        tuple(registers.items()),
        flags,
        after[STACK],
    )
    return enriched_row, (row, canonical)


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
        default=Path(__file__).parent / "oracle_vectors/func_a757_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    commander_rows = []
    sequel_rows = []
    for case_index, vector in enumerate(vectors):
        commander_row, (commander_legacy, commander_result) = execute(
            commander_executable, COMMANDER, vector, case_index
        )
        sequel_row, (sequel_legacy, sequel_result) = execute(
            sequel_executable, SEQUEL, vector, case_index
        )
        assert commander_legacy == vector, vector["name"]
        assert sequel_legacy == commander_legacy, vector["name"]
        assert sequel_result == commander_result, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        commander_rows.append(commander_row)
        sequel_rows.append(sequel_row)

    assert sequel_rows == commander_rows
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in sequel_rows
        )
    )
    print(f"verified {len(sequel_rows)} BBB presentation queue-init cases")


if __name__ == "__main__":
    main()
