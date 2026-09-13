#!/usr/bin/env python3
"""Compare Big Bug Bang's ship projection matrix with Commander Blood."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
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
FS_DATA = 0x70000
STACK = 0x80000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("c33c5aa59669")
TABLE_SAMPLE_COUNT = 181
TABLE_SIZE = TABLE_SAMPLE_COUNT * 4
WORKSPACE_SIZE = 66

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
    angles: int
    workspace: int
    matrix: int
    angle_table: int


COMMANDER = Routine(
    name="Commander Blood",
    span=(0x98B9, 0x9A10),
    body_sha256="ee9cefae7bb3c3bcc0acfa72dd6f6f3731e166b91b2e15c3d3e62eee82653bb5",
    angles=0x2F6D,
    workspace=0x2F7D,
    matrix=0x2F95,
    angle_table=0x4F45,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    span=(0xB058, 0xB1AF),
    body_sha256="a28c096643598d99675575a4d398ccab58c7109f64d77f95837dd4c9611f21af",
    angles=0x333D,
    workspace=0x334D,
    matrix=0x3365,
    angle_table=0x5315,
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

STORE_OFFSETS = (0x98, 0xAA, 0xCD, 0xF0, 0x102, 0x125, 0x134, 0x13A, 0x148)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def signed32(value: int) -> int:
    value &= 0xFFFFFFFF
    return value - 0x100000000 if value & 0x80000000 else value


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def make_table(vector: dict[str, object]) -> bytearray:
    table = bytearray(TABLE_SIZE)
    for index in range(TABLE_SAMPLE_COUNT):
        cosine = ((index * 193 + 17) & 0xFFFF) - 0x8000
        sine = ((index * 389 + 91) & 0xFFFF) - 0x8000
        struct.pack_into("<hh", table, index * 4, cosine, sine)
    for angle, pair in zip(
        vector["angles_a_b_c"], vector["table_pairs_a_b_c"], strict=True
    ):
        struct.pack_into("<hh", table, int(angle) * 4, *(int(value) for value in pair))
    return table


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
]:
    table = make_table(vector)
    angle_a, angle_b, angle_c = (int(value) for value in vector["angles_a_b_c"])
    angle_words = struct.pack("<HHH", angle_b, angle_c, angle_a)
    terms = [int(value) for value in vector["doubled_terms_b_c_a"]]
    matrix = [int(value) for value in vector["matrix"]]
    expected_work = struct.pack("<6i9i", *(terms + matrix))
    tail = bytes((0xA0 + index * 7) & 0xFF for index in range(6))
    work_before = bytes((0x31 + index * 13) & 0xFF for index in range(60)) + tail

    game_before = seeded_segment(case_index, 31, 0x41)
    game_before[routine.angles : routine.angles + 6] = angle_words
    game_before[routine.workspace : routine.workspace + WORKSPACE_SIZE] = work_before
    game_before[routine.angle_table : routine.angle_table + TABLE_SIZE] = bytes(
        (0x4B + index * 17) & 0xFF for index in range(TABLE_SIZE)
    )
    game_expected = bytearray(game_before)
    game_expected[routine.workspace : routine.workspace + WORKSPACE_SIZE] = (
        expected_work + tail
    )

    data_before = seeded_segment(case_index, 17, 0x63)
    data_before[routine.angles : routine.angles + 6] = bytes.fromhex("5aa596698778")
    data_before[routine.workspace : routine.workspace + WORKSPACE_SIZE] = bytes(
        (0x6D + index * 5) & 0xFF for index in range(WORKSPACE_SIZE)
    )
    data_before[routine.angle_table : routine.angle_table + TABLE_SIZE] = bytes(
        (0xD3 + index * 11) & 0xFF for index in range(TABLE_SIZE)
    )

    extra_before = seeded_segment(case_index, 11, 0x85)
    extra_before[routine.angles : routine.angles + 6] = bytes.fromhex("5aa596698778")[
        ::-1
    ]
    extra_before[routine.workspace : routine.workspace + WORKSPACE_SIZE] = bytes(
        (0x6D + index * 5) & 0xFF for index in range(WORKSPACE_SIZE)
    )[::-1]
    extra_before[routine.angle_table : routine.angle_table + TABLE_SIZE] = bytes(
        (0xD3 + index * 11) & 0xFF for index in range(TABLE_SIZE)
    )[::-1]

    stack_before = seeded_segment(case_index, 7, 0xA7)
    stack_before[routine.angle_table : routine.angle_table + TABLE_SIZE] = table
    stack_before[STACK_POINTER : STACK_POINTER + 4] = struct.pack("<HH", RETURN_IP, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    return (
        game_before,
        game_expected,
        data_before,
        extra_before,
        stack_before,
        table,
        expected_work + tail,
    )


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], dict[int, int], int, bytes, list[tuple[object, ...]]]:
    name = str(vector["name"])
    (
        game_before,
        game_expected,
        data_before,
        extra_before,
        stack_before,
        table,
        expected_work,
    ) = initialize_memory(routine, vector, case_index)
    fs_before = bytes(seeded_segment(case_index, 5, 0xC9))
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

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    phases: list[dict[str, object]] = []
    writes: list[tuple[object, ...]] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return
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
        offset = file_offset - routine.span[0]
        if offset == 0x12:
            phases.append(
                {
                    "offset": offset,
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        elif offset == 0x71:
            phases.append(
                {
                    "offset": offset,
                    "terms": list(
                        struct.unpack("<6i", cpu.mem_read(GAME + routine.workspace, 24))
                    ),
                }
            )
        elif offset in STORE_OFFSETS:
            phases.append(
                {
                    "offset": offset,
                    "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                    "eax": signed32(cpu.reg_read(UC_X86_REG_EAX)),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        elif offset == 0x14A:
            phases.append(
                {"offset": offset, "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF}
            )

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, value: int, _context
    ) -> None:
        if GAME <= address < address + size <= GAME + SEGMENT_SIZE:
            offset = address - GAME
            assert routine.workspace <= offset < routine.workspace + 60
            writes.append(("game", offset - routine.workspace, size, value))
        elif STACK <= address < address + size <= STACK + SEGMENT_SIZE:
            offset = address - STACK
            assert 0xFEE8 <= offset < STACK_POINTER
            writes.append(("stack", offset, size, value))
        else:
            raise AssertionError((routine.name, name, hex(address), size, value))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(image_address(routine.span[0]), 0, count=1024)
    assert reached_return
    assert bytes(machine.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    actual_game = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    assert actual_game == bytes(game_expected), (routine.name, name)
    actual_stack = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert actual_stack[routine.angle_table : routine.angle_table + TABLE_SIZE] == table
    assert actual_stack[STACK_POINTER + 4 : STACK_POINTER + 10] == STACK_SENTINEL
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)

    expected_phases: list[dict[str, object]] = [
        {"offset": 0x12, "ds": GAME // 16, "es": GAME // 16},
        {"offset": 0x71, "terms": list(vector["doubled_terms_b_c_a"])},
    ]
    expected_phases.extend(
        {
            "offset": offset,
            "di": routine.matrix + index * 4,
            "eax": int(vector["matrix"][index]),
            "es": GAME // 16,
        }
        for index, offset in enumerate(STORE_OFFSETS)
    )
    expected_phases.append({"offset": 0x14A, "di": routine.matrix + 36})
    assert phases == expected_phases, (routine.name, name, phases)

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = 0xFF04
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        "cf": bool(flags & 1),
        "pf": bool(flags & 4),
        "zf": bool(flags & 0x40),
        "sf": bool(flags & 0x80),
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, flags)

    angles_raw = struct.unpack("<HHH", actual_game[routine.angles : routine.angles + 6])
    angles = [angles_raw[2], angles_raw[0], angles_raw[1]]
    pairs = [list(struct.unpack_from("<hh", table, angle * 4)) for angle in angles]
    terms_and_matrix = struct.unpack(
        "<6i9i", actual_game[routine.workspace : routine.workspace + 60]
    )
    row = {
        "name": name,
        "angles_a_b_c": angles,
        "table_pairs_a_b_c": pairs,
        "doubled_terms_b_c_a": list(terms_and_matrix[:6]),
        "matrix": list(terms_and_matrix[6:]),
        "workspace_sha256": hashlib.sha256(expected_work).hexdigest(),
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    normalized_stack = bytearray(actual_stack)
    stack_seed = seeded_segment(case_index, 7, 0xA7)
    normalized_stack[routine.angle_table : routine.angle_table + TABLE_SIZE] = (
        stack_seed[routine.angle_table : routine.angle_table + TABLE_SIZE]
    )
    normalized_stack[COMMANDER.angle_table : COMMANDER.angle_table + TABLE_SIZE] = table
    return row, registers, flags, bytes(normalized_stack), writes


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
        default=Path(__file__).parent / "oracle_vectors/func_98b9_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    rows = []
    for index, vector in enumerate(vectors):
        commander_result = execute(commander_executable, COMMANDER, vector, index)
        sequel_result = execute(sequel_executable, SEQUEL, vector, index)
        (
            commander_row,
            commander_registers,
            commander_flags,
            commander_stack,
            commander_writes,
        ) = commander_result
        sequel_row, sequel_registers, sequel_flags, sequel_stack, sequel_writes = (
            sequel_result
        )
        assert commander_row == vector, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        assert sequel_flags == commander_flags, vector["name"]
        assert sequel_stack == commander_stack, vector["name"]
        assert sequel_writes == commander_writes, vector["name"]
        rows.append(sequel_row)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} BBB ship-projection matrix cases")


if __name__ == "__main__":
    main()
