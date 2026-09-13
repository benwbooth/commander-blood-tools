#!/usr/bin/env python3
"""Verify BBB's unchanged startup resource-copy coordinator."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_copy.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_280f_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x2B8F
END = 0x2BFB
BODY_SHA256 = "e1648024c65949e63b6677eaf90340789a7e9db9047dd32e02b3d66b1b24233c"
LOOKUP_ENTRY = 0x2C4A
BUFFER_POINTER = 0x0C74
SHARED_HANDLE = 0x0C7C

DATA_SEGMENT = 0x2000
GAME_SEGMENT = 0x3200
BUFFER_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
SOURCE_PATH_OFFSET = 0x4100
DESTINATION_PATH_OFFSET = 0x4300
BUFFER_OFFSET = 0x0100
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
    {"name": "zero_size_skips_dos", "byte_count": 0},
    {"name": "source_open_failure", "byte_count": 7, "source_open": False},
    {
        "name": "destination_create_failure_leaks_source",
        "byte_count": 9,
        "destination_create": False,
    },
    {"name": "single_chunk_copy", "byte_count": 5, "read_counts": [5]},
    {
        "name": "fixed_request_multi_chunk",
        "byte_count": 0xFA03,
        "read_counts": [0xFA00, 3],
    },
    {
        "name": "full_32bit_remaining",
        "byte_count": 0x10005,
        "read_counts": [0xFA00, 0x0605],
    },
    {
        "name": "read_carry_ignored",
        "byte_count": 4,
        "read_counts": [4],
        "read_error": True,
    },
    {
        "name": "write_carry_ignored",
        "byte_count": 6,
        "read_counts": [6],
        "write_error": True,
        "source_close_error": True,
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
        source_open = bool(case.get("source_open", True))
        destination_create = bool(case.get("destination_create", True))
        read_counts = [int(value) for value in case.get("read_counts", [])]
        read_error = bool(case.get("read_error", False))
        write_error = bool(case.get("write_error", False))
        source_close_error = bool(case.get("source_close_error", False))
        source_handle = 0x3100 + case_index
        destination_handle = 0x4100 + case_index
        initial_shared_handle = 0xA500 + case_index
        source_path = f"SOURCE{case_index:02d}.DAT".encode("ascii") + b"\0"
        destination_path = f"DEST{case_index:02d}.DAT".encode("ascii") + b"\0"
        direction_flag = bool(case_index & 1)
        initial = {
            "eax": 0xA1A11230 + case_index,
            "ebx": 0xB2B22340 + case_index,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D44560 + case_index,
            "esi": 0xE5E50000 | SOURCE_PATH_OFFSET,
            "edi": 0xF6F60000 | DESTINATION_PATH_OFFSET,
            "ebp": 0x97977890 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "flags": 0x0292 | (0x0400 if direction_flag else 0),
        }
        calls: list[dict[str, Any]] = []
        read_index = 0
        close_index = 0

        data_before = seeded_region(SEGMENT_SIZE, case_index, 19, 0x31)
        game_before = seeded_region(SEGMENT_SIZE, case_index, 23, 0x43)
        buffer_before = seeded_region(SEGMENT_SIZE, case_index, 29, 0x59)
        buffer_expected = bytearray(buffer_before)
        extra_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 31, 0x67))
        fs_before = bytes(seeded_region(SEGMENT_SIZE, case_index, 37, 0x79))
        stack_before = seeded_region(SEGMENT_SIZE, case_index, 41, 0x8B)
        data_before[SOURCE_PATH_OFFSET : SOURCE_PATH_OFFSET + len(source_path)] = (
            source_path
        )
        data_before[
            DESTINATION_PATH_OFFSET : DESTINATION_PATH_OFFSET + len(destination_path)
        ] = destination_path
        struct.pack_into(
            "<HH", game_before, BUFFER_POINTER, BUFFER_OFFSET, BUFFER_SEGMENT
        )
        struct.pack_into("<H", game_before, SHARED_HANDLE, initial_shared_handle)
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(LOOKUP_ENTRY, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(BUFFER_SEGMENT * 16, bytes(buffer_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
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
            if address == LOOKUP_ENTRY:
                calls.append(
                    {
                        "call": "resource_name_lookup",
                        "path": [
                            cpu.reg_read(UC_X86_REG_DS),
                            cpu.reg_read(UC_X86_REG_SI),
                        ],
                        "eax": cpu.reg_read(UC_X86_REG_EAX),
                        "return_frame": return_frame(cpu),
                    }
                )
                cpu.reg_write(UC_X86_REG_EBP, byte_count)

        def interrupt(cpu: Uc, number: int, _data: object) -> None:
            nonlocal read_index, close_index
            assert number == 0x21, f"{name}: unexpected INT {number:#x}"
            function = cpu.reg_read(UC_X86_REG_AX)
            if function == 0x3D00:
                call = {
                    "call": "dos_open_read_only",
                    "path": [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_DX)],
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    "success": source_open,
                }
                calls.append(call)
                assert call["path"] == [DATA_SEGMENT, SOURCE_PATH_OFFSET], (name, call)
                assert call["shared_handle"] == initial_shared_handle, (name, call)
                cpu.reg_write(UC_X86_REG_AX, source_handle if source_open else 2)
                set_carry(cpu, not source_open)
                return
            if function == 0x3C00:
                call = {
                    "call": "dos_create_truncate",
                    "path": [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_DX)],
                    "attributes": cpu.reg_read(UC_X86_REG_CX),
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    "success": destination_create,
                }
                calls.append(call)
                assert call["path"] == [DATA_SEGMENT, DESTINATION_PATH_OFFSET], (
                    name,
                    call,
                )
                assert call["attributes"] == 0, (name, call)
                assert call["shared_handle"] == source_handle, (name, call)
                cpu.reg_write(
                    UC_X86_REG_AX,
                    destination_handle if destination_create else 5,
                )
                set_carry(cpu, not destination_create)
                return
            if function == 0x3F00:
                assert read_index < len(read_counts), f"{name}: unexpected extra read"
                returned = read_counts[read_index]
                failed = read_error and read_index == 0
                call = {
                    "call": "dos_read",
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "buffer": [
                        cpu.reg_read(UC_X86_REG_DS),
                        cpu.reg_read(UC_X86_REG_DX),
                    ],
                    "requested": cpu.reg_read(UC_X86_REG_CX),
                    "returned": returned,
                    "carry": failed,
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                }
                calls.append(call)
                assert call["handle"] == source_handle, (name, call)
                assert call["buffer"] == [BUFFER_SEGMENT, BUFFER_OFFSET], (name, call)
                assert call["requested"] == 0xFA00, (name, call)
                assert call["shared_handle"] == destination_handle, (name, call)
                payload = bytes(
                    ((index * 29 + read_index * 47 + case_index * 61) & 0xFF)
                    for index in range(returned)
                )
                cpu.mem_write(BUFFER_SEGMENT * 16 + BUFFER_OFFSET, payload)
                buffer_expected[BUFFER_OFFSET : BUFFER_OFFSET + returned] = payload
                cpu.reg_write(UC_X86_REG_AX, returned)
                set_carry(cpu, failed)
                read_index += 1
                return
            if function == 0x4000:
                failed = write_error and read_index == 1
                count = cpu.reg_read(UC_X86_REG_CX)
                buffer = [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_DX)]
                call = {
                    "call": "dos_write",
                    "handle": cpu.reg_read(UC_X86_REG_BX),
                    "buffer": buffer,
                    "count": count,
                    "carry": failed,
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                    "data_sha256": hashlib.sha256(
                        bytes(cpu.mem_read(buffer[0] * 16 + buffer[1], count))
                    ).hexdigest(),
                }
                calls.append(call)
                assert call["handle"] == destination_handle, (name, call)
                assert buffer == [BUFFER_SEGMENT, BUFFER_OFFSET], (name, call)
                assert call["shared_handle"] == source_handle, (name, call)
                assert count == read_counts[read_index - 1], (name, call)
                cpu.reg_write(UC_X86_REG_AX, 5 if failed else count)
                set_carry(cpu, failed)
                return
            if function == 0x3E00:
                handle = cpu.reg_read(UC_X86_REG_BX)
                expected_handle = (
                    destination_handle if close_index == 0 else source_handle
                )
                failed = source_close_error and close_index == 1
                call = {
                    "call": "dos_close",
                    "handle": handle,
                    "carry": failed,
                    "shared_handle": game_u16(cpu, SHARED_HANDLE),
                }
                calls.append(call)
                assert handle == expected_handle, (name, call)
                assert call["shared_handle"] == source_handle, (name, call)
                cpu.reg_write(UC_X86_REG_AX, 6 if failed else 0)
                set_carry(cpu, failed)
                close_index += 1
                return
            raise AssertionError(f"{name}: unexpected DOS function {function:#x}")

        machine.hook_add(UC_HOOK_CODE, capture)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(ENTRY, 0, count=2000)
        assert reached_return == [RETURN_ADDRESS], (name, reached_return)

        expected_names = ["resource_name_lookup"]
        if byte_count != 0:
            expected_names.append("dos_open_read_only")
            if source_open:
                expected_names.append("dos_create_truncate")
                if destination_create:
                    for _ in read_counts:
                        expected_names.extend(("dos_read", "dos_write"))
                    expected_names.extend(("dos_close", "dos_close"))
        assert [call["call"] for call in calls] == expected_names, (name, calls)
        assert calls[0] == {
            "call": "resource_name_lookup",
            "path": [DATA_SEGMENT, SOURCE_PATH_OFFSET],
            "eax": 0,
            "return_frame": [0x2B9E, 0],
        }, (name, calls[0])
        if destination_create and source_open and byte_count != 0:
            assert sum(read_counts) == byte_count
            assert read_index == len(read_counts)
            assert close_index == 2

        expected_shared_handle = (
            source_handle if byte_count != 0 and source_open else initial_shared_handle
        )
        game_expected = bytearray(game_before)
        struct.pack_into("<H", game_expected, SHARED_HANDLE, expected_shared_handle)
        assert game_u16(machine, SHARED_HANDLE) == expected_shared_handle
        expected_registers = {key: initial[key] for key in GENERAL_REGISTERS}
        expected_registers.update({key: initial[key] for key in SEGMENT_REGISTERS})
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, {
            key: (actual_registers[key], value)
            for key, value in expected_registers.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if byte_count == 0:
            expected_carry = False
        elif not source_open or not destination_create:
            expected_carry = True
        else:
            expected_carry = source_close_error
        assert bool(flags & 1) == expected_carry, name
        assert bool(flags & 0x0400) == direction_flag, name

        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_expected
        assert (
            bytes(machine.mem_read(BUFFER_SEGMENT * 16, SEGMENT_SIZE))
            == buffer_expected
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_stack_ownership(stack_before, stack_after)

        rows.append(
            {
                "name": name,
                "byte_count": byte_count,
                "read_counts": read_counts,
                "fixed_read_request": 0xFA00 if read_counts else None,
                "read_carry_ignored": read_error,
                "write_carry_ignored": write_error,
                "source_close_carry": source_close_error,
                "final_shared_handle": expected_shared_handle,
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
        "read_counts",
        "fixed_read_request",
        "read_carry_ignored",
        "write_carry_ignored",
        "source_close_carry",
        "final_shared_handle",
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
        raise SystemExit(f"BBB startup resource-copy body changed: {actual}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_resource_copy_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantic_partition(rows),
        "routine_sha256": BODY_SHA256,
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB startup resource-copy cases")


if __name__ == "__main__":
    main()
