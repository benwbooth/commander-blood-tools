#!/usr/bin/env python3
"""Verify BBB's unchanged VGA page-offset helper."""

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
    UC_X86_INS_OUT,
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
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_page_offset.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY = 0x1971
END = 0x199B
BODY_SHA256 = "130238d31d3a2c25455f1460421594329c274a41bde4fb296aa8f764ef55f475"

DATA_SEGMENT = 0x3000
GLOBALS_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x6E00
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")
DRAW_POINTER = 0x55E9
SCREEN_POINTER = 0x55ED
CRTC_PORT = 0x0C96

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
    "of": 0x0800,
}
CASES = (
    ("both_zero", 0x0000, 0x0000, 0x03D4),
    ("ordinary", 0x0001, 0x1234, 0x03B4),
    ("first_negative", 0x8000, 0x2345, 0x03D4),
    ("second_negative", 0x3456, 0xFFFF, 0x03B4),
    ("maximum_positive", 0x7FFF, 0x7FFF, 0x03D4),
    ("both_negative", 0xC000, 0x8001, 0x03B4),
    ("high_positive_wraps_sign", 0x7000, 0x6000, 0xFFFF),
    ("offset_words_wrap_independently", 0x4000, 0x7F00, 0x0000),
)


def add16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left + right) & 0xFFFF
    return {
        "cf": left + right > 0xFFFF,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool((~(left ^ right) & (left ^ result)) & 0x8000),
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
        "ds": DATA_SEGMENT,
        "es": EXTRA_SEGMENT,
        "fs": 0x7800,
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


def observed_flags(machine: Uc) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(value & mask) for name, mask in FLAG_MASKS.items()}


def offset_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, draw_offset, screen_offset, crtc_port) in enumerate(CASES):
        initial = initial_registers(case_index)
        draw_segment = 0x1111 + case_index
        screen_segment = 0x2222 + case_index
        data_before = bytearray(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        struct.pack_into(
            "<HHHH",
            data_before,
            DRAW_POINTER,
            draw_offset,
            draw_segment,
            screen_offset,
            screen_segment,
        )
        struct.pack_into("<H", data_before, CRTC_PORT, crtc_port)
        globals_before = bytearray(
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
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202 | (0x0400 if case_index & 1 else 0))

        outputs: list[list[int]] = []
        writes: list[tuple[int, int]] = []
        allowed_data_addresses = {
            DATA_SEGMENT * 16 + DRAW_POINTER,
            DATA_SEGMENT * 16 + DRAW_POINTER + 1,
            DATA_SEGMENT * 16 + SCREEN_POINTER,
            DATA_SEGMENT * 16 + SCREEN_POINTER + 1,
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
            assert set(range(address, address + size)) <= allowed_data_addresses, hex(
                address
            )
            writes.append((address, size))

        def output_port(
            _cpu: Uc, port: int, size: int, value: int, _context: Any
        ) -> None:
            outputs.append([port, size, value])

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=1_000)

        expected_draw = 0 if draw_offset & 0x8000 else (draw_offset + 0x4000) & 0xFFFF
        expected_screen = (
            0 if screen_offset & 0x8000 else (screen_offset + 0x4000) & 0xFFFF
        )
        data_expected = bytearray(data_before)
        struct.pack_into("<H", data_expected, DRAW_POINTER, expected_draw)
        struct.pack_into("<H", data_expected, SCREEN_POINTER, expected_screen)
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            globals_before
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_before
        )
        assert bytes(machine.mem_read(0, len(executable))) == executable
        assert len(writes) == 2

        expected_output = [[crtc_port, 2, (expected_screen & 0xFF00) | 0x000C]]
        assert outputs == expected_output, (name, outputs, expected_output)
        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_output[0][
            2
        ]
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | crtc_port
        assert snapshot_registers(machine) == expected_registers
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2

        expected_flags = (
            {
                "cf": False,
                "pf": True,
                "af": False,
                "zf": True,
                "sf": False,
                "of": False,
            }
            if screen_offset & 0x8000
            else add16_flags(screen_offset, 0x4000)
        )
        assert observed_flags(machine) == expected_flags
        rows.append(
            {
                "name": name,
                "draw_offset_before": draw_offset,
                "draw_offset_after": expected_draw,
                "screen_offset_before": screen_offset,
                "screen_offset_after": expected_screen,
                "crtc_port": crtc_port,
                "port_write": outputs[0],
                "final_ax": machine.reg_read(UC_X86_REG_AX),
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    assert digest == BODY_SHA256
    return {
        "format": "big_bug_bang_page_offset_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routine": {
            "operation": "vga_page_offset_advance",
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": digest,
        },
        "cases": offset_cases(executable),
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
    print(f"verified {len(fixture['cases'])} BBB VGA page-offset cases")


if __name__ == "__main__":
    main()
