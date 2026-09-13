#!/usr/bin/env python3
"""Compare Big Bug Bang's queue-state predicate with Commander Blood."""

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
    state: int
    branch: tuple[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA40B, 0xA41A),
    body_sha256="849d11b4ac03e9e5ba5e20040766d551aeb86a1ade414058cddafd2d98ec8901",
    state=0x0D5F,
    branch=(0xA411, (0xA413, 0xA419)),
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBBF5, 0xBC04),
    body_sha256="0bc03e9897f972522b6ab265ea5aef10f756a906a873c037c9d9fb37471a67ee",
    state=0x0FAD,
    branch=(0xBBFB, (0xBBFD, 0xBC03)),
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


def seeded_segment(state: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + state * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    source, destinations = routine.branch
    return {
        (source - routine.span[0], destination - routine.span[0])
        for destination in destinations
    }


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
    state: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[bool, dict[int, int], int, bytes]:
    data_before = seeded_segment(state, 17, 0x31)
    extra_before = seeded_segment(state, 11, 0x53)
    fs_before = seeded_segment(state, 7, 0x75)
    game_before = seeded_segment(state, 5, 0x97)
    game_before[routine.state] = state
    data_before[routine.state] = state ^ 0x55
    extra_before[routine.state] = state ^ 0xAA
    fs_before[routine.state] = state ^ 0xFF
    stack_before = seeded_segment(state, 19, 0xB9)
    struct.pack_into("<H", stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A51111,
        UC_X86_REG_EBX: 0xB6B62222,
        UC_X86_REG_ECX: 0xC7C73333,
        UC_X86_REG_EDX: 0xD8D84444,
        UC_X86_REG_ESI: 0xE9E95555,
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
            state,
            hex(file_address),
        )
        normalized = file_address - routine.span[0]
        if previous == routine.branch[0] - routine.span[0]:
            covered_edges.add((previous, normalized))
        previous = normalized

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=10)
    assert reached_return, (routine.name, state)
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after == bytes(stack_before)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, state, registers)

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    zero_flag = bool(flags & (1 << 6))
    assert zero_flag == (state <= 1), (routine.name, state, flags)
    return zero_flag, registers, flags, stack_after


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
        default=Path(__file__).parent / "oracle_vectors/func_a40b_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    expected_summary = json.loads(args.commander_vectors.read_text())
    assert len(expected_summary) == 1

    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    zero_flag_states = []
    for state in range(0x100):
        commander_result = execute(
            commander_executable, COMMANDER, state, commander_edges
        )
        sequel_result = execute(sequel_executable, SEQUEL, state, sequel_edges)
        assert sequel_result == commander_result, state
        if sequel_result[0]:
            zero_flag_states.append(state)

    summary = [
        {
            "tested_state_count": 0x100,
            "zero_flag_set_states": zero_flag_states,
            "zero_flag_clear_range": [2, 0xFF],
            "logical_result": "state <= 1",
        }
    ]
    assert summary == expected_summary
    assert commander_edges == normalized_edges(COMMANDER)
    assert sequel_edges == normalized_edges(SEQUEL)
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in summary
        )
    )
    print(
        "verified all 256 BBB presentation queue-state values covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
