#!/usr/bin/env python3
"""Verify BBB's unchanged DESCRIPT background-cache handler."""

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

from unicorn import (  # noqa: E402
    Uc,
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_AH,
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
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_background_cache.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
BODY_SHA256 = "b5ebd2ee6048de3f5497e2ae9e677fc540ab61a9a92aa202637c26e7f066bd6a"

ENTRY = 0x85A0
END = 0x8654
WRITE_DIRECTORY = 0x01E6 * 16 + 0x04E3
SOURCE_SELECT = 0x01E6 * 16 + 0x03B3
RESOURCE_LOOKUP = 0x01E6 * 16 + 0x05EA
CALLBACKS = (WRITE_DIRECTORY, SOURCE_SELECT, RESOURCE_LOOKUP)
GLOBALS_SEGMENT = 0x3000
INPUT_SEGMENT = 0x4000
BUFFER_SEGMENT = 0x5000
DECOY_SEGMENT = 0x6000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
PATH_OFFSET = 0x1015
NAME_OFFSET = 0x1018
TABLE_OFFSET = 0x1025
EMBEDDED_FLAG = 0x0CEB
SOURCE_SIZE = 0x0C8A
BUFFER_POINTER = 0x55F9
BUFFER_OFFSET = 0x5200
SOURCE_HANDLE = 0x2468
EMBEDDED_HANDLE = 0x3579
OUTPUT_HANDLE = 0x468A
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
INITIAL_GENERAL = {
    "eax": 0xA5A51234,
    "ebx": 0xB6B62345,
    "ecx": 0xC7C73456,
    "edx": 0xD8D84567,
    "esi": 0xE9E90000,
    "edi": 0xFAFA4567,
    "ebp": 0xABCD789A,
}

CASES: tuple[dict[str, Any], ...] = (
    {
        "name": "exact_cache_hit",
        "slot": 1,
        "filename": b"same.lbm",
        "stop": 0,
        "target": b"same.lbm\0",
        "matched": True,
    },
    {
        "name": "prefix_cache_hit",
        "slot": 2,
        "filename": b"short",
        "stop": 0,
        "target": b"shorter.lbm\0",
        "matched": True,
    },
    {
        "name": "standalone_full_copy",
        "slot": 3,
        "filename": b"new3.lbm",
        "stop": 0,
        "target": b"old3.lbm\0",
        "source_data": b"ABCDE",
    },
    {
        "name": "embedded_short_read",
        "slot": 4,
        "filename": b"new4.lbm",
        "stop": 0,
        "target": b"old4.lbm\0",
        "source_data": b"EMBEDDED",
        "read_count": 3,
        "embedded": True,
    },
    {
        "name": "ignored_source_open_error",
        "slot": 1,
        "filename": b"fresh.lbm",
        "stop": 0,
        "target": b"stale.lbm\0",
        "source_data": b"ERR",
        "open_error": 2,
    },
    {
        "name": "ignored_destination_create_error",
        "slot": 2,
        "filename": b"write.lbm",
        "stop": 0,
        "target": b"prior.lbm\0",
        "source_data": b"WRITE",
        "create_error": 5,
    },
    {
        "name": "wrapped_source_low_control_stop",
        "slot": 3,
        "filename": b"wrap",
        "stop": 0x1F,
        "target": b"oldwrap.lbm\0",
        "source_data": b"WRAPDATA",
        "start": 0xFFFC,
    },
    {
        "name": "high_stop_decrement_before_sign_extend",
        "slot": 0x80,
        "filename": b"high",
        "stop": 0x80,
        "target": b"oldhigh.lbm\0",
        "source_data": b"HIGH",
    },
)


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


def low_word(value: int, replacement: int) -> int:
    return (value & 0xFFFF0000) | (replacement & 0xFFFF)


def set_carry(machine: Uc, carry: bool) -> None:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    machine.reg_write(UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1)


def run_case(executable: bytes, case_index: int, case: dict[str, Any]) -> dict[str, Any]:
    name = str(case["name"])
    slot = int(case["slot"])
    filename = bytes(case["filename"])
    stop_byte = int(case["stop"])
    target_before = bytes(case["target"])
    matched = bool(case.get("matched", False))
    embedded = bool(case.get("embedded", False))
    source_data = bytes(case.get("source_data", b""))
    source_size = int(case.get("source_size", len(source_data)))
    read_count = int(case.get("read_count", len(source_data)))
    open_error = int(case.get("open_error", 0))
    create_error = int(case.get("create_error", 0))
    start = int(case.get("start", 0x6200 + case_index * 0x40))

    decremented_slot = (slot - 1) & 0xFF
    signed_slot = decremented_slot if decremented_slot < 0x80 else decremented_slot - 0x100
    target_offset = (TABLE_OFFSET + (signed_slot << 4)) & 0xFFFF
    stream = bytes((slot,)) + filename + bytes((stop_byte,))
    stop_offset = (start + 1 + len(filename)) & 0xFFFF
    target_size = max(16, len(target_before) + 4)
    target_storage = target_before.ljust(target_size, b"\xCC")
    source_raw_handle = open_error or SOURCE_HANDLE
    output_raw_handle = create_error or OUTPUT_HANDLE

    globals_before = bytearray([0xA5]) * SEGMENT_SIZE
    input_before = bytearray([0xCC]) * SEGMENT_SIZE
    buffer_before = bytearray([0x69]) * SEGMENT_SIZE
    decoy_before = bytearray([0x96]) * SEGMENT_SIZE
    write_wrapped(input_before, start, stream)
    globals_before[PATH_OFFSET:NAME_OFFSET] = b"bg/"
    globals_before[target_offset : target_offset + target_size] = target_storage
    globals_before[EMBEDDED_FLAG] = 0
    struct.pack_into("<I", globals_before, SOURCE_SIZE, 0x44332211)
    struct.pack_into("<HH", globals_before, BUFFER_POINTER, BUFFER_OFFSET, BUFFER_SEGMENT)

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    expected_image = bytearray(executable)
    for address in CALLBACKS:
        machine.mem_write(address, b"\xcb")
        expected_image[address] = 0xCB
    machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(INPUT_SEGMENT * 16, bytes(input_before))
    machine.mem_write(BUFFER_SEGMENT * 16, bytes(buffer_before))
    machine.mem_write(DECOY_SEGMENT * 16, bytes(decoy_before))
    machine.mem_write(
        STACK_SEGMENT * 16 + CALLER_SP,
        struct.pack("<H", RETURN_OFFSET) + STACK_SENTINEL,
    )

    for register_name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, INITIAL_GENERAL[register_name])
    machine.reg_write(UC_X86_REG_ESI, low_word(INITIAL_GENERAL["esi"], start))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_DS, INPUT_SEGMENT)
    machine.reg_write(UC_X86_REG_ES, GLOBALS_SEGMENT)
    machine.reg_write(UC_X86_REG_FS, 0x5800)
    machine.reg_write(UC_X86_REG_GS, GLOBALS_SEGMENT)
    machine.reg_write(UC_X86_REG_SS, STACK_SEGMENT)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0202)

    helper_calls: list[dict[str, int | str]] = []
    dos_calls: list[dict[str, int | str | bool]] = []
    written_payloads: list[bytes] = []
    allowed_globals = offsets(NAME_OFFSET, len(filename) + 1)
    if not matched:
        allowed_globals.update(offsets(target_offset, len(filename) + 1))
        allowed_globals.add(EMBEDDED_FLAG)
        allowed_globals.update(offsets(SOURCE_SIZE, 4))
    allowed_writes = {GLOBALS_SEGMENT * 16 + offset for offset in allowed_globals}
    allowed_writes.update(
        range(STACK_SEGMENT * 16 + CALLER_SP - 20, STACK_SEGMENT * 16 + CALLER_SP)
    )
    writes: set[int] = set()

    def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
        assert (ENTRY <= address and address + size <= END) or address in CALLBACKS, hex(address)
        if address not in CALLBACKS:
            return
        kind = {
            WRITE_DIRECTORY: "write_directory",
            SOURCE_SELECT: "source_select",
            RESOURCE_LOOKUP: "resource_lookup",
        }[address]
        helper_calls.append(
            {
                "kind": kind,
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "si": cpu.reg_read(UC_X86_REG_SI),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                "dx": cpu.reg_read(UC_X86_REG_DX),
                "bx": cpu.reg_read(UC_X86_REG_BX),
                "sp": cpu.reg_read(UC_X86_REG_SP),
                "cs": cpu.reg_read(UC_X86_REG_CS),
            }
        )
        if address == SOURCE_SELECT:
            cpu.mem_write(
                GLOBALS_SEGMENT * 16 + EMBEDDED_FLAG, bytes((int(embedded),))
            )
            if embedded:
                cpu.mem_write(
                    GLOBALS_SEGMENT * 16 + SOURCE_SIZE,
                    struct.pack("<I", source_size),
                )
                cpu.reg_write(UC_X86_REG_BX, EMBEDDED_HANDLE)
        elif address == RESOURCE_LOOKUP:
            cpu.reg_write(UC_X86_REG_EBP, source_size)

    def write_hook(
        _cpu: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: Any,
    ) -> None:
        written = set(range(address, address + size))
        assert written <= allowed_writes, (hex(address), size)
        writes.update(written)

    def interrupt(cpu: Uc, number: int, _context: Any) -> None:
        assert number == 0x21, hex(number)
        function = cpu.reg_read(UC_X86_REG_AH)
        call: dict[str, int | str | bool] = {
            "function": function,
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "dx": cpu.reg_read(UC_X86_REG_DX),
            "bx": cpu.reg_read(UC_X86_REG_BX),
            "cx": cpu.reg_read(UC_X86_REG_CX),
        }
        if function == 0x41:
            call["kind"] = "delete"
            cpu.reg_write(UC_X86_REG_AX, 2)
            set_carry(cpu, True)
        elif function == 0x3C:
            call["kind"] = "create"
            call["error"] = bool(create_error)
            cpu.reg_write(UC_X86_REG_AX, output_raw_handle)
            set_carry(cpu, bool(create_error))
        elif function == 0x3D:
            call["kind"] = "open"
            call["error"] = bool(open_error)
            cpu.reg_write(UC_X86_REG_AX, source_raw_handle)
            set_carry(cpu, bool(open_error))
        elif function == 0x3F:
            call["kind"] = "read"
            transferred = source_data[:read_count]
            if transferred:
                cpu.mem_write(
                    cpu.reg_read(UC_X86_REG_DS) * 16 + cpu.reg_read(UC_X86_REG_DX),
                    transferred,
                )
            cpu.reg_write(UC_X86_REG_AX, len(transferred))
            set_carry(cpu, False)
        elif function == 0x40:
            call["kind"] = "write"
            payload = bytes(
                cpu.mem_read(
                    cpu.reg_read(UC_X86_REG_DS) * 16 + cpu.reg_read(UC_X86_REG_DX),
                    cpu.reg_read(UC_X86_REG_CX),
                )
            )
            written_payloads.append(payload)
            cpu.reg_write(UC_X86_REG_AX, len(payload))
            set_carry(cpu, False)
        elif function == 0x3E:
            call["kind"] = "close"
            cpu.reg_write(UC_X86_REG_AX, 0)
            set_carry(cpu, False)
        else:
            raise AssertionError(f"{name}: unexpected DOS function {function:#x}")
        dos_calls.append(call)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(ENTRY, RETURN_OFFSET, count=10_000)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
    assert bytes(machine.mem_read(0, len(expected_image))) == expected_image
    assert bytes(machine.mem_read(INPUT_SEGMENT * 16, SEGMENT_SIZE)) == input_before
    assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == decoy_before
    assert (
        bytes(machine.mem_read(STACK_SEGMENT * 16 + CALLER_SP + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )

    expected_helpers = (
        []
        if matched
        else ["write_directory", "source_select"]
        if embedded
        else ["write_directory", "source_select", "resource_lookup"]
    )
    assert [call["kind"] for call in helper_calls] == expected_helpers
    for call in helper_calls:
        assert call["ds"] == GLOBALS_SEGMENT
        assert call["cs"] == 0x01E6
    if not matched:
        assert helper_calls[0]["dx"] == target_offset
        assert helper_calls[1]["dx"] == PATH_OFFSET
        if not embedded:
            assert helper_calls[2]["si"] == PATH_OFFSET

    globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
    assert globals_after[NAME_OFFSET : NAME_OFFSET + len(filename) + 1] == filename + b"\0"
    expected_target = target_storage if matched else filename + b"\0" + target_storage[len(filename) + 1 :]
    assert globals_after[target_offset : target_offset + target_size] == expected_target
    differences = {
        index
        for index, (before, after) in enumerate(zip(globals_before, globals_after))
        if before != after
    }
    assert differences <= allowed_globals, sorted(hex(item) for item in differences)

    if matched:
        expected_dos: list[str] = []
        expected_written: list[bytes] = []
    else:
        expected_dos = ["delete", "create"]
        if not embedded:
            expected_dos.append("open")
        expected_dos.extend(["read", "write"])
        if not embedded:
            expected_dos.append("close")
        expected_dos.append("close")
        expected_written = [source_data[:read_count]]
    assert [str(call["kind"]) for call in dos_calls] == expected_dos
    assert written_payloads == expected_written
    if not matched:
        read_call = next(call for call in dos_calls if call["kind"] == "read")
        write_call = next(call for call in dos_calls if call["kind"] == "write")
        assert read_call["bx"] == (EMBEDDED_HANDLE if embedded else source_raw_handle)
        assert read_call["cx"] == source_size & 0xFFFF
        assert write_call["bx"] == output_raw_handle
        assert write_call["cx"] == len(source_data[:read_count])
        close_handles = [int(call["bx"]) for call in dos_calls if call["kind"] == "close"]
        assert close_handles == ([output_raw_handle] if embedded else [source_raw_handle, output_raw_handle])

    buffer_after = bytes(machine.mem_read(BUFFER_SEGMENT * 16, SEGMENT_SIZE))
    expected_buffer = bytearray(buffer_before)
    expected_buffer[BUFFER_OFFSET : BUFFER_OFFSET + min(read_count, len(source_data))] = source_data[:read_count]
    assert buffer_after == expected_buffer
    assert machine.reg_read(UC_X86_REG_EBX) == INITIAL_GENERAL["ebx"]
    assert machine.reg_read(UC_X86_REG_ESI) == low_word(INITIAL_GENERAL["esi"], stop_offset)
    assert machine.reg_read(UC_X86_REG_DS) == INPUT_SEGMENT
    assert machine.reg_read(UC_X86_REG_ES) == GLOBALS_SEGMENT
    assert machine.reg_read(UC_X86_REG_FS) == 0x5800
    assert machine.reg_read(UC_X86_REG_GS) == GLOBALS_SEGMENT
    assert machine.reg_read(UC_X86_REG_SS) == STACK_SEGMENT
    if not matched:
        assert machine.reg_read(UC_X86_REG_EAX) == 0

    return {
        "name": name,
        "entry": f"0x{ENTRY:04x}",
        "body_sha256": BODY_SHA256,
        "slot": slot,
        "signed_slot_after_decrement": signed_slot,
        "target_offset": target_offset,
        "copied_name": filename.decode("ascii"),
        "stopping_byte": stop_byte,
        "cache_hit": matched,
        "embedded_source": embedded,
        "helper_calls": expected_helpers,
        "dos_calls": expected_dos,
        "requested_bytes": 0 if matched else source_size & 0xFFFF,
        "written_bytes": 0 if matched else len(source_data[:read_count]),
        "source_handle": None if matched else EMBEDDED_HANDLE if embedded else source_raw_handle,
        "output_handle": None if matched else output_raw_handle,
        "final_source_offset": stop_offset,
        "write_count": len(writes),
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
    if hashlib.sha256(executable[ENTRY:END]).hexdigest() != BODY_SHA256:
        raise SystemExit("BBB background-cache body changed")
    rows = [run_case(executable, index, case) for index, case in enumerate(CASES)]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"verified {len(rows)} BBB background-cache cases")


if __name__ == "__main__":
    main()
