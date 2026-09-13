#!/usr/bin/env python3
"""Verify BBB's relocated startup transient-file cleanup loop."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_startup_cleanup.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_147f_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x163D
END = 0x1659
BODY_SHA256 = "b890aa634f827f7199b7dc7d3d1560c6ad66d96f5f42e9aaeea873c4e7e34099"
PATH_TABLE_OFFSET = 0x1025
PATH_SLOT_COUNT = 4
PATH_SLOT_SIZE = 16

DATA_SEGMENT = 0x3000
GAME_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x9000
STACK_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x9000
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
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "df": 0x0400,
    "of": 0x0800,
}
CASES = (
    ("all_skip", ("xONE", "xTWO", "xTHREE", "xFOUR")),
    ("all_empty", ("", "", "", "")),
    ("mixed", ("xKEEP", "ONE.TMP", "x", "TWO.$$$")),
    ("case_sensitive_marker", ("XUPPER", "xlower", "z.dat", "last.tmp")),
)
DELETE_RESULTS = ((2, True), (5, False), (0, True), (0xFFFF, False))


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


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


def read_path(machine: Uc, offset: int) -> str:
    raw = bytes(machine.mem_read(DATA_SEGMENT * 16 + offset, PATH_SLOT_SIZE))
    return raw.split(b"\0", 1)[0].decode("ascii")


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (name, paths) in enumerate(CASES):
        direction_set = bool(case_index & 1)
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6C62468 + case_index,
            "ecx": 0xC7D7369C + case_index,
            "edx": 0xD8E855AA + case_index,
            "esi": 0xE9F96789 + case_index,
            "edi": 0xFA0A789A + case_index,
            "ebp": 0x0B1B1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        table = b"".join(
            path.encode("ascii") + bytes(PATH_SLOT_SIZE - len(path)) for path in paths
        )
        data_before = seeded_segment(case_index, 17, 0x21)
        data_before[PATH_TABLE_OFFSET : PATH_TABLE_OFFSET + len(table)] = table
        game_before = bytes(seeded_segment(case_index, 19, 0x31))
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 29, 0x59))
        stack_before = seeded_segment(case_index, 31, 0x67)
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, 1)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_OFFSET, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_SEGMENT * 16, game_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293 | (0x0400 if direction_set else 0))

        calls: list[dict[str, Any]] = []
        writes: list[tuple[int, int]] = []
        reached_return: list[int] = []

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_OFFSET:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def interrupt(cpu: Uc, number: int, _data: object) -> None:
            assert number == 0x21, (name, number)
            assert cpu.reg_read(UC_X86_REG_AX) == 0x4100, name
            offset = cpu.reg_read(UC_X86_REG_DX)
            result, carry = DELETE_RESULTS[len(calls)]
            calls.append(
                {
                    "offset": offset,
                    "path": read_path(cpu, offset),
                    "result_ax": result,
                    "result_carry": carry,
                }
            )
            cpu.reg_write(UC_X86_REG_AX, result)
            set_carry(cpu, carry)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            assert address == STACK_SEGMENT * 16 + CALLER_SP - 2
            assert size == 2
            writes.append((address, size))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=1_000)
        assert reached_return == [RETURN_OFFSET], (name, reached_return)

        expected_paths = [path for path in paths if not path.startswith("x")]
        assert [call["path"] for call in calls] == expected_paths, (name, calls)
        expected_offsets = [
            PATH_TABLE_OFFSET + index * PATH_SLOT_SIZE
            for index, path in enumerate(paths)
            if not path.startswith("x")
        ]
        assert [call["offset"] for call in calls] == expected_offsets, (name, calls)
        assert len(writes) == PATH_SLOT_COUNT

        expected_registers = dict(initial)
        expected_registers["ecx"] = initial["ecx"] & 0xFFFF0000
        expected_registers["edx"] = PATH_TABLE_OFFSET + len(table)
        if calls:
            expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | calls[-1][
                "result_ax"
            ]
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, {
            key: (actual_registers[key], value)
            for key, value in expected_registers.items()
            if actual_registers[key] != value
        }
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        expected_flags = {
            "cf": False,
            "pf": True,
            "af": False,
            "zf": False,
            "sf": False,
            "df": direction_set,
            "of": False,
        }
        assert observed_flags(machine) == expected_flags, name

        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "paths": list(paths),
                "delete_calls": calls,
                "final_dx": expected_registers["edx"],
                "final_cx": 0,
                "defined_flags": expected_flags,
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> str:
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(rows) == len(expected_rows)
    for row, expected in zip(rows, expected_rows, strict=True):
        assert row["name"] == expected["name"]
        assert row["paths"] == expected["paths"]
        assert row["final_cx"] == expected["final_cx"]
        assert row["final_dx"] - PATH_TABLE_OFFSET == expected["final_dx"] - 0x0DD7
        assert [call["path"] for call in row["delete_calls"]] == [
            call["path"] for call in expected["delete_calls"]
        ]
        assert [call["offset"] - PATH_TABLE_OFFSET for call in row["delete_calls"]] == [
            call["offset"] - 0x0DD7 for call in expected["delete_calls"]
        ]
    return hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB startup-cleanup body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_startup_cleanup_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantics(rows),
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "path_table_offset": PATH_TABLE_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB startup-cleanup cases")


if __name__ == "__main__":
    main()
