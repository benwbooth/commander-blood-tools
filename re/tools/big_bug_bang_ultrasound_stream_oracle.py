#!/usr/bin/env python3
"""Exercise Big Bug Bang's Gravis Ultrasound stream controller."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
    UC_X86_REG_AX,
    UC_X86_REG_CS,
    UC_X86_REG_DI,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ROUTINES = {
    "start": {
        "start": 0xD9F3,
        "stop": 0xDA5F,
        "sha256": "6e01af39a1e621d157a9e3162cff7faca3f90d7cb83c6c8c01f96f6e66a3d7e5",
        "instruction_count": 42,
    },
    "service": {
        "start": 0xDA5F,
        "stop": 0xDB08,
        "sha256": "b9e862683efd12897fca1bdbe5e3dca15d6f8ab8ccbdd2f28f4bcfa2ab78fc32",
        "instruction_count": 59,
    },
    "interrupt": {
        "start": 0xE0ED,
        "stop": 0xE186,
        "sha256": "f8fd1aaa9b164e873ac17c2f82b055d33aa0338374499c76d12e8033c5d4a309",
        "instruction_count": 66,
    },
}

FIELDS = {
    "sound": 0x0CE7,
    "first_buffer": 0x0D93,
    "second_buffer": 0x0D9B,
    "pending": 0x0DAA,
    "packed": 0x0DAC,
    "channel": 0x0DAD,
    "page": 0x0DAF,
    "page_count": 0x0DB1,
    "final_length": 0x0DB3,
    "base_port": 0x0EE5,
    "irq": 0x0EF1,
    "refill": 0x0F20,
}

STOP_HELPER = 0xDDD2
PAGE_HELPER = 0xDBD4
SUBMIT_HELPER = 0xDB08
DELAY_HELPER = 0xE0DE
GAME_SEGMENT = 0x2000
HEADER_SEGMENT = 0x3000
DATA_SEGMENT = 0x4000
FS_SEGMENT = 0x5000
STACK_SEGMENT = 0x6000
UNOWNED_SEGMENT = 0x7000
HEADER_OFFSET = 0x1200
STACK_POINTER = 0xFF00
RETURN_IP = 0x7400
SEGMENT_SIZE = 0x10000
MACHINE_SIZE = 0x90000
FLAG_MASK = 0x08D5

REGISTER_IDS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}

START_CASES = (
    {"name": "unpacked_header", "packed": False},
    {"name": "packed_header", "packed": True},
)

SERVICE_CASES = (
    {
        "name": "ultrasound_disabled",
        "ultrasound": 0,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 0,
        "states": [0, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "sound_disabled",
        "ultrasound": 1,
        "sound": 0,
        "channel": 1,
        "pending": 2,
        "refill": 0,
        "states": [0, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "channel_disabled",
        "ultrasound": 1,
        "sound": 1,
        "channel": 0,
        "pending": 2,
        "refill": 0,
        "states": [0, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "stream_inactive",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 0,
        "refill": 0,
        "states": [0, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "restart_first_ready_then_load_second",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 1,
        "states": [1, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "restart_second_ready_then_load_first",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 1,
        "states": [0, 1],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "restart_without_ready_buffer",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 1,
        "states": [0, 0],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "both_buffers_owned",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 0,
        "states": [2, 2],
        "page": 0,
        "page_count": 3,
    },
    {
        "name": "first_free_final_page",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 0,
        "states": [0, 2],
        "page": 1,
        "page_count": 2,
    },
    {
        "name": "second_free_intermediate_page",
        "ultrasound": 1,
        "sound": 1,
        "channel": 1,
        "pending": 2,
        "refill": 0,
        "states": [2, 0],
        "page": 0,
        "page_count": 3,
    },
)

INTERRUPT_CASES = (
    {
        "name": "unrelated_dma_interrupt",
        "status": 0,
        "voice": 1,
        "irq": 5,
        "states": [2, 1],
    },
    {
        "name": "other_voice_interrupt",
        "status": 0x20,
        "voice": 3,
        "irq": 11,
        "states": [2, 1],
    },
    {
        "name": "first_finished_second_ready",
        "status": 0x20,
        "voice": 1,
        "irq": 5,
        "states": [2, 1],
    },
    {
        "name": "first_finished_second_empty",
        "status": 0x20,
        "voice": 1,
        "irq": 11,
        "states": [2, 0],
    },
    {
        "name": "second_finished_first_ready",
        "status": 0x20,
        "voice": 1,
        "irq": 5,
        "states": [1, 2],
    },
    {
        "name": "second_finished_first_empty",
        "status": 0x20,
        "voice": 1,
        "irq": 11,
        "states": [0, 2],
    },
    {
        "name": "voice_interrupt_without_owned_buffer",
        "status": 0x20,
        "voice": 1,
        "irq": 5,
        "states": [0, 0],
    },
)


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (index * 73 + seed * 41 + 19) & 0xFF for index in range(SEGMENT_SIZE)
    )


def initial_registers(case_index: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A10000 | case_index,
        UC_X86_REG_EBX: 0xB2B23456,
        UC_X86_REG_ECX: 0xC3C34567,
        UC_X86_REG_EDX: 0xD4D45678,
        UC_X86_REG_ESI: 0xE5E589AB,
        UC_X86_REG_EDI: 0xF6F61234,
        UC_X86_REG_EBP: 0x97975678,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: HEADER_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202 | (case_index & 1),
    }


def write_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def read_word(machine: Uc, offset: int) -> int:
    return struct.unpack("<H", machine.mem_read(GAME_SEGMENT * 16 + offset, 2))[0]


def read_dword(machine: Uc, offset: int) -> int:
    return struct.unpack("<I", machine.mem_read(GAME_SEGMENT * 16 + offset, 4))[0]


def descriptor(
    data: bytearray, offset: int, address: int, length: int, state: int
) -> None:
    write_dword(data, offset, address)
    write_word(data, offset + 4, length)
    data[offset + 6] = state


def conditional_edges(executable: bytes, start: int, stop: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    edges: set[tuple[int, int]] = set()
    for instruction in decoder.disasm(executable[start:stop], start):
        encoded = instruction.bytes
        conditional = 0x70 <= encoded[0] <= 0x7F or (
            encoded[0] == 0x0F and 0x80 <= encoded[1] <= 0x8F
        )
        if not conditional:
            continue
        target = int(instruction.operands[0].imm)
        edges.add((instruction.address, target))
        edges.add((instruction.address, instruction.address + instruction.size))
    return edges


def map_machine(
    executable: bytes,
    game: bytearray,
    header: bytearray,
    data: bytearray,
    fs_data: bytearray,
    stack: bytearray,
    unowned: bytearray,
    initial: dict[int, int],
    patches: tuple[int, ...],
) -> tuple[Uc, bytes]:
    patched = bytearray(executable)
    for address in patches:
        patched[address] = 0xC3
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(patched))
    for segment, contents in (
        (GAME_SEGMENT, game),
        (HEADER_SEGMENT, header),
        (DATA_SEGMENT, data),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    ):
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)
    return machine, bytes(patched)


def register_state(machine: Uc) -> dict[str, int]:
    return {name: machine.reg_read(register) for name, register in REGISTER_IDS.items()}


def assert_restored(
    machine: Uc,
    initial: dict[int, int],
    stack_advance: int,
    overrides: dict[str, int] | None = None,
) -> None:
    expected = {name: initial[register] for name, register in REGISTER_IDS.items()}
    expected["sp"] = STACK_POINTER + stack_advance
    expected.update(overrides or {})
    actual = register_state(machine)
    assert actual == expected, (actual, expected)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )


def edge_hook(
    executable: bytes,
    routine_name: str,
    covered: set[tuple[int, int]],
):
    routine = ROUTINES[routine_name]
    start = int(routine["start"])
    stop = int(routine["stop"])
    sources = {source for source, _ in conditional_edges(executable, start, stop)}
    previous: int | None = None

    def observe(address: int) -> None:
        nonlocal previous
        if previous is not None and start <= address < stop:
            covered.add((previous, address))
        previous = address if address in sources else None

    def reset() -> None:
        nonlocal previous
        previous = None

    return observe, reset


def execute_start(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    initial = initial_registers(case_index)
    initial[UC_X86_REG_DS] = GAME_SEGMENT
    initial[UC_X86_REG_ES] = HEADER_SEGMENT
    initial[UC_X86_REG_EDI] = (initial[UC_X86_REG_EDI] & 0xFFFF0000) | HEADER_OFFSET
    game = seeded_segment(case_index + 1)
    header = seeded_segment(case_index + 17)
    data = seeded_segment(case_index + 33)
    fs_data = seeded_segment(case_index + 49)
    stack = seeded_segment(case_index + 65)
    unowned = seeded_segment(case_index + 81)
    header[HEADER_OFFSET + 4] = 0xD3 if case["packed"] else 0xA6
    stack[STACK_POINTER : STACK_POINTER + 8] = struct.pack("<H", RETURN_IP) + b"STABLE"
    before = {
        HEADER_SEGMENT: bytes(header),
        DATA_SEGMENT: bytes(data),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    game_before = bytes(game)
    stack_before = bytes(stack)
    machine, patched = map_machine(
        executable,
        game,
        header,
        data,
        fs_data,
        stack,
        unowned,
        initial,
        (STOP_HELPER, PAGE_HELPER, SUBMIT_HELPER),
    )
    covered: set[tuple[int, int]] = set()
    observe_edge, reset_edge = edge_hook(executable, "start", covered)
    stops: list[int] = []
    pages: list[dict[str, int]] = []
    submits: list[dict[str, int]] = []
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == STOP_HELPER:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            stops.append(cpu.reg_read(UC_X86_REG_AX))
            reset_edge()
            return
        if address == PAGE_HELPER:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            pages.append(
                {
                    "page": cpu.reg_read(UC_X86_REG_AX),
                    "address": cpu.reg_read(UC_X86_REG_EBX),
                }
            )
            cpu.reg_write(UC_X86_REG_ES, HEADER_SEGMENT)
            cpu.reg_write(UC_X86_REG_DI, HEADER_OFFSET)
            reset_edge()
            return
        if address == SUBMIT_HELPER:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            bp = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
            submits.append(
                {
                    "descriptor": bp,
                    "address": read_dword(cpu, bp),
                    "length": read_word(cpu, bp + 4),
                    "state": cpu.mem_read(GAME_SEGMENT * 16 + bp + 6, 1)[0],
                }
            )
            reset_edge()
            return
        assert ROUTINES["start"]["start"] <= address < ROUTINES["start"]["stop"]
        observe_edge(address)

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(int(ROUTINES["start"]["start"]), 0, count=500)
    except UcError as error:
        raise RuntimeError(
            f"start failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    assert stops == [0, 1]
    assert pages == [{"page": 0, "address": 0x8000}]
    assert submits == [
        {
            "descriptor": FIELDS["first_buffer"],
            "address": 0x8000,
            "length": 0x2000,
            "state": 2,
        }
    ]
    assert machine.mem_read(GAME_SEGMENT * 16 + FIELDS["refill"], 1)[0] == 0
    assert machine.mem_read(GAME_SEGMENT * 16 + FIELDS["pending"], 1)[0] == 2
    assert machine.mem_read(GAME_SEGMENT * 16 + FIELDS["packed"], 1)[0] == int(
        case["packed"]
    )
    assert read_word(machine, FIELDS["page"]) == 1
    expected_game = bytearray(game_before)
    expected_game[FIELDS["refill"]] = 0
    expected_game[FIELDS["pending"]] = 2
    expected_game[FIELDS["packed"]] = int(case["packed"])
    write_word(expected_game, FIELDS["page"], 1)
    descriptor(expected_game, FIELDS["first_buffer"], 0x8000, 0x2000, 2)
    descriptor(expected_game, FIELDS["second_buffer"], 0xA000, 0x2000, 0)
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    for segment, contents in before.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    assert bytes(machine.mem_read(0, len(executable))) == patched
    actual_stack = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert actual_stack[:0xFEEE] == stack_before[:0xFEEE]
    assert actual_stack[STACK_POINTER + 2 :] == stack_before[STACK_POINTER + 2 :]
    assert all(
        STACK_SEGMENT * 16 + 0xFEEE <= address
        and address + size <= STACK_SEGMENT * 16 + STACK_POINTER + 2
        or GAME_SEGMENT * 16 + FIELDS["first_buffer"] <= address
        and address + size <= GAME_SEGMENT * 16 + FIELDS["page"] + 2
        or GAME_SEGMENT * 16 + FIELDS["refill"] <= address
        and address + size <= GAME_SEGMENT * 16 + FIELDS["refill"] + 1
        for address, size in writes
    )
    assert_restored(machine, initial, 2)
    flags = machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
    return {
        "routine": "start",
        "name": case["name"],
        "packed": bool(case["packed"]),
        "states_after": [2, 0],
        "pending_after": 2,
        "page_after": 1,
        "submitted_buffer": 0,
        "defined_flags": flags,
    }, covered


def execute_service(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    initial = initial_registers(case_index + 20)
    game = seeded_segment(case_index + 101)
    header = seeded_segment(case_index + 117)
    data = seeded_segment(case_index + 133)
    fs_data = seeded_segment(case_index + 149)
    stack = seeded_segment(case_index + 165)
    unowned = seeded_segment(case_index + 181)
    game[0x0F1F] = int(case["ultrasound"])
    game[FIELDS["sound"]] = int(case["sound"])
    game[FIELDS["channel"]] = int(case["channel"])
    game[FIELDS["pending"]] = int(case["pending"])
    game[FIELDS["refill"]] = int(case["refill"])
    write_word(game, FIELDS["page"], int(case["page"]))
    write_word(game, FIELDS["page_count"], int(case["page_count"]))
    write_word(game, FIELDS["final_length"], 0x0456)
    descriptor(game, FIELDS["first_buffer"], 0x8000, 0x1111, int(case["states"][0]))
    descriptor(game, FIELDS["second_buffer"], 0xA000, 0x2222, int(case["states"][1]))
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + b"STABLE"
    )
    game_before = bytes(game)
    stack_before = bytes(stack)
    before = {
        HEADER_SEGMENT: bytes(header),
        DATA_SEGMENT: bytes(data),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    machine, patched = map_machine(
        executable,
        game,
        header,
        data,
        fs_data,
        stack,
        unowned,
        initial,
        (STOP_HELPER, PAGE_HELPER, SUBMIT_HELPER),
    )
    covered: set[tuple[int, int]] = set()
    observe_edge, reset_edge = edge_hook(executable, "service", covered)
    stops: list[int] = []
    pages: list[dict[str, int]] = []
    submits: list[int] = []
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address in (STOP_HELPER, PAGE_HELPER, SUBMIT_HELPER):
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF0
            if address == STOP_HELPER:
                stops.append(cpu.reg_read(UC_X86_REG_AX))
            elif address == PAGE_HELPER:
                pages.append(
                    {
                        "buffer": 0
                        if cpu.reg_read(UC_X86_REG_SI) == FIELDS["first_buffer"]
                        else 1,
                        "page": cpu.reg_read(UC_X86_REG_AX),
                        "address": cpu.reg_read(UC_X86_REG_EBX),
                    }
                )
            else:
                bp = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
                assert cpu.mem_read(GAME_SEGMENT * 16 + bp + 6, 1)[0] == 2
                submits.append(0 if bp == FIELDS["first_buffer"] else 1)
            reset_edge()
            return
        assert ROUTINES["service"]["start"] <= address < ROUTINES["service"]["stop"]
        observe_edge(address)

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(int(ROUTINES["service"]["start"]), 0, count=800)
    except UcError as error:
        raise RuntimeError(
            f"service failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    active = bool(
        int(case["ultrasound"]) & 1
        and int(case["sound"]) & 1
        and int(case["channel"]) & 1
        and int(case["pending"]) & 2
    )
    states = list(case["states"])
    refill = int(case["refill"])
    page = int(case["page"])
    lengths = [0x1111, 0x2222]
    expected_stops: list[int] = []
    expected_submits: list[int] = []
    expected_pages: list[dict[str, int]] = []
    if active:
        if refill & 1:
            expected_stops.append(1)
            restart = 0 if states[0] & 1 else 1 if states[1] & 1 else None
            if restart is not None:
                refill = 0
                states = [0, 0]
                states[restart] = 2
                expected_submits.append(restart)
        selected = 0 if states[0] & 3 == 0 else 1 if states[1] & 3 == 0 else None
        if selected is not None:
            expected_pages.append(
                {
                    "buffer": selected,
                    "page": page,
                    "address": 0x8000 if selected == 0 else 0xA000,
                }
            )
            states[selected] = 1
            lengths[selected] = 0x2000
            candidate = (page + 1) & 0xFFFF
            if candidate >= int(case["page_count"]):
                lengths[selected] = 0x0456
                page = 0
            else:
                page = candidate
    assert stops == expected_stops
    assert submits == expected_submits
    assert pages == expected_pages
    actual_states = [
        machine.mem_read(GAME_SEGMENT * 16 + FIELDS["first_buffer"] + 6, 1)[0],
        machine.mem_read(GAME_SEGMENT * 16 + FIELDS["second_buffer"] + 6, 1)[0],
    ]
    actual_lengths = [
        read_word(machine, FIELDS["first_buffer"] + 4),
        read_word(machine, FIELDS["second_buffer"] + 4),
    ]
    assert actual_states == states
    assert actual_lengths == lengths
    assert machine.mem_read(GAME_SEGMENT * 16 + FIELDS["refill"], 1)[0] == refill
    assert read_word(machine, FIELDS["page"]) == page
    expected_game = bytearray(game_before)
    expected_game[FIELDS["refill"]] = refill
    expected_game[FIELDS["first_buffer"] + 6] = states[0]
    expected_game[FIELDS["second_buffer"] + 6] = states[1]
    write_word(expected_game, FIELDS["first_buffer"] + 4, lengths[0])
    write_word(expected_game, FIELDS["second_buffer"] + 4, lengths[1])
    write_word(expected_game, FIELDS["page"], page)
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    for segment, contents in before.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    assert bytes(machine.mem_read(0, len(executable))) == patched
    actual_stack = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert actual_stack[:0xFEF0] == stack_before[:0xFEF0]
    assert actual_stack[STACK_POINTER + 4 :] == stack_before[STACK_POINTER + 4 :]
    game_ranges = (
        (FIELDS["first_buffer"] + 4, FIELDS["first_buffer"] + 7),
        (FIELDS["second_buffer"] + 4, FIELDS["second_buffer"] + 7),
        (FIELDS["page"], FIELDS["page"] + 2),
        (FIELDS["refill"], FIELDS["refill"] + 1),
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEF0 <= address
        and address + size <= STACK_SEGMENT * 16 + STACK_POINTER + 4
        or any(
            GAME_SEGMENT * 16 + lo <= address
            and address + size <= GAME_SEGMENT * 16 + hi
            for lo, hi in game_ranges
        )
        for address, size in writes
    )
    expected_ebp = initial[UC_X86_REG_EBP]
    if expected_submits:
        descriptor_offset = (
            FIELDS["first_buffer"]
            if expected_submits[-1] == 0
            else FIELDS["second_buffer"]
        )
        expected_ebp = (expected_ebp & 0xFFFF0000) | descriptor_offset
    assert_restored(machine, initial, 4, {"ebp": expected_ebp})
    return {
        "routine": "service",
        "name": case["name"],
        "active": active,
        "states_before": list(case["states"]),
        "states_after": states,
        "refill_before": int(case["refill"]),
        "refill_after": refill,
        "page_before": int(case["page"]),
        "page_after": page,
        "loaded_buffer": expected_pages[0]["buffer"] if expected_pages else None,
        "submitted_buffer": expected_submits[0] if expected_submits else None,
        "defined_flags": machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK,
    }, covered


def execute_interrupt(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    initial = initial_registers(case_index + 60)
    initial_flags = initial[UC_X86_REG_EFLAGS] & 0xFFFF
    game = seeded_segment(case_index + 201)
    header = seeded_segment(case_index + 217)
    data = seeded_segment(case_index + 233)
    fs_data = seeded_segment(case_index + 249)
    stack = seeded_segment(case_index + 9)
    unowned = seeded_segment(case_index + 25)
    base_port = 0x0240
    write_word(game, FIELDS["base_port"], base_port)
    write_word(game, FIELDS["irq"], int(case["irq"]))
    game[FIELDS["refill"]] = 0
    descriptor(game, FIELDS["first_buffer"], 0x8000, 0x2000, int(case["states"][0]))
    descriptor(game, FIELDS["second_buffer"], 0xA000, 0x2000, int(case["states"][1]))
    stack[STACK_POINTER : STACK_POINTER + 12] = (
        struct.pack("<HHH", RETURN_IP, 0, initial_flags) + b"STABLE"
    )
    game_before = bytes(game)
    stack_before = bytes(stack)
    before = {
        HEADER_SEGMENT: bytes(header),
        DATA_SEGMENT: bytes(data),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    machine, patched = map_machine(
        executable,
        game,
        header,
        data,
        fs_data,
        stack,
        unowned,
        initial,
        (SUBMIT_HELPER,),
    )
    covered: set[tuple[int, int]] = set()
    observe_edge, reset_edge = edge_hook(executable, "interrupt", covered)
    inputs: list[tuple[int, int]] = []
    outputs: list[tuple[int, int, int]] = []
    submits: list[int] = []
    writes: list[tuple[int, int]] = []
    reached_return = False

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        inputs.append((port, size))
        if port == base_port + 6:
            return int(case["status"])
        if port == base_port + 0x105:
            return int(case["voice"])
        assert port == 0x0300
        return 0x5A

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == SUBMIT_HELPER:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF6
            bp = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
            assert cpu.mem_read(GAME_SEGMENT * 16 + bp + 6, 1)[0] == 2
            submits.append(0 if bp == FIELDS["first_buffer"] else 1)
            reset_edge()
            return
        if ROUTINES["interrupt"]["start"] <= address < ROUTINES["interrupt"]["stop"]:
            observe_edge(address)
        else:
            assert DELAY_HELPER <= address < 0xE0ED
            reset_edge()

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(int(ROUTINES["interrupt"]["start"]), 0, count=2_000)
    except UcError as error:
        raise RuntimeError(
            f"interrupt failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    states = list(case["states"])
    refill = 0
    expected_submits: list[int] = []
    voice_interrupt = bool(
        int(case["status"]) & 0x20 and int(case["voice"]) & 0x1F == 1
    )
    if voice_interrupt:
        if states[0] & 2:
            states[0] = 0
            if states[1] & 1:
                states[1] = 2
                expected_submits.append(1)
            else:
                refill = 1
        elif states[1] & 2:
            states[1] = 0
            if states[0] & 1:
                states[0] = 2
                expected_submits.append(0)
            else:
                refill = 1
        else:
            refill = 1
    assert submits == expected_submits
    actual_states = [
        machine.mem_read(GAME_SEGMENT * 16 + FIELDS["first_buffer"] + 6, 1)[0],
        machine.mem_read(GAME_SEGMENT * 16 + FIELDS["second_buffer"] + 6, 1)[0],
    ]
    assert actual_states == states
    assert machine.mem_read(GAME_SEGMENT * 16 + FIELDS["refill"], 1)[0] == refill
    expected_game = bytearray(game_before)
    expected_game[FIELDS["first_buffer"] + 6] = states[0]
    expected_game[FIELDS["second_buffer"] + 6] = states[1]
    expected_game[FIELDS["refill"]] = refill
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    for segment, contents in before.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    assert bytes(machine.mem_read(0, len(executable))) == patched
    actual_stack = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert actual_stack[:0xFEF4] == stack_before[:0xFEF4]
    assert actual_stack[STACK_POINTER + 6 :] == stack_before[STACK_POINTER + 6 :]
    game_ranges = (
        (FIELDS["first_buffer"] + 6, FIELDS["first_buffer"] + 7),
        (FIELDS["second_buffer"] + 6, FIELDS["second_buffer"] + 7),
        (FIELDS["refill"], FIELDS["refill"] + 1),
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEF4 <= address
        and address + size <= STACK_SEGMENT * 16 + STACK_POINTER + 6
        or any(
            GAME_SEGMENT * 16 + lo <= address
            and address + size <= GAME_SEGMENT * 16 + hi
            for lo, hi in game_ranges
        )
        for address, size in writes
    )
    assert_restored(machine, initial, 6)
    assert machine.reg_read(UC_X86_REG_EFLAGS) & 0xFFFF == initial_flags

    if int(case["status"]) & 0x20:
        expected_inputs = (
            [(base_port + 6, 1)] + [(0x0300, 1)] * 14 + [(base_port + 0x105, 1)]
        )
        expected_outputs = [
            (base_port + 0x103, 1, 0x8F),
            (base_port + 0x102, 1, int(case["voice"]) & 0x1F),
            (base_port + 0x103, 1, 0),
            (base_port + 0x105, 1, 3),
            (base_port + 0x103, 1, 9),
            (base_port + 0x104, 2, 0),
        ]
    else:
        expected_inputs = [(base_port + 6, 1)] + [(0x0300, 1)] * 7
        expected_outputs = []
    expected_outputs.extend(
        ([(0x00A0, 1, 0x20)] if int(case["irq"]) > 7 else []) + [(0x0020, 1, 0x20)]
    )
    assert inputs == expected_inputs, (case["name"], inputs, expected_inputs)
    assert outputs == expected_outputs, (case["name"], outputs, expected_outputs)
    result = "ignored"
    if voice_interrupt:
        result = "handoff" if expected_submits else "refill"
    return {
        "routine": "interrupt",
        "name": case["name"],
        "irq": int(case["irq"]),
        "voice": int(case["voice"]),
        "states_before": list(case["states"]),
        "states_after": states,
        "refill_after": refill,
        "submitted_buffer": expected_submits[0] if expected_submits else None,
        "result": result,
        "defined_flags": initial_flags & FLAG_MASK,
    }, covered


def assert_routines(executable: bytes) -> None:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    for name, routine in ROUTINES.items():
        start = int(routine["start"])
        stop = int(routine["stop"])
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        if routine["sha256"]:
            assert digest == routine["sha256"], (name, digest)
        count = len(list(decoder.disasm(body, start)))
        if routine["instruction_count"]:
            assert count == routine["instruction_count"], (name, count)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.sequel_executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != SEQUEL_SHA256:
        raise SystemExit(f"unsupported BBB executable SHA-256 {digest}")
    assert_routines(executable)

    rows: list[dict[str, Any]] = []
    coverage = {name: set() for name in ROUTINES}
    for index, case in enumerate(START_CASES):
        row, edges = execute_start(executable, case, index)
        rows.append(row)
        coverage["start"].update(edges)
    for index, case in enumerate(SERVICE_CASES):
        row, edges = execute_service(executable, case, index)
        rows.append(row)
        coverage["service"].update(edges)
    for index, case in enumerate(INTERRUPT_CASES):
        row, edges = execute_interrupt(executable, case, index)
        rows.append(row)
        coverage["interrupt"].update(edges)

    total_edges = 0
    for name, routine in ROUTINES.items():
        expected = conditional_edges(
            executable, int(routine["start"]), int(routine["stop"])
        )
        missing = expected - coverage[name]
        assert not missing, f"{name} missing conditional edges: {sorted(missing)}"
        assert coverage[name] == expected
        total_edges += len(expected)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB Ultrasound stream-controller cases "
        f"across {total_edges} conditional edges"
    )


if __name__ == "__main__":
    main()
