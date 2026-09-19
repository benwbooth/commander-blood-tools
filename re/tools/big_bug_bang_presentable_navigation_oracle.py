#!/usr/bin/env python3
"""Verify BBB's location-only ship-HUD target-list builder."""

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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_presentable_navigation.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_7259_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "a04608c81cd768a6b512f2c570fee6299021590e45ec89eca62735dcf678d434"
)

ENTRY = 0x8270
END = 0x82C6
BODY_SHA256 = "19bb62979e3e34e07277a22423b2796cf145e1fdf8193988f91f36e263bd8c62"
HELPER_ENTRY = 0x685D
HELPER_RETURN = 0x8280
SOURCE_OFFSET = 0x6C2E
OUTPUT_OFFSET = 0x275D
ARCHE_OFFSET = 0x6B22
ARK_OFFSET = 0x6B28
COMMANDER_SOURCE_OFFSET = 0x6886
COMMANDER_OUTPUT_OFFSET = 0x250B
COMMANDER_ARCHE_OFFSET = 0x6752
LOCATION_KIND_BIT = 0x0080
IN_PLAY_BIT = 0x02
NAME_OFFSET = 4
TERMINATOR = 0xFFFF

INCOMING_ES_SEGMENT = 0x2000
DATA_SEGMENT = 0x3000
GAME_SEGMENT = 0x4000
DECOY_RECORD_SEGMENT = 0x5000
RECORD_SEGMENT = 0x7000
STACK_SEGMENT = 0xA000
FS_SEGMENT = 0xC000
SEGMENT_SIZE = 0x10000
MEMORY_SIZE = 0x100000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x1800
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("a5875a693cc37896")

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


CASES: tuple[dict[str, Any], ...] = (
    {
        "name": "target_location_accepted",
        "target": 0x0100,
        "source": [],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {0x0100: (0x0080, 0x02)},
    },
    {
        "name": "planet_target_rejected_location_child_accepted",
        "target": 0x0100,
        "source": [0x0200],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {0x0100: (0x0008, 0x02), 0x0200: (0x0080, 0x02)},
    },
    {
        "name": "only_location_kind_is_presentable",
        "target": 0x0100,
        "source": [0x0120, 0x0140, 0x0160, 0x0180],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0008, 0x02),
            0x0120: (0x0010, 0x02),
            0x0140: (0x0080, 0x02),
            0x0160: (0x0100, 0x02),
            0x0180: (0x0002, 0x02),
        },
    },
    {
        "name": "native_location_bit_accepts_composite_kinds",
        "target": 0x0100,
        "source": [0x0120, 0x0140],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0098, 0x02),
            0x0120: (0x0180, 0x02),
            0x0140: (0x0081, 0x02),
        },
        "typed_supported": False,
    },
    {
        "name": "in_play_bit_two_required",
        "target": 0x0100,
        "source": [0x0120, 0x0140, 0x0160],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0080, 0x00),
            0x0120: (0x0080, 0x01),
            0x0140: (0x0080, 0x02),
            0x0160: (0x0080, 0xFE),
        },
    },
    {
        "name": "exclude_arche_and_ark_after_record_tests",
        "target": 0x0100,
        "source": [0x0120, 0x0140, 0x0160],
        "arche": 0x0120,
        "ark": 0x0140,
        "objects": {
            0x0100: (0x0080, 0x02),
            0x0120: (0x0080, 0x02),
            0x0140: (0x0080, 0x02),
            0x0160: (0x0080, 0x02),
        },
    },
    {
        "name": "zero_offset_is_valid",
        "target": 0x0000,
        "source": [0x0006],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {0x0000: (0x0080, 0x02), 0x0006: (0x0008, 0x02)},
    },
    {
        "name": "target_and_name_offset_wrap",
        "target": 0xFFFE,
        "source": [0x8000],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {0xFFFE: (0x0080, 0x02), 0x8000: (0x0080, 0x02)},
    },
    {
        "name": "unsigned_source_fffe_is_not_sentinel",
        "target": 0x0100,
        "source": [0x8000, 0xFFFE],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0001, 0x02),
            0x8000: (0x0080, 0x02),
            0xFFFE: (0x0080, 0x02),
        },
    },
    {
        "name": "all_rejected",
        "target": 0x0100,
        "source": [0x0120, 0x0140, 0x0160],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0001, 0x00),
            0x0120: (0x0008, 0x02),
            0x0140: (0x0080, 0x00),
            0x0160: (0x0002, 0x02),
        },
    },
    {
        "name": "inherited_reverse_direction",
        "target": 0x0100,
        "source": [0x0120, 0x0140],
        "arche": 0x7777,
        "ark": 0x6666,
        "objects": {
            0x0100: (0x0080, 0x02),
            0x0120: (0x0080, 0x02),
            0x0140: (0x0080, 0x02),
        },
        "direction_flag": True,
    },
    {
        "name": "shared_arche_and_ark_exclusion",
        "target": 0x0100,
        "source": [0x0120, 0x0140],
        "arche": 0x0120,
        "ark": 0x0120,
        "objects": {
            0x0100: (0x0080, 0x02),
            0x0120: (0x0080, 0x02),
            0x0140: (0x0080, 0x02),
        },
    },
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(memory: bytearray, segment: int, offset: int, data: bytes) -> None:
    base = segment * 16
    for index, value in enumerate(data):
        memory[base + ((offset + index) & 0xFFFF)] = value


def read_wrapped(memory: bytes, segment: int, offset: int, size: int) -> bytes:
    base = segment * 16
    return bytes(memory[base + ((offset + index) & 0xFFFF)] for index in range(size))


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


def model_case(case: dict[str, Any]) -> list[int]:
    output = []
    for offset in [int(case["target"]), *map(int, case["source"])]:
        kind, flags = case["objects"][offset]
        if kind & LOCATION_KIND_BIT == 0 or flags & IN_PLAY_BIT == 0:
            continue
        if offset in (int(case["arche"]), int(case["ark"])):
            continue
        output.append((offset + NAME_OFFSET) & 0xFFFF)
    return output


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows = []
    for case_index, case in enumerate(CASES):
        name = str(case["name"])
        target = int(case["target"])
        source = [int(value) for value in case["source"]]
        arche = int(case["arche"])
        ark = int(case["ark"])
        output = model_case(case)
        direction_flag = bool(case.get("direction_flag", False))
        source_step = -2 if direction_flag else 2

        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62345 + case_index,
            "ecx": 0xC7C73456 + case_index,
            "edx": 0xD8D84567 + case_index,
            "esi": 0xE9E95678 + case_index,
            "edi": 0xFAFA0000 | target,
            "ebp": 0xABCD789A + case_index,
            "ds": DATA_SEGMENT,
            "es": RECORD_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        memory = bytearray(MEMORY_SIZE)
        memory[: len(executable)] = executable
        memory[HELPER_ENTRY] = 0xCB
        memory[RETURN_ADDRESS] = 0xCC
        for segment, multiplier, salt in (
            (INCOMING_ES_SEGMENT, 11, 0x13),
            (DATA_SEGMENT, 13, 0x25),
            (GAME_SEGMENT, 17, 0x37),
            (DECOY_RECORD_SEGMENT, 19, 0x49),
            (RECORD_SEGMENT, 23, 0x5B),
            (STACK_SEGMENT, 29, 0x6D),
            (FS_SEGMENT, 31, 0x7F),
        ):
            write_wrapped(
                memory, segment, 0, seeded_segment(case_index, multiplier, salt)
            )

        write_wrapped(memory, GAME_SEGMENT, ARCHE_OFFSET, struct.pack("<H", arche))
        write_wrapped(memory, GAME_SEGMENT, ARK_OFFSET, struct.pack("<H", ark))
        write_wrapped(
            memory, DATA_SEGMENT, ARCHE_OFFSET, struct.pack("<H", arche ^ 0xFFFF)
        )
        write_wrapped(memory, DATA_SEGMENT, ARK_OFFSET, struct.pack("<H", ark ^ 0xFFFF))
        write_wrapped(
            memory,
            GAME_SEGMENT,
            COMMANDER_ARCHE_OFFSET,
            struct.pack("<H", arche ^ 0x5A5A),
        )
        write_wrapped(
            memory, GAME_SEGMENT, COMMANDER_SOURCE_OFFSET, b"\x39\x17\x5b\x7d"
        )
        write_wrapped(
            memory, STACK_SEGMENT, COMMANDER_OUTPUT_OFFSET, b"\x68\x24\xac\xe0"
        )
        write_wrapped(memory, DATA_SEGMENT, SOURCE_OFFSET, b"\xa5\x5a\xc3\x3c")
        write_wrapped(memory, STACK_SEGMENT, OUTPUT_OFFSET, b"\x87\x69\x96\x78")
        write_wrapped(
            memory,
            STACK_SEGMENT,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )

        for object_offset, (kind, flags) in case["objects"].items():
            write_wrapped(
                memory,
                RECORD_SEGMENT,
                int(object_offset),
                struct.pack("<H", int(kind)),
            )
            write_wrapped(
                memory,
                RECORD_SEGMENT,
                int(object_offset) + 2,
                bytes([int(flags)]),
            )
            write_wrapped(
                memory,
                DECOY_RECORD_SEGMENT,
                int(object_offset),
                struct.pack("<HB", int(kind) ^ 0xFFFF, int(flags) ^ 0xFF),
            )

        source_words = [*source, TERMINATOR]
        for index, value in enumerate(source_words):
            write_wrapped(
                memory,
                GAME_SEGMENT,
                SOURCE_OFFSET + index * source_step,
                struct.pack("<H", value),
            )

        expected_events = []

        def event(kind: str, offset: int, value: int) -> None:
            expected_events.append(
                {
                    "kind": kind,
                    "offset": offset & 0xFFFF,
                    "size": 2,
                    "value": value & 0xFFFF,
                }
            )
            write_wrapped(
                memory,
                STACK_SEGMENT,
                offset,
                struct.pack("<H", value & 0xFFFF),
            )

        for index, value in enumerate(
            (
                initial["ds"],
                initial["esi"],
                initial["ebx"],
                initial["eax"],
                initial["edi"],
                0,
                HELPER_RETURN,
            ),
            start=1,
        ):
            event("stack", CALLER_SP - index * 2, value)
        for index, value in enumerate([*output, TERMINATOR]):
            event("output", OUTPUT_OFFSET + index * 2, value)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, MEMORY_SIZE)
        # Publish the complete original image before synthetic state/stub writes,
        # so the coverage runner can identify its file-offset mapping. It still
        # compares each executed instruction with the original bytes.
        machine.mem_write(0, executable)
        machine.mem_write(0, bytes(memory))
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

        actual_events = []
        helper_calls = []
        reached_return = []
        output_addresses = {
            STACK_SEGMENT * 16 + ((OUTPUT_OFFSET + index * 2) & 0xFFFF)
            for index in range(len(output) + 1)
        }

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
                        "ebx": cpu.reg_read(UC_X86_REG_EBX),
                        "esi": cpu.reg_read(UC_X86_REG_ESI),
                        "ebp": cpu.reg_read(UC_X86_REG_EBP),
                        "target": [
                            cpu.reg_read(UC_X86_REG_ES),
                            cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        ],
                        "ds": cpu.reg_read(UC_X86_REG_DS),
                        "gs": cpu.reg_read(UC_X86_REG_GS),
                        "ss": cpu.reg_read(UC_X86_REG_SS),
                        "return": list(
                            struct.unpack(
                                "<HH",
                                cpu.mem_read(STACK_SEGMENT * 16 + stack_pointer, 4),
                            )
                        ),
                    }
                )
                cpu.reg_write(
                    UC_X86_REG_EBP,
                    (cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF0000)
                    | ((SOURCE_OFFSET + len(source) * 2) & 0xFFFF),
                )
                return
            assert ENTRY <= address < END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
        ) -> None:
            assert size == 2, (hex(address), size)
            offset = (address - STACK_SEGMENT * 16) & 0xFFFF
            actual_events.append(
                {
                    "kind": "output" if address in output_addresses else "stack",
                    "offset": offset,
                    "size": size,
                    "value": value,
                }
            )

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=2_000)

        expected_registers = dict(initial)
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | (
            (OUTPUT_OFFSET + len(output) * 2) & 0xFFFF
        )
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
        expected_helper = {
            "eax": (initial["eax"] & 0xFFFF0000) | GAME_SEGMENT,
            "ebx": initial["ebx"],
            "esi": initial["esi"],
            "ebp": (initial["ebp"] & 0xFFFF0000) | SOURCE_OFFSET,
            "target": [RECORD_SEGMENT, target],
            "ds": GAME_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
            "return": [HELPER_RETURN, 0],
        }

        assert helper_calls == [expected_helper], name
        assert reached_return == [RETURN_ADDRESS], name
        assert actual_events == expected_events, name
        assert snapshot_registers(machine) == expected_registers, name
        assert snapshot_flags(machine) == expected_flags, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        actual_memory = bytes(machine.mem_read(0, MEMORY_SIZE))
        assert actual_memory == bytes(memory), name
        assert (
            read_wrapped(
                actual_memory, STACK_SEGMENT, CALLER_SP + 4, len(STACK_SENTINEL)
            )
            == STACK_SENTINEL
        )

        rows.append(
            {
                "name": name,
                "target": [RECORD_SEGMENT, target],
                "source": source,
                "arche": arche,
                "ark": ark,
                "objects": {
                    f"{int(offset):#06x}": [int(kind), int(flags)]
                    for offset, (kind, flags) in case["objects"].items()
                },
                "output_name_offsets": output,
                "terminator_bp": (OUTPUT_OFFSET + len(output) * 2) & 0xFFFF,
                "direction_flag": direction_flag,
                "typed_supported": bool(case.get("typed_supported", True)),
                "helper": expected_helper,
                "ordered_write_events": actual_events,
                "registers_after": expected_registers,
                "defined_flags": expected_flags,
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
        raise SystemExit(f"BBB presentable-navigation body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    commander_digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert commander_digest == COMMANDER_FIXTURE_SHA256, commander_digest
    rows = vectors(verify_executable(args.executable))
    result = {
        "format": "big_bug_bang_presentable_navigation_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "helper_entry": f"0x{HELPER_ENTRY:04x}",
            "source_offset": SOURCE_OFFSET,
            "output_offset": OUTPUT_OFFSET,
            "arche_offset": ARCHE_OFFSET,
            "ark_offset": ARK_OFFSET,
            "kind_bit": LOCATION_KIND_BIT,
            "in_play_bit": IN_PLAY_BIT,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB presentable-navigation cases")


if __name__ == "__main__":
    main()
