#!/usr/bin/env python3
"""Verify BBB's unchanged VGA retrace-phase calibration routine."""

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
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_retrace_calibration.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY = 0x0D3D
END = 0x0DD2
BODY_SHA256 = "2229616542b7a55b6c97e8458ee54b9012d9df27179f47e6cd5442b80ade5ff3"

GLOBALS_SEGMENT = 0x3000
DECOY_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")
CRTC_BASE = 0x0C96
RETRACE_PHASE = 0x0D1C
TIMER_RELOAD = 0x0D2F
CALIBRATION_TICKS = 0x0D3F

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
CASES: tuple[dict[str, Any], ...] = (
    {
        "name": "timeout_before_edge",
        "crtc": 0x03D4,
        "status": (0, 0),
        "timeout": True,
        "timer": (),
        "phase_delta": 0,
    },
    {
        "name": "short_second_clear_phase",
        "crtc": 0x03D4,
        "status": (0, 8, 0, 0, 8, 8, 8, 0),
        "timeout": False,
        "timer": (0x9C, 0xFF, 0x6A, 0xFF),
        "phase_delta": 2,
    },
    {
        "name": "short_second_set_phase",
        "crtc": 0x03B4,
        "status": (8, 0, 8, 8, 0, 0, 0, 8),
        "timeout": False,
        "timer": (0x9C, 0xFF, 0x6A, 0xFF),
        "phase_delta": 1,
    },
    {
        "name": "long_second_clear_phase",
        "crtc": 0xFFFC,
        "status": (0, 8, 0, 0, 8, 8, 8, 0),
        "timeout": False,
        "timer": (0x9C, 0xFF, 0xD4, 0xFE),
        "phase_delta": 1,
    },
    {
        "name": "long_second_set_phase",
        "crtc": 0xFFFE,
        "status": (8, 0, 8, 8, 0, 0, 0, 8),
        "timeout": False,
        "timer": (0x9C, 0xFF, 0xD4, 0xFE),
        "phase_delta": 2,
    },
)


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


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def calibration_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, case in enumerate(CASES):
        name = str(case["name"])
        crtc = int(case["crtc"])
        status_values = list(case["status"])
        timer_values = list(case["timer"])
        timeout = bool(case["timeout"])
        phase_before = 0x20 + case_index * 3
        initial = initial_registers(case_index)
        globals_before = bytearray(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        struct.pack_into("<H", globals_before, CRTC_BASE, crtc)
        globals_before[RETRACE_PHASE] = phase_before
        struct.pack_into("<H", globals_before, TIMER_RELOAD, 0x9669)
        struct.pack_into("<H", globals_before, CALIBRATION_TICKS, 0x5AA5)
        decoy_before = bytes(
            (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
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
        machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
        machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        inputs: list[list[int]] = []
        outputs: list[list[int]] = []
        writes: list[tuple[int, int]] = []
        status_reads = 0
        allowed_global_offsets = {
            RETRACE_PHASE,
            TIMER_RELOAD,
            TIMER_RELOAD + 1,
            CALIBRATION_TICKS,
            CALIBRATION_TICKS + 1,
        }
        allowed_global_addresses = {
            GLOBALS_SEGMENT * 16 + offset for offset in allowed_global_offsets
        }
        allowed_stack_offsets = set(range(CALLER_SP - 8, CALLER_SP))
        allowed_stack_addresses = {
            STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
        }

        def instruction(_cpu: Uc, address: int, size: int, _context: Any) -> None:
            assert ENTRY <= address and address + size <= END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            assert (
                touched <= allowed_global_addresses
                or touched <= allowed_stack_addresses
            ), hex(address)
            writes.append((address, size))

        def input_port(cpu: Uc, port: int, size: int, _context: Any) -> int:
            nonlocal status_reads
            status_port = (crtc + 6) & 0xFFFF
            assert size == 1
            if port == status_port:
                assert status_values, f"{name}: extra status read"
                value = status_values.pop(0)
                status_reads += 1
                if timeout and status_reads == 2:
                    cpu.mem_write(
                        GLOBALS_SEGMENT * 16 + CALIBRATION_TICKS,
                        struct.pack("<H", 0),
                    )
            elif port == 0x61:
                value = 0xA4
            elif port == 0x42:
                assert timer_values, f"{name}: extra timer read"
                value = timer_values.pop(0)
            else:
                raise AssertionError(f"{name}: unexpected read {port:#x}")
            inputs.append([port, size, value])
            return value

        def output_port(
            _cpu: Uc, port: int, size: int, value: int, _context: Any
        ) -> None:
            outputs.append([port, size, value])

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=10_000)

        assert not status_values and not timer_values
        expected_outputs = []
        if not timeout:
            expected_outputs = [
                [0x61, 1, 0xA5],
                [0x43, 1, 0xB0],
                [0x42, 1, 0xFF],
                [0x42, 1, 0xFF],
                [0x43, 1, 0x80],
                [0x43, 1, 0x80],
            ]
        assert outputs == expected_outputs

        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        globals_expected = bytearray(globals_before)
        globals_expected[RETRACE_PHASE] = (
            phase_before + int(case["phase_delta"])
        ) & 0xFF
        struct.pack_into("<H", globals_expected, TIMER_RELOAD, 3)
        struct.pack_into("<H", globals_expected, CALIBRATION_TICKS, 0 if timeout else 2)
        assert globals_after == bytes(globals_expected)
        assert bytes(machine.mem_read(0, len(executable))) == executable
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
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
        assert len(writes) == 6 + int(case["phase_delta"]), (name, writes)

        rows.append(
            {
                "name": name,
                "status_port": (crtc + 6) & 0xFFFF,
                "port_inputs": inputs,
                "port_outputs": outputs,
                "phase_before": phase_before,
                "phase_after": globals_expected[RETRACE_PHASE],
                "calibration_ticks": 0 if timeout else 2,
                "timer_reload_ticks": 3,
                "external_timeout_write": timeout,
                "instruction_write_count": len(writes),
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    assert digest == BODY_SHA256
    return {
        "format": "big_bug_bang_retrace_calibration_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routine": {
            "operation": "vga_retrace_phase_calibrate",
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": digest,
        },
        "cases": calibration_cases(executable),
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
    print(f"verified {len(fixture['cases'])} BBB retrace calibration cases")


if __name__ == "__main__":
    main()
