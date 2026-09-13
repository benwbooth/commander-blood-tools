#!/usr/bin/env python3
"""Verify BBB's relocated CD-audio stop command."""

from __future__ import annotations

import argparse
import copy
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
    UC_HOOK_INTR,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_cd_audio_stop.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1397_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "ad73e2cee5c85efb0595b7e2eb493967178bdf793c8b66dd62fcd99aa2133a2e"
)

ENTRY = 0x1555
END = 0x1582
BODY_SHA256 = "d0940ef219168c9151cbf323faf8d152b71e874b2bd46689e0a7edbb5afd497c"
PRESENT_OFFSET = 0x0CEF
DRIVE_OFFSET = 0x0205
REQUEST_OFFSET = 0x0D7C
COMMANDER_REQUEST_OFFSET = 0x0B72
REQUEST_SIZE = 22

GAME_SEGMENT = 0x3000
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
INTERRUPT_FLAGS = 0x0881

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
    ("disabled_zero", 0x00, 0x00),
    ("disabled_other_bits", 0xFE, 0x17),
    ("enabled_drive_d", 0x01, 0x03),
    ("enabled_all_bits", 0xFF, 0xFF),
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


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, (name, present, drive) in enumerate(CASES):
        enabled = present & 1 != 0
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0x1B1B1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        request_before = bytes(
            (0x41 + case_index * 0x1B + index * 0x0D) & 0xFF
            for index in range(REQUEST_SIZE)
        )
        game_before = seeded_segment(case_index, 17, 0x21)
        game_before[PRESENT_OFFSET] = present
        game_before[DRIVE_OFFSET] = drive
        game_before[REQUEST_OFFSET : REQUEST_OFFSET + REQUEST_SIZE] = request_before
        game_expected = bytearray(game_before)
        if enabled:
            game_expected[REQUEST_OFFSET] = 0x0D
            game_expected[REQUEST_OFFSET + 2] = 0x85
        data_before = seeded_segment(case_index, 19, 0x31)
        data_before[PRESENT_OFFSET] = present ^ 0x5A
        data_before[DRIVE_OFFSET] = drive ^ 0xA5
        data_before[REQUEST_OFFSET : REQUEST_OFFSET + REQUEST_SIZE] = bytes(
            (0xA7 + case_index * 7 + index * 5) & 0xFF for index in range(REQUEST_SIZE)
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
        struct.pack_into("<H", stack_expected, CALLER_SP - 4, initial["es"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 6, initial["ebx"] & 0xFFFF)
        struct.pack_into("<H", stack_expected, CALLER_SP - 8, initial["ecx"] & 0xFFFF)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
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

        interrupts: list[dict[str, Any]] = []
        reached_return: list[int] = []
        allowed_addresses = set(
            range(STACK_SEGMENT * 16 + CALLER_SP - 8, STACK_SEGMENT * 16 + CALLER_SP)
        )
        if enabled:
            allowed_addresses.update(
                {
                    GAME_SEGMENT * 16 + REQUEST_OFFSET,
                    GAME_SEGMENT * 16 + REQUEST_OFFSET + 2,
                }
            )

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def interrupt(cpu: Uc, number: int, _data: object) -> None:
            interrupts.append(
                {
                    "number": number,
                    "ax": cpu.reg_read(UC_X86_REG_AX),
                    "bx": cpu.reg_read(UC_X86_REG_BX),
                    "cx": cpu.reg_read(UC_X86_REG_CX),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "request": bytes(
                        cpu.mem_read(GAME_SEGMENT * 16 + REQUEST_OFFSET, REQUEST_SIZE)
                    ).hex(),
                }
            )
            cpu.reg_write(UC_X86_REG_EFLAGS, INTERRUPT_FLAGS)

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
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=200)

        expected_interrupts = []
        if enabled:
            expected_interrupts.append(
                {
                    "number": 0x2F,
                    "ax": 0x1510,
                    "bx": REQUEST_OFFSET,
                    "cx": drive,
                    "es": GAME_SEGMENT,
                    "request": bytes(
                        game_expected[REQUEST_OFFSET : REQUEST_OFFSET + REQUEST_SIZE]
                    ).hex(),
                }
            )
        assert interrupts == expected_interrupts, (name, interrupts)
        assert reached_return == [RETURN_ADDRESS], name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == RETURN_SEGMENT
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        flag_mask = 0x08D5 if enabled else 0x08C5
        defined_flags = machine.reg_read(UC_X86_REG_EFLAGS) & flag_mask
        assert defined_flags == (INTERRUPT_FLAGS if enabled else 0x0044), name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
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
                "cdrom_present": present,
                "drive": drive,
                "interrupts": interrupts,
                "final_request": bytes(
                    game_expected[REQUEST_OFFSET : REQUEST_OFFSET + REQUEST_SIZE]
                ).hex(),
                "defined_flags": defined_flags,
            }
        )
    return rows


def verify_commander_semantics(rows: list[dict[str, Any]]) -> None:
    actual_sha256 = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert actual_sha256 == COMMANDER_FIXTURE_SHA256, actual_sha256
    expected = json.loads(COMMANDER_FIXTURE.read_text())
    normalized = copy.deepcopy(rows)
    for row in normalized:
        for call in row["interrupts"]:
            call["bx"] = COMMANDER_REQUEST_OFFSET
    assert normalized == expected


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB CD-audio stop body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    verify_commander_semantics(rows)
    result = {
        "format": "big_bug_bang_cd_audio_stop_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "present_offset": PRESENT_OFFSET,
            "drive_offset": DRIVE_OFFSET,
            "request_offset": REQUEST_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB CD-audio stop cases")


if __name__ == "__main__":
    main()
