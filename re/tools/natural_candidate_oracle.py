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
from collections.abc import Callable
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EAX,
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
    "eax": UC_X86_REG_EAX,
    "ax": UC_X86_REG_AX,
    "bx": UC_X86_REG_BX,
    "cx": UC_X86_REG_CX,
    "dx": UC_X86_REG_DX,
    "si": UC_X86_REG_SI,
    "di": UC_X86_REG_DI,
    "bp": UC_X86_REG_BP,
    "sp": UC_X86_REG_SP,
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
    interrupt_handler: Callable[[Uc, int], None] | None = None,
    code_handler: Callable[[Uc, int, int], None] | None = None,
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
        elif code_handler is not None:
            code_handler(machine, address, _size)

    machine.hook_add(UC_HOOK_CODE, stop_at_return)
    if interrupt_handler is not None:

        def handle_interrupt(
            machine: Uc, number: int, _data: object
        ) -> None:
            interrupt_handler(machine, number)

        machine.hook_add(UC_HOOK_INTR, handle_interrupt)
    # The stop address is a global execution boundary in Unicorn, so it cannot
    # be the routine's RET when a nested call targets a higher address.
    machine.emu_start(entry, 0x2ffff0, count=20000)
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


def list_read_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    cases = [
        {
            "name": "no_file_handle",
            "handle": 0,
            "head": 0x0100,
            "byte_count": 0x0020,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "zero_extent",
            "handle": 1,
            "head": 0x0000,
            "byte_count": 0x0000,
            "source_offset": 0,
            "source_remaining": 2,
            "reads": [(0x0000, 2, False)],
        },
        {
            "name": "ordinary_extent",
            "handle": 5,
            "head": 0x0120,
            "byte_count": 0x0040,
            "source_offset": 0x00123456,
            "source_remaining": 0x00010000,
            "reads": [(0x1234, 2, False)],
        },
        {
            "name": "head_and_count_wrap",
            "handle": 0x7FFF,
            "head": 0xFFFF,
            "byte_count": 0xFFFF,
            "source_offset": 0xFFFFFFFF,
            "source_remaining": 1,
            "reads": [(0xBEEF, 2, False)],
        },
        {
            "name": "short_read_retry",
            "handle": 9,
            "head": 0x2200,
            "byte_count": 0x3333,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010001,
            "reads": [(0x00A5, 1, False), (0xCAFE, 2, False)],
        },
        {
            "name": "carry_set_short_retry",
            "handle": 0x1234,
            "head": 0x3456,
            "byte_count": 0xFFFE,
            "source_offset": 0xFFFF0000,
            "source_remaining": 0x00000000,
            "reads": [(0x0005, 1, True), (0x55AA, 2, False)],
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        handle = int(case["handle"])
        head = int(case["head"])
        byte_count = int(case["byte_count"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case["reads"])
        calls: list[dict[str, int | bool | str]] = []
        read_index = 0
        initial_eax = 0xA5A50000 | (0x1000 + case_index)
        initial = {
            "eax": initial_eax,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": data_segment,
            "flags": 0x0ED7,
        }

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            responses: list[object] = reads,
        ) -> None:
            nonlocal read_index
            if number != 0x21:
                raise AssertionError(
                    f"0xA622 {case_name} invoked unexpected INT {number:#x}"
                )

            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset_high": machine.reg_read(UC_X86_REG_CX),
                        "offset_low": machine.reg_read(UC_X86_REG_DX),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, source_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, source_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F or read_index >= len(responses):
                raise AssertionError(
                    f"0xA622 {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )

            value, returned, failed = responses[read_index]
            read_index += 1
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            call_log.append(
                {
                    "call": "read",
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": machine.reg_read(UC_X86_REG_CX),
                    "destination_segment": destination_segment,
                    "destination_offset": destination_offset,
                    "returned": int(returned),
                    "carry": bool(failed),
                }
            )
            machine.mem_write(
                destination_segment * 16 + destination_offset,
                struct.pack("<H", int(value))[: int(returned)],
            )
            machine.reg_write(UC_X86_REG_AX, int(returned))
            set_carry(machine, bool(failed))

        machine = execute(
            0xA622,
            0xA633,
            initial,
            [
                (data_segment, 0x0D5B, struct.pack("<H", handle)),
                (data_segment, 0x0D84, struct.pack("<I", source_offset)),
                (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
                (
                    data_segment,
                    0x0D8C,
                    struct.pack("<HH", head, buffer_segment),
                ),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x0DBC, b"\x00"),
            ],
            interrupt_handler,
        )

        success = handle >= 1
        expected_extent = int(reads[-1][0]) if success else initial_eax & 0xFFFF
        expected_head = (head + 2) & 0xFFFF if success else head
        expected_byte_count = (
            (byte_count + 2) & 0xFFFF if success else byte_count
        )
        expected_source_offset = (
            (source_offset + 2) & 0xFFFFFFFF if success else source_offset
        )
        expected_source_remaining = (
            (source_remaining - 2) & 0xFFFFFFFF if success else source_remaining
        )
        expected_registers = {
            "eax": (initial_eax & 0xFFFF0000) | expected_extent,
            "bx": handle,
            "cx": 2,
            "dx": head if success else initial["dx"],
            "si": expected_head if success else initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": data_segment,
            "es": buffer_segment if success else initial["es"],
            "gs": data_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA622 {name} {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        observed = {
            "head": read_u16(machine, 0x0D8C),
            "byte_count": read_u16(machine, 0x0D9A),
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
        }
        expected_state = {
            "head": expected_head,
            "byte_count": expected_byte_count,
            "source_offset": expected_source_offset,
            "source_remaining": expected_source_remaining,
        }
        if observed != expected_state:
            raise AssertionError(
                f"0xA622 {name} state={observed}, expected={expected_state}"
            )
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA622 {name} carry={carry}, expected={int(not success)}"
            )
        if read_index != len(reads):
            raise AssertionError(f"0xA622 {name} did not consume all read responses")
        if len(calls) != len(reads) * 2:
            raise AssertionError(f"0xA622 {name} produced an unexpected call count")
        for call in calls:
            if call["handle"] != handle:
                raise AssertionError(f"0xA622 {name} used an unexpected file handle")
            if call["call"] == "seek" and (
                call["offset_high"] != source_offset >> 16
                or call["offset_low"] != source_offset & 0xFFFF
            ):
                raise AssertionError(f"0xA622 {name} sought to an unexpected offset")
            if call["call"] == "read" and (
                call["requested"] != 2
                or call["destination_segment"] != buffer_segment
                or call["destination_offset"] != head
            ):
                raise AssertionError(f"0xA622 {name} used an unexpected read request")

        vectors.append(
            {
                "name": name,
                "success": success,
                "handle": handle,
                "initial_head": head,
                "initial_byte_count": byte_count,
                "extent": expected_extent if success else None,
                "calls": calls,
                "result": observed,
                "result_cursor_segment": machine.reg_read(UC_X86_REG_ES),
                "result_cursor_offset": machine.reg_read(UC_X86_REG_SI),
                "result_carry": carry,
            }
        )

    return vectors


def banked_list_load_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    wrapper = 0xF000
    cases = [
        {
            "name": "initial_no_handle",
            "file_handle": 0,
            "buffer_end": 0x9000,
            "extent": 10,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "extent_two_empty_body",
            "file_handle": 4,
            "buffer_end": 0x9000,
            "extent": 2,
            "source_offset": 0,
            "source_remaining": 2,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 2),
                },
                {
                    "stage": "body",
                    "returned": 0,
                    "carry": False,
                    "payload": b"",
                },
            ],
        },
        {
            "name": "ordinary_extent",
            "file_handle": 5,
            "buffer_end": 0xA000,
            "extent": 10,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010020,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 10),
                },
                {
                    "stage": "body",
                    "returned": 8,
                    "carry": False,
                    "payload": b"BODYDATA",
                },
            ],
        },
        {
            "name": "short_reads_ignore_carry",
            "file_handle": 6,
            "buffer_end": 0x7F00,
            "extent": 12,
            "source_offset": 0x10203040,
            "source_remaining": 0x55667788,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 1,
                    "carry": False,
                    "payload": b"\x0c",
                },
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": True,
                    "payload": struct.pack("<H", 12),
                },
                {
                    "stage": "body",
                    "returned": 3,
                    "carry": True,
                    "payload": b"bad",
                },
                {
                    "stage": "body",
                    "returned": 10,
                    "carry": False,
                    "payload": b"0123456789",
                },
            ],
        },
        {
            "name": "body_handle_removed",
            "file_handle": 7,
            "buffer_end": 0x8800,
            "extent": 6,
            "source_offset": 0xFFFFFFFE,
            "source_remaining": 8,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 6),
                    "drop_handle": True,
                }
            ],
        },
        {
            "name": "extent_one_wraps_body_count",
            "file_handle": 8,
            "buffer_end": 0x7000,
            "extent": 1,
            "source_offset": 0x01020304,
            "source_remaining": 0x00020000,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 1),
                },
                {
                    "stage": "body",
                    "returned": 0xFFFF,
                    "carry": False,
                    "payload": b"",
                },
            ],
        },
    ]
    vectors = []
    wrapper_code = b"\xe8" + struct.pack("<h", 0xA642 - (wrapper + 3))

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        file_handle = int(case["file_handle"])
        buffer_end = int(case["buffer_end"])
        extent = int(case["extent"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case["reads"])
        body_count = (extent - 2) & 0xFFFF
        entry_start = (buffer_end - extent - 2) & 0xFFFF
        body_start = (entry_start + 2) & 0xFFFF
        read_index = 0
        calls: list[dict[str, int | bool | str]] = []

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            responses: list[object] = reads,
            call_log: list[dict[str, int | bool | str]] = calls,
        ) -> None:
            nonlocal read_index
            if number != 0x21 or read_index >= len(responses):
                raise AssertionError(
                    f"0xA642 {case_name} invoked an unexpected interrupt"
                )

            response = responses[read_index]
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            stage = str(response["stage"])
            expected_offset = source_offset + (2 if stage == "body" else 0)
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "stage": stage,
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset": (
                            machine.reg_read(UC_X86_REG_CX) << 16
                            | machine.reg_read(UC_X86_REG_DX)
                        ),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, expected_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, expected_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F:
                raise AssertionError(
                    f"0xA642 {case_name} invoked DOS function {function:#x}"
                )
            requested = machine.reg_read(UC_X86_REG_CX)
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            expected_count = 2 if stage == "initial" else body_count
            expected_destination = 0 if stage == "initial" else body_start
            if requested != expected_count or destination_offset != expected_destination:
                raise AssertionError(f"0xA642 {case_name} issued an unexpected read")
            if destination_segment != buffer_segment:
                raise AssertionError(
                    f"0xA642 {case_name} selected an unexpected buffer segment"
                )
            payload = bytes(response["payload"])
            if payload:
                machine.mem_write(
                    destination_segment * 16 + destination_offset, payload
                )
            call_log.append(
                {
                    "call": "read",
                    "stage": stage,
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": requested,
                    "destination": destination_offset,
                    "returned": int(response["returned"]),
                    "carry": bool(response["carry"]),
                }
            )
            machine.reg_write(UC_X86_REG_AX, int(response["returned"]))
            set_carry(machine, bool(response["carry"]))
            if response.get("drop_handle"):
                machine.mem_write(
                    data_segment * 16 + 0x0D5B, struct.pack("<H", 0)
                )
            read_index += 1

        initial_words = {
            0x0D8C: 0x1111,
            0x0D8E: 0x2222,
            0x0D90: 0x3333,
            0x0D92: 0x4444,
            0x0D96: 0x5555,
            0x0D98: 0x6666,
            0x0D9A: 0x7777,
            0x0DA0: 0x8888,
        }
        memory = [
            (0, wrapper, wrapper_code),
            (data_segment, 0x0A56, struct.pack("<H", 0xFFFF)),
            (data_segment, 0x0A58, struct.pack("<H", 0xFFFF)),
            (data_segment, 0x0A7E, struct.pack("<H", buffer_segment)),
            (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
            (data_segment, 0x0D84, struct.pack("<I", source_offset)),
            (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
            (data_segment, 0x0DBC, b"\x00"),
            (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            (buffer_segment, 0, bytes([0xCC]) * 0x10000),
        ]
        memory.extend(
            (data_segment, offset, struct.pack("<H", value))
            for offset, value in initial_words.items()
        )
        initial_eax = 0xA5A50000 | case_index
        machine = execute(
            wrapper,
            wrapper + len(wrapper_code),
            {
                "eax": initial_eax,
                "bx": 0x2222,
                "cx": 0x3333,
                "dx": 0x4444,
                "si": 0x5555,
                "di": 0x6666,
                "bp": 0x7777,
                "sp": 0xFF00,
                "ds": data_segment,
                "es": 0x4000,
                "gs": data_segment,
                "flags": 0x0202,
            },
            memory,
            interrupt_handler,
        )

        initial_failed = file_handle < 1
        body_failed = bool(reads and reads[-1].get("drop_handle"))
        success = not initial_failed and not body_failed
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA642 {name} carry={carry}, expected={int(not success)}"
            )
        if read_index != len(reads) or len(calls) != len(reads) * 2:
            raise AssertionError(f"0xA642 {name} did not consume all DOS responses")
        for call in calls:
            if call["handle"] != file_handle:
                raise AssertionError(f"0xA642 {name} changed the DOS handle")
            expected_offset = source_offset + (
                2 if call["stage"] == "body" else 0
            )
            if call["call"] == "seek" and call["offset"] != expected_offset:
                raise AssertionError(f"0xA642 {name} sought unexpectedly")

        if initial_failed:
            transferred = 0
            expected_tail = 0
            expected_head = 0
            expected_queued = 0
        elif body_failed:
            transferred = 2
            expected_tail = entry_start
            expected_head = body_start
            expected_queued = 2
        else:
            transferred = 2 + body_count
            expected_tail = entry_start
            expected_head = (body_start + body_count) & 0xFFFF
            expected_queued = (2 + body_count) & 0xFFFF

        observed = {
            "head": read_u16(machine, 0x0D8C),
            "head_segment": read_u16(machine, 0x0D8E),
            "tail": read_u16(machine, 0x0D90),
            "tail_segment": read_u16(machine, 0x0D92),
            "active": read_u16(machine, 0x0D96),
            "wrap_limit": read_u16(machine, 0x0D98),
            "byte_count": read_u16(machine, 0x0D9A),
            "iteration_count": read_u16(machine, 0x0DA0),
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
        }
        expected = {
            "head": expected_head,
            "head_segment": buffer_segment,
            "tail": expected_tail,
            "tail_segment": buffer_segment,
            "active": 0,
            "wrap_limit": buffer_end,
            "byte_count": expected_queued,
            "iteration_count": 0,
            "source_offset": (source_offset + transferred) & 0xFFFFFFFF,
            "source_remaining": (
                source_remaining - transferred
            ) & 0xFFFFFFFF,
        }
        if observed != expected:
            raise AssertionError(
                f"0xA642 {name} state={observed}, expected={expected}"
            )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF00:
            raise AssertionError(f"0xA642 {name} did not restore the caller stack")

        if not initial_failed:
            initial_header = struct.unpack(
                "<H", machine.mem_read(buffer_segment * 16, 2)
            )[0]
            relocated_header = struct.unpack(
                "<H",
                machine.mem_read(buffer_segment * 16 + entry_start, 2),
            )[0]
            if initial_header != extent or relocated_header != extent:
                raise AssertionError(f"0xA642 {name} misplaced the extent header")
        if success:
            body_responses = [
                response for response in reads if response["stage"] == "body"
            ]
            expected_payload = bytes(body_responses[-1]["payload"])
            if expected_payload and machine.mem_read(
                buffer_segment * 16 + body_start, len(expected_payload)
            ) != expected_payload:
                raise AssertionError(f"0xA642 {name} misplaced the entry body")

        vectors.append(
            {
                "name": name,
                "success": success,
                "extent": extent,
                "body_count": body_count if not initial_failed else None,
                "entry_start": entry_start if not initial_failed else None,
                "calls": calls,
                "result": observed,
                "result_carry": carry,
            }
        )

    return vectors


def ems_paged_read_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    page_frame_segment = 0x5000
    cases = [
        {
            "name": "direct_no_handle",
            "mode": "direct",
            "byte_count": 4,
            "file_handle": 0,
            "head": 0x0100,
            "queued": 0x0020,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "direct_zero_count",
            "mode": "direct",
            "byte_count": 0,
            "file_handle": 1,
            "head": 0,
            "queued": 0,
            "source_offset": 0,
            "source_remaining": 0,
            "reads": [(0, False, b"")],
        },
        {
            "name": "direct_short_retry",
            "mode": "direct",
            "byte_count": 4,
            "file_handle": 5,
            "head": 0x0220,
            "queued": 0x0030,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010001,
            "reads": [(2, False, b"no"), (4, False, b"DATA")],
        },
        {
            "name": "direct_oversized_result",
            "mode": "direct",
            "byte_count": 2,
            "file_handle": 0x7FFF,
            "head": 0xFFFE,
            "queued": 0xFFFF,
            "source_offset": 0xFFFFFFFF,
            "source_remaining": 1,
            "reads": [(3, True, b"XYZ")],
        },
        {
            "name": "banked_ems_cross_page",
            "mode": "ems",
            "byte_count": 9,
            "ems_handle": 0x2345,
            "head": 0x1234,
            "queued": 0x4567,
            "source_offset": 0x0000BFFD,
            "source_remaining": 0x00020000,
            "payload": b"EMS-CROSS",
        },
        {
            "name": "banked_ems_page_wrap_zero_count",
            "mode": "ems",
            "byte_count": 0,
            "ems_handle": 0xFFFE,
            "head": 0xAAAA,
            "queued": 0x5555,
            "source_offset": 0xFFFFC000,
            "source_remaining": 0,
            "payload": b"",
        },
        {
            "name": "banked_xms_even",
            "mode": "xms",
            "byte_count": 6,
            "xms_handle": 0x1357,
            "head": 0x0300,
            "queued": 0x0400,
            "source_offset": 0x10203040,
            "source_remaining": 0x55667788,
            "payload": b"XMS123",
        },
        {
            "name": "banked_xms_odd_rounds_move",
            "mode": "xms",
            "byte_count": 5,
            "xms_handle": 0x2468,
            "head": 0xFFFC,
            "queued": 0xFFFE,
            "source_offset": 0xFFFFFFFE,
            "source_remaining": 3,
            "payload": b"odd5!+",
        },
        {
            "name": "banked_without_memory_falls_back_to_file",
            "mode": "fallback",
            "byte_count": 3,
            "file_handle": 9,
            "head": 0x0800,
            "queued": 0x0900,
            "source_offset": 0xAABBCCDD,
            "source_remaining": 0x11223344,
            "reads": [(3, False, b"DOS")],
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        mode = str(case["mode"])
        byte_count = int(case["byte_count"])
        file_handle = int(case.get("file_handle", 0))
        ems_handle = int(case.get("ems_handle", 0xFFFF))
        xms_handle = int(case.get("xms_handle", 0xFFFF))
        head = int(case["head"])
        queued = int(case["queued"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case.get("reads", []))
        payload = bytes(case.get("payload", b""))
        calls: list[dict[str, int | bool | str]] = []
        read_index = 0
        initial_eax = 0xA5A50000 | (0x2000 + case_index)
        initial = {
            "eax": initial_eax,
            "bx": 0x2222,
            "cx": byte_count,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": data_segment,
            "flags": 0x0202,
        }
        destination_seed = bytes([0xCC]) * 0x10000
        page_frame_seed = bytearray([0xEE]) * 0x10000
        if mode == "ems":
            page_offset = source_offset & 0x3FFF
            page_frame_seed[page_offset : page_offset + len(payload)] = payload
        xms_descriptor_seed = bytes(range(0x80, 0x90))

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            responses: list[object] = reads,
        ) -> None:
            nonlocal read_index
            if number == 0x67:
                ax = machine.reg_read(UC_X86_REG_AX)
                if ax >> 8 != 0x44:
                    raise AssertionError(
                        f"0xA664 {case_name} invoked unexpected EMS function"
                    )
                call_log.append(
                    {
                        "call": "ems_map",
                        "handle": machine.reg_read(UC_X86_REG_DX),
                        "logical_page": machine.reg_read(UC_X86_REG_BX),
                        "physical_page": ax & 0xFF,
                    }
                )
                machine.reg_write(UC_X86_REG_AX, ax & 0x00FF)
                return

            if number != 0x21:
                raise AssertionError(
                    f"0xA664 {case_name} invoked unexpected INT {number:#x}"
                )
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset_high": machine.reg_read(UC_X86_REG_CX),
                        "offset_low": machine.reg_read(UC_X86_REG_DX),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, source_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, source_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F or read_index >= len(responses):
                raise AssertionError(
                    f"0xA664 {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )
            returned, failed, response_payload = responses[read_index]
            read_index += 1
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            call_log.append(
                {
                    "call": "read",
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": machine.reg_read(UC_X86_REG_CX),
                    "destination_segment": destination_segment,
                    "destination_offset": destination_offset,
                    "returned": int(returned),
                    "carry": bool(failed),
                }
            )
            machine.mem_write(
                destination_segment * 16 + destination_offset,
                bytes(response_payload),
            )
            machine.reg_write(UC_X86_REG_AX, int(returned))
            set_carry(machine, bool(failed))

        def code_handler(
            machine: Uc,
            address: int,
            _size: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            xms_payload: bytes = payload,
        ) -> None:
            if address == 0xA6AA:
                transferred = machine.reg_read(UC_X86_REG_EAX)
                source_segment = machine.reg_read(UC_X86_REG_DS)
                source_pointer = machine.reg_read(UC_X86_REG_SI)
                destination_segment = machine.reg_read(UC_X86_REG_ES)
                destination_offset = machine.reg_read(UC_X86_REG_DI)
                call_log.append(
                    {
                        "call": "far_memmove",
                        "byte_count": transferred,
                        "source_segment": source_segment,
                        "source_offset": source_pointer,
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                copied = bytes(
                    machine.mem_read(
                        source_segment * 16 + source_pointer, transferred
                    )
                )
                machine.mem_write(
                    destination_segment * 16 + destination_offset, copied
                )
                flags = machine.reg_read(UC_X86_REG_EFLAGS)
                machine.reg_write(UC_X86_REG_EFLAGS, flags & ~(1 << 10))
            elif address == 0xA6F0:
                descriptor = struct.unpack(
                    "<IHIHI",
                    machine.mem_read(data_segment * 16 + 0x0A6C, 16),
                )
                destination_offset = descriptor[4] & 0xFFFF
                destination_segment = descriptor[4] >> 16
                call_log.append(
                    {
                        "call": "xms_move",
                        "function": machine.reg_read(UC_X86_REG_EAX),
                        "length": descriptor[0],
                        "source_handle": descriptor[1],
                        "source_offset": descriptor[2],
                        "destination_handle": descriptor[3],
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                machine.mem_write(
                    destination_segment * 16 + destination_offset,
                    xms_payload[: descriptor[0]],
                )
                machine.reg_write(UC_X86_REG_AX, 1)
                set_carry(machine, False)

        memory = [
            (0, 0xA6AA, b"\x90" * 5),
            (0, 0xA6F0, b"\x90" * 4),
            (data_segment, 0x0A56, struct.pack("<H", xms_handle)),
            (data_segment, 0x0A58, struct.pack("<H", ems_handle)),
            (data_segment, 0x0A66, struct.pack("<H", page_frame_segment)),
            (data_segment, 0x0A6C, xms_descriptor_seed),
            (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
            (data_segment, 0x0D84, struct.pack("<I", source_offset)),
            (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
            (
                data_segment,
                0x0D8C,
                struct.pack("<HH", head, buffer_segment),
            ),
            (data_segment, 0x0D9A, struct.pack("<H", queued)),
            (
                data_segment,
                0x0DBC,
                bytes([1 if mode in {"ems", "xms", "fallback"} else 0]),
            ),
            (buffer_segment, 0, destination_seed),
            (page_frame_segment, 0, bytes(page_frame_seed)),
        ]
        machine = execute(
            0xA664,
            0xA73D,
            initial,
            memory,
            interrupt_handler,
            code_handler,
        )

        success = not (mode in {"direct", "fallback"} and file_handle < 1)
        if mode in {"ems", "xms"}:
            transferred = byte_count
        elif success:
            transferred = int(reads[-1][0])
        else:
            transferred = initial_eax & 0xFFFF
        expected_increment = transferred if success else 0
        expected_state = {
            "source_offset": (
                source_offset + expected_increment
            ) & 0xFFFFFFFF,
            "source_remaining": (
                source_remaining - expected_increment
            ) & 0xFFFFFFFF,
            "head": (head + expected_increment) & 0xFFFF,
            "queued": (queued + expected_increment) & 0xFFFF,
        }
        observed_state = {
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
            "head": read_u16(machine, 0x0D8C),
            "queued": read_u16(machine, 0x0D9A),
        }
        if observed_state != expected_state:
            raise AssertionError(
                f"0xA664 {name} state={observed_state}, "
                f"expected={expected_state}"
            )

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA664 {name} carry={carry}, expected={int(not success)}"
            )
        expected_eax = initial_eax
        if success and mode in {"ems", "xms"}:
            expected_eax = transferred
        elif success:
            expected_eax = (initial_eax & 0xFFFF0000) | transferred
        expected_bx = initial["bx"]
        expected_dx = initial["dx"]
        if mode == "ems":
            expected_bx = ((source_offset >> 14) + 4) & 0xFFFF
            expected_dx = ems_handle
        elif mode in {"direct", "fallback"}:
            expected_bx = file_handle
            if success:
                expected_dx = head
        expected_registers = {
            "eax": expected_eax,
            "bx": expected_bx,
            "cx": byte_count,
            "dx": expected_dx,
            "si": initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA664 {name} {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        if mode == "ems":
            map_calls = [call for call in calls if call["call"] == "ems_map"]
            expected_pages = [
                ((source_offset >> 14) + index) & 0xFFFF for index in range(4)
            ]
            if [call["logical_page"] for call in map_calls] != expected_pages:
                raise AssertionError(f"0xA664 {name} mapped unexpected EMS pages")
            if [call["physical_page"] for call in map_calls] != list(range(4)):
                raise AssertionError(f"0xA664 {name} mapped unexpected EMS frames")
            if any(call["handle"] != ems_handle for call in map_calls):
                raise AssertionError(f"0xA664 {name} used an unexpected EMS handle")
            move_calls = [
                call for call in calls if call["call"] == "far_memmove"
            ]
            if move_calls != [
                {
                    "call": "far_memmove",
                    "byte_count": byte_count,
                    "source_segment": page_frame_segment,
                    "source_offset": source_offset & 0x3FFF,
                    "destination_segment": buffer_segment,
                    "destination_offset": head,
                }
            ]:
                raise AssertionError(f"0xA664 {name} used an unexpected far move")
        elif mode == "xms":
            rounded_length = byte_count + (byte_count & 1)
            expected_xms_call = {
                "call": "xms_move",
                "function": 0x0B00,
                "length": rounded_length,
                "source_handle": xms_handle,
                "source_offset": source_offset,
                "destination_handle": 0,
                "destination_segment": buffer_segment,
                "destination_offset": head,
            }
            if calls != [expected_xms_call]:
                raise AssertionError(f"0xA664 {name} built an unexpected XMS move")
        else:
            if read_index != len(reads):
                raise AssertionError(f"0xA664 {name} missed a DOS response")
            if len(calls) != len(reads) * 2:
                raise AssertionError(f"0xA664 {name} made unexpected DOS calls")
            for call in calls:
                if call["handle"] != file_handle:
                    raise AssertionError(
                        f"0xA664 {name} used an unexpected DOS handle"
                    )
                if call["call"] == "seek" and (
                    call["offset_high"] != source_offset >> 16
                    or call["offset_low"] != source_offset & 0xFFFF
                ):
                    raise AssertionError(f"0xA664 {name} sought unexpectedly")
                if call["call"] == "read" and (
                    call["requested"] != byte_count
                    or call["destination_segment"] != buffer_segment
                    or call["destination_offset"] != head
                ):
                    raise AssertionError(f"0xA664 {name} read unexpectedly")

        expected_payload = b""
        if mode in {"ems", "xms"}:
            expected_payload = payload
        elif success:
            expected_payload = bytes(reads[-1][2])
        actual_payload = bytes(
            machine.mem_read(
                buffer_segment * 16 + head, len(expected_payload)
            )
        )
        if actual_payload != expected_payload:
            raise AssertionError(f"0xA664 {name} copied unexpected bytes")
        descriptor = bytes(machine.mem_read(data_segment * 16 + 0x0A6C, 16))
        if mode != "xms" and descriptor != xms_descriptor_seed:
            raise AssertionError(f"0xA664 {name} changed the XMS descriptor")

        vectors.append(
            {
                "name": name,
                "mode": mode,
                "success": success,
                "requested": byte_count,
                "transferred": transferred if success else None,
                "calls": calls,
                "result": observed_state,
                "result_carry": carry,
                "xms_descriptor": list(descriptor) if mode == "xms" else None,
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


def presentation_queue_finish_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("nonzero_one", 0x00, 0x0001, 0x0000, 0x1234),
        ("nonzero_high_bit", 0x02, 0x8000, 0x0000, 0x1234),
        ("nonzero_max", 0xFE, 0xFFFF, 0x0000, 0x1234),
        ("zero_no_handle", 0x00, 0x0000, 0x0000, 0x1234),
        ("zero_reserved_handle", 0x01, 0x0000, 0x1234, 0x1234),
        ("zero_preserve_high_bits", 0xFC, 0x0000, 0xABCD, 0xABCD),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, state, byte_count, file_handle, reserved_handle in cases:
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
            "gs": 0x3000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0xA2DD,
            0xA2F1,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", reserved_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5F, bytes([state])),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
            ],
        )

        expected_state = state | 1
        close_called = byte_count == 0
        if close_called:
            expected_state |= 2
        actual_state = machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0]
        actual_count = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
        )[0]
        actual_handle = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
        )[0]
        if actual_state != expected_state:
            raise AssertionError(
                f"0xA2DD {name} state={actual_state:#x}, expected={expected_state:#x}"
            )
        if actual_count != byte_count or actual_handle != file_handle:
            raise AssertionError(f"0xA2DD {name} changed queue count or file handle")

        expected_registers = dict(initial)
        if close_called:
            expected_registers["bx"] = file_handle
            expected_registers["cx"] = 0
        for register, value in expected_registers.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA2DD {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_zero = int(byte_count == 0)
        expected_sign = int(bool(byte_count & 0x8000))
        expected_parity = parity_even(byte_count)
        actual_zero = (flags >> 6) & 1
        actual_sign = (flags >> 7) & 1
        actual_parity = (flags >> 2) & 1
        if (actual_zero, actual_sign, actual_parity) != (
            expected_zero,
            expected_sign,
            expected_parity,
        ):
            raise AssertionError(f"0xA2DD {name} produced unexpected status flags")
        if flags & ((1 << 0) | (1 << 11)):
            raise AssertionError(f"0xA2DD {name} did not clear CF/OF")

        vectors.append(
            {
                "name": name,
                "initial_state": state,
                "byte_count": byte_count,
                "file_handle": file_handle,
                "reserved_handle": reserved_handle,
                "result_state": expected_state,
                "close_called": close_called,
                "result_bx": expected_registers["bx"],
                "result_cx": expected_registers["cx"],
                "result_zero_flag": actual_zero,
            }
        )

    return vectors


def resource_descriptor_lookup_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("first_entry", 0x0000, 0x2069),
        ("second_entry", 0x0001, 0x207F),
        ("ordinary_index_nine", 0x0009, 0x1357),
        ("highest_reachable_word_start", 0x3812, 0x2468),
        ("stride_wraps_to_first", 0x4000, 0x369C),
        ("signed_high_bit", 0x8000, 0x48AD),
        ("addition_overflow", 0x2000, 0x5ABE),
        ("maximum_index", 0xFFFF, 0x6BCF),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, index, record_offset in cases:
        table_offset = (0x1FB5 + index * 4) & 0xFFFF
        before_final_add = (0x1FB5 + index * 3) & 0xFFFF
        full_sum = before_final_add + index
        initial = {
            "ax": index,
            "bx": 0xA55A,
            "cx": 0x1357,
            "dx": 0x2468,
            "si": 0x369C,
            "di": 0x48AD,
            "bp": 0x5ABE,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0x9F80,
            0x9F8D,
            initial,
            [
                (data_segment, table_offset, struct.pack("<H", record_offset)),
                (initial["es"], table_offset, struct.pack("<H", 0xDEAD)),
                (initial["gs"], table_offset, struct.pack("<H", 0xBEEF)),
            ],
        )

        actual_bx = machine.reg_read(UC_X86_REG_BX)
        if actual_bx != record_offset:
            raise AssertionError(
                f"0x9F80 {name} bx={actual_bx:#x}, expected={record_offset:#x}"
            )
        for register in ("ax", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != initial[register]:
                raise AssertionError(f"0x9F80 did not preserve {register}")

        result = full_sum & 0xFFFF
        expected_flags = {
            "carry": int(full_sum > 0xFFFF),
            "parity": parity_even(result),
            "auxiliary_carry": int(
                ((before_final_add & 0xF) + (index & 0xF)) > 0xF
            ),
            "zero": int(result == 0),
            "sign": int(bool(result & 0x8000)),
            "overflow": int(
                bool((~(before_final_add ^ index) & (before_final_add ^ result)) & 0x8000)
            ),
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "auxiliary_carry": (flags >> 4) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9F80 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "index": index,
                "table_offset": table_offset,
                "record_offset": record_offset,
                "result_bx": actual_bx,
                "flags_from_final_add": actual_flags,
            }
        )

    return vectors


def presentation_update_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("inactive_zero", 0x00, 0x0000, 0x08, 0x5A, 0x1234, 0xFF, 0x00),
        ("inactive_other_bits", 0xFE, 0x0001, 0x08, 0xA5, 0x2468, 0x83, 0x40),
        ("active_no_redraw", 0x01, 0x0001, 0x00, 0x5A, 0x1357, 0x03, 0x80),
        ("active_redraw", 0xFF, 0xFFFF, 0x08, 0xA5, 0x2468, 0xFF, 0x02),
        ("active_high_ship_byte_only", 0x03, 0x8000, 0x00, 0x7E, 0x369C, 0x02, 0x04),
        ("active_zero_count", 0x01, 0x0000, 0x08, 0x5A, 0x48AD, 0x00, 0x08),
        ("active_reserved_handle", 0x81, 0x0000, 0x08, 0xC3, 0x5ABE, 0xFC, 0x10),
        ("active_request_sign", 0x01, 0x0001, 0x08, 0x11, 0x6BCF, 0x82, 0x20),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for (
        name,
        gate,
        byte_count,
        ship_low,
        redraw,
        active_line,
        request_flags,
        list_state,
    ) in cases:
        ship_flags = ship_low | (0x0800 if name == "active_high_ship_byte_only" else 0)
        file_handle = 0x1234 if name == "active_reserved_handle" else 0
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0x9F53,
            0x9F7F,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5F, bytes([list_state])),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x1FB2, bytes([gate])),
                (data_segment, 0x24F3, struct.pack("<H", ship_flags)),
                (data_segment, 0x27D8, bytes([redraw])),
                (data_segment, 0x6788, struct.pack("<H", active_line)),
                (data_segment, 0x67AA, bytes([request_flags])),
                (initial["gs"], 0x0D5F, bytes([0x99])),
                (initial["gs"], 0x0D9A, struct.pack("<H", 0x5555)),
                (initial["gs"], 0x1FB2, bytes([0xA4])),
                (initial["gs"], 0x24F3, struct.pack("<H", 0x8888)),
                (initial["gs"], 0x27D8, bytes([0x66])),
                (initial["gs"], 0x6788, struct.pack("<H", 0x7777)),
                (initial["gs"], 0x67AA, bytes([0xBB])),
            ],
        )

        active = bool(gate & 1)
        expected = {
            "gate": 0 if active else gate,
            "redraw": 1 if active and (ship_flags & 8) else redraw,
            "active_line": 0xFFFF if active else active_line,
            "request_flags": request_flags & 0xFD if active else request_flags,
            "list_state": list_state
            | (1 if active else 0)
            | (2 if active and byte_count == 0 else 0),
            "byte_count": byte_count,
            "file_handle": file_handle,
            "ship_flags": ship_flags,
        }
        observed = {
            "gate": machine.mem_read(data_segment * 16 + 0x1FB2, 1)[0],
            "redraw": machine.mem_read(data_segment * 16 + 0x27D8, 1)[0],
            "active_line": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x6788, 2)
            )[0],
            "request_flags": machine.mem_read(data_segment * 16 + 0x67AA, 1)[0],
            "list_state": machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            "file_handle": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
            )[0],
            "ship_flags": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x24F3, 2)
            )[0],
        }
        if observed != expected:
            raise AssertionError(
                f"0x9F53 {name} state={observed}, expected={expected}"
            )
        for register, value in initial.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x9F53 {name} did not preserve {register}")

        gs_decoys = {
            "list_state": machine.mem_read(initial["gs"] * 16 + 0x0D5F, 1)[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0D9A, 2)
            )[0],
            "gate": machine.mem_read(initial["gs"] * 16 + 0x1FB2, 1)[0],
            "ship_flags": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x24F3, 2)
            )[0],
            "redraw": machine.mem_read(initial["gs"] * 16 + 0x27D8, 1)[0],
            "active_line": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x6788, 2)
            )[0],
            "request_flags": machine.mem_read(initial["gs"] * 16 + 0x67AA, 1)[0],
        }
        expected_decoys = {
            "list_state": 0x99,
            "byte_count": 0x5555,
            "gate": 0xA4,
            "ship_flags": 0x8888,
            "redraw": 0x66,
            "active_line": 0x7777,
            "request_flags": 0xBB,
        }
        if gs_decoys != expected_decoys:
            raise AssertionError(f"0x9F53 {name} accessed GS-owned decoy data")

        flag_result = expected["request_flags"] if active else gate & 1
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": parity_even(flag_result),
            "zero": int(flag_result == 0),
            "sign": int(bool(flag_result & 0x80)),
            "overflow": 0,
            "interrupt": 1,
            "direction": 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9F53 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "initial_gate": gate,
                "byte_count": byte_count,
                "ship_flags": ship_flags,
                "initial_redraw": redraw,
                "initial_active_line": active_line,
                "initial_request_flags": request_flags,
                "initial_list_state": list_state,
                "result": observed,
                "final_flags": actual_flags,
            }
        )

    return vectors


def close_file_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero_handle_zero_reserved", 0x0000, 0x0000, None),
        ("zero_handle", 0x0000, 0x1234, None),
        ("reserved_handle", 0x1234, 0x1234, None),
        ("reserved_max_handle", 0xFFFF, 0xFFFF, None),
        ("close_success", 0x0005, 0x1234, None),
        ("close_failure", 0x2468, 0x1234, 0x0006),
        ("close_max_handle", 0xFFFF, 0x0000, None),
    ]
    initial_bounds = (0x1111, 0x2222, 0x3333, 0x4444)
    vectors = []

    for case_index, (name, handle, reserved_handle, dos_error) in enumerate(cases):
        initial_ax = (0x1200 + case_index * 0x111 + 0x5A) & 0xFFFF
        initial = {
            "ax": initial_ax,
            "bx": 0xA55A,
            "cx": 0xB66B,
            "dx": 0xC77C,
            "si": 0xD88D,
            "di": 0xE99E,
            "bp": 0xFAAF,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        interrupts = []

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            calls: list[dict[str, int]] = interrupts,
            error: int | None = dos_error,
        ) -> None:
            if number != 0x21 or machine.reg_read(UC_X86_REG_AX) >> 8 != 0x3E:
                raise AssertionError(
                    f"0xA141 {case_name} invoked unexpected INT {number:#x}"
                )
            calls.append(
                {
                    "number": number,
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "stored_handle": struct.unpack(
                        "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
                    )[0],
                }
            )
            flags = machine.reg_read(UC_X86_REG_EFLAGS)
            if error is None:
                machine.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
            else:
                machine.reg_write(UC_X86_REG_AX, error)
                machine.reg_write(UC_X86_REG_EFLAGS, flags | 1)

        machine = execute(
            0xA141,
            0xA15E,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", reserved_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", handle)),
                (data_segment, 0x0D60, struct.pack("<4H", *initial_bounds)),
                (initial["gs"], 0x0A86, struct.pack("<H", 0xABCD)),
                (initial["gs"], 0x0D5B, struct.pack("<H", 0xBCDE)),
                (
                    initial["gs"],
                    0x0D60,
                    struct.pack("<4H", 0x5555, 0x6666, 0x7777, 0x8888),
                ),
            ],
            interrupt_handler,
        )

        closed = handle != 0 and handle != reserved_handle
        expected_bounds = (
            (0x0000, 0x0000, 0xFFFF, 0xFFFF) if closed else initial_bounds
        )
        observed_handle = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
        )[0]
        observed_bounds = struct.unpack(
            "<4H", machine.mem_read(data_segment * 16 + 0x0D60, 8)
        )
        if observed_handle != (0 if closed else handle):
            raise AssertionError(f"0xA141 {name} produced an unexpected stored handle")
        if observed_bounds != expected_bounds:
            raise AssertionError(
                f"0xA141 {name} bounds={observed_bounds}, expected={expected_bounds}"
            )
        if len(interrupts) != int(closed):
            raise AssertionError(f"0xA141 {name} produced unexpected interrupt count")
        if closed and interrupts != [
            {"number": 0x21, "handle": handle, "stored_handle": 0}
        ]:
            raise AssertionError(f"0xA141 {name} violated DOS-close ordering")

        expected_ax = initial_ax
        if closed:
            expected_ax = (
                dos_error if dos_error is not None else 0x3E00 | (initial_ax & 0xFF)
            )
        expected_registers = {
            "ax": expected_ax,
            "bx": handle,
            "cx": 0,
            "dx": initial["dx"],
            "si": initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, value in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA141 {name} {register}={actual_register:#x}, expected={value:#x}"
                )

        gs_decoys = (
            struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0A86, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0D5B, 2)
            )[0],
            struct.unpack(
                "<4H", machine.mem_read(initial["gs"] * 16 + 0x0D60, 8)
            ),
        )
        if gs_decoys != (
            0xABCD,
            0xBCDE,
            (0x5555, 0x6666, 0x7777, 0x8888),
        ):
            raise AssertionError(f"0xA141 {name} accessed GS-owned decoy data")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": 1,
            "zero": 1,
            "sign": 0,
            "overflow": 0,
            "interrupt": 1,
            "direction": 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0xA141 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "initial_handle": handle,
                "reserved_handle": reserved_handle,
                "dos_error": dos_error,
                "closed": closed,
                "interrupts": interrupts,
                "result_handle": observed_handle,
                "result_bounds": list(observed_bounds),
                "result_registers": expected_registers,
                "final_flags": actual_flags,
            }
        )

    return vectors


def resource_palette_blocks_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    stream_segment = 0x3000
    state_segment = 0x4000
    stream_offset = 0x1000
    cases = [
        ("immediate_terminator", [], 1, 0x1234, False),
        ("zero_count", [(7, b"")], 2, 0x5678, False),
        (
            "single_block_render_copy",
            [(2, bytes([0x3F, 0x20, 0x01, 0x02, 0x03, 0x04]))],
            3,
            0x9ABC,
            True,
        ),
        (
            "multiple_blocks_metric",
            [(1, bytes([9, 8, 7])), (5, bytes([1, 3, 5, 7, 9, 11]))],
            0,
            0x0040,
            False,
        ),
        ("metric_underflow", [(10, bytes([0xAA, 0xBB, 0xCC]))], 0, 3, True),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, blocks, wrap_index, initial_metric, copy_render_state in cases:
        stream = bytearray()
        for palette_start, payload in blocks:
            if len(payload) % 3 != 0:
                raise AssertionError(f"0xA0C3 {name} payload is not RGB-aligned")
            stream.extend((palette_start, len(payload) // 3))
            stream.extend(payload)
        stream.extend(b"\xff\xff")
        consumed = len(stream)

        palette = bytearray((index * 17 + 5) & 0xFF for index in range(768))
        expected_palette = bytearray(palette)
        for palette_start, payload in blocks:
            destination = palette_start * 3
            expected_palette[destination : destination + len(payload)] = payload

        render_destination = bytes(
            (index * 29 + 7) & 0xFF for index in range(0x180)
        )
        expected_render_destination = (
            bytes(expected_palette[:0x180])
            if copy_render_state
            else render_destination
        )
        stream_palette_decoy = bytes(
            (index * 31 + 11) & 0xFF for index in range(768)
        )
        data_stream_decoy = bytes(
            (index * 13 + 3) & 0xFF for index in range(len(stream))
        )

        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": stream_offset,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": stream_segment,
            "gs": state_segment,
            "flags": 0x0293,
        }
        machine = execute(
            0xA0C3,
            0xA116,
            initial,
            [
                (data_segment, stream_offset, data_stream_decoy),
                (data_segment, 0x5251, bytes(palette)),
                (data_segment, 0x5851, render_destination),
                (stream_segment, stream_offset, bytes(stream)),
                (stream_segment, 0x5251, stream_palette_decoy),
                (
                    state_segment,
                    0x2751,
                    bytes([0 if copy_render_state else 1]),
                ),
                (state_segment, 0x0D60, struct.pack("<H", wrap_index)),
                (state_segment, 0x0DAF, struct.pack("<H", initial_metric)),
                (state_segment, 0x5B55, b"\xA5"),
            ],
        )

        actual_palette = bytes(
            machine.mem_read(data_segment * 16 + 0x5251, len(palette))
        )
        if actual_palette != bytes(expected_palette):
            raise AssertionError(f"0xA0C3 {name} produced an unexpected palette")
        actual_render_destination = bytes(
            machine.mem_read(data_segment * 16 + 0x5851, 0x180)
        )
        if actual_render_destination != expected_render_destination:
            raise AssertionError(
                f"0xA0C3 {name} produced an unexpected render-state copy"
            )
        if (
            machine.mem_read(stream_segment * 16 + 0x5251, 768)
            != stream_palette_decoy
        ):
            raise AssertionError(f"0xA0C3 {name} changed the stream-segment decoy")
        if (
            machine.mem_read(data_segment * 16 + stream_offset, len(stream))
            != data_stream_decoy
        ):
            raise AssertionError(f"0xA0C3 {name} read the data-segment decoy stream")
        if machine.mem_read(state_segment * 16 + 0x5B55, 1) != b"\x01":
            raise AssertionError(f"0xA0C3 {name} did not set the palette dirty flag")

        expected_metric = initial_metric
        if wrap_index == 0:
            remaining = (initial_metric - consumed) & 0xFFFF
            expected_metric = ((remaining >> 2) - 2) & 0xFFFF
        actual_metric = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x0DAF, 2)
        )[0]
        if actual_metric != expected_metric:
            raise AssertionError(
                f"0xA0C3 {name} metric={actual_metric:#x}, "
                f"expected={expected_metric:#x}"
            )

        expected_di = initial["di"]
        if blocks:
            last_start, last_payload = blocks[-1]
            expected_di = 0x5251 + last_start * 3 + len(last_payload)
        if copy_render_state:
            expected_di = 0x5851 + 0x180
        expected_registers = {
            "ax": initial["ax"],
            "bx": initial["bx"],
            "cx": initial["cx"],
            "dx": initial["dx"],
            "si": (stream_offset + consumed) & 0xFFFF,
            "di": expected_di & 0xFFFF,
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, value in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA0C3 {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        if wrap_index == 0:
            shifted = ((initial_metric - consumed) & 0xFFFF) >> 2
            flag_result = (shifted - 2) & 0xFFFF
            carry = int(shifted < 2)
        else:
            flag_result = wrap_index
            carry = 0
        expected_flags = {
            "carry": carry,
            "parity": parity_even(flag_result),
            "zero": int(flag_result == 0),
            "sign": (flag_result >> 15) & 1,
            "overflow": 0,
            "interrupt": 1,
            "direction": 0,
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0xA0C3 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "blocks": [
                    {"start": start, "payload": list(payload)}
                    for start, payload in blocks
                ],
                "consumed_bytes": consumed,
                "copied_render_state": copy_render_state,
                "initial_wrap_index": wrap_index,
                "initial_metric": initial_metric,
                "result_metric": actual_metric,
                "result_stream_offset": expected_registers["si"],
                "result_di": expected_registers["di"],
                "final_flags": actual_flags,
            }
        )

    return vectors


def resource_switch_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    dta_segment = 0x5000
    dta_offset = 0x0200
    record_offset = 0x3000
    buffer_end = 0xFFFE
    cases = [
        {
            "name": "banked_primary_bounds",
            "mode": "banked",
            "resource_id": 2,
            "record_flags": 0x00,
            "variant": 0x0C,
            "inner_skip": 6,
            "palette_blocks": [],
            "padding": 2,
            "render_copy": False,
            "archive_size": 0x00123456,
            "primary_relative": 0x00000121,
            "alternate_relative": 0x00000341,
            "index_relative": 0x00000561,
            "read_failure": None,
        },
        {
            "name": "embedded_alternate_bounds",
            "mode": "embedded",
            "resource_id": 5,
            "record_flags": 0x04,
            "variant": 0x07,
            "inner_skip": 10,
            "palette_blocks": [(3, bytes([1, 2, 3]))],
            "padding": 4,
            "render_copy": True,
            "archive_offset": 0x12345670,
            "archive_remaining": 0x01020304,
            "path_handle": 0x0042,
            "primary_relative": 0x00001121,
            "alternate_relative": 0x00002231,
            "index_relative": 0x00003341,
            "read_failure": None,
        },
        {
            "name": "external_file_success",
            "mode": "external",
            "resource_id": 8,
            "record_flags": 0x00,
            "variant": 0x09,
            "inner_skip": 14,
            "palette_blocks": [
                (1, bytes([0x3F, 0x20, 0x10, 0x08, 0x04, 0x02])),
                (9, bytes([])),
            ],
            "padding": 1,
            "render_copy": False,
            "file_size": 0x00020000,
            "open_handle": 0x0033,
            "old_handle": 0x0022,
            "primary_relative": 0x00000211,
            "alternate_relative": 0x00000421,
            "index_relative": 0x00000631,
            "read_failure": None,
        },
        {
            "name": "wrapped_inner_cursor",
            "mode": "banked",
            "resource_id": 11,
            "record_flags": 0x00,
            "variant": 0x02,
            "inner_skip": 0xFFFF,
            "palette_blocks": [],
            "padding": 3,
            "render_copy": False,
            "archive_size": 0x00018000,
            "primary_relative": 0x00000031,
            "alternate_relative": 0x00000051,
            "index_relative": 0x00000071,
            "read_failure": None,
        },
        {
            "name": "external_open_failure",
            "mode": "external",
            "resource_id": 13,
            "record_flags": 0x04,
            "variant": 0x05,
            "file_size": 0x00009999,
            "open_error": 2,
            "path_handle": 0x1357,
            "archive_offset": 0x11112222,
            "archive_remaining": 0x33334444,
            "read_failure": "open",
        },
        {
            "name": "initial_read_failure",
            "mode": "banked",
            "resource_id": 17,
            "record_flags": 0x00,
            "variant": 0x03,
            "archive_size": 0x00012345,
            "read_failure": "initial",
        },
        {
            "name": "body_read_failure",
            "mode": "embedded",
            "resource_id": 19,
            "record_flags": 0x00,
            "variant": 0x01,
            "archive_offset": 0x01020304,
            "archive_remaining": 0x00054321,
            "path_handle": 0x0055,
            "read_failure": "body",
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def write_u32(machine: Uc, offset: int, value: int) -> None:
        machine.mem_write(
            data_segment * 16 + offset, struct.pack("<I", value & 0xFFFFFFFF)
        )

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        mode = str(case["mode"])
        resource_id = int(case["resource_id"])
        record_flags = int(case["record_flags"])
        variant = int(case["variant"])
        read_failure = case["read_failure"]
        old_handle = int(case.get("old_handle", 0))
        archive_offset = int(case.get("archive_offset", 0x55667788))
        archive_remaining = int(case.get("archive_remaining", 0x11223344))
        archive_size = int(case.get("archive_size", 0x01010101))
        file_size = int(case.get("file_size", 0x00020000))
        path_handle = int(case.get("path_handle", 0x2468))
        open_handle = int(case.get("open_handle", 0x0033))

        palette_blocks = list(case.get("palette_blocks", []))
        palette_stream = bytearray()
        for palette_start, payload in palette_blocks:
            palette_stream.extend((palette_start, len(payload) // 3))
            palette_stream.extend(payload)
        palette_stream.extend(b"\xff\xff")
        palette_consumed = len(palette_stream)
        read_extent = palette_consumed + 32
        final_metric = 6

        buffer = bytearray(0x10000)
        if read_failure not in {"open", "initial", "body"}:
            inner_skip = int(case["inner_skip"])
            struct.pack_into("<H", buffer, 0, inner_skip)
            cursor_after_length = 2
            unwrapped_cursor = cursor_after_length + inner_skip
            if unwrapped_cursor > 0xFFFF or unwrapped_cursor > buffer_end:
                palette_offset = 0
            else:
                palette_offset = cursor_after_length
            buffer[
                palette_offset : palette_offset + palette_consumed
            ] = palette_stream
            metadata_offset = (palette_offset + palette_consumed) & 0xFFFF
            padding = int(case["padding"])
            buffer[metadata_offset : metadata_offset + padding] = bytes(
                [0xFF]
            ) * padding
            metadata_offset = (metadata_offset + padding) & 0xFFFF
            primary_relative = int(case["primary_relative"])
            alternate_relative = int(case["alternate_relative"])
            index_relative = int(case["index_relative"])
            struct.pack_into("<I", buffer, metadata_offset, primary_relative)
            struct.pack_into(
                "<I", buffer, metadata_offset + 0x10, alternate_relative
            )
            struct.pack_into(
                "<I",
                buffer,
                metadata_offset + final_metric * 4,
                index_relative,
            )
        else:
            metadata_offset = 0
            primary_relative = 0
            alternate_relative = 0
            index_relative = 0

        initial_palette = bytes(
            (index * 7 + 3) & 0x3F for index in range(768)
        )
        initial_render = bytes(
            (index * 11 + 5) & 0xFF for index in range(0x180)
        )
        expected_palette = bytearray(initial_palette)
        for palette_start, payload in palette_blocks:
            destination = palette_start * 3
            expected_palette[destination : destination + len(payload)] = payload

        table_offset = 0x1FB5 + resource_id * 4
        table_entry = struct.pack("<HH", record_offset, 0xA000 + resource_id)
        record = bytes([record_flags, 0xA5]) + b"RESOURCE.DAT\0"
        initial_eax = 0x89AB0000 | resource_id
        calls: list[dict[str, int | str]] = []

        def code_handler(
            machine: Uc,
            address: int,
            _size: int,
            case_name: str = name,
            call_log: list[dict[str, int | str]] = calls,
            selected_mode: str = mode,
            selected_path_handle: int = path_handle,
            selected_failure: object = read_failure,
            selected_read_extent: int = read_extent,
        ) -> None:
            if address == 0x9FCB:
                if machine.reg_read(UC_X86_REG_DX) != record_offset + 2:
                    raise AssertionError(
                        f"0x9F8E {case_name} passed an unexpected filename pointer"
                    )
                call_log.append({"call": "path", "filename": record_offset + 2})
                machine.reg_write(UC_X86_REG_BX, selected_path_handle)
                machine.mem_write(
                    data_segment * 16 + 0x0AE2,
                    bytes([1 if selected_mode == "embedded" else 0]),
                )
            elif address == 0xA021:
                call_log.append({"call": "initial_read", "bytes": 2})
                if selected_failure == "initial":
                    set_carry(machine, True)
                    return
                source_offset = read_u32(machine, 0x0D84)
                source_remaining = read_u32(machine, 0x0D88)
                write_u32(machine, 0x0D84, source_offset + 2)
                write_u32(machine, 0x0D88, source_remaining - 2)
                machine.reg_write(UC_X86_REG_AX, selected_read_extent)
                machine.reg_write(UC_X86_REG_ES, buffer_segment)
                machine.reg_write(UC_X86_REG_SI, read_u16(machine, 0x0D8C))
                set_carry(machine, False)
            elif address == 0xA03E:
                byte_count = machine.reg_read(UC_X86_REG_CX)
                call_log.append({"call": "body_read", "bytes": byte_count})
                if selected_failure == "body":
                    set_carry(machine, True)
                    return
                source_offset = read_u32(machine, 0x0D84)
                source_remaining = read_u32(machine, 0x0D88)
                write_u32(machine, 0x0D84, source_offset + byte_count)
                write_u32(machine, 0x0D88, source_remaining - byte_count)
                machine.reg_write(UC_X86_REG_AX, byte_count)
                set_carry(machine, False)

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | str]] = calls,
            selected_file_size: int = file_size,
            selected_find_error: bool = case_index % 2 == 0,
            selected_failure: object = read_failure,
            selected_open_error: int = int(case.get("open_error", 0)),
            selected_open_handle: int = open_handle,
        ) -> None:
            if number != 0x21:
                raise AssertionError(
                    f"0x9F8E {case_name} invoked unexpected INT {number:#x}"
                )
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x3E:
                call_log.append(
                    {"call": "close", "handle": machine.reg_read(UC_X86_REG_BX)}
                )
                set_carry(machine, False)
            elif function == 0x2F:
                call_log.append({"call": "get_dta"})
                machine.reg_write(UC_X86_REG_ES, dta_segment)
                machine.reg_write(UC_X86_REG_BX, dta_offset)
            elif function == 0x4E:
                call_log.append({"call": "find_first"})
                machine.mem_write(
                    dta_segment * 16 + dta_offset + 0x1A,
                    struct.pack("<I", selected_file_size),
                )
                set_carry(machine, selected_find_error)
            elif function == 0x3D:
                call_log.append({"call": "open"})
                if selected_failure == "open":
                    machine.reg_write(UC_X86_REG_AX, selected_open_error)
                    set_carry(machine, True)
                else:
                    machine.reg_write(UC_X86_REG_AX, selected_open_handle)
                    set_carry(machine, False)
            else:
                raise AssertionError(
                    f"0x9F8E {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )

        memory = [
            (0, 0x9FCB, b"\x90" * 5),
            (0, 0xA021, b"\x90" * 3),
            (0, 0xA03E, b"\x90" * 3),
            (data_segment, 0x0A52, struct.pack("<I", archive_size)),
            (data_segment, 0x0A7E, struct.pack("<H", buffer_segment)),
            (data_segment, 0x0A86, struct.pack("<H", 0x7FFF)),
            (data_segment, 0x0A8A, struct.pack("<I", archive_offset)),
            (data_segment, 0x0A8E, struct.pack("<I", archive_remaining)),
            (data_segment, 0x0AE2, b"\xA5"),
            (data_segment, 0x0D5B, struct.pack("<H", old_handle)),
            (data_segment, 0x0D60, struct.pack("<4H", 9, 8, 7, 6)),
            (data_segment, 0x0DBC, bytes([1 if mode == "banked" else 0])),
            (data_segment, 0x1FB1, bytes([variant])),
            (data_segment, table_offset, table_entry),
            (data_segment, record_offset, record),
            (data_segment, 0x2751, bytes([0 if case.get("render_copy") else 1])),
            (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            (data_segment, 0x5251, initial_palette),
            (data_segment, 0x5851, initial_render),
            (buffer_segment, 0, bytes(buffer)),
        ]
        machine = execute(
            0x9F8E,
            0xA0C2,
            {
                "eax": initial_eax,
                "bx": 0x2222,
                "cx": 0x3333,
                "dx": 0x4444,
                "si": 0x5555,
                "di": 0x6666,
                "bp": 0x7777,
                "sp": 0xFF00,
                "ds": data_segment,
                "es": 0x4000,
                "gs": data_segment,
                "flags": 0x0202,
            },
            memory,
            interrupt_handler,
            code_handler,
        )

        expected_success = read_failure is None
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not expected_success):
            raise AssertionError(
                f"0x9F8E {name} carry={carry}, expected={int(not expected_success)}"
            )
        if machine.reg_read(UC_X86_REG_EAX) != initial_eax:
            raise AssertionError(f"0x9F8E {name} did not preserve EAX")
        if machine.reg_read(UC_X86_REG_SP) != 0xFF00:
            raise AssertionError(f"0x9F8E {name} did not restore SP")
        if read_u16(machine, 0x0D80) != resource_id:
            raise AssertionError(f"0x9F8E {name} stored an unexpected requested id")
        if read_u16(machine, 0x0D82) != resource_id:
            raise AssertionError(f"0x9F8E {name} stored an unexpected active id")
        expected_flags_word = record_flags | variant << 8
        if read_u16(machine, 0x0D76) != expected_flags_word:
            raise AssertionError(f"0x9F8E {name} stored unexpected resource flags")
        if machine.mem_read(data_segment * 16 + record_offset + 1, 1) != bytes(
            [variant]
        ):
            raise AssertionError(f"0x9F8E {name} did not update the record variant")
        if machine.mem_read(data_segment * 16 + 0x0D5F, 1) != b"\x00":
            raise AssertionError(f"0x9F8E {name} did not clear list state")
        if struct.unpack(
            "<4H", machine.mem_read(data_segment * 16 + 0x0D60, 8)
        ) != (0, 0, 0xFFFF, 0xFFFF):
            raise AssertionError(f"0x9F8E {name} did not reset list bounds")

        if mode == "banked":
            source_base = 0
            source_total = archive_size
            expected_handle = 0
            expected_path_calls = 0
            expected_dos_calls = 0
        elif mode == "embedded":
            source_base = archive_offset
            source_total = archive_remaining
            expected_handle = path_handle
            expected_path_calls = 1
            expected_dos_calls = 0
        else:
            source_base = 0 if read_failure != "open" else archive_offset
            source_total = file_size
            expected_handle = (
                path_handle if read_failure == "open" else open_handle
            )
            expected_path_calls = 1
            expected_dos_calls = 3

        actual_path_calls = sum(call["call"] == "path" for call in calls)
        actual_dos_calls = sum(
            call["call"] in {"get_dta", "find_first", "open"} for call in calls
        )
        if actual_path_calls != expected_path_calls:
            raise AssertionError(f"0x9F8E {name} produced unexpected path calls")
        if actual_dos_calls != expected_dos_calls:
            raise AssertionError(f"0x9F8E {name} produced unexpected DOS calls")
        if read_u16(machine, 0x0D5B) != expected_handle:
            raise AssertionError(f"0x9F8E {name} stored an unexpected file handle")

        expected_source_offset = source_base
        expected_source_remaining = source_total
        if read_failure not in {"open", "initial"}:
            expected_source_offset = (expected_source_offset + 2) & 0xFFFFFFFF
            expected_source_remaining = (
                expected_source_remaining - 2
            ) & 0xFFFFFFFF
        if read_failure is None:
            expected_source_offset = (
                expected_source_offset + read_extent - 2
            ) & 0xFFFFFFFF
            expected_source_remaining = (
                expected_source_remaining - (read_extent - 2)
            ) & 0xFFFFFFFF
        if read_u32(machine, 0x0D84) != expected_source_offset:
            raise AssertionError(f"0x9F8E {name} stored an unexpected source offset")
        if read_u32(machine, 0x0D88) != expected_source_remaining:
            raise AssertionError(
                f"0x9F8E {name} stored unexpected source remaining bytes"
            )

        if expected_success:
            actual_metric = read_u16(machine, 0x0DAF)
            if actual_metric != final_metric:
                raise AssertionError(
                    f"0x9F8E {name} metric={actual_metric:#x}, "
                    f"expected={final_metric:#x}"
                )
            selected_relative = (
                alternate_relative if record_flags & 4 else primary_relative
            )
            expected_ranges = (
                (expected_source_offset + selected_relative) & 0xFFFFFFFF,
                (expected_source_remaining - selected_relative) & 0xFFFFFFFF,
                (expected_source_offset + index_relative) & 0xFFFFFFFF,
                (expected_source_remaining - index_relative) & 0xFFFFFFFF,
            )
            actual_ranges = tuple(
                read_u32(machine, offset)
                for offset in (0x0D6E, 0x0D72, 0x0D78, 0x0D7C)
            )
            if actual_ranges != expected_ranges:
                raise AssertionError(
                    f"0x9F8E {name} ranges={actual_ranges}, "
                    f"expected={expected_ranges}"
                )
            if machine.mem_read(data_segment * 16 + 0x0DB7, 1) != b"\xff":
                raise AssertionError(f"0x9F8E {name} did not set resource marker")
            actual_palette = bytes(
                machine.mem_read(data_segment * 16 + 0x5251, 768)
            )
            if actual_palette != bytes(expected_palette):
                raise AssertionError(f"0x9F8E {name} produced unexpected palette")
            expected_render = (
                bytes(expected_palette[:0x180])
                if case.get("render_copy")
                else initial_render
            )
            if machine.mem_read(
                data_segment * 16 + 0x5851, 0x180
            ) != expected_render:
                raise AssertionError(
                    f"0x9F8E {name} produced unexpected render-state bytes"
                )

        vectors.append(
            {
                "name": name,
                "mode": mode,
                "resource_id": resource_id,
                "record_flags": expected_flags_word,
                "success": expected_success,
                "calls": calls,
                "source_offset": read_u32(machine, 0x0D84),
                "source_remaining": read_u32(machine, 0x0D88),
                "file_handle": read_u16(machine, 0x0D5B),
                "entry_metric": read_u16(machine, 0x0DAF),
                "metadata_offset": metadata_offset,
                "range_start": read_u32(machine, 0x0D6E),
                "range_remaining": read_u32(machine, 0x0D72),
                "index_start": read_u32(machine, 0x0D78),
                "index_remaining": read_u32(machine, 0x0D7C),
                "carry": carry,
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
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
        VECTOR_ROOT / "func_a622_natural.json", list_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a642_natural.json",
        banked_list_load_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a664_natural.json", ems_paged_read_vectors(), args.check
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
    update_vector(
        VECTOR_ROOT / "func_a2dd_natural.json",
        presentation_queue_finish_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f80_natural.json",
        resource_descriptor_lookup_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f53_natural.json",
        presentation_update_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a141_natural.json",
        close_file_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a0c3_natural.json",
        resource_palette_blocks_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f8e_natural.json",
        resource_switch_vectors(),
        args.check,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
