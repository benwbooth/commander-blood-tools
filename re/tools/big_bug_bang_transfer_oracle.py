#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited CD transfer handler and entered helpers.

The original owner/field lookups, aboard removal/insertion, guard failure, and
DESCRIPT parser run unmodified. DOS file calls read an owned synthetic database;
no native helper result is patched. Run with ``python3 -P`` so the adjacent
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER = 0x800
CODE_BASE = 0x5020
DISPATCH_TABLE = 0x16A78
HANDLER = (0x75CD, 0x76AD)
REMOVE_HELPER = (0x65E8, 0x6606)
INSERT_HELPER = (0x6606, 0x6633)
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
BRANCH_TARGETS = (0x5AA5, 0x6BB6)

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
SPECIAL_OBJECT = 0x6B1E
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
ROSTER = 0x70E6
UI_STATE = 0x2A33
REQUEST_FLAGS = 0x6B80
C2_GATE = 0x2200
ACTIVE_LINE = 0x6B5A
SCRATCH_POINTER = 0x0CB4

PLAYER_KIND = 0x0001
ACTOR_KIND = 0x0002
LOCATION_KIND = 0x0080
INVENTORY_KIND = 0x0400
ACTIVE_FLAG = 0x0001
ROSTER_CAPACITY = 16
INITIAL_HOLDER = 0x1234
INITIAL_C2_GATE = 0xA5
INITIAL_ACTIVE_LINE = 0x1357


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def kind_size(kind):
    return {
        PLAYER_KIND: 34,
        ACTOR_KIND: 74,
        LOCATION_KIND: 26,
        INVENTORY_KIND: 24,
    }[kind]


def holder_offset(kind):
    return {PLAYER_KIND: 24, ACTOR_KIND: 24, LOCATION_KIND: 20, INVENTORY_KIND: 20}[kind]


def in_range(address, size, span):
    start, end = span
    return start - HEADER <= address and address + size <= end - HEADER


def synthetic_database(name, payload=b"\xff"):
    return struct.pack("<H16sHBH", 1, name, 21, 15, len(payload) + 2) + payload


def object_layout(item_kind):
    kinds = [PLAYER_KIND, ACTOR_KIND, item_kind, LOCATION_KIND, LOCATION_KIND]
    kinds.extend([LOCATION_KIND] * ROSTER_CAPACITY)
    offsets = []
    cursor = 0
    for kind in kinds:
        offsets.append(cursor)
        cursor += kind_size(kind)
    return kinds, offsets, cursor


def directory_image(offsets, state_end):
    encoded = bytearray((len(offsets) + 1) * 20)
    for index, offset in enumerate(offsets):
        entry = index * 20
        name = b"item" if index == 2 else f"object{index}".encode("ascii")
        encoded[entry : entry + len(name)] = name
        struct.pack_into("<HH", encoded, entry + 16, offset, 1)
    struct.pack_into("<H", encoded, len(offsets) * 20 + 16, state_end)
    return encoded


def roster_image(mode, item, fillers):
    if mode == "free":
        return [0] * ROSTER_CAPACITY
    if mode == "existing":
        return [item] + [0] * (ROSTER_CAPACITY - 1)
    if mode == "full":
        return list(fillers)
    raise AssertionError(mode)


def execute(executable, case):
    kinds, offsets, state_end = object_layout(case["item_kind"])
    special, source, item, destination, alternate = offsets[:5]
    fillers = offsets[5:]
    source_record = (special + 24) if case["source"] == "special" else (source + 58)
    destination_operand = special if case["destination"] == "special" else destination
    stored = {
        "exact": (0xCD, item, destination_operand),
        "wrong_kind": (0xC4, item, destination_operand),
        "wrong_item": (0xCD, alternate, destination_operand),
        "wrong_destination": (0xCD, item, alternate),
    }[case["stored_variant"]]
    roster_before = roster_image(case["roster_mode"], item, fillers)

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
    struct.pack_into("<H", globals_before, SPECIAL_OBJECT, special)
    struct.pack_into("<HH", globals_before, SCRATCH_POINTER, 0, SCRATCH // 16)
    struct.pack_into("<H", globals_before, GUARD_TOP, case["guard_depth_before"] * 2)
    struct.pack_into("<2H", globals_before, GUARD_STACK, *BRANCH_TARGETS)
    struct.pack_into(f"<{ROSTER_CAPACITY}H", globals_before, ROSTER, *roster_before)
    token = bytes([0xCD]) + (b"\xA1" if case["inverted"] else b"")
    token += struct.pack("<HHH", source_record, item, destination_operand)
    globals_before[SCRIPT : SCRIPT + len(token)] = token
    cpu.mem_write(GLOBALS, bytes(globals_before))

    state_before = bytearray(SIZE)
    for index, (kind, offset) in enumerate(zip(kinds, offsets)):
        flags = ACTIVE_FLAG
        if index == 0 and destination_operand == special:
            flags = ACTIVE_FLAG if case["destination_active"] else 0
        elif index == 2:
            flags = ACTIVE_FLAG if case["item_active"] else 0
        elif index == 3:
            flags = ACTIVE_FLAG if case["destination_active"] else 0
        struct.pack_into("<HH", state_before, offset, kind, flags)
    state_before[item + 4 : item + 9] = b"item\0"
    struct.pack_into("<H", state_before, item + holder_offset(case["item_kind"]), INITIAL_HOLDER)
    struct.pack_into("<3H", state_before, source_record, *stored)
    cpu.mem_write(STATE, bytes(state_before))

    directory_before = directory_image(offsets, state_end)
    cpu.mem_write(DIRECTORY, bytes(directory_before))
    database_name = b"item" if case["descriptor_available"] else b"other"
    database = synthetic_database(database_name)
    position = 0
    opened = False
    io = []
    calls = {"owner": 0, "field": 0, "remove": 0, "insert": 0, "failure": 0, "descriptor": 0}
    entry_names = {
        OWNER_HELPER[0]: "owner",
        FIELD_HELPER[0]: "field",
        REMOVE_HELPER[0]: "remove",
        INSERT_HELPER[0]: "insert",
        FAILURE_HELPER[0]: "failure",
        0x8450: "descriptor",
    }

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, GLOBALS // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_ES, STATE // 16),
        (UC_X86_REG_SS, GLOBALS // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT + 1),
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
        (STACK - 128, 128),
        (0x2A89, 1),
        (0x156C, 1),
        (0x21FB, 4),
        (0x1568, 2),
        (0x1166, 4),
        (0x0D20, 1),
        (0x0CEB, 1),
        (0x0CA6, 4),
    )
    terminal_reached = False

    def cstring(address):
        return bytes(cpu.mem_read(address, 256)).split(b"\0", 1)[0]

    def instruction(machine, address, size, _context):
        nonlocal terminal_reached
        spans = (HANDLER, REMOVE_HELPER, INSERT_HELPER, FIELD_HELPER, OWNER_HELPER, FAILURE_HELPER)
        assert any(in_range(address, size, span) for span in spans + DESCRIPTOR_RANGES), hex(
            address + HEADER
        )
        file_address = address + HEADER
        if file_address in entry_names:
            calls[entry_names[file_address]] += 1
        if file_address == HANDLER[1] - 1:
            terminal_reached = True
            machine.emu_stop()

    def write_hook(_cpu, _access, address, size, _value, _context):
        if GLOBALS <= address < GLOBALS + SIZE:
            offset = address - GLOBALS
            assert any(
                start <= offset and offset + size <= start + length
                for start, length in allowed_global_writes
            ), hex(address)
            return
        holder = STATE + item + holder_offset(case["item_kind"])
        assert address == holder and size == 2, (hex(address), size)

    def interrupt(_cpu, number, _context):
        nonlocal opened, position
        assert number == 0x21
        ax = cpu.reg_read(UC_X86_REG_AX)
        bx = cpu.reg_read(UC_X86_REG_BX)
        count = cpu.reg_read(UC_X86_REG_CX)
        dx = cpu.reg_read(UC_X86_REG_DX)
        target = cpu.reg_read(UC_X86_REG_DS) * 16 + dx
        if ax == 0x3D00:
            assert cstring(target) == b"descript.des"
            assert not opened
            opened = True
            position = 0
            cpu.reg_write(UC_X86_REG_AX, 5)
            cpu.reg_write(UC_X86_REG_EFLAGS, cpu.reg_read(UC_X86_REG_EFLAGS) & ~1)
            io.append(["open"])
        elif ax == 0x3F00:
            assert opened and bx == 5
            assert (
                target in (GLOBALS + 0x0CA6, GLOBALS + 0x0CA8) and count == 2
                or target == SCRATCH and count <= SIZE
            )
            data = database[position : position + count]
            cpu.mem_write(target, data)
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
    cpu.emu_start(HANDLER[0] - HEADER, 0x100000, count=30000)
    assert terminal_reached
    assert cpu.reg_read(UC_X86_REG_SP) == STACK
    assert cpu.reg_read(UC_X86_REG_DS) == GLOBALS // 16
    assert cpu.reg_read(UC_X86_REG_ES) == STATE // 16
    assert not opened
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(DIRECTORY, len(directory_before))) == directory_before

    globals_after = bytes(cpu.mem_read(GLOBALS, SIZE))
    state_after = bytes(cpu.mem_read(STATE, SIZE))
    result = {
        **case,
        "object_offsets": offsets,
        "source_record": source_record,
        "item_offset": item,
        "destination_offset": destination_operand,
        "alternate_offset": alternate,
        "stored_kind": stored[0],
        "stored_item": stored[1],
        "stored_destination": stored[2],
        "roster_before": roster_before,
        "roster_after": list(struct.unpack_from(f"<{ROSTER_CAPACITY}H", globals_after, ROSTER)),
        "holder_before": INITIAL_HOLDER,
        "holder_after": word(state_after, item + holder_offset(case["item_kind"])),
        "query_after": globals_after[QUERY],
        "guard_depth_after": word(globals_after, GUARD_TOP) // 2,
        "request_after": globals_after[REQUEST_FLAGS],
        "c2_gate_after": globals_after[C2_GATE],
        "active_line_after": word(globals_after, ACTIVE_LINE),
        "cursor": cpu.reg_read(UC_X86_REG_SI),
        "calls": calls,
        "io": io,
    }

    restored_globals = bytearray(globals_after)
    for start, length in allowed_global_writes:
        restored_globals[start : start + length] = globals_before[start : start + length]
    assert restored_globals == globals_before
    restored_state = bytearray(state_after)
    holder = item + holder_offset(case["item_kind"])
    restored_state[holder : holder + 2] = state_before[holder : holder + 2]
    assert restored_state == state_before
    assert bool(io) == bool(calls["descriptor"])
    return result


def expected(row):
    query = row["query_before"] & 1 != 0
    if query:
        matches = (
            row["stored_kind"] == 0xCD
            and row["stored_item"] == row["item_offset"]
            and row["stored_destination"] == row["destination_offset"]
        )
        failed = matches == row["inverted"]
        return {
            "failed": failed,
            "holder_changed": False,
            "descriptor_checked": False,
            "presentation_requested": False,
            "roster_after": row["roster_before"],
            "holder_after": row["holder_before"],
            "query_after": 0 if failed else row["query_before"],
            "guard_depth_after": row["guard_depth_before"] - int(failed),
            "request_after": row["request_before"],
            "c2_gate_after": INITIAL_C2_GATE,
            "active_line_after": INITIAL_ACTIVE_LINE,
            "cursor": BRANCH_TARGETS[row["guard_depth_before"] - 1]
            if failed
            else SCRIPT + 7 + int(row["inverted"]),
            "calls": {
                "owner": 0,
                "field": 0,
                "remove": 0,
                "insert": 0,
                "failure": int(failed),
                "descriptor": 0,
            },
        }

    roster = list(row["roster_before"])
    remove_called = row["source"] == "special"
    if remove_called and row["item_offset"] in roster:
        roster[roster.index(row["item_offset"])] = 0
    insert_called = row["destination"] == "special"
    insert_success = False
    if insert_called:
        if row["item_offset"] in roster:
            insert_success = True
        elif 0 in roster:
            roster[roster.index(0)] = row["item_offset"]
            insert_success = True
    holder_changed = not insert_called or insert_success
    holder_after = row["holder_before"]
    if holder_changed:
        holder_after = 0xFFFF if insert_called else row["destination_offset"]
    descriptor_checked = (
        holder_changed
        and row["ui_state"] & 1 == 0
        and row["request_before"] & 2 == 0
        and row["item_kind"] == INVENTORY_KIND
    )
    presentation_requested = descriptor_checked and row["descriptor_available"]
    return {
        "failed": False,
        "holder_changed": holder_changed,
        "descriptor_checked": descriptor_checked,
        "presentation_requested": presentation_requested,
        "roster_after": roster,
        "holder_after": holder_after,
        "query_after": row["query_before"],
        "guard_depth_after": row["guard_depth_before"],
        "request_after": row["request_before"] | (2 if presentation_requested else 0),
        "c2_gate_after": 0 if presentation_requested else INITIAL_C2_GATE,
        "active_line_after": 43 if presentation_requested else INITIAL_ACTIVE_LINE,
        "cursor": SCRIPT + 7,
        "calls": {
            "owner": 1,
            "field": 2,
            "remove": int(remove_called),
            "insert": int(insert_called),
            "failure": 0,
            "descriptor": int(descriptor_checked),
        },
    }


def query_cases():
    for query, inverted, stored, source, depth in itertools.product(
        (1, 3, 0xFF),
        (False, True),
        ("exact", "wrong_kind", "wrong_item", "wrong_destination"),
        ("special", "ordinary"),
        (1, 2),
    ):
        yield {
            "query_before": query,
            "inverted": inverted,
            "source": source,
            "destination": "ordinary",
            "item_kind": INVENTORY_KIND,
            "item_active": True,
            "destination_active": True,
            "stored_variant": stored,
            "roster_mode": "free",
            "ui_state": 0,
            "request_before": 0,
            "descriptor_available": False,
            "guard_depth_before": depth,
        }


def assignment_cases():
    for query, source, destination, kind, item_active, destination_active, roster in itertools.product(
        (0, 2, 0xFE),
        ("special", "ordinary"),
        ("special", "ordinary"),
        (ACTOR_KIND, INVENTORY_KIND, LOCATION_KIND),
        (False, True),
        (False, True),
        ("free", "existing", "full"),
    ):
        yield {
            "query_before": query,
            "inverted": False,
            "source": source,
            "destination": destination,
            "item_kind": kind,
            "item_active": item_active,
            "destination_active": destination_active,
            "stored_variant": "wrong_kind",
            "roster_mode": roster,
            "ui_state": 0,
            "request_before": 0,
            "descriptor_available": True,
            "guard_depth_before": 1,
        }
    for query, ui, request, available in itertools.product(
        (0, 2, 0xFE), (0, 1, 2, 0xFF), (0, 2, 0x40), (False, True)
    ):
        yield {
            "query_before": query,
            "inverted": False,
            "source": "ordinary",
            "destination": "ordinary",
            "item_kind": INVENTORY_KIND,
            "item_active": True,
            "destination_active": True,
            "stored_variant": "wrong_kind",
            "roster_mode": "free",
            "ui_state": ui,
            "request_before": request,
            "descriptor_available": available,
            "guard_depth_before": 1,
        }


def vectors(executable):
    encoded = struct.unpack_from("<H", executable, DISPATCH_TABLE + (0xCD - 0xA0) * 2)[0]
    assert encoded == HANDLER[0] - (HEADER + CODE_BASE)
    rows = []
    for case in itertools.chain(query_cases(), assignment_cases()):
        row = execute(executable, case)
        expectation = expected(row)
        row["failed"] = expectation["failed"]
        row["holder_changed"] = expectation["holder_changed"]
        row["descriptor_checked"] = expectation["descriptor_checked"]
        row["presentation_requested"] = expectation["presentation_requested"]
        for key, value in expectation.items():
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
        "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows)
    )
    print(f"captured {len(rows)} original CD transfer cases")


if __name__ == "__main__":
    main()
