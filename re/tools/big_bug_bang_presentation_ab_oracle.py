#!/usr/bin/env python3
"""Compare BBB's AB presentation decoder with Commander Blood."""

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
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
MACHINE_SIZE = 0x90000
SOURCE = 0x20000
DESTINATION = 0x38000
GAME = 0x50000
STACK = 0x70000
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
COMMANDER_FIXTURE_SHA256 = (
    "fa4a5804716b84492c884d9a1153dfd807ef5a28094c408b5a0dfc2012f42941"
)

BRANCHES = {
    0x3B: (0x3D, 0x42),
    0x3D: (0x3F, 0x4A),
    0x48: (0x4A, 0x3F),
    0x4E: (0x50, 0x56),
    0x56: (0x58, 0x86),
    0x5A: (0x5C, 0x62),
    0x66: (0x68, 0x6E),
    0x95: (0x97, 0x73),
    0xA0: (0xA2, 0x73),
}
EXPECTED_EDGES = {
    (branch, target) for branch, targets in BRANCHES.items() for target in targets
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
)
FLAG_MASKS = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
ARITHMETIC_FLAG_MASK = sum(FLAG_MASKS.values())


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    mode_offset: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    "Commander Blood",
    0x600,
    (0xA867, 0xA914),
    "68f21ed3bcbe8308592cb8c1c2773657b5066df70b3a5ce647b352863e477d56",
    0x0AA0,
)
SEQUEL = Routine(
    "Big Bug Bang",
    0x800,
    (0xC051, 0xC0FE),
    "87eaf6aa34a8a48e7b6359cc425be67b80523b06c632a2b84fa5fe7d9dae58c4",
    0x0C98,
)


def seeded_segment(case_index: int, multiplier: int, page_step: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * page_step + case_index * 31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def decode_hex(encoded: str) -> bytes:
    return bytes.fromhex(encoded)


def encode_tokens(
    tokens: list[list[object]],
) -> tuple[bytes, bytes, list[int], int]:
    stream = bytearray((0x31, 0x42, 0x53, 0x64, 0x75, 0x86))
    output = bytearray()
    control_words: list[int] = []
    control_position = -1
    control_bit_count = 16

    def emit_bit(value: int) -> None:
        nonlocal control_position, control_bit_count
        if control_bit_count == 16:
            control_position = len(stream)
            stream.extend(b"\x00\x00")
            control_words.append(0)
            control_bit_count = 0
        if value:
            control_words[-1] |= 1 << control_bit_count
            struct.pack_into("<H", stream, control_position, control_words[-1])
        control_bit_count += 1

    def append_match(displacement: int, length: int) -> None:
        copy_index = len(output) + displacement
        if copy_index < 0:
            raise AssertionError("invalid AB oracle match")
        for _ in range(length):
            output.append(output[copy_index])
            copy_index += 1

    for token in tokens:
        kind = str(token[0])
        if kind == "literal":
            value = int(token[1])
            emit_bit(1)
            stream.append(value)
            output.append(value)
        elif kind == "short":
            displacement = int(token[1])
            length = int(token[2])
            assert -256 <= displacement <= -1 and 2 <= length <= 5
            length_code = length - 2
            emit_bit(0)
            emit_bit(0)
            emit_bit((length_code >> 1) & 1)
            emit_bit(length_code & 1)
            stream.append(displacement & 0xFF)
            append_match(displacement, length)
        elif kind == "long":
            displacement = int(token[1])
            length = int(token[2])
            assert -8192 <= displacement <= -1 and 3 <= length <= 257
            emit_bit(0)
            emit_bit(1)
            if length <= 9:
                stream.extend(
                    struct.pack("<H", ((displacement & 0x1FFF) << 3) | (length - 2))
                )
            else:
                stream.extend(struct.pack("<H", (displacement & 0x1FFF) << 3))
                stream.append(length - 2)
            append_match(displacement, length)
        elif kind == "end":
            emit_bit(0)
            emit_bit(1)
            stream.extend(b"\x00\x00\x00")
        else:
            raise AssertionError(f"unknown AB oracle token {kind}")

    assert tokens and tokens[-1][0] == "end"
    result_bit_buffer = (0x8000 | (control_words[-1] >> 1)) >> (control_bit_count - 1)
    return bytes(stream), bytes(output), control_words, result_bit_buffer


def sub16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
    }


def synthetic_case(
    name: str,
    literal_count: int,
    source_offset: int,
    destination_offset: int,
    mode_before: int,
) -> dict[str, object]:
    tokens: list[list[object]] = [
        ["literal", 0x40 + index] for index in range(literal_count)
    ]
    tokens.extend((["short", -1, 2], ["end"]))
    stream, output, control_words, result_bit_buffer = encode_tokens(tokens)
    return {
        "name": name,
        "tokens": tokens,
        "compressed_stream_hex": stream.hex(),
        "control_words": control_words,
        "decoded_hex": output.hex(),
        "source_offset": source_offset,
        "source_result_offset": (source_offset + len(stream)) & 0xFFFF,
        "destination_offset": destination_offset,
        "decoded_length": len(output),
        "result_bit_buffer": result_bit_buffer,
        "mode_before": mode_before,
        "mode_after": 1,
        "defined_flags": sub16_flags(
            (destination_offset + len(output)) & 0xFFFF, destination_offset
        ),
    }


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[slice(*routine.span)]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(f"{routine.name} AB decoder body changed: {body_digest}")
    return executable


def load_cases(path: Path) -> list[dict[str, object]]:
    encoded = path.read_bytes()
    digest = hashlib.sha256(encoded).hexdigest()
    if digest != COMMANDER_FIXTURE_SHA256:
        raise SystemExit(f"Commander AB fixture changed: {digest}")
    cases = json.loads(encoded)
    if len(cases) != 10:
        raise SystemExit(f"expected 10 Commander AB cases, found {len(cases)}")
    for case in cases:
        stream, output, control_words, result_bit_buffer = encode_tokens(case["tokens"])
        assert stream.hex() == case["compressed_stream_hex"], case["name"]
        assert output.hex() == case["decoded_hex"], case["name"]
        assert control_words == case["control_words"], case["name"]
        assert result_bit_buffer == case["result_bit_buffer"], case["name"]
    cases.extend(
        (
            synthetic_case("second_bit_refill", 15, 0x3000, 0x5000, 0x8101),
            synthetic_case("short_first_length_refill", 14, 0x4000, 0x6000, 0x8202),
            synthetic_case("short_second_length_refill", 13, 0x5000, 0x7000, 0x8303),
        )
    )
    return cases


def execute(
    executable: bytes,
    routine: Routine,
    case: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], tuple[object, ...], set[tuple[int, int]]]:
    name = str(case["name"])
    source_offset = int(case["source_offset"])
    destination_offset = int(case["destination_offset"])
    stream = decode_hex(str(case["compressed_stream_hex"]))
    decoded = decode_hex(str(case["decoded_hex"]))

    source_before = seeded_segment(case_index, 17, 7)
    destination_before = seeded_segment(case_index, 23, 11)
    game_before = seeded_segment(case_index, 37, 13)
    stack_before = seeded_segment(case_index, 41, 17)
    for index, value in enumerate(stream):
        source_before[(source_offset + index) & 0xFFFF] = value
    write16(game_before, routine.mode_offset, int(case["mode_before"]))
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B20000,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E50000 | source_offset,
        UC_X86_REG_EDI: 0xF6F60000 | destination_offset,
        UC_X86_REG_EBP: 0x97970000 | ((0x2468 + case_index) & 0xFFFF),
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: SOURCE // 16,
        UC_X86_REG_ES: DESTINATION // 16,
        UC_X86_REG_FS: 0x1800,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    module = executable[routine.header_size :]
    machine.mem_write(0, module)
    for base, contents in (
        (SOURCE, source_before),
        (DESTINATION, destination_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    previous: int | None = None
    covered_edges: set[tuple[int, int]] = set()

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if routine.span[0] <= file_address < routine.span[1]:
            normalized = file_address - routine.span[0]
            if previous in BRANCHES:
                covered_edges.add((previous, normalized))
            previous = normalized
        else:
            previous = None

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=50_000)
    assert reached_return, (routine.name, name)

    destination_expected = bytearray(destination_before)
    for index, value in enumerate(decoded):
        destination_expected[(destination_offset + index) & 0xFFFF] = value
    game_expected = bytearray(game_before)
    write16(game_expected, routine.mode_offset, 1)
    stack_expected = bytearray(stack_before)
    write16(stack_expected, STACK_POINTER - 2, initial[UC_X86_REG_ECX])
    write16(stack_expected, STACK_POINTER - 4, initial[UC_X86_REG_EDI])
    write16(stack_expected, STACK_POINTER - 6, initial[UC_X86_REG_DS])

    expected_memory = bytearray(MACHINE_SIZE)
    expected_memory[: len(module)] = module
    for base, contents in (
        (SOURCE, source_before),
        (DESTINATION, destination_expected),
        (GAME, game_expected),
        (STACK, stack_expected),
    ):
        expected_memory[base : base + SEGMENT_SIZE] = contents
    memory_after = bytes(machine.mem_read(0, MACHINE_SIZE))
    assert memory_after == bytes(expected_memory), (routine.name, name, "memory")

    expected = dict(initial)
    expected[UC_X86_REG_EAX] = (expected[UC_X86_REG_EAX] & 0xFFFF0000) | 0xE000
    expected[UC_X86_REG_EBX] = (expected[UC_X86_REG_EBX] & 0xFFFF0000) | 0xE000
    expected[UC_X86_REG_ECX] = (expected[UC_X86_REG_ECX] & 0xFFFF0000) | int(
        case["decoded_length"]
    )
    if any(str(token[0]) in ("short", "long") for token in case["tokens"]):
        expected[UC_X86_REG_EDX] = (expected[UC_X86_REG_EDX] & 0xFFFF0000) | (
            DESTINATION // 16
        )
    expected[UC_X86_REG_ESI] = (expected[UC_X86_REG_ESI] & 0xFFFF0000) | int(
        case["source_result_offset"]
    )
    expected[UC_X86_REG_EBP] = (expected[UC_X86_REG_EBP] & 0xFFFF0000) | int(
        case["result_bit_buffer"]
    )
    expected[UC_X86_REG_SP] = STACK_POINTER + 2

    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {register: expected[register] for register in REGISTERS}, (
        routine.name,
        name,
        "registers",
    )
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {flag: bool(flags & mask) for flag, mask in FLAG_MASKS.items()}
    assert defined_flags == case["defined_flags"], (routine.name, name, "flags")
    assert flags & ~ARITHMETIC_FLAG_MASK == 0x0202, (routine.name, name, flags)

    canonical = (
        tuple(registers.items()),
        flags,
        bytes(machine.mem_read(SOURCE, SEGMENT_SIZE)),
        bytes(machine.mem_read(DESTINATION, SEGMENT_SIZE)),
        bytes(machine.mem_read(STACK, SEGMENT_SIZE)),
        struct.unpack("<H", machine.mem_read(GAME + routine.mode_offset, 2))[0],
    )
    return case, canonical, covered_edges


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
        default=Path(__file__).with_name("oracle_vectors") / "func_a867_natural.json",
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
    for case_index, case in enumerate(cases):
        commander_row, commander_result, edges = execute(
            commander, COMMANDER, case, case_index
        )
        commander_edges.update(edges)
        sequel_row, sequel_result, edges = execute(sequel, SEQUEL, case, case_index)
        sequel_edges.update(edges)
        assert sequel_result == commander_result, case["name"]
        assert sequel_row == commander_row, case["name"]
        rows.append(sequel_row)

    assert commander_edges == EXPECTED_EDGES, (
        sorted(commander_edges),
        sorted(EXPECTED_EDGES),
    )
    assert sequel_edges == EXPECTED_EDGES, (
        sorted(sequel_edges),
        sorted(EXPECTED_EDGES),
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB AB decoder cases and "
        f"{len(sequel_edges)} reachable edges"
    )


if __name__ == "__main__":
    main()
