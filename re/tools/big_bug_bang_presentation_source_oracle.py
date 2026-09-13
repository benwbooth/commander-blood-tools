#!/usr/bin/env python3
"""Compare BBB's queue-change wrapper and source close with Commander Blood."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16, Uc
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
DATA = 0x30000
EXTRA = 0x50000
GAME = 0x70000
FS_DATA = 0x80000
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
    wrapper_span: tuple[int, int]
    wrapper_sha256: str
    close_span: tuple[int, int]
    close_sha256: str
    bounds_span: tuple[int, int]
    bounds_sha256: str
    service: int
    handle: int
    reserved_handle: int
    bounds: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    wrapper_span=(0xA134, 0xA141),
    wrapper_sha256="018b0302640f2cb0b4d8b944c06f0851e4b578a2b045d5891d6652264183ddbb",
    close_span=(0xA141, 0xA15F),
    close_sha256="04deb165f2b81e49c1debc75c0a6481d6b5910c315a03dfef2da7634c8f8e43f",
    bounds_span=(0xA73E, 0xA757),
    bounds_sha256="0e917d5682932dee6f5d2a043ab17eb0e7c422840fa501810af051633ac3e21d",
    service=0xA1B4,
    handle=0x0D5B,
    reserved_handle=0x0A86,
    bounds=0x0D60,
    branches={
        0xA147: (0xA149, 0xA15C),
        0xA14D: (0xA14F, 0xA15C),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    wrapper_span=(0xB917, 0xB924),
    wrapper_sha256="63e0d7d16219a907bc039d4b024b3e264299d0b0b98b3ade137b2f66f2a009fb",
    close_span=(0xB924, 0xB942),
    close_sha256="8294310aa8f0beca4310e5145fcab21e32d555f2eaa139608837499b121ddae5",
    bounds_span=(0xBF28, 0xBF41),
    bounds_sha256="895b533c98a30c20e02c5bc5e460bdca4a02c9c80ffd6aa565026d527028cbe8",
    service=0xB997,
    handle=0x0FA9,
    reserved_handle=0x0C7E,
    bounds=0x0FAE,
    branches={
        0xB92A: (0xB92C, 0xB93F),
        0xB930: (0xB932, 0xB93F),
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

WRAPPER_READ_RESULTS = (0x0000, 0x0001, 0x0001, 0x0002, 0x0000, 0x1234, 0x7FFF)
WRAPPER_READ_INPUTS = (0x0000, 0x0000, 0x0001, 0x0001, 0xFFFF, 0x1234, 0x8000)


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


def final_flags(flags: int) -> dict[str, int]:
    return {
        "carry": (flags >> 0) & 1,
        "parity": (flags >> 2) & 1,
        "auxiliary_carry": (flags >> 4) & 1,
        "zero": (flags >> 6) & 1,
        "sign": (flags >> 7) & 1,
        "overflow": (flags >> 11) & 1,
        "interrupt": (flags >> 9) & 1,
        "direction": (flags >> 10) & 1,
    }


def fixture_flags(flags: int) -> dict[str, int]:
    result = final_flags(flags)
    del result["auxiliary_carry"]
    return result


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
) -> None:
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or any(start <= offset < end for start, end in allowed):
            continue
        raise AssertionError(f"{label} changed outside owned state at {offset:#x}")


def verify_span(
    executable: bytes, span: tuple[int, int], expected_sha256: str, label: str
) -> None:
    digest = hashlib.sha256(executable[slice(*span)]).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"{label} span {span[0]:#x}..{span[1]:#x} changed: {digest}")


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    verify_span(executable, routine.wrapper_span, routine.wrapper_sha256, routine.name)
    verify_span(executable, routine.close_span, routine.close_sha256, routine.name)
    verify_span(executable, routine.bounds_span, routine.bounds_sha256, routine.name)
    for span in (routine.wrapper_span, routine.close_span, routine.bounds_span):
        assert executable[span[1] - 1] == 0xC3
    return executable


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.close_span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_wrapper_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    write16(
        result,
        STACK_POINTER - 4,
        COMMANDER.image_address(COMMANDER.wrapper_span[0] + 7),
    )
    return bytes(result)


def normalized_close_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    if read16(result, STACK_POINTER - 2) == routine.image_address(
        routine.close_span[0] + 27
    ):
        write16(
            result,
            STACK_POINTER - 2,
            COMMANDER.image_address(COMMANDER.close_span[0] + 27),
        )
    return bytes(result)


def initial_machine(
    executable: bytes,
    routine: Routine,
    case_index: int,
    data_before: bytes,
    extra_before: bytes,
    game_before: bytes,
    fs_before: bytes,
    stack_before: bytes,
    registers: dict[int, int],
) -> tuple[Uc, bytes]:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, data_before)
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(GAME, game_before)
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, stack_before)
    for register, value in registers.items():
        machine.reg_write(register, value)
    return machine, bytes(expected_module)


def execute_wrapper(
    executable: bytes,
    routine: Routine,
    case_index: int,
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    read_before = WRAPPER_READ_INPUTS[case_index]
    read_after = WRAPPER_READ_RESULTS[case_index]
    data_before = seeded_segment(case_index, 17, 0x31)
    write16(data_before, routine.bounds, read_before)
    extra_before = seeded_segment(case_index, 11, 0x53)
    write16(extra_before, routine.bounds, 0xDEAD)
    game_before = seeded_segment(case_index, 7, 0x75)
    write16(game_before, routine.bounds, 0xBEEF)
    fs_before = seeded_segment(case_index, 5, 0x97)
    stack_before = seeded_segment(case_index, 3, 0xB9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    registers = {
        UC_X86_REG_EAX: 0xA1A12468,
        UC_X86_REG_EBX: 0xB2B23579,
        UC_X86_REG_ECX: 0xC3C3468A,
        UC_X86_REG_EDX: 0xD4D4579B,
        UC_X86_REG_ESI: 0xE5E568AC,
        UC_X86_REG_EDI: 0xF6F679BD,
        UC_X86_REG_EBP: 0x97978ACE,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0ED7,
    }
    machine, expected_module = initial_machine(
        executable,
        routine,
        case_index,
        bytes(data_before),
        bytes(extra_before),
        bytes(game_before),
        bytes(fs_before),
        bytes(stack_before),
        registers,
    )
    reached_return = False
    service_calls = 0
    entry = routine.image_address(routine.wrapper_span[0])

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, service_calls
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.service:
            service_calls += 1
            sp = cpu.reg_read(UC_X86_REG_SP)
            assert sp == STACK_POINTER - 4
            assert machine_word(cpu, STACK + sp) == routine.image_address(
                routine.wrapper_span[0] + 7
            )
            assert machine_word(cpu, STACK + sp + 2) == read_before
            cpu.mem_write(DATA + routine.bounds, struct.pack("<H", read_after))
            for register, value in (
                (UC_X86_REG_EBX, 0x1357 + case_index),
                (UC_X86_REG_ECX, 0x2468 + case_index),
                (UC_X86_REG_EDX, 0x3579 + case_index),
                (UC_X86_REG_ESI, 0x468A + case_index),
                (UC_X86_REG_EDI, 0x579B + case_index),
                (UC_X86_REG_EBP, 0x68AC + case_index),
            ):
                write_low16(cpu, register, value)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0ED7)
            near_return(cpu)
            return
        assert routine.wrapper_span[0] <= file_address < routine.wrapper_span[1], (
            routine.name,
            hex(file_address),
        )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(entry, 0, count=40)
    assert reached_return and service_calls == 1
    assert bytes(machine.mem_read(0, len(expected_module))) == expected_module
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [(routine.bounds, routine.bounds + 2)],
        routine.name,
    )
    assert read16(data_after, routine.bounds) == read_after
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 4] == bytes(stack_before[: STACK_POINTER - 4])
    assert read16(stack_after, STACK_POINTER - 2) == read_before
    assert machine.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    changed = read_before != read_after
    assert bool((flags >> 6) & 1) is not changed
    row = {
        "read_before": read_before,
        "read_after": read_after,
        "changed": changed,
        "service_calls": service_calls,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "final_flags": final_flags(flags),
    }
    actual_registers = {register: machine.reg_read(register) for register in REGISTERS}
    canonical_state = struct.pack("<H", read16(data_after, routine.bounds))
    return (
        row,
        actual_registers,
        flags,
        normalized_wrapper_stack(routine, stack_after),
        canonical_state,
    )


def execute_close(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    handle = int(vector["initial_handle"])
    reserved = int(vector["reserved_handle"])
    dos_error = vector["dos_error"]
    initial_bounds = (0x1111, 0x2222, 0x3333, 0x4444)
    data_before = seeded_segment(case_index, 17, 0x41)
    write16(data_before, routine.handle, handle)
    write16(data_before, routine.reserved_handle, reserved)
    data_before[routine.bounds : routine.bounds + 8] = struct.pack(
        "<4H", *initial_bounds
    )
    extra_before = seeded_segment(case_index, 11, 0x63)
    write16(extra_before, routine.handle, 0xBCDE)
    write16(extra_before, routine.reserved_handle, 0xABCD)
    game_before = seeded_segment(case_index, 7, 0x85)
    write16(game_before, routine.handle, 0xCDEF)
    fs_before = seeded_segment(case_index, 5, 0xA7)
    stack_before = seeded_segment(case_index, 3, 0xC9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    initial_ax = (0x1200 + case_index * 0x111 + 0x5A) & 0xFFFF
    registers = {
        UC_X86_REG_EAX: 0xA1A10000 | initial_ax,
        UC_X86_REG_EBX: 0xB2B2A55A,
        UC_X86_REG_ECX: 0xC3C3B66B,
        UC_X86_REG_EDX: 0xD4D4C77C,
        UC_X86_REG_ESI: 0xE5E5D88D,
        UC_X86_REG_EDI: 0xF6F6E99E,
        UC_X86_REG_EBP: 0x9797FAAF,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0ED7,
    }
    machine, expected_module = initial_machine(
        executable,
        routine,
        case_index,
        bytes(data_before),
        bytes(extra_before),
        bytes(game_before),
        bytes(fs_before),
        bytes(stack_before),
        registers,
    )
    reached_return = False
    interrupts: list[dict[str, int]] = []
    previous: int | None = None
    entry = routine.image_address(routine.close_span[0])

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if routine.close_span[0] <= file_address < routine.close_span[1]:
            normalized = file_address - routine.close_span[0]
            if (
                previous is not None
                and previous + routine.close_span[0] in routine.branches
            ):
                covered_edges.add((previous, normalized))
            previous = normalized
            return
        previous = None
        assert routine.bounds_span[0] <= file_address < routine.bounds_span[1], (
            routine.name,
            hex(file_address),
        )

    def interrupt(cpu: Uc, number: int, _context) -> None:
        assert number == 0x21
        assert cpu.reg_read(UC_X86_REG_EAX) >> 8 & 0xFF == 0x3E
        interrupts.append(
            {
                "number": number,
                "handle": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                "stored_handle": machine_word(cpu, DATA + routine.handle),
            }
        )
        flags = cpu.reg_read(UC_X86_REG_EFLAGS)
        if dos_error is None:
            cpu.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
        else:
            write_low16(cpu, UC_X86_REG_EAX, int(dos_error))
            cpu.reg_write(UC_X86_REG_EFLAGS, flags | 1)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(entry, 0, count=80)
    assert reached_return
    assert bytes(machine.mem_read(0, len(expected_module))) == expected_module
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [(routine.handle, routine.handle + 2), (routine.bounds, routine.bounds + 8)],
        routine.name,
    )
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 2] == bytes(stack_before[: STACK_POINTER - 2])
    closed = bool(vector["closed"])
    expected_bounds = (0, 0, 0xFFFF, 0xFFFF) if closed else initial_bounds
    assert read16(data_after, routine.handle) == (0 if closed else handle)
    assert struct.unpack_from("<4H", data_after, routine.bounds) == expected_bounds
    assert interrupts == vector["interrupts"]
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    result_registers = {
        "ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
        "cx": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
        "dx": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
        "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
        "bp": machine.reg_read(UC_X86_REG_EBP) & 0xFFFF,
        "sp": machine.reg_read(UC_X86_REG_SP) & 0xFFFF,
        "ds": machine.reg_read(UC_X86_REG_DS),
        "es": machine.reg_read(UC_X86_REG_ES),
        "gs": machine.reg_read(UC_X86_REG_GS),
    }
    row = {
        "name": vector["name"],
        "initial_handle": handle,
        "reserved_handle": reserved,
        "dos_error": dos_error,
        "closed": closed,
        "interrupts": interrupts,
        "result_handle": read16(data_after, routine.handle),
        "result_bounds": list(struct.unpack_from("<4H", data_after, routine.bounds)),
        "result_registers": result_registers,
        "final_flags": fixture_flags(flags),
    }
    actual_registers = {register: machine.reg_read(register) for register in REGISTERS}
    canonical_state = struct.pack(
        "<6H", reserved, read16(data_after, routine.handle), *row["result_bounds"]
    )
    return (
        row,
        actual_registers,
        flags,
        normalized_close_stack(routine, stack_after),
        canonical_state,
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
        default=Path(__file__).parent / "oracle_vectors/func_a141_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    assert len(vectors) == len(WRAPPER_READ_INPUTS)
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(vectors):
        commander_wrapper = execute_wrapper(commander_executable, COMMANDER, case_index)
        sequel_wrapper = execute_wrapper(sequel_executable, SEQUEL, case_index)
        assert sequel_wrapper == commander_wrapper, vector["name"]
        commander_close = execute_close(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_close = execute_close(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        commander_row = commander_close[0]
        sequel_row = sequel_close[0]
        for key in (
            "name",
            "initial_handle",
            "reserved_handle",
            "dos_error",
            "closed",
            "interrupts",
            "result_handle",
            "result_bounds",
            "final_flags",
        ):
            assert commander_row[key] == vector[key], (vector["name"], key)
        for register in ("ax", "bx", "cx", "dx", "si", "di", "bp"):
            assert (
                commander_row["result_registers"][register]
                == vector["result_registers"][register]
            ), (vector["name"], register)
        assert sequel_row == commander_row, vector["name"]
        assert sequel_close[1:] == commander_close[1:], vector["name"]
        sequel_row["queue_change_wrapper"] = sequel_wrapper[0]
        rows.append(sequel_row)

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
        f"verified {len(rows)} BBB source-close cases and queue-change wrappers "
        f"covering {len(sequel_edges)} close branch edges"
    )


if __name__ == "__main__":
    main()
