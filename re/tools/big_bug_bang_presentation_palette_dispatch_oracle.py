#!/usr/bin/env python3
"""Compare BBB's queued palette dispatch with Commander Blood."""

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
DATA = 0x20000
EXTRA = 0x70000
FS_DATA = 0x80000
GAME = 0xA0000
STACK = 0xB0000
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
    parser: int
    head: int
    head_segment: int
    payload_offset: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA778, 0xA784),
    body_sha256="eda7d9fd962cfc6acd23ec76178fe2308e8f4e6d5a28c1cd01d0acd74fc9b92d",
    parser=0xA0C3,
    head=0x0D8C,
    head_segment=0x0D8E,
    payload_offset=0x0D9E,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBF62, 0xBF6E),
    body_sha256="7d8135813de7ebf0664028ca8f187a29231ec5f2fa273a6718d3f8dd5d0d4e33",
    parser=0xB8A6,
    head=0x0FDA,
    head_segment=0x0FDC,
    payload_offset=0x0FEC,
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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def machine16(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def with_low16(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def near_return(machine: Uc) -> None:
    stack_pointer = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine16(machine, STACK + stack_pointer)
    machine.reg_write(UC_X86_REG_SP, (stack_pointer + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    write16(
        result,
        STACK_POINTER - 2,
        COMMANDER.image_address(COMMANDER.span[0] + 11),
    )
    return bytes(result)


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body = executable[slice(*routine.span)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} span {routine.span[0]:#x}..{routine.span[1]:#x} "
            f"changed: {body_digest}"
        )
    assert len(body) == 12 and body[-1] == 0xC3
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
) -> tuple[dict[str, object], tuple[object, ...]]:
    name = str(vector["name"])
    head_offset = int(vector["head_offset_ignored"])
    buffer_segment = int(vector["buffer_segment"])
    payload_offset = int(vector["payload_offset"])

    game_before = seeded_segment(case_index, 17, 0x13)
    data_before = seeded_segment(case_index, 19, 0x25)
    extra_before = seeded_segment(case_index, 23, 0x37)
    fs_before = seeded_segment(case_index, 29, 0x49)
    stack_before = seeded_segment(case_index, 31, 0x5B)
    write16(game_before, routine.head, head_offset)
    write16(game_before, routine.head_segment, buffer_segment)
    write16(game_before, routine.payload_offset, payload_offset)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | case_index,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E55555,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: DATA // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0A93,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xC0000)
    module = executable[routine.header_size :]
    machine.mem_write(0, module)
    for base, contents in (
        (DATA, data_before),
        (EXTRA, extra_before),
        (FS_DATA, fs_before),
        (GAME, game_before),
        (STACK, stack_before),
    ):
        machine.mem_write(base, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, object]] = []
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_address = address + routine.header_size
        if file_address == routine.parser:
            stack_pointer = cpu.reg_read(UC_X86_REG_SP)
            calls.append(
                {
                    "call": "resource_palette_blocks_apply",
                    "stream_segment": cpu.reg_read(UC_X86_REG_ES),
                    "stream_offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    "stack_pointer": stack_pointer,
                    "return_offset": machine16(cpu, STACK + stack_pointer)
                    + routine.header_size
                    - routine.span[0],
                }
            )
            near_return(cpu)
            return
        assert routine.span[0] <= file_address < routine.span[1], (
            routine.name,
            name,
            hex(file_address),
        )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=100)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == module

    after = {
        base: bytes(machine.mem_read(base, SEGMENT_SIZE))
        for base in (DATA, EXTRA, FS_DATA, GAME, STACK)
    }
    for base, expected, label in (
        (DATA, data_before, "GS decoy"),
        (EXTRA, extra_before, "ES decoy"),
        (FS_DATA, fs_before, "FS decoy"),
        (GAME, game_before, "game"),
    ):
        assert after[base] == bytes(expected), (routine.name, name, label)
    for offset, (before, after_byte) in enumerate(
        zip(stack_before, after[STACK], strict=True)
    ):
        assert before == after_byte or STACK_POINTER - 2 <= offset < STACK_POINTER, (
            routine.name,
            name,
            hex(offset),
        )
    assert after[STACK][STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] == (
        STACK_SENTINEL
    )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_ESI] = with_low16(
        initial[UC_X86_REG_ESI], payload_offset
    )
    expected_registers[UC_X86_REG_ES] = buffer_segment
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (routine.name, name, registers, expected_registers)
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    assert flags == initial[UC_X86_REG_EFLAGS], (routine.name, name, hex(flags))

    legacy_calls = [
        {key: value for key, value in call.items() if key not in {"stack_pointer", "return_offset"}}
        for call in calls
    ]
    row = {
        "name": name,
        "head_offset_ignored": head_offset,
        "buffer_segment": buffer_segment,
        "payload_offset": payload_offset,
        "calls": legacy_calls,
        "result_es": machine.reg_read(UC_X86_REG_ES),
        "result_si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
    }
    enriched_row = {
        **row,
        "callback_frame": {
            key: value for key, value in calls[0].items() if key in {"stack_pointer", "return_offset"}
        },
        "result_flags": flags & 0xFFFF,
    }
    canonical = (
        tuple(json.dumps(call, sort_keys=True) for call in calls),
        tuple(registers.items()),
        flags,
        normalized_stack(routine, after[STACK]),
    )
    return enriched_row, (row, canonical)


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
        default=Path(__file__).parent / "oracle_vectors/func_a778_natural.json",
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
    for case_index, vector in enumerate(vectors):
        commander_row, (commander_legacy, commander_result) = execute(
            commander_executable, COMMANDER, vector, case_index
        )
        sequel_row, (sequel_legacy, sequel_result) = execute(
            sequel_executable, SEQUEL, vector, case_index
        )
        assert commander_legacy == vector, vector["name"]
        assert sequel_legacy == commander_legacy, vector["name"]
        assert sequel_result == commander_result, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        rows.append(sequel_row)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} BBB presentation palette-dispatch cases")


if __name__ == "__main__":
    main()
