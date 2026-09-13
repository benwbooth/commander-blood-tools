#!/usr/bin/env python3
"""Verify BBB's unchanged DOS timer installation and restoration routines."""

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
    UC_HOOK_INSN,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_INS_OUT,
    UC_X86_REG_AX,
    UC_X86_REG_BX,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_timer_lifecycle.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

START_ENTRY = 0x09A2
START_END = 0x09F0
STOP_ENTRY = 0x09F0
STOP_END = 0x0A19
ROUTINES = (
    (
        START_ENTRY,
        START_END,
        "install_timer_isr_hook",
        "4331a04f52347cc3d9d9d6f9203298f38511b17e23363626e8c7dc756bc1c625",
    ),
    (
        STOP_ENTRY,
        STOP_END,
        "restore_timer_isr_hook",
        "7b12f83fc86f97defa9d046ea76ff352161961f3607fd5cd9834028ab30590b6",
    ),
)

GLOBALS_SEGMENT = 0x3000
DECOY_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

PREVIOUS_VECTOR_OFFSET = 0x0D27
PREVIOUS_VECTOR_SEGMENT = 0x0D29
TIMER_ACTIVE = 0x0D2B
TIMER_DIVIDER = 0x0D2C
TIMER_RELOAD = 0x0D2F
SUBTICK_LIMIT = 0x0D31
LOADED_ISR_OFFSET = 0x0219

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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "if": 0x0200,
    "df": 0x0400,
    "of": 0x0800,
}
CASES = (
    ("ordinary_vector", 0x1234, 0x5678, 0x0202, 0x0AD7),
    ("zero_vector", 0x0000, 0x0000, 0x0602, 0x0246),
    ("maximum_vector", 0xFFFF, 0xFFFF, 0x0A93, 0x0A12),
    ("mixed_vector", 0x8000, 0x7FFF, 0x0203, 0x0643),
)


def initial_registers(case_index: int, flags: int) -> dict[str, int]:
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
        "flags": flags,
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


def run_case(
    executable: bytes,
    entry: int,
    end: int,
    case_index: int,
    previous_offset: int,
    previous_segment: int,
    initial_flags: int,
    dos_flags: int,
) -> tuple[
    Uc, bytearray, list[dict[str, int | bool]], list[list[int]], list[tuple[int, int]]
]:
    initial = initial_registers(case_index, initial_flags)
    globals_before = bytearray(
        (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
    )
    if entry == STOP_ENTRY:
        struct.pack_into(
            "<HHB",
            globals_before,
            PREVIOUS_VECTOR_OFFSET,
            previous_offset,
            previous_segment,
            1,
        )
    decoy_before = bytes(
        (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
    )
    stack_before = bytearray(
        (index * 13 + case_index * 17 + 0x3C) & 0xFF for index in range(SEGMENT_SIZE)
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
    machine.reg_write(UC_X86_REG_EFLAGS, initial_flags)

    interrupts: list[dict[str, int | bool]] = []
    outputs: list[list[int]] = []
    writes: list[tuple[int, int]] = []
    allowed_global_offsets = (
        {
            PREVIOUS_VECTOR_OFFSET,
            PREVIOUS_VECTOR_OFFSET + 1,
            PREVIOUS_VECTOR_SEGMENT,
            PREVIOUS_VECTOR_SEGMENT + 1,
            TIMER_ACTIVE,
            TIMER_DIVIDER,
            TIMER_RELOAD,
            TIMER_RELOAD + 1,
            SUBTICK_LIMIT,
            SUBTICK_LIMIT + 1,
        }
        if entry == START_ENTRY
        else {TIMER_ACTIVE}
    )
    allowed_global_addresses = {
        GLOBALS_SEGMENT * 16 + offset for offset in allowed_global_offsets
    }
    stack_depth = 10 if entry == START_ENTRY else 6
    allowed_stack_offsets = set(range(CALLER_SP - stack_depth, CALLER_SP))
    allowed_stack_addresses = {
        STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
    }

    def instruction(_machine: Uc, address: int, size: int, _context: Any) -> None:
        assert entry <= address and address + size <= end, hex(address)

    def write_hook(
        _machine: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: Any,
    ) -> None:
        touched = set(range(address, address + size))
        assert (
            touched <= allowed_global_addresses or touched <= allowed_stack_addresses
        ), hex(address)
        writes.append((address, size))

    def interrupt(cpu: Uc, number: int, _context: Any) -> None:
        interrupts.append(
            {
                "number": number,
                "ax": cpu.reg_read(UC_X86_REG_AX),
                "bx": cpu.reg_read(UC_X86_REG_BX),
                "dx": cpu.reg_read(UC_X86_REG_DX),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "if": bool(cpu.reg_read(UC_X86_REG_EFLAGS) & 0x0200),
            }
        )
        if entry == START_ENTRY and len(interrupts) == 1:
            cpu.reg_write(UC_X86_REG_AX, 0xA508)
            cpu.reg_write(UC_X86_REG_BX, previous_offset)
            cpu.reg_write(UC_X86_REG_ES, previous_segment)
            cpu.reg_write(UC_X86_REG_EFLAGS, initial_flags ^ 0x0855)
        else:
            cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
            cpu.reg_write(UC_X86_REG_DX, 0xBEEF)
            cpu.reg_write(UC_X86_REG_EFLAGS, dos_flags)

    def output_port(
        _machine: Uc, port: int, size: int, value: int, _context: Any
    ) -> None:
        outputs.append([port, size, value])

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    machine.emu_start(entry, RETURN_OFFSET, count=1_000)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
    assert bytes(machine.mem_read(0, len(executable))) == executable
    assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
    assert snapshot_registers(machine) == {
        name: value for name, value in initial.items() if name != "flags"
    }
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert_unchanged_outside(stack_before, stack_after, allowed_stack_offsets)
    assert (
        stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
        == stack_before[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
    )
    return machine, globals_before, interrupts, outputs, writes


def start_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, offset, segment, initial_flags, dos_flags) in enumerate(
        CASES
    ):
        machine, before, interrupts, outputs, writes = run_case(
            executable,
            START_ENTRY,
            START_END,
            case_index,
            offset,
            segment,
            initial_flags,
            dos_flags,
        )
        expected_interrupts = [
            {
                "number": 0x21,
                "ax": 0x3508,
                "bx": (0x2468 + case_index) & 0xFFFF,
                "dx": (0x55AA + case_index) & 0xFFFF,
                "ds": DECOY_SEGMENT,
                "es": 0x4800,
                "if": bool(initial_flags & 0x0200),
            },
            {
                "number": 0x21,
                "ax": 0x2508,
                "bx": 0,
                "dx": LOADED_ISR_OFFSET,
                "ds": 0,
                "es": segment,
                "if": bool((initial_flags ^ 0x0855) & 0x0200),
            },
        ]
        assert interrupts == expected_interrupts, (
            name,
            interrupts,
            expected_interrupts,
        )
        expected_outputs = [[0x43, 1, 0x36], [0x40, 1, 0x46], [0x40, 1, 0x17]]
        assert outputs == expected_outputs
        after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        expected = bytearray(before)
        struct.pack_into("<HH", expected, PREVIOUS_VECTOR_OFFSET, offset, segment)
        expected[TIMER_ACTIVE] = 1
        expected[TIMER_DIVIDER] = 0x0B
        struct.pack_into("<H", expected, TIMER_RELOAD, 3)
        struct.pack_into("<H", expected, SUBTICK_LIMIT, 25)
        assert after == bytes(expected)
        assert len(writes) == 11, (name, writes)
        expected_flags = flags_from(dos_flags | 0x0200)
        assert observed_flags(machine) == expected_flags
        rows.append(
            {
                "name": name,
                "previous_vector": [offset, segment],
                "installed_vector": [LOADED_ISR_OFFSET, 0],
                "interrupts": interrupts,
                "pit_outputs": outputs,
                "timer_state": {
                    "active": 1,
                    "divider": 0x0B,
                    "reload_ticks": 3,
                    "subtick_limit": 25,
                },
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def stop_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, offset, segment, initial_flags, dos_flags) in enumerate(
        CASES
    ):
        machine, before, interrupts, outputs, writes = run_case(
            executable,
            STOP_ENTRY,
            STOP_END,
            case_index,
            offset,
            segment,
            initial_flags,
            dos_flags,
        )
        expected_interrupts = [
            {
                "number": 0x21,
                "ax": 0x2508,
                "bx": (0x2468 + case_index) & 0xFFFF,
                "dx": offset,
                "ds": segment,
                "es": 0x4800,
                "if": True,
            }
        ]
        assert interrupts == expected_interrupts, (
            name,
            interrupts,
            expected_interrupts,
        )
        expected_outputs = [[0x43, 1, 0x36], [0x40, 1, 0xFF], [0x40, 1, 0xFF]]
        assert outputs == expected_outputs
        after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        expected = bytearray(before)
        expected[TIMER_ACTIVE] = 0
        assert after == bytes(expected)
        assert len(writes) == 4, (name, writes)
        expected_flags = flags_from(dos_flags)
        assert observed_flags(machine) == expected_flags
        rows.append(
            {
                "name": name,
                "restored_vector": [offset, segment],
                "interrupts": interrupts,
                "pit_outputs": outputs,
                "timer_active": 0,
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
        "format": "big_bug_bang_timer_lifecycle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routines,
        "start_cases": start_cases(executable),
        "stop_cases": stop_cases(executable),
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
        f"verified {len(fixture['start_cases'])} BBB timer-start and "
        f"{len(fixture['stop_cases'])} timer-stop cases"
    )


if __name__ == "__main__":
    main()
