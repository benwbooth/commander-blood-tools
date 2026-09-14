#!/usr/bin/env python3
"""Verify BBB's two relocated PBM back-buffer presentation wrappers."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

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
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_back_buffer_wrappers.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
DATA_FILE_OFFSET = 0xF7F0

PBM_SEGMENT = 0x01E6
PBM_OFFSET = 0x0923
PBM_ADDRESS = PBM_SEGMENT * 16 + PBM_OFFSET
CHUNKY_SEGMENT = 0x02B1
CHUNKY_OFFSET = 0x103B
CHUNKY_ADDRESS = CHUNKY_SEGMENT * 16 + CHUNKY_OFFSET

BACK_BUFFER_POINTER_OFFSET = 0x55F9
DISPLAY_POINTER_OFFSET = 0x55E9
PALETTE_FLAG_OFFSET = 0x5F23
TRANSPARENCY_FLAG_OFFSET = 0x5F27
COMMANDER_BACK_BUFFER_POINTER_OFFSET = 0x5229
COMMANDER_DISPLAY_POINTER_OFFSET = 0x5219
COMMANDER_PALETTE_FLAG_OFFSET = 0x5B53
COMMANDER_TRANSPARENCY_FLAG_OFFSET = 0x5B57

DATA_SEGMENT = 0x3000
BACK_SEGMENT_MIN = 0x5000
GAME_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
FS_SEGMENT = 0xA000
EXTRA_SEGMENT = 0xB000
RETURN_SEGMENT = 0xE000
RETURN_OFFSET = 0x0100
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
FLAG_MASK = 0x0CD7
FULL_FLAG_MASK = FLAG_MASK | 0x0200

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


@dataclass(frozen=True)
class Routine:
    name: str
    resource_path: str
    resource_selector: int
    commander_entry: int
    commander_selector: int
    commander_fixture: Path
    commander_fixture_sha256: str
    entry: int
    end: int
    body_sha256: str

    @property
    def pbm_return(self) -> int:
        return self.entry + 26

    @property
    def chunky_return(self) -> int:
        return self.entry + 51


ROUTINES = (
    Routine(
        "chart_back_buffer",
        "chart.fd",
        0x00E9,
        0x17D9,
        0x00EA,
        ROOT / "re/tools/oracle_vectors/func_17d9_natural.json",
        "f2c886fc513be4277bca9964d07123010a16358e5ae8d230fe4cb7dca806b2af",
        0x199B,
        0x19D9,
        "8e90ef59126c70eaed598a33a2ee744a8af643ad66d7eb71d7683236562d4550",
    ),
    Routine(
        "orx_back_buffer",
        "orx.fd",
        0x00E2,
        0x1817,
        0x00E3,
        ROOT / "re/tools/oracle_vectors/func_1817_natural.json",
        "2505c2373b90995e7fd8f4d8d2106b936a0099dd1cf5cb53f683945326eda7d1",
        0x19D9,
        0x1A17,
        "bb831a3c513b0efbc6dcd8de5e94fba7a4f58865a05529f3757f55584bcc27d3",
    ),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def c_string_at(data: bytes, start: int) -> str:
    end = data.index(0, start)
    return data[start:end].decode("ascii")


def commander_vectors(routine: Routine) -> list[dict[str, Any]]:
    encoded = routine.commander_fixture.read_bytes()
    assert sha256(encoded) == routine.commander_fixture_sha256
    rows = json.loads(encoded)
    assert len(rows) == 6
    return rows


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register) for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def snapshot_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(flags & 0x0001),
        "pf": bool(flags & 0x0004),
        "af": bool(flags & 0x0010),
        "zf": bool(flags & 0x0040),
        "sf": bool(flags & 0x0080),
        "if": bool(flags & 0x0200),
        "df": bool(flags & 0x0400),
        "of": bool(flags & 0x0800),
    }


def write_event(
    target: bytearray,
    segment: int,
    offset: int,
    size: int,
    value: int,
    events: list[tuple[int, int, int]],
) -> None:
    struct.pack_into({1: "<B", 2: "<H", 4: "<I"}[size], target, offset, value)
    events.append((physical_address(segment, offset), size, value))


def normalized_row(row: dict[str, Any], expected: dict[str, Any]) -> dict[str, Any]:
    normalized = deepcopy(row)
    normalized["image_path_offset"] = expected["image_path_offset"]
    normalized["path_prefix"] = expected["path_prefix"]
    normalized["calls"][0]["si"] = expected["calls"][0]["si"]
    normalized["calls"][0]["cs"] = expected["calls"][0]["cs"]
    normalized["calls"][0]["ip"] = expected["calls"][0]["ip"]
    normalized["calls"][0]["path_prefix"] = expected["calls"][0]["path_prefix"]
    normalized["calls"][1]["cs"] = expected["calls"][1]["cs"]
    normalized["calls"][1]["ip"] = expected["calls"][1]["ip"]
    normalized.pop("final_defined_flags")
    return normalized


def routine_vectors(executable: bytes, routine: Routine) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    expected_rows = commander_vectors(routine)
    for case_index, expected in enumerate(expected_rows):
        name = str(expected["name"])
        back_segment = int(expected["back_buffer"]["segment"])
        back_offset = int(expected["back_buffer"]["offset"])
        assert back_segment >= BACK_SEGMENT_MIN
        saved_draw_offset = int(expected["saved_draw_framebuffer"]["offset"])
        saved_draw_segment = int(expected["saved_draw_framebuffer"]["segment"])
        saved_draw = saved_draw_offset | saved_draw_segment << 16
        pbm_result = int(expected["pbm_result"])
        entry_flags = int(expected["calls"][0]["flags"]) | 0x0202
        pbm_flags = int(expected["calls"][1]["flags"]) | 0x0202
        chunky_flags = int(expected["final_flags"]) | 0x0202

        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0x0B0B1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 13, 0x11)
        data_expected = bytearray(data_before)
        extra_before = seeded_segment(case_index, 17, 0x23)
        fs_before = seeded_segment(case_index, 19, 0x35)
        game_before = seeded_segment(case_index, 23, 0x47)
        game_expected = bytearray(game_before)
        back_before = seeded_segment(case_index, 29, 0x59)
        stack_before = seeded_segment(case_index, 31, 0x6B)
        stack_expected = bytearray(stack_before)
        return_before = seeded_segment(case_index, 37, 0x7D)

        data_before[0x00E2:0x00F2] = b"orx.fd\0chart.fd\0"
        data_expected[:] = data_before
        struct.pack_into(
            "<HH", data_before, BACK_BUFFER_POINTER_OFFSET, back_offset, back_segment
        )
        struct.pack_into(
            "<HH", data_expected, BACK_BUFFER_POINTER_OFFSET, back_offset, back_segment
        )
        struct.pack_into(
            "<HH",
            data_before,
            COMMANDER_BACK_BUFFER_POINTER_OFFSET,
            back_offset ^ 0xA5A5,
            EXTRA_SEGMENT,
        )
        struct.pack_into(
            "<HH",
            data_expected,
            COMMANDER_BACK_BUFFER_POINTER_OFFSET,
            back_offset ^ 0xA5A5,
            EXTRA_SEGMENT,
        )
        for offset in (PALETTE_FLAG_OFFSET, TRANSPARENCY_FLAG_OFFSET):
            data_before[offset] = 0xA5 ^ case_index
            data_expected[offset] = 0
        for offset in (
            COMMANDER_PALETTE_FLAG_OFFSET,
            COMMANDER_TRANSPARENCY_FLAG_OFFSET,
        ):
            data_before[offset] = 0x5A ^ case_index
            data_expected[offset] = 0x5A ^ case_index

        struct.pack_into("<I", game_before, DISPLAY_POINTER_OFFSET, saved_draw)
        struct.pack_into("<I", game_expected, DISPLAY_POINTER_OFFSET, saved_draw)
        struct.pack_into(
            "<I", game_before, COMMANDER_DISPLAY_POINTER_OFFSET, saved_draw ^ 0xA5A5A5A5
        )
        struct.pack_into(
            "<I",
            game_expected,
            COMMANDER_DISPLAY_POINTER_OFFSET,
            saved_draw ^ 0xA5A5A5A5,
        )
        for decoy in (extra_before, fs_before):
            struct.pack_into(
                "<I", decoy, DISPLAY_POINTER_OFFSET, saved_draw ^ 0x5A5A5A5A
            )
            struct.pack_into(
                "<HH",
                decoy,
                BACK_BUFFER_POINTER_OFFSET,
                back_offset ^ 0x5A5A,
                DATA_SEGMENT,
            )
            decoy[PALETTE_FLAG_OFFSET] = 0xC3
            decoy[TRANSPARENCY_FLAG_OFFSET] = 0x3C

        struct.pack_into("<HH", stack_before, CALLER_SP, RETURN_OFFSET, RETURN_SEGMENT)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected[:] = stack_before
        return_before[RETURN_OFFSET] = 0xCC

        expected_writes: list[tuple[int, int, int]] = []
        for offset, value in (
            (CALLER_SP - 2, DATA_SEGMENT),
            (CALLER_SP - 4, initial["esi"] & 0xFFFF),
            (CALLER_SP - 6, EXTRA_SEGMENT),
            (CALLER_SP - 8, initial["edi"] & 0xFFFF),
        ):
            write_event(
                stack_expected, STACK_SEGMENT, offset, 2, value, expected_writes
            )
        write_event(
            data_expected,
            DATA_SEGMENT,
            PALETTE_FLAG_OFFSET,
            1,
            0,
            expected_writes,
        )
        write_event(
            data_expected,
            DATA_SEGMENT,
            TRANSPARENCY_FLAG_OFFSET,
            1,
            0,
            expected_writes,
        )
        for offset, value in (
            (CALLER_SP - 10, 0),
            (CALLER_SP - 12, routine.pbm_return),
            (CALLER_SP - 14, pbm_flags),
        ):
            write_event(
                stack_expected, STACK_SEGMENT, offset, 2, value, expected_writes
            )
        write_event(
            stack_expected,
            STACK_SEGMENT,
            CALLER_SP - 12,
            4,
            saved_draw,
            expected_writes,
        )
        write_event(
            game_expected,
            GAME_SEGMENT,
            DISPLAY_POINTER_OFFSET,
            4,
            0xA000C000,
            expected_writes,
        )
        for offset, value in (
            (CALLER_SP - 14, 0),
            (CALLER_SP - 16, routine.chunky_return),
            (CALLER_SP - 18, chunky_flags),
        ):
            write_event(
                stack_expected, STACK_SEGMENT, offset, 2, value, expected_writes
            )
        write_event(
            game_expected,
            GAME_SEGMENT,
            DISPLAY_POINTER_OFFSET,
            4,
            saved_draw,
            expected_writes,
        )

        patched = bytearray(executable)
        patched[PBM_ADDRESS : PBM_ADDRESS + 8] = (
            b"\xb8"
            + struct.pack("<H", pbm_result)
            + b"\x68"
            + struct.pack("<H", pbm_flags)
            + b"\x9d\xcb"
        )
        patched[CHUNKY_ADDRESS : CHUNKY_ADDRESS + 8] = (
            b"\xba\xc4\x03\x68" + struct.pack("<H", chunky_flags) + b"\x9d\xcb"
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, bytes(patched))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(back_segment * 16, bytes(back_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_SEGMENT * 16, bytes(return_before))
        module_before = bytes(machine.mem_read(0, len(patched)))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, entry_flags)

        calls: list[dict[str, Any]] = []
        reached_return: list[int] = []
        observed_writes: list[tuple[int, int, int]] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            observed_calls: list[dict[str, Any]] = calls,
            reached: list[int] = reached_return,
            case_initial: dict[str, int] = initial,
            case_name: str = name,
            case_saved_draw: int = saved_draw,
            case_saved_draw_offset: int = saved_draw_offset,
            case_saved_draw_segment: int = saved_draw_segment,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            if address == PBM_ADDRESS:
                path_prefix = bytes(
                    cpu.mem_read(
                        physical_address(DATA_SEGMENT, routine.resource_selector), 8
                    )
                ).hex()
                observed_calls.append(
                    {
                        "callee": "pbm_image_load_and_decode",
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "si": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "es": cpu.reg_read(UC_X86_REG_ES),
                        "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "gs": cpu.reg_read(UC_X86_REG_GS),
                        "ss": cpu.reg_read(UC_X86_REG_SS),
                        "sp": cpu.reg_read(UC_X86_REG_SP),
                        "cs": cpu.reg_read(UC_X86_REG_CS),
                        "ip": cpu.reg_read(UC_X86_REG_IP),
                        "flags": cpu.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK,
                        "path_prefix": path_prefix,
                    }
                )
                stack_words = struct.unpack(
                    "<8H",
                    cpu.mem_read(physical_address(STACK_SEGMENT, CALLER_SP - 12), 16),
                )
                assert stack_words == (
                    routine.pbm_return,
                    0,
                    case_initial["edi"] & 0xFFFF,
                    EXTRA_SEGMENT,
                    case_initial["esi"] & 0xFFFF,
                    DATA_SEGMENT,
                    RETURN_OFFSET,
                    RETURN_SEGMENT,
                ), case_name
                flags = bytes(
                    cpu.mem_read(physical_address(DATA_SEGMENT, PALETTE_FLAG_OFFSET), 5)
                )
                assert flags[0] == 0 and flags[4] == 0, case_name
                draw_pointer = struct.unpack(
                    "<I",
                    cpu.mem_read(
                        physical_address(GAME_SEGMENT, DISPLAY_POINTER_OFFSET), 4
                    ),
                )[0]
                assert draw_pointer == case_saved_draw, case_name
                return
            if PBM_ADDRESS < address < PBM_ADDRESS + 8:
                return
            if address == CHUNKY_ADDRESS:
                observed_calls.append(
                    {
                        "callee": "chunky_to_planar_framebuffer",
                        "ax": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                        "dx": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "si": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "es": cpu.reg_read(UC_X86_REG_ES),
                        "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "gs": cpu.reg_read(UC_X86_REG_GS),
                        "ss": cpu.reg_read(UC_X86_REG_SS),
                        "sp": cpu.reg_read(UC_X86_REG_SP),
                        "cs": cpu.reg_read(UC_X86_REG_CS),
                        "ip": cpu.reg_read(UC_X86_REG_IP),
                        "flags": cpu.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK,
                        "draw_offset": struct.unpack(
                            "<H",
                            cpu.mem_read(
                                physical_address(GAME_SEGMENT, DISPLAY_POINTER_OFFSET),
                                2,
                            ),
                        )[0],
                        "draw_segment": struct.unpack(
                            "<H",
                            cpu.mem_read(
                                physical_address(
                                    GAME_SEGMENT, DISPLAY_POINTER_OFFSET + 2
                                ),
                                2,
                            ),
                        )[0],
                    }
                )
                stack_words = struct.unpack(
                    "<10H",
                    cpu.mem_read(physical_address(STACK_SEGMENT, CALLER_SP - 16), 20),
                )
                assert stack_words == (
                    routine.chunky_return,
                    0,
                    case_saved_draw_offset,
                    case_saved_draw_segment,
                    case_initial["edi"] & 0xFFFF,
                    EXTRA_SEGMENT,
                    case_initial["esi"] & 0xFFFF,
                    DATA_SEGMENT,
                    RETURN_OFFSET,
                    RETURN_SEGMENT,
                ), case_name
                return
            if CHUNKY_ADDRESS < address < CHUNKY_ADDRESS + 8:
                return
            assert routine.entry <= address < routine.end, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            writes: list[tuple[int, int, int]] = observed_writes,
        ) -> None:
            writes.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(routine.entry, 0, count=100)

        assert reached_return == [RETURN_ADDRESS], name
        assert observed_writes == expected_writes, (
            name,
            observed_writes,
            expected_writes,
        )
        assert len(calls) == 2, name
        assert [call["callee"] for call in calls] == [
            "pbm_image_load_and_decode",
            "chunky_to_planar_framebuffer",
        ], name
        assert (
            calls[0]["ds"] == DATA_SEGMENT
            and calls[0]["si"] == routine.resource_selector
        )
        assert calls[0]["es"] == back_segment and calls[0]["di"] == back_offset
        assert (
            calls[1]["ax"] == pbm_result and calls[1]["dx"] == initial["edx"] & 0xFFFF
        )
        assert calls[1]["ds"] == back_segment and calls[1]["si"] == back_offset
        assert calls[1]["es"] == back_segment and calls[1]["di"] == back_offset
        assert calls[1]["draw_offset"] == 0xC000
        assert calls[1]["draw_segment"] == 0xA000

        expected_registers = dict(initial)
        expected_registers["eax"] = initial["eax"] & 0xFFFF0000 | pbm_result
        expected_registers["edx"] = initial["edx"] & 0xFFFF0000 | 0x03C4
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_EFLAGS) & FULL_FLAG_MASK == (
            chunky_flags & FULL_FLAG_MASK
        ), name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT, name
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, name
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4, name
        assert bytes(machine.mem_read(0, len(patched))) == module_before, name
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        ), name
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        ), name
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        ), name
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        ), name
        assert bytes(machine.mem_read(back_segment * 16, SEGMENT_SIZE)) == bytes(
            back_before
        ), name
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        ), name
        assert bytes(machine.mem_read(RETURN_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            return_before
        ), name
        assert (
            bytes(
                machine.mem_read(
                    physical_address(STACK_SEGMENT, CALLER_SP + 4),
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        ), name

        row = deepcopy(expected)
        row["image_path_offset"] = routine.resource_selector
        row["path_prefix"] = calls[0]["path_prefix"]
        row["calls"] = calls
        row["final_eax"] = machine.reg_read(UC_X86_REG_EAX)
        row["final_edx"] = machine.reg_read(UC_X86_REG_EDX)
        row["final_flags"] = machine.reg_read(UC_X86_REG_EFLAGS) & FLAG_MASK
        row["final_defined_flags"] = snapshot_flags(machine)
        assert normalized_row(row, expected) == expected, name
        rows.append(row)
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = sha256(executable)
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    for routine in ROUTINES:
        body_digest = sha256(executable[routine.entry : routine.end])
        if body_digest != routine.body_sha256:
            raise SystemExit(f"BBB {routine.name} body changed: {body_digest}")
        resource = c_string_at(executable, DATA_FILE_OFFSET + routine.resource_selector)
        if resource != routine.resource_path:
            raise SystemExit(
                f"BBB {routine.name} selector resolves to {resource!r}, "
                f"expected {routine.resource_path!r}"
            )
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    executable = verify_executable(args.executable)
    result = {
        "format": "big_bug_bang_back_buffer_wrappers_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "data_file_offset": DATA_FILE_OFFSET,
        "routines": [
            {
                "name": routine.name,
                "resource_path": routine.resource_path,
                "resource_selector": routine.resource_selector,
                "commander_entry": f"0x{routine.commander_entry:04x}",
                "commander_selector": routine.commander_selector,
                "commander_fixture_sha256": routine.commander_fixture_sha256,
                "entry": f"0x{routine.entry:04x}",
                "end": f"0x{routine.end:04x}",
                "body_sha256": routine.body_sha256,
                "rows": routine_vectors(executable, routine),
            }
            for routine in ROUTINES
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(
        f"verified {sum(len(routine['rows']) for routine in result['routines'])} "
        "BBB back-buffer wrapper cases"
    )


if __name__ == "__main__":
    main()
