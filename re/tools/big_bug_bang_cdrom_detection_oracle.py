#!/usr/bin/env python3
"""Verify BBB's unchanged MSCDEX drive-detection wrapper."""

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
    UC_X86_REG_BX,
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


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_cdrom_detection.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_0b32_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "cbb8d1f2023a4d16bb05da6c561d018ed06c039181d62701d227b59539bb0c0c"
)
ENTRY = 0x0D2D
END = 0x0D3D
BODY_SHA256 = "7c83117930913384d02c89b2253d94577304ee43bfacce4765d1e45295bf12d6"

GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")
CDROM_PRESENT = 0x0CEF
DRIVE_COUNTS = (0x0000, 0x0001, 0x0002, 0x7FFF, 0xFFFF)

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


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


def detection_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, drive_count in enumerate(DRIVE_COUNTS):
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
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, data_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[name])
        for name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202 | (0x0400 if case_index & 1 else 0))

        interrupts: list[dict[str, int]] = []
        writes: list[tuple[int, int, int]] = []

        def instruction(_cpu: Uc, address: int, size: int, _context: Any) -> None:
            assert ENTRY <= address and address + size <= END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            interrupts.append(
                {
                    "number": number,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "bx": cpu.reg_read(UC_X86_REG_BX),
                }
            )
            cpu.reg_write(UC_X86_REG_AX, 0xADAD)
            cpu.reg_write(UC_X86_REG_BX, drive_count)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0A93 if case_index & 1 else 0x0246)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _context: Any,
        ) -> None:
            assert address == GAME_SEGMENT * 16 + CDROM_PRESENT
            assert size == 1
            writes.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=100)

        expected_present = int(drive_count != 0)
        assert interrupts == [{"number": 0x2F, "ax": 0x1500, "bx": 0}]
        assert writes == [(GAME_SEGMENT * 16 + CDROM_PRESENT, 1, expected_present)]
        game_expected = bytearray(game_before)
        game_expected[CDROM_PRESENT] = expected_present
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_before
        )
        assert bytes(machine.mem_read(0, len(executable))) == executable
        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | 0xADAD
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | drive_count
        assert snapshot_registers(machine) == expected_registers
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": flags & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": int((drive_count & 0xFF).bit_count() % 2 == 0),
            "zero": int(drive_count == 0),
            "sign": (drive_count >> 15) & 1,
            "overflow": 0,
        }
        assert actual_flags == expected_flags
        rows.append(
            {
                "drive_count": drive_count,
                "interrupt_ax": interrupts[0]["ax"],
                "interrupt_bx": interrupts[0]["bx"],
                "cdrom_present": expected_present,
                "result_ax": machine.reg_read(UC_X86_REG_AX),
                "result_bx": machine.reg_read(UC_X86_REG_BX),
                "flags": actual_flags,
            }
        )
    return rows


def build_fixture(executable: bytes) -> list[dict[str, Any]]:
    assert hashlib.sha256(executable[ENTRY:END]).hexdigest() == BODY_SHA256
    assert sha256(COMMANDER_FIXTURE) == COMMANDER_FIXTURE_SHA256
    rows = detection_cases(executable)
    assert rows == json.loads(COMMANDER_FIXTURE.read_text())
    return rows


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
    print(f"verified {len(fixture)} BBB MSCDEX detection cases")


if __name__ == "__main__":
    main()
