#!/usr/bin/env python3
"""Verify BBB's unchanged XMS and EMS resource-loading adapters."""

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
    UC_X86_REG_AH,
    UC_X86_REG_AL,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_extended_memory_load.json"
COMMANDER_XMS_FIXTURE = ROOT / "re/tools/oracle_vectors/func_2901_natural.json"
COMMANDER_EMS_FIXTURE = ROOT / "re/tools/oracle_vectors/func_29f2_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

SOURCE_SELECT = 0x2A13
XMS_ENTRY = 0x2C86
XMS_END = 0x2D77
XMS_SHA256 = "82ef6e361b7fc8b649e9b399175515edeec92fb11030f2764d30f199196f517a"
EMS_ENTRY = 0x2D77
EMS_END = 0x2E40
EMS_SHA256 = "30f7a71bf9156835fe7d14027d72e047cd85420f41a000b23c9c1808bca8fdf1"

DATA_SEGMENT = 0x2000
GAME_SEGMENT = 0x3200
DTA_SEGMENT = 0x4400
STORAGE_SEGMENT = 0x5800
EXTRA_SEGMENT = 0x6A00
CALLBACK_SEGMENT = 0x7B00
CALLBACK_OFFSET = 0x0100
CALLBACK_ADDRESS = CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET
FS_SEGMENT = 0x7D00
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
STORAGE_SIZE = 0x10002
CALLER_SP = 0xFF00
RETURN_SEGMENT = 0x1800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16
PATH_OFFSET = 0x4100
STAGING_OFFSET = 0x1200
STACK_SENTINEL = bytes.fromhex("3cc387e11e785aa5")

XMS_ENTRYPOINT = 0x0C42
STORAGE_CURSOR = 0x0C46
PUBLISHED_SIZE = 0x0C4A
XMS_HANDLE = 0x0C4E
EMS_HANDLE = 0x0C50
PAGE_FRAME_SEGMENT = 0x0C5E
XMS_REQUEST = 0x0C64
SHARED_HANDLE = 0x0C7C
SOURCE_SIZE = 0x0C86
SOURCE_REMAINING = 0x0C8A
EMBEDDED_FLAG = 0x0CEB

INITIAL_REQUEST = bytes.fromhex("a55a69968778c33cf00f1ee1d22d4bb4")
REQUEST_DECOY = bytes.fromhex("4bb42dd21ee10ff03cc3877869965aa5")

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

XMS_CASES = (
    {
        "name": "embedded_odd_single_move",
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
        "dta_offset": 0xFFF0,
    },
    {
        "name": "standalone_open_failure",
        "embedded_flag": 0,
        "byte_count": 17,
        "open_success": False,
    },
    {
        "name": "empty_file_moves_zero_bytes",
        "embedded_flag": 0,
        "byte_count": 0,
        "read_counts": [0],
    },
    {
        "name": "partial_reads_use_fixed_xms_stride",
        "embedded_flag": 0,
        "byte_count": 0x7D07,
        "read_counts": [0x7001, 0x0D04, 2],
    },
    {
        "name": "xms_and_read_errors_are_ignored",
        "embedded_flag": 1,
        "byte_count": 4,
        "read_counts": [4],
        "backend_error": True,
        "read_error": True,
    },
    {
        "name": "nonzero_upper_ecx_keeps_nominal_request",
        "embedded_flag": 1,
        "byte_count": 7,
        "read_counts": [7],
        "ecx_high": 0xC3C30000,
    },
    {
        "name": "direction_flag_reverses_request_stores",
        "embedded_flag": 1,
        "byte_count": 4,
        "read_counts": [4],
        "direction_flag": True,
    },
)

EMS_CASES = (
    {
        "name": "embedded_single_read",
        "embedded_flag": 1,
        "byte_count": 7,
        "read_counts": [7],
    },
    {
        "name": "embedded_bit_zero_and_full_u32",
        "embedded_flag": 3,
        "byte_count": 0x10005,
        "read_counts": [0x8000, 0x8000, 5],
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
        "dta_offset": 0xFFF0,
    },
    {
        "name": "standalone_open_failure",
        "embedded_flag": 0,
        "byte_count": 17,
        "open_success": False,
    },
    {
        "name": "empty_file_maps_and_reads_zero",
        "embedded_flag": 0,
        "byte_count": 0,
        "read_counts": [0],
    },
    {
        "name": "partial_reads_remap_page_pairs",
        "embedded_flag": 0,
        "byte_count": 0x8007,
        "read_counts": [0x7000, 0x1005, 2],
    },
    {
        "name": "ems_and_read_errors_are_ignored",
        "embedded_flag": 1,
        "byte_count": 4,
        "read_counts": [4],
        "backend_error": True,
        "read_error": True,
    },
    {
        "name": "nonzero_upper_ecx_keeps_nominal_request",
        "embedded_flag": 1,
        "byte_count": 7,
        "read_counts": [7],
        "ecx_high": 0xC3C30000,
    },
)


def seeded_region(size: int, case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(size)
    )


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


def expected_request_size(remaining: int, ecx_high: int, chunk_size: int) -> int:
    difference = (remaining - (ecx_high | chunk_size)) & 0xFFFFFFFF
    if difference & 0x80000000:
        return remaining & 0xFFFF
    return chunk_size


class LoaderHarness:
    def __init__(
        self,
        executable: bytes,
        case_index: int,
        case: dict[str, object],
        *,
        backend: str,
    ) -> None:
        self.executable = executable
        self.case_index = case_index
        self.case = case
        self.backend = backend
        self.name = str(case["name"])
        self.embedded_flag = int(case["embedded_flag"])
        self.embedded = self.embedded_flag & 1 != 0
        self.byte_count = int(case["byte_count"])
        self.read_counts = [int(value) for value in case.get("read_counts", [])]
        self.find_success = bool(case.get("find_success", True))
        self.open_success = bool(case.get("open_success", True))
        self.backend_error = bool(case.get("backend_error", False))
        self.read_error = bool(case.get("read_error", False))
        self.direction_flag = bool(case.get("direction_flag", False))
        self.ecx_high = int(case.get("ecx_high", 0))
        self.dta_offset = int(case.get("dta_offset", 0x0200))
        self.dta_size_offset = (self.dta_offset + 0x1A) & 0xFFFF
        self.embedded_handle = 0x3100 + case_index
        self.standalone_handle = 0x4100 + case_index
        self.selected_handle = (
            self.embedded_handle if self.embedded else self.standalone_handle
        )
        self.backend_handle = 0x5200 + case_index
        self.initial_shared_handle = 0xA700 + case_index
        self.initial_cursor = 0xEBEC0000 + case_index
        self.initial_published_size = 0xB8B90000 + case_index
        self.path = f"{backend.upper()}{case_index:02d}.DAT".encode("ascii") + b"\0"
        self.calls: list[dict[str, Any]] = []
        self.read_index = 0

        initial_es = STORAGE_SEGMENT if backend == "xms" else EXTRA_SEGMENT
        self.initial = {
            "eax": 0xA1A11230 + case_index,
            "ebx": 0xB2B22340 + case_index,
            "ecx": self.ecx_high | (0x3450 + case_index),
            "edx": 0xD4D44560 + case_index,
            "esi": 0xE5E50000 | PATH_OFFSET,
            "edi": 0xF6F60000 | STAGING_OFFSET,
            "ebp": 0x97977890 + case_index,
            "ds": DATA_SEGMENT,
            "es": initial_es,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0292 | (0x0400 if self.direction_flag else 0),
        }

        self.data_before = seeded_region(SEGMENT_SIZE, case_index, 19, 0x31)
        self.data_before[PATH_OFFSET : PATH_OFFSET + len(self.path)] = self.path
        self.data_before[XMS_REQUEST : XMS_REQUEST + len(REQUEST_DECOY)] = REQUEST_DECOY
        self.game_before = seeded_region(SEGMENT_SIZE, case_index, 7, 0x53)
        struct.pack_into(
            "<HH", self.game_before, XMS_ENTRYPOINT, CALLBACK_OFFSET, CALLBACK_SEGMENT
        )
        struct.pack_into("<I", self.game_before, STORAGE_CURSOR, self.initial_cursor)
        struct.pack_into(
            "<I", self.game_before, PUBLISHED_SIZE, self.initial_published_size
        )
        struct.pack_into("<H", self.game_before, XMS_HANDLE, self.backend_handle)
        struct.pack_into("<H", self.game_before, EMS_HANDLE, self.backend_handle)
        struct.pack_into("<H", self.game_before, PAGE_FRAME_SEGMENT, STORAGE_SEGMENT)
        self.game_before[XMS_REQUEST : XMS_REQUEST + len(INITIAL_REQUEST)] = (
            INITIAL_REQUEST
        )
        struct.pack_into(
            "<H", self.game_before, SHARED_HANDLE, self.initial_shared_handle
        )
        struct.pack_into("<I", self.game_before, SOURCE_SIZE, 0xC9CA0000 + case_index)
        struct.pack_into(
            "<I", self.game_before, SOURCE_REMAINING, 0xDADB0000 + case_index
        )
        self.game_before[EMBEDDED_FLAG] = self.embedded_flag ^ 0xFF
        self.game_expected = bytearray(self.game_before)

        self.dta_before = seeded_region(SEGMENT_SIZE, case_index, 11, 0x75)
        initial_dta_size = (
            self.byte_count
            if not self.find_success
            else (self.byte_count ^ 0xFFFFFFFF) & 0xFFFFFFFF
        )
        struct.pack_into("<I", self.dta_before, self.dta_size_offset, initial_dta_size)
        self.dta_expected = bytearray(self.dta_before)
        if not self.embedded and self.find_success:
            struct.pack_into(
                "<I", self.dta_expected, self.dta_size_offset, self.byte_count
            )

        self.storage_before = seeded_region(STORAGE_SIZE, case_index, 13, 0x97)
        self.storage_expected = bytearray(self.storage_before)
        self.extra_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 23, 0xB9))
        self.fs_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 31, 0xDB))
        self.stack_before = seeded_region(SEGMENT_SIZE, case_index, 37, 0xFD)
        self.stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        self.stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        self.machine = Uc(UC_ARCH_X86, UC_MODE_16)
        self.machine.mem_map(0, 0x100000)
        self.machine.mem_write(0, executable)
        self.machine.mem_write(SOURCE_SELECT, b"\xcb")
        self.machine.mem_write(DATA_SEGMENT * 16, bytes(self.data_before))
        self.machine.mem_write(GAME_SEGMENT * 16, bytes(self.game_before))
        self.machine.mem_write(DTA_SEGMENT * 16, bytes(self.dta_before))
        self.machine.mem_write(STORAGE_SEGMENT * 16, bytes(self.storage_before))
        self.machine.mem_write(EXTRA_SEGMENT * 16, self.extra_before)
        self.machine.mem_write(FS_SEGMENT * 16, self.fs_before)
        self.machine.mem_write(STACK_SEGMENT * 16, bytes(self.stack_before))
        for name, register in GENERAL_REGISTERS.items():
            self.machine.reg_write(register, self.initial[name])
        for name, register in SEGMENT_REGISTERS.items():
            self.machine.reg_write(register, self.initial[name])
        self.machine.reg_write(UC_X86_REG_CS, 0)
        self.machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        self.machine.reg_write(UC_X86_REG_EFLAGS, self.initial["flags"])

    @property
    def succeeded(self) -> bool:
        return self.embedded or self.open_success

    def select_source(self, machine: Uc, expected_return: int) -> None:
        frame = return_frame(machine)
        call = {
            "call": "resource_source_select",
            "path": [
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_DX),
            ],
            "si": machine.reg_read(UC_X86_REG_SI),
            "return_frame": frame,
        }
        self.calls.append(call)
        assert call == {
            "call": "resource_source_select",
            "path": [DATA_SEGMENT, PATH_OFFSET],
            "si": PATH_OFFSET,
            "return_frame": [expected_return, 0],
        }
        machine.mem_write(
            GAME_SEGMENT * 16 + EMBEDDED_FLAG, bytes((self.embedded_flag,))
        )
        if self.embedded:
            machine.mem_write(
                GAME_SEGMENT * 16 + SOURCE_SIZE, struct.pack("<I", self.byte_count)
            )
            machine.mem_write(
                GAME_SEGMENT * 16 + SOURCE_REMAINING,
                struct.pack("<I", self.byte_count),
            )
            machine.reg_write(UC_X86_REG_BX, self.embedded_handle)
        else:
            machine.reg_write(UC_X86_REG_BX, 0x7100 + self.case_index)
        simulate_far_return(machine)

    def handle_dos(self, machine: Uc, number: int, chunk_size: int) -> None:
        assert number == 0x21, hex(number)
        function = machine.reg_read(UC_X86_REG_AX)
        if function == 0x2F00:
            self.calls.append({"call": "dos_get_dta"})
            machine.reg_write(UC_X86_REG_ES, DTA_SEGMENT)
            machine.reg_write(UC_X86_REG_BX, self.dta_offset)
            return
        if function == 0x4E00:
            call = {
                "call": "dos_find_first",
                "path": [
                    machine.reg_read(UC_X86_REG_DS),
                    machine.reg_read(UC_X86_REG_DX),
                ],
                "attributes": machine.reg_read(UC_X86_REG_CX),
                "success": self.find_success,
            }
            self.calls.append(call)
            assert call["path"] == [DATA_SEGMENT, PATH_OFFSET]
            assert call["attributes"] == 0
            if self.find_success:
                machine.mem_write(
                    DTA_SEGMENT * 16 + self.dta_size_offset,
                    struct.pack("<I", self.byte_count),
                )
                machine.reg_write(UC_X86_REG_AX, 0)
            else:
                machine.reg_write(UC_X86_REG_AX, 2)
            set_carry(machine, not self.find_success)
            return
        if function == 0x3D00:
            call = {
                "call": "dos_open_read_only",
                "path": [
                    machine.reg_read(UC_X86_REG_DS),
                    machine.reg_read(UC_X86_REG_DX),
                ],
                "source_size": game_u32(machine, SOURCE_SIZE),
                "remaining": game_u32(machine, SOURCE_REMAINING),
                "success": self.open_success,
            }
            self.calls.append(call)
            assert call["path"] == [DATA_SEGMENT, PATH_OFFSET]
            assert call["source_size"] == self.byte_count
            assert call["remaining"] == self.byte_count
            machine.reg_write(
                UC_X86_REG_AX,
                self.standalone_handle if self.open_success else 2,
            )
            set_carry(machine, not self.open_success)
            return
        if function == 0x3F00:
            assert self.read_index < len(self.read_counts), (
                self.name,
                "unexpected read",
            )
            remaining = game_u32(machine, SOURCE_REMAINING)
            expected_request = expected_request_size(
                remaining, self.ecx_high, chunk_size
            )
            returned = self.read_counts[self.read_index]
            assert returned <= expected_request
            failed = self.read_error and self.read_index == 0
            destination = [
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_DX),
            ]
            expected_destination = [
                STORAGE_SEGMENT,
                STAGING_OFFSET if self.backend == "xms" else 0,
            ]
            call = {
                "call": "dos_read",
                "handle": machine.reg_read(UC_X86_REG_BX),
                "destination": destination,
                "requested": machine.reg_read(UC_X86_REG_CX),
                "returned": returned,
                "remaining_before": remaining,
                "carry": failed,
                "shared_handle": game_u16(machine, SHARED_HANDLE),
                "cursor": game_u32(machine, STORAGE_CURSOR),
            }
            self.calls.append(call)
            assert call["handle"] == self.selected_handle
            assert call["destination"] == expected_destination
            assert call["requested"] == expected_request
            assert call["shared_handle"] == self.selected_handle
            payload = bytes(
                (
                    index * 31
                    + self.read_index * 43
                    + self.case_index * 59
                    + (17 if self.backend == "ems" else 0)
                )
                & 0xFF
                for index in range(returned)
            )
            target_offset = expected_destination[1]
            if payload:
                machine.mem_write(
                    STORAGE_SEGMENT * 16 + target_offset,
                    payload,
                )
                self.storage_expected[target_offset : target_offset + returned] = (
                    payload
                )
            machine.reg_write(UC_X86_REG_AX, returned)
            set_carry(machine, failed)
            self.read_index += 1
            return
        assert function == 0x3E00, hex(function)
        call = {
            "call": "dos_close",
            "handle": machine.reg_read(UC_X86_REG_BX),
            "shared_handle": game_u16(machine, SHARED_HANDLE),
        }
        self.calls.append(call)
        assert call["handle"] == self.standalone_handle
        assert call["shared_handle"] == self.standalone_handle
        machine.reg_write(UC_X86_REG_AX, 0)
        set_carry(machine, False)

    def finish_expected_game(self, cursor: int) -> None:
        self.game_expected[EMBEDDED_FLAG] = self.embedded_flag
        struct.pack_into("<I", self.game_expected, SOURCE_SIZE, self.byte_count)
        struct.pack_into(
            "<I",
            self.game_expected,
            SOURCE_REMAINING,
            0 if self.succeeded else self.byte_count,
        )
        if self.succeeded:
            struct.pack_into(
                "<H", self.game_expected, SHARED_HANDLE, self.selected_handle
            )
            struct.pack_into("<I", self.game_expected, STORAGE_CURSOR, cursor)
            struct.pack_into("<I", self.game_expected, PUBLISHED_SIZE, self.byte_count)

    def assert_final_state(self) -> None:
        actual_registers = {
            name: self.machine.reg_read(register)
            for name, register in GENERAL_REGISTERS.items()
        }
        actual_registers.update(
            {
                name: self.machine.reg_read(register)
                for name, register in SEGMENT_REGISTERS.items()
            }
        )
        expected_registers = dict(self.initial)
        del expected_registers["flags"]
        assert actual_registers == expected_registers, {
            name: (actual_registers[name], value)
            for name, value in expected_registers.items()
            if actual_registers[name] != value
        }
        assert self.machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert self.machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        flags = self.machine.reg_read(UC_X86_REG_EFLAGS)
        assert flags & 1 == 0
        assert bool(flags & 0x0400) == self.direction_flag
        if not self.succeeded:
            assert flags & 0x40

        expected_image = bytearray(self.executable)
        expected_image[SOURCE_SELECT] = 0xCB
        assert bytes(self.machine.mem_read(0, len(self.executable))) == bytes(
            expected_image
        )
        assert bytes(self.machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            self.data_before
        )
        game_after = bytes(self.machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
        game_differences = [
            (offset, actual, expected)
            for offset, (actual, expected) in enumerate(
                zip(game_after, self.game_expected)
            )
            if actual != expected
        ]
        assert not game_differences, (
            self.name,
            game_differences[:32],
            game_after[0x0C50:0x0C78].hex(),
            bytes(self.game_expected[0x0C50:0x0C78]).hex(),
        )
        assert bytes(self.machine.mem_read(DTA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            self.dta_expected
        )
        assert bytes(
            self.machine.mem_read(STORAGE_SEGMENT * 16, STORAGE_SIZE)
        ) == bytes(self.storage_expected)
        assert (
            bytes(self.machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE))
            == self.extra_before
        )
        assert (
            bytes(self.machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE))
            == self.fs_before
        )
        stack_after = bytes(self.machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        differences = {
            offset
            for offset, (old, new) in enumerate(zip(self.stack_before, stack_after))
            if old != new
        }
        assert differences <= set(range(CALLER_SP - 0x50, CALLER_SP)), sorted(
            hex(offset) for offset in differences
        )
        assert (
            stack_after[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)]
            == STACK_SENTINEL
        )

    def semantic_row(self, cursor: int) -> dict[str, Any]:
        return {
            "name": self.name,
            "embedded_flag": self.embedded_flag,
            "byte_count": self.byte_count,
            "find_success": self.find_success if not self.embedded else None,
            "open_success": self.open_success if not self.embedded else None,
            "read_counts": self.read_counts,
            "backend_errors_ignored": self.backend_error,
            "read_carry_ignored": self.read_error,
            "upper_ecx": self.ecx_high,
            "direction_flag": self.direction_flag,
            "succeeded": self.succeeded,
            "final_storage_cursor": cursor if self.succeeded else self.initial_cursor,
            "published_size": (
                self.byte_count if self.succeeded else self.initial_published_size
            ),
            "calls": self.calls,
        }


def expected_call_names(harness: LoaderHarness, backend: str) -> list[str]:
    names = ["resource_source_select"]
    if not harness.embedded:
        names.extend(("dos_get_dta", "dos_find_first", "dos_open_read_only"))
    if harness.succeeded:
        for _ in harness.read_counts:
            if backend == "xms":
                names.extend(("dos_read", "xms_move"))
            else:
                names.extend(("ems_map_page", "ems_map_page", "dos_read"))
        if not harness.embedded:
            names.append("dos_close")
    return names


def xms_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    for case_index, case in enumerate(XMS_CASES):
        harness = LoaderHarness(executable, case_index, case, backend="xms")
        machine = harness.machine
        move_index = 0

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            nonlocal move_index
            if address == RETURN_ADDRESS:
                cpu.emu_stop()
                return
            if address == SOURCE_SELECT:
                harness.select_source(cpu, 0x2C96)
                return
            if address == CALLBACK_ADDRESS:
                assert move_index < len(harness.read_counts)
                frame = return_frame(cpu)
                assert frame == [0x2D41, 0]
                request = bytes(
                    cpu.mem_read(GAME_SEGMENT * 16 + XMS_REQUEST, len(INITIAL_REQUEST))
                )
                fields = struct.unpack("<IHHHHI", request)
                returned = harness.read_counts[move_index]
                rounded = (returned + 1) & ~1
                cursor_before = move_index * 0x7D00
                if harness.direction_flag:
                    expected_request = bytearray(INITIAL_REQUEST)
                    struct.pack_into("<I", expected_request, 0, rounded)
                    struct.pack_into("<I", harness.game_expected, XMS_REQUEST, rounded)
                    struct.pack_into("<H", harness.game_expected, XMS_REQUEST - 4, 0)
                    struct.pack_into(
                        "<H",
                        harness.game_expected,
                        XMS_REQUEST - 6,
                        STAGING_OFFSET,
                    )
                    struct.pack_into(
                        "<H",
                        harness.game_expected,
                        XMS_REQUEST - 8,
                        STORAGE_SEGMENT,
                    )
                    struct.pack_into(
                        "<H",
                        harness.game_expected,
                        XMS_REQUEST - 10,
                        harness.backend_handle,
                    )
                    struct.pack_into(
                        "<I",
                        harness.game_expected,
                        XMS_REQUEST - 12,
                        cursor_before,
                    )
                else:
                    expected_request = struct.pack(
                        "<IHHHHI",
                        rounded,
                        0,
                        STAGING_OFFSET,
                        STORAGE_SEGMENT,
                        harness.backend_handle,
                        cursor_before,
                    )
                    harness.game_expected[
                        XMS_REQUEST : XMS_REQUEST + len(expected_request)
                    ] = expected_request
                assert request == bytes(expected_request), (harness.name, request.hex())
                (
                    length,
                    source_handle,
                    source_offset,
                    source_segment,
                    destination_handle,
                    destination_offset,
                ) = fields
                call = {
                    "call": "xms_move",
                    "function": cpu.reg_read(UC_X86_REG_EAX),
                    "request": [
                        cpu.reg_read(UC_X86_REG_DS),
                        cpu.reg_read(UC_X86_REG_SI),
                    ],
                    "length": length,
                    "source_handle": source_handle,
                    "source": [source_segment, source_offset],
                    "destination_handle": destination_handle,
                    "destination_offset": destination_offset,
                    "cursor_after": game_u32(cpu, STORAGE_CURSOR),
                    "success": not harness.backend_error,
                    "return_frame": frame,
                }
                harness.calls.append(call)
                assert call["function"] == 0x0B00
                assert call["request"] == [GAME_SEGMENT, XMS_REQUEST]
                assert call["cursor_after"] == (move_index + 1) * 0x7D00
                if not harness.direction_flag:
                    assert call["source_handle"] == 0
                    assert call["source"] == [STORAGE_SEGMENT, STAGING_OFFSET]
                    assert call["destination_handle"] == harness.backend_handle
                    assert call["destination_offset"] == cursor_before
                    source = bytes(
                        cpu.mem_read(
                            STORAGE_SEGMENT * 16 + STAGING_OFFSET,
                            rounded,
                        )
                    )
                    assert source == bytes(
                        harness.storage_expected[
                            STAGING_OFFSET : STAGING_OFFSET + rounded
                        ]
                    )
                    call["source_sha256"] = hashlib.sha256(source).hexdigest()
                    call["rounded_tail"] = source[-1] if source else None
                struct.pack_into(
                    "<I",
                    harness.game_expected,
                    STORAGE_CURSOR,
                    (move_index + 1) * 0x7D00,
                )
                cpu.reg_write(UC_X86_REG_AX, 0 if harness.backend_error else 1)
                move_index += 1
                simulate_far_return(cpu)
                return
            assert XMS_ENTRY <= address and address + size <= XMS_END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            harness.handle_dos(cpu, number, 0x7D00)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(XMS_ENTRY, 0, count=5000)

        assert [call["call"] for call in harness.calls] == expected_call_names(
            harness, "xms"
        )
        if harness.succeeded:
            assert sum(harness.read_counts) == harness.byte_count
            assert harness.read_index == len(harness.read_counts)
            assert move_index == len(harness.read_counts)
        expected_cursor = len(harness.read_counts) * 0x7D00
        harness.finish_expected_game(expected_cursor)
        harness.assert_final_state()
        row = harness.semantic_row(expected_cursor)
        row["backward_store_window"] = (
            list(
                machine.mem_read(
                    GAME_SEGMENT * 16 + XMS_REQUEST - 14,
                    18,
                )
            )
            if harness.direction_flag
            else None
        )
        vectors.append(row)
    return vectors


def ems_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    for case_index, case in enumerate(EMS_CASES):
        harness = LoaderHarness(executable, case_index, case, backend="ems")
        machine = harness.machine
        map_index = 0

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == RETURN_ADDRESS:
                cpu.emu_stop()
                return
            if address == SOURCE_SELECT:
                harness.select_source(cpu, 0x2D86)
                return
            assert EMS_ENTRY <= address and address + size <= EMS_END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            nonlocal map_index
            if number == 0x67:
                assert cpu.reg_read(UC_X86_REG_AH) == 0x44
                call = {
                    "call": "ems_map_page",
                    "handle": cpu.reg_read(UC_X86_REG_DX),
                    "logical_page": cpu.reg_read(UC_X86_REG_BX),
                    "physical_page": cpu.reg_read(UC_X86_REG_AL),
                    "cursor_before": game_u32(cpu, STORAGE_CURSOR),
                    "success": not harness.backend_error,
                }
                harness.calls.append(call)
                assert call["handle"] == harness.backend_handle
                assert call["logical_page"] == map_index
                assert call["physical_page"] == map_index & 1
                cpu.reg_write(UC_X86_REG_AH, 0x80 if harness.backend_error else 0)
                map_index += 1
                return
            harness.handle_dos(cpu, number, 0x8000)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(EMS_ENTRY, 0, count=5000)

        assert [call["call"] for call in harness.calls] == expected_call_names(
            harness, "ems"
        )
        if harness.succeeded:
            assert sum(harness.read_counts) == harness.byte_count
            assert harness.read_index == len(harness.read_counts)
            assert map_index == len(harness.read_counts) * 2
        expected_cursor = len(harness.read_counts) * 2
        harness.finish_expected_game(expected_cursor)
        harness.assert_final_state()
        vectors.append(harness.semantic_row(expected_cursor))
    return vectors


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    spans = (
        ("XMS loader", XMS_ENTRY, XMS_END, XMS_SHA256),
        ("EMS loader", EMS_ENTRY, EMS_END, EMS_SHA256),
    )
    for name, start, end, expected in spans:
        actual = hashlib.sha256(executable[start:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"BBB {name} body changed: {actual}")
    return executable


def verify_commander_semantic_partition(
    rows: list[dict[str, Any]], fixture: Path
) -> str:
    expected_rows = json.loads(fixture.read_text())
    assert len(rows) == len(expected_rows)
    comparable_fields = (
        "name",
        "embedded_flag",
        "byte_count",
        "find_success",
        "open_success",
        "read_counts",
        "upper_ecx",
        "final_storage_cursor",
    )
    for row, expected in zip(rows, expected_rows, strict=True):
        for field in comparable_fields:
            assert row[field] == expected[field], (fixture.name, field, row["name"])
        assert [call["call"] for call in row["calls"]] == [
            call["call"] for call in expected["calls"]
        ], (fixture.name, row["name"])
    return hashlib.sha256(fixture.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    executable = verify_executable(args.executable)
    xms = xms_vectors(executable)
    ems = ems_vectors(executable)
    result = {
        "format": "big_bug_bang_extended_memory_load_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": {
            "func_2901_natural.json": verify_commander_semantic_partition(
                xms, COMMANDER_XMS_FIXTURE
            ),
            "func_29f2_natural.json": verify_commander_semantic_partition(
                ems, COMMANDER_EMS_FIXTURE
            ),
        },
        "routine_sha256": {
            "0x2c86": XMS_SHA256,
            "0x2d77": EMS_SHA256,
        },
        "xms": xms,
        "ems": ems,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(
        f"verified {len(result['xms'])} BBB XMS and "
        f"{len(result['ems'])} EMS resource-load cases"
    )


if __name__ == "__main__":
    main()
