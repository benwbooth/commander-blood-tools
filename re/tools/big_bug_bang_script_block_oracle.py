#!/usr/bin/env python3
"""Verify BBB's nested BloodScript block dispatcher against Commander."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
from unicorn.x86_const import (
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_script_block.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_56a6_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "984e3edcccb9022089e6c69f87cbf4fbb5752c72df16653b986cbab213152e4c"
)

ENTRY = 0x5B65
END = 0x5BAE
BODY_SHA256 = "517e1eeea2d389d0bda774b4728ef2f26f88a3f19f8c45ed8ce16a8873bce602"
HANDLER_TABLE_FILE_OFFSET = 0x16A78
HANDLER_TABLE_OFFSET = 0x7288
HANDLER_COUNT = 56
HANDLER_TABLE_SHA256 = (
    "efd6b9d917315c89b43b2ece38f1d54a67192c8f38e1e25478acb4d426e6a9fe"
)
DESCRIPTOR_TABLE_FILE_OFFSET = 0x16AEA
DESCRIPTOR_TABLE_OFFSET = 0x72FA
DESCRIPTOR_TABLE_SHA256 = (
    "ba00a3aadce0293b6a7ffbb7dcd315677f0a574019ad29c79ff370ede028d679"
)
FIRST_OPCODE = 0xA0
LAST_COMMANDER_OPCODE = 0xD2
LAST_SEQUEL_OPCODE = 0xD7

SIGNAL_OFFSET = 0x6B8A
SKIP_OFFSET = 0x6B81
QUERY_OFFSET = 0x6B83
COMMANDER_SIGNAL_OFFSET = 0x67B4
COMMANDER_SKIP_OFFSET = 0x67AB
COMMANDER_QUERY_OFFSET = 0x67AD
COMMANDER_HANDLER_TABLE_OFFSET = 0x6EB0
COMMANDER_DESCRIPTOR_TABLE_OFFSET = 0x6F18

SKIP_HELPER_ENTRY = 0x68C8
SKIP_HELPER_END = 0x694B
SKIP_HELPER_SHA256 = "018bec58a3b0150c066938fa65fe5124134844e77afd5e4fed7ecfca4e32becc"
HANDLER_ENTRY = 0xE000
DECOY_HANDLER_ENTRY = 0xE100
RETURN_OFFSET = 0xF410
DATA_SEGMENT = 0x3000
GAME_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xF800
STACK_TRANSIENT_START = CALLER_SP - 8
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
STATUS_MASK = 0x0CD5

EXCLUDED_COMMANDER_CASES = {
    "invalid_below_range",
    "d3_table_sentinel_is_not_executable",
}

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


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
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


def commander_vectors() -> list[dict[str, Any]]:
    encoded = COMMANDER_FIXTURE.read_bytes()
    assert sha256(encoded) == COMMANDER_FIXTURE_SHA256
    rows = json.loads(encoded)
    assert len(rows) == 15
    return [row for row in rows if row["name"] not in EXCLUDED_COMMANDER_CASES]


def handler_code(name: str) -> bytes:
    if name == "handler_stop_signal":
        return bytes.fromhex("65c6068a6b0165c606816b07c3")
    if name == "handler_resume_signal_clears_skip":
        return bytes.fromhex("65c6068a6b0265c606816b09c3")
    if name == "skip_two_single_byte_tokens":
        return bytes.fromhex("65c606816b02c3")
    if name == "skip_variable_length_token":
        return bytes.fromhex("65c606816b01c3")
    if name == "handler_threads_cursor":
        return bytes.fromhex("83c602c3")
    return b"\xc3"


def apply_event(
    target: bytearray,
    segment: int,
    offset: int,
    size: int,
    value: int,
    events: list[tuple[int, int, int]],
) -> None:
    struct.pack_into({1: "<B", 2: "<H"}[size], target, offset, value)
    events.append((physical_address(segment, offset), size, value))


def expected_execution_events(
    row: dict[str, Any],
    game: bytearray,
    stack: bytearray,
    initial: dict[str, int],
) -> list[tuple[int, int, int]]:
    events: list[tuple[int, int, int]] = []
    script = bytes.fromhex(row["script_hex"])
    direction = -1 if row["reverse_direction"] else 1
    start = int(row["start_offset"])
    script_bytes = {
        (start + direction * index) & 0xFFFF: value
        for index, value in enumerate(script)
    }
    cursor = start
    signal = int(row["yield_before"])
    skip = int(row["skip_before"])
    query = int(row["query_before"])
    plan = str(row["name"])

    while True:
        opcode = script_bytes[cursor]
        cursor = (cursor + direction) & 0xFFFF
        if opcode == 0xFF:
            break
        index = opcode - FIRST_OPCODE
        assert 0 <= index < HANDLER_COUNT
        apply_event(game, GAME_SEGMENT, SIGNAL_OFFSET, 1, 0, events)
        signal = 0
        apply_event(stack, STACK_SEGMENT, CALLER_SP - 2, 2, 0x5B7E, events)

        if plan == "handler_stop_signal":
            apply_event(game, GAME_SEGMENT, SIGNAL_OFFSET, 1, 1, events)
            apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, 7, events)
            signal, skip = 1, 7
        elif plan == "handler_resume_signal_clears_skip":
            apply_event(game, GAME_SEGMENT, SIGNAL_OFFSET, 1, 2, events)
            apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, 9, events)
            signal, skip = 2, 9
        elif plan == "skip_two_single_byte_tokens":
            apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, 2, events)
            skip = 2
        elif plan == "skip_variable_length_token":
            apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, 1, events)
            skip = 1
        elif plan == "handler_threads_cursor":
            cursor = (cursor + 2) & 0xFFFF

        if signal == 0:
            if skip & 0x0F:
                while skip:
                    token = script_bytes[cursor]
                    apply_event(
                        stack,
                        STACK_SEGMENT,
                        CALLER_SP - 2,
                        2,
                        0x5B91,
                        events,
                    )
                    apply_event(
                        stack,
                        STACK_SEGMENT,
                        CALLER_SP - 4,
                        2,
                        initial["eax"] & 0xFF00,
                        events,
                    )
                    apply_event(
                        stack,
                        STACK_SEGMENT,
                        CALLER_SP - 6,
                        2,
                        index * 2,
                        events,
                    )
                    apply_event(
                        stack,
                        STACK_SEGMENT,
                        CALLER_SP - 8,
                        2,
                        initial["ebp"] & 0xFFFF,
                        events,
                    )
                    if token == 0xA0:
                        apply_event(game, GAME_SEGMENT, QUERY_OFFSET, 1, 1, events)
                        query = 1
                        cursor = (cursor + 3) & 0xFFFF
                    elif token == 0xA1:
                        apply_event(game, GAME_SEGMENT, QUERY_OFFSET, 1, 0, events)
                        query = 0
                        cursor = (cursor + 1) & 0xFFFF
                    else:
                        raise AssertionError((plan, hex(token)))
                    skip = (skip - 1) & 0xFF
                    apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, skip, events)
        elif signal == 1:
            break
        else:
            skip = 0
            apply_event(game, GAME_SEGMENT, SKIP_OFFSET, 1, 0, events)

    assert cursor == row["final_cursor"], row["name"]
    assert signal == row["yield_after"], row["name"]
    assert skip == row["skip_after"], row["name"]
    assert query == row["query_after"], row["name"]
    return events


def vectors(executable: bytes) -> list[dict[str, Any]]:
    descriptor_table = executable[
        DESCRIPTOR_TABLE_FILE_OFFSET : DESCRIPTOR_TABLE_FILE_OFFSET + HANDLER_COUNT * 2
    ]
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = str(expected["name"])
        start = int(expected["start_offset"])
        reverse = bool(expected["reverse_direction"])
        script = bytes.fromhex(expected["script_hex"])
        direction = -1 if reverse else 1
        handlers = [int(opcode) for opcode in expected["handler_opcodes"]]
        code = handler_code(name)
        initial_flags = 0x0602 if reverse else 0x0202
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }

        data_before = seeded_segment(case_index, 13, 0x11)
        extra_before = seeded_segment(case_index, 17, 0x23)
        fs_before = seeded_segment(case_index, 19, 0x35)
        game_before = seeded_segment(case_index, 23, 0x47)
        game_expected = bytearray(game_before)
        stack_before = seeded_segment(case_index, 29, 0x59)
        stack_expected = bytearray(stack_before)

        handler_table = [DECOY_HANDLER_ENTRY] * HANDLER_COUNT
        for opcode in handlers:
            handler_table[opcode - FIRST_OPCODE] = HANDLER_ENTRY
        handler_table_bytes = struct.pack(f"<{HANDLER_COUNT}H", *handler_table)
        game_before[
            HANDLER_TABLE_OFFSET : HANDLER_TABLE_OFFSET + len(handler_table_bytes)
        ] = handler_table_bytes
        game_expected[:] = game_before
        for offset, value in (
            (SIGNAL_OFFSET, int(expected["yield_before"])),
            (SKIP_OFFSET, int(expected["skip_before"])),
            (QUERY_OFFSET, int(expected["query_before"])),
        ):
            game_before[offset] = value
            game_expected[offset] = value
        game_before[
            COMMANDER_HANDLER_TABLE_OFFSET : COMMANDER_HANDLER_TABLE_OFFSET
            + len(handler_table_bytes)
        ] = bytes(value ^ 0xA5 for value in handler_table_bytes)
        game_expected[:] = game_before
        for offset in (
            COMMANDER_SIGNAL_OFFSET,
            COMMANDER_SKIP_OFFSET,
            COMMANDER_QUERY_OFFSET,
        ):
            game_before[offset] = 0xA5 ^ case_index
            game_expected[offset] = 0xA5 ^ case_index

        for target in (data_before, extra_before, fs_before):
            target[HANDLER_TABLE_OFFSET : HANDLER_TABLE_OFFSET + 4] = (
                b"\x5a\xa5\xc3\x3c"
            )
            target[SIGNAL_OFFSET] = 0x5A
            target[SKIP_OFFSET] = 0xA5
            target[QUERY_OFFSET] = 0xC3
        stack_before[
            DESCRIPTOR_TABLE_OFFSET : DESCRIPTOR_TABLE_OFFSET + len(descriptor_table)
        ] = descriptor_table
        stack_before[
            COMMANDER_DESCRIPTOR_TABLE_OFFSET : COMMANDER_DESCRIPTOR_TABLE_OFFSET + 8
        ] = bytes.fromhex("5aa5c33c96697887")
        struct.pack_into("<H", stack_before, CALLER_SP, RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected[:] = stack_before
        for index, value in enumerate(script):
            data_before[(start + direction * index) & 0xFFFF] = value

        expected_events = expected_execution_events(
            expected, game_expected, stack_expected, initial
        )
        patched = bytearray(executable)
        patched[HANDLER_ENTRY : HANDLER_ENTRY + len(code)] = code
        patched[DECOY_HANDLER_ENTRY] = 0xC3
        patched[RETURN_OFFSET] = 0xCC

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, bytes(patched))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        module_before = bytes(machine.mem_read(0, len(patched)))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, initial_flags)

        handler_calls: list[dict[str, int]] = []
        token_calls: list[dict[str, int]] = []
        reached_return: list[int] = []
        observed_events: list[tuple[int, int, int]] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls: list[dict[str, int]] = handler_calls,
            skips: list[dict[str, int]] = token_calls,
            reached: list[int] = reached_return,
            case_name: str = name,
            handler_size: int = len(code),
        ) -> None:
            if address == RETURN_OFFSET:
                reached.append(address)
                cpu.emu_stop()
                return
            if address == HANDLER_ENTRY:
                calls.append(
                    {
                        "opcode": FIRST_OPCODE
                        + (cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF) // 2,
                        "cursor": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "signal": cpu.mem_read(
                            physical_address(GAME_SEGMENT, SIGNAL_OFFSET), 1
                        )[0],
                    }
                )
                assert cpu.reg_read(UC_X86_REG_SP) == CALLER_SP - 2, case_name
                frame = struct.unpack(
                    "<H",
                    cpu.mem_read(physical_address(STACK_SEGMENT, CALLER_SP - 2), 2),
                )[0]
                assert frame == 0x5B7E, case_name
                return
            if HANDLER_ENTRY < address < HANDLER_ENTRY + handler_size:
                return
            if address == DECOY_HANDLER_ENTRY:
                raise AssertionError(f"{case_name}: decoy handler dispatched")
            if address == SKIP_HELPER_ENTRY:
                skips.append(
                    {
                        "cursor": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "skip": cpu.mem_read(
                            physical_address(GAME_SEGMENT, SKIP_OFFSET), 1
                        )[0],
                    }
                )
                assert cpu.reg_read(UC_X86_REG_SP) == CALLER_SP - 2, case_name
                return
            if ENTRY <= address < END or SKIP_HELPER_ENTRY < address < SKIP_HELPER_END:
                return
            raise AssertionError(hex(address))

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            events: list[tuple[int, int, int]] = observed_events,
        ) -> None:
            events.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=200_000)

        assert reached_return == [RETURN_OFFSET], name
        assert observed_events == expected_events, (
            name,
            observed_events,
            expected_events,
        )
        assert [call["opcode"] for call in handler_calls] == handlers, name
        assert [call["cursor"] for call in handler_calls] == expected[
            "handler_cursors"
        ], name
        assert all(call["signal"] == 0 for call in handler_calls), name
        assert [call["cursor"] for call in token_calls] == expected[
            "token_advance_offsets"
        ], name
        assert [call["skip"] for call in token_calls] == expected[
            "token_skip_values"
        ], name

        last_bx = initial["ebx"] & 0xFFFF
        if handlers:
            last_bx = (handlers[-1] - FIRST_OPCODE) * 2
        expected_registers = dict(initial)
        expected_registers.update(
            eax=initial["eax"] & 0xFFFF0000,
            ebx=initial["ebx"] & 0xFFFF0000 | last_bx,
            esi=initial["esi"] & 0xFFFF0000 | int(expected["final_cursor"]),
            edi=initial["edi"] & 0xFFFF0000 | HANDLER_TABLE_OFFSET,
        )
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2, name
        assert machine.reg_read(UC_X86_REG_CS) == 0, name
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, name
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        assert flags & STATUS_MASK == expected["defined_status_flags"], name
        assert bool(flags & 0x0200), name
        assert bytes(machine.mem_read(0, len(patched))) == module_before, name
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        ), name
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        ), name
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        ), name
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        ), name
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        ), name
        assert (
            bytes(
                machine.mem_read(
                    physical_address(STACK_SEGMENT, CALLER_SP + 2),
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        ), name
        assert all(
            physical_address(STACK_SEGMENT, STACK_TRANSIENT_START)
            <= address
            < physical_address(STACK_SEGMENT, CALLER_SP)
            or address
            in {
                physical_address(GAME_SEGMENT, SIGNAL_OFFSET),
                physical_address(GAME_SEGMENT, SKIP_OFFSET),
                physical_address(GAME_SEGMENT, QUERY_OFFSET),
            }
            for address, _size, _value in observed_events
        ), name

        row = dict(expected)
        row["handler_calls"] = handler_calls
        row["token_calls"] = token_calls
        row["ordered_write_events"] = [list(event) for event in observed_events]
        row["final_interrupt_flag"] = bool(flags & 0x0200)
        row["final_direction_flag"] = bool(flags & 0x0400)
        rows.append(row)
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = sha256(executable)
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    checks = (
        ("block dispatcher", ENTRY, END, BODY_SHA256),
        ("skip helper", SKIP_HELPER_ENTRY, SKIP_HELPER_END, SKIP_HELPER_SHA256),
        (
            "handler table",
            HANDLER_TABLE_FILE_OFFSET,
            HANDLER_TABLE_FILE_OFFSET + HANDLER_COUNT * 2,
            HANDLER_TABLE_SHA256,
        ),
        (
            "descriptor table",
            DESCRIPTOR_TABLE_FILE_OFFSET,
            DESCRIPTOR_TABLE_FILE_OFFSET + HANDLER_COUNT * 2,
            DESCRIPTOR_TABLE_SHA256,
        ),
    )
    for label, start, end, expected in checks:
        actual = sha256(executable[start:end])
        if actual != expected:
            raise SystemExit(f"BBB {label} changed: {actual}")

    handlers = struct.unpack(
        f"<{HANDLER_COUNT + 1}H",
        executable[
            HANDLER_TABLE_FILE_OFFSET : HANDLER_TABLE_FILE_OFFSET
            + (HANDLER_COUNT + 1) * 2
        ],
    )
    if not all(
        handlers[index - FIRST_OPCODE]
        for index in range(FIRST_OPCODE, LAST_SEQUEL_OPCODE + 1)
    ):
        raise SystemExit("BBB A0-D7 handler table contains a null entry")
    if handlers[LAST_SEQUEL_OPCODE + 1 - FIRST_OPCODE] != 0:
        raise SystemExit("BBB D8 handler-table sentinel changed")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    executable = verify_executable(args.executable)
    rows = vectors(executable)
    result = {
        "format": "big_bug_bang_script_block_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "handler_table_offset": HANDLER_TABLE_OFFSET,
            "first_opcode": FIRST_OPCODE,
            "last_opcode": LAST_SEQUEL_OPCODE,
            "next_handler_is_null": True,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB nested script-block cases")


if __name__ == "__main__":
    main()
