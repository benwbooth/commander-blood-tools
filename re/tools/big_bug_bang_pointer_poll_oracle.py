#!/usr/bin/env python3
"""Verify BBB's changed mouse-poll publication semantics."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_pointer_poll.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY = 0x0F09
END = 0x0F3E
BODY_SHA256 = "023092bb695efa9c2771f588beb92a8ece4892dac680f828087b2df986e04ed7"

GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

CURRENT_X = 0x0C22
CURRENT_Y = 0x0C24
CURRENT_BUTTONS = 0x0C26
PREVIOUS_X = 0x0C30
PREVIOUS_Y = 0x0C32
CASES = (
    ("unchanged", 0x0123, 0x0456, 0x0123, 0x0456, 0x0000),
    ("x_changed", 0x0124, 0x0456, 0x0123, 0x0456, 0x0001),
    ("y_changed", 0x0123, 0x0457, 0x0123, 0x0456, 0x0002),
    ("both_changed", 0x8000, 0x7FFF, 0x7FFF, 0x8000, 0xFFFF),
    ("unsigned_wrap_compare", 0x0000, 0xFFFF, 0xFFFF, 0xFFFF, 0x00A5),
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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def sub16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool((left ^ right) & (left ^ result) & 0x8000),
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


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def pointer_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, x, y, last_x, last_y, buttons) in enumerate(CASES):
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B60000 | (0x2468 + case_index),
            "ecx": 0xC7C70000 | (0x369C + case_index),
            "edx": 0xD8D80000 | (0x55AA + case_index),
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
        struct.pack_into("<HH", game_before, PREVIOUS_X, last_x, last_y)
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

        interrupts: list[dict[str, int]] = []
        writes: list[tuple[int, int, int]] = []
        allowed_game_offsets = set(range(CURRENT_X, CURRENT_BUTTONS + 2)) | set(
            range(PREVIOUS_X, PREVIOUS_Y + 2)
        )
        allowed_game_addresses = {
            GAME_SEGMENT * 16 + offset for offset in allowed_game_offsets
        }
        allowed_stack_offsets = set(range(CALLER_SP - 8, CALLER_SP))
        allowed_stack_addresses = {
            STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
        }

        def instruction(_cpu: Uc, address: int, size: int, _context: Any) -> None:
            assert ENTRY <= address and address + size <= END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            interrupts.append(
                {
                    "number": number,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "bx": cpu.reg_read(UC_X86_REG_BX),
                    "cx": cpu.reg_read(UC_X86_REG_CX),
                    "dx": cpu.reg_read(UC_X86_REG_DX),
                }
            )
            cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
            cpu.reg_write(UC_X86_REG_BX, buttons)
            cpu.reg_write(UC_X86_REG_CX, x)
            cpu.reg_write(UC_X86_REG_DX, y)
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0643)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            assert (
                touched <= allowed_game_addresses or touched <= allowed_stack_addresses
            ), hex(address)
            writes.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=500)

        expected_interrupt = {
            "number": 0x33,
            "ax": 3,
            "bx": initial["ebx"] & 0xFFFF,
            "cx": initial["ecx"] & 0xFFFF,
            "dx": initial["edx"] & 0xFFFF,
        }
        assert interrupts == [expected_interrupt]
        moved = x != last_x or y != last_y
        game_expected = bytearray(game_before)
        struct.pack_into("<HHH", game_expected, CURRENT_X, x, y, buttons)
        if moved:
            struct.pack_into("<HH", game_expected, PREVIOUS_X, x, y)
        game_after = bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
        assert game_after == bytes(game_expected)
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
        compared_left, compared_right = (x, last_x) if x != last_x else (y, last_y)
        expected_flags = sub16_flags(compared_left, compared_right)
        assert observed_flags(machine) == expected_flags
        game_writes = [
            (address, size, value)
            for address, size, value in writes
            if address in allowed_game_addresses
        ]
        assert len(game_writes) == (5 if moved else 3), (name, game_writes)
        assert len(writes) == len(game_writes) + 4, (name, writes)

        idle_before = (0xA100 + case_index * 0x111) & 0xFFFF
        rows.append(
            {
                "name": name,
                "driver": {"x": x, "y": y, "buttons": buttons},
                "previous": {"x": last_x, "y": last_y, "idle": idle_before},
                "moved": moved,
                "stored_idle": idle_before,
                "interrupt": expected_interrupt,
                "defined_flags": expected_flags,
            }
        )
    return rows


def build_fixture(executable: bytes) -> list[dict[str, Any]]:
    digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    assert digest == BODY_SHA256
    return pointer_cases(executable)


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
    print(f"verified {len(fixture)} BBB pointer-poll cases")


if __name__ == "__main__":
    main()
