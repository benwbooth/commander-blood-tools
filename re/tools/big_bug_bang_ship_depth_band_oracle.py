#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang ship depth-band copies directly."""

# ruff: noqa: E402

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INSN, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
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
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xB6DD, 0xB75C),
        "sha256": "616bbe2388ea24026c85002272ceaa97797ba660dbfa0531582defe5309fbe55",
        "fields": {
            "gate": 0x252E,
            "depth": 0x2527,
            "pointer": 0x5219,
            "increment": 0x524D,
            "percent": 0x524F,
        },
    },
    "sequel": {
        "routine": (0xCE6A, 0xCEE9),
        "sha256": "b190ad52e10f3438682a4489e565b2d8a8914f7e3fbb9a3711459a9262eb2619",
        "fields": {
            "gate": 0x2780,
            "depth": 0x2779,
            "pointer": 0x55E9,
            "increment": 0x561D,
            "percent": 0x561F,
        },
    },
}

MACHINE_SIZE = 0xD0000
SEGMENT_SIZE = 0x10000
DATA = 0x40000
GAME = 0x50000
BUFFER = 0x70000
INCOMING_ES = 0x90000
STACK = 0xB0000
STACK_POINTER = 0xFF00
STACK_TRANSIENT_START = 0xFEE8
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

CASES = (
    ("gate_clear_zero", 0x00, 0x0020, 9, 0xA111, 0x0200, 0x13, False),
    ("gate_clear_other_bits", 0xFE, 0x0040, 9, 0xA222, 0x0400, 0x27, False),
    ("increment_ten_preserves_percent", 0x01, 0x0041, 10, 0xA333, 0x0600, 0x3B, False),
    ("depth_zero_sets_percent_100", 0x01, 0x0000, 9, 0xA444, 0x0800, 0x4F, False),
    ("depth_25_sets_percent_50", 0x03, 0x0019, 11, 0xA555, 0x0A00, 0x63, False),
    ("depth_50_sets_percent_zero", 0x01, 0x0032, 12, 0xA666, 0x0C00, 0x77, False),
    ("depth_51_clamps_doubled_value", 0x01, 0x0033, 13, 0xA777, 0x0E00, 0x8B, False),
    (
        "signed_doubled_depth_is_not_clamped",
        0x01,
        0x4000,
        14,
        0xA888,
        0x1000,
        0x9F,
        False,
    ),
    (
        "low_byte_band_count_wraps_to_zero",
        0x01,
        0x00DD,
        15,
        0xA999,
        0x1200,
        0xB3,
        False,
    ),
    (
        "maximum_band_count_wraps_second_destination",
        0x01,
        0x00DC,
        16,
        0xAAAA,
        0x1400,
        0xC7,
        False,
    ),
    ("destination_offsets_wrap", 0x01, 0x0041, 17, 0xBBBB, 0xF200, 0xDB, False),
    ("inherited_backward_direction", 0x01, 0x0005, 18, 0xCCCC, 0x1800, 0xEF, True),
)


def write_word(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", memory, offset, value & 0xFFFF)


def read_word(memory: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", memory, offset)[0]


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


def add16_flags(left: int, right: int) -> dict[str, bool]:
    result = (left + right) & 0xFFFF
    return {
        "cf": left + right > 0xFFFF,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "of": bool((~(left ^ right) & (left ^ result)) & 0x8000),
    }


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    result = set()
    for instruction in decoder.disasm(body, start):
        if capstone.CS_GRP_JUMP not in instruction.groups:
            continue
        if instruction.id in (X86_INS_JMP, X86_INS_LJMP):
            continue
        immediate = next(
            (
                operand.imm
                for operand in instruction.operands
                if operand.type == X86_OP_IMM
            ),
            None,
        )
        assert immediate is not None
        source = instruction.address - start
        result.add((source, int(immediate) - start))
        result.add((source, instruction.address + instruction.size - start))
    return result


def case_dict(values: tuple[Any, ...]) -> dict[str, Any]:
    names = (
        "name",
        "gate",
        "depth",
        "transition_increment",
        "percent_before",
        "destination_offset",
        "graphics_mode",
        "backward",
    )
    return dict(zip(names, values, strict=True))


def initialize(branch: dict[str, Any], case: dict[str, Any], case_index: int):
    fields = branch["fields"]
    data = bytearray(
        (offset * 17 + (offset >> 8) * 13 + case_index * 29 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    game = bytearray(
        (offset * 23 + (offset >> 8) * 7 + case_index * 31 + 0x65) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    buffer = bytearray(
        (offset * 29 + (offset >> 8) * 17 + case_index * 41 + 0x35) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    incoming_es = bytearray(
        (offset * 11 + (offset >> 8) * 19 + case_index * 37 + 0x87) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    stack = bytearray(
        (offset * 13 + (offset >> 8) * 31 + case_index * 47 + 0xED) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    data[int(fields["gate"])] = int(case["gate"])
    write_word(data, int(fields["depth"]), int(case["depth"]))
    write_word(data, int(fields["increment"]), int(case["transition_increment"]))
    write_word(data, int(fields["percent"]), int(case["percent_before"]))
    pointer = (BUFFER // 16 << 16) | int(case["destination_offset"])
    struct.pack_into("<I", data, int(fields["pointer"]), pointer)
    stack[STACK_POINTER : STACK_POINTER + 12] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    return data, game, buffer, incoming_es, stack


def expected_images(branch: dict[str, Any], case: dict[str, Any], before):
    data, game, buffer, incoming_es, stack = (bytearray(image) for image in before)
    fields = branch["fields"]
    active = int(case["gate"]) & 1 != 0
    depth = int(case["depth"])
    destination = int(case["destination_offset"])
    backward = bool(case["backward"])
    byte_count = ((depth + 35) & 0xFF) * 80
    first_source = (0xDF40 - byte_count) & 0xFFFF
    second_source = 0xDF40
    second_destination = (destination + 0x3E80 - byte_count) & 0xFFFF

    if active and int(case["transition_increment"]) != 10:
        doubled = (depth + depth) & 0xFFFF
        signed_doubled = doubled if doubled < 0x8000 else doubled - 0x10000
        if signed_doubled > 100:
            doubled = 100
        write_word(data, int(fields["percent"]), (100 - doubled) & 0xFFFF)

    def copy_bytes(source: int, target: int) -> None:
        step = -1 if backward else 1
        for _ in range(byte_count):
            buffer[target] = buffer[source]
            source = (source + step) & 0xFFFF
            target = (target + step) & 0xFFFF

    if active:
        copy_bytes(first_source, destination)
        copy_bytes(second_source, second_destination)
    return data, game, buffer, incoming_es, stack


def execute(
    executable: bytes,
    branch_name: str,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    fields = branch["fields"]
    before = initialize(branch, case, case_index)
    expected = expected_images(branch, case, before)
    data, game, buffer, incoming_es, stack = before
    active = int(case["gate"]) & 1 != 0
    backward = bool(case["backward"])
    depth = int(case["depth"])
    destination = int(case["destination_offset"])
    byte_count = ((depth + 35) & 0xFF) * 80
    first_source = (0xDF40 - byte_count) & 0xFFFF
    second_source = 0xDF40
    second_destination = (destination + 0x3E80 - byte_count) & 0xFFFF

    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: INCOMING_ES // 16,
        UC_X86_REG_FS: 0xA000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202 | (0x0400 if backward else 0),
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for address, image in (
        (DATA, data),
        (GAME, game),
        (BUFFER, buffer),
        (INCOMING_ES, incoming_es),
        (STACK, stack),
    ):
        machine.mem_write(address, bytes(image))
    for register, value in initial.items():
        machine.reg_write(register, value)

    inputs: list[tuple[int, int]] = []
    outputs: list[tuple[int, int, int]] = []
    covered_edges: set[tuple[int, int]] = set()
    previous_branch: int | None = None
    reached_return = False

    def input_port(_machine: Uc, port: int, size: int, _context: object) -> int:
        inputs.append((port, size))
        assert (port, size) == (0x03CF, 1), (case["name"], branch_name, port, size)
        return int(case["graphics_mode"])

    def output_port(
        _machine: Uc,
        port: int,
        size: int,
        value: int,
        _context: object,
    ) -> None:
        outputs.append((port, size, value))

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_branch, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        assert start <= address < stop, (case["name"], branch_name, hex(address))
        offset = address - start
        if previous_branch is not None:
            covered_edges.add((previous_branch, offset))
        encoded = bytes(cpu.mem_read(address, 2))
        previous_branch = (
            offset
            if 0x70 <= encoded[0] <= 0x7F
            or (encoded[0] == 0x0F and 0x80 <= encoded[1] <= 0x8F)
            else None
        )

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_INSN, input_port, None, 1, 0, UC_X86_INS_IN)
    machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
    try:
        machine.emu_start(start, 0, count=100_000)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (case["name"], branch_name, "did not return")

    expected_inputs = [(0x03CF, 1)] if active else []
    expected_outputs = (
        [
            (0x03C4, 2, 0x0F02),
            (0x03CE, 1, 5),
            (0x03CF, 1, (int(case["graphics_mode"]) & 0xFC) | 1),
            (0x03CF, 1, int(case["graphics_mode"])),
        ]
        if active
        else []
    )
    assert inputs == expected_inputs, (
        case["name"],
        branch_name,
        inputs,
        expected_inputs,
    )
    assert outputs == expected_outputs, (
        case["name"],
        branch_name,
        outputs,
        expected_outputs,
    )

    actual_images = tuple(
        bytes(machine.mem_read(address, SEGMENT_SIZE))
        for address in (DATA, GAME, BUFFER, INCOMING_ES, STACK)
    )
    for label, actual, wanted in zip(
        ("data", "game", "buffer", "incoming ES"),
        actual_images[:4],
        expected[:4],
        strict=True,
    ):
        assert actual == bytes(wanted), (
            case["name"],
            branch_name,
            label,
            first_difference(actual, bytes(wanted)),
        )
    stack_after = actual_images[4]
    assert stack_after[:STACK_TRANSIENT_START] == bytes(
        expected[4][:STACK_TRANSIENT_START]
    ), (case["name"], branch_name, "stack prefix")
    assert stack_after[STACK_POINTER:] == bytes(expected[4][STACK_POINTER:]), (
        case["name"],
        branch_name,
        "caller stack",
    )
    assert bytes(machine.mem_read(0, len(executable))) == executable, (
        case["name"],
        branch_name,
        "executable",
    )
    assert bytes(machine.mem_read(len(executable), DATA - len(executable))) == bytes(
        DATA - len(executable)
    ), (case["name"], branch_name, "unowned executable/data gap")
    assert bytes(
        machine.mem_read(DATA + SEGMENT_SIZE, GAME - DATA - SEGMENT_SIZE)
    ) == bytes(GAME - DATA - SEGMENT_SIZE), (
        case["name"],
        branch_name,
        "unowned data/game gap",
    )
    assert bytes(
        machine.mem_read(GAME + SEGMENT_SIZE, BUFFER - GAME - SEGMENT_SIZE)
    ) == bytes(BUFFER - GAME - SEGMENT_SIZE), (
        case["name"],
        branch_name,
        "unowned game/buffer gap",
    )
    assert bytes(
        machine.mem_read(BUFFER + SEGMENT_SIZE, INCOMING_ES - BUFFER - SEGMENT_SIZE)
    ) == bytes(INCOMING_ES - BUFFER - SEGMENT_SIZE), (
        case["name"],
        branch_name,
        "unowned buffer/ES gap",
    )
    assert bytes(
        machine.mem_read(INCOMING_ES + SEGMENT_SIZE, STACK - INCOMING_ES - SEGMENT_SIZE)
    ) == bytes(STACK - INCOMING_ES - SEGMENT_SIZE), (
        case["name"],
        branch_name,
        "unowned ES/stack gap",
    )
    assert bytes(
        machine.mem_read(STACK + SEGMENT_SIZE, MACHINE_SIZE - STACK - SEGMENT_SIZE)
    ) == bytes(MACHINE_SIZE - STACK - SEGMENT_SIZE), (
        case["name"],
        branch_name,
        "unowned tail",
    )

    register_ids = {
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
    actual_registers = {
        name: machine.reg_read(register) for name, register in register_ids.items()
    }
    expected_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": DATA // 16,
        "es": INCOMING_ES // 16,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME // 16,
        "ss": STACK // 16,
    }
    assert actual_registers == expected_registers, (
        case["name"],
        branch_name,
        actual_registers,
        expected_registers,
    )

    if active:
        expected_flags = add16_flags(
            (destination + 0x1F40) & 0xFFFF,
            (0x1F40 - byte_count) & 0xFFFF,
        )
    else:
        tested = int(case["gate"]) & 1
        expected_flags = {
            "cf": False,
            "pf": tested.bit_count() % 2 == 0,
            "zf": tested == 0,
            "sf": bool(tested & 0x80),
            "of": False,
        }
    expected_flags["df"] = backward
    flag_bits = machine.reg_read(UC_X86_REG_EFLAGS)
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "df": 0x0400,
        "of": 0x0800,
    }
    actual_flags = {name: bool(flag_bits & flag_masks[name]) for name in expected_flags}
    assert actual_flags == expected_flags, (
        case["name"],
        branch_name,
        actual_flags,
        expected_flags,
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    data_after, _, buffer_after, _, _ = actual_images
    return (
        {
            "name": case["name"],
            "gate": int(case["gate"]),
            "depth": depth,
            "transition_increment": int(case["transition_increment"]),
            "percent_before": int(case["percent_before"]),
            "percent_after": read_word(data_after, int(fields["percent"])),
            "destination_offset": destination,
            "byte_count": byte_count if active else 0,
            "first_source_offset": first_source if active else None,
            "second_source_offset": second_source if active else None,
            "second_destination_offset": second_destination if active else None,
            "graphics_mode": int(case["graphics_mode"]),
            "direction": "backward" if backward else "forward",
            "port_inputs": [list(values) for values in inputs],
            "port_outputs": [list(values) for values in outputs],
            "framebuffer_before_sha256": hashlib.sha256(buffer).hexdigest(),
            "framebuffer_after_sha256": hashlib.sha256(buffer_after).hexdigest(),
            "registers_after": actual_registers,
            "defined_flags": actual_flags,
            "return": "far",
        },
        covered_edges,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    commander = COMMANDER_PATH.read_bytes()
    sequel = args.sequel_executable.read_bytes()
    for name, executable, expected in (
        ("Commander", commander, COMMANDER_SHA256),
        ("BBB", sequel, SEQUEL_SHA256),
    ):
        digest = hashlib.sha256(executable).hexdigest()
        if digest != expected:
            raise SystemExit(f"unsupported {name} executable SHA-256 {digest}")

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, values in enumerate(CASES):
        case = case_dict(values)
        commander_row, commander_edges = execute(
            commander, "commander", case, case_index
        )
        sequel_row, sequel_edges = execute(sequel, "sequel", case, case_index)
        assert commander_row == sequel_row, (case["name"], commander_row, sequel_row)
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges, (
            branch_name,
            "conditional coverage",
            sorted(expected_edges - coverage[branch_name]),
            sorted(coverage[branch_name] - expected_edges),
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual ship-depth-band cases and "
        f"{len(coverage['sequel'])} conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} branch sites"
    )


if __name__ == "__main__":
    main()
