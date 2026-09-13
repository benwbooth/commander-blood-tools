#!/usr/bin/env python3
"""Verify BBB's presentation AD decoder against the Commander grammar vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct
from typing import Any

from unicorn import Uc, UcError, UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_EAX,
    UC_X86_REG_EBP,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDI,
    UC_X86_REG_EDX,
    UC_X86_REG_EIP,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_ESI,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
COMMANDER_FIXTURE = REPO_ROOT / "re/tools/oracle_vectors/func_a914_natural.json"
COMMANDER_FIXTURE_SHA256 = (
    "5549f370e39004b8f042dd173a752efa00a89c73e3565335540166c250cc87aa"
)
EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
ENTRY = 0xC0FE
ROUTINE_END = 0xC2A4
ROUTINE_SHA256 = (
    "3a37675c4b2da2f23fcf453b39a236a9278d700f0932bd8a08aca675976b9d70"
)
HELPER_ENTRY = 0xC2A4
HELPER_END = 0xC30D
HELPER_SHA256 = (
    "6f9aba91cf84930a552caddcbe3511006f7b3c6cca5ee86f53b57935f6816775"
)
HELPER_RETURN = 0xC127
CODE_BIAS_ALIASES = (0x0E25, 0x0E55)
HELPER_BIAS_OPERANDS = (0xC2D5, 0xC305)
SOURCE_SEGMENT = 0x2000
DESTINATION_SEGMENT = 0x3800
STACK_SEGMENT = 0x7000
RETURN_ADDRESS = 0xF000
STACK_POINTER = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DEFINED_FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}
REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_wrapped(memory: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        memory[(offset + index) & 0xFFFF] = value


def read_wrapped(memory: bytes | bytearray, offset: int, length: int) -> bytes:
    return bytes(memory[(offset + index) & 0xFFFF] for index in range(length))


def build_resource(vector: dict[str, Any]) -> bytes:
    extent = int(vector["declared_output_extent"])
    staged_count = len(vector["staged_values"])
    flags = int(vector["flags"])
    checksum = (
        0xAD - sum(struct.pack("<HHB", extent, staged_count, flags))
    ) & 0xFF
    return b"".join(
        (
            struct.pack("<HHBB", extent, staged_count, flags, checksum),
            bytes.fromhex(vector["prefix_hex"]),
            bytes.fromhex(vector["staging_stream_hex"]),
            bytes.fromhex(vector["main_stream_hex"]),
        )
    )


def patterned_memory(
    case_index: int,
    step: int,
    page_step: int,
    case_step: int,
    base: int = 0,
) -> bytearray:
    return bytearray(
        (
            offset * step
            + (offset >> 8) * page_step
            + case_index * case_step
            + base
        )
        & 0xFF
        for offset in range(0x10000)
    )


def execute_case(
    executable: bytes, vector: dict[str, Any], case_index: int
) -> dict[str, Any]:
    name = str(vector["name"])
    flags = int(vector["flags"])
    literal_bias = int(vector["literal_bias"])
    source_offset = int(vector["source_offset"])
    destination_offset = int(vector["destination_offset"])
    output_offset = int(vector["output_offset"])
    output_extent = int(vector["declared_output_extent"])
    output_end = (output_offset + output_extent) & 0xFFFF
    staged_offset = int(vector["staged_offset"])
    staged_values = [int(value) for value in vector["staged_values"]]
    helper_values = staged_values or [0x31 + literal_bias]

    resource = build_resource(vector)
    source_before = patterned_memory(case_index, 17, 7, 29)
    write_wrapped(source_before, source_offset, resource)

    destination_before = patterned_memory(case_index, 23, 11, 31)
    destination_expected = destination_before[:]
    prefix = bytes.fromhex(vector["prefix_hex"])
    write_wrapped(destination_expected, destination_offset, prefix)
    write_wrapped(destination_expected, staged_offset, bytes(helper_values))
    write_wrapped(
        destination_expected,
        output_offset,
        bytes.fromhex(vector["decoded_hex"]),
    )

    stack_before = patterned_memory(case_index, 41, 0, 17, 0x43)
    stack_before[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<H", RETURN_ADDRESS) + STACK_SENTINEL
    )
    initial = {
        "eax": 0xA1A11234 + case_index,
        "ebx": 0xB2B22345 + case_index,
        "ecx": 0xC3C33456 + case_index,
        "edx": 0xD4D44567 + case_index,
        "esi": 0xE5E50000 | source_offset,
        "edi": 0xF6F60000 | destination_offset,
        "ebp": 0x97972468 + case_index,
        "sp": STACK_POINTER,
        "ds": SOURCE_SEGMENT,
        "es": DESTINATION_SEGMENT,
        "fs": 0x1800,
        "gs": 0x5000,
        "ss": STACK_SEGMENT,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    machine.mem_write(CODE_BIAS_ALIASES[0], b"\x5a")
    machine.mem_write(CODE_BIAS_ALIASES[1], b"\xa5")
    machine.mem_write(SOURCE_SEGMENT * 16, bytes(source_before))
    machine.mem_write(DESTINATION_SEGMENT * 16, bytes(destination_before))
    machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0202)
    for register, value in initial.items():
        machine.reg_write(REGISTERS[register], value)

    helper_calls: list[dict[str, int]] = []
    reached_return: list[int] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        if address == RETURN_ADDRESS:
            reached_return.append(address)
            cpu.emu_stop()
            return
        if not ENTRY <= address < HELPER_END:
            raise AssertionError(f"{name}: escaped decoder at {address:#x}")
        if address != HELPER_ENTRY:
            return
        if helper_calls:
            raise AssertionError(f"{name}: duplicate helper call")

        actual = {
            "bx": cpu.reg_read(UC_X86_REG_BX),
            "si": cpu.reg_read(UC_X86_REG_SI),
            "di": cpu.reg_read(UC_X86_REG_DI),
            "bp": cpu.reg_read(UC_X86_REG_BP),
            "sp": cpu.reg_read(UC_X86_REG_SP),
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "es": cpu.reg_read(UC_X86_REG_ES),
        }
        helper_source_offset = (
            source_offset + 6 + len(prefix)
        ) & 0xFFFF
        expected = {
            "bx": source_offset,
            "si": helper_source_offset,
            "di": staged_offset,
            "bp": output_end,
            "sp": 0xFEF8,
            "ds": SOURCE_SEGMENT,
            "es": DESTINATION_SEGMENT,
        }
        if actual != expected:
            raise AssertionError(
                f"{name}: helper state={actual}, expected={expected}"
            )

        saved_ax = (initial["eax"] & 0xFF00) | flags
        frame = struct.unpack(
            "<5H", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF8, 10)
        )
        expected_frame = (
            HELPER_RETURN,
            staged_offset,
            output_offset,
            saved_ax,
            RETURN_ADDRESS,
        )
        if frame != expected_frame:
            raise AssertionError(
                f"{name}: helper frame={frame}, expected={expected_frame}"
            )

        aliases = bytes(cpu.mem_read(CODE_BIAS_ALIASES[0], 1)) + bytes(
            cpu.mem_read(CODE_BIAS_ALIASES[1], 1)
        )
        if aliases != bytes((literal_bias, literal_bias)):
            raise AssertionError(f"{name}: literal bias stored late")

        # In the loaded DOS image, these CS-relative aliases address the two
        # helper immediates. Mirror that segment relationship in the flat image.
        for operand in HELPER_BIAS_OPERANDS:
            cpu.mem_write(operand, bytes((literal_bias,)))
        helper_calls.append(actual)

    machine.hook_add(UC_HOOK_CODE, instruction)
    try:
        machine.emu_start(ENTRY, 0, count=100000)
    except UcError as error:
        raise RuntimeError(
            f"{name}: execution failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_EIP):#x}"
        ) from error

    if reached_return != [RETURN_ADDRESS]:
        raise AssertionError(f"{name}: did not return exactly once")
    if len(helper_calls) != 1:
        raise AssertionError(f"{name}: helper was not called exactly once")
    if bytes(machine.mem_read(SOURCE_SEGMENT * 16, 0x10000)) != bytes(
        source_before
    ):
        raise AssertionError(f"{name}: source memory changed")
    if bytes(machine.mem_read(DESTINATION_SEGMENT * 16, 0x10000)) != bytes(
        destination_expected
    ):
        raise AssertionError(f"{name}: destination memory differs")
    if bytes(
        machine.mem_read(STACK_SEGMENT * 16 + STACK_POINTER + 2, len(STACK_SENTINEL))
    ) != STACK_SENTINEL:
        raise AssertionError(f"{name}: stack sentinel changed")

    produced = int(vector["actual_output_extent"])
    terminal_offset = (int(vector["staged_result_offset"]) - 1) & 0xFFFF
    terminal_value = destination_expected[terminal_offset]
    expected_registers = dict(initial)
    expected_registers["eax"] = (
        initial["eax"] & 0xFFFF0000
    ) | terminal_value | (terminal_value << 8)
    expected_registers["ebx"] = (
        initial["ebx"] & 0xFFFF0000
    ) | int(vector["main_source_result_offset"])
    expected_registers["ecx"] = (
        initial["ecx"] & 0xFFFF0000
    ) | int(vector["pending_length"])
    expected_registers["edx"] = (
        initial["edx"] & 0xFFFF0000
    ) | int(vector["result_bit_buffer"])
    expected_registers["esi"] = (
        initial["esi"] & 0xFFFF0000
    ) | int(vector["staged_result_offset"])
    expected_registers["edi"] = (
        initial["edi"] & 0xFFFF0000
    ) | ((output_offset + produced) & 0xFFFF)
    expected_registers["ebp"] = (
        initial["ebp"] & 0xFFFF0000
    ) | output_end
    expected_registers["sp"] = STACK_POINTER + 2
    for register, expected in expected_registers.items():
        actual = machine.reg_read(REGISTERS[register])
        if actual != expected:
            raise AssertionError(
                f"{name}: {register}={actual:#x}, expected={expected:#x}"
            )
    if machine.reg_read(UC_X86_REG_CS) != 0:
        raise AssertionError(f"{name}: near return changed CS")

    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        flag: bool(flags_after & mask)
        for flag, mask in DEFINED_FLAG_MASKS.items()
    }
    if actual_flags != vector["defined_flags"]:
        raise AssertionError(
            f"{name}: flags={actual_flags}, expected={vector['defined_flags']}"
        )

    code_expected = bytearray(executable)
    for alias in CODE_BIAS_ALIASES:
        code_expected[alias] = literal_bias
    for operand in HELPER_BIAS_OPERANDS:
        code_expected[operand] = literal_bias
    if bytes(machine.mem_read(0, len(executable))) != bytes(code_expected):
        raise AssertionError(f"{name}: unexpected code memory write")

    return {
        **vector,
        "bbb_terminal_value": terminal_value,
        "bbb_output_result_offset": (output_offset + produced) & 0xFFFF,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    if sha256(executable) != EXECUTABLE_SHA256:
        raise ValueError("unrecognized BLOOD2PG.EXE")
    if sha256(executable[ENTRY:ROUTINE_END]) != ROUTINE_SHA256:
        raise ValueError("BBB presentation AD routine bytes changed")
    if sha256(executable[HELPER_ENTRY:HELPER_END]) != HELPER_SHA256:
        raise ValueError("BBB pair-LZ helper bytes changed")

    fixture_bytes = COMMANDER_FIXTURE.read_bytes()
    if sha256(fixture_bytes) != COMMANDER_FIXTURE_SHA256:
        raise ValueError("Commander presentation AD fixture changed")
    vectors = json.loads(fixture_bytes)
    if not isinstance(vectors, list) or len(vectors) != 9:
        raise ValueError("expected nine Commander presentation AD vectors")

    verified = [
        execute_case(executable, vector, case_index)
        for case_index, vector in enumerate(vectors)
    ]
    report = {
        "executable_sha256": EXECUTABLE_SHA256,
        "entry": ENTRY,
        "routine_end": ROUTINE_END,
        "routine_sha256": ROUTINE_SHA256,
        "helper_entry": HELPER_ENTRY,
        "helper_end": HELPER_END,
        "helper_sha256": HELPER_SHA256,
        "commander_fixture": str(COMMANDER_FIXTURE.relative_to(REPO_ROOT)),
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "vectors": verified,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"verified {len(verified)} original BBB presentation AD cases")


if __name__ == "__main__":
    main()
