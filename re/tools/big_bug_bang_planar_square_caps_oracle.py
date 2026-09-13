#!/usr/bin/env python3
"""Execute BBB's complete planar square-cap text renderer."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct
from typing import Any

from unicorn import (
    Uc,
    UcError,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_MODE_16,
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
    UC_X86_REG_EIP,
    UC_X86_REG_ES,
    UC_X86_REG_ESI,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
COMMANDER_FIXTURE = REPO_ROOT / "re/tools/oracle_vectors/func_3428_natural.json"
COMMANDER_FIXTURE_SHA256 = (
    "897ef40570ea178886fa694ca9e71dc3d06449488ed71db41506b803a836158d"
)
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
ENTRY = 0x37A8
END = 0x38FC
ROUTINE_SHA256 = "5e3c124ef4a89cefd19c0bd11593a58bfeebb5501740a2d816e1cfb6fea40b97"
TEXT_SEGMENT = 0x2000
GAME_SEGMENT = 0x4000
PRIMARY_SEGMENT = 0x7000
SECONDARY_SEGMENT = 0x8000
STACK_SEGMENT = 0xA000
RETURN_ADDRESS = 0x6FC0
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("e12dd23cc34bb45a")
WIDTH_OFFSET = 0x2A6D
CLIP_TOP_OFFSET = 0x5609
CLIP_BOTTOM_OFFSET = 0x560B
PRIMARY_POINTER_OFFSET = 0x55E9
SECONDARY_POINTER_OFFSET = 0x55ED
INVENTORY_LINE_OFFSET = 0x6B94
CHARACTER_MAP_OFFSET = 0x7CF8
ADVANCES_OFFSET = 0x7DE0
GLYPHS_OFFSET = 0x7E12
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}
REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sub16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": ((left ^ right ^ result) & 0x10) != 0,
        "zf": result == 0,
        "sf": (result & 0x8000) != 0,
        "of": (((left ^ right) & (left ^ result)) & 0x8000) != 0,
    }


def case_inputs(name: str) -> tuple[bytes, dict[int, int], dict[int, int]]:
    if name in {
        "above_bottom_clipped_before_port_or_text",
        "top_minus_height_edge_clipped",
        "single_glyph_starting_plane_zero",
        "single_glyph_starting_plane_one",
        "single_glyph_starting_plane_two",
        "single_glyph_starting_plane_three",
    }:
        text = b"A\0"
    elif name == "empty_string_selects_map_mask_register_only":
        text = b"\0"
    elif name in {
        "two_glyphs_signed_advances_cross_plane_boundary",
        "source_offset_wrap",
        "inherited_backward_source_direction",
    }:
        text = b"AB\0"
    elif name == "full_word_row_formula_and_output_wrap":
        text = b"C\0"
    elif name == "width_accumulator_wrap_with_positive_advance":
        text = b"Z" * 517 + b"\0"
    else:
        raise ValueError(f"unknown planar square-caps case {name}")

    entries = {
        "single_glyph_starting_plane_zero": ((0x41, 2, 7),),
        "single_glyph_starting_plane_one": ((0x41, 3, 11),),
        "single_glyph_starting_plane_two": ((0x41, 4, 9),),
        "single_glyph_starting_plane_three": ((0x41, 5, 14),),
        "two_glyphs_signed_advances_cross_plane_boundary": (
            (0x41, 6, 7),
            (0x42, 7, 0xFB),
        ),
        "source_offset_wrap": ((0x41, 8, 5), (0x42, 9, 13)),
        "inherited_backward_source_direction": (
            (0x41, 10, 6),
            (0x42, 11, 12),
        ),
        "full_word_row_formula_and_output_wrap": ((0x43, 12, 15),),
        "width_accumulator_wrap_with_positive_advance": ((0x5A, 13, 0x7F),),
    }.get(name, ())
    mapping = {character: glyph for character, glyph, _advance in entries}
    advances = {glyph: advance for _character, glyph, advance in entries}
    return text, mapping, advances


def patterned_memory(
    case_index: int, multiplier: int, case_multiplier: int, base: int
) -> bytearray:
    return bytearray(
        (index * multiplier + case_index * case_multiplier + base) & 0xFF
        for index in range(0x10000)
    )


def execute_case(
    executable: bytes, vector: dict[str, Any], case_index: int
) -> dict[str, Any]:
    name = str(vector["name"])
    text, mapping, advances = case_inputs(name)
    text_offset = int(vector.get("text_offset", 0x3200))
    screen_offset = int(vector["screen_offset"])
    x = int(vector["x"])
    y = int(vector["y"])
    color = int(vector["color"])
    top = int(vector["clip_top"])
    bottom = int(vector["clip_bottom"])
    direction_flag = bool(vector["direction_flag"])
    direction = -1 if direction_flag else 1
    clipped = bool(vector["clipped"])
    use_primary = case_index % 2 == 0

    text_memory = patterned_memory(case_index, 13, 19, 5)
    for index, value in enumerate(text):
        text_memory[(text_offset + direction * index) & 0xFFFF] = value

    game_before = patterned_memory(case_index, 17, 23, 0x39)
    for character, glyph_index in mapping.items():
        game_before[(CHARACTER_MAP_OFFSET + character) & 0xFFFF] = glyph_index
    for glyph_index, advance in advances.items():
        game_before[(ADVANCES_OFFSET + glyph_index) & 0xFFFF] = advance
    game_before[WIDTH_OFFSET : WIDTH_OFFSET + 2] = struct.pack(
        "<H", 0xA500 + case_index
    )
    game_before[PRIMARY_POINTER_OFFSET : PRIMARY_POINTER_OFFSET + 4] = struct.pack(
        "<HH", screen_offset, PRIMARY_SEGMENT
    )
    game_before[SECONDARY_POINTER_OFFSET : SECONDARY_POINTER_OFFSET + 4] = struct.pack(
        "<HH", screen_offset, SECONDARY_SEGMENT
    )
    game_before[CLIP_TOP_OFFSET : CLIP_TOP_OFFSET + 2] = struct.pack("<H", top)
    game_before[CLIP_BOTTOM_OFFSET : CLIP_BOTTOM_OFFSET + 2] = struct.pack("<H", bottom)
    game_before[INVENTORY_LINE_OFFSET : INVENTORY_LINE_OFFSET + 2] = struct.pack(
        "<H", 0x1234 if use_primary else 0
    )

    stack_memory = patterned_memory(case_index, 29, 31, 0x6D)
    for glyph_index in set(mapping.values()):
        patterns = (
            0x0000,
            0x8000,
            0x4001,
            0x0001,
            0xFFFF,
            0x00F0,
            0x8101,
            0x7FFE,
            0xA5A5,
            (glyph_index * 0x1111 + case_index * 0x0101) & 0xFFFF,
        )
        for row, bits in enumerate(patterns):
            glyph_offset = GLYPHS_OFFSET + glyph_index * 20 + row * 2
            stack_memory[glyph_offset : glyph_offset + 2] = bits.to_bytes(2, "big")
    stack_memory[STACK_POINTER : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        struct.pack("<HH", RETURN_ADDRESS, 0) + STACK_SENTINEL
    )

    output_seed = bytes(
        (index * 37 + case_index * 41 + 0x2B) & 0xFF for index in range(0x10000)
    )
    output_model = bytearray(output_seed)
    expected_ports: list[tuple[int, int, int]] = []
    top_minus_height = (top - 10) & 0xFFFF
    signed_y = y - 0x10000 if y >= 0x8000 else y
    signed_top = (
        top_minus_height - 0x10000 if top_minus_height >= 0x8000 else top_minus_height
    )
    clipped_bottom = y > bottom
    calculated_clipped = clipped_bottom or signed_y <= signed_top
    if calculated_clipped != clipped:
        raise AssertionError(f"{name}: fixture clipping differs")
    width = 0
    characters_drawn = 0
    text_cursor = text_offset

    if clipped_bottom:
        expected_flags = sub16_flags(y, bottom)
    elif clipped:
        expected_flags = sub16_flags(y, top_minus_height)
    else:
        expected_flags = {
            "cf": False,
            "pf": True,
            "zf": True,
            "sf": False,
            "of": False,
        }
        expected_ports.append((0x3C4, 1, 2))
        row_offset = ((y << 4) + (y << 6)) & 0xFFFF
        glyph_origin = (screen_offset + row_offset + (x >> 2)) & 0xFFFF
        plane = x & 3
        while True:
            character = text_memory[text_cursor]
            text_cursor = (text_cursor + direction) & 0xFFFF
            if character == 0:
                break
            glyph_index = game_before[(CHARACTER_MAP_OFFSET + character) & 0xFFFF]
            advance = game_before[(ADVANCES_OFFSET + glyph_index) & 0xFFFF]
            advance_delta = advance - 0x100 if advance >= 0x80 else advance
            glyph_start = (GLYPHS_OFFSET + glyph_index * 20) & 0xFFFF

            for plane_number in range(4):
                map_mask = 0x11 << ((plane + plane_number) & 3)
                expected_ports.append((0x3C5, 1, map_mask))
                row_origin = (
                    glyph_origin + (1 if plane + plane_number >= 4 else 0)
                ) & 0xFFFF
                glyph_offset = glyph_start
                for _row in range(10):
                    row_bits = (
                        stack_memory[glyph_offset] << 8
                        | stack_memory[(glyph_offset + 1) & 0xFFFF]
                    )
                    row_bits = (row_bits << plane_number) & 0xFFFF
                    pixel = row_origin
                    while row_bits != 0:
                        if row_bits & 0x8000:
                            output_model[pixel] = color
                        row_bits = (row_bits << 4) & 0xFFFF
                        if row_bits != 0:
                            pixel = (pixel + 1) & 0xFFFF
                    glyph_offset = (glyph_offset + 2) & 0xFFFF
                    row_origin = (row_origin + 80) & 0xFFFF

            width = (width + advance_delta) & 0xFFFF
            advance_word = advance_delta & 0xFFFF
            advance_word = (advance_word & 0xFF00) | (
                ((advance_word & 0xFF) + plane) & 0xFF
            )
            plane = advance_word & 3
            glyph_origin = (glyph_origin + (advance_word >> 2)) & 0xFFFF
            characters_drawn += 1

    initial = {
        "eax": 0xA1A10000 | color,
        "ebx": 0xB2B20000 | x,
        "ecx": 0xC3C33450 + case_index,
        "edx": 0xD4D40000 | y,
        "esi": 0xE5E50000 | text_offset,
        "edi": 0xF6F66780 + case_index,
        "ebp": 0x97977890 + case_index,
        "sp": STACK_POINTER,
        "ds": TEXT_SEGMENT,
        "es": 0x3000,
        "fs": 0x5000,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(TEXT_SEGMENT * 16, bytes(text_memory))
    machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
    machine.mem_write(PRIMARY_SEGMENT * 16, output_seed)
    machine.mem_write(SECONDARY_SEGMENT * 16, output_seed)
    machine.mem_write(STACK_SEGMENT * 16, bytes(stack_memory))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0292 | (0x0400 if direction_flag else 0))
    for register, value in initial.items():
        machine.reg_write(REGISTERS[register], value)

    ports: list[tuple[int, int, int]] = []
    returned: list[int] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        if address == RETURN_ADDRESS:
            returned.append(address)
            cpu.emu_stop()
            return
        if not ENTRY <= address < END:
            raise AssertionError(f"{name}: escaped renderer at {address:#x}")

    def output_port(
        _cpu: Uc, port: int, size: int, value: int, _context: object
    ) -> None:
        ports.append((port, size, value))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    try:
        machine.emu_start(ENTRY, 0, count=1000000)
    except UcError as error:
        raise RuntimeError(
            f"{name}: execution failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_EIP):#x}"
        ) from error

    if returned != [RETURN_ADDRESS]:
        raise AssertionError(f"{name}: did not far-return exactly once")
    if ports != expected_ports:
        raise AssertionError(f"{name}: VGA port writes differ")
    if characters_drawn != int(vector["characters_drawn"]):
        raise AssertionError(f"{name}: character count differs")
    if width != int(vector["draw_width"]):
        raise AssertionError(f"{name}: width differs from Commander fixture")

    game_expected = game_before[:]
    game_expected[WIDTH_OFFSET : WIDTH_OFFSET + 2] = struct.pack("<H", width)
    if bytes(machine.mem_read(GAME_SEGMENT * 16, 0x10000)) != bytes(game_expected):
        raise AssertionError(f"{name}: unexpected game-state write")
    if bytes(machine.mem_read(TEXT_SEGMENT * 16, 0x10000)) != bytes(text_memory):
        raise AssertionError(f"{name}: source text changed")
    primary = bytes(machine.mem_read(PRIMARY_SEGMENT * 16, 0x10000))
    secondary = bytes(machine.mem_read(SECONDARY_SEGMENT * 16, 0x10000))
    if clipped:
        if primary != output_seed or secondary != output_seed:
            raise AssertionError(f"{name}: clipped draw changed a destination")
        selected_output = primary
        destination = "none"
    elif use_primary:
        if primary != bytes(output_model) or secondary != output_seed:
            raise AssertionError(f"{name}: primary destination selection differs")
        selected_output = primary
        destination = "primary"
    else:
        if secondary != bytes(output_model) or primary != output_seed:
            raise AssertionError(f"{name}: secondary destination selection differs")
        selected_output = secondary
        destination = "secondary"
    if bytes(machine.mem_read(0, len(executable))) != executable:
        raise AssertionError(f"{name}: executable code changed")

    expected_registers = dict(initial)
    expected_registers["eax"] = initial["eax"] if clipped else initial["eax"] & 0xFFFF
    expected_registers["sp"] = STACK_POINTER + 4
    for register, expected in expected_registers.items():
        actual = machine.reg_read(REGISTERS[register])
        if actual != expected:
            raise AssertionError(
                f"{name}: {register}={actual:#x}, expected={expected:#x}"
            )
    if machine.reg_read(UC_X86_REG_CS) != 0:
        raise AssertionError(f"{name}: far return changed CS")
    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        flag: bool(flags_after & FLAG_MASKS[flag]) for flag in expected_flags
    }
    if actual_flags != expected_flags:
        raise AssertionError(f"{name}: flags={actual_flags}, expected={expected_flags}")
    if bool(flags_after & 0x0400) != direction_flag:
        raise AssertionError(f"{name}: direction flag changed")
    if (
        bytes(
            machine.mem_read(
                STACK_SEGMENT * 16 + STACK_POINTER + 4, len(STACK_SENTINEL)
            )
        )
        != STACK_SENTINEL
    ):
        raise AssertionError(f"{name}: stack sentinel changed")

    port_bytes = b"".join(
        struct.pack("<HBB", port, size, value) for port, size, value in ports
    )
    return {
        **vector,
        "destination": destination,
        "inventory_line": 0x1234 if use_primary else 0,
        "port_write_count": len(ports),
        "port_writes_head": [list(item) for item in ports[:9]],
        "port_writes_sha256": sha256(port_bytes),
        "defined_flags": expected_flags,
        "output_segment_sha256": sha256(selected_output),
        "other_segment_sha256": sha256(output_seed),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    if sha256(executable) != EXECUTABLE_SHA256:
        raise ValueError("unrecognized BLOOD2PG.EXE")
    if sha256(executable[ENTRY:END]) != ROUTINE_SHA256:
        raise ValueError("BBB planar square-cap renderer bytes changed")

    fixture_bytes = COMMANDER_FIXTURE.read_bytes()
    if sha256(fixture_bytes) != COMMANDER_FIXTURE_SHA256:
        raise ValueError("Commander planar square-cap fixture changed")
    vectors = json.loads(fixture_bytes)
    if not isinstance(vectors, list) or len(vectors) != 12:
        raise ValueError("expected twelve Commander planar square-cap vectors")

    verified = [
        execute_case(executable, vector, case_index)
        for case_index, vector in enumerate(vectors)
    ]
    report = {
        "executable_sha256": EXECUTABLE_SHA256,
        "entry": ENTRY,
        "end": END,
        "routine_sha256": ROUTINE_SHA256,
        "commander_fixture": str(COMMANDER_FIXTURE.relative_to(REPO_ROOT)),
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "vectors": verified,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"verified {len(verified)} original BBB planar square-cap cases")


if __name__ == "__main__":
    main()
