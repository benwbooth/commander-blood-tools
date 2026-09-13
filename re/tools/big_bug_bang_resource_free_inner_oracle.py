#!/usr/bin/env python3
"""Verify BBB's relocated resource-pool compactor."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_free_inner.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_529c_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "4e9abce9f1720f872bbcbd013fdc473022126528af09c695346c7ad28533f64c"
)

ENTRY = 0x5714
END = 0x578C
BODY_SHA256 = "f587700c0100841e08e64363afd29252bb9f1c5b939d43257d27dcf2d441c43a"
MEMMOVE_CALL = 0x577A
MEMMOVE_RETURN = 0x577F
MEMMOVE_SEGMENT = 0x01E6
MEMMOVE_OFFSET = 0x0B94
MEMMOVE_ADDRESS = MEMMOVE_SEGMENT * 16 + MEMMOVE_OFFSET

FREE_BYTES_OFFSET = 0x0C3E
POOL_END_OFFSET = 0x0C62
COMMANDER_FREE_BYTES_OFFSET = 0x0A46
COMMANDER_POOL_END_OFFSET = 0x0A6A
RESIDENT_LIST_OFFSET = 0x0800
RESOURCE_ENTRY_SIZE = 8

TABLE_SEGMENT = 0x2000
DATA_SEGMENT = 0x4000
EXTRA_SEGMENT = 0x6000
GAME_SEGMENT = 0x8000
POOL_SEGMENT = 0xA000
STACK_SEGMENT = 0xC000
RETURN_SEGMENT = 0xE000
RETURN_OFFSET = 0x0100
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
STACK_SENTINEL = bytes.fromhex("4bb42dd21ee10ff0")
DEFINED_FLAG_MASK = 0x08C5

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

FOLLOWING_SIZES = {
    "last_resource_removes_without_move": [],
    "first_resource_compacts_two_followers": [0x30, 0x10],
    "middle_resource_preserves_predecessor": [0x25],
    "nonparagraph_size_uses_floor_shift": [0x31],
    "zero_sized_followers_skip_memmove": [0, 0],
    "any_signed_negative_word_terminates": [],
}


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


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


def snapshot_defined_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(flags & 0x0001),
        "pf": bool(flags & 0x0004),
        "zf": bool(flags & 0x0040),
        "sf": bool(flags & 0x0080),
        "df": bool(flags & 0x0400),
        "of": bool(flags & 0x0800),
    }


def or_flags(value: int) -> dict[str, bool]:
    value &= 0xFFFFFFFF
    return {
        "cf": False,
        "pf": (value & 0xFF).bit_count() % 2 == 0,
        "zf": value == 0,
        "sf": bool(value & 0x80000000),
        "df": False,
        "of": False,
    }


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def write_value(
    target: bytearray,
    offset: int,
    size: int,
    value: int,
    segment: int,
    events: list[tuple[int, int, int]],
) -> None:
    struct.pack_into({2: "<H", 4: "<I"}[size], target, offset, value)
    events.append((physical_address(segment, offset), size, value))


def push_value(
    target: bytearray,
    offset: int,
    size: int,
    value: int,
    events: list[tuple[int, int, int]],
) -> None:
    write_value(target, offset, size, value, STACK_SEGMENT, events)


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        handle = expected["handle"]
        resident = expected["resident_before"][:-1]
        terminator = expected["resident_before"][-1]
        released_index = resident.index(handle)
        following = resident[released_index + 1 :]
        following_sizes = FOLLOWING_SIZES[name]
        assert len(following) == len(following_sizes), name

        released_size = expected["released_size"]
        paragraphs = released_size >> 4
        moved_bytes = sum(following_sizes) & 0xFFFFFFFF
        assert paragraphs == expected["released_paragraphs"], name
        assert moved_bytes == expected["moved_bytes"], name

        initial_free_bytes = (0x10203040 + case_index * 0x11111111) & 0xFFFFFFFF
        initial_pool_end = (0x6800 + case_index * 0x31) & 0xFFFF
        table_before = seeded_segment(case_index, 17, 0x05)
        table_expected = bytearray(table_before)
        data_before = seeded_segment(case_index, 19, 0x17)
        extra_before = seeded_segment(case_index, 23, 0x29)
        game_before = seeded_segment(case_index, 29, 0x3B)
        game_expected = bytearray(game_before)
        pool_before = seeded_segment(case_index, 31, 0x4D)
        pool_expected = bytearray(pool_before)
        stack_before = seeded_segment(case_index, 37, 0x5F)
        stack_expected = bytearray(stack_before)
        expected_write_events: list[tuple[int, int, int]] = []

        def seed_entry(
            entry_handle: int,
            segment: int,
            flags: int,
            size: int,
            target: bytearray = table_before,
        ) -> None:
            struct.pack_into(
                "<HHI",
                target,
                (entry_handle * RESOURCE_ENTRY_SIZE) & 0xFFFF,
                segment & 0xFFFF,
                flags & 0xFFFF,
                size & 0xFFFFFFFF,
            )

        seed_entry(handle, POOL_SEGMENT, 0xA503, released_size)
        for position, entry_handle in enumerate(resident):
            if entry_handle == handle:
                continue
            if position < released_index:
                segment = POOL_SEGMENT - 0x20 * (released_index - position)
                size = 0x20
            else:
                follower_index = position - released_index - 1
                segment = POOL_SEGMENT + paragraphs + follower_index * 0x20
                size = following_sizes[follower_index]
            seed_entry(entry_handle, segment, 0xB703, size)
        for position, entry_handle in enumerate(expected["resident_before"]):
            struct.pack_into(
                "<H", table_before, RESIDENT_LIST_OFFSET + position * 2, entry_handle
            )
        table_expected[:] = table_before

        entry_offset = (handle * RESOURCE_ENTRY_SIZE) & 0xFFFF
        write_value(
            table_expected,
            entry_offset + 2,
            2,
            0xA500,
            TABLE_SEGMENT,
            expected_write_events,
        )
        write_value(
            game_expected,
            FREE_BYTES_OFFSET,
            4,
            (initial_free_bytes + released_size) & 0xFFFFFFFF,
            GAME_SEGMENT,
            expected_write_events,
        )
        write_value(
            game_expected,
            POOL_END_OFFSET,
            2,
            (initial_pool_end - paragraphs) & 0xFFFF,
            GAME_SEGMENT,
            expected_write_events,
        )
        for follower_index, entry_handle in enumerate([*following, terminator]):
            resident_offset = (
                RESIDENT_LIST_OFFSET + (released_index + follower_index) * 2
            )
            write_value(
                table_expected,
                resident_offset,
                2,
                entry_handle,
                TABLE_SEGMENT,
                expected_write_events,
            )
            if entry_handle & 0x8000:
                continue
            follower_offset = (entry_handle * RESOURCE_ENTRY_SIZE) & 0xFFFF
            old_segment = struct.unpack_from("<H", table_expected, follower_offset)[0]
            write_value(
                table_expected,
                follower_offset,
                2,
                (old_segment - paragraphs) & 0xFFFF,
                TABLE_SEGMENT,
                expected_write_events,
            )

        struct.pack_into("<I", game_before, FREE_BYTES_OFFSET, initial_free_bytes)
        struct.pack_into("<H", game_before, POOL_END_OFFSET, initial_pool_end)
        struct.pack_into(
            "<I",
            game_expected,
            FREE_BYTES_OFFSET,
            (initial_free_bytes + released_size) & 0xFFFFFFFF,
        )
        struct.pack_into(
            "<H",
            game_expected,
            POOL_END_OFFSET,
            (initial_pool_end - paragraphs) & 0xFFFF,
        )
        struct.pack_into(
            "<I",
            game_before,
            COMMANDER_FREE_BYTES_OFFSET,
            initial_free_bytes ^ 0xA5A5A5A5,
        )
        struct.pack_into(
            "<H", game_before, COMMANDER_POOL_END_OFFSET, initial_pool_end ^ 0xA5A5
        )
        struct.pack_into(
            "<I",
            game_expected,
            COMMANDER_FREE_BYTES_OFFSET,
            initial_free_bytes ^ 0xA5A5A5A5,
        )
        struct.pack_into(
            "<H", game_expected, COMMANDER_POOL_END_OFFSET, initial_pool_end ^ 0xA5A5
        )

        for decoy in (data_before, extra_before):
            struct.pack_into("<H", decoy, RESIDENT_LIST_OFFSET, terminator)
            struct.pack_into("<HHI", decoy, entry_offset, 0x1357, 0x2468, 0x89ABCDEF)
            struct.pack_into("<I", decoy, FREE_BYTES_OFFSET, 0xDEADBEEF)
            struct.pack_into("<H", decoy, POOL_END_OFFSET, 0xBEEF)

        if moved_bytes:
            source_offset = paragraphs * 16
            pool_expected[:moved_bytes] = pool_before[
                source_offset : source_offset + moved_bytes
            ]

        initial = {
            "eax": 0xA1A10000 | handle,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": TABLE_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        struct.pack_into("<HH", stack_before, CALLER_SP, RETURN_OFFSET, RETURN_SEGMENT)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected[:] = stack_before

        stack_events: list[tuple[int, int, int]] = []
        for offset, size, value in (
            (CALLER_SP - 4, 4, initial["eax"]),
            (CALLER_SP - 6, 2, initial["ebx"] & 0xFFFF),
            (CALLER_SP - 8, 2, initial["ecx"] & 0xFFFF),
            (CALLER_SP - 12, 4, initial["edx"]),
            (CALLER_SP - 14, 2, DATA_SEGMENT),
            (CALLER_SP - 16, 2, initial["esi"] & 0xFFFF),
            (CALLER_SP - 18, 2, EXTRA_SEGMENT),
            (CALLER_SP - 20, 2, initial["edi"] & 0xFFFF),
            (CALLER_SP - 24, 4, initial["ebp"]),
            (CALLER_SP - 26, 2, POOL_SEGMENT),
        ):
            push_value(stack_expected, offset, size, value, stack_events)
        expected_write_events[0:0] = stack_events
        if moved_bytes:
            far_call_events: list[tuple[int, int, int]] = []
            push_value(stack_expected, CALLER_SP - 26, 2, 0, far_call_events)
            push_value(
                stack_expected,
                CALLER_SP - 28,
                2,
                MEMMOVE_RETURN,
                far_call_events,
            )
            expected_write_events.extend(far_call_events)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(MEMMOVE_ADDRESS, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(TABLE_SEGMENT * 16, bytes(table_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(POOL_SEGMENT * 16, bytes(pool_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293)

        calls: list[dict[str, Any]] = []
        reached_return: list[int] = []
        observed_write_events: list[tuple[int, int, int]] = []
        pre_call_sp: list[int] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls_for_case: list[dict[str, Any]] = calls,
            reached: list[int] = reached_return,
            expected_row: dict[str, Any] = expected,
            case_name: str = name,
            case_moved_bytes: int = moved_bytes,
            case_paragraphs: int = paragraphs,
            call_stack_pointers: list[int] = pre_call_sp,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            if address == MEMMOVE_ADDRESS:
                byte_count = cpu.reg_read(UC_X86_REG_EAX)
                source_segment = cpu.reg_read(UC_X86_REG_DS)
                source_offset = cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF
                destination_segment = cpu.reg_read(UC_X86_REG_ES)
                destination_offset = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
                assert byte_count == case_moved_bytes, case_name
                assert source_segment == POOL_SEGMENT + case_paragraphs, case_name
                assert source_offset == 0, case_name
                assert destination_segment == POOL_SEGMENT, case_name
                assert destination_offset == 0, case_name
                assert cpu.reg_read(UC_X86_REG_SP) == CALLER_SP - 28, (
                    case_name,
                    hex(cpu.reg_read(UC_X86_REG_SP)),
                )
                assert list(
                    struct.unpack(
                        "<HH",
                        cpu.mem_read(STACK_SEGMENT * 16 + CALLER_SP - 28, 4),
                    )
                ) == [MEMMOVE_RETURN, 0], case_name
                payload = bytes(
                    cpu.mem_read(
                        physical_address(source_segment, source_offset), byte_count
                    )
                )
                cpu.mem_write(
                    physical_address(destination_segment, destination_offset), payload
                )
                calls_for_case.append(dict(expected_row["calls"][0]))
                return
            assert ENTRY <= address < END, hex(address)
            if address == MEMMOVE_CALL:
                call_stack_pointers.append(cpu.reg_read(UC_X86_REG_SP))

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            events: list[tuple[int, int, int]] = observed_write_events,
        ) -> None:
            events.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=200)

        assert reached_return == [RETURN_ADDRESS], name
        assert calls == expected["calls"], name
        assert pre_call_sp == ([CALLER_SP - 24] if moved_bytes else []), name
        assert observed_write_events == expected_write_events, (
            name,
            observed_write_events,
            expected_write_events,
        )
        assert snapshot_registers(machine) == initial, name
        assert snapshot_defined_flags(machine) == or_flags(0), (
            name,
            snapshot_defined_flags(machine),
            or_flags(0),
        )
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0200), name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT, name
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, name
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4, name
        assert bytes(machine.mem_read(TABLE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            table_expected
        ), name
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        ), name
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        ), name
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        ), name
        assert bytes(machine.mem_read(POOL_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            pool_expected
        ), name
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        ), name
        assert bytes(machine.mem_read(0, len(executable))) == module_before, name
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 4,
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        ), name

        rows.append(dict(expected))
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB resource-free body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_resource_free_inner_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "free_bytes_offset": FREE_BYTES_OFFSET,
            "pool_end_offset": POOL_END_OFFSET,
            "memmove": f"{MEMMOVE_SEGMENT:04x}:{MEMMOVE_OFFSET:04x}",
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB resource-free cases")


if __name__ == "__main__":
    main()
