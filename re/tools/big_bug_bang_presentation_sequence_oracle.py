#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation sequence loader with Commander Blood."""

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
BUFFER = 0x40000
EXTRA = 0x50000
FS_DATA = 0x60000
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
    resource_switch: int
    banked_load: int
    activate_entry: int
    active_present: int
    list_init: int
    refill: int
    storage_segment: int
    tick: int
    read_index: int
    wrap_count: int
    resource_flags: int
    tail_pointer: int
    previous_tick: int
    sequence: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA15F, 0xA1B4),
    body_sha256="3e1ca8aa98aaf77324b482441d46184dea56f924e68737181bede861f493c665",
    resource_switch=0x9F8E,
    banked_load=0xA642,
    activate_entry=0xA552,
    active_present=0xA41A,
    list_init=0xA757,
    refill=0xA2AB,
    storage_segment=0x0ABE,
    tick=0x0B29,
    read_index=0x0D60,
    wrap_count=0x0D62,
    resource_flags=0x0D76,
    tail_pointer=0x0D90,
    previous_tick=0x0DA2,
    sequence=0x131C,
    branches={
        0xA16B: (0xA16D, 0xA1AA),
        0xA170: (0xA172, 0xA1AA),
        0xA198: (0xA19A, 0xA1A4),
        0xA1A2: (0xA19D, 0xA1A4),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB942, 0xB997),
    body_sha256="76266f48f89ba1139665d23c268e67e5ba27186449a9b5dde18723062cf6d748",
    resource_switch=0xB771,
    banked_load=0xBE2C,
    activate_entry=0xBD3C,
    active_present=0xBC04,
    list_init=0xBF41,
    refill=0xBA95,
    storage_segment=0x0CB6,
    tick=0x0D33,
    read_index=0x0FAE,
    wrap_count=0x0FB0,
    resource_flags=0x0FC4,
    tail_pointer=0x0FDE,
    previous_tick=0x0FF0,
    sequence=0x156A,
    branches={
        0xB94E: (0xB950, 0xB98D),
        0xB953: (0xB955, 0xB98D),
        0xB97B: (0xB97D, 0xB987),
        0xB985: (0xB980, 0xB987),
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

RETURN_OFFSETS = (12, 17, 32, 36, 40, 66)
TAIL_OFFSET = 0x2340
ENTRY_EXTENT = 0x0137
STORAGE_SEGMENT = 0x4560
REFILL_STEP = 7


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


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(vector["name"])
    switch_succeeds = name != "resource_switch_failure"
    banked_load_succeeds = name not in (
        "resource_switch_failure",
        "banked_list_load_failure",
    )
    resource_id = int(vector["resource_id"])
    resource_flags = int(vector["resource_flags"])
    initial_read_index = 0xFFFE
    initial_sequence = 0x3456
    initial_wrap_count = 0xFFFF
    tick = 0x789A + case_index

    data_before = seeded_segment(case_index, 17, 0x31)
    write16(data_before, routine.storage_segment, STORAGE_SEGMENT)
    write16(data_before, routine.tick, tick)
    write16(data_before, routine.read_index, initial_read_index)
    write16(data_before, routine.wrap_count, initial_wrap_count)
    write16(data_before, routine.resource_flags, resource_flags)
    data_before[routine.tail_pointer : routine.tail_pointer + 4] = struct.pack(
        "<HH", TAIL_OFFSET, BUFFER // 16
    )
    write16(data_before, routine.previous_tick, 0x1111)
    write16(data_before, routine.sequence, initial_sequence)
    buffer_before = seeded_segment(case_index, 11, 0x53)
    write16(buffer_before, TAIL_OFFSET, ENTRY_EXTENT)
    extra_before = seeded_segment(case_index, 7, 0x75)
    fs_before = seeded_segment(case_index, 5, 0x97)
    game_before = seeded_segment(case_index, 3, 0xB9)
    for offset, value in (
        (routine.storage_segment, 0xDEAD),
        (routine.tick, 0x2222),
        (routine.read_index, 0x3333),
        (routine.wrap_count, 0x4444),
        (routine.resource_flags, 0xFFFF),
        (routine.tail_pointer, 0x5555),
        (routine.tail_pointer + 2, 0x6666),
        (routine.previous_tick, 0x7777),
        (routine.sequence, 0x8888),
    ):
        write16(game_before, offset, value)
    stack_before = seeded_segment(case_index, 19, 0xDB)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | resource_id,
        UC_X86_REG_EBX: 0xB6B61234,
        UC_X86_REG_ECX: 0xC7C72345,
        UC_X86_REG_EDX: 0xD8D83456,
        UC_X86_REG_ESI: 0xE9E94567,
        UC_X86_REG_EDI: 0xFAFA5678,
        UC_X86_REG_EBP: 0xABCD6789,
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
    machine.mem_write(BUFFER, bytes(buffer_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    helper_names = {
        routine.resource_switch: "resource_switch",
        routine.banked_load: "banked_list_load",
        routine.activate_entry: "list_d8c_activate_entry",
        routine.active_present: "list_d8c_active_present",
        routine.list_init: "list_d8c_init",
        routine.refill: "list_d8c_refill",
    }
    calls: list[dict[str, object]] = []
    refill_targets: list[int] = []
    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        helper = helper_names.get(file_address)
        if helper is not None:
            previous = None
            return_ip = machine_word(cpu, STACK + cpu.reg_read(UC_X86_REG_SP))
            call: dict[str, object] = {
                "call": helper,
                "return_ip": canonical_file_address(routine, return_ip),
            }
            if helper == "resource_switch":
                call["resource_id"] = cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF
                write_low16(cpu, UC_X86_REG_EAX, 0x9F00 + case_index)
                set_carry(cpu, not switch_succeeds)
                near_return(cpu)
            elif helper == "banked_list_load":
                write_low16(cpu, UC_X86_REG_EAX, 0xA600 + case_index)
                cpu.reg_write(UC_X86_REG_ES, 0x5A00 + case_index)
                write_low16(cpu, UC_X86_REG_ESI, 0x6B00 + case_index)
                set_carry(cpu, not banked_load_succeeds)
                near_return(cpu)
            elif helper == "list_d8c_activate_entry":
                call.update(
                    {
                        "entry_extent": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                        "entry_segment": cpu.reg_read(UC_X86_REG_ES),
                        "entry_offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "storage_segment": cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF,
                    }
                )
                write_low16(cpu, UC_X86_REG_EAX, 0xA552)
                near_return(cpu)
            elif helper == "list_d8c_active_present":
                call["return_cs"] = machine_word(
                    cpu, STACK + cpu.reg_read(UC_X86_REG_SP) + 2
                )
                write_low16(cpu, UC_X86_REG_EAX, 0xA41A)
                far_return(cpu)
            elif helper == "list_d8c_init":
                call["return_cs"] = machine_word(
                    cpu, STACK + cpu.reg_read(UC_X86_REG_SP) + 2
                )
                write_low16(cpu, UC_X86_REG_EAX, 0xA757)
                far_return(cpu)
            else:
                link_target = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
                refill_targets.append(link_target)
                call["link_target_offset"] = link_target
                write_low16(cpu, UC_X86_REG_EBP, link_target + REFILL_STEP)
                write_low16(cpu, UC_X86_REG_EAX, 0xA200 + len(refill_targets))
                write_low16(cpu, UC_X86_REG_EBX, 0xBEEF)
                write_low16(cpu, UC_X86_REG_ECX, 0xDEAD)
                write_low16(cpu, UC_X86_REG_EDX, 0xD00D)
                write_low16(cpu, UC_X86_REG_ESI, 0x5151)
                write_low16(cpu, UC_X86_REG_EDI, 0xD1D1)
                cpu.reg_write(UC_X86_REG_EFLAGS, 0x0AD7)
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

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=1000)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [
            (routine.read_index, routine.read_index + 2),
            (routine.wrap_count, routine.wrap_count + 2),
            (routine.previous_tick, routine.previous_tick + 2),
            (routine.sequence, routine.sequence + 2),
        ],
        f"{routine.name} {name}",
    )
    assert bytes(machine.mem_read(BUFFER, SEGMENT_SIZE)) == bytes(buffer_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 64] == bytes(
        stack_before[: STACK_POINTER - 64]
    )

    successful = switch_succeeds and banked_load_succeeds
    expected_refills = (
        50 if successful and resource_flags.to_bytes(2, "little")[0] & 0x40 == 0 else 0
    )
    expected_targets = [
        (STORAGE_SEGMENT + index * REFILL_STEP) & 0xFFFF
        for index in range(expected_refills)
    ]
    assert refill_targets == expected_targets, (routine.name, name)
    prefix_count = len(calls) - len(refill_targets)
    calls_before_refill = calls[:prefix_count]
    assert calls_before_refill == vector["calls_before_refill"], (routine.name, name)
    for call in calls[prefix_count:]:
        assert call["return_ip"] == 0xA1A1

    state = {
        "read_index": read16(data_after, routine.read_index),
        "sequence": read16(data_after, routine.sequence),
        "wrap_count": read16(data_after, routine.wrap_count),
        "previous_tick": read16(data_after, routine.previous_tick),
    }
    row = {
        "name": name,
        "resource_id": resource_id,
        "resource_flags": resource_flags,
        "calls_before_refill": calls_before_refill,
        "refill_count": len(refill_targets),
        "first_refill_target": refill_targets[0] if refill_targets else None,
        "last_refill_target": refill_targets[-1] if refill_targets else None,
        "refill_target_step": REFILL_STEP if refill_targets else None,
        "result": state,
        "return_carry": machine.reg_read(UC_X86_REG_EFLAGS) & 1,
    }
    assert row == vector, (routine.name, name)
    expected_registers = {register: initial[register] for register in REGISTERS}
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    actual_registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert actual_registers == expected_registers, (
        routine.name,
        name,
        {
            register: (actual_registers[register], expected_registers[register])
            for register in REGISTERS
            if actual_registers[register] != expected_registers[register]
        },
    )
    canonical_state = struct.pack(
        "<4H",
        state["read_index"],
        state["sequence"],
        state["wrap_count"],
        state["previous_tick"],
    )
    return (
        row,
        actual_registers,
        machine.reg_read(UC_X86_REG_EFLAGS),
        normalized_stack(routine, stack_after),
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
        default=Path(__file__).parent / "oracle_vectors/func_a15f_natural.json",
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
        f"verified {len(rows)} BBB presentation-sequence cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
