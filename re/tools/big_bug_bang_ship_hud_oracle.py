#!/usr/bin/env python3
"""Execute Big Bug Bang's ship HUD coordinator at 0xC859 directly."""

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
ROUTINE = (0xC859, 0xCA7C)
ROUTINE_SHA256 = "66271e559245260d139a29be75c18df658507586209e67cf73f6acbee4180c8b"

MACHINE_SIZE = 0xF0000
SEGMENT_SIZE = 0x10000
DATA = 0x30000
GAME = 0x50000
INCOMING_ES = 0x70000
RECORDS = 0x90000
RECORD_BYTES = 0x20000
DISPLAY = 0xC0000
STACK = 0xE0000
STACK_POINTER = 0xF800
STACK_TRANSIENT_START = 0xF7E0
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

INITIAL_EAX = 0x0001BEEF
RECORD_BASE_OFFSET = 0x0100
ARCHE_OFFSET = 0x1200
RECORD_LINK = 0x1234
FIRST_NAME_OFFSET = 0x2349
FIRST_TARGET = FIRST_NAME_OFFSET - 4
NAMED_OBJECT_OFFSET = 0x2A00
DISPLAY_OFFSET = 0x3200
SCENE_TOP_ROW = 0x1357

FIELDS = {
    "initialized": (0x277B, 1),
    "sequence_active": (0x277C, 1),
    "target_phase": (0x277D, 1),
    "scene_blocked": (0x277F, 1),
    "depth_band": (0x2780, 1),
    "depth_opening": (0x2781, 1),
    "exit_pending": (0x2784, 1),
    "init_pending": (0x2787, 1),
    "ship_flags": (0x2745, 2),
    "subtitle_mode": (0x2A82, 1),
    "ui_flags": (0x2A33, 2),
    "seek_arc": (0x2A3B, 2),
    "view_frame": (0x2A35, 2),
    "redraw_pending": (0x2A78, 1),
    "animation": (0x0C2A, 2),
    "target_center": (0x0CBE, 2),
    "preserve_widths": (0x0CE5, 1),
    "extra_entry": (0x0CE6, 1),
    "transition_steps": (0x0CE3, 1),
    "current_target": (0x276D, 2),
    "first_name": (0x275D, 2),
    "active_line": (0x6B5A, 2),
    "vm_enabled": (0x6B7E, 1),
    "scene_top": (0x21F3, 2),
    "resource_vertical": (0x21F5, 2),
    "presentation_gate": (0x2200, 1),
    "frame_presented": (0x5619, 2),
    "transition_percent": (0x561F, 2),
    "transition_increment": (0x561D, 2),
    "transition_first": (0x5F21, 1),
    "transition_last": (0x5F22, 1),
    "text_active": (0x6234, 1),
    "music_changed": (0x0DAB, 1),
}

# Relocated far calls execute at image-linear addresses. The one PUSH CS +
# near CALL boundary remains at its file offset and also returns far.
HELPERS = {
    0x11D9: ("backclear", (0xC88B,)),
    0x7954: ("state", (0xC8AC,)),
    0x7A70: ("presentable", (0xC8E5,)),
    0x7C50: ("c2", (0xC8F6,)),
    0xACB0: ("dispatch", (0xC916,)),
    0x3AD8: ("fullscreen", (0xC925,)),
    0xCE6A: ("band", (0xC92A, 0xCA10)),
    0xA5F5: ("bridge", (0xC973,)),
    0x1E60: ("palette", (0xC998,)),
    0x4074: ("commit", (0xC9B5,)),
    0x4D1A: ("dirty", (0xC9BD,)),
    0xCB3A: ("driver", (0xCA1A,)),
    0xCD61: ("source", (0xCA22,)),
    0xCB63: ("stream", (0xCA27,)),
}

CASES: tuple[dict[str, Any], ...] = (
    {
        "name": "exit_pending_defers_close_while_opening",
        "exit_pending": 1,
        "opening": 1,
        "outcome": "close_deferred",
        "calls": [],
    },
    {
        "name": "exit_pending_finishes_close",
        "exit_pending": 1,
        "opening": 0,
        "outcome": "closed",
        "calls": [],
    },
    {
        "name": "inactive_text_returns_after_frame_present",
        "text_active": 0,
        "outcome": "text_inactive",
        "calls": ["bridge", "commit", "dirty"],
    },
    {
        "name": "nonzero_text_cursor_blocks_target",
        "text_character": 0x41,
        "outcome": "text_revealing",
        "calls": ["bridge", "commit", "dirty"],
    },
    {
        "name": "palette_path_queues_current_target",
        "ui_flags": 8,
        "outcome": "target_queued",
        "calls": ["bridge", "palette", "commit", "dirty", "stream"],
    },
    {
        "name": "transition_complete_resets_increment",
        "transition_percent": 100,
        "transition_increment": 10,
        "outcome": "target_queued",
        "calls": ["bridge", "commit", "dirty", "stream"],
    },
    {
        "name": "transition_percent_alone_preserves_increment",
        "transition_percent": 100,
        "transition_increment": 9,
        "outcome": "target_queued",
        "calls": ["bridge", "commit", "dirty", "stream"],
    },
    {
        "name": "zero_current_target_closes",
        "current_target": 0,
        "opening": 1,
        "outcome": "close_deferred",
        "calls": ["bridge", "commit", "dirty"],
    },
    {
        "name": "any_negative_current_target_closes",
        "current_target": 0x8000,
        "opening": 1,
        "outcome": "close_deferred",
        "calls": ["bridge", "commit", "dirty"],
    },
    {
        "name": "positive_current_target_queues_without_selector_or_lookup",
        "current_target": 0x3456,
        "outcome": "target_queued",
        "calls": ["bridge", "commit", "dirty", "stream"],
    },
    {
        "name": "changed_music_rebuilds_plane_and_audio_source",
        "ui_flags": 8,
        "current_target": 0x3456,
        "music_changed": 1,
        "outcome": "target_queued",
        "calls": [
            "bridge",
            "palette",
            "commit",
            "dirty",
            "band",
            "driver",
            "source",
            "stream",
        ],
    },
    {
        "name": "initialization_uses_prebuilt_first_name_when_probe_mask_set",
        "initialized": 0,
        "probe_mask": 0x0140,
        "vm_enabled": 0,
        "outcome": "target_queued",
        "calls": [
            "backclear",
            "state",
            "c2",
            "dispatch",
            "fullscreen",
            "band",
            "bridge",
            "palette",
            "commit",
            "dirty",
            "stream",
        ],
    },
    {
        "name": "initialization_uses_record_link_when_probe_mask_clear",
        "initialized": 0,
        "probe_mask": 0,
        "outcome": "target_queued",
        "calls": [
            "backclear",
            "state",
            "presentable",
            "c2",
            "dispatch",
            "fullscreen",
            "band",
            "bridge",
            "palette",
            "commit",
            "dirty",
            "stream",
        ],
    },
    {
        "name": "pending_initialization_copies_hud_vertex_palette_alias",
        "initialized": 0,
        "init_pending": 1,
        "probe_mask": 0x0040,
        "outcome": "target_queued",
        "calls": [
            "backclear",
            "state",
            "c2",
            "dispatch",
            "fullscreen",
            "band",
            "bridge",
            "palette",
            "commit",
            "dirty",
            "stream",
        ],
    },
    {
        "name": "probe_uses_full_eax_index",
        "initialized": 0,
        "probe_mask": 0x0140,
        "wrapped_probe_mask": 0,
        "outcome": "target_queued",
        "calls": [
            "backclear",
            "state",
            "c2",
            "dispatch",
            "fullscreen",
            "band",
            "bridge",
            "palette",
            "commit",
            "dirty",
            "stream",
        ],
    },
)

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


def patterned_segment(
    case_index: int, step: int, page_step: int, case_step: int, base: int
) -> bytearray:
    return bytearray(
        (offset * step + (offset >> 8) * page_step + case_index * case_step + base)
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def put_value(image: bytearray, field: str, value: int) -> None:
    offset, width = FIELDS[field]
    if width == 1:
        image[offset] = value & 0xFF
    else:
        struct.pack_into("<H", image, offset, value & 0xFFFF)


def get_value(image: bytes | bytearray, field: str) -> int:
    offset, width = FIELDS[field]
    if width == 1:
        return image[offset]
    return struct.unpack_from("<H", image, offset)[0]


def put_word(image: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", image, offset, value & 0xFFFF)


def put_dword(image: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", image, offset, value & 0xFFFFFFFF)


def far_return(machine: Uc) -> None:
    sp = machine.reg_read(UC_X86_REG_SP)
    ip, cs = struct.unpack("<HH", machine.mem_read(STACK + sp, 4))
    machine.reg_write(UC_X86_REG_SP, (sp + 4) & 0xFFFF)
    machine.reg_write(UC_X86_REG_IP, ip)
    machine.reg_write(UC_X86_REG_CS, cs)


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
    data = patterned_segment(case_index, 7, 13, 17, 0x21)
    game = patterned_segment(case_index, 11, 19, 23, 0x43)
    incoming_es = patterned_segment(case_index, 17, 29, 31, 0x65)
    records = bytearray(
        (offset * 5 + (offset >> 8) * 37 + case_index * 41 + 0x87) & 0xFF
        for offset in range(RECORD_BYTES)
    )
    display = patterned_segment(case_index, 3, 43, 47, 0xA9)
    stack = patterned_segment(case_index, 19, 31, 53, 0xCB)

    initialized = int(case.get("initialized", 1))
    values = {
        "initialized": initialized,
        "sequence_active": 0xA5,
        "target_phase": 0x7D,
        "scene_blocked": 0xD2,
        "depth_band": 0x80,
        "depth_opening": int(case.get("opening", 1)),
        "exit_pending": int(case.get("exit_pending", 0)),
        "init_pending": int(case.get("init_pending", 0)),
        "ship_flags": 0xF3F3,
        "subtitle_mode": 0xE2,
        "ui_flags": int(case.get("ui_flags", 0)),
        "seek_arc": 0x9B9B,
        "view_frame": 0x9595,
        "redraw_pending": 0xD8,
        "animation": 0xA2A2,
        "target_center": 0xBEBE,
        "preserve_widths": 0xE5,
        "extra_entry": 0xE6,
        "transition_steps": 0xE3,
        "current_target": int(case.get("current_target", FIRST_TARGET)),
        "first_name": FIRST_NAME_OFFSET,
        "active_line": 0x5A5A,
        "vm_enabled": int(case.get("vm_enabled", 0x7E)),
        "scene_top": SCENE_TOP_ROW,
        "resource_vertical": 0xA7A7,
        "presentation_gate": 0xB2,
        "frame_presented": 0x4949,
        "transition_percent": int(case.get("transition_percent", 37)),
        "transition_increment": int(case.get("transition_increment", 6)),
        "transition_first": 0x51,
        "transition_last": 0x52,
        "text_active": int(case.get("text_active", 1)),
        "music_changed": int(case.get("music_changed", 0)),
    }
    for field, value in values.items():
        put_value(data, field, value)

    cursor_offset = 0x6800
    put_word(data, 0x6228, cursor_offset)
    data[cursor_offset] = int(case.get("text_character", 0))
    put_dword(data, 0x6AEC, ((RECORDS // 16) << 16) | RECORD_BASE_OFFSET)
    put_word(data, 0x6B22, ARCHE_OFFSET)
    put_word(data, 0x6B20, NAMED_OBJECT_OFFSET)
    put_dword(data, 0x55F1, ((DISPLAY // 16) << 16) | DISPLAY_OFFSET)
    put_dword(data, 0x55E9, 0xA1001111)
    put_dword(data, 0x55ED, 0xB2002222)
    put_word(data, 0x5609, 0x0909)
    put_word(data, 0x560B, 0x0B0B)

    put_word(records, ARCHE_OFFSET + 0x16, RECORD_LINK)
    put_word(records, RECORD_LINK, int(case.get("wrapped_probe_mask", 0x0140)))
    put_word(
        records,
        (INITIAL_EAX & 0xFFFF0000) + RECORD_LINK,
        int(case.get("probe_mask", 0x0140)),
    )

    struct.pack_into("<HH", stack, STACK_POINTER, RETURN_IP, 0)
    stack[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = STACK_SENTINEL
    return data, game, incoming_es, records, display, stack


def expected_images(case: dict[str, Any], images):
    data, game, incoming_es, records, display, stack = images
    data = data[:]
    game = game[:]
    incoming_es = incoming_es[:]
    records = records[:]

    initialized = int(case.get("initialized", 1))
    opening = int(case.get("opening", 1))
    exit_pending = int(case.get("exit_pending", 0))
    ui_flags = int(case.get("ui_flags", 0))
    text_active = int(case.get("text_active", 1))
    text_character = int(case.get("text_character", 0))
    current_target = int(case.get("current_target", FIRST_TARGET))
    transition_percent = int(case.get("transition_percent", 37))
    transition_increment = int(case.get("transition_increment", 6))

    if initialized == 0:
        if int(case.get("init_pending", 0)) & 1:
            put_value(data, "init_pending", 0)
            put_value(data, "subtitle_mode", 0)
            incoming_es[0x5861 : 0x5861 + 0xC0] = data[0x6168 : 0x6168 + 0xC0]
        put_value(data, "seek_arc", 0)
        put_value(data, "view_frame", 0x00B3)
        ui_flags |= 8
        put_value(data, "ui_flags", ui_flags)
        put_value(data, "animation", 1)
        put_value(data, "initialized", 1)
        put_value(data, "target_phase", 1)
        put_value(data, "target_center", 0x50)
        put_value(data, "preserve_widths", 1)
        put_value(data, "extra_entry", 1)
        put_value(data, "transition_steps", 10)
        if int(case.get("probe_mask", 0x0140)) & 0x0140:
            current_target = FIRST_TARGET
        else:
            current_target = RECORD_LINK
        put_value(data, "current_target", current_target)
        put_value(data, "scene_blocked", 1)
        put_value(data, "active_line", 3)
        put_value(data, "depth_band", 1)
        put_value(data, "resource_vertical", SCENE_TOP_ROW)
        put_value(data, "presentation_gate", 0)
        put_value(data, "vm_enabled", 1)
        game[0x5C21 : 0x5C21 + 0x300] = data[0x5621 : 0x5621 + 0x300]
        game[0x5921 : 0x5921 + 0x240] = bytes(0x240)
        game[0x5B61 : 0x5B61 + 0x40] = data[0x5861 : 0x5861 + 0x40]
        put_value(data, "transition_percent", 0)
        put_value(data, "transition_increment", 10)
        put_value(data, "transition_first", 0)
        put_value(data, "transition_last", 0xC0)
        transition_percent = 0
        transition_increment = 10

    if exit_pending & 1:
        put_value(data, "exit_pending", 1)
        if opening & 1 == 0:
            close_presentation(data)
    else:
        if ui_flags & 8:
            put_word(data, 0x5609, 0)
            put_word(data, 0x560B, 200)
        put_value(data, "frame_presented", 1)
        if text_active & 1 and text_character == 0:
            if transition_percent == 100 and transition_increment == 10:
                put_value(data, "transition_increment", 0)
            if current_target == 0 or current_target & 0x8000:
                put_value(data, "exit_pending", 1)
                if opening & 1 == 0:
                    close_presentation(data)
            else:
                put_word(records, NAMED_OBJECT_OFFSET + 0x0A, 0x00C1)
                put_word(records, NAMED_OBJECT_OFFSET + 0x0C, current_target)
                put_word(records, NAMED_OBJECT_OFFSET + 0x0E, 0)
                put_value(data, "scene_blocked", 0)

    return data, game, incoming_es, records, display, stack


def close_presentation(data: bytearray) -> None:
    put_value(data, "ship_flags", 0x0011)
    put_value(data, "sequence_active", 0)
    put_value(data, "text_active", 0)
    put_value(data, "scene_blocked", 0)
    put_value(data, "redraw_pending", 0)
    put_value(data, "exit_pending", 0)


def execute(executable: bytes, case: dict[str, Any], case_index: int):
    name = str(case["name"])
    before = initialize(case, case_index)
    expected = expected_images(case, before)
    data, game, incoming_es, records, display, stack = before

    initial = {
        UC_X86_REG_EAX: INITIAL_EAX,
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
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GAME // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_EFLAGS: 0x0AD7,
    }
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, executable)
    for address, image in (
        (DATA, data),
        (GAME, game),
        (INCOMING_ES, incoming_es),
        (RECORDS, records),
        (DISPLAY, display),
        (STACK, stack),
    ):
        machine.mem_write(address, bytes(image))
    for register, value in initial.items():
        machine.reg_write(register, value)

    calls: list[dict[str, int | str]] = []
    helper_returns_seen = {helper_name: set() for helper_name, _ in HELPERS.values()}
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
            helper_name, return_ips = helper
            sp = cpu.reg_read(UC_X86_REG_SP)
            return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(STACK + sp, 4))
            assert return_ip in return_ips and return_cs == 0, (
                name,
                helper_name,
                "frame",
                hex(return_ip),
                hex(return_cs),
            )
            assert return_ip not in helper_returns_seen[helper_name], (
                name,
                helper_name,
                "duplicate return site",
            )
            helper_returns_seen[helper_name].add(return_ip)
            call = {
                "name": helper_name,
                "ax": cpu.reg_read(UC_X86_REG_AX),
                "di": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                "si": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "sp": sp,
            }
            verify_call(name, helper_name, call, case, initial)
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
        machine.emu_start(ROUTINE[0], 0, count=4_000)
    except UcError as error:
        raise RuntimeError(
            f"{name}: failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (name, "did not return")

    actual_names = [str(call["name"]) for call in calls]
    assert actual_names == case["calls"], (name, actual_names, case["calls"])
    actual_images = (
        bytes(machine.mem_read(DATA, SEGMENT_SIZE)),
        bytes(machine.mem_read(GAME, SEGMENT_SIZE)),
        bytes(machine.mem_read(INCOMING_ES, SEGMENT_SIZE)),
        bytes(machine.mem_read(RECORDS, RECORD_BYTES)),
        bytes(machine.mem_read(DISPLAY, SEGMENT_SIZE)),
        bytes(machine.mem_read(STACK, SEGMENT_SIZE)),
    )
    for label, actual, wanted in zip(
        ("data", "game", "incoming ES", "records", "display"),
        actual_images[:5],
        expected[:5],
        strict=True,
    ):
        assert actual == bytes(wanted), (
            name,
            label,
            first_difference(actual, bytes(wanted)),
        )

    stack_after = actual_images[5]
    assert stack_after[:STACK_TRANSIENT_START] == bytes(
        stack[:STACK_TRANSIENT_START]
    ), (
        name,
        "stack prefix",
    )
    assert stack_after[STACK_POINTER:] == bytes(stack[STACK_POINTER:]), (
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
        machine.mem_read(DISPLAY + SEGMENT_SIZE, STACK - DISPLAY - SEGMENT_SIZE)
    ) == bytes(STACK - DISPLAY - SEGMENT_SIZE), (name, "unowned display/stack gap")

    expected_registers = {
        "eax": INITIAL_EAX,
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 2,
        "ds": DATA // 16,
        "es": INCOMING_ES // 16,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME // 16,
        "ss": STACK // 16,
    }
    initialized = int(case.get("initialized", 1))
    music = int(case.get("music_changed", 0)) & 1
    current = int(case.get("current_target", FIRST_TARGET))
    reaches_music = (
        not int(case.get("exit_pending", 0)) & 1
        and int(case.get("text_active", 1)) & 1
        and int(case.get("text_character", 0)) == 0
        and current != 0
        and current & 0x8000 == 0
    )
    if initialized == 0 or music and reaches_music:
        expected_registers["eax"] &= 0xFFFF
    effective_ui_flags = int(case.get("ui_flags", 0)) | (8 if initialized == 0 else 0)
    if effective_ui_flags & 8 and not int(case.get("exit_pending", 0)) & 1:
        expected_registers["edx"] &= 0xFFFF0000
    registers = {key: machine.reg_read(register) for key, register in REGISTERS.items()}
    assert registers == expected_registers, (name, registers, expected_registers)
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    data_after, game_after, incoming_after, record_after = actual_images[:4]
    row = {
        "name": name,
        "initialized": initialized,
        "init_pending": int(case.get("init_pending", 0)),
        "exit_pending": int(case.get("exit_pending", 0)),
        "opening": int(case.get("opening", 1)),
        "ui_flags": int(case.get("ui_flags", 0)),
        "text_active": int(case.get("text_active", 1)),
        "text_character": int(case.get("text_character", 0)),
        "current_target_before": int(case.get("current_target", FIRST_TARGET)),
        "current_target_after": get_value(data_after, "current_target"),
        "music_changed": int(case.get("music_changed", 0)),
        "transition_percent": int(case.get("transition_percent", 37)),
        "transition_increment": int(case.get("transition_increment", 6)),
        "probe_eax": (INITIAL_EAX & 0xFFFF0000) | RECORD_LINK,
        "probe_mask": int(case.get("probe_mask", 0x0140)),
        "vm_enabled_before": get_value(data, "vm_enabled"),
        "vm_enabled_after": get_value(data_after, "vm_enabled"),
        "state_after": {
            field: get_value(data_after, field)
            for field in (
                "initialized",
                "sequence_active",
                "scene_blocked",
                "depth_band",
                "depth_opening",
                "exit_pending",
                "init_pending",
                "ship_flags",
                "subtitle_mode",
                "ui_flags",
                "redraw_pending",
                "current_target",
                "vm_enabled",
                "active_line",
                "resource_vertical",
                "presentation_gate",
                "frame_presented",
                "transition_percent",
                "transition_increment",
                "transition_first",
                "transition_last",
                "text_active",
                "music_changed",
            )
        },
        "outcome": str(case["outcome"]),
        "calls": calls,
        "c1_record": list(
            struct.unpack_from("<HHH", record_after, NAMED_OBJECT_OFFSET + 0x0A)
        ),
        "data_sha256": hashlib.sha256(data_after).hexdigest(),
        "game_sha256": hashlib.sha256(game_after).hexdigest(),
        "incoming_es_sha256": hashlib.sha256(incoming_after).hexdigest(),
        "records_sha256": hashlib.sha256(record_after).hexdigest(),
        "registers_after": registers,
        "eflags_after": machine.reg_read(UC_X86_REG_EFLAGS),
        "return": "near",
    }
    return row, covered_edges


def verify_call(
    case_name: str,
    helper_name: str,
    call: dict[str, int | str],
    case: dict[str, Any],
    initial: dict[int, int],
) -> None:
    if helper_name == "presentable":
        assert (call["es"], call["di"]) == (RECORDS // 16, RECORD_LINK), case_name
    elif helper_name == "c2":
        probe_mask = int(case.get("probe_mask", 0x0140))
        target = FIRST_TARGET if probe_mask & 0x0140 else RECORD_LINK
        assert (call["es"], call["di"]) == (RECORDS // 16, target + 4), case_name
    elif helper_name == "dispatch":
        assert call["ax"] == SCENE_TOP_ROW, case_name
        assert call["sp"] == STACK_POINTER - 18, case_name
        assert initial[UC_X86_REG_EBP] & 0xFFFF == 0x789A
    elif helper_name == "fullscreen":
        assert (call["ds"], call["si"], call["sp"]) == (
            DISPLAY // 16,
            DISPLAY_OFFSET,
            STACK_POINTER - 20,
        ), case_name
    elif helper_name == "palette":
        assert (call["ax"], call["di"], call["es"]) == (
            0xFFCE,
            0x62E1,
            GAME // 16 if int(case.get("initialized", 1)) == 0 else INCOMING_ES // 16,
        ), case_name
    elif helper_name == "commit":
        assert (call["ax"], call["sp"]) == (0, STACK_POINTER - 18), case_name
    elif helper_name == "dirty":
        assert call["di"] == 0x69E2, case_name
    elif helper_name == "source":
        assert call["si"] == 0x0F7B, case_name


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


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
        raise SystemExit(f"BBB ship HUD body changed: {body_digest}")

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
        f"verified {len(rows)} BBB ship HUD cases and "
        f"{len(covered_edges)} conditional edges across "
        f"{len({source for source, _ in covered_edges})} branch sites"
    )


if __name__ == "__main__":
    main()
