#!/usr/bin/env python3
"""Verify BBB's relocated palette-upload gate."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_palette_upload.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_178b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x194D
END = 0x1971
BODY_SHA256 = "20735937f628e051ed07aca3ddc169f79625cc3492434d6b43c27d6a77e4a55f"
RETRACE_ENTRY = 0x05D2
PALETTE_ENTRY_SEGMENT = 0x02B1
PALETTE_ENTRY = PALETTE_ENTRY_SEGMENT * 16
DIRTY_OFFSET = 0x5F25
PALETTE_OFFSET = 0x5621
PRIMARY_OFFSET = 0x0C36
SECONDARY_OFFSET = 0x0C37
PENDING_OFFSET = 0x0C38
PALETTE_BYTE_COUNT = 768

DATA_SEGMENT = 0x3000
GAME_DECOY_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x9000
STACK_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x9000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
HELPER_FLAGS = 0x0A93

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
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}
CASES = (
    ("clean_zero", 0x00),
    ("dirty_bit", 0x01),
    ("clean_other_bit", 0x02),
    ("dirty_with_other_bit", 0x03),
    ("clean_high_bit", 0x80),
    ("clean_all_even_bits", 0xFE),
    ("dirty_all_bits", 0xFF),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
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


def observed_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (name, dirty_value) in enumerate(CASES):
        dirty_path = bool(dirty_value & 1)
        primary_before = (0x31 + case_index) & 0xFF
        secondary_before = (0x51 + case_index) & 0xFF
        pending_before = (0x71 + case_index) & 0xFF
        palette = bytes(
            (index * 37 + case_index * 11) & 0x3F for index in range(PALETTE_BYTE_COUNT)
        )
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 17, 0x21)
        data_before[PALETTE_OFFSET : PALETTE_OFFSET + PALETTE_BYTE_COUNT] = palette
        data_before[DIRTY_OFFSET] = dirty_value
        data_before[PRIMARY_OFFSET : PENDING_OFFSET + 1] = bytes(
            [primary_before, secondary_before, pending_before]
        )
        data_expected = bytearray(data_before)
        if dirty_path:
            data_expected[DIRTY_OFFSET] = 0
            data_expected[PRIMARY_OFFSET] = 0
            data_expected[PENDING_OFFSET] = 0
        decoy_before = seeded_segment(case_index, 19, 0x31)
        decoy_before[PALETTE_OFFSET : PALETTE_OFFSET + PALETTE_BYTE_COUNT] = bytes(
            value ^ 0x3F for value in palette
        )
        decoy_before[DIRTY_OFFSET] = dirty_value ^ 0xA5
        decoy_before[PRIMARY_OFFSET : PENDING_OFFSET + 1] = bytes(
            [primary_before ^ 0xA5, secondary_before ^ 0xA5, pending_before ^ 0xA5]
        )
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 29, 0x59))
        stack_before = seeded_segment(case_index, 31, 0x67)
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        if dirty_path:
            struct.pack_into("<HH", stack_expected, CALLER_SP - 4, 0x1961, 0)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_OFFSET, b"\xcc")
        machine.mem_write(RETRACE_ENTRY, b"\xcb")
        machine.mem_write(PALETTE_ENTRY, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_DECOY_SEGMENT * 16, bytes(decoy_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202 | (case_index & 1))

        helper_calls: list[str] = []
        reached_return: list[int] = []

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_OFFSET:
                reached_return.append(address)
                cpu.emu_stop()
                return
            if address not in (RETRACE_ENTRY, PALETTE_ENTRY):
                assert ENTRY <= address < END, hex(address)
                return

            palette_call = address == PALETTE_ENTRY
            expected_call = dict(initial)
            if palette_call:
                expected_call["esi"] = (initial["esi"] & 0xFFFF0000) | PALETTE_OFFSET
            actual_call = snapshot_registers(cpu)
            assert actual_call == expected_call, (name, actual_call, expected_call)
            assert cpu.reg_read(UC_X86_REG_CS) == (
                PALETTE_ENTRY_SEGMENT if palette_call else 0
            )
            assert cpu.reg_read(UC_X86_REG_IP) == (0 if palette_call else RETRACE_ENTRY)
            assert cpu.reg_read(UC_X86_REG_SP) == CALLER_SP - 4
            expected_return = 0x1961 if palette_call else 0x1959
            assert struct.unpack(
                "<HHH", cpu.mem_read(STACK_SEGMENT * 16 + CALLER_SP - 4, 6)
            ) == (expected_return, 0, RETURN_OFFSET)
            helper_calls.append("palette" if palette_call else "retrace")

            if palette_call:
                assert (
                    cpu.mem_read(DATA_SEGMENT * 16 + DIRTY_OFFSET, 1)[0] == dirty_value
                )
                assert bytes(
                    cpu.mem_read(DATA_SEGMENT * 16 + PRIMARY_OFFSET, 3)
                ) == bytes([primary_before, secondary_before, pending_before])
                cpu.mem_write(DATA_SEGMENT * 16 + DIRTY_OFFSET, b"\xa6")
                cpu.mem_write(DATA_SEGMENT * 16 + PRIMARY_OFFSET, b"\xb7")
                cpu.mem_write(DATA_SEGMENT * 16 + PENDING_OFFSET, b"\xc8")
                cpu.reg_write(UC_X86_REG_EFLAGS, HELPER_FLAGS)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.emu_start(ENTRY, 0, count=1_000)
        assert reached_return == [RETURN_OFFSET], (name, reached_return)
        expected_calls = ["retrace", "palette"] if dirty_path else []
        assert helper_calls == expected_calls, (name, helper_calls)

        expected_registers = dict(initial)
        if dirty_path:
            expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | PALETTE_OFFSET
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, {
            key: (actual_registers[key], value)
            for key, value in expected_registers.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        expected_flags = (
            {name_: bool(HELPER_FLAGS & mask) for name_, mask in FLAG_MASKS.items()}
            if dirty_path
            else {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "of": False,
            }
        )
        assert observed_flags(machine) == expected_flags, name

        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(GAME_DECOY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            decoy_before
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "dirty_before": dirty_value,
                "dirty_path": dirty_path,
                "calls": expected_calls,
                "palette_offset": PALETTE_OFFSET if dirty_path else None,
                "dirty_after": data_expected[DIRTY_OFFSET],
                "primary_after": data_expected[PRIMARY_OFFSET],
                "secondary_after": data_expected[SECONDARY_OFFSET],
                "pending_after": data_expected[PENDING_OFFSET],
                "si_after": expected_registers["esi"] & 0xFFFF,
                "defined_flags": expected_flags,
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> str:
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(rows) == len(expected_rows)
    normalized = json.loads(json.dumps(rows))
    for row in normalized:
        if row["dirty_path"]:
            row["palette_offset"] = 0x5251
            row["si_after"] = 0x5251
    assert normalized == expected_rows
    return hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB palette-upload body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_palette_upload_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantics(rows),
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "dirty_offset": DIRTY_OFFSET,
            "palette_offset": PALETTE_OFFSET,
            "mouse_latch_offsets": [PRIMARY_OFFSET, SECONDARY_OFFSET, PENDING_OFFSET],
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB palette-upload cases")


if __name__ == "__main__":
    main()
