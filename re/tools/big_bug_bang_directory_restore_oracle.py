#!/usr/bin/env python3
"""Verify BBB's unchanged DOS startup-directory restoration routine."""

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

from unicorn import (  # noqa: E402
    Uc,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_AX,
    UC_X86_REG_CS,
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
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_directory_restore.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY = 0x2B69
END = 0x2B8F
BODY_SHA256 = "af73ab15c09b468c84b0e810aa37c20b873ff1055215e8e22d70b1eeeb2adadf"

GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

ORIGINAL_DRIVE = 0x0205
ORIGINAL_DIRECTORY = 0x0226
WRITE_DIRECTORY_ACTIVE = 0x0CE9
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "zf": 0x0040,
    "sf": 0x0080,
    "if": 0x0200,
    "df": 0x0400,
    "of": 0x0800,
}
CASES = (
    ("flag_0", 0, 2, "C:\\CBLOOD", 0x0246),
    ("flag_1_empty_root", 1, 0, "", 0x0893),
    ("flag_2", 2, 7, "D:\\IGNORED", 0x0643),
    ("flag_3_named_root", 3, 25, "Z:\\BIGBUG", 0x0013),
)

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


def observed_flags(machine: Uc) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(value & mask) for name, mask in FLAG_MASKS.items()}


def flags_from(value: int) -> dict[str, bool]:
    return {name: bool(value & mask) for name, mask in FLAG_MASKS.items()}


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def directory_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, initial_flag, drive, path, dos_flags) in enumerate(CASES):
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0xABCD1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": 0x6800,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        game_before = bytearray(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        path_bytes = path.encode("ascii") + b"\0"
        game_before[ORIGINAL_DRIVE] = drive
        game_before[ORIGINAL_DIRECTORY : ORIGINAL_DIRECTORY + len(path_bytes)] = (
            path_bytes
        )
        game_before[WRITE_DIRECTORY_ACTIVE] = initial_flag
        data_before = bytes(
            (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
        )
        extra_before = bytes(
            (index * 29 + case_index * 13 + 0x3C) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        stack_before = bytearray(
            (index * 13 + case_index * 17 + 0x3C) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack("<HH", RETURN_OFFSET, 0)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, data_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        calls: list[dict[str, int | str]] = []
        writes: list[tuple[int, int, int]] = []
        allowed_stack_offsets = set(range(CALLER_SP - 6, CALLER_SP))
        allowed_stack_addresses = {
            STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
        }
        flag_address = GAME_SEGMENT * 16 + WRITE_DIRECTORY_ACTIVE

        def instruction(_cpu: Uc, address: int, size: int, _context: Any) -> None:
            assert ENTRY <= address and address + size <= END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            assert number == 0x21, hex(number)
            function = cpu.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x0E:
                calls.append(
                    {
                        "function": "select_drive",
                        "drive": cpu.reg_read(UC_X86_REG_DX) & 0xFF,
                        "dx": cpu.reg_read(UC_X86_REG_DX),
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                    }
                )
                cpu.reg_write(UC_X86_REG_AX, 0x0E1A)
                cpu.reg_write(UC_X86_REG_DX, 0xDEAD)
                cpu.reg_write(UC_X86_REG_EFLAGS, dos_flags ^ 0x0855)
                return
            assert function == 0x3B, hex(function)
            offset = cpu.reg_read(UC_X86_REG_DX)
            segment = cpu.reg_read(UC_X86_REG_DS)
            directory = bytearray()
            for index in range(64):
                value = cpu.mem_read(segment * 16 + offset + index, 1)[0]
                if value == 0:
                    break
                directory.append(value)
            else:
                raise AssertionError(f"{name}: unterminated directory")
            calls.append(
                {
                    "function": "change_directory",
                    "offset": offset,
                    "path": directory.decode("ascii"),
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "ds": segment,
                }
            )
            cpu.reg_write(UC_X86_REG_AX, 0xBEEF)
            cpu.reg_write(UC_X86_REG_DX, 0xCAFE)
            cpu.reg_write(UC_X86_REG_EFLAGS, dos_flags)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            assert touched <= allowed_stack_addresses or (
                address == flag_address and size == 1
            ), hex(address)
            writes.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=500)

        active = initial_flag & 1 != 0
        expected_calls = (
            [
                {
                    "function": "select_drive",
                    "drive": drive,
                    "dx": (initial["edx"] & 0xFF00) | drive,
                    "ds": GAME_SEGMENT,
                },
                {
                    "function": "change_directory",
                    "offset": ORIGINAL_DIRECTORY,
                    "path": path,
                    "ax": 0x3B1A,
                    "ds": GAME_SEGMENT,
                },
            ]
            if active
            else []
        )
        assert calls == expected_calls, (name, calls, expected_calls)
        game_expected = bytearray(game_before)
        if active:
            game_expected[WRITE_DIRECTORY_ACTIVE] = 0
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(0, len(executable))) == executable
        assert snapshot_registers(machine) == initial
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_unchanged_outside(stack_before, stack_after, allowed_stack_offsets)
        assert (
            stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
            == stack_before[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
        )
        assert len(writes) == 3 + int(active), (name, writes)
        expected_flags = (
            flags_from(dos_flags)
            if active
            else {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "if": True,
                "df": False,
                "of": False,
            }
        )
        assert observed_flags(machine) == expected_flags
        rows.append(
            {
                "name": name,
                "initial_flag": initial_flag,
                "original_drive": drive,
                "original_directory": path,
                "calls": calls,
                "final_flag": game_expected[WRITE_DIRECTORY_ACTIVE],
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    assert digest == BODY_SHA256
    return {
        "format": "big_bug_bang_directory_restore_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routine": {
            "operation": "startup_original_directory_restore",
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": digest,
        },
        "cases": directory_cases(executable),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    fixture = build_fixture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixture, indent=2) + "\n")
    print(f"verified {len(fixture['cases'])} BBB directory-restore cases")


if __name__ == "__main__":
    main()
