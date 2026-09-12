#!/usr/bin/env python3
"""Execute Big Bug Bang's inherited C3-C8 action-record handlers.

The original handlers, field and owner lookups, and guard-failure helper run
unmodified under Unicorn. Rows contain semantic inputs and outputs only. Run
with ``python3 -P`` so the adjacent dis.py cannot shadow the standard library.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EFLAGS,
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
HANDLERS = {
    0xC3: (0x7AF4, 0x7B68),
    0xC4: (0x7884, 0x791E),
    0xC5: (0x791E, 0x7986),
    0xC6: (0x7986, 0x79D5),
    0xC7: (0x79D5, 0x7A3A),
    0xC8: (0x7B68, 0x7BBF),
}
FIELD_HELPER = (0x6633, 0x6644)
OWNER_HELPER = (0x6644, 0x665E)
FAILURE_HELPER = (0x697A, 0x6993)
GLOBALS = 0x30000
SOURCE = 0x40000
STATE = 0x50000
STACK_MEMORY = 0x60000
DIRECTORY = 0x70000
SCRIPT = 0x40
STACK = 0xFF00
RETURN = 0x1800
BRANCH_TARGET = 0x5AA5

STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
QUERY = 0x6B83
GUARD_STACK = 0x6C04
GUARD_TOP = 0x6C2C
FIELD_MATRIX = 0x7128
FIELD_MATRIX_FILE = 0x16918
FIELD_MATRIX_SIZE = 21 * 16

PLAYER_KIND = 0x0001
ACTOR_KIND = 0x0002
WORLD_STATE_KIND = 0x0200
LOCATION_KIND = 0x0080
ACTIVE_FLAG = 0x0001
OWNER_HELPER_OPCODES = (0xC3, 0xC4)
QUERY_VALUES = (1, 3, 0xFF)
ASSIGNMENT_VALUES = (0, 2, 0xFE)


def kind_size(kind):
    return {
        PLAYER_KIND: 34,
        ACTOR_KIND: 74,
        WORLD_STATE_KIND: 38,
        LOCATION_KIND: 26,
    }[kind]


def action_offset(kind):
    return {PLAYER_KIND: 8, ACTOR_KIND: 58}.get(kind)


def in_range(address, size, span):
    start, end = span
    start -= HEADER
    end -= HEADER
    return start <= address and address + size <= end


def directory_image(offsets):
    encoded = bytearray(80)
    for index, offset in enumerate(offsets[:-1]):
        entry = index * 20
        name = f"object{index}".encode("ascii")
        encoded[entry:entry + len(name)] = name
        struct.pack_into("<HH", encoded, entry + 16, offset, 1)
    struct.pack_into("<H", encoded, 3 * 20 + 16, offsets[-1])
    return encoded


def other_kind(opcode):
    return 0xC5 if opcode != 0xC5 else 0xC6


def query_records(opcode, related, alternate):
    return {
        "exact": [opcode, related, 0],
        "exact_other_third": [opcode, related, 0xA55A],
        "other_relation": [opcode, alternate, 0],
        "other_kind": [other_kind(opcode), related, 0],
        "empty": [0, 0, 0],
    }


def assignment_records(opcode, related, alternate):
    common = {
        "empty": [0, 0, 0],
        "occupied": [0x9999, alternate, 0xA55A],
        "actor": [0xC4, alternate, 0],
    }
    if opcode == 0xC3:
        common["queued"] = [0xC3, alternate, 1]
    elif opcode == 0xC6:
        common["travel"] = [0xC6, alternate, 0]
    elif opcode == 0xC7:
        common["active_object"] = [0xC7, alternate, 0]
    elif opcode == 0xC8:
        common["marker"] = [0xC8, alternate, 0]
    return common


def execute(executable, case):
    opcode = case["opcode"]
    handler = HANDLERS[opcode]
    owner_kind = case["owner_kind"]
    related_kind = case["related_kind"]
    owner_size = kind_size(owner_kind)
    related_size = kind_size(related_kind)
    owner = 0
    related = owner_size
    alternate = related + related_size
    state_end = alternate + kind_size(LOCATION_KIND)
    target = owner + action_offset(owner_kind)
    reciprocal = action_offset(related_kind)
    reciprocal = None if reciprocal is None else related + reciprocal

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER:]
    cpu.mem_write(0, module)

    globals_before = bytearray(0x10000)
    globals_before[QUERY] = case["query_before"]
    globals_before[FIELD_MATRIX:FIELD_MATRIX + FIELD_MATRIX_SIZE] = executable[
        FIELD_MATRIX_FILE:FIELD_MATRIX_FILE + FIELD_MATRIX_SIZE
    ]
    struct.pack_into("<HH", globals_before, STATE_POINTER, 0, STATE // 16)
    struct.pack_into("<HH", globals_before, DIRECTORY_POINTER, 20, DIRECTORY // 16)
    struct.pack_into("<H", globals_before, GUARD_TOP, 2)
    cpu.mem_write(GLOBALS, bytes(globals_before))

    operand = (b"\xA1" if case["inverted"] else b"")
    operand += struct.pack("<HH", target, related)
    cpu.mem_write(SOURCE + SCRIPT, operand)

    state_before = bytearray(0x10000)
    struct.pack_into("<HH", state_before, owner, owner_kind, case["owner_flags"])
    struct.pack_into(
        "<HH", state_before, related, related_kind, case["related_flags"]
    )
    struct.pack_into("<HH", state_before, alternate, LOCATION_KIND, ACTIVE_FLAG)
    struct.pack_into("<3H", state_before, target, *case["target_before"])
    if reciprocal is not None:
        struct.pack_into("<3H", state_before, reciprocal, *case["reciprocal_before"])
    cpu.mem_write(STATE, bytes(state_before))

    directory_before = directory_image((owner, related, alternate, state_end))
    cpu.mem_write(DIRECTORY, bytes(directory_before))
    cpu.mem_write(STACK_MEMORY + GUARD_STACK, struct.pack("<H", BRANCH_TARGET))
    cpu.mem_write(STACK_MEMORY + STACK, struct.pack("<H", RETURN))

    for register, value in (
        (UC_X86_REG_CS, CODE_BASE // 16),
        (UC_X86_REG_DS, SOURCE // 16),
        (UC_X86_REG_GS, GLOBALS // 16),
        (UC_X86_REG_SS, STACK_MEMORY // 16),
        (UC_X86_REG_SP, STACK),
        (UC_X86_REG_SI, SCRIPT),
        (UC_X86_REG_EFLAGS, 2),
    ):
        cpu.reg_write(register, value)

    allowed_writes = (
        (STATE + target, 6),
        (GLOBALS + QUERY, 1),
        (GLOBALS + GUARD_TOP, 2),
        (STACK_MEMORY + STACK - 16, 18),
    )
    owner_lookup_called = False
    field_lookup_called = False

    def instruction(_cpu, address, size, _context):
        nonlocal owner_lookup_called, field_lookup_called
        assert any(
            in_range(address, size, span)
            for span in (handler, FIELD_HELPER, OWNER_HELPER, FAILURE_HELPER)
        ), hex(address + HEADER)
        owner_lookup_called |= address + HEADER == OWNER_HELPER[0]
        field_lookup_called |= address + HEADER == FIELD_HELPER[0]

    def write_hook(_cpu, _access, address, size, _value, _context):
        assert any(
            first <= address and address + size <= first + length
            for first, length in allowed_writes
        ), hex(address)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(handler[0] - HEADER, CODE_BASE + RETURN, count=1000)
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN
    assert cpu.reg_read(UC_X86_REG_SP) == STACK + 2
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert bytes(cpu.mem_read(SOURCE + SCRIPT, len(operand))) == operand
    assert bytes(cpu.mem_read(DIRECTORY, len(directory_before))) == directory_before

    globals_after = bytes(cpu.mem_read(GLOBALS, len(globals_before)))
    state_after = bytes(cpu.mem_read(STATE, len(state_before)))
    target_after = list(struct.unpack_from("<3H", state_after, target))
    reciprocal_after = (
        None
        if reciprocal is None
        else list(struct.unpack_from("<3H", state_after, reciprocal))
    )
    return {
        **case,
        "target_offset": target,
        "related_offset": related,
        "alternate_offset": alternate,
        "reciprocal_offset": reciprocal,
        "target_after": target_after,
        "reciprocal_after": reciprocal_after,
        "query_after": globals_after[QUERY],
        "guard_depth_after": struct.unpack_from("<H", globals_after, GUARD_TOP)[0]
        // 2,
        "cursor": cpu.reg_read(UC_X86_REG_SI),
        "owner_lookup_called": owner_lookup_called,
        "field_lookup_called": field_lookup_called,
    }


def expected(case):
    opcode = case["opcode"]
    query_mode = case["query_before"] & 1 != 0
    target = case["target_before"]
    owner_active = case["owner_flags"] & ACTIVE_FLAG != 0
    related_active = case["related_flags"] & ACTIVE_FLAG != 0
    owner_kind = case["owner_kind"]
    related_kind = case["related_kind"]
    related = kind_size(owner_kind)
    reciprocal = (
        case["reciprocal_before"]
        if action_offset(related_kind) is not None
        else None
    )

    if query_mode:
        matches = target[0] == opcode and target[1] == related
        if opcode in OWNER_HELPER_OPCODES:
            matches &= owner_active
        failed = matches == case["inverted"]
        target_after = target
    else:
        if opcode == 0xC3:
            failed = not owner_active or not related_active or target[0] == 0xC4
            target_after = [0xC3, related, 1]
        elif opcode == 0xC4:
            reciprocal_actor = reciprocal is not None and reciprocal[0] == 0xC4
            failed = (
                not owner_active
                or not related_active
                or (
                    owner_kind != PLAYER_KIND
                    and related_kind != PLAYER_KIND
                    and (target[0] == 0xC4 or reciprocal_actor)
                )
            )
            target_after = [0xC4, related, 0]
        elif opcode == 0xC5:
            failed = not related_active or related_kind != WORLD_STATE_KIND or target[0] != 0
            target_after = [0xC5, related, 0]
        elif opcode == 0xC6:
            failed = False
            target_after = [0xC6, related, 0]
        elif opcode == 0xC7:
            failed = not related_active or target[0] not in (0, 0xC4)
            target_after = [0xC7, related, 0]
        else:
            failed = target[0] != 0
            target_after = [0xC8, 0, 0]
        if failed:
            target_after = target

    operand_size = 5 + int(case["inverted"])
    return {
        "target_after": target_after,
        "reciprocal_after": reciprocal,
        "query_after": 0 if failed else case["query_before"],
        "guard_depth_after": 0 if failed else 1,
        "cursor": BRANCH_TARGET if failed else SCRIPT + operand_size - 1,
        "failed": failed,
    }


def query_cases():
    cases = []
    for opcode in HANDLERS:
        owner_flags = (0, ACTIVE_FLAG) if opcode in OWNER_HELPER_OPCODES else (ACTIVE_FLAG,)
        related_kind = WORLD_STATE_KIND if opcode == 0xC5 else ACTOR_KIND
        owner_size = kind_size(ACTOR_KIND)
        related = owner_size
        alternate = related + kind_size(related_kind)
        for query, inverted, target_variant, owner_flag, related_flag in itertools.product(
            QUERY_VALUES,
            (False, True),
            query_records(opcode, related, alternate),
            owner_flags,
            (0, ACTIVE_FLAG),
        ):
            cases.append(
                {
                    "opcode": opcode,
                    "query_before": query,
                    "inverted": inverted,
                    "owner_kind": ACTOR_KIND,
                    "owner_flags": owner_flag,
                    "related_kind": related_kind,
                    "related_flags": related_flag,
                    "target_variant": target_variant,
                    "reciprocal_variant": "empty",
                    "target_before": query_records(opcode, related, alternate)[target_variant],
                    "reciprocal_before": [0, 0, 0],
                }
            )
    return cases


def assignment_cases():
    cases = []
    for opcode in HANDLERS:
        if opcode == 0xC4:
            kind_pairs = itertools.product(
                (ACTOR_KIND, PLAYER_KIND),
                (ACTOR_KIND, PLAYER_KIND, WORLD_STATE_KIND),
            )
        elif opcode == 0xC5:
            kind_pairs = ((ACTOR_KIND, kind) for kind in (WORLD_STATE_KIND, ACTOR_KIND))
        else:
            kind_pairs = ((ACTOR_KIND, ACTOR_KIND),)

        for owner_kind, related_kind in kind_pairs:
            owner_size = kind_size(owner_kind)
            related = owner_size
            alternate = related + kind_size(related_kind)
            records = assignment_records(opcode, related, alternate)
            reciprocal_variants = (
                ("empty", [0, 0, 0]),
                ("actor", [0xC4, 0, 0]),
            )
            if opcode != 0xC4 or action_offset(related_kind) is None:
                reciprocal_variants = (("empty", [0, 0, 0]),)
            owner_flags = (0, ACTIVE_FLAG) if opcode in OWNER_HELPER_OPCODES else (ACTIVE_FLAG,)
            for query, owner_flag, related_flag, target_variant, reciprocal in itertools.product(
                ASSIGNMENT_VALUES,
                owner_flags,
                (0, ACTIVE_FLAG),
                records,
                reciprocal_variants,
            ):
                reciprocal_variant, reciprocal_before = reciprocal
                cases.append(
                    {
                        "opcode": opcode,
                        "query_before": query,
                        "inverted": False,
                        "owner_kind": owner_kind,
                        "owner_flags": owner_flag,
                        "related_kind": related_kind,
                        "related_flags": related_flag,
                        "target_variant": target_variant,
                        "reciprocal_variant": reciprocal_variant,
                        "target_before": records[target_variant],
                        "reciprocal_before": reciprocal_before,
                    }
                )
    return cases


def vectors(executable):
    for opcode, handler in HANDLERS.items():
        encoded = struct.unpack_from(
            "<H", executable, DISPATCH_TABLE + (opcode - 0xA0) * 2
        )[0]
        assert encoded == handler[0] - (HEADER + CODE_BASE)

    rows = []
    for case in query_cases() + assignment_cases():
        row = execute(executable, case)
        expected_values = expected(case)
        row["failed"] = expected_values["failed"]
        for key, value in expected_values.items():
            assert row[key] == value, (case, key, row[key], value)
        assert row["owner_lookup_called"] == (case["opcode"] in OWNER_HELPER_OPCODES)
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
    print(f"captured {len(rows)} original C3-C8 action-record cases")


if __name__ == "__main__":
    main()
