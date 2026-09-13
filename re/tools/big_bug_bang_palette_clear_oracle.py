#!/usr/bin/env python3
"""Verify BBB's relocated scene-palette clear helper."""

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

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16  # noqa: E402
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_palette_clear.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_248b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x280B
END = 0x2826
BODY_SHA256 = "442971c67e618b103ce99eaf753d0f3d190287d7207e1232561f655fc6a4ca12"
PALETTE_OFFSET = 0x5621
PALETTE_BYTE_COUNT = 0x300
CLEAR_BYTE_COUNT = 0x240
CLEAR_DWORD_COUNT = CLEAR_BYTE_COUNT // 4
WINDOW_PALETTE_INDEX = 0x351
WINDOW_OFFSET = PALETTE_OFFSET - WINDOW_PALETTE_INDEX
WINDOW_SIZE = 0x700

GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x9000
STACK_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_SEGMENT = 0x1800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

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
    "df": 0x0400,
    "of": 0x0800,
}
CASES = (
    ("ascending_pattern", False, 0x11),
    ("ascending_already_zero", False, 0x00),
    ("ascending_ffff", False, 0xFF),
    ("descending_pattern", True, 0x53),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def patterned_window(seed: int) -> bytes:
    if seed == 0:
        return bytes(WINDOW_SIZE)
    if seed == 0xFF:
        return bytes([0xFF]) * WINDOW_SIZE
    return bytes(
        (seed + index * 0x2D + (index >> 3)) & 0xFF for index in range(WINDOW_SIZE)
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
    for case_index, (name, direction_set, seed) in enumerate(CASES):
        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        game_before = seeded_segment(case_index, 17, 0x21)
        game_before[WINDOW_OFFSET : WINDOW_OFFSET + WINDOW_SIZE] = patterned_window(
            seed
        )
        game_expected = bytearray(game_before)
        if direction_set:
            clear_start = PALETTE_OFFSET - CLEAR_BYTE_COUNT + 4
            clear_end = PALETTE_OFFSET + 4
            final_di = clear_start - 4
        else:
            clear_start = PALETTE_OFFSET
            clear_end = PALETTE_OFFSET + CLEAR_BYTE_COUNT
            final_di = clear_end
        game_expected[clear_start:clear_end] = bytes(CLEAR_BYTE_COUNT)

        data_before = bytes(seeded_segment(case_index, 19, 0x31))
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 29, 0x59))
        stack_before = seeded_segment(case_index, 31, 0x67)
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, EXTRA_SEGMENT)
        struct.pack_into("<H", stack_expected, CALLER_SP - 4, initial["edi"] & 0xFFFF)
        struct.pack_into("<I", stack_expected, CALLER_SP - 8, initial["eax"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 10, initial["ecx"] & 0xFFFF)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, data_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))

        for name_, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[name_])
        for name_, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[name_])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293 | (0x0400 if direction_set else 0))

        phases: list[dict[str, int]] = []
        writes: list[tuple[int, int]] = []
        reached_return: list[int] = []
        game_addresses = set(
            range(GAME_SEGMENT * 16 + clear_start, GAME_SEGMENT * 16 + clear_end)
        )
        stack_addresses = set(
            range(STACK_SEGMENT * 16 + CALLER_SP - 10, STACK_SEGMENT * 16 + CALLER_SP)
        )

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)
            if address in (0x281D, 0x2820):
                if address == 0x281D and phases:
                    return
                phases.append(
                    {
                        "address": address,
                        "eax": cpu.reg_read(UC_X86_REG_EAX),
                        "ecx": cpu.reg_read(UC_X86_REG_ECX),
                        "edi": cpu.reg_read(UC_X86_REG_EDI),
                        "es": cpu.reg_read(UC_X86_REG_ES),
                    }
                )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= game_addresses or addresses <= stack_addresses, hex(
                address
            )
            writes.append((address, size))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=1_000)
        assert reached_return == [RETURN_ADDRESS], (name, reached_return)

        expected_phases = [
            {
                "address": 0x281D,
                "eax": 0,
                "ecx": (initial["ecx"] & 0xFFFF0000) | CLEAR_DWORD_COUNT,
                "edi": (initial["edi"] & 0xFFFF0000) | PALETTE_OFFSET,
                "es": GAME_SEGMENT,
            },
            {
                "address": 0x2820,
                "eax": 0,
                "ecx": initial["ecx"] & 0xFFFF0000,
                "edi": (initial["edi"] & 0xFFFF0000) | final_di,
                "es": GAME_SEGMENT,
            },
        ]
        assert phases == expected_phases, (name, phases, expected_phases)
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert (
            sum(address in game_addresses for address, _ in writes) == CLEAR_DWORD_COUNT
        )
        assert sum(address in stack_addresses for address, _ in writes) == 4

        actual_registers = snapshot_registers(machine)
        assert actual_registers == initial, {
            key: (actual_registers[key], value)
            for key, value in initial.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        expected_flags = {
            "cf": False,
            "pf": True,
            "zf": True,
            "sf": False,
            "df": direction_set,
            "of": False,
        }
        assert observed_flags(machine) == expected_flags, name

        palette_before = game_before[
            PALETTE_OFFSET : PALETTE_OFFSET + PALETTE_BYTE_COUNT
        ]
        palette_after = game_expected[
            PALETTE_OFFSET : PALETTE_OFFSET + PALETTE_BYTE_COUNT
        ]
        rows.append(
            {
                "name": name,
                "direction": "descending" if direction_set else "ascending",
                "seed": seed,
                "clear_start": clear_start,
                "clear_end_exclusive": clear_end,
                "cleared_bytes": clear_end - clear_start,
                "palette_entries_cleared": 192 if not direction_set else None,
                "upper_palette_entries_preserved": 64 if not direction_set else None,
                "palette_before_sha256": hashlib.sha256(palette_before).hexdigest(),
                "palette_after_sha256": hashlib.sha256(palette_after).hexdigest(),
                "result_sha256": hashlib.sha256(
                    game_expected[WINDOW_OFFSET : WINDOW_OFFSET + WINDOW_SIZE]
                ).hexdigest(),
                "defined_flags": expected_flags,
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> str:
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(rows) == len(expected_rows)
    comparable_fields = (
        "name",
        "direction",
        "seed",
        "cleared_bytes",
        "palette_entries_cleared",
        "upper_palette_entries_preserved",
        "palette_before_sha256",
        "palette_after_sha256",
        "result_sha256",
        "defined_flags",
    )
    for row, expected in zip(rows, expected_rows, strict=True):
        for field in comparable_fields:
            assert row[field] == expected[field], (field, row["name"])
        assert row["clear_start"] - PALETTE_OFFSET == expected["clear_start"] - 0x5251
        assert (
            row["clear_end_exclusive"] - PALETTE_OFFSET
            == expected["clear_end_exclusive"] - 0x5251
        )
    return hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB palette-clear body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_palette_clear_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantics(rows),
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "palette_offset": PALETTE_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB scene-palette clear cases")


if __name__ == "__main__":
    main()
