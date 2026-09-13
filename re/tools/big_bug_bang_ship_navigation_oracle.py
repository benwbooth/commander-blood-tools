#!/usr/bin/env python3
"""Execute Big Bug Bang's ship-navigation coordinator at 0xCB0F directly."""

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
ROUTINE = (0xCB0F, 0xCD69)
ROUTINE_SHA256 = "7189145b251008e8b0e36c00c913802d3e9dea36c996bd4c88533eee7e37f509"

MACHINE_SIZE = 0xF0000
SEGMENT_SIZE = 0x10000
DATA = 0x30000
GAME = 0x50000
INCOMING_ES = 0x70000
RECORDS = 0x90000
DISPLAY = 0xC0000
STACK = 0xE0000
STACK_POINTER = 0xF800
STACK_TRANSIENT_START = 0xF7D0
RETURN_IP = 0xF000
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

CURRENT_TARGET = 0x1200
RECORD_BASE = 0x0100
REDIRECT_TARGET = 0x1800
FIRST_CANDIDATE = 0x3000
SECOND_CANDIDATE = 0x3400
DEFAULT_ARK = 0x4444
DISPLAY_OFFSET = 0x3200

FIELDS = {
    "trigger": (0x2A78, 1),
    "sequence": (0x277C, 1),
    "exit": (0x2784, 1),
    "opening": (0x2781, 1),
    "defer": (0x6B86, 1),
    "presentation_active": (0x6B82, 1),
    "request_flags": (0x6B80, 1),
    "subtitle_mode": (0x2A82, 1),
    "owner": (0x6B6C, 2),
    "actor": (0x0C2A, 2),
    "previous_actor": (0x0C2E, 2),
    "ark": (0x6B28, 2),
    "deferred_kind": (0x6B3A, 2),
    "deferred_target": (0x6B3C, 2),
    "ui": (0x2A33, 2),
    "transition_step": (0x0CE4, 1),
    "transition_duration": (0x0CE3, 1),
    "choice_x": (0x279F, 2),
    "choice_width": (0x27A3, 2),
    "resource_vertical": (0x21F5, 2),
    "scene_cache": (0x21F1, 2),
    "text_menu": (0x2201, 1),
    "text_selection": (0x21F9, 2),
    "depth_closing": (0x2782, 1),
    "depth_step": (0x2783, 1),
    "frame_presented": (0x1006, 1),
    "bridge_seek": (0x2A3B, 2),
    "bridge_distance": (0x2A3D, 2),
    "navigation_rebuild": (0x2A79, 1),
    "navigation_snapshot": (0x29C7, 1),
    "ship_flags": (0x2745, 2),
    "active_line": (0x6B5A, 2),
    "presentation_gate": (0x2200, 1),
    "hud_initialized": (0x277B, 1),
    "text_active": (0x6234, 1),
    "hold_ready": (0x6B92, 1),
    "depth_band": (0x2780, 1),
    "word_choice_phase": (0x6B90, 1),
    "palette_last": (0x5F22, 1),
    "palette_percent": (0x561F, 2),
    "palette_increment": (0x561D, 2),
}

# Relocated far calls execute at image-linear addresses. The alien callback is
# entered by PUSH CS + near CALL but returns through the same far frame.
HELPERS = {
    0x7903: ("candidate_build", (0xCB51,)),
    0x7C50: ("c2", (0xCB99,)),
    0x8D8A: ("layout", (0xCBB7, 0xCCA3)),
    0x3A3C: ("back_fill", (0xCBF2,)),
    0x2783: ("pbm", (0xCC17,)),
    0x1E60: ("palette", (0xCC59,)),
    0xCD69: ("alien", (0xCC6B,)),
    0xA5F5: ("bridge", (0xCC70,)),
    0x3AC3: ("fullscreen", (0xCC81,)),
    0x18CE: ("interpolate", (0xCC99,)),
    0x39F8: ("display_fill", (0xCCD8,)),
    0x200B: ("palette_clear", (0xCCDD,)),
    0x119B: ("back_init", (0xCD32,)),
    0x96A6: ("vm_stop", (0xCD37,)),
}

CASES: tuple[dict[str, Any], ...] = (
    {"name": "idle_arms_opening_and_exit", "calls": []},
    {"name": "idle_defer_gate_blocks_opening", "defer": 1, "calls": []},
    {
        "name": "active_sequence_blocked_before_frame_copy",
        "sequence": 1,
        "presentation_active": 1,
        "calls": ["alien", "bridge"],
    },
    {
        "name": "active_sequence_nonlayout_duration_copies_frame",
        "sequence": 1,
        "duration": 5,
        "calls": ["alien", "bridge", "fullscreen"],
    },
    {
        "name": "active_sequence_waits_for_interpolation",
        "sequence": 1,
        "interpolation_complete": 0,
        "calls": ["alien", "bridge", "fullscreen", "interpolate"],
    },
    {
        "name": "completed_interpolation_negative_query_keeps_sequence",
        "sequence": 1,
        "interpolation_complete": 1,
        "layout_result": 0xFFFF,
        "calls": ["alien", "bridge", "fullscreen", "interpolate", "layout"],
    },
    {
        "name": "completed_interpolation_selection_arms_exit",
        "sequence": 1,
        "interpolation_complete": 1,
        "layout_result": 2,
        "calls": ["alien", "bridge", "fullscreen", "interpolate", "layout"],
    },
    {
        "name": "exit_while_opening_reenters_active_sequence",
        "exit_pending": 1,
        "opening": 1,
        "presentation_active": 1,
        "calls": ["alien", "bridge"],
    },
    {
        "name": "closed_exit_runs_final_reset",
        "exit_pending": 1,
        "calls": ["display_fill", "palette_clear", "back_init", "vm_stop"],
    },
    {
        "name": "trigger_accepts_unrestricted_visible_candidate",
        "trigger": 1,
        "filter_flags": 2,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 1,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "c2",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_opens_list_when_no_candidate_exists",
        "trigger": 1,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "layout",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_accepts_visible_candidate_related_to_record_base",
        "trigger": 1,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 1,
        "candidate_relation": RECORD_BASE,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "c2",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_rejects_candidate_related_only_to_current",
        "trigger": 1,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 1,
        "candidate_relation": CURRENT_TARGET,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "layout",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_ark_relation_opens_target_list",
        "trigger": 1,
        "filter_flags": 2,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 1,
        "candidate_relation": DEFAULT_ARK,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "layout",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_redirects_access_counter",
        "trigger": 1,
        "current_kind": 0x0080,
        "counter_link": REDIRECT_TARGET,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "layout",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_skips_nonvisible_candidate_and_opens_list",
        "trigger": 1,
        "filter_flags": 2,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 0,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "layout",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_skips_nonvisible_then_accepts_visible_candidate",
        "trigger": 1,
        "filter_flags": 2,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 0,
        "second_candidate": SECOND_CANDIDATE,
        "second_visible": 1,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "c2",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
    {
        "name": "trigger_accepts_visible_candidate_when_ark_is_current",
        "trigger": 1,
        "filter_flags": 2,
        "candidate": FIRST_CANDIDATE,
        "candidate_visible": 1,
        "candidate_relation": CURRENT_TARGET,
        "ark": CURRENT_TARGET,
        "presentation_active": 1,
        "calls": [
            "candidate_build",
            "c2",
            "back_fill",
            "pbm",
            "palette",
            "alien",
            "bridge",
        ],
    },
)


def patterned_segment(
    case_index: int,
    multiplier: int,
    page_multiplier: int,
    case_multiplier: int,
    bias: int,
) -> bytearray:
    return bytearray(
        (
            offset * multiplier
            + (offset >> 8) * page_multiplier
            + case_index * case_multiplier
            + bias
        )
        & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def read_value(image: bytes | bytearray, field: str) -> int:
    offset, width = FIELDS[field]
    if width == 1:
        return image[offset]
    return struct.unpack_from("<H", image, offset)[0]


def put_value(image: bytearray, field: str, value: int) -> None:
    offset, width = FIELDS[field]
    if width == 1:
        image[offset] = value & 0xFF
    else:
        struct.pack_into("<H", image, offset, value & 0xFFFF)


def read_word(image: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", image, offset)[0]


def put_word(image: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", image, offset, value & 0xFFFF)


def put_dword(image: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", image, offset, value & 0xFFFFFFFF)


def candidate_rows(case: dict[str, Any]) -> list[tuple[int, int, bool]]:
    rows = []
    first = int(case.get("candidate", 0))
    if first:
        rows.append(
            (
                first,
                int(case.get("candidate_relation", 0x5555)),
                bool(int(case.get("candidate_visible", 1))),
            )
        )
    second = int(case.get("second_candidate", 0))
    if second:
        rows.append(
            (
                second,
                int(case.get("second_relation", 0x6666)),
                bool(int(case.get("second_visible", 1))),
            )
        )
    return rows


def accepted_candidate(
    case: dict[str, Any],
) -> tuple[int | None, int]:
    unrestricted = int(case.get("filter_flags", 0)) & 2 != 0
    ark = int(case.get("ark", DEFAULT_ARK))
    fetched = 0
    for candidate, relation, visible in candidate_rows(case):
        fetched += 1
        if not visible:
            continue
        if not unrestricted and relation != RECORD_BASE:
            continue
        if ark != CURRENT_TARGET and relation == ark:
            return None, fetched
        return candidate, fetched
    return None, fetched + 1


def initialize(case: dict[str, Any], case_index: int):
    data = patterned_segment(case_index, 7, 13, 17, 0x21)
    game = patterned_segment(case_index, 11, 19, 23, 0x43)
    incoming_es = patterned_segment(case_index, 17, 29, 31, 0x65)
    records = patterned_segment(case_index, 5, 37, 41, 0x87)
    display = patterned_segment(case_index, 3, 43, 47, 0xA9)
    stack = patterned_segment(case_index, 19, 31, 53, 0xCB)

    values = {
        "trigger": int(case.get("trigger", 0)),
        "sequence": int(case.get("sequence", 0)),
        "exit": int(case.get("exit_pending", 0)),
        "opening": int(case.get("opening", 0)),
        "defer": int(case.get("defer", 0)),
        "presentation_active": int(case.get("presentation_active", 0)),
        "request_flags": 0xAB,
        "subtitle_mode": 0xE2,
        "owner": 0x6C6C,
        "actor": 0xA22A,
        "previous_actor": 0xA62E,
        "ark": int(case.get("ark", DEFAULT_ARK)),
        "deferred_kind": 0x6B3A,
        "deferred_target": 0x6B3C,
        "ui": 0x9393,
        "transition_step": 0x9B,
        "transition_duration": int(case.get("duration", 6)),
        "choice_x": 0x4D4D,
        "choice_width": 0x5151,
        "resource_vertical": 0xA7A7,
        "scene_cache": 0xA3A3,
        "text_menu": 0xB3,
        "text_selection": 0xABAB,
        "depth_closing": 0x30,
        "depth_step": 0x31,
        "frame_presented": 0xB8,
        "bridge_seek": 0x9B9B,
        "bridge_distance": 0x9D9D,
        "navigation_rebuild": 0xD9,
        "navigation_snapshot": 0x39,
        "ship_flags": 0xF3F3,
        "active_line": 0x8888,
        "presentation_gate": 0xB2,
        "hud_initialized": 0x29,
        "text_active": 0x64,
        "hold_ready": 0xBC,
        "depth_band": 0x2E,
        "word_choice_phase": 0xBA,
        "palette_last": 0x52,
        "palette_percent": 0x4F4F,
        "palette_increment": 0x4D4D,
    }
    for field, value in values.items():
        put_value(data, field, value)

    put_word(data, 0x276D, CURRENT_TARGET)
    put_dword(data, 0x6AEC, ((RECORDS // 16) << 16) | RECORD_BASE)
    put_word(data, 0x2D4B, 0x1A2B)
    put_word(data, 0x2D4F, 0x3A4B)
    put_word(data, 0x5609, 0x3939)
    put_word(data, 0x560B, 0x3B3B)
    put_dword(data, 0x55F9, ((DISPLAY // 16) << 16) | DISPLAY_OFFSET)
    data[0x5F23] = 0x53
    data[0x5F27] = 0x57
    data[0x0CEA] = 0xE1

    current_kind = int(case.get("current_kind", 0x0002))
    counter_link = int(case.get("counter_link", 0))
    put_word(records, CURRENT_TARGET, current_kind)
    put_word(records, CURRENT_TARGET + 0x14, counter_link)
    records[RECORD_BASE + 2] = int(case.get("filter_flags", 0))
    if counter_link:
        put_word(records, counter_link + 0x14, 0x7FFE)
    for candidate, relation, visible in candidate_rows(case):
        records[candidate + 2] = (records[candidate + 2] & ~4) | (4 if visible else 0)
        put_word(records, candidate + 0x18, relation)

    candidates = [candidate for candidate, _relation, _visible in candidate_rows(case)]
    for index, candidate in enumerate((*candidates, 0)):
        put_word(stack, 0x2E03 + index * 2, candidate)
    put_word(data, 0x2E03, 0x7777)
    put_word(data, 0x2E05, 0x7777)

    struct.pack_into("<HH", stack, STACK_POINTER, RETURN_IP, 0)
    stack[STACK_POINTER + 4 : STACK_POINTER + 4 + len(STACK_SENTINEL)] = STACK_SENTINEL
    return data, game, incoming_es, records, display, stack


def expected_images(case: dict[str, Any], images):
    data, game, incoming_es, records, display, stack = images
    data = data[:]
    game = game[:]
    incoming_es = incoming_es[:]
    records = records[:]

    trigger = int(case.get("trigger", 0))
    sequence = int(case.get("sequence", 0))
    exit_pending = int(case.get("exit_pending", 0))
    opening = int(case.get("opening", 0))
    defer = int(case.get("defer", 0))
    presentation_active = int(case.get("presentation_active", 0))
    duration = int(case.get("duration", 6))

    if trigger & 1:
        put_value(data, "request_flags", read_value(data, "request_flags") & 0xFC)
        put_value(data, "subtitle_mode", 0)
        put_value(data, "owner", 0)
        put_value(data, "actor", read_value(data, "previous_actor"))
        counter = (
            int(case.get("counter_link", 0))
            if int(case.get("current_kind", 2)) & 0x80
            else CURRENT_TARGET
        )
        put_word(records, counter + 0x14, read_word(records, counter + 0x14) + 1)
        accepted, _fetched = accepted_candidate(case)
        if accepted is None:
            put_value(data, "ui", read_value(data, "ui") | 4)
            put_value(data, "transition_step", 0)
            put_value(data, "transition_duration", 6)
            data[0x2A87] = 0
            put_value(data, "choice_x", read_word(data, 0x2D4B))
            put_value(data, "choice_width", read_word(data, 0x2D4F))
        else:
            put_value(data, "deferred_kind", 0x00C4)
            put_value(data, "deferred_target", accepted)

        put_value(data, "trigger", 0)
        put_value(data, "sequence", 1)
        put_value(data, "resource_vertical", 0x23)
        put_value(data, "scene_cache", 0xFFFF)
        put_word(data, 0x5609, 0)
        put_word(data, 0x560B, 200)
        data[0x5F23] = 0
        data[0x5F27] = 0
        data[0x0CEA] = 0
        incoming_es[0x5DA1:0x5E61] = data[0x57A1:0x5861]
        put_value(data, "text_menu", 0)
        put_value(data, "text_selection", 0xFFFF)
        put_value(data, "depth_closing", 1)
        put_value(data, "depth_step", 2)
        sequence = 1
        duration = read_value(data, "transition_duration")

    ran_active = bool((exit_pending & 1 and opening & 1) or sequence & 1)
    if exit_pending & 1:
        if opening & 1 == 0:
            put_value(data, "ui", 9)
            put_value(data, "bridge_seek", 0)
            put_value(data, "bridge_distance", 50)
            put_value(data, "navigation_rebuild", 1)
            put_value(data, "navigation_snapshot", 1)
            put_value(data, "ship_flags", 0)
            put_value(data, "resource_vertical", 0)
            put_value(data, "text_selection", 0xFFFF)
            put_value(data, "active_line", 0xFFFF)
            put_value(data, "presentation_gate", 0)
            put_value(data, "exit", 0)
            put_value(data, "hud_initialized", 0)
            put_value(data, "text_active", 0)
            put_value(data, "defer", 0)
            put_value(data, "hold_ready", 0)
            put_value(data, "depth_band", 0)
            put_value(data, "sequence", 0)
            put_value(data, "request_flags", read_value(data, "request_flags") & 0xFC)
            put_value(data, "word_choice_phase", 0)
            game[0x5C21:0x5E61] = data[0x5F28:0x6168]
            game[0x5921:0x5C21] = bytes(0x300)
            put_value(data, "palette_last", 0xFF)
            put_value(data, "palette_percent", 0)
            put_value(data, "palette_increment", 10)
    elif sequence & 1 == 0:
        if defer & 1 == 0:
            put_value(data, "exit", 1)
            put_value(data, "opening", 1)

    if ran_active and presentation_active == 0:
        put_value(data, "frame_presented", 1)
        if duration == 6 and int(case.get("interpolation_complete", 0)):
            if int(case.get("layout_result", 0xFFFF)) < 0x8000:
                put_value(data, "sequence", 0)
                put_value(data, "exit", 1)

    return data, game, incoming_es, records, display, stack


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


def verify_call(
    case_name: str,
    helper_name: str,
    call: dict[str, int | str],
    case: dict[str, Any],
) -> None:
    if helper_name == "candidate_build":
        assert (call["es"], call["di"]) == (RECORDS // 16, CURRENT_TARGET), case_name
    elif helper_name == "c2":
        accepted, _fetched = accepted_candidate(case)
        assert accepted is not None
        assert (call["es"], call["di"]) == (RECORDS // 16, accepted + 4), case_name
    elif helper_name == "layout":
        assert call["si"] == 0x278D, case_name
    elif helper_name == "back_fill":
        assert call["ax"] == 0, case_name
    elif helper_name == "pbm":
        assert (call["ds"], call["si"], call["es"], call["di"]) == (
            DATA // 16,
            0x1025,
            DISPLAY // 16,
            DISPLAY_OFFSET,
        ), case_name
    elif helper_name == "palette":
        assert (
            call["ax"],
            call["bx"],
            call["cx"],
            call["dx"],
            call["di"],
            call["es"],
        ) == (0xFFCE, 0, 0, 0, 0x62E1, INCOMING_ES // 16), case_name
    elif helper_name == "fullscreen":
        assert (call["ds"], call["si"]) == (DISPLAY // 16, DISPLAY_OFFSET), case_name
    elif helper_name == "interpolate":
        assert (call["si"], call["di"]) == (0x2D4B, 0x279F), case_name


def replace_low_word(value: int, low: int) -> int:
    return (value & 0xFFFF0000) | (low & 0xFFFF)


def expected_registers(case: dict[str, Any], initial: dict[int, int]) -> dict[str, int]:
    trigger = int(case.get("trigger", 0)) & 1
    sequence = int(case.get("sequence", 0)) & 1
    exit_pending = int(case.get("exit_pending", 0)) & 1
    opening = int(case.get("opening", 0)) & 1
    presentation_active = int(case.get("presentation_active", 0)) & 1
    duration = int(case.get("duration", 6))
    if trigger:
        sequence = 1
        duration = 6 if accepted_candidate(case)[0] is None else duration

    edx = initial[UC_X86_REG_EDX]
    esi = initial[UC_X86_REG_ESI]
    edi = initial[UC_X86_REG_EDI]
    ebp = initial[UC_X86_REG_EBP]
    if trigger:
        edx &= 0xFFFF0000
        esi = replace_low_word(esi, 0x5861)
        edi = replace_low_word(edi, 0x62E1)
        _accepted, fetched = accepted_candidate(case)
        ebp = replace_low_word(ebp, 0x2E03 + 2 * fetched)
    elif exit_pending and not opening:
        esi = replace_low_word(esi, 0x6168)
        edi = replace_low_word(edi, 0x5C21)
    elif (sequence or (exit_pending and opening)) and not presentation_active:
        esi = replace_low_word(esi, DISPLAY_OFFSET)
        if duration == 6:
            esi = replace_low_word(esi, 0x2D4B)
            edi = replace_low_word(edi, 0x279F)
            if int(case.get("interpolation_complete", 0)):
                esi = replace_low_word(esi, 0x278D)

    return {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": edx,
        "esi": esi,
        "edi": edi,
        "ebp": ebp,
        "sp": STACK_POINTER + 2,
        "ds": DATA // 16,
        "es": GAME // 16 if exit_pending and not opening else INCOMING_ES // 16,
        "fs": initial[UC_X86_REG_FS],
        "gs": GAME // 16,
        "ss": STACK // 16,
    }


def expected_flag_mask(case: dict[str, Any]) -> int:
    exit_pending = int(case.get("exit_pending", 0)) & 1
    opening = int(case.get("opening", 0)) & 1
    trigger = int(case.get("trigger", 0)) & 1
    sequence = int(case.get("sequence", 0)) & 1
    presentation_active = int(case.get("presentation_active", 0)) & 1
    if exit_pending and not opening:
        return 0x044
    if trigger or sequence or (exit_pending and opening):
        if presentation_active:
            return 0
        duration = int(case.get("duration", 6))
        if duration != 6:
            return 0x085
        if not int(case.get("interpolation_complete", 0)):
            return 0x044
        result = int(case.get("layout_result", 0xFFFF))
        return (
            (0x080 if result & 0x8000 else 0)
            | (0x040 if result == 0 else 0)
            | (0x004 if (result & 0xFF).bit_count() % 2 == 0 else 0)
        )
    return 0x044 if int(case.get("defer", 0)) & 1 == 0 else 0


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


def execute(executable: bytes, case: dict[str, Any], case_index: int):
    name = str(case["name"])
    before = initialize(case, case_index)
    expected = expected_images(case, before)
    data, game, incoming_es, records, display, stack = before
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
    helper_returns_seen = {
        helper_name: set() for helper_name, _returns in HELPERS.values()
    }
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
                "bx": cpu.reg_read(UC_X86_REG_BX),
                "cx": cpu.reg_read(UC_X86_REG_CX),
                "dx": cpu.reg_read(UC_X86_REG_DX),
                "si": cpu.reg_read(UC_X86_REG_SI),
                "di": cpu.reg_read(UC_X86_REG_DI),
                "bp": cpu.reg_read(UC_X86_REG_BP),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "sp": sp,
                "return_ip": return_ip,
            }
            verify_call(name, helper_name, call, case)
            calls.append(call)
            if helper_name == "candidate_build":
                cpu.reg_write(UC_X86_REG_ES, RECORDS // 16)
                cpu.reg_write(UC_X86_REG_DI, RECORD_BASE)
            elif helper_name == "layout":
                cpu.reg_write(UC_X86_REG_AX, int(case.get("layout_result", 0xFFFF)))
            elif helper_name == "interpolate":
                flags = cpu.reg_read(UC_X86_REG_EFLAGS)
                cpu.reg_write(
                    UC_X86_REG_EFLAGS,
                    flags | 1
                    if int(case.get("interpolation_complete", 0))
                    else flags & ~1,
                )
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
        machine.emu_start(ROUTINE[0], 0, count=5_000)
    except UcError as error:
        raise RuntimeError(
            f"{name}: failed at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (name, "did not return")
    assert [call["name"] for call in calls] == case["calls"], (
        name,
        calls,
        case["calls"],
    )

    actual_images = (
        bytes(machine.mem_read(DATA, SEGMENT_SIZE)),
        bytes(machine.mem_read(GAME, SEGMENT_SIZE)),
        bytes(machine.mem_read(INCOMING_ES, SEGMENT_SIZE)),
        bytes(machine.mem_read(RECORDS, SEGMENT_SIZE)),
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

    wanted_registers = expected_registers(case, initial)
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
        name: machine.reg_read(register) for name, register in register_ids.items()
    }
    assert actual_registers == wanted_registers, (
        name,
        actual_registers,
        wanted_registers,
    )
    flag_mask = 0x8C5
    assert machine.reg_read(UC_X86_REG_EFLAGS) & flag_mask == expected_flag_mask(
        case
    ), (
        name,
        hex(machine.reg_read(UC_X86_REG_EFLAGS)),
        hex(expected_flag_mask(case)),
    )
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    data_after, game_after, incoming_after, records_after = actual_images[:4]
    accepted, _fetched = accepted_candidate(case)
    counter = (
        int(case.get("counter_link", 0))
        if int(case.get("current_kind", 2)) & 0x80
        else CURRENT_TARGET
    )
    row = {
        "name": name,
        "trigger": int(case.get("trigger", 0)),
        "sequence": int(case.get("sequence", 0)),
        "exit_pending": int(case.get("exit_pending", 0)),
        "opening": int(case.get("opening", 0)),
        "defer": int(case.get("defer", 0)),
        "presentation_active": int(case.get("presentation_active", 0)),
        "duration": int(case.get("duration", 6)),
        "interpolation_complete": int(case.get("interpolation_complete", 0)),
        "layout_result": int(case.get("layout_result", 0xFFFF)),
        "candidate": int(case.get("candidate", 0)),
        "candidate_visible": int(case.get("candidate_visible", 1)),
        "candidate_relation": int(case.get("candidate_relation", 0x5555)),
        "second_candidate": int(case.get("second_candidate", 0)),
        "second_visible": int(case.get("second_visible", 1)),
        "second_relation": int(case.get("second_relation", 0x6666)),
        "record_base_offset": RECORD_BASE,
        "filter_flags": int(case.get("filter_flags", 0)),
        "ark": int(case.get("ark", DEFAULT_ARK)),
        "redirected": int(int(case.get("current_kind", 2)) & 0x80 != 0),
        "accepted_candidate": accepted or 0,
        "access_count_after": read_word(records_after, counter + 0x14),
        "calls": calls,
        "state_after": {field: read_value(data_after, field) for field in FIELDS},
        "data_sha256": hashlib.sha256(data_after).hexdigest(),
        "game_sha256": hashlib.sha256(game_after).hexdigest(),
        "incoming_es_sha256": hashlib.sha256(incoming_after).hexdigest(),
        "record_sha256": hashlib.sha256(records_after).hexdigest(),
        "registers_after": actual_registers,
        "defined_flags": machine.reg_read(UC_X86_REG_EFLAGS) & flag_mask,
        "return": "near",
    }
    return row, covered_edges


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
        raise SystemExit(f"BBB ship-navigation body changed: {body_digest}")

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
        f"verified {len(rows)} BBB ship-navigation cases and "
        f"{len(covered_edges)} conditional edges across "
        f"{len({source for source, _ in covered_edges})} branch sites"
    )


if __name__ == "__main__":
    main()
