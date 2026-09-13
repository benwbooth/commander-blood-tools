#!/usr/bin/env python3
"""Compare Big Bug Bang's rectangle blitter with Commander Blood."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

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

SEGMENT_SIZE = 0x10000
SOURCE = 0x20000
DESTINATION = 0x50000
STACK = 0x70000
EXTRA = 0x90000
GAME = 0xA0000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
BODY_SHA256 = "d68fc64fe931eda8ecabf762096b253b2324a77d9dcc17eb4298953f7d99fbdc"

BRANCHES = {
    0x17: (0x19, 0x44),
    0x21: (0x23, 0x2F),
    0x2B: (0x2D, 0x23),
    0x34: (0x36, 0x39),
    0x3A: (0x3C, 0x31),
    0x40: (0x42, 0x2F),
    0x4D: (0x4F, 0x55),
    0x5A: (0x5C, 0x5F),
    0x60: (0x62, 0x57),
}


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA4ED, 0xA552),
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBCD7, 0xBD3C),
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


def source_segment(case_index: int) -> bytearray:
    return bytearray(
        0
        if (offset + case_index) % 5 == 0
        else (offset * 17 + (offset >> 8) * 7 + case_index * 29) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 11 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def expected_edges() -> set[tuple[int, int]]:
    return {
        (source, destination)
        for source, destinations in BRANCHES.items()
        for destination in destinations
    }


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: tuple[int, int], label: str
) -> None:
    start, end = allowed
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or start <= offset < end:
            continue
        raise AssertionError(f"{label} changed outside stack envelope at {offset:#x}")


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert len(body) == 101 and body[-1] == 0xC3
    return executable


def expected_destination(
    source: bytes,
    destination: bytes,
    vector: dict[str, object],
) -> tuple[bytes, int, int, int]:
    width = int(vector["width"])
    row_mode = int(vector["row_mode"])
    rows = row_mode & 0xFF
    transparent = row_mode >> 8 == 0xFF
    reverse = vector["direction"] == "backward"
    source_offset = int(vector["source_offset"])
    x = int(vector["x"])
    y = int(vector["y"])
    step = -1 if reverse else 1
    destination_offset = (((y & 0xFF) << 8) + (y >> 8) + (y << 6) + x) & 0xFFFF
    result = bytearray(destination)

    def copy_pixels(count: int) -> None:
        nonlocal source_offset, destination_offset
        for _ in range(count):
            value = source[source_offset]
            if not transparent or value != 0:
                result[destination_offset] = value
            source_offset = (source_offset + step) & 0xFFFF
            destination_offset = (
                destination_offset + (1 if transparent else step)
            ) & 0xFFFF

    if width == 320:
        count = (rows * width) & 0xFFFF
        if transparent and count == 0:
            count = 0x10000
        copy_pixels(count)
        row_iterations = rows
    else:
        row_iterations = rows if rows != 0 else 0x100
        pitch = (320 - width) & 0xFFFF
        for _ in range(row_iterations):
            count = width
            if transparent and count == 0:
                count = 0x10000
            copy_pixels(count)
            destination_offset = (destination_offset + pitch) & 0xFFFF

    return bytes(result), source_offset, destination_offset, row_iterations


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], tuple]:
    name = str(vector["name"])
    width = int(vector["width"])
    row_mode = int(vector["row_mode"])
    rows = row_mode & 0xFF
    x = int(vector["x"])
    y = int(vector["y"])
    source_offset = int(vector["source_offset"])
    reverse = vector["direction"] == "backward"

    source_before = source_segment(case_index)
    destination_before = seeded_segment(case_index, 23, 0)
    destination_expected, source_result, destination_result, row_iterations = (
        expected_destination(source_before, destination_before, vector)
    )
    assert source_result == int(vector["source_result_offset"]), name
    assert destination_result == int(vector["destination_result_offset"]), name
    assert row_iterations == int(vector["row_iterations"]), name
    changed_bytes = sum(
        before != after
        for before, after in zip(destination_before, destination_expected, strict=True)
    )
    assert changed_bytes == int(vector["changed_bytes"]), name

    extra_before = seeded_segment(case_index, 29, 0x65)
    game_before = seeded_segment(case_index, 31, 0x87)
    stack_before = seeded_segment(case_index, 41, 0xA9)
    struct.pack_into("<H", stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B20000 | y,
        UC_X86_REG_ECX: 0xC3C30000 | row_mode,
        UC_X86_REG_EDX: 0xD4D40000 | x,
        UC_X86_REG_ESI: 0xE5E50000 | source_offset,
        UC_X86_REG_EDI: 0xF6F60000 | width,
        UC_X86_REG_EBP: 0x97972468 + case_index,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: SOURCE // 16,
        UC_X86_REG_ES: DESTINATION // 16,
        UC_X86_REG_FS: EXTRA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0602 if reverse else 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(SOURCE, bytes(source_before))
    machine.mem_write(DESTINATION, bytes(destination_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        assert routine.span[0] <= file_address < routine.span[1], (
            routine.name,
            name,
            hex(file_address),
        )
        normalized = file_address - routine.span[0]
        if previous in BRANCHES:
            covered_edges.add((previous, normalized))
        previous = normalized

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=500000)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    assert bytes(machine.mem_read(SOURCE, SEGMENT_SIZE)) == bytes(source_before)
    destination_after = bytes(machine.mem_read(DESTINATION, SEGMENT_SIZE))
    assert destination_after == destination_expected, (routine.name, name, "destination")
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert_unchanged_outside(
        bytes(stack_before), stack_after, (0xFEFC, 0xFF02), f"{routine.name} {name}"
    )
    assert stack_after[0xFF02 : 0xFF02 + len(STACK_SENTINEL)] == STACK_SENTINEL

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_ECX] &= 0xFFFF0000
    expected_registers[UC_X86_REG_EDX] = (
        initial[UC_X86_REG_EDX] & 0xFFFF0000
    ) | (
        ((rows * width) >> 16)
        if width == 320
        else (initial[UC_X86_REG_EDX] & 0xFF00)
    )
    expected_registers[UC_X86_REG_ESI] = (
        initial[UC_X86_REG_ESI] & 0xFFFF0000
    ) | source_result
    expected_registers[UC_X86_REG_EDI] = (
        initial[UC_X86_REG_EDI] & 0xFFFF0000
    ) | destination_result
    expected_registers[UC_X86_REG_EBP] = (
        initial[UC_X86_REG_EBP] & 0xFFFF0000
    ) | width
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers)
    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)

    row = {
        "name": name,
        "width": width,
        "row_mode": row_mode,
        "rows": rows,
        "row_iterations": row_iterations,
        "transparent_zero": row_mode >> 8 == 0xFF,
        "x": x,
        "y": y,
        "direction": str(vector["direction"]),
        "source_offset": source_offset,
        "source_result_offset": source_result,
        "destination_result_offset": destination_result,
        "changed_bytes": changed_bytes,
        "result_flags": result_flags & 0xFFFF,
        "destination_sha256": hashlib.sha256(destination_after).hexdigest(),
    }
    for key, value in vector.items():
        assert row[key] == value, (routine.name, name, key, row[key], value)
    canonical = (
        destination_after,
        tuple(registers.items()),
        result_flags,
        stack_after,
    )
    return row, canonical


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
        default=Path(__file__).parent / "oracle_vectors/func_a4ed_natural.json",
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
    for case_index, vector in enumerate(vectors):
        commander_row, commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_row, sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        assert commander_row == sequel_row, vector["name"]
        rows.append(sequel_row)

    assert commander_edges == expected_edges(), (commander_edges, expected_edges())
    assert sequel_edges == expected_edges(), (sequel_edges, expected_edges())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB presentation rectangle-blit cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
