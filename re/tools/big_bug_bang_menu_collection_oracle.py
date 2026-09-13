#!/usr/bin/env python3
"""Verify BBB's relocated BAS A3 menu-word collector."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_menu_collection.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_5afd_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "26052262ea89bfd26385e797eac0c38376a6c6712b47523c3bb7179be5ea07d7"
)

ENTRY = 0x6104
END = 0x613F
BODY_SHA256 = "2be92857f59e8c648b7ff55e4809d692329b88b9b7a2b302bc2cd9aabb57fe28"
CODE_POINTER_OFFSET = 0x6AF8
DEFERRED_WORD_OFFSET = 0x6B42
PROGRAM_COUNTER_OFFSET = 0x6B44
OUTPUT_OFFSET = 0x6BDC
COMMANDER_OUTPUT_OFFSET = 0x67F8
OUTPUT_WINDOW_START = OUTPUT_OFFSET - 16
OUTPUT_WINDOW_SIZE = 64
MENU_OPCODE = 0xA3

CODE_SEGMENT = 0x1800
DATA_SEGMENT = 0x3000
INCOMING_ES_SEGMENT = 0x4000
FS_SEGMENT = 0x5000
GAME_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
SOURCE_WINDOW_SIZE = SEGMENT_SIZE + 1
CALLER_SP = 0xFE00
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


def seeded_bytes(case_index: int, multiplier: int, salt: int, size: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(size)
    )


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


def snapshot_flags(machine: Uc, names: list[str]) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & FLAG_MASKS[name]) for name in names}


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        opcode = expected["opcode"]
        source_words = expected["source_words"]
        deferred = expected["deferred_before"]
        program_counter = expected["program_counter"]
        pointer_offset = expected["code_pointer_offset_ignored"]
        direction = -1 if expected["direction"] == "backward" else 1
        assert expected["code_segment"] == CODE_SEGMENT
        assert expected["storage_segment"] == "gs"

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
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        source_before = seeded_bytes(case_index, 17, 0x21, SOURCE_WINDOW_SIZE)
        source_before[program_counter] = opcode
        if pointer_offset != program_counter:
            source_before[pointer_offset] = MENU_OPCODE
        source_offset = (program_counter + 1) & 0xFFFF
        source_word_offsets = []
        for word in [*source_words, 0]:
            source_word_offsets.append(source_offset)
            source_before[source_offset] = word & 0xFF
            source_before[source_offset + 1] = word >> 8
            source_offset = (source_offset + direction * 2) & 0xFFFF
        assert source_word_offsets == expected["source_word_offsets"], name

        data_before = seeded_bytes(case_index, 19, 0x31, SEGMENT_SIZE)
        write_wrapped(
            data_before,
            CODE_POINTER_OFFSET,
            struct.pack("<HH", pointer_offset ^ 0xA5A5, INCOMING_ES_SEGMENT),
        )
        write_wrapped(
            data_before,
            DEFERRED_WORD_OFFSET,
            struct.pack("<HH", deferred ^ 0xA5A5, program_counter ^ 0x5A5A),
        )
        incoming_es_before = bytes(seeded_bytes(case_index, 23, 0x43, SEGMENT_SIZE))
        fs_before = bytes(seeded_bytes(case_index, 29, 0x59, SEGMENT_SIZE))
        game_before = seeded_bytes(case_index, 31, 0x67, SEGMENT_SIZE)
        write_wrapped(
            game_before,
            CODE_POINTER_OFFSET,
            struct.pack("<HH", pointer_offset, CODE_SEGMENT),
        )
        write_wrapped(
            game_before,
            DEFERRED_WORD_OFFSET,
            struct.pack("<HH", deferred, program_counter),
        )
        game_expected = bytearray(game_before)

        written_words: list[list[int]] = []
        allowed_addresses: set[int] = set()
        if opcode == MENU_OPCODE:
            destination = OUTPUT_OFFSET
            normalized_destination = COMMANDER_OUTPUT_OFFSET
            for word in source_words:
                write_wrapped(game_expected, destination, struct.pack("<H", word))
                written_words.append([normalized_destination, word])
                allowed_addresses |= physical_addresses(GAME_SEGMENT, destination, 2)
                destination = (destination + direction * 2) & 0xFFFF
                normalized_destination = (
                    normalized_destination + direction * 2
                ) & 0xFFFF
            if deferred != 0:
                write_wrapped(game_expected, destination, struct.pack("<H", deferred))
                written_words.append([normalized_destination, deferred])
                allowed_addresses |= physical_addresses(GAME_SEGMENT, destination, 2)
                destination = (destination + direction * 2) & 0xFFFF
                normalized_destination = (
                    normalized_destination + direction * 2
                ) & 0xFFFF
                write_wrapped(game_expected, DEFERRED_WORD_OFFSET, struct.pack("<H", 0))
                allowed_addresses |= physical_addresses(
                    GAME_SEGMENT, DEFERRED_WORD_OFFSET, 2
                )
            write_wrapped(game_expected, destination, struct.pack("<H", 0))
            written_words.append([normalized_destination, 0])
            allowed_addresses |= physical_addresses(GAME_SEGMENT, destination, 2)

        stack_before = seeded_bytes(case_index, 37, 0x71, SEGMENT_SIZE)
        write_wrapped(stack_before, CALLER_SP, struct.pack("<H", RETURN_OFFSET))
        write_wrapped(stack_before, CALLER_SP + 2, STACK_SENTINEL)
        stack_expected = bytearray(stack_before)
        for offset, value in (
            (CALLER_SP - 2, initial["eax"]),
            (CALLER_SP - 4, initial["es"]),
            (CALLER_SP - 6, initial["edi"]),
            (CALLER_SP - 8, initial["ds"]),
            (CALLER_SP - 10, initial["esi"]),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_addresses |= physical_addresses(STACK_SEGMENT, offset, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(CODE_SEGMENT * 16, bytes(source_before))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0212 | (0x0400 if direction < 0 else 0))

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

        actual_flags = snapshot_flags(machine, list(expected["defined_flags"]))
        assert reached_return == [RETURN_ADDRESS], name
        assert observed_write_addresses == allowed_addresses, name
        assert snapshot_registers(machine) == initial, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert actual_flags == expected["defined_flags"], name
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        assert bool(flags & 0x0400) == (direction < 0), name
        assert bool(flags & 0x0200), name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(CODE_SEGMENT * 16, SOURCE_WINDOW_SIZE)) == bytes(
            source_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_expected
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "opcode": opcode,
                "code_pointer_offset_ignored": pointer_offset,
                "code_segment": CODE_SEGMENT,
                "program_counter": program_counter,
                "source_word_offsets": source_word_offsets,
                "source_words": source_words,
                "deferred_before": deferred,
                "deferred_after": 0
                if opcode == MENU_OPCODE and deferred != 0
                else deferred,
                "written_words": written_words,
                "direction": expected["direction"],
                "defined_flags": actual_flags,
                "storage_segment": "gs",
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
        raise SystemExit(f"BBB menu-collection body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    expected = commander_vectors()
    assert rows == expected
    result = {
        "format": "big_bug_bang_menu_collection_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "code_pointer_offset": CODE_POINTER_OFFSET,
            "deferred_word_offset": DEFERRED_WORD_OFFSET,
            "program_counter_offset": PROGRAM_COUNTER_OFFSET,
            "output_offset": OUTPUT_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB menu-collection cases")


if __name__ == "__main__":
    main()
