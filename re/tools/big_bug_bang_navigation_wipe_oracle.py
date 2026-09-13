#!/usr/bin/env python3
"""Execute Big Bug Bang's navigation center-wipe span-table builder."""

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
ROUTINE = (0xAAFE, 0xAB8F)
ROUTINE_SHA256 = "be5cc5d0ffe3888a258dba7cd268d73d0ff8b28a125994175869e7d37921d5f6"

DATA = 0x34000
ENTRY_ES = 0x48000
OUTPUT = 0x60000
STACK = 0x78000
GAME = 0x90000
SEGMENT_SIZE = 0x10000
OUTPUT_POINTER = 0x55F1
RETURN_IP = 0x1800
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

CASES = (
    {"name": "shipped_phase_0", "point": (160, 0)},
    {"name": "shipped_phase_1", "point": (140, 0)},
    {"name": "shipped_phase_2", "point": (120, 0)},
    {"name": "shipped_phase_3", "point": (60, 0)},
    {"name": "shipped_phase_4", "point": (0, 0)},
    {"name": "shipped_phase_5", "point": (0, 50)},
    {"name": "shipped_phase_6", "point": (0, 90)},
    {"name": "shipped_phase_7", "point": (0, 130)},
    {"name": "shipped_phase_8", "point": (0, 190)},
    {"name": "steep_up_right", "point": (150, 50)},
    {"name": "steep_up_left", "point": (200, 50)},
    {"name": "shallow_up_right", "point": (100, 100)},
    {"name": "shallow_down_right", "point": (220, 120)},
    {"name": "shallow_down_left", "point": (100, 120)},
    {"name": "vertical_up", "point": (160, 105)},
    {"name": "horizontal_has_no_spans", "point": (100, 110)},
    {"name": "diagonal_uses_vertical_major_path", "point": (150, 100)},
    {
        "name": "output_offset_wrap",
        "point": (150, 108),
        "output_offset": 0xFFFC,
    },
    {
        "name": "inherited_reverse_direction",
        "point": (180, 100),
        "direction": "reverse",
        "input_offset": 0x3100,
        "output_offset": 0x4200,
    },
    {
        "name": "center_point_runs_wrapped_zero_counter",
        "point": (160, 110),
        "output_offset": 0x5000,
    },
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write_word_wrap(memory: bytearray, offset: int, value: int) -> None:
    memory[offset & 0xFFFF] = value & 0xFF
    memory[(offset + 1) & 0xFFFF] = (value >> 8) & 0xFF


def signed16(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


def add16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left + right) & 0xFFFF
    return {
        "cf": left + right > 0xFFFF,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": (left & 0x0F) + (right & 0x0F) > 0x0F,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool((~(left ^ right) & (left ^ result)) & 0x8000),
    }


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 11 + case_index * 23 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    name = str(case["name"])
    loaded_x, loaded_y = (int(value) & 0xFFFF for value in case["point"])
    input_offset = int(case.get("input_offset", 0x2800 + case_index * 0x80))
    output_offset = int(case.get("output_offset", 0x1800 + case_index * 0x200))
    reverse = case.get("direction") == "reverse"

    data_before = seeded_segment(case_index, 17, 0x35)
    output_before = seeded_segment(case_index, 13, 0x57)
    entry_es_before = bytes(seeded_segment(case_index, 19, 0x79))
    game_before = bytes(seeded_segment(case_index, 23, 0x9B))
    stack_before = seeded_segment(case_index, 5, 0xBD)

    write_word_wrap(data_before, OUTPUT_POINTER, output_offset)
    write_word_wrap(data_before, OUTPUT_POINTER + 2, OUTPUT // 16)
    write_word_wrap(data_before, input_offset, loaded_x)
    second_input_offset = input_offset - 2 if reverse else input_offset + 2
    write_word_wrap(data_before, second_input_offset, loaded_y)
    write_word_wrap(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    start_x = loaded_x
    start_y = loaded_y
    end_x = 160
    end_y = 110
    if signed16(start_y) >= 110:
        start_x, end_x = 160, start_x
        start_y, end_y = 110, start_y

    horizontal_delta = (end_x - start_x) & 0xFFFF
    vertical_delta = (end_y - start_y) & 0xFFFF
    x_step = 1
    if signed16(horizontal_delta) < 0:
        horizontal_delta = (-horizontal_delta) & 0xFFFF
        x_step = -1

    output_expected = bytearray(output_before)
    output_cursor = output_offset & 0xFFFF
    cursor_step = -2 if reverse else 2
    spans: list[tuple[int, int]] = []

    def emit_word(value: int) -> None:
        nonlocal output_cursor
        write_word_wrap(output_expected, output_cursor, value)
        output_cursor = (output_cursor + cursor_step) & 0xFFFF

    def emit_span(x: int) -> None:
        width = (((160 - x) & 0xFFFF) * 2) & 0xFFFF
        spans.append((x & 0xFFFF, width))
        emit_word(x)
        emit_word(width)

    if signed16(vertical_delta) >= signed16(horizontal_delta):
        major = vertical_delta
        doubled_minor = (horizontal_delta * 2) & 0xFFFF
        doubled_major = (vertical_delta * 2) & 0xFFFF
        error = (doubled_minor - vertical_delta) & 0xFFFF
        remaining = major
        while True:
            emit_span(start_x)
            if signed16(error) >= 0:
                start_x = (start_x + x_step) & 0xFFFF
                error = (error - doubled_major) & 0xFFFF
            add_left = error
            error = (error + doubled_minor) & 0xFFFF
            final_flags = add16_flags(add_left, doubled_minor)
            remaining = (remaining - 1) & 0xFFFF
            if remaining == 0:
                break
        path = "vertical_major"
    else:
        major = horizontal_delta
        doubled_minor = (vertical_delta * 2) & 0xFFFF
        doubled_major = (horizontal_delta * 2) & 0xFFFF
        error = (doubled_minor - horizontal_delta) & 0xFFFF
        remaining = major
        while True:
            error_nonnegative = signed16(error) >= 0
            start_x = (start_x + x_step) & 0xFFFF
            if error_nonnegative:
                emit_span(start_x)
                error = (error - doubled_major) & 0xFFFF
            add_left = error
            error = (error + doubled_minor) & 0xFFFF
            final_flags = add16_flags(add_left, doubled_minor)
            remaining = (remaining - 1) & 0xFFFF
            if remaining == 0:
                break
        path = "horizontal_major"

    emit_word(0xFFFF)
    emit_word(0xFFFF)
    expected_flags = dict(final_flags)
    expected_flags["df"] = reverse

    initial = {
        UC_X86_REG_EAX: 0xA1A11111 + case_index,
        UC_X86_REG_EBX: 0xB2B22468,
        UC_X86_REG_ECX: 0xC3C3369C,
        UC_X86_REG_EDX: 0xD4D455AA,
        UC_X86_REG_ESI: 0xE5E50000 | (input_offset & 0xFFFF),
        UC_X86_REG_EDI: 0xF6F6789A,
        UC_X86_REG_EBP: 0x97971357,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: ENTRY_ES // 16,
        UC_X86_REG_FS: 0xA000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0602 if reverse else 0x0202,
    }
    stack_expected = bytearray(stack_before)
    for offset, value in (
        (STACK_POINTER - 2, initial[UC_X86_REG_EAX]),
        (STACK_POINTER - 4, initial[UC_X86_REG_EBX]),
        (STACK_POINTER - 6, initial[UC_X86_REG_ECX]),
        (STACK_POINTER - 8, initial[UC_X86_REG_EDX]),
        (STACK_POINTER - 10, initial[UC_X86_REG_EBP]),
        (STACK_POINTER - 12, initial[UC_X86_REG_ES]),
        (STACK_POINTER - 14, initial[UC_X86_REG_EDI]),
        (STACK_POINTER - 16, initial[UC_X86_REG_ESI]),
    ):
        write_word_wrap(stack_expected, offset, value)

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(ENTRY_ES, entry_es_before)
    cpu.mem_write(OUTPUT, bytes(output_before))
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
    instruction_limit = 1_500_000 if major == 0 else 20_000
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=instruction_limit)
    assert reached_return, name

    output_after = bytes(cpu.mem_read(OUTPUT, SEGMENT_SIZE))
    assert bytes(cpu.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before), name
    assert bytes(cpu.mem_read(ENTRY_ES, SEGMENT_SIZE)) == entry_es_before, name
    assert output_after == bytes(output_expected), name
    stack_after = bytes(cpu.mem_read(STACK, SEGMENT_SIZE))
    transient_start = STACK_POINTER - 20
    saved_frame_start = STACK_POINTER - 16
    assert stack_after[:transient_start] == bytes(stack_before[:transient_start]), name
    assert stack_after[saved_frame_start:] == bytes(
        stack_expected[saved_frame_start:]
    ), name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == game_before, name
    assert bytes(cpu.mem_read(0, len(module))) == module, name

    expected_registers = dict(initial)
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
            name,
            register,
            hex(cpu.reg_read(register)),
            hex(expected_registers[register]),
        )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(flags & mask)
        for flag, mask in (
            ("cf", 1),
            ("pf", 4),
            ("af", 0x10),
            ("zf", 0x40),
            ("sf", 0x80),
            ("df", 0x400),
            ("of", 0x800),
        )
    }
    assert defined_flags == expected_flags, (name, defined_flags, expected_flags)

    span_bytes = b"".join(
        int(value).to_bytes(2, "little") for span in spans for value in span
    )
    return {
        "name": name,
        "loaded_point": [loaded_x, loaded_y],
        "direction": "reverse" if reverse else "forward",
        "path": path,
        "horizontal_delta": horizontal_delta,
        "vertical_delta": vertical_delta,
        "output_pointer": [output_offset & 0xFFFF, OUTPUT // 16],
        "span_count": len(spans),
        "spans_head": [list(span) for span in spans[:8]],
        "spans_tail": [list(span) for span in spans[-8:]],
        "span_bytes_sha256": hashlib.sha256(span_bytes).hexdigest(),
        "defined_flags": defined_flags,
        "output_sha256": hashlib.sha256(output_after).hexdigest(),
        "return": "near",
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert len(rows) == len(commander)
    fields = tuple(commander[0])
    for sequel, original in zip(rows, commander, strict=True):
        assert {field: sequel[field] for field in fields} == original, sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9364_natural.json",
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
    print(f"verified {len(rows)} original BBB navigation-wipe cases")


if __name__ == "__main__":
    main()
