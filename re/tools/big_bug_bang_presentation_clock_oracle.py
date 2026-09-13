#!/usr/bin/env python3
"""Compare Big Bug Bang's presentation clock with Commander Blood."""

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
EXTRA = 0x40000
CALLBACK = 0x50000
CALLBACK_OFFSET = 0x0100
FS_DATA = 0x60000
GAME = 0x70000
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
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    primary_mode: int
    secondary_mode: int
    audio_enabled: int
    software_timed_audio: int | None
    audio_callback: int
    audio_phase: int
    tick: int
    previous_tick: int
    threshold: int
    tick_reread: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA240, 0xA291),
    body_sha256="13d0c4bc7f9892797b231cf4df9424aa2f9d30bf34791cf7cf98e351beed0b73",
    primary_mode=0x27E0,
    secondary_mode=0x27E1,
    audio_enabled=0x0ADE,
    software_timed_audio=None,
    audio_callback=0x0CF3,
    audio_phase=0x0C41,
    tick=0x0B29,
    previous_tick=0x0DA2,
    threshold=0x0D77,
    tick_reread=0xA289,
    branches={
        0xA245: (0xA247, 0xA274),
        0xA24C: (0xA24E, 0xA274),
        0xA253: (0xA255, 0xA274),
        0xA263: (0xA265, 0xA268),
        0xA26C: (0xA26E, 0xA290),
        0xA27B: (0xA27D, 0xA27F),
        0xA281: (0xA283, 0xA289),
        0xA287: (0xA289, 0xA290),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xBA23, 0xBA7B),
    body_sha256="7f8fffb29e00869a219bc384c570a6fd6ec0584077bb5f561a92da386a70cf4e",
    primary_mode=0x2A80,
    secondary_mode=0x2A81,
    audio_enabled=0x0CE7,
    software_timed_audio=0x0F1F,
    audio_callback=0x0F41,
    audio_phase=0x0E4B,
    tick=0x0D33,
    previous_tick=0x0FF0,
    threshold=0x0FC5,
    tick_reread=0xBA73,
    branches={
        0xBA28: (0xBA2A, 0xBA5E),
        0xBA2F: (0xBA31, 0xBA5E),
        0xBA36: (0xBA38, 0xBA5E),
        0xBA3D: (0xBA3F, 0xBA5E),
        0xBA4D: (0xBA4F, 0xBA52),
        0xBA56: (0xBA58, 0xBA7A),
        0xBA65: (0xBA67, 0xBA69),
        0xBA6B: (0xBA6D, 0xBA73),
        0xBA71: (0xBA73, 0xBA7A),
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

SEQUEL_ONLY_CASES = (
    {
        "name": "ultrasound_backend_fallback_below",
        "mode_27e0": 0x81,
        "mode_27e1": 0x03,
        "audio_enabled": 0x05,
        "callback_value": 0x3000,
        "previous_phase": 0x0C68,
        "tick": 0x2104,
        "previous_tick": 0x2100,
        "threshold": 5,
        "reread_tick": 0x2104,
        "software_timed_audio": True,
    },
    {
        "name": "ultrasound_backend_fallback_exact_and_reread",
        "mode_27e0": 0x03,
        "mode_27e1": 0x81,
        "audio_enabled": 0xFF,
        "callback_value": 0x3E00,
        "previous_phase": 0x3E68,
        "tick": 0x2205,
        "previous_tick": 0x2200,
        "threshold": 5,
        "reread_tick": 0x2207,
        "software_timed_audio": True,
    },
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def machine_word(machine: Uc, address: int) -> int:
    return struct.unpack("<H", machine.mem_read(address, 2))[0]


def write_low16(machine: Uc, register: int, value: int) -> None:
    current = machine.reg_read(register)
    machine.reg_write(register, (current & 0xFFFF0000) | (value & 0xFFFF))


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    return_ip = machine_word(machine, STACK + sp)
    return_cs = machine_word(machine, STACK + sp + 2)
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_CS, return_cs)
    machine.reg_write(UC_X86_REG_IP, return_ip)


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    start = routine.span[0]
    return {
        (source - start, destination - start)
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    result = bytearray(stack)
    callback_return_offset = 25 if routine is COMMANDER else 32
    callback_return = routine.image_address(routine.span[0] + callback_return_offset)
    if read16(result, STACK_POINTER - 4) == callback_return:
        write16(
            result,
            STACK_POINTER - 4,
            COMMANDER.image_address(COMMANDER.span[0] + 25),
        )
    return bytes(result)


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


def modeled_row(vector: dict[str, object]) -> dict[str, object]:
    primary_mode = int(vector["mode_27e0"])
    secondary_mode = int(vector["mode_27e1"])
    audio_enabled = int(vector["audio_enabled"])
    software_timed_audio = bool(vector.get("software_timed_audio", False))
    callback_value = int(vector["callback_value"])
    previous_phase = int(vector["previous_phase"])
    tick = int(vector["tick"])
    previous_tick = int(vector["previous_tick"])
    threshold = int(vector["threshold"])
    reread_tick = int(vector["reread_tick"])
    audio_clock = bool(
        primary_mode & 1
        and secondary_mode & 1
        and audio_enabled & 1
        and not software_timed_audio
    )
    if audio_clock:
        current = (0x4000 - callback_value) & 0xFFFF
        normalized_delta = (current - previous_phase) & 0xFFFF
        if normalized_delta & 0x8000:
            normalized_delta = (normalized_delta + 0x4000) & 0xFFFF
        due = normalized_delta >= 0x0398
        result_ax = current
    else:
        normalized_delta = (tick - previous_tick) & 0xFFFF
        if normalized_delta & 0x8000:
            normalized_delta = (-normalized_delta) & 0xFFFF
        due = bool(normalized_delta & 0xFF00) or (normalized_delta & 0xFF) >= threshold
        result_ax = reread_tick if due else normalized_delta
    return {
        "name": str(vector["name"]),
        "audio_clock": audio_clock,
        "due": due,
        "mode_27e0": primary_mode,
        "mode_27e1": secondary_mode,
        "audio_enabled": audio_enabled,
        "software_timed_audio": software_timed_audio,
        "callback_value": callback_value,
        "previous_phase": previous_phase,
        "tick": tick,
        "previous_tick": previous_tick,
        "threshold": threshold,
        "reread_tick": reread_tick,
        "normalized_delta": normalized_delta,
        "result_ax": result_ax,
        "result_carry": int(not due),
    }


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, tuple[int, int]]:
    expected = modeled_row(vector)
    name = str(expected["name"])
    audio_clock = bool(expected["audio_clock"])
    due = bool(expected["due"])
    software_timed_audio = bool(expected["software_timed_audio"])
    callback_value = int(expected["callback_value"])
    reread_tick = int(expected["reread_tick"])

    data_before = seeded_segment(case_index, 17, 0x31)
    data_before[routine.primary_mode] = int(expected["mode_27e0"])
    data_before[routine.secondary_mode] = int(expected["mode_27e1"])
    data_before[routine.audio_enabled] = int(expected["audio_enabled"])
    if routine.software_timed_audio is not None:
        data_before[routine.software_timed_audio] = int(software_timed_audio)
    data_before[routine.audio_callback : routine.audio_callback + 4] = struct.pack(
        "<HH", CALLBACK_OFFSET, CALLBACK // 16
    )
    write16(data_before, routine.audio_phase, int(expected["previous_phase"]))
    write16(data_before, routine.tick, int(expected["tick"]))
    write16(data_before, routine.previous_tick, int(expected["previous_tick"]))
    data_before[routine.threshold] = int(expected["threshold"])
    extra_before = seeded_segment(case_index, 11, 0x53)
    fs_before = seeded_segment(case_index, 7, 0x75)
    game_before = seeded_segment(case_index, 5, 0x97)
    for offset, value in (
        (routine.primary_mode, 0xDE),
        (routine.secondary_mode, 0xAD),
        (routine.audio_enabled, 0xBE),
        (routine.audio_callback, 0x1111),
        (routine.audio_callback + 2, 0x2222),
        (routine.audio_phase, 0x3333),
        (routine.tick, 0x4444),
        (routine.previous_tick, 0x5555),
        (routine.threshold, 0xEF),
    ):
        if (
            isinstance(value, int)
            and value <= 0xFF
            and offset
            in (
                routine.primary_mode,
                routine.secondary_mode,
                routine.audio_enabled,
                routine.threshold,
            )
        ):
            game_before[offset] = value
        else:
            write16(game_before, offset, value)
    if routine.software_timed_audio is not None:
        game_before[routine.software_timed_audio] = 0x7F

    stack_before = seeded_segment(case_index, 19, 0xB9)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    initial = {
        UC_X86_REG_EAX: 0xA5A50000 | case_index,
        UC_X86_REG_EBX: 0xB6B61234,
        UC_X86_REG_ECX: 0xC7C72345,
        UC_X86_REG_EDX: 0xD8D83456,
        UC_X86_REG_ESI: 0xE9E94567,
        UC_X86_REG_EDI: 0xFAFA5678,
        UC_X86_REG_EBP: 0xABCD6789,
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
    machine.mem_map(0, 0xB0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(EXTRA, bytes(extra_before))
    machine.mem_write(FS_DATA, bytes(fs_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    callback_calls = 0
    tick_rereads = 0
    reached_return = False
    previous: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal callback_calls, tick_rereads, reached_return, previous
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == CALLBACK + CALLBACK_OFFSET:
            previous = None
            callback_calls += 1
            callback_return_offset = 25 if routine is COMMANDER else 32
            assert machine_word(cpu, STACK + cpu.reg_read(UC_X86_REG_SP)) == (
                routine.image_address(routine.span[0] + callback_return_offset)
            )
            assert machine_word(cpu, STACK + cpu.reg_read(UC_X86_REG_SP) + 2) == 0
            write_low16(cpu, UC_X86_REG_EAX, callback_value)
            far_return(cpu)
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
        if file_address == routine.tick_reread:
            tick_rereads += 1
            write16_at = DATA + routine.tick
            cpu.mem_write(write16_at, struct.pack("<H", reread_tick))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=100)
    assert reached_return, (routine.name, name)
    assert callback_calls == int(audio_clock), (routine.name, name, callback_calls)
    assert tick_rereads == int(not audio_clock and due), (
        routine.name,
        name,
        tick_rereads,
    )
    assert bytes(machine.mem_read(0, len(expected_module))) == bytes(expected_module)
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    expected_data = bytearray(data_before)
    if audio_clock and due:
        write16(
            expected_data,
            routine.audio_phase,
            (0x4000 - callback_value) & 0xFFFF,
        )
    if not audio_clock and due:
        write16(expected_data, routine.tick, reread_tick)
        write16(expected_data, routine.previous_tick, reread_tick)
    assert data_after == bytes(expected_data), (routine.name, name)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before)
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == bytes(fs_before)
    assert bytes(machine.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before)
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert stack_after[: STACK_POINTER - 64] == bytes(
        stack_before[: STACK_POINTER - 64]
    )

    row = dict(expected)
    row["result_ax"] = machine.reg_read(UC_X86_REG_EAX) & 0xFFFF
    row["result_carry"] = machine.reg_read(UC_X86_REG_EFLAGS) & 1
    assert row == expected, (routine.name, name, row, expected)
    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | int(
        expected["result_ax"]
    )
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    assert registers == {
        register: expected_registers[register] for register in REGISTERS
    }, (
        routine.name,
        name,
    )
    state = (
        read16(data_after, routine.audio_phase),
        read16(data_after, routine.tick),
        read16(data_after, routine.previous_tick),
    )
    return (
        row,
        registers,
        machine.reg_read(UC_X86_REG_EFLAGS),
        normalized_stack(routine, stack_after),
        state,
    )


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
        default=Path(__file__).parent / "oracle_vectors/func_a240_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    shared_vectors = json.loads(args.commander_vectors.read_text())
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(shared_vectors):
        commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        assert sequel_result == commander_result, vector["name"]
        rows.append(sequel_result[0])

    for case_index, vector in enumerate(SEQUEL_ONLY_CASES, start=len(shared_vectors)):
        rows.append(
            execute(sequel_executable, SEQUEL, vector, case_index, sequel_edges)[0]
        )

    assert commander_edges == normalized_edges(COMMANDER)
    assert sequel_edges == normalized_edges(SEQUEL)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(shared_vectors)} shared and {len(SEQUEL_ONLY_CASES)} "
        f"BBB-only presentation clock cases covering {len(sequel_edges)} sequel edges"
    )


if __name__ == "__main__":
    main()
