#!/usr/bin/env python3
"""Compare Big Bug Bang's ship point-cloud projector with Commander Blood."""

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
EXTRA = 0x60000
FRAMEBUFFER = 0x70000
FS_DATA = 0x80000
STACK = 0x90000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
POINT_COUNT = 1000

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
    counter: int
    points: int
    work: int
    framebuffer_segment: int
    matrix: int
    camera: int
    plot_entry: int
    plot_return: int
    branches: dict[int, tuple[int, int]]


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x9A10, 0x9B04),
    body_sha256="10a3734ef018c6766adeb2def5cb606e4af026d9ae3f26463eb2baf9c5cd45da",
    counter=0x2F77,
    points=0x2FC1,
    work=0x4F01,
    framebuffer_segment=0x5223,
    matrix=0x2F95,
    camera=0x2F65,
    plot_entry=0x9B04,
    plot_return=0x9AEE,
    branches={
        0x9A7A: (0x9AEE, 0x9A7C),
        0x9A7C: (0x9AEE, 0x9A7E),
        0x9AF2: (0x9A34, 0x9AF6),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xB1AF, 0xB2A3),
    body_sha256="8e6bd6c8aba46c72eae7df6c684f94d33cfcf6214cca2a2c26dec9d93c355a12",
    counter=0x3347,
    points=0x3391,
    work=0x52D1,
    framebuffer_segment=0x55F3,
    matrix=0x3365,
    camera=0x3335,
    plot_entry=0xB2A3,
    plot_return=0xB28D,
    branches={
        0xB219: (0xB28D, 0xB21B),
        0xB21B: (0xB28D, 0xB21D),
        0xB291: (0xB1D3, 0xB295),
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

CASE_SEEDS = (0x11, 0x2B, 0x45, 0x5F, 0x79, 0x93)
CALLBACK_CARRY = (True, False, True, False, False, True)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def signed16(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


def signed32(value: int) -> int:
    value &= 0xFFFFFFFF
    return value - 0x100000000 if value & 0x80000000 else value


def multiply32(left: int, right: int) -> int:
    return signed32(left * right)


def add32(left: int, right: int) -> int:
    return signed32(left + right)


def dot32(components: list[int], terms: list[int]) -> int:
    partial = add32(
        multiply32(components[0], terms[0]),
        multiply32(components[1], terms[1]),
    )
    return add32(partial, multiply32(components[2], terms[2]))


def divide_toward_zero(dividend: int, divisor: int) -> int:
    magnitude = abs(dividend) // abs(divisor)
    return -magnitude if (dividend < 0) != (divisor < 0) else magnitude


def seeded_segment(
    seed: int, multiplier: int, high_multiplier: int, salt: int
) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * high_multiplier + seed + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def make_points(
    case_index: int, seed: int, camera: list[int], iteration_count: int
) -> list[list[int]]:
    points = []
    for point_index in range(POINT_COUNT):
        x = (point_index * 977 + seed * 0x101 + 0x1234) & 0xFFFF
        y = (point_index * 613 + seed * 0x83 + 0x4321) & 0xFFFF
        if case_index == 3:
            z = (point_index * 37 + 1) & 0x7FFF
        elif case_index == 0:
            z = (0, 1, 1000, 0x7FFF, 0xFFFF, 0x8000)[point_index % 6]
        else:
            z = (point_index * 283 + seed * 17 + 1) & 0xFFFF
        scratch = (point_index * 0x1111 + seed * 0x0101) & 0xFFFF
        points.append([x, y, z, scratch])
    final_index = iteration_count - 1
    points[final_index][:3] = [camera[0], camera[1], (camera[2] + 1000) & 0xFFFF]
    return points


def expected_calls(
    points: list[list[int]], matrix: list[int], camera: list[int], iteration_count: int
) -> tuple[list[dict[str, object]], list[int]]:
    calls = []
    last_work = []
    for point_index in range(iteration_count):
        point = points[point_index]
        translated_words = [(point[axis] - camera[axis]) & 0xFFFF for axis in range(3)]
        translated = [signed16(value) for value in translated_words]
        last_work = translated_words + [point[3]]
        depth = dot32(translated, matrix[6:9]) >> 15
        if depth <= 0:
            continue
        x_axis = dot32(translated, matrix[0:3]) >> 7
        y_axis = dot32(translated, matrix[3:6]) >> 7
        calls.append(
            {
                "point_index": point_index,
                "projected": [
                    (divide_toward_zero(x_axis, depth) + 160) & 0xFFFF,
                    (divide_toward_zero(y_axis, depth) + 100) & 0xFFFF,
                    depth & 0xFFFF,
                ],
                "remaining_at_call": iteration_count - point_index,
                "source_si": (COMMANDER.points + (point_index + 1) * 8) & 0xFFFF,
                "work": last_work,
            }
        )
    assert last_work
    return calls, last_work


def initialize_memory(
    routine: Routine, vector: dict[str, object], case_index: int
) -> tuple[
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    bytearray,
    list[dict[str, object]],
]:
    seed = CASE_SEEDS[case_index]
    matrix = [int(value) for value in vector["matrix"]]
    camera = [int(value) for value in vector["camera"]]
    iteration_count = int(vector["iterations"])
    split_segments = not bool(vector["runtime_ds_equals_gs"])
    points = make_points(case_index, seed, camera, iteration_count)
    calls, final_work = expected_calls(points, matrix, camera, iteration_count)

    game_before = seeded_segment(seed, 29, 13, 0)
    struct.pack_into("<3H", game_before, routine.camera, *camera)
    struct.pack_into("<H", game_before, routine.framebuffer_segment, FRAMEBUFFER // 16)
    if split_segments:
        struct.pack_into("<H", game_before, routine.counter, iteration_count)
    for point_index, point in enumerate(points):
        struct.pack_into("<4H", game_before, routine.points + point_index * 8, *point)
    game_expected = bytearray(game_before)
    struct.pack_into("<4H", game_expected, routine.work, *final_work)
    struct.pack_into("<H", game_expected, routine.counter, 0)

    data_before = seeded_segment(seed, 17, 7, 0x31)
    data_expected = bytearray(data_before)
    if split_segments:
        struct.pack_into("<H", data_expected, routine.counter, POINT_COUNT)
    extra_before = seeded_segment(seed, 11, 5, 0x53)
    framebuffer_before = seeded_segment(seed, 19, 3, 0x75)
    stack_before = seeded_segment(seed, 7, 23, 0x97)
    struct.pack_into("<9i", stack_before, routine.matrix, *matrix)
    stack_before[STACK_POINTER : STACK_POINTER + 4] = struct.pack("<HH", RETURN_IP, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    stack_expected = bytearray(stack_before)
    initial_ds = DATA // 16 if split_segments else GAME // 16
    initial_values = (
        0xA1A11234 + case_index,
        0xB2B22345 + case_index,
        0xC3C33456 + case_index,
        0xD4D44567 + case_index,
    )
    for offset, value in zip(
        (0xFEFC, 0xFEF8, 0xFEF4, 0xFEF0), initial_values, strict=True
    ):
        struct.pack_into("<I", stack_expected, offset, value)
    struct.pack_into("<H", stack_expected, 0xFEEE, (0x9797789A + case_index) & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEEC, initial_ds)
    struct.pack_into("<H", stack_expected, 0xFEEA, (0xF6F66789 + case_index) & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEE8, EXTRA // 16)
    struct.pack_into("<H", stack_expected, 0xFEE6, (0xE5E55678 + case_index) & 0xFFFF)
    if calls:
        struct.pack_into(
            "<H", stack_expected, 0xFEE4, image_address(routine.plot_return)
        )
        struct.pack_into(
            "<3H", stack_expected, routine.matrix + 36, *calls[-1]["projected"]
        )
    return (
        game_before,
        game_expected,
        data_before,
        data_expected,
        extra_before,
        framebuffer_before,
        stack_before,
        stack_expected,
        calls,
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes]:
    name = str(vector["name"])
    (
        game_before,
        game_expected,
        data_before,
        data_expected,
        extra_before,
        framebuffer_before,
        stack_before,
        stack_expected,
        expected,
    ) = initialize_memory(routine, vector, case_index)
    fs_before = bytes(seeded_segment(CASE_SEEDS[case_index], 5, 17, 0xA7))
    split_segments = not bool(vector["runtime_ds_equals_gs"])
    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E55678 + case_index,
        UC_X86_REG_EDI: 0xF6F66789 + case_index,
        UC_X86_REG_EBP: 0x9797789A + case_index,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16 if split_segments else GAME // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    expected_module[image_address(routine.plot_entry)] = 0xC3
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FRAMEBUFFER, bytes(framebuffer_before))
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    observed = []
    observed_bytes = bytearray()
    reached_return = False
    previous_file_offset: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return, previous_file_offset
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == image_address(routine.plot_entry):
            call_index = len(observed)
            assert call_index < len(expected), (routine.name, name, call_index)
            expected_call = expected[call_index]
            sp = cpu.reg_read(UC_X86_REG_SP)
            assert struct.unpack("<H", cpu.mem_read(STACK + sp, 2))[0] == image_address(
                routine.plot_return
            )
            actual = {
                "point_index": int(expected_call["point_index"]),
                "projected": list(
                    struct.unpack("<3H", cpu.mem_read(STACK + routine.matrix + 36, 6))
                ),
                "remaining_at_call": struct.unpack(
                    "<H", cpu.mem_read(GAME + routine.counter, 2)
                )[0],
                "source_si": (
                    (cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF)
                    - routine.points
                    + COMMANDER.points
                )
                & 0xFFFF,
                "work": list(
                    struct.unpack("<4H", cpu.mem_read(GAME + routine.work, 8))
                ),
            }
            assert actual == expected_call, (routine.name, name, actual, expected_call)
            assert cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF == routine.matrix
            assert cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF == routine.work
            assert cpu.reg_read(UC_X86_REG_DS) == GAME // 16
            assert cpu.reg_read(UC_X86_REG_ES) == FRAMEBUFFER // 16
            assert sp == 0xFEE4
            observed.append(actual)
            observed_bytes.extend(
                struct.pack("<I3H", int(actual["point_index"]), *actual["projected"])
            )
            cpu.reg_write(UC_X86_REG_EFLAGS, 0x0202 | int(CALLBACK_CARRY[case_index]))
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

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(image_address(routine.span[0]), 0, count=300000)
    assert reached_return and observed == expected, (routine.name, name)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_expected)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FRAMEBUFFER, SEGMENT_SIZE)) == bytes(
        framebuffer_before
    )
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    actual_stack = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert actual_stack == bytes(stack_expected), (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = 0xFF04
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    flag_masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    defined_flags = {
        flag: bool(flags & flag_masks[flag]) for flag in vector["defined_flags"]
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, flags)

    final_work = list(
        struct.unpack("<4H", game_expected[routine.work : routine.work + 8])
    )
    row = {
        "name": name,
        "runtime_ds_equals_gs": not split_segments,
        "iterations": int(vector["iterations"]),
        "matrix": list(vector["matrix"]),
        "camera": list(vector["camera"]),
        "plot_calls": len(observed),
        "plot_sequence_sha256": hashlib.sha256(observed_bytes).hexdigest(),
        "first_plot": observed[0] if observed else None,
        "last_plot": observed[-1] if observed else None,
        "final_work": final_work,
        "entry_ds_counter_after": (
            struct.unpack("<H", data_expected[routine.counter : routine.counter + 2])[0]
            if split_segments
            else None
        ),
        "game_counter_after": struct.unpack(
            "<H", game_expected[routine.counter : routine.counter + 2]
        )[0],
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    normalized_stack = bytearray(actual_stack)
    stack_seed = seeded_segment(CASE_SEEDS[case_index], 7, 23, 0x97)
    normalized_stack[routine.matrix : routine.matrix + 42] = stack_seed[
        routine.matrix : routine.matrix + 42
    ]
    normalized_stack[COMMANDER.matrix : COMMANDER.matrix + 36] = actual_stack[
        routine.matrix : routine.matrix + 36
    ]
    if observed:
        normalized_stack[COMMANDER.matrix + 36 : COMMANDER.matrix + 42] = actual_stack[
            routine.matrix + 36 : routine.matrix + 42
        ]
        struct.pack_into(
            "<H", normalized_stack, 0xFEE4, image_address(COMMANDER.plot_return)
        )
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
    assert executable[routine.span[1] - 1] == 0xCB
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
        default=Path(__file__).parent / "oracle_vectors/func_9a10_natural.json",
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
        f"verified {len(rows)} BBB point-cloud cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
