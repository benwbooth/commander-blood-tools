#!/usr/bin/env python3
"""Execute Big Bug Bang's location-information panel and candidate filter.

The original 0xA5E0 dispatcher and its 0xA98B candidate-list helper execute
unmodified. Established resource, renderer, source-list, text, statistic, and
geometry boundaries are captured while all persistent mutations are modeled.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AL,
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
DISPATCHER = (0xA5E0, 0xA98B)
DISPATCHER_SHA256 = "737b757a50f829ff4a82425baddf8a84994cf43cba05703ea6d71efb82a38e8d"
CANDIDATE_FILTER = (0xA98B, 0xA9D3)
CANDIDATE_FILTER_SHA256 = (
    "4e1f2d3f2e67c4d39129a1dc37c70dc0853bbd3f309020f7a2f3ed7258dd0439"
)

GLOBALS = 0x30000
INCOMING_ES = 0x50000
RECORDS = 0x70000
RESOURCE = 0xB0000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
CONTEXT_OFFSET = 0x7A00
COMPARISON_OFFSET = 0x6200
COMPARISON_SEGMENT = 0xBEEF
SELECTED = 0x1800
EXCLUDED = 0x1900
LOCATION_A = 0x2000
LOCATION_B = 0x2100
WRONG_KIND = 0x2200
INACTIVE_LOCATION = 0x2300
DETAIL_WRONG = 0x3000
DETAIL_ACTOR = 0x3100
DETAIL_INACTIVE = 0x3200
DETAIL_NEGATIVE = 0x3300
RESOURCE_OFFSET = 0x4100
FRAME_OFFSET = 0x4400
NUMBER_BUFFER = 0x4000

OFFSETS = {
    "mouse_x": 0x0C22,
    "mouse_y": 0x0C24,
    "primary_pressed": 0x0C36,
    "interpolation_current": 0x0CE4,
    "interpolation_total": 0x0CE3,
    "source_width": 0x2A0C,
    "target_rect": 0x2A0E,
    "hovered_location": 0x2A16,
    "phase": 0x2A22,
    "artwork_present": 0x2A23,
    "scale": 0x2A24,
    "panel_active": 0x2A27,
    "choice_mode": 0x2A2E,
    "candidate_count": 0x2A2F,
    "selected": 0x2A5F,
    "text_width": 0x2A6D,
    "current_rect": 0x2D4B,
    "candidate_list": 0x2E63,
    "artwork": 0x2F97,
    "entity_frame_pointer": 0x65E6,
    "record_segment": 0x6AEE,
    "excluded_location": 0x6B28,
    "deferred": 0x6B3C,
    "panel_ready": 0x6B7E,
    "source_list": 0x6C2E,
    "resource_pointer": 0x0C74,
}

LABELS = {
    0x012D: "PLANETE: ",
    0x0137: "VAISSEAU: ",
    0x0142: "TROU NOIR: ",
    0x014E: "VIE PRESENTE:",
    0x01C2: "POPULATION:",
    0x01CE: "AGRESSIVITE:",
    0x01DB: "ENERGIE:",
    0x01E4: "EVOLUTION:",
    0x01EF: "CHEF",
    0x01F4: "LIEU:",
}

EXTERNALS = {
    0x2924: ("compare", "far"),
    0x4444: ("resource", "far"),
    0x45CB: ("setter", "far"),
    0x2660: ("palette", "far"),
    0xA9D3: ("entity", "near"),
    0x48EE: ("render", "far"),
    0x20CE: ("interpolate", "far"),
    0x371E: ("remap", "far"),
    0x3512: ("text", "far"),
    0x685D: ("source_list", "far"),
    0x2832: ("integer", "far"),
    0x40E9: ("stat", "far"),
    0x464E: ("transition", "far"),
}

EMPTY: dict[int, list[int]] = {}
DETAILS = {SELECTED: [DETAIL_WRONG, DETAIL_ACTOR]}
CANDIDATES = {
    SELECTED: [EXCLUDED, WRONG_KIND, INACTIVE_LOCATION, LOCATION_A, LOCATION_B]
}
CANDIDATE_DETAILS = {
    **CANDIDATES,
    LOCATION_B: [DETAIL_WRONG, DETAIL_ACTOR],
}
SINGLE_CANDIDATE = {
    SELECTED: [LOCATION_A],
    LOCATION_A: [DETAIL_WRONG, DETAIL_ACTOR],
}
INACTIVE_DETAILS = {SELECTED: [DETAIL_INACTIVE, DETAIL_ACTOR]}
NEGATIVE_DETAILS = {SELECTED: [DETAIL_NEGATIVE]}

CASES = (
    {"name": "opening_continues_without_repeating_setup", "phase": 1, "scale": 3},
    {
        "name": "opening_completion_enters_steady_state",
        "phase": 1,
        "scale": 7,
        "complete": True,
    },
    {
        "name": "first_open_frame_scans_to_second_art_entry",
        "phase": 1,
        "scale": 0,
        "record_name": "TARGET",
    },
    {
        "name": "first_open_frame_tolerates_missing_art_entry",
        "phase": 1,
        "scale": 0,
        "record_name": "MISSING",
    },
    {
        "name": "steady_planet_draws_first_eligible_actor_details",
        "phase": 0,
        "kind": 0x0008,
        "source_lists": DETAILS,
    },
    {"name": "steady_ship_selects_ship_title", "phase": 0, "kind": 0x0010},
    {
        "name": "steady_black_hole_title_overrides_ship_bit",
        "phase": 0,
        "kind": 0x0110,
    },
    {"name": "unrecognized_state_bits_follow_steady_path", "phase": 0x80},
    {"name": "closing_decrements_scale_and_waits", "phase": 2, "scale": 5},
    {
        "name": "closing_completion_releases_entity_and_links",
        "phase": 2,
        "scale": 1,
        "complete": True,
    },
    {
        "name": "mouse_close_arms_delayed_transition",
        "phase": 0,
        "scale": 5,
        "primary": 1,
    },
    {
        "name": "planet_multiple_locations_lists_candidates",
        "phase": 0,
        "kind": 0x0008,
        "source_lists": CANDIDATES,
    },
    {
        "name": "candidate_hover_highlights_second_location",
        "phase": 0,
        "kind": 0x0008,
        "mouse": (120, 47),
        "source_lists": CANDIDATES,
    },
    {
        "name": "candidate_click_selects_second_location",
        "phase": 0,
        "kind": 0x0008,
        "mouse": (120, 47),
        "primary": 1,
        "source_lists": CANDIDATES,
    },
    {
        "name": "selected_location_draws_first_eligible_actor_details",
        "phase": 0,
        "kind": 0x0008,
        "choice_mode": 2,
        "hovered_location": LOCATION_B,
        "source_lists": CANDIDATE_DETAILS,
    },
    {
        "name": "selected_location_click_returns_to_candidate_list",
        "phase": 0,
        "kind": 0x0008,
        "choice_mode": 2,
        "hovered_location": LOCATION_B,
        "primary": 1,
        "source_lists": CANDIDATE_DETAILS,
    },
    {
        "name": "single_candidate_auto_selects_and_draws_details",
        "phase": 0,
        "kind": 0x0008,
        "source_lists": SINGLE_CANDIDATE,
    },
    {
        "name": "single_selected_candidate_click_closes_panel",
        "phase": 0,
        "kind": 0x0008,
        "primary": 1,
        "source_lists": SINGLE_CANDIDATE,
    },
    {
        "name": "inactive_presentable_actor_stops_detail_scan",
        "phase": 0,
        "kind": 0x0008,
        "source_lists": INACTIVE_DETAILS,
    },
    {
        "name": "signed_statistics_only_clamp_negative_energy_bar",
        "phase": 0,
        "kind": 0x0008,
        "source_lists": NEGATIVE_DETAILS,
    },
)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def read8(data: bytes | bytearray, offset: int) -> int:
    return data[offset]


def write8(data: bytearray, offset: int, value: int) -> None:
    data[offset] = value & 0xFF


def read16(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def write16(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def write_far_pointer(data: bytearray, offset: int, segment: int, pointer: int) -> None:
    struct.pack_into("<HH", data, offset, pointer, segment)


def write_string(data: bytearray, offset: int, value: str, size: int) -> None:
    encoded = value.encode("ascii")
    assert len(encoded) < size
    data[offset : offset + size] = bytes(size)
    data[offset : offset + len(encoded)] = encoded


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 17 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def source_lists(case: dict[str, object]) -> dict[int, list[int]]:
    return {
        int(owner): [int(source) for source in sources]
        for owner, sources in dict(case.get("source_lists", EMPTY)).items()
    }


def initialize(
    case: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray, bytearray, bytearray]:
    globals_before = seeded_segment(case_index, 7, 0x21)
    records_before = seeded_segment(case_index, 5, 0x65)
    incoming_es_before = seeded_segment(case_index, 11, 0x43)
    resource_before = seeded_segment(case_index, 3, 0x87)
    phase = int(case.get("phase", 0))
    scale = int(case.get("scale", 4))
    complete = bool(case.get("complete", False))
    mouse_x, mouse_y = (int(value) for value in case.get("mouse", (300, 190)))

    write8(globals_before, OFFSETS["phase"], phase)
    write8(globals_before, OFFSETS["scale"], scale)
    write8(globals_before, OFFSETS["artwork_present"], 1)
    write8(globals_before, OFFSETS["panel_active"], 1)
    write8(globals_before, OFFSETS["choice_mode"], int(case.get("choice_mode", 0)))
    write8(globals_before, OFFSETS["candidate_count"], 0)
    write16(
        globals_before,
        OFFSETS["hovered_location"],
        int(case.get("hovered_location", 0)),
    )
    write8(globals_before, OFFSETS["primary_pressed"], int(case.get("primary", 0)))
    write8(globals_before, OFFSETS["interpolation_total"], 8)
    write8(globals_before, OFFSETS["interpolation_current"], 8 if complete else 3)
    write16(globals_before, OFFSETS["mouse_x"], mouse_x)
    write16(globals_before, OFFSETS["mouse_y"], mouse_y)
    write16(globals_before, OFFSETS["selected"], SELECTED)
    write16(globals_before, OFFSETS["deferred"], 0x6A6A)
    write16(globals_before, OFFSETS["excluded_location"], EXCLUDED)
    write16(globals_before, OFFSETS["record_segment"], RECORDS // 16)
    write16(globals_before, OFFSETS["source_width"], 777)
    write16(globals_before, OFFSETS["text_width"], 0x6D6D)
    write8(globals_before, OFFSETS["panel_ready"], 0x7E)
    struct.pack_into("<hhhh", globals_before, OFFSETS["target_rect"], 110, 25, 96, 70)
    struct.pack_into("<hhhh", globals_before, OFFSETS["current_rect"], 123, 77, 4, 4)
    write_far_pointer(
        globals_before,
        OFFSETS["resource_pointer"],
        RESOURCE // 16,
        RESOURCE_OFFSET,
    )
    write_far_pointer(
        globals_before,
        OFFSETS["entity_frame_pointer"],
        RESOURCE // 16,
        FRAME_OFFSET,
    )
    for offset, label in LABELS.items():
        write_string(globals_before, offset, label, len(label) + 1)
    write_string(globals_before, OFFSETS["artwork"], "OTHER", 16)
    write16(globals_before, OFFSETS["artwork"] + 16, 0x0020)
    write16(globals_before, OFFSETS["artwork"] + 18, 0x001F)
    write_string(globals_before, OFFSETS["artwork"] + 22, "TARGET", 16)
    write16(globals_before, OFFSETS["artwork"] + 38, 0x005E)
    write16(globals_before, OFFSETS["artwork"] + 40, 0x001F)
    globals_before[OFFSETS["artwork"] + 44] = 0

    write16(records_before, SELECTED, int(case.get("kind", 0)))
    write16(records_before, SELECTED + 2, 3)
    write_string(
        records_before, SELECTED + 4, str(case.get("record_name", "TARGET")), 0x32
    )
    records = {
        EXCLUDED: (0x0080, 2, "ARCHE"),
        LOCATION_A: (0x0080, 2, "FIRST"),
        LOCATION_B: (0x0080, 2, "SECOND"),
        WRONG_KIND: (0x0002, 2, "WRONGKIND"),
        INACTIVE_LOCATION: (0x0080, 0, "INACTIVE"),
        DETAIL_WRONG: (0x0002, 1, "WRONGDETAIL"),
        DETAIL_ACTOR: (0x0002, 5, "LEADER"),
        DETAIL_INACTIVE: (0x0002, 4, "INACTIVELEADER"),
        DETAIL_NEGATIVE: (0x0002, 5, "NEGATIVE"),
    }
    for offset, (kind, flags, name) in records.items():
        write16(records_before, offset, kind)
        write16(records_before, offset + 2, flags)
        write_string(records_before, offset + 4, name, 0x32)
    write16(records_before, DETAIL_ACTOR + 0x16, 1234)
    write16(records_before, DETAIL_ACTOR + 0x32, 99)
    write16(records_before, DETAIL_ACTOR + 0x34, 401)
    write16(records_before, DETAIL_ACTOR + 0x38, 78)
    for field in (0x16, 0x32, 0x34, 0x38):
        write16(records_before, DETAIL_NEGATIVE + field, 0xFFE7)

    write16(resource_before, FRAME_OFFSET, 0x012F)
    write16(resource_before, FRAME_OFFSET + 2, 0x0030)
    write16(globals_before, STACK_POINTER, RETURN_IP)
    globals_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    write16(globals_before, CONTEXT_OFFSET + 4, COMPARISON_OFFSET)
    write16(globals_before, CONTEXT_OFFSET + 6, COMPARISON_SEGMENT)
    return globals_before, records_before, incoming_es_before, resource_before


def title_offset(kind: int, choice_mode: int) -> int:
    if choice_mode & 2:
        return 0x01F4
    if kind & 0x0100:
        return 0x0142
    if kind & 0x0010:
        return 0x0137
    return 0x012D


def candidate_records(records: bytes | bytearray, direct: list[int]) -> list[int]:
    return [
        record
        for record in direct
        if record != EXCLUDED
        and read16(records, record) & 0x0080
        and read8(records, record + 2) & 2
    ]


def eligible_detail_record(records: bytes | bytearray, direct: list[int]) -> int | None:
    for record in direct:
        if read16(records, record) & 2 and read8(records, record + 2) & 4:
            return record if read8(records, record + 2) & 1 else None
    return None


def write_source_list(data: bytearray, sources: list[int]) -> None:
    for index, value in enumerate([*sources, 0xFFFF]):
        write16(data, OFFSETS["source_list"] + index * 2, value)


def expected(
    case: dict[str, object], globals_before: bytearray, records: bytearray
) -> tuple[bytearray, list[str]]:
    after = bytearray(globals_before)
    phase = int(case.get("phase", 0))
    scale = int(case.get("scale", 4))
    complete = bool(case.get("complete", False))
    lists = source_lists(case)
    names: list[str] = []

    def source_list(owner: int) -> list[int]:
        values = lists.get(owner, [])
        names.append("source_list")
        write_source_list(after, values)
        return values

    def draw_text(offset: int, record_segment: bool = False) -> None:
        names.append("text")
        source = records if record_segment else after
        write16(after, OFFSETS["text_width"], len(read_c_string(source, offset)) * 5)

    def format_integer(value: int) -> None:
        names.append("integer")
        signed = struct.unpack("<h", struct.pack("<H", value))[0]
        encoded = str(signed).encode("ascii") + b"\0"
        after[NUMBER_BUFFER : NUMBER_BUFFER + len(encoded)] = encoded

    def draw_content() -> None:
        names.extend(("render", "remap"))
        kind = read16(records, SELECTED)
        mode = read8(after, OFFSETS["choice_mode"])
        candidates: list[int] = []
        if kind & 8:
            candidates = candidate_records(records, source_list(SELECTED))
            write8(after, OFFSETS["candidate_count"], len(candidates))
            for index, record in enumerate(candidates):
                write16(after, OFFSETS["candidate_list"] + index * 2, record)
        if len(candidates) == 1:
            write16(after, OFFSETS["hovered_location"], candidates[0])
            write8(after, OFFSETS["choice_mode"], 2)
            mode = 2

        display_record = (
            read16(after, OFFSETS["hovered_location"]) if mode & 2 else SELECTED
        )
        draw_text(title_offset(kind, mode))
        draw_text(display_record + 4, record_segment=True)
        y = 35
        if kind & 8 and mode & 2 == 0 and len(candidates) > 1:
            write8(after, OFFSETS["choice_mode"], 1)
            write16(after, OFFSETS["hovered_location"], 0)
            mouse_x = read16(after, OFFSETS["mouse_x"])
            mouse_y = read16(after, OFFSETS["mouse_y"])
            for record in candidates:
                if 110 <= mouse_x < 250 and y <= mouse_y < y + 10:
                    write16(after, OFFSETS["hovered_location"], record)
                draw_text(record + 4, record_segment=True)
                y += 10
            if read8(after, OFFSETS["primary_pressed"]) & 1:
                hovered = read16(after, OFFSETS["hovered_location"])
                if hovered:
                    write8(after, OFFSETS["choice_mode"], 2)
                    write8(after, OFFSETS["primary_pressed"], 0)
                    return
        elif mode & 1 == 0:
            details = source_list(display_record)
            actor = eligible_detail_record(records, details)
            if actor is not None:
                draw_text(0x01EF)
                draw_text(actor + 4, record_segment=True)
                y += 10
                draw_text(0x01C2)
                format_integer(read16(records, actor + 0x16))
                draw_text(NUMBER_BUFFER)
                y += 10
                for label, field in ((0x01CE, 0x32), (0x01DB, 0x34), (0x01E4, 0x38)):
                    draw_text(label)
                    format_integer(read16(records, actor + field))
                    names.append("stat")
                    y += 10

        if read8(after, OFFSETS["primary_pressed"]) & 1 == 0:
            return
        write8(after, OFFSETS["primary_pressed"], 0)
        mode = read8(after, OFFSETS["choice_mode"])
        count = read8(after, OFFSETS["candidate_count"])
        if mode & 2 and count > 1:
            write8(after, OFFSETS["choice_mode"], mode - 1)
            return
        write8(after, OFFSETS["panel_ready"], 0)
        write8(after, OFFSETS["panel_active"], 0)
        write8(after, OFFSETS["phase"], 2)
        write8(after, OFFSETS["interpolation_current"], 0)
        write8(after, OFFSETS["scale"], read8(after, OFFSETS["scale"]) + 1)

    if phase & 1:
        write8(after, OFFSETS["panel_ready"], 0)
        write8(after, OFFSETS["choice_mode"], 0)
        write8(after, OFFSETS["candidate_count"], 0)
        write16(after, OFFSETS["hovered_location"], 0)
        if scale == 0:
            write8(after, OFFSETS["artwork_present"], 0)
            names.extend(("compare", "compare"))
            if str(case.get("record_name", "TARGET")) == "TARGET":
                write8(after, OFFSETS["artwork_present"], 1)
                names.extend(("resource", "setter", "palette"))
                write_far_pointer(
                    after,
                    OFFSETS["entity_frame_pointer"],
                    RESOURCE // 16,
                    FRAME_OFFSET,
                )
                write16(after, OFFSETS["source_width"], (0x2F * 14) >> 5)
        write8(after, OFFSETS["primary_pressed"], 0)
        write8(after, OFFSETS["scale"], scale + 1)
        names.extend(("entity", "render", "interpolate"))
        if not complete:
            return after, names
        write8(after, OFFSETS["panel_ready"], 1)
        write8(after, OFFSETS["phase"], 0)
        draw_content()
        return after, names

    if phase & 2:
        write8(after, OFFSETS["scale"], scale - 1)
        names.extend(("entity", "render", "interpolate"))
        if complete:
            write8(after, OFFSETS["panel_ready"], 1)
            names.append("transition")
            write8(after, OFFSETS["phase"], 0)
            write16(after, OFFSETS["selected"], 0)
            write16(after, OFFSETS["deferred"], 0)
        return after, names

    draw_content()
    return after, names


def stack_word(cpu: Uc, index: int = 0) -> int:
    sp = cpu.reg_read(UC_X86_REG_SP)
    return struct.unpack("<H", cpu.mem_read(GLOBALS + sp + index * 2, 2))[0]


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


def read_c_string(data: bytes | bytearray, offset: int) -> str:
    end = data.index(0, offset, min(offset + 64, len(data)))
    return bytes(data[offset:end]).decode("ascii")


def read_machine_string(cpu: Uc, segment: int, offset: int) -> str:
    value = bytearray()
    for index in range(64):
        character = cpu.mem_read(segment * 16 + ((offset + index) & 0xFFFF), 1)[0]
        if character == 0:
            return value.decode("ascii")
        value.append(character)
    raise AssertionError("unterminated helper string")


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    globals_before, records_before, incoming_es_before, resource_before = initialize(
        case, case_index
    )
    expected_globals, expected_names = expected(case, globals_before, records_before)
    lists = source_lists(case)
    complete = bool(case.get("complete", False))
    module = executable[HEADER_SIZE:]

    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(RECORDS, bytes(records_before))
    cpu.mem_write(INCOMING_ES, bytes(incoming_es_before))
    cpu.mem_write(RESOURCE, bytes(resource_before))
    initial_registers = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E55678 + case_index,
        UC_X86_REG_EDI: 0xF6F66789 + case_index,
        UC_X86_REG_EBP: 0x97970000 | CONTEXT_OFFSET,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: INCOMING_ES // 16,
        UC_X86_REG_FS: 0xD000,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: GLOBALS // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_IP:
            reached_return = True
            machine.emu_stop()
            return
        file_offset = address + HEADER_SIZE
        helper = EXTERNALS.get(file_offset)
        if helper is None:
            assert image_address(
                DISPATCHER[0]
            ) <= address < address + size <= image_address(
                DISPATCHER[1]
            ) or image_address(
                CANDIDATE_FILTER[0]
            ) <= address < address + size <= image_address(CANDIDATE_FILTER[1]), (
                case["name"],
                hex(file_offset),
            )
            return

        name, return_kind = helper
        event: dict[str, object] = {"name": name}
        if name == "compare":
            left = read_machine_string(
                machine,
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_SI),
            )
            right = read_machine_string(
                machine,
                machine.reg_read(UC_X86_REG_ES),
                machine.reg_read(UC_X86_REG_DI),
            )
            event["strings"] = [left, right]
            flags = machine.reg_read(UC_X86_REG_EFLAGS)
            machine.reg_write(
                UC_X86_REG_EFLAGS, (flags | 1) if left == right else (flags & ~1)
            )
        elif name == "resource":
            event["resource"] = machine.reg_read(UC_X86_REG_AX)
        elif name == "setter":
            event["entity"] = machine.reg_read(UC_X86_REG_AX)
            event["pointer"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
            ]
            machine.mem_write(
                GLOBALS + OFFSETS["entity_frame_pointer"],
                struct.pack("<HH", FRAME_OFFSET, RESOURCE // 16),
            )
        elif name == "palette":
            event["color"] = machine.reg_read(UC_X86_REG_AX)
        elif name == "entity":
            event["comparison"] = [
                read16(globals_before, CONTEXT_OFFSET + 4),
                read16(globals_before, CONTEXT_OFFSET + 6),
            ]
        elif name == "render":
            event["range"] = [
                machine.reg_read(UC_X86_REG_AX),
                machine.reg_read(UC_X86_REG_BX),
            ]
        elif name == "interpolate":
            event["direction"] = (
                "closing"
                if machine.reg_read(UC_X86_REG_SI) == OFFSETS["current_rect"]
                else "opening"
            )
            flags = machine.reg_read(UC_X86_REG_EFLAGS)
            machine.reg_write(
                UC_X86_REG_EFLAGS, (flags | 1) if complete else (flags & ~1)
            )
        elif name == "remap":
            event["rect"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
                machine.reg_read(UC_X86_REG_DX),
                machine.reg_read(UC_X86_REG_BP),
            ]
        elif name == "text":
            text = read_machine_string(
                machine,
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_SI),
            )
            event["text"] = text
            event["position"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_DX),
            ]
            event["color"] = machine.reg_read(UC_X86_REG_AL)
            machine.mem_write(
                GLOBALS + OFFSETS["text_width"],
                struct.pack("<H", len(text) * 5),
            )
        elif name == "source_list":
            owner = machine.reg_read(UC_X86_REG_DI)
            values = lists.get(owner, [])
            event["owner"] = owner
            event["sources"] = values
            payload = b"".join(struct.pack("<H", value) for value in [*values, 0xFFFF])
            machine.mem_write(GLOBALS + OFFSETS["source_list"], payload)
        elif name == "integer":
            value = machine.reg_read(UC_X86_REG_AX)
            event["value"] = value
            signed = struct.unpack("<h", struct.pack("<H", value))[0]
            encoded = str(signed).encode("ascii") + b"\0"
            machine.mem_write(GLOBALS + NUMBER_BUFFER, encoded)
            machine.reg_write(UC_X86_REG_DI, NUMBER_BUFFER)
        elif name == "stat":
            event["position"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
            ]
            event["value"] = machine.reg_read(UC_X86_REG_DX)
            event["extent"] = machine.reg_read(UC_X86_REG_BP)
            event["color"] = machine.reg_read(UC_X86_REG_AL)
        elif name == "transition":
            event["entity"] = machine.reg_read(UC_X86_REG_AX)
        calls.append(event)
        (far_return if return_kind == "far" else near_return)(machine)

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert GLOBALS <= address < address + size <= GLOBALS + SEGMENT_SIZE, (
            case["name"],
            hex(address),
            size,
        )

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(DISPATCHER[0]), 0, count=10000)
    assert reached_return, case["name"]
    actual_names = [str(call["name"]) for call in calls]
    assert actual_names == expected_names, (case["name"], actual_names, expected_names)

    compare_calls = [call for call in calls if call["name"] == "compare"]
    if compare_calls:
        record_name = str(case.get("record_name", "TARGET"))
        assert [call["strings"] for call in compare_calls] == [
            ["OTHER", record_name],
            ["TARGET", record_name],
        ]
    for call in calls:
        if call["name"] == "entity":
            assert call["comparison"] == [COMPARISON_OFFSET, COMPARISON_SEGMENT]
        elif call["name"] == "remap":
            assert call["rect"] == [110, 25, 96, 70]
        elif call["name"] == "resource":
            assert call["resource"] == 0x805E
        elif call["name"] == "setter":
            assert call["entity"] == 0 and call["pointer"] == list(
                case.get("mouse", (300, 190))
            ), (
                case["name"],
                call,
            )
        elif call["name"] == "palette":
            assert call["color"] == 0xFFCE

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    checked_size = 0x8000
    if globals_after[:checked_size] != bytes(expected_globals[:checked_size]):
        mismatch = next(
            offset
            for offset, pair in enumerate(
                zip(
                    globals_after[:checked_size],
                    expected_globals[:checked_size],
                    strict=True,
                )
            )
            if pair[0] != pair[1]
        )
        raise AssertionError(
            f"{case['name']}: global {mismatch:#x}={globals_after[mismatch]:#x}, "
            f"expected {expected_globals[mismatch]:#x}; calls={calls}"
        )
    assert bytes(cpu.mem_read(RECORDS, SEGMENT_SIZE)) == bytes(records_before)
    assert bytes(cpu.mem_read(INCOMING_ES, SEGMENT_SIZE)) == bytes(incoming_es_before)
    assert bytes(cpu.mem_read(RESOURCE, SEGMENT_SIZE)) == bytes(resource_before)
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert (
        bytes(cpu.mem_read(GLOBALS + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )
    for register in (UC_X86_REG_DS, UC_X86_REG_FS, UC_X86_REG_GS, UC_X86_REG_SS):
        assert cpu.reg_read(register) == initial_registers[register], (
            case["name"],
            register,
        )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    return {
        "name": case["name"],
        "phase_before": int(case.get("phase", 0)),
        "phase_after": read8(globals_after, OFFSETS["phase"]),
        "scale_before": int(case.get("scale", 4)),
        "scale_after": read8(globals_after, OFFSETS["scale"]),
        "primary_before": bool(case.get("primary", 0)),
        "primary_after": bool(read8(globals_after, OFFSETS["primary_pressed"]) & 1),
        "interpolation_complete": complete,
        "calls": calls,
        "choice_mode_after": read8(globals_after, OFFSETS["choice_mode"]),
        "candidate_count_after": read8(globals_after, OFFSETS["candidate_count"]),
        "hovered_location_after": read16(globals_after, OFFSETS["hovered_location"]),
        "panel_active_after": bool(read8(globals_after, OFFSETS["panel_active"]) & 1),
        "artwork_present_after": bool(
            read8(globals_after, OFFSETS["artwork_present"]) & 1
        ),
        "panel_ready_after": bool(read8(globals_after, OFFSETS["panel_ready"]) & 1),
        "selected_after": read16(globals_after, OFFSETS["selected"]),
        "deferred_after": read16(globals_after, OFFSETS["deferred"]),
        "globals_sha256": hashlib.sha256(globals_after[:checked_size]).hexdigest(),
        "defined_flags": {
            name: bool(flags & mask)
            for name, mask in (
                ("cf", 1),
                ("pf", 4),
                ("af", 0x10),
                ("zf", 0x40),
                ("sf", 0x80),
                ("of", 0x800),
            )
        },
        "return": "near",
    }


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander = json.loads(commander_path.read_text())
    assert len(commander) == 11 and len(rows) >= 11
    shared = ((0, 0), (2, 2), (3, 3), (8, 8), (9, 9))
    for sequel_index, commander_index in shared:
        sequel = rows[sequel_index]
        original = commander[commander_index]
        assert sequel["name"] == original["name"]
        assert sequel["phase_before"] == original["state_before"]
        assert sequel["phase_after"] == original["state_after"]
        assert sequel["scale_before"] == original["scale_before"]
        assert sequel["scale_after"] == original["scale_after"]
        assert [call["name"] for call in sequel["calls"]] == [
            call["name"] for call in original["calls"]
        ], sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9083_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    for span, expected in (
        (DISPATCHER, DISPATCHER_SHA256),
        (CANDIDATE_FILTER, CANDIDATE_FILTER_SHA256),
    ):
        actual = hashlib.sha256(executable[slice(*span)]).hexdigest()
        if actual != expected:
            raise SystemExit(
                f"native span {span[0]:#x}..{span[1]:#x} changed: {actual}"
            )
    assert executable[DISPATCHER[1] - 1] == 0xC3
    assert executable[CANDIDATE_FILTER[1] - 1] == 0xC3

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB location-panel cases")


if __name__ == "__main__":
    main()
