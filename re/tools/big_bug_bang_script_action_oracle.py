#!/usr/bin/env python3
"""Execute Big Bug Bang's complete post-frame action dispatcher.

The original 0x613F dispatcher and 0x6633 field-offset helper execute
unmodified under Unicorn. External renderer, audio, roster, and nested-script
boundaries are captured. The first 32 cases mirror Commander Blood's native
oracle; one additional case proves BBB's sequel-only travel-gate bypass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
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
DISPATCHER = (0x613F, 0x65E8)
FIELD_HELPER = (0x6633, 0x6644)
DISPATCHER_SHA256 = "c897f0f92da4b6befc887b569b7269e100caab140489fe28b987b6c41ec3d57e"
FIELD_HELPER_SHA256 = "fb7ec0e721e99c38e166f3c8538a44a18b99e12810b183c3b7659b660f955520"
DEFECT_BOUNDARY = 0x6343

CODE_SEGMENT = 0x502
GLOBALS = 0x30000
STATE = 0x50000
DECOY = 0x70000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
RETURN_LINEAR = CODE_SEGMENT * 16 + RETURN_IP
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

OWNER = 0x1800
RECORD = 0x2800
RELATED = 0x3800
PRIOR = 0x4800
PRIMARY = 0x5000
PRIMARY_RELATED = 0x5800
SOURCE_POSITION = 0x6000
FIELD_MATRIX = 0x7128
FIELD_MATRIX_FILE = 0x16918
FIELD_MATRIX_SIZE = 21 * 16

GLOBALS_OFFSETS = {
    "sequel_travel": 0x0CF1,
    "c2_gate": 0x2200,
    "interface_word_a": 0x21F3,
    "interface_word_b": 0x21F5,
    "ship_flags": 0x2745,
    "current_target": 0x276D,
    "interface_reset": 0x277B,
    "camera_view_active": 0x2A25,
    "camera_countdown": 0x2A26,
    "phase": 0x2A2D,
    "ui": 0x2A33,
    "word_choice": 0x2A77,
    "interface_byte": 0x2A78,
    "screen_rebuild": 0x2A79,
    "approach": 0x2A7F,
    "actor_busy": 0x2D1B,
    "draw_framebuffer": 0x55E9,
    "saved_framebuffer": 0x55ED,
    "record_segment": 0x6AEE,
    "arche": 0x6B22,
    "wildcard": 0x6B1E,
    "owner": 0x6B2C,
    "primary": 0x6B30,
    "active_line": 0x6B5A,
    "replacement_kind": 0x6B64,
    "replacement_related": 0x6B66,
    "active_counterpart": 0x6B6A,
    "request_flags": 0x6B80,
    "pair_guard": 0x6B8C,
    "scene_offset": 0x6BDC,
    "voc_enabled": 0x0CE7,
    "audio_request": 0x0D23,
    "clip_state": 0x0D43,
    "music_changed": 0x0DAB,
}

EXTERNALS = {
    0x65E8: ("remove", "near"),
    0x6606: ("insert", "near"),
    0x67B8: ("position", "far"),
    0x83E2: ("cod", "near"),
    0x8450: ("descript", "far"),
    0xCE6A: ("plane", "far"),
    0xD33A: ("driver", "far"),
    0xD561: ("source", "far"),
    0xD363: ("start", "far"),
    0xD05D: ("clip", "far"),
    0x464E: ("transition", "far"),
    0x9EA6: ("hud_reset", "far"),
}

CASES = (
    {"name": "unknown_record_is_ignored", "record_kind": 0xD7},
    {
        "name": "c1_arche_waits_for_approach_phase_four",
        "record_kind": 0xC1,
        "owner_kind": 0x10,
        "owner_is_arche": True,
        "approach": 3,
    },
    {
        "name": "c1_ship_relinks_and_copies_position",
        "record_kind": 0xC1,
        "owner_kind": 0x10,
    },
    {
        "name": "c1_special_inactive_still_copies_position",
        "record_kind": 0xC1,
        "owner_kind": 0x200,
        "ship_flags": 0,
    },
    {
        "name": "c1_special_descript_failure_skips_hud_reset",
        "record_kind": 0xC1,
        "owner_kind": 0x200,
        "ship_flags": 1,
        "descript_result": 0,
    },
    {
        "name": "c1_special_same_target_resets_hud_and_pair",
        "record_kind": 0xC1,
        "owner_kind": 0x200,
        "ship_flags": 1,
        "same_target": True,
        "primary_kind": 0xC4,
    },
    {
        "name": "c1_special_changed_target_resets_hud_without_audio",
        "record_kind": 0xC1,
        "owner_kind": 0x200,
        "ship_flags": 1,
        "descript_result": 1,
        "music_changed": 0,
    },
    {
        "name": "c1_special_changed_target_runs_audio_chain",
        "record_kind": 0xC1,
        "owner_kind": 0x200,
        "ship_flags": 1,
        "descript_result": 1,
        "music_changed": 1,
        "primary_kind": 0xC4,
    },
    {
        "name": "c1_null_position_source_stops_after_resolve",
        "record_kind": 0xC1,
        "owner_kind": 0x10,
        "position_result": 0,
    },
    {
        "name": "c2_full_special_list_returns_normally",
        "record_kind": 0xC2,
        "related_kind": 2,
        "insert_success": False,
    },
    {
        "name": "c2_character_state_before_shipped_stack_defect",
        "record_kind": 0xC2,
        "related_kind": 2,
        "insert_success": True,
        "defect_boundary": True,
    },
    {
        "name": "c2_descript_state_before_shipped_stack_defect",
        "record_kind": 0xC2,
        "related_kind": 0x400,
        "insert_success": True,
        "descript_result": 1,
        "defect_boundary": True,
    },
    {
        "name": "c3_nonwildcard_promotes_to_c4",
        "record_kind": 0xC3,
        "record_related": 0x3100,
    },
    {
        "name": "c3_wildcard_claims_owner_while_ui_inactive",
        "record_kind": 0xC3,
        "ui": 0,
    },
    {
        "name": "c3_wildcard_requests_audio_but_honors_busy_state",
        "record_kind": 0xC3,
        "ui": 1,
        "voc_enabled": 0,
        "clip_state": 3,
    },
    {
        "name": "c3_wildcard_plays_radio_clip",
        "record_kind": 0xC3,
        "ui": 1,
        "voc_enabled": 1,
        "clip_state": 0,
    },
    {"name": "c4_nonzero_value_is_ignored", "record_kind": 0xC4, "record_value": 1},
    {
        "name": "c4_pair_write_guard_is_honored",
        "record_kind": 0xC4,
        "record_value": 0,
        "pair_guard": 1,
    },
    {
        "name": "c4_actor_owner_updates_related_counter",
        "record_kind": 0xC4,
        "record_value": 0,
        "owner_kind": 1,
        "related_kind": 2,
        "counter_field": 0x36,
    },
    {
        "name": "c4_actor_related_updates_owner_counter",
        "record_kind": 0xC4,
        "record_value": 0,
        "owner_kind": 2,
        "related_kind": 1,
        "counter_field": 0x36,
    },
    {
        "name": "c4_nonactors_only_write_reciprocal",
        "record_kind": 0xC4,
        "record_value": 0,
        "owner_kind": 0x10,
        "related_kind": 2,
    },
    {
        "name": "c6_phase_zero_waits_for_actor",
        "record_kind": 0xC6,
        "phase": 0,
        "actor_busy": 0,
    },
    {
        "name": "c6_phase_zero_starts_camera_transition",
        "record_kind": 0xC6,
        "phase": 0,
        "actor_busy": 1,
    },
    {
        "name": "c6_phase_one_waits_for_camera",
        "record_kind": 0xC6,
        "phase": 1,
        "camera_countdown": 5,
    },
    {
        "name": "c6_phase_one_starts_line_44",
        "record_kind": 0xC6,
        "phase": 1,
        "camera_countdown": 0,
        "actor_busy": 1,
        "camera_view_active": 1,
    },
    {
        "name": "c6_final_phase_waits_for_presentation_gate",
        "record_kind": 0xC6,
        "phase": 2,
        "camera_countdown": 0,
        "c2_gate": 1,
    },
    {
        "name": "c6_final_phase_uses_matching_position_pair",
        "record_kind": 0xC6,
        "owner_kind": 0x10,
        "related_kind": 0x100,
        "phase": 2,
        "camera_countdown": 0,
        "owner_relation": 0x1234,
        "related_compare": 0x1234,
    },
    {
        "name": "c6_final_phase_uses_mismatching_position_pair",
        "record_kind": 0xC6,
        "owner_kind": 0x10,
        "related_kind": 0x100,
        "phase": 2,
        "camera_countdown": 0,
        "owner_relation": 0x1234,
        "related_compare": 0x5678,
    },
    {
        "name": "c9_clears_matching_reciprocal",
        "record_kind": 0xC9,
        "related_kind": 2,
        "reciprocal_kind": 0xC4,
        "reciprocal_owner_matches": True,
    },
    {
        "name": "c9_preserves_nonmatching_reciprocal",
        "record_kind": 0xC9,
        "related_kind": 2,
        "reciprocal_kind": 0xC4,
        "reciprocal_owner_matches": False,
    },
    {
        "name": "cd_restores_related_link_and_replaces_record",
        "record_kind": 0xCD,
        "related_kind": 0x80,
    },
    {
        "name": "cd_descript_queues_line_43",
        "record_kind": 0xCD,
        "related_kind": 0x400,
        "descript_result": 1,
    },
    {
        "name": "c1_sequel_gate_bypasses_arche_wait",
        "record_kind": 0xC1,
        "owner_kind": 0x10,
        "owner_is_arche": True,
        "approach": 3,
        "sequel_travel": 1,
    },
)


def put_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def put_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def field_offset(executable: bytes, selector: int, kind: int) -> int:
    column = (kind & -kind).bit_length() - 1
    assert 0 <= selector < 21 and 0 <= column < 16
    return executable[FIELD_MATRIX_FILE + selector * 16 + column]


def near_return(cpu: Uc) -> None:
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip = struct.unpack("<H", cpu.mem_read(stack, 2))[0]
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 2) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def far_return(cpu: Uc) -> None:
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(stack, 4))
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def changes(before: bytes | bytearray, after: bytes) -> list[list[int]]:
    return [
        [offset, old, new]
        for offset, (old, new) in enumerate(zip(before, after, strict=True))
        if old != new
    ]


def initialize_images(executable: bytes, case: dict[str, object], case_index: int):
    globals_before = bytearray(
        (offset * 7 + (offset >> 8) * 13 + case_index * 17 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    state_before = bytearray(
        (offset * 11 + (offset >> 8) * 19 + case_index * 23 + 0x53) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    decoy_before = bytes(
        (offset * 29 + case_index * 31 + 0x75) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    stack_before = bytearray(
        (offset * 37 + case_index * 41 + 0x97) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    globals_before[FIELD_MATRIX : FIELD_MATRIX + FIELD_MATRIX_SIZE] = executable[
        FIELD_MATRIX_FILE : FIELD_MATRIX_FILE + FIELD_MATRIX_SIZE
    ]

    owner_kind = int(case.get("owner_kind", 0x40))
    related_kind = int(case.get("related_kind", 0x80))
    related = int(case.get("record_related", RELATED))
    put_word(globals_before, GLOBALS_OFFSETS["record_segment"], STATE // 16)
    put_word(
        globals_before,
        GLOBALS_OFFSETS["arche"],
        OWNER if case.get("owner_is_arche") else 0x1111,
    )
    globals_before[GLOBALS_OFFSETS["approach"]] = int(case.get("approach", 4))
    globals_before[GLOBALS_OFFSETS["sequel_travel"]] = int(case.get("sequel_travel", 0))
    put_word(
        globals_before, GLOBALS_OFFSETS["ship_flags"], int(case.get("ship_flags", 0))
    )
    put_word(
        globals_before,
        GLOBALS_OFFSETS["current_target"],
        related if case.get("same_target") else 0x2222,
    )
    put_word(globals_before, GLOBALS_OFFSETS["primary"], PRIMARY)
    put_word(globals_before, GLOBALS_OFFSETS["scene_offset"], 0xA1F8)
    globals_before[GLOBALS_OFFSETS["word_choice"]] = 0xD7
    globals_before[GLOBALS_OFFSETS["request_flags"]] = int(case.get("request_flags", 0))
    globals_before[GLOBALS_OFFSETS["c2_gate"]] = int(case.get("c2_gate", 0))
    put_word(globals_before, GLOBALS_OFFSETS["interface_reset"], 0xA529)
    globals_before[GLOBALS_OFFSETS["interface_byte"]] = 0xD8
    put_word(globals_before, GLOBALS_OFFSETS["interface_word_a"], 0x5A5A)
    put_word(globals_before, GLOBALS_OFFSETS["interface_word_b"], 0xA7A7)
    put_word(globals_before, GLOBALS_OFFSETS["active_line"], 0xA888)
    put_word(globals_before, GLOBALS_OFFSETS["ui"], int(case.get("ui", 0)))
    put_word(globals_before, GLOBALS_OFFSETS["wildcard"], RELATED)
    put_word(globals_before, GLOBALS_OFFSETS["owner"], 0xA75A)
    globals_before[GLOBALS_OFFSETS["voc_enabled"]] = int(case.get("voc_enabled", 1))
    globals_before[GLOBALS_OFFSETS["audio_request"]] = 0xB8
    put_word(
        globals_before, GLOBALS_OFFSETS["clip_state"], int(case.get("clip_state", 0))
    )
    globals_before[GLOBALS_OFFSETS["pair_guard"]] = int(case.get("pair_guard", 0))
    put_word(globals_before, GLOBALS_OFFSETS["active_counterpart"], 0xA998)
    globals_before[GLOBALS_OFFSETS["phase"]] = int(case.get("phase", 0))
    globals_before[GLOBALS_OFFSETS["camera_countdown"]] = int(
        case.get("camera_countdown", 0)
    )
    globals_before[GLOBALS_OFFSETS["actor_busy"]] = int(case.get("actor_busy", 0))
    globals_before[GLOBALS_OFFSETS["camera_view_active"]] = int(
        case.get("camera_view_active", 0)
    )
    globals_before[GLOBALS_OFFSETS["screen_rebuild"]] = 0xD9
    put_word(globals_before, GLOBALS_OFFSETS["replacement_kind"], 0xC2C2)
    put_word(globals_before, GLOBALS_OFFSETS["replacement_related"], 0x3434)
    put_dword(
        globals_before, GLOBALS_OFFSETS["draw_framebuffer"], (0x1357 << 16) | 0x2468
    )
    put_dword(
        globals_before, GLOBALS_OFFSETS["saved_framebuffer"], (0x3579 << 16) | 0x468A
    )
    globals_before[GLOBALS_OFFSETS["music_changed"]] = int(case.get("music_changed", 0))

    put_word(state_before, OWNER, owner_kind)
    put_word(state_before, OWNER + 2, 0x1235)
    owner_link = field_offset(executable, 0x11, owner_kind)
    if owner_link:
        put_word(state_before, OWNER + owner_link, PRIOR)
    owner_counter = field_offset(executable, 0x08, owner_kind)
    if owner_counter:
        put_word(state_before, OWNER + owner_counter, 0x1050)
    owner_relation = field_offset(executable, 0x0E, owner_kind)
    if owner_relation:
        put_word(
            state_before,
            OWNER + owner_relation,
            int(case.get("owner_relation", 0x1234)),
        )

    put_word(state_before, RECORD, int(case["record_kind"]))
    put_word(state_before, RECORD + 2, related)
    put_word(state_before, RECORD + 4, int(case.get("record_value", 0x4567)))
    put_word(state_before, related, related_kind)
    put_word(state_before, related + 2, 0x5679)
    state_before[related + 4 : related + 16] = b"RELATED-NAME".ljust(12, b"\0")
    related_link = field_offset(executable, 0x11, related_kind)
    if related_link:
        put_word(state_before, related + related_link, 0xABCD)
    reciprocal = field_offset(executable, 0x13, related_kind)
    if reciprocal:
        put_word(
            state_before, related + reciprocal, int(case.get("reciprocal_kind", 0xB4B4))
        )
        put_word(
            state_before,
            related + reciprocal + 2,
            OWNER if case.get("reciprocal_owner_matches") else 0x9999,
        )
        put_word(state_before, related + reciprocal + 4, 0x4444)
    related_counter = field_offset(executable, 0x08, related_kind)
    if related_counter:
        put_word(state_before, related + related_counter, 0x2050)
    related_compare = field_offset(executable, 0x0C, related_kind)
    if related_compare:
        put_word(
            state_before,
            related + related_compare,
            int(case.get("related_compare", 0x5678)),
        )
    related_relation = field_offset(executable, 0x0D, related_kind)
    if related_relation:
        put_word(state_before, related + related_relation, 0x0D0D)
    related_position_a = field_offset(executable, 0x09, related_kind)
    if related_position_a:
        put_dword(state_before, related + related_position_a, 0xAABBCCDD)
    related_position_b = field_offset(executable, 0x0A, related_kind)
    if related_position_b:
        put_dword(state_before, related + related_position_b, 0x11223344)

    put_word(state_before, PRIOR, int(case.get("prior_kind", 0x20)))
    put_word(state_before, PRIOR + 2, 0x33F5)
    put_word(state_before, PRIMARY, int(case.get("primary_kind", 0)))
    put_word(state_before, PRIMARY + 2, PRIMARY_RELATED)
    put_word(state_before, PRIMARY + 4, 0xBEEF)
    put_word(state_before, PRIMARY_RELATED, 2)
    primary_action = field_offset(executable, 0x13, 2)
    put_word(state_before, PRIMARY_RELATED + primary_action, 0xC4)
    put_word(state_before, PRIMARY_RELATED + primary_action + 2, 0x7777)
    put_word(state_before, PRIMARY_RELATED + primary_action + 4, 0x8888)
    put_dword(state_before, SOURCE_POSITION, 0xCAFEBABE)

    stack_before[STACK_POINTER : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    return globals_before, state_before, decoy_before, stack_before, related


def expected_images(
    executable: bytes,
    case: dict[str, object],
    globals_before: bytearray,
    state_before: bytearray,
    related: int,
):
    expected_globals = bytearray(globals_before)
    expected_state = bytearray(state_before)
    expected_calls: list[dict[str, object]] = []
    owner_kind = int(case.get("owner_kind", 0x40))
    related_kind = int(case.get("related_kind", 0x80))
    record_kind = int(case["record_kind"])

    def model_field(selector: int, kind: int) -> int:
        result = field_offset(executable, selector, kind)
        expected_calls.append(
            {"name": "field", "selector": selector, "kind": kind, "result": result}
        )
        return result

    owner_for_position = OWNER
    sequel_bypass = bool(expected_globals[GLOBALS_OFFSETS["sequel_travel"]] & 1)
    arche_wait = (
        case.get("owner_is_arche")
        and expected_globals[GLOBALS_OFFSETS["approach"]] < 4
        and not sequel_bypass
    )
    if record_kind == 0xC1 and not arche_wait:
        owner_link = model_field(0x11, owner_kind)
        old_link = word(expected_state, OWNER + owner_link)
        if word(expected_state, old_link) == 0x20:
            expected_state[old_link + 2] &= 0xFE
        put_word(expected_state, OWNER + owner_link, related)
        put_word(expected_state, RECORD, 0)
        continue_to_position = owner_kind == 0x10
        if owner_kind == 0x200:
            continue_to_position = True
            if word(expected_globals, GLOBALS_OFFSETS["ship_flags"]) & 1:
                reset_hud = related == word(
                    expected_globals, GLOBALS_OFFSETS["current_target"]
                )
                if not reset_hud:
                    result = int(case.get("descript_result", 0))
                    expected_calls.append(
                        {
                            "name": "descript",
                            "segment": STATE // 16,
                            "name_offset": related + 4,
                            "result": result,
                        }
                    )
                    reset_hud = bool(result)
                    if (
                        reset_hud
                        and expected_globals[GLOBALS_OFFSETS["music_changed"]] & 1
                    ):
                        expected_calls.extend(
                            [
                                {"name": "plane", "draw_framebuffer": 0x3579468A},
                                {"name": "driver"},
                                {
                                    "name": "source",
                                    "ds": GLOBALS // 16,
                                    "path_offset": 0x0F7B,
                                },
                                {"name": "start"},
                            ]
                        )
                        owner_for_position = 0x0F7B
                if reset_hud:
                    if word(expected_state, PRIMARY) == 0xC4:
                        primary_related = word(expected_state, PRIMARY + 2)
                        put_word(expected_state, PRIMARY, 0)
                        put_word(expected_state, PRIMARY + 2, 0)
                        primary_kind = word(expected_state, primary_related)
                        primary_action = model_field(0x13, primary_kind)
                        for delta in (0, 2, 4):
                            put_word(
                                expected_state,
                                primary_related + primary_action + delta,
                                0,
                            )
                    put_word(
                        expected_globals, GLOBALS_OFFSETS["current_target"], related
                    )
                    put_word(expected_globals, GLOBALS_OFFSETS["ship_flags"], 9)
                    put_word(expected_globals, GLOBALS_OFFSETS["scene_offset"], 0)
                    expected_globals[GLOBALS_OFFSETS["word_choice"]] = 0
                    expected_globals[GLOBALS_OFFSETS["request_flags"]] = 0
                    expected_globals[GLOBALS_OFFSETS["c2_gate"]] = 0
                    put_word(expected_globals, GLOBALS_OFFSETS["interface_reset"], 1)
                    expected_globals[GLOBALS_OFFSETS["interface_byte"]] = 0
                    put_word(
                        expected_globals,
                        GLOBALS_OFFSETS["interface_word_b"],
                        word(expected_globals, GLOBALS_OFFSETS["interface_word_a"]),
                    )
                    put_word(expected_globals, GLOBALS_OFFSETS["active_line"], 3)
        elif owner_kind != 0x10:
            continue_to_position = False
        if continue_to_position:
            position_result = int(case.get("position_result", SOURCE_POSITION))
            expected_calls.append(
                {
                    "name": "position",
                    "record": related,
                    "compare": owner_link,
                    "result": position_result,
                }
            )
            if position_result:
                position = model_field(0x0B, owner_kind)
                expected_state[
                    owner_for_position + position : owner_for_position + position + 4
                ] = expected_state[position_result : position_result + 4]
    elif record_kind == 0xC2:
        success = bool(case.get("insert_success", True))
        expected_calls.append(
            {"name": "insert", "owner": related, "success_carry": success}
        )
        if success:
            related_link = model_field(0x11, related_kind)
            put_word(expected_state, related + related_link, 0xFFFF)
            put_word(expected_state, RECORD, 0)
            if (
                word(expected_globals, GLOBALS_OFFSETS["ui"]) & 1 == 0
                and expected_globals[GLOBALS_OFFSETS["request_flags"]] & 2 == 0
            ):
                if related_kind == 2:
                    expected_globals[GLOBALS_OFFSETS["c2_gate"]] = 0
                    put_word(expected_globals, GLOBALS_OFFSETS["active_line"], 0x27)
                elif related_kind == 0x400:
                    result = int(case.get("descript_result", 0))
                    expected_calls.append(
                        {
                            "name": "descript",
                            "segment": STATE // 16,
                            "name_offset": related + 4,
                            "result": result,
                        }
                    )
                    if result:
                        expected_globals[GLOBALS_OFFSETS["c2_gate"]] = 0
                        put_word(expected_globals, GLOBALS_OFFSETS["active_line"], 0x2B)
                        expected_globals[GLOBALS_OFFSETS["request_flags"]] |= 2
    elif record_kind == 0xC3:
        if related != word(expected_globals, GLOBALS_OFFSETS["wildcard"]):
            put_word(expected_state, RECORD, 0xC4)
            put_word(expected_state, RECORD + 4, 0)
        else:
            put_word(expected_globals, GLOBALS_OFFSETS["owner"], OWNER)
            if word(expected_globals, GLOBALS_OFFSETS["ui"]) & 1:
                if expected_globals[GLOBALS_OFFSETS["voc_enabled"]] & 1 == 0:
                    expected_globals[GLOBALS_OFFSETS["audio_request"]] |= 1
                if word(expected_globals, GLOBALS_OFFSETS["clip_state"]) == 0:
                    expected_calls.append({"name": "clip", "clip": 6})
                    put_word(expected_globals, GLOBALS_OFFSETS["clip_state"], 2)
    elif record_kind == 0xC4:
        if (
            word(expected_state, RECORD + 4) == 0
            and expected_globals[GLOBALS_OFFSETS["pair_guard"]] & 1 == 0
        ):
            put_word(expected_state, RECORD + 4, 0xFFFF)
            skip_related_actor = False
            if owner_kind == 1:
                put_word(expected_globals, GLOBALS_OFFSETS["owner"], 0)
                counter = model_field(0x08, related_kind)
                if counter:
                    put_word(
                        expected_state,
                        related + counter,
                        word(expected_state, related + counter) + 1,
                    )
                    put_word(
                        expected_state,
                        OWNER + 2,
                        word(expected_state, OWNER + 2) | 0x8000,
                    )
                    put_word(
                        expected_globals, GLOBALS_OFFSETS["active_counterpart"], related
                    )
                    expected_calls.append({"name": "cod", "object": related})
                    skip_related_actor = True
            if not skip_related_actor and related_kind == 1:
                counter = model_field(0x08, owner_kind)
                if counter:
                    put_word(
                        expected_state,
                        OWNER + counter,
                        word(expected_state, OWNER + counter) + 1,
                    )
                    put_word(
                        expected_state,
                        OWNER + 2,
                        word(expected_state, OWNER + 2) | 0x8000,
                    )
                    put_word(
                        expected_globals, GLOBALS_OFFSETS["active_counterpart"], OWNER
                    )
                    expected_calls.append({"name": "cod", "object": OWNER})
            reciprocal = model_field(0x13, related_kind)
            put_word(expected_state, related + reciprocal, 0xC4)
            put_word(expected_state, related + reciprocal + 2, OWNER)
            put_word(expected_state, related + reciprocal + 4, 0xFFFF)
    elif record_kind == 0xC6:
        phase = expected_globals[GLOBALS_OFFSETS["phase"]]
        if phase == 0:
            if expected_globals[GLOBALS_OFFSETS["actor_busy"]] == 1:
                expected_globals[GLOBALS_OFFSETS["phase"]] = 1
                expected_globals[GLOBALS_OFFSETS["camera_countdown"]] = 8
                expected_calls.append({"name": "transition", "object_id": 4})
        elif expected_globals[GLOBALS_OFFSETS["camera_countdown"]] == 0:
            if phase == 1:
                expected_globals[GLOBALS_OFFSETS["phase"]] = 2
                expected_globals[GLOBALS_OFFSETS["actor_busy"]] = 0
                expected_globals[GLOBALS_OFFSETS["camera_view_active"]] = 0
                put_word(expected_globals, GLOBALS_OFFSETS["active_line"], 0x2C)
            elif expected_globals[GLOBALS_OFFSETS["c2_gate"]] & 1 == 0:
                expected_globals[GLOBALS_OFFSETS["phase"]] = 0
                expected_globals[GLOBALS_OFFSETS["screen_rebuild"]] = 1
                expected_calls.append({"name": "hud_reset"})
                expected_globals[GLOBALS_OFFSETS["ui"]] &= 0xFB
                put_word(expected_state, RECORD, 0)
                put_word(expected_state, RECORD + 2, 0)
                put_word(expected_state, RECORD + 4, 0)
                owner_relation = model_field(0x0E, owner_kind)
                relation = word(expected_state, OWNER + owner_relation)
                owner_position = model_field(0x0B, owner_kind)
                compare = model_field(0x0C, related_kind)
                comparison = word(expected_state, related + compare)
                if relation == comparison:
                    match_relation = model_field(0x0D, related_kind)
                    relation = word(expected_state, related + match_relation)
                    related_position = model_field(0x0A, related_kind)
                else:
                    compare = model_field(0x0C, related_kind)
                    relation = comparison
                    related_position = model_field(0x09, related_kind)
                expected_state[OWNER + owner_position : OWNER + owner_position + 4] = (
                    expected_state[
                        related + related_position : related + related_position + 4
                    ]
                )
                put_word(expected_state, OWNER + owner_relation, relation)
    elif record_kind == 0xC9:
        put_word(expected_state, RECORD, 0)
        put_word(expected_state, RECORD + 2, 0)
        put_word(expected_state, RECORD + 4, 0)
        reciprocal = model_field(0x13, related_kind)
        reciprocal += related
        if (
            word(expected_state, reciprocal) == 0xC4
            and word(expected_state, reciprocal + 2) == OWNER
        ):
            put_word(expected_state, reciprocal, 0)
            put_word(expected_state, reciprocal + 2, 0)
            put_word(expected_state, reciprocal + 4, 0)
    elif record_kind == 0xCD:
        expected_calls.append(
            {
                "name": "remove",
                "owner": related,
                "success_carry": bool(case.get("remove_success", False)),
            }
        )
        related_link = model_field(0x11, related_kind)
        put_word(
            expected_state, related + related_link, word(expected_state, RECORD + 4)
        )
        put_word(
            expected_state,
            RECORD,
            word(expected_globals, GLOBALS_OFFSETS["replacement_kind"]),
        )
        put_word(
            expected_state,
            RECORD + 2,
            word(expected_globals, GLOBALS_OFFSETS["replacement_related"]),
        )
        put_word(expected_state, RECORD + 4, 0)
        if (
            word(expected_globals, GLOBALS_OFFSETS["ui"]) & 1 == 0
            and expected_globals[GLOBALS_OFFSETS["request_flags"]] & 2 == 0
            and related_kind == 0x400
        ):
            result = int(case.get("descript_result", 0))
            expected_calls.append(
                {
                    "name": "descript",
                    "segment": STATE // 16,
                    "name_offset": related + 4,
                    "result": result,
                }
            )
            if result:
                expected_globals[GLOBALS_OFFSETS["c2_gate"]] = 0
                put_word(expected_globals, GLOBALS_OFFSETS["active_line"], 0x2B)
                expected_globals[GLOBALS_OFFSETS["request_flags"]] |= 2
    return expected_globals, expected_state, expected_calls


def capture_external(cpu: Uc, name: str, case: dict[str, object]) -> dict[str, object]:
    if name in ("insert", "remove"):
        success = bool(
            case.get(
                "insert_success" if name == "insert" else "remove_success",
                name == "insert",
            )
        )
        flags = cpu.reg_read(UC_X86_REG_EFLAGS)
        cpu.reg_write(UC_X86_REG_EFLAGS, flags | 1 if success else flags & ~1)
        return {
            "name": name,
            "owner": cpu.reg_read(UC_X86_REG_AX),
            "success_carry": success,
        }
    if name == "position":
        result = int(case.get("position_result", SOURCE_POSITION))
        call = {
            "name": name,
            "record": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
            "compare": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
            "result": result,
        }
        cpu.reg_write(UC_X86_REG_AX, result)
        return call
    if name == "cod":
        return {"name": name, "object": cpu.reg_read(UC_X86_REG_BX)}
    if name == "descript":
        result = int(case.get("descript_result", 0))
        call = {
            "name": name,
            "segment": cpu.reg_read(UC_X86_REG_ES),
            "name_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
            "result": result,
        }
        cpu.reg_write(UC_X86_REG_AX, result)
        return call
    if name == "plane":
        draw = struct.unpack(
            "<I", cpu.mem_read(GLOBALS + GLOBALS_OFFSETS["draw_framebuffer"], 4)
        )[0]
        return {"name": name, "draw_framebuffer": draw}
    if name == "source":
        return {
            "name": name,
            "ds": cpu.reg_read(UC_X86_REG_DS),
            "path_offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
        }
    if name == "clip":
        return {"name": name, "clip": cpu.reg_read(UC_X86_REG_AX)}
    if name == "transition":
        return {"name": name, "object_id": cpu.reg_read(UC_X86_REG_AX)}
    return {"name": name}


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    globals_before, state_before, decoy_before, stack_before, related = (
        initialize_images(executable, case, case_index)
    )
    for address, data in (
        (GLOBALS, globals_before),
        (STATE, state_before),
        (DECOY, decoy_before),
        (STACK, stack_before),
    ):
        cpu.mem_write(address, bytes(data))

    initial_registers = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E50000 | OWNER,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x97970000 | RECORD,
        UC_X86_REG_CS: CODE_SEGMENT,
        UC_X86_REG_DS: STATE // 16,
        UC_X86_REG_ES: DECOY // 16,
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD6,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    expected_globals, expected_state, expected_calls = expected_images(
        executable, case, globals_before, state_before, related
    )
    calls: list[dict[str, object]] = []
    pending_field: dict[str, object] | None = None
    reached_stop = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal pending_field, reached_stop
        file_offset = address + HEADER_SIZE
        stop = (
            image_address(DEFECT_BOUNDARY)
            if case.get("defect_boundary")
            else RETURN_LINEAR
        )
        if address == stop:
            reached_stop = True
            machine.emu_stop()
            return
        external = EXTERNALS.get(file_offset)
        if external is not None:
            name, return_kind = external
            calls.append(capture_external(machine, name, case))
            (near_return if return_kind == "near" else far_return)(machine)
            return
        if file_offset == FIELD_HELPER[0]:
            assert pending_field is None
            pending_field = {
                "name": "field",
                "selector": machine.reg_read(UC_X86_REG_AX),
                "kind": machine.reg_read(UC_X86_REG_BX),
            }
        elif file_offset == FIELD_HELPER[1] - 1:
            assert pending_field is not None
            pending_field["result"] = machine.reg_read(UC_X86_REG_AX)
            assert pending_field["result"] == field_offset(
                executable, int(pending_field["selector"]), int(pending_field["kind"])
            )
            calls.append(pending_field)
            pending_field = None
        assert any(
            image_address(start) <= address < address + size <= image_address(end)
            for start, end in (DISPATCHER, FIELD_HELPER)
        ), (
            case["name"],
            hex(file_offset),
            hex(machine.reg_read(UC_X86_REG_CS)),
            hex(machine.reg_read(UC_X86_REG_IP)),
            hex(machine.reg_read(UC_X86_REG_SP)),
        )

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert any(
            start <= address < address + size <= start + SEGMENT_SIZE
            for start in (GLOBALS, STATE, STACK)
        ), (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(DISPATCHER[0]), 0, count=5000)
    assert reached_stop, case["name"]
    assert pending_field is None
    assert calls == expected_calls, (case["name"], calls, expected_calls)

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    state_after = bytes(cpu.mem_read(STATE, SEGMENT_SIZE))
    assert globals_after == bytes(expected_globals), case["name"]
    assert state_after == bytes(expected_state), case["name"]
    assert bytes(cpu.mem_read(DECOY, SEGMENT_SIZE)) == decoy_before
    assert bytes(cpu.mem_read(0, len(module))) == module

    if case.get("defect_boundary"):
        expected_sp = STACK_POINTER - 20
        assert cpu.reg_read(UC_X86_REG_SP) == expected_sp
        saved_frame = struct.pack(
            "<HHHHIHHI",
            initial_registers[UC_X86_REG_EBP] & 0xFFFF,
            initial_registers[UC_X86_REG_ESI] & 0xFFFF,
            initial_registers[UC_X86_REG_EDI] & 0xFFFF,
            initial_registers[UC_X86_REG_ES],
            initial_registers[UC_X86_REG_EDX] & 0xFFFFFFFF,
            initial_registers[UC_X86_REG_ECX] & 0xFFFF,
            initial_registers[UC_X86_REG_EBX] & 0xFFFF,
            initial_registers[UC_X86_REG_EAX] & 0xFFFFFFFF,
        )
        assert bytes(cpu.mem_read(STACK + expected_sp, len(saved_frame))) == saved_frame
    else:
        expected_registers = {
            UC_X86_REG_EAX: initial_registers[UC_X86_REG_EAX],
            UC_X86_REG_ECX: initial_registers[UC_X86_REG_ECX],
            UC_X86_REG_EDX: initial_registers[UC_X86_REG_EDX],
            UC_X86_REG_ESI: initial_registers[UC_X86_REG_ESI] & 0xFFFF,
            UC_X86_REG_EDI: initial_registers[UC_X86_REG_EDI] & 0xFFFF,
            UC_X86_REG_EBP: initial_registers[UC_X86_REG_EBP],
            UC_X86_REG_CS: CODE_SEGMENT,
            UC_X86_REG_DS: STATE // 16,
            UC_X86_REG_ES: DECOY // 16,
            UC_X86_REG_FS: initial_registers[UC_X86_REG_FS],
            UC_X86_REG_GS: GLOBALS // 16,
            UC_X86_REG_SS: STACK // 16,
            UC_X86_REG_SP: STACK_POINTER + 2,
        }
        for register, expected in expected_registers.items():
            assert cpu.reg_read(register) == expected, (
                case["name"],
                register,
                hex(cpu.reg_read(register)),
                hex(expected),
            )
        assert cpu.reg_read(UC_X86_REG_BX) == initial_registers[UC_X86_REG_EBX] & 0xFFFF
        assert (
            bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
            == STACK_SENTINEL
        )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    return {
        "name": case["name"],
        "record_kind": int(case["record_kind"]),
        "calls": calls,
        "stopped_before_unmatched_pop_es": bool(case.get("defect_boundary")),
        "sequel_travel_gate": bool(case.get("sequel_travel")),
        "global_changes": changes(globals_before, globals_after),
        "state_changes": changes(state_before, state_after),
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "state_sha256": hashlib.sha256(state_after).hexdigest(),
        "defined_flags": {
            name: bool(flags & mask)
            for name, mask in (
                ("cf", 0x0001),
                ("pf", 0x0004),
                ("af", 0x0010),
                ("zf", 0x0040),
                ("sf", 0x0080),
                ("of", 0x0800),
            )
        },
    }


def normalized_calls(calls: list[dict[str, object]]) -> list[tuple[object, ...]]:
    normalized = []
    for call in calls:
        name = str(call["name"])
        if name == "field":
            continue
        if name in ("insert", "remove"):
            normalized.append((name, call["success_carry"]))
        elif name == "position":
            normalized.append((name, bool(call["result"])))
        elif name == "descript":
            normalized.append((name, call["result"]))
        elif name == "plane":
            normalized.append((name, call["draw_framebuffer"]))
        elif name in ("clip", "transition"):
            normalized.append((name, call[next(key for key in call if key != "name")]))
        else:
            normalized.append((name,))
    return normalized


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander_rows = json.loads(commander_path.read_text())
    assert len(commander_rows) == len(CASES) - 1
    for sequel, commander in zip(rows, commander_rows, strict=False):
        assert sequel["name"] == commander["name"]
        assert sequel["record_kind"] == commander["record_kind"]
        assert (
            sequel["stopped_before_unmatched_pop_es"]
            == commander["stopped_before_unmatched_pop_es"]
        )
        assert normalized_calls(sequel["calls"]) == normalized_calls(
            commander["calls"]
        ), sequel["name"]
    assert rows[-1]["name"] == "c1_sequel_gate_bypasses_arche_wait"
    assert normalized_calls(rows[-1]["calls"]) == [
        ("position", True),
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_5b38_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    for (start, end), expected in (
        (DISPATCHER, DISPATCHER_SHA256),
        (FIELD_HELPER, FIELD_HELPER_SHA256),
    ):
        actual = hashlib.sha256(executable[start:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"native span {start:#x}..{end:#x} changed")
    assert executable[DEFECT_BOUNDARY] == 0x07
    assert executable[DISPATCHER[1] - 1] == 0xC3
    assert tuple(
        field_offset(executable, 0x13, kind) for kind in (1, 2, 0x10, 0x200)
    ) == (8, 58, 28, 10)

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB action-dispatch cases")


if __name__ == "__main__":
    main()
