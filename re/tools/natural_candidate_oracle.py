#!/usr/bin/env python3
"""Verify selected natural-C semantics against direct BLOODPRG execution."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_GS,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
EXE = (REPO_ROOT / "re/bin/BLOODPRG.EXE").read_bytes()
VECTOR_ROOT = REPO_ROOT / "re/tools/oracle_vectors"

REGISTERS = {
    "ax": UC_X86_REG_AX,
    "bx": UC_X86_REG_BX,
    "cx": UC_X86_REG_CX,
    "dx": UC_X86_REG_DX,
    "si": UC_X86_REG_SI,
    "di": UC_X86_REG_DI,
    "bp": UC_X86_REG_BP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "gs": UC_X86_REG_GS,
    "flags": UC_X86_REG_EFLAGS,
}


def execute(
    entry: int,
    return_address: int,
    registers: dict[str, int],
    memory: list[tuple[int, int, bytes]],
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x300000)
    machine.mem_write(0, EXE + bytes(0x120000 - len(EXE)))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_SS, 0x9000)
    machine.reg_write(UC_X86_REG_SP, 0xff00)

    for name, value in registers.items():
        machine.reg_write(REGISTERS[name], value)
    for segment, offset, data in memory:
        machine.mem_write(segment * 16 + offset, data)

    returned = []

    def stop_at_return(machine: Uc, address: int, _size: int, _data: object) -> None:
        if address == return_address:
            returned.append(address)
            machine.emu_stop()

    machine.hook_add(UC_HOOK_CODE, stop_at_return)
    machine.emu_start(entry, return_address + 1, count=20000)
    if not returned:
        raise RuntimeError(f"{entry:#x}: did not reach return at {return_address:#x}")
    return machine


def text_width_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    font_segment = 0x2600
    faces = {
        0: (0x7362, 0x7412, 0x14782, 0x14832),
        1: (0x7802, 0x78b2, 0x14c22, 0x14cd2),
    }
    texts = [
        b"",
        b"A",
        b"TALK",
        b"Commander Blood",
        b"               ",
        b"$?",
        bytes([0x82, 0x84, 0x87, 0x94]),
    ]
    vectors = []

    for selector in (0, 1, 0xffff):
        face = 0 if selector == 0 else 1
        map_offset, advance_offset, map_file, advance_file = faces[face]
        character_map = EXE[map_file:advance_file]
        advance_region = EXE[advance_file : advance_file + 256]
        if len(character_map) != 176 or len(advance_region) != 256:
            raise RuntimeError("font table extraction did not produce the expected extent")

        for text in texts:
            expected = sum(
                advance_region[character_map[character]] for character in text
            )
            expected = (expected - 2) & 0xffff
            initial = {
                "ax": selector,
                "bx": 0x1357,
                "cx": 0x2468,
                "dx": 0x369c,
                "si": 0x0100,
                "di": 0x55aa,
                "bp": 0x6789,
                "ds": data_segment,
                "gs": font_segment,
            }
            machine = execute(
                0x30CD,
                0x3105,
                initial,
                [
                    (data_segment, 0x0100, text + b"\0"),
                    (font_segment, map_offset, character_map),
                    (font_segment, advance_offset, advance_region),
                ],
            )
            actual = machine.reg_read(UC_X86_REG_AX)
            if actual != expected:
                raise AssertionError(
                    f"0x30CD selector={selector:#x} text={text!r}: "
                    f"actual={actual:#x}, expected={expected:#x}"
                )
            for name in ("bx", "cx", "dx", "si", "di", "bp"):
                actual_register = machine.reg_read(REGISTERS[name])
                if actual_register != initial[name]:
                    raise AssertionError(f"0x30CD did not preserve {name}")

            vectors.append(
                {
                    "selector": selector,
                    "text": list(text),
                    "width_minus_trailing_gap": actual,
                }
            )

    return vectors


def mask_overlay_vectors() -> list[dict[str, object]]:
    patterns = [
        [0x0000] * 16,
        [0x8000 >> row for row in range(16)],
        [0xffff] * 16,
        [0xaaaa, 0x5555] * 8,
        [1 << (15 - row) for row in range(16)],
        [
            0x8001,
            0x4002,
            0x2004,
            0x1008,
            0x0810,
            0x0420,
            0x0240,
            0x0180,
        ]
        * 2,
    ]
    data_segment = 0x2000
    framebuffer_segment = 0x3000
    framebuffer_size = 0x4000
    table = bytearray()
    for rows in patterns:
        for bits in rows:
            table.extend(struct.pack(">H", bits))

    vectors = []
    for index, rows in enumerate(patterns):
        pointer_offset = 0 if index % 2 == 0 else 0x4567
        initial_framebuffer = bytes([0x5a]) * framebuffer_size
        machine = execute(
            0x7CB4,
            0x7CE7,
            {
                "ax": 0x1234,
                "bx": 0x2345,
                "cx": 0x3456,
                "dx": 0x4567,
                "si": 0x5678,
                "di": 0x6789,
                "bp": 0x789a,
                "ds": data_segment,
                "es": 0x3456,
            },
            [
                (data_segment, 0x27e3, bytes([index])),
                (
                    data_segment,
                    0x5221,
                    struct.pack("<HH", pointer_offset, framebuffer_segment),
                ),
                (data_segment, 0x7bb8, bytes(table)),
                (framebuffer_segment, 0, initial_framebuffer),
            ],
        )
        actual = bytes(
            machine.mem_read(framebuffer_segment * 16, framebuffer_size)
        )
        expected = bytearray(initial_framebuffer)
        changed_offsets = []
        for row, bits in enumerate(rows):
            for column in range(16):
                if (bits & (0x8000 >> column)) != 0:
                    offset = 0x12c5 + row * 320 + column
                    expected[offset] = 0xfe
                    changed_offsets.append(offset)
        if actual != bytes(expected):
            raise AssertionError(f"0x7CB4 mask {index} produced unexpected pixels")
        if machine.reg_read(UC_X86_REG_ES) != 0x3456:
            raise AssertionError("0x7CB4 did not preserve ES")
        if machine.reg_read(UC_X86_REG_DI) != 0x6789:
            raise AssertionError("0x7CB4 did not preserve DI")

        vectors.append(
            {
                "index": index,
                "input_pointer_offset_ignored": pointer_offset,
                "big_endian_rows": rows,
                "color": 0xfe,
                "changed_offsets": changed_offsets,
            }
        )

    return vectors


def queue_consume_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    tail_segment = 0x3000
    cases = [
        {
            "name": "ordinary_advance",
            "tail": 0x0100,
            "entry_bytes": 0x0020,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "sequence": 0x0010,
            "read_index": 0x0003,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_equals_end",
            "tail": 0x0F00,
            "entry_bytes": 0x00FE,
            "byte_count": 0x0200,
            "buffer_end": 0x1000,
            "sequence": 0x0100,
            "read_index": 0x0006,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_past_end",
            "tail": 0x0F00,
            "entry_bytes": 0x0100,
            "byte_count": 0x0200,
            "buffer_end": 0x1000,
            "sequence": 0x1234,
            "read_index": 0x0002,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_add_carry",
            "tail": 0xFF00,
            "entry_bytes": 0x0200,
            "byte_count": 0x0300,
            "buffer_end": 0xFFFF,
            "sequence": 0xFFFE,
            "read_index": 0x0010,
            "read_limit": 0x0020,
        },
        {
            "name": "read_index_past_limit",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0010,
            "buffer_end": 0x1000,
            "sequence": 0xFFFF,
            "read_index": 0x0007,
            "read_limit": 0x0007,
        },
        {
            "name": "read_index_equals_limit",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0010,
            "buffer_end": 0x1000,
            "sequence": 0x2222,
            "read_index": 0x0006,
            "read_limit": 0x0007,
        },
        {
            "name": "read_index_word_wrap",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0008,
            "buffer_end": 0x1000,
            "sequence": 0x3333,
            "read_index": 0xFFFF,
            "read_limit": 0x0000,
        },
        {
            "name": "tail_plus_header_wrap_is_discarded",
            "tail": 0xFFFF,
            "entry_bytes": 0x0000,
            "byte_count": 0x0000,
            "buffer_end": 0x0001,
            "sequence": 0x4444,
            "read_index": 0x0000,
            "read_limit": 0xFFFF,
        },
    ]
    vectors = []

    for case in cases:
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        tail = int(case["tail"])
        entry_bytes = int(case["entry_bytes"])
        byte_count = int(case["byte_count"])
        buffer_end = int(case["buffer_end"])
        sequence = int(case["sequence"])
        read_index = int(case["read_index"])
        read_limit = int(case["read_limit"])
        machine = execute(
            0xA3D0,
            0xA40A,
            initial,
            [
                (
                    data_segment,
                    0x0D90,
                    struct.pack("<HH", tail, tail_segment),
                ),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
                (data_segment, 0x131C, struct.pack("<H", sequence)),
                (data_segment, 0x0D60, struct.pack("<H", read_index)),
                (data_segment, 0x0D64, struct.pack("<H", read_limit)),
                (tail_segment, tail, struct.pack("<H", entry_bytes)),
            ],
        )

        after_header = (tail + 2) & 0xFFFF
        sum_after_header = after_header + entry_bytes
        candidate = sum_after_header & 0xFFFF
        wrapped = sum_after_header > 0xFFFF or candidate > buffer_end
        expected_tail = (
            (entry_bytes - 2) & 0xFFFF
            if wrapped
            else (tail + entry_bytes) & 0xFFFF
        )
        expected_count = (byte_count - entry_bytes) & 0xFFFF
        expected_sequence = (sequence + 1) & 0xFFFF
        expected_index = (read_index + 1) & 0xFFFF
        expected_limit = read_limit
        if expected_index > read_limit:
            expected_index = 1
            expected_limit = 0xFFFF

        observed = {
            "tail": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D90, 2)
            )[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            "sequence": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x131C, 2)
            )[0],
            "read_index": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D60, 2)
            )[0],
            "read_limit": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D64, 2)
            )[0],
        }
        expected = {
            "tail": expected_tail,
            "byte_count": expected_count,
            "sequence": expected_sequence,
            "read_index": expected_index,
            "read_limit": expected_limit,
        }
        if observed != expected:
            raise AssertionError(
                f"0xA3D0 {case['name']}: actual={observed}, expected={expected}"
            )
        for name in ("bx", "cx", "dx", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA3D0 did not preserve {name}")

        vectors.append(
            {
                **case,
                "wrapped": wrapped,
                "result_tail": observed["tail"],
                "result_byte_count": observed["byte_count"],
                "result_sequence": observed["sequence"],
                "result_read_index": observed["read_index"],
                "result_read_limit": observed["read_limit"],
            }
        )

    return vectors


def queue_wrap_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        {
            "name": "ordinary_advance",
            "cursor": 0x0100,
            "byte_count": 0x0020,
            "buffer_end": 0x1000,
            "head": 0x0555,
            "wrap_limit": 0xAAAA,
            "wrap_count": 0x0001,
        },
        {
            "name": "next_equals_end",
            "cursor": 0x0F00,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "head": 0x0666,
            "wrap_limit": 0xBBBB,
            "wrap_count": 0x0010,
        },
        {
            "name": "next_past_end",
            "cursor": 0x0F01,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "head": 0x0777,
            "wrap_limit": 0xCCCC,
            "wrap_count": 0x0100,
        },
        {
            "name": "cursor_add_carry",
            "cursor": 0xFF00,
            "byte_count": 0x0200,
            "buffer_end": 0xFFFF,
            "head": 0x0888,
            "wrap_limit": 0xDDDD,
            "wrap_count": 0x1000,
        },
        {
            "name": "zero_byte_count",
            "cursor": 0x1234,
            "byte_count": 0x0000,
            "buffer_end": 0xFFFF,
            "head": 0x0999,
            "wrap_limit": 0xEEEE,
            "wrap_count": 0xFFFE,
        },
        {
            "name": "one_byte_count",
            "cursor": 0x0000,
            "byte_count": 0x0001,
            "buffer_end": 0x0000,
            "head": 0x0AAA,
            "wrap_limit": 0x1111,
            "wrap_count": 0xFFFF,
        },
    ]
    vectors = []

    for case in cases:
        cursor = int(case["cursor"])
        byte_count = int(case["byte_count"])
        buffer_end = int(case["buffer_end"])
        head = int(case["head"])
        wrap_limit = int(case["wrap_limit"])
        wrap_count = int(case["wrap_count"])
        initial = {
            "ax": byte_count,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": cursor,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        machine = execute(
            0xA38E,
            0xA3AC,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D98, struct.pack("<H", wrap_limit)),
                (data_segment, 0x0DA0, struct.pack("<H", 0x5A5A)),
                (data_segment, 0x0D62, struct.pack("<H", wrap_count)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            ],
        )

        full_next = cursor + byte_count
        next_cursor = full_next & 0xFFFF
        wrapped = full_next > 0xFFFF or next_cursor > buffer_end
        expected = {
            "head": 0 if wrapped else head,
            "wrap_limit": head if wrapped else wrap_limit,
            "iteration_count": (byte_count - 2) & 0xFFFF,
            "wrap_count": (wrap_count + 1) & 0xFFFF,
        }
        observed = {
            "head": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
            )[0],
            "wrap_limit": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D98, 2)
            )[0],
            "iteration_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0DA0, 2)
            )[0],
            "wrap_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D62, 2)
            )[0],
        }
        if observed != expected:
            raise AssertionError(
                f"0xA38E {case['name']}: actual={observed}, expected={expected}"
            )

        expected_registers = {
            "ax": (byte_count - 2) & 0xFFFF,
            "si": next_cursor,
            "cx": head if wrapped else initial["cx"],
        }
        for name, expected_register in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != expected_register:
                raise AssertionError(
                    f"0xA38E {case['name']} {name}: "
                    f"actual={actual_register:#x}, expected={expected_register:#x}"
                )
        for name in ("bx", "dx", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA38E did not preserve {name}")

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        expected_carry = int(byte_count < 2)
        if carry != expected_carry:
            raise AssertionError(
                f"0xA38E {case['name']} carry={carry}, expected={expected_carry}"
            )

        vectors.append(
            {
                **case,
                "wrapped": wrapped,
                "result_cursor": next_cursor,
                "result_head": observed["head"],
                "result_wrap_limit": observed["wrap_limit"],
                "result_iteration_count": observed["iteration_count"],
                "result_wrap_count": observed["wrap_count"],
                "result_carry": carry,
            }
        )

    return vectors


def queue_room_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        {
            "name": "ordinary_room",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0x0100,
            "wrap_limit": 0x0200,
            "request": 0x0020,
        },
        {
            "name": "gap_too_small",
            "head": 0x0100,
            "tail": 0x0120,
            "byte_count": 0x0010,
            "wrap_limit": 0x1000,
            "request": 0x0010,
        },
        {
            "name": "gap_exactly_large_enough",
            "head": 0x0100,
            "tail": 0x0122,
            "byte_count": 0x0010,
            "wrap_limit": 0x0040,
            "request": 0x0010,
        },
        {
            "name": "total_exceeds_wrap_limit",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0x0100,
            "wrap_limit": 0x0129,
            "request": 0x0020,
        },
        {
            "name": "total_second_add_carry",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0xFFF0,
            "wrap_limit": 0xFFFF,
            "request": 0x0010,
        },
        {
            "name": "count_plus_ten_carry_is_discarded",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0xFFFC,
            "wrap_limit": 0x0007,
            "request": 0x0001,
        },
        {
            "name": "head_plus_request_carry_is_discarded",
            "head": 0xFFF0,
            "tail": 0xFFF5,
            "byte_count": 0x0000,
            "wrap_limit": 0x002A,
            "request": 0x0020,
        },
        {
            "name": "head_plus_padding_carry_is_discarded",
            "head": 0xFFF0,
            "tail": 0xFFF5,
            "byte_count": 0x0000,
            "wrap_limit": 0x000A,
            "request": 0x0000,
        },
    ]
    vectors = []

    for case in cases:
        head = int(case["head"])
        tail = int(case["tail"])
        byte_count = int(case["byte_count"])
        wrap_limit = int(case["wrap_limit"])
        request = int(case["request"])
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": request,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        globals_before = struct.pack("<HHHH", head, tail, byte_count, wrap_limit)
        machine = execute(
            0xA3AD,
            0xA3CF,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D90, struct.pack("<H", tail)),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x0D98, struct.pack("<H", wrap_limit)),
            ],
        )

        gap_needed = ((head + request) & 0xFFFF) + 0x12
        gap_needed &= 0xFFFF
        early_failure = head < tail < gap_needed
        total_base = (byte_count + 0x0A) & 0xFFFF
        total_sum = total_base + request
        total_needed = total_sum & 0xFFFF
        total_carry = total_sum > 0xFFFF
        if early_failure:
            result_ax = gap_needed
            insufficient_room = True
        else:
            result_ax = total_needed
            insufficient_room = total_carry or wrap_limit < total_needed

        actual_ax = machine.reg_read(UC_X86_REG_AX)
        actual_bx = machine.reg_read(UC_X86_REG_BX)
        actual_carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if actual_ax != result_ax:
            raise AssertionError(
                f"0xA3AD {case['name']} AX={actual_ax:#x}, expected={result_ax:#x}"
            )
        if actual_bx != tail:
            raise AssertionError(
                f"0xA3AD {case['name']} BX={actual_bx:#x}, expected={tail:#x}"
            )
        if actual_carry != int(insufficient_room):
            raise AssertionError(
                f"0xA3AD {case['name']} carry={actual_carry}, "
                f"expected={int(insufficient_room)}"
            )
        for name in ("cx", "dx", "si", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA3AD did not preserve {name}")

        globals_after = struct.pack(
            "<HHHH",
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D90, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D98, 2)
            )[0],
        )
        if globals_after != globals_before:
            raise AssertionError(f"0xA3AD {case['name']} changed queue globals")

        vectors.append(
            {
                **case,
                "early_gap_failure": early_failure,
                "result_needed": result_ax,
                "has_room": not insufficient_room,
                "result_carry": actual_carry,
            }
        )

    return vectors


def queue_state_le_one_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    zero_flag_states = []

    for state in range(0x100):
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": 0x2400,
            "es": 0x2800,
            "gs": data_segment,
        }
        machine = execute(
            0xA40B,
            0xA419,
            initial,
            [(data_segment, 0x0D5F, bytes([state]))],
        )

        zero_flag = (machine.reg_read(UC_X86_REG_EFLAGS) >> 6) & 1
        expected = int(state <= 1)
        if zero_flag != expected:
            raise AssertionError(
                f"0xA40B state={state:#x} ZF={zero_flag}, expected={expected}"
            )
        if zero_flag:
            zero_flag_states.append(state)
        for name, value in initial.items():
            if name in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[name])
                if actual_register != value:
                    raise AssertionError(f"0xA40B did not preserve {name}")
        actual_state = machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA40B changed state byte {state:#x}")

    return [
        {
            "tested_state_count": 0x100,
            "zero_flag_set_states": zero_flag_states,
            "zero_flag_clear_range": [2, 0xFF],
            "logical_result": "state <= 1",
        }
    ]


def flag_test_b17_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600

    for state in range(0x100):
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
            "flags": 0x0E93,
        }
        machine = execute(
            0xA634,
            0xA641,
            initial,
            [
                (data_segment, 0x0B17, bytes([state ^ 1])),
                (state_segment, 0x0B17, bytes([state])),
            ],
        )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        zero_flag = (flags >> 6) & 1
        parity_flag = (flags >> 2) & 1
        expected_zero = int((state & 1) == 0)
        if zero_flag != expected_zero or parity_flag != expected_zero:
            raise AssertionError(
                f"0xA634 state={state:#x} ZF/PF={zero_flag}/{parity_flag}, "
                f"expected={expected_zero}"
            )
        if flags & ((1 << 0) | (1 << 7) | (1 << 11)):
            raise AssertionError(f"0xA634 state={state:#x} did not clear CF/SF/OF")
        if flags & ((1 << 9) | (1 << 10)) != initial["flags"] & (
            (1 << 9) | (1 << 10)
        ):
            raise AssertionError(f"0xA634 state={state:#x} changed IF/DF")
        for name in (
            "ax",
            "bx",
            "cx",
            "dx",
            "si",
            "di",
            "bp",
            "ds",
            "es",
            "gs",
        ):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA634 did not preserve {name}")
        actual_state = machine.mem_read(state_segment * 16 + 0x0B17, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA634 changed state byte {state:#x}")

    return [
        {
            "tested_state_count": 0x100,
            "zero_flag_set_rule": "(state & 1) == 0",
            "zero_flag_clear_rule": "(state & 1) != 0",
            "logical_result": "(state & 1) != 0",
            "caller_branch": "JE skips the enabled-only store when the result is false",
        }
    ]


def queue_enqueue_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero", 0x0000, 0x0000, 0x0000),
        ("ordinary", 0x0100, 0x0200, 0x0020),
        ("head_wrap", 0xFFFF, 0x0000, 0x0001),
        ("count_wrap", 0x0000, 0xFFFF, 0x0001),
        ("both_wrap_to_zero", 0x8000, 0x8000, 0x8000),
        ("maximum_increment", 0x1234, 0xFFFF, 0xFFFF),
        ("both_adds_carry", 0xFFF0, 0xFFF8, 0x0020),
        ("high_bit_increment", 0xAAAA, 0x5555, 0x8001),
    ]
    vectors = []

    for name, head, byte_count, increment in cases:
        initial = {
            "ax": increment,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
        }
        machine = execute(
            0xA734,
            0xA73D,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
            ],
        )

        result_head = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
        )[0]
        result_byte_count = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
        )[0]
        expected_head = (head + increment) & 0xFFFF
        expected_byte_count = (byte_count + increment) & 0xFFFF
        if result_head != expected_head or result_byte_count != expected_byte_count:
            raise AssertionError(
                f"0xA734 {name}: head={result_head:#x}/{expected_head:#x}, "
                f"count={result_byte_count:#x}/{expected_byte_count:#x}"
            )
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0xA734 did not preserve {register}")
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != 0:
            raise AssertionError(f"0xA734 {name} did not clear carry")

        vectors.append(
            {
                "name": name,
                "head": head,
                "byte_count": byte_count,
                "increment": increment,
                "result_head": result_head,
                "result_byte_count": result_byte_count,
                "result_carry": carry,
            }
        )

    return vectors


def list_init_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero_bounds", 0x0000, 0x0000),
        ("ordinary", 0x3456, 0x4000),
        ("maximum_segment", 0xFFFF, 0x1234),
        ("maximum_end", 0x1357, 0xFFFF),
        ("matching_values", 0xAAAA, 0xAAAA),
    ]
    reset_offsets = (0x0D8C, 0x0D90, 0x0D96, 0x0D9A, 0x0DA0)
    vectors = []

    for name, base_segment, buffer_end in cases:
        initial_words = {
            0x0D8C: 0x1111,
            0x0D8E: 0x2222,
            0x0D90: 0x3333,
            0x0D92: 0x4444,
            0x0D94: 0x5555,
            0x0D96: 0x6666,
            0x0D98: 0x7777,
            0x0D9A: 0x8888,
            0x0D9C: 0x9999,
            0x0D9E: 0xAAAA,
            0x0DA0: 0xBBBB,
        }
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
        }
        memory = [
            (data_segment, offset, struct.pack("<H", value))
            for offset, value in initial_words.items()
        ]
        memory.extend(
            [
                (data_segment, 0x0A7E, struct.pack("<H", base_segment)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            ]
        )
        machine = execute(0xA757, 0xA777, initial, memory)

        expected_words = dict(initial_words)
        expected_words[0x0D8E] = base_segment
        expected_words[0x0D92] = base_segment
        expected_words[0x0D98] = buffer_end
        for offset in reset_offsets:
            expected_words[offset] = 0
        observed_words = {
            offset: struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + offset, 2)
            )[0]
            for offset in initial_words
        }
        if observed_words != expected_words:
            raise AssertionError(
                f"0xA757 {name}: actual={observed_words}, expected={expected_words}"
            )
        expected_registers = dict(initial)
        expected_registers["ax"] = buffer_end
        for register, value in expected_registers.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0xA757 did not preserve {register}")

        vectors.append(
            {
                "name": name,
                "base_segment": base_segment,
                "buffer_end": buffer_end,
                "head_pointer": [0, base_segment],
                "tail_pointer": [0, base_segment],
                "cleared_offsets": list(reset_offsets),
                "result_wrap_limit": buffer_end,
                "preserved_sentinel_offsets": [0x0D94, 0x0D9C, 0x0D9E],
            }
        )

    return vectors


def mem_copy_words_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("disjoint", 0x0100, 0x0200),
        ("same_pointer", 0x0100, 0x0100),
        ("forward_overlap", 0x0100, 0x0102),
        ("backward_overlap", 0x0102, 0x0100),
        ("source_offset_wrap", 0xFFFC, 0x0200),
        ("destination_offset_wrap", 0x0100, 0xFFFC),
    ]
    vectors = []

    def read_word(buffer: bytearray, offset: int) -> int:
        low = buffer[offset]
        high = buffer[(offset + 1) & 0xFFFF]
        return low | high << 8

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        buffer[offset] = value & 0xFF
        buffer[(offset + 1) & 0xFFFF] = value >> 8

    for name, source_offset, destination_offset in cases:
        initial_memory = bytearray((index * 37 + 11) & 0xFF for index in range(0x10000))
        source_words = (0x1122, 0x3344, 0x5566, 0x7788)
        for index, value in enumerate(source_words):
            write_word(initial_memory, (source_offset + index * 2) & 0xFFFF, value)

        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": source_offset,
            "di": destination_offset,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
            "flags": 0x0AD7,
        }
        machine = execute(
            0xA7E6,
            0xA7EC,
            initial,
            [(data_segment, 0, bytes(initial_memory))],
        )

        expected_memory = bytearray(initial_memory)
        copied_words = []
        for index in range(4):
            source = (source_offset + index * 2) & 0xFFFF
            destination = (destination_offset + index * 2) & 0xFFFF
            value = read_word(expected_memory, source)
            copied_words.append(value)
            write_word(expected_memory, destination, value)
        actual_memory = bytes(
            machine.mem_read(data_segment * 16, len(expected_memory))
        )
        if actual_memory != bytes(expected_memory):
            raise AssertionError(f"0xA7E6 {name} produced unexpected memory")

        expected_registers = dict(initial)
        expected_registers["si"] = (source_offset + 8) & 0xFFFF
        expected_registers["di"] = (destination_offset + 8) & 0xFFFF
        expected_registers["es"] = data_segment
        for register, value in expected_registers.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(
                        f"0xA7E6 {name} {register}={actual_register:#x}, "
                        f"expected={value:#x}"
                    )

        vectors.append(
            {
                "name": name,
                "source_offset": source_offset,
                "destination_offset": destination_offset,
                "copied_words_in_order": copied_words,
                "result_source_offset": expected_registers["si"],
                "result_destination_offset": expected_registers["di"],
                "result_es": data_segment,
                "preserved_flags": expected_registers["flags"],
            }
        )

    return vectors


def flag_gated_copy_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x4000
    render_segment = 0x6000
    copy_size = 0x60 * 4
    cases = [
        ("clear_zero", 0x00),
        ("clear_other_bits", 0xFE),
        ("set_one", 0x01),
        ("set_all_bits", 0xFF),
    ]
    source = bytes((index * 43 + 17) & 0xFF for index in range(copy_size))
    destination = bytes((index * 19 + 7) & 0xFF for index in range(copy_size))
    vectors = []

    for name, state in cases:
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": render_segment,
            "gs": state_segment,
            "flags": 0x0293,
        }
        render_memory = bytearray(0x800)
        render_memory[:] = bytes((index * 29 + 3) & 0xFF for index in range(0x800))
        source_index = 0x5251 - 0x5200
        destination_index = 0x5851 - 0x5200
        render_memory[source_index : source_index + copy_size] = source
        render_memory[destination_index : destination_index + copy_size] = destination
        data_decoy = bytes((index * 7 + 5) & 0xFF for index in range(0x800))
        state_decoy = bytes((index * 11 + 9) & 0xFF for index in range(0x800))

        machine = execute(
            0xA117,
            0xA133,
            initial,
            [
                (data_segment, 0x5200, data_decoy),
                (state_segment, 0x2751, bytes([state])),
                (state_segment, 0x5200, state_decoy),
                (render_segment, 0x5200, bytes(render_memory)),
            ],
        )

        expected_render = bytearray(render_memory)
        copied = (state & 1) == 0
        if copied:
            expected_render[
                destination_index : destination_index + copy_size
            ] = source
        actual_render = bytes(machine.mem_read(render_segment * 16 + 0x5200, 0x800))
        if actual_render != bytes(expected_render):
            raise AssertionError(f"0xA117 {name} produced unexpected ES memory")
        if machine.mem_read(data_segment * 16 + 0x5200, 0x800) != data_decoy:
            raise AssertionError(f"0xA117 {name} changed DS decoy memory")
        if machine.mem_read(state_segment * 16 + 0x5200, 0x800) != state_decoy:
            raise AssertionError(f"0xA117 {name} changed GS decoy memory")
        actual_state = machine.mem_read(state_segment * 16 + 0x2751, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA117 {name} changed the gate byte")

        expected_registers = dict(initial)
        if copied:
            expected_registers["cx"] = 0
            expected_registers["di"] = 0x5851 + copy_size
        for register, value in expected_registers.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA117 {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_zero = int((state & 1) == 0)
        zero_flag = (flags >> 6) & 1
        parity_flag = (flags >> 2) & 1
        if zero_flag != expected_zero or parity_flag != expected_zero:
            raise AssertionError(
                f"0xA117 {name} ZF/PF={zero_flag}/{parity_flag}, "
                f"expected={expected_zero}"
            )
        if flags & ((1 << 0) | (1 << 7) | (1 << 11)):
            raise AssertionError(f"0xA117 {name} did not clear CF/SF/OF")
        if flags & (1 << 9) != initial["flags"] & (1 << 9):
            raise AssertionError(f"0xA117 {name} changed IF")

        vectors.append(
            {
                "name": name,
                "state": state,
                "copied_384_bytes": copied,
                "result_cx": expected_registers["cx"],
                "result_di": expected_registers["di"],
                "preserved_si": expected_registers["si"],
                "preserved_ds": expected_registers["ds"],
                "result_zero_flag": zero_flag,
            }
        )

    return vectors


def update_vector(path: Path, vectors: list[dict[str, object]], check: bool) -> None:
    encoded = json.dumps(vectors, indent=2) + "\n"
    if check:
        if not path.is_file() or path.read_text(encoding="ascii") != encoded:
            raise SystemExit(f"{path}: stale or missing; regenerate without --check")
        print(f"OK: {path.relative_to(REPO_ROOT)} ({len(vectors)} vectors)")
        return

    path.write_text(encoded, encoding="ascii")
    print(f"wrote {path.relative_to(REPO_ROOT)} ({len(vectors)} vectors)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail unless committed vectors are current"
    )
    args = parser.parse_args()

    VECTOR_ROOT.mkdir(parents=True, exist_ok=True)
    update_vector(
        VECTOR_ROOT / "func_30cd_natural.json", text_width_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_7cb4_natural.json", mask_overlay_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a3d0_natural.json", queue_consume_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a38e_natural.json", queue_wrap_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a3ad_natural.json", queue_room_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a40b_natural.json", queue_state_le_one_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a634_natural.json", flag_test_b17_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a734_natural.json", queue_enqueue_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a757_natural.json", list_init_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a7e6_natural.json", mem_copy_words_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a117_natural.json", flag_gated_copy_vectors(), args.check
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
