#!/usr/bin/env python3
"""Verify BBB's unchanged resource file-write coordinator."""

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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_write.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_2b6b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x2EF0
END = 0x2F73
BODY_SHA256 = "c76528ca88c43dc89459af08745032c1798cfff1d33ed9c141efa978ffc6f9e7"
WRITE_DIRECTORY_ENTRY = 0x2B43
SHARED_HANDLE = 0x0C7C
SOURCE_REMAINING = 0x0C8A

DATA_SEGMENT = 0x2000
GAME_SEGMENT = 0x3200
SOURCE_SEGMENT = 0x5000
FS_SEGMENT = 0x7600
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
SOURCE_SIZE = 0x20000
PATH_OFFSET = 0x4100
CALLER_SP = 0xFF00
RETURN_SEGMENT = 0x1800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("1ee187785aa5c33c")

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

CASES = (
    {"name": "small_file", "byte_count": 7, "write_counts": [7]},
    {"name": "create_failure", "byte_count": 17, "create_success": False},
    {"name": "empty_file_writes_zero_once", "byte_count": 0, "write_counts": [0]},
    {
        "name": "low_word_above_nominal_chunk",
        "byte_count": 0x8305,
        "write_counts": [0x8305],
    },
    {
        "name": "high_word_uses_7d00_then_low_word",
        "byte_count": 0x10005,
        "write_counts": [0x7D00, 0x8305],
    },
    {
        "name": "partial_writes_and_offset_wrap",
        "byte_count": 0x7D07,
        "write_counts": [0x7000, 0x0D05, 2],
        "source_offset": 0xFFFC,
    },
    {
        "name": "write_carry_is_ignored",
        "byte_count": 4,
        "write_counts": [4],
        "write_error": True,
    },
    {
        "name": "close_carry_is_ignored",
        "byte_count": 1,
        "write_counts": [1],
        "close_error": True,
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


def assert_stack_ownership(before: bytes | bytearray, after: bytes) -> None:
    allowed = set(range(CALLER_SP - 0x30, CALLER_SP))
    differences = {
        offset for offset, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(offset) for offset in differences)
    assert after[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] == STACK_SENTINEL


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []

    for case_index, case in enumerate(CASES):
        name = str(case["name"])
        byte_count = int(case["byte_count"])
        write_counts = [int(value) for value in case.get("write_counts", [])]
        create_success = bool(case.get("create_success", True))
        write_error = bool(case.get("write_error", False))
        close_error = bool(case.get("close_error", False))
        source_offset = int(case.get("source_offset", 0x0123))
        file_handle = 0x4300 + case_index
        initial_shared_handle = 0xA600 + case_index
        initial_remaining = 0xB7B80000 + case_index
        path = f"WRITE{case_index:02d}.SAV".encode("ascii") + b"\0"
        source_before = bytes(
            ((index * 37 + case_index * 53) & 0xFF) for index in range(SOURCE_SIZE)
        )
        direction_flag = bool(case_index & 1)
        initial = {
            "eax": byte_count,
            "ebx": 0xB2B22340 + case_index,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D44560 + case_index,
            "esi": 0xE5E50000 | PATH_OFFSET,
            "edi": 0xF6F60000 | source_offset,
            "ebp": 0x97977890 + case_index,
            "ds": DATA_SEGMENT,
            "es": SOURCE_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0292 | (0x0400 if direction_flag else 0),
        }
        calls: list[dict[str, Any]] = []
        write_index = 0

        data_before = seeded_region(SEGMENT_SIZE, case_index, 19, 0x31)
        game_before = seeded_region(SEGMENT_SIZE, case_index, 23, 0x43)
        fs_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 29, 0x59))
        stack_before = seeded_region(SEGMENT_SIZE, case_index, 41, 0x8B)
        data_before[PATH_OFFSET : PATH_OFFSET + len(path)] = path
        struct.pack_into("<H", game_before, SHARED_HANDLE, initial_shared_handle)
        struct.pack_into("<I", game_before, SOURCE_REMAINING, initial_remaining)
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x120000)
        machine.mem_write(0, executable)
        machine.mem_write(WRITE_DIRECTORY_ENTRY, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(SOURCE_SEGMENT * 16, source_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_ADDRESS, b"\xcc")

        for register, value in GENERAL_REGISTERS.items():
            machine.reg_write(value, initial[register])
        for register, value in SEGMENT_REGISTERS.items():
            machine.reg_write(value, initial[register])
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_EFLAGS, initial["flags"])
        reached_return: list[int] = []

        def capture(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            if address == WRITE_DIRECTORY_ENTRY:
                calls.append(
                    {
                        "call": "startup_write_directory_enter",
                        "byte_count": cpu.reg_read(UC_X86_REG_EAX),
                        "path": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_SI),
                        ],
                        "source": [
                            cpu.reg_read(UC_X86_REG_ES),
                            cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        ],
                        "remaining_before": game_u32(cpu, SOURCE_REMAINING),
                        "return_frame": return_frame(cpu),
                    }
                )

        def interrupt(cpu: Uc, number: int, _data: object) -> None:
            nonlocal write_index
            assert number == 0x21, f"{name}: unexpected INT {number:#x}"
            function = cpu.reg_read(UC_X86_REG_AX)
            if function == 0x3C00:
                call = {
                    "call": "dos_create_truncate",
                    "path": [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_DX)],
                    "attributes": cpu.reg_read(UC_X86_REG_CX),
                    "remaining": game_u32(cpu, SOURCE_REMAINING),
                    "success": create_success,
                }
                calls.append(call)
                assert call["path"] == [DATA_SEGMENT, PATH_OFFSET], (name, call)
                assert call["attributes"] == 0 and call["remaining"] == byte_count, (
                    name,
                    call,
                )
                cpu.reg_write(UC_X86_REG_AX, file_handle if create_success else 5)
                set_carry(cpu, not create_success)
                return
            if function == 0x4000:
                assert write_index < len(write_counts), (
                    f"{name}: unexpected extra write"
                )
                returned = write_counts[write_index]
                failed = write_error and write_index == 0
                segment = cpu.reg_read(UC_X86_REG_DS)
                offset = cpu.reg_read(UC_X86_REG_DX)
                requested = cpu.reg_read(UC_X86_REG_CX)
                call = {
                    "call": "dos_write",
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "source": [segment, offset],
                    "requested": requested,
                    "returned": returned,
                    "remaining_before": game_u32(cpu, SOURCE_REMAINING),
                    "carry": failed,
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    "payload_prefix": list(
                        cpu.mem_read(segment * 16 + offset, min(requested, 8))
                    ),
                }
                calls.append(call)
                assert call["handle"] == file_handle, (name, call)
                assert call["shared_handle"] == file_handle, (name, call)
                assert returned <= requested, (name, call)
                cpu.reg_write(UC_X86_REG_AX, returned)
                set_carry(cpu, failed)
                write_index += 1
                return
            if function == 0x3E00:
                call = {
                    "call": "dos_close",
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    "carry": close_error,
                }
                calls.append(call)
                assert call["handle"] == file_handle, (name, call)
                assert call["shared_handle"] == file_handle, (name, call)
                cpu.reg_write(UC_X86_REG_AX, 5 if close_error else 0)
                set_carry(cpu, close_error)
                return
            raise AssertionError(f"{name}: unexpected DOS function {function:#x}")

        machine.hook_add(UC_HOOK_CODE, capture)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(ENTRY, 0, count=2000)
        assert reached_return == [RETURN_ADDRESS], (name, reached_return)

        expected_names = ["startup_write_directory_enter", "dos_create_truncate"]
        if create_success:
            expected_names.extend("dos_write" for _ in write_counts)
            expected_names.append("dos_close")
        assert [call["call"] for call in calls] == expected_names, (name, calls)
        assert calls[0] == {
            "call": "startup_write_directory_enter",
            "byte_count": byte_count,
            "path": [DATA_SEGMENT, PATH_OFFSET],
            "source": [SOURCE_SEGMENT, source_offset],
            "remaining_before": initial_remaining,
            "return_frame": [0x2EFC, 0],
        }, (name, calls[0])

        expected_segment = SOURCE_SEGMENT
        expected_offset = source_offset
        remaining = byte_count
        write_calls = [call for call in calls if call["call"] == "dos_write"]
        if create_success:
            assert sum(write_counts) == byte_count
            assert write_index == len(write_counts)
            for call, returned in zip(write_calls, write_counts, strict=True):
                expected_request = 0x7D00 if remaining >> 16 else remaining & 0xFFFF
                assert call["source"] == [expected_segment, expected_offset], (
                    name,
                    call,
                )
                assert call["requested"] == expected_request, (name, call)
                assert call["remaining_before"] == remaining, (name, call)
                region_offset = (
                    expected_segment * 16 + expected_offset - SOURCE_SEGMENT * 16
                )
                expected_prefix = list(
                    source_before[
                        region_offset : region_offset + min(expected_request, 8)
                    ]
                )
                assert call["payload_prefix"] == expected_prefix, (name, call)
                remaining = (remaining - returned) & 0xFFFFFFFF
                expected_segment = (expected_segment + (returned >> 4)) & 0xFFFF
                expected_offset = (expected_offset + (returned & 0x0F)) & 0xFFFF

        expected_return = byte_count if create_success else 0
        expected_handle = file_handle if create_success else initial_shared_handle
        expected_remaining = 0 if create_success else byte_count
        assert machine.reg_read(UC_X86_REG_EAX) == expected_return
        assert game_u16(machine, SHARED_HANDLE) == expected_handle
        assert game_u32(machine, SOURCE_REMAINING) == expected_remaining

        expected_registers = {key: initial[key] for key in GENERAL_REGISTERS}
        expected_registers.update({key: initial[key] for key in SEGMENT_REGISTERS})
        expected_registers["eax"] = expected_return
        expected_dx = PATH_OFFSET if not create_success else expected_offset
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | expected_dx
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, {
            key: (actual_registers[key], value)
            for key, value in expected_registers.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_carry = close_error if create_success else False
        assert bool(flags & 1) == expected_carry, name
        assert bool(flags & 0x0400) == direction_flag, name
        if not create_success:
            assert flags & 0x0040, name

        game_expected = bytearray(game_before)
        struct.pack_into("<H", game_expected, SHARED_HANDLE, expected_handle)
        struct.pack_into("<I", game_expected, SOURCE_REMAINING, expected_remaining)
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_expected
        assert (
            bytes(machine.mem_read(SOURCE_SEGMENT * 16, SOURCE_SIZE)) == source_before
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_stack_ownership(stack_before, stack_after)

        rows.append(
            {
                "name": name,
                "byte_count": byte_count,
                "create_success": create_success,
                "write_counts": write_counts,
                "write_carry_ignored": write_error,
                "close_carry_ignored": close_error,
                "returned_size": expected_return,
                "final_shared_handle": expected_handle,
                "final_remaining": expected_remaining,
                "calls": calls,
            }
        )

    return rows


def verify_commander_semantic_partition(rows: list[dict[str, Any]]) -> str:
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(rows) == len(expected_rows)
    comparable_fields = (
        "name",
        "byte_count",
        "create_success",
        "write_counts",
        "write_carry_ignored",
        "close_carry_ignored",
        "returned_size",
    )
    for row, expected in zip(rows, expected_rows, strict=True):
        for field in comparable_fields:
            assert row[field] == expected[field], (field, row["name"])
        assert [call["call"] for call in row["calls"]] == [
            call["call"] for call in expected["calls"]
        ], row["name"]
    return hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    actual = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if actual != BODY_SHA256:
        raise SystemExit(f"BBB resource writer body changed: {actual}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_resource_write_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantic_partition(rows),
        "routine_sha256": BODY_SHA256,
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB resource-write cases")


if __name__ == "__main__":
    main()
