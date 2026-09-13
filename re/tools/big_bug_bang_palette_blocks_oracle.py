#!/usr/bin/env python3
"""Compare Big Bug Bang's palette-block applier with Commander Blood."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
STREAM = 0x40000
STATE = 0x60000
EXTRA = 0x70000
FS_DATA = 0x80000
STACK = 0x90000
STACK_POINTER = 0xFF00
STREAM_OFFSET = 0x1000
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

COMMANDER_EXECUTABLE_SHA256 = (
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
)
SEQUEL_EXECUTABLE_SHA256 = (
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
)


@dataclass(frozen=True)
class Routine:
    name: str
    header_size: int
    span: tuple[int, int]
    body_sha256: str
    snapshot_span: tuple[int, int]
    snapshot_sha256: str
    live_palette: int
    render_state: int
    dirty: int
    render_update_flags: int
    wrap_index: int
    metric: int
    branches: dict[int, tuple[int, int]]

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0xA0C3, 0xA117),
    body_sha256="4721d1394bf610e0c221c0ec8a92ff143d65247d3275a157c4bde8906f0918ef",
    snapshot_span=(0xA117, 0xA134),
    snapshot_sha256="6cd00db04e9af49e2284c0a02160bf35661d5bd7051e0c780f6e58aafc8d0489",
    live_palette=0x5251,
    render_state=0x5851,
    dirty=0x5B55,
    render_update_flags=0x2751,
    wrap_index=0x0D60,
    metric=0x0DAF,
    branches={
        0xA0D7: (0xA0D9, 0xA0EE),
        0xA0F7: (0xA0F9, 0xA10E),
        0xA11F: (0xA121, 0xA131),
    },
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB8A6, 0xB8FA),
    body_sha256="8d515dce5b9b722d3aa3140cdcd964c5e7a3acd6c31baa1207fc30b25f50fe3f",
    snapshot_span=(0xB8FA, 0xB917),
    snapshot_sha256="4ffd715c656d5ca9165ca68b63380fb792c4895adb0fc4ecfd9d1e92204cac03",
    live_palette=0x5621,
    render_state=0x5C21,
    dirty=0x5F25,
    render_update_flags=0x29DF,
    wrap_index=0x0FAE,
    metric=0x0FFD,
    branches={
        0xB8BA: (0xB8BC, 0xB8D1),
        0xB8DA: (0xB8DC, 0xB8F1),
        0xB902: (0xB904, 0xB914),
    },
)

REGISTERS = (
    UC_X86_REG_EAX,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDX,
    UC_X86_REG_ESI,
    UC_X86_REG_EDI,
    UC_X86_REG_EBP,
    UC_X86_REG_SP,
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_ES,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SS,
)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def read16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def palette_stream(vector: dict[str, object]) -> bytes:
    stream = bytearray()
    for block in vector["blocks"]:
        payload = bytes(block["payload"])
        assert len(payload) % 3 == 0
        stream.extend((int(block["start"]), len(payload) // 3))
        stream.extend(payload)
    stream.extend(b"\xff\xff")
    assert len(stream) == vector["consumed_bytes"]
    return bytes(stream)


def expected_palette(vector: dict[str, object], initial: bytes) -> bytes:
    expected = bytearray(initial)
    for block in vector["blocks"]:
        payload = bytes(block["payload"])
        destination = int(block["start"]) * 3
        expected[destination : destination + len(payload)] = payload
    return bytes(expected)


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
) -> None:
    for offset, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or any(start <= offset < end for start, end in allowed):
            continue
        raise AssertionError(f"{label} changed outside owned state at {offset:#x}")


def normalized_edges(routine: Routine) -> set[tuple[int, int]]:
    return {
        (source - routine.span[0], destination - routine.span[0])
        for source, destinations in routine.branches.items()
        for destination in destinations
    }


def normalized_stack(routine: Routine, stack: bytes) -> bytes:
    normalized = bytearray(stack)
    if routine is SEQUEL:
        sequel_return = SEQUEL.image_address(0xB8D4)
        commander_return = COMMANDER.image_address(0xA0F1)
        for offset in range(0xFEC0, STACK_POINTER, 2):
            if read16(normalized, offset) == sequel_return:
                write16(normalized, offset, commander_return)
    return bytes(normalized)


def canonical_registers(routine: Routine, machine: Uc) -> dict[int, int]:
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    edi = registers[UC_X86_REG_EDI]
    di = edi & 0xFFFF
    for source, target, size in (
        (routine.live_palette, COMMANDER.live_palette, 768),
        (routine.render_state, COMMANDER.render_state, 0x180),
    ):
        if source <= di <= source + size:
            registers[UC_X86_REG_EDI] = (edi & 0xFFFF0000) | (target + di - source)
            break
    return registers


def final_flags(flags: int) -> dict[str, int]:
    return {
        "carry": (flags >> 0) & 1,
        "parity": (flags >> 2) & 1,
        "zero": (flags >> 6) & 1,
        "sign": (flags >> 7) & 1,
        "overflow": (flags >> 11) & 1,
        "interrupt": (flags >> 9) & 1,
        "direction": (flags >> 10) & 1,
    }


def verify_span(
    executable: bytes, span: tuple[int, int], expected_sha256: str, label: str
) -> None:
    digest = hashlib.sha256(executable[slice(*span)]).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"{label} span {span[0]:#x}..{span[1]:#x} changed: {digest}")


def verify_executable(path: Path, expected_sha256: str, routine: Routine) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != expected_sha256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    verify_span(executable, routine.span, routine.body_sha256, f"{routine.name} body")
    verify_span(
        executable,
        routine.snapshot_span,
        routine.snapshot_sha256,
        f"{routine.name} snapshot",
    )
    assert executable[routine.span[1] - 1] == 0xC3
    assert executable[routine.snapshot_span[1] - 1] == 0xC3
    return executable


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(vector["name"])
    stream = palette_stream(vector)
    initial_palette = bytes((index * 17 + 5) & 0xFF for index in range(768))
    result_palette = expected_palette(vector, initial_palette)
    initial_render = bytes((index * 29 + 7) & 0xFF for index in range(0x180))
    result_render = (
        result_palette[:0x180] if vector["copied_render_state"] else initial_render
    )

    data_before = seeded_segment(case_index, 17, 0x31)
    data_before[routine.live_palette : routine.live_palette + 768] = initial_palette
    data_before[routine.render_state : routine.render_state + 0x180] = initial_render
    data_before[STREAM_OFFSET : STREAM_OFFSET + len(stream)] = bytes(
        (index * 13 + 3) & 0xFF for index in range(len(stream))
    )
    stream_before = seeded_segment(case_index, 11, 0x53)
    stream_before[STREAM_OFFSET : STREAM_OFFSET + len(stream)] = stream
    stream_before[routine.live_palette : routine.live_palette + 768] = bytes(
        (index * 31 + 11) & 0xFF for index in range(768)
    )
    state_before = seeded_segment(case_index, 7, 0x75)
    state_before[routine.render_update_flags] = int(
        not bool(vector["copied_render_state"])
    )
    write16(state_before, routine.wrap_index, int(vector["initial_wrap_index"]))
    write16(state_before, routine.metric, int(vector["initial_metric"]))
    state_before[routine.dirty] = 0xA5
    extra_before = bytes(seeded_segment(case_index, 5, 0x97))
    fs_before = bytes(seeded_segment(case_index, 3, 0xB1))
    stack_before = seeded_segment(case_index, 19, 0xD3)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    initial = {
        UC_X86_REG_EAX: 0xA1A11111,
        UC_X86_REG_EBX: 0xB2B22222,
        UC_X86_REG_ECX: 0xC3C33333,
        UC_X86_REG_EDX: 0xD4D44444,
        UC_X86_REG_ESI: 0xE5E50000 | STREAM_OFFSET,
        UC_X86_REG_EDI: 0xF6F66666,
        UC_X86_REG_EBP: 0x97977777,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: STREAM // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: STATE // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0293,
    }

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xA0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    machine.mem_write(DATA, bytes(data_before))
    machine.mem_write(STREAM, bytes(stream_before))
    machine.mem_write(STATE, bytes(state_before))
    machine.mem_write(EXTRA, extra_before)
    machine.mem_write(FS_DATA, fs_before)
    machine.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        machine.reg_write(register, value)

    reached_return = False
    pending_branch: int | None = None

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal pending_branch, reached_return
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        file_offset = address + routine.header_size
        if pending_branch is not None:
            covered_edges.add(
                (pending_branch - routine.span[0], file_offset - routine.span[0])
            )
            pending_branch = None
        if file_offset in routine.branches:
            pending_branch = file_offset

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(routine.image_address(routine.span[0]), 0, count=10000)
    assert reached_return, (routine.name, name)

    data_after = bytes(machine.mem_read(DATA, SEGMENT_SIZE))
    stream_after = bytes(machine.mem_read(STREAM, SEGMENT_SIZE))
    state_after = bytes(machine.mem_read(STATE, SEGMENT_SIZE))
    stack_after = bytes(machine.mem_read(STACK, SEGMENT_SIZE))
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)
    assert stream_after == bytes(stream_before)
    assert bytes(machine.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before
    assert bytes(machine.mem_read(FS_DATA, SEGMENT_SIZE)) == fs_before
    assert stack_after[STACK_POINTER:] == bytes(stack_before[STACK_POINTER:])
    assert_unchanged_outside(
        bytes(data_before),
        data_after,
        [
            (routine.live_palette, routine.live_palette + 768),
            (routine.render_state, routine.render_state + 0x180),
        ],
        f"{routine.name} {name} data",
    )
    assert_unchanged_outside(
        bytes(state_before),
        state_after,
        [(routine.dirty, routine.dirty + 1), (routine.metric, routine.metric + 2)],
        f"{routine.name} {name} state",
    )
    assert (
        data_after[routine.live_palette : routine.live_palette + 768] == result_palette
    )
    assert (
        data_after[routine.render_state : routine.render_state + 0x180] == result_render
    )
    assert state_after[routine.dirty] == 1
    assert read16(state_after, routine.metric) == vector["result_metric"]

    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    canonical_di = canonical_registers(routine, machine)[UC_X86_REG_EDI] & 0xFFFF
    row = {
        "name": name,
        "blocks": vector["blocks"],
        "consumed_bytes": len(stream),
        "copied_render_state": bool(vector["copied_render_state"]),
        "initial_wrap_index": int(vector["initial_wrap_index"]),
        "initial_metric": int(vector["initial_metric"]),
        "result_metric": read16(state_after, routine.metric),
        "result_stream_offset": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        "result_di": canonical_di,
        "final_flags": final_flags(flags),
    }
    canonical_state = bytes(
        [state_after[routine.dirty]]
        + list(state_after[routine.metric : routine.metric + 2])
        + list(data_after[routine.live_palette : routine.live_palette + 768])
        + list(data_after[routine.render_state : routine.render_state + 0x180])
    )
    return (
        row,
        canonical_registers(routine, machine),
        flags,
        canonical_state,
        normalized_stack(routine, stack_after),
    )


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
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_a0c3_natural.json",
    )
    args = parser.parse_args()

    commander_executable = verify_executable(
        args.commander_executable, COMMANDER_EXECUTABLE_SHA256, COMMANDER
    )
    sequel_executable = verify_executable(
        args.executable, SEQUEL_EXECUTABLE_SHA256, SEQUEL
    )
    vectors = json.loads(args.commander_vectors.read_text())
    commander_edges: set[tuple[int, int]] = set()
    sequel_edges: set[tuple[int, int]] = set()
    rows = []
    for case_index, vector in enumerate(vectors):
        commander_result = execute(
            commander_executable, COMMANDER, vector, case_index, commander_edges
        )
        sequel_result = execute(
            sequel_executable, SEQUEL, vector, case_index, sequel_edges
        )
        (
            commander_row,
            commander_registers,
            commander_flags,
            commander_state,
            commander_stack,
        ) = commander_result
        sequel_row, sequel_registers, sequel_flags, sequel_state, sequel_stack = (
            sequel_result
        )
        assert commander_row == vector, (vector["name"], commander_row, vector)
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        assert sequel_flags == commander_flags, vector["name"]
        assert sequel_state == commander_state, vector["name"]
        assert sequel_stack == commander_stack, vector["name"]
        rows.append(sequel_row)

    assert commander_edges == normalized_edges(COMMANDER)
    assert sequel_edges == normalized_edges(SEQUEL)
    assert sequel_edges == commander_edges
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB palette-block cases covering "
        f"{len(sequel_edges)} normalized conditional edges"
    )


if __name__ == "__main__":
    main()
