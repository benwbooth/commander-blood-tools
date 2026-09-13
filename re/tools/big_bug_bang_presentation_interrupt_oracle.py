#!/usr/bin/env python3
"""Compare BBB's far presentation interrupt service with Commander Blood."""

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
RETURN_CS = 0
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
    readiness: int
    advance: int
    present: int
    consume: int
    rollover_state: int
    sound_offset: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA7ED, 0xA824),
    body_sha256="cfadc90a9b437cdbbe2edfef3fd4092375405d657711439d12add0dce53fae8d",
    readiness=0xA20C,
    advance=0xA240,
    present=0xA41A,
    consume=0xA3D0,
    rollover_state=0x0DAC,
    sound_offset=0x0D9E,
    branches={
        0xA801: (0xA803, 0xA816),
        0xA808: (0xA80A, 0xA816),
        0xA80D: (0xA80F, 0xA816),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBFD7, 0xC00E),
    body_sha256="8b2021a4c1b3dd240639b244393042810537b934558a8bde4e56d37351a60e52",
    readiness=0xB9EF,
    advance=0xBA23,
    present=0xBC04,
    consume=0xBBBA,
    rollover_state=0x0FFA,
    sound_offset=0x0FEC,
    branches={
        0xBFEB: (0xBFED, 0xC000),
        0xBFF2: (0xBFF4, 0xC000),
        0xBFF7: (0xBFF9, 0xC000),
    },
)

CASES = (
    {
        "name": "not_ready",
        "rollover_state": 0,
        "readiness": "not_ready",
        "sound_offset": 0xFFFF,
        "due": False,
    },
    {
        "name": "new_activation_defers",
        "rollover_state": 1,
        "readiness": "activated",
        "sound_offset": 0xFFFF,
        "due": False,
    },
    {
        "name": "active_sound_pending",
        "rollover_state": 0x7F,
        "readiness": "active",
        "sound_offset": 0x1234,
        "due": False,
    },
    {
        "name": "active_not_due",
        "rollover_state": 0x80,
        "readiness": "active",
        "sound_offset": 0xFFFF,
        "due": False,
    },
    {
        "name": "active_due",
        "rollover_state": 0xFF,
        "readiness": "active",
        "sound_offset": 0xFFFF,
        "due": True,
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


def write_low16(machine: Uc, register: int, value: int) -> None:
    current = machine.reg_read(register)
    machine.reg_write(register, (current & 0xFFFF0000) | (value & 0xFFFF))


def set_condition(machine: Uc, *, carry: bool, zero: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS) & ~(1 | 0x40)
    if carry:
        flags |= 1
    if zero:
        flags |= 0x40
    machine.reg_write(UC_X86_REG_EFLAGS, flags)


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    target = struct.unpack("<H", machine.mem_read(STACK + sp, 2))[0]
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, target)


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    target_ip, target_cs = struct.unpack("<HH", machine.mem_read(STACK + sp, 4))
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_CS, target_cs)
    machine.reg_write(UC_X86_REG_IP, target_ip)


def callback_clobber(machine: Uc, salt: int) -> None:
    for register, value in (
        (UC_X86_REG_EAX, 0xA000 | salt),
        (UC_X86_REG_EBX, 0xB000 | salt),
        (UC_X86_REG_ECX, 0xC000 | salt),
        (UC_X86_REG_EDX, 0xD000 | salt),
        (UC_X86_REG_ESI, 0xE000 | salt),
        (UC_X86_REG_EDI, 0xF000 | salt),
        (UC_X86_REG_EBP, 0x9000 | salt),
    ):
        write_low16(machine, register, value)
    machine.reg_write(UC_X86_REG_ES, (0x4000 + salt) & 0xFFFF)


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
    assert len(body) == 55 and body[-1] == 0xCB
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(vector["name"])
    rollover_state = int(vector["rollover_state"])
    readiness = str(vector["readiness"])
    sound_offset = int(vector["sound_offset"])
    due = bool(vector["due"])

    data_before = seeded_segment(case_index, 17, 0x13)
    extra_before = seeded_segment(case_index, 19, 0x25)
    fs_before = seeded_segment(case_index, 23, 0x37)
    game_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    game_before[routine.rollover_state] = rollover_state
    write16(game_before, routine.sound_offset, sound_offset)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    write16(stack_before, STACK_POINTER + 2, RETURN_CS)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

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
        UC_X86_REG_EFLAGS: 0x0A93,
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

    calls = []
    reached_return = False
    previous: int | None = None

    def return_offset(cpu: Uc, far: bool = False) -> int:
        sp = cpu.reg_read(UC_X86_REG_SP)
        frame = struct.unpack(
            "<" + ("HH" if far else "H"), cpu.mem_read(STACK + sp, 4 if far else 2)
        )
        if far:
            assert frame[1] == 0, (routine.name, name, frame)
        return frame[0] + routine.header_size - routine.span[0]

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.readiness:
            assert return_offset(cpu) == 0x14
            assert cpu.mem_read(GAME + routine.rollover_state, 1) == b"\0"
            calls.append({"call": "activation_readiness", "result": readiness})
            cpu.mem_write(GAME + routine.rollover_state, b"\x5a")
            callback_clobber(cpu, 1)
            if readiness == "not_ready":
                set_condition(cpu, carry=True, zero=False)
            elif readiness == "activated":
                set_condition(cpu, carry=False, zero=True)
            else:
                assert readiness == "active"
                set_condition(cpu, carry=False, zero=False)
            near_return(cpu)
            previous = None
            return
        if file_address == routine.advance:
            assert return_offset(cpu) == 0x20
            calls.append({"call": "advance_due", "due": due})
            callback_clobber(cpu, 2)
            set_condition(cpu, carry=not due, zero=False)
            near_return(cpu)
            previous = None
            return
        if file_address == routine.present:
            assert return_offset(cpu, far=True) == 0x26
            calls.append({"call": "present_active_entry"})
            callback_clobber(cpu, 3)
            far_return(cpu)
            previous = None
            return
        if file_address == routine.consume:
            assert return_offset(cpu) == 0x29
            calls.append({"call": "consume_entry"})
            callback_clobber(cpu, 4)
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
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=300)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == module

    expected_calls = [{"call": "activation_readiness", "result": readiness}]
    if readiness == "active" and sound_offset == 0xFFFF:
        expected_calls.append({"call": "advance_due", "due": due})
        if due:
            expected_calls.extend(
                [{"call": "present_active_entry"}, {"call": "consume_entry"}]
            )
    assert calls == expected_calls, (routine.name, name, calls, expected_calls)
    game_after = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    game_expected = bytearray(game_before)
    game_expected[routine.rollover_state] = rollover_state
    assert game_after == bytes(game_expected), (routine.name, name, "game")
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 32] == bytes(
        stack_before[: STACK_POINTER - 32]
    )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (
        initial[UC_X86_REG_EAX] & 0xFFFF0000
    ) | rollover_state
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags == initial[UC_X86_REG_EFLAGS], (routine.name, name, hex(flags))

    outcome = (
        "not_ready"
        if readiness == "not_ready"
        else "activated"
        if readiness == "activated"
        else "sound_pending"
        if sound_offset != 0xFFFF
        else "waiting"
        if not due
        else "presented"
    )
    row = {
        "name": name,
        "rollover_state": rollover_state,
        "readiness": readiness,
        "sound_offset": sound_offset,
        "due": due,
        "calls": calls,
        "outcome": outcome,
        "result_ax": registers[UC_X86_REG_EAX] & 0xFFFF,
        "result_flags": flags & 0xFFFF,
    }
    canonical = (
        tuple(tuple(sorted(call.items())) for call in calls),
        outcome,
        tuple(registers.items()),
        flags,
        rollover_state,
        sound_offset,
        bytes(machine.mem_read(DATA, SEGMENT_SIZE)),
        bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)),
        bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)),
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
        f"verified {len(rows)} BBB presentation interrupt cases and {len(covered_edges)} edges"
    )


if __name__ == "__main__":
    main()
