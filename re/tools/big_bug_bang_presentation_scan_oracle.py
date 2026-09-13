#!/usr/bin/env python3
"""Execute Big Bug Bang's post-frame presentation scan.

The complete 0x5DD7 scanner and real 0x6633 field-offset helper execute
unmodified. Established BAS, action, renderer, resource, and pre-frame state
processor boundaries are captured. The resulting cases are also checked
against the equivalent Commander Blood native scan vectors after normalizing
relocated fields and the sequel-only state-processor call.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
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


EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
SCAN = (0x5DD7, 0x6038)
FIELD_HELPER = (0x6633, 0x6644)
SCAN_SHA256 = "70fcc62d9754643188da0e0c115aaf8a7f25778159b6b9a0caa82d11e15fef12"
FIELD_HELPER_SHA256 = "fb7ec0e721e99c38e166f3c8538a44a18b99e12810b183c3b7659b660f955520"

CODE_SEGMENT = 0x502
GLOBALS = 0x30000
STATE = 0x50000
DIRECTORY = 0x70000
HISTORY = 0x80000
STACK = 0x90000
DECOY = 0xA0000
RESOURCE = 0xB0000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
RETURN_LINEAR = CODE_SEGMENT * 16 + RETURN_IP
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

DIRECTORY_OFFSET = 0x1800
HISTORY_OFFSET = 0x2200
RESOURCE_OFFSET = 0x3000
PRIMARY_OFFSET = 0x3000
ARCHE_OFFSET = 0x3200
WILDCARD = 0x2400
FIELD_MATRIX = 0x7128
FIELD_MATRIX_FILE = 0x16918
FIELD_MATRIX_SIZE = 21 * 16

GLOBALS_OFFSETS = {
    "state_pointer": 0x6AEC,
    "directory_pointer": 0x6AF0,
    "history_pointer": 0x6B16,
    "wildcard": 0x6B1E,
    "arche": 0x6B22,
    "primary": 0x6B30,
    "deferred_type": 0x6B3A,
    "deferred_related": 0x6B3C,
    "deferred_value": 0x6B3E,
    "presentation_active": 0x6B82,
    "pair_guard": 0x6B8C,
    "start_lock": 0x6B8D,
    "c2_gate": 0x2200,
    "word_choice": 0x2A77,
    "ui": 0x2A33,
    "effect_active": 0x2A89,
    "resource_pointer": 0x0C78,
}

EXTERNALS = {
    0x5BAE: ("control", "near"),
    0x6038: ("state_processor", "near"),
    0x613F: ("action", "near"),
    0x8450: ("descript", "far"),
    0x4444: ("resource", "far"),
    0x45CB: ("setter", "far"),
    0x464E: ("transition", "far"),
}

CASES = (
    {
        "name": "inactive_first_entry_only_clears_pair_write_guard",
        "entries": [{"kind": 1, "flags": 0, "record_kind": 0xC4}],
    },
    {
        "name": "kind2_handoff_then_action",
        "active": 1,
        "entries": [
            {"kind": 2, "record_kind": 0xC4, "record_value": 3, "target": 0x3456}
        ],
    },
    {
        "name": "kind2_blocked_owner_still_runs_action",
        "active": 1,
        "entries": [
            {
                "kind": 2,
                "flags": 0x8001,
                "record_kind": 0xC4,
                "record_value": 4,
                "target": 0x4567,
            }
        ],
    },
    {
        "name": "kind2_negative_value_suppresses_action",
        "active": 1,
        "entries": [
            {
                "kind": 2,
                "record_kind": 0xC4,
                "record_value": 0x8000,
                "target": 0,
            }
        ],
    },
    {
        "name": "kind1_starts_presentation_without_name_lookup",
        "entries": [
            {
                "kind": 1,
                "record_kind": 0xC4,
                "record_value": 5,
                "related_flags": 0x0021,
            }
        ],
    },
    {
        "name": "kind1_start_runs_descript_effect_chain",
        "ui": 0x0101,
        "effect_active": 1,
        "entries": [
            {
                "kind": 1,
                "record_kind": 0xC4,
                "record_value": 6,
                "related_flags": 0x0001,
            }
        ],
    },
    {
        "name": "kind1_active_c4_drains_ordinary_deferred_record",
        "active": 1,
        "deferred": (0xC2, 0x4A4A, 0x1234),
        "entries": [
            {
                "kind": 1,
                "record_kind": 0xC4,
                "record_value": 7,
                "related_flags": 0x0020,
            }
        ],
    },
    {
        "name": "kind1_teardown_clears_history_before_action",
        "active": 1,
        "entries": [{"kind": 1, "record_kind": 0xC2, "record_value": 8}],
    },
    {
        "name": "kind1_c1_deferred_targets_arche_ship_field",
        "active": 1,
        "deferred": (0xC1, 0x5151, 0x9999),
        "entries": [{"kind": 1, "record_kind": 0xC4, "record_value": 9}],
    },
    {
        "name": "kind1_c6_deferred_targets_arche_ship_field",
        "active": 1,
        "deferred": (0xC6, 0x6161, 0x8888),
        "entries": [{"kind": 1, "record_kind": 0xC4, "record_value": 10}],
    },
    {
        "name": "deferred_negative_value_is_tested_after_overwrite",
        "active": 1,
        "deferred": (0xC2, 0x7171, 0xFFFF),
        "entries": [{"kind": 1, "record_kind": 0xC4, "record_value": 11}],
    },
    {
        "name": "deferred_related_without_type_is_retained",
        "active": 1,
        "deferred": (0, 0x7272, 0x9999),
        "entries": [{"kind": 1, "record_kind": 0xC4, "record_value": 17}],
    },
    {
        "name": "deferred_type_without_related_is_retained",
        "active": 1,
        "deferred": (0xC2, 0, 0x8888),
        "entries": [{"kind": 1, "record_kind": 0xC4, "record_value": 18}],
    },
    {
        "name": "ship_and_special_entries_both_run_action",
        "entries": [
            {
                "kind": 0x10,
                "directory_kind": 7,
                "record_kind": 0xC1,
                "record_value": 12,
            },
            {
                "kind": 0x200,
                "directory_kind": 1,
                "record_kind": 0xC6,
                "record_value": 13,
            },
        ],
    },
    {
        "name": "next_directory_kind_must_equal_full_word_one",
        "entries": [
            {"kind": 0x10, "record_kind": 0xC1, "record_value": 14},
            {
                "kind": 0x200,
                "directory_kind": 0xAB01,
                "record_kind": 0xC6,
                "record_value": 15,
            },
        ],
    },
    {
        "name": "unknown_active_kind_has_no_action",
        "entries": [{"kind": 0x40, "record_kind": 0xC4, "record_value": 16}],
    },
)


def put_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def put_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def field_offset(executable: bytes, selector: int, kind: int) -> int:
    column = (kind & -kind).bit_length() - 1
    assert 0 <= selector < 21 and 0 <= column < 16
    return executable[FIELD_MATRIX_FILE + selector * 16 + column]


def near_return(cpu: Uc) -> None:
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip = struct.unpack("<H", cpu.mem_read(stack, 2))[0]
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 2) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def far_return(cpu: Uc) -> None:
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(stack, 4))
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def changes(before: bytes | bytearray, after: bytes) -> list[list[int]]:
    return [
        [offset, old, new]
        for offset, (old, new) in enumerate(zip(before, after, strict=True))
        if old != new
    ]


def normalized_calls(calls: list[dict[str, object]]) -> list[list[object]]:
    normalized = []
    for call in calls:
        name = str(call["name"])
        if name in ("field", "state_processor"):
            continue
        if name == "control":
            normalized.append([name, call["object"], call["target"]])
        elif name == "action":
            normalized.append([name, call["object"], call["triple"]])
        elif name == "resource":
            normalized.append([name, call["resource_id"]])
        elif name == "setter":
            normalized.append(
                [name, call["entity"], call["x"], call["y"], call["frame"]]
            )
        elif name == "transition":
            normalized.append([name, call["object_id"]])
        elif name == "descript":
            normalized.append([name])
        else:
            raise AssertionError(f"unknown call {call!r}")
    return normalized


def initial_images(executable: bytes, case: dict[str, object], case_index: int):
    globals_before = bytearray(
        (offset * 7 + (offset >> 8) * 11 + case_index * 17 + 0x21) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    state_before = bytearray(
        (offset * 13 + (offset >> 8) * 19 + case_index * 23 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    directory_before = bytearray(
        (offset * 5 + (offset >> 8) * 29 + case_index * 31 + 0x65) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    history_before = bytearray(
        (offset * 17 + case_index * 37 + 0x87) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    globals_before[FIELD_MATRIX : FIELD_MATRIX + FIELD_MATRIX_SIZE] = executable[
        FIELD_MATRIX_FILE : FIELD_MATRIX_FILE + FIELD_MATRIX_SIZE
    ]
    put_dword(
        globals_before,
        GLOBALS_OFFSETS["state_pointer"],
        (STATE // 16) << 16 | 0x5555,
    )
    put_dword(
        globals_before,
        GLOBALS_OFFSETS["directory_pointer"],
        (DIRECTORY // 16) << 16 | DIRECTORY_OFFSET,
    )
    put_dword(
        globals_before,
        GLOBALS_OFFSETS["history_pointer"],
        (HISTORY // 16) << 16 | HISTORY_OFFSET,
    )
    put_dword(
        globals_before,
        GLOBALS_OFFSETS["resource_pointer"],
        (RESOURCE // 16) << 16 | RESOURCE_OFFSET,
    )
    put_word(globals_before, GLOBALS_OFFSETS["wildcard"], WILDCARD)
    put_word(globals_before, GLOBALS_OFFSETS["arche"], ARCHE_OFFSET)
    put_word(globals_before, GLOBALS_OFFSETS["primary"], PRIMARY_OFFSET)
    deferred = tuple(case.get("deferred", (0, 0, 0)))
    for name, value in zip(
        ("deferred_type", "deferred_related", "deferred_value"),
        deferred,
        strict=True,
    ):
        put_word(globals_before, GLOBALS_OFFSETS[name], int(value))
    globals_before[GLOBALS_OFFSETS["pair_guard"]] = 0xA6
    globals_before[GLOBALS_OFFSETS["presentation_active"]] = int(case.get("active", 0))
    globals_before[GLOBALS_OFFSETS["c2_gate"]] = int(case.get("c2_gate", 0))
    globals_before[GLOBALS_OFFSETS["word_choice"]] = int(case.get("word_choice", 0))
    globals_before[GLOBALS_OFFSETS["start_lock"]] = int(case.get("start_lock", 0))
    put_word(globals_before, GLOBALS_OFFSETS["ui"], int(case.get("ui", 0xA100)))
    globals_before[GLOBALS_OFFSETS["effect_active"]] = int(case.get("effect_active", 0))

    put_word(state_before, PRIMARY_OFFSET, 0xC4)
    put_word(state_before, PRIMARY_OFFSET + 2, 0xDEAD)
    put_word(state_before, PRIMARY_OFFSET + 4, 0xBEEF)
    entries = [dict(entry) for entry in case["entries"]]
    cursor = DIRECTORY_OFFSET
    for entry_index, entry in enumerate(entries):
        object_offset = 0x1200 + entry_index * 0x100
        related_offset = 0x2400 + entry_index * 0x100
        kind = int(entry["kind"])
        flags = int(entry.get("flags", 1))
        record_offset = object_offset + field_offset(executable, 0x13, kind)
        record_related = int(entry.get("record_related", WILDCARD))
        if kind == 1:
            record_related = related_offset
        entry.update(
            object_offset=object_offset,
            related_offset=related_offset,
            record_offset=record_offset,
        )
        name = f"ENTRY{entry_index}".encode("ascii").ljust(16, b"\0")
        directory_before[cursor : cursor + 20] = struct.pack(
            "<16sHH",
            name,
            object_offset,
            int(entry.get("directory_kind", 1)),
        )
        cursor += 20
        put_word(state_before, object_offset, kind)
        put_word(state_before, object_offset + 2, flags)
        if record_offset >= object_offset + 4:
            put_word(state_before, record_offset, int(entry.get("record_kind", 0xC4)))
            put_word(state_before, record_offset + 2, record_related)
            put_word(state_before, record_offset + 4, int(entry.get("record_value", 1)))
        if kind == 2:
            put_word(
                state_before,
                object_offset + field_offset(executable, 2, kind),
                int(entry.get("target", 0)),
            )
        if kind == 1:
            put_word(state_before, related_offset, 0x7777)
            put_word(
                state_before,
                related_offset + 2,
                int(entry.get("related_flags", 1)),
            )
            state_before[related_offset + 4 : related_offset + 16] = (
                f"ACTOR{entry_index}".encode("ascii").ljust(12, b"\0")
            )
    directory_before[cursor : cursor + 20] = struct.pack(
        "<16sHH", b"SENTINEL".ljust(16, b"\0"), 0xDEAD, 0
    )
    return globals_before, state_before, directory_before, history_before, entries


def capture_call(cpu: Uc, name: str) -> dict[str, object]:
    if name == "control":
        return {
            "name": name,
            "object": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
            "target": cpu.reg_read(UC_X86_REG_BX) & 0xFFFF,
        }
    if name == "action":
        record = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
        return {
            "name": name,
            "object": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
            "record": record,
            "triple": list(struct.unpack("<HHH", cpu.mem_read(STATE + record, 6))),
        }
    if name == "descript":
        return {
            "name": name,
            "segment": cpu.reg_read(UC_X86_REG_ES),
            "name_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
        }
    if name == "resource":
        return {
            "name": name,
            "resource_id": cpu.reg_read(UC_X86_REG_AX),
            "segment": cpu.reg_read(UC_X86_REG_ES),
            "offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
        }
    if name == "setter":
        return {
            "name": name,
            "entity": cpu.reg_read(UC_X86_REG_AX),
            "segment": cpu.reg_read(UC_X86_REG_ES),
            "offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
            "x": cpu.reg_read(UC_X86_REG_BX),
            "y": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
            "frame": cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF,
        }
    if name == "transition":
        return {"name": name, "object_id": cpu.reg_read(UC_X86_REG_AX)}
    if name == "state_processor":
        return {"name": name}
    raise AssertionError(name)


def execute(executable: bytes, case: dict[str, object], case_index: int):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    globals_before, state_before, directory_before, history_before, entries = (
        initial_images(executable, case, case_index)
    )
    decoy_before = bytes(
        (offset * 23 + case_index * 43 + 0xCB) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    resource_before = bytes(
        (offset * 31 + case_index * 47 + 0xED) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    stack_before = bytearray(
        (offset * 11 + case_index * 53 + 0x0F) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    stack_before[STACK_POINTER : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    for address, data in (
        (GLOBALS, globals_before),
        (STATE, state_before),
        (DIRECTORY, directory_before),
        (HISTORY, history_before),
        (STACK, stack_before),
        (DECOY, decoy_before),
        (RESOURCE, resource_before),
    ):
        cpu.mem_write(address, bytes(data))

    initial_registers = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_CS: CODE_SEGMENT,
        UC_X86_REG_DS: DECOY // 16,
        UC_X86_REG_ES: RESOURCE // 16,
        UC_X86_REG_FS: 0xD000,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    processed_entries: list[int] = []
    pending_field: dict[str, object] | None = None
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal pending_field, reached_return
        file_offset = address + HEADER_SIZE
        if address == RETURN_LINEAR:
            reached_return = True
            machine.emu_stop()
            return
        external = EXTERNALS.get(file_offset)
        if external is not None:
            name, return_kind = external
            calls.append(capture_call(machine, name))
            (near_return if return_kind == "near" else far_return)(machine)
            return
        if file_offset == 0x5DF0:
            directory_pointer = machine.reg_read(UC_X86_REG_EDI) & 0xFFFF
            processed_entries.append((directory_pointer - DIRECTORY_OFFSET) // 20)
        if file_offset == FIELD_HELPER[0]:
            assert pending_field is None
            pending_field = {
                "name": "field",
                "selector": machine.reg_read(UC_X86_REG_AX),
                "kind": machine.reg_read(UC_X86_REG_BX),
            }
        elif file_offset == FIELD_HELPER[1] - 1:
            assert pending_field is not None
            pending_field["result"] = machine.reg_read(UC_X86_REG_AX)
            assert pending_field["result"] == field_offset(
                executable,
                int(pending_field["selector"]),
                int(pending_field["kind"]),
            )
            calls.append(pending_field)
            pending_field = None
        assert any(
            image_address(start) <= address < address + size <= image_address(end)
            for start, end in (SCAN, FIELD_HELPER)
        ), hex(file_offset)

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert any(
            start <= address < address + size <= start + SEGMENT_SIZE
            for start in (GLOBALS, STATE, HISTORY, STACK)
        ), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(SCAN[0]), 0, count=5000)
    assert reached_return, case["name"]
    assert pending_field is None

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    state_after = bytes(cpu.mem_read(STATE, SEGMENT_SIZE))
    history_after = bytes(cpu.mem_read(HISTORY, SEGMENT_SIZE))
    stop_kind = cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF
    assert processed_entries
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DIRECTORY, SEGMENT_SIZE)) == bytes(directory_before)
    assert bytes(cpu.mem_read(DECOY, SEGMENT_SIZE)) == decoy_before
    assert bytes(cpu.mem_read(RESOURCE, SEGMENT_SIZE)) == resource_before
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )
    last_entry = entries[processed_entries[-1]]
    assert cpu.reg_read(UC_X86_REG_ESI) == int(last_entry["object_offset"])
    assert cpu.reg_read(UC_X86_REG_EDI) == initial_registers[UC_X86_REG_EDI]
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert cpu.reg_read(UC_X86_REG_DS) == STATE // 16
    assert cpu.reg_read(UC_X86_REG_ES) == DIRECTORY // 16
    assert cpu.reg_read(UC_X86_REG_GS) == GLOBALS // 16
    assert bool(cpu.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == bool(
        initial_registers[UC_X86_REG_EFLAGS] & 0x0400
    )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        name: bool(flags & mask)
        for name, mask in (
            ("cf", 0x0001),
            ("pf", 0x0004),
            ("af", 0x0010),
            ("zf", 0x0040),
            ("sf", 0x0080),
            ("of", 0x0800),
        )
    }
    return {
        "name": case["name"],
        "processed_entries": processed_entries,
        "stop_directory_kind": stop_kind,
        "calls": calls,
        "presentation_active_after": globals_after[
            GLOBALS_OFFSETS["presentation_active"]
        ],
        "deferred_after": [
            word(globals_after, GLOBALS_OFFSETS[name])
            for name in ("deferred_type", "deferred_related", "deferred_value")
        ],
        "global_changes": changes(globals_before, globals_after),
        "state_changes": changes(state_before, state_after),
        "history_changes": changes(history_before, history_after),
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "state_sha256": hashlib.sha256(state_after).hexdigest(),
        "history_sha256": hashlib.sha256(history_after).hexdigest(),
        "defined_flags": defined_flags,
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander_rows = json.loads(commander_path.read_text())
    assert len(rows) == len(commander_rows) == len(CASES)
    for sequel, commander in zip(rows, commander_rows, strict=True):
        name = sequel["name"]
        assert name == commander["name"]
        for field in (
            "processed_entries",
            "stop_directory_kind",
            "presentation_active_after",
            "deferred_after",
            "defined_flags",
        ):
            assert sequel[field] == commander[field], (name, field)
        assert normalized_calls(sequel["calls"]) == normalized_calls(
            commander["calls"]
        ), name
        state_processor_calls = [
            call for call in sequel["calls"] if call["name"] == "state_processor"
        ]
        # The extra helper occurs once after each processed active actor.
        expected_state_processors = sum(
            int(
                int(entry["kind"]) == 2
                and int(entry.get("flags", 1)) & 1 != 0
                and index in sequel["processed_entries"]
            )
            for index, entry in enumerate(
                next(case for case in CASES if case["name"] == name)["entries"]
            )
        )
        assert len(state_processor_calls) == expected_state_processors, name


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_5816_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    for (start, end), expected in (
        (SCAN, SCAN_SHA256),
        (FIELD_HELPER, FIELD_HELPER_SHA256),
    ):
        actual = hashlib.sha256(executable[start:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"native span {start:#x}..{end:#x} changed")
    assert tuple(
        field_offset(executable, 0x13, kind) for kind in (1, 2, 0x10, 0x200)
    ) == (8, 58, 28, 10)
    assert field_offset(executable, 2, 2) == 26

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB presentation-scan cases")


if __name__ == "__main__":
    main()
