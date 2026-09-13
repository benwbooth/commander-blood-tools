#!/usr/bin/env python3
"""Verify BBB's relocated resource-palette file-block parser."""

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
    UC_HOOK_INTR,
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
    ROOT / "re/tools/oracle_vectors/big_bug_bang_resource_palette_file_blocks.json"
)
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_4086_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "33f1fadd9bc18a330623c4d6c61c33aab2c2fb8393bbe6c099e661c8d156ed58"
)

ENTRY = 0x4503
END = 0x454D
BODY_SHA256 = "7f6653c74fb30079bf3ed87ca6d63f7c122e2573f85928f0716d407261494bfc"
PALETTE_OFFSET = 0x5621
DIRTY_OFFSET = 0x5F25
PALETTE_STORAGE_BYTE_COUNT = 0x900
HEADER_OFFSET = 0x0AF2

DATA_SEGMENT = 0x2000
EXTRA_SEGMENT = 0x4000
FS_SEGMENT = 0x6000
DECOY_SEGMENT = 0x8000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x6F00
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

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


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytes:
    return bytes(
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def palette_image(case_index: int) -> bytes:
    return bytes(
        (index * 17 + case_index * 29 + 0x43) & 0xFF
        for index in range(PALETTE_STORAGE_BYTE_COUNT)
    )


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        blocks = expected["blocks"]
        handle = 0x3100 + case_index

        source = bytearray()
        expected_reads: list[dict[str, Any]] = []
        expected_palette = bytearray(palette_image(case_index))
        source_cursor = 0
        remaining = expected["remaining_before"]
        for block_index, block in enumerate(blocks):
            start = block["start"]
            count = block["count"]
            payload_size = count * 3
            source.extend(struct.pack("<H", start | (count << 8)))
            expected_reads.append(
                {
                    "kind": "header",
                    "count": 2,
                    "destination": HEADER_OFFSET,
                    "source_offset": source_cursor,
                    "remaining": remaining & 0xFFFFFFFF,
                }
            )
            source_cursor += 2
            remaining = (remaining - 2) & 0xFFFFFFFF
            payload = bytes(
                (index * 31 + block_index * 47 + case_index * 53 + 5) & 0xFF
                for index in range(payload_size)
            )
            source.extend(payload)
            destination = PALETTE_OFFSET + start * 3
            expected_palette[start * 3 : start * 3 + payload_size] = payload
            remaining = (remaining - payload_size) & 0xFFFFFFFF
            expected_reads.append(
                {
                    "kind": "payload",
                    "count": payload_size,
                    "destination": destination,
                    "source_offset": source_cursor,
                    "remaining": remaining,
                }
            )
            source_cursor += payload_size
        source.extend(b"\xff\xff")
        expected_reads.append(
            {
                "kind": "header",
                "count": 2,
                "destination": HEADER_OFFSET,
                "source_offset": source_cursor,
                "remaining": remaining,
            }
        )
        remaining = (remaining - 2) & 0xFFFFFFFF
        assert remaining == expected["remaining_after"], name

        data_before = bytearray(seeded_segment(case_index, 19, 0x37))
        write_wrapped(data_before, PALETTE_OFFSET, palette_image(case_index))
        data_before[DIRTY_OFFSET] = 0xA5
        write_wrapped(data_before, HEADER_OFFSET, b"\x12\x34")
        data_expected = bytearray(data_before)
        data_expected[DIRTY_OFFSET] = 1
        write_wrapped(data_expected, HEADER_OFFSET, b"\xff\xff")
        write_wrapped(data_expected, PALETTE_OFFSET, bytes(expected_palette))

        extra_before = seeded_segment(case_index, 23, 0x49)
        fs_before = seeded_segment(case_index, 29, 0x5B)
        decoy_before = bytearray(seeded_segment(case_index, 31, 0x6D))
        write_wrapped(
            decoy_before,
            PALETTE_OFFSET,
            bytes(value ^ 0xFF for value in palette_image(case_index)),
        )
        decoy_before[DIRTY_OFFSET] = 0x5A
        write_wrapped(decoy_before, HEADER_OFFSET, b"\x87\x96")

        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D40000 | HEADER_OFFSET,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | handle,
            "ebp": expected["remaining_before"],
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": DECOY_SEGMENT,
            "ss": STACK_SEGMENT,
        }
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)

        stack_before = bytearray(seeded_segment(case_index, 37, 0x7F))
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<H", RETURN_OFFSET) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        for offset, value, size in (
            (CALLER_SP - 4, initial["eax"], 4),
            (CALLER_SP - 6, initial["ebx"], 2),
            (CALLER_SP - 8, initial["ecx"], 2),
            (CALLER_SP - 10, initial["edx"], 2),
            (CALLER_SP - 12, initial["esi"], 2),
        ):
            write_wrapped(
                stack_expected,
                offset,
                (value & ((1 << (size * 8)) - 1)).to_bytes(size, "little"),
            )
        if blocks:
            write_wrapped(
                stack_expected,
                CALLER_SP - 14,
                struct.pack("<H", HEADER_OFFSET),
            )

        allowed_cpu_writes = physical_addresses(DATA_SEGMENT, DIRTY_OFFSET, 1)
        allowed_cpu_writes |= physical_addresses(STACK_SEGMENT, CALLER_SP - 12, 12)
        if blocks:
            allowed_cpu_writes |= physical_addresses(STACK_SEGMENT, CALLER_SP - 14, 2)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, extra_before)
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(DECOY_SEGMENT * 16, bytes(decoy_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, initial_flags)

        reads: list[dict[str, Any]] = []
        source_position = [0]
        reached_return: list[int] = []
        observed_cpu_writes: set[int] = set()
        observed_dos_writes: set[int] = set()

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

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _context: object,
            allowed: set[int] = allowed_cpu_writes,
            observed: set[int] = observed_cpu_writes,
            case_name: str = name,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= allowed, (case_name, hex(address), size)
            observed.update(addresses)

        def interrupt(
            cpu: Uc,
            number: int,
            _context: object,
            case_name: str = name,
            reads_for_case: list[dict[str, Any]] = reads,
            expected_for_case: list[dict[str, Any]] = expected_reads,
            source_bytes: bytes = bytes(source),
            position: list[int] = source_position,
            expected_handle: int = handle,
            dos_error: bool = expected["dos_error_ignored"],
            dos_writes: set[int] = observed_dos_writes,
        ) -> None:
            assert number == 0x21, (case_name, hex(number))
            assert cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF == 0x3F00, case_name
            assert len(reads_for_case) < len(expected_for_case), case_name
            expected_read = expected_for_case[len(reads_for_case)]
            destination = cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF
            count = cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF
            read = {
                "kind": "header" if destination == HEADER_OFFSET else "payload",
                "count": count,
                "destination": destination,
                "source_offset": position[0],
                "remaining": cpu.reg_read(UC_X86_REG_EBP),
            }
            assert read == expected_read, (case_name, read, expected_read)
            assert cpu.reg_read(UC_X86_REG_EBX) & 0xFFFF == expected_handle, case_name
            assert cpu.reg_read(UC_X86_REG_DS) == DATA_SEGMENT, case_name
            assert cpu.mem_read(DATA_SEGMENT * 16 + DIRTY_OFFSET, 1)[0] == 1, case_name

            chunk = source_bytes[position[0] : position[0] + count]
            assert len(chunk) == count, (case_name, len(chunk), count)
            if chunk:
                cpu.mem_write(DATA_SEGMENT * 16 + destination, chunk)
                dos_writes.update(physical_addresses(DATA_SEGMENT, destination, count))
            position[0] += count
            cpu.reg_write(
                UC_X86_REG_EAX,
                (cpu.reg_read(UC_X86_REG_EAX) & 0xFFFF0000)
                | (1 if dos_error else count),
            )
            flags = cpu.reg_read(UC_X86_REG_EFLAGS)
            cpu.reg_write(
                UC_X86_REG_EFLAGS,
                (flags | 1) if dos_error else (flags & ~1),
            )
            reads_for_case.append(read)

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INTR, interrupt)
        machine.emu_start(ENTRY, 0, count=2_000)

        expected_dos_writes: set[int] = set()
        for read in expected_reads:
            expected_dos_writes |= physical_addresses(
                DATA_SEGMENT, read["destination"], read["count"]
            )
        expected_registers = dict(initial)
        expected_registers["ebp"] = expected["remaining_after"]
        actual_flags = snapshot_flags(machine)
        assert reached_return == [RETURN_ADDRESS], name
        assert reads == expected_reads, name
        assert source_position == [len(source)], name
        assert observed_cpu_writes == allowed_cpu_writes, name
        assert observed_dos_writes == expected_dos_writes, name
        assert snapshot_registers(machine) == expected_registers, name
        assert actual_flags == expected["defined_flags"], name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        assert machine.reg_read(UC_X86_REG_EFLAGS) & 0x0200 == 0x0200
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(DECOY_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            decoy_before
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        actual_palette = bytes(
            machine.mem_read(
                DATA_SEGMENT * 16 + PALETTE_OFFSET,
                PALETTE_STORAGE_BYTE_COUNT,
            )
        )
        assert hashlib.sha256(actual_palette).hexdigest() == expected["palette_sha256"]
        rows.append(
            {
                "name": name,
                "blocks": blocks,
                "dos_read_count": len(reads),
                "payload_bytes": expected["payload_bytes"],
                "remaining_before": expected["remaining_before"],
                "remaining_after": expected["remaining_after"],
                "dos_error_ignored": expected["dos_error_ignored"],
                "palette_sha256": expected["palette_sha256"],
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
        raise SystemExit(f"BBB resource-palette parser body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_resource_palette_file_blocks_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_semantic_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "palette_offset": PALETTE_OFFSET,
            "dirty_offset": DIRTY_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB resource-palette file-block cases")


if __name__ == "__main__":
    main()
