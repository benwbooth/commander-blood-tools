#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio-driver stop wrappers."""

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

import capstone  # noqa: E402
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM  # noqa: E402
from unicorn import (  # noqa: E402
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (  # noqa: E402
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
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_bb9d_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xBB9D, 0xBBB3),
        "sha256": "e7fc7b7a0177bf7cbb2bd10dad92f9144b1bb5af1d5fb5d71b53b43f03ac725b",
        "callback_pointer": 0x0CDF,
        "callback_return": 0xBBAA,
        "pending": 0x0BA0,
        "ultrasound": None,
        "hardware_helper": None,
    },
    "sequel": {
        "routine": (0xD33A, 0xD363),
        "sha256": "427bace9b5d5ed4bb182de42a0335c6be9becb316b816ddb15afc7e8f7396588",
        "callback_pointer": 0x0F2D,
        "callback_return": 0xD35A,
        "pending": 0x0DAA,
        "ultrasound": 0x0F1F,
        "hardware_helper": 0xDDD2,
    },
}

CASES = (
    {
        "name": "pending_zero",
        "pending": 0x00,
        "callback_pending": 0x5A,
        "callback_flags": 0x0202,
        "clobber": False,
    },
    {
        "name": "pending_bit_zero",
        "pending": 0x01,
        "callback_pending": 0xA5,
        "callback_flags": 0x0247,
        "clobber": True,
    },
    {
        "name": "pending_bit_one",
        "pending": 0x02,
        "callback_pending": 0x3C,
        "callback_flags": 0x0296,
        "clobber": False,
    },
    {
        "name": "pending_all_bits",
        "pending": 0xFF,
        "callback_pending": 0xC3,
        "callback_flags": 0x0AD7,
        "clobber": True,
    },
    {
        "name": "callback_rewrites_pending",
        "pending": 0x80,
        "callback_pending": 0x7E,
        "callback_flags": 0x0883,
        "clobber": True,
    },
    {
        "name": "shared_ds_and_gs",
        "pending": 0x55,
        "callback_pending": 0xE1,
        "callback_flags": 0x0612,
        "clobber": False,
        "shared_ds": True,
    },
)

ULTRASOUND_CASES = (
    {"name": "ultrasound_pending_zero", "pending": 0x00, "shared_ds": False},
    {"name": "ultrasound_pending_bits", "pending": 0xA5, "shared_ds": True},
)

MACHINE_SIZE = 0xC0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x4000
DATA_SEGMENT = 0x5000
EXTRA_SEGMENT = 0x6000
FS_SEGMENT = 0x7000
STACK_SEGMENT = 0x8000
CALLBACK_SEGMENT = 0x9000
TRAP_SEGMENT = 0xA000
UNOWNED_SEGMENT = 0xB000
CALLBACK_OFFSET = 0x0100
TRAP_OFFSET = 0x0200
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("69965aa5c33c")
VECTOR_GAME_SEGMENT = 0x2C00

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
LOW_REGISTER_IDS = {
    "ax": UC_X86_REG_AX,
    "bx": UC_X86_REG_BX,
    "cx": UC_X86_REG_CX,
    "dx": UC_X86_REG_DX,
    "si": UC_X86_REG_SI,
    "di": UC_X86_REG_DI,
    "bp": UC_X86_REG_BP,
    "es": UC_X86_REG_ES,
}
CALLBACK_CHANGES = {
    "ax": 0xCAFE,
    "bx": 0x1357,
    "cx": 0x2468,
    "dx": 0x369C,
    "si": 0x48AD,
    "di": 0x5ABE,
    "bp": 0x6BCF,
    "es": 0x7CE0,
}
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (offset * 19 + (offset >> 8) * 7 + seed * 31 + 0x53) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    edges = set()
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
        edges.add((source, int(immediate) - start))
        edges.add((source, instruction.address + instruction.size - start))
    return edges


def write_pointer(
    memory: bytearray, offset: int, pointer_offset: int, segment: int
) -> None:
    struct.pack_into("<HH", memory, offset, pointer_offset, segment)


def initial_registers(case_index: int, incoming_ds: int) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
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
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }


def map_segments(
    machine: Uc,
    executable: bytes,
    segments: tuple[tuple[int, bytearray], ...],
) -> None:
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for segment, contents in segments:
        machine.mem_write(segment * 16, bytes(contents))


def final_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def execute_callback_path(
    executable: bytes,
    branch_name: str,
    case: dict[str, Any],
    expected_row: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    shared_ds = bool(case.get("shared_ds", False))
    incoming_ds = GAME_SEGMENT if shared_ds else DATA_SEGMENT
    initial = initial_registers(case_index, incoming_ds)

    game = seeded_segment(case_index + 1)
    data = seeded_segment(case_index + 17)
    extra = seeded_segment(case_index + 33)
    fs_data = seeded_segment(case_index + 49)
    stack = seeded_segment(case_index + 65)
    callback = seeded_segment(case_index + 81)
    trap = seeded_segment(case_index + 97)
    unowned = seeded_segment(case_index + 113)
    pending_offset = int(branch["pending"])
    callback_pointer = int(branch["callback_pointer"])
    game[pending_offset] = int(case["pending"])
    write_pointer(game, callback_pointer, CALLBACK_OFFSET, CALLBACK_SEGMENT)
    if branch["ultrasound"] is not None:
        game[int(branch["ultrasound"])] = 0
    data_pending = (0x31 + case_index * 17) & 0xFF
    if not shared_ds:
        data[pending_offset] = data_pending
        write_pointer(data, callback_pointer, TRAP_OFFSET, TRAP_SEGMENT)
    callback[CALLBACK_OFFSET] = 0xCB
    trap[TRAP_OFFSET] = 0xCC
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )

    game_before = bytes(game)
    data_before = bytes(data)
    extra_before = bytes(extra)
    fs_before = bytes(fs_data)
    callback_before = bytes(callback)
    trap_before = bytes(trap)
    unowned_before = bytes(unowned)
    stack_before = bytearray(stack)

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    map_segments(
        machine,
        executable,
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (EXTRA_SEGMENT, extra),
            (FS_SEGMENT, fs_data),
            (STACK_SEGMENT, stack),
            (CALLBACK_SEGMENT, callback),
            (TRAP_SEGMENT, trap),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected_edges = conditional_edges(executable[start:stop], start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    reached_return = False
    callback_calls = 0
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return, callback_calls
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        linear = cpu.reg_read(UC_X86_REG_CS) * 16 + cpu.reg_read(UC_X86_REG_IP)
        if linear == CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET:
            callback_calls += 1
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF6
            frame = struct.unpack(
                "<HHHHH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF6, 10)
            )
            assert frame == (
                int(branch["callback_return"]),
                0,
                EXTRA_SEGMENT,
                incoming_ds,
                initial[UC_X86_REG_EAX] & 0xFFFF,
            ), (case["name"], branch_name, frame)
            assert cpu.reg_read(UC_X86_REG_EAX) == initial[UC_X86_REG_EAX] & 0xFFFF0000
            assert cpu.reg_read(UC_X86_REG_DS) == GAME_SEGMENT
            assert cpu.mem_read(GAME_SEGMENT * 16 + pending_offset, 1)[0] == int(
                case["pending"]
            )
            cpu.mem_write(
                GAME_SEGMENT * 16 + pending_offset,
                bytes((int(case["callback_pending"]),)),
            )
            if bool(case["clobber"]):
                for name, value in CALLBACK_CHANGES.items():
                    cpu.reg_write(LOW_REGISTER_IDS[name], value)
            cpu.reg_write(UC_X86_REG_EFLAGS, int(case["callback_flags"]))
            previous_source = None
            return
        assert cpu.reg_read(UC_X86_REG_CS) == 0 and start <= address < stop, (
            case["name"],
            branch_name,
            hex(address),
            hex(linear),
        )
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(start, 0, count=200)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return and callback_calls == 1

    expected_game = bytearray(game_before)
    expected_game[pending_offset] = 0
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_game
    )
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
    assert (
        bytes(machine.mem_read(CALLBACK_SEGMENT * 16, SEGMENT_SIZE)) == callback_before
    )
    assert bytes(machine.mem_read(TRAP_SEGMENT * 16, SEGMENT_SIZE)) == trap_before
    assert bytes(machine.mem_read(UNOWNED_SEGMENT * 16, SEGMENT_SIZE)) == unowned_before
    assert bytes(machine.mem_read(0, len(executable))) == executable

    expected_stack = bytearray(stack_before)
    struct.pack_into(
        "<HHHHH",
        expected_stack,
        0xFEF6,
        int(branch["callback_return"]),
        0,
        EXTRA_SEGMENT,
        incoming_ds,
        initial[UC_X86_REG_EAX] & 0xFFFF,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEF6 <= address
        and address + size <= STACK_SEGMENT * 16 + 0xFF00
        or GAME_SEGMENT * 16 + pending_offset <= address
        and address + size <= GAME_SEGMENT * 16 + pending_offset + 1
        for address, size in writes
    ), (case["name"], branch_name, writes)

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
        "sp": STACK_POINTER + 4,
        "ds": incoming_ds,
        "es": EXTRA_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    if bool(case["clobber"]):
        for name in ("ebx", "ecx", "edx", "esi", "edi", "ebp"):
            expected_registers[name] = (
                expected_registers[name] & 0xFFFF0000 | CALLBACK_CHANGES[name[1:]]
            )
    assert actual_registers == expected_registers, (
        case["name"],
        branch_name,
        actual_registers,
        expected_registers,
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    flags_after = final_flags(machine)
    expected_flags = {
        name: bool(int(case["callback_flags"]) & mask)
        for name, mask in FLAG_MASKS.items()
    }
    assert flags_after == expected_flags

    row = {
        "name": case["name"],
        "pending_before": int(case["pending"]),
        "pending_written_by_callback": int(case["callback_pending"]),
        "pending_after": 0,
        "callback_ax": 0,
        "callback_ds": VECTOR_GAME_SEGMENT,
        "callback_sp": 0xFEF6,
        "callback_clobbers_passed_through": bool(case["clobber"]),
        "defined_flags": expected_flags,
    }
    assert row == expected_row, (case["name"], branch_name, row, expected_row)
    return row, covered_edges


def execute_ultrasound_path(
    executable: bytes,
    case: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES["sequel"]
    start, stop = branch["routine"]
    helper = int(branch["hardware_helper"])
    incoming_ds = GAME_SEGMENT if bool(case["shared_ds"]) else DATA_SEGMENT
    initial = initial_registers(case_index + 20, incoming_ds)

    game = seeded_segment(case_index + 129)
    data = seeded_segment(case_index + 145)
    extra = seeded_segment(case_index + 161)
    fs_data = seeded_segment(case_index + 177)
    stack = seeded_segment(case_index + 193)
    unowned = seeded_segment(case_index + 209)
    pending_offset = int(branch["pending"])
    game[pending_offset] = int(case["pending"])
    game[int(branch["ultrasound"])] = 1
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    game_before = bytes(game)
    data_before = bytes(data)
    extra_before = bytes(extra)
    fs_before = bytes(fs_data)
    stack_before = bytearray(stack)
    unowned_before = bytes(unowned)

    patched = bytearray(executable)
    patched[helper] = 0xC3
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    map_segments(
        machine,
        bytes(patched),
        (
            (GAME_SEGMENT, game),
            (DATA_SEGMENT, data),
            (EXTRA_SEGMENT, extra),
            (FS_SEGMENT, fs_data),
            (STACK_SEGMENT, stack),
            (UNOWNED_SEGMENT, unowned),
        ),
    )
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected_edges = conditional_edges(executable[start:stop], start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    reached_return = False
    helper_calls: list[dict[str, int]] = []
    writes: list[tuple[int, int]] = []

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == helper and cpu.reg_read(UC_X86_REG_CS) == 0:
            call_index = len(helper_calls)
            expected_return = (0xD34A, 0xD34E)[call_index]
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEF8
            frame = struct.unpack("<HHHH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEF8, 8))
            assert frame == (
                expected_return,
                EXTRA_SEGMENT,
                incoming_ds,
                initial[UC_X86_REG_EAX] & 0xFFFF,
            )
            helper_calls.append(
                {
                    "channel": cpu.reg_read(UC_X86_REG_AX),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                }
            )
            previous_source = None
            return
        assert cpu.reg_read(UC_X86_REG_CS) == 0 and start <= address < stop
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    def memory_write(
        _cpu: Uc, _access: int, address: int, size: int, _value: int, _context: object
    ) -> None:
        writes.append((address, size))

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.hook_add(UC_HOOK_MEM_WRITE, memory_write)
    try:
        machine.emu_start(start, 0, count=100)
    except UcError as error:
        raise RuntimeError(
            f"{case['name']}: failed at {machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return
    assert helper_calls == [
        {"channel": 0, "ds": incoming_ds, "sp": 0xFEF8},
        {"channel": 1, "ds": incoming_ds, "sp": 0xFEF8},
    ]
    assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == game_before
    assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == data_before
    assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
    assert bytes(machine.mem_read(UNOWNED_SEGMENT * 16, SEGMENT_SIZE)) == unowned_before
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)

    expected_stack = bytearray(stack_before)
    struct.pack_into(
        "<HHHH",
        expected_stack,
        0xFEF8,
        0xD34E,
        EXTRA_SEGMENT,
        incoming_ds,
        initial[UC_X86_REG_EAX] & 0xFFFF,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEF8 <= address
        and address + size <= STACK_SEGMENT * 16 + 0xFF00
        for address, size in writes
    ), (case["name"], writes)

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
        "sp": STACK_POINTER + 4,
        "ds": incoming_ds,
        "es": EXTRA_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    assert actual_registers == expected_registers
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )
    flags_after = final_flags(machine)
    assert flags_after == {name: False for name in FLAG_MASKS}
    return {
        "name": case["name"],
        "pending_before": int(case["pending"]),
        "pending_after": int(case["pending"]),
        "helper_calls": helper_calls,
        "defined_flags": flags_after,
        "return": "far",
    }, covered_edges


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
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(CASES) == len(expected_rows) == 6
    assert [case["name"] for case in CASES] == [row["name"] for row in expected_rows]

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, (case, expected_row) in enumerate(
        zip(CASES, expected_rows, strict=True)
    ):
        commander_row, commander_edges = execute_callback_path(
            commander, "commander", case, expected_row, case_index
        )
        sequel_row, sequel_edges = execute_callback_path(
            sequel, "sequel", case, expected_row, case_index
        )
        assert commander_row == sequel_row
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    ultrasound_rows = []
    for case_index, case in enumerate(ULTRASOUND_CASES):
        row, edges = execute_ultrasound_path(sequel, case, case_index)
        ultrasound_rows.append(row)
        coverage["sequel"].update(edges)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges, (
            branch_name,
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
        f"verified {len(rows)} dual callback cases, "
        f"{len(ultrasound_rows)} BBB ULTRASND stop cases, and "
        f"{len(coverage['sequel'])} BBB conditional edges"
    )


if __name__ == "__main__":
    main()
