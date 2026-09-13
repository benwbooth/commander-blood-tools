#!/usr/bin/env python3
"""Execute Big Bug Bang's bridge presentation-mode selector."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
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
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
ROUTINE = (0xACAA, 0xACE4)
ROUTINE_SHA256 = "05b3a38a7653dff61c732f9c01200f81b6114cb9d670da7928e78edbb750df6b"

DATA = 0x30000
EXTRA = 0x50000
STACK = 0x70000
GAME = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
STATE = 0x2A33
FRAME = 0x2A35

CASES = (
    ("gate_set_clears_modes", 0xA5F2, -0x8000),
    ("gate_set_preserves_other_bits", 0x5A0E, 0x1234),
    ("signed_minimum", 0x80F1, -0x8000),
    ("negative_one", 0x0109, -1),
    ("zero", 0x0000, 0),
    ("below_lower_boundary", 0x4401, 21),
    ("lower_boundary", 0x2204, 22),
    ("first_band_start", 0x3308, 23),
    ("first_band_end", 0x1201, 67),
    ("second_band_start", 0x2404, 68),
    ("second_band_end", 0x3608, 112),
    ("third_band_start", 0x4801, 113),
    ("third_band_end", 0x5A04, 157),
    ("above_upper_boundary", 0x6C08, 158),
    ("signed_maximum", 0x7E01, 0x7FFF),
)

BRANCH_EDGES = {
    0xACB5: (0xACDE, 0xACB7),
    0xACC1: (0xACD9, 0xACC3),
    0xACC7: (0xACD9, 0xACC9),
    0xACCE: (0xACD9, 0xACD0),
    0xACD5: (0xACD9, 0xACD7),
}


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def expected_result(state_before: int, frame: int) -> tuple[int, int, bool]:
    masked_state = state_before & 0xFF0F
    gate_set = masked_state & 2 != 0
    if gate_set or frame <= 22 or frame > 157:
        mode = 0 if gate_set else 0x10
    elif frame <= 67:
        mode = 0x20
    elif frame <= 112:
        mode = 0x40
    else:
        mode = 0x80
    return masked_state | mode, mode, gate_set


def execute(
    executable: bytes,
    case: tuple[str, int, int],
    case_index: int,
    covered_branch_edges: set[tuple[int, int]],
) -> dict[str, object]:
    name, state_before, frame = case
    expected_state, mode, gate_set = expected_result(state_before, frame)
    frame_word = frame & 0xFFFF

    data_before = seeded_segment(case_index, 7, 0x21)
    extra_before = seeded_segment(case_index, 11, 0x43)
    stack_before = seeded_segment(case_index, 19, 0x65)
    game_before = seeded_segment(case_index, 17, 0x87)
    write16(data_before, STATE, state_before)
    write16(data_before, FRAME, frame_word)
    write16(extra_before, STATE, 0x7887)
    write16(extra_before, FRAME, 0x3CC3)
    write16(game_before, STATE, 0xA55A)
    write16(game_before, FRAME, 0x9669)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    data_expected = bytearray(data_before)
    write16(data_expected, STATE, expected_state)
    stack_expected = bytearray(stack_before)

    initial = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0ED7,
    }
    write16(stack_expected, STACK_POINTER - 2, initial[UC_X86_REG_EBX])
    write16(stack_expected, STACK_POINTER - 4, initial[UC_X86_REG_EDX])

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(EXTRA, bytes(extra_before))
    cpu.mem_write(STACK, bytes(stack_before))
    cpu.mem_write(GAME, bytes(game_before))
    for register, value in initial.items():
        cpu.reg_write(register, value)

    phases: list[dict[str, int]] = []
    phase_offsets = {0xACB7, 0xACD9, 0xACDE}
    previous_file_offset: int | None = None
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal previous_file_offset, reached_return
        file_offset = address + HEADER_SIZE
        if previous_file_offset in BRANCH_EDGES:
            covered_branch_edges.add((previous_file_offset, file_offset))
        previous_file_offset = file_offset
        if address == RETURN_IP:
            reached_return = True
            machine.emu_stop()
            return
        assert (
            image_address(ROUTINE[0])
            <= address
            < address + size
            <= image_address(ROUTINE[1])
        ), (name, hex(file_offset))
        if file_offset in phase_offsets:
            phases.append(
                {
                    "file_offset": file_offset,
                    "ax": machine.reg_read(UC_X86_REG_AX),
                    "bx": machine.reg_read(UC_X86_REG_BX),
                    "dx": machine.reg_read(UC_X86_REG_DX),
                }
            )

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        state_write = DATA + STATE <= address < address + size <= DATA + STATE + 2
        stack_write = (
            STACK + STACK_POINTER - 4
            <= address
            < address + size
            <= STACK + STACK_POINTER
        )
        assert state_write or stack_write, (name, hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=1000)
    assert reached_return, name

    masked_state = state_before & 0xFF0F
    if gate_set:
        expected_phases = [
            {
                "file_offset": 0xACDE,
                "ax": masked_state,
                "bx": initial[UC_X86_REG_EBX] & 0xFFFF,
                "dx": initial[UC_X86_REG_EDX] & 0xFFFF,
            }
        ]
    else:
        expected_phases = [
            {
                "file_offset": 0xACB7,
                "ax": masked_state,
                "bx": initial[UC_X86_REG_EBX] & 0xFFFF,
                "dx": initial[UC_X86_REG_EDX] & 0xFFFF,
            },
            {
                "file_offset": 0xACD9,
                "ax": masked_state,
                "bx": mode >> 4,
                "dx": frame_word,
            },
            {
                "file_offset": 0xACDE,
                "ax": expected_state,
                "bx": mode,
                "dx": frame_word,
            },
        ]
    assert phases == expected_phases, (name, phases, expected_phases)

    assert bytes(cpu.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected), name
    assert bytes(cpu.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before), name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before), name
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), name
    assert bytes(cpu.mem_read(0, len(module))) == module, name

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (
        initial[UC_X86_REG_EAX] & 0xFFFF0000
    ) | expected_state
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 2
    for register, value in expected_registers.items():
        if register == UC_X86_REG_EFLAGS:
            continue
        assert cpu.reg_read(register) == value, (
            name,
            register,
            hex(cpu.reg_read(register)),
            hex(value),
        )
    assert cpu.reg_read(UC_X86_REG_CS) == 0, name
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    ), name

    flag_result = 2 if gate_set else expected_state
    expected_flags = {
        "cf": False,
        "pf": (flag_result & 0xFF).bit_count() % 2 == 0,
        "zf": flag_result == 0,
        "sf": bool(flag_result & 0x8000),
        "of": False,
    }
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    defined_flags = {
        flag: bool(flags & mask)
        for flag, mask in (
            ("cf", 1),
            ("pf", 4),
            ("zf", 0x40),
            ("sf", 0x80),
            ("of", 0x800),
        )
    }
    assert defined_flags == expected_flags, (name, defined_flags, expected_flags)

    return {
        "name": name,
        "state_before": state_before,
        "signed_frame": frame,
        "bit_one_gate_set": gate_set,
        "selected_mode": mode,
        "state_after": read16(data_expected, STATE),
        "ax_result": cpu.reg_read(UC_X86_REG_AX),
        "defined_flags": defined_flags,
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert rows == commander


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9510_natural.json",
    )
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    actual = hashlib.sha256(executable[slice(*ROUTINE)]).hexdigest()
    if actual != ROUTINE_SHA256:
        raise SystemExit(
            f"native span {ROUTINE[0]:#x}..{ROUTINE[1]:#x} changed: {actual}"
        )
    assert executable[ROUTINE[1] - 1] == 0xC3

    covered_branch_edges: set[tuple[int, int]] = set()
    rows = [
        execute(executable, case, index, covered_branch_edges)
        for index, case in enumerate(CASES)
    ]
    expected_branch_edges = {
        (source, destination)
        for source, destinations in BRANCH_EDGES.items()
        for destination in destinations
    }
    assert covered_branch_edges == expected_branch_edges, sorted(
        expected_branch_edges - covered_branch_edges
    )
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB presentation-mode cases")


if __name__ == "__main__":
    main()
