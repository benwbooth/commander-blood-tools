#!/usr/bin/env python3
"""Verify BBB's relocated navigation actor-target list builder."""

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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_navigation_actor_targets.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_71cf_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "efe64f9dc59f4042d75a19d9521385ee418c45eab8651d94689df50dd4170297"
)

ENTRY = 0x81E6
END = 0x8231
BODY_SHA256 = "ab45f9f851a1f0ab2c13dede016d5b1c1c0ae37db2ece33c4a114d12c91e0084"
HELPER_ENTRY = 0x665E
HELPER_RETURN = 0x81F3
SOURCE_OFFSET = 0x6DBE
RECORD_POINTER_OFFSET = 0x6AEC
HONK_OFFSET = 0x6B24
RADIO_OFFSET = 0x6B26
OUTPUT_OFFSET = 0x2DC3
TERMINATOR = 0xFFFF

INCOMING_ES_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
FS_SEGMENT = 0x4000
GAME_SEGMENT = 0x5000
DECOY_RECORD_SEGMENT = 0x6000
STACK_SEGMENT = 0x9000
RECORD_SEGMENT = 0xA000
SEGMENT_SIZE = 0x10000
RECORD_IMAGE_SIZE = 0x20020
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x2A00
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("a5875a693c9678c3")

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


def seeded_bytes(size: int, case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 29 + salt) & 0xFF
        for offset in range(size)
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
        source = expected["source"]
        output = expected["output"]
        honk = expected["excluded"]["honk"]
        radio = expected["excluded"]["menu"]
        record_base = expected["record_base"][1]
        direction_flag = expected["direction_flag"]
        source_step = -2 if direction_flag else 2

        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62345 + case_index,
            "ecx": 0xC7C73456 + case_index,
            "edx": 0xD8D84567 + case_index,
            "esi": 0xE9E95678 + case_index,
            "edi": 0x2468 + case_index,
            "ebp": 0xABCD789A + case_index,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        incoming_es_before = seeded_bytes(SEGMENT_SIZE, case_index, 11, 0x17)
        data_before = bytearray(seeded_bytes(SEGMENT_SIZE, case_index, 13, 0x29))
        fs_before = seeded_bytes(SEGMENT_SIZE, case_index, 19, 0x3B)
        game_before = bytearray(seeded_bytes(SEGMENT_SIZE, case_index, 23, 0x4D))
        decoy_record_before = bytearray(
            seeded_bytes(RECORD_IMAGE_SIZE, case_index, 31, 0x61)
        )
        record_before = bytearray(seeded_bytes(RECORD_IMAGE_SIZE, case_index, 37, 0x73))

        write_wrapped(
            data_before,
            RECORD_POINTER_OFFSET,
            struct.pack("<HH", record_base, RECORD_SEGMENT),
        )
        write_wrapped(data_before, HONK_OFFSET, struct.pack("<HH", 0xDEAD, 0xBEEF))
        write_wrapped(
            game_before,
            RECORD_POINTER_OFFSET,
            struct.pack("<HH", 0x0400, DECOY_RECORD_SEGMENT),
        )
        write_wrapped(game_before, HONK_OFFSET, struct.pack("<HH", honk, radio))
        write_wrapped(data_before, OUTPUT_OFFSET, b"\x19\x37\x5b\x7d")

        source_words = [*source, TERMINATOR]
        data_expected = bytearray(data_before)
        for index, value in enumerate(source_words):
            write_wrapped(
                data_expected,
                SOURCE_OFFSET + index * source_step,
                struct.pack("<H", value),
            )

        for encoded_offset, values in expected["objects"].items():
            object_offset = int(encoded_offset, 16)
            kind, flags = values
            object_address = record_base + object_offset
            struct.pack_into("<H", record_before, object_address, kind)
            record_before[object_address + 2] = flags
            decoy_address = 0x0400 + object_offset
            struct.pack_into("<H", decoy_record_before, decoy_address, kind ^ 2)
            decoy_record_before[decoy_address + 2] = flags ^ 2

        stack_before = bytearray(seeded_bytes(SEGMENT_SIZE, case_index, 41, 0x85))
        write_wrapped(
            stack_before,
            OUTPUT_OFFSET,
            b"".join(struct.pack("<H", 0xD000 + index) for index in range(16)),
        )
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        output_words = [*output, TERMINATOR]
        allowed_writes: set[int] = set()
        for index, value in enumerate(output_words):
            destination = OUTPUT_OFFSET + index * 2
            write_wrapped(stack_expected, destination, struct.pack("<H", value))
            allowed_writes |= physical_addresses(STACK_SEGMENT, destination, 2)
        for offset, value in (
            (CALLER_SP - 2, initial["ebx"]),
            (CALLER_SP - 4, initial["es"]),
            (CALLER_SP - 6, initial["edi"]),
            (CALLER_SP - 8, initial["esi"]),
            (CALLER_SP - 10, initial["ebp"]),
            (CALLER_SP - 12, HELPER_RETURN),
        ):
            write_wrapped(stack_expected, offset, struct.pack("<H", value & 0xFFFF))
            allowed_writes |= physical_addresses(STACK_SEGMENT, offset, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(HELPER_ENTRY, b"\xc3")
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(DECOY_RECORD_SEGMENT * 16, bytes(decoy_record_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        machine.mem_write(RECORD_SEGMENT * 16, bytes(record_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(
            UC_X86_REG_EFLAGS,
            0x0293 | (0x0400 if direction_flag else 0),
        )

        helper_calls: list[dict[str, int]] = []
        reached_return: list[int] = []
        observed_write_addresses: set[int] = set()

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            if address == HELPER_ENTRY:
                stack_pointer = cpu.reg_read(UC_X86_REG_SP)
                helper_calls.append(
                    {
                        "eax": cpu.reg_read(UC_X86_REG_EAX),
                        "cx": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "gs": cpu.reg_read(UC_X86_REG_GS),
                        "return_ip": struct.unpack(
                            "<H",
                            cpu.mem_read(STACK_SEGMENT * 16 + stack_pointer, 2),
                        )[0],
                    }
                )
                for index, value in enumerate(source_words):
                    cpu.mem_write(
                        DATA_SEGMENT * 16
                        + ((SOURCE_OFFSET + index * source_step) & 0xFFFF),
                        struct.pack("<H", value),
                    )
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
        machine.emu_start(ENTRY, 0, count=2_000)

        actual_output = []
        stack_after = bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE))
        for index in range(len(output_words)):
            actual_output.append(
                struct.unpack(
                    "<H",
                    read_wrapped(stack_after, OUTPUT_OFFSET + index * 2, 2),
                )[0]
            )
        expected_helper = {
            "eax": 0,
            "cx": 0,
            "ds": DATA_SEGMENT,
            "gs": GAME_SEGMENT,
            "return_ip": HELPER_RETURN,
        }
        expected_registers = dict(initial)
        expected_registers["eax"] = len(output)
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | len(output)
        expected_flags = {
            "cf": False,
            "pf": True,
            "af": False,
            "zf": True,
            "sf": False,
            "if": True,
            "df": direction_flag,
            "of": False,
        }

        assert helper_calls == [expected_helper], name
        assert reached_return == [RETURN_ADDRESS], name
        assert actual_output == output_words, name
        assert observed_write_addresses == allowed_writes, name
        assert snapshot_registers(machine) == expected_registers, name
        assert snapshot_flags(machine) == expected_flags, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(
            machine.mem_read(DECOY_RECORD_SEGMENT * 16, RECORD_IMAGE_SIZE)
        ) == bytes(decoy_record_before)
        assert stack_after == bytes(stack_expected)
        assert bytes(machine.mem_read(RECORD_SEGMENT * 16, RECORD_IMAGE_SIZE)) == bytes(
            record_before
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
                "source": source,
                "excluded": expected["excluded"],
                "record_base": expected["record_base"],
                "objects": expected["objects"],
                "output": actual_output[:-1],
                "count": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                "direction_flag": direction_flag,
                "helper": expected["helper"],
                "return": "far",
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
        raise SystemExit(f"BBB navigation actor-target body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_navigation_actor_targets_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "helper_entry": f"0x{HELPER_ENTRY:04x}",
            "source_offset": SOURCE_OFFSET,
            "record_pointer_offset": RECORD_POINTER_OFFSET,
            "honk_offset": HONK_OFFSET,
            "radio_offset": RADIO_OFFSET,
            "output_offset": OUTPUT_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB navigation actor-target cases")


if __name__ == "__main__":
    main()
