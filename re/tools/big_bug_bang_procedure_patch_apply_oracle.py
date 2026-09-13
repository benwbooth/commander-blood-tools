#!/usr/bin/env python3
"""Verify BBB's relocated procedure patch-stream applier."""

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
DEFAULT_OUTPUT = (
    ROOT / "re/tools/oracle_vectors/big_bug_bang_procedure_patch_apply.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1d74_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

ENTRY = 0x1FE5
END = 0x2005
BODY_SHA256 = "b61eebdee63369177477272b3eb76ccdba8015016a3e8ff4f47fa54ca36d442a"
SOURCE_POINTER_OFFSET = 0x0CB4
TARGET_POINTER_OFFSET = 0x6AF4

GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x4000
SOURCE_SEGMENT = 0x6000
TARGET_SEGMENT = 0x8000
SOURCE_DECOY_SEGMENT = 0xA000
TARGET_DECOY_SEGMENT = 0xC000
STACK_SEGMENT = 0xE000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x9000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
TARGET_BASE_OFFSET = 0x3800

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
    ("single_absolute_offset", 0x0003, 0x0200, ((0x0010, 0xA1),), False),
    (
        "ordered_duplicate_and_boundaries",
        0x000C,
        0x1400,
        ((0x0000, 0x11), (0xFFFF, 0x22), (0x1234, 0x33), (0x1234, 0x44)),
        False,
    ),
    (
        "source_offset_wrap",
        0x0009,
        0xFFFD,
        ((0x2222, 0x55), (0x3333, 0x66), (0x4444, 0x77)),
        False,
    ),
    ("zero_count_full_wrap", 0x0000, 0x7A31, (), True),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 19 + case_index * 41 + salt) & 0xFF
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


def simulate(
    byte_count: int,
    source_offset: int,
    source: bytes | bytearray,
    source_linear_tail: int,
    target_before: bytes,
) -> tuple[bytearray, int, int]:
    target = bytearray(target_before)
    cx = byte_count
    si = source_offset
    iterations = 0
    final_target_offset = 0
    while True:
        final_target_offset = source[si] | (
            (source_linear_tail if si == 0xFFFF else source[si + 1]) << 8
        )
        si = (si + 2) & 0xFFFF
        target[final_target_offset] = source[si]
        si = (si + 1) & 0xFFFF
        cx = (cx - 3) & 0xFFFF
        iterations += 1
        if cx == 0:
            return target, final_target_offset, iterations
        assert iterations <= 0x10000


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (
        name,
        byte_count,
        source_offset,
        records,
        fill_entire_stream,
    ) in enumerate(CASES):
        if fill_entire_stream:
            source_before = bytearray(
                (offset * 73 + (offset >> 8) * 19 + 0x2D) & 0xFF
                for offset in range(SEGMENT_SIZE)
            )
        else:
            source_before = bytearray([0xCC] * SEGMENT_SIZE)
            cursor = source_offset
            for target_offset, value in records:
                source_before[cursor] = target_offset & 0xFF
                source_before[(cursor + 1) & 0xFFFF] = target_offset >> 8
                source_before[(cursor + 2) & 0xFFFF] = value
                cursor = (cursor + 3) & 0xFFFF
        target_before = bytes(
            (offset * 29 + case_index * 41 + 0x17) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        source_linear_tail = (0x9D + case_index * 7) & 0xFF
        target_expected, final_target_offset, iterations = simulate(
            byte_count,
            source_offset,
            source_before,
            source_linear_tail,
            target_before,
        )

        source_pointer = struct.pack("<HH", source_offset, SOURCE_SEGMENT)
        target_pointer = struct.pack("<HH", TARGET_BASE_OFFSET, TARGET_SEGMENT)
        game_before = seeded_segment(case_index, 17, 0x21)
        game_before[SOURCE_POINTER_OFFSET : SOURCE_POINTER_OFFSET + 4] = source_pointer
        game_before[TARGET_POINTER_OFFSET : TARGET_POINTER_OFFSET + 4] = target_pointer
        data_before = seeded_segment(case_index, 23, 0x43)
        data_before[SOURCE_POINTER_OFFSET : SOURCE_POINTER_OFFSET + 4] = struct.pack(
            "<HH", 0x2400, SOURCE_DECOY_SEGMENT
        )
        data_before[TARGET_POINTER_OFFSET : TARGET_POINTER_OFFSET + 4] = struct.pack(
            "<HH", 0x4200, TARGET_DECOY_SEGMENT
        )
        source_decoy_before = bytes([0xD1]) * SEGMENT_SIZE
        target_decoy_before = bytes([0xE2]) * SEGMENT_SIZE
        stack_before = seeded_segment(case_index, 31, 0x67)
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        initial = {
            "eax": 0xA1A10000 | byte_count,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": DATA_SEGMENT,
            "es": TARGET_DECOY_SEGMENT,
            "fs": 0x1000,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, initial["ds"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 4, initial["esi"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 6, initial["es"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 8, initial["edi"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 10, initial["ecx"] & 0xFFFF)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_OFFSET, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(SOURCE_SEGMENT * 16, bytes(source_before))
        machine.mem_write(
            SOURCE_SEGMENT * 16 + SEGMENT_SIZE, bytes([source_linear_tail])
        )
        machine.mem_write(TARGET_SEGMENT * 16, target_before)
        machine.mem_write(SOURCE_DECOY_SEGMENT * 16, source_decoy_before)
        machine.mem_write(TARGET_DECOY_SEGMENT * 16, target_decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        write_counts = {"target": 0, "stack": 0}
        reached_return: list[int] = []

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_OFFSET:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            if TARGET_SEGMENT * 16 <= address < TARGET_SEGMENT * 16 + SEGMENT_SIZE:
                assert size == 1
                write_counts["target"] += 1
                return
            assert (
                STACK_SEGMENT * 16 + CALLER_SP - 10
                <= address
                < STACK_SEGMENT * 16 + CALLER_SP
            )
            assert size == 2
            write_counts["stack"] += 1

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=iterations * 6 + 64)
        assert reached_return == [RETURN_OFFSET], (name, reached_return)
        assert write_counts == {"target": iterations, "stack": 5}, name

        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | final_target_offset
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
            "zf": True,
            "sf": False,
            "of": False,
        }
        assert observed_flags(machine) == expected_flags, name

        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(SOURCE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            source_before
        )
        assert (
            machine.mem_read(SOURCE_SEGMENT * 16 + SEGMENT_SIZE, 1)[0]
            == source_linear_tail
        )
        assert bytes(machine.mem_read(TARGET_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            target_expected
        )
        assert (
            bytes(machine.mem_read(SOURCE_DECOY_SEGMENT * 16, SEGMENT_SIZE))
            == source_decoy_before
        )
        assert (
            bytes(machine.mem_read(TARGET_DECOY_SEGMENT * 16, SEGMENT_SIZE))
            == target_decoy_before
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        changed_offsets = [
            offset
            for offset, (before, after) in enumerate(
                zip(target_before, target_expected, strict=True)
            )
            if before != after
        ]
        rows.append(
            {
                "name": name,
                "byte_count": byte_count,
                "iterations": iterations,
                "source_offset": source_offset,
                "target_pointer_offset_ignored": TARGET_BASE_OFFSET,
                "records": [
                    {"target_offset": target_offset, "value": value}
                    for target_offset, value in records
                ],
                "changed_target_bytes": len(changed_offsets),
                "target_sha256": hashlib.sha256(target_expected).hexdigest(),
                "result_ax": final_target_offset,
                "defined_flags": expected_flags,
            }
        )
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB procedure patch-apply body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    commander_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert rows == commander_rows
    result = {
        "format": "big_bug_bang_procedure_patch_apply_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": hashlib.sha256(
            COMMANDER_FIXTURE.read_bytes()
        ).hexdigest(),
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "source_pointer_offset": SOURCE_POINTER_OFFSET,
            "target_pointer_offset": TARGET_POINTER_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB procedure patch-apply cases")


if __name__ == "__main__":
    main()
