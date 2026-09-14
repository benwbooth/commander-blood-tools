#!/usr/bin/env python3
"""Verify Big Bug Bang's bounded aboard-object roster rebuild."""

from __future__ import annotations

import argparse
import copy
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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_aboard_roster_rebuild.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_555b_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "a7c9fc0953ce7f894d0da3b9ebefebda330c87b650cd35c140a34c516bdbe235"
)

ENTRY = 0x59FF
END = 0x5A53
FIELD_HELPER_ENTRY = 0x6633
FIELD_HELPER_END = 0x6644
BODY_SHA256 = "6f0ef3f76105243244fb406b17669d2e995278e82a92647d2eccc502131e2d4c"
DIRECTORY_POINTER_OFFSET = 0x6AF0
STATE_POINTER_OFFSET = 0x6AEC
ROSTER_OFFSET = 0x70E6
FIELD_MATRIX_OFFSET = 0x7128
COMMANDER_DIRECTORY_POINTER_OFFSET = 0x672C
COMMANDER_STATE_POINTER_OFFSET = 0x6724
COMMANDER_FIELD_MATRIX_OFFSET = 0x6D60
HOLDER_SELECTOR = 0x11
DIRECTORY_RECORD_SIZE = 20
DIRECTORY_OBJECT_FIELD = 0x10
DIRECTORY_KIND_FIELD = 0x12
ACTIVE_DIRECTORY_KIND = 1
ROSTER_CAPACITY = 16
TERMINATOR = 0xFFFF

INCOMING_ES_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
GAME_SEGMENT = 0x4000
DIRECTORY_SEGMENT = 0x5000
RECORD_SEGMENT = 0x6000
DECOY_DIRECTORY_SEGMENT = 0x8000
DECOY_RECORD_SEGMENT = 0x9000
STACK_SEGMENT = 0xA000
INITIAL_FS_SEGMENT = 0xB000
SEGMENT_SIZE = 0x10000
MEMORY_SIZE = 0x100000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x1800
RETURN_ADDRESS = RETURN_OFFSET
FIELD_RETURN_OFFSET = 0x5A2D
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
    "if": 0x0200,
    "df": 0x0400,
    "of": 0x0800,
}


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_physical(memory: bytearray, segment: int, offset: int, data: bytes) -> None:
    address = segment * 16 + offset
    memory[address : address + len(data)] = data


def write_wrapped(memory: bytearray, segment: int, offset: int, data: bytes) -> None:
    base = segment * 16
    for index, value in enumerate(data):
        memory[base + ((offset + index) & 0xFFFF)] = value


def sub16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
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


def snapshot_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def build_cases() -> list[dict[str, Any]]:
    cases = copy.deepcopy(commander_vectors())
    inherited = cases[-1]
    assert inherited["name"] == "inherited_esi_high_word_participates_in_field_address"
    inherited.update(
        name="inherited_esi_high_word_is_cleared",
        entries=[
            {
                "object_offset": 0x0500,
                "entry_kind": 1,
                "kind_mask": 0x0001,
                "field_offset": 5,
                "field_value": 0,
                "high_word_decoy": -1,
            },
            {
                "object_offset": 0x0740,
                "entry_kind": 2,
                "kind_mask": 0x0002,
                "field_offset": 21,
                "field_value": -1,
            },
        ],
        scanned_entries=1,
        slots_after=inherited["slots_before"],
        stop_reason="directory_kind",
    )

    capacity_entries = [
        {
            "object_offset": 0x0200 + index * 0x40,
            "entry_kind": 1,
            "kind_mask": 0x0002,
            "field_offset": 24,
            "field_value": -1,
        }
        for index in range(ROSTER_CAPACITY + 1)
    ]
    cases.append(
        {
            "name": "sixteen_matches_stop_at_capacity",
            "directory_offset": 0x1800,
            "record_pointer_offset_ignored": 0x3500,
            "inherited_esi_high_word": 0xA5A5,
            "entries": capacity_entries,
            "slots_before": [0x7000 + index for index in range(ROSTER_CAPACITY)],
            "typed_capacity_error": True,
        }
    )
    nonmatch_entries = [
        {
            "object_offset": 0x0800 + index * 0x40,
            "entry_kind": 1,
            "kind_mask": 0x0002,
            "field_offset": 24,
            "field_value": index,
        }
        for index in range(ROSTER_CAPACITY + 1)
    ]
    nonmatch_entries.append(
        {
            "object_offset": 0x0D00,
            "entry_kind": 0,
            "kind_mask": 0x0002,
            "field_offset": 24,
            "field_value": -1,
        }
    )
    cases.append(
        {
            "name": "seventeen_nonmatches_do_not_consume_capacity",
            "directory_offset": 0x1C00,
            "record_pointer_offset_ignored": 0x3700,
            "inherited_esi_high_word": 0x5A5A,
            "entries": nonmatch_entries,
            "slots_before": [0x7100 + index for index in range(ROSTER_CAPACITY)],
            "typed_capacity_error": False,
        }
    )
    return cases


def model_case(case: dict[str, Any]) -> dict[str, Any]:
    slots_before = list(case["slots_before"])
    slots_before.extend(
        0x6000 + index for index in range(len(slots_before), ROSTER_CAPACITY)
    )
    assert len(slots_before) == ROSTER_CAPACITY
    assert all(
        value != TERMINATOR for value in slots_before[len(case["slots_before"]) :]
    )
    slots_after = list(slots_before)
    selected_offsets: list[int] = []
    scanned_entries = 0
    remaining = ROSTER_CAPACITY
    stop_reason = ""
    terminal_left = 0
    terminal_right = 0

    for entry in case["entries"]:
        entry_kind = int(entry["entry_kind"])
        if entry_kind != ACTIVE_DIRECTORY_KIND:
            stop_reason = "directory_kind"
            terminal_left = entry_kind
            terminal_right = ACTIVE_DIRECTORY_KIND
            break
        scanned_entries += 1
        if int(entry["field_value"]) != -1:
            continue
        destination = len(selected_offsets)
        slots_after[destination] = int(entry["object_offset"])
        selected_offsets.append(int(entry["object_offset"]))
        remaining -= 1
        if remaining == 0:
            stop_reason = "capacity"
            terminal_left = 1
            terminal_right = 1
            break
        if slots_after[len(selected_offsets)] == TERMINATOR:
            stop_reason = "slot_sentinel"
            terminal_left = TERMINATOR
            terminal_right = TERMINATOR
            break
    if not stop_reason:
        raise AssertionError(f"{case['name']}: case does not terminate")

    flags = sub16_flags(terminal_left, terminal_right)
    if stop_reason == "capacity":
        flags["cf"] = False
    return {
        "slots_before": slots_before,
        "slots_after": slots_after,
        "selected_offsets": selected_offsets,
        "scanned_entries": scanned_entries,
        "stop_reason": stop_reason,
        "defined_flags": flags,
    }


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, case in enumerate(build_cases()):
        name = str(case["name"])
        model = model_case(case)
        directory_offset = int(case["directory_offset"])
        record_pointer_offset = int(case["record_pointer_offset_ignored"])
        entries = case["entries"]
        inherited_esi_high = int(case.get("inherited_esi_high_word", 0))

        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": ((inherited_esi_high & 0xFFFF) << 16) | 0x5678,
            "edi": 0xF6F66789,
            "ebp": 0xA7A7789A,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": INITIAL_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        memory = bytearray(MEMORY_SIZE)
        memory[: len(executable)] = executable
        memory[RETURN_ADDRESS] = 0xCC
        for segment, multiplier, salt in (
            (INCOMING_ES_SEGMENT, 11, 0x13),
            (DATA_SEGMENT, 13, 0x21),
            (GAME_SEGMENT, 17, 0x35),
            (DIRECTORY_SEGMENT, 19, 0x47),
            (DECOY_DIRECTORY_SEGMENT, 23, 0x59),
            (DECOY_RECORD_SEGMENT, 29, 0x6B),
            (STACK_SEGMENT, 31, 0x7D),
            (INITIAL_FS_SEGMENT, 37, 0x8F),
        ):
            write_physical(
                memory, segment, 0, seeded_segment(case_index, multiplier, salt)
            )
        write_physical(
            memory,
            RECORD_SEGMENT,
            0,
            seeded_segment(case_index, 41, 0xA1) + seeded_segment(case_index, 43, 0xB3),
        )

        write_wrapped(
            memory,
            GAME_SEGMENT,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", directory_offset, DIRECTORY_SEGMENT),
        )
        write_wrapped(
            memory,
            GAME_SEGMENT,
            STATE_POINTER_OFFSET,
            struct.pack("<HH", record_pointer_offset, RECORD_SEGMENT),
        )
        write_wrapped(
            memory,
            DATA_SEGMENT,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", 0x2200, DECOY_DIRECTORY_SEGMENT),
        )
        write_wrapped(
            memory,
            DATA_SEGMENT,
            STATE_POINTER_OFFSET,
            struct.pack("<HH", 0x2600, DECOY_RECORD_SEGMENT),
        )
        write_wrapped(
            memory,
            GAME_SEGMENT,
            COMMANDER_DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", 0x2A00, DECOY_DIRECTORY_SEGMENT),
        )
        write_wrapped(
            memory,
            GAME_SEGMENT,
            COMMANDER_STATE_POINTER_OFFSET,
            struct.pack("<HH", 0x2E00, DECOY_RECORD_SEGMENT),
        )
        write_wrapped(
            memory,
            STACK_SEGMENT,
            ROSTER_OFFSET,
            struct.pack(f"<{ROSTER_CAPACITY}H", *model["slots_before"]),
        )
        write_wrapped(
            memory,
            STACK_SEGMENT,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )

        for index, entry in enumerate(entries):
            object_offset = int(entry["object_offset"])
            entry_kind = int(entry["entry_kind"])
            kind_mask = int(entry["kind_mask"])
            field_offset = int(entry["field_offset"])
            field_value = int(entry["field_value"])
            entry_offset = (directory_offset + index * DIRECTORY_RECORD_SIZE) & 0xFFFF
            write_wrapped(
                memory,
                DIRECTORY_SEGMENT,
                entry_offset + DIRECTORY_OBJECT_FIELD,
                struct.pack("<H", object_offset),
            )
            write_wrapped(
                memory,
                DIRECTORY_SEGMENT,
                entry_offset + DIRECTORY_KIND_FIELD,
                struct.pack("<H", entry_kind),
            )
            write_wrapped(
                memory,
                RECORD_SEGMENT,
                object_offset,
                struct.pack("<H", kind_mask),
            )
            bit_index = (kind_mask & -kind_mask).bit_length() - 1
            matrix_offset = FIELD_MATRIX_OFFSET + HOLDER_SELECTOR * 16 + bit_index
            write_wrapped(
                memory, GAME_SEGMENT, matrix_offset, bytes([field_offset & 0xFF])
            )
            commander_matrix = (
                COMMANDER_FIELD_MATRIX_OFFSET + HOLDER_SELECTOR * 16 + bit_index
            )
            write_wrapped(
                memory,
                GAME_SEGMENT,
                commander_matrix,
                bytes([(field_offset + 1) & 0xFF]),
            )
            signed_offset = (
                field_offset if field_offset >= 0 else field_offset + (1 << 32)
            )
            effective_offset = (object_offset + signed_offset) & 0xFFFFFFFF
            write_physical(
                memory,
                RECORD_SEGMENT,
                effective_offset,
                struct.pack("<H", field_value & 0xFFFF),
            )
            if "high_word_decoy" in entry:
                write_physical(
                    memory,
                    RECORD_SEGMENT,
                    effective_offset + 0x10000,
                    struct.pack("<H", int(entry["high_word_decoy"]) & 0xFFFF),
                )
            if name == "field_address_crosses_64k_without_wrap":
                write_wrapped(memory, RECORD_SEGMENT, effective_offset, b"\0\0")
                write_physical(
                    memory,
                    RECORD_SEGMENT,
                    effective_offset,
                    struct.pack("<H", field_value & 0xFFFF),
                )
            write_wrapped(
                memory,
                RECORD_SEGMENT,
                record_pointer_offset + object_offset,
                struct.pack("<H", kind_mask ^ 0xFFFF),
            )
            write_wrapped(
                memory,
                DECOY_RECORD_SEGMENT,
                object_offset,
                struct.pack("<H", kind_mask ^ 0x5A5A),
            )

        expected_events: list[dict[str, Any]] = []

        def event(kind: str, offset: int, value: int) -> None:
            expected_events.append(
                {
                    "kind": kind,
                    "offset": offset & 0xFFFF,
                    "size": 2,
                    "value": value & 0xFFFF,
                }
            )

        for index, (kind, value) in enumerate(
            (
                ("stack", initial["eax"]),
                ("stack", initial["ebx"]),
                ("stack", initial["es"]),
                ("stack", initial["edi"]),
                ("stack", initial["ds"]),
                ("stack", initial["esi"]),
                ("stack", initial["ebp"]),
                ("stack", initial["ecx"]),
            ),
            start=1,
        ):
            event(kind, CALLER_SP - index * 2, value)
        selected_index = 0
        for entry in entries[: model["scanned_entries"]]:
            event("stack", CALLER_SP - 18, FIELD_RETURN_OFFSET)
            event("stack", CALLER_SP - 20, int(entry["kind_mask"]))
            if int(entry["field_value"]) == -1:
                event(
                    "roster",
                    ROSTER_OFFSET + selected_index * 2,
                    int(entry["object_offset"]),
                )
                selected_index += 1

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, MEMORY_SIZE)
        machine.mem_write(0, executable)
        machine.mem_write(0, bytes(memory))
        memory_before = bytes(machine.mem_read(0, MEMORY_SIZE))
        expected_memory = bytearray(memory_before)
        for item in expected_events:
            write_wrapped(
                expected_memory,
                STACK_SEGMENT,
                int(item["offset"]),
                int(item["value"]).to_bytes(int(item["size"]), "little"),
            )

        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        inherited_df = bool(case_index & 1)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202 | (0x0400 if inherited_df else 0))

        reached_return: list[int] = []
        field_calls: list[dict[str, int]] = []
        actual_events: list[dict[str, Any]] = []

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert (
                ENTRY <= address < END
                or FIELD_HELPER_ENTRY <= address < FIELD_HELPER_END
            ), hex(address)
            if address == FIELD_HELPER_ENTRY:
                field_calls.append(
                    {
                        "selector": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                        "kind_mask": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "state_offset": cpu.reg_read(UC_X86_REG_ESI),
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "gs": cpu.reg_read(UC_X86_REG_GS),
                        "sp": cpu.reg_read(UC_X86_REG_SP),
                    }
                )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
        ) -> None:
            stack_base = STACK_SEGMENT * 16
            assert stack_base <= address < stack_base + SEGMENT_SIZE, hex(address)
            offset = address - stack_base
            kind = "roster" if ROSTER_OFFSET <= offset < ROSTER_OFFSET + 32 else "stack"
            actual_events.append(
                {"kind": kind, "offset": offset, "size": size, "value": value}
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=5_000)

        expected_registers = dict(initial)
        expected_registers["esi"] = initial["esi"] & 0xFFFF
        if model["scanned_entries"]:
            last_field_offset = int(
                entries[model["scanned_entries"] - 1]["field_offset"]
            )
            high = 0xFFFF if last_field_offset < 0 else 0
            expected_registers["eax"] = (high << 16) | (initial["eax"] & 0xFFFF)
        expected_flags = dict(
            model["defined_flags"], **{"if": True, "df": inherited_df}
        )
        assert reached_return == [RETURN_ADDRESS], name
        assert actual_events == expected_events, name
        assert snapshot_registers(machine) == expected_registers, name
        assert snapshot_flags(machine) == expected_flags, name
        assert machine.reg_read(UC_X86_REG_CS) == 0, name
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, name
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4, name
        assert bytes(machine.mem_read(0, MEMORY_SIZE)) == bytes(expected_memory), name
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 4, len(STACK_SENTINEL)
                )
            )
            == STACK_SENTINEL
        ), name

        expected_calls = [
            {
                "selector": HOLDER_SELECTOR,
                "kind_mask": int(entry["kind_mask"]),
                "state_offset": int(entry["object_offset"]),
                "ds": RECORD_SEGMENT,
                "gs": GAME_SEGMENT,
                "sp": CALLER_SP - 18,
            }
            for entry in entries[: model["scanned_entries"]]
        ]
        assert field_calls == expected_calls, name

        rows.append(
            {
                "name": name,
                "directory_offset": directory_offset,
                "record_pointer_offset_ignored": record_pointer_offset,
                "inherited_esi_high_word": inherited_esi_high,
                "entries": entries,
                "scanned_entries": model["scanned_entries"],
                "selected_offsets": model["selected_offsets"],
                "slots_before": model["slots_before"],
                "slots_after": model["slots_after"],
                "stop_reason": model["stop_reason"],
                "typed_capacity_error": bool(case.get("typed_capacity_error", False)),
                "field_calls": field_calls,
                "ordered_write_events": actual_events,
                "registers_after": expected_registers,
                "defined_flags": expected_flags,
                "return": "far",
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
        raise SystemExit(f"BBB aboard-roster body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    for commander, sequel in zip(commander_vectors()[:9], rows[:9], strict=True):
        assert sequel["name"] == commander["name"]
        assert sequel["scanned_entries"] == commander["scanned_entries"]
        assert (
            sequel["slots_before"][: len(commander["slots_before"])]
            == commander["slots_before"]
        )
        assert (
            sequel["slots_after"][: len(commander["slots_after"])]
            == commander["slots_after"]
        )
        assert sequel["stop_reason"] == commander["stop_reason"]
        assert sequel["registers_after"]["eax"] == commander["eax"]
        assert {
            flag: sequel["defined_flags"][flag]
            for flag in ("cf", "pf", "af", "zf", "sf", "of")
        } == commander["defined_flags"]
    result = {
        "format": "big_bug_bang_aboard_roster_rebuild_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "directory_pointer_offset": DIRECTORY_POINTER_OFFSET,
            "state_pointer_offset": STATE_POINTER_OFFSET,
            "roster_offset": ROSTER_OFFSET,
            "field_matrix_offset": FIELD_MATRIX_OFFSET,
            "capacity": ROSTER_CAPACITY,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB aboard-roster rebuild cases")


if __name__ == "__main__":
    main()
