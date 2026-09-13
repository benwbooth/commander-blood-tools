#!/usr/bin/env python3
"""Verify BBB's relocated inclusive palette interpolator."""

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

from unicorn import (  # noqa: E402
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_palette_interpolation.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_23c5_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "0b4ae7041fd5c25ed068bd83c28ffb359717d0ad675dd888c2891f450eaa0a88"
)

ENTRY = 0x2745
END = 0x27AD
BODY_SHA256 = "d20ed6eaff410820e205b6eb9b621a1af1aa879a7bd66d26482e4ee26b59509c"
DESTINATION_OFFSET = 0x5621
PALETTE_BYTE_COUNT = 768

SOURCE_SEGMENT = 0x3000
SOURCE_OFFSET = 0x1800
TARGET_SEGMENT = 0x5000
TARGET_OFFSET = 0x2400
INCOMING_FS_SEGMENT = 0x6000
GAME_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x6F40
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DEFINED_FLAG_MASK = 0x0CD5

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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def source_palette() -> bytes:
    return bytes(
        component
        for index in range(256)
        for component in (
            (index * 7 + 3) & 0x3F,
            (index * 11 + 19) & 0x3F,
            (index * 13 + 37) & 0x3F,
        )
    )


def target_palette() -> bytes:
    return bytes(
        component
        for index in range(256)
        for component in (
            (63 - index * 5) & 0x3F,
            (41 - index * 9) & 0x3F,
            (23 - index * 15) & 0x3F,
        )
    )


def signed8(value: int) -> int:
    return value - 0x100 if value & 0x80 else value


def signed_divide(dividend: int, divisor: int) -> tuple[int, int]:
    quotient = abs(dividend) // abs(divisor)
    if (dividend < 0) != (divisor < 0):
        quotient = -quotient
    return quotient, dividend - quotient * divisor


def interpolate(
    destination: bytearray,
    source: bytes,
    target: bytes,
    percent: int,
    first: int,
    last: int,
) -> tuple[int, int, int]:
    final_quotient = 0
    final_remainder = 0
    final_target = 0
    for color in range(first, last + 1):
        for component in range(3):
            offset = color * 3 + component
            delta = signed8((source[offset] - target[offset]) & 0xFF)
            final_quotient, final_remainder = signed_divide(delta * percent, 100)
            final_target = target[offset]
            destination[offset] = (final_target + final_quotient) & 0xFF
    return final_quotient, final_remainder, final_target


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def physical_addresses(segment: int, offset: int, size: int) -> set[int]:
    return {segment * 16 + ((offset + index) & 0xFFFF) for index in range(size)}


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


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    source = source_palette()
    target = target_palette()
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        percent = expected["percent"]
        first = expected["first"]
        last = expected["last"]
        percent_raw = percent & 0xFF

        source_before = seeded_segment(case_index, 11, 0x19)
        target_before = seeded_segment(case_index, 13, 0x2B)
        fs_before = seeded_segment(case_index, 19, 0x3D)
        game_before = seeded_segment(case_index, 23, 0x4F)
        write_wrapped(source_before, SOURCE_OFFSET, source)
        write_wrapped(target_before, TARGET_OFFSET, target)
        write_wrapped(
            fs_before,
            TARGET_OFFSET,
            bytes(value ^ 0xA5 for value in target),
        )
        write_wrapped(
            target_before,
            DESTINATION_OFFSET,
            bytes(
                value ^ 0x5A
                for value in game_before[
                    DESTINATION_OFFSET : DESTINATION_OFFSET + PALETTE_BYTE_COUNT
                ]
            ),
        )
        palette_before = bytes(
            (0x71 + case_index * 13 + index * 17) & 0xFF
            for index in range(PALETTE_BYTE_COUNT)
        )
        write_wrapped(game_before, DESTINATION_OFFSET, palette_before)
        game_expected = bytearray(game_before)
        expected_palette = bytearray(palette_before)
        final_quotient, final_remainder, final_target = interpolate(
            expected_palette,
            source,
            target,
            percent,
            first,
            last,
        )
        write_wrapped(game_expected, DESTINATION_OFFSET, bytes(expected_palette))

        initial = {
            "eax": 0xA5A55A00 | percent_raw,
            "ebx": 0xB6B60000 | first,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D80000 | last,
            "esi": 0xE9E90000 | SOURCE_OFFSET,
            "edi": 0xFAFA0000 | TARGET_OFFSET,
            "ebp": 0x0B0B1357 + case_index,
            "ds": SOURCE_SEGMENT,
            "es": TARGET_SEGMENT,
            "fs": INCOMING_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        stack_before = seeded_segment(case_index, 29, 0x61)
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        allowed_writes = physical_addresses(
            GAME_SEGMENT,
            DESTINATION_OFFSET + first * 3,
            expected["updated_components"],
        )
        for offset, value in (
            (CALLER_SP - 2, initial["fs"]),
            (CALLER_SP - 4, initial["ebp"]),
            (CALLER_SP - 6, initial["ecx"]),
            (CALLER_SP - 8, initial["esi"]),
            (CALLER_SP - 10, initial["es"]),
            (CALLER_SP - 12, initial["edi"]),
            (
                CALLER_SP - 14,
                initial["gs"] if first == 0 else initial["eax"],
            ),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_writes |= physical_addresses(STACK_SEGMENT, offset, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(SOURCE_SEGMENT * 16, bytes(source_before))
        machine.mem_write(TARGET_SEGMENT * 16, bytes(target_before))
        machine.mem_write(INCOMING_FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293)

        reached_return: list[int] = []
        observed_write_addresses: set[int] = set()

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= allowed_writes, hex(address)
            observed_write_addresses.update(addresses)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=50_000)

        actual_palette = bytes(
            machine.mem_read(
                GAME_SEGMENT * 16 + DESTINATION_OFFSET,
                PALETTE_BYTE_COUNT,
            )
        )
        expected_registers = dict(initial)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | (
            ((final_remainder & 0xFF) << 8) | ((final_target + final_quotient) & 0xFF)
        )
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | 0x6400 | percent_raw
        range_difference = (last - first) & 0xFFFF
        final_result = (final_target + final_quotient) & 0xFF
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | (
            (range_difference & 0xFF00) | final_result
        )

        assert reached_return == [RETURN_ADDRESS], name
        assert actual_palette == bytes(expected_palette), name
        assert (
            hashlib.sha256(actual_palette).hexdigest()
            == expected["result_palette_sha256"]
        )
        assert observed_write_addresses == allowed_writes, name
        assert snapshot_registers(machine) == expected_registers, name
        assert (
            machine.reg_read(UC_X86_REG_EFLAGS) & DEFINED_FLAG_MASK
            == expected["defined_flags"]
        )
        assert machine.reg_read(UC_X86_REG_EFLAGS) & 0x0600 == 0x0200
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(SOURCE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            source_before
        )
        assert bytes(machine.mem_read(TARGET_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            target_before
        )
        assert bytes(machine.mem_read(INCOMING_FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        if stack_after != bytes(stack_expected):
            mismatch = next(
                index
                for index, (actual, wanted) in enumerate(
                    zip(stack_after, stack_expected, strict=True)
                )
                if actual != wanted
            )
            raise AssertionError(
                f"{name}: stack[{mismatch:#06x}]={stack_after[mismatch]:#04x}, "
                f"expected={stack_expected[mismatch]:#04x}"
            )
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 4,
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        )

        rows.append(
            {
                "name": name,
                "percent": percent,
                "first": first,
                "last": last,
                "updated_components": expected["updated_components"],
                "result_palette_sha256": hashlib.sha256(actual_palette).hexdigest(),
                "final_ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                "final_bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                "final_dx": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                "defined_flags": machine.reg_read(UC_X86_REG_EFLAGS)
                & DEFINED_FLAG_MASK,
            }
        )
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB palette-interpolation body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_palette_interpolation_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "destination_offset": DESTINATION_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB palette-interpolation cases")


if __name__ == "__main__":
    main()
