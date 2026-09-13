#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang sound-driver initialization."""

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
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

ROOT = Path(__file__).resolve().parents[2]
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_b7b0_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xB7B0, 0xB7E3),
        "sha256": "a4c1c0b88b5a0946e63d332601244eae5acfc57d56ab020bb232920255f67b3c",
        "table": 0x0CD3,
        "callback_pointer": 0x0AEC,
        "configuration": 0x0C45,
        "callback_return": 0xB7DA,
        "final_di": 0x0CF9,
    },
    "sequel": {
        "routine": (0xCF40, 0xCF73),
        "sha256": "1e977b4799b218d2ed870ec0c3dcfff9c072490c54d6bc0e7f1d16f52f19cf58",
        "table": 0x0F21,
        "callback_pointer": 0x0CF6,
        "configuration": 0x0E4F,
        "callback_return": 0xCF6A,
        "final_di": 0x0F47,
    },
}

MACHINE_SIZE = 0x120000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x2000
DATA_SEGMENT = 0x4000
EXTRA_SEGMENT = 0x6000
UNOWNED_SEGMENT = 0x8000
STACK_SEGMENT = 0xA000
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("69965aa5c33c")

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
CALLBACK_REGISTER_IDS = {
    "ax": UC_X86_REG_AX,
    "bx": UC_X86_REG_BX,
    "cx": UC_X86_REG_CX,
    "dx": UC_X86_REG_DX,
    "si": UC_X86_REG_SI,
    "di": UC_X86_REG_DI,
    "bp": UC_X86_REG_BP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
}
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def image(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 13 + seed * 29 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


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
        loop_control = instruction.mnemonic in {
            "jcxz",
            "jecxz",
            "loop",
            "loope",
            "loopne",
        }
        if capstone.CS_GRP_JUMP not in instruction.groups and not loop_control:
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


def execute(
    executable: bytes,
    branch_name: str,
    vector: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    incoming_ds = (
        GAME_SEGMENT if vector["name"] == "shared_ds_and_gs" else DATA_SEGMENT
    )
    driver_segment = int(vector["driver_segment"])
    offsets = [int(value) for value in vector["entry_offsets"]]
    callback_offset = offsets[0]
    callback_address = driver_segment * 16 + callback_offset
    configuration = int(vector["configuration"])
    callback_result = int(vector["callback_result_ax"])
    callback_flags = sum(
        FLAG_MASKS[name]
        for name, enabled in vector["defined_flags"].items()
        if enabled
    ) | 0x0002

    game = image(case_index + 1)
    data = image(case_index + 17)
    extra = image(case_index + 33)
    unowned = image(case_index + 49)
    stack = image(case_index + 65)
    driver = image(case_index + 81)
    initial_segments = [
        (0x1000 + case_index * 0x101 + index * 0x77) & 0xFFFF
        for index in range(9)
    ]
    table_before = b"".join(
        struct.pack("<HH", offset, segment)
        for offset, segment in zip(offsets, initial_segments, strict=True)
    )
    table_after = b"".join(
        struct.pack("<HH", offset, driver_segment) for offset in offsets
    )
    table_offset = int(branch["table"])
    pointer_offset = int(branch["callback_pointer"])
    configuration_offset = int(branch["configuration"])
    pointer_before = bytes.fromhex("a55a6996")
    game[table_offset : table_offset + len(table_before)] = table_before
    game[pointer_offset : pointer_offset + 4] = pointer_before
    struct.pack_into("<H", game, configuration_offset, configuration)
    if incoming_ds != GAME_SEGMENT:
        data[table_offset : table_offset + len(table_before)] = bytes(
            value ^ 0xFF for value in table_before
        )
        data[pointer_offset : pointer_offset + 4] = pointer_before[::-1]
        struct.pack_into("<H", data, configuration_offset, configuration ^ 0xFFFF)
    driver[callback_offset] = 0xCB
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    expected_game = bytearray(game)
    expected_game[table_offset : table_offset + len(table_after)] = table_after
    expected_game[pointer_offset : pointer_offset + 4] = struct.pack("<HH", 0x011D, 0)
    expected_stack = bytearray(stack)

    initial = {
        UC_X86_REG_EAX: 0xA1A10000 | driver_segment,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: incoming_ds,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: 0x5000,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    callback_changes = {
        "ax": callback_result,
        "bx": 0x1357,
        "cx": 0x2468,
        "dx": 0x369C,
        "si": 0x48AD,
        "di": 0x5ABE,
        "bp": 0x6BCF,
        "ds": 0x7CE0,
        "es": 0x8DF1,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in (
        (GAME_SEGMENT, game),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (UNOWNED_SEGMENT, unowned),
        (STACK_SEGMENT, stack),
        (driver_segment, driver),
    ):
        machine.mem_write(segment * 16, bytes(contents))
    for register, value in initial.items():
        machine.reg_write(register, value)

    body = executable[start:stop]
    expected_edges = conditional_edges(body, start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    callback_calls = 0
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal callback_calls, previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == callback_address:
            callback_calls += 1
            assert bytes(cpu.mem_read(GAME_SEGMENT * 16 + table_offset, 36)) == table_after
            assert bytes(cpu.mem_read(GAME_SEGMENT * 16 + pointer_offset, 4)) == struct.pack(
                "<HH", 0x011D, 0
            )
            actual_entry = {
                "ax": cpu.reg_read(UC_X86_REG_AX),
                "bx": cpu.reg_read(UC_X86_REG_BX),
                "cx": cpu.reg_read(UC_X86_REG_CX),
                "dx": cpu.reg_read(UC_X86_REG_DX),
                "si": cpu.reg_read(UC_X86_REG_SI),
                "di": cpu.reg_read(UC_X86_REG_DI),
                "bp": cpu.reg_read(UC_X86_REG_BP),
                "sp": cpu.reg_read(UC_X86_REG_SP),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "cs": cpu.reg_read(UC_X86_REG_CS),
                "ip": cpu.reg_read(UC_X86_REG_IP),
            }
            expected_entry = {
                "ax": configuration,
                "bx": GAME_SEGMENT,
                "cx": 0,
                "dx": initial[UC_X86_REG_EDX] & 0xFFFF,
                "si": initial[UC_X86_REG_ESI] & 0xFFFF,
                "di": int(branch["final_di"]),
                "bp": initial[UC_X86_REG_EBP] & 0xFFFF,
                "sp": 0xFEEC,
                "ds": GAME_SEGMENT,
                "es": EXTRA_SEGMENT,
                "cs": driver_segment,
                "ip": callback_offset,
            }
            assert actual_entry == expected_entry, (vector["name"], branch_name, actual_entry)
            expected_frame = (
                int(branch["callback_return"]),
                0,
                initial[UC_X86_REG_EBP] & 0xFFFF,
                initial[UC_X86_REG_ESI] & 0xFFFF,
                incoming_ds,
                initial[UC_X86_REG_EDI] & 0xFFFF,
                EXTRA_SEGMENT,
                initial[UC_X86_REG_EDX] & 0xFFFF,
                initial[UC_X86_REG_ECX] & 0xFFFF,
                initial[UC_X86_REG_EBX] & 0xFFFF,
            )
            actual_frame = struct.unpack(
                "<HHHHHHHHHH",
                cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEC, 20),
            )
            assert actual_frame == expected_frame, (
                vector["name"],
                branch_name,
                actual_frame,
            )
            struct.pack_into("<HHHHHHHHHH", expected_stack, 0xFEEC, *expected_frame)
            for register, value in callback_changes.items():
                cpu.reg_write(CALLBACK_REGISTER_IDS[register], value)
            cpu.reg_write(UC_X86_REG_EFLAGS, callback_flags)
            previous_source = None
            return
        assert start <= address < stop, (vector["name"], branch_name, hex(address))
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

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
    assert callback_calls == 1, (vector["name"], branch_name, callback_calls)

    for label, segment, expected in (
        ("game", GAME_SEGMENT, expected_game),
        ("data", DATA_SEGMENT, data),
        ("extra", EXTRA_SEGMENT, extra),
        ("unowned", UNOWNED_SEGMENT, unowned),
        ("stack", STACK_SEGMENT, expected_stack),
        ("driver", driver_segment, driver),
    ):
        actual = bytes(machine.mem_read(segment * 16, SEGMENT_SIZE))
        assert actual == bytes(expected), (
            vector["name"],
            branch_name,
            label,
            first_difference(actual, bytes(expected)),
        )
    assert bytes(machine.mem_read(0, len(executable))) == executable, (
        vector["name"],
        branch_name,
        "executable changed",
    )

    expected_registers = {
        "eax": (initial[UC_X86_REG_EAX] & 0xFFFF0000) | callback_result,
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": incoming_ds,
        "es": EXTRA_SEGMENT,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    assert actual_registers == expected_registers, (
        vector["name"],
        branch_name,
        actual_registers,
        expected_registers,
    )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}
    assert actual_flags == vector["defined_flags"], (
        vector["name"],
        branch_name,
        actual_flags,
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    assert covered_edges == expected_edges, (
        vector["name"],
        branch_name,
        expected_edges - covered_edges,
        covered_edges - expected_edges,
    )

    row = dict(vector)
    row.update(
        {
            "callback_calls": callback_calls,
            "loop_iterations": 9,
            "registers_after": actual_registers,
            "return": "far",
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
    assert len(vectors) == 6

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
        digest = hashlib.sha256(executable[start:stop]).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(executable[start:stop], start)
        assert coverage[branch_name] == expected_edges

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual audio-driver-init cases and "
        f"{len(coverage['sequel'])} conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} branch sites"
    )


if __name__ == "__main__":
    main()
