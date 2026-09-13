#!/usr/bin/env python3
"""Execute Big Bug Bang's subtitle-reveal coordinator."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
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

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
ROUTINE = (0xAB8F, 0xACAA)
ROUTINE_SHA256 = "d25bc57ecb735f12480a79569736122a9d8d1fad18c2dfd4cea94d3ba288c495"

DATA = 0x30000
EXTRA = 0x50000
STACK = 0x70000
GAME = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
TEXT_OFFSET = 0x1066
SUBTITLE_OWNER = 0x6234
PRIMARY_TABLE = 0x623F
SECONDARY_TABLE = 0x627F

OFFSETS = {
    "mode": 0x2A82,
    "active": 0x6234,
    "hold_ready": 0x6B92,
    "owner": 0x6B6C,
    "cursor": 0x6228,
    "phase": 0x6235,
    "pulse": 0x0D41,
    "delay": 0x0D3B,
    "text_speed": 0x0CC2,
    "hold_countdown": 0x0D3F,
    "ship_flags": 0x2745,
    "hold_complete": 0x6B91,
    "voice": 0x0F49,
    "remap": 0x5F26,
    "text_x": 0x622C,
    "text_y": 0x622E,
}

HELPERS = {
    0x3549: "span",
    0x3641: "vertical",
    0x31BE: "draw",
}

BRANCH_EDGES = {
    0xAB99: (0xABB5, 0xAB9B),
    0xABA0: (0xABB5, 0xABA2),
    0xABA7: (0xACA4, 0xABAB),
    0xABB1: (0xACA4, 0xABB5),
    0xABBE: (0xABD6, 0xABC0),
    0xABE3: (0xABF1, 0xABE5),
    0xABE7: (0xABF1, 0xABE9),
    0xABF6: (0xAC15, 0xABF8),
    0xAC05: (0xAC0E, 0xAC07),
    0xAC26: (0xACA4, 0xAC28),
    0xAC3C: (0xAC54, 0xAC3E),
    0xAC43: (0xAC7C, 0xAC45),
    0xAC59: (0xAC7C, 0xAC5B),
    0xAC60: (0xAC7C, 0xAC62),
    0xAC67: (0xAC7C, 0xAC69),
    0xAC9B: (0xACA4, 0xAC9D),
}

CASES = (
    {"name": "all_display_gates_clear"},
    {"name": "hold_owner_mismatch", "hold_ready": 1, "owner": 0x1111},
    {
        "name": "hold_owner_fallback_draws",
        "hold_ready": 1,
        "owner": SUBTITLE_OWNER,
        "phase": 0,
    },
    {
        "name": "zero_cursor_initializes_opening_frame",
        "mode": 2,
        "cursor": 0,
        "phase": 0x7777,
    },
    {
        "name": "phase_two_zero_pulse_advances_phase",
        "active": 1,
        "phase": 2,
        "pulse": 0,
    },
    {
        "name": "phase_one_uses_primary_frame_and_fe_color",
        "mode": 2,
        "phase": 1,
        "pulse": 1,
    },
    {
        "name": "phase_zero_delay_holds_cursor_and_draws_lines",
        "active": 1,
        "phase": 0,
        "delay": 3,
    },
    {
        "name": "phase_zero_delay_advances_one_character",
        "mode": 2,
        "phase": 0,
        "delay": 0,
    },
    {
        "name": "terminal_text_starts_line_hold",
        "active": 1,
        "phase": 0,
        "cursor": TEXT_OFFSET + 6,
    },
    {
        "name": "terminal_text_ship_gate_blocks_line_hold",
        "active": 1,
        "phase": 0,
        "cursor": TEXT_OFFSET + 6,
        "ship_flags": 4,
    },
    {
        "name": "terminal_text_existing_hold_blocks_reload",
        "mode": 2,
        "phase": 0,
        "cursor": TEXT_OFFSET + 6,
        "hold_complete": 1,
    },
    {
        "name": "terminal_text_ready_hold_blocks_reload",
        "hold_ready": 1,
        "owner": SUBTITLE_OWNER,
        "phase": 0,
        "cursor": TEXT_OFFSET + 6,
    },
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def write8(data: bytearray, offset: int, value: int) -> None:
    data[offset] = value & 0xFF


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_record(
    data: bytearray, offset: int, kind: int, x: int, y: int, extent: int
) -> None:
    struct.pack_into("<hHHH", data, offset, kind, x, y, extent)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initialize(
    case: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray, bytes, bytes]:
    data = seeded_segment(case_index, 7, 0x21)
    stack = seeded_segment(case_index, 19, 0xCB)
    extra = bytes(seeded_segment(case_index, 17, 0x65))
    game = bytes(seeded_segment(case_index, 11, 0x43))

    write8(data, OFFSETS["mode"], int(case.get("mode", 0)))
    write8(data, OFFSETS["active"], int(case.get("active", 0)))
    write8(data, OFFSETS["hold_ready"], int(case.get("hold_ready", 0)))
    write16(data, OFFSETS["owner"], int(case.get("owner", 0x2222)))
    write16(data, OFFSETS["cursor"], int(case.get("cursor", TEXT_OFFSET)))
    write16(data, OFFSETS["phase"], int(case.get("phase", 0)))
    write16(data, OFFSETS["pulse"], int(case.get("pulse", 1)))
    write16(data, OFFSETS["delay"], int(case.get("delay", 3)))
    write16(data, OFFSETS["text_speed"], 8)
    write16(data, OFFSETS["hold_countdown"], 0x3535)
    write16(data, OFFSETS["ship_flags"], int(case.get("ship_flags", 0)))
    write8(data, OFFSETS["hold_complete"], int(case.get("hold_complete", 0)))
    write8(data, OFFSETS["voice"], 0xFB)
    write8(data, OFFSETS["remap"], 0x56)
    write16(data, OFFSETS["text_x"], 10)
    write16(data, OFFSETS["text_y"], 8)
    data[TEXT_OFFSET : TEXT_OFFSET + 7] = b"AB\rCD\r\0"

    write_record(stack, PRIMARY_TABLE, 0, 11, 22, 33)
    write_record(stack, PRIMARY_TABLE + 8, 1, 44, 55, 66)
    write_record(stack, PRIMARY_TABLE + 16, -1, 0, 0, 0)
    write_record(stack, SECONDARY_TABLE, 2, 77, 88, 99)
    write_record(stack, SECONDARY_TABLE + 8, -1, 0, 0, 0)
    write_record(data, PRIMARY_TABLE, -1, 0, 0, 0)
    write_record(data, SECONDARY_TABLE, -1, 0, 0, 0)

    write16(stack, STACK_POINTER, RETURN_IP)
    write16(stack, STACK_POINTER + 2, 0)
    stack[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = STACK_SENTINEL
    return data, stack, extra, game


def expected(
    case: dict[str, object], data_before: bytearray
) -> tuple[bytearray, list[tuple[str, tuple[int, ...]]], bool, int]:
    after = bytearray(data_before)
    mode = int(case.get("mode", 0))
    active = int(case.get("active", 0))
    hold_ready = int(case.get("hold_ready", 0))
    owner = int(case.get("owner", 0x2222))
    cursor = int(case.get("cursor", TEXT_OFFSET))
    phase = int(case.get("phase", 0))
    pulse = int(case.get("pulse", 1))
    delay = int(case.get("delay", 3))
    ship_flags = int(case.get("ship_flags", 0))
    hold_complete = int(case.get("hold_complete", 0))
    calls: list[tuple[str, tuple[int, ...]]] = []

    entered = bool(
        mode & 2 or active & 1 or (hold_ready & 1 and owner == SUBTITLE_OWNER)
    )
    if not entered:
        return after, calls, entered, phase

    if cursor == 0:
        write16(after, OFFSETS["delay"], 2)
        write16(after, OFFSETS["pulse"], 1)
        write16(after, OFFSETS["cursor"], TEXT_OFFSET)
        write16(after, OFFSETS["phase"], 2)
        cursor = TEXT_OFFSET
        phase = 2
        pulse = 1

    if phase == 2:
        calls.extend(
            (
                ("span", (0x00FF, 11, 22, 33)),
                ("vertical", (0x00FF, 44, 55, 66)),
            )
        )
    elif phase == 1:
        calls.extend(
            (
                ("span", (0x00FE, 11, 22, 33)),
                ("vertical", (0x00FE, 44, 55, 66)),
            )
        )
    else:
        calls.append(("span", (0x00FE, 77, 88, 99)))
    write8(after, OFFSETS["remap"], 0)

    if phase != 0:
        if pulse == 0:
            write16(after, OFFSETS["pulse"], 1)
            write16(after, OFFSETS["phase"], phase - 1)
        return after, calls, entered, phase

    if data_before[cursor] != 0:
        if delay == 0:
            write16(
                after, OFFSETS["delay"], read16(data_before, OFFSETS["text_speed"]) >> 2
            )
            write16(after, OFFSETS["cursor"], cursor + 1)
    elif ship_flags & 4 == 0 and hold_complete & 1 == 0 and hold_ready & 1 == 0:
        write8(after, OFFSETS["voice"], 0)
        write16(
            after,
            OFFSETS["hold_countdown"],
            read16(data_before, OFFSETS["text_speed"]) << 2,
        )
        write8(after, OFFSETS["hold_complete"], 1)
    calls.extend(
        (
            ("draw", (TEXT_OFFSET, 10, 8)),
            ("draw", (TEXT_OFFSET + 3, 10, 16)),
        )
    )
    return after, calls, entered, phase


def stack_word(cpu: Uc, index: int = 0) -> int:
    sp = cpu.reg_read(UC_X86_REG_SP)
    return struct.unpack("<H", cpu.mem_read(STACK + sp + index * 2, 2))[0]


def far_return(cpu: Uc) -> None:
    return_ip = stack_word(cpu)
    return_cs = stack_word(cpu, 1)
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def execute(
    executable: bytes,
    case: dict[str, object],
    case_index: int,
    covered_branch_edges: set[tuple[int, int]],
) -> dict[str, object]:
    name = str(case["name"])
    data_before, stack_before, extra_before, game_before = initialize(case, case_index)
    data_expected, expected_calls, entered, effective_phase = expected(
        case, data_before
    )
    module = executable[HEADER_SIZE:]

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(EXTRA, extra_before)
    cpu.mem_write(STACK, bytes(stack_before))
    cpu.mem_write(GAME, game_before)

    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
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
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    for register, value in initial.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, int | str]] = []
    reached_return = False
    previous_file_offset: int | None = None

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
        helper = HELPERS.get(address)
        if helper is not None:
            calls.append(
                {
                    "name": helper,
                    "ax": machine.reg_read(UC_X86_REG_AX),
                    "bx": machine.reg_read(UC_X86_REG_BX),
                    "cx": machine.reg_read(UC_X86_REG_CX),
                    "dx": machine.reg_read(UC_X86_REG_DX),
                    "si": machine.reg_read(UC_X86_REG_SI),
                    "di": machine.reg_read(UC_X86_REG_DI),
                    "bp": machine.reg_read(UC_X86_REG_BP),
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "es": machine.reg_read(UC_X86_REG_ES),
                    "sp": machine.reg_read(UC_X86_REG_SP),
                }
            )
            far_return(machine)
            return
        assert (
            image_address(ROUTINE[0])
            <= address
            < address + size
            <= image_address(ROUTINE[1])
        ), (name, hex(address + HEADER_SIZE))

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        data_write = DATA <= address < address + size <= DATA + SEGMENT_SIZE
        stack_write = (
            STACK + STACK_POINTER - 14
            <= address
            < address + size
            <= STACK + STACK_POINTER
        )
        assert data_write or stack_write, (name, hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=5000)
    assert reached_return, name

    actual_names = [str(call["name"]) for call in calls]
    assert actual_names == [item[0] for item in expected_calls], (
        name,
        actual_names,
        expected_calls,
    )
    for call, (_, arguments) in zip(calls, expected_calls, strict=True):
        if call["name"] == "draw":
            actual = (int(call["si"]), int(call["bx"]), int(call["dx"]))
            assert actual == arguments, (name, actual, arguments)
            assert (call["ds"], call["es"]) == (DATA // 16, DATA // 16), name
        else:
            actual = (
                int(call["ax"]),
                int(call["bx"]),
                int(call["cx"]),
                int(call["dx"]),
            )
            assert actual == arguments, (name, actual, arguments)

    data_after = bytes(cpu.mem_read(DATA, SEGMENT_SIZE))
    if data_after != bytes(data_expected):
        mismatch = next(
            offset
            for offset, pair in enumerate(zip(data_after, data_expected, strict=True))
            if pair[0] != pair[1]
        )
        raise AssertionError(
            f"{name}: DS {mismatch:#x}={data_after[mismatch]:#x}, "
            f"expected {data_expected[mismatch]:#x}"
        )
    assert bytes(cpu.mem_read(EXTRA, SEGMENT_SIZE)) == extra_before, name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == game_before, name
    assert bytes(cpu.mem_read(0, len(module))) == module, name

    stack_expected = bytearray(stack_before)
    struct.pack_into("<I", stack_expected, STACK_POINTER - 4, initial[UC_X86_REG_EAX])
    write16(stack_expected, STACK_POINTER - 6, initial[UC_X86_REG_DS])
    write16(stack_expected, STACK_POINTER - 8, initial[UC_X86_REG_ESI])
    write16(stack_expected, STACK_POINTER - 10, initial[UC_X86_REG_EBP])
    if calls:
        return_ips = {"span": 0xA40C, "vertical": 0xA413, "draw": 0xA48D}
        write16(stack_expected, STACK_POINTER - 14, return_ips[str(calls[-1]["name"])])
        write16(stack_expected, STACK_POINTER - 12, 0)
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), name

    preserved = {
        UC_X86_REG_EAX: initial[UC_X86_REG_EAX],
        UC_X86_REG_ESI: initial[UC_X86_REG_ESI],
        UC_X86_REG_EBP: initial[UC_X86_REG_EBP],
        UC_X86_REG_DS: initial[UC_X86_REG_DS],
        UC_X86_REG_FS: initial[UC_X86_REG_FS],
        UC_X86_REG_GS: initial[UC_X86_REG_GS],
        UC_X86_REG_SS: initial[UC_X86_REG_SS],
        UC_X86_REG_SP: STACK_POINTER + 4,
    }
    for register, value in preserved.items():
        assert cpu.reg_read(register) == value, (
            name,
            register,
            cpu.reg_read(register),
            value,
        )
    expected_es = DATA // 16 if entered and effective_phase == 0 else EXTRA // 16
    assert cpu.reg_read(UC_X86_REG_ES) == expected_es, name
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 4, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    ), name

    registers = {
        "eax": UC_X86_REG_EAX,
        "ax": UC_X86_REG_AX,
        "ebx": UC_X86_REG_EBX,
        "bx": UC_X86_REG_BX,
        "ecx": UC_X86_REG_ECX,
        "cx": UC_X86_REG_CX,
        "edx": UC_X86_REG_EDX,
        "dx": UC_X86_REG_DX,
        "esi": UC_X86_REG_ESI,
        "si": UC_X86_REG_SI,
        "edi": UC_X86_REG_EDI,
        "di": UC_X86_REG_DI,
        "bp": UC_X86_REG_BP,
        "ebp": UC_X86_REG_EBP,
        "sp": UC_X86_REG_SP,
        "ds": UC_X86_REG_DS,
        "es": UC_X86_REG_ES,
        "fs": UC_X86_REG_FS,
        "gs": UC_X86_REG_GS,
        "ss": UC_X86_REG_SS,
        "flags": UC_X86_REG_EFLAGS,
    }
    return {
        "name": name,
        "mode": int(case.get("mode", 0)),
        "active": int(case.get("active", 0)),
        "hold_ready": int(case.get("hold_ready", 0)),
        "owner": int(case.get("owner", 0x2222)),
        "cursor": int(case.get("cursor", TEXT_OFFSET)),
        "phase": int(case.get("phase", 0)),
        "pulse": int(case.get("pulse", 1)),
        "delay": int(case.get("delay", 3)),
        "ship_flags": int(case.get("ship_flags", 0)),
        "hold_complete": int(case.get("hold_complete", 0)),
        "calls": calls,
        "data_sha256": hashlib.sha256(data_after).hexdigest(),
        "registers_after": {
            register_name: cpu.reg_read(register)
            for register_name, register in registers.items()
        },
        "return": "far",
    }


def semantic_row(
    row: dict[str, object], text_offset: int, owner: int
) -> dict[str, object]:
    cursor = int(row["cursor"])
    calls = []
    for raw_call in row["calls"]:
        call = dict(raw_call)
        if call["name"] == "draw":
            calls.append(
                {
                    "name": "draw",
                    "text_offset": int(call["si"]) - text_offset,
                    "position": [int(call["bx"]), int(call["dx"])],
                }
            )
        else:
            calls.append(
                {
                    "name": call["name"],
                    "color": int(call["ax"]),
                    "origin": [int(call["bx"]), int(call["cx"])],
                    "extent": int(call["dx"]),
                }
            )
    return {
        "name": row["name"],
        "mode": row["mode"],
        "active": row["active"],
        "hold_ready": row["hold_ready"],
        "owner_matches": int(row["owner"]) == owner,
        "cursor": None if cursor == 0 else cursor - text_offset,
        "phase": row["phase"],
        "pulse": row["pulse"],
        "delay": row["delay"],
        "ship_flags": row["ship_flags"],
        "hold_complete": row["hold_complete"],
        "calls": calls,
        "return": row["return"],
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert len(rows) >= len(commander)
    for sequel, original in zip(rows[: len(commander)], commander, strict=True):
        assert semantic_row(sequel, TEXT_OFFSET, SUBTITLE_OWNER) == semantic_row(
            original, 0x0E18, 0x5E64
        ), sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_93f5_natural.json",
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
    assert executable[ROUTINE[1] - 1] == 0xCB

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
    print(f"verified {len(rows)} original BBB subtitle-reveal cases")


if __name__ == "__main__":
    main()
