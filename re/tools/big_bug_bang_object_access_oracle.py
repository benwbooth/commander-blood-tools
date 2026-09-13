#!/usr/bin/env python3
"""Verify BBB's relocated object-access counter update."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_object_access.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_149b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "fd2c16c00dcebd8453dcb7cf2c38be52f3391b7cb3719cf75edb66b425c374fe"
)

ENTRY = 0x1659
END = 0x1688
BODY_SHA256 = "bf28e15ef604cfd2e4a4338f75ba41ca3114a99137ee7154210a9ef2f9392bcc"
OBJECT_POINTER_OFFSET = 0x6AEC
OBJECT_SEGMENT_OFFSET = 0x6AEE
DIRECTORY_POINTER_OFFSET = 0x6AF0
DIRECTORY_RECORD_SIZE = 0x14
OBJECT_OFFSET_FIELD = 0x10
DIRECTORY_KIND_FIELD = 0x12
OBJECT_FLAGS_OFFSET = 0x02
OBJECT_ACCESS_COUNT_OFFSET = 0x14
ACCESS_KIND_MASK = 0x0118
IN_PLAY_FLAG = 0x02

INCOMING_ES_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
GAME_DECOY_SEGMENT = 0x4000
DIRECTORY_SEGMENT = 0x5000
FS_SEGMENT = 0x6000
OBJECT_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_SEGMENT = 0
RETURN_OFFSET = 0x2800
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
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
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}
CASES = (
    (
        "first_entry_kind_ignored",
        0x0200,
        0x3100,
        (
            (0x0100, 0x7777, 0x0008, 0x02, 0x10, 0x00),
            (0x0200, 0x0000, 0x0118, 0x02, 0x20, 0x00),
        ),
    ),
    (
        "kind_mask_required",
        0x0400,
        0x3200,
        (
            (0x0300, 0x0001, 0x0200, 0x02, 0x30, 0x00),
            (0x0400, 0x0002, 0x0118, 0x02, 0x40, 0x00),
        ),
    ),
    (
        "low_flag_byte_required",
        0x0600,
        0x3300,
        (
            (0x0500, 0x0001, 0x0118, 0x00, 0x50, 0x02),
            (0x0600, 0x8000, 0x0118, 0x02, 0x60, 0x00),
        ),
    ),
    (
        "all_mask_bits_and_counter_wrap",
        0x0800,
        0x3400,
        (
            (0x1000, 0x2222, 0x0008, 0x02, 0x10, 0xA1),
            (0x1100, 0x0001, 0x0010, 0x02, 0xFE, 0xA2),
            (0x1200, 0x0001, 0x0100, 0x03, 0xFF, 0xA3),
            (0x1300, 0x0001, 0x0118, 0x01, 0x40, 0xA4),
            (0x1400, 0xFFFF, 0x0118, 0x02, 0x50, 0xA5),
        ),
    ),
    (
        "directory_and_object_offsets_wrap",
        0xFFF0,
        0x3500,
        (
            (0xFFF8, 0x3333, 0x0118, 0x02, 0x7F, 0xB1),
            (0x0020, 0x0001, 0x0008, 0x02, 0x80, 0xB2),
            (0x0040, 0x1234, 0x0118, 0x02, 0x90, 0xB3),
        ),
    ),
    (
        "duplicate_object_is_incremented_twice",
        0x0A00,
        0x3600,
        (
            (0x2000, 0x4444, 0x0118, 0x02, 0xFE, 0xC1),
            (0x2000, 0x0001, 0x0118, 0x02, 0xFE, 0xC1),
            (0x2200, 0x0002, 0x0118, 0x02, 0x22, 0xC2),
        ),
    ),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


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
        directory_offset,
        heap_base_offset,
        entries,
    ) in enumerate(CASES):
        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0xA7A7789A + case_index,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 17, 0x21)
        game_decoy_before = seeded_segment(case_index, 19, 0x31)
        directory_before = seeded_segment(case_index, 23, 0x43)
        object_before = seeded_segment(case_index, 29, 0x59)
        incoming_es_before = bytes(seeded_segment(case_index, 31, 0x67))
        fs_before = bytes(seeded_segment(case_index, 37, 0x71))
        write_wrapped(
            data_before,
            OBJECT_POINTER_OFFSET,
            struct.pack("<HH", heap_base_offset, OBJECT_SEGMENT),
        )
        write_wrapped(
            data_before,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", directory_offset, DIRECTORY_SEGMENT),
        )
        write_wrapped(
            game_decoy_before,
            OBJECT_POINTER_OFFSET,
            struct.pack("<HH", 0x1111, 0x2222),
        )
        write_wrapped(
            game_decoy_before,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", 0x3333, 0x4444),
        )

        object_fields: dict[int, tuple[int, int, int, int]] = {}
        for entry_index, (
            object_offset,
            entry_kind,
            object_kind,
            flags,
            access_count,
            flag_neighbor,
        ) in enumerate(entries):
            entry_offset = (
                directory_offset + entry_index * DIRECTORY_RECORD_SIZE
            ) & 0xFFFF
            write_wrapped(
                directory_before,
                entry_offset + OBJECT_OFFSET_FIELD,
                struct.pack("<H", object_offset),
            )
            write_wrapped(
                directory_before,
                entry_offset + DIRECTORY_KIND_FIELD,
                struct.pack("<H", entry_kind),
            )
            state = (object_kind, flags, access_count, flag_neighbor)
            previous = object_fields.setdefault(object_offset, state)
            assert previous == state, name

        for object_offset, (
            object_kind,
            flags,
            access_count,
            flag_neighbor,
        ) in object_fields.items():
            write_wrapped(object_before, object_offset, struct.pack("<H", object_kind))
            write_wrapped(
                object_before,
                object_offset + OBJECT_FLAGS_OFFSET,
                bytes((flags, flag_neighbor)),
            )
            write_wrapped(
                object_before,
                object_offset + OBJECT_ACCESS_COUNT_OFFSET,
                bytes((access_count,)),
            )
            write_wrapped(
                object_before,
                heap_base_offset + object_offset + OBJECT_ACCESS_COUNT_OFFSET,
                bytes(((0xD0 + len(rows)) & 0xFF,)),
            )

        processed_entries = next(
            index for index in range(1, len(entries)) if entries[index][1] != 1
        )
        expected_counts = {
            object_offset: state[2] for object_offset, state in object_fields.items()
        }
        allowed_object_addresses: set[int] = set()
        for object_offset, _, object_kind, flags, _, _ in entries[:processed_entries]:
            if object_kind & ACCESS_KIND_MASK and flags & IN_PLAY_FLAG:
                expected_counts[object_offset] = (
                    expected_counts[object_offset] + 1
                ) & 0xFF
                allowed_object_addresses.add(
                    OBJECT_SEGMENT * 16
                    + ((object_offset + OBJECT_ACCESS_COUNT_OFFSET) & 0xFFFF)
                )
        object_expected = bytearray(object_before)
        for object_offset, expected_count in expected_counts.items():
            write_wrapped(
                object_expected,
                object_offset + OBJECT_ACCESS_COUNT_OFFSET,
                bytes((expected_count,)),
            )

        stack_before = seeded_segment(case_index, 41, 0x83)
        write_wrapped(stack_before, CALLER_SP, struct.pack("<H", RETURN_OFFSET))
        write_wrapped(stack_before, CALLER_SP + 2, STACK_SENTINEL)
        stack_expected = bytearray(stack_before)
        write_wrapped(stack_expected, CALLER_SP - 2, struct.pack("<H", initial["es"]))
        write_wrapped(
            stack_expected,
            CALLER_SP - 4,
            struct.pack("<H", initial["edi"] & 0xFFFF),
        )
        write_wrapped(stack_expected, CALLER_SP - 6, struct.pack("<H", initial["ds"]))
        write_wrapped(
            stack_expected,
            CALLER_SP - 8,
            struct.pack("<H", initial["esi"] & 0xFFFF),
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_DECOY_SEGMENT * 16, bytes(game_decoy_before))
        machine.mem_write(DIRECTORY_SEGMENT * 16, bytes(directory_before))
        machine.mem_write(OBJECT_SEGMENT * 16, bytes(object_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93 | (0x0400 if case_index & 1 else 0))

        reached_return: list[int] = []
        allowed_addresses = allowed_object_addresses | set(
            range(STACK_SEGMENT * 16 + CALLER_SP - 8, STACK_SEGMENT * 16 + CALLER_SP)
        )

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
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
            assert set(range(address, address + size)) <= allowed_addresses, hex(
                address
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=500)

        assert reached_return == [RETURN_ADDRESS], name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(GAME_DECOY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_decoy_before
        )
        assert bytes(machine.mem_read(DIRECTORY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            directory_before
        )
        assert bytes(machine.mem_read(OBJECT_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            object_expected
        )
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        terminal_kind = entries[processed_entries][1]
        rows.append(
            {
                "name": name,
                "directory_offset": directory_offset,
                "heap_base_offset_ignored": heap_base_offset,
                "processed_entries": processed_entries,
                "entries": [
                    {
                        "object_offset": object_offset,
                        "entry_kind": entry_kind,
                        "object_kind": object_kind,
                        "flags": flags,
                        "access_count_before": access_count,
                        "access_count_after": expected_counts[object_offset],
                    }
                    for (
                        object_offset,
                        entry_kind,
                        object_kind,
                        flags,
                        access_count,
                        _flag_neighbor,
                    ) in entries
                ],
                "terminal_kind": terminal_kind,
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
        raise SystemExit(f"BBB object-access body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    verify_commander_semantics(rows)
    result = {
        "format": "big_bug_bang_object_access_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "object_pointer_offset": OBJECT_POINTER_OFFSET,
            "object_segment_offset": OBJECT_SEGMENT_OFFSET,
            "directory_pointer_offset": DIRECTORY_POINTER_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB object-access cases")


if __name__ == "__main__":
    main()
