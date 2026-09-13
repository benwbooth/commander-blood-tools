#!/usr/bin/env python3
# ruff: noqa: E402
"""Exercise Big Bug Bang's low-level Gravis Ultrasound voice routines."""

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

SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ROUTINES = {
    "descriptor": (
        0xDB08,
        0xDBD4,
        101,
        "8b23ad5513e00c3ef3a0ff73e45cb78c7a145520bd782aed22865298177a7e5b",
    ),
    "clip": (
        0xDCE5,
        0xDDD2,
        115,
        "948c61bf4d501be8f20b055c1ddb442ff73b67e052b06c733fdf8a8f9a72501a",
    ),
    "stop": (
        0xDDD2,
        0xDDF8,
        24,
        "f00e860ac8a0116ec50e34049033ef2f258ef1b818b72cc510c8f2dc54c7ebd5",
    ),
    "upload": (
        0xDDF8,
        0xDE5B,
        51,
        "1409f11a4a3820cd1af52f290cd725edb4f719f809bee30b6e0e7877a9d18522",
    ),
    "delay": (
        0xE0DE,
        0xE0ED,
        13,
        "22b956a9f577da6fbf84f215ae52e51865c8031cbd92196f731e269948d771db",
    ),
}

GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
EXTRA_SEGMENT = 0x4000
FS_SEGMENT = 0x5000
STACK_SEGMENT = 0x6000
UNOWNED_SEGMENT = 0x7000
STACK_POINTER = 0xFF00
RETURN_IP = 0x7400
SEGMENT_SIZE = 0x10000
MACHINE_SIZE = 0x90000
BASE_PORT = 0x0240
SAMPLE_FREQUENCY = 2205
FLAG_MASK = 0x08D5

FIELDS = {
    "packed": 0x0DAC,
    "page": 0x0DAF,
    "compact_table": 0x0DC9,
    "stream_table": 0x0E61,
    "gus_base": 0x0EE1,
    "base_port": 0x0EE5,
    "frequency": 0x0EE9,
}

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

DESCRIPTOR_CASES = tuple(
    {
        "name": f"{'packed' if packed else 'plain'}_{'page_one' if page_one else 'other_page'}",
        "packed": packed,
        "page_one": page_one,
    }
    for packed in (False, True)
    for page_one in (False, True)
)
CLIP_CASES = (
    {"name": "resident_clip", "index": 2, "start": 0x2345, "length": 0x0234},
    {"name": "streamed_clip", "index": -1, "start": 0x34567, "length": 0x0321},
)
STOP_CASES = (
    {"name": "stop_voice_zero", "voice": 0},
    {"name": "stop_voice_one", "voice": 1},
)
UPLOAD_CASES = (
    {
        "name": "ordinary_upload",
        "address": 0x12345,
        "payload": [0x00, 0x7F, 0x80, 0xFF],
    },
    {"name": "bank_crossing_upload", "address": 0x0FFFE, "payload": [0x12, 0x34, 0x56]},
)


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (index * 67 + seed * 43 + 11) & 0xFF for index in range(SEGMENT_SIZE)
    )


def initial_registers(case_index: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A10000 | (0x1200 + case_index),
        UC_X86_REG_EBX: 0xB2B23456,
        UC_X86_REG_ECX: 0xC3C34567,
        UC_X86_REG_EDX: 0xD4D45678,
        UC_X86_REG_ESI: 0xE5E589AB,
        UC_X86_REG_EDI: 0xF6F61234,
        UC_X86_REG_EBP: 0x97975678,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }


def write_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


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


def edge_observer(
    executable: bytes,
    routine_name: str,
    covered: set[tuple[int, int]],
):
    start, stop, _count, _digest = ROUTINES[routine_name]
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


def map_machine(
    executable: bytes,
    game: bytearray,
    data: bytearray,
    extra: bytearray,
    fs_data: bytearray,
    stack: bytearray,
    unowned: bytearray,
    initial: dict[int, int],
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in (
        (GAME_SEGMENT, game),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    ):
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)
    return machine


def common_memory(case_index: int) -> tuple[bytearray, ...]:
    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    extra = seeded_segment(case_index + 33)
    fs_data = seeded_segment(case_index + 49)
    stack = seeded_segment(case_index + 65)
    unowned = seeded_segment(case_index + 81)
    write_word(game, FIELDS["base_port"], BASE_PORT)
    write_word(game, FIELDS["frequency"], SAMPLE_FREQUENCY)
    stack[STACK_POINTER : STACK_POINTER + 8] = struct.pack("<H", RETURN_IP) + b"STABLE"
    return game, data, extra, fs_data, stack, unowned


def port_hooks(
    machine: Uc,
) -> tuple[list[tuple[int, int]], list[tuple[int, int, int]]]:
    inputs: list[tuple[int, int]] = []
    outputs: list[tuple[int, int, int]] = []

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        assert port == 0x0300
        inputs.append((port, size))
        return 0x5A

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    return inputs, outputs


def register_state(machine: Uc) -> dict[str, int]:
    return {name: machine.reg_read(register) for name, register in REGISTER_IDS.items()}


def assert_return(
    machine: Uc,
    initial: dict[int, int],
    expected_registers: dict[str, int] | None = None,
) -> None:
    expected = {name: initial[register] for name, register in REGISTER_IDS.items()}
    expected["sp"] = STACK_POINTER + 2
    expected.update(expected_registers or {})
    actual = register_state(machine)
    assert actual == expected, (actual, expected)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )


def assert_memory(
    machine: Uc,
    executable: bytes,
    before: dict[int, bytes],
    stack_before: bytes,
    minimum_stack: int,
    writes: list[tuple[int, int]],
) -> None:
    assert bytes(machine.mem_read(0, len(executable))) == executable
    for segment, contents in before.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert stack_after[:minimum_stack] == stack_before[:minimum_stack]
    assert stack_after[STACK_POINTER + 2 :] == stack_before[STACK_POINTER + 2 :]
    assert all(
        STACK_SEGMENT * 16 + minimum_stack <= address
        and address + size <= STACK_SEGMENT * 16 + STACK_POINTER + 2
        for address, size in writes
    )


def stop_outputs(voice: int) -> list[tuple[int, int, int]]:
    return [
        (BASE_PORT + 0x102, 1, voice & 0xFF),
        (BASE_PORT + 0x103, 1, 9),
        (BASE_PORT + 0x104, 2, 0),
        (BASE_PORT + 0x103, 1, 0),
        (BASE_PORT + 0x105, 1, 3),
        (BASE_PORT + 0x105, 1, 3),
    ]


def append_address_registers(
    outputs: list[tuple[int, int, int]],
    start: int,
    end: int,
) -> None:
    for register, address in (
        (2, start),
        (3, start),
        (0x0A, start),
        (0x0B, start),
        (4, end),
        (5, end),
    ):
        outputs.append((BASE_PORT + 0x103, 1, register))
        value = (
            (address >> 7) & 0xFFFF
            if register in (2, 0x0A, 4)
            else (address << 9) & 0xFFFF
        )
        outputs.append((BASE_PORT + 0x104, 2, value))


def voice_outputs(
    voice: int,
    rate: int,
    start: int,
    end: int,
    control: int,
) -> list[tuple[int, int, int]]:
    outputs = [
        (BASE_PORT + 0x102, 1, voice),
        (BASE_PORT + 0x103, 1, 9),
        (BASE_PORT + 0x104, 2, 0xF000),
        (BASE_PORT + 0x103, 1, 0x0C),
        (BASE_PORT + 0x105, 1, 7),
        (BASE_PORT + 0x103, 1, 1),
        (BASE_PORT + 0x104, 2, rate),
    ]
    append_address_registers(outputs, start, end)
    outputs.extend(
        [
            (BASE_PORT + 0x103, 1, 0),
            (BASE_PORT + 0x105, 1, control),
            (BASE_PORT + 0x105, 1, control),
        ]
    )
    return outputs


def execute_descriptor(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, data, extra, fs_data, stack, unowned = common_memory(case_index)
    initial = initial_registers(case_index)
    descriptor_offset = 0x1800
    address = 0x00045ABC
    length = 0x0234
    write_dword(game, descriptor_offset, address)
    write_word(game, descriptor_offset + 4, length)
    game[FIELDS["packed"]] = int(case["packed"])
    game[FIELDS["page"]] = 1 if case["page_one"] else 3
    initial[UC_X86_REG_EBP] = (initial[UC_X86_REG_EBP] & 0xFFFF0000) | descriptor_offset
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    machine = map_machine(
        executable, game, data, extra, fs_data, stack, unowned, initial
    )
    inputs, outputs = port_hooks(machine)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "descriptor", covered)
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address_now: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address_now == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if ROUTINES["descriptor"][0] <= address_now < ROUTINES["descriptor"][1]:
            observe(address_now)
        else:
            assert ROUTINES["delay"][0] <= address_now < ROUTINES["delay"][1]
            reset()

    def memory_write(
        _cpu: Uc,
        _access: int,
        address_now: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address_now, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(ROUTINES["descriptor"][0], 0, count=1_000)
    except UcError as error:
        raise RuntimeError(
            f"descriptor failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    effective_start = address + (6 if case["page_one"] else 0)
    end = address + length - 1
    rate = (0x2B11 * (2 if case["packed"] else 1)) // SAMPLE_FREQUENCY
    assert outputs == voice_outputs(1, rate, effective_start, end, 0x20)
    assert inputs == [(0x0300, 1)] * 7
    assert_return(machine, initial)
    actual_flags = machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
    expected_flags = initial[UC_X86_REG_EFLAGS] & FLAG_MASK
    assert actual_flags == expected_flags, (actual_flags, expected_flags)
    assert_memory(machine, executable, before, stack_before, 0xFEEE, writes)
    return {
        "routine": "descriptor",
        "name": case["name"],
        "packed": bool(case["packed"]),
        "page_one": bool(case["page_one"]),
        "voice": 1,
        "rate": rate,
        "start": effective_start,
        "end": end,
        "port_writes": len(outputs),
    }, covered


def execute_stop(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> dict[str, Any]:
    game, data, extra, fs_data, stack, unowned = common_memory(case_index + 20)
    initial = initial_registers(case_index + 20)
    initial[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | int(
        case["voice"]
    )
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    machine = map_machine(
        executable, game, data, extra, fs_data, stack, unowned, initial
    )
    inputs, outputs = port_hooks(machine)
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address_now: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address_now == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        assert (
            ROUTINES["stop"][0] <= address_now < ROUTINES["stop"][1]
            or ROUTINES["delay"][0] <= address_now < ROUTINES["delay"][1]
        )

    def memory_write(
        _cpu: Uc,
        _access: int,
        address_now: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address_now, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(ROUTINES["stop"][0], 0, count=500)
    assert reached_return
    assert outputs == stop_outputs(int(case["voice"]))
    assert inputs == [(0x0300, 1)] * 7
    assert_return(machine, initial)
    actual_flags = machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
    expected_flags = initial[UC_X86_REG_EFLAGS] & FLAG_MASK
    assert actual_flags == expected_flags, (actual_flags, expected_flags)
    assert_memory(machine, executable, before, stack_before, 0xFEF4, writes)
    return {
        "routine": "stop",
        "name": case["name"],
        "voice": int(case["voice"]),
        "port_writes": len(outputs),
    }


def execute_clip(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, data, extra, fs_data, stack, unowned = common_memory(case_index + 40)
    initial = initial_registers(case_index + 40)
    index = int(case["index"])
    initial[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | (index & 0xFFFF)
    start = int(case["start"])
    length = int(case["length"])
    if index >= 0:
        table_offset = FIELDS["compact_table"] + index * 4
        write_word(game, table_offset, start)
        write_word(game, table_offset + 2, length)
        hardware_start = start + 6
        expected_ebx = length - 6
    else:
        table_offset = (FIELDS["stream_table"] + ((index & 0xFFFF) << 2)) & 0xFFFF
        write_dword(game, table_offset, start)
        write_dword(game, (table_offset + 4) & 0xFFFF, start + length)
        write_dword(game, FIELDS["gus_base"], 0x20000)
        hardware_start = start + 0x20000 + 6
        expected_ebx = (initial[UC_X86_REG_EBX] & 0xFFFF0000) | (length - 6)
    hardware_end = (hardware_start + expected_ebx - 1) & 0xFFFFFFFF
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    machine = map_machine(
        executable, game, data, extra, fs_data, stack, unowned, initial
    )
    inputs, outputs = port_hooks(machine)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "clip", covered)
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address_now: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address_now == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if ROUTINES["clip"][0] <= address_now < ROUTINES["clip"][1]:
            observe(address_now)
        else:
            assert (
                ROUTINES["stop"][0] <= address_now < ROUTINES["stop"][1]
                or ROUTINES["delay"][0] <= address_now < ROUTINES["delay"][1]
            )
            reset()

    def memory_write(
        _cpu: Uc,
        _access: int,
        address_now: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address_now, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(ROUTINES["clip"][0], 0, count=2_000)
    assert reached_return
    expected_outputs = stop_outputs(0) + voice_outputs(
        0,
        0x2B11 // SAMPLE_FREQUENCY,
        hardware_start,
        hardware_end,
        0,
    )
    assert outputs == expected_outputs, (case["name"], outputs, expected_outputs)
    assert inputs == [(0x0300, 1)] * 21
    expected_ebp = (initial[UC_X86_REG_EBP] & 0xFFFF0000) | table_offset
    expected_edx = (initial[UC_X86_REG_EDX] & 0xFFFF0000) | (BASE_PORT + 0x105)
    assert_return(
        machine,
        initial,
        {"ebx": expected_ebx, "ebp": expected_ebp, "edx": expected_edx},
    )
    actual_flags = machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
    expected_flags = 0x44
    assert actual_flags == expected_flags, (actual_flags, expected_flags)
    assert_memory(machine, executable, before, stack_before, 0xFEE8, writes)
    return {
        "routine": "clip",
        "name": case["name"],
        "source": "resident" if index >= 0 else "streamed",
        "index": index,
        "voice": 0,
        "start": hardware_start,
        "end": hardware_end,
        "payload_bytes": length - 6,
        "port_writes": len(outputs),
    }, covered


def upload_outputs(address: int, payload: list[int]) -> list[tuple[int, int, int]]:
    outputs = [
        (BASE_PORT + 0x103, 1, 0x44),
        (BASE_PORT + 0x105, 1, (address >> 16) & 0xFF),
        (BASE_PORT + 0x103, 1, 0x43),
        (BASE_PORT + 0x104, 2, address & 0xFFFF),
    ]
    cursor = address
    for value in payload:
        outputs.append((BASE_PORT + 0x107, 1, value ^ 0x80))
        cursor = (cursor + 1) & 0xFFFFFFFF
        if cursor & 0xFFFF == 0:
            outputs.extend(
                [
                    (BASE_PORT + 0x103, 1, 0x44),
                    (BASE_PORT + 0x105, 1, (cursor >> 16) & 0xFF),
                    (BASE_PORT + 0x103, 1, 0x43),
                ]
            )
        outputs.append((BASE_PORT + 0x104, 2, cursor & 0xFFFF))
    return outputs


def execute_upload(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, data, extra, fs_data, stack, unowned = common_memory(case_index + 60)
    initial = initial_registers(case_index + 60)
    source_offset = 0x1A00
    payload = list(case["payload"])
    data[source_offset : source_offset + len(payload)] = bytes(payload)
    initial[UC_X86_REG_EBX] = int(case["address"])
    initial[UC_X86_REG_ECX] = len(payload)
    initial[UC_X86_REG_ESI] = (initial[UC_X86_REG_ESI] & 0xFFFF0000) | source_offset
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    machine = map_machine(
        executable, game, data, extra, fs_data, stack, unowned, initial
    )
    inputs, outputs = port_hooks(machine)
    covered: set[tuple[int, int]] = set()
    observe, _reset = edge_observer(executable, "upload", covered)
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address_now: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address_now == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        assert ROUTINES["upload"][0] <= address_now < ROUTINES["upload"][1]
        observe(address_now)

    def memory_write(
        _cpu: Uc,
        _access: int,
        address_now: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address_now, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(ROUTINES["upload"][0], 0, count=2_000)
    assert reached_return
    assert outputs == upload_outputs(int(case["address"]), payload)
    assert not inputs
    assert_return(machine, initial)
    assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == 0x44
    assert_memory(machine, executable, before, stack_before, 0xFEEE, writes)
    return {
        "routine": "upload",
        "name": case["name"],
        "address": int(case["address"]),
        "payload_bytes": len(payload),
        "crossed_bank": int(case["address"]) >> 16
        != (int(case["address"]) + len(payload)) >> 16,
        "port_writes": len(outputs),
    }, covered


def execute_delay(executable: bytes) -> dict[str, Any]:
    game, data, extra, fs_data, stack, unowned = common_memory(100)
    initial = initial_registers(100)
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    machine = map_machine(
        executable, game, data, extra, fs_data, stack, unowned, initial
    )
    inputs, outputs = port_hooks(machine)
    writes: list[tuple[int, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address_now: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address_now == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        assert ROUTINES["delay"][0] <= address_now < ROUTINES["delay"][1]

    def memory_write(
        _cpu: Uc,
        _access: int,
        address_now: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address_now, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(ROUTINES["delay"][0], 0, count=100)
    assert reached_return
    assert inputs == [(0x0300, 1)] * 7
    assert not outputs
    assert_return(machine, initial)
    assert (
        machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
        == initial[UC_X86_REG_EFLAGS] & FLAG_MASK
    )
    assert_memory(machine, executable, before, stack_before, 0xFEFC, writes)
    return {
        "routine": "delay",
        "name": "seven_reads",
        "port_reads": len(inputs),
        "port_writes": 0,
    }


def assert_bodies(executable: bytes) -> None:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    for name, (start, stop, expected_count, expected_digest) in ROUTINES.items():
        body = executable[start:stop]
        assert hashlib.sha256(body).hexdigest() == expected_digest, name
        assert len(list(decoder.disasm(body, start))) == expected_count, name


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.sequel_executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != SEQUEL_SHA256:
        raise SystemExit(f"unsupported BBB executable SHA-256 {digest}")
    assert_bodies(executable)

    rows: list[dict[str, Any]] = []
    coverage = {name: set() for name in ("descriptor", "clip", "upload")}
    for index, case in enumerate(DESCRIPTOR_CASES):
        row, edges = execute_descriptor(executable, case, index)
        rows.append(row)
        coverage["descriptor"].update(edges)
    for index, case in enumerate(CLIP_CASES):
        row, edges = execute_clip(executable, case, index)
        rows.append(row)
        coverage["clip"].update(edges)
    for index, case in enumerate(STOP_CASES):
        rows.append(execute_stop(executable, case, index))
    for index, case in enumerate(UPLOAD_CASES):
        row, edges = execute_upload(executable, case, index)
        rows.append(row)
        coverage["upload"].update(edges)
    rows.append(execute_delay(executable))

    total_edges = 0
    for name, covered in coverage.items():
        start, stop, _count, _digest = ROUTINES[name]
        expected = conditional_edges(executable, start, stop)
        missing = expected - covered
        assert not missing, f"{name} missing conditional edges: {sorted(missing)}"
        assert covered == expected
        total_edges += len(expected)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB Ultrasound voice cases across "
        f"{total_edges} conditional edges"
    )


if __name__ == "__main__":
    main()
