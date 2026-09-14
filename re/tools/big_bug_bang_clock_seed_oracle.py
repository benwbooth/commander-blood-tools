#!/usr/bin/env python3
"""Verify BBB's relocated CMOS-seconds PRNG seed sampler."""

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

from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_clock_seed.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_2dd3_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "fd4df48050b91be5d377e9eada52ea525f7a1b643b43c909559d8e37c11e0e0e"
)

ENTRY = 0x3154
END = 0x3163
BODY_SHA256 = "a2bd9b5514d0b1e9dd4733f42bb5efb187274e14835662835a7ff7e6c44ffcad"
SEED_OFFSET = 0x0CDA
COMMANDER_SEED_OFFSET = 0x0AEE
CMOS_INDEX_PORT = 0x70
CMOS_DATA_PORT = 0x71

GAME_SEGMENT = 0x5000
DATA_SEGMENT = 0x7000
EXTRA_SEGMENT = 0x8000
FS_SEGMENT = 0x9000
STACK_SEGMENT = 0xC000
RETURN_SEGMENT = 0xE000
RETURN_OFFSET = 0x0100
RETURN_ADDRESS = RETURN_SEGMENT * 16 + RETURN_OFFSET
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa5c33c96697887")

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


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 29 + salt) & 0xFF
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


def snapshot_defined_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {
        "cf": bool(flags & 0x0001),
        "pf": bool(flags & 0x0004),
        "zf": bool(flags & 0x0040),
        "sf": bool(flags & 0x0080),
        "of": bool(flags & 0x0800),
    }


def commander_vectors() -> list[dict[str, Any]]:
    fixture = COMMANDER_FIXTURE.read_bytes()
    assert sha256(fixture) == COMMANDER_FIXTURE_SHA256
    return json.loads(fixture)


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        seconds = expected["seconds"]
        stored_word = seconds | seconds << 8
        assert stored_word == expected["stored_word"]

        initial = {
            "eax": expected["preserved_eax"],
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0xABCD1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        game_before = seeded_segment(case_index, 13, 0x17)
        data_before = seeded_segment(case_index, 19, 0x29)
        extra_before = seeded_segment(case_index, 23, 0x3B)
        fs_before = seeded_segment(case_index, 31, 0x4D)
        stack_before = seeded_segment(case_index, 37, 0x5F)
        return_before = seeded_segment(case_index, 41, 0x71)

        for segment in (game_before, data_before, extra_before, fs_before):
            struct.pack_into("<H", segment, SEED_OFFSET, 0x5AA5 ^ case_index)
            struct.pack_into("<H", segment, COMMANDER_SEED_OFFSET, 0xA55A ^ case_index)
        struct.pack_into("<HH", stack_before, CALLER_SP, RETURN_OFFSET, RETURN_SEGMENT)
        stack_before[CALLER_SP + 4 : CALLER_SP + 4 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        return_before[RETURN_OFFSET] = 0xCC

        game_expected = bytearray(game_before)
        struct.pack_into("<H", game_expected, SEED_OFFSET, stored_word)
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, initial["eax"] & 0xFFFF)
        expected_writes = [
            (
                physical_address(STACK_SEGMENT, CALLER_SP - 2),
                2,
                initial["eax"] & 0xFFFF,
            ),
            (physical_address(GAME_SEGMENT, SEED_OFFSET), 2, stored_word),
        ]

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RETURN_SEGMENT * 16, bytes(return_before))
        module_before = bytes(machine.mem_read(0, len(executable)))
        for name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[name])
        for name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        direction_flag = bool(case_index & 1)
        machine.reg_write(
            UC_X86_REG_EFLAGS,
            0x0293 | (0x0400 if direction_flag else 0),
        )

        reads: list[list[int]] = []
        writes_to_ports: list[list[int]] = []
        memory_writes: list[tuple[int, int, int]] = []
        reached_return: list[int] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _context: object,
            reached: list[int] = reached_return,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def input_port(
            _cpu: Uc,
            port: int,
            size: int,
            _context: object,
            observed: list[list[int]] = reads,
            value: int = seconds,
        ) -> int:
            observed.append([port, size])
            return value

        def output_port(
            _cpu: Uc,
            port: int,
            size: int,
            value: int,
            _context: object,
            observed: list[list[int]] = writes_to_ports,
        ) -> None:
            observed.append([port, size, value])

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _context: object,
            observed: list[tuple[int, int, int]] = memory_writes,
        ) -> None:
            observed.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        try:
            machine.emu_start(ENTRY, 0, count=100)
        except UcError as error:
            raise RuntimeError(
                f"seconds={seconds:#x}: failed at "
                f"{machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{machine.reg_read(UC_X86_REG_IP):#x}"
            ) from error

        assert reached_return == [RETURN_ADDRESS], seconds
        assert reads == expected["port_reads"] == [[CMOS_DATA_PORT, 1]], seconds
        assert (
            writes_to_ports == expected["port_writes"] == [[CMOS_INDEX_PORT, 1, 0]]
        ), seconds
        assert memory_writes == expected_writes, (seconds, memory_writes)
        assert snapshot_registers(machine) == initial, seconds
        assert snapshot_defined_flags(machine) == {
            "cf": False,
            "pf": True,
            "zf": True,
            "sf": False,
            "of": False,
        }, seconds
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        assert bool(flags & 0x0200), seconds
        assert bool(flags & 0x0400) == direction_flag, seconds
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT, seconds
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET, seconds
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4, seconds
        assert bytes(machine.mem_read(0, len(executable))) == module_before, seconds
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        ), seconds
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        ), seconds
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        ), seconds
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        ), seconds
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        ), seconds
        assert bytes(machine.mem_read(RETURN_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            return_before
        ), seconds
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 4,
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        ), seconds

        rows.append(dict(expected))
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = sha256(executable)
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = sha256(executable[ENTRY:END])
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB clock-seed body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_clock_seed_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "seed_offset": SEED_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB clock-seed cases")


if __name__ == "__main__":
    main()
