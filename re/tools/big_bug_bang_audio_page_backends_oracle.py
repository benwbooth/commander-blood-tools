#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio page-storage backends."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
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

ROOT = Path(__file__).resolve().parents[2]
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
FIXTURE_ROOT = ROOT / "re/tools/oracle_vectors"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "ems": {
            "routine": (0xBD26, 0xBD4E),
            "sha256": "db15b3cda28ad812dac847298327127233ee695c460da0f3a6ff517b2ba5a40e",
            "instruction_count": 23,
            "handle": 0x0A60,
            "page_frame": 0x0A66,
        },
        "xms": {
            "routine": (0xBD4E, 0xBD8D),
            "sha256": "264ce031bf2411ec0dd052e40c1d8d50e38c0e05d7c0816f4b7cb86f6089a023",
            "instruction_count": 23,
            "driver": 0x0A4A,
            "handle": 0x0A5E,
            "request": 0x0A6C,
            "callback_return": 0xBD88,
        },
        "file": {
            "routine": (0xBD8D, 0xBDB7),
            "sha256": "4ad91abe66caeda36b5ba7f2714f2dacabcddd3192b084b7f96c6b28fa9e9cf5",
            "instruction_count": 24,
            "handle": 0x0C49,
        },
    },
    "sequel": {
        "ems": {
            "routine": (0xD4D0, 0xD4F8),
            "sha256": "8f0cc5b6147f2bfa275dce21f3dde97e2fa4c7fc9c4c311d38814743b8020ef1",
            "instruction_count": 23,
            "handle": 0x0C58,
            "page_frame": 0x0C5E,
        },
        "xms": {
            "routine": (0xD4F8, 0xD537),
            "sha256": "0e3b898fbce88c12a70d4d7a51804749f578482637c626395e8ecb6d6b479db0",
            "instruction_count": 23,
            "driver": 0x0C42,
            "handle": 0x0C56,
            "request": 0x0C64,
            "callback_return": 0xD532,
        },
        "file": {
            "routine": (0xD537, 0xD561),
            "sha256": "41950ddbbde48e684ee782f6589dd0fc588522d6527a43891ec90fb89c73f81f",
            "instruction_count": 24,
            "handle": 0x0E53,
        },
    },
}

EMS_CASES = (
    (0x0246, False),
    (0x0893, False),
    (0x0203, True),
    (0x0AD7, False),
)
XMS_CASES = (
    (0x0246, False),
    (0x0893, False),
    (0x0203, False),
    (0x0AD7, True),
)
FILE_CASES = (
    (0x0000, 0x4000, 0x0202, 0x0246, False),
    (0x4000, 0x3FFF, 0x0203, 0x0893, False),
    (0x0005, 0x0000, 0x0203, 0x0203, True),
    (0xC000, 0x4000, 0x0AD7, 0x0AD7, False),
)
FIXTURE_STEMS = {
    "ems": "bd26",
    "xms": "bd4e",
    "file": "bd8d",
}

MACHINE_SIZE = 0xB0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
FS_SEGMENT = 0x4000
PAGE_FRAME_SEGMENT = 0x5000
ALTERNATE_PAGE_FRAME_SEGMENT = 0x6000
DESTINATION_SEGMENT = 0x7000
CALLBACK_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
UNOWNED_SEGMENT = 0xA000
CALLBACK_OFFSET = 0x0100
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778")

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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 11 + seed * 29 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initial_registers(page: int, destination_offset: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A10000 | page,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F60000 | destination_offset,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: DESTINATION_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }


def final_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def install_machine(
    executable: bytes,
    initial: dict[int, int],
    segments: tuple[tuple[int, bytearray], ...],
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in segments:
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)
    return machine


def run(
    machine: Uc,
    start: int,
    callback: Callable[[Uc, int, int, object], None] | None = None,
    interrupt: Callable[[Uc, int, object], None] | None = None,
) -> list[tuple[int, int]]:
    reached_return = False
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, size: int, context: object) -> None:
        nonlocal reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if callback is not None:
            callback(cpu, address, size, context)

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    if interrupt is not None:
        machine.hook_add(UC_HOOK_INTR, interrupt)
    try:
        machine.emu_start(start, 0, count=100_000)
    except UcError as error:
        raise RuntimeError(
            f"execution failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    return writes


def assert_registers(
    machine: Uc,
    initial: dict[int, int],
    expected_changes: dict[str, int] | None = None,
) -> None:
    expected = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 2,
        "ds": DATA_SEGMENT,
        "es": DESTINATION_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    if expected_changes is not None:
        expected.update(expected_changes)
    actual = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    assert actual == expected
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )


def assert_unchanged(
    machine: Uc,
    snapshots: dict[int, bytes],
    excluded: set[int],
) -> None:
    for segment, contents in snapshots.items():
        if segment in excluded:
            continue
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents


def execute_ems(
    executable: bytes,
    branch_name: str,
    row: dict[str, Any],
    case_index: int,
) -> dict[str, Any]:
    branch = BRANCHES[branch_name]["ems"]
    start, _stop = branch["routine"]
    response_flags, mutate_frame = EMS_CASES[case_index]
    page = int(row["page"])
    handle = int(row["snd_bank_ems_handle"])
    destination_offset = int(row["destination_offset"])
    initial = initial_registers(page, destination_offset)
    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    page_frame = seeded_segment(case_index + 33)
    alternate_frame = seeded_segment(case_index + 49)
    destination = seeded_segment(case_index + 65)
    fs_data = seeded_segment(case_index + 81)
    stack = seeded_segment(case_index + 97)
    unowned = seeded_segment(case_index + 113)
    source = bytes((index * 29 + case_index * 71 + 3) & 0xFF for index in range(0x4000))
    page_frame[:0x4000] = source
    alternate_frame[:0x4000] = source[::-1]
    struct.pack_into("<H", game, int(branch["handle"]), handle)
    struct.pack_into("<H", game, int(branch["page_frame"]), PAGE_FRAME_SEGMENT)
    struct.pack_into("<H", data, int(branch["handle"]), handle ^ 0x55AA)
    struct.pack_into("<H", data, int(branch["page_frame"]), PAGE_FRAME_SEGMENT ^ 0x1111)
    stack[STACK_POINTER : STACK_POINTER + 8] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    snapshots = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        PAGE_FRAME_SEGMENT: bytes(page_frame),
        ALTERNATE_PAGE_FRAME_SEGMENT: bytes(alternate_frame),
        DESTINATION_SEGMENT: bytes(destination),
        FS_SEGMENT: bytes(fs_data),
        STACK_SEGMENT: bytes(stack),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    expected_destination = bytearray(destination)
    for index, value in enumerate(source):
        expected_destination[(destination_offset + index) & 0xFFFF] = value
    calls: list[tuple[int, int, int, int, int]] = []

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        calls.append(
            (
                number,
                cpu.reg_read(UC_X86_REG_AX),
                cpu.reg_read(UC_X86_REG_BX),
                cpu.reg_read(UC_X86_REG_DX),
                cpu.reg_read(UC_X86_REG_DS),
            )
        )
        assert number == 0x67
        if mutate_frame:
            cpu.mem_write(
                GAME_SEGMENT * 16 + int(branch["page_frame"]),
                struct.pack("<H", ALTERNATE_PAGE_FRAME_SEGMENT),
            )
        cpu.reg_write(UC_X86_REG_AX, 0x00A5)
        cpu.reg_write(UC_X86_REG_EFLAGS, response_flags)

    machine = install_machine(
        executable,
        initial,
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (PAGE_FRAME_SEGMENT, page_frame),
            (ALTERNATE_PAGE_FRAME_SEGMENT, alternate_frame),
            (DESTINATION_SEGMENT, destination),
            (FS_SEGMENT, fs_data),
            (STACK_SEGMENT, stack),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    writes = run(machine, start, interrupt=interrupt)
    assert calls == [(0x67, 0x4400, page, handle, PAGE_FRAME_SEGMENT)]
    assert bytes(machine.mem_read(DESTINATION_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_destination
    )
    expected_game = bytearray(snapshots[GAME_SEGMENT])
    if mutate_frame:
        struct.pack_into(
            "<H", expected_game, int(branch["page_frame"]), ALTERNATE_PAGE_FRAME_SEGMENT
        )
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert_unchanged(
        machine, snapshots, {GAME_SEGMENT, DESTINATION_SEGMENT, STACK_SEGMENT}
    )
    expected_stack = bytearray(snapshots[STACK_SEGMENT])
    struct.pack_into(
        "<HHHHHHHH",
        expected_stack,
        0xFEF2,
        initial[UC_X86_REG_EDX] & 0xFFFF,
        initial[UC_X86_REG_ECX] & 0xFFFF,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_EAX] & 0xFFFF,
        initial[UC_X86_REG_EDI] & 0xFFFF,
        initial[UC_X86_REG_ESI] & 0xFFFF,
        DATA_SEGMENT,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    allowed = [
        (STACK_SEGMENT * 16 + 0xFEF2, STACK_SEGMENT * 16 + 0xFF02),
        (
            DESTINATION_SEGMENT * 16,
            DESTINATION_SEGMENT * 16 + SEGMENT_SIZE,
        ),
        (
            GAME_SEGMENT * 16 + int(branch["page_frame"]),
            GAME_SEGMENT * 16 + int(branch["page_frame"]) + 2,
        ),
    ]
    assert all(
        any(lo <= address and address + size <= hi for lo, hi in allowed)
        for address, size in writes
    )
    assert_registers(machine, initial)
    assert final_flags(machine) == row["defined_flags"]
    assert hashlib.sha256(source).hexdigest() == row["source_sha256"]
    return dict(row)


CALLBACK_CHANGES = {
    "eax": 0xCAFE1234,
    "ebx": 0xBEEF5678,
    "ecx": 0x13572468,
    "edx": 0x24681357,
    "esi": 0xFACE9ABC,
    "edi": 0x369C258B,
    "ebp": 0x48AD37CE,
    "es": 0x7CE0,
}


def execute_xms(
    executable: bytes,
    branch_name: str,
    row: dict[str, Any],
    case_index: int,
) -> dict[str, Any]:
    branch = BRANCHES[branch_name]["xms"]
    start, _stop = branch["routine"]
    callback_flags, clobber = XMS_CASES[case_index]
    page = int(row["page"])
    handle = int(row["snd_bank_xms_handle"])
    destination_offset = int(row["destination_offset"])
    initial = initial_registers(page, destination_offset)
    game = seeded_segment(case_index + 129)
    data = seeded_segment(case_index + 145)
    destination = seeded_segment(case_index + 161)
    fs_data = seeded_segment(case_index + 177)
    callback = seeded_segment(case_index + 193)
    stack = seeded_segment(case_index + 209)
    unowned = seeded_segment(case_index + 225)
    driver_pointer = struct.pack("<HH", CALLBACK_OFFSET, CALLBACK_SEGMENT)
    data_pointer = struct.pack("<HH", 0x0200, 0xE000)
    request_before = bytes((0x31 + index * 13) & 0xFF for index in range(16))
    data_request = bytes((0xA5 - index * 9) & 0xFF for index in range(16))
    game[int(branch["driver"]) : int(branch["driver"]) + 4] = driver_pointer
    struct.pack_into("<H", game, int(branch["handle"]), handle)
    game[int(branch["request"]) : int(branch["request"]) + 16] = request_before
    data[int(branch["driver"]) : int(branch["driver"]) + 4] = data_pointer
    struct.pack_into("<H", data, int(branch["handle"]), handle ^ 0x55AA)
    data[int(branch["request"]) : int(branch["request"]) + 16] = data_request
    callback[CALLBACK_OFFSET] = 0xCB
    stack[STACK_POINTER : STACK_POINTER + 8] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    snapshots = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        DESTINATION_SEGMENT: bytes(destination),
        FS_SEGMENT: bytes(fs_data),
        CALLBACK_SEGMENT: bytes(callback),
        STACK_SEGMENT: bytes(stack),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    expected_request = struct.pack(
        "<IHIHHH",
        0x4000,
        handle,
        (page << 14) & 0xFFFFFFFF,
        0,
        destination_offset,
        DESTINATION_SEGMENT,
    )
    callback_calls: list[dict[str, int]] = []

    def capture(cpu: Uc, _address: int, _size: int, _context: object) -> None:
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear != CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET:
            return
        assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF4
        frame = struct.unpack("<HHHHHHH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF4, 14))
        assert frame == (
            int(branch["callback_return"]),
            0,
            initial[UC_X86_REG_EAX] & 0xFFFF,
            initial[UC_X86_REG_EBX] & 0xFFFF,
            initial[UC_X86_REG_ESI] & 0xFFFF,
            DATA_SEGMENT,
            RETURN_IP,
        )
        callback_calls.append(
            {
                "eax": cpu.reg_read(UC_X86_REG_EAX),
                "ebx": cpu.reg_read(UC_X86_REG_EBX),
                "esi": cpu.reg_read(UC_X86_REG_ESI),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "sp": cpu.reg_read(UC_X86_REG_SP),
            }
        )
        assert (
            bytes(cpu.mem_read(GAME_SEGMENT * 16 + int(branch["request"]), 16))
            == expected_request
        )
        if clobber:
            for name, value in CALLBACK_CHANGES.items():
                cpu.reg_write(REGISTER_IDS[name], value)
        cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)

    machine = install_machine(
        executable,
        initial,
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (DESTINATION_SEGMENT, destination),
            (FS_SEGMENT, fs_data),
            (CALLBACK_SEGMENT, callback),
            (STACK_SEGMENT, stack),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    writes = run(machine, start, callback=capture)
    assert callback_calls == [
        {
            "eax": 0x00000B00,
            "ebx": (initial[UC_X86_REG_EBX] & 0xFFFF0000) | handle,
            "esi": (initial[UC_X86_REG_ESI] & 0xFFFF0000) | int(branch["request"]),
            "ds": GAME_SEGMENT,
            "es": DESTINATION_SEGMENT,
            "sp": 0xFEF4,
        }
    ]
    expected_game = bytearray(snapshots[GAME_SEGMENT])
    expected_game[int(branch["request"]) : int(branch["request"]) + 16] = (
        expected_request
    )
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert_unchanged(machine, snapshots, {GAME_SEGMENT, STACK_SEGMENT})
    expected_stack = bytearray(snapshots[STACK_SEGMENT])
    struct.pack_into(
        "<HHHHHHH",
        expected_stack,
        0xFEF4,
        int(branch["callback_return"]),
        0,
        initial[UC_X86_REG_EAX] & 0xFFFF,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_ESI] & 0xFFFF,
        DATA_SEGMENT,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    allowed = [
        (STACK_SEGMENT * 16 + 0xFEF4, STACK_SEGMENT * 16 + 0xFF02),
        (
            GAME_SEGMENT * 16 + int(branch["request"]),
            GAME_SEGMENT * 16 + int(branch["request"]) + 16,
        ),
    ]
    assert all(
        any(lo <= address and address + size <= hi for lo, hi in allowed)
        for address, size in writes
    )
    changes = None
    if clobber:
        changes = dict(CALLBACK_CHANGES)
        changes["eax"] = (CALLBACK_CHANGES["eax"] & 0xFFFF0000) | page
        changes["ebx"] = (CALLBACK_CHANGES["ebx"] & 0xFFFF0000) | (
            initial[UC_X86_REG_EBX] & 0xFFFF
        )
        changes["esi"] = (CALLBACK_CHANGES["esi"] & 0xFFFF0000) | (
            initial[UC_X86_REG_ESI] & 0xFFFF
        )
    else:
        changes = {"eax": page}
    assert_registers(machine, initial, changes)
    assert final_flags(machine) == row["defined_flags"]
    return dict(row)


def execute_file(
    executable: bytes,
    branch_name: str,
    row: dict[str, Any],
    case_index: int,
) -> dict[str, Any]:
    branch = BRANCHES[branch_name]["file"]
    start, _stop = branch["routine"]
    seek_ax, read_ax, seek_flags, read_flags, mutate_handle = FILE_CASES[case_index]
    page = int(row["page"])
    handle = int(row["snd_bank_file_handle"])
    destination_offset = int(row["destination_offset"])
    initial = initial_registers(page, destination_offset)
    game = seeded_segment(case_index + 241)
    data = seeded_segment(case_index + 257)
    destination = seeded_segment(case_index + 273)
    fs_data = seeded_segment(case_index + 289)
    stack = seeded_segment(case_index + 305)
    unowned = seeded_segment(case_index + 321)
    struct.pack_into("<H", game, int(branch["handle"]), handle)
    struct.pack_into("<H", data, int(branch["handle"]), handle ^ 0x55AA)
    stack[STACK_POINTER : STACK_POINTER + 8] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    read_data = bytes(
        (index * 17 + case_index * 43 + 7) & 0xFF for index in range(0x4000)
    )
    snapshots = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        DESTINATION_SEGMENT: bytes(destination),
        FS_SEGMENT: bytes(fs_data),
        STACK_SEGMENT: bytes(stack),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    calls: list[dict[str, int]] = []

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        call = {
            "number": number,
            "ax": cpu.reg_read(UC_X86_REG_AX),
            "bx": cpu.reg_read(UC_X86_REG_BX),
            "cx": cpu.reg_read(UC_X86_REG_CX),
            "dx": cpu.reg_read(UC_X86_REG_DX),
            "ds": cpu.reg_read(UC_X86_REG_DS),
        }
        calls.append(call)
        assert number == 0x21
        if len(calls) == 1:
            if mutate_handle:
                cpu.mem_write(
                    GAME_SEGMENT * 16 + int(branch["handle"]),
                    struct.pack("<H", handle ^ 0xFFFF),
                )
            cpu.reg_write(UC_X86_REG_AX, seek_ax)
            cpu.reg_write(UC_X86_REG_EFLAGS, seek_flags)
        elif len(calls) == 2:
            cpu.mem_write(call["ds"] * 16 + call["dx"], read_data)
            cpu.reg_write(UC_X86_REG_AX, read_ax)
            cpu.reg_write(UC_X86_REG_EFLAGS, read_flags)
        else:
            raise AssertionError("unexpected third DOS request")

    machine = install_machine(
        executable,
        initial,
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (DESTINATION_SEGMENT, destination),
            (FS_SEGMENT, fs_data),
            (STACK_SEGMENT, stack),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    writes = run(machine, start, interrupt=interrupt)
    seek_offset = (page << 14) & 0xFFFFFFFF
    assert calls == [
        {
            "number": 0x21,
            "ax": 0x4200,
            "bx": handle,
            "cx": (seek_offset >> 16) & 0xFFFF,
            "dx": seek_offset & 0xFFFF,
            "ds": DATA_SEGMENT,
        },
        {
            "number": 0x21,
            "ax": 0x3F00 | (seek_ax & 0xFF),
            "bx": handle,
            "cx": 0x4000,
            "dx": destination_offset,
            "ds": DESTINATION_SEGMENT,
        },
    ]
    assert (
        bytes(machine.mem_read(DESTINATION_SEGMENT * 16 + destination_offset, 0x4000))
        == read_data
    )
    expected_game = bytearray(snapshots[GAME_SEGMENT])
    if mutate_handle:
        struct.pack_into("<H", expected_game, int(branch["handle"]), handle ^ 0xFFFF)
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert_unchanged(
        machine, snapshots, {GAME_SEGMENT, DESTINATION_SEGMENT, STACK_SEGMENT}
    )
    expected_stack = bytearray(snapshots[STACK_SEGMENT])
    struct.pack_into(
        "<HHHHHHH",
        expected_stack,
        0xFEF4,
        DESTINATION_SEGMENT,
        initial[UC_X86_REG_EDX] & 0xFFFF,
        initial[UC_X86_REG_ECX] & 0xFFFF,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_EAX] & 0xFFFF,
        DATA_SEGMENT,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    allowed = [
        (STACK_SEGMENT * 16 + 0xFEF4, STACK_SEGMENT * 16 + 0xFF02),
        (
            DESTINATION_SEGMENT * 16 + destination_offset,
            DESTINATION_SEGMENT * 16 + destination_offset + 0x4000,
        ),
        (
            GAME_SEGMENT * 16 + int(branch["handle"]),
            GAME_SEGMENT * 16 + int(branch["handle"]) + 2,
        ),
    ]
    assert all(
        any(lo <= address and address + size <= hi for lo, hi in allowed)
        for address, size in writes
    )
    assert_registers(machine, initial)
    assert final_flags(machine) == row["defined_flags"]
    assert hashlib.sha256(read_data).hexdigest() == row["read_sha256"]
    return dict(row)


def assert_bodies(executable: bytes, branch_name: str) -> None:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    for backend, branch in BRANCHES[branch_name].items():
        start, stop = branch["routine"]
        body = executable[start:stop]
        assert hashlib.sha256(body).hexdigest() == branch["sha256"], backend
        assert len(list(decoder.disasm(body, start))) == branch["instruction_count"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    commander = COMMANDER_PATH.read_bytes()
    sequel = args.sequel_executable.read_bytes()
    for name, executable, expected in (
        ("Commander", commander, COMMANDER_SHA256),
        ("BBB", sequel, SEQUEL_SHA256),
    ):
        digest = hashlib.sha256(executable).hexdigest()
        if digest != expected:
            raise SystemExit(f"unsupported {name} executable SHA-256 {digest}")
    assert_bodies(commander, "commander")
    assert_bodies(sequel, "sequel")

    runners = {
        "ems": execute_ems,
        "xms": execute_xms,
        "file": execute_file,
    }
    rows = []
    for backend, runner in runners.items():
        expected_rows = json.loads(
            (FIXTURE_ROOT / f"func_{FIXTURE_STEMS[backend]}_natural.json").read_text()
        )
        assert len(expected_rows) == 4
        for case_index, expected_row in enumerate(expected_rows):
            commander_row = runner(commander, "commander", expected_row, case_index)
            sequel_row = runner(sequel, "sequel", expected_row, case_index)
            assert commander_row == expected_row
            assert sequel_row == expected_row
            rows.append({"backend": backend, **sequel_row})

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print("verified 12 dual audio page-backend cases across EMS, XMS, and file")


if __name__ == "__main__":
    main()
