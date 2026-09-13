#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation queue service with Commander Blood."""

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
RETURN_IP = 0xF300
MALFORMED_ESCAPE_IP = 0x2000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
HELPER_FLAGS = 0x0AD7
REFILL_STEP = 5

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
    ready: int
    advance: int
    refill: int
    palette: int
    present: int
    consume: int
    tail: int
    banked_mode: int
    file_handle: int
    resource_flags: int
    palette_offset: int
    rollover_latch: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA1B4, 0xA20C),
    body_sha256="40c7a6d363d6cebf8c0a8bf5918ec51f867652c93f859d5663c1ee419ee36a97",
    ready=0xA20C,
    advance=0xA240,
    refill=0xA2AB,
    palette=0xA778,
    present=0xA41A,
    consume=0xA3D0,
    tail=0xA1F3,
    banked_mode=0x0DBC,
    file_handle=0x0D5B,
    resource_flags=0x0D76,
    palette_offset=0x0D9E,
    rollover_latch=0x0DAC,
    branches={
        0xA1C1: (0xA1C3, 0xA1D4),
        0xA1C8: (0xA1CA, 0xA1FE),
        0xA1CF: (0xA1D1, 0xA1D4),
        0xA1D7: (0xA1D9, 0xA1DE),
        0xA1E1: (0xA1E3, 0xA1F3),
        0xA1E7: (0xA1E9, 0xA1EC),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB997, 0xB9EF),
    body_sha256="991ee71c903133fceb1100b9e4224f435182284a5d27ca06df8286a215d03ddc",
    ready=0xB9EF,
    advance=0xBA23,
    refill=0xBA95,
    palette=0xBF62,
    present=0xBC04,
    consume=0xBBBA,
    tail=0xB9D6,
    banked_mode=0x100A,
    file_handle=0x0FA9,
    resource_flags=0x0FC4,
    palette_offset=0x0FEC,
    rollover_latch=0x0FFA,
    branches={
        0xB9A4: (0xB9A6, 0xB9B7),
        0xB9AB: (0xB9AD, 0xB9E1),
        0xB9B2: (0xB9B4, 0xB9B7),
        0xB9BA: (0xB9BC, 0xB9C1),
        0xB9C4: (0xB9C6, 0xB9D6),
        0xB9CA: (0xB9CC, 0xB9CF),
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

RETURN_OFFSETS = (32, 35, 40, 45, 56, 60, 63, 74)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def machine_word(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def write_low16(machine: Uc, register: int, value: int) -> None:
    current = machine.reg_read(register)
    machine.reg_write(register, (current & 0xFFFF0000) | (value & 0xFFFF))


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine_word(machine, STACK + sp)
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine_word(machine, STACK + sp)
    return_cs = machine_word(machine, STACK + sp + 2)
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_CS, return_cs)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def canonical_file_address(routine: Routine, image_address: int) -> int:
    file_address = image_address + routine.header_size
    return COMMANDER.span[0] + file_address - routine.span[0]


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    return_map = {
        routine.image_address(routine.span[0] + offset): COMMANDER.image_address(
            COMMANDER.span[0] + offset
        )
        for offset in RETURN_OFFSETS
    }
    for offset in range(STACK_POINTER - 64, STACK_POINTER, 2):
        value = read16(result, offset)
        if value in return_map:
            write16(result, offset, return_map[value])
    return bytes(result)


def canonical_registers(routine: Routine, machine: Uc) -> dict[int, int]:
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    bp = registers[UC_X86_REG_EBP]
    if bp & 0xFFFF == routine.image_address(routine.span[0] + 32):
        registers[UC_X86_REG_EBP] = (bp & 0xFFFF0000) | COMMANDER.image_address(
            COMMANDER.span[0] + 32
        )
    return registers


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
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


def helper_responses(name: str) -> tuple[list[bool], list[bool]]:
    if name == "retry_twice_then_not_due":
        return [False, False, True], [False]
    if name in ("banked_due_without_palette", "file_due_with_palette"):
        return [True], [True]
    return [], []


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(vector["name"])
    malformed = bool(vector["malformed_call_frame"])
    banked_mode = int(vector["banked_mode"])
    file_handle = int(vector["file_handle"])
    resource_flags = int(vector["resource_flags"])
    palette_offset = int(vector["palette_offset"])
    link_target = int(vector["link_target_offset"])
    ready_responses, due_responses = helper_responses(name)

    data_before = seeded_segment(case_index, 17, 0x31)
    data_before[routine.banked_mode] = banked_mode
    write16(data_before, routine.file_handle, file_handle)
    write16(data_before, routine.resource_flags, resource_flags)
    write16(data_before, routine.palette_offset, palette_offset)
    data_before[routine.rollover_latch] = 0xA5
    extra_before = seeded_segment(case_index, 11, 0x53)
    fs_before = seeded_segment(case_index, 7, 0x75)
    game_before = seeded_segment(case_index, 5, 0x97)
    game_before[routine.banked_mode] = 1
    write16(game_before, routine.file_handle, 0xFFFF)
    write16(game_before, routine.resource_flags, 0xFF80)
    write16(game_before, routine.palette_offset, 0)
    game_before[routine.rollover_latch] = 0x3C
    stack_before = seeded_segment(case_index, 3, 0xB9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | (0x0100 + case_index),
        UC_X86_REG_EBX: 0xB6B61234,
        UC_X86_REG_ECX: 0xC7C72345,
        UC_X86_REG_EDX: 0xD8D83456,
        UC_X86_REG_ESI: 0xE9E94567,
        UC_X86_REG_EDI: 0xFAFA5678,
        UC_X86_REG_EBP: 0xABCD0000 | link_target,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
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

    helpers = {
        routine.ready: "list_d8c_activate_ready",
        routine.advance: "list_d8c_advance_due",
        routine.refill: "list_d8c_refill",
        routine.palette: "list_d8c_palette_blocks_apply",
        routine.present: "list_d8c_active_present",
        routine.consume: "queue_d8c_consume",
    }
    calls: list[dict[str, object]] = []
    ready_index = 0
    due_index = 0
    refill_index = 0
    reached_stop = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal ready_index, due_index, refill_index, reached_stop, previous
        stop_address = MALFORMED_ESCAPE_IP if malformed else RETURN_IP
        if address == stop_address:
            reached_stop = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        helper = helpers.get(file_address)
        if helper is not None:
            previous = None
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_ip = machine_word(cpu, STACK + sp)
            call: dict[str, object] = {
                "call": helper,
                "return_ip": canonical_file_address(routine, return_ip),
            }
            if helper == "list_d8c_activate_ready":
                assert ready_index < len(ready_responses), (routine.name, name)
                ready = ready_responses[ready_index]
                ready_index += 1
                call["ready"] = ready
                write_low16(cpu, UC_X86_REG_EAX, 0x2000 + ready_index)
                set_carry(cpu, not ready)
                near_return(cpu)
            elif helper == "list_d8c_advance_due":
                assert due_index < len(due_responses), (routine.name, name)
                due = due_responses[due_index]
                due_index += 1
                call["due"] = due
                write_low16(cpu, UC_X86_REG_EAX, 0x2400 + due_index)
                set_carry(cpu, not due)
                near_return(cpu)
            elif helper == "list_d8c_refill":
                call["link_target_offset"] = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
                call["rollover_latch"] = cpu.mem_read(DATA + routine.rollover_latch, 1)[
                    0
                ]
                write_low16(cpu, UC_X86_REG_EAX, 0xA200 + refill_index)
                refill_index += 1
                write_low16(
                    cpu,
                    UC_X86_REG_EBP,
                    (cpu.reg_read(UC_X86_REG_EBP) + REFILL_STEP) & 0xFFFF,
                )
                cpu.reg_write(UC_X86_REG_EFLAGS, HELPER_FLAGS)
                if call["return_ip"] == 0xA1FE:
                    cpu.mem_write(DATA + routine.rollover_latch, b"\x5a")
                near_return(cpu)
            elif helper == "list_d8c_palette_blocks_apply":
                call["palette_offset"] = palette_offset
                near_return(cpu)
            elif helper == "list_d8c_active_present":
                call["return_cs"] = machine_word(cpu, STACK + sp + 2)
                far_return(cpu)
            else:
                near_return(cpu)
            calls.append(call)
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
        if file_address == routine.tail:
            stack_top = machine_word(cpu, STACK + cpu.reg_read(UC_X86_REG_SP))
            malformed_return = routine.image_address(routine.span[0] + 32)
            if stack_top == malformed_return:
                stack_top = COMMANDER.span[0] + 32
            calls.append(
                {
                    "call": "refill_shared_tail_entry",
                    "stack_top": stack_top,
                }
            )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=500)
    assert reached_stop, (routine.name, name)
    assert ready_index == len(ready_responses), (routine.name, name)
    assert due_index == len(due_responses), (routine.name, name)
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [(routine.rollover_latch, routine.rollover_latch + 1)],
        f"{routine.name} {name}",
    )
    assert data_after[routine.rollover_latch] == 0
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 64] == bytes(
        stack_before[: STACK_POINTER - 64]
    )

    escaped_to = MALFORMED_ESCAPE_IP if malformed else RETURN_IP
    row = {
        "name": name,
        "banked_mode": banked_mode,
        "file_handle": file_handle,
        "resource_flags": resource_flags,
        "palette_offset": palette_offset,
        "link_target_offset": link_target,
        "refill_target_step": REFILL_STEP,
        "calls": calls,
        "malformed_call_frame": malformed,
        "escaped_to": escaped_to,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "result_sp": machine.reg_read(UC_X86_REG_SP),
        "rollover_latch": data_after[routine.rollover_latch],
    }
    assert row == vector, (routine.name, name, row, vector)
    registers = canonical_registers(routine, machine)
    return (
        row,
        registers,
        machine.reg_read(UC_X86_REG_EFLAGS),
        normalized_stack(routine, stack_after),
        bytes((data_after[routine.rollover_latch],)),
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
        default=Path(__file__).parent / "oracle_vectors/func_a1b4_natural.json",
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
        f"verified {len(rows)} BBB presentation-queue service cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
