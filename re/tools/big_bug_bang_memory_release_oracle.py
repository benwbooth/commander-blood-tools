#!/usr/bin/env python3
"""Verify BBB's unchanged EMS and XMS backend-release routine."""

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
    UC_X86_REG_AX,
    UC_X86_REG_CS,
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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_memory_release.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_0a99_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "87cafdbed3551d0ddb4e322a6dcb7521fe1d6740dc18a6a229a887059b63a183"
)
ENTRY = 0x0C94
END = 0x0D2D
BODY_SHA256 = "fdb9ad9553b8222db768effe441446be4f050f3784ef9a4b875283e5ff33c0d2"

GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
XMS_SEGMENT = 0x7000
XMS_OFFSET = 0x0300
XMS_LINEAR = XMS_SEGMENT * 16 + XMS_OFFSET
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFE00
RETURN_OFFSET = 0x7000
STACK_SENTINEL = bytes.fromhex("a55a6996c33c5aa5")

XMS_DRIVER = 0x0C42
POOL_HANDLES = {
    "resource": (0x0C4E, 0x0C50, 0x0D02),
    "secondary": (0x0C52, 0x0C54, 0x0D16),
    "snd_bank": (0x0C56, 0x0C58, 0x0D2A),
    "small": (0x0C5A, 0x0C5C, 0x0CEE),
}
POOL_ORDER = ("small", "resource", "secondary", "snd_bank")

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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "if": 0x0200,
    "df": 0x0400,
    "of": 0x0800,
}
ARITHMETIC_FLAG_MASK = sum(
    FLAG_MASKS[name] for name in ("cf", "pf", "af", "zf", "sf", "of")
)
CASES = (
    {
        "name": "none",
        "ems": (-1, -1, -1, -1),
        "xms": (-1, -1, -1, -1),
        "initial_flags": 0x0202,
        "ems_flags": 0x0643,
        "xms_flags": 0x0A12,
    },
    {
        "name": "all",
        "ems": (0x1101, 0x2202, 0x3303, 0x4404),
        "xms": (0x5505, 0x6606, 0x7707, 0x8808),
        "initial_flags": 0x0602,
        "ems_flags": 0x0246,
        "xms_flags": 0x0893,
    },
    {
        "name": "alternating",
        "ems": (0x0000, -1, 0x7FFF, -1),
        "xms": (-1, 0x8000, -1, 0x0001),
        "initial_flags": 0x0A93,
        "ems_flags": 0x0013,
        "xms_flags": 0x0642,
    },
    {
        "name": "negative_non_sentinel",
        "ems": (0xFFFE, 0x8000, -1, 0xFFFF),
        "xms": (0xFFFF, 0xFFFE, 0x8001, -1),
        "initial_flags": 0x0246,
        "ems_flags": 0x0893,
        "xms_flags": 0x0013,
    },
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


def observed_flags(machine: Uc) -> dict[str, bool]:
    value = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(value & mask) for name, mask in FLAG_MASKS.items()}


def flags_from(value: int) -> dict[str, bool]:
    return {name: bool(value & mask) for name, mask in FLAG_MASKS.items()}


def cmp_sentinel_flags(previous: int, handle: int) -> int:
    left = handle & 0xFFFF
    right = 0xFFFF
    result = (left - right) & 0xFFFF
    value = previous & ~ARITHMETIC_FLAG_MASK
    if left < right:
        value |= FLAG_MASKS["cf"]
    if (result & 0xFF).bit_count() % 2 == 0:
        value |= FLAG_MASKS["pf"]
    if (left ^ right ^ result) & 0x10:
        value |= FLAG_MASKS["af"]
    if result == 0:
        value |= FLAG_MASKS["zf"]
    if result & 0x8000:
        value |= FLAG_MASKS["sf"]
    if (left ^ right) & (left ^ result) & 0x8000:
        value |= FLAG_MASKS["of"]
    return value


def expected_final_flags(case: dict[str, Any]) -> dict[str, bool]:
    value = int(case["initial_flags"])
    for handle in case["ems"]:
        value = cmp_sentinel_flags(value, handle)
        if handle & 0xFFFF != 0xFFFF:
            value = int(case["ems_flags"])
    for handle in case["xms"]:
        value = cmp_sentinel_flags(value, handle)
        if handle & 0xFFFF != 0xFFFF:
            value = int(case["xms_flags"])
    return flags_from(value)


def assert_unchanged_outside(
    before: bytes | bytearray, after: bytes | bytearray, allowed: set[int]
) -> None:
    differences = {
        index for index, (old, new) in enumerate(zip(before, after)) if old != new
    }
    assert differences <= allowed, sorted(hex(index) for index in differences)


def release_cases(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, case in enumerate(CASES):
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0xABCD1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": 0x6800,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        game_before = bytearray(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        struct.pack_into("<HH", game_before, XMS_DRIVER, XMS_OFFSET, XMS_SEGMENT)
        for pool, ems_handle, xms_handle in zip(POOL_ORDER, case["ems"], case["xms"]):
            xms_handle_offset, ems_handle_offset, _return_offset = POOL_HANDLES[pool]
            struct.pack_into("<H", game_before, xms_handle_offset, xms_handle & 0xFFFF)
            struct.pack_into("<H", game_before, ems_handle_offset, ems_handle & 0xFFFF)
        data_before = bytes(
            (index * 19 + case_index * 7 + 0x69) & 0xFF for index in range(SEGMENT_SIZE)
        )
        extra_before = bytes(
            (index * 29 + case_index * 13 + 0x3C) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        xms_before = bytearray(
            (index * 17 + case_index * 23 + 0x96) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        xms_before[XMS_OFFSET] = 0xCB
        stack_before = bytearray(
            (index * 13 + case_index * 17 + 0x3C) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        stack_before[CALLER_SP : CALLER_SP + 4] = struct.pack("<HH", RETURN_OFFSET, 0)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, data_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(XMS_SEGMENT * 16, bytes(xms_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[name])
        for name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, case["initial_flags"])

        calls: list[dict[str, int | str]] = []
        writes: list[tuple[int, int, int]] = []
        allowed_stack_offsets = set(range(CALLER_SP - 8, CALLER_SP))
        allowed_stack_addresses = {
            STACK_SEGMENT * 16 + offset for offset in allowed_stack_offsets
        }

        def instruction(cpu: Uc, address: int, size: int, _context: Any) -> None:
            if address == XMS_LINEAR:
                stack_pointer = cpu.reg_read(UC_X86_REG_SP)
                return_offset, return_segment = struct.unpack(
                    "<HH",
                    cpu.mem_read(STACK_SEGMENT * 16 + stack_pointer, 4),
                )
                calls.append(
                    {
                        "call": "xms_release",
                        "function": cpu.reg_read(UC_X86_REG_AX) >> 8,
                        "handle": cpu.reg_read(UC_X86_REG_DX),
                        "return_offset": return_offset,
                        "return_segment": return_segment,
                    }
                )
                cpu.reg_write(UC_X86_REG_AX, 0xCAFE)
                cpu.reg_write(UC_X86_REG_DX, 0xBABE)
                cpu.reg_write(UC_X86_REG_EFLAGS, case["xms_flags"])
                return
            assert ENTRY <= address and address + size <= END, hex(address)

        def interrupt(cpu: Uc, number: int, _context: Any) -> None:
            assert number == 0x67, hex(number)
            calls.append(
                {
                    "call": "ems_release",
                    "function": cpu.reg_read(UC_X86_REG_AX) >> 8,
                    "handle": cpu.reg_read(UC_X86_REG_DX),
                }
            )
            cpu.reg_write(UC_X86_REG_AX, 0xDEAD)
            cpu.reg_write(UC_X86_REG_DX, 0xBEEF)
            cpu.reg_write(UC_X86_REG_EFLAGS, case["ems_flags"])

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _context: Any,
        ) -> None:
            assert set(range(address, address + size)) <= allowed_stack_addresses, hex(
                address
            )
            writes.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, RETURN_OFFSET, count=2_000)

        expected_calls: list[dict[str, int | str]] = [
            {
                "call": "ems_release",
                "function": 0x45,
                "handle": handle & 0xFFFF,
            }
            for handle in case["ems"]
            if handle & 0xFFFF != 0xFFFF
        ]
        expected_calls.extend(
            {
                "call": "xms_release",
                "function": 0x0A,
                "handle": handle & 0xFFFF,
                "return_offset": POOL_HANDLES[pool][2],
                "return_segment": 0,
            }
            for pool, handle in zip(POOL_ORDER, case["xms"])
            if handle & 0xFFFF != 0xFFFF
        )
        assert calls == expected_calls, (case["name"], calls, expected_calls)
        assert snapshot_registers(machine) == initial
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert observed_flags(machine) == expected_final_flags(case)
        assert bytes(machine.mem_read(0, len(executable))) == executable
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(XMS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            xms_before
        )
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        assert_unchanged_outside(stack_before, stack_after, allowed_stack_offsets)
        assert (
            stack_after[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
            == stack_before[CALLER_SP : CALLER_SP + 4 + len(STACK_SENTINEL)]
        )
        xms_call_count = sum(call["call"] == "xms_release" for call in calls)
        assert len(writes) == 2 + 2 * xms_call_count, (case["name"], writes)
        rows.append(
            {
                "name": case["name"],
                "ems_handles": list(case["ems"]),
                "xms_handles": list(case["xms"]),
                "calls": calls,
                "state_unchanged": True,
                "final_defined_flags": expected_final_flags(case),
                "stack_write_count": len(writes),
                "stack_low_water_mark": min(
                    address for address, _size, _value in writes
                )
                - STACK_SEGMENT * 16,
            }
        )
    return rows


def commander_projection(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {
            "name": row["name"],
            "ems_handles": row["ems_handles"],
            "xms_handles": row["xms_handles"],
            "calls": [
                {
                    "call": call["call"],
                    "function": call["function"],
                    "handle": call["handle"],
                }
                for call in row["calls"]
            ],
            "state_unchanged": row["state_unchanged"],
        }
        for row in rows
    ]


def build_fixture(executable: bytes) -> dict[str, Any]:
    digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    assert digest == BODY_SHA256
    assert sha256(COMMANDER_FIXTURE) == COMMANDER_FIXTURE_SHA256
    cases = release_cases(executable)
    assert commander_projection(cases) == json.loads(COMMANDER_FIXTURE.read_text())
    return {
        "format": "big_bug_bang_memory_release_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture": {
            "path": str(COMMANDER_FIXTURE.relative_to(ROOT)),
            "sha256": COMMANDER_FIXTURE_SHA256,
        },
        "routine": {
            "operation": "extended_memory_backends_release",
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": digest,
        },
        "cases": cases,
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
    print(f"verified {len(fixture['cases'])} BBB EMS/XMS release cases")


if __name__ == "__main__":
    main()
