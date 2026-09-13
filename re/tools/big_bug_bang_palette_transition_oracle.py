#!/usr/bin/env python3
"""Verify BBB's relocated palette-transition step and interpolation boundary."""

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
    UC_X86_REG_AX,
    UC_X86_REG_BX,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_palette_transition.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1f78_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "e3cfee664e1f4a89590d0bfecb0846af022c4a27ec19d51c174b0ee2c37e5bf1"
)

ENTRY = 0x220B
END = 0x224F
BODY_SHA256 = "9daca3215f709facb050a560d2aced0fd23d1700a6f2935ca5d268b8cf8ab7b0"
CALLBACK_SEGMENT = 0x01E6
CALLBACK_OFFSET = 0x00E5
CALLBACK_ADDRESS = CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET
CALLBACK_RETURN_OFFSET = 0x2248

INCREMENT_OFFSET = 0x561D
PERCENT_OFFSET = 0x561F
TARGET_PALETTE_OFFSET = 0x5921
SOURCE_PALETTE_OFFSET = 0x5C21
FIRST_COLOR_OFFSET = 0x5F21
LAST_COLOR_OFFSET = 0x5F22
DIRTY_OFFSET = 0x5F25

COMMANDER_CALLBACK_SEGMENT = 0x01CE
COMMANDER_INCREMENT_OFFSET = 0x524D
COMMANDER_PERCENT_OFFSET = 0x524F
COMMANDER_TARGET_PALETTE_OFFSET = 0x5551
COMMANDER_SOURCE_PALETTE_OFFSET = 0x5851
COMMANDER_FIRST_COLOR_OFFSET = 0x5B51
COMMANDER_LAST_COLOR_OFFSET = 0x5B52
COMMANDER_DIRTY_OFFSET = 0x5B55

DATA_SEGMENT = 0x3000
GAME_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
PALETTE_BYTE_COUNT = 768
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x6F20
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa5966987783cc3")

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
    "df": 0x0400,
    "of": 0x0800,
}


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


def snapshot_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def subtract16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": ((left ^ right ^ result) & 0x10) != 0,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
    }


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def data_image(case_index: int) -> bytearray:
    return bytearray(
        (offset * 29 + (offset >> 8) * 11 + case_index * 41 + 0x17) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def game_image(case_index: int) -> bytes:
    return bytes(
        (offset * 17 + (offset >> 7) * 5 + case_index * 23 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def seed_state(
    data: bytearray,
    increment_offset: int,
    percent_offset: int,
    first_offset: int,
    last_offset: int,
    dirty_offset: int,
    vector: dict[str, Any],
) -> None:
    write_wrapped(data, increment_offset, struct.pack("<H", vector["increment"]))
    write_wrapped(data, percent_offset, struct.pack("<H", vector["initial_percent"]))
    data[first_offset] = vector["first"]
    data[last_offset] = vector["last"]
    data[dirty_offset] = vector["dirty_before"]


def normalized_data_hash(case_index: int, vector: dict[str, Any]) -> str:
    data = data_image(case_index)
    seed_state(
        data,
        COMMANDER_INCREMENT_OFFSET,
        COMMANDER_PERCENT_OFFSET,
        COMMANDER_FIRST_COLOR_OFFSET,
        COMMANDER_LAST_COLOR_OFFSET,
        COMMANDER_DIRTY_OFFSET,
        vector,
    )
    if vector["active"]:
        write_wrapped(
            data,
            COMMANDER_PERCENT_OFFSET,
            struct.pack("<H", vector["result_percent"]),
        )
        data[COMMANDER_DIRTY_OFFSET] = vector["dirty_after"]
    return hashlib.sha256(data).hexdigest()


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        active = expected["active"]
        initial_flags = 0x0202 | (0x0400 if case_index & 1 else 0)
        helper_flags = 0x0295 | (0x0C40 if case_index & 1 else 0)

        data_before = data_image(case_index)
        seed_state(
            data_before,
            INCREMENT_OFFSET,
            PERCENT_OFFSET,
            FIRST_COLOR_OFFSET,
            LAST_COLOR_OFFSET,
            DIRTY_OFFSET,
            expected,
        )
        data_expected = bytearray(data_before)
        if active:
            write_wrapped(
                data_expected,
                PERCENT_OFFSET,
                struct.pack("<H", expected["result_percent"]),
            )
            data_expected[DIRTY_OFFSET] = expected["dirty_after"]

        game_before = game_image(case_index)
        extra_before = bytes(
            (offset * 13 + case_index * 19 + 0x53) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        fs_before = bytes(
            (offset * 23 + case_index * 17 + 0x67) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }

        stack_before = bytearray(
            (offset * 31 + (offset >> 8) * 7 + case_index * 29 + 0x79) & 0xFF
            for offset in range(SEGMENT_SIZE)
        )
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        allowed_addresses: set[int] = set()
        for offset, value in (
            (CALLER_SP - 2, initial["eax"]),
            (CALLER_SP - 4, initial["edi"]),
            (CALLER_SP - 6, initial["esi"]),
            (CALLER_SP - 8, initial["ebx"]),
            (CALLER_SP - 10, initial["edx"]),
            (CALLER_SP - 12, initial["es"]),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_addresses |= physical_addresses(STACK_SEGMENT, offset, 2)
        if active:
            write_wrapped(
                stack_expected,
                CALLER_SP - 16,
                struct.pack("<HH", CALLBACK_RETURN_OFFSET, 0),
            )
            allowed_addresses |= physical_addresses(STACK_SEGMENT, CALLER_SP - 16, 4)
            allowed_addresses |= physical_addresses(DATA_SEGMENT, DIRTY_OFFSET, 1)
            allowed_addresses |= physical_addresses(DATA_SEGMENT, PERCENT_OFFSET, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        machine.mem_write(CALLBACK_ADDRESS, b"\xcb")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(GAME_SEGMENT * 16, game_before)
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, initial_flags)

        calls: list[dict[str, Any]] = []
        reached_return: list[int] = []
        observed_write_addresses: set[int] = set()
        raw_expected_call = None
        normalized_call = None
        if active:
            normalized_call = dict(expected["helper_call"])
            raw_expected_call = {
                **normalized_call,
                "cs": CALLBACK_SEGMENT,
                "si": SOURCE_PALETTE_OFFSET,
                "di": TARGET_PALETTE_OFFSET,
                "source_sha256": hashlib.sha256(
                    data_before[
                        SOURCE_PALETTE_OFFSET : SOURCE_PALETTE_OFFSET
                        + PALETTE_BYTE_COUNT
                    ]
                ).hexdigest(),
                "target_sha256": hashlib.sha256(
                    game_before[
                        TARGET_PALETTE_OFFSET : TARGET_PALETTE_OFFSET
                        + PALETTE_BYTE_COUNT
                    ]
                ).hexdigest(),
            }

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls_for_case: list[dict[str, Any]] = calls,
            reached: list[int] = reached_return,
            expected_data: bytes = bytes(data_expected),
            expected_stack: bytes = bytes(stack_expected),
            expected_call: dict[str, Any] | None = raw_expected_call,
            normalized: dict[str, Any] | None = normalized_call,
            case_name: str = name,
            case_number: int = case_index,
            callback_flags: int = helper_flags,
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            if address != CALLBACK_ADDRESS:
                assert ENTRY <= address < END, hex(address)
                return

            assert bytes(cpu.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == expected_data
            assert (
                bytes(cpu.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == expected_stack
            )
            raw_call = {
                "callee": "palette_range_interpolate",
                "cs": cpu.reg_read(UC_X86_REG_CS),
                "ip": cpu.reg_read(UC_X86_REG_IP),
                "sp": cpu.reg_read(UC_X86_REG_SP),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "si": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                "es": cpu.reg_read(UC_X86_REG_ES),
                "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                "gs": cpu.reg_read(UC_X86_REG_GS),
                "ax": cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                "bx": cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                "dx": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                "percent_signed_byte": (
                    (normalized["ax"] & 0xFF) - 0x100
                    if normalized is not None and normalized["ax"] & 0x80
                    else normalized["ax"] & 0xFF
                ),
                "source_sha256": hashlib.sha256(
                    cpu.mem_read(
                        DATA_SEGMENT * 16 + SOURCE_PALETTE_OFFSET,
                        PALETTE_BYTE_COUNT,
                    )
                ).hexdigest(),
                "target_sha256": hashlib.sha256(
                    cpu.mem_read(
                        GAME_SEGMENT * 16 + TARGET_PALETTE_OFFSET,
                        PALETTE_BYTE_COUNT,
                    )
                ).hexdigest(),
            }
            assert expected_call is not None
            assert normalized is not None
            assert raw_call == expected_call, (case_name, raw_call, expected_call)
            calls_for_case.append(dict(normalized))
            cpu.reg_write(UC_X86_REG_AX, 0xE001 + case_number)
            cpu.reg_write(UC_X86_REG_BX, 0xE102 + case_number)
            cpu.reg_write(UC_X86_REG_DX, 0xE203 + case_number)
            cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
            allowed: set[int] = allowed_addresses,
            observed: set[int] = observed_write_addresses,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= allowed, hex(address)
            observed.update(addresses)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=512)

        actual_flags = snapshot_flags(machine)
        if not active:
            expected_flags = subtract16_flags(expected["initial_percent"], 100)
            expected_flags["df"] = bool(initial_flags & 0x0400)
            assert actual_flags == expected_flags, (name, actual_flags, expected_flags)
        assert actual_flags == expected["defined_flags"], name
        assert calls == ([expected["helper_call"]] if active else []), name
        assert reached_return == [RETURN_ADDRESS], name
        assert observed_write_addresses == allowed_addresses, name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert machine.reg_read(UC_X86_REG_EFLAGS) & 0x0200 == 0x0200
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_before
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        data_sha256 = normalized_data_hash(case_index, expected)
        assert data_sha256 == expected["data_sha256"], name
        rows.append(
            {
                "name": name,
                "initial_percent": expected["initial_percent"],
                "increment": expected["increment"],
                "result_percent": expected["result_percent"],
                "active": active,
                "first": expected["first"],
                "last": expected["last"],
                "dirty_before": expected["dirty_before"],
                "dirty_after": expected["dirty_after"],
                "helper_call": calls[0] if calls else None,
                "data_sha256": data_sha256,
                "defined_flags": actual_flags,
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
        raise SystemExit(f"BBB palette-transition body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_palette_transition_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "callback": f"{CALLBACK_SEGMENT:04x}:{CALLBACK_OFFSET:04x}",
            "increment_offset": INCREMENT_OFFSET,
            "percent_offset": PERCENT_OFFSET,
            "source_palette_offset": SOURCE_PALETTE_OFFSET,
            "target_palette_offset": TARGET_PALETTE_OFFSET,
            "first_color_offset": FIRST_COLOR_OFFSET,
            "last_color_offset": LAST_COLOR_OFFSET,
            "dirty_offset": DIRTY_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB palette-transition cases")


if __name__ == "__main__":
    main()
