#!/usr/bin/env python3
"""Verify BBB's relocated BIOS 8 by 8 text renderer."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_bios_font.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_3066_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "636182d795c7a9417c00b4048fe34c0f9e6ddf60d3e595e70438a397a379ecfa"
)

ENTRY = 0x33E6
END = 0x344D
BODY_SHA256 = "630748baacde49c7a97451d01f64e9ed5d1b5573b3f899faba0346c40243fcb8"
DISPLAY_POINTER_OFFSET = 0x55F1
FONT_POINTER_OFFSET = 0x55F5

TEXT_SEGMENT = 0x2000
INCOMING_ES_SEGMENT = 0x4000
INCOMING_FS_SEGMENT = 0x6000
GAME_SEGMENT = 0x8000
FONT_SEGMENT = 0xA000
OUTPUT_SEGMENT = 0xC000
STACK_SEGMENT = 0xE000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x6FA0
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("87e11e785aa56996")
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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


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


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def case_text(name: str) -> bytes:
    if name == "immediate_nul":
        return b"\0"
    if name == "single_character_count_limit":
        return b"AB\0"
    if name == "nul_after_two_font_offset_wrap":
        return bytes((1, 0xFF, 0))
    if name == "zero_limit_means_256_characters":
        return bytes((index % 255) + 1 for index in range(256)) + b"\x7f\0"
    if name == "text_offset_wrap":
        return b"CD\0"
    if name == "display_offset_wrap":
        return b"E\0"
    if name == "zero_color_and_inherited_direction":
        return b"F\0"
    if name == "full_word_y_uses_byte_swap_row_formula":
        return b"G\0"
    raise AssertionError(f"unknown BIOS font case {name}")


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        text = case_text(name)
        text_offset = expected["text_offset"]
        display_offset = expected["display_offset"]
        font_offset = expected["font_offset"]
        x = expected["x"]
        y = expected["y"]
        color = expected["color"]
        limit = expected["character_limit"]
        direction_flag = expected["direction_flag"]

        text_before = bytearray(
            (index * 11 + case_index * 17 + 3) & 0xFF for index in range(SEGMENT_SIZE)
        )
        write_wrapped(text_before, text_offset, text)
        write_wrapped(
            text_before,
            DISPLAY_POINTER_OFFSET,
            struct.pack("<HH", display_offset ^ 0xFFFF, INCOMING_ES_SEGMENT),
        )
        write_wrapped(
            text_before,
            FONT_POINTER_OFFSET,
            struct.pack("<HH", font_offset ^ 0xFFFF, INCOMING_FS_SEGMENT),
        )
        incoming_es_before = bytearray(seeded_segment(case_index, 13, 0x25))
        incoming_fs_before = bytearray(seeded_segment(case_index, 17, 0x37))
        for decoy in (incoming_es_before, incoming_fs_before):
            write_wrapped(
                decoy,
                DISPLAY_POINTER_OFFSET,
                struct.pack("<HH", display_offset ^ 0xA5A5, INCOMING_ES_SEGMENT),
            )
            write_wrapped(
                decoy,
                FONT_POINTER_OFFSET,
                struct.pack("<HH", font_offset ^ 0x5A5A, INCOMING_FS_SEGMENT),
            )
        game_before = bytearray(seeded_segment(case_index, 19, 0x49))
        write_wrapped(
            game_before,
            DISPLAY_POINTER_OFFSET,
            struct.pack("<HH", display_offset, OUTPUT_SEGMENT),
        )
        write_wrapped(
            game_before,
            FONT_POINTER_OFFSET,
            struct.pack("<HH", font_offset, FONT_SEGMENT),
        )
        font_before = bytes(
            (index * 29 + case_index * 37 + 0x61) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        output_before = bytes(
            (index * 31 + case_index * 43 + 0x27) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        output_expected = bytearray(output_before)

        initial = {
            "eax": 0xA1A10000 | x,
            "ebx": 0xB2B20000 | y,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D40000 | (limit << 8) | color,
            "esi": 0xE5E50000 | text_offset,
            "edi": 0xF6F66780 + case_index,
            "ebp": 0x97977890 + case_index,
            "ds": TEXT_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": INCOMING_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        stack_before = bytearray(seeded_segment(case_index, 41, 0x73))
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        expected_write_events: list[tuple[int, int, int]] = []

        def stack_write(
            offset: int,
            value: int,
            stack: bytearray = stack_expected,
            events: list[tuple[int, int, int]] = expected_write_events,
        ) -> None:
            write_wrapped(stack, offset, struct.pack("<H", value & 0xFFFF))
            events.append((physical_address(STACK_SEGMENT, offset), 2, value & 0xFFFF))

        for offset, register_name in (
            (CALLER_SP - 2, "eax"),
            (CALLER_SP - 4, "ebx"),
            (CALLER_SP - 6, "ecx"),
            (CALLER_SP - 8, "edx"),
            (CALLER_SP - 10, "es"),
            (CALLER_SP - 12, "edi"),
            (CALLER_SP - 14, "ds"),
            (CALLER_SP - 16, "fs"),
        ):
            stack_write(offset, initial[register_name])

        row_offset = (((y << 8) | (y >> 8)) + (y << 6)) & 0xFFFF
        glyph_origin = (display_offset + row_offset + x) & 0xFFFF
        text_cursor = text_offset
        characters_drawn = 0
        remaining = limit
        stopped_on_nul = False
        final_add_carry = False

        while True:
            character = text_before[text_cursor]
            if character == 0:
                stopped_on_nul = True
                break
            stack_write(CALLER_SP - 18, glyph_origin)
            glyph = (font_offset + character * 8) & 0xFFFF
            pixel = glyph_origin
            for row in range(8):
                stack_write(CALLER_SP - 20, 8 - row)
                bits = font_before[glyph]
                glyph = (glyph + 1) & 0xFFFF
                for _column in range(8):
                    if bits & 0x80:
                        output_expected[pixel] = color
                        expected_write_events.append(
                            (physical_address(OUTPUT_SEGMENT, pixel), 1, color)
                        )
                    bits = (bits << 1) & 0xFF
                    pixel = (pixel + 1) & 0xFFFF
                pixel = (pixel + 0x138) & 0xFFFF
            text_cursor = (text_cursor + 1) & 0xFFFF
            final_add_carry = glyph_origin > 0xFFF7
            glyph_origin = (glyph_origin + 8) & 0xFFFF
            characters_drawn += 1
            remaining = (remaining - 1) & 0xFF
            if remaining == 0:
                break

        expected_defined_flags = 0x0044 | (
            0 if stopped_on_nul or not final_add_carry else 0x0001
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(TEXT_SEGMENT * 16, bytes(text_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, bytes(incoming_es_before))
        machine.mem_write(INCOMING_FS_SEGMENT * 16, bytes(incoming_fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(FONT_SEGMENT * 16, font_before)
        machine.mem_write(OUTPUT_SEGMENT * 16, output_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(
            UC_X86_REG_EFLAGS,
            0x0293 | (0x0400 if direction_flag else 0),
        )

        reached_return: list[int] = []
        observed_write_events: list[tuple[int, int, int]] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls: list[int] = reached_return,
        ) -> None:
            if address == RETURN_ADDRESS:
                calls.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

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
        machine.emu_start(ENTRY, 0, count=500_000)

        actual_output = bytes(machine.mem_read(OUTPUT_SEGMENT * 16, SEGMENT_SIZE))
        expected_registers = dict(initial)
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | text_cursor

        assert reached_return == [RETURN_ADDRESS], name
        assert characters_drawn == expected["characters_drawn"], name
        assert text_cursor == expected["text_end_offset"], name
        assert (
            hashlib.sha256(actual_output).hexdigest()
            == expected["output_segment_sha256"]
        ), name
        assert actual_output == bytes(output_expected), name
        assert observed_write_events == expected_write_events, name
        assert snapshot_registers(machine) == expected_registers, name
        assert (
            machine.reg_read(UC_X86_REG_EFLAGS) & DEFINED_FLAG_MASK
            == expected_defined_flags
        ), name
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0200), name
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == direction_flag, (
            name
        )
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(TEXT_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            text_before
        )
        assert bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            incoming_es_before
        )
        assert bytes(machine.mem_read(INCOMING_FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            incoming_fs_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(FONT_SEGMENT * 16, SEGMENT_SIZE)) == font_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 4,
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        )

        rows.append(dict(expected))
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB BIOS-font body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_bios_font_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "display_pointer_offset": DISPLAY_POINTER_OFFSET,
            "font_pointer_offset": FONT_POINTER_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB BIOS-font cases")


if __name__ == "__main__":
    main()
