#!/usr/bin/env python3
"""Compare Big Bug Bang's ship point randomizer with Commander Blood."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
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
DATA = 0x20000
GAME = 0x40000
EXTRA = 0x60000
FS_DATA = 0x70000
STACK = 0x80000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
POINT_COUNT = 1000
POINT_SIZE = 8
OUTPUT_COUNT = POINT_COUNT * 3
CASE_SEEDS = (0x11, 0x37, 0x5D, 0x83)

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
    points: int
    prng_address: int
    prng_cs: int
    prng_ip: int
    return_offsets: tuple[int, int, int]
    loop_source: int
    loop_destinations: tuple[int, int]
    defined_flags: dict[str, bool]


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x9B67, 0x9B98),
    body_sha256="8e518bfaaeff24ed55e4ebbdaec0ea0e13b3ee7810410bcd5a1e23d47938df31",
    points=0x2FC1,
    prng_address=0x27E2,
    prng_cs=0x01CE,
    prng_ip=0x0B02,
    return_offsets=(0x9B7C, 0x9B85, 0x9B8E),
    loop_source=0x9B92,
    loop_destinations=(0x9B74, 0x9B94),
    defined_flags={
        "cf": False,
        "pf": False,
        "af": True,
        "zf": False,
        "sf": False,
        "of": False,
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xB306, 0xB337),
    body_sha256="99755d366dfeb572c7816d20eac246cfad2a350397b55956082341a4d0008365",
    points=0x3391,
    prng_address=0x2963,
    prng_cs=0x01E6,
    prng_ip=0x0B03,
    return_offsets=(0xB31B, 0xB324, 0xB32D),
    loop_source=0xB331,
    loop_destinations=(0xB313, 0xB333),
    defined_flags={
        "cf": False,
        "pf": True,
        "af": True,
        "zf": False,
        "sf": False,
        "of": False,
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


def seeded_segment(seed: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + seed + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def random_outputs(case_index: int) -> list[int]:
    if case_index == 0:
        return [0] * OUTPUT_COUNT
    if case_index == 1:
        return [(0x1234 + index * 37) % 0xFFFF for index in range(OUTPUT_COUNT)]
    if case_index == 2:
        extrema = (0x0000, 0x0001, 0x7FFF, 0x8000, 0xFFFD, 0xFFFE)
        return [extrema[index % len(extrema)] for index in range(OUTPUT_COUNT)]
    if case_index == 3:
        outputs = []
        state = 0xACE1
        for _ in range(OUTPUT_COUNT):
            state = (state * 25173 + 13849) & 0xFFFF
            outputs.append(state % 0xFFFF)
        return outputs
    raise AssertionError(f"unexpected randomizer fixture {case_index}")


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes]:
    name = str(vector["name"])
    seed = CASE_SEEDS[case_index]
    outputs = random_outputs(case_index)
    assert hashlib.sha256(struct.pack("<3000H", *outputs)).hexdigest() == vector[
        "prng_outputs_sha256"
    ]

    cloud_before = bytearray(
        (seed + index * 17) & 0xFF for index in range(POINT_COUNT * POINT_SIZE)
    )
    cloud_expected = bytearray(cloud_before)
    for record_index in range(POINT_COUNT):
        struct.pack_into(
            "<3H",
            cloud_expected,
            record_index * POINT_SIZE,
            *outputs[record_index * 3 : record_index * 3 + 3],
        )

    game_before = seeded_segment(seed, 29, 0x21)
    game_before[routine.points : routine.points + len(cloud_before)] = cloud_before
    game_expected = bytearray(game_before)
    game_expected[routine.points : routine.points + len(cloud_expected)] = cloud_expected
    data_before = seeded_segment(seed, 17, 0x43)
    extra_before = seeded_segment(seed, 11, 0x65)
    fs_before = seeded_segment(seed, 5, 0x87)
    stack_before = seeded_segment(seed, 7, 0xA9)
    stack_before[STACK_POINTER : STACK_POINTER + 4] = struct.pack("<HH", RETURN_IP, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    stack_expected = bytearray(stack_before)
    struct.pack_into("<H", stack_expected, 0xFEFE, initial[UC_X86_REG_ES])
    struct.pack_into("<H", stack_expected, 0xFEFC, initial[UC_X86_REG_EDI] & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEFA, initial[UC_X86_REG_EAX] & 0xFFFF)
    struct.pack_into("<H", stack_expected, 0xFEF8, 0)
    struct.pack_into(
        "<H", stack_expected, 0xFEF6, image_address(routine.return_offsets[2])
    )

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x90000)
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    expected_module[routine.prng_address] = 0xCB
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    call_count = 0
    call_samples = []
    reached_return = False
    previous_file_offset: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal call_count, reached_return, previous_file_offset
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == routine.prng_address:
            assert call_count < OUTPUT_COUNT, (routine.name, name, call_count)
            record_index, component = divmod(call_count, 3)
            expected_di = routine.points + record_index * POINT_SIZE + component * 2
            expected_return = routine.return_offsets[component]
            actual = {
                "ax": cpu.reg_read(UC_X86_REG_AX),
                "cx": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                "es": cpu.reg_read(UC_X86_REG_ES),
                "cs": cpu.reg_read(UC_X86_REG_CS),
                "ip": address - routine.prng_cs * 16,
                "sp": cpu.reg_read(UC_X86_REG_SP),
            }
            expected = {
                "ax": 0xFFFF,
                "cx": POINT_COUNT - record_index,
                "di": expected_di,
                "es": GAME // 16,
                "cs": routine.prng_cs,
                "ip": routine.prng_ip,
                "sp": 0xFEF6,
            }
            assert actual == expected, (routine.name, name, call_count, actual, expected)
            return_words = struct.unpack("<HH", cpu.mem_read(STACK + 0xFEF6, 4))
            assert return_words == (image_address(expected_return), 0), (
                routine.name,
                name,
                call_count,
                return_words,
            )
            for register in (
                UC_X86_REG_EBX,
                UC_X86_REG_EDX,
                UC_X86_REG_ESI,
                UC_X86_REG_EBP,
                UC_X86_REG_DS,
                UC_X86_REG_FS,
                UC_X86_REG_GS,
                UC_X86_REG_SS,
            ):
                assert cpu.reg_read(register) == initial[register]
            if call_count < 3 or call_count >= OUTPUT_COUNT - 3:
                call_samples.append(
                    {
                        "index": call_count,
                        "record": record_index,
                        "component": component,
                        "di": COMMANDER.points
                        + record_index * POINT_SIZE
                        + component * 2,
                        "cx": POINT_COUNT - record_index,
                        "return_ip": COMMANDER.return_offsets[component],
                    }
                )
            cpu.reg_write(UC_X86_REG_AX, outputs[call_count])
            call_count += 1
            return

        file_offset = address + HEADER_SIZE
        assert routine.span[0] <= file_offset < routine.span[1], (
            routine.name,
            name,
            hex(file_offset),
        )
        if previous_file_offset == routine.loop_source:
            covered_edges.add(
                (
                    previous_file_offset - routine.span[0],
                    file_offset - routine.span[0],
                )
            )
        previous_file_offset = file_offset

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(image_address(routine.span[0]), 0, count=100000)
    assert reached_return and call_count == OUTPUT_COUNT, (routine.name, name, call_count)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_expected)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    actual_stack = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert actual_stack == bytes(stack_expected), (routine.name, name)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_ECX] &= 0xFFFF0000
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    defined_flags = {flag: bool(flags & mask) for flag, mask in masks.items()}
    assert defined_flags == routine.defined_flags, (routine.name, name, flags)

    cloud_after = bytes(
        machine.mem_read(GAME + routine.points, POINT_COUNT * POINT_SIZE)
    )
    scratch = b"".join(
        cloud_after[index * POINT_SIZE + 6 : index * POINT_SIZE + 8]
        for index in range(POINT_COUNT)
    )
    row = {
        "name": name,
        "prng_call_count": call_count,
        "prng_outputs_sha256": hashlib.sha256(
            struct.pack("<3000H", *outputs)
        ).hexdigest(),
        "call_samples": call_samples,
        "first_record": list(struct.unpack("<4H", cloud_after[:POINT_SIZE])),
        "last_record": list(struct.unpack("<4H", cloud_after[-POINT_SIZE:])),
        "point_cloud_sha256": hashlib.sha256(cloud_after).hexdigest(),
        "scratch_sha256": hashlib.sha256(scratch).hexdigest(),
        "final_cx": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    normalized_stack = bytearray(actual_stack)
    struct.pack_into(
        "<H", normalized_stack, 0xFEF6, image_address(COMMANDER.return_offsets[2])
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
        (
            routine.loop_source - routine.span[0],
            destination - routine.span[0],
        )
        for destination in routine.loop_destinations
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
        default=Path(__file__).parent / "oracle_vectors/func_9b67_natural.json",
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
        sequel_semantics = dict(sequel_row)
        sequel_semantics["defined_flags"] = commander_row["defined_flags"]
        assert sequel_semantics == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        arithmetic_flags = 0x08D5
        assert sequel_flags & ~arithmetic_flags == commander_flags & ~arithmetic_flags
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
        f"verified {len(rows)} BBB point-randomizer cases covering "
        f"{len(sequel_edges)} normalized loop edges"
    )


if __name__ == "__main__":
    main()
