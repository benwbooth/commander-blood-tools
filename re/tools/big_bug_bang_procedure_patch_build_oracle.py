#!/usr/bin/env python3
"""Verify BBB's relocated procedure patch-stream builder."""

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
DEFAULT_OUTPUT = (
    ROOT / "re/tools/oracle_vectors/big_bug_bang_procedure_patch_build.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1d94_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "02aa7f1d6605fe43a56823b7673b30d39c1d90e4110bd10f5685b0d0ee1e2fe0"
)

ENTRY = 0x2005
END = 0x2049
BODY_SHA256 = "8d9fa74f4ab6f3871cae437e93ce12ce82ef07bd4d5c2d183bee71d4bf7d28af"
WORK_POINTER_OFFSET = 0x0CB4
DIRECTORY_POINTER_OFFSET = 0x6AF0
SCRIPT_POINTER_OFFSET = 0x6AF4
DIRECTORY_RECORD_SIZE = 20
DIRECTORY_OBJECT_FIELD = 0x10
DIRECTORY_KIND_FIELD = 0x12
PROCEDURE_KIND = 2

GAME_SEGMENT = 0x2000
WORK_SEGMENT = 0x3000
SCRIPT_SEGMENT = 0x4000
DIRECTORY_SEGMENT = 0x5000
DATA_SEGMENT = 0x6000
INCOMING_ES_SEGMENT = 0x7000
INITIAL_FS_SEGMENT = 0x8000
DECOY_SEGMENT = 0x9000
STACK_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
DIRECTORY_OFFSET = 0x1200
SCRIPT_POINTER_BASE = 0x4100
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x2F00
RETURN_ADDRESS = RETURN_OFFSET
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
    "of": 0x0800,
}


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def physical_addresses(segment: int, offset: int, size: int) -> set[int]:
    return {segment * 16 + ((offset + index) & 0xFFFF) for index in range(size)}


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


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        entries = expected["directory_entries"]
        work_offset = expected["work_offset"]

        script_before = bytearray(
            (offset * 37 + (offset >> 8) * 11 + case_index * 23 + 5) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        emitted_records: list[tuple[int, int]] = []
        for entry_index, entry in enumerate(entries):
            object_offset = entry["object_offset"]
            entry_kind = entry["entry_kind"]
            absolute_value = (0x91 + case_index * 19 + entry_index * 29) & 0xFF
            relative_offset = (SCRIPT_POINTER_BASE + object_offset) & 0xFFFF
            script_before[object_offset] = absolute_value
            if relative_offset != object_offset:
                script_before[relative_offset] = absolute_value ^ 0xFF
            if entry_kind == PROCEDURE_KIND:
                emitted_records.append((object_offset, absolute_value))

        trailing_offset = 0x4567
        script_before[trailing_offset] = (0xD0 + case_index) & 0xFF

        directory_before = bytearray(
            (offset * 13 + case_index * 31 + 0x27) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        directory_cursor = DIRECTORY_OFFSET
        directory_rows = [
            *((entry["object_offset"], entry["entry_kind"]) for entry in entries),
            (0xFFFF, 0xA55A),
            (trailing_offset, PROCEDURE_KIND),
        ]
        for row_index, (object_offset, entry_kind) in enumerate(directory_rows):
            name_bytes = bytes(
                (0x41 + row_index + column + case_index) & 0x7F for column in range(16)
            )
            row = name_bytes + struct.pack("<HH", object_offset, entry_kind)
            write_wrapped(directory_before, directory_cursor, row)
            directory_cursor = (directory_cursor + DIRECTORY_RECORD_SIZE) & 0xFFFF

        work_before = bytes(
            (offset * 17 + case_index * 43 + 0x39) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        work_expected = bytearray(work_before)
        output_cursor = work_offset
        allowed_addresses: set[int] = set()
        for object_offset, value in emitted_records:
            record = struct.pack("<HB", object_offset, value)
            write_wrapped(work_expected, output_cursor, record)
            allowed_addresses |= physical_addresses(WORK_SEGMENT, output_cursor, 3)
            output_cursor = (output_cursor + 3) & 0xFFFF

        game_before = bytearray(seeded_segment(case_index, 19, 0x43))
        data_before = bytearray(seeded_segment(case_index, 23, 0x57))
        incoming_es_before = seeded_segment(case_index, 29, 0x69)
        initial_fs_before = seeded_segment(case_index, 31, 0x7D)
        decoy_before = seeded_segment(case_index, 37, 0x91)
        work_pointer = struct.pack("<HH", work_offset, WORK_SEGMENT)
        script_pointer = struct.pack("<HH", SCRIPT_POINTER_BASE, SCRIPT_SEGMENT)
        directory_pointer = struct.pack("<HH", DIRECTORY_OFFSET, DIRECTORY_SEGMENT)
        write_wrapped(game_before, WORK_POINTER_OFFSET, work_pointer)
        write_wrapped(game_before, SCRIPT_POINTER_OFFSET, script_pointer)
        write_wrapped(game_before, DIRECTORY_POINTER_OFFSET, directory_pointer)
        write_wrapped(
            data_before,
            WORK_POINTER_OFFSET,
            struct.pack("<HH", 0x3300, INCOMING_ES_SEGMENT),
        )
        write_wrapped(
            data_before,
            SCRIPT_POINTER_OFFSET,
            struct.pack("<HH", 0x4400, INITIAL_FS_SEGMENT),
        )
        write_wrapped(
            data_before,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", 0x5500, DECOY_SEGMENT),
        )

        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": INITIAL_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        stack_before = bytearray(seeded_segment(case_index, 41, 0xA7))
        write_wrapped(stack_before, CALLER_SP, struct.pack("<H", RETURN_OFFSET))
        write_wrapped(stack_before, CALLER_SP + 2, STACK_SENTINEL)
        stack_expected = bytearray(stack_before)
        for offset, value in (
            (CALLER_SP - 2, initial["es"]),
            (CALLER_SP - 4, initial["edi"]),
            (CALLER_SP - 6, initial["ds"]),
            (CALLER_SP - 8, initial["esi"]),
            (CALLER_SP - 10, initial["fs"]),
            (CALLER_SP - 12, initial["ecx"]),
            (CALLER_SP - 14, initial["ebp"]),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_addresses |= physical_addresses(STACK_SEGMENT, offset, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(WORK_SEGMENT * 16, work_before)
        machine.mem_write(SCRIPT_SEGMENT * 16, bytes(script_before))
        machine.mem_write(DIRECTORY_SEGMENT * 16, bytes(directory_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(INITIAL_FS_SEGMENT * 16, initial_fs_before)
        machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        reached_return: list[int] = []
        observed_write_addresses: set[int] = set()

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            reached: list[int] = reached_return,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
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
            allowed: set[int] = allowed_addresses,
            observed: set[int] = observed_write_addresses,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= allowed, hex(address)
            observed.update(addresses)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=2_000)

        byte_count = len(emitted_records) * 3
        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | byte_count
        actual_flags = snapshot_flags(machine)
        assert reached_return == [RETURN_ADDRESS], name
        assert observed_write_addresses == allowed_addresses, name
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert machine.reg_read(UC_X86_REG_EFLAGS) & 0x0600 == 0x0200
        assert actual_flags == expected["defined_flags"], name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(WORK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            work_expected
        )
        assert bytes(machine.mem_read(SCRIPT_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            script_before
        )
        assert bytes(machine.mem_read(DIRECTORY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            directory_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert (
            bytes(machine.mem_read(INITIAL_FS_SEGMENT * 16, SEGMENT_SIZE))
            == initial_fs_before
        )
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "directory_entries": entries,
                "script_pointer_offset_ignored": SCRIPT_POINTER_BASE,
                "work_offset": work_offset,
                "emitted_records": [
                    {"target_offset": object_offset, "value": value}
                    for object_offset, value in emitted_records
                ],
                "byte_count": byte_count,
                "work_sha256": hashlib.sha256(work_expected).hexdigest(),
                "defined_flags": actual_flags,
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
        raise SystemExit(f"BBB procedure patch-build body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_procedure_patch_build_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "work_pointer_offset": WORK_POINTER_OFFSET,
            "directory_pointer_offset": DIRECTORY_POINTER_OFFSET,
            "script_pointer_offset": SCRIPT_POINTER_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB procedure patch-build cases")


if __name__ == "__main__":
    main()
