#!/usr/bin/env python3
"""Execute Big Bug Bang's bridge page-preparation coordinator."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
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

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
ROUTINE = (0xACE4, 0xAD37)
ROUTINE_SHA256 = "6c25d9912d1d45e090246ee126aa9e77ee797bca8addf5d249907575fe7367b8"

DATA = 0x40000
GAME = 0x50000
EXTRA = 0x60000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

DISPLAY_POINTER = 0x55F1
BACK_POINTER = 0x55F9
SHIP_FLAGS = 0x2745
BRIDGE_FRAME = 0x2A35
PALETTE_DIRTY = 0x5F25
TRANSPARENT_ZERO = 0x5F27
DIRTY_COPY = 0x5601

CASES = (
    ("ship_active", 0x23451234, 0x34562345, 0x0001, 0x002A, False, 0x0000),
    ("ship_active_high_bits", 0x45673456, 0x56784567, 0xFFFF, 0x7FFF, True, 0),
    ("inactive_zero_frame", 0x67895678, 0x789A6789, 0x0000, 0x0000, False, 0x08D5),
    ("inactive_other_flag", 0x89AB789A, 0x9ABC89AB, 0x0002, 0x7FFF, False, 0x0455),
    ("inactive_high_frame", 0xABCD9ABC, 0xBCDEABCD, 0x8000, 0xFFFF, True, 0x0891),
    ("inactive_signed_frame", 0xFFFF0000, 0x0000FFFF, 0xFFFE, 0x8000, True, 0x04C4),
    ("ship_active_direction_set", 0x13572468, 0x24681357, 0x0005, 1, False, 0),
)

EXTERNALS = {
    0x39F8: ("blit_fill_row_5221", "far", 0),
    0xA858: ("ship_3d_projection_matrix_build", "far", 0),
    0xA9AF: ("ship_3d_point_cloud_project", "far", 0),
    0xAB37: ("ship_3d_object_sprite_project", "far", 0),
    0x4074: ("sprite_slot_commit_dirty_range", "far", 0x15),
    0x40EE: ("sprite_slot_dirty_range_render", "far", 0x15),
    0xA7BA: ("bridge_panorama_frame_load", "near", None),
}

BRANCH_EDGES = {0xAD24: (0xAD36, 0xAD26)}


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def read32(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write32(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 11 + case_index * 37 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def stack_word(cpu: Uc, index: int = 0) -> int:
    sp = cpu.reg_read(UC_X86_REG_SP)
    return struct.unpack("<H", cpu.mem_read(STACK + sp + index * 2, 2))[0]


def near_return(cpu: Uc) -> None:
    return_ip = stack_word(cpu)
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 2) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def far_return(cpu: Uc) -> None:
    return_ip = stack_word(cpu)
    return_cs = stack_word(cpu, 1)
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def execute(
    executable: bytes,
    case: tuple[str, int, int, int, int, bool, int],
    case_index: int,
    covered_branch_edges: set[tuple[int, int]],
) -> dict[str, object]:
    (
        name,
        display_pointer,
        back_pointer,
        ship_flags,
        frame,
        mutate_display_in_dirty_render,
        bridge_flags,
    ) = case
    ship_active = ship_flags & 1 != 0

    data_before = seeded_segment(case_index, 23, 0x31)
    game_before = bytearray(
        (offset * 31 + (offset >> 8) * 5 + case_index * 19 + 0x53) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    extra_before = bytearray(
        (offset * 13 + case_index * 29 + 0x75) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    stack_before = seeded_segment(case_index, 7, 0x97)
    write32(data_before, DISPLAY_POINTER, display_pointer)
    write32(data_before, BACK_POINTER, back_pointer)
    write16(data_before, SHIP_FLAGS, ship_flags)
    write16(data_before, BRIDGE_FRAME, frame)
    write32(game_before, DISPLAY_POINTER, 0xA55A9669)
    write32(game_before, BACK_POINTER, 0x87783CC3)
    write32(extra_before, DISPLAY_POINTER, 0x69965AA5)
    write32(extra_before, BACK_POINTER, 0xC33C7887)
    write16(stack_before, STACK_POINTER, RETURN_IP)
    write16(stack_before, STACK_POINTER + 2, 0)
    stack_before[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    transparent_before = data_before[TRANSPARENT_ZERO]
    dirty_copy_before = data_before[DIRTY_COPY]
    data_expected = bytearray(data_before)
    data_expected[PALETTE_DIRTY] = 1
    if not ship_active:
        data_expected[TRANSPARENT_ZERO] = 1
        data_expected[DIRTY_COPY] = 1

    direction_set = name.endswith("direction_set")
    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: EXTRA // 16,
        UC_X86_REG_FS: 0x7000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0212 | (0x0400 if direction_set else 0),
    }

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(GAME, bytes(game_before))
    cpu.mem_write(EXTRA, bytes(extra_before))
    cpu.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        cpu.reg_write(register, value)

    expected_names = [
        "blit_fill_row_5221",
        "ship_3d_projection_matrix_build",
        "ship_3d_point_cloud_project",
        "ship_3d_object_sprite_project",
        "sprite_slot_commit_dirty_range",
        "sprite_slot_dirty_range_render",
    ]
    if not ship_active:
        expected_names.append("bridge_panorama_frame_load")

    calls: list[dict[str, object]] = []
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

        external = EXTERNALS.get(address)
        if external is None:
            assert (
                image_address(ROUTINE[0])
                <= address
                < address + size
                <= image_address(ROUTINE[1])
            ), (
                name,
                hex(file_offset),
            )
            return

        call_name, return_kind, fixed_ax = external
        expected_ax = frame if fixed_ax is None else fixed_ax
        call_index = len(calls)
        expected_bx = 0x001F if call_index >= 4 else initial[UC_X86_REG_EBX] & 0xFFFF
        expected_eax = (back_pointer & 0xFFFF0000) | expected_ax
        expected_ebx = (initial[UC_X86_REG_EBX] & 0xFFFF0000) | expected_bx
        expected_display = (
            display_pointer
            if call_name == "bridge_panorama_frame_load"
            else back_pointer
        )
        event: dict[str, object] = {
            "name": call_name,
            "cs": machine.reg_read(UC_X86_REG_CS),
            "ip": machine.reg_read(UC_X86_REG_IP),
            "sp": machine.reg_read(UC_X86_REG_SP),
            "eax": machine.reg_read(UC_X86_REG_EAX),
            "ebx": machine.reg_read(UC_X86_REG_EBX),
            "ds": machine.reg_read(UC_X86_REG_DS),
            "es": machine.reg_read(UC_X86_REG_ES),
            "gs": machine.reg_read(UC_X86_REG_GS),
            "display_pointer": struct.unpack(
                "<I", machine.mem_read(DATA + DISPLAY_POINTER, 4)
            )[0],
            "palette_dirty": machine.mem_read(DATA + PALETTE_DIRTY, 1)[0],
        }
        assert event["eax"] == expected_eax, (name, event, hex(expected_eax))
        assert event["ebx"] == expected_ebx, (name, event, hex(expected_ebx))
        assert event["display_pointer"] == expected_display, (name, event)
        assert event["palette_dirty"] == 1, (name, event)
        assert event["ds"] == DATA // 16, (name, event)
        assert event["es"] == EXTRA // 16, (name, event)
        assert event["gs"] == GAME // 16, (name, event)
        if call_name != "bridge_panorama_frame_load":
            assert (
                struct.unpack("<I", machine.mem_read(STACK + 0xFEFC, 4))[0]
                == display_pointer
            )
        else:
            assert machine.mem_read(DATA + TRANSPARENT_ZERO, 1)[0] == 1
            assert machine.mem_read(DATA + DIRTY_COPY, 1)[0] == 1
            machine.reg_write(UC_X86_REG_EFLAGS, bridge_flags)
        calls.append(event)
        if (
            call_name == "sprite_slot_dirty_range_render"
            and mutate_display_in_dirty_render
        ):
            machine.mem_write(DATA + DISPLAY_POINTER, struct.pack("<I", 0xDEADBEEF))
        (far_return if return_kind == "far" else near_return)(machine)

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        data_write = DATA <= address < address + size <= DATA + SEGMENT_SIZE
        stack_write = (
            STACK + 0xFEF8 <= address < address + size <= STACK + STACK_POINTER
        )
        assert data_write or stack_write, (name, hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=1000)
    assert reached_return, name
    assert [call["name"] for call in calls] == expected_names, (name, calls)

    assert bytes(cpu.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected), name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before), name
    assert bytes(cpu.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_before), name
    assert bytes(cpu.mem_read(0, len(module))) == module, name

    stack_expected = bytearray(stack_before)
    write16(stack_expected, 0xFEF8, 0xA519)
    write16(stack_expected, 0xFEFA, 0)
    write32(stack_expected, 0xFEFC, display_pointer)
    if not ship_active:
        write16(stack_expected, 0xFEFE, 0xA536)
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), name

    final_ax = 0x0015 if ship_active else frame
    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (back_pointer & 0xFFFF0000) | final_ax
    expected_registers[UC_X86_REG_EBX] = (initial[UC_X86_REG_EBX] & 0xFFFF0000) | 0x001F
    expected_registers[UC_X86_REG_SP] = STACK_POINTER + 4
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
        bytes(cpu.mem_read(STACK + STACK_POINTER + 4, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    ), name

    if ship_active:
        expected_flags = {
            "cf": False,
            "pf": False,
            "zf": False,
            "sf": False,
            "of": False,
        }
    else:
        expected_flags = {
            "cf": bool(bridge_flags & 0x0001),
            "pf": bool(bridge_flags & 0x0004),
            "af": bool(bridge_flags & 0x0010),
            "zf": bool(bridge_flags & 0x0040),
            "sf": bool(bridge_flags & 0x0080),
            "of": bool(bridge_flags & 0x0800),
        }
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
    actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
    assert actual_flags == expected_flags, (name, actual_flags, expected_flags)
    expected_direction = (
        bool(bridge_flags & 0x0400) if not ship_active else direction_set
    )
    assert bool(flags & 0x0400) == expected_direction, name

    return {
        "name": name,
        "display_pointer_before_and_after": display_pointer,
        "temporary_display_pointer": back_pointer,
        "ship_flags": ship_flags,
        "bridge_frame": None if ship_active else frame,
        "transparent_before": transparent_before,
        "transparent_after": data_expected[TRANSPARENT_ZERO],
        "dirty_copy_before": dirty_copy_before,
        "dirty_copy_after": data_expected[DIRTY_COPY],
        "calls": calls,
        "final_ax": final_ax,
        "final_bx": 0x001F,
        "defined_flags": actual_flags,
    }


def semantic_call(call: dict[str, object]) -> dict[str, object]:
    return {key: value for key, value in call.items() if key not in ("cs", "ip")}


def semantic_row(row: dict[str, object]) -> dict[str, object]:
    return {
        "name": row["name"],
        "display_pointer_before_and_after": row["display_pointer_before_and_after"],
        "temporary_display_pointer": row["temporary_display_pointer"],
        "ship_flags": row["ship_flags"],
        "bridge_frame": row["bridge_frame"],
        "transparent_before": int(row["transparent_before"]) != 0,
        "transparent_after": int(row["transparent_after"]) != 0,
        "dirty_copy_before": int(row["dirty_copy_before"]) != 0,
        "dirty_copy_after": int(row["dirty_copy_after"]) != 0,
        "calls": [semantic_call(call) for call in row["calls"]],
        "final_ax": row["final_ax"],
        "final_bx": row["final_bx"],
        "defined_flags": row["defined_flags"],
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert len(rows) == len(commander)
    for sequel, original in zip(rows, commander, strict=True):
        assert semantic_row(sequel) == semantic_row(original), sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_954a_natural.json",
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
    assert covered_branch_edges == expected_branch_edges
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB bridge-page cases")


if __name__ == "__main__":
    main()
