#!/usr/bin/env python3
"""Verify BBB's relocated selected-choice mask overlay."""

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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_selected_mask.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_7cb4_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "534431e99e4174877eb397c74f9ad58ef114eab4b7b4c7f8a13bf981e58a01bf"
)

ENTRY = 0x8D88
END = 0x8DBC
BODY_SHA256 = "2b9405cd31538431ff1e93e788074382e098f9b6e923c08d32a0759c76e1e926"
MASK_TABLE_OFFSET = 0x85DE
SELECTOR_OFFSET = 0x2A83
FRAMEBUFFER_POINTER_OFFSET = 0x55F1
AUTHORED_MASK_FILE_OFFSET = 0x17DCE
MASK_COUNT = 6
MASK_HEIGHT = 16
MASK_WIDTH = 16
MASK_BYTE_COUNT = MASK_COUNT * MASK_HEIGHT * 2
AUTHORED_MASK_SHA256 = (
    "490b2c73dd13da829570859683520e1f92979cfd9936782dc0ea33f23d25a324"
)
FRAMEBUFFER_ORIGIN = 0x12C5
FRAMEBUFFER_STRIDE = 320
MASK_COLOR = 0xFE

DATA_SEGMENT = 0x3000
FRAMEBUFFER_SEGMENT = 0x5000
INCOMING_ES_SEGMENT = 0x6000
GAME_DECOY_SEGMENT = 0x7000
FS_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
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
EXPECTED_FINAL_FLAGS = {
    "cf": False,
    "pf": True,
    "af": False,
    "zf": False,
    "sf": False,
    "of": False,
}
PATTERNS = (
    (0x0000,) * MASK_HEIGHT,
    tuple(0x8000 >> row for row in range(MASK_HEIGHT)),
    (0xFFFF,) * MASK_HEIGHT,
    (0xAAAA, 0x5555) * 8,
    tuple(1 << (15 - row) for row in range(MASK_HEIGHT)),
    (
        0x8001,
        0x4002,
        0x2004,
        0x1008,
        0x0810,
        0x0420,
        0x0240,
        0x0180,
    )
    * 2,
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


def snapshot_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def encoded_patterns() -> bytes:
    return b"".join(struct.pack(">H", row) for rows in PATTERNS for row in rows)


def changed_offsets(rows: tuple[int, ...]) -> list[int]:
    return [
        FRAMEBUFFER_ORIGIN + row * FRAMEBUFFER_STRIDE + column
        for row, bits in enumerate(rows)
        for column in range(MASK_WIDTH)
        if bits & (0x8000 >> column)
    ]


def vectors(executable: bytes) -> list[dict[str, Any]]:
    table = encoded_patterns()
    rows: list[dict[str, Any]] = []
    for index, pattern in enumerate(PATTERNS):
        pointer_offset = 0 if index % 2 == 0 else 0x4567
        offsets = changed_offsets(pattern)
        initial = {
            "eax": 0xA1A11234 + index,
            "ebx": 0xB2B22345 + index,
            "ecx": 0xC3C33456 + index,
            "edx": 0xD4D44567 + index,
            "esi": 0xE5E55678 + index,
            "edi": 0xF6F66789 + index,
            "ebp": 0xA7A7789A + index,
            "ds": DATA_SEGMENT,
            "es": INCOMING_ES_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        data_before = seeded_segment(index, 17, 0x21)
        data_before[MASK_TABLE_OFFSET : MASK_TABLE_OFFSET + len(table)] = table
        data_before[SELECTOR_OFFSET] = index
        struct.pack_into(
            "<HH",
            data_before,
            FRAMEBUFFER_POINTER_OFFSET,
            pointer_offset,
            FRAMEBUFFER_SEGMENT,
        )
        framebuffer_before = seeded_segment(index, 19, 0x31)
        framebuffer_expected = bytearray(framebuffer_before)
        for offset in offsets:
            framebuffer_expected[offset] = MASK_COLOR

        incoming_es_before = bytes(seeded_segment(index, 23, 0x43))
        game_decoy_before = seeded_segment(index, 29, 0x59)
        game_decoy_before[MASK_TABLE_OFFSET : MASK_TABLE_OFFSET + len(table)] = bytes(
            byte ^ 0xA5 for byte in table
        )
        game_decoy_before[SELECTOR_OFFSET] = index ^ 0xA5
        struct.pack_into(
            "<HH",
            game_decoy_before,
            FRAMEBUFFER_POINTER_OFFSET,
            pointer_offset ^ 0xA5A5,
            INCOMING_ES_SEGMENT,
        )
        fs_before = bytes(seeded_segment(index, 31, 0x67))
        stack_before = seeded_segment(index, 37, 0x71)
        struct.pack_into("<H", stack_before, CALLER_SP, RETURN_OFFSET)
        stack_before[CALLER_SP + 2 : CALLER_SP + 2 + len(STACK_SENTINEL)] = (
            STACK_SENTINEL
        )
        stack_expected = bytearray(stack_before)
        struct.pack_into("<H", stack_expected, CALLER_SP - 2, initial["es"])
        struct.pack_into("<H", stack_expected, CALLER_SP - 4, initial["edi"] & 0xFFFF)
        final_row_start = FRAMEBUFFER_ORIGIN + (MASK_HEIGHT - 1) * FRAMEBUFFER_STRIDE
        struct.pack_into("<H", stack_expected, CALLER_SP - 6, final_row_start)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(FRAMEBUFFER_SEGMENT * 16, bytes(framebuffer_before))
        machine.mem_write(INCOMING_ES_SEGMENT * 16, incoming_es_before)
        machine.mem_write(GAME_DECOY_SEGMENT * 16, bytes(game_decoy_before))
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0A93)

        reached_return: list[int] = []
        allowed_addresses = {
            FRAMEBUFFER_SEGMENT * 16 + offset for offset in offsets
        } | set(
            range(STACK_SEGMENT * 16 + CALLER_SP - 6, STACK_SEGMENT * 16 + CALLER_SP)
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
        machine.emu_start(ENTRY, 0, count=2_000)

        expected_registers = dict(initial)
        expected_registers["eax"] &= 0xFFFF0000
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["edx"] = (initial["edx"] & 0xFFFFFF00) | MASK_COLOR
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | (
            MASK_TABLE_OFFSET + (index + 1) * MASK_HEIGHT * 2
        )
        assert reached_return == [RETURN_ADDRESS], index
        assert snapshot_registers(machine) == expected_registers, index
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert snapshot_flags(machine) == EXPECTED_FINAL_FLAGS
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert bytes(machine.mem_read(FRAMEBUFFER_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            framebuffer_expected
        )
        assert (
            bytes(machine.mem_read(INCOMING_ES_SEGMENT * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(GAME_DECOY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_decoy_before
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "index": index,
                "input_pointer_offset_ignored": pointer_offset,
                "big_endian_rows": list(pattern),
                "color": MASK_COLOR,
                "changed_offsets": offsets,
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
        raise SystemExit(f"BBB selected-mask body changed: {body_digest}")
    authored_masks = executable[
        AUTHORED_MASK_FILE_OFFSET : AUTHORED_MASK_FILE_OFFSET + MASK_BYTE_COUNT
    ]
    authored_digest = hashlib.sha256(authored_masks).hexdigest()
    if authored_digest != AUTHORED_MASK_SHA256:
        raise SystemExit(f"BBB authored choice masks changed: {authored_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    verify_commander_semantics(rows)
    result = {
        "format": "big_bug_bang_selected_mask_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "mask_table_offset": MASK_TABLE_OFFSET,
            "selector_offset": SELECTOR_OFFSET,
            "framebuffer_pointer_offset": FRAMEBUFFER_POINTER_OFFSET,
        },
        "authored_masks": {
            "file_offset": f"0x{AUTHORED_MASK_FILE_OFFSET:05x}",
            "byte_count": MASK_BYTE_COUNT,
            "sha256": AUTHORED_MASK_SHA256,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB selected-mask cases")


if __name__ == "__main__":
    main()
