#!/usr/bin/env python3
"""Verify BBB's relocated pointer-button edge sampler."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

from unicorn import (  # noqa: E402
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_pointer_button_edges.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1fbc_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "c260657ea798a4db44247b04c738e2326e66bc4d38fc1da2933d56fbe8d11090"
)

ENTRY = 0x224F
END = 0x2281
BODY_SHA256 = "4a21bf2c5361cbfd9f8e9996b5054dfabfb01762abab7e12eea9c2f808da44c0"
SECOND_PREVIOUS_READ = 0x226A
FINAL_CURRENT_READ = 0x227A
CURRENT_OFFSET = 0x0C26
PREVIOUS_OFFSET = 0x0C28
PRIMARY_LATCH_OFFSET = 0x0C36
SECONDARY_LATCH_OFFSET = 0x0C37
PENDING_LATCH_OFFSET = 0x0C38

DATA_SEGMENT = 0x3000
GAME_DECOY_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
FS_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x2800
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("87a55a963cc37869")

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
    ("none", 0x0000, 0x0000, None, None),
    ("primary_new", 0x0001, 0x0000, None, None),
    ("primary_held", 0x0001, 0x0001, None, None),
    ("secondary_new", 0x0002, 0x0000, None, None),
    ("secondary_held", 0x0002, 0x0002, None, None),
    ("both_new_primary_only", 0x0003, 0x0000, None, None),
    ("primary_new_secondary_held_suppressed", 0x0003, 0x0002, None, None),
    ("primary_held_secondary_new_suppressed", 0x0003, 0x0001, None, None),
    ("primary_new_with_unrelated_previous", 0x0001, 0x0004, None, None),
    ("secondary_blocked_by_other_held", 0x0006, 0x0004, None, None),
    ("secondary_new_with_previous_primary", 0x0002, 0x0001, None, None),
    ("unwatched_button", 0x0004, 0x0000, None, None),
    ("high_words_ignored_for_edges", 0xA501, 0xB200, None, None),
    ("current_word_reloaded", 0xC301, 0xD400, None, 0xE502),
    ("previous_low_byte_reloaded", 0xF603, 0x9702, 0x9800, None),
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


def snapshot_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (
        name,
        current_initial,
        previous_initial,
        previous_second_read,
        current_final_override,
    ) in enumerate(CASES):
        primary_before = (0x40 + case_index) & 0xFF
        secondary_before = (0x60 + case_index) & 0xFF
        pending_before = (0x80 + case_index) & 0xFF
        current_final_read = (
            current_initial
            if current_final_override is None
            else current_final_override
        )

        working = current_initial & 0xFF
        primary_after = primary_before
        secondary_after = secondary_before
        pending_after = pending_before
        if working & 0x01:
            working &= previous_initial & 0xFF
            if working == 0:
                primary_after = 1
                pending_after = 1
        if working & 0x02:
            previous_low = (
                previous_initial
                if previous_second_read is None
                else previous_second_read
            ) & 0xFF
            working &= previous_low
            if working == 0:
                secondary_after = 1
                pending_after = 1

        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0xA7A7789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 17, 0x21)
        struct.pack_into(
            "<HH", data_before, CURRENT_OFFSET, current_initial, previous_initial
        )
        data_before[PRIMARY_LATCH_OFFSET : PENDING_LATCH_OFFSET + 1] = bytes(
            (primary_before, secondary_before, pending_before)
        )
        data_expected = bytearray(data_before)
        struct.pack_into("<H", data_expected, CURRENT_OFFSET, current_final_read)
        struct.pack_into("<H", data_expected, PREVIOUS_OFFSET, current_final_read)
        data_expected[PRIMARY_LATCH_OFFSET : PENDING_LATCH_OFFSET + 1] = bytes(
            (primary_after, secondary_after, pending_after)
        )

        game_decoy_before = seeded_segment(case_index, 19, 0x31)
        game_decoy_before[CURRENT_OFFSET : PREVIOUS_OFFSET + 2] = bytes(
            value ^ 0xA5
            for value in struct.pack("<HH", current_initial, previous_initial)
        )
        game_decoy_before[PRIMARY_LATCH_OFFSET : PENDING_LATCH_OFFSET + 1] = bytes(
            value ^ 0xA5 for value in (primary_before, secondary_before, pending_before)
        )
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 29, 0x59))
        stack_before = seeded_segment(case_index, 31, 0x67)
        struct.pack_into("<H", stack_before, CALLER_SP, RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_DECOY_SEGMENT * 16, bytes(game_decoy_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        machine.reg_write(UC_X86_REG_EFLAGS, initial_flags)

        reached_return: list[int] = []
        allowed_data_addresses = {
            DATA_SEGMENT * 16 + CURRENT_OFFSET,
            DATA_SEGMENT * 16 + CURRENT_OFFSET + 1,
            DATA_SEGMENT * 16 + PREVIOUS_OFFSET,
            DATA_SEGMENT * 16 + PREVIOUS_OFFSET + 1,
            DATA_SEGMENT * 16 + PRIMARY_LATCH_OFFSET,
            DATA_SEGMENT * 16 + SECONDARY_LATCH_OFFSET,
            DATA_SEGMENT * 16 + PENDING_LATCH_OFFSET,
        }

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)
            if address == SECOND_PREVIOUS_READ and previous_second_read is not None:
                cpu.mem_write(
                    DATA_SEGMENT * 16 + PREVIOUS_OFFSET,
                    struct.pack("<H", previous_second_read),
                )
            if address == FINAL_CURRENT_READ and current_final_override is not None:
                cpu.mem_write(
                    DATA_SEGMENT * 16 + CURRENT_OFFSET,
                    struct.pack("<H", current_final_override),
                )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            assert set(range(address, address + size)) <= allowed_data_addresses, hex(
                address
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=200)

        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | current_final_read
        assert reached_return == [RETURN_ADDRESS], name
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == bool(
            initial_flags & 0x0400
        )
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(GAME_DECOY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_decoy_before
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_before
        )

        rows.append(
            {
                "name": name,
                "current_initial": current_initial,
                "previous_initial": previous_initial,
                "previous_second_read": previous_second_read,
                "current_final_read": current_final_read,
                "primary_after": primary_after,
                "secondary_after": secondary_after,
                "pending_after": pending_after,
                "result_ax": current_final_read,
                "defined_flags": snapshot_flags(machine),
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> None:
    actual_sha256 = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert actual_sha256 == COMMANDER_FIXTURE_SHA256, actual_sha256
    assert rows == json.loads(COMMANDER_FIXTURE.read_text())


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB pointer-button edge body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    verify_commander_semantics(rows)
    result = {
        "format": "big_bug_bang_pointer_button_edges_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "current_offset": CURRENT_OFFSET,
            "previous_offset": PREVIOUS_OFFSET,
            "latch_offsets": [
                PRIMARY_LATCH_OFFSET,
                SECONDARY_LATCH_OFFSET,
                PENDING_LATCH_OFFSET,
            ],
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB pointer-button edge cases")


if __name__ == "__main__":
    main()
