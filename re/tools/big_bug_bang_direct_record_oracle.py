#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited direct-record handler family.

The original AD/AF/B2/B3/BA/BB/BC handler and every entered owner lookup,
aboard-list removal, aboard-list insertion, and guard-failure helper run
unmodified. Run with ``python3 -P`` so the adjacent dis.py cannot shadow the
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
HANDLER = (0x754E, 0x75CD)
RETURN = 0x75CC
REMOVE_HELPER = (0x65E8, 0x6606)
INSERT_HELPER = (0x6606, 0x6633)
OWNER_HELPER = (0x6644, 0x665E)
FAILURE_HELPER = (0x697A, 0x6993)

GLOBALS = 0x30000
STATE = 0x40000
SIZE = 0x10000
DIRECTORY = 0x8000
SCRIPT = 0x9000
STACK = 0xFF00
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
SPECIAL_OBJECT = 0x6B1E
PUBLISHED_VALUE = 0x6B54
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
ABOARD_ROSTER = 0x70E6
ABOARD_CAPACITY = 16

OPCODES = (0xAD, 0xAF, 0xB2, 0xB3, 0xBA, 0xBB, 0xBC)
QUERY_VALUES = (1, 3, 0xFF)
ASSIGNMENT_VALUES = (0, 2, 0xFE)
ACTIVE_FLAG = 1

PLAYER_KIND = 0x0001
ACTOR_KIND = 0x0002
AUXILIARY_KIND = 0x0040
LOCATION_KIND = 0x0080
KINDS = (
    AUXILIARY_KIND,
    AUXILIARY_KIND,
    PLAYER_KIND,
    ACTOR_KIND,
    LOCATION_KIND,
    LOCATION_KIND,
) + (LOCATION_KIND,) * ABOARD_CAPACITY
SIZES = {
    PLAYER_KIND: 34,
    ACTOR_KIND: 74,
    AUXILIARY_KIND: 20,
    LOCATION_KIND: 26,
}
OFFSETS = []
_cursor = 0
for _kind in KINDS:
    OFFSETS.append(_cursor)
    _cursor += SIZES[_kind]
STATE_END = _cursor
SPECIAL = OFFSETS[2]
OWNER = OFFSETS[3]
ORDINARY = OFFSETS[4]
ALTERNATE = OFFSETS[5]
FILLERS = OFFSETS[6:]
TARGET = OWNER + 24
TOPICS = (0, 5)
NATIVE_VALUE = 0x1234
INITIAL_PUBLICATION = 0xA55A


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


def roster(mode):
    if mode == "empty":
        return [0] * ABOARD_CAPACITY
    if mode == "owner":
        return [OWNER] + [0] * (ABOARD_CAPACITY - 1)
    if mode == "full_without_owner":
        return list(FILLERS)
    if mode == "full_with_owner":
        return list(FILLERS[:8]) + [OWNER] + list(FILLERS[8:15])
    raise AssertionError(mode)


def remove_owner(slots):
    after = list(slots)
    try:
        index = after.index(OWNER)
    except ValueError:
        return after, False
    after[index] = 0
    return after, True


def insert_owner(slots):
    after = list(slots)
    if OWNER in after:
        return after, True
    try:
        index = after.index(0)
    except ValueError:
        return after, False
    after[index] = OWNER
    return after, True


def requested_values(opcode):
    if opcode == 0xBC:
        return [("topic", value) for value in TOPICS]
    return (
        ("special", SPECIAL),
        ("object", ORDINARY),
        ("aboard", 0xFFFF),
        ("native", NATIVE_VALUE),
    )


def query_cases():
    cases = []
    for opcode, query in itertools.product(OPCODES, QUERY_VALUES):
        inversions = (False,) if opcode == 0xAD else (False, True)
        for inverted in inversions:
            for requested_kind, requested in requested_values(opcode):
                comparison = 0xFFFF if requested == SPECIAL else requested
                for current_variant in ("match", "different"):
                    current = comparison if current_variant == "match" else ALTERNATE
                    current_kind = (
                        "aboard"
                        if current == 0xFFFF
                        else requested_kind
                        if current_variant == "match"
                        else "object"
                    )
                    cases.append(
                        {
                            "opcode": opcode,
                            "query_before": query,
                            "inverted": inverted,
                            "requested_kind": requested_kind,
                            "requested_value": requested,
                            "field_before_kind": current_kind,
                            "field_before": current,
                            "roster_mode": "empty",
                        }
                    )
    return cases


def assignment_cases():
    cases = []
    for opcode, query in itertools.product(OPCODES, ASSIGNMENT_VALUES):
        for requested_kind, requested in requested_values(opcode):
            for current_kind, roster_mode in itertools.product(
                ("object", "aboard"),
                ("empty", "owner", "full_without_owner", "full_with_owner"),
            ):
                cases.append(
                    {
                        "mode": "assignment",
                        "opcode": opcode,
                        "query_before": query,
                        "inverted": False,
                        "requested_kind": requested_kind,
                        "requested_value": requested,
                        "field_before_kind": current_kind,
                        "field_before": ALTERNATE if current_kind == "object" else 0xFFFF,
                        "roster_mode": roster_mode,
                    }
                )
    return cases


def expected(case):
    query_mode = case["query_before"] & 1 != 0
    field_after = case["field_before"]
    field_after_kind = case["field_before_kind"]
    roster_after = roster(case["roster_mode"])
    published_after = INITIAL_PUBLICATION
    remove_called = False
    insert_called = False
    owner_calls = 0
    failed = False
    write_succeeded = False

    if query_mode:
        comparison = (
            0xFFFF
            if case["requested_value"] == SPECIAL
            else case["requested_value"]
        )
        matches = case["field_before"] == comparison
        failed = matches == case["inverted"]
    else:
        if case["opcode"] == 0xBC:
            published_after = case["requested_value"]
        if case["field_before"] == 0xFFFF:
            owner_calls += 1
            remove_called = True
            roster_after, _ = remove_owner(roster_after)
        if case["requested_value"] in (SPECIAL, 0xFFFF):
            owner_calls += 1
            insert_called = True
            roster_after, write_succeeded = insert_owner(roster_after)
            if write_succeeded:
                field_after = 0xFFFF
                field_after_kind = "aboard"
        else:
            write_succeeded = True
            field_after = case["requested_value"]
            field_after_kind = case["requested_kind"]

    return {
        "field_after": field_after,
        "field_after_kind": field_after_kind,
        "roster_after": roster_after,
        "published_after": published_after,
        "query_after": 0 if failed else case["query_before"],
        "guard_depth_after": 0 if failed else 1,
        "failed": failed,
        "write_succeeded": write_succeeded,
        "cursor": BRANCH_TARGET
        if failed
        else SCRIPT + 5 + int(case["inverted"]),
        "owner_calls": owner_calls,
        "remove_calls": int(remove_called),
        "insert_calls": int(insert_called),
        "failure_calls": int(failed),
    }


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
    struct.pack_into("<H", globals_before, SPECIAL_OBJECT, SPECIAL)
    struct.pack_into("<H", globals_before, PUBLISHED_VALUE, INITIAL_PUBLICATION)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    struct.pack_into("<H", globals_before, GUARD_STACK, BRANCH_TARGET)
    roster_before = roster(case["roster_mode"])
    struct.pack_into(
        f"<{ABOARD_CAPACITY}H", globals_before, ABOARD_ROSTER, *roster_before
    )
    directory = directory_image()
    globals_before[DIRECTORY : DIRECTORY + len(directory)] = directory
    script = bytes([case["opcode"]])
    if case["inverted"]:
        script += b"\xA1"
    script += struct.pack("<HH", TARGET, case["requested_value"])
    globals_before[SCRIPT : SCRIPT + len(script)] = script
    cpu.mem_write(GLOBALS, bytes(globals_before))

    state_before = bytearray(SIZE)
    for kind, offset in zip(KINDS, OFFSETS):
        struct.pack_into("<HH", state_before, offset, kind, ACTIVE_FLAG)
    struct.pack_into("<H", state_before, TARGET, case["field_before"])
    cpu.mem_write(STATE, bytes(state_before))

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_ES, STATE // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT + 1),
        (UC_X86_REG_DX, 0),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    entered = {"owner": 0, "remove": 0, "insert": 0, "failure": 0}
    entry_names = {
        OWNER_HELPER[0]: "owner",
        REMOVE_HELPER[0]: "remove",
        INSERT_HELPER[0]: "insert",
        FAILURE_HELPER[0]: "failure",
    }
    ranges = (HANDLER, REMOVE_HELPER, INSERT_HELPER, OWNER_HELPER, FAILURE_HELPER)
    reached_return = False
    allowed_global_writes = (
        (PUBLISHED_VALUE, 2),
        (QUERY, 1),
        (GUARD_TOP, 2),
        (ABOARD_ROSTER, ABOARD_CAPACITY * 2),
        (STACK - 64, 64),
    )

    def instruction(_cpu, address, size, _context):
        nonlocal reached_return
        file_address = address + HEADER
        assert any(in_range(address, size, span) for span in ranges), hex(file_address)
        if file_address in entry_names:
            entered[entry_names[file_address]] += 1
        if file_address == RETURN:
            reached_return = True
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
        assert TARGET <= start and start + size <= TARGET + 2, hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(HANDLER[0] - HEADER, 0x100000, count=100000)
    assert reached_return
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert bytes(cpu.mem_read(0, len(module))) == module

    globals_after = bytes(cpu.mem_read(GLOBALS, SIZE))
    state_after = bytes(cpu.mem_read(STATE, SIZE))
    result = {
        **case,
        "object_offsets": OFFSETS,
        "target_offset": TARGET,
        "special_offset": SPECIAL,
        "ordinary_offset": ORDINARY,
        "alternate_offset": ALTERNATE,
        "roster_before": roster_before,
        "roster_after": list(
            struct.unpack_from(f"<{ABOARD_CAPACITY}H", globals_after, ABOARD_ROSTER)
        ),
        "field_after": word(state_after, TARGET),
        "published_before": INITIAL_PUBLICATION,
        "published_after": word(globals_after, PUBLISHED_VALUE),
        "query_after": globals_after[QUERY],
        "guard_depth_after": word(globals_after, GUARD_TOP) // 2,
        "entered": entered,
        "cursor": cpu.reg_read(UC_X86_REG_SI),
    }

    restored_globals = bytearray(globals_after)
    for start, length in allowed_global_writes:
        restored_globals[start : start + length] = globals_before[start : start + length]
    assert restored_globals == globals_before
    restored_state = bytearray(state_after)
    restored_state[TARGET : TARGET + 2] = state_before[TARGET : TARGET + 2]
    assert restored_state == state_before
    return result


def vectors(executable):
    for opcode in OPCODES:
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert encoded == HANDLER[0] - (HEADER + CODE_BASE)

    rows = []
    for case in query_cases() + assignment_cases():
        row = execute(executable, case)
        expected_values = expected(case)
        row["field_after_kind"] = expected_values["field_after_kind"]
        row["failed"] = expected_values["failed"]
        row["write_succeeded"] = expected_values["write_succeeded"]
        for key in (
            "field_after",
            "roster_after",
            "published_after",
            "query_after",
            "guard_depth_after",
            "cursor",
        ):
            assert row[key] == expected_values[key], (case, key, row[key], expected_values[key])
        assert row["entered"]["owner"] == expected_values["owner_calls"]
        assert row["entered"]["remove"] == expected_values["remove_calls"]
        assert row["entered"]["insert"] == expected_values["insert_calls"]
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
    print(f"captured {len(rows)} original direct-record cases")


if __name__ == "__main__":
    main()
