#!/usr/bin/env python3
"""Verify BBB's unchanged resource-ID load coordinator."""

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

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_load_by_id.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_287b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x2BFB
END = 0x2C4A
BODY_SHA256 = "79467cc75e984fc5788985b0d0c10d0fc8e583431b791217dde9b77ef089e8d7"
LOOKUP_ENTRY = 0x2C4A
ALLOCATOR_SEGMENT = 0x04E1
ALLOCATOR_OFFSET = 0
ALLOCATOR_ADDRESS = ALLOCATOR_SEGMENT * 16 + ALLOCATOR_OFFSET
FILE_LOAD_ENTRY = 0x2E40
NAME_TABLE = 0x0C04

DATA_SEGMENT = 0x2000
GAME_SEGMENT = 0x3200
NAME_SEGMENT = 0x4400
DESTINATION_SEGMENT_BASE = 0x5800
EXTRA_SEGMENT = 0x6800
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_SEGMENT = 0x1800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

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
    {
        "name": "lookup_zero_skips_allocation",
        "resource_id": 3,
        "byte_count": 0,
        "allocation_status": None,
        "file_result": None,
    },
    {
        "name": "allocation_failure_skips_file_load",
        "resource_id": 4,
        "byte_count": 0x12345678,
        "allocation_status": -1,
        "file_result": None,
    },
    {
        "name": "already_loaded_is_success",
        "resource_id": 25,
        "byte_count": 0x00018002,
        "allocation_status": 1,
        "file_result": None,
    },
    {
        "name": "any_positive_allocation_status_is_success",
        "resource_id": 31,
        "byte_count": 1,
        "allocation_status": 0x7FFF,
        "file_result": None,
    },
    {
        "name": "fresh_buffer_file_failure",
        "resource_id": 44,
        "byte_count": 0x00007D00,
        "allocation_status": 0,
        "destination_offset": 0x0123,
        "file_result": 0,
    },
    {
        "name": "fresh_buffer_file_success",
        "resource_id": 52,
        "byte_count": 0x00010001,
        "allocation_status": 0,
        "destination_offset": 0xFFFE,
        "file_result": 1,
    },
    {
        "name": "full_dword_file_result_is_tested",
        "resource_id": 63,
        "byte_count": 0xFFFFFFFF,
        "allocation_status": 0,
        "destination_offset": 0x4567,
        "file_result": 0x80000000,
    },
    {
        "name": "resource_name_index_wraps_to_sixteen_bits",
        "resource_id": 0xFFF0,
        "byte_count": 0x89ABCDEF,
        "allocation_status": 0,
        "destination_offset": 0x89AB,
        "file_result": 0x00010000,
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


def return_frame(machine: Uc) -> list[int]:
    stack_pointer = machine.reg_read(UC_X86_REG_SP)
    return list(
        struct.unpack("<HH", machine.mem_read(STACK_SEGMENT * 16 + stack_pointer, 4))
    )


def defined_or_flags(value: int, width: int, direction_flag: bool) -> dict[str, bool]:
    mask = (1 << width) - 1
    value &= mask
    return {
        "cf": False,
        "pf": (value & 0xFF).bit_count() % 2 == 0,
        "zf": value == 0,
        "sf": bool(value & (1 << (width - 1))),
        "df": direction_flag,
        "of": False,
    }


def snapshot_defined_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(flags & 0x0001),
        "pf": bool(flags & 0x0004),
        "zf": bool(flags & 0x0040),
        "sf": bool(flags & 0x0080),
        "df": bool(flags & 0x0400),
        "of": bool(flags & 0x0800),
    }


def assert_stack_ownership(before: bytes | bytearray, after: bytes) -> None:
    allowed = set(range(CALLER_SP - 0x20, CALLER_SP))
    differences = {
        offset for offset, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(offset) for offset in differences)
    assert after[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] == STACK_SENTINEL


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    stub_addresses = (LOOKUP_ENTRY, ALLOCATOR_ADDRESS, FILE_LOAD_ENTRY)

    for case_index, case in enumerate(CASES):
        name = str(case["name"])
        resource_id = int(case["resource_id"])
        byte_count = int(case["byte_count"])
        allocation_status_value = case["allocation_status"]
        allocation_status = (
            None if allocation_status_value is None else int(allocation_status_value)
        )
        file_result_value = case["file_result"]
        file_result = None if file_result_value is None else int(file_result_value)
        destination_segment = DESTINATION_SEGMENT_BASE
        destination_offset = int(case.get("destination_offset", 0x1357))
        filename_offset = (NAME_TABLE + ((resource_id << 4) & 0xFFFF)) & 0xFFFF
        filename = (f"RESOURCE{case_index:02d}.DAT".encode("ascii") + b"\0").ljust(
            16, b"\0"
        )
        decoy = bytes(value ^ 0xFF for value in filename)
        direction_flag = bool(case_index & 1)
        initial = {
            "eax": 0xA1A10000 | resource_id,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": NAME_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0292 | (0x0400 if direction_flag else 0),
        }
        calls: list[dict[str, Any]] = []

        data_before = seeded_region(SEGMENT_SIZE, case_index, 19, 0x31)
        game_before = seeded_region(SEGMENT_SIZE, case_index, 23, 0x43)
        name_before = seeded_region(SEGMENT_SIZE, case_index, 29, 0x59)
        destination_before = seeded_region(SEGMENT_SIZE, case_index, 31, 0x67)
        extra_before = seeded_region(SEGMENT_SIZE, case_index, 37, 0x79)
        stack_before = seeded_region(SEGMENT_SIZE, case_index, 41, 0x8B)
        name_before[filename_offset : filename_offset + len(filename)] = filename
        data_before[filename_offset : filename_offset + len(decoy)] = decoy
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        for address in stub_addresses:
            machine.mem_write(address, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(NAME_SEGMENT * 16, bytes(name_before))
        machine.mem_write(destination_segment * 16, bytes(destination_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
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
            if address == LOOKUP_ENTRY:
                calls.append(
                    {
                        "call": "resource_name_lookup",
                        "resource_id": cpu.reg_read(UC_X86_REG_AX),
                        "filename": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_SI),
                        ],
                        "filename_backup_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "return_frame": return_frame(cpu),
                    }
                )
                cpu.reg_write(UC_X86_REG_EBP, byte_count)
                return
            if address == ALLOCATOR_ADDRESS:
                assert allocation_status is not None, f"{name}: unexpected allocation"
                calls.append(
                    {
                        "call": "resource_allocate",
                        "resource_id": cpu.reg_read(UC_X86_REG_AX),
                        "byte_count": cpu.reg_read(UC_X86_REG_EBP),
                        "filename": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_SI),
                        ],
                        "filename_backup_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "return_frame": return_frame(cpu),
                    }
                )
                cpu.reg_write(UC_X86_REG_AX, allocation_status & 0xFFFF)
                cpu.reg_write(UC_X86_REG_DS, destination_segment)
                cpu.reg_write(UC_X86_REG_SI, destination_offset)
                return
            if address == FILE_LOAD_ENTRY:
                assert file_result is not None, f"{name}: unexpected file load"
                calls.append(
                    {
                        "call": "resource_file_load",
                        "filename": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_SI),
                        ],
                        "destination": [
                            cpu.reg_read(UC_X86_REG_ES),
                            cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        ],
                        "return_frame": return_frame(cpu),
                    }
                )
                cpu.reg_write(UC_X86_REG_EAX, file_result)

        machine.hook_add(UC_HOOK_CODE, capture)
        machine.emu_start(ENTRY, 0, count=500)
        assert reached_return == [RETURN_ADDRESS], (name, reached_return)

        expected_calls: list[dict[str, Any]] = [
            {
                "call": "resource_name_lookup",
                "resource_id": resource_id,
                "filename": [NAME_SEGMENT, filename_offset],
                "filename_backup_offset": filename_offset,
                "return_frame": [0x2C16, 0],
            }
        ]
        if byte_count != 0:
            expected_calls.append(
                {
                    "call": "resource_allocate",
                    "resource_id": resource_id,
                    "byte_count": byte_count,
                    "filename": [NAME_SEGMENT, filename_offset],
                    "filename_backup_offset": filename_offset,
                    "return_frame": [0x2C20, 0],
                }
            )
        if allocation_status == 0:
            expected_calls.append(
                {
                    "call": "resource_file_load",
                    "filename": [NAME_SEGMENT, filename_offset],
                    "destination": [destination_segment, destination_offset],
                    "return_frame": [0x2C32, 0],
                }
            )
        assert calls == expected_calls, (name, calls, expected_calls)

        success = (
            byte_count != 0
            and allocation_status is not None
            and allocation_status >= 0
            and (allocation_status != 0 or file_result != 0)
        )
        expected_registers = {key: initial[key] for key in GENERAL_REGISTERS}
        expected_registers.update({key: initial[key] for key in SEGMENT_REGISTERS})
        expected_registers["eax"] = int(success)
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, {
            key: (actual_registers[key], value)
            for key, value in expected_registers.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4

        if not success:
            flag_operand, flag_width = 0, 32
        elif allocation_status != 0:
            flag_operand, flag_width = allocation_status, 16
        else:
            assert file_result is not None
            flag_operand, flag_width = file_result, 32
        expected_flags = defined_or_flags(flag_operand, flag_width, direction_flag)
        actual_flags = snapshot_defined_flags(machine)
        assert actual_flags == expected_flags, (name, actual_flags, expected_flags)

        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_before
        assert bytes(machine.mem_read(NAME_SEGMENT * 16, SEGMENT_SIZE)) == name_before
        assert (
            bytes(machine.mem_read(destination_segment * 16, SEGMENT_SIZE))
            == destination_before
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_stack_ownership(stack_before, stack_after)

        rows.append(
            {
                "name": name,
                "resource_id": resource_id,
                "filename_offset": filename_offset,
                "byte_count": byte_count,
                "allocation_status": allocation_status,
                "file_result": file_result,
                "success": success,
                "defined_flags": expected_flags,
                "calls": calls,
            }
        )

    return rows


def verify_commander_semantic_partition(rows: list[dict[str, Any]]) -> str:
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(rows) == len(expected_rows)
    comparable_fields = (
        "name",
        "resource_id",
        "filename_offset",
        "byte_count",
        "allocation_status",
        "file_result",
        "success",
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
        raise SystemExit(f"BBB resource-ID loader body changed: {actual}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_resource_load_by_id_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantic_partition(rows),
        "routine_sha256": BODY_SHA256,
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB resource-ID load cases")


if __name__ == "__main__":
    main()
