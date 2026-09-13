#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio-stream refill routines."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_CS,
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

ROOT = Path(__file__).resolve().parents[2]
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_bc50_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xBC50, 0xBD09),
        "instruction_count": 73,
        "sha256": "99c96376cc75ea56f0a0d77f7540549d10bfc80eac3e1be93a131a71a5b9a765",
        "occupied_mask": 2,
        "fields": {
            "sound": 0x0ADE,
            "first_buffer": 0x0B89,
            "second_buffer": 0x0B91,
            "header": 0x0B99,
            "pending": 0x0BA0,
            "channel": 0x0BA3,
            "page": 0x0BA5,
            "page_count": 0x0BA7,
            "final_length": 0x0BA9,
            "stream_pointer": 0x0BB7,
            "play_callback": 0x0CDB,
            "service_callback": 0x0CEB,
            "position_callback": 0x0CF3,
        },
        "page_helper": 0xBD09,
        "page_return": 0xBCB4,
        "position_return": 0xBC7D,
        "service_return": 0xBCE1,
        "play_return": 0xBCFE,
        "ultrasound": None,
    },
    "sequel": {
        "routine": (0xD40E, 0xD4B3),
        "instruction_count": 68,
        "sha256": "1288675adfc75d26bcb1f797b62721c7a87ebc5d4cd39c45ec59ffe5be783912",
        "occupied_mask": 3,
        "fields": {
            "sound": 0x0CE7,
            "first_buffer": 0x0D93,
            "second_buffer": 0x0D9B,
            "header": 0x0DA3,
            "pending": 0x0DAA,
            "channel": 0x0DAD,
            "page": 0x0DAF,
            "page_count": 0x0DB1,
            "final_length": 0x0DB3,
            "stream_pointer": 0x0DC1,
            "play_callback": 0x0F29,
            "service_callback": 0x0F39,
            "position_callback": 0x0F41,
        },
        "page_helper": 0xD4B3,
        "page_return": 0xD478,
        "position_return": 0xD441,
        "service_return": 0xD4A5,
        "play_return": 0xD4AB,
        "ultrasound": 0x0F1F,
    },
}

CASES = (
    ("sound_disabled", 0, 1, 2, 0, 0, 0x1234, 1, 4, 0x2222),
    ("channel_disabled", 1, 0, 2, 0, 0, 0x1234, 1, 4, 0x2222),
    ("stream_inactive", 1, 1, 1, 0, 0, 0x1234, 1, 4, 0x2222),
    ("both_busy_return", 1, 1, 2, 2, 2, 0x1234, 1, 4, 0x2222),
    ("first_page_service", 1, 1, 2, 0, 2, 0x1234, 0, 4, 0x2222),
    ("prefixed_page_service", 1, 1, 2, 0, 2, 0x3456, 2, 5, 0x2222),
    ("second_buffer_play", 1, 1, 2, 2, 0, 0x0000, 1, 5, 0x2222),
    ("busy_minus_one_final", 1, 1, 2, 2, 2, 0xFFFF, 3, 4, 0x0123),
    ("page_word_wrap", 1, 1, 2, 0, 2, 0x2345, 0xFFFF, 5, 0x3456),
)

COVERAGE_CASES = (
    ("both_busy_zero_position", 1, 1, 2, 2, 2, 0x0000, 1, 5, 0x2222),
    ("first_buffer_page_zero_play", 1, 1, 2, 0, 2, 0x0000, 0, 5, 0x2222),
)

READY_CASES = (
    ("first_ready_selects_second", 1, 0, 0x2345),
    ("both_ready_return", 1, 1, 0x2345),
)

ULTRASOUND_CASES = (
    ("ultrasound_active", 1, 1, 2),
    ("ultrasound_precedes_closed_gates", 0, 0, 0),
)

MACHINE_SIZE = 0xB0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x4000
AUDIO_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
FS_SEGMENT = 0x7000
STACK_SEGMENT = 0x8000
CALLBACK_SEGMENT = 0x9000
UNOWNED_SEGMENT = 0xA000
AUDIO_OFFSET = 0x1000
POSITION_OFFSET = 0x0100
SERVICE_OFFSET = 0x0200
PLAY_OFFSET = 0x0300
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778")

REGISTER_IDS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 11 + seed * 29 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    edges = set()
    for instruction in decoder.disasm(body, start):
        loop_control = instruction.mnemonic in {
            "jcxz",
            "jecxz",
            "loop",
            "loope",
            "loopne",
        }
        if capstone.CS_GRP_JUMP not in instruction.groups and not loop_control:
            continue
        if instruction.id in (X86_INS_JMP, X86_INS_LJMP):
            continue
        immediate = next(
            (
                operand.imm
                for operand in instruction.operands
                if operand.type == X86_OP_IMM
            ),
            None,
        )
        assert immediate is not None
        source = instruction.address - start
        edges.add((source, int(immediate) - start))
        edges.add((source, instruction.address + instruction.size - start))
    return edges


def write_pointer(
    memory: bytearray, offset: int, pointer_offset: int, segment: int
) -> None:
    struct.pack_into("<HH", memory, offset, pointer_offset, segment)


def in_ranges(address: int, size: int, ranges: list[tuple[int, int]]) -> bool:
    return any(start <= address and address + size <= stop for start, stop in ranges)


def initial_registers(case_index: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }


def final_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def map_segments(
    machine: Uc,
    executable: bytes,
    segments: tuple[tuple[int, bytearray], ...],
) -> None:
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in segments:
        machine.mem_write(segment * 16, bytes(contents))


def case_dict(values: tuple[Any, ...]) -> dict[str, Any]:
    keys = (
        "name",
        "sound",
        "channel",
        "pending",
        "first_state",
        "second_state",
        "position",
        "page",
        "page_count",
        "final_length",
    )
    return dict(zip(keys, values, strict=True))


def expected_saved_stack(initial: dict[int, int], stack_before: bytes) -> bytearray:
    expected = bytearray(stack_before)
    struct.pack_into(
        "<HHHHHHH",
        expected,
        0xFEF2,
        initial[UC_X86_REG_EDX] & 0xFFFF,
        DATA_SEGMENT,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_EAX] & 0xFFFF,
        initial[UC_X86_REG_ESI] & 0xFFFF,
        initial[UC_X86_REG_EDI] & 0xFFFF,
        EXTRA_SEGMENT,
    )
    return expected


def execute_case(
    executable: bytes,
    branch_name: str,
    case: dict[str, Any],
    case_index: int,
    *,
    ultrasound: bool = False,
) -> tuple[dict[str, Any], dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    fields = branch["fields"]
    start, stop = branch["routine"]
    occupied_mask = int(branch["occupied_mask"])
    first_occupied = bool(int(case["first_state"]) & occupied_mask)
    second_occupied = bool(int(case["second_state"]) & occupied_mask)
    gated = bool(
        not ultrasound
        and int(case["sound"]) & 1
        and int(case["channel"]) & 1
        and int(case["pending"]) & 2
    )
    immediate_return = bool(
        gated
        and first_occupied
        and second_occupied
        and int(case["position"]) not in (0, 0xFFFF)
    )
    fills = gated and not immediate_return
    selected_index = 1 if first_occupied else 0
    selected_offset = (
        int(fields["second_buffer"]) if selected_index else int(fields["first_buffer"])
    )
    destination = (AUDIO_OFFSET + (0x4008 if selected_index else 0)) & 0xFFFF
    page_destination = (destination + (6 if int(case["page"]) != 0 else 0)) & 0xFFFF
    next_candidate = (int(case["page"]) + 1) & 0xFFFF
    reaches_end = next_candidate >= int(case["page_count"])
    next_page = 0 if fills and reaches_end else next_candidate
    selected_length = int(case["final_length"]) if fills and reaches_end else 0x4000
    driver_kind = (
        "play"
        if fills and int(case["position"]) in (0, 0xFFFF)
        else "service"
        if fills
        else None
    )
    initial = initial_registers(case_index)

    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    audio = seeded_segment(case_index + 33)
    extra = seeded_segment(case_index + 49)
    fs_data = seeded_segment(case_index + 65)
    stack = seeded_segment(case_index + 81)
    callback = seeded_segment(case_index + 97)
    unowned = seeded_segment(case_index + 113)
    game[int(fields["sound"])] = int(case["sound"])
    game[int(fields["channel"])] = int(case["channel"])
    game[int(fields["pending"])] = int(case["pending"])
    struct.pack_into("<H", game, int(fields["page"]), int(case["page"]))
    struct.pack_into("<H", game, int(fields["page_count"]), int(case["page_count"]))
    struct.pack_into("<H", game, int(fields["final_length"]), int(case["final_length"]))
    header = struct.pack("<HHH", 0x1122, 0x3344, 0x5566)
    game[int(fields["header"]) : int(fields["header"]) + 6] = header
    first_descriptor = struct.pack(
        "<HHHBB",
        AUDIO_OFFSET,
        AUDIO_SEGMENT,
        0x1111,
        int(case["first_state"]),
        0xA5,
    )
    second_descriptor = struct.pack(
        "<HHHBB",
        (AUDIO_OFFSET + 0x4008) & 0xFFFF,
        AUDIO_SEGMENT,
        0x2222,
        int(case["second_state"]),
        0x5A,
    )
    game[int(fields["first_buffer"]) : int(fields["first_buffer"]) + 8] = (
        first_descriptor
    )
    game[int(fields["second_buffer"]) : int(fields["second_buffer"]) + 8] = (
        second_descriptor
    )
    write_pointer(game, int(fields["stream_pointer"]), AUDIO_OFFSET, AUDIO_SEGMENT)
    write_pointer(
        game, int(fields["position_callback"]), POSITION_OFFSET, CALLBACK_SEGMENT
    )
    write_pointer(
        game, int(fields["service_callback"]), SERVICE_OFFSET, CALLBACK_SEGMENT
    )
    write_pointer(game, int(fields["play_callback"]), PLAY_OFFSET, CALLBACK_SEGMENT)
    if branch["ultrasound"] is not None:
        game[int(branch["ultrasound"])] = int(ultrasound)
    data[int(fields["sound"])] = int(case["sound"]) ^ 0xFF
    data[int(fields["channel"])] = int(case["channel"]) ^ 0xFF
    data[int(fields["pending"])] = int(case["pending"]) ^ 0xFF
    callback[POSITION_OFFSET] = 0xCB
    callback[SERVICE_OFFSET] = 0xCB
    callback[PLAY_OFFSET] = 0xCB
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    page_data = bytes(
        (index * 29 + case_index * 37 + 5) & 0xFF for index in range(0x4000)
    )

    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        AUDIO_SEGMENT: bytes(audio),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        CALLBACK_SEGMENT: bytes(callback),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytes(stack)
    patched = bytearray(executable)
    patched[int(branch["page_helper"])] = 0xC3
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    map_segments(
        machine,
        bytes(patched),
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (AUDIO_SEGMENT, audio),
            (EXTRA_SEGMENT, extra),
            (FS_SEGMENT, fs_data),
            (STACK_SEGMENT, stack),
            (CALLBACK_SEGMENT, callback),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected_edges = conditional_edges(executable[start:stop], start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    reached_return = False
    position_calls: list[dict[str, int]] = []
    page_calls: list[dict[str, Any]] = []
    driver_calls: list[dict[str, Any]] = []
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear == CALLBACK_SEGMENT * 16 + POSITION_OFFSET:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            frame = struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEE, 4))
            assert frame == (int(branch["position_return"]), 0)
            returned = int(case["position"]) if not position_calls else 0x1234
            position_calls.append(
                {
                    "ax_before": cpu.reg_read(UC_X86_REG_AX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                    "returned": returned,
                }
            )
            cpu.reg_write(UC_X86_REG_AX, returned)
            previous_source = None
            return
        if address == int(branch["page_helper"]) and cpu.reg_read(UC_X86_REG_CS) == 0:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF0
            return_ip = struct.unpack(
                "<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF0, 2)
            )[0]
            assert return_ip == int(branch["page_return"])
            page_calls.append(
                {
                    "page": cpu.reg_read(UC_X86_REG_AX),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "di": cpu.reg_read(UC_X86_REG_DI),
                    "prefix": bytes(
                        cpu.mem_read(AUDIO_SEGMENT * 16 + destination, 6)
                    ).hex(),
                }
            )
            cpu.mem_write(AUDIO_SEGMENT * 16 + page_destination, page_data)
            previous_source = None
            return
        if linear in (
            CALLBACK_SEGMENT * 16 + SERVICE_OFFSET,
            CALLBACK_SEGMENT * 16 + PLAY_OFFSET,
        ):
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEE
            kind = (
                "service"
                if linear == CALLBACK_SEGMENT * 16 + SERVICE_OFFSET
                else "play"
            )
            expected_return = (
                int(branch["service_return"])
                if kind == "service"
                else int(branch["play_return"])
            )
            frame = struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEE, 4))
            assert frame == (expected_return, 0)
            driver_calls.append(
                {
                    "kind": kind,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "di": cpu.reg_read(UC_X86_REG_DI),
                    "first_state": cpu.mem_read(
                        GAME_SEGMENT * 16 + int(fields["first_buffer"]) + 6, 1
                    )[0],
                    "second_state": cpu.mem_read(
                        GAME_SEGMENT * 16 + int(fields["second_buffer"]) + 6, 1
                    )[0],
                    "length": struct.unpack(
                        "<H",
                        cpu.mem_read(GAME_SEGMENT * 16 + selected_offset + 4, 2),
                    )[0],
                    "next_page": struct.unpack(
                        "<H", cpu.mem_read(GAME_SEGMENT * 16 + int(fields["page"]), 2)
                    )[0],
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                }
            )
            cpu.mem_write(GAME_SEGMENT * 16 + int(fields["first_buffer"]) + 6, b"\x02")
            cpu.mem_write(GAME_SEGMENT * 16 + int(fields["second_buffer"]) + 6, b"\x02")
            previous_source = None
            return
        assert cpu.reg_read(UC_X86_REG_CS) == 0 and start <= address < stop, (
            case["name"],
            branch_name,
            hex(address),
            hex(linear),
        )
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(start, 0, count=2_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    expected_position_count = (
        0 if not gated else 2 if fills and branch_name == "commander" else 1
    )
    assert len(position_calls) == expected_position_count, (
        case["name"],
        branch_name,
        position_calls,
    )
    assert all(
        call["ds"] == GAME_SEGMENT and call["sp"] == 0xFEEE for call in position_calls
    )
    if position_calls:
        assert position_calls[0]["returned"] == int(case["position"])
    if len(position_calls) == 2:
        assert position_calls[1]["returned"] == 0x1234

    if fills:
        expected_prefix = (
            header
            if int(case["page"])
            else before[AUDIO_SEGMENT][destination : destination + 6]
        )
        assert page_calls == [
            {
                "page": int(case["page"]),
                "si": selected_offset,
                "ds": GAME_SEGMENT,
                "es": AUDIO_SEGMENT,
                "di": page_destination,
                "prefix": expected_prefix.hex(),
            }
        ], (case["name"], branch_name, page_calls)
        if branch_name == "commander" and driver_kind == "play":
            callback_first = int(selected_index == 0)
            callback_second = int(selected_index == 1)
            callback_di = (
                page_destination
                if page_destination == AUDIO_OFFSET
                else (page_destination - 6) & 0xFFFF
            )
        elif branch_name == "sequel" and driver_kind == "play":
            callback_first = int(case["first_state"])
            callback_second = int(case["second_state"])
            callback_di = page_destination
        else:
            callback_first = 1 if selected_index == 0 else int(case["first_state"])
            callback_second = 1 if selected_index == 1 else int(case["second_state"])
            callback_di = page_destination
        assert driver_calls == [
            {
                "kind": driver_kind,
                "ax": 0,
                "si": selected_offset,
                "ds": GAME_SEGMENT,
                "es": AUDIO_SEGMENT,
                "di": callback_di,
                "first_state": callback_first,
                "second_state": callback_second,
                "length": selected_length,
                "next_page": next_page,
                "sp": 0xFEEE,
            }
        ], (case["name"], branch_name, driver_calls)
    else:
        assert not page_calls and not driver_calls

    expected_game = bytearray(before[GAME_SEGMENT])
    expected_audio = bytearray(before[AUDIO_SEGMENT])
    if fills:
        struct.pack_into("<H", expected_game, selected_offset + 4, selected_length)
        struct.pack_into("<H", expected_game, int(fields["page"]), next_page)
        expected_game[int(fields["first_buffer"]) + 6] = 2
        expected_game[int(fields["second_buffer"]) + 6] = 2
        if int(case["page"]):
            expected_audio[destination : destination + 6] = header
        expected_audio[page_destination : page_destination + 0x4000] = page_data
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert (
        bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == before[DATA_SEGMENT]
    )
    assert bytes(machine.mem_read(AUDIO_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_audio
    )
    for segment in (
        EXTRA_SEGMENT,
        FS_SEGMENT,
        CALLBACK_SEGMENT,
        UNOWNED_SEGMENT,
    ):
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == before[segment]
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)

    expected_stack = expected_saved_stack(initial, stack_before)
    if position_calls:
        final_return = int(branch["position_return"])
        if fills and branch_name == "sequel":
            final_return = (
                int(branch["service_return"])
                if driver_kind == "service"
                else int(branch["play_return"])
            )
        struct.pack_into("<HH", expected_stack, 0xFEEE, final_return, 0)
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )

    allowed_writes = [(STACK_SEGMENT * 16 + 0xFEEE, STACK_SEGMENT * 16 + 0xFF00)]
    if fills:
        allowed_writes.extend(
            [
                (
                    AUDIO_SEGMENT * 16 + destination,
                    AUDIO_SEGMENT * 16 + page_destination + 0x4000,
                ),
                (
                    GAME_SEGMENT * 16 + int(fields["first_buffer"]) + 4,
                    GAME_SEGMENT * 16 + int(fields["first_buffer"]) + 7,
                ),
                (
                    GAME_SEGMENT * 16 + int(fields["second_buffer"]) + 4,
                    GAME_SEGMENT * 16 + int(fields["second_buffer"]) + 7,
                ),
                (
                    GAME_SEGMENT * 16 + int(fields["page"]),
                    GAME_SEGMENT * 16 + int(fields["page"]) + 2,
                ),
            ]
        )
    assert all(in_ranges(address, size, allowed_writes) for address, size in writes), (
        case["name"],
        branch_name,
        [item for item in writes if not in_ranges(*item, allowed_writes)],
    )

    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    expected_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": DATA_SEGMENT,
        "es": EXTRA_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    assert actual_registers == expected_registers
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    row = {
        "name": case["name"],
        "sound_enabled": int(case["sound"]),
        "channel_active": int(case["channel"]),
        "stream_pending": int(case["pending"]),
        "position": int(case["position"]) if gated else None,
        "selected_buffer": selected_index if fills else None,
        "page_read": int(case["page"]) if fills else None,
        "header_prefixed": bool(case["page"]) if fills else None,
        "driver_action": driver_kind,
        "next_page": next_page if fills else int(case["page"]),
        "selected_length": selected_length if fills else None,
    }
    details = {
        "logical_state": {
            "first_state": expected_game[int(fields["first_buffer"]) + 6],
            "second_state": expected_game[int(fields["second_buffer"]) + 6],
            "next_page": struct.unpack_from("<H", expected_game, int(fields["page"]))[
                0
            ],
            "selected_length": (
                struct.unpack_from("<H", expected_game, selected_offset + 4)[0]
                if fills
                else None
            ),
            "audio_sha256": hashlib.sha256(expected_audio).hexdigest(),
        },
        "position_call_count": len(position_calls),
        "driver_calls": driver_calls,
        "defined_flags": final_flags(machine),
    }
    return row, details, covered_edges


def assert_body(executable: bytes, branch_name: str) -> set[tuple[int, int]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    body = executable[start:stop]
    assert hashlib.sha256(body).hexdigest() == branch["sha256"]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    assert len(list(decoder.disasm(body, start))) == branch["instruction_count"]
    return conditional_edges(body, start)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    commander = COMMANDER_PATH.read_bytes()
    sequel = args.sequel_executable.read_bytes()
    for name, executable, expected in (
        ("Commander", commander, COMMANDER_SHA256),
        ("BBB", sequel, SEQUEL_SHA256),
    ):
        digest = hashlib.sha256(executable).hexdigest()
        if digest != expected:
            raise SystemExit(f"unsupported {name} executable SHA-256 {digest}")

    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    cases = [case_dict(values) for values in CASES]
    assert len(cases) == len(expected_rows) == 9
    assert [case["name"] for case in cases] == [row["name"] for row in expected_rows]
    expected_edges = {
        "commander": assert_body(commander, "commander"),
        "sequel": assert_body(sequel, "sequel"),
    }
    coverage = {branch_name: set() for branch_name in BRANCHES}

    rows = []
    for case_index, (case, expected_row) in enumerate(
        zip(cases, expected_rows, strict=True)
    ):
        commander_row, commander_details, commander_edges = execute_case(
            commander, "commander", case, case_index
        )
        sequel_row, sequel_details, sequel_edges = execute_case(
            sequel, "sequel", case, case_index
        )
        assert commander_row == expected_row, (
            case["name"],
            commander_row,
            expected_row,
        )
        assert sequel_row == expected_row, (case["name"], sequel_row, expected_row)
        assert commander_details["logical_state"] == sequel_details["logical_state"], (
            case["name"],
            commander_details,
            sequel_details,
        )
        if commander_row["selected_buffer"] is not None:
            assert commander_details["position_call_count"] == 2
            assert sequel_details["position_call_count"] == 1
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        sequel_row["defined_flags"] = sequel_details["defined_flags"]
        sequel_row["return"] = "far"
        rows.append(sequel_row)

    coverage_results = []
    for case_index, values in enumerate(COVERAGE_CASES):
        case = case_dict(values)
        commander_row, commander_details, commander_edges = execute_case(
            commander, "commander", case, case_index + 10
        )
        sequel_row, sequel_details, sequel_edges = execute_case(
            sequel, "sequel", case, case_index + 10
        )
        assert commander_row == sequel_row
        assert commander_details["logical_state"] == sequel_details["logical_state"]
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        coverage_results.append((commander_row, sequel_row))

    ready_results = []
    for case_index, (name, first_state, second_state, position) in enumerate(
        READY_CASES
    ):
        case = case_dict(
            (
                name,
                1,
                1,
                2,
                first_state,
                second_state,
                position,
                2,
                5,
                0x2222,
            )
        )
        row, details, edges = execute_case(sequel, "sequel", case, case_index + 20)
        coverage["sequel"].update(edges)
        ready_results.append((row, details))
    assert ready_results[0][0]["selected_buffer"] == 1
    assert ready_results[0][0]["driver_action"] == "service"
    assert ready_results[1][0]["selected_buffer"] is None
    assert ready_results[1][1]["position_call_count"] == 1

    ultrasound_results = []
    for case_index, (name, sound, channel, pending) in enumerate(ULTRASOUND_CASES):
        case = case_dict((name, sound, channel, pending, 0, 0, 0x1234, 1, 4, 0x2222))
        row, details, edges = execute_case(
            sequel, "sequel", case, case_index + 30, ultrasound=True
        )
        coverage["sequel"].update(edges)
        ultrasound_results.append((row, details))
    assert all(result[1]["position_call_count"] == 0 for result in ultrasound_results)
    assert all(result[0]["selected_buffer"] is None for result in ultrasound_results)

    for branch_name in BRANCHES:
        assert coverage[branch_name] == expected_edges[branch_name], (
            branch_name,
            sorted(expected_edges[branch_name] - coverage[branch_name]),
            sorted(coverage[branch_name] - expected_edges[branch_name]),
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual refill cases, {len(coverage_results)} dual coverage "
        f"probes, {len(ready_results)} BBB ready-state cases, "
        f"{len(ultrasound_results)} BBB ULTRASND bypasses, and "
        f"{len(coverage['sequel'])} BBB conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} sites"
    )


if __name__ == "__main__":
    main()
