#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio stream-source loading."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
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
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_bdb7_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES: dict[str, dict[str, Any]] = {
    "commander": {
        "routine": (0xBDB7, 0xC005),
        "sha256": "ddea3b50a1ab2133d199f5c84fd96458bcf9c7a67446852276c53927875e4381",
        "instruction_count": 198,
        "runtime_segment": 0x1CE,
        "reveal": (0x71E, 0x1C15),
        "return_ips": {
            "path": 0xBDE0,
            "lookup": 0xBDED,
            "reveal": 0xBE57,
            "directory": 0xBF92,
            "xms": 0xBF5C,
        },
        "fields": {
            "sound_enabled": 0x0ADE,
            "channel_active": 0x0BA3,
            "embedded": 0x0AE2,
            "archive_offset": 0x0A8A,
            "remaining": 0x0A92,
            "ems_handle": 0x0A60,
            "xms_handle": 0x0A5E,
            "page_frame": 0x0A66,
            "xms_driver": 0x0A4A,
            "backend_position": 0x0A4E,
            "source_pointer": 0x0BB7,
            "backend": 0x0B9F,
            "page_count": 0x0BA7,
            "final_page_bytes": 0x0BA9,
            "music_changed": 0x0BA1,
            "start_requested": 0x0BA0,
            "temp_handle": 0x0C49,
            "prompt_source": 0x0190,
            "prompt_destination": 0x0E18,
            "text_end": 0x5E58,
            "prompt_mode": 0x27E2,
            "prompt_phase": 0x5E65,
            "prompt_hold": 0x67BC,
            "framebuffer": 0x5219,
            "alternate_framebuffer": 0x521D,
            "prompt_defer": 0x67B0,
            "temp_name": 0x00AE,
            "xms_request": 0x0A6C,
        },
    },
    "sequel": {
        "routine": (0xD561, 0xD7EC),
        "sha256": "67e65a601dc809a47d8b252a5599bb1920207e39413ca95383970ad2e996b0f3",
        "instruction_count": 215,
        "runtime_segment": 0x1E6,
        "reveal": (0x803, 0x235F),
        "return_ips": {
            "path": 0xD58A,
            "lookup": 0xD597,
            "reveal": 0xD601,
            "directory": 0xD73C,
            "xms": 0xD706,
        },
        "fields": {
            "sound_enabled": 0x0CE7,
            "channel_active": 0x0DAD,
            "embedded": 0x0CEB,
            "archive_offset": 0x0C82,
            "remaining": 0x0C8A,
            "ems_handle": 0x0C58,
            "xms_handle": 0x0C56,
            "page_frame": 0x0C5E,
            "xms_driver": 0x0C42,
            "backend_position": 0x0C46,
            "source_pointer": 0x0DC1,
            "backend": 0x0DA9,
            "page_count": 0x0DB1,
            "final_page_bytes": 0x0DB3,
            "music_changed": 0x0DAB,
            "start_requested": 0x0DAA,
            "temp_handle": 0x0E53,
            "prompt_source": 0x0198,
            "prompt_destination": 0x1066,
            "text_end": 0x6228,
            "prompt_mode": 0x2A82,
            "prompt_phase": 0x6235,
            "prompt_hold": 0x6B92,
            "framebuffer": 0x55E9,
            "alternate_framebuffer": 0x55ED,
            "prompt_defer": 0x6B86,
            "temp_name": 0x00AD,
            "xms_request": 0x0C64,
            "ultrasnd": 0x0F1F,
        },
    },
}

EXTRA_CASES = (
    {
        "name": "ems_external_two_reads",
        "sound_enabled": 1,
        "channel_active": 1,
        "active": True,
        "source_kind": "standalone",
        "backend": "ems",
        "payload_bytes": 0x8001,
        "read_chunks": [0x8000, 1],
        "page_count": 3,
        "final_page_bytes": 1,
        "ultrasnd": 0,
    },
    {
        "name": "file_external_two_reads",
        "sound_enabled": 1,
        "channel_active": 1,
        "active": True,
        "source_kind": "standalone",
        "backend": "file",
        "payload_bytes": 0x8001,
        "read_chunks": [0x8000, 1],
        "page_count": 3,
        "final_page_bytes": 1,
        "ultrasnd": 0,
    },
    {
        "name": "ultrasnd_ems_exact_32k",
        "sound_enabled": 1,
        "channel_active": 1,
        "active": True,
        "source_kind": "standalone",
        "backend": "ems",
        "payload_bytes": 0x8000,
        "read_chunks": [0x8000],
        "page_count": 2,
        "final_page_bytes": 0x4000,
        "ultrasnd": 1,
    },
    {
        "name": "ultrasnd_xms_one_byte_tail",
        "sound_enabled": 1,
        "channel_active": 1,
        "active": True,
        "source_kind": "standalone",
        "backend": "xms",
        "payload_bytes": 0x8001,
        "read_chunks": [0x8000, 1],
        "page_count": 3,
        "final_page_bytes": 1,
        "ultrasnd": 1,
    },
    {
        "name": "ultrasnd_file_twelve_kib",
        "sound_enabled": 1,
        "channel_active": 1,
        "active": True,
        "source_kind": "standalone",
        "backend": "file",
        "payload_bytes": 0x3000,
        "read_chunks": [0x3000],
        "page_count": 1,
        "final_page_bytes": 0x3000,
        "ultrasnd": 1,
    },
)

MACHINE_SIZE = 0xB0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
STORAGE_SEGMENT = 0x4000
PAGE_FRAME_SEGMENT = 0x5000
FS_SEGMENT = 0x6000
CALLBACK_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
UNOWNED_SEGMENT = 0xA000
CALLBACK_OFFSET = 0x0100
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
PATH_OFFSET = 0x0D2D
STACK_SENTINEL = bytes.fromhex("5aa596698778")
PROMPT = b"WAIT COMMANDER ...\r\0"
ORIGINAL_FRAMEBUFFER = struct.pack("<HH", 0x8000, 0xA000)
ALTERNATE_FRAMEBUFFER = struct.pack("<HH", 0x4000, 0xA000)

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


def write_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def read_word(machine: Uc, offset: int) -> int:
    return struct.unpack("<H", machine.mem_read(GAME_SEGMENT * 16 + offset, 2))[0]


def read_dword(machine: Uc, offset: int) -> int:
    return struct.unpack("<I", machine.mem_read(GAME_SEGMENT * 16 + offset, 4))[0]


def final_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def assert_unchanged_outside_ranges(
    before: bytes, after: bytes, ranges: list[tuple[int, int]]
) -> None:
    assert len(before) == len(after)
    mutable = bytearray(len(before))
    for start, stop in ranges:
        mutable[start:stop] = b"\x01" * (stop - start)
    assert all(
        mutable[index] or before_byte == after[index]
        for index, before_byte in enumerate(before)
    )


def initial_registers(case_index: int, source_handle: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B20000 | source_handle,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E50000 | PATH_OFFSET,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: 0x4800,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202 | (case_index & 1),
    }


def expected_storage_geometry(
    backend: str, chunks: list[int], sequel: bool, ultrasnd: bool
) -> tuple[int, int, int]:
    page_count = (
        sum(2 for chunk in chunks if chunk != 0)
        if backend == "xms"
        else len(chunks) * 2
    )
    last_read = chunks[-1]
    final = last_read & 0x3FFF
    if final == last_read:
        page_count = (page_count - 1) & 0xFFFF
    final_bytes = final or 0x4000
    if sequel and 0 < final < 0x40:
        final_bytes = 0x40
    page_bytes = 0x4000
    if ultrasnd:
        page_count = (page_count * 2) & 0xFFFF
        previous = final
        final &= 0x1FFF
        if final == previous:
            page_count = (page_count - 1) & 0xFFFF
        final_bytes = final or 0x2000
        page_bytes = 0x2000
    return page_count, final_bytes, page_bytes


def conditional_sites(
    executable: bytes, branch_name: str
) -> dict[int, tuple[int, int]]:
    start, stop = BRANCHES[branch_name]["routine"]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    sites = {}
    for instruction in decoder.disasm(executable[start:stop], start):
        if (
            capstone.CS_GRP_JUMP not in instruction.groups
            or instruction.mnemonic == "jmp"
        ):
            continue
        assert len(instruction.operands) == 1
        sites[instruction.address] = (
            instruction.operands[0].imm & 0xFFFF,
            instruction.address + instruction.size,
        )
    return sites


def execute_case(
    executable: bytes,
    branch_name: str,
    row: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    fields = branch["fields"]
    sequel = branch_name == "sequel"
    ultrasnd = bool(row.get("ultrasnd", 0)) if sequel else False
    active = bool(row["active"])
    backend = str(row["backend"] or "ems")
    embedded = row["source_kind"] == "embedded"
    payload_size = int(row["payload_bytes"] or 0)
    source_total = payload_size + 0x1A
    source_handle = 0x2340 + case_index
    opened_handle = 0x4560 + case_index
    temp_handle = 0x6780 + case_index
    old_temp_handle = 0x1357 if backend == "file" and active and not embedded else 0
    archive_offset = 0x1234FFF8 if embedded else 0
    ems_handle = 0x1111 if backend == "ems" else 0xFFFF
    xms_handle = 0x2222 if backend == "xms" else 0xFFFF
    initial = initial_registers(case_index, source_handle)

    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    storage = seeded_segment(case_index + 33)
    page_frame = seeded_segment(case_index + 49)
    fs_data = seeded_segment(case_index + 65)
    stack = seeded_segment(case_index + 81)
    unowned = seeded_segment(case_index + 97)
    source_bytes = bytes(
        (index * 29 + case_index * 37 + 5) & 0xFF for index in range(payload_size)
    )
    path = f"mu\\voice{case_index}.voc".encode("ascii") + b"\0"
    game[fields["prompt_source"] : fields["prompt_source"] + len(PROMPT)] = PROMPT
    game[fields["temp_name"] : fields["temp_name"] + 8] = b"mus.snd\0"
    game[fields["sound_enabled"]] = int(row["sound_enabled"])
    game[fields["channel_active"]] = int(row["channel_active"])
    game[fields["embedded"]] = int(embedded)
    if sequel:
        game[fields["ultrasnd"]] = int(ultrasnd)
    write_dword(game, fields["archive_offset"], archive_offset)
    write_dword(
        game,
        fields["remaining"],
        source_total if embedded else 0xDEADBEEF,
    )
    write_word(game, fields["ems_handle"], ems_handle)
    write_word(game, fields["xms_handle"], xms_handle)
    write_word(game, fields["page_frame"], PAGE_FRAME_SEGMENT)
    write_word(game, fields["xms_driver"], CALLBACK_OFFSET)
    write_word(game, fields["xms_driver"] + 2, CALLBACK_SEGMENT)
    write_dword(
        game,
        fields["source_pointer"],
        (STORAGE_SEGMENT << 16) | 0x1000,
    )
    write_word(game, fields["temp_handle"], old_temp_handle)
    game[fields["framebuffer"] : fields["framebuffer"] + 4] = ORIGINAL_FRAMEBUFFER
    game[fields["alternate_framebuffer"] : fields["alternate_framebuffer"] + 4] = (
        ALTERNATE_FRAMEBUFFER
    )
    data[PATH_OFFSET : PATH_OFFSET + len(path)] = path
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    segments = {
        GAME_SEGMENT: game,
        DATA_SEGMENT: data,
        STORAGE_SEGMENT: storage,
        PAGE_FRAME_SEGMENT: page_frame,
        FS_SEGMENT: fs_data,
        STACK_SEGMENT: stack,
        UNOWNED_SEGMENT: unowned,
    }
    for segment, contents in segments.items():
        machine.mem_write(segment * 16, bytes(contents))
    callbacks = {
        "path": (int(branch["runtime_segment"]), 0x03B3),
        "lookup": (int(branch["runtime_segment"]), 0x05EA),
        "directory": (int(branch["runtime_segment"]), 0x04E3),
        "reveal": tuple(branch["reveal"]),
        "xms": (CALLBACK_SEGMENT, CALLBACK_OFFSET),
    }
    for segment, offset in callbacks.values():
        machine.mem_write(segment * 16 + offset, b"\xcb")
    executable_snapshot = bytes(machine.mem_read(0, 0x20000))
    for register, value in initial.items():
        machine.reg_write(register, value)

    path_calls: list[dict[str, Any]] = []
    lookup_calls: list[dict[str, Any]] = []
    reveal_calls: list[dict[str, Any]] = []
    directory_calls: list[dict[str, Any]] = []
    xms_calls: list[dict[str, Any]] = []
    dos_calls: list[dict[str, Any]] = []
    ems_calls: list[dict[str, Any]] = []
    written_chunks: list[bytes] = []
    writes: list[tuple[int, int]] = []
    trace: list[int] = []
    read_cursor = 0
    reached_return = False

    callback_addresses = {
        segment * 16 + offset: name for name, (segment, offset) in callbacks.items()
    }

    def far_frame(cpu: Uc) -> tuple[int, int]:
        sp = cpu.reg_read(UC_X86_REG_SP)
        return struct.unpack("<HH", cpu.mem_read(STACK_SEGMENT * 16 + sp, 4))

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return
        trace.append(address)
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        callback_name = callback_addresses.get(address)
        if callback_name is None:
            return
        expected_sp = {
            "reveal": 0xFEDE,
            "xms": 0xFEE2,
        }.get(callback_name, 0xFEE8)
        actual_sp = cpu.reg_read(UC_X86_REG_SP)
        assert actual_sp == expected_sp, (callback_name, actual_sp, expected_sp)
        assert far_frame(cpu) == (branch["return_ips"][callback_name], 0)
        if callback_name == "path":
            path_calls.append(
                {
                    "dx": cpu.reg_read(UC_X86_REG_DX),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        elif callback_name == "lookup":
            lookup_calls.append(
                {
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                }
            )
            cpu.reg_write(UC_X86_REG_EBP, source_total)
        elif callback_name == "reveal":
            destination = fields["prompt_destination"]
            reveal_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "text": bytes(
                        cpu.mem_read(GAME_SEGMENT * 16 + destination, len(PROMPT))
                    ),
                    "text_end": read_word(cpu, fields["text_end"]),
                    "mode": cpu.mem_read(GAME_SEGMENT * 16 + fields["prompt_mode"], 1)[
                        0
                    ],
                    "phase": read_word(cpu, fields["prompt_phase"]),
                    "hold": cpu.mem_read(GAME_SEGMENT * 16 + fields["prompt_hold"], 1)[
                        0
                    ],
                    "framebuffer": bytes(
                        cpu.mem_read(GAME_SEGMENT * 16 + fields["framebuffer"], 4)
                    ),
                }
            )
        elif callback_name == "directory":
            directory_calls.append(
                {
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                }
            )
        else:
            request = bytes(cpu.mem_read(GAME_SEGMENT * 16 + fields["xms_request"], 16))
            xms_calls.append(
                {
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "si": cpu.reg_read(UC_X86_REG_SI),
                    "request": request,
                }
            )

    def interrupt(cpu: Uc, number: int, _context: object) -> None:
        nonlocal read_cursor
        ax = cpu.reg_read(UC_X86_REG_AX)
        bx = cpu.reg_read(UC_X86_REG_BX)
        cx = cpu.reg_read(UC_X86_REG_CX)
        dx = cpu.reg_read(UC_X86_REG_DX)
        ds = cpu.reg_read(UC_X86_REG_DS)
        if number == 0x67:
            ems_calls.append({"ax": ax, "page": bx, "handle": dx, "ds": ds})
            cpu.reg_write(UC_X86_REG_AX, ax & 0x00FF)
            return
        assert number == 0x21
        function = ax >> 8
        if ax == 0x3D00:
            dos_calls.append({"op": "open", "handle": opened_handle})
            cpu.reg_write(UC_X86_REG_AX, opened_handle)
        elif ax == 0x4200:
            dos_calls.append({"op": "seek", "handle": bx, "offset": (cx << 16) | dx})
            cpu.reg_write(UC_X86_REG_AX, 0)
            cpu.reg_write(UC_X86_REG_DX, 0)
        elif function == 0x3F:
            actual = min(cx, payload_size - read_cursor)
            chunk = source_bytes[read_cursor : read_cursor + actual]
            cpu.mem_write(ds * 16 + dx, chunk)
            dos_calls.append(
                {
                    "op": "read",
                    "handle": bx,
                    "count": cx,
                    "actual": actual,
                    "ds": ds,
                    "dx": dx,
                }
            )
            read_cursor += actual
            cpu.reg_write(UC_X86_REG_AX, actual)
        elif function == 0x3E:
            dos_calls.append({"op": "close", "handle": bx})
            cpu.reg_write(UC_X86_REG_AX, 0)
        elif ax == 0x3C00:
            name = bytes(cpu.mem_read(ds * 16 + dx, 8))
            dos_calls.append({"op": "create", "name": name})
            cpu.reg_write(UC_X86_REG_AX, temp_handle)
        elif function == 0x40:
            chunk = bytes(cpu.mem_read(ds * 16 + dx, cx))
            written_chunks.append(chunk)
            dos_calls.append({"op": "write", "handle": bx, "count": cx})
            cpu.reg_write(UC_X86_REG_AX, cx)
        else:
            raise AssertionError(f"unexpected INT 21h AX={ax:#x}")
        cpu.reg_write(UC_X86_REG_EFLAGS, cpu.reg_read(UC_X86_REG_EFLAGS) & ~1)

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(branch["routine"][0], 0, count=200_000)
    except UcError as error:
        raise RuntimeError(
            f"execution failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

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
        "es": 0x4800,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    for name in ("eax", "ebx", "ecx", "edx", "esi", "edi"):
        actual_registers[name] &= 0xFFFF
        expected_registers[name] &= 0xFFFF
    assert actual_registers == expected_registers, (
        actual_registers,
        expected_registers,
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(0, 0x20000)) == executable_snapshot
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(data)
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(fs_data)
    assert bytes(machine.mem_read(UNOWNED_SEGMENT * 16, SEGMENT_SIZE)) == bytes(unowned)
    assert (
        bytes(
            machine.mem_read(
                STACK_SEGMENT * 16 + STACK_POINTER + 4, len(STACK_SENTINEL)
            )
        )
        == STACK_SENTINEL
    )
    game_ranges = [
        (fields["archive_offset"], fields["archive_offset"] + 4),
        (fields["remaining"], fields["remaining"] + 4),
        (fields["backend_position"], fields["backend_position"] + 4),
        (fields["xms_request"], fields["xms_request"] + 16),
        (fields["backend"], fields["backend"] + 1),
        (fields["page_count"], fields["page_count"] + 2),
        (fields["final_page_bytes"], fields["final_page_bytes"] + 2),
        (fields["music_changed"], fields["music_changed"] + 1),
        (fields["start_requested"], fields["start_requested"] + 1),
        (fields["temp_handle"], fields["temp_handle"] + 2),
        (
            fields["prompt_destination"],
            fields["prompt_destination"] + len(PROMPT),
        ),
        (fields["text_end"], fields["text_end"] + 2),
        (fields["prompt_mode"], fields["prompt_mode"] + 1),
        (fields["prompt_phase"], fields["prompt_phase"] + 2),
        (fields["prompt_hold"], fields["prompt_hold"] + 1),
        (fields["framebuffer"], fields["framebuffer"] + 4),
        (fields["prompt_defer"], fields["prompt_defer"] + 1),
    ]
    game_after = bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
    assert_unchanged_outside_ranges(bytes(game), game_after, game_ranges)

    expected_storage = bytearray(storage)
    expected_page_frame = bytearray(page_frame)
    cursor = 0
    for chunk in row["read_chunks"] if active else []:
        destination = expected_page_frame if backend == "ems" else expected_storage
        offset = 0 if backend == "ems" else 0x1000
        destination[offset : offset + chunk] = source_bytes[cursor : cursor + chunk]
        cursor += chunk
    assert bytes(machine.mem_read(STORAGE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_storage
    )
    assert bytes(machine.mem_read(PAGE_FRAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_page_frame
    )

    allowed = [
        (STACK_SEGMENT * 16 + 0xFED0, STACK_SEGMENT * 16 + STACK_POINTER),
        *[
            (GAME_SEGMENT * 16 + start, GAME_SEGMENT * 16 + stop)
            for start, stop in game_ranges
        ],
        (STORAGE_SEGMENT * 16 + 0x1000, STORAGE_SEGMENT * 16 + 0x9000),
        (PAGE_FRAME_SEGMENT * 16, PAGE_FRAME_SEGMENT * 16 + 0x8000),
    ]
    assert all(
        any(lo <= address and address + size <= hi for lo, hi in allowed)
        for address, size in writes
    )

    sites = conditional_sites(executable, branch_name)
    internal_trace = [
        address
        for address in trace
        if branch["routine"][0] <= address < branch["routine"][1]
    ]
    edges = {
        (address, following)
        for address, following in itertools.pairwise(internal_trace)
        if address in sites and following in sites[address]
    }

    if not active:
        assert not (
            path_calls
            or lookup_calls
            or reveal_calls
            or directory_calls
            or xms_calls
            or dos_calls
            or ems_calls
        )
        normalized = {
            key: row[key]
            for key in (
                "name",
                "sound_enabled",
                "channel_active",
                "active",
                "source_kind",
                "backend",
                "payload_bytes",
                "read_chunks",
                "page_count",
                "final_page_bytes",
            )
        }
        normalized["defined_flags"] = final_flags(machine)
        return normalized, edges

    assert path_calls == [
        {"dx": PATH_OFFSET, "si": PATH_OFFSET, "ds": DATA_SEGMENT, "es": GAME_SEGMENT}
    ]
    assert lookup_calls == (
        [] if embedded else [{"si": PATH_OFFSET, "ds": DATA_SEGMENT}]
    )
    assert reveal_calls == [
        {
            "eax": 0,
            "ds": GAME_SEGMENT,
            "es": GAME_SEGMENT,
            "text": PROMPT,
            "text_end": fields["prompt_destination"] + 0x12,
            "mode": 2,
            "phase": 0,
            "hold": 0,
            "framebuffer": ALTERNATE_FRAMEBUFFER,
        }
    ]

    effective_handle = source_handle if embedded else opened_handle
    expected_seek_offset = (archive_offset & 0xFFFF0000) | (
        (archive_offset + 0x1A) & 0xFFFF
    )
    assert [call for call in dos_calls if call["op"] == "seek"] == [
        {"op": "seek", "handle": effective_handle, "offset": expected_seek_offset}
    ]
    expected_chunks = list(row["read_chunks"])
    assert [
        call["actual"] for call in dos_calls if call["op"] == "read"
    ] == expected_chunks
    assert read_cursor == payload_size

    raw_page_count, raw_final_bytes, raw_page_bytes = expected_storage_geometry(
        backend, expected_chunks, sequel, ultrasnd
    )
    assert read_word(machine, fields["page_count"]) == raw_page_count
    assert read_word(machine, fields["final_page_bytes"]) == raw_final_bytes
    assert (
        machine.mem_read(GAME_SEGMENT * 16 + fields["backend"], 1)[0]
        == {
            "ems": 0,
            "xms": 1,
            "file": 2,
        }[backend]
    )
    assert read_dword(machine, fields["remaining"]) == 0
    assert machine.mem_read(GAME_SEGMENT * 16 + fields["music_changed"], 1)[0] == 0
    assert machine.mem_read(GAME_SEGMENT * 16 + fields["start_requested"], 1)[0] == 1
    assert (
        bytes(machine.mem_read(GAME_SEGMENT * 16 + fields["framebuffer"], 4))
        == ORIGINAL_FRAMEBUFFER
    )
    assert machine.mem_read(GAME_SEGMENT * 16 + fields["prompt_mode"], 1)[0] == 0
    assert machine.mem_read(GAME_SEGMENT * 16 + fields["prompt_defer"], 1)[0] == 0

    if backend == "ems":
        expected_maps = []
        for logical_page in range(0, len(expected_chunks) * 2, 2):
            expected_maps.extend(
                [
                    {
                        "ax": 0x4400,
                        "page": logical_page,
                        "handle": ems_handle,
                        "ds": PAGE_FRAME_SEGMENT,
                    },
                    {
                        "ax": 0x4401,
                        "page": logical_page + 1,
                        "handle": ems_handle,
                        "ds": PAGE_FRAME_SEGMENT,
                    },
                ]
            )
        assert ems_calls == expected_maps
        assert not xms_calls and not written_chunks
        assert (
            read_word(machine, fields["backend_position"]) == len(expected_chunks) * 2
        )
    elif backend == "xms":
        assert not ems_calls and not written_chunks
        nonempty_chunks = [chunk for chunk in expected_chunks if chunk]
        assert len(xms_calls) == len(nonempty_chunks)
        destination_offset = 0
        for call, chunk in zip(xms_calls, nonempty_chunks, strict=True):
            assert call == {
                "eax": 0x00000B00,
                "ds": GAME_SEGMENT,
                "si": fields["xms_request"],
                "request": struct.pack(
                    "<IHIHI",
                    chunk + (chunk & 1),
                    0,
                    (STORAGE_SEGMENT << 16) | 0x1000,
                    xms_handle,
                    destination_offset,
                ),
            }
            destination_offset += 0x8000
        assert (
            read_dword(machine, fields["backend_position"])
            == len(nonempty_chunks) * 0x8000
        )
    else:
        assert not ems_calls and not xms_calls
        assert b"".join(written_chunks) == source_bytes
        assert directory_calls == [{"ds": GAME_SEGMENT, "es": GAME_SEGMENT}]
        assert [call for call in dos_calls if call["op"] == "create"] == [
            {"op": "create", "name": b"mus.snd\0"}
        ]
        assert read_word(machine, fields["temp_handle"]) == temp_handle

    expected_close = [] if embedded else [effective_handle]
    if backend == "file" and old_temp_handle:
        expected_close.insert(0, old_temp_handle)
    assert [
        call["handle"] for call in dos_calls if call["op"] == "close"
    ] == expected_close

    normalized = {
        key: row[key]
        for key in (
            "name",
            "sound_enabled",
            "channel_active",
            "active",
            "source_kind",
            "backend",
            "payload_bytes",
            "read_chunks",
            "page_count",
            "final_page_bytes",
        )
    }
    normalized.update(
        {
            "ultrasnd": int(ultrasnd),
            "dos_page_bytes": raw_page_bytes,
            "dos_page_count": raw_page_count,
            "dos_final_page_bytes": raw_final_bytes,
            "defined_flags": final_flags(machine),
        }
    )
    return normalized, edges


def assert_body(executable: bytes, branch_name: str) -> None:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    body = executable[start:stop]
    assert hashlib.sha256(body).hexdigest() == branch["sha256"]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    assert len(list(decoder.disasm(body, start))) == branch["instruction_count"]


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
    assert_body(commander, "commander")
    assert_body(sequel, "sequel")

    commander_rows = json.loads(COMMANDER_FIXTURE.read_text())
    sequel_rows = []
    coverage: set[tuple[int, int]] = set()
    for case_index, row in enumerate(commander_rows):
        commander_row, _commander_edges = execute_case(
            commander, "commander", row, case_index
        )
        sequel_row, sequel_edges = execute_case(sequel, "sequel", row, case_index)
        sequel_logical = {
            key: value
            for key, value in sequel_row.items()
            if not key.startswith("dos_") and key != "ultrasnd"
        }
        commander_logical = {
            key: value
            for key, value in commander_row.items()
            if not key.startswith("dos_") and key != "ultrasnd"
        }
        assert sequel_logical == commander_logical, (
            sequel_logical,
            commander_logical,
        )
        sequel_rows.append(sequel_row)
        coverage.update(sequel_edges)
    for case_index, row in enumerate(EXTRA_CASES, start=len(commander_rows)):
        sequel_row, sequel_edges = execute_case(sequel, "sequel", row, case_index)
        sequel_rows.append(sequel_row)
        coverage.update(sequel_edges)

    sites = conditional_sites(sequel, "sequel")
    expected_edges = {
        (address, destination)
        for address, destinations in sites.items()
        for destination in destinations
    }
    missing = expected_edges - coverage
    assert not missing, f"missing BBB conditional edges: {sorted(missing)}"
    assert len(coverage) == len(expected_edges)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in sequel_rows
        )
    )
    print(
        f"verified {len(commander_rows)} shared and {len(EXTRA_CASES)} BBB-only "
        f"audio source-load cases across {len(expected_edges)} conditional edges"
    )


if __name__ == "__main__":
    main()
