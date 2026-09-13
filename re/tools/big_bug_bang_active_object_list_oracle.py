#!/usr/bin/env python3
"""Verify BBB's relocated active-object list builder."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_active_object_list.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_604e_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "38747f60318445ccd51487b42f02a8c86eedb26f62a45521aaf0322581f8a885"
)

ENTRY = 0x665E
END = 0x669F
BODY_SHA256 = "33f7a526d9a2a48efaf44f511db0b324124367b71333d6e7deccc1c855517a39"
OBJECT_POINTER_OFFSET = 0x6AEC
DIRECTORY_POINTER_OFFSET = 0x6AF0
OUTPUT_OFFSET = 0x6DBE
DIRECTORY_RECORD_SIZE = 20
DIRECTORY_OBJECT_FIELD = 0x10
DIRECTORY_KIND_FIELD = 0x12
OBJECT_FLAGS_FIELD = 2
IN_PLAY_FLAG = 2
TERMINATOR = 0xFFFF

INCOMING_ES_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
INITIAL_FS_SEGMENT = 0x4000
GAME_SEGMENT = 0x5000
DIRECTORY_SEGMENT = 0x6000
RECORD_SEGMENT = 0x7000
DECOY_DIRECTORY_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
DECOY_RECORD_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x2800
RETURN_ADDRESS = RETURN_OFFSET
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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def read_wrapped(source: bytes, offset: int, size: int) -> bytes:
    return bytes(source[(offset + index) & 0xFFFF] for index in range(size))


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


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        directory_offset = expected["directory_offset"]
        record_pointer_offset = expected["record_pointer_offset_ignored"]
        entries = expected["entries"]
        active_objects = expected["active_objects"]

        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0xA7A7789A + case_index,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": INITIAL_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(case_index, 17, 0x21)
        game_before = seeded_segment(case_index, 19, 0x31)
        directory_before = seeded_segment(case_index, 23, 0x43)
        record_before = seeded_segment(case_index, 29, 0x59)
        incoming_es_before = bytes(seeded_segment(case_index, 31, 0x67))
        initial_fs_before = bytes(seeded_segment(case_index, 37, 0x71))
        decoy_directory_before = seeded_segment(case_index, 41, 0x83)
        decoy_record_before = seeded_segment(case_index, 43, 0x97)

        write_wrapped(
            game_before,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", directory_offset, DIRECTORY_SEGMENT),
        )
        write_wrapped(
            game_before,
            OBJECT_POINTER_OFFSET,
            struct.pack("<HH", record_pointer_offset, RECORD_SEGMENT),
        )
        write_wrapped(
            data_before,
            DIRECTORY_POINTER_OFFSET,
            struct.pack("<HH", 0x1800, DECOY_DIRECTORY_SEGMENT),
        )
        write_wrapped(
            data_before,
            OBJECT_POINTER_OFFSET,
            struct.pack("<HH", 0x2200, DECOY_RECORD_SEGMENT),
        )
        write_wrapped(
            decoy_directory_before,
            0x1800 + DIRECTORY_KIND_FIELD,
            struct.pack("<H", 0),
        )

        for index, entry in enumerate(entries):
            object_offset = entry["object_offset"]
            entry_kind = entry["entry_kind"]
            flags = entry["flags"]
            entry_offset = (directory_offset + index * DIRECTORY_RECORD_SIZE) & 0xFFFF
            write_wrapped(
                directory_before,
                entry_offset + DIRECTORY_OBJECT_FIELD,
                struct.pack("<H", object_offset),
            )
            write_wrapped(
                directory_before,
                entry_offset + DIRECTORY_KIND_FIELD,
                struct.pack("<H", entry_kind),
            )
            write_wrapped(
                record_before,
                object_offset + OBJECT_FLAGS_FIELD,
                struct.pack("<H", flags),
            )
            write_wrapped(
                record_before,
                record_pointer_offset + object_offset + OBJECT_FLAGS_FIELD,
                struct.pack("<H", flags ^ IN_PLAY_FLAG),
            )
            write_wrapped(
                decoy_record_before,
                object_offset + OBJECT_FLAGS_FIELD,
                struct.pack("<H", flags ^ IN_PLAY_FLAG),
            )

        game_expected = bytearray(game_before)
        result_words = [*active_objects, TERMINATOR]
        allowed_addresses: set[int] = set()
        for index, word in enumerate(result_words):
            destination = (OUTPUT_OFFSET + index * 2) & 0xFFFF
            write_wrapped(game_expected, destination, struct.pack("<H", word))
            allowed_addresses |= physical_addresses(GAME_SEGMENT, destination, 2)

        stack_before = seeded_segment(case_index, 47, 0xA9)
        write_wrapped(stack_before, CALLER_SP, struct.pack("<H", RETURN_OFFSET))
        write_wrapped(stack_before, CALLER_SP + 2, STACK_SENTINEL)
        stack_expected = bytearray(stack_before)
        for offset, value in (
            (CALLER_SP - 2, initial["eax"]),
            (CALLER_SP - 4, initial["ebx"]),
            (CALLER_SP - 6, initial["ds"]),
            (CALLER_SP - 8, initial["esi"]),
            (CALLER_SP - 10, initial["es"]),
            (CALLER_SP - 12, initial["edi"]),
            (CALLER_SP - 14, initial["fs"]),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_addresses |= physical_addresses(STACK_SEGMENT, offset, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(INITIAL_FS_SEGMENT * 16, initial_fs_before)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DIRECTORY_SEGMENT * 16, bytes(directory_before))
        machine.mem_write(RECORD_SEGMENT * 16, bytes(record_before))
        machine.mem_write(DECOY_DIRECTORY_SEGMENT * 16, bytes(decoy_directory_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(DECOY_RECORD_SEGMENT * 16, bytes(decoy_record_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0AD7)

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
            assert addresses <= allowed_addresses, hex(address)
            observed_write_addresses.update(addresses)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=2_000)

        output_words = []
        for index in range(len(result_words)):
            encoded = read_wrapped(
                bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)),
                OUTPUT_OFFSET + index * 2,
                2,
            )
            output_words.append(struct.unpack("<H", encoded)[0])
        assert output_words == result_words, name
        actual_active_objects = output_words[:-1]
        actual_flags = snapshot_flags(machine)
        assert reached_return == [RETURN_ADDRESS], name
        assert observed_write_addresses == allowed_addresses, name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert actual_flags == expected["defined_flags"], name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert (
            bytes(machine.mem_read(INITIAL_FS_SEGMENT * 16, SEGMENT_SIZE))
            == initial_fs_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(DIRECTORY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            directory_before
        )
        assert bytes(machine.mem_read(RECORD_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            record_before
        )
        assert bytes(
            machine.mem_read(DECOY_DIRECTORY_SEGMENT * 16, SEGMENT_SIZE)
        ) == bytes(decoy_directory_before)
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )
        assert bytes(
            machine.mem_read(DECOY_RECORD_SEGMENT * 16, SEGMENT_SIZE)
        ) == bytes(decoy_record_before)

        rows.append(
            {
                "name": name,
                "directory_offset": directory_offset,
                "record_pointer_offset_ignored": record_pointer_offset,
                "entries": entries,
                "active_objects": actual_active_objects,
                "terminator": output_words[-1],
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
        raise SystemExit(f"BBB active-object-list body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_active_object_list_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "object_pointer_offset": OBJECT_POINTER_OFFSET,
            "directory_pointer_offset": DIRECTORY_POINTER_OFFSET,
            "output_offset": OUTPUT_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB active-object-list cases")


if __name__ == "__main__":
    main()
