#!/usr/bin/env python3
"""Compare Big Bug Bang's bridge steering directly with Commander Blood."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
from unicorn.x86_const import (
    UC_X86_REG_AX,
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
DATA = 0x44000
GAME = 0x2C000
EXTRA = 0x68000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

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
    offsets: dict[str, int]


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x9656, 0x981B),
    body_sha256="6fe70e0d16926546ddd268e3b76389d4ac4ba7fb4aa0792ff114be219e8be55f",
    offsets={
        "ui": 0x2793,
        "frame": 0x2795,
        "mouse_arc": 0x2797,
        "seek_target": 0x279B,
        "seek_initial": 0x279D,
        "frame_angle_bias": 0x27A7,
        "direction": 0x27DB,
        "projection_angle": 0x2F6D,
        "mouse": 0x0A2A,
        "mouse_y": 0x0A2C,
        "mouse_buttons": 0x0A2E,
        "mouse_drag_reference": 0x0A38,
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xADF5, 0xAFBA),
    body_sha256="7a5960767560ae3e11014cd35fe2385c520b4a7de7f7446f1fb1650406a138b3",
    offsets={
        "ui": 0x2A33,
        "frame": 0x2A35,
        "mouse_arc": 0x2A37,
        "seek_target": 0x2A3B,
        "seek_initial": 0x2A3D,
        "frame_angle_bias": 0x2A47,
        "direction": 0x2A7B,
        "projection_angle": 0x333D,
        "mouse": 0x0C22,
        "mouse_y": 0x0C24,
        "mouse_buttons": 0x0C26,
        "mouse_drag_reference": 0x0C30,
    },
)

SEQUEL_BRANCHES = {
    0xAE02: (0xAE9B, 0xAE06),
    0xAE0E: (0xAE1D, 0xAE10),
    0xAE1D: (0xAE20, 0xAE1F),
    0xAE25: (0xAE2C, 0xAE27),
    0xAE32: (0xAE3A, 0xAE34),
    0xAE3E: (0xAE44, 0xAE40),
    0xAE4C: (0xAE51, 0xAE4E),
    0xAE55: (0xAE58, 0xAE57),
    0xAE64: (0xAE6F, 0xAE66),
    0xAE76: (0xAE7E, 0xAE78),
    0xAE83: (0xAE8A, 0xAE85),
    0xAE8D: (0xAE92, 0xAE8F),
    0xAEA4: (0xAEAC, 0xAEA6),
    0xAEB0: (0xAEB6, 0xAEB2),
    0xAED8: (0xAF86, 0xAEDC),
    0xAEE2: (0xAEE5, 0xAEE4),
    0xAEEA: (0xAEF1, 0xAEEC),
    0xAEF5: (0xAF9B, 0xAEF9),
    0xAEFF: (0xAF4C, 0xAF01),
    0xAF05: (0xAF9B, 0xAF09),
    0xAF11: (0xAF17, 0xAF13),
    0xAF19: (0xAF2A, 0xAF1B),
    0xAF22: (0xAF33, 0xAF24),
    0xAF2D: (0xAF33, 0xAF2F),
    0xAF54: (0xAF5A, 0xAF56),
    0xAF5C: (0xAF6E, 0xAF5E),
    0xAF66: (0xAF80, 0xAF68),
    0xAF7A: (0xAF80, 0xAF7C),
    0xAFA4: (0xAFAA, 0xAFA6),
    0xAFAE: (0xAFB4, 0xAFB0),
}

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
    UC_X86_REG_EFLAGS,
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initialize_data(
    routine: Routine, vector: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray]:
    offsets = routine.offsets
    before = seeded_segment(case_index, 17, 0x43)
    for field in (
        "ui",
        "frame",
        "mouse",
        "mouse_arc",
        "mouse_drag_reference",
        "mouse_buttons",
        "seek_initial",
        "projection_angle",
        "frame_angle_bias",
    ):
        write16(before, offsets[field], int(vector[f"{field}_before"]))
    write16(before, offsets["mouse_y"], int(vector["mouse_y"]))
    write16(before, offsets["seek_target"], int(vector["seek_target"]))
    before[offsets["direction"]] = int(vector["direction_before"])

    expected = bytearray(before)
    for field in (
        "ui",
        "frame",
        "mouse",
        "mouse_arc",
        "mouse_drag_reference",
        "mouse_buttons",
        "seek_initial",
        "projection_angle",
        "frame_angle_bias",
    ):
        write16(expected, offsets[field], int(vector[f"{field}_after"]))
    expected[offsets["direction"]] = int(vector["direction_after"])
    return before, expected


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    branch_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], bytes]:
    name = str(vector["name"])
    data_before, data_expected = initialize_data(routine, vector, case_index)
    game_before = bytes(seeded_segment(case_index, 23, 0x71))
    extra_before = bytes(seeded_segment(case_index, 11, 0xA5))
    stack_before = seeded_segment(case_index, 7, 0x63)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    write16(stack_before, STACK_POINTER + 2, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    stack_expected = bytearray(stack_before)
    write16(stack_expected, STACK_POINTER - 2, int(vector["final_flags"]))

    initial = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x97970000 | int(vector["presentation_context_before"]),
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: 0x7400,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    machine.mem_write(0, module)
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, game_before)
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    warps: list[list[int]] = []
    reached_return = False
    previous_file_offset: int | None = None

    def instruction(cpu: Uc, address: int, size: int, _context) -> None:
        nonlocal previous_file_offset, reached_return
        file_offset = address + HEADER_SIZE
        if previous_file_offset is not None:
            normalized_source = previous_file_offset - routine.span[0]
            sequel_source = normalized_source + SEQUEL.span[0]
            if sequel_source in SEQUEL_BRANCHES:
                branch_edges.add((normalized_source, file_offset - routine.span[0]))
        previous_file_offset = file_offset
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        assert (
            image_address(routine.span[0])
            <= address
            < address + size
            <= image_address(routine.span[1])
        ), (routine.name, name, hex(file_offset))

    def interrupt(cpu: Uc, number: int, _context) -> None:
        assert number == 0x33, (routine.name, name, number)
        assert cpu.reg_read(UC_X86_REG_AX) == 4, (routine.name, name)
        warps.append(
            [
                cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
            ]
        )

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        data_write = DATA <= address < address + size <= DATA + SEGMENT_SIZE
        stack_write = STACK <= address < address + size <= STACK + SEGMENT_SIZE
        assert data_write or stack_write, (routine.name, name, hex(address), size)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.emu_start(image_address(routine.span[0]), 0, count=2000)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected), (
        routine.name,
        name,
    )
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == game_before, (
        routine.name,
        name,
    )
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before, (
        routine.name,
        name,
    )
    assert bytes(machine.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), (
        routine.name,
        name,
    )
    assert bytes(machine.mem_read(0, len(module))) == module, (routine.name, name)
    assert warps == vector["mouse_warps"], (routine.name, name, warps)

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    offsets = routine.offsets
    row = {
        "name": name,
        "ui_before": int(vector["ui_before"]),
        "ui_after": read16(data_after, offsets["ui"]),
        "frame_before": int(vector["frame_before"]),
        "frame_after": read16(data_after, offsets["frame"]),
        "mouse_before": int(vector["mouse_before"]),
        "mouse_after": read16(data_after, offsets["mouse"]),
        "mouse_y": read16(data_after, offsets["mouse_y"]),
        "mouse_arc_before": int(vector["mouse_arc_before"]),
        "mouse_arc_after": read16(data_after, offsets["mouse_arc"]),
        "mouse_drag_reference_before": int(vector["mouse_drag_reference_before"]),
        "mouse_drag_reference_after": read16(
            data_after, offsets["mouse_drag_reference"]
        ),
        "mouse_buttons_before": int(vector["mouse_buttons_before"]),
        "mouse_buttons_after": read16(data_after, offsets["mouse_buttons"]),
        "seek_target": read16(data_after, offsets["seek_target"]),
        "seek_initial_before": int(vector["seek_initial_before"]),
        "seek_initial_after": read16(data_after, offsets["seek_initial"]),
        "direction_before": int(vector["direction_before"]),
        "direction_after": data_after[offsets["direction"]],
        "projection_angle_before": int(vector["projection_angle_before"]),
        "projection_angle_after": read16(data_after, offsets["projection_angle"]),
        "frame_angle_bias_before": int(vector["frame_angle_bias_before"]),
        "frame_angle_bias_after": read16(data_after, offsets["frame_angle_bias"]),
        "presentation_context_before": int(vector["presentation_context_before"]),
        "presentation_context_after": machine.reg_read(UC_X86_REG_EBP) & 0xFFFF,
        "view_changed": bool(machine.reg_read(UC_X86_REG_EFLAGS) & 1),
        "mouse_warps": warps,
        "final_flags": machine.reg_read(UC_X86_REG_EFLAGS) & 0xFFFF,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    return row, registers, bytes(machine.mem_read(STACK, SEGMENT_SIZE))


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
    assert executable[routine.span[1] - 1] == 0xCB
    return executable


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
        default=Path(__file__).parent / "oracle_vectors/func_9656_natural.json",
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
        commander_row, commander_registers, commander_stack = execute(
            commander_executable, COMMANDER, vector, index, commander_edges
        )
        sequel_row, sequel_registers, sequel_stack = execute(
            sequel_executable, SEQUEL, vector, index, sequel_edges
        )
        assert commander_row == vector, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        assert sequel_stack == commander_stack, vector["name"]
        rows.append(sequel_row)

    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB steering cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
