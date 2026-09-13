#!/usr/bin/env python3
"""Verify BBB's relocated bridge pointer primitives."""

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
    Uc,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_bridge_pointer_primitives.json"
)
COMMANDER_LATCH_FIXTURE = ROOT / "re/tools/oracle_vectors/func_8269_natural.json"
COMMANDER_POLL_FIXTURE = ROOT / "re/tools/oracle_vectors/func_82c3_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_LATCH_FIXTURE_SHA256 = (
    "c1d0d7ae573e1765e921ea9e43a8e68bf2d9462157597cb1bc5fadc7dc600d56"
)
COMMANDER_POLL_FIXTURE_SHA256 = (
    "ae4636e7d013f6043843e4b9a5028da7dd7ca65e273553abc52a072f4ebf1629"
)

LATCH_ENTRY = 0x93CB
LATCH_END = 0x93F7
LATCH_BODY_SHA256 = "0a56b3b411048cc32c8bd3553cd40e85423e7788c34912c9e34b8d2ca283aa77"
HIT_TEST_ENTRY = 0x93F7
HIT_TEST_END = 0x9425
HIT_TEST_BODY_SHA256 = (
    "935201f0949bd3bc0185be51de2ac663b50cbdba3450931bd98511bba0df8975"
)
POLL_ENTRY = 0x9425
POLL_END = 0x944A
POLL_BODY_SHA256 = "14b5b288d6df0cafe6eccb5cdd31d512997ce6f77b397cf63ca5532f4a5071af"

MOUSE_X_OFFSET = 0x0C22
MOUSE_Y_OFFSET = 0x0C24
PRIMARY_OFFSET = 0x0C36
POLL_TABLE_OFFSET = 0x65E2
POLL_RECORD_OFFSET = 0x69C2
POLL_RECT_OFFSET = 0x69CA
RECORD_SIZE = 0x20
POLL_ATTEMPTS = 32

DATA_SEGMENT = 0x3000
DECOY_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_SEGMENT = 0x2800
NEAR_RETURN_OFFSET = 0x7000
FAR_RETURN_OFFSET = 0
FAR_RETURN_ADDRESS = RETURN_SEGMENT * 16 + FAR_RETURN_OFFSET
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

LATCH_CASES = (
    ("disabled_inside", 0x00, (10, 20), (10, 20, 30, 40), 0x52),
    ("non_primary_button_inside", 0x80, (20, 30), (10, 20, 30, 40), 0x52),
    ("zero_extent_origin", 0x01, (0, 0), (0, 0, 0, 0), 0x52),
    ("x_below", 0x01, (9, 20), (10, 20, 30, 40), 0x52),
    ("x_right_boundary", 0x01, (40, 20), (10, 20, 30, 40), 0x52),
    ("x_right_outside", 0x01, (41, 20), (10, 20, 30, 40), 0x52),
    ("y_bottom_boundary", 0x01, (10, 60), (10, 20, 30, 40), 0x52),
    ("y_bottom_outside", 0x01, (10, 61), (10, 20, 30, 40), 0x52),
    ("negative_rect_boundary", 0x01, (-90, -20), (-120, -40, 30, 20), 0x52),
    ("wrapped_negative_width_hit", 0x01, (32767, 0), (0, 0, -1, 0), 0x52),
    (
        "wrapped_positive_result_miss",
        0x01,
        (-32768, 0),
        (-32768, 0, 1, 0),
        0x52,
    ),
    ("hit_preserves_other_bits", 0xA5, (5, 5), (-5, -5, 10, 10), 0xD2),
)

POLL_CASES = (
    {
        "name": "disabled_for_all_attempts",
        "flags": 0xA5FE,
        "rect": (-20, -10, 40, 20),
        "mouse": (0, 0),
        "primary": 1,
        "expected": -1,
        "expected_calls": 0,
        "expected_iterations": 32,
    },
    {
        "name": "immediate_signed_hit",
        "flags": 0xA501,
        "rect": (-120, 100, 40, 30),
        "mouse": (-100, 123),
        "primary": 1,
        "expected": 31,
        "expected_calls": 1,
        "expected_iterations": 1,
    },
    {
        "name": "same_region_misses_32_times",
        "flags": 0x0001,
        "rect": (100, 100, 5, 5),
        "mouse": (0, 0),
        "primary": 1,
        "expected": -1,
        "expected_calls": 32,
        "expected_iterations": 32,
    },
    {
        "name": "mouse_enabled_on_third_call",
        "flags": 0x7F01,
        "rect": (-10, -10, 20, 20),
        "mouse": (0, 0),
        "primary": 0,
        "primary_on_call": 3,
        "expected": 29,
        "expected_calls": 3,
        "expected_iterations": 3,
    },
    {
        "name": "region_enabled_on_fifth_iteration",
        "flags": 0x55FE,
        "rect": (-20, -20, 40, 40),
        "mouse": (0, 0),
        "primary": 1,
        "flags_on_iteration": 5,
        "expected": 27,
        "expected_calls": 1,
        "expected_iterations": 5,
    },
    {
        "name": "mouse_gate_blocks_all_calls",
        "flags": 0x0001,
        "rect": (-1, -1, 2, 2),
        "mouse": (0, 0),
        "primary": 0,
        "expected": -1,
        "expected_calls": 32,
        "expected_iterations": 32,
    },
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def signed_word(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


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


def observed_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def make_machine(
    executable: bytes,
    initial: dict[str, int],
    data: bytes,
    decoy: bytes,
    extra: bytes,
    fs_data: bytes,
    stack: bytes,
) -> tuple[Uc, bytes]:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(NEAR_RETURN_OFFSET, b"\xcc")
    machine.mem_write(FAR_RETURN_ADDRESS, b"\xcc")
    module_before = bytes(machine.mem_read(0, len(executable)))
    machine.mem_write(DATA_SEGMENT * 16, data)
    machine.mem_write(DECOY_SEGMENT * 16, decoy)
    machine.mem_write(EXTRA_SEGMENT * 16, extra)
    machine.mem_write(FS_SEGMENT * 16, fs_data)
    machine.mem_write(STACK_SEGMENT * 16, stack)
    for name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, initial[name])
    for name, register in SEGMENT_REGISTERS.items():
        machine.reg_write(register, initial[name])
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    return machine, module_before


def latch_result(
    primary: int, mouse: tuple[int, int], rect: tuple[int, int, int, int]
) -> tuple[bool, str]:
    mouse_x, mouse_y = mouse
    rect_x, rect_y, rect_width, rect_height = rect
    adjusted_x = signed_word(mouse_x - rect_width)
    adjusted_y = signed_word(mouse_y - rect_height)
    if primary & 1 == 0:
        return False, "primary_test"
    if mouse_x < rect_x:
        return False, "x_lower_cmp"
    if adjusted_x > rect_x:
        return False, "x_upper_cmp"
    if mouse_y < rect_y:
        return False, "y_lower_cmp"
    if adjusted_y > rect_y:
        return False, "y_upper_cmp"
    return True, "hit_or"


def latch_vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    rect_offset = 0x6200
    flags_offset = 0x2A1B
    for case_index, (name, primary, mouse, rect, initial_hit_flags) in enumerate(
        LATCH_CASES
    ):
        hit, terminal = latch_result(primary, mouse, rect)
        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E50000 | rect_offset,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0xA7A70000 | flags_offset,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 29, 0x4D)
        struct.pack_into("<hh", data_before, MOUSE_X_OFFSET, *mouse)
        data_before[PRIMARY_OFFSET] = primary
        struct.pack_into("<hhhh", data_before, rect_offset, *rect)
        decoy_before = bytes(seeded_segment(case_index, 7, 0xB3))
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 31, 0x59))
        stack_before = seeded_segment(case_index, 11, 0x63)
        stack_before[flags_offset] = initial_hit_flags
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", NEAR_RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, initial["eax"] & 0xFFFF)
        if hit:
            stack_expected[flags_offset] = initial_hit_flags | 0x08

        machine, module_before = make_machine(
            executable,
            initial,
            bytes(data_before),
            decoy_before,
            extra_before,
            fs_before,
            bytes(stack_before),
        )
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0602)
        reached_return: list[int] = []
        allowed_stack_addresses = set(
            range(STACK_SEGMENT * 16 + CALLER_SP - 2, STACK_SEGMENT * 16 + CALLER_SP)
        ) | {STACK_SEGMENT * 16 + flags_offset}

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == NEAR_RETURN_OFFSET:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert LATCH_ENTRY <= address < LATCH_END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            assert set(range(address, address + size)) <= allowed_stack_addresses, hex(
                address
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(LATCH_ENTRY, 0, count=100)

        assert reached_return == [NEAR_RETURN_OFFSET], name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == NEAR_RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        defined_flags = observed_flags(machine)
        if terminal in {"primary_test", "hit_or"}:
            defined_flags.pop("af")
        rows.append(
            {
                "name": name,
                "primary": primary,
                "mouse": list(mouse),
                "rect": list(rect),
                "initial_hit_flags": initial_hit_flags,
                "hit": hit,
                "result_hit_flags": stack_expected[flags_offset],
                "terminal": terminal,
                "defined_flags": defined_flags,
                "preserved_registers": [
                    "eax",
                    "ebx",
                    "ecx",
                    "edx",
                    "esi",
                    "edi",
                    "ebp",
                    "ds",
                    "es",
                    "fs",
                    "gs",
                    "ss",
                ],
                "return": "near",
            }
        )
    return rows


def poll_vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, case in enumerate(POLL_CASES):
        name = str(case["name"])
        mouse = tuple(int(value) for value in case["mouse"])
        primary = int(case["primary"])
        flags = int(case["flags"])
        rect = tuple(int(value) for value in case["rect"])
        expected = int(case["expected"])
        expected_calls = int(case["expected_calls"])
        expected_iterations = int(case["expected_iterations"])
        primary_on_call = int(case.get("primary_on_call", 0))
        flags_on_iteration = int(case.get("flags_on_iteration", 0))
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62345 + case_index,
            "ecx": 0xC7C73456 + case_index,
            "edx": 0xD8D84567 + case_index,
            "esi": 0xE9E95678 + case_index,
            "edi": 0xFAFA6789 + case_index,
            "ebp": 0xABCD789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 29, 0x4D)
        struct.pack_into("<hh", data_before, MOUSE_X_OFFSET, *mouse)
        data_before[PRIMARY_OFFSET] = primary
        data_expected = bytearray(data_before)
        if primary_on_call:
            data_expected[PRIMARY_OFFSET] = 1
        decoy_before = bytes(seeded_segment(case_index, 7, 0xB3))
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 31, 0x59))
        stack_before = seeded_segment(case_index, 11, 0x63)
        for region_id in range(POLL_ATTEMPTS):
            record_offset = POLL_TABLE_OFFSET + region_id * RECORD_SIZE
            stack_before[record_offset : record_offset + RECORD_SIZE] = bytes(
                (region_id * 19 + byte_index * 23 + case_index * 11) & 0xFF
                for byte_index in range(RECORD_SIZE)
            )
            struct.pack_into("<H", stack_before, record_offset, 0xA5FE)
        struct.pack_into("<H", stack_before, POLL_RECORD_OFFSET, flags & 0xFFFF)
        struct.pack_into("<hhhh", stack_before, POLL_RECT_OFFSET, *rect)
        table_sha256 = hashlib.sha256(
            stack_before[
                POLL_TABLE_OFFSET : POLL_TABLE_OFFSET + POLL_ATTEMPTS * RECORD_SIZE
            ]
        ).hexdigest()
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", FAR_RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        if flags_on_iteration:
            struct.pack_into("<H", stack_expected, POLL_RECORD_OFFSET, flags | 1)

        machine, module_before = make_machine(
            executable,
            initial,
            bytes(data_before),
            decoy_before,
            extra_before,
            fs_before,
            bytes(stack_before),
        )
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202)
        calls: list[dict[str, Any]] = []
        iterations = 0
        reached_return: list[int] = []
        transient_start = CALLER_SP - 10
        allowed_write_addresses = set(
            range(
                STACK_SEGMENT * 16 + transient_start,
                STACK_SEGMENT * 16 + CALLER_SP,
            )
        )

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            nonlocal iterations
            if address == FAR_RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert (
                POLL_ENTRY <= address < POLL_END
                or HIT_TEST_ENTRY <= address < HIT_TEST_END
            ), hex(address)
            if address == 0x942A:
                iterations += 1
                if flags_on_iteration == iterations:
                    cpu.mem_write(
                        STACK_SEGMENT * 16 + POLL_RECORD_OFFSET,
                        struct.pack("<H", flags | 1),
                    )
                return
            if address != HIT_TEST_ENTRY:
                return
            call_number = len(calls) + 1
            if primary_on_call == call_number:
                cpu.mem_write(DATA_SEGMENT * 16 + PRIMARY_OFFSET, b"\x01")
            actual_rect_offset = cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF
            actual_rect = struct.unpack(
                "<hhhh",
                cpu.mem_read(STACK_SEGMENT * 16 + actual_rect_offset, 8),
            )
            calls.append(
                {
                    "attempts_remaining": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "rect_offset": actual_rect_offset,
                    "rect": list(actual_rect),
                }
            )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            assert set(range(address, address + size)) <= allowed_write_addresses, hex(
                address
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(POLL_ENTRY, 0, count=2_000)

        assert reached_return == [FAR_RETURN_ADDRESS], name
        assert iterations == expected_iterations, (name, iterations)
        assert len(calls) == expected_calls, (name, len(calls))
        assert all(call["rect_offset"] == POLL_RECT_OFFSET for call in calls), name
        expected_call_start = (
            POLL_ATTEMPTS - flags_on_iteration if flags_on_iteration else 31
        )
        assert [call["attempts_remaining"] for call in calls] == list(
            range(expected_call_start, expected_call_start - expected_calls, -1)
        ), name
        assert all(call["rect"] == list(rect) for call in calls), name
        assert machine.reg_read(UC_X86_REG_EAX) & 0xFFFF == expected & 0xFFFF, name
        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | (expected & 0xFFFF)
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_IP) == FAR_RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        carry = bool(machine.reg_read(UC_X86_REG_EFLAGS) & 1)
        assert carry == (expected >= 0), name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        actual_stack = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert actual_stack[:transient_start] == bytes(stack_expected[:transient_start])
        assert actual_stack[CALLER_SP:] == bytes(stack_expected[CALLER_SP:])

        rows.append(
            {
                "name": name,
                "mouse": list(mouse),
                "primary": primary,
                "initial_flags": flags,
                "rect": list(rect),
                "flags_on_iteration": flags_on_iteration or None,
                "primary_on_call": primary_on_call or None,
                "result": expected,
                "iterations": iterations,
                "calls": calls,
                "carry": carry,
                "table_sha256": table_sha256,
                "return": "far",
            }
        )
    return rows


def verify_fixture(path: Path, expected_sha256: str) -> list[dict[str, Any]]:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest == expected_sha256, (path, digest)
    return json.loads(path.read_text())


def semantic_poll_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    normalized = copy.deepcopy(rows)
    for row in normalized:
        row.pop("table_sha256")
        for call in row["calls"]:
            call.pop("rect_offset")
    return normalized


def verify_commander_semantics(
    latch_rows: list[dict[str, Any]], poll_rows: list[dict[str, Any]]
) -> dict[str, str]:
    commander_latch = verify_fixture(
        COMMANDER_LATCH_FIXTURE, COMMANDER_LATCH_FIXTURE_SHA256
    )
    commander_poll = verify_fixture(
        COMMANDER_POLL_FIXTURE, COMMANDER_POLL_FIXTURE_SHA256
    )
    assert latch_rows == commander_latch
    assert semantic_poll_rows(poll_rows) == semantic_poll_rows(commander_poll)
    return {
        "latch": COMMANDER_LATCH_FIXTURE_SHA256,
        "poll": COMMANDER_POLL_FIXTURE_SHA256,
    }


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    bodies = (
        (LATCH_ENTRY, LATCH_END, LATCH_BODY_SHA256, "latch"),
        (HIT_TEST_ENTRY, HIT_TEST_END, HIT_TEST_BODY_SHA256, "hit-test helper"),
        (POLL_ENTRY, POLL_END, POLL_BODY_SHA256, "poll"),
    )
    for entry, end, expected, name in bodies:
        actual = hashlib.sha256(executable[entry:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"BBB bridge {name} body changed: {actual}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    executable = verify_executable(args.executable)
    latch_rows = latch_vectors(executable)
    poll_rows = poll_vectors(executable)
    result = {
        "format": "big_bug_bang_bridge_pointer_primitives_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": verify_commander_semantics(
            latch_rows, poll_rows
        ),
        "routines": {
            "latch": {
                "entry": f"0x{LATCH_ENTRY:04x}",
                "end": f"0x{LATCH_END:04x}",
                "body_sha256": LATCH_BODY_SHA256,
                "mouse_offsets": [MOUSE_X_OFFSET, MOUSE_Y_OFFSET, PRIMARY_OFFSET],
            },
            "poll": {
                "entry": f"0x{POLL_ENTRY:04x}",
                "end": f"0x{POLL_END:04x}",
                "body_sha256": POLL_BODY_SHA256,
                "record_offset": POLL_RECORD_OFFSET,
                "rect_offset": POLL_RECT_OFFSET,
                "attempts": POLL_ATTEMPTS,
            },
        },
        "latch_rows": latch_rows,
        "poll_rows": poll_rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(
        f"verified {len(latch_rows)} BBB bridge-latch cases and "
        f"{len(poll_rows)} fixed-region poll cases"
    )


if __name__ == "__main__":
    main()
