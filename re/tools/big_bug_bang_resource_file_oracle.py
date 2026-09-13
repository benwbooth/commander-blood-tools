#!/usr/bin/env python3
"""Verify BBB resource-length lookup and conventional file loading."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_file.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

SOURCE_SELECT = 0x2A13
LENGTH_ENTRY = 0x2C4A
LENGTH_END = 0x2C86
LENGTH_SHA256 = "94338e5922b6e6fc9f019e6ce38f5dade7bf409b9faa85cac7d9a3c3c1f27abc"
LOAD_ENTRY = 0x2E40
LOAD_END = 0x2EF0
LOAD_SHA256 = "76dcb88ded720a6d414a7c1f34ccbfd9578c52b71b3caf748ef7a8d98aae5f08"

DATA_SEGMENT = 0x2000
GAME_SEGMENT = 0x3200
DTA_SEGMENT = 0x4600
EXTRA_SEGMENT = 0x5800
DESTINATION_SEGMENT = 0x6800
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
DESTINATION_SIZE = 0x18000
CALLER_SP = 0xFF00
RETURN_SEGMENT = 0x1800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16
PATH_OFFSET = 0x4100
DECOY_PATH_OFFSET = 0x4300
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

EMBEDDED_FLAG = 0x0CEB
SHARED_HANDLE = 0x0C7C
ARCHIVE_SIZE = 0x0C86
SOURCE_REMAINING = 0x0C8A

GENERAL_REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
}
SEGMENT_REGISTERS = {
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}

LENGTH_CASES = (
    {
        "name": "embedded_uses_archive_size",
        "embedded_flag": 1,
        "archive_size": 0x12345678,
    },
    {
        "name": "embedded_tests_only_bit_zero",
        "embedded_flag": 3,
        "archive_size": 0x80000000,
    },
    {
        "name": "standalone_find_success",
        "embedded_flag": 0,
        "archive_size": 0xA1A2A3A4,
        "dta_size_before": 0x11111111,
        "file_size": 0x00018002,
        "find_success": True,
    },
    {
        "name": "find_failure_returns_zero",
        "embedded_flag": 0,
        "archive_size": 0xB1B2B3B4,
        "dta_size_before": 0xDEADBEEF,
        "file_size": 0x01020304,
        "find_success": False,
    },
    {
        "name": "embedded_bit_one_alone_is_standalone",
        "embedded_flag": 2,
        "archive_size": 0xC1C2C3C4,
        "dta_size_before": 0x22222222,
        "file_size": 0x89ABCDEF,
        "find_success": True,
    },
    {
        "name": "dta_file_size_offset_wraps",
        "embedded_flag": 0,
        "archive_size": 0xD1D2D3D4,
        "dta_offset": 0xFFF0,
        "dta_size_before": 0x33333333,
        "file_size": 0x76543210,
        "find_success": True,
    },
    {
        "name": "standalone_full_high_dword_size",
        "embedded_flag": 0,
        "archive_size": 0xE1E2E3E4,
        "dta_size_before": 0x44444444,
        "file_size": 0x80000000,
        "find_success": True,
    },
    {
        "name": "source_selector_uses_si_not_incoming_dx",
        "embedded_flag": 0,
        "archive_size": 0xF1F2F3F4,
        "dta_size_before": 0x55555555,
        "file_size": 1,
        "find_success": True,
        "dx_decoy": True,
    },
)

LOAD_CASES = (
    {
        "name": "embedded_single_chunk",
        "embedded_flag": 1,
        "byte_count": 7,
        "read_counts": [7],
    },
    {
        "name": "embedded_bit_zero_and_full_u32",
        "embedded_flag": 3,
        "byte_count": 0x10005,
        "read_counts": [0x7D00, 0x7D00, 0x0605],
    },
    {
        "name": "standalone_find_success",
        "embedded_flag": 0,
        "byte_count": 11,
        "read_counts": [11],
    },
    {
        "name": "find_failure_uses_stale_dta",
        "embedded_flag": 0,
        "byte_count": 13,
        "read_counts": [13],
        "find_success": False,
    },
    {
        "name": "standalone_open_failure",
        "embedded_flag": 0,
        "byte_count": 17,
        "open_success": False,
    },
    {
        "name": "empty_file_reads_zero_once",
        "embedded_flag": 0,
        "byte_count": 0,
        "read_counts": [0],
    },
    {
        "name": "partial_reads_and_offset_wrap",
        "embedded_flag": 0,
        "byte_count": 0x7D07,
        "read_counts": [0x7D00, 5, 2],
        "destination_offset": 0xFFFC,
    },
    {
        "name": "read_carry_is_ignored",
        "embedded_flag": 0,
        "byte_count": 4,
        "read_counts": [4],
        "read_error": True,
    },
)


def seeded_region(size: int, case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(size)
    )


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register) for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def game_u16(machine: Uc, offset: int) -> int:
    return struct.unpack("<H", machine.mem_read(GAME_SEGMENT * 16 + offset, 2))[0]


def game_u32(machine: Uc, offset: int) -> int:
    return struct.unpack("<I", machine.mem_read(GAME_SEGMENT * 16 + offset, 4))[0]


def return_frame(machine: Uc) -> list[int]:
    stack_pointer = machine.reg_read(UC_X86_REG_SP)
    return list(
        struct.unpack("<HH", machine.mem_read(STACK_SEGMENT * 16 + stack_pointer, 4))
    )


def simulate_far_return(machine: Uc) -> None:
    stack_pointer = machine.reg_read(UC_X86_REG_SP)
    return_offset, return_segment = struct.unpack(
        "<HH", machine.mem_read(STACK_SEGMENT * 16 + stack_pointer, 4)
    )
    machine.reg_write(UC_X86_REG_SP, (stack_pointer + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_CS, return_segment)
    machine.reg_write(UC_X86_REG_IP, return_offset)


def assert_registers_restored(
    machine: Uc, initial: dict[str, int], *, expected_eax: int, expected_ebp: int
) -> None:
    expected = dict(initial)
    del expected["flags"]
    expected["eax"] = expected_eax
    expected["ebp"] = expected_ebp
    actual = snapshot_registers(machine)
    assert actual == expected, {
        key: (actual[key], value)
        for key, value in expected.items()
        if actual[key] != value
    }
    assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
    assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4


def assert_stack_ownership(before: bytes | bytearray, after: bytes) -> None:
    allowed = set(range(CALLER_SP - 0x40, CALLER_SP))
    differences = {
        offset for offset, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(offset) for offset in differences)
    assert after[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] == STACK_SENTINEL


def prepare_machine(
    executable: bytes,
    case_index: int,
    initial: dict[str, int],
    game_before: bytearray,
    dta_before: bytearray,
    destination_before: bytearray,
) -> tuple[Uc, bytes, bytes, bytes, bytearray]:
    data_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 19, 0x31))
    extra_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 23, 0x53))
    fs_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 31, 0x75))
    stack_before = seeded_region(SEGMENT_SIZE, case_index, 37, 0x97)
    stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
        "<HH", RETURN_OFFSET, RETURN_SEGMENT
    )
    stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = STACK_SENTINEL

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(SOURCE_SELECT, b"\xcb")
    machine.mem_write(DATA_SEGMENT * 16, data_before)
    machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
    machine.mem_write(DTA_SEGMENT * 16, bytes(dta_before))
    machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
    machine.mem_write(DESTINATION_SEGMENT * 16, bytes(destination_before))
    machine.mem_write(FS_SEGMENT * 16, fs_before)
    machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
    for name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, initial[name])
    for name, register in SEGMENT_REGISTERS.items():
        machine.reg_write(register, initial[name])
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, initial["flags"])
    return machine, data_before, extra_before, fs_before, stack_before


def assert_common_memory(
    machine: Uc,
    executable: bytes,
    data_before: bytes,
    extra_before: bytes,
    fs_before: bytes,
    stack_before: bytearray,
) -> None:
    expected_image = bytearray(executable)
    expected_image[SOURCE_SELECT] = 0xCB
    assert bytes(machine.mem_read(0, len(executable))) == bytes(expected_image)
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
    assert_stack_ownership(
        stack_before, bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    )


def length_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    for case_index, case in enumerate(LENGTH_CASES):
        name = str(case["name"])
        embedded_flag = int(case["embedded_flag"])
        embedded = embedded_flag & 1 != 0
        archive_size = int(case["archive_size"])
        dta_offset = int(case.get("dta_offset", 0x0200))
        dta_size_offset = (dta_offset + 0x1A) & 0xFFFF
        dta_size_before = int(case.get("dta_size_before", 0xA5A5A5A5))
        file_size = int(case.get("file_size", dta_size_before))
        find_success = bool(case.get("find_success", False))
        active_dx = DECOY_PATH_OFFSET if case.get("dx_decoy", False) else PATH_OFFSET
        path = f"LENGTH{case_index:02d}.DAT".encode("ascii") + b"\0"
        decoy_path = f"DECOY{case_index:02d}.DAT".encode("ascii") + b"\0"

        initial = {
            "eax": 0xA1A11230 + case_index,
            "ebx": 0xB2B22340 + case_index,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D40000 | active_dx,
            "esi": 0xE5E50000 | PATH_OFFSET,
            "edi": 0xF6F66780 + case_index,
            "ebp": 0x97977890 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0A92 | (0x0400 if case_index & 1 else 0),
        }
        game_before = seeded_region(SEGMENT_SIZE, case_index, 7, 0x19)
        struct.pack_into("<I", game_before, ARCHIVE_SIZE, archive_size ^ 0xFFFFFFFF)
        game_before[EMBEDDED_FLAG] = embedded_flag ^ 0xFF
        game_expected = bytearray(game_before)
        struct.pack_into("<I", game_expected, ARCHIVE_SIZE, archive_size)
        game_expected[EMBEDDED_FLAG] = embedded_flag

        dta_before = seeded_region(SEGMENT_SIZE, case_index, 11, 0x2B)
        struct.pack_into("<I", dta_before, dta_size_offset, dta_size_before)
        dta_expected = bytearray(dta_before)
        if not embedded and find_success:
            struct.pack_into("<I", dta_expected, dta_size_offset, file_size)
        destination_before = seeded_region(DESTINATION_SIZE, case_index, 13, 0x3D)

        machine, data_before, extra_before, fs_before, stack_before = prepare_machine(
            executable,
            case_index,
            initial,
            game_before,
            dta_before,
            destination_before,
        )
        machine.mem_write(DATA_SEGMENT * 16 + PATH_OFFSET, path)
        machine.mem_write(DATA_SEGMENT * 16 + DECOY_PATH_OFFSET, decoy_path)
        data_expected = bytearray(data_before)
        data_expected[PATH_OFFSET : PATH_OFFSET + len(path)] = path
        data_expected[DECOY_PATH_OFFSET : DECOY_PATH_OFFSET + len(decoy_path)] = (
            decoy_path
        )
        data_before = bytes(data_expected)

        calls: list[dict[str, Any]] = []

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == RETURN_ADDRESS:
                cpu.emu_stop()
                return
            if address == SOURCE_SELECT:
                frame = return_frame(cpu)
                calls.append(
                    {
                        "call": "resource_source_select",
                        "filename": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_DX),
                        ],
                        "return_frame": frame,
                    }
                )
                assert frame == [0x2C56, 0]
                cpu.mem_write(
                    GAME_SEGMENT * 16 + EMBEDDED_FLAG, bytes((embedded_flag,))
                )
                cpu.mem_write(
                    GAME_SEGMENT * 16 + ARCHIVE_SIZE, struct.pack("<I", archive_size)
                )
                cpu.reg_write(UC_X86_REG_BX, 0x7000 + case_index)
                simulate_far_return(cpu)
                return
            assert LENGTH_ENTRY <= address and address + size <= LENGTH_END, hex(
                address
            )

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            assert number == 0x21, hex(number)
            function = cpu.reg_read(UC_X86_REG_AX)
            if function == 0x2F00:
                calls.append({"call": "dos_get_dta"})
                cpu.reg_write(UC_X86_REG_ES, DTA_SEGMENT)
                cpu.reg_write(UC_X86_REG_BX, dta_offset)
                return
            assert function == 0x4E00, hex(function)
            calls.append(
                {
                    "call": "dos_find_first",
                    "filename": [
                        cpu.reg_read(UC_X86_REG_DS),
                        cpu.reg_read(UC_X86_REG_DX),
                    ],
                    "attributes": cpu.reg_read(UC_X86_REG_CX),
                    "success": find_success,
                }
            )
            if find_success:
                cpu.mem_write(
                    DTA_SEGMENT * 16 + dta_size_offset, struct.pack("<I", file_size)
                )
                cpu.reg_write(UC_X86_REG_AX, 0)
            else:
                cpu.reg_write(UC_X86_REG_AX, 2)
            set_carry(cpu, not find_success)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(LENGTH_ENTRY, 0, count=500)

        expected_calls: list[dict[str, Any]] = [
            {
                "call": "resource_source_select",
                "filename": [DATA_SEGMENT, PATH_OFFSET],
                "return_frame": [0x2C56, 0],
            }
        ]
        if not embedded:
            expected_calls.extend(
                [
                    {"call": "dos_get_dta"},
                    {
                        "call": "dos_find_first",
                        "filename": [DATA_SEGMENT, PATH_OFFSET],
                        "attributes": 0x18,
                        "success": find_success,
                    },
                ]
            )
        assert calls == expected_calls, (name, calls)

        expected_size = archive_size if embedded else file_size if find_success else 0
        assert_registers_restored(
            machine,
            initial,
            expected_eax=initial["eax"],
            expected_ebp=expected_size,
        )
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == bool(
            initial["flags"] & 0x0400
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DTA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            dta_expected
        )
        assert bytes(
            machine.mem_read(DESTINATION_SEGMENT * 16, DESTINATION_SIZE)
        ) == bytes(destination_before)
        assert_common_memory(
            machine, executable, data_before, extra_before, fs_before, stack_before
        )

        vectors.append(
            {
                "name": name,
                "embedded_flag": embedded_flag,
                "archive_size": archive_size,
                "find_success": find_success if not embedded else None,
                "file_size": file_size if not embedded and find_success else None,
                "returned_size": expected_size,
                "calls": calls,
            }
        )
    return vectors


def load_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    for case_index, case in enumerate(LOAD_CASES):
        name = str(case["name"])
        embedded_flag = int(case["embedded_flag"])
        embedded = embedded_flag & 1 != 0
        byte_count = int(case["byte_count"])
        read_counts = [int(value) for value in case.get("read_counts", [])]
        find_success = bool(case.get("find_success", True))
        open_success = bool(case.get("open_success", True))
        read_error = bool(case.get("read_error", False))
        destination_offset = int(case.get("destination_offset", 0x0123))
        dta_offset = 0x0200
        dta_size_offset = dta_offset + 0x1A
        embedded_handle = 0x3000 + case_index
        standalone_handle = 0x4000 + case_index
        selected_handle = embedded_handle if embedded else standalone_handle
        initial_shared_handle = 0xA500 + case_index
        path = f"LOAD{case_index:02d}.DAT".encode("ascii") + b"\0"

        initial = {
            "eax": 0xA1A11230 + case_index,
            "ebx": 0xB2B22340 + case_index,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D44560 + case_index,
            "esi": 0xE5E50000 | PATH_OFFSET,
            "edi": 0xF6F60000 | destination_offset,
            "ebp": 0x97977890 + case_index,
            "ds": DATA_SEGMENT,
            "es": DESTINATION_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0A92 | (0x0400 if case_index & 1 else 0),
        }
        game_before = seeded_region(SEGMENT_SIZE, case_index, 7, 0x49)
        struct.pack_into("<H", game_before, SHARED_HANDLE, initial_shared_handle)
        struct.pack_into("<I", game_before, ARCHIVE_SIZE, 0xB1B20000 + case_index)
        struct.pack_into("<I", game_before, SOURCE_REMAINING, 0xC3C40000 + case_index)
        game_before[EMBEDDED_FLAG] = embedded_flag ^ 0xFF
        game_expected = bytearray(game_before)
        game_expected[EMBEDDED_FLAG] = embedded_flag
        struct.pack_into("<I", game_expected, ARCHIVE_SIZE, byte_count)
        struct.pack_into(
            "<I",
            game_expected,
            SOURCE_REMAINING,
            byte_count if not embedded and not open_success else 0,
        )
        if embedded or open_success:
            struct.pack_into("<H", game_expected, SHARED_HANDLE, selected_handle)

        dta_before = seeded_region(SEGMENT_SIZE, case_index, 11, 0x5B)
        struct.pack_into("<I", dta_before, dta_size_offset, byte_count)
        dta_expected = bytearray(dta_before)
        if not embedded and find_success:
            struct.pack_into("<I", dta_expected, dta_size_offset, byte_count)
        destination_before = seeded_region(DESTINATION_SIZE, case_index, 13, 0x6D)
        destination_expected = bytearray(destination_before)

        machine, data_before, extra_before, fs_before, stack_before = prepare_machine(
            executable,
            case_index + len(LENGTH_CASES),
            initial,
            game_before,
            dta_before,
            destination_before,
        )
        machine.mem_write(DATA_SEGMENT * 16 + PATH_OFFSET, path)
        data_expected = bytearray(data_before)
        data_expected[PATH_OFFSET : PATH_OFFSET + len(path)] = path
        data_before = bytes(data_expected)

        calls: list[dict[str, Any]] = []
        read_index = 0

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == RETURN_ADDRESS:
                cpu.emu_stop()
                return
            if address == SOURCE_SELECT:
                frame = return_frame(cpu)
                calls.append(
                    {
                        "call": "resource_source_select",
                        "path": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_DX),
                        ],
                        "si": cpu.reg_read(UC_X86_REG_SI),
                        "return_frame": frame,
                    }
                )
                assert frame == [0x2E4E, 0]
                cpu.mem_write(
                    GAME_SEGMENT * 16 + EMBEDDED_FLAG, bytes((embedded_flag,))
                )
                if embedded:
                    cpu.mem_write(
                        GAME_SEGMENT * 16 + ARCHIVE_SIZE, struct.pack("<I", byte_count)
                    )
                    cpu.mem_write(
                        GAME_SEGMENT * 16 + SOURCE_REMAINING,
                        struct.pack("<I", byte_count),
                    )
                    cpu.reg_write(UC_X86_REG_BX, embedded_handle)
                else:
                    cpu.reg_write(UC_X86_REG_BX, 0x7000 + case_index)
                simulate_far_return(cpu)
                return
            assert LOAD_ENTRY <= address and address + size <= LOAD_END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            nonlocal read_index
            assert number == 0x21, hex(number)
            function = cpu.reg_read(UC_X86_REG_AX)
            if function == 0x2F00:
                calls.append({"call": "dos_get_dta"})
                cpu.reg_write(UC_X86_REG_ES, DTA_SEGMENT)
                cpu.reg_write(UC_X86_REG_BX, dta_offset)
                return
            if function == 0x4E00:
                calls.append(
                    {
                        "call": "dos_find_first",
                        "path": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_DX),
                        ],
                        "attributes": cpu.reg_read(UC_X86_REG_CX),
                        "success": find_success,
                    }
                )
                if find_success:
                    cpu.mem_write(
                        DTA_SEGMENT * 16 + dta_size_offset,
                        struct.pack("<I", byte_count),
                    )
                    cpu.reg_write(UC_X86_REG_AX, 0)
                else:
                    cpu.reg_write(UC_X86_REG_AX, 2)
                set_carry(cpu, not find_success)
                return
            if function == 0x3D00:
                calls.append(
                    {
                        "call": "dos_open_read_only",
                        "path": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_DX),
                        ],
                        "archive_size": game_u32(cpu, ARCHIVE_SIZE),
                        "remaining": game_u32(cpu, SOURCE_REMAINING),
                        "success": open_success,
                    }
                )
                cpu.reg_write(UC_X86_REG_AX, standalone_handle if open_success else 2)
                set_carry(cpu, not open_success)
                return
            if function == 0x3F00:
                assert read_index < len(read_counts), (name, "unexpected read")
                remaining = game_u32(cpu, SOURCE_REMAINING)
                requested = min(remaining, 0x7D00)
                returned = read_counts[read_index]
                assert returned <= requested
                destination_segment = cpu.reg_read(UC_X86_REG_DS)
                destination_offset_now = cpu.reg_read(UC_X86_REG_DX)
                destination_address = destination_segment * 16 + destination_offset_now
                relative_address = destination_address - DESTINATION_SEGMENT * 16
                payload = bytes(
                    ((index * 31 + read_index * 43 + case_index * 59) & 0xFF)
                    for index in range(returned)
                )
                calls.append(
                    {
                        "call": "dos_read",
                        "handle": cpu.reg_read(UC_X86_REG_BX),
                        "destination": [destination_segment, destination_offset_now],
                        "requested": cpu.reg_read(UC_X86_REG_CX),
                        "returned": returned,
                        "remaining_before": remaining,
                        "carry": read_error and read_index == 0,
                        "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    }
                )
                assert 0 <= relative_address <= DESTINATION_SIZE - returned
                if payload:
                    cpu.mem_write(destination_address, payload)
                    destination_expected[
                        relative_address : relative_address + returned
                    ] = payload
                cpu.reg_write(UC_X86_REG_AX, returned)
                set_carry(cpu, read_error and read_index == 0)
                read_index += 1
                return
            assert function == 0x3E00, hex(function)
            calls.append(
                {
                    "call": "dos_close",
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                }
            )
            cpu.reg_write(UC_X86_REG_AX, 0)
            set_carry(cpu, False)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(LOAD_ENTRY, 0, count=2000)

        expected_names = ["resource_source_select"]
        if not embedded:
            expected_names.extend(
                ("dos_get_dta", "dos_find_first", "dos_open_read_only")
            )
        if embedded or open_success:
            expected_names.extend("dos_read" for _ in read_counts)
            if not embedded:
                expected_names.append("dos_close")
        assert [call["call"] for call in calls] == expected_names, (name, calls)
        assert calls[0] == {
            "call": "resource_source_select",
            "path": [DATA_SEGMENT, PATH_OFFSET],
            "si": PATH_OFFSET,
            "return_frame": [0x2E4E, 0],
        }
        if embedded or open_success:
            assert sum(read_counts) == byte_count
            assert read_index == len(read_counts)
            read_calls = [call for call in calls if call["call"] == "dos_read"]
            remaining = byte_count
            expected_segment = DESTINATION_SEGMENT
            expected_offset = destination_offset
            for call, returned in zip(read_calls, read_counts, strict=True):
                assert call["handle"] == selected_handle
                assert call["shared_handle"] == selected_handle
                assert call["destination"] == [expected_segment, expected_offset]
                assert call["requested"] == min(remaining, 0x7D00)
                assert call["remaining_before"] == remaining
                remaining -= returned
                expected_segment = (expected_segment + (returned >> 4)) & 0xFFFF
                expected_offset = (expected_offset + (returned & 0x0F)) & 0xFFFF

        returned_size = byte_count if embedded or open_success else 0
        assert_registers_restored(
            machine,
            initial,
            expected_eax=returned_size,
            expected_ebp=initial["ebp"],
        )
        assert machine.reg_read(UC_X86_REG_EFLAGS) & 1 == 0
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == bool(
            initial["flags"] & 0x0400
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DTA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            dta_expected
        )
        assert bytes(
            machine.mem_read(DESTINATION_SEGMENT * 16, DESTINATION_SIZE)
        ) == bytes(destination_expected)
        assert_common_memory(
            machine, executable, data_before, extra_before, fs_before, stack_before
        )

        vectors.append(
            {
                "name": name,
                "embedded_flag": embedded_flag,
                "byte_count": byte_count,
                "find_success": find_success if not embedded else None,
                "open_success": open_success if not embedded else None,
                "read_counts": read_counts,
                "read_carry_ignored": read_error,
                "returned_size": returned_size,
                "final_shared_handle": (
                    selected_handle
                    if embedded or open_success
                    else initial_shared_handle
                ),
                "calls": calls,
            }
        )
    return vectors


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    spans = (
        ("resource length", LENGTH_ENTRY, LENGTH_END, LENGTH_SHA256),
        ("resource load", LOAD_ENTRY, LOAD_END, LOAD_SHA256),
    )
    for name, start, end, expected in spans:
        actual = hashlib.sha256(executable[start:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"BBB {name} body changed: {actual}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    executable = verify_executable(args.executable)
    result = {
        "format": "big_bug_bang_resource_file_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routine_sha256": {
            "0x2c4a": LENGTH_SHA256,
            "0x2e40": LOAD_SHA256,
        },
        "lengths": length_vectors(executable),
        "loads": load_vectors(executable),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(
        f"verified {len(result['lengths'])} BBB resource-length and "
        f"{len(result['loads'])} conventional-load cases"
    )


if __name__ == "__main__":
    main()
