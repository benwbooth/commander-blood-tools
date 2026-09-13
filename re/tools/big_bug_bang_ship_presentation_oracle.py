#!/usr/bin/env python3
"""Compare BBB's ship presentation FSM with Commander Blood."""

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
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

MACHINE_SIZE = 0x90000
SEGMENT_SIZE = 0x10000
DATA = 0x30000
GAME = 0x50000
STACK = 0x70000
STACK_POINTER = 0xF800
STACK_TRANSIENT_START = 0xF7F0
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
TAIL_AX = 0x3333
TAIL_DX = 0x4444

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)
COMMANDER_FIXTURE_SHA256 = (
    "e055811e9922a01d7884de39f620a8fcd2b36c5148a9a805751246c93a064d46"
)
FLAG_MASKS = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
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

FIELD_WIDTHS = {
    0x1FB2: 1,
    0x24F3: 2,
    0x24F5: 2,
    0x2527: 2,
    0x252D: 1,
    0x252F: 1,
    0x2530: 1,
    0x2534: 1,
    0x2535: 1,
    0x2793: 2,
    0x27D8: 1,
    0x5249: 2,
    0x524F: 2,
    0x6788: 2,
}


@dataclass(frozen=True)
class Helper:
    name: str
    address: int
    return_kind: str
    return_ips: tuple[int, ...]
    return_cs: int


@dataclass(frozen=True)
class Routine:
    name: str
    span: tuple[int, int]
    body_sha256: str
    fields: dict[int, int]
    helpers: tuple[Helper, ...]


COMMANDER = Routine(
    "Commander Blood",
    (0xAFA0, 0xB079),
    "097da2c66843f677d4d07cd154d36336f0a000e7fac1d2d575eb53e9c7bcfa34",
    {offset: offset for offset in FIELD_WIDTHS},
    (
        Helper("entity", 0x3BD1, "far", (0xAFBE, 0xAFC6), 0),
        Helper("depth", 0xB75C, "near", (0xAFED,), 0),
        Helper("band", 0xB6DD, "far", (0xAFF1,), 0),
        Helper("dispatch", 0x9710, "far", (0xAFF6,), 0),
        Helper("hud", 0xB079, "near", (0xB03F,), 0),
        Helper("fill", 0x377B, "far", (0xB059,), 0),
        Helper("nav", 0xB34E, "near", (0xB076,), 0),
    ),
)
SEQUEL = Routine(
    "Big Bug Bang",
    (0xC780, 0xC859),
    "8f67ffbdc0d60ed8bdcab4b9dd0ac76c74f5e008af94c3ef1053c66aaecd44ec",
    {
        0x1FB2: 0x2200,
        0x24F3: 0x2745,
        0x24F5: 0x2747,
        0x2527: 0x2779,
        0x252D: 0x277F,
        0x252F: 0x2781,
        0x2530: 0x2782,
        0x2534: 0x2786,
        0x2535: 0x2787,
        0x2793: 0x2A33,
        0x27D8: 0x2A78,
        0x5249: 0x5619,
        0x524F: 0x561F,
        0x6788: 0x6B5A,
    },
    (
        Helper("entity", 0x3E4E, "far", (0xC79E, 0xC7A6), 0),
        Helper("depth", 0xCEE9, "near", (0xC7CD,), 0),
        Helper("band", 0xCE6A, "far", (0xC7D1,), 0),
        Helper("dispatch", 0xACB0, "far", (0xC7D6,), 0),
        Helper("hud", 0xC859, "near", (0xC81F,), 0),
        Helper("fill", 0x39F8, "far", (0xC839,), 0),
        Helper("nav", 0xCB0F, "near", (0xC856,), 0),
    ),
)


def patterned_segment(
    case_index: int, step: int, page_step: int, case_step: int, base: int
) -> bytearray:
    return bytearray(
        (offset * step + (offset >> 8) * page_step + case_index * case_step + base)
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_value(data: bytearray, offset: int, width: int, value: int) -> None:
    if width == 1:
        data[offset] = value & 0xFF
    else:
        struct.pack_into("<H", data, offset, value & 0xFFFF)


def read_value(data: bytes | bytearray, offset: int, width: int) -> int:
    return data[offset] if width == 1 else struct.unpack_from("<H", data, offset)[0]


def near_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    ip = struct.unpack("<H", machine.mem_read(STACK + sp, 2))[0]
    machine.reg_write(UC_X86_REG_SP, (sp + 2) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, ip)


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    ip, cs = struct.unpack("<HH", machine.mem_read(STACK + sp, 4))
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, ip)
    machine.reg_write(UC_X86_REG_CS, cs)


def is_conditional_jump(machine: Uc, address: int) -> bool:
    encoded = bytes(machine.mem_read(address, 2))
    return 0x70 <= encoded[0] <= 0x7F or (
        encoded[0] == 0x0F and 0x80 <= encoded[1] <= 0x8F
    )


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[slice(*routine.span)]).hexdigest()
    if body_digest != routine.body_sha256:
        raise SystemExit(
            f"{routine.name} ship presentation body changed: {body_digest}"
        )
    return executable


def load_cases(path: Path) -> list[dict[str, Any]]:
    encoded = path.read_bytes()
    digest = hashlib.sha256(encoded).hexdigest()
    if digest != COMMANDER_FIXTURE_SHA256:
        raise SystemExit(f"Commander ship presentation fixture changed: {digest}")
    cases = json.loads(encoded)
    if len(cases) != 20:
        raise SystemExit(f"expected 20 Commander FSM cases, found {len(cases)}")
    return cases


def initialize_data(
    routine: Routine, vector: dict[str, Any], case_index: int
) -> tuple[bytearray, bytearray]:
    base = patterned_segment(case_index, 7, 11, 13, 0x21)
    data = base[:]
    values = {
        0x1FB2: int(vector["presentation_gate"]),
        0x24F3: int(vector["state_before"]),
        0x24F5: int(vector["dialogue_cycle_before"]),
        0x2527: 0x1357,
        0x252D: 0x7D,
        0x252F: 0x5F,
        0x2530: 0x60,
        0x2534: int(vector["ready"]),
        0x2535: int(vector["hud_pending"]),
        0x2793: 0xBEEF,
        0x27D8: int(vector["redraw"]),
        0x5249: 0xA55A,
        0x524F: int(vector["transition_percent"]),
        0x6788: 0x7777,
    }
    for canonical, value in values.items():
        write_value(data, routine.fields[canonical], FIELD_WIDTHS[canonical], value)
    return base, data


def initialize_game(
    routine: Routine, vector: dict[str, Any], case_index: int
) -> bytearray:
    game = patterned_segment(case_index, 17, 19, 23, 0x43)
    decoys = {
        0x1FB2: int(vector["presentation_gate"]) ^ 0x88,
        0x24F3: int(vector["state_before"]) ^ 0xFFFF,
        0x24F5: int(vector["dialogue_cycle_before"]) ^ 0xFFFF,
        0x2527: 0xECA8,
        0x252D: 0xD2,
        0x252F: 0xD0,
        0x2530: 0xCF,
        0x2534: int(vector["ready"]) ^ 0x81,
        0x2535: int(vector["hud_pending"]) ^ 0x82,
        0x2793: 0x4110,
        0x27D8: int(vector["redraw"]) ^ 0x84,
        0x5249: 0x5AA5,
        0x524F: int(vector["transition_percent"]) ^ 0xFFFF,
        0x6788: 0x8888,
    }
    for canonical, value in decoys.items():
        write_value(game, routine.fields[canonical], FIELD_WIDTHS[canonical], value)
    return game


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, Any],
    case_index: int,
) -> tuple[tuple[object, ...], set[tuple[int, int]]]:
    name = str(vector["name"])
    data_base, data_before = initialize_data(routine, vector, case_index)
    game_before = initialize_game(routine, vector, case_index)
    data_expected = data_before[:]
    for canonical, width, value in vector["writes"]:
        write_value(
            data_expected,
            routine.fields[int(canonical)],
            int(width),
            int(value),
        )

    stack_before = patterned_segment(case_index, 29, 31, 37, 0x65)
    struct.pack_into("<HH", stack_before, STACK_POINTER, RETURN_IP, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    initial = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: 0x1000,
        UC_X86_REG_FS: 0x9000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(GAME, bytes(game_before))
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    helper_by_address = {helper.address: helper for helper in routine.helpers}
    helper_counts = {helper.name: 0 for helper in routine.helpers}
    calls: list[tuple[str, int, int]] = []
    reached_return = False
    previous_branch: int | None = None
    covered_edges: set[tuple[int, int]] = set()

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal reached_return, previous_branch
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        helper = helper_by_address.get(address)
        if helper is not None:
            occurrence = helper_counts[helper.name]
            helper_counts[helper.name] += 1
            assert occurrence < len(helper.return_ips), (
                routine.name,
                name,
                helper.name,
                "duplicate call",
            )
            sp = cpu.reg_read(UC_X86_REG_SP)
            ax = cpu.reg_read(UC_X86_REG_AX)
            return_ip = struct.unpack("<H", cpu.mem_read(STACK + sp, 2))[0]
            assert return_ip == helper.return_ips[occurrence], (
                routine.name,
                name,
                helper.name,
                "return IP",
            )
            if helper.return_kind == "far":
                return_cs = struct.unpack("<H", cpu.mem_read(STACK + sp + 2, 2))[0]
                assert return_cs == helper.return_cs, (
                    routine.name,
                    name,
                    helper.name,
                    "return CS",
                )
            if helper.name == "dispatch":
                assert cpu.reg_read(UC_X86_REG_BP) == initial[UC_X86_REG_EBP] & 0xFFFF
            calls.append(
                (helper.name if helper.name != "entity" else f"entity:{ax}", ax, sp)
            )
            if helper.name in ("hud", "fill", "nav"):
                cpu.reg_write(UC_X86_REG_AX, TAIL_AX)
                cpu.reg_write(
                    UC_X86_REG_EDX,
                    (cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF0000) | TAIL_DX,
                )
                cpu.reg_write(UC_X86_REG_DS, 0x1111)
                cpu.reg_write(UC_X86_REG_SI, 0x2222)
            if helper.return_kind == "far":
                far_return(cpu)
            else:
                near_return(cpu)
            previous_branch = None
            return
        assert routine.span[0] <= address < routine.span[1], (
            routine.name,
            name,
            f"escaped to {address:#x}",
        )
        normalized = address - routine.span[0]
        if previous_branch is not None:
            covered_edges.add((previous_branch, normalized))
        previous_branch = normalized if is_conditional_jump(cpu, address) else None

    machine.hook_add(UC_HOOK_CODE, instruction)
    try:
        machine.emu_start(routine.span[0], 0, count=2_000)
    except UcError as error:
        raise RuntimeError(
            f"{routine.name} {name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (routine.name, name, "far return")

    expected_calls = [
        (str(call["name"]), int(call["ax"]), int(call["sp"]))
        for call in vector["calls"]
    ]
    assert calls == expected_calls, (routine.name, name, calls, expected_calls)
    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    game_after = bytes(machine.mem_read(GAME, SEGMENT_SIZE))
    assert data_after == bytes(data_expected), (routine.name, name, "data")
    assert game_after == bytes(game_before), (routine.name, name, "GS decoy")
    assert bytes(machine.mem_read(0, len(executable))) == executable, (
        routine.name,
        name,
        "executable",
    )

    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert stack_after[:STACK_TRANSIENT_START] == bytes(
        stack_before[:STACK_TRANSIENT_START]
    ), (routine.name, name, "stack prefix")
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:]), (
        routine.name,
        name,
        "stack caller frame",
    )
    expected_memory = bytearray(MACHINE_SIZE)
    expected_memory[: len(executable)] = executable
    expected_memory[DATA : DATA + SEGMENT_SIZE] = data_expected
    expected_memory[GAME : GAME + SEGMENT_SIZE] = game_before
    expected_memory[STACK : STACK + SEGMENT_SIZE] = stack_before
    memory_after = bytes(machine.mem_read(0, MACHINE_SIZE))
    transient_start = STACK + STACK_TRANSIENT_START
    transient_end = STACK + STACK_POINTER
    assert memory_after[:transient_start] == bytes(expected_memory[:transient_start]), (
        routine.name,
        name,
        "unowned prefix",
    )
    assert memory_after[transient_end:] == bytes(expected_memory[transient_end:]), (
        routine.name,
        name,
        "unowned suffix",
    )

    expected_registers = {
        key: int(value) for key, value in vector["registers_after"].items()
    }
    expected_registers["sp"] = STACK_POINTER + 4
    registers = {
        name: machine.reg_read(register) for name, register in REGISTERS.items()
    }
    assert registers == expected_registers, (routine.name, name, "registers")
    assert machine.reg_read(UC_X86_REG_CS) == 0
    assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(flags & FLAG_MASKS[flag]) for flag in vector["defined_flags"]
    }
    assert defined_flags == vector["defined_flags"], (routine.name, name, "flags")

    normalized_data = bytearray(data_after)
    semantic_values = []
    for canonical, width in FIELD_WIDTHS.items():
        actual = routine.fields[canonical]
        semantic_values.append((canonical, read_value(data_after, actual, width)))
        normalized_data[actual : actual + width] = data_base[actual : actual + width]
    canonical = (
        tuple(registers.items()),
        tuple(sorted(defined_flags.items())),
        tuple(semantic_values),
        bytes(normalized_data),
        calls,
        bytes(machine.mem_read(STACK, STACK_TRANSIENT_START)),
        bytes(machine.mem_read(STACK + STACK_POINTER, SEGMENT_SIZE - STACK_POINTER)),
    )
    return canonical, covered_edges


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-executable",
        type=Path,
        default=Path(__file__).parents[1] / "bin/BLOODPRG.EXE",
    )
    parser.add_argument(
        "--commander-fixture",
        type=Path,
        default=Path(__file__).with_name("oracle_vectors") / "func_afa0_natural.json",
    )
    args = parser.parse_args()

    commander = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel = verify_executable(args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL)
    cases = load_cases(args.commander_fixture)
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(cases):
        commander_result, edges = execute(commander, COMMANDER, vector, case_index)
        commander_edges.update(edges)
        sequel_result, edges = execute(sequel, SEQUEL, vector, case_index)
        sequel_edges.update(edges)
        assert sequel_result == commander_result, vector["name"]
        rows.append(vector)
    assert sequel_edges == commander_edges, (
        sorted(sequel_edges),
        sorted(commander_edges),
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    branch_sites = {edge[0] for edge in sequel_edges}
    complete_sites = sum(
        sum(1 for edge in sequel_edges if edge[0] == site) == 2 for site in branch_sites
    )
    print(
        f"verified {len(rows)} BBB ship presentation cases, "
        f"{len(sequel_edges)} edges, and {complete_sites}/{len(branch_sites)} "
        "observed branch sites with both outcomes"
    )


if __name__ == "__main__":
    main()
