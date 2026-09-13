#!/usr/bin/env python3
"""Compare Big Bug Bang's ship-object projector with Commander Blood."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import hashlib
import json
import struct
from dataclasses import dataclass
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

SEGMENT_SIZE = 0x10000
DATA = 0x20000
GAME = 0x40000
EXTRA = 0x50000
COMPARE = 0x68000
FRAME = 0x78000
FS_DATA = 0xB0000
STACK = 0xD0000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
COMPARE_OFFSET = 0x0200
ENTITY_COUNT = 11
ENTITY_SIZE = 32
ANCHOR_SIZE = 6
WORK_SIZE = 8

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
    anchors: int
    work: int
    matrix: int
    counter: int
    camera: int
    entity_base: int
    extent_file_span: tuple[int, int]
    extent_sha256: str
    extent_cs: int
    extent_ip: int
    position_file_span: tuple[int, int]
    position_sha256: str
    position_cs: int
    position_ip: int
    extent_return: int
    position_return: int

    def image_address(self, file_offset: int) -> int:
        return file_offset - self.header_size

    @property
    def extent_address(self) -> int:
        return self.extent_cs * 16 + self.extent_ip

    @property
    def position_address(self) -> int:
        return self.position_cs * 16 + self.position_ip


COMMANDER = Routine(
    name="Commander Blood",
    header_size=0x600,
    span=(0x9B98, 0x9D09),
    body_sha256="4c1e816863fe14d2f7835c89e6d0692195f55abacb012b3c7db8e1338a165051",
    anchors=0x4F09,
    work=0x4F01,
    matrix=0x2F95,
    counter=0x2F77,
    camera=0x2F65,
    entity_base=0x6212,
    extent_file_span=(0x42CD, 0x4316),
    extent_sha256="463f6e4fbb383556b88c63c4c2bc5d4cc6a37ee1694dc2a29da80b8395dfb225",
    extent_cs=0x0299,
    extent_ip=0x133D,
    position_file_span=(0x420D, 0x4240),
    position_sha256="b06aba7a8862bde0678fdb2cab5e6e25126e627df1a0c73bf442be4dd583b6df",
    position_cs=0x0299,
    position_ip=0x127D,
    extent_return=0x9CDB,
    position_return=0x9CF4,
)

SEQUEL = Routine(
    name="Big Bug Bang",
    header_size=0x800,
    span=(0xB337, 0xB4A8),
    body_sha256="ec35192c3845ed01c20b31410d69fd26704ae240ca5edf3f9d893f0f0e4ed471",
    anchors=0x52D9,
    work=0x52D1,
    matrix=0x3365,
    counter=0x3347,
    camera=0x3335,
    entity_base=0x65E2,
    extent_file_span=(0x474A, 0x4793),
    extent_sha256="8800d6eccc79fb4f9730b0f0ed40597887978be63f71f989254dec0d179f637e",
    extent_cs=0x02B1,
    extent_ip=0x143A,
    position_file_span=(0x468A, 0x46BD),
    position_sha256="510eb1fde1e73978056e0b0f6f1efc145c3a31c1957a964eceeda421581f1dbb",
    position_cs=0x02B1,
    position_ip=0x137A,
    extent_return=0xB47A,
    position_return=0xB493,
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

CASES = (
    ("all_visible_positive", 0x11, False, False, False),
    ("mixed_hidden_and_zero_depth", 0x2B, True, True, False),
    ("negative_depth_wrap", 0x45, False, False, True),
    ("source_equal_extent_flag_clear", 0x5F, False, False, False),
    ("modular_overflow_and_screen_wrap", 0x79, True, False, False),
)

BRANCHES = {
    0x26: (0x163, 0x2A),
    0x4C: (0x15C, 0x50),
    0x8B: (0x15C, 0x8F),
    0x8F: (0x98, 0x91),
}


def signed32(value: int) -> int:
    value &= 0xFFFFFFFF
    return value - 0x100000000 if value & 0x80000000 else value


def seeded_segment(
    seed: int, multiplier: int, high_multiplier: int, salt: int
) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * high_multiplier + seed + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def make_inputs(
    routine: Routine, case_index: int
) -> tuple[dict[int, int], dict[int, bytearray], list[int]]:
    name, seed, mixed_visibility, zero_depths, negative_depths = CASES[case_index]
    compare_dimensions = (32 + case_index * 3, 24 + case_index * 5)
    matrix = [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000]
    matrix[1] = signed32(COMPARE_OFFSET | ((COMPARE // 16) << 16))
    if name == "modular_overflow_and_screen_wrap":
        matrix[0] = 0x70000001
        matrix[2] = -0x3456789
        matrix[3] = 0x6ABCDEFF
        matrix[4] = -0x1234567
    camera = [0x0100 + seed, 0x8200, 0xFF00]

    anchors = []
    for anchor_index in range(ENTITY_COUNT):
        x = (0x1200 + seed * 17 + anchor_index * 977) & 0xFFFF
        y = (0x4300 + seed * 11 + anchor_index * 613) & 0xFFFF
        if negative_depths:
            z = (camera[2] - 1000 - anchor_index * 17) & 0xFFFF
        elif zero_depths and anchor_index in (2, 7):
            z = camera[2]
        else:
            z = (camera[2] + 900 + anchor_index * 37) & 0xFFFF
        anchors.append([x, y, z])
    anchors[-1] = [camera[0], camera[1], (camera[2] + 1024) & 0xFFFF]
    anchor_bytes = b"".join(struct.pack("<3H", *anchor) for anchor in anchors)
    trailing_anchor_word = (0xA500 + seed) & 0xFFFF
    anchor_window = anchor_bytes + struct.pack("<H", trailing_anchor_word)

    game = seeded_segment(seed, 29, 13, 0)
    game[routine.anchors : routine.anchors + len(anchor_window)] = anchor_window
    struct.pack_into("<3H", game, routine.camera, *camera)
    frame = seeded_segment(seed, 19, 7, 0x31)
    for entity_id in range(21, 32):
        record_offset = routine.entity_base + entity_id * ENTITY_SIZE
        anchor_index = 31 - entity_id
        visible = not mixed_visibility or anchor_index % 3 != 1
        flags = (0xAB00 | 0x0001 | (0x0080 if visible else 0)) & 0xFFFF
        if name == "source_equal_extent_flag_clear":
            flags |= 0x0010
        source_offset = 0x1000 + entity_id * 8
        source_dimensions = (
            compare_dimensions
            if name == "source_equal_extent_flag_clear"
            else (17 + entity_id * 2, 13 + entity_id * 3)
        )
        struct.pack_into("<2H", frame, source_offset, *source_dimensions)
        struct.pack_into("<H", game, record_offset, flags)
        struct.pack_into("<HH", game, record_offset + 4, source_offset, FRAME // 16)
        struct.pack_into(
            "<4H",
            game,
            record_offset + 8,
            (0x7000 + entity_id * 13) & 0xFFFF,
            (0x8000 + entity_id * 17) & 0xFFFF,
            (9 + entity_id) & 0xFFFF,
            (11 + entity_id) & 0xFFFF,
        )

    compare = seeded_segment(seed, 7, 17, 0x97)
    struct.pack_into("<2H", compare, COMPARE_OFFSET, *compare_dimensions)
    stack = seeded_segment(seed, 5, 23, 0xB9)
    struct.pack_into("<9i", stack, routine.matrix, *matrix)
    stack[STACK_POINTER : STACK_POINTER + 4] = struct.pack("<HH", RETURN_IP, 0)
    stack[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    segments = {
        DATA: seeded_segment(seed, 17, 5, 0x53),
        GAME: game,
        EXTRA: seeded_segment(seed, 11, 3, 0x75),
        COMPARE: compare,
        FRAME: frame,
        FS_DATA: seeded_segment(seed, 13, 9, 0xA7),
        STACK: stack,
    }
    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E55678 + case_index,
        UC_X86_REG_EDI: 0xF6F66789 + case_index,
        UC_X86_REG_EBP: 0x9797789A + case_index,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: FS_DATA // 16,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    return initial, segments, matrix


def assert_unchanged_outside(
    before: bytes, after: bytes, allowed: list[tuple[int, int]], label: str
) -> None:
    for index, (old, new) in enumerate(zip(before, after, strict=True)):
        if old == new or any(start <= index < end for start, end in allowed):
            continue
        raise AssertionError(f"{label} changed outside owned state at {index:#x}")


def semantic_game(routine: Routine, game: bytes) -> bytes:
    fields = bytearray()
    fields.extend(game[routine.counter : routine.counter + 2])
    fields.extend(game[routine.camera : routine.camera + 6])
    fields.extend(game[routine.work : routine.work + WORK_SIZE])
    fields.extend(game[routine.anchors : routine.anchors + ENTITY_COUNT * ANCHOR_SIZE + 2])
    for entity_id in range(21, 32):
        offset = routine.entity_base + entity_id * ENTITY_SIZE
        fields.extend(game[offset : offset + 2])
        fields.extend(game[offset + 4 : offset + 16])
    return bytes(fields)


def normalized_stack(routine: Routine, actual: bytes, initial: bytes) -> bytes:
    normalized = bytearray(actual)
    matrix_state = actual[routine.matrix : routine.matrix + 44]
    del initial
    normalized[COMMANDER.matrix : COMMANDER.matrix + 44] = bytes(44)
    normalized[SEQUEL.matrix : SEQUEL.matrix + 44] = bytes(44)
    normalized[COMMANDER.matrix : COMMANDER.matrix + 44] = matrix_state

    entry = routine.image_address(routine.span[0])
    end = routine.image_address(routine.span[1])
    commander_entry = COMMANDER.image_address(COMMANDER.span[0])
    for offset in range(0xFEC0, STACK_POINTER, 2):
        word = struct.unpack_from("<H", normalized, offset)[0]
        replacement = None
        if word == routine.matrix:
            replacement = COMMANDER.matrix
        elif word == routine.work:
            replacement = COMMANDER.work
        elif routine.anchors <= word < routine.anchors + ENTITY_COUNT * ANCHOR_SIZE + 2:
            replacement = COMMANDER.anchors + word - routine.anchors
        elif (
            routine.entity_base + 21 * ENTITY_SIZE
            <= word
            < routine.entity_base + 32 * ENTITY_SIZE
            and (word - routine.entity_base) % ENTITY_SIZE == 0
        ):
            replacement = COMMANDER.entity_base + word - routine.entity_base
        elif entry <= word < end:
            replacement = commander_entry + word - entry
        if replacement is not None:
            struct.pack_into("<H", normalized, offset, replacement)
    return bytes(normalized)


def execute(
    executable: bytes,
    routine: Routine,
    vector: dict[str, object],
    case_index: int,
    covered_edges: set[tuple[int, int]],
) -> tuple[dict[str, object], dict[int, int], int, bytes, bytes]:
    name = str(vector["name"])
    initial, segments, _matrix = make_inputs(routine, case_index)
    before = {address: bytes(data) for address, data in segments.items()}

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0xE0000)
    module = executable[routine.header_size :]
    expected_module = bytearray(module)
    expected_module[RETURN_IP] = 0xCC
    machine.mem_write(0, bytes(expected_module))
    for address, data in segments.items():
        machine.mem_write(address, bytes(data))
    for register, value in initial.items():
        machine.reg_write(register, value)

    events: list[tuple[object, ...]] = []
    entity_ids = []
    extent_comparison_loads = 0
    reached_return = False
    previous_offset: int | None = None
    entry_address = routine.image_address(routine.span[0])

    def instruction(cpu: Uc, address: int, _size: int, _context) -> None:
        nonlocal extent_comparison_loads, reached_return, previous_offset
        if address == RETURN_IP:
            reached_return = True
            cpu.emu_stop()
            return
        if address == entry_address + 0x47:
            entity_id = (cpu.reg_read(UC_X86_REG_SI) - routine.entity_base) // ENTITY_SIZE
            entity_ids.append(entity_id)
        if address == routine.extent_address + 0x17:
            assert cpu.reg_read(UC_X86_REG_DS) == COMPARE // 16
            assert cpu.reg_read(UC_X86_REG_SI) == COMPARE_OFFSET
            extent_comparison_loads += 1
            return
        if address in (routine.extent_address, routine.position_address):
            kind = "extent" if address == routine.extent_address else "position"
            entity_id = cpu.reg_read(UC_X86_REG_AX)
            projected = struct.unpack(
                "<2H", cpu.mem_read(STACK + routine.matrix + 36, 4)
            )
            scale = struct.unpack(
                "<H", cpu.mem_read(STACK + routine.matrix + 42, 2)
            )[0]
            if kind == "extent":
                values = (cpu.reg_read(UC_X86_REG_CX), cpu.reg_read(UC_X86_REG_DX))
                expected_return = routine.extent_return
                assert cpu.reg_read(UC_X86_REG_CS) == routine.extent_cs
                assert address - routine.extent_cs * 16 == routine.extent_ip
            else:
                values = (cpu.reg_read(UC_X86_REG_BX), cpu.reg_read(UC_X86_REG_CX))
                expected_return = routine.position_return
                assert cpu.reg_read(UC_X86_REG_CS) == routine.position_cs
                assert address - routine.position_cs * 16 == routine.position_ip
            assert cpu.reg_read(UC_X86_REG_BP) == routine.matrix
            assert cpu.reg_read(UC_X86_REG_DS) == GAME // 16
            assert cpu.reg_read(UC_X86_REG_ES) == GAME // 16
            assert cpu.reg_read(UC_X86_REG_DI) == routine.work
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEE0
            assert struct.unpack("<HH", cpu.mem_read(STACK + 0xFEE0, 4)) == (
                routine.image_address(expected_return),
                0,
            )
            events.append((kind, entity_id, *values, *projected, scale))
            return

        if entry_address <= address < routine.image_address(routine.span[1]):
            relative = address - entry_address
            if previous_offset in BRANCHES:
                covered_edges.add((previous_offset, relative))
            previous_offset = relative

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(entry_address, 0, count=50000)
    assert reached_return, (routine.name, name)
    assert bytes(machine.mem_read(0, len(module))) == bytes(expected_module)

    after = {
        address: bytes(machine.mem_read(address, SEGMENT_SIZE)) for address in segments
    }
    for address in (DATA, EXTRA, COMPARE, FRAME, FS_DATA):
        assert after[address] == before[address], (routine.name, name, hex(address))
    game_allowed = [
        (routine.counter, routine.counter + 2),
        (routine.work, routine.work + WORK_SIZE),
    ] + [
        (
            routine.entity_base + entity_id * ENTITY_SIZE,
            routine.entity_base + (entity_id + 1) * ENTITY_SIZE,
        )
        for entity_id in range(21, 32)
    ]
    assert_unchanged_outside(before[GAME], after[GAME], game_allowed, routine.name)
    stack_allowed = [
        (routine.matrix + 36, routine.matrix + 44),
        (0xFEC0, STACK_POINTER + 4),
    ]
    assert_unchanged_outside(before[STACK], after[STACK], stack_allowed, routine.name)
    assert after[STACK][STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] == (
        STACK_SENTINEL
    )

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
    for register in REGISTERS:
        assert machine.reg_read(register) == expected_registers[register], (
            routine.name,
            name,
            register,
        )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    defined_flags = {flag: bool(flags & mask) for flag, mask in masks.items()}
    assert defined_flags == vector["defined_flags"], (routine.name, name, flags)

    event_bytes = b"".join(
        struct.pack("<B6H", int(event[0] == "position"), *event[1:])
        for event in events
    )
    work = list(struct.unpack("<4H", after[GAME][routine.work : routine.work + 8]))
    counter = struct.unpack(
        "<H", after[GAME][routine.counter : routine.counter + 2]
    )[0]
    row = {
        "name": name,
        "anchors": ENTITY_COUNT,
        "entity_ids_in_order": entity_ids,
        "helper_events": len(events),
        "extent_comparison_loads": extent_comparison_loads,
        "helper_sequence_sha256": hashlib.sha256(event_bytes).hexdigest(),
        "first_event": list(events[0]) if events else None,
        "last_event": list(events[-1]) if events else None,
        "final_work": work,
        "final_counter": counter,
        "defined_flags": defined_flags,
    }
    registers = {register: machine.reg_read(register) for register in REGISTERS}
    stack = normalized_stack(routine, after[STACK], before[STACK])
    return row, registers, flags, semantic_game(routine, after[GAME]), stack


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
    verify_span(executable, routine.span, routine.body_sha256, routine.name)
    verify_span(
        executable,
        routine.extent_file_span,
        routine.extent_sha256,
        f"{routine.name} extent helper",
    )
    verify_span(
        executable,
        routine.position_file_span,
        routine.position_sha256,
        f"{routine.name} position helper",
    )
    assert executable[routine.span[1] - 1] == 0xCB
    return executable


def expected_edges() -> set[tuple[int, int]]:
    return {
        (source, destination)
        for source, destinations in BRANCHES.items()
        for destination in destinations
    }


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
        default=Path(__file__).parent / "oracle_vectors/func_9b98_natural.json",
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
        commander_row, commander_registers, commander_flags, commander_game, commander_stack = (
            commander_result
        )
        sequel_row, sequel_registers, sequel_flags, sequel_game, sequel_stack = sequel_result
        assert commander_row == vector, vector["name"]
        assert sequel_row == commander_row, vector["name"]
        assert sequel_registers == commander_registers, vector["name"]
        assert sequel_flags == commander_flags, vector["name"]
        assert sequel_game == commander_game, vector["name"]
        assert sequel_stack == commander_stack, vector["name"]
        rows.append(sequel_row)

    assert commander_edges == expected_edges()
    assert sequel_edges == expected_edges()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB object-projector cases covering "
        f"{len(sequel_edges)} main conditional edges"
    )


if __name__ == "__main__":
    main()
