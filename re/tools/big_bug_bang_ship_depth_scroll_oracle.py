#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang ship depth-scroll steps directly."""

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
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
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
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_b75c_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xB75C, 0xB7A8),
        "sha256": "7b169cde9fa6c63a0388539519b45c9d087b3079b8bfc60fe27c28ade04553dd",
        "fields": {
            "depth": 0x2527,
            "opening": 0x252F,
            "closing": 0x2530,
            "step": 0x2531,
        },
    },
    "sequel": {
        "routine": (0xCEE9, 0xCF35),
        "sha256": "ea3b7cd8bb7efdd5637c0e2c2974b664e58a1048c6a24ff6e50b529df4a38490",
        "fields": {
            "depth": 0x2779,
            "opening": 0x2781,
            "closing": 0x2782,
            "step": 0x2783,
        },
    },
}

MACHINE_SIZE = 0xC0000
SEGMENT_SIZE = 0x10000
DATA = 0x40000
GAME = 0x60000
INCOMING_ES = 0x80000
STACK = 0xA0000
STACK_POINTER = 0xFF00
STACK_TRANSIENT_START = 0xFEFC
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

REGISTER_IDS = {
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
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def write_word(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", memory, offset, value & 0xFFFF)


def read_word(memory: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", memory, offset)[0]


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


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


def initialize(branch: dict[str, Any], vector: dict[str, Any], case_index: int):
    fields = branch["fields"]
    data = bytearray(
        (offset * 17 + (offset >> 8) * 13 + case_index * 29 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    game = bytearray(
        (offset * 23 + (offset >> 8) * 7 + case_index * 31 + 0x65) & 0xFF
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
    write_word(data, int(fields["depth"]), int(vector["depth_before"]))
    data[int(fields["opening"])] = int(vector["opening_before"])
    data[int(fields["closing"])] = int(vector["closing_before"])
    data[int(fields["step"])] = int(vector["step"])
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    return data, game, incoming_es, stack


def expected_images(branch: dict[str, Any], vector: dict[str, Any], before):
    data, game, incoming_es, stack = (bytearray(image) for image in before)
    fields = branch["fields"]
    write_word(data, int(fields["depth"]), int(vector["depth_after"]))
    data[int(fields["opening"])] = int(vector["opening_after"])
    data[int(fields["closing"])] = int(vector["closing_after"])
    return data, game, incoming_es, stack


def execute(
    executable: bytes,
    branch_name: str,
    vector: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    fields = branch["fields"]
    before = initialize(branch, vector, case_index)
    expected = expected_images(branch, vector, before)
    data, game, incoming_es, stack = before
    backward = case_index == 16

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
        UC_X86_REG_FS: 0x9000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0AD7 | (0x0400 if backward else 0),
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for address, image in (
        (DATA, data),
        (GAME, game),
        (INCOMING_ES, incoming_es),
        (STACK, stack),
    ):
        machine.mem_write(address, bytes(image))
    for register, value in initial.items():
        machine.reg_write(register, value)

    covered_edges: set[tuple[int, int]] = set()
    previous_branch: int | None = None
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_branch, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        assert start <= address < stop, (vector["name"], branch_name, hex(address))
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
    try:
        machine.emu_start(start, 0, count=500)
    except UcError as error:
        raise RuntimeError(
            f"{vector['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (vector["name"], branch_name, "did not return")

    actual_images = tuple(
        bytes(machine.mem_read(address, SEGMENT_SIZE))
        for address in (DATA, GAME, INCOMING_ES, STACK)
    )
    for label, actual, wanted in zip(
        ("data", "game", "incoming ES"),
        actual_images[:3],
        expected[:3],
        strict=True,
    ):
        assert actual == bytes(wanted), (
            vector["name"],
            branch_name,
            label,
            first_difference(actual, bytes(wanted)),
        )
    stack_after = actual_images[3]
    assert stack_after[:STACK_TRANSIENT_START] == bytes(
        expected[3][:STACK_TRANSIENT_START]
    ), (vector["name"], branch_name, "stack prefix")
    assert stack_after[STACK_POINTER:] == bytes(expected[3][STACK_POINTER:]), (
        vector["name"],
        branch_name,
        "caller stack",
    )
    assert bytes(machine.mem_read(0, len(executable))) == executable, (
        vector["name"],
        branch_name,
        "executable",
    )
    for label, first, last in (
        ("executable/data", len(executable), DATA),
        ("data/game", DATA + SEGMENT_SIZE, GAME),
        ("game/ES", GAME + SEGMENT_SIZE, INCOMING_ES),
        ("ES/stack", INCOMING_ES + SEGMENT_SIZE, STACK),
        ("tail", STACK + SEGMENT_SIZE, MACHINE_SIZE),
    ):
        assert bytes(machine.mem_read(first, last - first)) == bytes(last - first), (
            vector["name"],
            branch_name,
            f"unowned {label} gap",
        )

    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    expected_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 2,
        "ds": DATA // 16,
        "es": INCOMING_ES // 16,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME // 16,
        "ss": STACK // 16,
    }
    assert actual_registers == expected_registers, (
        vector["name"],
        branch_name,
        actual_registers,
        expected_registers,
    )

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        name: bool(flags & FLAG_MASKS[name]) for name in vector["defined_flags"]
    }
    assert actual_flags == vector["defined_flags"], (
        vector["name"],
        branch_name,
        actual_flags,
        vector["defined_flags"],
    )
    assert bool(flags & 0x0400) == backward, (
        vector["name"],
        branch_name,
        "DF changed",
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    data_after = actual_images[0]
    row = dict(vector)
    assert read_word(data_after, int(fields["depth"])) == int(vector["depth_after"])
    row.update(
        {
            "direction": "backward" if backward else "forward",
            "registers_after": actual_registers,
            "return": "near",
        }
    )
    return row, covered_edges


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
    vectors = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(vectors) == 17

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, vector in enumerate(vectors):
        commander_row, commander_edges = execute(
            commander, "commander", vector, case_index
        )
        sequel_row, sequel_edges = execute(sequel, "sequel", vector, case_index)
        assert commander_row == sequel_row, (vector["name"], commander_row, sequel_row)
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
        f"verified {len(rows)} dual ship-depth-scroll cases and "
        f"{len(coverage['sequel'])} conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} branch sites"
    )


if __name__ == "__main__":
    main()
