#!/usr/bin/env python3
"""Compare Big Bug Bang's resource-descriptor lookup with Commander Blood."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

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
GAME = 0x60000
FS_DATA = 0x80000
STACK = 0xA0000
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
    table: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0x9F80, 0x9F8E),
    body_sha256="2e3eabf98179886172a5201127a38719a696bdb5eec9828c91d10a9b422ae85a",
    table=0x1FB5,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB763, 0xB771),
    body_sha256="427c8807cfb2a84648814a0dbfb17c96daab6eea131b58d4bfbff515576f1993",
    table=0x2203,
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

CASES = (
    ("first_entry", 0x0000, 0x2069),
    ("second_entry", 0x0001, 0x207F),
    ("ordinary_index_nine", 0x0009, 0x1357),
    ("highest_reachable_word_start", 0x3812, 0x2468),
    ("stride_wraps_to_first", 0x4000, 0x369C),
    ("signed_high_bit", 0x8000, 0x48AD),
    ("addition_overflow", 0x2000, 0x5ABE),
    ("maximum_index", 0xFFFF, 0x6BCF),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def flags_from_add(flags: int) -> dict[str, int]:
    return {
        "carry": (flags >> 0) & 1,
        "parity": (flags >> 2) & 1,
        "auxiliary_carry": (flags >> 4) & 1,
        "zero": (flags >> 6) & 1,
        "sign": (flags >> 7) & 1,
        "overflow": (flags >> 11) & 1,
    }


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], dict[int, int], int, bytes]:
    name, index, record_offset = CASES[case_index]
    assert name == vector["name"] and index == vector["index"]
    table_offset = (routine.table + index * 4) & 0xFFFF
    canonical_offset = (COMMANDER.table + index * 4) & 0xFFFF

    data_before = seeded_segment(case_index, 17, 0x31)
    struct.pack_into("<H", data_before, table_offset, record_offset)
    extra_before = seeded_segment(case_index, 11, 0x53)
    struct.pack_into("<H", extra_before, table_offset, 0xDEAD)
    game_before = seeded_segment(case_index, 7, 0x75)
    struct.pack_into("<H", game_before, table_offset, 0xBEEF)
    fs_before = seeded_segment(case_index, 5, 0x97)
    stack_before = seeded_segment(case_index, 3, 0xB9)
    stack_before[STACK_POINTER : STACK_POINTER + 2] = struct.pack("<H", RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | index,
        UC_X86_REG_EBX: 0xB2B2A55A,
        UC_X86_REG_ECX: 0xC3C31357,
        UC_X86_REG_EDX: 0xD4D42468,
        UC_X86_REG_ESI: 0xE5E5369C,
        UC_X86_REG_EDI: 0xF6F648AD,
        UC_X86_REG_EBP: 0x97975ABE,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0ED7,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    entry = routine.image_address(routine.span[0])
    end = routine.image_address(routine.span[1])

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        assert entry <= address < end, (routine.name, name, hex(address))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(entry, 0, count=20)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    actual_stack = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert actual_stack == bytes(stack_before)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EBX] = (
        initial[UC_X86_REG_EBX] & 0xFFFF0000
    ) | record_offset
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    row = {
        "name": name,
        "index": index,
        "table_offset": canonical_offset,
        "record_offset": record_offset,
        "result_bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
        "flags_from_final_add": flags_from_add(flags),
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    return row, registers, flags, actual_stack


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
        default=Path(__file__).parent / "oracle_vectors/func_9f80_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    rows = []
    for case_index, vector in enumerate(vectors):
        commander_row, commander_registers, commander_flags, commander_stack = execute(
            commander_executable, COMMANDER, vector, case_index
        )
        sequel_row, sequel_registers, sequel_flags, sequel_stack = execute(
            sequel_executable, SEQUEL, vector, case_index
        )
        assert commander_row == vector, vector["name"]
        sequel_semantics = dict(sequel_row)
        sequel_semantics["flags_from_final_add"] = commander_row[
            "flags_from_final_add"
        ]
        assert sequel_semantics == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        arithmetic_flags = 0x08D5
        assert sequel_flags & ~arithmetic_flags == commander_flags & ~arithmetic_flags
        assert sequel_stack == commander_stack, vector["name"]
        rows.append(sequel_row)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} BBB resource-descriptor lookup cases")


if __name__ == "__main__":
    main()
