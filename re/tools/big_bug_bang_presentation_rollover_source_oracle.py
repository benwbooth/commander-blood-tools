#!/usr/bin/env python3
"""Compare BBB's presentation rollover-source setter with Commander Blood."""

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


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    active_resource: int
    secondary_wrap_limit: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA784, 0xA78C),
    body_sha256="98837ef84fd625927712ecdc43dc4d817c97c906568ebd63ffa9f1fb0ec1348b",
    active_resource=0x0D82,
    secondary_wrap_limit=0x0D66,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBF6E, 0xBF76),
    body_sha256="795e45f7015948f12351c926634946853cb82847dbbb53fce54f41aa030bc880",
    active_resource=0x0FD0,
    secondary_wrap_limit=0x0FB4,
)

CASES = (
    {"name": "zero_resource_and_limit", "active_resource": 0, "secondary_wrap_limit": 0},
    {"name": "ordinary_resource_and_limit", "active_resource": 7, "secondary_wrap_limit": 3},
    {"name": "independent_words", "active_resource": 0x1234, "secondary_wrap_limit": 0xFEDC},
    {"name": "absent_resource_and_limit", "active_resource": 0xFFFF, "secondary_wrap_limit": 0xFFFF},
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
    allowed: tuple[tuple[int, int], ...],
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
    assert len(body) == 8 and body[-1] == 0xC3
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(vector["name"])
    active_resource = int(vector["active_resource"])
    secondary_wrap_limit = int(vector["secondary_wrap_limit"])

    game_before = seeded_segment(case_index, 17, 0x13)
    data_before = seeded_segment(case_index, 19, 0x25)
    extra_before = seeded_segment(case_index, 23, 0x37)
    fs_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    write16(game_before, routine.active_resource, 0xA1A1)
    write16(game_before, routine.secondary_wrap_limit, 0xB2B2)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = STACK_SENTINEL

    game_expected = bytearray(game_before)
    write16(game_expected, routine.active_resource, active_resource)
    write16(game_expected, routine.secondary_wrap_limit, secondary_wrap_limit)

    initial_eax = with_low16(0xA5A50000, secondary_wrap_limit)
    initial_ebx = with_low16(0xB2B20000, active_resource)
    initial = {
        UC_X86_REG_EAX: initial_eax,
        UC_X86_REG_EBX: initial_ebx,
        UC_X86_REG_ECX: 0xC3C33333 + case_index,
        UC_X86_REG_EDX: 0xD4D44444 + case_index,
        UC_X86_REG_ESI: 0xE5E55555 + case_index,
        UC_X86_REG_EDI: 0xF6F66666 + case_index,
        UC_X86_REG_EBP: 0x97977777 + case_index,
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
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=20)
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
    assert_unchanged_outside(
        game_before,
        after[GAME],
        (
            (routine.active_resource, routine.active_resource + 2),
            (routine.secondary_wrap_limit, routine.secondary_wrap_limit + 2),
        ),
        routine.name,
    )
    assert after[GAME] == bytes(game_expected), (routine.name, name, "game")

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags == initial[UC_X86_REG_EFLAGS], (routine.name, name, hex(flags))

    row = {
        "name": name,
        "active_resource": read16(after[GAME], routine.active_resource),
        "secondary_wrap_limit": read16(after[GAME], routine.secondary_wrap_limit),
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "result_bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
        "result_flags": flags & 0xFFFF,
        "return_stack_advance": 2,
    }
    canonical = (
        read16(after[GAME], routine.active_resource),
        read16(after[GAME], routine.secondary_wrap_limit),
        tuple(registers.items()),
        flags,
        after[STACK],
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

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL)
    rows = []
    for case_index, vector in enumerate(CASES):
        commander_row, commander_result = execute(
            commander_executable, COMMANDER, vector, case_index
        )
        sequel_row, sequel_result = execute(sequel_executable, SEQUEL, vector, case_index)
        assert sequel_result == commander_result, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        rows.append(sequel_row)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} BBB presentation rollover-source cases")


if __name__ == "__main__":
    main()
