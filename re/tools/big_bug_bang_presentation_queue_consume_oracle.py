#!/usr/bin/env python3
"""Compare Big Bug Bang's queue consumer with Commander Blood."""

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

SEGMENT_SIZE = 0x10000
TAIL_REGION_SIZE = SEGMENT_SIZE + 1
DATA = 0x20000
TAIL = 0x40000
EXTRA = 0x60000
FS_DATA = 0x70000
GAME = 0x80000
STACK = 0xA0000
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
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    tail: int
    byte_count: int
    buffer_end: int
    sequence: int
    read_index: int
    read_limit: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA3D0, 0xA40B),
    body_sha256="4c199e1c8299b742a5d4150cc4185349f9c2b836c9c526973c792330dd904f7e",
    tail=0x0D90,
    byte_count=0x0D9A,
    buffer_end=0x5233,
    sequence=0x131C,
    read_index=0x0D60,
    read_limit=0x0D64,
    branches={
        0xA3DC: (0xA3DE, 0xA3E4),
        0xA3E2: (0xA3E4, 0xA3EC),
        0xA3FC: (0xA3FE, 0xA407),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBBBA, 0xBBF5),
    body_sha256="cd850756d06df775771c668ca74ff5435985aae66656d625b540100f8824599c",
    tail=0x0FDE,
    byte_count=0x0FE8,
    buffer_end=0x5603,
    sequence=0x156A,
    read_index=0x0FAE,
    read_limit=0x0FB2,
    branches={
        0xBBC6: (0xBBC8, 0xBBCE),
        0xBBCC: (0xBBCE, 0xBBD6),
        0xBBE6: (0xBBE8, 0xBBF1),
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


def seeded_region(size: int, case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(size)
    )


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


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


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes, tuple]:
    name = str(vector["name"])
    tail = int(vector["tail"])
    entry_bytes = int(vector["entry_bytes"])
    byte_count = int(vector["byte_count"])
    buffer_end = int(vector["buffer_end"])
    sequence = int(vector["sequence"])
    read_index = int(vector["read_index"])
    read_limit = int(vector["read_limit"])

    data_before = seeded_region(SEGMENT_SIZE, case_index, 17, 0x31)
    write16(data_before, routine.tail, tail)
    write16(data_before, routine.tail + 2, TAIL // 16)
    write16(data_before, routine.byte_count, byte_count)
    write16(data_before, routine.buffer_end, buffer_end)
    write16(data_before, routine.sequence, sequence)
    write16(data_before, routine.read_index, read_index)
    write16(data_before, routine.read_limit, read_limit)

    tail_before = seeded_region(TAIL_REGION_SIZE, case_index, 11, 0x53)
    write16(tail_before, tail, entry_bytes)
    extra_before = seeded_region(SEGMENT_SIZE, case_index, 7, 0x75)
    fs_before = seeded_region(SEGMENT_SIZE, case_index, 5, 0x97)
    game_before = seeded_region(SEGMENT_SIZE, case_index, 3, 0xB9)
    for offset, value in (
        (routine.tail, 0x1111),
        (routine.tail + 2, 0x2222),
        (routine.byte_count, 0x3333),
        (routine.buffer_end, 0x4444),
        (routine.sequence, 0x5555),
        (routine.read_index, 0x6666),
        (routine.read_limit, 0x7777),
    ):
        write16(game_before, offset, value)

    stack_before = seeded_region(SEGMENT_SIZE, case_index, 19, 0xDB)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A51111,
        UC_X86_REG_EBX: 0xB6B62222,
        UC_X86_REG_ECX: 0xC7C73333,
        UC_X86_REG_EDX: 0xD8D84444,
        UC_X86_REG_ESI: 0xE9E95555,
        UC_X86_REG_EDI: 0xFAFA6666,
        UC_X86_REG_EBP: 0xABCD7777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xC0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(TAIL, bytes(tail_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    previous: int | None = None
    writes: list[tuple[str, int, int]] = []
    field_names = {
        routine.tail: "tail",
        routine.byte_count: "byte_count",
        routine.sequence: "sequence",
        routine.read_index: "read_index",
        routine.read_limit: "read_limit",
    }

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
        if previous is not None and previous + routine.span[0] in routine.branches:
            covered_edges.add((previous, normalized))
        previous = normalized

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, value: int, _context
    ) -> None:
        offset = address - DATA
        assert offset in field_names, (routine.name, name, hex(address), size, value)
        writes.append((field_names[offset], size, value & ((1 << (size * 8)) - 1)))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=100)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)

    expected_data = bytearray(data_before)
    write16(expected_data, routine.tail, int(vector["result_tail"]))
    write16(expected_data, routine.byte_count, int(vector["result_byte_count"]))
    write16(expected_data, routine.sequence, int(vector["result_sequence"]))
    write16(expected_data, routine.read_index, int(vector["result_read_index"]))
    write16(expected_data, routine.read_limit, int(vector["result_read_limit"]))
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    assert data_after == bytes(expected_data), (routine.name, name, "data")
    assert bytes(machine.mem_read(TAIL, TAIL_REGION_SIZE)) == bytes(tail_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after == bytes(stack_before)

    expected_writes = [("byte_count", 2, int(vector["result_byte_count"]))]
    if bool(vector["wrapped"]):
        expected_writes.append(("tail", 2, int(vector["result_tail"])))
    expected_writes.append(("tail", 2, int(vector["result_tail"])))
    expected_writes.append(("sequence", 2, int(vector["result_sequence"])))
    if int(vector["result_read_limit"]) != read_limit:
        expected_writes.append(("read_limit", 2, int(vector["result_read_limit"])))
    expected_writes.append(("read_index", 2, int(vector["result_read_index"])))
    assert writes == expected_writes, (routine.name, name, writes, expected_writes)

    result_flags = machine.reg_read(UC_X86_REG_EFLAGS)
    legacy_row = {
        "name": name,
        "tail": tail,
        "entry_bytes": entry_bytes,
        "byte_count": byte_count,
        "buffer_end": buffer_end,
        "sequence": sequence,
        "read_index": read_index,
        "read_limit": read_limit,
        "wrapped": bool(vector["wrapped"]),
        "result_tail": read16(data_after, routine.tail),
        "result_byte_count": read16(data_after, routine.byte_count),
        "result_sequence": read16(data_after, routine.sequence),
        "result_read_index": read16(data_after, routine.read_index),
        "result_read_limit": read16(data_after, routine.read_limit),
    }
    assert legacy_row == vector, (routine.name, name, legacy_row, vector)

    result_si = (tail + 2 + entry_bytes) & 0xFFFF
    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | int(
        vector["result_read_index"]
    )
    expected_registers[UC_X86_REG_ESI] = (initial[UC_X86_REG_ESI] & 0xFFFF0000) | result_si
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    expected_registers[UC_X86_REG_ES] = TAIL // 16
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers)

    row = {
        **legacy_row,
        "result_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
        "result_si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        "result_es": machine.reg_read(UC_X86_REG_ES),
        "result_flags": result_flags & 0xFFFF,
        "writes": [[field, value] for field, _size, value in writes],
    }
    state = struct.pack(
        "<6H",
        read16(data_after, routine.tail),
        read16(data_after, routine.byte_count),
        read16(data_after, routine.sequence),
        read16(data_after, routine.read_index),
        read16(data_after, routine.read_limit),
        read16(data_after, routine.tail + 2),
    )
    return row, registers, result_flags, stack_after, state, tuple(writes)


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
        default=Path(__file__).parent / "oracle_vectors/func_a3d0_natural.json",
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
        commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        rows.append(sequel_result[0])

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
        f"verified {len(rows)} BBB presentation queue-consume cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
