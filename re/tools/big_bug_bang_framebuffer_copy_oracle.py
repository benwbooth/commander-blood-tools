#!/usr/bin/env python3
"""Execute Big Bug Bang's work-surface-to-back-buffer span copy."""

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
ROUTINE = (0xAAD4, 0xAAFE)
ROUTINE_SHA256 = "87d09f1d6ebce2104e96d4134a42cc9c984aa36d235cb01d641fb8b911b1f828"

GAME = 0x20000
DATA = 0x30000
EXTRA = 0x40000
SOURCE = 0x50000
DESTINATION = 0x70000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
RETURN_IP = 0x1800
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

SOURCE_POINTER = 0x0CB4
DESTINATION_POINTER = 0x55F9

CASES = (
    ("zero_width", 0, 0, 0, 0, 0),
    ("single_origin", 0, 0, 1, 0, 0),
    ("partial_row", 37, 12, 17, 0, 0),
    ("full_row", 0, 110, 320, 0, 0),
    ("last_screen_row_tail", 311, 199, 9, 0, 0),
    ("offset_wrap", 251, 204, 12, 0, 0),
    ("max_byte_row", 5, 255, 7, 0, 0),
    ("high_row_machine_formula", 3, 0x0100, 5, 0, 0),
    ("offset_add_carry", 0x0200, 204, 3, 0, 0),
    ("offset_add_auxiliary_carry", 1, 0x0F00, 2, 0, 0),
    ("offset_add_signed_overflow", 0x0300, 100, 1, 0, 0),
    ("nonzero_pointer_offsets_ignored", 7, 4, 6, 0x1234, 0x4321),
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write_word(memory: bytearray, offset: int, value: int) -> None:
    memory[offset : offset + 2] = (value & 0xFFFF).to_bytes(2, "little")


def write_far_pointer(
    memory: bytearray, offset: int, pointer_offset: int, segment: int
) -> None:
    write_word(memory, offset, pointer_offset)
    write_word(memory, offset + 2, segment)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


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


def execute(
    executable: bytes,
    case: tuple[str, int, int, int, int, int],
    case_index: int,
) -> dict[str, object]:
    name, x, y, width, source_pointer_offset, destination_pointer_offset = case
    source = bytes(
        (index * 37 + case_index * 29 + 11) & 0xFF for index in range(SEGMENT_SIZE)
    )
    destination = bytes(
        (index * 13 + case_index * 17 + 7) & 0xFF for index in range(SEGMENT_SIZE)
    )
    expected_destination = bytearray(destination)
    swapped_y = ((y & 0xFF) << 8) | (y >> 8)
    row_offset = (swapped_y + ((y << 6) & 0xFFFF)) & 0xFFFF
    machine_offset = (row_offset + x) & 0xFFFF
    natural_offset = (y * 320 + x) & 0xFFFF
    for index in range(width):
        copy_offset = (machine_offset + index) & 0xFFFF
        expected_destination[copy_offset] = source[copy_offset]

    game_before = seeded_segment(case_index, 17, 0x31)
    data_before = seeded_segment(case_index, 19, 0x43)
    extra_before = seeded_segment(case_index, 23, 0x57)
    stack_before = seeded_segment(case_index, 7, 0x69)
    write_far_pointer(
        game_before,
        SOURCE_POINTER,
        source_pointer_offset,
        SOURCE // 16,
    )
    write_far_pointer(
        game_before,
        DESTINATION_POINTER,
        destination_pointer_offset,
        DESTINATION // 16,
    )
    write_far_pointer(data_before, SOURCE_POINTER, 0x1111, EXTRA // 16)
    write_far_pointer(data_before, DESTINATION_POINTER, 0x2222, EXTRA // 16)
    write_far_pointer(extra_before, SOURCE_POINTER, 0x3333, DATA // 16)
    write_far_pointer(extra_before, DESTINATION_POINTER, 0x4444, DATA // 16)
    write_word(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B20000 | x,
        UC_X86_REG_ECX: 0xC3C30000 | y,
        UC_X86_REG_EDX: 0xD4D40000 | width,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    stack_expected = bytearray(stack_before)
    for offset, value in (
        (STACK_POINTER - 2, initial[UC_X86_REG_ES]),
        (STACK_POINTER - 4, initial[UC_X86_REG_EDI]),
        (STACK_POINTER - 6, initial[UC_X86_REG_DS]),
        (STACK_POINTER - 8, initial[UC_X86_REG_ESI]),
        (STACK_POINTER - 10, initial[UC_X86_REG_ECX]),
        (STACK_POINTER - 12, initial[UC_X86_REG_EAX]),
    ):
        write_word(stack_expected, offset, value)

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(GAME, bytes(game_before))
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(EXTRA, bytes(extra_before))
    cpu.mem_write(SOURCE, source)
    cpu.mem_write(DESTINATION, destination)
    cpu.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        cpu.reg_write(register, value)

    phase_offsets = (6, 11, 16, 33, 35)
    phases: list[dict[str, int]] = []
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
        file_offset = address + HEADER_SIZE
        if file_offset not in tuple(ROUTINE[0] + offset for offset in phase_offsets):
            return
        if file_offset == ROUTINE[0] + 33 and any(
            phase["file_offset"] == file_offset for phase in phases
        ):
            return
        phases.append(
            {
                "file_offset": file_offset,
                "ds": machine.reg_read(UC_X86_REG_DS),
                "es": machine.reg_read(UC_X86_REG_ES),
                "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                "cx": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
            }
        )

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=max(20_000, width + 64))
    assert reached_return, name

    expected_phases = [
        {
            "file_offset": ROUTINE[0] + 6,
            "ds": DATA // 16,
            "es": EXTRA // 16,
            "si": initial[UC_X86_REG_ESI] & 0xFFFF,
            "di": initial[UC_X86_REG_EDI] & 0xFFFF,
            "cx": y,
        },
        {
            "file_offset": ROUTINE[0] + 11,
            "ds": DATA // 16,
            "es": DESTINATION // 16,
            "si": initial[UC_X86_REG_ESI] & 0xFFFF,
            "di": destination_pointer_offset,
            "cx": y,
        },
        {
            "file_offset": ROUTINE[0] + 16,
            "ds": SOURCE // 16,
            "es": DESTINATION // 16,
            "si": source_pointer_offset,
            "di": destination_pointer_offset,
            "cx": y,
        },
        {
            "file_offset": ROUTINE[0] + 33,
            "ds": SOURCE // 16,
            "es": DESTINATION // 16,
            "si": machine_offset,
            "di": machine_offset,
            "cx": width,
        },
        {
            "file_offset": ROUTINE[0] + 35,
            "ds": SOURCE // 16,
            "es": DESTINATION // 16,
            "si": (machine_offset + width) & 0xFFFF,
            "di": (machine_offset + width) & 0xFFFF,
            "cx": 0,
        },
    ]
    assert phases == expected_phases, (name, phases, expected_phases)

    destination_after = bytes(cpu.mem_read(DESTINATION, SEGMENT_SIZE))
    assert destination_after == bytes(expected_destination), name
    assert bytes(cpu.mem_read(SOURCE, SEGMENT_SIZE)) == source, name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before), name
    assert bytes(cpu.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before), name
    assert bytes(cpu.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before), name
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), name
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

    expected_flags = add16_flags(row_offset, x)
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(flags & mask)
        for flag, mask in (
            ("cf", 1),
            ("pf", 4),
            ("af", 0x10),
            ("zf", 0x40),
            ("sf", 0x80),
            ("of", 0x800),
        )
    }
    assert defined_flags == expected_flags, (name, defined_flags, expected_flags)

    copied = bytes(source[(machine_offset + index) & 0xFFFF] for index in range(width))
    return {
        "name": name,
        "x": x,
        "y": y,
        "width": width,
        "source_pointer_offset": source_pointer_offset,
        "destination_pointer_offset": destination_pointer_offset,
        "machine_offset": machine_offset,
        "natural_y_times_320_offset": natural_offset,
        "natural_offset_matches": machine_offset == natural_offset,
        "copied_sha256": hashlib.sha256(copied).hexdigest(),
        "defined_flags": defined_flags,
        "destination_sha256": hashlib.sha256(destination_after).hexdigest(),
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
        default=Path(__file__).parent / "oracle_vectors/func_933a_natural.json",
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
    print(f"verified {len(rows)} original BBB framebuffer-copy cases")


if __name__ == "__main__":
    main()
