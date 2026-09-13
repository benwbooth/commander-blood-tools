#!/usr/bin/env python3
"""Verify BBB's unchanged command-tail and startup-option handlers."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_startup_command.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
DATA_IMAGE_OFFSET = 0xF7F0

COMMAND_ENTRY = 0x08EF
COMMAND_END = 0x0924
OPTION_ENTRY = 0x0924
OPTION_END = 0x09A2
NUMBER_ENTRY = 0x2992
NUMBER_END = 0x29E5
ROUTINES = (
    (
        COMMAND_ENTRY,
        COMMAND_END,
        "startup_command_line_parse",
        "b4aa1c36b44947a3c1b0e67f4b360cb46e541cb55c3afd63a4bd077df357fc02",
    ),
    (
        OPTION_ENTRY,
        OPTION_END,
        "startup_option_apply",
        "520aea88629b99c90e10a998f5ac93fa1b16475397e80a3bd1758b7aef90b17b",
    ),
    (
        NUMBER_ENTRY,
        NUMBER_END,
        "startup_audio_number_parse",
        "79aec148e4473edca687e714d782a1a810be0ea7a2b45d096a29b59bb4277366",
    ),
)

GLOBALS_SEGMENT = 0x3000
TOKEN_SEGMENT = 0x4000
PSP_SEGMENT = 0x5000
COMMAND_SEGMENT = PSP_SEGMENT + 8
DECOY_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x6000
TOKEN_OFFSET = 0x6400
TOKEN_BUFFER = 0x0CFC
WRITE_DIRECTORY = 0x0206
OPTION_TABLE = 0x0286
AUDIO_DRIVER = 0x0E45
AUDIO_CONFIGURATION = 0x0E4F
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

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

COMMAND_CASES = (
    ("empty", b"", []),
    ("single", b"AMR", [(0, "AMR")]),
    (
        "shipped_arguments",
        b" AMR S162227 EMS WRIC:\\cblood\\",
        [(30, ""), (26, "AMR"), (18, "S162227"), (14, "EMS"), (0, "WRIC:\\cblood\\")],
    ),
    ("repeated_spaces", b"A  B", [(3, "A"), (2, ""), (0, "B")]),
    ("trailing_space", b"A ", [(1, "A")]),
)

OPTION_CASES: tuple[dict[str, Any], ...] = (
    {"name": "no_match", "token": b"ABC\0"},
    {"name": "matched_no_action", "token": b"MID1234\0", "match": "MID"},
    {
        "name": "write_directory",
        "token": b"WRIC:\\bbb\\\0",
        "match": "WRI",
        "directory": b"C:\\bbb\0",
    },
    {
        "name": "empty_write_directory",
        "token": b"WRI\0",
        "match": "WRI",
        "underflow_write": True,
    },
    {
        "name": "sb16_audio",
        "token": b"S162227\0",
        "match": "S16",
        "parse": "222",
        "driver_id": 42,
        "configuration": 0x0DE7,
    },
    {
        "name": "sound_driver_b_audio",
        "token": b"SDB1234\0",
        "match": "SDB",
        "parse": "123",
        "driver_id": 42,
        "configuration": 0x07B4,
    },
    {
        "name": "sound_blaster_pro_audio",
        "token": b"SBP0015\0",
        "match": "SBP",
        "parse": "001",
        "driver_id": 42,
        "configuration": 0x0015,
    },
    {"name": "removed_gravis_option", "token": b"GRV1234\0"},
    {
        "name": "sequel_hex_trailing_character",
        "token": b"S16-12x\0",
        "match": "S16",
        "parse": "-12",
        "driver_id": 42,
        "configuration": 0xFF41,
    },
)


def loaded(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def low_word(original: int, value: int) -> int:
    return (original & 0xFFFF0000) | (value & 0xFFFF)


def initial_registers(case_index: int) -> dict[str, int]:
    return {
        "eax": 0xA5A51234 + case_index,
        "ebx": 0xB6B62468 + case_index,
        "ecx": 0xC7C7369C + case_index,
        "edx": 0xD8D855AA + case_index,
        "esi": 0xE9E96789 + case_index,
        "edi": 0xFAFA789A + case_index,
        "ebp": 0xABCD1357 + case_index,
        "ds": DECOY_SEGMENT,
        "es": TOKEN_SEGMENT,
        "fs": 0x7800,
        "gs": GLOBALS_SEGMENT,
        "ss": STACK_SEGMENT,
    }


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register)
        for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def subtraction_flags(left: int, right: int, bits: int) -> dict[str, bool]:
    mask = (1 << bits) - 1
    sign = 1 << (bits - 1)
    left &= mask
    right &= mask
    result = (left - right) & mask
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & sign),
        "of": bool(((left ^ right) & (left ^ result)) & sign),
    }


def logic_flags(result: int, bits: int) -> dict[str, bool]:
    mask = (1 << bits) - 1
    sign = 1 << (bits - 1)
    result &= mask
    return {
        "cf": False,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "zf": result == 0,
        "sf": bool(result & sign),
        "of": False,
    }


def observed_flags(machine: Uc, names: set[str]) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 0x001, "pf": 0x004, "af": 0x010, "zf": 0x040, "sf": 0x080, "of": 0x800}
    return {name: bool(value & masks[name]) for name in masks if name in names}


def read_c_string(machine: Uc, segment: int, offset: int, limit: int = 128) -> bytes:
    result = bytearray()
    for index in range(limit):
        value = machine.mem_read(segment * 16 + ((offset + index) & 0xFFFF), 1)[0]
        if value == 0:
            return bytes(result)
        result.append(value)
    raise AssertionError("unterminated synthetic string")


def new_machine(executable: bytes, module_patch: tuple[int, bytes] | None = None) -> tuple[Uc, bytes]:
    module = executable[HEADER_SIZE:]
    expected_module = bytearray(module)
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, module)
    if module_patch is not None:
        address, content = module_patch
        machine.mem_write(address, content)
        expected_module[address : address + len(content)] = content
    return machine, bytes(expected_module)


def command_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, (name, command, expected_calls) in enumerate(COMMAND_CASES):
        initial = initial_registers(case_index)
        initial["es"] = PSP_SEGMENT
        command_before = bytes((len(command),)) + command + bytes([0xA5]) * 8
        globals_before = bytearray([0x69]) * SEGMENT_SIZE
        stack_before = bytearray([0x3C]) * SEGMENT_SIZE
        decoy_before = bytes([0x96]) * SEGMENT_SIZE
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = STACK_SENTINEL

        machine, expected_module = new_machine(executable, (loaded(OPTION_ENTRY), b"\xC3"))
        machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
        machine.mem_write(COMMAND_SEGMENT * 16, command_before)
        machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register, value in initial.items():
            target = GENERAL_REGISTERS.get(register, SEGMENT_REGISTERS.get(register))
            assert target is not None
            machine.reg_write(target, value)
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        calls: list[dict[str, int | str]] = []
        writes: list[tuple[int, int]] = []
        global_limit = TOKEN_BUFFER + max((len(token) for _, token in expected_calls), default=-1) + 1
        allowed_globals = set(range(TOKEN_BUFFER, global_limit))
        allowed_stack = set(range(CALLER_SP - 8, CALLER_SP))

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == loaded(OPTION_ENTRY):
                calls.append(
                    {
                        "remaining": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                        "token": read_c_string(
                            cpu, cpu.reg_read(UC_X86_REG_ES), cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
                        ).decode("ascii"),
                    }
                )
                return
            assert loaded(COMMAND_ENTRY) <= address and address + size <= loaded(COMMAND_END), hex(
                address + HEADER_SIZE
            )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            global_offsets = {item - GLOBALS_SEGMENT * 16 for item in touched}
            stack_offsets = {item - STACK_SEGMENT * 16 for item in touched}
            assert global_offsets <= allowed_globals or stack_offsets <= allowed_stack, hex(address)
            writes.append((address, size))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(loaded(COMMAND_ENTRY), RETURN_OFFSET, count=2_000)

        expected_call_rows = [
            {"remaining": remaining, "token": token} for remaining, token in expected_calls
        ]
        assert calls == expected_call_rows, (name, calls)
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert bytes(machine.mem_read(0, len(expected_module))) == expected_module
        assert bytes(machine.mem_read(COMMAND_SEGMENT * 16, len(command_before))) == command_before
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before

        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        assert_unchanged_outside(globals_before, globals_after, allowed_globals)
        final_token = expected_calls[-1][1] if expected_calls else None
        if final_token is not None:
            assert globals_after[TOKEN_BUFFER : TOKEN_BUFFER + len(final_token) + 1] == (
                final_token.encode("ascii") + b"\0"
            )
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_unchanged_outside(stack_before, stack_after, allowed_stack)
        assert stack_after[CALLER_SP : CALLER_SP + 2 + len(STACK_SENTINEL)] == stack_before[
            CALLER_SP : CALLER_SP + 2 + len(STACK_SENTINEL)
        ]

        expected_registers = dict(initial)
        expected_registers["eax"] = low_word(initial["eax"], command[-1] if command else 0)
        expected_registers["esi"] = low_word(initial["esi"], len(command) + 1)
        if final_token is not None:
            expected_registers["edi"] = low_word(initial["edi"], TOKEN_BUFFER)
        actual_registers = snapshot_registers(machine)
        assert actual_registers == expected_registers, (
            name,
            actual_registers,
            expected_registers,
        )

        if not command:
            expected_flags = logic_flags(0, 16)
        elif command.endswith(b" "):
            expected_flags = subtraction_flags(1, 1, 16)
        else:
            expected_flags = subtraction_flags(command[-1], 0x20, 8)
        actual_flags = observed_flags(machine, set(expected_flags))
        assert actual_flags == expected_flags, (name, actual_flags, expected_flags)
        rows.append(
            {
                "name": name,
                "command": command.decode("ascii"),
                "calls": calls,
                "final_token": final_token,
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def option_table(executable: bytes) -> tuple[bytes, list[dict[str, int | str]]]:
    native_data = executable[DATA_IMAGE_OFFSET:]
    table = bytearray()
    rows = []
    offset = OPTION_TABLE
    while native_data[offset] != 0:
        record = native_data[offset : offset + 5]
        assert len(record) == 5
        table.extend(record)
        rows.append(
            {
                "prefix": record[:3].decode("ascii"),
                "flags": record[3],
                "driver_id": record[4],
            }
        )
        offset += 5
    table.append(0)
    expected = [
        {"prefix": "S16", "flags": 2, "driver_id": 42},
        {"prefix": "MID", "flags": 0, "driver_id": 1},
        {"prefix": "SDB", "flags": 2, "driver_id": 42},
        {"prefix": "SBP", "flags": 2, "driver_id": 42},
        {"prefix": "WRI", "flags": 1, "driver_id": 0},
    ]
    assert rows == expected
    return bytes(table), rows


def option_cases(executable: bytes, table: bytes) -> list[dict[str, Any]]:
    rows = []
    native_data = executable[DATA_IMAGE_OFFSET:]
    assert native_data[OPTION_TABLE : OPTION_TABLE + len(table)] == table
    for case_index, case in enumerate(OPTION_CASES):
        name = str(case["name"])
        token = bytes(case["token"])
        initial = initial_registers(case_index)
        initial["es"] = TOKEN_SEGMENT
        initial["edi"] = low_word(initial["edi"], TOKEN_OFFSET)

        globals_before = bytearray([0x69]) * SEGMENT_SIZE
        globals_before[AUDIO_DRIVER] = 0x69
        struct.pack_into("<H", globals_before, AUDIO_CONFIGURATION, 0x1234)
        token_before = bytearray([0xDD]) * SEGMENT_SIZE
        token_before[TOKEN_OFFSET : TOKEN_OFFSET + len(token)] = token
        stack_before = bytearray([0x3C]) * SEGMENT_SIZE
        stack_before[: len(native_data)] = native_data
        path_before = b"\xA5\x5A" + b"c" * 32
        stack_before[WRITE_DIRECTORY - 2 : WRITE_DIRECTORY - 2 + len(path_before)] = path_before
        stack_before[CALLER_SP : CALLER_SP + 2] = struct.pack("<H", RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = STACK_SENTINEL
        decoy_before = bytes([0x96]) * SEGMENT_SIZE

        machine, expected_module = new_machine(executable)
        machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
        machine.mem_write(TOKEN_SEGMENT * 16, bytes(token_before))
        machine.mem_write(DECOY_SEGMENT * 16, decoy_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register, value in initial.items():
            target = GENERAL_REGISTERS.get(register, SEGMENT_REGISTERS.get(register))
            assert target is not None
            machine.reg_write(target, value)
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        audio = "configuration" in case
        directory = bytes(case["directory"]) if "directory" in case else None
        underflow = bool(case.get("underflow_write", False))
        stack_frame_offsets = set(range(CALLER_SP - 26, CALLER_SP))
        path_offsets: set[int] = set()
        if directory is not None:
            path_offsets = set(range(WRITE_DIRECTORY, WRITE_DIRECTORY + len(directory)))
        elif underflow:
            path_offsets = {WRITE_DIRECTORY - 1}
        token_offsets = {TOKEN_OFFSET + 6} if audio else set()
        global_offsets = (
            {AUDIO_DRIVER, AUDIO_CONFIGURATION, AUDIO_CONFIGURATION + 1} if audio else set()
        )
        helper_calls: list[dict[str, int | str]] = []
        writes: list[tuple[int, int]] = []

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == loaded(NUMBER_ENTRY):
                helper_calls.append(
                    {
                        "segment": cpu.reg_read(UC_X86_REG_DS),
                        "offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "text": read_c_string(
                            cpu, cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF
                        ).decode("ascii"),
                    }
                )
            spans = (
                (loaded(OPTION_ENTRY), loaded(OPTION_END)),
                (loaded(NUMBER_ENTRY), loaded(NUMBER_END)),
            )
            assert any(start <= address and address + size <= end for start, end in spans), hex(
                address + HEADER_SIZE
            )

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _context: Any,
        ) -> None:
            touched = set(range(address, address + size))
            stack_offsets = {item - STACK_SEGMENT * 16 for item in touched}
            token_write_offsets = {item - TOKEN_SEGMENT * 16 for item in touched}
            global_write_offsets = {item - GLOBALS_SEGMENT * 16 for item in touched}
            assert (
                stack_offsets <= stack_frame_offsets | path_offsets
                or token_write_offsets <= token_offsets
                or global_write_offsets <= global_offsets
            ), hex(address)
            writes.append((address, size))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(loaded(OPTION_ENTRY), RETURN_OFFSET, count=4_000)

        expected_helpers = []
        if audio:
            expected_helpers = [
                {"segment": TOKEN_SEGMENT, "offset": TOKEN_OFFSET + 3, "text": str(case["parse"])}
            ]
        assert helper_calls == expected_helpers, (name, helper_calls)
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert bytes(machine.mem_read(0, len(expected_module))) == expected_module
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before

        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        expected_driver = int(case.get("driver_id", 0x69))
        expected_configuration = int(case.get("configuration", 0x1234))
        assert globals_after[AUDIO_DRIVER] == expected_driver
        assert struct.unpack_from("<H", globals_after, AUDIO_CONFIGURATION)[0] == expected_configuration
        assert_unchanged_outside(globals_before, globals_after, global_offsets)

        token_after = bytes(machine.mem_read(TOKEN_SEGMENT * 16, SEGMENT_SIZE))
        expected_token = bytearray(token_before)
        if audio:
            expected_token[TOKEN_OFFSET + 6] = 0
        assert token_after == bytes(expected_token)

        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        expected_stack = bytearray(stack_before)
        if directory is not None:
            expected_stack[WRITE_DIRECTORY : WRITE_DIRECTORY + len(directory)] = directory
        elif underflow:
            expected_stack[WRITE_DIRECTORY - 1] = 0
        assert_unchanged_outside(
            expected_stack, stack_after, stack_frame_offsets
        )
        assert stack_after[OPTION_TABLE : OPTION_TABLE + len(table)] == table
        assert stack_after[CALLER_SP : CALLER_SP + 2 + len(STACK_SENTINEL)] == stack_before[
            CALLER_SP : CALLER_SP + 2 + len(STACK_SENTINEL)
        ]

        after_registers = snapshot_registers(machine)
        for register in ("ecx", "edx", "esi", "edi", "ebp", "ds", "es", "fs", "gs", "ss"):
            assert after_registers[register] == initial[register], (name, register)

        if audio:
            expected_flags = logic_flags(expected_configuration & 0xFF, 8)
        else:
            expected_flags = logic_flags(0, 8)
        actual_flags = observed_flags(machine, set(expected_flags))
        assert actual_flags == expected_flags, (name, actual_flags, expected_flags)

        owned_directory = "c" * 32
        if directory is not None:
            owned_directory = directory.rstrip(b"\0").decode("ascii")
        rows.append(
            {
                "name": name,
                "token_before": token.rstrip(b"\0").decode("ascii"),
                "token_after": read_c_string(machine, TOKEN_SEGMENT, TOKEN_OFFSET).decode("ascii"),
                "matched_prefix": case.get("match"),
                "helper_calls": helper_calls,
                "owned_write_directory": owned_directory,
                "raw_directory_underflow_write": underflow,
                "driver_id": expected_driver,
                "configuration": expected_configuration,
                "defined_flags": expected_flags,
                "write_count": len(writes),
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    routines = []
    for entry, end, operation, expected_hash in ROUTINES:
        digest = hashlib.sha256(executable[entry:end]).hexdigest()
        assert digest == expected_hash, hex(entry)
        routines.append(
            {
                "operation": operation,
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "body_sha256": digest,
            }
        )
    table, options = option_table(executable)
    return {
        "format": "big_bug_bang_startup_command_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routines,
        "startup_options": options,
        "command_cases": command_cases(executable),
        "option_cases": option_cases(executable, table),
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
        f"verified {len(fixture['command_cases'])} BBB command-tail and "
        f"{len(fixture['option_cases'])} startup-option cases"
    )


if __name__ == "__main__":
    main()
