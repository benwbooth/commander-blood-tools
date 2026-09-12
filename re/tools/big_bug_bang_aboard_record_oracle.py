#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited C2 aboard-record handler.

The original handler, roster insertion, owner and field lookups, guard-failure
helper, DESCRIPT parser, and entered descriptor helpers run unmodified under
Unicorn. DOS file calls read an owned in-memory descriptor database; no helper
result is patched or substituted. Run with ``python3 -P`` so the adjacent
dis.py cannot shadow the standard library.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import (
    Uc,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
HANDLER = (0x7A3A, 0x7AF4)
SLOT_HELPER = (0x6606, 0x6633)
FIELD_HELPER = (0x6633, 0x6644)
OWNER_HELPER = (0x6644, 0x665E)
FAILURE_HELPER = (0x697A, 0x6993)
DESCRIPTOR_RANGES = (
    (0x8450, 0x8560),
    (0x8584, 0x85A0),
    (0x8654, 0x866B),
    (0x86B1, 0x86C6),
    (0x2A13, 0x2A4F),
    (0x2B43, 0x2B69),
)

GLOBALS = 0x30000
STATE = 0x40000
DIRECTORY = 0x50000
SCRATCH = 0x60000
SIZE = 0x10000
SCRIPT = 0x9000
STACK = 0xFF00
RETURN = 0x1800
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
ROSTER = 0x70E6
UI_STATE = 0x2A33
REQUEST_FLAGS = 0x6B80
C2_GATE = 0x2200
ACTIVE_LINE = 0x6B5A
SCRATCH_POINTER = 0x0CB4

ACTOR_KIND = 0x0002
LOCATION_KIND = 0x0080
INVENTORY_KIND = 0x0400
ACTIVE_FLAG = 0x0001
PRESENTABLE_FLAG = 0x0020
OWNER_OFFSET = 0
TARGET_OFFSET = 58
RELATED_OFFSET = 74
ROSTER_CAPACITY = 16
QUERY_VALUES = (1, 3, 0xFF)
ASSIGNMENT_VALUES = (0, 2, 0xFE)
INITIAL_HOLDER = 0x1234
INITIAL_C2_GATE = 0xA5
INITIAL_ACTIVE_LINE = 0x1357


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def kind_size(kind):
    return {ACTOR_KIND: 74, LOCATION_KIND: 26, INVENTORY_KIND: 24}[kind]


def holder_offset(kind):
    return {ACTOR_KIND: 24, LOCATION_KIND: 20, INVENTORY_KIND: 20}[kind]


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def synthetic_database(name, payload=b"\xff"):
    return struct.pack("<H16sHBH", 1, name, 21, 15, len(payload) + 2) + payload


def object_layout(related_kind):
    kinds = [ACTOR_KIND, related_kind, LOCATION_KIND]
    kinds.extend([LOCATION_KIND] * ROSTER_CAPACITY)
    offsets = []
    cursor = 0
    for kind in kinds:
        offsets.append(cursor)
        cursor += kind_size(kind)
    assert offsets[0] == OWNER_OFFSET
    assert offsets[1] == RELATED_OFFSET
    return kinds, offsets, cursor


def directory_image(offsets, state_end):
    encoded = bytearray((len(offsets) + 1) * 20)
    for index, offset in enumerate(offsets):
        entry = index * 20
        name = f"object{index}".encode("ascii")
        encoded[entry : entry + len(name)] = name
        struct.pack_into("<HH", encoded, entry + 16, offset, 1)
    struct.pack_into("<H", encoded, len(offsets) * 20 + 16, state_end)
    return encoded


def target_records(related, alternate):
    return {
        "exact": [0xC2, related, 0],
        "exact_other_third": [0xC2, related, 0xA55A],
        "other_relation": [0xC2, alternate, 0],
        "other_kind": [0xC3, related, 0],
        "empty": [0, 0, 0],
    }


def roster_image(mode, related, fillers):
    if mode == "free":
        return [0] * ROSTER_CAPACITY
    if mode == "existing":
        return [related] + [0] * (ROSTER_CAPACITY - 1)
    if mode == "full":
        assert len(fillers) == ROSTER_CAPACITY
        return list(fillers)
    raise AssertionError(mode)


def execute(executable, case):
    related_kind = case["related_kind"]
    kinds, offsets, state_end = object_layout(related_kind)
    related = offsets[1]
    alternate = offsets[2]
    fillers = offsets[3:]
    roster_before = roster_image(case["roster_mode"], related, fillers)

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)

    globals_before = bytearray(SIZE)
    native_data = executable[0xF7F0:]
    globals_before[: len(native_data)] = native_data
    globals_before[QUERY] = case["query_before"]
    globals_before[UI_STATE] = case["ui_state"]
    globals_before[REQUEST_FLAGS] = case["request_before"]
    globals_before[C2_GATE] = INITIAL_C2_GATE
    globals_before[0x0CEA] = 1
    globals_before[0x0CE9] = 1
    globals_before[0x7086] = 0
    struct.pack_into("<H", globals_before, ACTIVE_LINE, INITIAL_ACTIVE_LINE)
    struct.pack_into("<HH", globals_before, STATE_POINTER, 0, STATE // 16)
    struct.pack_into("<HH", globals_before, DIRECTORY_POINTER, 20, DIRECTORY // 16)
    struct.pack_into("<HH", globals_before, SCRATCH_POINTER, 0, SCRATCH // 16)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    struct.pack_into(f"<{ROSTER_CAPACITY}H", globals_before, ROSTER, *roster_before)

    operand = b"\xA1" if case["inverted"] else b""
    operand += struct.pack("<HH", TARGET_OFFSET, related)
    globals_before[SCRIPT : SCRIPT + len(operand)] = operand
    struct.pack_into("<H", globals_before, GUARD_STACK, BRANCH_TARGET)
    struct.pack_into("<H", globals_before, STACK, RETURN)
    cpu.mem_write(GLOBALS, bytes(globals_before))

    state_before = bytearray(SIZE)
    for index, (kind, offset) in enumerate(zip(kinds, offsets)):
        flags = 1
        if index == 0:
            flags = case["owner_flags"]
        elif index == 1:
            flags = case["related_flags"]
        struct.pack_into("<HH", state_before, offset, kind, flags)
    state_before[related + 4 : related + 9] = b"item\0"
    struct.pack_into(
        "<H", state_before, related + holder_offset(related_kind), INITIAL_HOLDER
    )
    struct.pack_into("<3H", state_before, TARGET_OFFSET, *case["target_before"])
    cpu.mem_write(STATE, bytes(state_before))

    directory_before = directory_image(offsets, state_end)
    cpu.mem_write(DIRECTORY, bytes(directory_before))

    database_name = b"item" if case["descriptor_available"] else b"other"
    database = synthetic_database(database_name)
    position = 0
    opened = False
    io = []
    calls = set()

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_ES, STATE // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    allowed_global_writes = (
        (QUERY, 1),
        (GUARD_TOP, 2),
        (ROSTER, ROSTER_CAPACITY * 2),
        (C2_GATE, 1),
        (REQUEST_FLAGS, 1),
        (ACTIVE_LINE, 2),
        (STACK - 128, 130),
        (0x2A89, 1),
        (0x156C, 1),
        (0x21FB, 4),
        (0x1568, 2),
        (0x1166, 4),
        (0x0D20, 1),
        (0x0CEB, 1),
        (0x0CA6, 4),
    )

    def cstring(address):
        return bytes(cpu.mem_read(address, 256)).split(b"\0", 1)[0]

    def instruction(_cpu, address, size, _context):
        spans = (HANDLER, SLOT_HELPER, FIELD_HELPER, OWNER_HELPER, FAILURE_HELPER)
        assert any(in_range(address, size, span) for span in spans + DESCRIPTOR_RANGES), hex(
            address + HEADER
        )
        file_address = address + HEADER
        for entry in (
            HANDLER[0],
            SLOT_HELPER[0],
            FIELD_HELPER[0],
            OWNER_HELPER[0],
            FAILURE_HELPER[0],
            0x8450,
        ):
            if file_address == entry:
                calls.add(entry)

    def write_hook(_cpu, _access, address, size, _value, _context):
        if GLOBALS <= address < GLOBALS + SIZE:
            start = address - GLOBALS
            assert any(
                allowed <= start and start + size <= allowed + length
                for allowed, length in allowed_global_writes
            ), hex(address)
            return
        holder = STATE + related + holder_offset(related_kind)
        assert address == holder and size == 2, (hex(address), size)

    def interrupt(_cpu, number, _context):
        nonlocal opened, position
        assert number == 0x21
        ax = cpu.reg_read(UC_X86_REG_AX)
        bx = cpu.reg_read(UC_X86_REG_BX)
        count = cpu.reg_read(UC_X86_REG_CX)
        dx = cpu.reg_read(UC_X86_REG_DX)
        destination = cpu.reg_read(UC_X86_REG_DS) * 16 + dx
        if ax == 0x3D00:
            assert cstring(destination) == b"descript.des"
            assert not opened
            opened = True
            position = 0
            cpu.reg_write(UC_X86_REG_AX, 5)
            cpu.reg_write(UC_X86_REG_EFLAGS, cpu.reg_read(UC_X86_REG_EFLAGS) & ~1)
            io.append(["open"])
        elif ax == 0x3F00:
            assert opened and bx == 5
            assert (
                destination in (GLOBALS + 0x0CA6, GLOBALS + 0x0CA8) and count == 2
                or destination == SCRATCH and count <= SIZE
            )
            data = database[position : position + count]
            cpu.mem_write(destination, data)
            position += len(data)
            cpu.reg_write(UC_X86_REG_AX, len(data))
            io.append(["read", count, len(data)])
        elif ax == 0x4200:
            assert opened and bx == 5 and count == 0
            position = dx
            cpu.reg_write(UC_X86_REG_AX, position)
            cpu.reg_write(UC_X86_REG_DX, 0)
            io.append(["seek", position])
        elif ax == 0x3E00:
            assert opened and bx == 5
            opened = False
            io.append(["close"])
        else:
            raise AssertionError(hex(ax))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.hook_add(UC_HOOK_INTR, interrupt)
    cpu.emu_start(HANDLER[0] - HEADER, CODE_BASE + RETURN, count=30000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert cpu.reg_read(UC_X86_REG_DS) == GLOBALS // 16
    assert cpu.reg_read(UC_X86_REG_ES) == STATE // 16
    assert not opened
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DIRECTORY, len(directory_before))) == directory_before

    globals_after = bytes(cpu.mem_read(GLOBALS, SIZE))
    state_after = bytes(cpu.mem_read(STATE, SIZE))
    roster_after = list(
        struct.unpack_from(f"<{ROSTER_CAPACITY}H", globals_after, ROSTER)
    )
    descriptor_called = 0x8450 in calls
    result = {
        **case,
        "target_offset": TARGET_OFFSET,
        "related_offset": related,
        "alternate_offset": alternate,
        "object_offsets": offsets,
        "roster_before": roster_before,
        "roster_after": roster_after,
        "query_after": globals_after[QUERY],
        "request_after": globals_after[REQUEST_FLAGS],
        "c2_gate_after": globals_after[C2_GATE],
        "active_line_after": word(globals_after, ACTIVE_LINE),
        "holder_after": word(state_after, related + holder_offset(related_kind)),
        "guard_depth_after": word(globals_after, GUARD_TOP) // 2,
        "cursor": cpu.reg_read(UC_X86_REG_SI),
        "owner_lookup_called": OWNER_HELPER[0] in calls,
        "slot_insert_called": SLOT_HELPER[0] in calls,
        "field_lookup_called": FIELD_HELPER[0] in calls,
        "descriptor_called": descriptor_called,
        "descriptor_result": int(descriptor_called and case["descriptor_available"]),
        "io": io,
    }

    restored_globals = bytearray(globals_after)
    for start, length in allowed_global_writes:
        restored_globals[start : start + length] = globals_before[start : start + length]
    assert restored_globals == globals_before
    restored_state = bytearray(state_after)
    holder = related + holder_offset(related_kind)
    restored_state[holder : holder + 2] = state_before[holder : holder + 2]
    assert restored_state == state_before
    assert descriptor_called == bool(io)
    return result


def expected(case):
    query_mode = case["query_before"] & 1 != 0
    owner_active = case["owner_flags"] & ACTIVE_FLAG != 0
    related_presentable = case["related_flags"] & PRESENTABLE_FLAG != 0
    related_kind = case["related_kind"]
    related = RELATED_OFFSET
    target = case["target_before"]
    operand_size = 4 + int(case["inverted"])

    if query_mode:
        matches = owner_active and target[0] == 0xC2 and target[1] == related
        failed = matches == case["inverted"]
        insert_called = False
        insert_succeeded = False
    else:
        failed = False
        insert_called = owner_active and related_presentable
        insert_succeeded = insert_called and case["roster_mode"] != "full"

    descriptor_called = (
        insert_succeeded
        and case["ui_state"] & 1 == 0
        and case["request_before"] & 2 == 0
        and related_kind == INVENTORY_KIND
    )
    descriptor_succeeded = descriptor_called and case["descriptor_available"]
    actor_presentation = (
        insert_succeeded
        and case["ui_state"] & 1 == 0
        and case["request_before"] & 2 == 0
        and related_kind == ACTOR_KIND
    )

    roster_after = roster_image(case["roster_mode"], related, case["object_offsets"][3:])
    if insert_succeeded and case["roster_mode"] == "free":
        roster_after[0] = related
    return {
        "failed": failed,
        "query_after": 0 if failed else case["query_before"],
        "guard_depth_after": 0 if failed else 1,
        "cursor": BRANCH_TARGET if failed else SCRIPT + operand_size,
        "owner_lookup_called": True,
        "slot_insert_called": insert_called,
        "slot_insert_succeeded": insert_succeeded,
        "field_lookup_called": insert_succeeded,
        "descriptor_called": descriptor_called,
        "descriptor_result": int(descriptor_succeeded),
        "holder_after": 0xFFFF if insert_succeeded else INITIAL_HOLDER,
        "roster_after": roster_after,
        "request_after": case["request_before"] | (2 if descriptor_succeeded else 0),
        "c2_gate_after": 0
        if actor_presentation or descriptor_succeeded
        else INITIAL_C2_GATE,
        "active_line_after": 39
        if actor_presentation
        else 43
        if descriptor_succeeded
        else INITIAL_ACTIVE_LINE,
    }


def query_cases():
    cases = []
    for kind in (ACTOR_KIND, INVENTORY_KIND, LOCATION_KIND):
        records = target_records(RELATED_OFFSET, RELATED_OFFSET + kind_size(kind))
        for query, inverted, variant, active, presentable in itertools.product(
            QUERY_VALUES,
            (False, True),
            records,
            (False, True),
            (False, True),
        ):
            cases.append(
                {
                    "query_before": query,
                    "inverted": inverted,
                    "owner_flags": ACTIVE_FLAG if active else 0,
                    "related_kind": kind,
                    "related_flags": PRESENTABLE_FLAG if presentable else 0,
                    "target_variant": variant,
                    "target_before": records[variant],
                    "roster_mode": "free",
                    "ui_state": 0,
                    "request_before": 0,
                    "descriptor_available": False,
                }
            )
    return cases


def assignment_cases():
    cases = []
    empty = [0, 0, 0]
    for query, inverted, active, presentable, roster, kind in itertools.product(
        ASSIGNMENT_VALUES,
        (False, True),
        (False, True),
        (False, True),
        ("free", "existing", "full"),
        (ACTOR_KIND, INVENTORY_KIND, LOCATION_KIND),
    ):
        cases.append(
            {
                "query_before": query,
                "inverted": inverted,
                "owner_flags": ACTIVE_FLAG if active else 0,
                "related_kind": kind,
                "related_flags": PRESENTABLE_FLAG if presentable else 0,
                "target_variant": "ignored",
                "target_before": empty,
                "roster_mode": roster,
                "ui_state": 0,
                "request_before": 0,
                "descriptor_available": True,
            }
        )

    for query, kind, ui, request, available in itertools.product(
        ASSIGNMENT_VALUES,
        (ACTOR_KIND, INVENTORY_KIND, LOCATION_KIND),
        (0, 1),
        (0, 2, 0x40),
        (False, True),
    ):
        cases.append(
            {
                "query_before": query,
                "inverted": False,
                "owner_flags": ACTIVE_FLAG,
                "related_kind": kind,
                "related_flags": PRESENTABLE_FLAG,
                "target_variant": "presentation_gate",
                "target_before": empty,
                "roster_mode": "free",
                "ui_state": ui,
                "request_before": request,
                "descriptor_available": available,
            }
        )
    return cases


def vectors(executable):
    encoded = struct.unpack_from("<H", executable, DISPATCH_TABLE + (0xC2 - 0xA0) * 2)[0]
    assert encoded == HANDLER[0] - (HEADER + CODE_BASE)
    rows = []
    for case in query_cases() + assignment_cases():
        row = execute(executable, case)
        expected_values = expected(row)
        row["failed"] = expected_values["failed"]
        row["slot_insert_succeeded"] = expected_values["slot_insert_succeeded"]
        for key, value in expected_values.items():
            assert row[key] == value, (case, key, row[key], value)
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
    print(f"captured {len(rows)} original C2 aboard-record cases")


if __name__ == "__main__":
    main()
