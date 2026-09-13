#!/usr/bin/env python3
"""Verify BBB's simple unchanged DESCRIPT parser handlers."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_parser_handlers.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

GLOBALS_SEGMENT = 0x3000
INPUT_SEGMENT = 0x4000
SPRITE_SEGMENT = 0x5000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
RECORD_BOUNDARY = 0x0D20
LOCATION_LAYOUT = 0x21F3
SPRITE_DIRTY = 0x2A89

ROUTINES = (
    (0x8584, 0x858B, "location_boundary"),
    (0x858B, 0x8592, "character_boundary"),
    (0x8592, 0x8599, "object_boundary"),
    (0x8599, 0x85A0, "sequence_boundary"),
    (0x866B, 0x8680, "location_video"),
    (0x86FC, 0x8702, "location_layout"),
    (0x8702, 0x8717, "character_right_video"),
    (0x8717, 0x872C, "character_left_video"),
    (0x87CA, 0x87EB, "character_sprite"),
)
ROUTINE_HASHES = {
    0x8584: "3d2174e1a123a0135fd3810054da74d48dc0c36d34c143a20665d2d29c28d7d7",
    0x858B: "3d2174e1a123a0135fd3810054da74d48dc0c36d34c143a20665d2d29c28d7d7",
    0x8592: "3d2174e1a123a0135fd3810054da74d48dc0c36d34c143a20665d2d29c28d7d7",
    0x8599: "3d2174e1a123a0135fd3810054da74d48dc0c36d34c143a20665d2d29c28d7d7",
    0x866B: "43a5acd93a9f4d20daa46d76c835d04f133a03b07fa0ee273fe7716fa888fedf",
    0x86FC: "0320042d20faf2bdf739ab24069a4e367457395b42ccc414fe8a1e691c4f38e8",
    0x8702: "edc9f3bf5a711387d836e838786ef303056e563d2f5f2101a1c1fee47d2dc5d7",
    0x8717: "3c122147c705d92c1dfb68fbaa0a08adc745dfacf37a1534168a545568f66bdd",
    0x87CA: "7690029ba518b08a94250be8588201485b60ac9d9ef9f9d3fa56c0df7da7673e",
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


def setup_machine(
    executable: bytes,
    globals_before: bytearray,
    input_before: bytearray,
    sprite_before: bytearray,
    source_offset: int,
) -> tuple[Uc, dict[str, str]]:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(GLOBALS_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(INPUT_SEGMENT * 16, bytes(input_before))
    machine.mem_write(SPRITE_SEGMENT * 16, bytes(sprite_before))
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
    machine.reg_write(UC_X86_REG_FS, SPRITE_SEGMENT)
    machine.reg_write(UC_X86_REG_GS, GLOBALS_SEGMENT)
    machine.reg_write(UC_X86_REG_SS, STACK_SEGMENT)
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0202)
    return machine, register_snapshot(machine)


def execute(
    executable: bytes,
    entry: int,
    end: int,
    globals_before: bytearray,
    input_before: bytearray,
    sprite_before: bytearray,
    source_offset: int,
    expected_writes: set[int],
) -> tuple[Uc, dict[str, str], dict[str, str]]:
    machine, registers_before = setup_machine(
        executable, globals_before, input_before, sprite_before, source_offset
    )
    writes: set[int] = set()
    executed: list[tuple[int, int]] = []

    def instruction(_machine: Uc, address: int, size: int, _context: Any) -> None:
        assert entry <= address and address + size <= end, hex(address)
        executed.append((address, size))

    def write_hook(
        _machine: Uc,
        _access: int,
        address: int,
        size: int,
        _value: int,
        _context: Any,
    ) -> None:
        written = set(range(address, address + size))
        assert written <= expected_writes, (hex(address), size)
        writes.update(written)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    machine.emu_start(entry, RETURN_OFFSET, count=256)

    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
    assert machine.reg_read(UC_X86_REG_SP) == (CALLER_SP + 2) & 0xFFFF
    assert writes == expected_writes, (sorted(writes), sorted(expected_writes))
    assert executed and executed[0][0] == entry
    assert bytes(machine.mem_read(0, len(executable))) == executable
    assert bytes(machine.mem_read(INPUT_SEGMENT * 16, SEGMENT_SIZE)) == input_before
    return machine, registers_before, register_snapshot(machine)


def assert_unchanged_outside(
    before: bytes | bytearray,
    after: bytes | bytearray,
    allowed_offsets: set[int],
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed_offsets, sorted(hex(index) for index in differences)


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


def boundary_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for entry, end, operation in ROUTINES[:4]:
        for name, flag_before in (("already_set", 1), ("overwrite_marker", 0xA5)):
            globals_before = bytearray([0xA5]) * SEGMENT_SIZE
            globals_before[RECORD_BOUNDARY] = flag_before
            input_before = bytearray([0xCC]) * SEGMENT_SIZE
            sprite_before = bytearray([0x5A]) * SEGMENT_SIZE
            expected_writes = {GLOBALS_SEGMENT * 16 + RECORD_BOUNDARY}
            machine, registers_before, registers_after = execute(
                executable,
                entry,
                end,
                globals_before,
                input_before,
                sprite_before,
                0x6123,
                expected_writes,
            )
            globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
            sprite_after = bytes(machine.mem_read(SPRITE_SEGMENT * 16, SEGMENT_SIZE))
            assert globals_after[RECORD_BOUNDARY] == 1
            assert_unchanged_outside(globals_before, globals_after, {RECORD_BOUNDARY})
            assert sprite_after == sprite_before
            assert registers_after == {
                **registers_before,
                "flags": registers_before["flags"],
            }
            rows.append(
                {
                    "type": "boundary",
                    "operation": operation,
                    "name": name,
                    "entry": f"0x{entry:04x}",
                    "flag_before": flag_before,
                    "flag_after": globals_after[RECORD_BOUNDARY],
                    "stack_pointer_before": CALLER_SP,
                    "stack_pointer_after": machine.reg_read(UC_X86_REG_SP),
                    "registers_before": registers_before,
                    "registers_after": registers_after,
                }
            )
    return rows


PRINTABLE_CASES = (
    ("zero_at_offset_zero", 0x0000, b"\x00"),
    ("immediate_control", 0x6800, b"\x1f"),
    ("immediate_high", 0x6820, b"\x80"),
    ("max_printable", 0x6840, b"\x7f\x00"),
    ("text_then_control", 0x6860, b"ABC\x1f"),
    ("text_then_high", 0x6880, b"Z\xff"),
    ("script_wrap", 0xFFFE, b"AB\x00"),
    ("high_at_segment_end", 0xFFFF, b"\x80"),
)
VIDEO_ROUTINES = (
    (0x866B, 0x8680, "location_video", 0x230A),
    (0x8702, 0x8717, "character_right_video", 0x26B2),
    (0x8717, 0x872C, "character_left_video", 0x26CC),
)


def copied_prefix(payload: bytes) -> tuple[bytes, int]:
    for index, value in enumerate(payload):
        if value < 0x20 or value >= 0x80:
            return payload[:index], value
    raise AssertionError("printable case has no stopping byte")


def printable_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for entry, end, operation, destination in VIDEO_ROUTINES:
        for name, source_offset, payload in PRINTABLE_CASES:
            copied, stopping_byte = copied_prefix(payload)
            globals_before = bytearray([0xA5]) * SEGMENT_SIZE
            input_before = bytearray([0xCC]) * SEGMENT_SIZE
            sprite_before = bytearray([0x5A]) * SEGMENT_SIZE
            write_wrapped(input_before, source_offset, payload)
            destination_offsets = set(range(destination, destination + len(copied) + 1))
            expected_writes = {
                GLOBALS_SEGMENT * 16 + offset for offset in destination_offsets
            }
            machine, registers_before, registers_after = execute(
                executable,
                entry,
                end,
                globals_before,
                input_before,
                sprite_before,
                source_offset,
                expected_writes,
            )
            globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
            sprite_after = bytes(machine.mem_read(SPRITE_SEGMENT * 16, SEGMENT_SIZE))
            assert globals_after[destination : destination + len(copied)] == copied
            assert globals_after[destination + len(copied)] == 0
            assert_unchanged_outside(globals_before, globals_after, destination_offsets)
            assert sprite_after == sprite_before
            final_source = (source_offset + len(copied)) & 0xFFFF
            final_destination = destination + len(copied)
            assert machine.reg_read(UC_X86_REG_ESI) == low_word(
                INITIAL_GENERAL["esi"], final_source
            )
            assert machine.reg_read(UC_X86_REG_EDI) == low_word(
                INITIAL_GENERAL["edi"], final_destination
            )
            for register in ("ebx", "ecx", "edx", "ebp"):
                assert registers_after[register] == registers_before[register]
            for register in SEGMENT_REGISTERS:
                assert registers_after[register] == registers_before[register]
            rows.append(
                {
                    "type": "video",
                    "operation": operation,
                    "name": name,
                    "entry": f"0x{entry:04x}",
                    "source_offset": source_offset,
                    "input_hex": payload.hex(),
                    "copied_hex": copied.hex(),
                    "stopping_byte": stopping_byte,
                    "destination_offset": destination,
                    "final_source_offset": final_source,
                    "final_destination_offset": final_destination,
                    "stack_pointer_before": CALLER_SP,
                    "stack_pointer_after": machine.reg_read(UC_X86_REG_SP),
                    "registers_before": registers_before,
                    "registers_after": registers_after,
                }
            )
    return rows


LAYOUT_CASES = (
    ("zero", 0x6000, 0x0000, 0x3100),
    ("all_ones", 0x6020, 0xFFFF, 0x3211),
    ("little_endian", 0x6040, 0x1234, 0x3322),
    ("high_bit", 0x6060, 0x8001, 0x3433),
    ("low_byte_ff", 0x6080, 0x12FF, 0x3544),
    ("high_byte_ff", 0x60A0, 0xFF12, 0x3655),
    ("unaligned", 0x60C1, 0xA55A, 0x3766),
    ("source_end_wrap", 0xFFFE, 0xCAFE, 0x3877),
)


def layout_cases(executable: bytes) -> list[dict[str, Any]]:
    entry, end = 0x86FC, 0x8702
    rows = []
    for name, source_offset, operand, destination_before in LAYOUT_CASES:
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        sprite_before = bytearray([0x5A]) * SEGMENT_SIZE
        set_word(globals_before, LOCATION_LAYOUT, destination_before)
        write_wrapped(input_before, source_offset, struct.pack("<H", operand))
        destination_offsets = {LOCATION_LAYOUT, LOCATION_LAYOUT + 1}
        expected_writes = {
            GLOBALS_SEGMENT * 16 + offset for offset in destination_offsets
        }
        machine, registers_before, registers_after = execute(
            executable,
            entry,
            end,
            globals_before,
            input_before,
            sprite_before,
            source_offset,
            expected_writes,
        )
        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        sprite_after = bytes(machine.mem_read(SPRITE_SEGMENT * 16, SEGMENT_SIZE))
        assert word(globals_after, LOCATION_LAYOUT) == operand
        assert_unchanged_outside(globals_before, globals_after, destination_offsets)
        assert sprite_after == sprite_before
        final_source = (source_offset + 2) & 0xFFFF
        assert machine.reg_read(UC_X86_REG_EAX) == low_word(INITIAL_GENERAL["eax"], operand)
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(
            INITIAL_GENERAL["esi"], final_source
        )
        for register in ("ebx", "ecx", "edx", "edi", "ebp"):
            assert registers_after[register] == registers_before[register]
        for register in SEGMENT_REGISTERS:
            assert registers_after[register] == registers_before[register]
        assert registers_after["flags"] == registers_before["flags"]
        rows.append(
            {
                "type": "layout",
                "operation": "location_layout",
                "name": name,
                "entry": f"0x{entry:04x}",
                "source_offset": source_offset,
                "input_hex": struct.pack("<H", operand).hex(),
                "operand": operand,
                "destination_before": destination_before,
                "destination_after": word(globals_after, LOCATION_LAYOUT),
                "final_source_offset": final_source,
                "stack_pointer_before": CALLER_SP,
                "stack_pointer_after": machine.reg_read(UC_X86_REG_SP),
                "registers_before": registers_before,
                "registers_after": registers_after,
            }
        )
    return rows


def sprite_cases(executable: bytes) -> list[dict[str, Any]]:
    entry, end, destination = 0x87CA, 0x87EB, 0x0C74
    rows = []
    for name, source_offset, payload in PRINTABLE_CASES:
        copied, stopping_byte = copied_prefix(payload)
        globals_before = bytearray([0xA5]) * SEGMENT_SIZE
        globals_before[SPRITE_DIRTY] = 0xA5
        input_before = bytearray([0xCC]) * SEGMENT_SIZE
        sprite_before = bytearray([0x5A]) * SEGMENT_SIZE
        write_wrapped(input_before, source_offset, payload)
        destination_offsets = set(range(destination, destination + len(copied) + 1))
        stack_offsets = {CALLER_SP - 2, CALLER_SP - 1}
        expected_writes = {
            SPRITE_SEGMENT * 16 + offset for offset in destination_offsets
        }
        expected_writes.add(GLOBALS_SEGMENT * 16 + SPRITE_DIRTY)
        expected_writes.update(STACK_SEGMENT * 16 + offset for offset in stack_offsets)
        machine, registers_before, registers_after = execute(
            executable,
            entry,
            end,
            globals_before,
            input_before,
            sprite_before,
            source_offset,
            expected_writes,
        )
        globals_after = bytes(machine.mem_read(GLOBALS_SEGMENT * 16, SEGMENT_SIZE))
        sprite_after = bytes(machine.mem_read(SPRITE_SEGMENT * 16, SEGMENT_SIZE))
        assert globals_after[SPRITE_DIRTY] == 1
        assert_unchanged_outside(globals_before, globals_after, {SPRITE_DIRTY})
        assert sprite_after[destination : destination + len(copied)] == copied
        assert sprite_after[destination + len(copied)] == 0
        assert_unchanged_outside(sprite_before, sprite_after, destination_offsets)
        final_source = (source_offset + len(copied)) & 0xFFFF
        final_destination = destination + len(copied)
        assert machine.reg_read(UC_X86_REG_ESI) == low_word(
            INITIAL_GENERAL["esi"], final_source
        )
        assert machine.reg_read(UC_X86_REG_EDI) == low_word(
            INITIAL_GENERAL["edi"], final_destination
        )
        for register in ("ebx", "ecx", "edx", "ebp"):
            assert registers_after[register] == registers_before[register]
        for register in SEGMENT_REGISTERS:
            assert registers_after[register] == registers_before[register]
        rows.append(
            {
                "type": "sprite",
                "operation": "character_sprite",
                "name": name,
                "entry": f"0x{entry:04x}",
                "source_offset": source_offset,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stopping_byte,
                "destination_offset": destination,
                "final_source_offset": final_source,
                "final_destination_offset": final_destination,
                "dirty_before": 0xA5,
                "dirty_after": globals_after[SPRITE_DIRTY],
                "stack_pointer_before": CALLER_SP,
                "stack_pointer_after": machine.reg_read(UC_X86_REG_SP),
                "registers_before": registers_before,
                "registers_after": registers_after,
            }
        )
    return rows


def build_fixture(executable: bytes) -> dict[str, Any]:
    return {
        "format": "big_bug_bang_simple_parser_handlers_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "routines": routine_rows(executable),
        "cases": [
            *boundary_cases(executable),
            *printable_cases(executable),
            *layout_cases(executable),
            *sprite_cases(executable),
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
        f"{len(fixture['routines'])} BBB parser handlers"
    )


if __name__ == "__main__":
    main()
