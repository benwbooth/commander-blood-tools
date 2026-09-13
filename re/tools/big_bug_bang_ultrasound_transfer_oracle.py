#!/usr/bin/env python3
# ruff: noqa: E402
"""Exercise Big Bug Bang's Gravis page and streamed-bank transfers."""

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
    UC_HOOK_INTR,
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
    "page": (
        0xDBD4,
        0xDC92,
        73,
        "37767389b08ee4fdbfd93cfb7c7a01d7398d2e63fd4265a43d15020d0ec7c10e",
    ),
    "stream": (
        0xDC92,
        0xDCE5,
        31,
        "1bd176c3d14b171e7806e59bb6e5bc03f15a17a0c8272f724b2431ed570e4b03",
    ),
}

UPLOAD_HELPER = 0xDDF8
UPLOAD_STOP = 0xDE5B
GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
EXTRA_SEGMENT = 0x4000
FS_SEGMENT = 0x5000
STACK_SEGMENT = 0x6000
UNOWNED_SEGMENT = 0x7000
CALLBACK_SEGMENT = 0x8000
CALLBACK_OFFSET = 0x1200
STACK_POINTER = 0xFF00
RETURN_IP = 0x7400
SEGMENT_SIZE = 0x10000
MACHINE_SIZE = 0xA0000
TRANSFER_OFFSET = 0x1800
WORK_OFFSET = 0x0200
STREAM_BUFFER_OFFSET = WORK_OFFSET + 0x7D00
PAGE_BYTE_COUNT = 0x2000
STREAM_CHUNK_BYTE_COUNT = 0x7D00
BASE_PORT = 0x0240
FLAG_MASK = 0x08D5

FIELDS = {
    "xms_driver": 0x0C42,
    "xms_handle": 0x0C56,
    "ems_handle": 0x0C58,
    "page_frame": 0x0C5E,
    "xms_request": 0x0C64,
    "backend": 0x0DA9,
    "transfer_pointer": 0x0DC1,
    "file_handle": 0x0E53,
    "gus_address": 0x0EE1,
    "base_port": 0x0EE5,
    "work_pointer": 0x0CB4,
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

PAGE_CASES = (
    {
        "name": "ems_zero_even",
        "backend": "ems",
        "selector": 0x00,
        "page": 4,
        "destination": 0x00018000,
    },
    {
        "name": "ems_signed_81_odd",
        "backend": "ems",
        "selector": 0x81,
        "page": 5,
        "destination": 0x0001F800,
    },
    {
        "name": "xms_one",
        "backend": "xms",
        "selector": 0x01,
        "page": 18,
        "destination": 0x0002A000,
    },
    {
        "name": "file_80",
        "backend": "file",
        "selector": 0x80,
        "page": 9,
        "destination": 0x0000FFF0,
    },
    {
        "name": "file_short_read",
        "backend": "file",
        "selector": 0x02,
        "page": 7,
        "destination": 0x00034000,
        "read_return": 0x1234,
    },
)

STREAM_CASES = (
    {
        "name": "tiny_tail",
        "total_bytes": 3,
        "read_returns": [3],
        "destination": 0x00012345,
    },
    {
        "name": "exact_chunk",
        "total_bytes": STREAM_CHUNK_BYTE_COUNT,
        "read_returns": [STREAM_CHUNK_BYTE_COUNT],
        "destination": 0x0001A000,
    },
    {
        "name": "short_multichunk",
        "total_bytes": 40_003,
        "read_returns": [12_000, 28_000, 3],
        "destination": 0x0002FFFE,
    },
)


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (index * 67 + seed * 43 + 11) & 0xFF for index in range(SEGMENT_SIZE)
    )


def generated_bytes(length: int, seed: int) -> bytes:
    return bytes((index * 29 + seed * 47 + 13) & 0xFF for index in range(length))


def write_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def initial_registers(case_index: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A11200 | case_index,
        UC_X86_REG_EBX: 0xB2B23456,
        UC_X86_REG_ECX: 0xC3C34567,
        UC_X86_REG_EDX: 0xD4D45678,
        UC_X86_REG_ESI: 0xE5E589AB,
        UC_X86_REG_EDI: 0xF6F61234,
        UC_X86_REG_EBP: 0x97975678,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: EXTRA_SEGMENT,
        UC_X86_REG_ES: FS_SEGMENT,
        UC_X86_REG_FS: UNOWNED_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }


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
    sources = {source for source, _target in conditional_edges(executable, start, stop)}
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


def common_memory(case_index: int) -> tuple[bytearray, ...]:
    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    extra = seeded_segment(case_index + 33)
    fs_data = seeded_segment(case_index + 49)
    stack = seeded_segment(case_index + 65)
    unowned = seeded_segment(case_index + 81)
    callback = seeded_segment(case_index + 97)
    write_word(game, FIELDS["base_port"], BASE_PORT)
    stack[STACK_POINTER : STACK_POINTER + 8] = struct.pack("<H", RETURN_IP) + b"STABLE"
    callback[CALLBACK_OFFSET] = 0xCB
    return game, data, extra, fs_data, stack, unowned, callback


def map_machine(
    executable: bytes,
    segments: tuple[tuple[int, bytearray], ...],
    initial: dict[int, int],
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in segments:
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)
    return machine


def register_state(machine: Uc) -> dict[str, int]:
    return {name: machine.reg_read(register) for name, register in REGISTER_IDS.items()}


def assert_return(
    machine: Uc,
    initial: dict[int, int],
    changes: dict[str, int],
) -> None:
    expected = {name: initial[register] for name, register in REGISTER_IDS.items()}
    expected["sp"] = STACK_POINTER + 2
    expected.update(changes)
    actual = register_state(machine)
    assert actual == expected, (actual, expected)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )


def assert_memory(
    machine: Uc,
    executable: bytes,
    expected: dict[int, bytes],
    stack_before: bytes,
    minimum_stack: int,
    writes: list[tuple[int, int]],
    allowed: list[tuple[int, int]],
) -> None:
    assert bytes(machine.mem_read(0, len(executable))) == executable
    for segment, contents in expected.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert stack_after[:minimum_stack] == stack_before[:minimum_stack]
    assert stack_after[STACK_POINTER + 2 :] == stack_before[STACK_POINTER + 2 :]
    assert all(
        any(low <= address and address + size <= high for low, high in allowed)
        for address, size in writes
    )


def port_hooks(machine: Uc) -> list[tuple[int, int, int]]:
    outputs: list[tuple[int, int, int]] = []

    def input_port(_cpu: Uc, port: int, size: int, _context: object) -> int:
        raise AssertionError(f"unexpected port read {port:#x}/{size}")

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        outputs.append((port, size, value))

    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    return outputs


def upload_outputs(address: int, payload: bytes) -> list[tuple[int, int, int]]:
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


def transcript_sha256(outputs: list[tuple[int, int, int]]) -> str:
    encoded = json.dumps(outputs, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def execute_page(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, data, extra, fs_data, stack, unowned, callback = common_memory(case_index)
    initial = initial_registers(case_index)
    page = int(case["page"])
    destination = int(case["destination"])
    selector = int(case["selector"])
    backend = str(case["backend"])
    initial[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | page
    initial[UC_X86_REG_EBX] = destination

    ems_handle = 0x1357
    xms_handle = 0x2468
    file_handle = 0x369C
    game[FIELDS["backend"]] = selector
    write_word(game, FIELDS["ems_handle"], ems_handle)
    write_word(game, FIELDS["xms_handle"], xms_handle)
    write_word(game, FIELDS["page_frame"], DATA_SEGMENT)
    write_word(game, FIELDS["file_handle"], file_handle)
    write_word(game, FIELDS["base_port"], BASE_PORT)
    game[FIELDS["xms_driver"] : FIELDS["xms_driver"] + 4] = struct.pack(
        "<HH", CALLBACK_OFFSET, CALLBACK_SEGMENT
    )
    game[FIELDS["transfer_pointer"] : FIELDS["transfer_pointer"] + 4] = struct.pack(
        "<HH", TRANSFER_OFFSET, DATA_SEGMENT
    )
    request_before = generated_bytes(16, case_index + 113)
    game[FIELDS["xms_request"] : FIELDS["xms_request"] + 16] = request_before

    source_byte_offset = page * PAGE_BYTE_COUNT
    source_payload = generated_bytes(PAGE_BYTE_COUNT, case_index + 129)
    read_return = int(case.get("read_return", PAGE_BYTE_COUNT))
    payload = source_payload
    if backend == "file" and read_return < PAGE_BYTE_COUNT:
        staged = bytearray(data[TRANSFER_OFFSET : TRANSFER_OFFSET + PAGE_BYTE_COUNT])
        staged[:read_return] = source_payload[:read_return]
        payload = bytes(staged)
    ems_physical_page = generated_bytes(PAGE_BYTE_COUNT * 2, case_index + 145)
    if backend == "ems":
        half_offset = (page & 1) * PAGE_BYTE_COUNT
        ems_physical_page = (
            ems_physical_page[:half_offset]
            + source_payload
            + ems_physical_page[half_offset + PAGE_BYTE_COUNT :]
        )

    segments = (
        (GAME_SEGMENT, game),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
        (CALLBACK_SEGMENT, callback),
    )
    machine = map_machine(executable, segments, initial)
    outputs = port_hooks(machine)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "page", covered)
    writes: list[tuple[int, int]] = []
    interrupts: list[dict[str, int]] = []
    xms_calls: list[dict[str, int]] = []
    upload_calls: list[dict[str, int]] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if linear == CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET:
            assert backend == "xms"
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            assert struct.unpack(
                "<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEE, 4)
            ) == (0xDC50, 0)
            request = bytes(cpu.mem_read(GAME_SEGMENT * 16 + FIELDS["xms_request"], 16))
            expected_request = struct.pack(
                "<IHIHHH",
                PAGE_BYTE_COUNT,
                xms_handle,
                source_byte_offset,
                0,
                TRANSFER_OFFSET,
                DATA_SEGMENT,
            )
            assert request == expected_request
            xms_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ebx": cpu.reg_read(UC_X86_REG_EBX),
                    "esi": cpu.reg_read(UC_X86_REG_ESI),
                    "edi": cpu.reg_read(UC_X86_REG_EDI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                }
            )
            cpu.mem_write(DATA_SEGMENT * 16 + TRANSFER_OFFSET, payload)
            writes.append((DATA_SEGMENT * 16 + TRANSFER_OFFSET, len(payload)))
            cpu.reg_write(UC_X86_REG_AX, 1)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0246)
            reset()
            return
        if address == UPLOAD_HELPER and cpu.reg_read(UC_X86_REG_CS) == 0:
            upload_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ebx": cpu.reg_read(UC_X86_REG_EBX),
                    "ecx": cpu.reg_read(UC_X86_REG_ECX),
                    "esi": cpu.reg_read(UC_X86_REG_ESI),
                    "edi": cpu.reg_read(UC_X86_REG_EDI),
                    "ebp": cpu.reg_read(UC_X86_REG_EBP),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                    "return": struct.unpack(
                        "<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF0, 2)
                    )[0],
                }
            )
            reset()
            return
        if ROUTINES["page"][0] <= address < ROUTINES["page"][1]:
            observe(address)
            return
        assert UPLOAD_HELPER <= address < UPLOAD_STOP
        reset()

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        if number == 0x67:
            assert backend == "ems"
            call = {
                "number": number,
                "eax": cpu.reg_read(UC_X86_REG_EAX),
                "ebx": cpu.reg_read(UC_X86_REG_EBX),
                "edx": cpu.reg_read(UC_X86_REG_EDX),
                "esi": cpu.reg_read(UC_X86_REG_ESI),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "sp": cpu.reg_read(UC_X86_REG_SP),
            }
            interrupts.append(call)
            assert call == {
                "number": 0x67,
                "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x4400,
                "ebx": (initial[UC_X86_REG_EBX] & 0xFFFF0000) | (page >> 1),
                "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | ems_handle,
                "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000)
                | ((page & 1) * PAGE_BYTE_COUNT),
                "ds": DATA_SEGMENT,
                "sp": 0xFEEE,
            }
            cpu.mem_write(DATA_SEGMENT * 16, ems_physical_page)
            writes.append((DATA_SEGMENT * 16, len(ems_physical_page)))
            cpu.reg_write(UC_X86_REG_AX, 0)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0203)
            return
        assert number == 0x21 and backend == "file"
        function = (cpu.reg_read(UC_X86_REG_AX) >> 8) & 0xFF
        call = {
            "number": number,
            "function": function,
            "eax": cpu.reg_read(UC_X86_REG_EAX),
            "ebx": cpu.reg_read(UC_X86_REG_EBX),
            "ecx": cpu.reg_read(UC_X86_REG_ECX),
            "edx": cpu.reg_read(UC_X86_REG_EDX),
            "esi": cpu.reg_read(UC_X86_REG_ESI),
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "es": cpu.reg_read(UC_X86_REG_ES),
            "sp": cpu.reg_read(UC_X86_REG_SP),
        }
        interrupts.append(call)
        if function == 0x42:
            assert call == {
                "number": 0x21,
                "function": 0x42,
                "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x4200,
                "ebx": (initial[UC_X86_REG_EBX] & 0xFFFF0000) | file_handle,
                "ecx": (initial[UC_X86_REG_ECX] & 0xFFFF0000) | (page >> 3),
                "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | ((page << 13) & 0xFFFF),
                "esi": initial[UC_X86_REG_ESI],
                "ds": initial[UC_X86_REG_DS],
                "es": DATA_SEGMENT,
                "sp": 0xFEEE,
            }
            cpu.reg_write(UC_X86_REG_AX, 0xA55A)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0246)
            return
        assert function == 0x3F
        assert call == {
            "number": 0x21,
            "function": 0x3F,
            "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0x3F5A,
            "ebx": (initial[UC_X86_REG_EBX] & 0xFFFF0000) | file_handle,
            "ecx": PAGE_BYTE_COUNT,
            "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | TRANSFER_OFFSET,
            "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | TRANSFER_OFFSET,
            "ds": DATA_SEGMENT,
            "es": DATA_SEGMENT,
            "sp": 0xFEEE,
        }
        cpu.mem_write(DATA_SEGMENT * 16 + TRANSFER_OFFSET, source_payload[:read_return])
        writes.append((DATA_SEGMENT * 16 + TRANSFER_OFFSET, read_return))
        cpu.reg_write(UC_X86_REG_AX, read_return)
        cpu.reg_write(UC_X86_REG_EFLAGS, 0x0283)

    def memory_write(
        _cpu: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(ROUTINES["page"][0], 0, count=2_000_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    source_offset = (
        (page & 1) * PAGE_BYTE_COUNT if backend == "ems" else TRANSFER_OFFSET
    )
    expected_upload_eax = {
        "ems": (initial[UC_X86_REG_EAX] & 0xFFFF0000),
        "xms": 1,
        "file": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | read_return,
    }[backend]
    expected_return = {"ems": 0xDC0F, "xms": 0xDC5D, "file": 0xDC8A}[backend]
    expected_upload_calls = [
        {
            "eax": expected_upload_eax,
            "ebx": destination,
            "ecx": PAGE_BYTE_COUNT,
            "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | source_offset,
            "edi": (initial[UC_X86_REG_EDI] & 0xFFFF0000) | source_offset,
            "ebp": initial[UC_X86_REG_EBP],
            "ds": DATA_SEGMENT,
            "es": DATA_SEGMENT,
            "sp": 0xFEF0,
            "return": expected_return,
        }
    ]
    assert upload_calls == expected_upload_calls, (
        case["name"],
        upload_calls,
        expected_upload_calls,
    )
    if backend == "xms":
        assert xms_calls == [
            {
                "eax": 0x00000B00,
                "ebx": destination,
                "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | FIELDS["xms_request"],
                "edi": (initial[UC_X86_REG_EDI] & 0xFFFF0000) | TRANSFER_OFFSET,
                "ds": GAME_SEGMENT,
                "es": DATA_SEGMENT,
                "sp": 0xFEEE,
            }
        ]
    else:
        assert not xms_calls
    assert len(interrupts) == {"ems": 1, "xms": 0, "file": 2}[backend]
    expected_outputs = upload_outputs(destination, payload)
    assert outputs == expected_outputs
    return_changes = {
        "edi": (initial[UC_X86_REG_EDI] & 0xFFFF0000) | source_offset,
        "es": DATA_SEGMENT,
    }
    if backend == "xms":
        return_changes["eax"] = page
    assert_return(machine, initial, return_changes)
    assert machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK == 0x44

    expected_game = bytearray(game)
    expected_data = bytearray(data)
    allowed = [
        (
            STACK_SEGMENT * 16 + 0xFEDE,
            STACK_SEGMENT * 16 + STACK_POINTER + 2,
        )
    ]
    if backend == "ems":
        expected_data[: len(ems_physical_page)] = ems_physical_page
        allowed.append((DATA_SEGMENT * 16, DATA_SEGMENT * 16 + len(ems_physical_page)))
    else:
        expected_data[TRANSFER_OFFSET : TRANSFER_OFFSET + read_return] = source_payload[
            :read_return
        ]
        allowed.append(
            (
                DATA_SEGMENT * 16 + TRANSFER_OFFSET,
                DATA_SEGMENT * 16 + TRANSFER_OFFSET + len(payload),
            )
        )
    if backend == "xms":
        expected_game[FIELDS["xms_request"] : FIELDS["xms_request"] + 16] = struct.pack(
            "<IHIHHH",
            PAGE_BYTE_COUNT,
            xms_handle,
            source_byte_offset,
            0,
            TRANSFER_OFFSET,
            DATA_SEGMENT,
        )
        allowed.append(
            (
                GAME_SEGMENT * 16 + FIELDS["xms_request"],
                GAME_SEGMENT * 16 + FIELDS["xms_request"] + 16,
            )
        )
    assert_memory(
        machine,
        executable,
        {
            GAME_SEGMENT: bytes(expected_game),
            DATA_SEGMENT: bytes(expected_data),
            EXTRA_SEGMENT: bytes(extra),
            FS_SEGMENT: bytes(fs_data),
            UNOWNED_SEGMENT: bytes(unowned),
            CALLBACK_SEGMENT: bytes(callback),
        },
        bytes(stack),
        0xFEDE,
        writes,
        allowed,
    )
    return {
        "routine": "page",
        "name": case["name"],
        "backend": backend,
        "selector": selector,
        "page": page,
        "source_byte_offset": source_byte_offset,
        "read_bytes": read_return,
        "payload_bytes": len(payload),
        "destination": destination,
        "crossed_bank": destination >> 16 != (destination + len(payload)) >> 16,
        "port_writes": len(outputs),
        "payload_sha256": hashlib.sha256(payload).hexdigest(),
        "source_sha256": hashlib.sha256(source_payload).hexdigest(),
        "port_sha256": transcript_sha256(outputs),
    }, covered


def execute_stream(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    game, data, extra, fs_data, stack, unowned, callback = common_memory(
        case_index + 40
    )
    initial = initial_registers(case_index + 40)
    total_bytes = int(case["total_bytes"])
    destination = int(case["destination"])
    read_returns = [int(value) for value in case["read_returns"]]
    file_handle = 0x2468 + case_index
    initial[UC_X86_REG_EBX] = (initial[UC_X86_REG_EBX] & 0xFFFF0000) | file_handle
    initial[UC_X86_REG_EBP] = total_bytes
    game[FIELDS["work_pointer"] : FIELDS["work_pointer"] + 4] = struct.pack(
        "<HH", WORK_OFFSET, DATA_SEGMENT
    )
    write_dword(game, FIELDS["gus_address"], destination)
    write_word(game, FIELDS["base_port"], BASE_PORT)
    payload = generated_bytes(total_bytes, case_index + 201)

    segments = (
        (GAME_SEGMENT, game),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
        (CALLBACK_SEGMENT, callback),
    )
    machine = map_machine(executable, segments, initial)
    outputs = port_hooks(machine)
    covered: set[tuple[int, int]] = set()
    observe, reset = edge_observer(executable, "stream", covered)
    writes: list[tuple[int, int]] = []
    reads: list[dict[str, int]] = []
    upload_calls: list[dict[str, int]] = []
    reached_return = False
    payload_cursor = 0

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == UPLOAD_HELPER and cpu.reg_read(UC_X86_REG_CS) == 0:
            upload_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ebx": cpu.reg_read(UC_X86_REG_EBX),
                    "ecx": cpu.reg_read(UC_X86_REG_ECX),
                    "edx": cpu.reg_read(UC_X86_REG_EDX),
                    "esi": cpu.reg_read(UC_X86_REG_ESI),
                    "edi": cpu.reg_read(UC_X86_REG_EDI),
                    "ebp": cpu.reg_read(UC_X86_REG_EBP),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                    "return": struct.unpack(
                        "<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF6, 2)
                    )[0],
                }
            )
            reset()
            return
        if ROUTINES["stream"][0] <= address < ROUTINES["stream"][1]:
            observe(address)
            return
        assert UPLOAD_HELPER <= address < UPLOAD_STOP
        reset()

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        nonlocal payload_cursor
        assert number == 0x21
        assert (cpu.reg_read(UC_X86_REG_AX) >> 8) & 0xFF == 0x3F
        call_index = len(reads)
        assert call_index < len(read_returns)
        remaining = total_bytes - payload_cursor
        request = min(remaining, STREAM_CHUNK_BYTE_COUNT)
        returned = read_returns[call_index]
        assert 0 < returned <= request
        subtraction = (remaining - STREAM_CHUNK_BYTE_COUNT) & 0xFFFFFFFF
        expected_eax = (subtraction & 0xFFFF0000) | 0x3F00 | (subtraction & 0xFF)
        call = {
            "eax": cpu.reg_read(UC_X86_REG_EAX),
            "ebx": cpu.reg_read(UC_X86_REG_EBX),
            "ecx": cpu.reg_read(UC_X86_REG_ECX),
            "edx": cpu.reg_read(UC_X86_REG_EDX),
            "esi": cpu.reg_read(UC_X86_REG_ESI),
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "es": cpu.reg_read(UC_X86_REG_ES),
            "sp": cpu.reg_read(UC_X86_REG_SP),
            "request": request,
            "returned": returned,
        }
        expected_ebx = initial[UC_X86_REG_EBX] if call_index == 0 else file_handle
        assert call == {
            "eax": expected_eax,
            "ebx": expected_ebx,
            "ecx": request,
            "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
            "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
            "ds": DATA_SEGMENT,
            "es": initial[UC_X86_REG_ES],
            "sp": 0xFEF4,
            "request": request,
            "returned": returned,
        }
        reads.append(call)
        chunk = payload[payload_cursor : payload_cursor + returned]
        cpu.mem_write(DATA_SEGMENT * 16 + STREAM_BUFFER_OFFSET, chunk)
        writes.append((DATA_SEGMENT * 16 + STREAM_BUFFER_OFFSET, len(chunk)))
        payload_cursor += returned
        cpu.reg_write(UC_X86_REG_AX, returned)
        cpu.reg_write(UC_X86_REG_EFLAGS, 0x0246 if call_index & 1 else 0x0283)

    def memory_write(
        _cpu: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: object,
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(ROUTINES["stream"][0], 0, count=8_000_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    assert payload_cursor == total_bytes
    assert len(reads) == len(read_returns)

    expected_requests: list[int] = []
    expected_uploads: list[dict[str, int]] = []
    expected_outputs: list[tuple[int, int, int]] = []
    cursor = 0
    remaining = total_bytes
    for returned in read_returns:
        request = min(remaining, STREAM_CHUNK_BYTE_COUNT)
        expected_requests.append(request)
        chunk = payload[cursor : cursor + returned]
        target = destination + cursor
        expected_uploads.append(
            {
                "eax": returned,
                "ebx": target,
                "ecx": returned,
                "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
                "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
                "edi": initial[UC_X86_REG_EDI],
                "ebp": remaining,
                "ds": DATA_SEGMENT,
                "es": initial[UC_X86_REG_ES],
                "sp": 0xFEF6,
                "return": 0xDCCB,
            }
        )
        expected_outputs.extend(upload_outputs(target, chunk))
        cursor += returned
        remaining -= returned
    assert upload_calls == expected_uploads, (
        case["name"],
        upload_calls,
        expected_uploads,
    )
    assert outputs == expected_outputs
    assert_return(
        machine,
        initial,
        {
            "eax": 0,
            "ebx": file_handle,
            "ecx": 0,
            "edx": (initial[UC_X86_REG_EDX] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
            "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | STREAM_BUFFER_OFFSET,
            "ds": DATA_SEGMENT,
        },
    )
    assert (
        machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
        == initial[UC_X86_REG_EFLAGS] & FLAG_MASK
    )

    expected_data = bytearray(data)
    cursor = 0
    for returned in read_returns:
        expected_data[STREAM_BUFFER_OFFSET : STREAM_BUFFER_OFFSET + returned] = payload[
            cursor : cursor + returned
        ]
        cursor += returned
    allowed = [
        (
            STACK_SEGMENT * 16 + 0xFEE4,
            STACK_SEGMENT * 16 + STACK_POINTER + 2,
        ),
        (
            DATA_SEGMENT * 16 + STREAM_BUFFER_OFFSET,
            DATA_SEGMENT * 16 + STREAM_BUFFER_OFFSET + max(read_returns),
        ),
    ]
    assert_memory(
        machine,
        executable,
        {
            GAME_SEGMENT: bytes(game),
            DATA_SEGMENT: bytes(expected_data),
            EXTRA_SEGMENT: bytes(extra),
            FS_SEGMENT: bytes(fs_data),
            UNOWNED_SEGMENT: bytes(unowned),
            CALLBACK_SEGMENT: bytes(callback),
        },
        bytes(stack),
        0xFEE4,
        writes,
        allowed,
    )
    return {
        "routine": "stream",
        "name": case["name"],
        "total_bytes": total_bytes,
        "read_requests": expected_requests,
        "read_returns": read_returns,
        "destination": destination,
        "final_destination": destination + total_bytes,
        "crossed_bank": destination >> 16 != (destination + total_bytes) >> 16,
        "port_writes": len(outputs),
        "payload_sha256": hashlib.sha256(payload).hexdigest(),
        "port_sha256": transcript_sha256(outputs),
    }, covered


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
    coverage = {name: set() for name in ROUTINES}
    for index, case in enumerate(PAGE_CASES):
        row, edges = execute_page(executable, case, index)
        rows.append(row)
        coverage["page"].update(edges)
    for index, case in enumerate(STREAM_CASES):
        row, edges = execute_stream(executable, case, index)
        rows.append(row)
        coverage["stream"].update(edges)

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
        f"verified {len(rows)} BBB Ultrasound transfer cases across "
        f"{total_edges} conditional edges"
    )


if __name__ == "__main__":
    main()
