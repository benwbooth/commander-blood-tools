#!/usr/bin/env python3
"""Execute Big Bug Bang's alien-overlay cycle coordinator at 0xCD69."""

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
from pathlib import Path
from typing import Any

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_CS,
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
ROUTINE = (0xCD69, 0xCE6A)
ROUTINE_SHA256 = "ffb093cb9b88e17768178a2bdb2e344e3d41f2fd0428930b9d067df4fcf7d703"
DATA_IMAGE_FILE_OFFSET = 0xF7F0

MACHINE_SIZE = 0xC0000
SEGMENT_SIZE = 0x10000
DATA = 0x40000
GAME = 0x50000
INCOMING_ES = 0x60000
OUTPUT = 0x70000
OVERLAY_WINDOW = 0x80000
STACK = 0x90000
HEAP = 0xA0000
OVERLAY_SEGMENT = 0x8200
OVERLAY_OFFSET = 0x1200
OVERLAY_ENTRY = OVERLAY_SEGMENT * 16 + OVERLAY_OFFSET
STACK_POINTER = 0xFF00
STACK_TRANSIENT_START = 0xFEF0
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

PATHS = (0x0087, 0x0090, 0x009C)
FIELDS = {
    "trigger": 0x0CED,
    "armed": 0x0CEC,
    "phase": 0x0CEE,
    "path_table": 0x0CC8,
    "overlay_pointer": 0x0C8E,
    "mouse_x": 0x0C22,
    "mouse_y": 0x0C24,
    "timing_offset": 0x6B6E,
    "heap_segment": 0x6AEE,
    "sound_header": 0x0DC5,
    "loader_flags": 0x0DAA,
    "viewport_pointer": 0x55FD,
    "back_pointer": 0x55F9,
    "transition_row": 0x0D45,
    "palette_dirty": 0x5F25,
    "sequence": 0x277C,
    "plane_band": 0x2780,
    "loaded_scene": 0x21F1,
    "pbm_palette": 0x5F23,
    "pbm_transparent": 0x5F27,
}

HELPERS = {
    0x2640: ("resource_file_load", (0xCDAA, 0xCDFA)),
    0xCFEC: ("snd_bank_loader", (0xCDB9, 0xCDED)),
    0x0D82: ("cdrom_audio_play_track_2", (0xCDD6,)),
    OVERLAY_ENTRY: ("alien_overlay_entry", (0xCDDA,)),
    0x0D55: ("cdrom_audio_stop", (0xCDDF,)),
    0x39F8: ("blit_fill_row_5221", (0xCE01,)),
    0x11D9: ("backbuffer_clear_flags", (0xCE40,)),
    0x119B: ("back_buffer_init", (0xCE52,)),
    0x2783: ("pbm_image_load_and_decode", (0xCE68,)),
}

CASES: tuple[dict[str, Any], ...] = (
    {"name": "trigger_clear", "trigger": 0, "phase": 0, "sequence": 0},
    {
        "name": "trigger_high_bit_ignored",
        "trigger": 0x80,
        "phase": 1,
        "sequence": 1,
    },
    {"name": "phase_0_sequence", "phase": 0, "sequence": 1},
    {"name": "phase_1_wraps_nonsequence", "phase": 1, "sequence": 0},
    {"name": "phase_1_wraps_sequence", "phase": 1, "sequence": 1},
    {
        "name": "sequence_high_bits_use_low_bit",
        "phase": 0,
        "sequence": 0x81,
    },
    {
        "name": "overlay_switches_to_nonsequence",
        "phase": 1,
        "sequence": 1,
        "overlay_sequence": 0,
    },
    {
        "name": "overlay_switches_to_sequence",
        "phase": 0,
        "sequence": 0,
        "overlay_sequence": 1,
    },
    {
        "name": "overlay_replaces_slot_pointer",
        "phase": 0,
        "sequence": 1,
        "overlay_pointer_after": 0x86202468,
    },
    {
        "name": "back_init_replaces_back_pointer",
        "phase": 0,
        "sequence": 0,
        "back_after_init": 0x7C003456,
    },
    {
        "name": "viewport_offset_wrap",
        "phase": 1,
        "sequence": 0,
        "output_offset": 0xFFF8,
    },
    {
        "name": "inherited_reverse_direction",
        "phase": 1,
        "sequence": 1,
        "direction": "reverse",
        "output_offset": 0x4200,
    },
)


def word(memory: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", memory, offset)[0]


def dword(memory: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", memory, offset)[0]


def write_word(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", memory, offset, value & 0xFFFF)


def write_dword(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", memory, offset, value & 0xFFFFFFFF)


def write_wrapped(memory: bytearray, offset: int, payload: bytes) -> None:
    for index, value in enumerate(payload):
        memory[(offset + index) & 0xFFFF] = value


def replace_low_word(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    ip, cs = struct.unpack("<HH", machine.mem_read(STACK + sp, 4))
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, ip)
    machine.reg_write(UC_X86_REG_CS, cs)


def flag_values(bits: int, *, include_af: bool = True) -> dict[str, bool]:
    masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "df": 0x0400,
        "of": 0x0800,
    }
    if not include_af:
        del masks["af"]
    return {name: bool(bits & mask) for name, mask in masks.items()}


def conditional_edges(body: bytes) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    result = set()
    for instruction in decoder.disasm(body, ROUTINE[0]):
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
        source = instruction.address - ROUTINE[0]
        result.add((source, int(immediate) - ROUTINE[0]))
        result.add((source, instruction.address + instruction.size - ROUTINE[0]))
    return result


def initialize(case: dict[str, Any], case_index: int):
    data = bytearray(
        (offset * 17 + (offset >> 8) * 13 + case_index * 29 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    game = bytearray(
        (offset * 23 + (offset >> 8) * 7 + case_index * 31 + 0x65) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    incoming_es = bytearray(
        (offset * 11 + (offset >> 8) * 19 + case_index * 37 + 0x87) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    output = bytearray(
        (offset * 7 + (offset >> 8) * 29 + case_index * 43 + 0xCB) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    overlay_window = bytearray(
        (offset * 31 + (offset >> 8) * 3 + case_index * 11 + 0x57) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    stack = bytearray(
        (offset * 13 + (offset >> 8) * 31 + case_index * 47 + 0xED) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    heap = bytearray(
        (offset * 5 + (offset >> 8) * 23 + case_index * 41 + 0xA9) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )

    phase = int(case["phase"]) & 0xFF
    sequence = int(case["sequence"]) & 0xFF
    output_offset = int(case.get("output_offset", 0x2400 + case_index * 0x180))
    original_mouse = (
        (0x1234 + case_index * 0x111) & 0xFFFF,
        (0xFEDC - case_index * 0x101) & 0xFFFF,
    )
    vbio_offset = (0x3600 + case_index * 0x22) & 0xFFFF
    callback_pointer = (0x1357 + case_index * 3) | (0x2468 << 16)
    sound_header = (0x31415926 + case_index * 0x01010101) & 0xFFFFFFFF
    loader_flags = (0xA700 | (0x31 + case_index)) & 0xFFFF
    back_pointer = (0x7800 << 16) | ((0x3100 + case_index * 0x40) & 0xFFFF)

    data[FIELDS["trigger"]] = int(case.get("trigger", 1)) & 0xFF
    data[FIELDS["armed"]] = 0xD7
    data[FIELDS["phase"]] = phase
    for index, path_offset in enumerate(PATHS):
        write_word(data, FIELDS["path_table"] + index * 2, path_offset)
    for offset, value in (
        (0x0087, b"amer.xdb\0"),
        (0x0090, b"croolis.xdb\0"),
        (0x009C, b"amer.xdb\0"),
        (0x0112, b"manu3.xdb\0"),
        (0x00F2, b"frigo.fd\0"),
        (0x0F4A, b"sn\\tb.snd\0"),
        (0x0F71, b"sn\\3D.snd\0"),
    ):
        data[offset : offset + len(value)] = value
    write_dword(
        data, FIELDS["overlay_pointer"], (OVERLAY_SEGMENT << 16) | OVERLAY_OFFSET
    )
    write_word(data, FIELDS["mouse_x"], original_mouse[0])
    write_word(data, FIELDS["mouse_y"], original_mouse[1])
    write_word(data, FIELDS["timing_offset"], vbio_offset)
    write_word(data, FIELDS["heap_segment"], HEAP // 16)
    write_dword(data, FIELDS["sound_header"], sound_header)
    write_word(data, FIELDS["loader_flags"], loader_flags)
    write_dword(data, FIELDS["viewport_pointer"], (OUTPUT // 16 << 16) | output_offset)
    write_dword(data, FIELDS["back_pointer"], back_pointer)
    write_word(data, FIELDS["transition_row"], 0xBEEF)
    data[FIELDS["palette_dirty"]] = 0x9A
    data[FIELDS["sequence"]] = sequence
    data[FIELDS["plane_band"]] = 0x7D
    write_word(data, FIELDS["loaded_scene"], 0x2468)
    data[FIELDS["pbm_palette"]] = 0x6B
    data[FIELDS["pbm_transparent"]] = 0xC3
    write_word(heap, vbio_offset, 0x0012 + case_index)
    write_dword(stack, 0x0CF6, callback_pointer)
    stack[STACK_POINTER : STACK_POINTER + 12] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    return data, game, incoming_es, output, overlay_window, stack, heap


def expected_images(case: dict[str, Any], case_index: int, before):
    data, game, incoming_es, output, overlay_window, stack, heap = (
        bytearray(image) for image in before
    )
    trigger = int(case.get("trigger", 1)) & 0xFF
    phase = int(case["phase"]) & 0xFF
    reverse = case.get("direction") == "reverse"
    output_offset = int(case.get("output_offset", 0x2400 + case_index * 0x180))
    overlay_sequence = int(case.get("overlay_sequence", case["sequence"])) & 0xFF
    overlay_pointer_after = int(
        case.get("overlay_pointer_after", (OVERLAY_SEGMENT << 16) | OVERLAY_OFFSET)
    )
    back_pointer = dword(data, FIELDS["back_pointer"])
    back_after_init = int(case.get("back_after_init", back_pointer))
    original_mouse = (word(data, FIELDS["mouse_x"]), word(data, FIELDS["mouse_y"]))
    vbio_offset = word(data, FIELDS["timing_offset"])

    if trigger & 1:
        next_phase = (phase + 1) & 0xFF
        if next_phase == 2:
            next_phase = 0
        data[FIELDS["trigger"]] = 0
        data[FIELDS["armed"]] = 0
        data[FIELDS["phase"]] = next_phase
        write_word(heap, vbio_offset, (word(heap, vbio_offset) + 1) & 0xFFFF)
        data[FIELDS["sequence"]] = overlay_sequence
        write_dword(data, FIELDS["overlay_pointer"], overlay_pointer_after)

        descriptor_fields = (
            struct.pack("<H", 0),
            struct.pack("<H", 1),
            struct.pack("<I", 4),
            struct.pack("<H", 320),
            struct.pack("<H", 200),
            struct.pack("<I", 0),
        )
        cursor = output_offset
        for field in descriptor_fields:
            write_wrapped(output, cursor, field)
            cursor = (cursor - len(field) if reverse else cursor + len(field)) & 0xFFFF

        write_word(data, FIELDS["transition_row"], 0)
        data[FIELDS["palette_dirty"]] = 1
        write_word(data, FIELDS["mouse_x"], original_mouse[0])
        write_word(data, FIELDS["mouse_y"], original_mouse[1])
        if overlay_sequence & 1:
            data[FIELDS["plane_band"]] = 1
            write_word(data, FIELDS["loaded_scene"], 0xFFFF)
        else:
            write_dword(data, FIELDS["back_pointer"], back_after_init)
            data[FIELDS["pbm_palette"]] = 0
            data[FIELDS["pbm_transparent"]] = 0
        write_word(stack, 0x0CF2, vbio_offset)
        write_word(stack, 0x0CF4, HEAP // 16)
    return data, game, incoming_es, output, overlay_window, stack, heap


def call_names(case: dict[str, Any]) -> list[str]:
    if not int(case.get("trigger", 1)) & 1:
        return []
    names = [
        "resource_file_load_overlay",
        "snd_bank_loader_3d",
        "cdrom_audio_play_track_2",
        "alien_overlay_entry",
        "cdrom_audio_stop",
        "snd_bank_loader_tb",
        "resource_file_load_manu3",
        "blit_fill_row_5221",
    ]
    sequence = int(case.get("overlay_sequence", case["sequence"])) & 1
    if sequence:
        names.append("backbuffer_clear_flags")
    else:
        names.extend(("back_buffer_init", "pbm_image_load_and_decode"))
    return names


def execute(executable: bytes, case: dict[str, Any], case_index: int):
    name = str(case["name"])
    before = initialize(case, case_index)
    expected = expected_images(case, case_index, before)
    data, game, incoming_es, output, overlay_window, stack, heap = before
    expected_data, _, _, expected_output, _, expected_stack, expected_heap = expected

    reverse = case.get("direction") == "reverse"
    phase = int(case["phase"]) & 0xFF
    trigger = int(case.get("trigger", 1)) & 0xFF
    output_offset = int(case.get("output_offset", 0x2400 + case_index * 0x180))
    overlay_sequence = int(case.get("overlay_sequence", case["sequence"])) & 0xFF
    overlay_pointer_after = int(
        case.get("overlay_pointer_after", (OVERLAY_SEGMENT << 16) | OVERLAY_OFFSET)
    )
    back_pointer = dword(data, FIELDS["back_pointer"])
    back_after_init = int(case.get("back_after_init", back_pointer))
    original_mouse = [word(data, FIELDS["mouse_x"]), word(data, FIELDS["mouse_y"])]
    mutated_mouse = [
        (0xA100 + case_index) & 0xFFFF,
        (0xB200 + case_index) & 0xFFFF,
    ]
    vbio_offset = word(data, FIELDS["timing_offset"])
    callback_pointer = dword(stack, 0x0CF6)
    sound_header = dword(data, FIELDS["sound_header"])
    temporary_header = (0xA5A50000 | case_index) & 0xFFFFFFFF
    restored_load_header = (0x5A5A0000 | case_index) & 0xFFFFFFFF
    loader_flags = word(data, FIELDS["loader_flags"])
    clear_result = (0x4100 + case_index) & 0xFFFF
    clear_dx = (0x5100 + case_index) & 0xFFFF
    back_init_result = (0x6100 + case_index) & 0xFFFF
    back_init_dx = (0x7100 + case_index) & 0xFFFF
    pbm_result = (0x2100 + case_index) & 0xFFFF
    terminal_flags = 0x08D5 if overlay_sequence & 1 else 0x0015
    if reverse:
        terminal_flags |= 0x0400

    initial = {
        UC_X86_REG_EAX: 0xA1A11234,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: DATA // 16,
        UC_X86_REG_ES: INCOMING_ES // 16,
        UC_X86_REG_FS: 0x5C00,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0202 | (0x0400 if reverse else 0),
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for address, image in (
        (DATA, data),
        (GAME, game),
        (INCOMING_ES, incoming_es),
        (OUTPUT, output),
        (OVERLAY_WINDOW, overlay_window),
        (STACK, stack),
        (HEAP, heap),
    ):
        machine.mem_write(address, bytes(image))
    for register, value in initial.items():
        machine.reg_write(register, value)

    expected_names = call_names(case)
    calls: list[dict[str, Any]] = []
    helper_returns_seen = {helper: set() for helper, _returns in HELPERS.values()}
    covered_edges: set[tuple[int, int]] = set()
    previous_branch: int | None = None
    reached_return = False

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_branch, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return

        helper = HELPERS.get(address)
        if helper is not None:
            generic_name, return_ips = helper
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(STACK + sp, 4))
            assert return_ip in return_ips and return_cs == 0, (
                name,
                generic_name,
                "frame",
                hex(return_ip),
                hex(return_cs),
            )
            assert return_ip not in helper_returns_seen[generic_name], (
                name,
                generic_name,
                "duplicate return site",
            )
            helper_returns_seen[generic_name].add(return_ip)
            call = {
                "call": generic_name,
                "cs": cpu.reg_read(UC_X86_REG_CS),
                "ip": cpu.reg_read(UC_X86_REG_IP),
                "sp": sp,
            }

            if generic_name == "resource_file_load":
                call["call"] = (
                    "resource_file_load_overlay"
                    if return_ip == 0xCDAA
                    else "resource_file_load_manu3"
                )
                call["path"] = [
                    cpu.reg_read(UC_X86_REG_DS),
                    cpu.reg_read(UC_X86_REG_SI),
                ]
                call["destination"] = [
                    cpu.reg_read(UC_X86_REG_ES),
                    cpu.reg_read(UC_X86_REG_DI),
                ]
                assert call["path"] == [
                    DATA // 16,
                    PATHS[phase] if return_ip == 0xCDAA else 0x0112,
                ], name
                assert call["destination"] == [OVERLAY_SEGMENT, OVERLAY_OFFSET], name
                if return_ip == 0xCDFA:
                    call["sound_header"] = dword(
                        cpu.mem_read(DATA, SEGMENT_SIZE), FIELDS["sound_header"]
                    )
                    assert call["sound_header"] == sound_header, name
                cpu.reg_write(
                    UC_X86_REG_EAX,
                    0x11112222 if return_ip == 0xCDAA else 0x33334444,
                )
            elif generic_name == "snd_bank_loader":
                call["call"] = (
                    "snd_bank_loader_3d"
                    if return_ip == 0xCDB9
                    else "snd_bank_loader_tb"
                )
                call["mode"] = cpu.reg_read(UC_X86_REG_AX)
                call["path"] = [
                    cpu.reg_read(UC_X86_REG_DS),
                    cpu.reg_read(UC_X86_REG_SI),
                ]
                call["sound_header"] = dword(
                    cpu.mem_read(DATA, SEGMENT_SIZE), FIELDS["sound_header"]
                )
                call["loader_flags"] = word(
                    cpu.mem_read(DATA, SEGMENT_SIZE), FIELDS["loader_flags"]
                )
                wanted_path = 0x0F71 if return_ip == 0xCDB9 else 0x0F4A
                wanted_header = (
                    sound_header if return_ip == 0xCDB9 else temporary_header
                )
                assert call["mode"] == 0, name
                assert call["path"] == [DATA // 16, wanted_path], name
                assert call["sound_header"] == wanted_header, name
                assert call["loader_flags"] == loader_flags, name
                cpu.mem_write(
                    DATA + FIELDS["sound_header"],
                    struct.pack(
                        "<I",
                        temporary_header
                        if return_ip == 0xCDB9
                        else restored_load_header,
                    ),
                )
            elif generic_name in ("cdrom_audio_play_track_2", "cdrom_audio_stop"):
                call["loader_flags"] = word(
                    cpu.mem_read(DATA, SEGMENT_SIZE), FIELDS["loader_flags"]
                )
                assert call["loader_flags"] == loader_flags & 0xFF00, name
            elif generic_name == "alien_overlay_entry":
                request_offset = cpu.reg_read(UC_X86_REG_BP)
                request = bytes(cpu.mem_read(STACK + request_offset, 8))
                timing_offset, timing_segment, callback_offset, callback_segment = (
                    struct.unpack("<HHHH", request)
                )
                call.update(
                    {
                        "bp": request_offset,
                        "timing_scale": [timing_offset, timing_segment],
                        "frame_callback": [callback_offset, callback_segment],
                        "phase_after": cpu.mem_read(DATA + FIELDS["phase"], 1)[0],
                        "mouse": list(
                            struct.unpack(
                                "<HH", cpu.mem_read(DATA + FIELDS["mouse_x"], 4)
                            )
                        ),
                    }
                )
                wanted_phase = 0 if phase == 1 else phase + 1
                assert call["bp"] == 0x0CF2, name
                assert call["timing_scale"] == [vbio_offset, HEAP // 16], name
                assert call["frame_callback"] == [
                    callback_pointer & 0xFFFF,
                    callback_pointer >> 16,
                ], name
                assert call["phase_after"] == wanted_phase, name
                assert call["mouse"] == original_mouse, name
                current_scale = struct.unpack(
                    "<H", cpu.mem_read(HEAP + vbio_offset, 2)
                )[0]
                cpu.mem_write(
                    HEAP + vbio_offset,
                    struct.pack("<H", (current_scale + 1) & 0xFFFF),
                )
                cpu.mem_write(
                    DATA + FIELDS["mouse_x"], struct.pack("<HH", *mutated_mouse)
                )
                cpu.mem_write(DATA + FIELDS["sequence"], bytes((overlay_sequence,)))
                cpu.mem_write(
                    DATA + FIELDS["overlay_pointer"],
                    struct.pack("<I", overlay_pointer_after),
                )
            elif generic_name == "blit_fill_row_5221":
                call["color"] = cpu.reg_read(UC_X86_REG_AX)
                assert call["color"] == 0, name
            elif generic_name == "backbuffer_clear_flags":
                call["plane_copy_enabled"] = cpu.mem_read(
                    DATA + FIELDS["plane_band"], 1
                )[0]
                assert call["plane_copy_enabled"] == 0, name
                cpu.reg_write(UC_X86_REG_AX, clear_result)
                cpu.reg_write(UC_X86_REG_DX, clear_dx)
                cpu.reg_write(UC_X86_REG_EFLAGS, terminal_flags | 0x0002)
            elif generic_name == "back_buffer_init":
                call["back_buffer_before"] = dword(
                    cpu.mem_read(DATA, SEGMENT_SIZE), FIELDS["back_pointer"]
                )
                assert call["back_buffer_before"] == back_pointer, name
                cpu.mem_write(
                    DATA + FIELDS["back_pointer"], struct.pack("<I", back_after_init)
                )
                cpu.reg_write(UC_X86_REG_AX, back_init_result)
                cpu.reg_write(UC_X86_REG_DX, back_init_dx)
            elif generic_name == "pbm_image_load_and_decode":
                call["path"] = [
                    cpu.reg_read(UC_X86_REG_DS),
                    cpu.reg_read(UC_X86_REG_SI),
                ]
                call["file_buffer_end"] = [
                    cpu.reg_read(UC_X86_REG_ES),
                    cpu.reg_read(UC_X86_REG_DI),
                ]
                call["palette_refresh"] = cpu.mem_read(DATA + FIELDS["pbm_palette"], 1)[
                    0
                ]
                call["transparent_zero"] = cpu.mem_read(
                    DATA + FIELDS["pbm_transparent"], 1
                )[0]
                assert call["path"] == [DATA // 16, 0x00F2], name
                assert call["file_buffer_end"] == [
                    back_after_init >> 16,
                    back_after_init & 0xFFFF,
                ], name
                assert call["palette_refresh"] == 0, name
                assert call["transparent_zero"] == 0, name
                cpu.reg_write(UC_X86_REG_AX, pbm_result)
                cpu.reg_write(UC_X86_REG_EFLAGS, terminal_flags | 0x0002)

            assert len(calls) < len(expected_names), (name, call)
            assert call["call"] == expected_names[len(calls)], (
                name,
                call,
                expected_names,
            )
            calls.append(call)
            far_return(cpu)
            previous_branch = None
            return

        assert ROUTINE[0] <= address < ROUTINE[1], (name, f"escaped to {address:#x}")
        offset = address - ROUTINE[0]
        if previous_branch is not None:
            covered_edges.add((previous_branch, offset))
        encoded = bytes(cpu.mem_read(address, 2))
        previous_branch = (
            offset
            if 0x70 <= encoded[0] <= 0x7F
            or (encoded[0] == 0x0F and 0x80 <= encoded[1] <= 0x8F)
            else None
        )

    machine.hook_add(UC_HOOK_CODE, instruction)
    try:
        machine.emu_start(ROUTINE[0], 0, count=2_000)
    except UcError as error:
        raise RuntimeError(
            f"{name}: failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (name, "did not return")
    assert [call["call"] for call in calls] == expected_names, (
        name,
        calls,
        expected_names,
    )

    actual_images = tuple(
        bytes(machine.mem_read(address, SEGMENT_SIZE))
        for address in (DATA, GAME, INCOMING_ES, OUTPUT, OVERLAY_WINDOW, STACK, HEAP)
    )
    for label, actual, wanted in zip(
        ("data", "game", "incoming ES", "output", "overlay", "heap"),
        actual_images[:5] + actual_images[6:],
        expected[:5] + expected[6:],
        strict=True,
    ):
        assert actual == bytes(wanted), (
            name,
            label,
            first_difference(actual, bytes(wanted)),
        )
    stack_after = actual_images[5]
    assert stack_after[:0x0CF2] == bytes(expected_stack[:0x0CF2]), (
        name,
        "stack prefix",
    )
    assert stack_after[0x0CF2:0x0CFA] == bytes(expected_stack[0x0CF2:0x0CFA]), (
        name,
        "request",
    )
    assert stack_after[0x0CFA:STACK_TRANSIENT_START] == bytes(
        expected_stack[0x0CFA:STACK_TRANSIENT_START]
    ), (name, "stack body")
    assert stack_after[STACK_POINTER:] == bytes(expected_stack[STACK_POINTER:]), (
        name,
        "caller stack",
    )
    assert bytes(machine.mem_read(0, len(executable))) == executable, (
        name,
        "executable",
    )
    assert bytes(machine.mem_read(len(executable), DATA - len(executable))) == bytes(
        DATA - len(executable)
    ), (name, "unowned executable/data gap")
    assert bytes(
        machine.mem_read(HEAP + SEGMENT_SIZE, MACHINE_SIZE - HEAP - SEGMENT_SIZE)
    ) == bytes(MACHINE_SIZE - HEAP - SEGMENT_SIZE), (name, "unowned tail")

    wanted_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": DATA // 16,
        "es": INCOMING_ES // 16,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME // 16,
        "ss": STACK // 16,
    }
    if trigger & 1:
        descriptor_cursor = output_offset
        for size in (2, 2, 4, 2, 2, 4):
            descriptor_cursor = (
                descriptor_cursor - size if reverse else descriptor_cursor + size
            ) & 0xFFFF
        wanted_registers["ebp"] = replace_low_word(wanted_registers["ebp"], 0x0CF2)
        if overlay_sequence & 1:
            wanted_registers["eax"] = clear_result
            wanted_registers["edx"] = replace_low_word(
                wanted_registers["edx"], clear_dx
            )
            wanted_registers["esi"] = replace_low_word(wanted_registers["esi"], 0x0112)
            wanted_registers["edi"] = replace_low_word(
                wanted_registers["edi"], descriptor_cursor
            )
        else:
            wanted_registers["eax"] = pbm_result
            wanted_registers["edx"] = replace_low_word(
                wanted_registers["edx"], back_init_dx
            )
            wanted_registers["esi"] = replace_low_word(wanted_registers["esi"], 0x00F2)
            wanted_registers["edi"] = replace_low_word(
                wanted_registers["edi"], back_after_init & 0xFFFF
            )
    register_ids = {
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
    actual_registers = {
        register_name: machine.reg_read(register)
        for register_name, register in register_ids.items()
    }
    assert actual_registers == wanted_registers, (
        name,
        actual_registers,
        wanted_registers,
    )

    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    if trigger & 1:
        expected_flags = flag_values(terminal_flags)
    else:
        tested = trigger & 1
        expected_flags = {
            "cf": False,
            "pf": tested.bit_count() % 2 == 0,
            "zf": tested == 0,
            "sf": False,
            "df": reverse,
            "of": False,
        }
    actual_flags = {flag: flag_values(flags_after)[flag] for flag in expected_flags}
    assert actual_flags == expected_flags, (name, actual_flags, expected_flags)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    data_after, _, _, output_after, _, _, heap_after = actual_images
    next_phase = phase
    if trigger & 1:
        next_phase = 0 if phase == 1 else (phase + 1) & 0xFF
    descriptor_cursor = output_offset
    if trigger & 1:
        for size in (2, 2, 4, 2, 2, 4):
            descriptor_cursor = (
                descriptor_cursor - size if reverse else descriptor_cursor + size
            ) & 0xFFFF
    return (
        {
            "name": name,
            "trigger": trigger,
            "phase_before": phase,
            "selected_path_offset": PATHS[phase] if trigger & 1 else None,
            "phase_after": next_phase,
            "sequence_before": int(case["sequence"]) & 0xFF,
            "sequence_after_callbacks": overlay_sequence
            if trigger & 1
            else int(case["sequence"]) & 0xFF,
            "overlay_pointer_after": dword(data_after, FIELDS["overlay_pointer"]),
            "tail": "inactive"
            if not trigger & 1
            else "sequence"
            if overlay_sequence & 1
            else "nonsequence",
            "direction": "reverse" if reverse else "forward",
            "viewport_pointer": [output_offset, OUTPUT // 16],
            "viewport_cursor_after": descriptor_cursor,
            "vbio_timing_before": word(heap, vbio_offset),
            "vbio_timing_after": word(heap_after, vbio_offset),
            "calls": calls,
            "registers_after": actual_registers,
            "defined_flags": expected_flags,
            "data_sha256": hashlib.sha256(data_after).hexdigest(),
            "output_sha256": hashlib.sha256(output_after).hexdigest(),
            "return": "far",
        },
        covered_edges,
    )


def verify_shipped_data(executable: bytes) -> None:
    base = DATA_IMAGE_FILE_OFFSET
    assert struct.unpack_from("<HHH", executable, base + FIELDS["path_table"]) == PATHS
    for offset, expected in (
        (0x0087, b"amer.xdb\0"),
        (0x0090, b"croolis.xdb\0"),
        (0x009C, b"amer.xdb\0"),
        (0x0112, b"manu3.xdb\0"),
        (0x00F2, b"frigo.fd\0"),
        (0x0F4A, b"sn\\tb.snd\0"),
        (0x0F71, b"sn\\3D.snd\0"),
    ):
        assert executable[base + offset : base + offset + len(expected)] == expected


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {args.executable.name} SHA-256 {digest}")
    body = executable[slice(*ROUTINE)]
    body_digest = hashlib.sha256(body).hexdigest()
    if body_digest != ROUTINE_SHA256:
        raise SystemExit(f"BBB alien-overlay-cycle body changed: {body_digest}")
    verify_shipped_data(executable)

    rows = []
    covered_edges: set[tuple[int, int]] = set()
    for case_index, case in enumerate(CASES):
        row, edges = execute(executable, case, case_index)
        rows.append(row)
        covered_edges.update(edges)
    expected_edges = conditional_edges(body)
    assert covered_edges == expected_edges, (
        "conditional coverage",
        sorted(expected_edges - covered_edges),
        sorted(covered_edges - expected_edges),
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} BBB alien-overlay-cycle cases and "
        f"{len(covered_edges)} conditional edges across "
        f"{len({source for source, _ in covered_edges})} branch sites"
    )


if __name__ == "__main__":
    main()
