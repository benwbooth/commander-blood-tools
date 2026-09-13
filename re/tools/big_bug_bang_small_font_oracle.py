#!/usr/bin/env python3
"""Verify BBB's relocated planar 4x5 small-font renderer."""

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

from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_INS_OUT,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_small_font.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_36ea_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "b93a876eaee664919a58ad8ea36e173bd55f89fc87086a692f3a9859f0688411"
)

ENTRY = 0x3A78
END = 0x3B03
BODY_SHA256 = "1685643fec0b475d7582de4414d97cde4a9c799a7ceaf13ff8960f7972057109"
DISPLAY_POINTER_OFFSET = 0x55E9
CHARACTER_MAP_OFFSET = 0x78AE
GLYPH_TABLE_OFFSET = 0x792E
COMMANDER_DISPLAY_POINTER_OFFSET = 0x5219
COMMANDER_CHARACTER_MAP_OFFSET = 0x6FA8
COMMANDER_GLYPH_TABLE_OFFSET = 0x7028

TEXT_SEGMENT = 0x2000
EXTRA_SEGMENT = 0x4000
FS_SEGMENT = 0x6000
GAME_SEGMENT = 0x8000
OUTPUT_SEGMENT = 0xA000
STACK_SEGMENT = 0xC000
RETURN_SEGMENT = 0xE000
RETURN_OFFSET = 0x0100
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
STACK_SENTINEL = bytes.fromhex("e12dd23cc34bb45a")
VGA_SEQUENCER_INDEX_PORT = 0x03C4
VGA_SEQUENCER_DATA_PORT = 0x03C5
PLANAR_ROW_BYTE_COUNT = 80
GLYPH_HEIGHT = 5
GLYPH_WIDTH = 4

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

CASES = (
    {
        "name": "empty_string_writes_starting_mask",
        "text": b"\0",
        "mapping": {},
    },
    {
        "name": "single_glyph_starting_plane_zero",
        "text": b"A\0",
        "mapping": {0x41: 2},
    },
    {
        "name": "single_glyph_starting_plane_one",
        "text": b"A\0",
        "mapping": {0x41: 3},
    },
    {
        "name": "single_glyph_starting_plane_two",
        "text": b"A\0",
        "mapping": {0x41: 4},
    },
    {
        "name": "single_glyph_starting_plane_three",
        "text": b"A\0",
        "mapping": {0x41: 5},
    },
    {
        "name": "high_bit_map_skip_retains_fixed_advance",
        "text": b"XAB\0",
        "mapping": {0x58: 0xFF, 0x41: 6, 0x42: 7},
    },
    {
        "name": "source_offset_wrap",
        "text": b"AB\0",
        "mapping": {0x41: 8, 0x42: 9},
    },
    {
        "name": "inherited_backward_source_direction",
        "text": b"AB\0",
        "mapping": {0x41: 10, 0x42: 11},
    },
    {
        "name": "full_word_row_formula_and_output_wrap",
        "text": b"C\0",
        "mapping": {0x43: 12},
    },
    {
        "name": "high_character_indexes_past_nominal_map_extent",
        "text": b"\xe9\0",
        "mapping": {0xE9: 13},
    },
    {
        "name": "all_skipped_characters_leave_pixels_unchanged",
        "text": b"XYZ\0",
        "mapping": {0x58: 0x80, 0x59: 0xFE, 0x5A: 0xFF},
    },
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
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


def snapshot_defined_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(flags & 0x0001),
        "pf": bool(flags & 0x0004),
        "af": bool(flags & 0x0010),
        "zf": bool(flags & 0x0040),
        "sf": bool(flags & 0x0080),
        "of": bool(flags & 0x0800),
    }


def add16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left + right) & 0xFFFF
    return {
        "cf": left + right > 0xFFFF,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": ((left ^ right ^ result) & 0x10) != 0,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": ((~(left ^ right) & (left ^ result)) & 0x8000) != 0,
    }


def commander_vectors() -> list[dict[str, Any]]:
    fixture = COMMANDER_FIXTURE.read_bytes()
    assert sha256(fixture) == COMMANDER_FIXTURE_SHA256
    return json.loads(fixture)


def glyph_pattern(glyph_index: int, case_index: int) -> bytes:
    return bytes(
        (
            0x00,
            0x80,
            0x50,
            0x10,
            (glyph_index * 0x11 + case_index * 7) & 0xF0,
        )
    )


def stack_write(
    target: bytearray,
    offset: int,
    value: int,
    events: list[tuple[int, int, int]],
) -> None:
    struct.pack_into("<H", target, offset, value & 0xFFFF)
    events.append((physical_address(STACK_SEGMENT, offset), 2, value & 0xFFFF))


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    expected_rows = commander_vectors()
    assert len(CASES) == len(expected_rows)

    for case_index, (case, expected) in enumerate(
        zip(CASES, expected_rows, strict=True)
    ):
        name = case["name"]
        assert name == expected["name"]
        text = case["text"]
        mapping = case["mapping"]
        text_offset = expected["text_offset"]
        framebuffer_offset = expected["framebuffer_offset"]
        x = expected["x"]
        y = expected["y"]
        color = expected["color"]
        direction_flag = expected["direction_flag"]
        direction = -1 if direction_flag else 1

        text_before = bytearray(
            (offset * 13 + case_index * 19 + 0x05) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        for index, value in enumerate(text):
            text_before[(text_offset + direction * index) & 0xFFFF] = value
        extra_before = seeded_segment(case_index, 17, 0x17)
        fs_before = seeded_segment(case_index, 19, 0x29)
        game_before = bytearray(
            (offset * 17 + case_index * 23 + 0x39) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        output_before = bytes(
            (offset * 37 + case_index * 41 + 0x2B) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        output_expected = bytearray(output_before)
        stack_before = bytearray(
            (offset * 29 + case_index * 31 + 0x6D) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        stack_expected = bytearray(stack_before)

        real_pointer = struct.pack("<HH", framebuffer_offset, OUTPUT_SEGMENT)
        wrong_pointer = struct.pack("<HH", framebuffer_offset ^ 0xA5A5, EXTRA_SEGMENT)
        write_wrapped(game_before, DISPLAY_POINTER_OFFSET, real_pointer)
        write_wrapped(game_before, COMMANDER_DISPLAY_POINTER_OFFSET, wrong_pointer)
        for decoy in (text_before, extra_before, fs_before):
            write_wrapped(decoy, DISPLAY_POINTER_OFFSET, wrong_pointer)
            write_wrapped(decoy, COMMANDER_DISPLAY_POINTER_OFFSET, wrong_pointer)

        for character, glyph_index in mapping.items():
            game_before[(CHARACTER_MAP_OFFSET + character) & 0xFFFF] = glyph_index
            game_before[(COMMANDER_CHARACTER_MAP_OFFSET + character) & 0xFFFF] = (
                glyph_index ^ 0x7F
            )
            for decoy in (text_before, extra_before, fs_before):
                decoy[(CHARACTER_MAP_OFFSET + character) & 0xFFFF] = glyph_index ^ 0x55
                decoy[(COMMANDER_CHARACTER_MAP_OFFSET + character) & 0xFFFF] = (
                    glyph_index ^ 0x33
                )
            if glyph_index & 0x80:
                continue
            pattern = glyph_pattern(glyph_index, case_index)
            glyph_offset = (GLYPH_TABLE_OFFSET + glyph_index * GLYPH_HEIGHT) & 0xFFFF
            write_wrapped(stack_before, glyph_offset, pattern)
            write_wrapped(
                stack_before,
                (COMMANDER_GLYPH_TABLE_OFFSET + glyph_index * GLYPH_HEIGHT) & 0xFFFF,
                bytes(value ^ 0xFF for value in pattern),
            )
            for decoy in (text_before, extra_before, fs_before, game_before):
                write_wrapped(
                    decoy, glyph_offset, bytes(value ^ 0xA5 for value in pattern)
                )

        initial = {
            "eax": 0xA1A10000 | x,
            "ebx": 0xB2B20000 | y,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D45A00 | color,
            "esi": 0xE5E50000 | text_offset,
            "edi": 0xF6F66780 + case_index,
            "ebp": 0x97977890 + case_index,
            "ds": TEXT_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        struct.pack_into("<HH", stack_before, CALLER_SP, RETURN_OFFSET, RETURN_SEGMENT)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected[:] = stack_before

        expected_write_events: list[tuple[int, int, int]] = []
        for offset, value in (
            (CALLER_SP - 2, initial["eax"]),
            (CALLER_SP - 4, initial["ebx"]),
            (CALLER_SP - 6, initial["ecx"]),
            (CALLER_SP - 8, initial["edx"]),
            (CALLER_SP - 10, EXTRA_SEGMENT),
            (CALLER_SP - 12, initial["edi"]),
            (CALLER_SP - 14, initial["esi"]),
            (CALLER_SP - 16, initial["ebp"]),
        ):
            stack_write(stack_expected, offset, value, expected_write_events)

        row_offset = ((y << 4) + (y << 6)) & 0xFFFF
        coordinate_ax = ((x >> 2) + row_offset) & 0xFFFF
        glyph_origin = (framebuffer_offset + coordinate_ax) & 0xFFFF
        starting_plane = x & 3
        starting_mask = (0x11 << starting_plane) & 0xFF
        saved_ax = (coordinate_ax & 0xFF00) | starting_mask
        expected_ports = [(VGA_SEQUENCER_INDEX_PORT, 1, 2)]
        characters_processed = 0
        glyphs_drawn = 0
        text_cursor = text_offset

        while True:
            expected_ports.append((VGA_SEQUENCER_DATA_PORT, 1, starting_mask))
            stack_write(stack_expected, CALLER_SP - 18, saved_ax, expected_write_events)
            stack_write(
                stack_expected, CALLER_SP - 20, glyph_origin, expected_write_events
            )
            character = text_before[text_cursor]
            text_cursor = (text_cursor + direction) & 0xFFFF
            if character == 0:
                break

            characters_processed += 1
            glyph_index = game_before[(CHARACTER_MAP_OFFSET + character) & 0xFFFF]
            if glyph_index & 0x80 == 0:
                glyphs_drawn += 1
                pattern = glyph_pattern(glyph_index, case_index)
                mask = starting_mask
                for plane_number in range(GLYPH_WIDTH):
                    plane_origin = (
                        glyph_origin
                        + (1 if starting_plane + plane_number >= GLYPH_WIDTH else 0)
                    ) & 0xFFFF
                    stack_write(
                        stack_expected,
                        CALLER_SP - 22,
                        plane_origin,
                        expected_write_events,
                    )
                    row_origin = plane_origin
                    for row_bits in pattern:
                        if row_bits & (0x80 >> plane_number):
                            output_expected[row_origin] = color
                            expected_write_events.append(
                                (physical_address(OUTPUT_SEGMENT, row_origin), 1, color)
                            )
                        row_origin = (row_origin + PLANAR_ROW_BYTE_COUNT) & 0xFFFF
                    mask = ((mask << 1) | (mask >> 7)) & 0xFF
                    expected_ports.append((VGA_SEQUENCER_DATA_PORT, 1, mask))
            glyph_origin = (glyph_origin + 1) & 0xFFFF

        assert characters_processed == expected["characters_processed"], name
        assert glyphs_drawn == expected["glyphs_drawn"], name
        port_bytes = b"".join(
            struct.pack("<HBB", port, size, value)
            for port, size, value in expected_ports
        )
        assert len(expected_ports) == expected["port_write_count"], name
        assert [list(item) for item in expected_ports[:10]] == expected[
            "port_writes_head"
        ], name
        assert sha256(port_bytes) == expected["port_writes_sha256"], name
        assert sha256(bytes(output_expected)) == expected["output_segment_sha256"], name
        expected_flags = add16_flags(CALLER_SP - 20, 4)
        assert expected_flags == expected["defined_flags"], name

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(TEXT_SEGMENT * 16, bytes(text_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(OUTPUT_SEGMENT * 16, output_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(
            UC_X86_REG_EFLAGS,
            0x0292 | (0x0400 if direction_flag else 0),
        )

        ports: list[tuple[int, int, int]] = []
        writes: list[tuple[int, int, int]] = []
        reached_return: list[int] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            reached: list[int] = reached_return,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def output_port(
            _cpu: Uc,
            port: int,
            size: int,
            value: int,
            _data: object,
            observed: list[tuple[int, int, int]] = ports,
        ) -> None:
            observed.append((port, size, value))

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            observed: list[tuple[int, int, int]] = writes,
        ) -> None:
            observed.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        try:
            machine.emu_start(ENTRY, 0, count=1_000_000)
        except UcError as error:
            raise RuntimeError(
                f"{name}: failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{machine.reg_read(UC_X86_REG_IP):#x}"
            ) from error

        assert reached_return == [RETURN_ADDRESS], name
        assert ports == expected_ports, name
        assert writes == expected_write_events, (name, writes, expected_write_events)
        assert snapshot_registers(machine) == initial, name
        assert snapshot_defined_flags(machine) == expected_flags, name
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        assert bool(flags & 0x0200), name
        assert bool(flags & 0x0400) == direction_flag, name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT, name
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, name
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4, name
        assert bytes(machine.mem_read(0, len(executable))) == module_before, name
        assert bytes(machine.mem_read(TEXT_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            text_before
        ), name
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        ), name
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        ), name
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        ), name
        assert bytes(machine.mem_read(OUTPUT_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            output_expected
        ), name
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        ), name
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
    digest = sha256(executable)
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = sha256(executable[ENTRY:END])
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB small-font body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_small_font_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "display_pointer_offset": DISPLAY_POINTER_OFFSET,
            "character_map_offset": CHARACTER_MAP_OFFSET,
            "glyph_table_offset": GLYPH_TABLE_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB small-font cases")


if __name__ == "__main__":
    main()
