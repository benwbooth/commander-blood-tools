#!/usr/bin/env python3
"""Verify BBB platform routines replaced by SDL and wgpu host behavior."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
from typing import Any, Callable

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
    UC_X86_INS_IN,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_platform_adapters.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

RETRACE_ENTRY = 0x0DD2
RETRACE_END = 0x0DFA
CTRL_BREAK_ENTRY = 0x0DFA
CTRL_BREAK_END = 0x0E14
VIDEO_RESTORE_ENTRY = 0x0EBB
VIDEO_RESTORE_END = 0x0EC6
ROUTINES = (
    (
        RETRACE_ENTRY,
        RETRACE_END,
        "video_retrace_phase_wait",
        "1b0d517e390bb82f1c3902451d5aed13bd4af1f6891b6ad09f082b12028c19fe",
    ),
    (
        CTRL_BREAK_ENTRY,
        CTRL_BREAK_END,
        "install_ctrl_break_handlers",
        "7530ef488085eac191e1acd4efc2df0b3c4ce5523748aa13c7cf0d80ea6ad609",
    ),
    (
        VIDEO_RESTORE_ENTRY,
        VIDEO_RESTORE_END,
        "restore_video_mode",
        "7b6640667ec3334e11c582870972fead6cf2660cef27fe8a9161ac72226a1c5d",
    ),
)

GLOBALS_SEGMENT = 0x3000
DECOY_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")
CRTC_BASE = 0x0C96
RETRACE_PHASE = 0x0D1C
SAVED_VIDEO_MODE = 0x5602

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
LOGIC_FLAG_MASKS = {"cf": 0x001, "pf": 0x004, "zf": 0x040, "sf": 0x080, "of": 0x800}
ALL_FLAG_MASKS = {
    "cf": 0x001,
    "pf": 0x004,
    "af": 0x010,
    "zf": 0x040,
    "sf": 0x080,
    "if": 0x200,
    "df": 0x400,
    "of": 0x800,
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
        name: machine.reg_read(register) for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def observed_flags(machine: Uc, masks: dict[str, int]) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(value & mask) for name, mask in masks.items()}


def flags_from(value: int, masks: dict[str, int]) -> dict[str, bool]:
    return {name: bool(value & mask) for name, mask in masks.items()}


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
    end: int,
    case_index: int,
    globals_before: bytes | bytearray,
    stack_depth: int,
    interrupt: Callable[[Uc, int, Any], None] | None = None,
    input_port: Callable[[Uc, int, int, Any], int] | None = None,
) -> tuple[Uc, list[tuple[int, int]]]:
    initial = initial_registers(case_index)
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
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

    writes: list[tuple[int, int]] = []
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
        assert set(range(address, address + size)) <= allowed_stack_addresses, hex(
            address
        )
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    if interrupt is not None:
        machine.hook_add(UC_HOOK_INTR, interrupt)
    if input_port is not None:
        machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.emu_start(entry, RETURN_OFFSET, count=1_000)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
    assert bytes(machine.mem_read(0, len(executable))) == executable
    assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
    assert snapshot_registers(machine) == initial
    stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
    assert_unchanged_outside(stack_before, stack_after, allowed_stack_offsets)
    assert (
        stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
        == stack_before[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
    )
    return machine, writes


def patterned_globals(case_index: int) -> bytearray:
    return bytearray(
        (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
    )


def retrace_wait_cases(executable: bytes) -> list[dict[str, Any]]:
    cases = (
        ("disabled", 0x00, 0x03D4, ()),
        ("phase_one_direct", 0x01, 0x03D4, (0x00,)),
        ("phase_one_wait", 0x01, 0x03B4, (0xFF, 0x18, 0x07)),
        ("phase_two_wait", 0x02, 0x03D4, (0x00, 0x07, 0x08)),
        ("phase_ff_wrapped_port", 0xFF, 0xFFFC, (0xF7, 0xFF)),
    )
    rows = []
    for case_index, (name, phase, crtc_base, input_values) in enumerate(cases):
        globals_before = patterned_globals(case_index)
        struct.pack_into("<H", globals_before, CRTC_BASE, crtc_base)
        globals_before[RETRACE_PHASE] = phase
        reads: list[list[int]] = []
        values = iter(input_values)

        def input_port(_cpu: Uc, port: int, size: int, _context: Any) -> int:
            try:
                value = next(values)
            except StopIteration as error:
                raise AssertionError(
                    f"{name}: unexpected extra retrace read"
                ) from error
            reads.append([port, size, value])
            return value

        machine, writes = execute(
            executable,
            RETRACE_ENTRY,
            RETRACE_END,
            case_index,
            globals_before,
            4,
            input_port=input_port if input_values else None,
        )
        assert len(reads) == len(input_values)
        status_port = (crtc_base + 6) & 0xFFFF
        assert all(read[:2] == [status_port, 1] for read in reads)
        after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        assert after == bytes(globals_before)
        assert len(writes) == 2
        expected_flags = {
            "cf": False,
            "pf": phase == 0,
            "zf": phase == 0,
            "sf": False,
            "of": False,
        }
        assert observed_flags(machine, LOGIC_FLAG_MASKS) == expected_flags
        rows.append(
            {
                "name": name,
                "phase": phase,
                "crtc_base": crtc_base,
                "status_port": status_port,
                "input_values": list(input_values),
                "reads": reads,
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def ctrl_break_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, dos_flags in enumerate((0x0202, 0x0AD7, 0x0646, 0x0A12)):
        globals_before = patterned_globals(case_index)
        interrupts: list[dict[str, int]] = []

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            interrupts.append(
                {
                    "number": number,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "dx": cpu.reg_read(UC_X86_REG_DX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                }
            )
            if len(interrupts) == 1:
                cpu.reg_write(UC_X86_REG_AX, 0x2523)
            else:
                cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
                cpu.reg_write(UC_X86_REG_EFLAGS, dos_flags)

        machine, writes = execute(
            executable,
            CTRL_BREAK_ENTRY,
            CTRL_BREAK_END,
            case_index,
            globals_before,
            6,
            interrupt=interrupt,
        )
        expected_interrupts = [
            {"number": 0x21, "ax": 0x2523, "dx": 0x0614, "ds": 0},
            {"number": 0x21, "ax": 0x2524, "dx": 0x0615, "ds": 0},
        ]
        assert interrupts == expected_interrupts
        assert bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            globals_before
        )
        assert len(writes) == 3
        expected_flags = flags_from(dos_flags, ALL_FLAG_MASKS)
        assert observed_flags(machine, ALL_FLAG_MASKS) == expected_flags
        rows.append(
            {
                "case": case_index,
                "vectors": [[0x23, 0x0614, 0], [0x24, 0x0615, 0]],
                "interrupts": interrupts,
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def video_restore_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, mode in enumerate((0x00, 0x03, 0x13, 0x7F, 0xFF)):
        globals_before = patterned_globals(case_index)
        globals_before[SAVED_VIDEO_MODE] = mode
        interrupts: list[dict[str, int]] = []
        dos_flags = (0x0202, 0x0AD7, 0x0646, 0x0A12, 0x0246)[case_index]

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            interrupts.append({"number": number, "ax": cpu.reg_read(UC_X86_REG_AX)})
            cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
            cpu.reg_write(UC_X86_REG_EFLAGS, dos_flags)

        machine, writes = execute(
            executable,
            VIDEO_RESTORE_ENTRY,
            VIDEO_RESTORE_END,
            case_index,
            globals_before,
            2,
            interrupt=interrupt,
        )
        assert interrupts == [{"number": 0x10, "ax": mode}]
        assert bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            globals_before
        )
        assert len(writes) == 1
        expected_flags = flags_from(dos_flags, ALL_FLAG_MASKS)
        assert observed_flags(machine, ALL_FLAG_MASKS) == expected_flags
        rows.append(
            {
                "saved_mode": mode,
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
        "format": "big_bug_bang_platform_adapters_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routines,
        "retrace_wait_cases": retrace_wait_cases(executable),
        "ctrl_break_cases": ctrl_break_cases(executable),
        "video_restore_cases": video_restore_cases(executable),
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
        f"verified {len(fixture['retrace_wait_cases'])} BBB retrace-wait, "
        f"{len(fixture['ctrl_break_cases'])} Ctrl-Break, and "
        f"{len(fixture['video_restore_cases'])} video-restore cases"
    )


if __name__ == "__main__":
    main()
