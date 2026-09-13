#!/usr/bin/env python3
"""Verify BBB's remaining non-background DESCRIPT parser handlers."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
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


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = (
    ROOT / "re/tools/oracle_vectors/big_bug_bang_complex_parser_handlers.json"
)
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

GLOBALS_SEGMENT = 0x3000
INPUT_SEGMENT = 0x4000
STACK_SEGMENT = 0x9000
BUFFER_SEGMENT = 0x6000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
SOUND_LOADER = 0x0C74 * 16 + 0x08AC
IDLE_PATH_LOADER = 0x01E6 * 16 + 0x0717
IDLE_FILE_LOADER = 0x01E6 * 16 + 0x0626

ROUTINES = (
    (0x8680, 0x86B1, "sound_bank"),
    (0x86C6, 0x86FC, "talk_clip"),
    (0x872C, 0x8796, "idle_clip"),
    (0x8796, 0x87B8, "sequence_video"),
    (0x87B8, 0x87CA, "sequence_subtitle"),
    (0x87EB, 0x8825, "music"),
)
ROUTINE_HASHES = {
    0x8680: "67e7ef104a07efac367741f69fabc4f565055c2fae46c204bcb24b128f2a2314",
    0x86C6: "df5447056456fd2ee88a6d13289ade1ee9f231a4bced62338842e6314cde4e9a",
    0x872C: "b4bc11f16d74e82d2d589a0dfc2f04bc63c36e0ed06e667bbe7148d7736089e2",
    0x8796: "45d86b57d3e3eeffc22092079bb73e21ddf9002d98e3b17e10c01edc8cc03fd1",
    0x87B8: "beb7b334dd3658c8424d95a139bd9c5c7457f962c14e643dc091ca20432fb710",
    0x87EB: "f3297fadb1830c3afd9cadd506b09919ec8d386a594c998f71f46e827b1d7dc7",
}

GENERAL_REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
}
SEGMENT_REGISTERS = {
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}
INITIAL_GENERAL = {
    "eax": 0xA5A51234,
    "ebx": 0xB6B62345,
    "ecx": 0xC7C73456,
    "edx": 0xD8D84567,
    "esi": 0xE9E90000,
    "edi": 0xFAFA4567,
    "ebp": 0xABCD789A,
}


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def set_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_wrapped(data: bytearray, offset: int, content: bytes) -> None:
    for index, value in enumerate(content):
        data[(offset + index) & 0xFFFF] = value


def read_wrapped(data: bytes | bytearray, offset: int, length: int) -> bytes:
    return bytes(data[(offset + index) & 0xFFFF] for index in range(length))


def offsets(offset: int, length: int) -> set[int]:
    return {(offset + index) & 0xFFFF for index in range(length)}


def absolute(segment: int, relative: set[int]) -> set[int]:
    return {segment * 16 + offset for offset in relative}


def low_word(value: int, replacement: int) -> int:
    return (value & 0xFFFF0000) | (replacement & 0xFFFF)


def register_snapshot(machine: Uc) -> dict[str, str]:
    result = {
        name: f"0x{machine.reg_read(register):08x}"
        for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: f"0x{machine.reg_read(register):04x}"
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    result["flags"] = f"0x{machine.reg_read(UC_X86_REG_EFLAGS):08x}"
    return result


def execute(
    executable: bytes,
    entry: int,
    end: int,
    globals_before: bytearray,
    input_before: bytearray,
    source_offset: int,
    allowed_global_offsets: set[int],
    required_global_offsets: set[int],
    *,
    flags: int = 0x0202,
    callback_addresses: tuple[int, ...] = (),
    callback: Any = None,
) -> tuple[Uc, bytes, dict[str, str], dict[str, str], set[int]]:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    expected_image = bytearray(executable)
    for address in callback_addresses:
        machine.mem_write(address, b"\xcb")
        if address < len(expected_image):
            expected_image[address] = 0xCB
    machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(INPUT_SEGMENT * 16, bytes(input_before))
    machine.mem_write(BUFFER_SEGMENT * 16, bytes([0x69]) * SEGMENT_SIZE)
    machine.mem_write(
        STACK_SEGMENT * 16 + CALLER_SP,
        struct.pack("<H", RETURN_OFFSET) + STACK_SENTINEL,
    )

    for name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, INITIAL_GENERAL[name])
    machine.reg_write(UC_X86_REG_ESI, low_word(INITIAL_GENERAL["esi"], source_offset))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_DS, INPUT_SEGMENT)
    machine.reg_write(UC_X86_REG_ES, GLOBALS_SEGMENT)
    machine.reg_write(UC_X86_REG_FS, 0x5000)
    machine.reg_write(UC_X86_REG_GS, GLOBALS_SEGMENT)
    machine.reg_write(UC_X86_REG_SS, STACK_SEGMENT)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, flags)
    registers_before = register_snapshot(machine)

    allowed_writes = absolute(GLOBALS_SEGMENT, allowed_global_offsets)
    allowed_writes.update(
        range(STACK_SEGMENT * 16 + CALLER_SP - 16, STACK_SEGMENT * 16 + CALLER_SP)
    )
    writes: set[int] = set()

    def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
        assert (
            entry <= address and address + size <= end
        ) or address in callback_addresses, hex(address)
        if address in callback_addresses and callback is not None:
            callback(cpu, address)

    def write_hook(
        _machine: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: Any,
    ) -> None:
        written = set(range(address, address + size))
        assert written <= allowed_writes, (hex(address), size)
        writes.update(written)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.emu_start(entry, RETURN_OFFSET, count=512)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == (CALLER_SP + 2) & 0xFFFF
    assert absolute(GLOBALS_SEGMENT, required_global_offsets) <= writes
    assert bytes(machine.mem_read(0, len(expected_image))) == expected_image
    assert bytes(machine.mem_read(INPUT_SEGMENT * 16, SEGMENT_SIZE)) == input_before
    assert (
        bytes(machine.mem_read(STACK_SEGMENT * 16 + CALLER_SP + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )
    globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
    differences = {
        index
        for index, (before, after) in enumerate(zip(globals_before, globals_after))
        if before != after
    }
    assert differences <= allowed_global_offsets, sorted(hex(item) for item in differences)
    return (
        machine,
        globals_after,
        registers_before,
        register_snapshot(machine),
        writes,
    )


def split_printable(payload: bytes) -> tuple[bytes, int]:
    for index, value in enumerate(payload):
        if value < 0x20 or value >= 0x80:
            return payload[:index], value
    raise AssertionError("printable stream has no stop byte")


def routine_rows(executable: bytes) -> list[dict[str, str]]:
    rows = []
    for entry, end, operation in ROUTINES:
        digest = hashlib.sha256(executable[entry:end]).hexdigest()
        assert digest == ROUTINE_HASHES[entry], hex(entry)
        rows.append(
            {
                "operation": operation,
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "body_sha256": digest,
            }
        )
    return rows


SOUND_CASES = (
    ("call_empty", 0x6800, b"\x00", 0x0000),
    ("skip_empty", 0x6820, b"\x00", 0x0001),
    ("call_text", 0x6840, b"BANK.SND\x1f", 0xA500),
    ("skip_text", 0x6880, b"SON.SND\x80", 0x5A01),
    ("call_max_printable", 0x68C0, b"\x7f\x00", 0xFFFE),
    ("skip_high", 0x68E0, b"\xff", 0x0003),
    ("call_script_wrap", 0xFFFE, b"AB\x00", 0x0000),
    ("skip_high_at_end", 0xFFFF, b"\x80", 0x8001),
)


def sound_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    destination = 0x0F57
    gate_offset = 0x2A33
    for name, source_offset, payload, presentation_state in SOUND_CASES:
        copied, stopping_byte = split_printable(payload)
        should_call = presentation_state & 1 == 0
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        write_wrapped(input_before, source_offset, payload)
        set_word(globals_before, gate_offset, presentation_state)
        globals_before[0x0F54:0x0F57] = b"sn/"
        destination_offsets = offsets(destination, len(copied) + 1)
        calls = []

        def capture(machine: Uc, address: int) -> None:
            calls.append(
                {
                    "address": f"0x{address:05x}",
                    "ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                }
            )

        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x8680,
            0x86B1,
            globals_before,
            input_before,
            source_offset,
            destination_offsets,
            destination_offsets,
            callback_addresses=(SOUND_LOADER,),
            callback=capture,
        )
        expected_calls = (
            [{"address": f"0x{SOUND_LOADER:05x}", "ax": 1, "ds": GLOBALS_SEGMENT, "si": 0x0F54}]
            if should_call
            else []
        )
        assert calls == expected_calls, (name, calls)
        assert read_wrapped(globals_after, destination, len(copied) + 1) == copied + b"\0"
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(
            INITIAL_GENERAL["esi"], (source_offset + len(copied)) & 0xFFFF
        )
        for register in ("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "sound_bank",
                "name": name,
                "source_offset": source_offset,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stopping_byte,
                "presentation_state": presentation_state,
                "loader_called": should_call,
                "calls": calls,
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


TALK_CASES = (
    ("id_01_empty", 0x6000, 0x01, b"\x00", False),
    ("id_02_text", 0x6080, 0x02, b"TALK.HNM\x1f", False),
    ("id_04_high_stop", 0x6100, 0x04, b"A\x80", False),
    ("id_ff_none", 0x6180, 0xFF, b"MG_SCR1.HNM\x00", False),
    ("id_80_invalid", 0x6200, 0x80, b"\xff", False),
    ("id_00_invalid", 0x6280, 0x00, b"FD\x00", False),
    ("source_wrap", 0xFFFD, 0x03, b"A\x00", False),
    ("inherited_sign_flag", 0x6380, 0x01, b"B\x00", True),
)


def talk_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for index, (name, source_offset, asset_id, detail, sign_flag) in enumerate(TALK_CASES):
        copied, stopping_byte = split_printable(detail)
        asset_cursor = 0x3000 + index * 8
        detail_cursor = 0x4000 + index * 0x20
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        stream = bytes([asset_id]) + detail
        write_wrapped(input_before, source_offset, stream)
        set_word(globals_before, 0x21FD, asset_cursor)
        set_word(globals_before, 0x21FB, detail_cursor)
        signed_id = asset_id if asset_id < 0x80 else asset_id - 0x100
        stored_id = (
            signed_id & 0xFFFF
            if sign_flag
            else (((signed_id & 0xFFFF) - 1) * 16 + 0x1025) & 0xFFFF
        )
        asset_offsets = offsets(asset_cursor, 2)
        detail_offsets = offsets(detail_cursor, len(copied) + 1)
        pointer_offsets = offsets(0x21FB, 4)
        allowed = asset_offsets | detail_offsets | pointer_offsets
        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x86C6,
            0x86FC,
            globals_before,
            input_before,
            source_offset,
            allowed,
            allowed,
            flags=0x0282 if sign_flag else 0x0202,
        )
        assert word(globals_after, asset_cursor) == stored_id
        assert word(globals_after, 0x21FD) == (asset_cursor + 4) & 0xFFFF
        assert word(globals_after, 0x21FB) == (detail_cursor + 0x1A) & 0xFFFF
        assert read_wrapped(globals_after, detail_cursor, len(copied) + 1) == copied + b"\0"
        final_source = (source_offset + 1 + len(copied)) & 0xFFFF
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(INITIAL_GENERAL["esi"], final_source)
        for register in ("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "talk_clip",
                "name": name,
                "source_offset": source_offset,
                "input_hex": stream.hex(),
                "asset_id": asset_id,
                "copied_hex": copied.hex(),
                "stopping_byte": stopping_byte,
                "sign_flag_before": sign_flag,
                "stored_asset_index": stored_id,
                "asset_cursor_before": asset_cursor,
                "asset_cursor_after": word(globals_after, 0x21FD),
                "detail_cursor_before": detail_cursor,
                "detail_cursor_after": word(globals_after, 0x21FB),
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


IDLE_CASES = (
    ("busy_empty", 0x6200, 0x01, b"\x00", 1, "path"),
    ("busy_none", 0x6280, 0xFF, b"NEG\x80", 0xA503, "file"),
    ("path_text", 0x6300, 0x02, b"FILE.BIN\x00", 0, "path"),
    ("path_high", 0x6380, 0x03, b"A\x1f", 0, "path"),
    ("file_text", 0x6400, 0x04, b"ASSET.DAT\x00", 0, "file"),
    ("source_wrap", 0xFFFD, 0x80, b"A\x00", 0, "file"),
)


def idle_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    index_destination = 0x2225
    name_destination = 0x238C
    gate_offset = 0x2A33
    for name, source_offset, asset_id, detail, presentation_state, backend in IDLE_CASES:
        copied, stopping_byte = split_printable(detail)
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        stream = bytes([asset_id]) + detail
        write_wrapped(input_before, source_offset, stream)
        set_word(globals_before, gate_offset, presentation_state)
        set_word(globals_before, 0x0C50, 0 if backend == "path" else 0xFFFF)
        set_word(globals_before, 0x0C4E, 0 if backend == "file" else 0xFFFF)
        struct.pack_into("<HH", globals_before, 0x55F9, 0x0100, BUFFER_SEGMENT)
        globals_before[0x2389:0x238C] = b"hn/"
        signed_id = asset_id if asset_id < 0x80 else asset_id - 0x100
        stored_id = (((signed_id & 0xFFFF) - 1) * 16 + 0x1025) & 0xFFFF
        target_offsets = offsets(index_destination, 2) | offsets(
            name_destination, len(copied) + 1
        )
        calls = []

        def capture(machine: Uc, address: int) -> None:
            calls.append(
                {
                    "backend": "path" if address == IDLE_PATH_LOADER else "file",
                    "address": f"0x{address:05x}",
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                }
            )

        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x872C,
            0x8796,
            globals_before,
            input_before,
            source_offset,
            target_offsets,
            target_offsets,
            callback_addresses=(IDLE_PATH_LOADER, IDLE_FILE_LOADER),
            callback=capture,
        )
        should_call = presentation_state & 1 == 0
        assert len(calls) == int(should_call), (name, calls)
        if calls:
            assert calls[0]["backend"] == backend
            assert calls[0]["ds"] == GLOBALS_SEGMENT
            assert calls[0]["si"] == 0x2389
        assert word(globals_after, index_destination) == stored_id
        assert read_wrapped(globals_after, name_destination, len(copied) + 1) == copied + b"\0"
        final_source = (source_offset + 1 + len(copied)) & 0xFFFF
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(INITIAL_GENERAL["esi"], final_source)
        for register in ("ebx", "ecx", "edx", "edi", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "idle_clip",
                "name": name,
                "source_offset": source_offset,
                "input_hex": stream.hex(),
                "asset_id": asset_id,
                "copied_hex": copied.hex(),
                "stopping_byte": stopping_byte,
                "presentation_state": presentation_state,
                "loader_called": should_call,
                "backend": backend,
                "calls": calls,
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


SEQUENCE_VIDEO_CASES = (
    ("immediate_nul", 0x6000, b"\x00", 0x3000, 0),
    ("immediate_high", 0x6020, b"\xff", 0x3020, 1),
    ("lower_printable", 0x6040, b"\x20\x1f", 0x3040, 2),
    ("upper_printable", 0x6060, b"\x7f\x80", 0x3060, 0x7E),
    ("ordinary_text", 0x6080, b"ENTRY\x00", 0x3080, 0xA5),
    ("source_wrap", 0xFFFF, b"A\x00", 0x30A, 0xFE),
    ("destination_wrap", 0x60C0, b"AB\x00", 0xFFFF, 0x10),
    ("cursor_and_count_wrap", 0x60E0, b"\x00", 0xFFF0, 0xFF),
)


def sequence_video_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for name, source_offset, payload, cursor, count in SEQUENCE_VIDEO_CASES:
        copied, stopping_byte = split_printable(payload)
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        write_wrapped(input_before, source_offset, payload)
        set_word(globals_before, 0x1568, cursor)
        globals_before[0x156C] = count
        destination_offsets = offsets(cursor, len(copied) + 1)
        state_offsets = offsets(0x1568, 2) | {0x156C}
        allowed = destination_offsets | state_offsets
        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x8796,
            0x87B8,
            globals_before,
            input_before,
            source_offset,
            allowed,
            allowed,
        )
        assert read_wrapped(globals_after, cursor, len(copied) + 1) == copied + b"\0"
        assert word(globals_after, 0x1568) == (cursor + 0x10) & 0xFFFF
        assert globals_after[0x156C] == (count + 1) & 0xFF
        final_source = (source_offset + len(copied)) & 0xFFFF
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(INITIAL_GENERAL["esi"], final_source)
        for register in ("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "sequence_video",
                "name": name,
                "source_offset": source_offset,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stopping_byte,
                "destination_cursor_before": cursor,
                "destination_cursor_after": word(globals_after, 0x1568),
                "entry_count_before": count,
                "entry_count_after": globals_after[0x156C],
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


SEQUENCE_SUBTITLE_CASES = (
    ("empty", 0x6200, 0x0000, b"\x00", 0x3200),
    ("ordinary", 0x6220, 0x1234, b"ABC\x00", 0x3220),
    ("high_bytes", 0x6240, 0xA55A, b"\x80\xff\x00", 0x3240),
    ("embedded_nul", 0x6260, 0xBEEF, b"A\x00Z", 0x3260),
    ("unaligned", 0x6281, 0x0102, b"XY\x00", 0x3281),
    ("source_wrap", 0xFFFE, 0xCAFE, b"Q\x00", 0x32A0),
    ("destination_wrap", 0x62C0, 0xBEEF, b"A\x00", 0xFFFE),
)


def sequence_subtitle_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for name, source_offset, leading_word, suffix, cursor in SEQUENCE_SUBTITLE_CASES:
        copied_suffix = suffix[: suffix.index(0) + 1]
        stream = struct.pack("<H", leading_word) + suffix
        copied = struct.pack("<H", leading_word) + copied_suffix
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        write_wrapped(input_before, source_offset, stream)
        set_word(globals_before, 0x1166, cursor)
        destination_offsets = offsets(cursor, len(copied))
        state_offsets = offsets(0x1166, 2)
        allowed = destination_offsets | state_offsets
        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x87B8,
            0x87CA,
            globals_before,
            input_before,
            source_offset,
            allowed,
            allowed,
        )
        assert read_wrapped(globals_after, cursor, len(copied)) == copied
        final_cursor = (cursor + len(copied)) & 0xFFFF
        assert word(globals_after, 0x1166) == final_cursor
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(
            INITIAL_GENERAL["esi"], (source_offset + len(copied)) & 0xFFFF
        )
        for register in ("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "sequence_subtitle",
                "name": name,
                "source_offset": source_offset,
                "leading_word": leading_word,
                "suffix_hex": suffix.hex(),
                "copied_hex": copied.hex(),
                "destination_cursor_before": cursor,
                "destination_cursor_after": final_cursor,
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


MUSIC_CASES = (
    ("immediate_space", 0x6600, b"\x20", b"OLD", 0x00),
    ("immediate_nul", 0x6620, b"\x00", b"OLD", 0x80),
    ("immediate_high", 0x6640, b"\xff", b"OLD", 0x40),
    ("equal_upper", 0x6660, b"ABC\x20", b"ABCZ", 0x20),
    ("lowercase_matches", 0x6680, b"abc\x00", b"ABCZ", 0x10),
    ("mismatch_sets_changed", 0x66A0, b"ABC\x00", b"AXCZ", 0x44),
    ("backtick_not_masked", 0x66C0, b"`\x00", b"`Z", 0x08),
    ("brace_masks_to_bracket", 0x66E0, b"{\x00", b"[Z", 0x04),
    ("prior_changed_is_reset", 0x6700, b"A\x00", b"AZ", 0x22),
    ("prefix_reuse", 0x6720, b"A\x00", b"AZ", 0x40),
    ("source_wrap", 0xFFFF, b"a\x00", b"AZ", 0x00),
)


def music_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    destination = 0x0F7E
    changed_offset = 0x0DAB
    unchanged_offset = 0x0DAA
    for name, source_offset, payload, prior, unchanged_before in MUSIC_CASES:
        stop_index = next(
            index for index, value in enumerate(payload) if value <= 0x20 or value >= 0x80
        )
        accepted = payload[:stop_index]
        stopping_byte = payload[stop_index]
        transformed = bytes(value & 0xDF if value >= 0x61 else value for value in accepted)
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        write_wrapped(input_before, source_offset, payload)
        globals_before[changed_offset] = 0xA5
        globals_before[unchanged_offset] = unchanged_before
        globals_before[destination : destination + len(prior)] = prior
        changed_after = int(
            any(
                value != globals_before[destination + index]
                for index, value in enumerate(transformed)
            )
        )
        unchanged_after = unchanged_before if changed_after else unchanged_before | 1
        destination_offsets = offsets(destination, len(transformed) + 1)
        state_offsets = {changed_offset}
        if not changed_after:
            state_offsets.add(unchanged_offset)
        allowed = destination_offsets | state_offsets
        machine, globals_after, before_regs, after_regs, _writes = execute(
            executable,
            0x87EB,
            0x8825,
            globals_before,
            input_before,
            source_offset,
            allowed,
            destination_offsets | {changed_offset},
        )
        assert read_wrapped(globals_after, destination, len(transformed) + 1) == transformed + b"\0"
        assert globals_after[changed_offset] == changed_after
        assert globals_after[unchanged_offset] == unchanged_after
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(
            INITIAL_GENERAL["esi"], (source_offset + len(transformed)) & 0xFFFF
        )
        for register in ("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_regs[register] == before_regs[register], (name, register)
        rows.append(
            {
                "type": "music",
                "name": name,
                "source_offset": source_offset,
                "input_hex": payload.hex(),
                "prior_hex": prior.hex(),
                "transformed_hex": transformed.hex(),
                "stopping_byte": stopping_byte,
                "changed_after": changed_after,
                "unchanged_before": unchanged_before,
                "unchanged_after": unchanged_after,
                "registers_before": before_regs,
                "registers_after": after_regs,
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    return {
        "format": "big_bug_bang_complex_parser_handlers_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routine_rows(executable),
        "cases": [
            *sound_cases(executable),
            *talk_cases(executable),
            *idle_cases(executable),
            *sequence_video_cases(executable),
            *sequence_subtitle_cases(executable),
            *music_cases(executable),
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    fixture = build_fixture(executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixture, indent=2) + "\n")
    print(
        f"verified {len(fixture['cases'])} cases across "
        f"{len(fixture['routines'])} BBB complex parser handlers"
    )


if __name__ == "__main__":
    main()
