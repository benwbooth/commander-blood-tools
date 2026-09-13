#!/usr/bin/env python3
"""Execute Big Bug Bang's navigation-chart object picker.

The original 0xAA3D body executes unmodified. Its record, global, and
stack-list inputs are modeled directly and compared with Commander Blood's
established semantic vectors.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
ROUTINE = (0xAA3D, 0xAAD4)
ROUTINE_SHA256 = "2dbdb27c3cd9e9aa4529fe6c09d7044507f139dc8defa474b7867311905da25b"

GLOBALS = 0x30000
RECORDS = 0x50000
STACK = 0x70000
GAME = 0x90000
SEGMENT_SIZE = 0x10000
RETURN_IP = 0x1800
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

OFFSETS = {
    "mouse_x": 0x0C22,
    "mouse_y": 0x0C24,
    "scratch_width": 0x2A08,
    "scratch_height": 0x2A0A,
    "object_count": 0x2A61,
    "object_list": 0x2D73,
    "arche": 0x6B22,
}

CASES = (
    {
        "name": "empty_list_preserves_scratch",
        "mouse": (48, 58),
        "objects": (),
        "scratch": (0xA55A, 0x5AA5),
    },
    {
        "name": "default_lower_edges_inclusive",
        "mouse": (48, 58),
        "objects": ({"offset": 0x1000, "kind": 0, "near": (50, 60)},),
    },
    {
        "name": "default_upper_edges_inclusive",
        "mouse": (60, 69),
        "objects": ({"offset": 0x1100, "kind": 8, "near": (50, 60)},),
    },
    {
        "name": "first_miss_second_hit",
        "mouse": (98, 98),
        "objects": (
            {"offset": 0x1200, "kind": 8, "near": (20, 20)},
            {"offset": 0x1300, "kind": 8, "near": (100, 100)},
        ),
    },
    {
        "name": "first_overlapping_hit_wins",
        "mouse": (78, 78),
        "objects": (
            {"offset": 0x1400, "kind": 8, "near": (80, 80)},
            {"offset": 0x1500, "kind": 8, "near": (80, 80)},
        ),
    },
    {
        "name": "ship_wide_far_edge",
        "mouse": (119, 68),
        "objects": ({"offset": 0x1600, "kind": 0x10, "near": (100, 60)},),
    },
    {
        "name": "ship_short_y_rejects",
        "mouse": (100, 69),
        "objects": ({"offset": 0x1700, "kind": 0x10, "near": (100, 60)},),
    },
    {
        "name": "black_hole_near_endpoint",
        "mouse": (167, 70),
        "arche_context": 7,
        "objects": (
            {
                "offset": 0x1800,
                "kind": 0x100,
                "endpoint_context": 7,
                "near": (150, 60),
                "far": (250, 90),
            },
        ),
    },
    {
        "name": "black_hole_far_endpoint",
        "mouse": (248, 88),
        "arche_context": 9,
        "objects": (
            {
                "offset": 0x1900,
                "kind": 0x100,
                "endpoint_context": 7,
                "near": (150, 60),
                "far": (250, 90),
            },
        ),
    },
    {
        "name": "both_bits_near_uses_ship_box",
        "mouse": (69, 120),
        "arche_context": 7,
        "objects": (
            {
                "offset": 0x1A00,
                "kind": 0x110,
                "endpoint_context": 7,
                "near": (50, 120),
                "far": (200, 150),
            },
        ),
    },
    {
        "name": "both_bits_far_keeps_black_hole_box",
        "mouse": (200, 160),
        "arche_context": 7,
        "objects": (
            {
                "offset": 0x1B00,
                "kind": 0x110,
                "endpoint_context": 8,
                "near": (50, 120),
                "far": (200, 150),
            },
        ),
    },
    {
        "name": "wrapped_origin_rejects_between_wrapped_bounds",
        "mouse": (0xFFFF, 8),
        "objects": ({"offset": 0x1C00, "kind": 8, "near": (1, 10)},),
    },
    {
        "name": "wrapped_object_fields_and_reverse_df",
        "mouse": (38, 48),
        "arche_offset": 0xFFF0,
        "arche_context": 0x1234,
        "direction": "reverse",
        "objects": (
            {
                "offset": 0xFFF0,
                "kind": 0x100,
                "endpoint_context": 0x1234,
                "near": (40, 50),
                "far": (240, 150),
            },
        ),
    },
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write_word_wrap(memory: bytearray, offset: int, value: int) -> None:
    memory[offset & 0xFFFF] = value & 0xFF
    memory[(offset + 1) & 0xFFFF] = (value >> 8) & 0xFF


def read_word_wrap(memory: bytes | bytearray, offset: int) -> int:
    return memory[offset & 0xFFFF] | (memory[(offset + 1) & 0xFFFF] << 8)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def sub16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
    }


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    name = str(case["name"])
    objects = tuple(case["objects"])
    mouse_x, mouse_y = (int(value) & 0xFFFF for value in case["mouse"])
    arche_offset = int(case.get("arche_offset", 0x4000)) & 0xFFFF
    arche_context = int(case.get("arche_context", 0x7777)) & 0xFFFF
    scratch_before = tuple(
        int(value) & 0xFFFF for value in case.get("scratch", (0xCCCC, 0xDDDD))
    )
    reverse = case.get("direction") == "reverse"

    globals_before = seeded_segment(case_index, 17, 0x31)
    records_before = seeded_segment(case_index, 11, 0x53)
    stack_before = seeded_segment(case_index, 23, 0x79)
    game_before = bytes(seeded_segment(case_index, 13, 0x97))

    write_word_wrap(globals_before, OFFSETS["object_count"], len(objects))
    write_word_wrap(globals_before, OFFSETS["mouse_x"], mouse_x)
    write_word_wrap(globals_before, OFFSETS["mouse_y"], mouse_y)
    write_word_wrap(globals_before, OFFSETS["arche"], arche_offset)
    write_word_wrap(globals_before, OFFSETS["scratch_width"], scratch_before[0])
    write_word_wrap(globals_before, OFFSETS["scratch_height"], scratch_before[1])
    for index, object_case in enumerate(objects):
        object_offset = int(object_case["offset"]) & 0xFFFF
        write_word_wrap(
            stack_before,
            OFFSETS["object_list"] + index * 2,
            object_offset,
        )
        write_word_wrap(
            globals_before,
            OFFSETS["object_list"] + index * 2,
            object_offset ^ 0x5555,
        )
        write_word_wrap(records_before, object_offset, int(object_case["kind"]))
        write_word_wrap(
            records_before,
            object_offset + 0x14,
            int(object_case.get("endpoint_context", 0xBEEF)),
        )
        near_x, near_y = object_case["near"]
        far_x, far_y = object_case.get("far", (0xCAFE, 0xBABE))
        write_word_wrap(records_before, object_offset + 0x18, int(near_x))
        write_word_wrap(records_before, object_offset + 0x1A, int(near_y))
        write_word_wrap(records_before, object_offset + 0x1C, int(far_x))
        write_word_wrap(records_before, object_offset + 0x1E, int(far_y))
    write_word_wrap(records_before, arche_offset + 0x22, arche_context)
    write_word_wrap(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    width, height = scratch_before
    expected_ax = 0
    expected_bx = 0x2468
    expected_cx = len(objects)
    expected_dx = 0x55AA
    expected_bp = 0x1357
    expected_di = 0x789A
    expected_flags = {
        "cf": False,
        "pf": True,
        "zf": True,
        "sf": False,
        "of": False,
    }
    hit_offset = 0
    endpoint_index = 0
    terminal_path = "empty"

    if objects:
        expected_ax = mouse_x
        expected_dx = mouse_y
        expected_bp = OFFSETS["object_list"]
        for object_index, object_case in enumerate(objects):
            object_offset = int(object_case["offset"]) & 0xFFFF
            kind = read_word_wrap(records_before, object_offset)
            expected_di = (object_offset + 0x18) & 0xFFFF
            width, height = (0x0C, 0x0B)
            endpoint_index = 0
            skip_ship_test = False
            if kind & 0x100:
                width, height = (0x13, 0x0C)
                endpoint_context = read_word_wrap(records_before, object_offset + 0x14)
                if endpoint_context != read_word_wrap(
                    records_before, arche_offset + 0x22
                ):
                    expected_di = (expected_di + 4) & 0xFFFF
                    endpoint_index = 1
                    skip_ship_test = True
            if not skip_ship_test and kind & 0x10:
                width, height = (0x15, 0x0A)

            left = (read_word_wrap(records_before, expected_di) - 2) & 0xFFFF
            expected_bx = left
            if mouse_x < left:
                expected_flags = sub16_flags(mouse_x, left)
                terminal_path = "x_below"
            else:
                right = (left + width) & 0xFFFF
                expected_bx = right
                if mouse_x > right:
                    expected_flags = sub16_flags(mouse_x, right)
                    terminal_path = "x_above"
                else:
                    top = (read_word_wrap(records_before, expected_di + 2) - 2) & 0xFFFF
                    expected_bx = top
                    if mouse_y < top:
                        expected_flags = sub16_flags(mouse_y, top)
                        terminal_path = "y_below"
                    else:
                        bottom = (top + height) & 0xFFFF
                        expected_bx = bottom
                        expected_flags = sub16_flags(mouse_y, bottom)
                        if mouse_y <= bottom:
                            expected_ax = object_offset
                            expected_cx = len(objects) - object_index
                            hit_offset = object_offset
                            terminal_path = "hit"
                            break
                        terminal_path = "y_above"
            expected_bp = (expected_bp + 2) & 0xFFFF
            expected_cx -= 1
        else:
            expected_ax = 0
            expected_flags = {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "of": False,
            }

    globals_expected = bytearray(globals_before)
    if objects:
        write_word_wrap(globals_expected, OFFSETS["scratch_width"], width)
        write_word_wrap(globals_expected, OFFSETS["scratch_height"], height)

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | (0x1111 + case_index),
        UC_X86_REG_EBX: 0xB2B22468,
        UC_X86_REG_ECX: 0xC3C3369C,
        UC_X86_REG_EDX: 0xD4D455AA,
        UC_X86_REG_ESI: 0xE5E56789,
        UC_X86_REG_EDI: 0xF6F6789A,
        UC_X86_REG_EBP: 0x97971357,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: RECORDS // 16,
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0602 if reverse else 0x0202,
    }

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(RECORDS, bytes(records_before))
    cpu.mem_write(STACK, bytes(stack_before))
    cpu.mem_write(GAME, game_before)
    for register, value in initial.items():
        cpu.reg_write(register, value)

    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            machine.emu_stop()
            return
        assert (
            image_address(ROUTINE[0])
            <= address
            < address + size
            <= image_address(ROUTINE[1])
        ), (name, hex(address + HEADER_SIZE))

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=500)
    assert reached_return, name

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    assert globals_after == bytes(globals_expected), name
    assert bytes(cpu.mem_read(RECORDS, SEGMENT_SIZE)) == bytes(records_before), name
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_before), name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == game_before, name
    assert bytes(cpu.mem_read(0, len(module))) == module, name

    expected_registers = dict(initial)
    expected_registers.update(
        {
            UC_X86_REG_EAX: (initial[UC_X86_REG_EAX] & 0xFFFF0000) | expected_ax,
            UC_X86_REG_EBX: (initial[UC_X86_REG_EBX] & 0xFFFF0000) | expected_bx,
            UC_X86_REG_ECX: (initial[UC_X86_REG_ECX] & 0xFFFF0000) | expected_cx,
            UC_X86_REG_EDX: (initial[UC_X86_REG_EDX] & 0xFFFF0000) | expected_dx,
            UC_X86_REG_EDI: (initial[UC_X86_REG_EDI] & 0xFFFF0000) | expected_di,
            UC_X86_REG_EBP: (initial[UC_X86_REG_EBP] & 0xFFFF0000) | expected_bp,
            UC_X86_REG_SP: STACK_POINTER + 2,
        }
    )
    for register in (
        UC_X86_REG_EAX,
        UC_X86_REG_EBX,
        UC_X86_REG_ECX,
        UC_X86_REG_EDX,
        UC_X86_REG_ESI,
        UC_X86_REG_EDI,
        UC_X86_REG_EBP,
        UC_X86_REG_DS,
        UC_X86_REG_ES,
        UC_X86_REG_FS,
        UC_X86_REG_GS,
        UC_X86_REG_SS,
        UC_X86_REG_SP,
    ):
        assert cpu.reg_read(register) == expected_registers[register], (
            name,
            register,
            hex(cpu.reg_read(register)),
            hex(expected_registers[register]),
        )
    stack_after = bytes(cpu.mem_read(STACK, SEGMENT_SIZE))
    assert (
        stack_after[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)]
        == STACK_SENTINEL
    ), name

    expected_flags = {key: bool(value) for key, value in expected_flags.items()}
    expected_flags["df"] = reverse
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(flags & mask)
        for flag, mask in (
            ("cf", 1),
            ("pf", 4),
            ("zf", 0x40),
            ("sf", 0x80),
            ("of", 0x800),
            ("df", 0x400),
        )
    }
    assert defined_flags == expected_flags, (name, defined_flags, expected_flags)

    return {
        "name": name,
        "mouse": [mouse_x, mouse_y],
        "object_offsets": [int(item["offset"]) & 0xFFFF for item in objects],
        "arche_offset": arche_offset,
        "arche_context": arche_context,
        "result": hit_offset,
        "terminal_path": terminal_path,
        "terminal_endpoint": endpoint_index,
        "scratch_before": list(scratch_before),
        "scratch_after": [width, height],
        "remaining_count": expected_cx,
        "list_cursor": expected_bp,
        "defined_flags": defined_flags,
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "return": "near",
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert len(rows) == len(commander)
    fields = (
        "name",
        "mouse",
        "object_offsets",
        "arche_offset",
        "arche_context",
        "result",
        "terminal_path",
        "terminal_endpoint",
        "scratch_before",
        "scratch_after",
        "remaining_count",
        "defined_flags",
    )
    for sequel, original in zip(rows, commander, strict=True):
        assert {field: sequel[field] for field in fields} == {
            field: original[field] for field in fields
        }, sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_92a3_natural.json",
    )
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    actual = hashlib.sha256(executable[slice(*ROUTINE)]).hexdigest()
    if actual != ROUTINE_SHA256:
        raise SystemExit(
            f"native span {ROUTINE[0]:#x}..{ROUTINE[1]:#x} changed: {actual}"
        )
    assert executable[ROUTINE[1] - 1] == 0xC3

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB navigation-pick cases")


if __name__ == "__main__":
    main()
