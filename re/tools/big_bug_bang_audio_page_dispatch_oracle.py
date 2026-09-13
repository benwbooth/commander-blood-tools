#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang audio page-read dispatch."""

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

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
    UcError,
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
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_bd09_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xBD09, 0xBD26),
        "sha256": "c4f057a1f1e81d9fb22d0a27a472bcb804659cf240fed7127de452fa2d5dc071",
        "mode": 0x0B9F,
        "helpers": {
            "snd_bank_ems_page_read": (0xBD26, 0xBD16),
            "snd_bank_xms_page_read": (0xBD4E, 0xBD1F),
            "snd_bank_file_page_read": (0xBD8D, 0xBD24),
        },
    },
    "sequel": {
        "routine": (0xD4B3, 0xD4D0),
        "sha256": "e1b4c838a59722eaac1cb93b813c5a320453940204d566af6c35c116308e257a",
        "mode": 0x0DA9,
        "helpers": {
            "snd_bank_ems_page_read": (0xD4D0, 0xD4C0),
            "snd_bank_xms_page_read": (0xD4F8, 0xD4C9),
            "snd_bank_file_page_read": (0xD537, 0xD4CE),
        },
    },
}

MACHINE_SIZE = 0xB0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x3000
DATA_SEGMENT = 0x2000
EXTRA_SEGMENT = 0x4800
FS_SEGMENT = 0x6000
STACK_SEGMENT = 0x7000
UNOWNED_SEGMENT = 0xA000
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("5aa596698778")

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


def seeded_segment(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 11 + seed * 29 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    edges = set()
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
        edges.add((source, int(immediate) - start))
        edges.add((source, instruction.address + instruction.size - start))
    return edges


def initial_registers(mode: int, row: dict[str, Any]) -> dict[int, int]:
    return {
        UC_X86_REG_EAX: 0xA1A10000 | int(row["value_ax"]),
        UC_X86_REG_EBX: 0xB2B2ABCD,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F60000 | int(row["destination_di"]),
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: FS_SEGMENT,
        UC_X86_REG_GS: GAME_SEGMENT,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202 | (mode & 1),
    }


def final_flags(machine: Uc) -> dict[str, bool]:
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    return {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}


def execute_case(
    executable: bytes,
    branch_name: str,
    mode: int,
    expected_row: dict[str, Any],
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    start, stop = branch["routine"]
    helpers = branch["helpers"]
    expected_name = str(expected_row["callee"])
    helper, helper_return = helpers[expected_name]
    initial = initial_registers(mode, expected_row)

    game = seeded_segment(mode + 1)
    data = seeded_segment(mode + 17)
    extra = seeded_segment(mode + 33)
    fs_data = seeded_segment(mode + 49)
    stack = seeded_segment(mode + 65)
    unowned = seeded_segment(mode + 81)
    game[int(branch["mode"])] = mode
    data[int(branch["mode"])] = (mode + 0x55) & 0xFF
    stack[STACK_POINTER : STACK_POINTER + 8] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    before = {
        GAME_SEGMENT: bytes(game),
        DATA_SEGMENT: bytes(data),
        EXTRA_SEGMENT: bytes(extra),
        FS_SEGMENT: bytes(fs_data),
        STACK_SEGMENT: bytes(stack),
        UNOWNED_SEGMENT: bytes(unowned),
    }

    patched = bytearray(executable)
    for address, _return_ip in helpers.values():
        patched[int(address)] = 0xC3
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(patched))
    for segment, contents in (
        (GAME_SEGMENT, game),
        (DATA_SEGMENT, data),
        (EXTRA_SEGMENT, extra),
        (FS_SEGMENT, fs_data),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    ):
        machine.mem_write(segment * 16, bytes(contents))
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
        if address in {int(item[0]) for item in helpers.values()}:
            assert cpu.reg_read(UC_X86_REG_CS) == 0
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEFC
            stack_words = struct.unpack(
                "<HHH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEFC, 6)
            )
            assert stack_words == (
                int(helper_return),
                initial[UC_X86_REG_EBX] & 0xFFFF,
                RETURN_IP,
            )
            helper_calls.append(
                {
                    "address": address,
                    "eax": cpu.reg_read(UC_X86_REG_EAX),
                    "ebx": cpu.reg_read(UC_X86_REG_EBX),
                    "ecx": cpu.reg_read(UC_X86_REG_ECX),
                    "edx": cpu.reg_read(UC_X86_REG_EDX),
                    "esi": cpu.reg_read(UC_X86_REG_ESI),
                    "edi": cpu.reg_read(UC_X86_REG_EDI),
                    "ebp": cpu.reg_read(UC_X86_REG_EBP),
                    "sp": cpu.reg_read(UC_X86_REG_SP),
                    "ds": cpu.reg_read(UC_X86_REG_DS),
                    "es": cpu.reg_read(UC_X86_REG_ES),
                    "fs": cpu.reg_read(UC_X86_REG_FS),
                    "gs": cpu.reg_read(UC_X86_REG_GS),
                    "ss": cpu.reg_read(UC_X86_REG_SS),
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
            f"mode {mode:#04x} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return

    expected_call = {
        "address": int(helper),
        "eax": initial[UC_X86_REG_EAX],
        "ebx": (initial[UC_X86_REG_EBX] & 0xFFFFFF00) | int(expected_row["bl_at_call"]),
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": 0xFEFC,
        "ds": DATA_SEGMENT,
        "es": EXTRA_SEGMENT,
        "fs": FS_SEGMENT,
        "gs": GAME_SEGMENT,
        "ss": STACK_SEGMENT,
    }
    assert helper_calls == [expected_call], (mode, branch_name, helper_calls)

    for segment, contents in before.items():
        if segment == STACK_SEGMENT:
            continue
        actual = bytes(machine.mem_read(segment * 16, SEGMENT_SIZE))
        assert actual == contents, (
            mode,
            branch_name,
            hex(segment),
            next(
                index
                for index, (left, right) in enumerate(zip(actual, contents))
                if left != right
            ),
        )
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched)
    expected_stack = bytearray(before[STACK_SEGMENT])
    struct.pack_into(
        "<HHH",
        expected_stack,
        0xFEFC,
        int(helper_return),
        initial[UC_X86_REG_EBX] & 0xFFFF,
        RETURN_IP,
    )
    assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
        expected_stack
    )
    assert all(
        STACK_SEGMENT * 16 + 0xFEFC <= address
        and address + size <= STACK_SEGMENT * 16 + 0xFF02
        for address, size in writes
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
        "ds": DATA_SEGMENT,
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
    flags = final_flags(machine)
    assert flags == expected_row["defined_flags"]

    row = {
        "mode": mode,
        "callee": expected_name,
        "callee_address": int(helper),
        "value_ax": int(expected_row["value_ax"]),
        "destination_es": EXTRA_SEGMENT,
        "destination_di": int(expected_row["destination_di"]),
        "bl_at_call": int(expected_row["bl_at_call"]),
        "defined_flags": flags,
    }
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

    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(expected_rows) == 256
    coverage = {branch_name: set() for branch_name in BRANCHES}
    rows = []
    for mode, expected_row in enumerate(expected_rows):
        assert int(expected_row["mode"]) == mode
        commander_row, commander_edges = execute_case(
            commander, "commander", mode, expected_row
        )
        sequel_row, sequel_edges = execute_case(sequel, "sequel", mode, expected_row)
        assert commander_row == expected_row
        assert {
            key: value
            for key, value in commander_row.items()
            if key != "callee_address"
        } == {
            key: value for key, value in sequel_row.items() if key != "callee_address"
        }
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        assert hashlib.sha256(body).hexdigest() == branch["sha256"]
        decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
        assert len(list(decoder.disasm(body, start))) == 13
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual page-dispatch modes and "
        f"{len(coverage['sequel'])} BBB conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} sites"
    )


if __name__ == "__main__":
    main()
