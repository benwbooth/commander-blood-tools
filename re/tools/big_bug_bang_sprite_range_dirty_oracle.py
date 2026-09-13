#!/usr/bin/env python3
"""Verify BBB's relocated bridge-sprite range dirty transition."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_sprite_range_dirty.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_4240_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "da764ce19282c34dfa24b1408d6a13c2b702d3f53b7feb170ffd60fff2ba410a"
)

ENTRY = 0x46BD
END = 0x46EA
BODY_SHA256 = "143771d27c97685cf13341e219ffd94040d61d316b71fe1ec6ff3625549351f8"
TABLE_OFFSET = 0x65E2
RECORD_SIZE = 0x20
ACTIVE_FLAG = 0x0080
DIRTY_FLAG = 0x0002

STATE_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_SEGMENT = 0x2800
RETURN_OFFSET = 0
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("87a55a963cc37869")

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
CASES = (
    ("single_inactive", 4, (0x0000,)),
    ("single_active", 7, (0x0080,)),
    ("single_preserve_high", 9, (0xA5FF,)),
    ("mixed_range", 0x15, (0x0080, 0x0001, 0x0183, 0x55C0)),
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


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


def transitioned_flags(flags: int) -> int:
    if flags & ACTIVE_FLAG == 0:
        return flags
    return (flags & 0xFF00) | ((flags & 0x007E) | DIRTY_FLAG)


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (name, first_id, input_flags) in enumerate(CASES):
        last_id = first_id + len(input_flags) - 1
        initial = {
            "eax": 0xD1D10000 | first_id,
            "ebx": 0xB2B20000 | last_id,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0xA7A7789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": STATE_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        state_before = seeded_segment(case_index, 17, 0x21)
        data_before = seeded_segment(case_index, 19, 0x31)
        state_expected = bytearray(state_before)
        allowed_state_addresses: set[int] = set()
        output_flags = []
        for index, flags in enumerate(input_flags):
            object_id = first_id + index
            record_offset = TABLE_OFFSET + object_id * RECORD_SIZE
            record = bytes(
                (byte_index * 23 + object_id) & 0xFF
                for byte_index in range(RECORD_SIZE)
            )
            state_before[record_offset : record_offset + RECORD_SIZE] = record
            data_before[record_offset : record_offset + RECORD_SIZE] = bytes(
                byte ^ 0x5A for byte in record
            )
            struct.pack_into("<H", state_before, record_offset, flags)
            result = transitioned_flags(flags)
            struct.pack_into("<H", state_expected, record_offset, result)
            state_expected[record_offset + 2 : record_offset + RECORD_SIZE] = (
                state_before[record_offset + 2 : record_offset + RECORD_SIZE]
            )
            output_flags.append(result)
            if flags & ACTIVE_FLAG:
                allowed_state_addresses.update(
                    {
                        STATE_SEGMENT * 16 + record_offset,
                        STATE_SEGMENT * 16 + record_offset + 1,
                    }
                )
        extra_before = bytes(seeded_segment(case_index, 23, 0x43))
        fs_before = bytes(seeded_segment(case_index, 29, 0x59))
        stack_before = seeded_segment(case_index, 31, 0x67)
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack(
            "<HH", RETURN_OFFSET, RETURN_SEGMENT
        )
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, initial["eax"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 4, initial["ebx"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 6, initial["ecx"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 8, initial["ds"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 10, initial["esi"] & 0xFFFF)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(STATE_SEGMENT * 16, bytes(state_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0293)

        reached_return: list[int] = []
        allowed_addresses = allowed_state_addresses | set(
            range(
                STACK_SEGMENT * 16 + CALLER_SP - 10,
                STACK_SEGMENT * 16 + CALLER_SP,
            )
        )

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
            assert set(range(address, address + size)) <= allowed_addresses, hex(
                address
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=500)

        assert reached_return == [RETURN_ADDRESS], name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(STATE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            state_expected
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "first_object_id": first_id,
                "last_object_id": last_id,
                "input_flags": list(input_flags),
                "output_flags": output_flags,
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> None:
    actual_sha256 = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert actual_sha256 == COMMANDER_FIXTURE_SHA256, actual_sha256
    assert rows == json.loads(COMMANDER_FIXTURE.read_text())


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB sprite-range dirty body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    verify_commander_semantics(rows)
    result = {
        "format": "big_bug_bang_sprite_range_dirty_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "table_offset": TABLE_OFFSET,
            "record_size": RECORD_SIZE,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB sprite-range dirty cases")


if __name__ == "__main__":
    main()
