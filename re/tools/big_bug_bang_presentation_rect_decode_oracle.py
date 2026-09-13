#!/usr/bin/env python3
"""Compare BBB's rectangular presentation decoder with Commander Blood."""

# ruff: noqa: E402

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
from typing import Any

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

MACHINE_SIZE = 0x90000
SEGMENT_SIZE = 0x10000
SOURCE = 0x20000
STAGING = 0x38000
FRAMEBUFFER = 0x50000
GAME = 0x68000
STACK = 0x78000
STACK_POINTER = 0xFF00
STACK_TRANSIENT_START = 0xFE00
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
COMMANDER_FIXTURE_SHA256 = (
    "03652ed54348c9f46d230e662096d7c59d0b24d7cc25d2ee478f2c460d6e8b09"
)
FLAG_MASKS = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
ARITHMETIC_FLAG_MASK = sum(FLAG_MASKS.values())
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


@dataclass(frozen=True)
class Routine:
    name: str
    entry: int
    end: int
    body_sha256: str
    helper_entry: int
    helper_end: int
    helper_return: int
    helper_sha256: str
    code_bias_aliases: tuple[int, int]
    helper_bias_operands: tuple[int, int]
    low_main_entry: int
    high_main_entry: int
    scanline_helper: int
    mode_offset: int
    staging_segment_offset: int
    raw_width_offset: int
    raw_rows_offset: int
    vertical_offset: int
    framebuffer_segment_offset: int


COMMANDER = Routine(
    "Commander Blood",
    0xAB25,
    0xAF95,
    "ba6707b90afc944be901c66398b2f1272535c6c5502dd573b7d17160404379cf",
    0xAABC,
    0xAB25,
    0xAB67,
    "6f9aba91cf84930a552caddcbe3511006f7b3c6cca5ee86f53b57935f6816775",
    (0x0DDD, 0x0E0D),
    (0xAAED, 0xAB1D),
    0xABCE,
    0xADC3,
    0xAD96,
    0x0AA0,
    0x0ABE,
    0x0DA4,
    0x0DA6,
    0x1FA7,
    0x5223,
)
SEQUEL = Routine(
    "Big Bug Bang",
    0xC30D,
    0xC77D,
    "b8faa7f10d48be4f3133b0bdefa751587130d1436243a57935a17b18ad4a949f",
    0xC2A4,
    0xC30D,
    0xC34F,
    "6f9aba91cf84930a552caddcbe3511006f7b3c6cca5ee86f53b57935f6816775",
    (0x0E25, 0x0E55),
    (0xC2D5, 0xC305),
    0xC3B6,
    0xC5AB,
    0xC57E,
    0x0C98,
    0x0CB6,
    0x0FF2,
    0x0FF4,
    0x21F5,
    0x55F3,
)


def patterned_segment(
    case_index: int, step: int, page_step: int, case_step: int, base: int = 0
) -> bytearray:
    return bytearray(
        (offset * step + (offset >> 8) * page_step + case_index * case_step + base)
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_wrapped(data: bytearray, offset: int, encoded: bytes) -> None:
    for index, value in enumerate(encoded):
        data[(offset + index) & 0xFFFF] = value


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[routine.entry : routine.end]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(f"{routine.name} rectangle decoder changed: {body_digest}")
    helper_digest = hashlib.sha256(
        executable[routine.helper_entry : routine.helper_end]
    ).hexdigest()
    if helper_digest != routine.helper_sha256:
        raise SystemExit(f"{routine.name} pair helper changed: {helper_digest}")
    return executable


def load_cases(path: Path) -> list[dict[str, Any]]:
    encoded = path.read_bytes()
    digest = hashlib.sha256(encoded).hexdigest()
    if digest != COMMANDER_FIXTURE_SHA256:
        raise SystemExit(f"Commander rectangle fixture changed: {digest}")
    cases = json.loads(encoded)
    if len(cases) != 8:
        raise SystemExit(f"expected 8 Commander rectangle cases, found {len(cases)}")
    return cases


class StreamReader:
    def __init__(self, stream: bytes) -> None:
        self.stream = stream
        self.cursor = 0
        self.remaining_bits = 0
        self.dx = 0

    def bit(self) -> int:
        if self.remaining_bits == 0:
            if self.cursor + 2 > len(self.stream):
                raise AssertionError("rectangle model exhausted its controls")
            word = struct.unpack_from("<H", self.stream, self.cursor)[0]
            self.cursor += 2
            result = word >> 15
            self.dx = ((word << 1) | 1) & 0xFFFF
            self.remaining_bits = 15
            return result
        result = self.dx >> 15
        self.dx = (self.dx << 1) & 0xFFFF
        self.remaining_bits -= 1
        return result

    def byte(self) -> int:
        if self.cursor >= len(self.stream):
            raise AssertionError("rectangle model exhausted its descriptors")
        result = self.stream[self.cursor]
        self.cursor += 1
        return result


def model_frame(
    stream: bytes,
    staged: bytearray,
    staged_offset: int,
    framebuffer: bytearray,
    row_offset: int,
    row_width: int,
    rows_remaining: int,
    high_layout: bool,
    initial_ah: int,
) -> tuple[int, int, int, int, int, int, int, int]:
    reader = StreamReader(stream)
    staged_cursor = staged_offset
    destination = row_offset
    pending_length = 0
    al = 0
    ah = initial_ah
    row_remaining = row_width

    def read_value() -> int:
        nonlocal staged_cursor
        result = staged[staged_cursor]
        staged_cursor = (staged_cursor + 1) & 0xFFFF
        return result

    for _ in range(100_000):
        if reader.bit() == 0:
            al = read_value()
            length = 1
        else:
            al = read_value()
            ah = al
            if high_layout:
                if reader.bit() == 0:
                    length = 0
                elif reader.bit() == 0:
                    length = 2
                elif reader.bit() == 0:
                    length = 3
                else:
                    length = 4
            elif reader.bit() == 0:
                length = 2
            elif reader.bit() == 0:
                length = 3
            elif reader.bit() == 0:
                length = 4
            else:
                length = 0

            if length == 0:
                if pending_length > 4:
                    length = pending_length
                    pending_length = 0
                elif pending_length == 4:
                    length = reader.byte() + 20
                    pending_length = 0
                else:
                    descriptor = reader.byte()
                    high_nibble = descriptor >> 4
                    length = high_nibble + 4 if high_nibble else reader.byte() + 20
                    pending_length = (descriptor & 0x0F) + 4

        while length:
            chunk = min(length, row_remaining)
            if al:
                for _ in range(chunk):
                    framebuffer[destination] = al
                    destination = (destination + 1) & 0xFFFF
            else:
                destination = (destination + chunk) & 0xFFFF
            row_remaining -= chunk
            length -= chunk
            if row_remaining:
                continue
            rows_remaining = (rows_remaining - 1) & 0xFF
            if rows_remaining == 0:
                return (
                    reader.cursor,
                    staged_cursor,
                    destination,
                    pending_length,
                    reader.dx,
                    al,
                    ah,
                    row_offset,
                )
            row_offset = (row_offset + 320) & 0xFFFF
            destination = row_offset
            row_remaining = row_width
    raise AssertionError("rectangle model did not exhaust its rows")


def is_conditional_jump(cpu: Uc, address: int) -> bool:
    encoded = bytes(cpu.mem_read(address, 2))
    return 0x70 <= encoded[0] <= 0x7F or (
        encoded[0] == 0x0F and 0x80 <= encoded[1] <= 0x8F
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, Any],
    case_index: int,
) -> tuple[tuple[object, ...], set[tuple[int, int]]]:
    name = str(vector["name"])
    flags = int(vector["flags"])
    literal_bias = int(vector["literal_bias"])
    source_offset = int(vector["source_offset"])
    staging_offset = int(vector["staging_offset"])
    staged_values = bytes(int(value) for value in vector["staged_values"])
    extent = len(staged_values)
    coordinates = vector["coordinates"]
    coordinate_bytes = (
        b""
        if coordinates is None
        else struct.pack("<HH", int(coordinates[0]), int(coordinates[1]))
    )
    staging_stream = bytes.fromhex(vector["staging_stream_hex"])
    main_stream = bytes.fromhex(vector["main_stream_hex"])
    checksum = (0xAD - sum(struct.pack("<HHB", extent, extent, flags))) & 0xFF
    resource = b"".join(
        (
            struct.pack("<HHBB", extent, extent, flags, checksum),
            coordinate_bytes,
            staging_stream,
            main_stream,
        )
    )

    source_before = patterned_segment(case_index, 17, 7, 29)
    write_wrapped(source_before, source_offset, resource)
    staging_before = patterned_segment(case_index, 23, 11, 31)
    staging_expected = staging_before[:]
    write_wrapped(staging_expected, staging_offset, staged_values)
    framebuffer_before = patterned_segment(case_index, 37, 13, 19)
    framebuffer_expected = framebuffer_before[:]

    row_offset = int(vector["first_row_offset"])
    row_width = int(vector["row_width"])
    rows = int(vector["rows"])
    model = model_frame(
        main_stream,
        staging_expected,
        staging_offset,
        framebuffer_expected,
        row_offset,
        row_width,
        rows,
        vector["layout"] == "high",
        (0x1234 + case_index) >> 8,
    )
    (
        main_consumed,
        staged_result,
        destination_result,
        pending_length,
        bit_buffer,
        final_al,
        final_ah,
        final_row_offset,
    ) = model
    main_source_offset = (
        source_offset + 6 + len(coordinate_bytes) + len(staging_stream)
    ) & 0xFFFF
    main_source_result = (main_source_offset + main_consumed) & 0xFFFF
    assert main_source_result == int(vector["main_source_result_offset"]), name
    assert staged_result == int(vector["staged_result_offset"]), name
    assert destination_result == int(vector["destination_result_offset"]), name
    assert final_row_offset == int(vector["final_row_offset"]), name
    assert pending_length == int(vector["pending_length"]), name
    assert bit_buffer == int(vector["result_bit_buffer"]), name
    assert sum(a != b for a, b in zip(framebuffer_before, framebuffer_expected)) == int(
        vector["changed_pixels"]
    ), name

    game_before = patterned_segment(case_index, 41, 5, 17)
    write16(game_before, routine.mode_offset, 0x7100 + case_index)
    write16(game_before, routine.staging_segment_offset, STAGING // 16)
    write16(game_before, routine.raw_width_offset, int(vector["raw_width"]))
    write16(game_before, routine.raw_rows_offset, int(vector["raw_rows"]))
    write16(game_before, routine.vertical_offset, int(vector["vertical_offset"]))
    write16(game_before, routine.framebuffer_segment_offset, FRAMEBUFFER // 16)
    game_expected = game_before[:]
    write16(game_expected, routine.mode_offset, 3)

    stack_before = patterned_segment(case_index, 43, 0, 13, 0x51)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E50000 | source_offset,
        UC_X86_REG_EDI: 0xF6F60000 | staging_offset,
        UC_X86_REG_EBP: 0x97972468 + case_index,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: SOURCE // 16,
        UC_X86_REG_ES: 0x2C00,
        UC_X86_REG_FS: 0x1800,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    machine.mem_write(routine.code_bias_aliases[0], b"\x5a")
    machine.mem_write(routine.code_bias_aliases[1], b"\xa5")
    for base, contents in (
        (SOURCE, source_before),
        (STAGING, staging_before),
        (FRAMEBUFFER, framebuffer_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    helper_calls = 0
    main_entries = 0
    scanline_calls = 0
    reached_return = False
    previous_branch: int | None = None
    covered_edges: set[tuple[int, int]] = set()

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal helper_calls, main_entries, scanline_calls, reached_return
        nonlocal previous_branch
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if not (
            routine.helper_entry <= address < routine.helper_end
            or routine.entry <= address < routine.end
        ):
            raise AssertionError(f"{routine.name} {name}: escaped to {address:#x}")

        normalized = address - routine.entry
        if previous_branch is not None:
            covered_edges.add((previous_branch, normalized))
        previous_branch = normalized if is_conditional_jump(cpu, address) else None

        if address == routine.helper_entry:
            helper_calls += 1
            assert helper_calls == 1, (routine.name, name, "duplicate helper")
            helper_source = (source_offset + 6 + len(coordinate_bytes)) & 0xFFFF
            actual = (
                cpu.reg_read(UC_X86_REG_BX),
                cpu.reg_read(UC_X86_REG_SI),
                cpu.reg_read(UC_X86_REG_DI),
                cpu.reg_read(UC_X86_REG_BP),
                cpu.reg_read(UC_X86_REG_SP),
                cpu.reg_read(UC_X86_REG_DS),
                cpu.reg_read(UC_X86_REG_ES),
            )
            expected = (
                source_offset,
                helper_source,
                staging_offset,
                (staging_offset + extent) & 0xFFFF,
                0xFEF2,
                SOURCE // 16,
                STAGING // 16,
            )
            assert actual == expected, (routine.name, name, "helper ABI")
            saved_ax = (initial[UC_X86_REG_EAX] & 0xFF00) | flags
            frame = struct.unpack("<8H", cpu.mem_read(STACK + 0xFEF2, 16))
            expected_frame = (
                routine.helper_return,
                staging_offset,
                staging_offset,
                0 if coordinates is None else int(coordinates[0]),
                0 if coordinates is None else int(coordinates[1]),
                saved_ax,
                SOURCE // 16,
                RETURN_IP,
            )
            assert frame == expected_frame, (routine.name, name, "helper frame")
            aliases = bytes(cpu.mem_read(routine.code_bias_aliases[0], 1)) + bytes(
                cpu.mem_read(routine.code_bias_aliases[1], 1)
            )
            assert aliases == bytes((literal_bias, literal_bias)), (
                routine.name,
                name,
                "literal aliases",
            )
            for operand in routine.helper_bias_operands:
                cpu.mem_write(operand, bytes((literal_bias,)))

        expected_main_entry = (
            routine.high_main_entry
            if vector["layout"] == "high"
            else routine.low_main_entry
        )
        if address == expected_main_entry and main_entries == 0:
            main_entries += 1
            actual = (
                cpu.reg_read(UC_X86_REG_AX),
                cpu.reg_read(UC_X86_REG_BX),
                cpu.reg_read(UC_X86_REG_CX),
                cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                cpu.reg_read(UC_X86_REG_SI),
                cpu.reg_read(UC_X86_REG_DI),
                cpu.reg_read(UC_X86_REG_BP),
                cpu.reg_read(UC_X86_REG_SP),
                cpu.reg_read(UC_X86_REG_DS),
                cpu.reg_read(UC_X86_REG_ES),
                cpu.reg_read(UC_X86_REG_FS),
            )
            expected = (
                (initial[UC_X86_REG_EAX] & 0xFF00) | flags,
                main_source_offset,
                row_width,
                0x8000,
                staging_offset,
                row_offset,
                0xFEFC,
                0xFEEC,
                STAGING // 16,
                FRAMEBUFFER // 16,
                SOURCE // 16,
            )
            assert actual == expected, (routine.name, name, "main ABI")
            locals_words = struct.unpack("<5H", cpu.mem_read(STACK + 0xFEF2, 10))
            assert locals_words[0] == row_width, (routine.name, name, "row width")
            assert locals_words[2] & 0xFF == rows, (routine.name, name, "rows")
            assert locals_words[4] == 0, (routine.name, name, "pending length")
        if address == routine.scanline_helper:
            scanline_calls += 1

    machine.hook_add(UC_HOOK_CODE, instruction)
    try:
        machine.emu_start(routine.entry, 0, count=250_000)
    except UcError as error:
        raise RuntimeError(
            f"{routine.name} {name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return and helper_calls == 1 and main_entries == 1, (
        routine.name,
        name,
        "call topology",
    )
    assert scanline_calls == int(vector["scanline_helper_calls"]), (
        routine.name,
        name,
        "scanline calls",
    )

    expected_memory = bytearray(MACHINE_SIZE)
    expected_memory[: len(executable)] = executable
    for alias in routine.code_bias_aliases:
        expected_memory[alias] = literal_bias
    for operand in routine.helper_bias_operands:
        expected_memory[operand] = literal_bias
    for base, contents in (
        (SOURCE, source_before),
        (STAGING, staging_expected),
        (FRAMEBUFFER, framebuffer_expected),
        (GAME, game_expected),
        (STACK, stack_before),
    ):
        expected_memory[base : base + SEGMENT_SIZE] = contents
    memory_after = bytes(machine.mem_read(0, MACHINE_SIZE))
    transient_start = STACK + STACK_TRANSIENT_START
    transient_end = STACK + STACK_POINTER
    assert memory_after[:transient_start] == bytes(expected_memory[:transient_start]), (
        routine.name,
        name,
        "memory before transient stack",
    )
    assert memory_after[transient_end:] == bytes(expected_memory[transient_end:]), (
        routine.name,
        name,
        "memory after transient stack",
    )

    expected = dict(initial)
    expected[UC_X86_REG_EAX] = (
        (expected[UC_X86_REG_EAX] & 0xFFFF0000) | final_al | (final_ah << 8)
    )
    expected[UC_X86_REG_EBX] = (
        expected[UC_X86_REG_EBX] & 0xFFFF0000
    ) | main_source_result
    expected[UC_X86_REG_ECX] &= 0xFFFF0000
    expected[UC_X86_REG_EDX] = (expected[UC_X86_REG_EDX] & 0xFFFF0000) | bit_buffer
    expected[UC_X86_REG_ESI] = (expected[UC_X86_REG_ESI] & 0xFFFF0000) | staged_result
    expected[UC_X86_REG_EDI] = (
        expected[UC_X86_REG_EDI] & 0xFFFF0000
    ) | destination_result
    expected[UC_X86_REG_EBP] = (expected[UC_X86_REG_EBP] & 0xFFFF0000) | (
        (staging_offset + extent) & 0xFFFF
    )
    expected[UC_X86_REG_SP] = STACK_POINTER + 2
    expected[UC_X86_REG_DS] = SOURCE // 16
    expected[UC_X86_REG_ES] = FRAMEBUFFER // 16
    expected[UC_X86_REG_FS] = SOURCE // 16
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {register: expected[register] for register in REGISTERS}, (
        routine.name,
        name,
        "registers",
    )
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP
    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(result_flags & mask) for flag, mask in FLAG_MASKS.items()
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, "flags")
    assert result_flags & ~ARITHMETIC_FLAG_MASK == 0x0202, (
        routine.name,
        name,
        result_flags,
    )

    canonical = (
        tuple(registers.items()),
        result_flags,
        bytes(machine.mem_read(SOURCE, SEGMENT_SIZE)),
        bytes(machine.mem_read(STAGING, SEGMENT_SIZE)),
        bytes(machine.mem_read(FRAMEBUFFER, SEGMENT_SIZE)),
        bytes(machine.mem_read(STACK, STACK_TRANSIENT_START)),
        bytes(machine.mem_read(STACK + STACK_POINTER, SEGMENT_SIZE - STACK_POINTER)),
        scanline_calls,
    )
    return canonical, covered_edges


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
        "--commander-fixture",
        type=Path,
        default=Path(__file__).with_name("oracle_vectors") / "func_ab25_natural.json",
    )
    args = parser.parse_args()

    commander = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel = verify_executable(args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL)
    cases = load_cases(args.commander_fixture)
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(cases):
        commander_result, edges = execute(commander, COMMANDER, vector, case_index)
        commander_edges.update(edges)
        sequel_result, edges = execute(sequel, SEQUEL, vector, case_index)
        sequel_edges.update(edges)
        assert sequel_result == commander_result, vector["name"]
        rows.append(vector)
    assert sequel_edges == commander_edges, (
        sorted(sequel_edges),
        sorted(commander_edges),
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    branch_sites = {edge[0] for edge in sequel_edges}
    complete_sites = sum(
        sum(1 for edge in sequel_edges if edge[0] == site) == 2 for site in branch_sites
    )
    print(
        f"verified {len(rows)} BBB rectangle decoder cases, "
        f"{len(sequel_edges)} edges, and {complete_sites}/{len(branch_sites)} "
        "observed branch sites with both outcomes"
    )


if __name__ == "__main__":
    main()
