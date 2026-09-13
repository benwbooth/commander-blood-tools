#!/usr/bin/env python3
"""Execute Big Bug Bang's location-panel entity geometry routine.

The original 0xA9D3 body executes unmodified. Its two established entity
callbacks are captured while their observable mutation boundary is modeled.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
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

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
ROUTINE = (0xA9D3, 0xAA3D)
ROUTINE_SHA256 = "b2719dffd9b3cf10f95c1fedb427d2b60079c698d07af1e39e087f234380b884"

GLOBALS = 0x30000
FRAME = 0x60000
INCOMING_ES = 0x80000
STACK = 0xA0000
SEGMENT_SIZE = 0x10000
RETURN_IP = 0x1800
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

EXTENT_HELPER = 0x3F4A  # 02B1:143A
POSITION_HELPER = 0x3E8A  # 02B1:137A

OFFSETS = {
    "source_width": 0x2A0C,
    "target": 0x2A0E,
    "artwork_present": 0x2A23,
    "zoom": 0x2A24,
    "current": 0x2D4B,
    "entity": 0x65E2,
}

CASES = (
    {
        "name": "nominal_positive",
        "zoom": 10,
        "source_extent": (40, 30),
        "source_width": 40,
        "target": (200, 120),
        "current": (100, 80),
    },
    {
        "name": "source_high_bytes_ignored",
        "zoom": 4,
        "source_extent": (0x12F0, 0xAB80),
        "source_width": 75,
        "target": (800, 500),
        "current": (400, 300),
    },
    {
        "name": "scale_product_low_byte_wrap",
        "zoom": 86,
        "source_extent": (0xFFFF, 0xFEFE),
        "source_width": 100,
        "target": (1500, 1200),
        "current": (1000, 800),
    },
    {
        "name": "signed_scale_128",
        "zoom": 170,
        "source_extent": (16, 8),
        "source_width": 0,
        "target": (1013, 977),
        "current": (1000, 1000),
    },
    {
        "name": "negative_division_truncates_to_zero",
        "zoom": 10,
        "source_extent": (17, 19),
        "source_width": 0,
        "target": (975, 765),
        "current": (1000, 800),
    },
    {
        "name": "sixteen_bit_delta_wrap",
        "zoom": 8,
        "source_extent": (255, 254),
        "source_width": 1,
        "target": (0, 0xFFF6),
        "current": (0xFFFF, 0),
    },
    {
        "name": "helper_mutation_visible_to_position",
        "zoom": 12,
        "source_extent": (31, 47),
        "source_width": 7,
        "target": (400, 500),
        "current": (200, 300),
        "mutated_source_width": 20,
        "mutated_target": (900, 700),
        "mutated_current": (600, 650),
    },
    {
        "name": "frame_offset_wrap_and_reverse_df",
        "zoom": 1,
        "source_extent": (0x55AA, 0xCC33),
        "source_width": 0xFFF0,
        "target": (0x0010, 0x0008),
        "current": (0x0000, 0xFFFE),
        "frame_offset": 0xFFFE,
        "direction": "reverse",
    },
    {
        "name": "comparison_context_offset",
        "zoom": 0xFF,
        "source_extent": (0x0101, 0x0202),
        "source_width": 12,
        "target": (1300, 900),
        "current": (1100, 850),
        "context_offset": 0x3210,
        "comparison_pointer": (0xBEEF, 0xC000),
    },
    {
        "name": "target_y_bias_wrap",
        "zoom": 6,
        "source_extent": (63, 127),
        "source_width": 0,
        "target": (13, 0xFFFA),
        "current": (0, 0),
    },
    {
        "name": "missing_artwork_skips_geometry",
        "artwork_present": 0,
        "zoom": 10,
        "source_extent": (40, 30),
        "source_width": 40,
        "target": (200, 120),
        "current": (100, 80),
    },
    {
        "name": "non_presence_bits_do_not_enable_geometry",
        "artwork_present": 0xFE,
        "zoom": 255,
        "source_extent": (0xFFFF, 0xFFFF),
        "source_width": 0xFFFF,
        "target": (0xFFFF, 0xFFFF),
        "current": (0xFFFF, 0xFFFF),
        "direction": "reverse",
    },
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write_word_wrap(data: bytearray, offset: int, value: int) -> None:
    data[offset & 0xFFFF] = value & 0xFF
    data[(offset + 1) & 0xFFFF] = (value >> 8) & 0xFF


def read_word_wrap(data: bytes | bytearray, offset: int) -> int:
    return data[offset & 0xFFFF] | (data[(offset + 1) & 0xFFFF] << 8)


def signed8(value: int) -> int:
    value &= 0xFF
    return value - 0x100 if value & 0x80 else value


def signed16(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


def trunc_div(value: int, divisor: int) -> int:
    quotient = abs(value) // abs(divisor)
    return -quotient if (value < 0) != (divisor < 0) else quotient


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def stack_word(cpu: Uc, index: int = 0) -> int:
    offset = (cpu.reg_read(UC_X86_REG_SP) + index * 2) & 0xFFFF
    return struct.unpack("<H", cpu.mem_read(STACK + offset, 2))[0]


def far_return(cpu: Uc) -> None:
    return_ip = stack_word(cpu)
    return_cs = stack_word(cpu, 1)
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def execute(executable: bytes, case: dict[str, object], case_index: int) -> dict[str, object]:
    artwork_present = int(case.get("artwork_present", 1)) & 0xFF
    enabled = bool(artwork_present & 1)
    zoom = int(case["zoom"]) & 0xFF
    source_extent = tuple(int(value) & 0xFFFF for value in case["source_extent"])
    source_width = int(case["source_width"]) & 0xFFFF
    target = tuple(int(value) & 0xFFFF for value in case["target"])
    current = tuple(int(value) & 0xFFFF for value in case["current"])
    frame_offset = int(case.get("frame_offset", 0x5100 + case_index * 0x31))
    context_offset = int(case.get("context_offset", 0x3800 + case_index * 0x23))
    comparison_offset, comparison_segment = case.get(
        "comparison_pointer", (0x6200 + case_index * 0x17, 0xC000)
    )
    reverse = case.get("direction") == "reverse"

    globals_before = seeded_segment(case_index, 17, 0x43)
    frame_before = seeded_segment(case_index, 13, 0x95)
    incoming_es_before = seeded_segment(case_index, 5, 0xA7)
    stack_before = seeded_segment(case_index, 7, 0x63)
    write_word_wrap(globals_before, OFFSETS["entity"] + 4, frame_offset)
    write_word_wrap(globals_before, OFFSETS["entity"] + 6, FRAME // 16)
    globals_before[OFFSETS["artwork_present"]] = artwork_present
    globals_before[OFFSETS["zoom"]] = zoom
    write_word_wrap(globals_before, OFFSETS["source_width"], source_width)
    write_word_wrap(globals_before, OFFSETS["target"], target[0])
    write_word_wrap(globals_before, OFFSETS["target"] + 2, target[1])
    write_word_wrap(globals_before, OFFSETS["current"], current[0])
    write_word_wrap(globals_before, OFFSETS["current"] + 2, current[1])
    write_word_wrap(frame_before, frame_offset, source_extent[0])
    write_word_wrap(frame_before, frame_offset + 2, source_extent[1])
    write_word_wrap(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = STACK_SENTINEL
    write_word_wrap(stack_before, context_offset + 4, int(comparison_offset))
    write_word_wrap(stack_before, context_offset + 6, int(comparison_segment))

    globals_expected = bytearray(globals_before)
    scale = ((((zoom * 3) & 0xFF) >> 1) + 1) & 0xFF
    scaled_extent = (
        ((source_extent[0] & 0xFF) * scale) >> 4,
        ((source_extent[1] & 0xFF) * scale) >> 4,
    )
    positioned_source_width = int(case.get("mutated_source_width", source_width)) & 0xFFFF
    positioned_target = tuple(int(value) & 0xFFFF for value in case.get("mutated_target", target))
    positioned_current = tuple(int(value) & 0xFFFF for value in case.get("mutated_current", current))
    quotient_x = trunc_div(
        signed16(positioned_target[0] - positioned_source_width - positioned_current[0]),
        13,
    )
    quotient_y = trunc_div(signed16(positioned_target[1] + 10 - positioned_current[1]), 13)
    if enabled and not (-128 <= quotient_x <= 127 and -128 <= quotient_y <= 127):
        raise AssertionError(f"{case['name']}: vector would raise #DE")
    position = (
        (positioned_current[0] + signed8(quotient_x) * signed8(scale)) & 0xFFFF,
        (positioned_current[1] + signed8(quotient_y) * signed8(scale)) & 0xFFFF,
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B20000 | ((0x40 + case_index * 13) & 0xFF),
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x97970000 | context_offset,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: INCOMING_ES // 16,
        UC_X86_REG_FS: 0xD000,
        UC_X86_REG_GS: 0xE000,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0602 if reverse else 0x0202,
    }

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(FRAME, bytes(frame_before))
    cpu.mem_write(INCOMING_ES, bytes(incoming_es_before))
    cpu.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            machine.emu_stop()
            return
        if address == EXTENT_HELPER:
            calls.append(
                {
                    "name": "extent",
                    "extent": [machine.reg_read(UC_X86_REG_CX), machine.reg_read(UC_X86_REG_DX)],
                    "comparison": [
                        read_word_wrap(stack_before, context_offset + 4),
                        read_word_wrap(stack_before, context_offset + 6),
                    ],
                    "frame": [machine.reg_read(UC_X86_REG_DI), machine.reg_read(UC_X86_REG_ES)],
                }
            )
            if "mutated_target" in case:
                write_word_wrap(globals_expected, OFFSETS["source_width"], positioned_source_width)
                write_word_wrap(globals_expected, OFFSETS["target"], positioned_target[0])
                write_word_wrap(globals_expected, OFFSETS["target"] + 2, positioned_target[1])
                write_word_wrap(globals_expected, OFFSETS["current"], positioned_current[0])
                write_word_wrap(globals_expected, OFFSETS["current"] + 2, positioned_current[1])
                machine.mem_write(
                    GLOBALS + OFFSETS["source_width"],
                    bytes(globals_expected[OFFSETS["source_width"] : OFFSETS["target"] + 4]),
                )
                machine.mem_write(
                    GLOBALS + OFFSETS["current"],
                    bytes(globals_expected[OFFSETS["current"] : OFFSETS["current"] + 4]),
                )
            far_return(machine)
            return
        if address == POSITION_HELPER:
            calls.append(
                {
                    "name": "position",
                    "position": [machine.reg_read(UC_X86_REG_BX), machine.reg_read(UC_X86_REG_CX)],
                }
            )
            far_return(machine)
            return
        assert image_address(ROUTINE[0]) <= address < address + size <= image_address(ROUTINE[1]), (
            case["name"],
            hex(address + HEADER_SIZE),
        )

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=300)
    assert reached_return, case["name"]

    expected_calls: list[dict[str, object]] = []
    if enabled:
        expected_calls = [
            {
                "name": "extent",
                "extent": list(scaled_extent),
                "comparison": [int(comparison_offset) & 0xFFFF, int(comparison_segment) & 0xFFFF],
                "frame": [frame_offset & 0xFFFF, FRAME // 16],
            },
            {"name": "position", "position": list(position)},
        ]
    assert calls == expected_calls, (case["name"], calls, expected_calls)

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    stack_after = bytes(cpu.mem_read(STACK, SEGMENT_SIZE))
    assert globals_after == bytes(globals_expected), case["name"]
    assert bytes(cpu.mem_read(FRAME, SEGMENT_SIZE)) == bytes(frame_before), case["name"]
    assert bytes(cpu.mem_read(INCOMING_ES, SEGMENT_SIZE)) == bytes(incoming_es_before), case["name"]
    assert bytes(cpu.mem_read(0, len(module))) == module, case["name"]
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2, case["name"]
    assert stack_after[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] == STACK_SENTINEL

    expected_registers = dict(initial)
    if enabled:
        expected_registers.update(
            {
                UC_X86_REG_EAX: initial[UC_X86_REG_EAX] & 0xFFFF0000,
                UC_X86_REG_EBX: (initial[UC_X86_REG_EBX] & 0xFFFF0000) | position[0],
                UC_X86_REG_ECX: (initial[UC_X86_REG_ECX] & 0xFFFF0000) | position[1],
                UC_X86_REG_EDX: (initial[UC_X86_REG_EDX] & 0xFFFF0000) | (scale << 8) | 13,
                UC_X86_REG_ESI: (initial[UC_X86_REG_ESI] & 0xFFFF0000) | OFFSETS["entity"],
                UC_X86_REG_EDI: (initial[UC_X86_REG_EDI] & 0xFFFF0000) | (frame_offset & 0xFFFF),
            }
        )
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
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
            case["name"],
            register,
            hex(cpu.reg_read(register)),
            hex(expected_registers[register]),
        )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        name: bool(flags & mask)
        for name, mask in (("cf", 1), ("pf", 4), ("zf", 0x40), ("sf", 0x80), ("df", 0x400), ("of", 0x800))
    }
    assert defined_flags == {
        "cf": False,
        "pf": True,
        "zf": True,
        "sf": False,
        "df": reverse,
        "of": False,
    }, (case["name"], defined_flags)

    return {
        "name": case["name"],
        "artwork_present": artwork_present,
        "executed": enabled,
        "zoom": zoom,
        "scale": scale if enabled else None,
        "source_extent": list(source_extent),
        "scaled_extent": list(scaled_extent) if enabled else None,
        "source_width": source_width,
        "target_before": list(target),
        "current_before": list(current),
        "target_for_position": list(positioned_target),
        "current_for_position": list(positioned_current),
        "draw_position": list(position) if enabled else None,
        "comparison_pointer": [int(comparison_offset) & 0xFFFF, int(comparison_segment) & 0xFFFF],
        "calls": calls,
        "defined_flags": defined_flags,
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "return": "near",
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = {row["name"]: row for row in json.loads(commander_path.read_text())}
    for row in rows:
        if not row["executed"]:
            continue
        original = commander[row["name"]]
        assert row["scale"] == original["scale"]
        assert row["scaled_extent"] == original["scaled_extent"]
        assert row["draw_position"] == original["draw_position"]
        assert [call["name"] for call in row["calls"]] == ["extent", "position"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9240_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    actual = hashlib.sha256(executable[slice(*ROUTINE)]).hexdigest()
    if actual != ROUTINE_SHA256:
        raise SystemExit(f"native span {ROUTINE[0]:#x}..{ROUTINE[1]:#x} changed: {actual}")
    assert executable[ROUTINE[1] - 1] == 0xC3

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows)
    )
    print(f"verified {len(rows)} original BBB location-panel geometry cases")


if __name__ == "__main__":
    main()
