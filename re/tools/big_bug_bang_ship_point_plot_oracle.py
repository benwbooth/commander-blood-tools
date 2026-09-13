#!/usr/bin/env python3
"""Compare Big Bug Bang's ship point plotter directly with Commander Blood."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
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

HEADER_SIZE = 0x800
SEGMENT_SIZE = 0x10000
DATA = 0x20000
GAME = 0x40000
FS_DATA = 0x60000
STACK = 0x80000
FRAMEBUFFER = 0xA0000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("69965aa5c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)


@dataclass(frozen=True)
class Routine:
    name: str
    span: tuple[int, int]
    body_sha256: str
    context: int
    clip: int
    branches: dict[int, tuple[int, int]]


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x9B04, 0x9B48),
    body_sha256="ac19f28f8de11959599f3709ac9a949cf4c83428d206d71a312b3cba58fd68a2",
    context=0x2F95,
    clip=0x5235,
    branches={
        0x9B0E: (0x9B44, 0x9B10),
        0x9B14: (0x9B44, 0x9B16),
        0x9B1D: (0x9B44, 0x9B1F),
        0x9B23: (0x9B44, 0x9B25),
        0x9B35: (0x9B44, 0x9B37),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xB2A3, 0xB2E7),
    body_sha256="488f5bd72d3ee2502c509e4c04cf5167b50866d954bc445ff79de9e5f477571a",
    context=0x3365,
    clip=0x5605,
    branches={
        0xB2AD: (0xB2E3, 0xB2AF),
        0xB2B3: (0xB2E3, 0xB2B5),
        0xB2BC: (0xB2E3, 0xB2BE),
        0xB2C2: (0xB2E3, 0xB2C4),
        0xB2D4: (0xB2E3, 0xB2D6),
    },
)

REGISTERS = (
    UC_X86_REG_EAX,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDX,
    UC_X86_REG_ESI,
    UC_X86_REG_EDI,
    UC_X86_REG_EBP,
    UC_X86_REG_SP,
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_ES,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SS,
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def machine_offset(x_word: int, y_word: int) -> int:
    swapped_y = ((y_word & 0xFF) << 8) | (y_word >> 8)
    return (((y_word << 6) & 0xFFFF) + swapped_y + x_word) & 0xFFFF


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes]:
    name = str(vector["name"])
    x = int(vector["x"])
    y = int(vector["y"])
    depth = int(vector["depth"])
    x_word = x & 0xFFFF
    y_word = y & 0xFFFF
    offset = machine_offset(x_word, y_word)
    natural_offset = (y_word * 320 + x_word) & 0xFFFF
    outcome = str(vector["outcome"])

    framebuffer_before = bytearray(
        (0x31 + index * 29) & 0xFF for index in range(SEGMENT_SIZE)
    )
    framebuffer_before[offset] = int(vector["pixel_before"])
    framebuffer_expected = bytearray(framebuffer_before)
    if outcome == "draw":
        framebuffer_expected[offset] = int(vector["pixel_after"])

    context = bytes((0xA5 + index * 7) & 0xFF for index in range(36))
    context += struct.pack("<HHH", x_word, y_word, depth)
    context_decoy = bytes((0x59 + index * 11) & 0xFF for index in range(42))
    clip_words = struct.pack("<4H", *(int(value) & 0xFFFF for value in vector["clip"]))
    clip_decoy = bytes.fromhex("5aa596698778c33c")

    data_before = seeded_segment(case_index, 17, 0x41)
    data_before[routine.context : routine.context + 42] = context_decoy
    data_before[routine.clip : routine.clip + 8] = clip_words
    game_before = seeded_segment(case_index, 23, 0x63)
    game_before[routine.context : routine.context + 42] = context_decoy[::-1]
    game_before[routine.clip : routine.clip + 8] = clip_decoy
    fs_before = bytes(seeded_segment(case_index, 11, 0x85))
    stack_before = seeded_segment(case_index, 7, 0xA7)
    stack_before[routine.context : routine.context + 42] = context
    stack_before[routine.clip : routine.clip + 8] = clip_decoy[::-1]
    stack_before[STACK_POINTER : STACK_POINTER + 2] = struct.pack("<H", RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    stack_expected = bytearray(stack_before)

    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x97970000 | routine.context,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: FRAMEBUFFER // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    struct.pack_into("<H", stack_expected, 0xFEFE, initial[UC_X86_REG_EAX] & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEFC, initial[UC_X86_REG_EBX] & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEFA, initial[UC_X86_REG_EDI] & 0xFFFF)

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    machine.mem_write(FRAMEBUFFER, bytes(framebuffer_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    phases: list[dict[str, object]] = []
    reached_return = False
    previous_file_offset: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous_file_offset
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_offset = address + HEADER_SIZE
        assert routine.span[0] <= file_offset < routine.span[1], (
            routine.name,
            name,
            hex(file_offset),
        )
        if previous_file_offset in routine.branches:
            covered_edges.add(
                (
                    previous_file_offset - routine.span[0],
                    file_offset - routine.span[0],
                )
            )
        previous_file_offset = file_offset
        instruction_offset = file_offset - routine.span[0]
        if instruction_offset == 0x2C:
            phases.append(
                {
                    "offset": instruction_offset,
                    "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        elif instruction_offset == 0x3D:
            phases.append(
                {
                    "offset": instruction_offset,
                    "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                    "al": cpu.reg_read(UC_X86_REG_EAX) & 0xFF,
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        elif instruction_offset == 0x40:
            phases.append({"offset": instruction_offset})

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(image_address(routine.span[0]), 0, count=256)
    assert reached_return

    expected_phases: list[dict[str, object]] = []
    if outcome in ("draw", "occupied"):
        expected_phases.append({"offset": 0x2C, "di": offset, "es": FRAMEBUFFER // 16})
        if outcome == "draw":
            expected_phases.append(
                {
                    "offset": 0x3D,
                    "di": offset,
                    "al": int(vector["pixel_after"]),
                    "es": FRAMEBUFFER // 16,
                }
            )
    expected_phases.append({"offset": 0x40})
    assert phases == expected_phases, (routine.name, name, phases)

    framebuffer_after = bytes(machine.mem_read(FRAMEBUFFER, SEGMENT_SIZE))
    assert framebuffer_after == bytes(framebuffer_expected), (routine.name, name)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    actual_stack = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert actual_stack == bytes(stack_expected), (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = 0xFF02
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    defined_flags = {
        flag: bool(flags & masks[flag]) for flag in vector["defined_flags"]
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, flags)

    plotted = outcome in ("draw", "occupied")
    row = {
        "name": name,
        "x": x,
        "y": y,
        "depth": depth,
        "clip": list(vector["clip"]),
        "outcome": outcome,
        "machine_offset": offset if plotted else None,
        "natural_offset": natural_offset if plotted else None,
        "natural_offset_matches": offset == natural_offset,
        "pixel_before": int(vector["pixel_before"]),
        "pixel_after": framebuffer_after[offset],
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    registers[UC_X86_REG_EBP] = (
        registers[UC_X86_REG_EBP] & 0xFFFF0000
    ) | COMMANDER.context
    normalized_stack = bytearray(actual_stack)
    stack_seed = seeded_segment(case_index, 7, 0xA7)
    normalized_stack[routine.context : routine.context + 42] = stack_seed[
        routine.context : routine.context + 42
    ]
    normalized_stack[routine.clip : routine.clip + 8] = stack_seed[
        routine.clip : routine.clip + 8
    ]
    normalized_stack[COMMANDER.context : COMMANDER.context + 42] = context
    normalized_stack[COMMANDER.clip : COMMANDER.clip + 8] = clip_decoy[::-1]
    return row, registers, flags, bytes(normalized_stack)


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[slice(*routine.span)]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert executable[routine.span[1] - 1] == 0xC3
    return executable


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    return {
        (source - routine.span[0], destination - routine.span[0])
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-executable",
        type=Path,
        default=Path(__file__).parents[1] / "bin/BLOODPRG.EXE",
    )
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9b04_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for index, vector in enumerate(vectors):
        commander_result = execute(
            commander_executable, COMMANDER, vector, index, commander_edges
        )
        sequel_result = execute(sequel_executable, SEQUEL, vector, index, sequel_edges)
        commander_row, commander_registers, commander_flags, commander_stack = (
            commander_result
        )
        sequel_row, sequel_registers, sequel_flags, sequel_stack = sequel_result
        assert commander_row == vector, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        assert sequel_flags == commander_flags, vector["name"]
        assert sequel_stack == commander_stack, vector["name"]
        rows.append(sequel_row)

    assert commander_edges == normalized_edges(COMMANDER)
    assert sequel_edges == normalized_edges(SEQUEL)
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB point-plot cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
