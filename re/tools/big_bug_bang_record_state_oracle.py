#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited C1 navigation-record handler.

The original handler and every entered owner, field, distance, position,
source-list, link-bit, square-root, and guard-failure helper run unmodified.
Execution stops at the shared epilogue so the shipped successful-query and
exhausted-source stack defect can be recorded without following its bogus
return. Run with ``python3 -P`` so the adjacent dis.py cannot shadow the
standard library.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_GS,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
HANDLER = (0x7752, 0x7884)
EPILOGUE = 0x7882
FIELD_HELPER = (0x6633, 0x6644)
OWNER_HELPER = (0x6644, 0x665E)
DISTANCE_HELPER = (0x66ED, 0x67B8)
POSITION_HELPER = (0x67B8, 0x6822)
BIT_HELPER = (0x6822, 0x685D)
SOURCE_HELPER = (0x685D, 0x68A5)
FAILURE_HELPER = (0x697A, 0x6993)
SQRT_HELPER = (0x31B4, 0x31F4)

GLOBALS = 0x30000
STATE = 0x40000
SIZE = 0x10000
DIRECTORY = 0x8000
SCRIPT = 0x9000
STACK = 0xFF00
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
ACTIVE_REFERENCE_POINTER = 0x6B22
RELATED_OPERAND = 0x6B06
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
SOURCE_LIST = 0x6C2E

PLAYER_KIND = 0x0001
ACTOR_KIND = 0x0002
AUXILIARY_KIND = 0x0040
LOCATION_KIND = 0x0080
WORLD_STATE_KIND = 0x0200
NAVIGATION_KIND = 0x0010
ACTIVE_FLAG = 0x0001
IN_PLAY_FLAG = 0x0002
QUERY_VALUES = (1, 3, 0xFF)
ASSIGNMENT_VALUES = (0, 2, 0xFE)

KINDS = (
    AUXILIARY_KIND,
    WORLD_STATE_KIND,
    ACTOR_KIND,
    ACTOR_KIND,
    LOCATION_KIND,
    LOCATION_KIND,
    NAVIGATION_KIND,
    NAVIGATION_KIND,
    NAVIGATION_KIND,
    NAVIGATION_KIND,
    LOCATION_KIND,
    PLAYER_KIND,
    ACTOR_KIND,
    ACTOR_KIND,
    LOCATION_KIND,
)
SIZES = {
    PLAYER_KIND: 34,
    ACTOR_KIND: 74,
    AUXILIARY_KIND: 20,
    LOCATION_KIND: 26,
    WORLD_STATE_KIND: 38,
    NAVIGATION_KIND: 36,
}
HOLDER_OFFSETS = {
    PLAYER_KIND: 6,
    ACTOR_KIND: 24,
    LOCATION_KIND: 20,
    WORLD_STATE_KIND: 4,
    NAVIGATION_KIND: 22,
}
ACTION_OFFSETS = {PLAYER_KIND: 8, ACTOR_KIND: 58, NAVIGATION_KIND: 28}

OFFSETS = []
_cursor = 0
for _kind in KINDS:
    OFFSETS.append(_cursor)
    _cursor += SIZES[_kind]
STATE_END = _cursor
(
    ALIAS,
    PRIMARY,
    SECONDARY,
    DIRECT_OWNER,
    SPECIAL_OWNER,
    OPERAND,
    NAV_QUERY,
    NAV_SPECIAL,
    NAV_OWNER,
    NAV_REGULAR,
    SOURCE_UNKNOWN,
    SOURCE_PLAYER,
    SOURCE_ACTOR_REJECT,
    SOURCE_ACTOR_ACCEPT,
    WRONG_PARENT,
) = OFFSETS
DIRECT_SLOT = DIRECT_OWNER + ACTION_OFFSETS[ACTOR_KIND]
SPECIAL_SLOT = SPECIAL_OWNER + 4
NAV_QUERY_SLOT = NAV_QUERY + ACTION_OFFSETS[NAVIGATION_KIND]
NAV_OWNER_SLOT = NAV_OWNER + ACTION_OFFSETS[NAVIGATION_KIND]
NAV_REGULAR_SLOT = NAV_REGULAR + ACTION_OFFSETS[NAVIGATION_KIND]
TRACKED_SLOTS = (
    DIRECT_SLOT,
    SPECIAL_SLOT,
    NAV_QUERY_SLOT,
    NAV_OWNER_SLOT,
    NAV_REGULAR_SLOT,
)
LINK_MASK = 1 << (7 - OFFSETS.index(OPERAND))


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def directory_image():
    encoded = bytearray((len(OFFSETS) + 1) * 20)
    for index, offset in enumerate(OFFSETS):
        entry = index * 20
        name = f"object{index}".encode("ascii")
        encoded[entry : entry + len(name)] = name
        struct.pack_into("<HH", encoded, entry + 16, offset, 1)
    struct.pack_into("<H", encoded, len(OFFSETS) * 20 + 16, STATE_END)
    return encoded


def set_parent(state, object_offset, kind, parent):
    field = HOLDER_OFFSETS.get(kind)
    if field is not None:
        struct.pack_into("<H", state, object_offset + field, parent)


def set_position(state, object_offset, kind, position):
    field = {WORLD_STATE_KIND: 6, NAVIGATION_KIND: 24}[kind]
    struct.pack_into("<HH", state, object_offset + field, *position)


def records(operand, other=OPERAND):
    return {
        "exact": [0xC1, operand, 0],
        "exact_other_third": [0xC1, operand, 0xA55A],
        "other_relation": [0xC1, other, 0],
        "other_kind": [0xC2, operand, 0],
        "empty": [0, 0, 0],
        "occupied": [0x9999, other, 0x5AA5],
    }


def source_offsets(mode, special=False):
    entries = [SPECIAL_OWNER] if special else []
    entries.extend(
        {
            "none": [],
            "unknown": [SOURCE_UNKNOWN],
            "player_reject": [SOURCE_PLAYER],
            "player_accept": [SOURCE_PLAYER],
            "actor_reject": [SOURCE_ACTOR_REJECT],
            "actor_accept": [SOURCE_ACTOR_ACCEPT],
            "actor_reject_accept": [SOURCE_ACTOR_REJECT, SOURCE_ACTOR_ACCEPT],
        }[mode]
    )
    return entries


def build_state(case):
    state = bytearray(SIZE)
    for kind, offset in zip(KINDS, OFFSETS):
        struct.pack_into("<HH", state, offset, kind, ACTIVE_FLAG)
        set_parent(state, offset, kind, 0xFFFF)

    # Native operands 1 and 2 are deliberate unaligned aliases. Match them to
    # the typed primary WorldState and secondary Actor used by the Rust host.
    alias_flags = 0x0202 if case["operand"] == 1 else 0x0002
    struct.pack_into("<H", state, ALIAS + 2, alias_flags)
    if case["operand"] == 2:
        state[ALIAS + 4] |= IN_PLAY_FLAG
    alias_position = (NAV_SPECIAL, 200)
    struct.pack_into("<HH", state, 7, *alias_position)
    set_position(state, PRIMARY, WORLD_STATE_KIND, alias_position)
    set_parent(state, SECONDARY, ACTOR_KIND, NAV_SPECIAL)
    struct.pack_into("<H", state, SECONDARY + 2, ACTIVE_FLAG | IN_PLAY_FLAG)
    set_position(state, NAV_SPECIAL, NAVIGATION_KIND, alias_position)

    set_parent(state, DIRECT_OWNER, ACTOR_KIND, NAV_QUERY)
    struct.pack_into("<H", state, DIRECT_OWNER + 6, NAV_QUERY)
    set_parent(state, SPECIAL_OWNER, LOCATION_KIND, NAV_OWNER)
    owner_position = alias_position if case.get("distance") == "same" else (NAV_SPECIAL + 8, 200)
    set_position(state, NAV_OWNER, NAVIGATION_KIND, owner_position)
    set_parent(state, WRONG_PARENT, LOCATION_KIND, 0xFFFF)

    owner = {
        "direct_query": DIRECT_OWNER,
        "direct_set": DIRECT_OWNER,
        "special_set": SPECIAL_OWNER,
        "nav_set": NAV_REGULAR,
    }[case["family"]]
    flags = ACTIVE_FLAG if case["owner_active"] else 0
    struct.pack_into("<H", state, owner + 2, flags)
    if case["family"] == "special_set" and case.get("parent") == "wrong":
        set_parent(state, SPECIAL_OWNER, LOCATION_KIND, WRONG_PARENT)

    mode = case.get("source_mode", "none")
    source_target = NAV_OWNER if case["family"] == "special_set" else NAV_REGULAR
    for source in source_offsets(mode):
        kind = KINDS[OFFSETS.index(source)]
        set_parent(state, source, kind, source_target)
    operand_flags = ACTIVE_FLAG | (IN_PLAY_FLAG if mode == "player_accept" else 0)
    struct.pack_into("<H", state, OPERAND + 2, operand_flags)
    state[SOURCE_ACTOR_ACCEPT + 30] = LINK_MASK
    state[SOURCE_ACTOR_REJECT + 30] = 0

    for slot in TRACKED_SLOTS:
        struct.pack_into("<3H", state, slot, 0, 0, 0)
    operand = case["operand"]
    if case["family"] == "direct_query":
        direct = records(operand)[case["direct_variant"]]
        resolved = records(operand)[case["resolved_variant"]]
        struct.pack_into("<3H", state, DIRECT_SLOT, *direct)
        struct.pack_into("<3H", state, NAV_QUERY_SLOT, *resolved)
    elif case["family"] == "direct_set":
        before = records(operand)[case["target_variant"]]
        struct.pack_into("<3H", state, DIRECT_SLOT, *before)
    elif case["family"] == "special_set":
        before = records(operand)[case["target_variant"]]
        destination = records(operand)[case["destination_variant"]]
        struct.pack_into("<3H", state, SPECIAL_SLOT, *before)
        struct.pack_into("<3H", state, NAV_OWNER_SLOT, *destination)
    else:
        before = records(operand)[case["destination_variant"]]
        struct.pack_into("<3H", state, NAV_REGULAR_SLOT, *before)
    return state


def execute(executable, case):
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)

    globals_before = bytearray(SIZE)
    native_data = executable[0xF7F0:]
    globals_before[: len(native_data)] = native_data
    globals_before[QUERY] = case["query_before"]
    struct.pack_into("<HH", globals_before, STATE_POINTER, 0, STATE // 16)
    struct.pack_into("<HH", globals_before, DIRECTORY_POINTER, DIRECTORY, GLOBALS // 16)
    struct.pack_into("<H", globals_before, ACTIVE_REFERENCE_POINTER, NAV_OWNER)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    struct.pack_into("<H", globals_before, GUARD_STACK, BRANCH_TARGET)
    struct.pack_into("<H", globals_before, SOURCE_LIST, 0xFFFF)

    operand_bytes = b"\xA1" if case["inverted"] else b""
    operand_bytes += struct.pack("<HH", case["target_offset"], case["operand"])
    globals_before[SCRIPT : SCRIPT + len(operand_bytes)] = operand_bytes
    directory = directory_image()
    globals_before[DIRECTORY : DIRECTORY + len(directory)] = directory

    mode = case.get("source_mode", "none")
    special = case["family"] == "special_set"
    entries = source_offsets(mode, special=special)
    if SOURCE_ACTOR_ACCEPT in entries:
        position = entries.index(SOURCE_ACTOR_ACCEPT)
        globals_before[SOURCE_LIST + (position + 1) * 2 + 30] = LINK_MASK
    struct.pack_into("<H", globals_before, STACK, 0x1800)
    cpu.mem_write(GLOBALS, bytes(globals_before))

    state_before = build_state(case)
    cpu.mem_write(STATE, bytes(state_before))
    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_ES, STATE // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT),
        (UC_X86_REG_DX, 0),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    entered = {
        "owner": 0,
        "field": 0,
        "distance": 0,
        "position": 0,
        "bit": 0,
        "source": 0,
        "sqrt": 0,
        "failure": 0,
    }
    entry_names = {
        OWNER_HELPER[0]: "owner",
        FIELD_HELPER[0]: "field",
        DISTANCE_HELPER[0]: "distance",
        POSITION_HELPER[0]: "position",
        BIT_HELPER[0]: "bit",
        SOURCE_HELPER[0]: "source",
        SQRT_HELPER[0]: "sqrt",
        FAILURE_HELPER[0]: "failure",
    }
    reached_epilogue = False
    ranges = (
        HANDLER,
        FIELD_HELPER,
        OWNER_HELPER,
        DISTANCE_HELPER,
        POSITION_HELPER,
        BIT_HELPER,
        SOURCE_HELPER,
        FAILURE_HELPER,
        SQRT_HELPER,
    )
    allowed_global_writes = (
        (RELATED_OPERAND, 2),
        (QUERY, 1),
        (GUARD_TOP, 2),
        (SOURCE_LIST, (len(OFFSETS) + 1) * 2),
        (STACK - 256, 258),
    )

    def instruction(_cpu, address, size, _context):
        nonlocal reached_epilogue
        file_address = address + HEADER
        assert any(in_range(address, size, span) for span in ranges), hex(file_address)
        if file_address in entry_names:
            entered[entry_names[file_address]] += 1
        if file_address == EPILOGUE:
            reached_epilogue = True
            _cpu.emu_stop()

    def write_hook(_cpu, _access, address, size, _value, _context):
        if GLOBALS <= address < GLOBALS + SIZE:
            start = address - GLOBALS
            assert any(
                allowed <= start and start + size <= allowed + length
                for allowed, length in allowed_global_writes
            ), hex(address)
            return
        assert STATE <= address < STATE + SIZE
        start = address - STATE
        assert any(slot <= start and start + size <= slot + 6 for slot in TRACKED_SLOTS), hex(
            address
        )

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(HANDLER[0] - HEADER, 0x100000, count=100000)
    assert reached_epilogue
    assert bytes(cpu.mem_read(0, len(module))) == module

    globals_after = bytes(cpu.mem_read(GLOBALS, SIZE))
    state_after = bytes(cpu.mem_read(STATE, SIZE))
    stack_pointer = cpu.reg_read(UC_X86_REG_SP)
    assert stack_pointer in (STACK - 2, STACK - 6), hex(stack_pointer)
    unrestored_frame = stack_pointer == STACK - 6
    actual_cursor = cpu.reg_read(UC_X86_REG_SI)
    logical_cursor = word(globals_after, stack_pointer) if unrestored_frame else actual_cursor
    source_list = []
    if entered["source"]:
        for index in range(len(OFFSETS) + 1):
            value = word(globals_after, SOURCE_LIST + index * 2)
            if value == 0xFFFF:
                break
            source_list.append(value)
        else:
            raise AssertionError("unterminated source list")

    result = {
        **case,
        "object_offsets": OFFSETS,
        "tracked_offsets": TRACKED_SLOTS,
        "records_before": [
            list(struct.unpack_from("<3H", state_before, slot)) for slot in TRACKED_SLOTS
        ],
        "records_after": [
            list(struct.unpack_from("<3H", state_after, slot)) for slot in TRACKED_SLOTS
        ],
        "query_after": globals_after[QUERY],
        "guard_depth_after": word(globals_after, GUARD_TOP) // 2,
        "related_operand_after": word(globals_after, RELATED_OPERAND),
        "source_entries": source_list,
        "entered": entered,
        "unrestored_frame": unrestored_frame,
        "actual_cursor": actual_cursor,
        "cursor": logical_cursor,
    }

    restored_globals = bytearray(globals_after)
    for start, length in allowed_global_writes:
        restored_globals[start : start + length] = globals_before[start : start + length]
    assert restored_globals == globals_before
    restored_state = bytearray(state_after)
    for slot in TRACKED_SLOTS:
        restored_state[slot : slot + 6] = state_before[slot : slot + 6]
    assert restored_state == state_before
    return result


def expected(case):
    before = case["records_before"]
    after = [list(record) for record in before]
    operand = case["operand"]
    failed = False
    destination = None
    source_called = False
    accepted = False

    if case["family"] == "direct_query":
        direct = before[TRACKED_SLOTS.index(DIRECT_SLOT)]
        resolved = before[TRACKED_SLOTS.index(NAV_QUERY_SLOT)]
        comparison = (
            resolved if operand in (1, 2) and direct[0] != 0xC1 else direct
        )
        matches = comparison[0] == 0xC1 and comparison[1] == operand
        failed = matches == case["inverted"]
    elif not case["owner_active"]:
        failed = True
    elif case["family"] == "direct_set":
        destination = DIRECT_SLOT
    elif case["family"] == "special_set":
        if case["distance"] == "same":
            destination = SPECIAL_SLOT
        elif case["parent"] == "wrong":
            failed = True
        else:
            source_called = True
            accepted = case["source_mode"] == "player_accept"
            if accepted:
                destination = NAV_OWNER_SLOT
    else:
        source_called = True
        accepted = case["source_mode"] in (
            "player_accept",
            "actor_accept",
            "actor_reject_accept",
        )
        if accepted:
            destination = NAV_REGULAR_SLOT

    if destination is not None and not failed:
        index = TRACKED_SLOTS.index(destination)
        if before[index][0] != 0:
            failed = True
        else:
            after[index] = [0xC1, operand, 2]

    query_mode = case["query_before"] & 1 != 0
    exhausted = source_called and not accepted
    unrestored = (query_mode and not failed) or exhausted
    source_entries_expected = (
        source_offsets(
            case.get("source_mode", "none"),
            special=case["family"] == "special_set",
        )
        if source_called
        else []
    )
    bit_calls = 0
    if source_called:
        bit_calls = {
            "actor_reject": 1,
            "actor_accept": 1,
            "actor_reject_accept": 2,
        }.get(case.get("source_mode"), 0)
    return {
        "records_after": after,
        "failed": failed,
        "query_after": 0 if failed else case["query_before"],
        "guard_depth_after": 0 if failed else 1,
        "related_operand_after": operand,
        "source_entries": source_entries_expected,
        "unrestored_frame": unrestored,
        "cursor": BRANCH_TARGET
        if failed
        else SCRIPT + 4 + int(case["inverted"]),
        "owner_calls": 1,
        "distance_calls": int(
            not query_mode and case["family"] == "special_set" and case["owner_active"]
        ),
        "source_calls_min": int(source_called),
        "bit_calls": bit_calls,
        "failure_calls": int(failed),
        "written_offset": destination if destination is not None and not failed else None,
    }


def direct_query_cases():
    cases = []
    for query, inverted, active, variant in itertools.product(
        QUERY_VALUES,
        (False, True),
        (False, True),
        ("exact", "exact_other_third", "other_relation", "other_kind", "empty"),
    ):
        cases.append(
            {
                "family": "direct_query",
                "query_before": query,
                "inverted": inverted,
                "owner_active": active,
                "operand": OPERAND,
                "target_offset": DIRECT_SLOT,
                "direct_variant": variant,
                "resolved_variant": "empty",
            }
        )
    for query, inverted, active, operand, direct, resolved in itertools.product(
        QUERY_VALUES,
        (False, True),
        (False, True),
        (1, 2),
        ("exact", "other_relation", "other_kind", "empty"),
        ("exact", "other_relation", "empty"),
    ):
        cases.append(
            {
                "family": "direct_query",
                "query_before": query,
                "inverted": inverted,
                "owner_active": active,
                "operand": operand,
                "target_offset": DIRECT_SLOT,
                "direct_variant": direct,
                "resolved_variant": resolved,
            }
        )
    return cases


def assignment_cases():
    cases = []
    for query, inverted, active, target in itertools.product(
        ASSIGNMENT_VALUES,
        (False, True),
        (False, True),
        ("empty", "occupied"),
    ):
        cases.append(
            {
                "family": "direct_set",
                "query_before": query,
                "inverted": inverted,
                "owner_active": active,
                "operand": OPERAND,
                "target_offset": DIRECT_SLOT,
                "target_variant": target,
            }
        )

    for query, inverted, operand in itertools.product(
        ASSIGNMENT_VALUES, (False, True), (1, 2)
    ):
        cases.append(
            {
                "family": "special_set",
                "query_before": query,
                "inverted": inverted,
                "owner_active": False,
                "operand": operand,
                "target_offset": SPECIAL_SLOT,
                "distance": "same",
                "parent": "navigation",
                "source_mode": "none",
                "target_variant": "empty",
                "destination_variant": "empty",
            }
        )
        for distance, parent, source, target, destination in (
            ("same", "navigation", "none", "empty", "empty"),
            ("same", "navigation", "none", "occupied", "empty"),
            ("different", "wrong", "none", "empty", "empty"),
            ("different", "navigation", "none", "empty", "empty"),
            ("different", "navigation", "player_accept", "empty", "empty"),
            ("different", "navigation", "player_accept", "empty", "occupied"),
        ):
            cases.append(
                {
                    "family": "special_set",
                    "query_before": query,
                    "inverted": inverted,
                    "owner_active": True,
                    "operand": operand,
                    "target_offset": SPECIAL_SLOT,
                    "distance": distance,
                    "parent": parent,
                    "source_mode": source,
                    "target_variant": target,
                    "destination_variant": destination,
                }
            )

    modes = (
        "none",
        "unknown",
        "player_reject",
        "player_accept",
        "actor_reject",
        "actor_accept",
        "actor_reject_accept",
    )
    for query, inverted, active, mode, destination in itertools.product(
        ASSIGNMENT_VALUES,
        (False, True),
        (False, True),
        modes,
        ("empty", "occupied"),
    ):
        cases.append(
            {
                "family": "nav_set",
                "query_before": query,
                "inverted": inverted,
                "owner_active": active,
                "operand": OPERAND,
                "target_offset": NAV_REGULAR_SLOT,
                "source_mode": mode,
                "destination_variant": destination,
            }
        )
    return cases


def vectors(executable):
    encoded = struct.unpack_from("<H", executable, DISPATCH_TABLE + (0xC1 - 0xA0) * 2)[0]
    assert encoded == HANDLER[0] - (HEADER + CODE_BASE)
    rows = []
    for case in direct_query_cases() + assignment_cases():
        row = execute(executable, case)
        expected_values = expected(row)
        row["failed"] = expected_values["failed"]
        row["written_offset"] = expected_values["written_offset"]
        for key in (
            "records_after",
            "query_after",
            "guard_depth_after",
            "related_operand_after",
            "source_entries",
            "unrestored_frame",
            "cursor",
        ):
            assert row[key] == expected_values[key], (case, key, row[key], expected_values[key])
        assert row["entered"]["owner"] == expected_values["owner_calls"]
        assert row["entered"]["distance"] == expected_values["distance_calls"]
        assert bool(row["entered"]["source"]) == bool(expected_values["source_calls_min"])
        assert row["entered"]["bit"] == expected_values["bit_calls"]
        assert row["entered"]["failure"] == expected_values["failure_calls"]
        rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    rows = vectors(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"captured {len(rows)} original C1 record-state cases")


if __name__ == "__main__":
    main()
