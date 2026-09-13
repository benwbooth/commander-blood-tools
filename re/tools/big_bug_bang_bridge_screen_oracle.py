#!/usr/bin/env python3
"""Execute Big Bug Bang's complete bridge-screen initializer."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX,
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
ROUTINE = (0xAD37, 0xADDE)
ROUTINE_SHA256 = "f9e0217330e6c9bc070b222bf2d2b21577f73497a454bf2426ac40dc8e6f87bc"

DATA = 0x44000
EXTRA = 0x54000
GAME = 0x68000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

OFFSETS = {
    "rebuild": 0x2A79,
    "transition": 0x2A7A,
    "mode": 0x2A80,
    "completion": 0x2A86,
    "frame": 0x2A35,
    "ship_depth": 0x2779,
    "dirty_copy": 0x5601,
    "clip_snapshot": 0x5619,
    "live_palette": 0x5621,
    "palette_refresh": 0x5F23,
    "palette_dirty": 0x5F25,
    "transparent_zero": 0x5F27,
    "panorama_palette": 0x5F28,
    "dark_remap": 0x62E1,
    "console_tint": 0x63E1,
    "vm_enabled": 0x6B7E,
}

EXTERNALS = {
    0xA7BA: ("bridge_panorama_frame_load", "near", 0, 0xA7BA),
    0x39F8: ("blit_fill_row_5221", "far", 0x02B1, 0x0EE8),
    0x3D4D: ("entity_object_populate", "far", 0x02B1, 0x123D),
    0xA4E4: ("page_flip", "far", 0, 0xA4E4),
    0x3E4E: ("entity_flag_state_transition", "far", 0x02B1, 0x133E),
    0x1E60: ("palette_blend_remap_table_build", "far", 0x01E6, 0),
    0x1FAD: ("tint_table_build_banked", "far", 0x01E6, 0x014D),
    0xA5DE: ("matrix_table_clear_2a1b", "far", 0, 0xA5DE),
}

BRANCH_EDGES = {
    0xAD5C: (0xAD88, 0xAD5E),
    0xADD1: (0xADD7, 0xADD3),
}

VM_BEFORE_VALUES = (0, 1, 0x80, 0xA5)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 31 + salt) & 0xFF
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


def state_snapshot(machine: Uc) -> dict[str, int]:
    return {
        "rebuild": machine.mem_read(DATA + OFFSETS["rebuild"], 1)[0],
        "palette_refresh": machine.mem_read(DATA + OFFSETS["palette_refresh"], 1)[0],
        "palette_dirty": machine.mem_read(DATA + OFFSETS["palette_dirty"], 1)[0],
        "completion": machine.mem_read(DATA + OFFSETS["completion"], 1)[0],
        "clip_snapshot": struct.unpack(
            "<H", machine.mem_read(DATA + OFFSETS["clip_snapshot"], 2)
        )[0],
        "transparent_zero": machine.mem_read(DATA + OFFSETS["transparent_zero"], 1)[0],
        "dirty_copy": machine.mem_read(DATA + OFFSETS["dirty_copy"], 1)[0],
        "mode": machine.mem_read(DATA + OFFSETS["mode"], 1)[0],
        "ship_depth": struct.unpack(
            "<H", machine.mem_read(DATA + OFFSETS["ship_depth"], 2)
        )[0],
        "vm_enabled": machine.mem_read(DATA + OFFSETS["vm_enabled"], 1)[0],
    }


def execute(
    executable: bytes,
    commander: dict[str, object],
    case_index: int,
    covered_branch_edges: set[tuple[int, int]],
) -> dict[str, object]:
    name = str(commander["name"])
    transition = int(commander["transition"])
    mode_before = int(commander["mode_before"])
    mode_after = int(commander["mode_after"])
    frame = int(commander["frame"])
    direction = -1 if commander["direction"] == "backward" else 1
    palette_mutated = bool(commander["palette_mutated"])
    first_call = commander["calls"][0]
    assert isinstance(first_call, dict)
    vm_before = VM_BEFORE_VALUES[case_index % len(VM_BEFORE_VALUES)]

    data_before = seeded_segment(case_index, 17, 0x43)
    extra_before = seeded_segment(case_index, 29, 0x65)
    game_before = seeded_segment(case_index, 11, 0x87)
    stack_before = seeded_segment(case_index, 7, 0x93)
    data_before[OFFSETS["transition"]] = transition
    data_before[OFFSETS["mode"]] = mode_before
    data_before[OFFSETS["transparent_zero"]] = int(first_call["transparent_zero"])
    data_before[OFFSETS["dirty_copy"]] = int(first_call["dirty_copy"])
    data_before[OFFSETS["vm_enabled"]] = vm_before
    write16(data_before, OFFSETS["frame"], frame)
    write16(data_before, OFFSETS["ship_depth"], int(first_call["ship_depth"]))
    write16(stack_before, STACK_POINTER, RETURN_IP)
    stack_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )

    data_expected = bytearray(data_before)
    extra_expected = bytearray(extra_before)
    data_expected[OFFSETS["rebuild"]] = 0
    data_expected[OFFSETS["palette_refresh"]] = 1
    data_expected[OFFSETS["palette_dirty"]] = 1
    data_expected[OFFSETS["completion"]] = 0
    write16(data_expected, OFFSETS["clip_snapshot"], 1)
    if transition & 1:
        data_expected[OFFSETS["transparent_zero"]] = 0
        data_expected[OFFSETS["dirty_copy"]] = 0
    data_expected[OFFSETS["vm_enabled"]] = 1
    data_expected[OFFSETS["palette_refresh"]] = 0
    write16(data_expected, OFFSETS["ship_depth"], 0)

    mutation_call = {
        "page_flip_callback_sets_mode": "page_flip",
        "entity_callback_clears_mode": "entity_flag_state_transition",
        "populate_callback_updates_palette_and_mode": "entity_object_populate",
    }.get(name)
    if mutation_call is not None:
        data_expected[OFFSETS["mode"]] = mode_after
    if palette_mutated:
        for index in range(768):
            data_expected[OFFSETS["panorama_palette"] + index] ^= 0xA5

    source = OFFSETS["panorama_palette"]
    destination = OFFSETS["live_palette"]
    for _ in range(0xC0):
        source_bytes = bytes(
            data_expected[(source + byte_index) & 0xFFFF] for byte_index in range(4)
        )
        extra_expected[destination : destination + 4] = source_bytes
        source = (source + direction * 4) & 0xFFFF
        destination = (destination + direction * 4) & 0xFFFF

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
        UC_X86_REG_FS: 0x7400,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202 | (0x0400 if direction < 0 else 0),
    }
    stack_expected = bytearray(stack_before)
    final_helper_return = 0xA5D7 if bool(commander["matrix_clear"]) else 0xA5CC
    for offset, value in (
        (0xFEF0, final_helper_return),
        (0xFEF2, 0),
        (0xFEF4, initial[UC_X86_REG_EDI]),
        (0xFEF6, initial[UC_X86_REG_ESI]),
        (0xFEF8, initial[UC_X86_REG_EDX]),
        (0xFEFA, initial[UC_X86_REG_ECX]),
        (0xFEFC, initial[UC_X86_REG_EBX]),
        (0xFEFE, initial[UC_X86_REG_EBP]),
    ):
        write16(stack_expected, offset, value)

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    cpu.mem_write(DATA, bytes(data_before))
    cpu.mem_write(EXTRA, bytes(extra_before))
    cpu.mem_write(GAME, bytes(game_before))
    cpu.mem_write(STACK, bytes(stack_before))
    for register, value in initial.items():
        cpu.reg_write(register, value)

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
            ), (name, hex(file_offset))
            return

        call_name, return_kind, expected_cs, expected_ip = external
        event: dict[str, object] = {
            "call": call_name,
            "cs": machine.reg_read(UC_X86_REG_CS),
            "ip": machine.reg_read(UC_X86_REG_IP),
            "sp": machine.reg_read(UC_X86_REG_SP),
            "ds": machine.reg_read(UC_X86_REG_DS),
            "es": machine.reg_read(UC_X86_REG_ES),
            "gs": machine.reg_read(UC_X86_REG_GS),
            **state_snapshot(machine),
        }
        assert event["cs"] == expected_cs and event["ip"] == expected_ip, (
            name,
            event,
        )
        if call_name == "bridge_panorama_frame_load":
            event["frame"] = machine.reg_read(UC_X86_REG_AX)
        elif call_name == "blit_fill_row_5221":
            event["color"] = machine.reg_read(UC_X86_REG_AX)
        elif call_name == "entity_object_populate":
            event.update(
                {
                    "object_id": machine.reg_read(UC_X86_REG_AX),
                    "resource_id": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                    "x": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                    "y": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "frame_index": machine.reg_read(UC_X86_REG_EBP) & 0xFFFF,
                }
            )
        elif call_name == "entity_flag_state_transition":
            event["object_id"] = machine.reg_read(UC_X86_REG_AX)
        elif call_name == "palette_blend_remap_table_build":
            event.update(
                {
                    "percent": struct.unpack(
                        "<h", struct.pack("<H", machine.reg_read(UC_X86_REG_AX))
                    )[0],
                    "red": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                    "green": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "blue": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                    "table": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                }
            )
        elif call_name == "tint_table_build_banked":
            event.update(
                {
                    "bank": machine.reg_read(UC_X86_REG_AX),
                    "table": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                }
            )
        calls.append(event)

        if call_name == mutation_call:
            machine.mem_write(DATA + OFFSETS["mode"], bytes((mode_after,)))
            if palette_mutated:
                palette = bytes(
                    value ^ 0xA5
                    for value in machine.mem_read(
                        DATA + OFFSETS["panorama_palette"], 768
                    )
                )
                machine.mem_write(DATA + OFFSETS["panorama_palette"], palette)
        (far_return if return_kind == "far" else near_return)(machine)

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        data_write = DATA <= address < address + size <= DATA + SEGMENT_SIZE
        extra_write = EXTRA <= address < address + size <= EXTRA + SEGMENT_SIZE
        stack_write = STACK <= address < address + size <= STACK + SEGMENT_SIZE
        assert data_write or extra_write or stack_write, (name, hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(ROUTINE[0]), 0, count=1000)
    assert reached_return, name

    assert bytes(cpu.mem_read(DATA, SEGMENT_SIZE)) == bytes(data_expected), name
    assert bytes(cpu.mem_read(EXTRA, SEGMENT_SIZE)) == bytes(extra_expected), name
    assert bytes(cpu.mem_read(GAME, SEGMENT_SIZE)) == bytes(game_before), name
    assert bytes(cpu.mem_read(STACK, SEGMENT_SIZE)) == bytes(stack_expected), name
    assert bytes(cpu.mem_read(0, len(module))) == module, name
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2, name
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    ), name

    expected_registers = dict(initial)
    expected_registers[UC_X86_REG_EAX] = (initial[UC_X86_REG_EAX] & 0xFFFF0000) | 0xE0
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

    tested = mode_after & 1
    expected_flags = {
        "cf": False,
        "pf": tested.bit_count() % 2 == 0,
        "zf": tested == 0,
        "sf": False,
        "of": False,
        "df": direction < 0,
    }
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "df": 0x400, "of": 0x800}
    actual_flags = {flag: bool(flags & mask) for flag, mask in masks.items()}
    assert actual_flags == expected_flags, (name, actual_flags, expected_flags)
    for call in calls:
        expected_vm = (
            1
            if call["call"]
            in (
                "palette_blend_remap_table_build",
                "tint_table_build_banked",
                "matrix_table_clear_2a1b",
            )
            else vm_before
        )
        assert call["vm_enabled"] == expected_vm, (name, call)

    return {
        "name": name,
        "transition": transition,
        "mode_before": mode_before,
        "mode_after": mode_after,
        "frame": frame,
        "direction": commander["direction"],
        "matrix_clear": commander["matrix_clear"],
        "palette_mutated": palette_mutated,
        "vm_execution_before": vm_before,
        "vm_execution_after": data_expected[OFFSETS["vm_enabled"]],
        "calls": calls,
        "defined_flags": actual_flags,
    }


def semantic_call(call: dict[str, object]) -> dict[str, object]:
    normalized = {
        key: value
        for key, value in call.items()
        if key not in ("cs", "ip", "vm_enabled")
    }
    if call["call"] == "palette_blend_remap_table_build":
        normalized["table"] = 0x5F11
    elif call["call"] == "tint_table_build_banked":
        normalized["table"] = 0x6011
    return normalized


def compare_commander(
    rows: list[dict[str, object]], commander: list[dict[str, object]]
) -> None:
    assert len(rows) == len(commander)
    for sequel, original in zip(rows, commander, strict=True):
        normalized = {
            key: value
            for key, value in sequel.items()
            if key not in ("vm_execution_before", "vm_execution_after")
        }
        normalized["calls"] = [semantic_call(call) for call in sequel["calls"]]
        expected = dict(original)
        expected["calls"] = [semantic_call(call) for call in original["calls"]]
        assert normalized == expected, sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_959d_natural.json",
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

    commander = json.loads(args.commander_vectors.read_text())
    covered_branch_edges: set[tuple[int, int]] = set()
    rows = [
        execute(executable, row, index, covered_branch_edges)
        for index, row in enumerate(commander)
    ]
    expected_branch_edges = {
        (source, destination)
        for source, destinations in BRANCH_EDGES.items()
        for destination in destinations
    }
    assert covered_branch_edges == expected_branch_edges
    compare_commander(rows, commander)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB bridge-screen cases")


if __name__ == "__main__":
    main()
