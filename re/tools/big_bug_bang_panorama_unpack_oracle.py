#!/usr/bin/env python3
"""Verify BBB's relocated bridge-panorama ByteRun unpacker."""

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
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
from unicorn.x86_const import (
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_panorama_unpack.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_2d50_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "79587889a19f54f15804f164f97481a58fe6b9821ab4133b7206191f22e4b405"
)

ENTRY = 0x30D6
END = 0x3144
BODY_SHA256 = "46ff14c5c95b663dd39f30ea5e8a3541d4e3139770997be6493039110eb865b4"
DISPLAY_POINTER_OFFSET = 0x55F9
TRANSPARENT_FLAG_OFFSET = 0x5F27
COMMANDER_DISPLAY_POINTER_OFFSET = 0x5229
COMMANDER_TRANSPARENT_FLAG_OFFSET = 0x5B57
FRAME_PIXEL_COUNT = 64_000

SOURCE_SEGMENT = 0x2000
INCOMING_ES_SEGMENT = 0x4000
INCOMING_FS_SEGMENT = 0x5000
GAME_SEGMENT = 0x7000
OUTPUT_SEGMENT = 0x9000
STACK_SEGMENT = 0xB000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x6F80
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("4bb42dd21ee10ff0")
DEFINED_FLAG_MASK = 0x08C5

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
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


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


def byte_run_stream(transparent: bool) -> bytes:
    stream = bytearray()
    pixels = 0

    def repeat(count: int, value: int) -> None:
        nonlocal pixels
        stream.extend(((1 - count) & 0xFF, value))
        pixels += count

    def literal(values: bytes) -> None:
        nonlocal pixels
        stream.append(len(values) - 1)
        stream.extend(values)
        pixels += len(values)

    if transparent:
        repeat(5, 0)
        literal(bytes((1, 0, 2, 0, 3)))
        repeat(4, 7)
        for run in range(496):
            repeat(129, 0 if run & 1 == 0 else (run * 13 + 17) & 0xFF)
        repeat(2, 9)
    else:
        literal(bytes((1, 2, 0, 4)))
        repeat(2, 0)
        for run in range(496):
            repeat(129, (run * 13 + 17) & 0xFF)
        repeat(10, 0xA7)

    assert pixels == FRAME_PIXEL_COUNT
    return bytes(stream)


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        transparent = expected["transparent_zero"]
        direction_flag = expected["direction_flag"]
        direction = -1 if direction_flag else 1
        source_offset = expected["source_offset"]
        output_offset = expected["output_offset"]
        stream = byte_run_stream(transparent)
        assert len(stream) == expected["source_bytes"], name

        source_before = seeded_segment(case_index, 19, 7)
        for index, value in enumerate(stream):
            source_before[(source_offset + direction * index) & 0xFFFF] = value
        incoming_es_before = seeded_segment(case_index, 13, 0x25)
        incoming_fs_before = seeded_segment(case_index, 17, 0x37)
        game_before = seeded_segment(case_index, 29, 0x49)
        output_before = bytes(
            (index * 23 + case_index * 41 + 0x5D) & 0xFF
            for index in range(SEGMENT_SIZE)
        )
        output_expected = bytearray(output_before)

        real_pointer = struct.pack("<HH", output_offset, OUTPUT_SEGMENT)
        wrong_pointer = struct.pack("<HH", output_offset ^ 0xA5A5, INCOMING_ES_SEGMENT)
        for decoy in (source_before, incoming_es_before, incoming_fs_before):
            write_wrapped(decoy, DISPLAY_POINTER_OFFSET, wrong_pointer)
            write_wrapped(decoy, COMMANDER_DISPLAY_POINTER_OFFSET, wrong_pointer)
            decoy[TRANSPARENT_FLAG_OFFSET] = 0 if transparent else 1
            decoy[COMMANDER_TRANSPARENT_FLAG_OFFSET] = 0 if transparent else 1
        write_wrapped(game_before, DISPLAY_POINTER_OFFSET, real_pointer)
        game_before[TRANSPARENT_FLAG_OFFSET] = 1 if transparent else 0
        write_wrapped(game_before, COMMANDER_DISPLAY_POINTER_OFFSET, wrong_pointer)
        game_before[COMMANDER_TRANSPARENT_FLAG_OFFSET] = 0 if transparent else 1

        initial = {
            "eax": 0xA1A11230 + case_index,
            "ebx": 0xB2B22340 + case_index,
            "ecx": 0xC3C33450 + case_index,
            "edx": 0xD4D44560 + case_index,
            "esi": 0xE5E50000 | source_offset,
            "edi": 0xF6F67890 + case_index,
            "ebp": 0x979789A0 + case_index,
            "ds": SOURCE_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": INCOMING_FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        stack_before = seeded_segment(case_index, 41, 0x73)
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<HH", RETURN_OFFSET, 0) + STACK_SENTINEL,
        )

        stream_cursor = 0
        destination = output_offset
        output_remaining = FRAME_PIXEL_COUNT
        return_al = 0
        expected_write_events: list[tuple[int, int, int]] = []
        while output_remaining != 0:
            control_byte = stream[stream_cursor]
            stream_cursor += 1
            control = control_byte - 0x100 if control_byte >= 0x80 else control_byte
            if control < 0:
                count = -control + 1
                value = stream[stream_cursor]
                stream_cursor += 1
                return_al = value
                output_remaining -= count
                if transparent and value == 0:
                    destination = (destination + count) & 0xFFFF
                    continue
                for _ in range(count):
                    output_expected[destination] = value
                    expected_write_events.append(
                        (physical_address(OUTPUT_SEGMENT, destination), 1, value)
                    )
                    destination = (destination + direction) & 0xFFFF
                continue

            count = control + 1
            return_al = count
            output_remaining -= count
            for _ in range(count):
                value = stream[stream_cursor]
                stream_cursor += 1
                if transparent:
                    return_al = value
                if not transparent or value != 0:
                    output_expected[destination] = value
                    expected_write_events.append(
                        (physical_address(OUTPUT_SEGMENT, destination), 1, value)
                    )
                destination = (destination + (1 if transparent else direction)) & 0xFFFF

        assert stream_cursor == len(stream), name
        source_end = (source_offset + direction * len(stream)) & 0xFFFF
        assert source_end == expected["source_end_offset"], name
        assert destination == expected["output_end_offset"], name

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(SOURCE_SEGMENT * 16, bytes(source_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, bytes(incoming_es_before))
        machine.mem_write(INCOMING_FS_SEGMENT * 16, bytes(incoming_fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(OUTPUT_SEGMENT * 16, output_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
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

        reached_return: list[int] = []
        observed_write_events: list[tuple[int, int, int]] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls: list[int] = reached_return,
        ) -> None:
            if address == RETURN_ADDRESS:
                calls.append(address)
                cpu.emu_stop()
                return
            assert ENTRY <= address < END, hex(address)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            events: list[tuple[int, int, int]] = observed_write_events,
        ) -> None:
            events.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=300_000)

        actual_output = bytes(machine.mem_read(OUTPUT_SEGMENT * 16, SEGMENT_SIZE))
        expected_registers = dict(initial)
        expected_registers.update(
            {
                "eax": return_al,
                "ecx": initial["ecx"] & 0xFFFF0000,
                "esi": (initial["esi"] & 0xFFFF0000) | source_end,
                "edi": (initial["edi"] & 0xFFFF0000) | destination,
                "ebp": 0,
                "es": OUTPUT_SEGMENT,
            }
        )

        assert reached_return == [RETURN_ADDRESS], name
        assert actual_output == bytes(output_expected), name
        assert observed_write_events == expected_write_events, name
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_EAX) == expected["return_eax"], name
        assert (
            hashlib.sha256(actual_output).hexdigest()
            == expected["output_segment_sha256"]
        ), name
        assert machine.reg_read(UC_X86_REG_EFLAGS) & DEFINED_FLAG_MASK == 0x0044, name
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0200), name
        assert bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x0400) == direction_flag, (
            name
        )
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 4
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(SOURCE_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            source_before
        )
        assert bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            incoming_es_before
        )
        assert bytes(machine.mem_read(INCOMING_FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            incoming_fs_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_before
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

        rows.append(dict(expected))
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB panorama-unpack body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_panorama_unpack_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "display_pointer_offset": DISPLAY_POINTER_OFFSET,
            "transparent_flag_offset": TRANSPARENT_FLAG_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB panorama-unpack cases")


if __name__ == "__main__":
    main()
