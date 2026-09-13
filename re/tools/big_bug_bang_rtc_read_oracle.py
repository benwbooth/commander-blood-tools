#!/usr/bin/env python3
"""Verify BBB's unchanged RTC hour and date readers."""

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
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_rtc_read.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

HOUR_ENTRY = 0x0B36
HOUR_END = 0x0B4B
DATE_ENTRY = 0x0B4B
DATE_END = 0x0B81
BCD_ENTRY = 0x0B81
BCD_END = 0x0B92
ROUTINES = (
    (
        HOUR_ENTRY,
        HOUR_END,
        "rtc_time_read",
        "3b59a379da6626c99908dd4835a4fa0858ecbceb979edaf68fbbea0a33837bb5",
    ),
    (
        DATE_ENTRY,
        DATE_END,
        "rtc_date_read",
        "97de9590cd952939bc1fd105a89726d506cee4045c70ef057c6c7f4ad16b9e11",
    ),
    (
        BCD_ENTRY,
        BCD_END,
        "packed_bcd_to_binary",
        "65bd730970b38f982176699cde398c15640791e77c3fb13d0764ec6a3e7db9d5",
    ),
)

GLOBALS_SEGMENT = 0x3000
DECOY_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
CURRENT_HOUR = 0x0C9E
CURRENT_DAY = 0x0CA0
CURRENT_MONTH = 0x0CA2
CURRENT_YEAR = 0x0CA4
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

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

HOURS = (0x00, 0x09, 0x12, 0x23, 0x59, 0x99, 0xFF)
DATE_CASES = (
    ("century_13_path", 0x13, 0x94, 0x12, 0x31),
    ("century_20_path", 0x20, 0x24, 0x08, 0x07),
    ("bcd_19_uses_else_path", 0x19, 0x99, 0x11, 0x30),
    ("all_zero", 0x00, 0x00, 0x00, 0x00),
    ("signed_invalid_bcd", 0xFF, 0xFF, 0xFE, 0xFD),
    ("high_valid_digits", 0x13, 0x79, 0x59, 0x59),
)


def low_word(original: int, value: int) -> int:
    return (original & 0xFFFF0000) | (value & 0xFFFF)


def packed_bcd_to_signed(value: int) -> int:
    decoded = (((value >> 4) * 10) + (value & 0x0F)) & 0xFF
    return decoded if decoded < 0x80 else decoded - 0x100


def arithmetic_flags(left: int, right: int, bits: int) -> dict[str, bool]:
    mask = (1 << bits) - 1
    sign = 1 << (bits - 1)
    result = (left + right) & mask
    return {
        "cf": left + right > mask,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & sign),
        "of": bool((~(left ^ right) & (left ^ result)) & sign),
    }


def observed_flags(machine: Uc) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(value & 0x0001),
        "pf": bool(value & 0x0004),
        "af": bool(value & 0x0010),
        "zf": bool(value & 0x0040),
        "sf": bool(value & 0x0080),
        "of": bool(value & 0x0800),
    }


def initial_registers(case_index: int) -> dict[str, int]:
    return {
        "eax": 0xA5A51234 + case_index,
        "ebx": 0xB6B62468 + case_index,
        "ecx": 0xC7C7369C + case_index,
        "edx": 0xD8D855AA + case_index,
        "esi": 0xE9E96789 + case_index,
        "edi": 0xFAFA789A + case_index,
        "ebp": 0xABCD1357 + case_index,
        "ds": DECOY_SEGMENT,
        "es": 0x4800,
        "fs": 0x5000,
        "gs": GLOBALS_SEGMENT,
        "ss": STACK_SEGMENT,
    }


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register)
        for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def execute(
    executable: bytes,
    entry: int,
    case_index: int,
    interrupt_values: tuple[int, int],
    expected_globals: tuple[int, ...],
) -> tuple[Uc, bytearray, list[dict[str, int]], list[tuple[int, int]], dict[str, int]]:
    initial = initial_registers(case_index)
    globals_before = bytearray(
        (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
    )
    decoy_before = bytes(
        (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
    )
    stack_before = bytearray(
        (index * 13 + case_index * 17 + 0x3C) & 0xFF
        for index in range(SEGMENT_SIZE)
    )
    stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack("<HH", RETURN_OFFSET, 0)
    stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = STACK_SENTINEL

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
    machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
    for name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, initial[name])
    for name, register in SEGMENT_REGISTERS.items():
        machine.reg_write(register, initial[name])
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

    interrupts: list[dict[str, int]] = []
    writes: list[tuple[int, int]] = []
    allowed_global_addresses = {
        GLOBALS_SEGMENT * 16 + offset + byte
        for offset in expected_globals
        for byte in range(2)
    }
    allowed_stack_addresses = {
        STACK_SEGMENT * 16 + offset
        for offset in range(CALLER_SP - 10, CALLER_SP)
    }

    def instruction(_machine: Uc, address: int, size: int, _context: Any) -> None:
        spans = ((entry, DATE_END if entry == DATE_ENTRY else HOUR_END), (BCD_ENTRY, BCD_END))
        assert any(start <= address and address + size <= end for start, end in spans), hex(
            address
        )

    def write_hook(
        _machine: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: Any,
    ) -> None:
        touched = set(range(address, address + size))
        assert touched <= allowed_global_addresses or touched <= allowed_stack_addresses, hex(
            address
        )
        writes.append((address, size))

    def interrupt(cpu: Uc, number: int, _context: Any) -> None:
        interrupts.append({"number": number, "ax": cpu.reg_read(UC_X86_REG_AX)})
        cpu.reg_write(UC_X86_REG_CX, interrupt_values[0])
        cpu.reg_write(UC_X86_REG_DX, interrupt_values[1])
        cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
        cpu.reg_write(UC_X86_REG_EFLAGS, 0x0643)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(entry, RETURN_OFFSET, count=1_000)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
    assert bytes(machine.mem_read(0, len(executable))) == executable
    assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
    assert snapshot_registers(machine) == initial
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert_unchanged_outside(
        stack_before, stack_after, set(range(CALLER_SP - 10, CALLER_SP))
    )
    assert stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)] == stack_before[
        CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)
    ]
    return machine, globals_before, interrupts, writes, initial


def hour_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, packed_hour in enumerate(HOURS):
        initial = initial_registers(case_index)
        interrupt_ax = 0x0200 | (initial["eax"] & 0xFF)
        machine, before, interrupts, writes, _ = execute(
            executable,
            HOUR_ENTRY,
            case_index,
            ((packed_hour << 8) | 0x5A, 0xBEEF),
            (CURRENT_HOUR,),
        )
        assert interrupts == [{"number": 0x1A, "ax": interrupt_ax}]
        decoded = packed_bcd_to_signed(packed_hour)
        after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        assert struct.unpack_from("<h", after, CURRENT_HOUR)[0] == decoded
        assert_unchanged_outside(before, after, {CURRENT_HOUR, CURRENT_HOUR + 1})
        expected_flags = arithmetic_flags((packed_hour >> 4) * 10, packed_hour & 0x0F, 8)
        assert observed_flags(machine) == expected_flags
        assert (GLOBALS_SEGMENT * 16 + CURRENT_HOUR, 2) in writes
        rows.append(
            {
                "name": f"packed_{packed_hour:02x}",
                "packed_hour": packed_hour,
                "stored_hour": decoded,
                "interrupt": interrupts[0],
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def date_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, century, year, month, day) in enumerate(DATE_CASES):
        initial = initial_registers(case_index)
        interrupt_ax = 0x0400 | (initial["eax"] & 0xFF)
        machine, before, interrupts, writes, _ = execute(
            executable,
            DATE_ENTRY,
            case_index,
            (((century & 0xFF) << 8) | year, ((month & 0xFF) << 8) | day),
            (CURRENT_DAY, CURRENT_MONTH, CURRENT_YEAR),
        )
        assert interrupts == [{"number": 0x1A, "ax": interrupt_ax}]
        decoded_day = packed_bcd_to_signed(day)
        decoded_month = packed_bcd_to_signed(month)
        decoded_year = packed_bcd_to_signed(year)
        base_year = 1900 if century == 0x13 else 2000
        stored_year = (decoded_year + base_year) & 0xFFFF
        after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        assert struct.unpack_from("<hhh", after, CURRENT_DAY) == (
            decoded_day,
            decoded_month,
            stored_year if stored_year < 0x8000 else stored_year - 0x10000,
        )
        allowed = set(range(CURRENT_DAY, CURRENT_YEAR + 2))
        assert_unchanged_outside(before, after, allowed)
        expected_flags = arithmetic_flags(decoded_year & 0xFFFF, base_year, 16)
        assert observed_flags(machine) == expected_flags
        for offset in (CURRENT_DAY, CURRENT_MONTH, CURRENT_YEAR):
            assert (GLOBALS_SEGMENT * 16 + offset, 2) in writes
        rows.append(
            {
                "name": name,
                "packed": {
                    "century": century,
                    "year": year,
                    "month": month,
                    "day": day,
                },
                "stored": {
                    "year": stored_year if stored_year < 0x8000 else stored_year - 0x10000,
                    "month": decoded_month,
                    "day": decoded_day,
                },
                "interrupt": interrupts[0],
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    routines = []
    for entry, end, operation, expected_hash in ROUTINES:
        digest = hashlib.sha256(executable[entry:end]).hexdigest()
        assert digest == expected_hash, hex(entry)
        routines.append(
            {
                "operation": operation,
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "body_sha256": digest,
            }
        )
    return {
        "format": "big_bug_bang_rtc_read_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routines,
        "hour_cases": hour_cases(executable),
        "date_cases": date_cases(executable),
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
    print(
        f"verified {len(fixture['hour_cases'])} BBB RTC hour and "
        f"{len(fixture['date_cases'])} date cases"
    )


if __name__ == "__main__":
    main()
