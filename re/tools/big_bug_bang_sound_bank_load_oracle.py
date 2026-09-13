#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang SND bank loading."""

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
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_c005_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES: dict[str, dict[str, Any]] = {
    "commander": {
        "routine": (0xC005, 0xC1E6),
        "sha256": "c792ab3ce5c558301f850256089f963a2738a41bad701b7cc866a4f1629ee662",
        "instruction_count": 187,
        "runtime_segment": 0x1CE,
        "return_ips": {
            "path": 0xC025,
            "lookup": 0xC031,
            "directory": 0xC19C,
            "xms": 0xC177,
        },
        "fields": {
            "sound_enabled": 0x0ADE,
            "embedded": 0x0AE2,
            "remaining": 0x0A92,
            "ems_handle": 0x0A5C,
            "xms_handle": 0x0A5A,
            "page_frame": 0x0A66,
            "xms_driver": 0x0A4A,
            "backend_position": 0x0A4E,
            "work_pointer": 0x0ABC,
            "bank_pointer": 0x0BB3,
            "source_header": 0x0BBB,
            "compact_table": 0x0BBF,
            "source_table": 0x0F1A,
            "stream_count": 0x0C53,
            "stream_table": 0x0C57,
            "xms_request": 0x0A6C,
            "temp_handle": 0x0C47,
            "temp_name": 0x00A6,
        },
    },
    "sequel": {
        "routine": (0xD7EC, 0xD9F3),
        "sha256": "f1d0c931b19687bd7b14a66c1623965f3e1308dcaa81296c9167a1f09495c138",
        "instruction_count": 197,
        "runtime_segment": 0x1E6,
        "return_ips": {
            "path": 0xD80C,
            "lookup": 0xD818,
            "directory": 0xD9A9,
            "xms": 0xD984,
            "resident_ultrasnd": 0xD89C,
            "stream_ultrasnd": 0xD8C7,
        },
        "helpers": {
            "resident_ultrasnd": 0xDDF8,
            "stream_ultrasnd": 0xDC92,
        },
        "fields": {
            "sound_enabled": 0x0CE7,
            "embedded": 0x0CEB,
            "remaining": 0x0C8A,
            "ems_handle": 0x0C54,
            "xms_handle": 0x0C52,
            "page_frame": 0x0C5E,
            "xms_driver": 0x0C42,
            "backend_position": 0x0C46,
            "work_pointer": 0x0CB4,
            "bank_pointer": 0x0DBD,
            "source_header": 0x0DC5,
            "compact_table": 0x0DC9,
            "source_table": 0x1168,
            "stream_count": 0x0E5D,
            "stream_table": 0x0E61,
            "xms_request": 0x0C64,
            "temp_handle": 0x0E51,
            "temp_name": 0x00A5,
            "ultrasnd": 0x0F1F,
        },
    },
}

ULTRASND_CASES = (
    {
        "name": "ultrasnd_resident_external",
        "sound_enabled": 1,
        "mode": 0,
        "source_kind": "standalone",
        "backend": "memory",
        "clip_count": 3,
        "payload_bytes": 0x3456,
        "payload_chunks": [0x3456],
        "ultrasnd": 1,
    },
    {
        "name": "ultrasnd_streamed_embedded",
        "sound_enabled": 1,
        "mode": 1,
        "source_kind": "embedded",
        "backend": "memory",
        "clip_count": 3,
        "payload_bytes": 0x4321,
        "payload_chunks": [0x4321],
        "ultrasnd": 1,
    },
)

MACHINE_SIZE = 0xB0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x2000
BANK_SEGMENT = 0x3000
WORK_SEGMENT = 0x4000
PAGE_FRAME_SEGMENT = 0x5000
FS_SEGMENT = 0x6000
CALLBACK_SEGMENT = 0x7000
STACK_SEGMENT = GAME_SEGMENT
UNOWNED_SEGMENT = 0xA000
BANK_OFFSET = 0x1000
WORK_OFFSET = 0x0200
STAGING_OFFSET = WORK_OFFSET + 0x7D00
CALLBACK_OFFSET = 0x0100
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
PATH_OFFSET = 0x0D06
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
    mutable = bytearray(len(before))
    for start, stop in ranges:
        mutable[start:stop] = b"\x01" * (stop - start)
    assert all(
        mutable[index] or before_byte == after[index]
        for index, before_byte in enumerate(before)
    )


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
        sites[instruction.address] = (
            instruction.operands[0].imm & 0xFFFF,
            instruction.address + instruction.size,
        )
    return sites


def build_source(
    row: dict[str, Any], case_index: int
) -> tuple[bytes, bytes, bytes, list[int]]:
    clip_count = int(row["clip_count"] or 0)
    payload_size = int(row["payload_bytes"] or 0)
    offsets = (
        [payload_size * index // clip_count for index in range(clip_count + 1)]
        if clip_count
        else [0]
    )
    table = b"".join(struct.pack("<I", offset) for offset in offsets)
    header = struct.pack("<HBB", clip_count, 90, 165)
    payload = bytes(
        (index * 31 + case_index * 43 + 7) & 0xFF for index in range(payload_size)
    )
    return header + table + payload, table, payload, offsets


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
    active = bool(int(row["sound_enabled"]) & 1)
    mode = int(row["mode"])
    embedded = row["source_kind"] == "embedded"
    backend = str(row["backend"] or "memory")
    source, table, payload, offsets = build_source(row, case_index)
    clip_count = int(row["clip_count"] or 0)
    payload_size = len(payload)
    source_handle = 0x2300 + case_index
    opened_handle = 0x4500 + case_index
    temp_handle = 0x6700 + case_index
    old_temp_handle = 0x1357 if backend == "file" and active and not embedded else 0
    ems_handle = 0x1111 if backend == "ems" else 0xFFFF
    xms_handle = 0x2222 if backend == "xms" else 0xFFFF

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | mode,
        UC_X86_REG_EBX: 0xB2B23456,
        UC_X86_REG_ECX: 0xC3C34567,
        UC_X86_REG_EDX: 0xD4D45678,
        UC_X86_REG_ESI: 0xE5E50000 | PATH_OFFSET,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME_SEGMENT,
        UC_X86_REG_ES: 0x4800,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202 | (case_index & 1),
    }
    game = seeded_segment(case_index + 1)
    bank = seeded_segment(case_index + 17)
    work = seeded_segment(case_index + 33)
    page_frame = seeded_segment(case_index + 49)
    fs_data = seeded_segment(case_index + 65)
    unowned = seeded_segment(case_index + 97)
    path = f"sn\\bank{case_index}.snd".encode("ascii") + b"\0"
    game[PATH_OFFSET : PATH_OFFSET + len(path)] = path
    game[fields["sound_enabled"]] = int(row["sound_enabled"])
    game[fields["embedded"]] = int(embedded)
    if sequel:
        game[fields["ultrasnd"]] = int(ultrasnd)
    write_dword(game, fields["remaining"], len(source) if embedded else 0xDEADBEEF)
    write_word(game, fields["ems_handle"], ems_handle)
    write_word(game, fields["xms_handle"], xms_handle)
    write_word(game, fields["page_frame"], PAGE_FRAME_SEGMENT)
    write_word(game, fields["xms_driver"], CALLBACK_OFFSET)
    write_word(game, fields["xms_driver"] + 2, CALLBACK_SEGMENT)
    write_dword(game, fields["work_pointer"], (WORK_SEGMENT << 16) | WORK_OFFSET)
    write_dword(game, fields["bank_pointer"], (BANK_SEGMENT << 16) | BANK_OFFSET)
    write_word(game, fields["temp_handle"], old_temp_handle)
    game[fields["temp_name"] : fields["temp_name"] + 8] = b"son.snd\0"
    game[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    segments = {
        GAME_SEGMENT: game,
        BANK_SEGMENT: bank,
        WORK_SEGMENT: work,
        PAGE_FRAME_SEGMENT: page_frame,
        FS_SEGMENT: fs_data,
        UNOWNED_SEGMENT: unowned,
    }
    for segment, contents in segments.items():
        machine.mem_write(segment * 16, bytes(contents))

    callbacks = {
        "path": (int(branch["runtime_segment"]), 0x03B3),
        "lookup": (int(branch["runtime_segment"]), 0x05EA),
        "directory": (int(branch["runtime_segment"]), 0x04E3),
        "xms": (CALLBACK_SEGMENT, CALLBACK_OFFSET),
    }
    callback_addresses = {
        segment * 16 + offset: name for name, (segment, offset) in callbacks.items()
    }
    for segment, offset in callbacks.values():
        machine.mem_write(segment * 16 + offset, b"\xcb")
    helper_addresses = {}
    if sequel:
        helper_addresses = {
            address: name for name, address in branch["helpers"].items()
        }
        for address in helper_addresses:
            machine.mem_write(address, b"\xc3")
    executable_snapshot = bytes(machine.mem_read(0, 0x20000))
    for register, value in initial.items():
        machine.reg_write(register, value)

    path_calls: list[dict[str, int]] = []
    lookup_calls: list[dict[str, int]] = []
    directory_calls: list[dict[str, int]] = []
    xms_calls: list[dict[str, Any]] = []
    helper_calls: list[dict[str, Any]] = []
    dos_calls: list[dict[str, Any]] = []
    ems_calls: list[dict[str, int]] = []
    written_chunks: list[bytes] = []
    writes: list[tuple[int, int]] = []
    trace: list[int] = []
    read_cursor = 0
    reached_return = False

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
        if callback_name is not None:
            expected_sp = (
                0xFEE2
                if callback_name == "xms"
                else 0xFEE4
                if callback_name == "directory"
                else 0xFEE6
            )
            assert cpu.reg_read(UC_X86_REG_SP) == expected_sp
            assert far_frame(cpu) == (branch["return_ips"][callback_name], 0)
            if callback_name == "path":
                path_calls.append(
                    {
                        "dx": cpu.reg_read(UC_X86_REG_DX),
                        "si": cpu.reg_read(UC_X86_REG_SI),
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                    }
                )
                cpu.reg_write(UC_X86_REG_BX, source_handle)
            elif callback_name == "lookup":
                lookup_calls.append(
                    {
                        "si": cpu.reg_read(UC_X86_REG_SI),
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                    }
                )
                cpu.reg_write(UC_X86_REG_EBP, len(source))
            elif callback_name == "directory":
                directory_calls.append(
                    {
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "es": cpu.reg_read(UC_X86_REG_ES),
                    }
                )
            else:
                xms_calls.append(
                    {
                        "eax": cpu.reg_read(UC_X86_REG_EAX),
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "si": cpu.reg_read(UC_X86_REG_SI),
                        "request": bytes(
                            cpu.mem_read(GAME_SEGMENT * 16 + fields["xms_request"], 16)
                        ),
                    }
                )
            return
        helper_name = helper_addresses.get(address)
        if helper_name is None:
            return
        assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEA
        return_ip = struct.unpack("<H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEA, 2))[0]
        assert return_ip == branch["return_ips"][helper_name]
        helper_calls.append(
            {
                "name": helper_name,
                "eax": cpu.reg_read(UC_X86_REG_EAX),
                "ebx": cpu.reg_read(UC_X86_REG_EBX),
                "ecx": cpu.reg_read(UC_X86_REG_ECX),
                "ebp": cpu.reg_read(UC_X86_REG_EBP),
                "si": cpu.reg_read(UC_X86_REG_SI),
                "edi": cpu.reg_read(UC_X86_REG_EDI),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
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
        elif function == 0x3F:
            actual = min(cx, len(source) - read_cursor)
            chunk = source[read_cursor : read_cursor + actual]
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
            dos_calls.append(
                {"op": "create", "name": bytes(cpu.mem_read(ds * 16 + dx, 8))}
            )
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
        machine.emu_start(branch["routine"][0], 0, count=250_000)
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
        "ds": GAME_SEGMENT,
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
    assert actual_registers == expected_registers
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(0, 0x20000)) == executable_snapshot
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
    base_row = {
        key: row[key]
        for key in (
            "name",
            "sound_enabled",
            "mode",
            "source_kind",
            "backend",
            "clip_count",
            "payload_bytes",
            "payload_chunks",
        )
    }
    base_row["defined_flags"] = final_flags(machine)
    if not active:
        assert not (
            path_calls
            or lookup_calls
            or directory_calls
            or xms_calls
            or helper_calls
            or dos_calls
            or ems_calls
        )
        assert_unchanged_outside_ranges(
            bytes(game),
            bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)),
            [(0xFED0, STACK_POINTER)],
        )
        assert bytes(machine.mem_read(BANK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(bank)
        assert bytes(machine.mem_read(WORK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(work)
        assert bytes(machine.mem_read(PAGE_FRAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            page_frame
        )
        return base_row, edges

    assert path_calls == [{"dx": PATH_OFFSET, "si": PATH_OFFSET, "ds": GAME_SEGMENT}]
    assert lookup_calls == (
        [] if embedded else [{"si": PATH_OFFSET, "ds": GAME_SEGMENT}]
    )
    effective_handle = source_handle if embedded else opened_handle
    reads = [call for call in dos_calls if call["op"] == "read"]
    assert [call["count"] for call in reads[:2]] == [4, len(table)]
    assert (
        bytes(machine.mem_read(GAME_SEGMENT * 16 + fields["source_header"], 4))
        == source[:4]
    )
    assert (
        bytes(machine.mem_read(GAME_SEGMENT * 16 + fields["source_table"], len(table)))
        == table
    )

    if mode == 0:
        expected_chunks = [payload_size]
        compact = b"".join(
            struct.pack(
                "<HH",
                offsets[index] & 0xFFFF,
                ((offsets[index + 1] - offsets[index]) - 1) & 0xFFFF,
            )
            for index in range(clip_count)
        )
        actual_compact = bytes(
            machine.mem_read(GAME_SEGMENT * 16 + fields["compact_table"], len(compact))
        )
        assert actual_compact == compact, (
            row["name"],
            actual_compact,
            compact,
        )
        assert (
            bytes(machine.mem_read(BANK_SEGMENT * 16 + BANK_OFFSET, payload_size))
            == payload
        )
        if ultrasnd:
            expected_helpers = [
                {
                    "name": "resident_ultrasnd",
                    "eax": payload_size,
                    "ebx": 0,
                    "ecx": payload_size,
                    "ebp": fields["compact_table"] + clip_count * 4,
                    "si": BANK_OFFSET,
                    "edi": initial[UC_X86_REG_EDI],
                    "ds": BANK_SEGMENT,
                    "es": 0x4800,
                }
            ]
            assert helper_calls == expected_helpers, (helper_calls, expected_helpers)
        else:
            assert not helper_calls
        assert (
            not ems_calls
            and not xms_calls
            and not written_chunks
            and not directory_calls
        )
    else:
        assert read_word(machine, fields["stream_count"]) == clip_count
        assert (
            bytes(
                machine.mem_read(GAME_SEGMENT * 16 + fields["stream_table"], len(table))
            )
            == table
        )
        if ultrasnd:
            expected_chunks = []
            expected_helpers = [
                {
                    "name": "stream_ultrasnd",
                    "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | GAME_SEGMENT,
                    "ebx": (initial[UC_X86_REG_EBX] & 0xFFFF0000) | source_handle,
                    "ecx": 0,
                    "ebp": payload_size,
                    "si": fields["source_table"] + len(table),
                    "edi": (initial[UC_X86_REG_EDI] & 0xFFFF0000)
                    | (fields["stream_table"] + len(table)),
                    "ds": GAME_SEGMENT,
                    "es": GAME_SEGMENT,
                }
            ]
            assert helper_calls == expected_helpers, (helper_calls, expected_helpers)
            assert (
                not ems_calls
                and not xms_calls
                and not written_chunks
                and not directory_calls
            )
        else:
            expected_chunks = list(row["payload_chunks"])
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
            elif backend == "xms":
                assert not ems_calls and not written_chunks
                assert len(xms_calls) == len(expected_chunks)
                destination_offset = 0
                for call, chunk in zip(xms_calls, expected_chunks, strict=True):
                    assert call == {
                        "eax": 0x00000B00,
                        "ds": GAME_SEGMENT,
                        "si": fields["xms_request"],
                        "request": struct.pack(
                            "<IHIHI",
                            chunk + (chunk & 1),
                            0,
                            (WORK_SEGMENT << 16) | STAGING_OFFSET,
                            xms_handle,
                            destination_offset,
                        ),
                    }
                    destination_offset += 0x7D00
            else:
                assert not ems_calls and not xms_calls
                assert b"".join(written_chunks) == payload
                assert directory_calls == [{"ds": GAME_SEGMENT, "es": GAME_SEGMENT}]
                assert [call for call in dos_calls if call["op"] == "create"] == [
                    {"op": "create", "name": b"son.snd\0"}
                ]
                assert read_word(machine, fields["temp_handle"]) == temp_handle

    assert [call["actual"] for call in reads[2:]] == expected_chunks
    assert read_cursor == 4 + len(table) + sum(expected_chunks)
    expected_close = (
        [] if embedded else [0 if ultrasnd and mode == 0 else effective_handle]
    )
    if mode and backend == "file" and old_temp_handle and not ultrasnd:
        expected_close.insert(0, old_temp_handle)
    assert [
        call["handle"] for call in dos_calls if call["op"] == "close"
    ] == expected_close
    assert read_dword(machine, fields["remaining"]) == payload_size

    game_ranges = [
        (fields["remaining"], fields["remaining"] + 4),
        (fields["backend_position"], fields["backend_position"] + 4),
        (fields["source_header"], fields["source_header"] + 4),
        (fields["compact_table"], fields["compact_table"] + max(1, clip_count * 4)),
        (fields["source_table"], fields["source_table"] + len(table)),
        (fields["stream_count"], fields["stream_count"] + 2),
        (fields["stream_table"], fields["stream_table"] + len(table)),
        (fields["xms_request"], fields["xms_request"] + 16),
        (fields["temp_handle"], fields["temp_handle"] + 2),
        (0xFED0, STACK_POINTER),
    ]
    game_after = bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
    assert_unchanged_outside_ranges(bytes(game), game_after, game_ranges)
    expected_bank = bytearray(bank)
    expected_work = bytearray(work)
    expected_page_frame = bytearray(page_frame)
    cursor = 0
    for chunk in expected_chunks:
        if mode == 0:
            destination = expected_bank
            offset = BANK_OFFSET
        elif backend == "ems":
            destination = expected_page_frame
            offset = 0
        else:
            destination = expected_work
            offset = STAGING_OFFSET
        destination[offset : offset + chunk] = payload[cursor : cursor + chunk]
        cursor += chunk
    assert bytes(machine.mem_read(BANK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_bank
    )
    assert bytes(machine.mem_read(WORK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_work
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
        (
            BANK_SEGMENT * 16 + BANK_OFFSET,
            BANK_SEGMENT * 16 + BANK_OFFSET + payload_size,
        ),
        (
            WORK_SEGMENT * 16 + STAGING_OFFSET,
            WORK_SEGMENT * 16 + STAGING_OFFSET + 0x7D00,
        ),
        (PAGE_FRAME_SEGMENT * 16, PAGE_FRAME_SEGMENT * 16 + 0x8000),
    ]
    assert all(
        any(lo <= address and address + size <= hi for lo, hi in allowed)
        for address, size in writes
    )
    base_row.update(
        {
            "ultrasnd": int(ultrasnd),
            "raw_backend": "ultrasnd" if ultrasnd else backend,
            "raw_payload_chunks": expected_chunks,
        }
    )
    return base_row, edges


def assert_body(executable: bytes, branch_name: str) -> None:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    body = executable[start:stop]
    assert hashlib.sha256(body).hexdigest() == branch["sha256"]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    instruction_count = len(list(decoder.disasm(body, start)))
    assert instruction_count == branch["instruction_count"], instruction_count


def logical_row(row: dict[str, Any]) -> dict[str, Any]:
    return {
        key: value
        for key, value in row.items()
        if key not in {"defined_flags", "ultrasnd", "raw_backend", "raw_payload_chunks"}
    }


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
        assert logical_row(commander_row) == logical_row(sequel_row)
        assert commander_row["defined_flags"] == sequel_row["defined_flags"]
        sequel_rows.append(sequel_row)
        coverage.update(sequel_edges)
    for case_index, row in enumerate(ULTRASND_CASES, start=len(commander_rows)):
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
        f"verified {len(commander_rows)} shared and {len(ULTRASND_CASES)} BBB-only "
        f"sound-bank cases across {len(expected_edges)} conditional edges"
    )


if __name__ == "__main__":
    main()
