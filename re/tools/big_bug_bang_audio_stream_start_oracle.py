#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio-stream start wrappers."""

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
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_bbb3_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xBBB3, 0xBC50),
        "sha256": "e2568920091990dad94d80153b8d6981b474cca099647f0ff716b90198dae376",
        "fields": {
            "sound": 0x0ADE,
            "channel": 0x0BA3,
            "pending": 0x0BA0,
            "packed": 0x0BA2,
            "first_buffer": 0x0B89,
            "second_buffer": 0x0B91,
            "header": 0x0B99,
            "page": 0x0BA5,
            "stream_pointer": 0x0BB7,
            "play_callback": 0x0CDB,
        },
        "page_helper": 0xBD09,
        "page_return": 0xBBFA,
        "stop_helper": 0xBB9D,
        "stop_return": 0xBC37,
        "play_return": 0xBC49,
        "ultrasound": None,
        "ultrasound_helper": None,
        "ultrasound_return": None,
    },
    "sequel": {
        "routine": (0xD363, 0xD40E),
        "sha256": "4a10124c3f3692d1c4a765d7d7d8d1e303bcce5b44422dc210ef4a5decce3fc7",
        "fields": {
            "sound": 0x0CE7,
            "channel": 0x0DAD,
            "pending": 0x0DAA,
            "packed": 0x0DAC,
            "first_buffer": 0x0D93,
            "second_buffer": 0x0D9B,
            "header": 0x0DA3,
            "page": 0x0DAF,
            "stream_pointer": 0x0DC1,
            "play_callback": 0x0F29,
        },
        "page_helper": 0xD4B3,
        "page_return": 0xD3B8,
        "stop_helper": 0xD33A,
        "stop_return": 0xD3F5,
        "play_return": 0xD407,
        "ultrasound": 0x0F1F,
        "ultrasound_helper": 0xD9F3,
        "ultrasound_return": 0xD39B,
    },
}

CASES = (
    {
        "name": "sound_disabled",
        "sound": 0x00,
        "channel": 0x01,
        "pending": 0x03,
        "packed": False,
    },
    {
        "name": "channel_disabled",
        "sound": 0x01,
        "channel": 0x00,
        "pending": 0x03,
        "packed": False,
    },
    {
        "name": "no_start_request",
        "sound": 0x01,
        "channel": 0x01,
        "pending": 0x04,
        "packed": False,
    },
    {
        "name": "request_bit_zero",
        "sound": 0x01,
        "channel": 0x01,
        "pending": 0x01,
        "packed": False,
    },
    {
        "name": "request_bit_one_packed",
        "sound": 0x01,
        "channel": 0x01,
        "pending": 0x02,
        "packed": True,
    },
    {
        "name": "both_request_bits",
        "sound": 0xFF,
        "channel": 0xFF,
        "pending": 0xFF,
        "packed": False,
    },
)

ULTRASOUND_CASES = (
    {"name": "ultrasound_request_bit_zero", "pending": 0x01},
    {"name": "ultrasound_both_request_bits", "pending": 0x03},
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
PLAY_OFFSET = 0x0200
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778")
PACKED_RATE_CODE = 0xD3

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


def expected_saved_stack(initial: dict[int, int], stack_before: bytearray) -> bytearray:
    expected = bytearray(stack_before)
    struct.pack_into(
        "<HHHHHH",
        expected,
        0xFEF4,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_EAX] & 0xFFFF,
        initial[UC_X86_REG_ESI] & 0xFFFF,
        DATA_SEGMENT,
        initial[UC_X86_REG_EDI] & 0xFFFF,
        EXTRA_SEGMENT,
    )
    return expected


def execute_shared(
    executable: bytes,
    branch_name: str,
    case: dict[str, Any],
    expected_row: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    fields = branch["fields"]
    start, stop = branch["routine"]
    active = bool(
        int(case["sound"]) & 1 and int(case["channel"]) & 1 and int(case["pending"]) & 3
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
    game[fields["sound"]] = int(case["sound"])
    game[fields["channel"]] = int(case["channel"])
    game[fields["pending"]] = int(case["pending"])
    write_pointer(game, fields["stream_pointer"], AUDIO_OFFSET, AUDIO_SEGMENT)
    write_pointer(game, fields["play_callback"], PLAY_OFFSET, CALLBACK_SEGMENT)
    if branch["ultrasound"] is not None:
        game[int(branch["ultrasound"])] = 0
    data[fields["sound"]] = int(case["sound"]) ^ 0xFF
    data[fields["channel"]] = int(case["channel"]) ^ 0xFF
    data[fields["pending"]] = int(case["pending"]) ^ 0xFF
    callback[PLAY_OFFSET] = 0xCB
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    page_data = bytearray(
        (index * 29 + case_index * 37 + 5) & 0xFF for index in range(0x4000)
    )
    if bool(case["packed"]):
        page_data[4] = PACKED_RATE_CODE
    elif page_data[4] == PACKED_RATE_CODE:
        page_data[4] ^= 0x80

    game_before = bytes(game)
    data_before = bytes(data)
    audio_before = bytes(audio)
    extra_before = bytes(extra)
    fs_before = bytes(fs_data)
    stack_before = bytearray(stack)
    callback_before = bytes(callback)
    unowned_before = bytes(unowned)

    patched = bytearray(executable)
    patched[int(branch["page_helper"])] = 0xC3
    patched[int(branch["stop_helper"])] = 0xCB
    if branch["ultrasound_helper"] is not None:
        patched[int(branch["ultrasound_helper"])] = 0xC3
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
    page_calls: list[dict[str, int]] = []
    stop_calls: list[dict[str, int]] = []
    play_calls: list[dict[str, Any]] = []
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == int(branch["page_helper"]) and cpu.reg_read(UC_X86_REG_CS) == 0:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF2
            return_ip = struct.unpack(
                "<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF2, 2)
            )[0]
            assert return_ip == int(branch["page_return"])
            page_calls.append(
                {
                    "page": cpu.reg_read(UC_X86_REG_AX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "di": cpu.reg_read(UC_X86_REG_DI),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                }
            )
            cpu.mem_write(AUDIO_SEGMENT * 16 + AUDIO_OFFSET, bytes(page_data))
            previous_source = None
            return
        if address == int(branch["stop_helper"]) and cpu.reg_read(UC_X86_REG_CS) == 0:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF0
            frame = struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF0, 4))
            assert frame == (int(branch["stop_return"]), 0)
            stop_calls.append(
                {
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "pending": cpu.mem_read(GAME_SEGMENT * 16 + fields["pending"], 1)[
                        0
                    ],
                }
            )
            cpu.mem_write(GAME_SEGMENT * 16 + fields["pending"], b"\0")
            previous_source = None
            return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear == CALLBACK_SEGMENT * 16 + PLAY_OFFSET:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF0
            frame = struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF0, 4))
            assert frame == (int(branch["play_return"]), 0)
            play_calls.append(
                {
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "di": cpu.reg_read(UC_X86_REG_DI),
                    "descriptor": bytes(
                        cpu.mem_read(GAME_SEGMENT * 16 + fields["first_buffer"], 8)
                    ).hex(),
                }
            )
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
        machine.emu_start(start, 0, count=1_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    expected_game = bytearray(game_before)
    expected_audio = bytearray(audio_before)
    expected_stack = expected_saved_stack(initial, stack_before)
    if active:
        first = struct.pack(
            "<HHHBB",
            AUDIO_OFFSET,
            AUDIO_SEGMENT,
            0x4000,
            1,
            expected_game[fields["first_buffer"] + 7],
        )
        second = struct.pack(
            "<HHHBB",
            (AUDIO_OFFSET + 0x4008) & 0xFFFF,
            AUDIO_SEGMENT,
            0x4000,
            0,
            expected_game[fields["second_buffer"] + 7],
        )
        expected_game[fields["first_buffer"] : fields["first_buffer"] + 8] = first
        expected_game[fields["second_buffer"] : fields["second_buffer"] + 8] = second
        expected_game[fields["header"] : fields["header"] + 6] = page_data[:6]
        expected_game[fields["pending"]] = 2
        expected_game[fields["packed"]] = int(bool(case["packed"]))
        struct.pack_into("<H", expected_game, fields["page"], 1)
        expected_audio[AUDIO_OFFSET : AUDIO_OFFSET + 0x4000] = page_data
        struct.pack_into("<HH", expected_stack, 0xFEF0, int(branch["play_return"]), 0)
        assert page_calls == [
            {
                "page": 0,
                "ds": GAME_SEGMENT,
                "es": AUDIO_SEGMENT,
                "di": AUDIO_OFFSET,
                "sp": 0xFEF2,
            }
        ]
        assert stop_calls == [
            {
                "ax": struct.unpack_from("<H", page_data, 4)[0],
                "ds": GAME_SEGMENT,
                "pending": int(case["pending"]),
            }
        ], (case["name"], branch_name, stop_calls)
        assert play_calls == [
            {
                "ax": 0,
                "si": fields["first_buffer"],
                "ds": GAME_SEGMENT,
                "es": AUDIO_SEGMENT,
                "di": (AUDIO_OFFSET + 0x4008) & 0xFFFF,
                "descriptor": first.hex(),
            }
        ]
    else:
        assert not page_calls and not stop_calls and not play_calls

    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
    assert bytes(machine.mem_read(AUDIO_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_audio
    )
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    assert (
        bytes(machine.mem_read(CALLBACK_SEGMENT * 16, SEGMENT_SIZE)) == callback_before
    )
    assert bytes(machine.mem_read(UNOWNED_SEGMENT * 16, SEGMENT_SIZE)) == unowned_before
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)

    allowed_writes = [
        (STACK_SEGMENT * 16 + 0xFEF0, STACK_SEGMENT * 16 + 0xFF00),
        (AUDIO_SEGMENT * 16 + AUDIO_OFFSET, AUDIO_SEGMENT * 16 + AUDIO_OFFSET + 0x4000),
        (
            GAME_SEGMENT * 16 + fields["first_buffer"],
            GAME_SEGMENT * 16 + fields["second_buffer"] + 8,
        ),
        (
            GAME_SEGMENT * 16 + fields["pending"],
            GAME_SEGMENT * 16 + fields["pending"] + 1,
        ),
        (
            GAME_SEGMENT * 16 + fields["packed"],
            GAME_SEGMENT * 16 + fields["packed"] + 1,
        ),
        (
            GAME_SEGMENT * 16 + fields["header"],
            GAME_SEGMENT * 16 + fields["header"] + 6,
        ),
        (
            GAME_SEGMENT * 16 + fields["page"],
            GAME_SEGMENT * 16 + fields["page"] + 2,
        ),
    ]
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
        "start_request": int(case["pending"]),
        "started": active,
        "first_page": 0 if active else None,
        "next_page": 1 if active else None,
        "packed_header": bool(case["packed"]) if active else None,
        "first_buffer_state": 1 if active else None,
        "second_buffer_state": 0 if active else None,
        "pending_after": 2 if active else int(case["pending"]),
    }
    assert row == expected_row, (case["name"], branch_name, row, expected_row)
    row.update({"defined_flags": final_flags(machine), "return": "far"})
    return row, covered_edges


def execute_ultrasound(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES["sequel"]
    fields = branch["fields"]
    start, stop = branch["routine"]
    helper = int(branch["ultrasound_helper"])
    initial = initial_registers(case_index + 20)
    game = seeded_segment(case_index + 129)
    data = seeded_segment(case_index + 145)
    audio = seeded_segment(case_index + 161)
    extra = seeded_segment(case_index + 177)
    fs_data = seeded_segment(case_index + 193)
    stack = seeded_segment(case_index + 209)
    unowned = seeded_segment(case_index + 225)
    game[fields["sound"]] = 1
    game[fields["channel"]] = 1
    game[fields["pending"]] = int(case["pending"])
    game[int(branch["ultrasound"])] = 1
    write_pointer(game, fields["stream_pointer"], AUDIO_OFFSET, AUDIO_SEGMENT)
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        AUDIO_SEGMENT: bytes(audio),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        UNOWNED_SEGMENT: bytes(unowned),
    }
    stack_before = bytearray(stack)
    patched = bytearray(executable)
    patched[helper] = 0xC3
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
    helper_calls: list[dict[str, int]] = []
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == helper and cpu.reg_read(UC_X86_REG_CS) == 0:
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF2
            return_ip = struct.unpack(
                "<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF2, 2)
            )[0]
            assert return_ip == int(branch["ultrasound_return"])
            helper_calls.append(
                {
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "pending": cpu.mem_read(GAME_SEGMENT * 16 + fields["pending"], 1)[
                        0
                    ],
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                }
            )
            previous_source = None
            return
        assert cpu.reg_read(UC_X86_REG_CS) == 0 and start <= address < stop
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
        machine.emu_start(start, 0, count=200)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']}: failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    assert helper_calls == [
        {"ds": GAME_SEGMENT, "pending": int(case["pending"]), "sp": 0xFEF2}
    ]
    for segment, contents in before.items():
        assert bytes(machine.mem_read(segment * 16, SEGMENT_SIZE)) == contents
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)
    expected_stack = expected_saved_stack(initial, stack_before)
    struct.pack_into("<H", expected_stack, 0xFEF2, int(branch["ultrasound_return"]))
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEF2 <= address
        and address + size <= STACK_SEGMENT * 16 + 0xFF00
        for address, size in writes
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
    flags_after = final_flags(machine)
    assert flags_after == {name: False for name in FLAG_MASKS}
    return {
        "name": case["name"],
        "start_request_before": int(case["pending"]),
        "start_request_after": int(case["pending"]),
        "helper_calls": helper_calls,
        "defined_flags": flags_after,
        "return": "far",
    }, covered_edges


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
    assert len(CASES) == len(expected_rows) == 6
    assert [case["name"] for case in CASES] == [row["name"] for row in expected_rows]

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, (case, expected_row) in enumerate(
        zip(CASES, expected_rows, strict=True)
    ):
        commander_row, commander_edges = execute_shared(
            commander, "commander", case, expected_row, case_index
        )
        sequel_row, sequel_edges = execute_shared(
            sequel, "sequel", case, expected_row, case_index
        )
        assert commander_row == sequel_row, (case["name"], commander_row, sequel_row)
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    ultrasound_rows = []
    for case_index, case in enumerate(ULTRASOUND_CASES):
        row, edges = execute_ultrasound(sequel, case, case_index)
        ultrasound_rows.append(row)
        coverage["sequel"].update(edges)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges, (
            branch_name,
            sorted(expected_edges - coverage[branch_name]),
            sorted(coverage[branch_name] - expected_edges),
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual stream-start cases, "
        f"{len(ultrasound_rows)} BBB ULTRASND delegates, and "
        f"{len(coverage['sequel'])} BBB conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} sites"
    )


if __name__ == "__main__":
    main()
