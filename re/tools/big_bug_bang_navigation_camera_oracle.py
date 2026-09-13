#!/usr/bin/env python3
"""Execute Big Bug Bang's complete navigation-camera coordinator.

The original 0x9EDE routine executes unmodified. Established renderer, entity,
wipe, overview, panel, picking, and text boundaries are captured. The inherited
cases are compared with Commander Blood's 0x8CCE fixture while retaining BBB's
overview callback and current-location selection differences.
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
COORDINATOR = (0x9EDE, 0xA286)
COORDINATOR_SHA256 = "83f540a41fa474430279ed4463e138277c9688b50823a8d06b8b6d73ab1b27ba"

GLOBALS = 0x30000
INCOMING_DS = 0x44000
INCOMING_ES = 0x55000
RECORDS = 0x70000
WORK = 0xA0000
SPANS = 0xB0000
BACK = 0xC0000
STACK = GLOBALS
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
CONTEXT_OFFSET = 0x7800
EXTENT_OFFSET = 0x6200
EXTENT_SEGMENT = 0xBEEF
SPAN_OFFSET = 0x6000
WORK_OFFSET = 0x2000
BACK_OFFSET = 0x1000
ARCHE_OFFSET = 0x2000
CURRENT_OFFSET = 0x2100

OFFSETS = {
    "mouse_x": 0x0C22,
    "mouse_y": 0x0C24,
    "hand_requested": 0x0C2A,
    "hand_current": 0x0C2C,
    "primary_pressed": 0x0C36,
    "press_pending": 0x0C38,
    "entity_state_mask": 0x0D49,
    "panel_phase": 0x2A22,
    "panel_art_present": 0x2A23,
    "panel_scale": 0x2A24,
    "active": 0x2A25,
    "transition_step": 0x2A26,
    "panel_active": 0x2A27,
    "primary_marker": 0x2A2A,
    "secondary_marker_count": 0x2A2B,
    "wipe_complete": 0x2A2C,
    "overview_active": 0x2A30,
    "ui_flags": 0x2A33,
    "selected": 0x2A5F,
    "chart_object_count": 0x2A61,
    "panel_rect": 0x2D4B,
    "object_list": 0x2D73,
    "wipe_endpoints": 0x29E0,
    "work_pointer": 0x0CB4,
    "palette_refresh": 0x5F23,
    "span_pointer": 0x55F1,
    "back_pointer": 0x55F9,
    "entity_base": 0x65E2,
    "record_pointer": 0x6AEC,
    "arche": 0x6B22,
    "deferred": 0x6B3C,
}

EXTERNALS = {
    0x42ED: ("vga", "far"),
    0x8231: ("list", "far"),
    0x454D: ("populate", "far"),
    0x48EE: ("render", "far"),
    0x464E: ("transition", "far"),
    0xAAFE: ("wipe", "near"),
    0xAAD4: ("copy", "near"),
    0x551A: ("dirty", "far"),
    0xAFBA: ("panorama", "near"),
    0x9EA6: ("reset", "far"),
    0xACE4: ("flip", "far"),
    0xA5E0: ("panel", "near"),
    0xAA3D: ("pick", "near"),
    0x344D: ("width", "far"),
    0x3512: ("text", "far"),
    0xA286: ("overlay", "near"),
}

CASES = (
    {"name": "inactive", "state": 0, "active": 0},
    {
        "name": "interactive_waits_for_wipe",
        "state": 0,
        "active": 1,
        "wipe_complete": 0,
    },
    {
        "name": "selected_panel_forwards_inherited_extent",
        "state": 0,
        "active": 1,
        "wipe_complete": 1,
        "selected": 0x1000,
        "interaction": "panel",
    },
    {
        "name": "hover_draws_clamped_object_label",
        "state": 0,
        "active": 1,
        "wipe_complete": 1,
        "picked": 0x1000,
        "mouse": (12, 7),
        "label_width": 30,
        "interaction": "hover",
    },
    {
        "name": "click_current_location_opens_panel",
        "commander_name": "click_current_location_only_updates_hand_and_input",
        "state": 0,
        "active": 1,
        "wipe_complete": 1,
        "picked": CURRENT_OFFSET,
        "mouse": (100, 70),
        "pressed": 1,
        "interaction": "click_current",
    },
    {
        "name": "click_new_right_location_starts_panel",
        "state": 0,
        "active": 1,
        "wipe_complete": 1,
        "picked": 0x1000,
        "mouse": (200, 80),
        "pressed": 1,
        "interaction": "click_new",
    },
    {
        "name": "closing_completion_copies_outside_center",
        "state": 1,
        "active": 0,
        "endpoint_y": 109,
        "spans": [(40, 60)],
        "wipe": "closing",
    },
    {
        "name": "closing_upper_half_copies_inside_then_tail",
        "state": 4,
        "active": 0,
        "endpoint_y": 110,
        "spans": [(70, 180), (80, 160)],
        "wipe": "closing",
    },
    {
        "name": "opening_lower_half_reveals_rows_in_reverse",
        "state": 4,
        "active": 1,
        "endpoint_y": 3,
        "spans": [(90, 140), (80, 160)],
        "wipe": "opening",
    },
    {
        "name": "opening_upper_half_copies_outside_center",
        "state": 3,
        "active": 1,
        "endpoint_y": 110,
        "spans": [(65, 190)],
        "wipe": "opening",
    },
    {
        "name": "closing_first_frame_builds_chart_entities",
        "state": 8,
        "active": 0,
        "endpoint_y": 110,
        "spans": [(75, 170)],
        "wipe": "closing",
        "build_chart": True,
    },
    {
        "name": "opening_first_frame_restores_panorama_buffer",
        "state": 8,
        "active": 1,
        "endpoint_y": 0,
        "spans": [(100, 120)],
        "wipe": "opening",
        "opening_setup": True,
    },
    {
        "name": "overview_consumes_primary_before_pick",
        "state": 0,
        "active": 1,
        "wipe_complete": 1,
        "picked": 0x1000,
        "mouse": (100, 70),
        "pressed": 1,
        "label_width": 30,
        "interaction": "overlay_consumes",
        "overlay_consumes_primary": True,
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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 19 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initialize(
    case: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray, bytearray, bytearray, bytearray, bytearray]:
    globals_before = seeded_segment(case_index, 7, 0x21)
    incoming_ds_before = seeded_segment(case_index, 11, 0x43)
    incoming_es_before = seeded_segment(case_index, 17, 0x65)
    records_before = seeded_segment(case_index, 5, 0x87)
    work_before = seeded_segment(case_index, 13, 0xCB)
    back_before = seeded_segment(case_index, 19, 0xED)

    write8(globals_before, OFFSETS["active"], int(case["active"]))
    write8(globals_before, OFFSETS["transition_step"], int(case["state"]))
    write8(globals_before, OFFSETS["primary_marker"], 0xA5)
    write8(globals_before, OFFSETS["secondary_marker_count"], 2)
    write8(globals_before, OFFSETS["wipe_complete"], int(case.get("wipe_complete", 0)))
    write8(globals_before, OFFSETS["overview_active"], 0x8D)
    write8(globals_before, OFFSETS["ui_flags"], 0x81)
    write8(globals_before, OFFSETS["entity_state_mask"], 1)
    write8(globals_before, OFFSETS["panel_phase"], 0)
    write8(globals_before, OFFSETS["panel_art_present"], 0)
    write8(globals_before, OFFSETS["panel_scale"], 7)
    write8(globals_before, OFFSETS["panel_active"], 0)
    write8(globals_before, OFFSETS["primary_pressed"], int(case.get("pressed", 0)))
    write8(globals_before, OFFSETS["press_pending"], 0x83)
    mouse_x, mouse_y = (int(value) for value in case.get("mouse", (80, 50)))
    write16(globals_before, OFFSETS["mouse_x"], mouse_x)
    write16(globals_before, OFFSETS["mouse_y"], mouse_y)
    write16(globals_before, OFFSETS["hand_requested"], 0x3232)
    write16(globals_before, OFFSETS["hand_current"], 0x3434)
    write8(globals_before, 0x0CE3, 0xDA)
    write8(globals_before, 0x0CE4, 0xDB)
    write16(globals_before, OFFSETS["selected"], int(case.get("selected", 0)))
    write16(globals_before, OFFSETS["chart_object_count"], 0xC1C1)
    write16(globals_before, OFFSETS["arche"], ARCHE_OFFSET)
    write16(globals_before, OFFSETS["deferred"], 0x6A6A)
    write8(globals_before, OFFSETS["palette_refresh"], 1)
    write_far_pointer(globals_before, OFFSETS["record_pointer"], RECORDS // 16, 0)
    write_far_pointer(globals_before, OFFSETS["work_pointer"], WORK // 16, WORK_OFFSET)
    write_far_pointer(globals_before, OFFSETS["span_pointer"], SPANS // 16, SPAN_OFFSET)
    write_far_pointer(globals_before, OFFSETS["back_pointer"], BACK // 16, BACK_OFFSET)
    for entity in (1, 5, 6):
        write16(
            globals_before,
            OFFSETS["entity_base"] + entity * 0x20,
            0xA0 + entity,
        )

    direction = str(case.get("wipe", ""))
    state = int(case["state"])
    endpoint_index = state - 1 if direction == "closing" else 9 - state
    if direction:
        endpoint = OFFSETS["wipe_endpoints"] + endpoint_index * 4
        write16(globals_before, endpoint, 160)
        write16(globals_before, endpoint + 2, int(case["endpoint_y"]))

    write16(globals_before, OFFSETS["object_list"], 0x1000)
    write16(globals_before, OFFSETS["object_list"] + 2, 0x1100)
    write16(globals_before, OFFSETS["object_list"] + 4, 0xFFFF)
    write16(records_before, 0x1000, 0x0100)
    write16(records_before, 0x1014, 0)
    write16(records_before, 0x1018, 50)
    write16(records_before, 0x101A, 60)
    records_before[0x1004:0x100A] = b"ALPHA\0"
    write16(records_before, 0x1100, 0x0010)
    write16(records_before, 0x1114, 1)
    write16(records_before, 0x1118, 100)
    write16(records_before, 0x111A, 70)
    write16(records_before, ARCHE_OFFSET + 0x16, CURRENT_OFFSET)
    write16(records_before, ARCHE_OFFSET + 0x18, 12)
    write16(records_before, ARCHE_OFFSET + 0x1A, 10)
    write16(records_before, CURRENT_OFFSET, 0x0110)
    write16(globals_before, STACK_POINTER, RETURN_IP)
    globals_before[STACK_POINTER + 2 : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        STACK_SENTINEL
    )
    write16(globals_before, CONTEXT_OFFSET + 4, EXTENT_OFFSET)
    write16(globals_before, CONTEXT_OFFSET + 6, EXTENT_SEGMENT)
    return (
        globals_before,
        records_before,
        incoming_ds_before,
        incoming_es_before,
        work_before,
        back_before,
    )


def expected_wipe_copies(
    direction: str, row: int, spans: list[tuple[int, int]]
) -> list[tuple[int, int, int]]:
    copies: list[tuple[int, int, int]] = []
    if direction == "closing":
        if row < 110:
            copies.extend((0, y, 320) for y in range(110, 200))
            for left, width in spans:
                copies.append((0, row, left))
                right = left + width
                copies.append((right, row, 320 - right))
                row += 1
        else:
            row = 110
            for left, width in spans:
                copies.append((left, row, width))
                row += 1
            copies.extend((0, y, 320) for y in range(row, 200))
    elif row < 110:
        copies.extend((0, y, 320) for y in range(row - 1, 0, -1))
        for left, width in spans:
            copies.append((left, row, width))
            row += 1
    else:
        row = 110
        copies.extend((0, y, 320) for y in range(109, 0, -1))
        for left, width in spans:
            copies.append((0, row, left))
            right = left + width
            copies.append((right, row, 320 - right))
            row += 1
    return copies


def expected(
    case: dict[str, object], globals_before: bytearray
) -> tuple[bytearray, list[str], list[tuple[int, int, int]]]:
    after = bytearray(globals_before)
    state = int(case["state"])
    active = int(case["active"])
    selected = int(case.get("selected", 0))
    picked = int(case.get("picked", 0))
    pressed = int(case.get("pressed", 0))
    direction = str(case.get("wipe", ""))
    spans = [(int(left), int(width)) for left, width in case.get("spans", [])]
    calls: list[str] = []
    copies: list[tuple[int, int, int]] = []

    if state == 0:
        if active & 1 and read8(after, OFFSETS["wipe_complete"]) & 1:
            write8(after, OFFSETS["ui_flags"], read8(after, OFFSETS["ui_flags"]) | 4)
            if selected:
                calls.append("panel")
            else:
                for entity in (5, 6):
                    offset = OFFSETS["entity_base"] + entity * 0x20
                    write8(after, offset, read8(after, offset) | 3)
                offset = OFFSETS["entity_base"] + 0x20
                write8(after, offset, (read8(after, offset) | 3) & 0xFE)
                calls.extend(("overlay", "pick"))
                if bool(case.get("overlay_consumes_primary", False)):
                    write8(after, OFFSETS["primary_pressed"], 0)
                    write8(after, OFFSETS["press_pending"], 0)
                    pressed = 0
                if picked and not pressed:
                    calls.extend(("width", "text"))
                elif picked:
                    write16(after, OFFSETS["hand_current"], 0)
                    mouse_x = read16(after, OFFSETS["mouse_x"])
                    write16(
                        after, OFFSETS["hand_requested"], 12 if mouse_x > 160 else 11
                    )
                    write8(after, OFFSETS["primary_pressed"], 0)
                    write8(after, OFFSETS["press_pending"], 0)
                    write8(after, OFFSETS["panel_active"], 1)
                    write16(after, OFFSETS["deferred"], picked)
                    write16(after, OFFSETS["selected"], picked)
                    struct.pack_into(
                        "<hhhh",
                        after,
                        OFFSETS["panel_rect"],
                        read16(after, OFFSETS["mouse_x"]),
                        read16(after, OFFSETS["mouse_y"]),
                        4,
                        4,
                    )
                    write8(after, 0x0CE4, 0)
                    write8(after, 0x0CE3, 8)
                    write8(after, OFFSETS["panel_phase"], 1)
                    write8(after, OFFSETS["panel_scale"], 0)
                    calls.extend(("transition", "transition", "transition"))
        return after, calls, copies

    write16(after, OFFSETS["selected"], 0)
    if bool(case.get("build_chart", False)):
        calls.extend(
            (
                "vga",
                "list",
                "populate",
                "populate",
                "render",
                "populate",
                "render",
                "transition",
                "populate",
                "transition",
                "transition",
                "transition",
            )
        )
        write16(after, OFFSETS["chart_object_count"], 2)
        write8(after, OFFSETS["primary_marker"], 0)
        write8(after, OFFSETS["secondary_marker_count"], 1)
        write16(after, OFFSETS["object_list"], 0x1000)
        write16(after, OFFSETS["object_list"] + 2, 0x1100)
        write16(after, OFFSETS["object_list"] + 4, 0xFFFF)
    if bool(case.get("opening_setup", False)):
        write8(after, OFFSETS["wipe_complete"], 0)
        write8(after, OFFSETS["overview_active"], 0)
        calls.extend(
            ("transition", "transition", "transition", "panorama", "reset", "flip")
        )
        write8(after, OFFSETS["palette_refresh"], 0)
    if direction == "closing":
        write8(after, OFFSETS["wipe_complete"], 1 if state == 1 else 0)
    calls.append("wipe")
    copies = expected_wipe_copies(direction, int(case["endpoint_y"]), spans)
    calls.extend("copy" for _copy in copies)
    calls.append("dirty")
    write8(after, OFFSETS["transition_step"], state - 1)
    return after, calls, copies


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


def read_string(cpu: Uc, segment: int, offset: int) -> str:
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
    (
        globals_before,
        records_before,
        incoming_ds_before,
        incoming_es_before,
        work_before,
        back_before,
    ) = initialize(case, case_index)
    expected_globals, expected_names, expected_copies = expected(case, globals_before)
    spans = [(int(left), int(width)) for left, width in case.get("spans", [])]
    picked = int(case.get("picked", 0))
    build_chart = bool(case.get("build_chart", False))

    module = executable[HEADER_SIZE:]
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, module)
    for address, data in (
        (GLOBALS, globals_before),
        (INCOMING_DS, incoming_ds_before),
        (INCOMING_ES, incoming_es_before),
        (RECORDS, records_before),
        (WORK, work_before),
        (BACK, back_before),
    ):
        cpu.mem_write(address, bytes(data))
    span_before = seeded_segment(case_index, 3, 0xA9)
    cpu.mem_write(SPANS, bytes(span_before))
    cpu.mem_write(
        STACK + STACK_POINTER,
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL,
    )
    cpu.mem_write(
        STACK + CONTEXT_OFFSET + 4,
        struct.pack("<HH", EXTENT_OFFSET, EXTENT_SEGMENT),
    )

    initial_registers = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345 + case_index,
        UC_X86_REG_ECX: 0xC3C33456 + case_index,
        UC_X86_REG_EDX: 0xD4D44567 + case_index,
        UC_X86_REG_ESI: 0xE5E55678 + case_index,
        UC_X86_REG_EDI: 0xF6F66789 + case_index,
        UC_X86_REG_EBP: 0x97970000 | CONTEXT_OFFSET,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: INCOMING_DS // 16,
        UC_X86_REG_ES: INCOMING_ES // 16,
        UC_X86_REG_FS: 0xD000,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    copies: list[tuple[int, int, int]] = []
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
            assert (
                image_address(COORDINATOR[0])
                <= address
                < address + size
                <= image_address(COORDINATOR[1])
            ), (case["name"], hex(file_offset))
            return

        name, return_kind = helper
        event: dict[str, object] = {"name": name}
        if name == "vga":
            event["source"] = [
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_SI),
            ]
            event["destination"] = [
                machine.reg_read(UC_X86_REG_ES),
                machine.reg_read(UC_X86_REG_DI),
            ]
        elif name == "list":
            machine.reg_write(UC_X86_REG_AX, 2 if build_chart else 0)
            event["result"] = machine.reg_read(UC_X86_REG_AX)
        elif name == "populate":
            event["entity"] = machine.reg_read(UC_X86_REG_AX)
            event["resource"] = machine.reg_read(UC_X86_REG_DX)
            event["position"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
            ]
            event["frame"] = machine.reg_read(UC_X86_REG_BP)
            if build_chart and machine.reg_read(UC_X86_REG_AX) == 0:
                record = machine.reg_read(UC_X86_REG_DI)
                marker = int(read16(records_before, record + 0x14) == 0)
                machine.mem_write(GLOBALS + OFFSETS["primary_marker"], bytes((marker,)))
        elif name == "render":
            event["range"] = [
                machine.reg_read(UC_X86_REG_AX),
                machine.reg_read(UC_X86_REG_BX),
            ]
        elif name == "transition":
            event["entity"] = machine.reg_read(UC_X86_REG_AX)
        elif name == "wipe":
            endpoint_offset = machine.reg_read(UC_X86_REG_SI)
            event["endpoint_offset"] = endpoint_offset
            event["endpoint"] = [
                read16(globals_before, endpoint_offset),
                read16(globals_before, endpoint_offset + 2),
            ]
            payload = b"".join(struct.pack("<HH", left, width) for left, width in spans)
            machine.mem_write(SPANS + SPAN_OFFSET, payload + b"\xff\xff\xff\xff")
        elif name == "copy":
            rect = (
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
                machine.reg_read(UC_X86_REG_DX),
            )
            event["rect"] = list(rect)
            copies.append(rect)
        elif name == "dirty":
            event["rectangles"] = [
                machine.reg_read(UC_X86_REG_ES),
                machine.reg_read(UC_X86_REG_DI),
            ]
        elif name == "panorama":
            event["eax"] = machine.reg_read(UC_X86_REG_EAX)
        elif name == "panel":
            bp = machine.reg_read(UC_X86_REG_BP)
            pointer = bytes(machine.mem_read(STACK + bp + 4, 4))
            event["context"] = [bp, read16(pointer, 0), read16(pointer, 2)]
        elif name == "overlay":
            if bool(case.get("overlay_consumes_primary", False)):
                machine.mem_write(GLOBALS + OFFSETS["primary_pressed"], b"\0")
                machine.mem_write(GLOBALS + OFFSETS["press_pending"], b"\0")
        elif name == "pick":
            event["record_segment"] = machine.reg_read(UC_X86_REG_ES)
            machine.reg_write(UC_X86_REG_AX, picked)
            event["result"] = picked
        elif name == "width":
            event["text"] = read_string(
                machine,
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_SI),
            )
            event["font"] = machine.reg_read(UC_X86_REG_AX)
            machine.reg_write(UC_X86_REG_AX, int(case.get("label_width", 30)))
        elif name == "text":
            event["text"] = read_string(
                machine,
                machine.reg_read(UC_X86_REG_DS),
                machine.reg_read(UC_X86_REG_SI),
            )
            event["position"] = [
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_DX),
            ]
            event["color"] = machine.reg_read(UC_X86_REG_AL)
        calls.append(event)
        (far_return if return_kind == "far" else near_return)(machine)

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert any(
            start <= address < address + size <= start + SEGMENT_SIZE
            for start in (GLOBALS, STACK)
        ), (case["name"], hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(COORDINATOR[0]), 0, count=10000)
    assert reached_return, case["name"]
    actual_names = [str(call["name"]) for call in calls]
    assert actual_names == expected_names, (case["name"], actual_names, expected_names)
    assert copies == expected_copies, (case["name"], copies[:4], expected_copies[:4])

    if case.get("interaction") == "panel":
        panel = next(call for call in calls if call["name"] == "panel")
        assert panel["context"] == [CONTEXT_OFFSET, EXTENT_OFFSET, EXTENT_SEGMENT]
    if case.get("interaction") in ("hover", "overlay_consumes"):
        width = next(call for call in calls if call["name"] == "width")
        text = next(call for call in calls if call["name"] == "text")
        assert width == {"name": "width", "text": "ALPHA", "font": 1}
        assert text == {
            "name": "text",
            "text": "ALPHA",
            "position": [
                0 if case.get("interaction") == "hover" else 70,
                0 if case.get("interaction") == "hover" else 60,
            ],
            "color": 0xEF,
        }
    if build_chart:
        populate = [call for call in calls if call["name"] == "populate"]
        assert populate == [
            {
                "name": "populate",
                "entity": 0,
                "resource": 0x2C,
                "position": [50, 60],
                "frame": 1,
            },
            {
                "name": "populate",
                "entity": 5,
                "resource": 0x2C,
                "position": [47, 57],
                "frame": 4,
            },
            {
                "name": "populate",
                "entity": 0,
                "resource": 0x2C,
                "position": [100, 70],
                "frame": 2,
            },
            {
                "name": "populate",
                "entity": 1,
                "resource": 0x2C,
                "position": [8, 2],
                "frame": 6,
            },
        ]
        assert [call["entity"] for call in calls if call["name"] == "transition"] == [
            0,
            1,
            5,
            31,
        ]
    if case.get("opening_setup"):
        assert [call["entity"] for call in calls if call["name"] == "transition"] == [
            1,
            5,
            6,
        ]
        assert next(call for call in calls if call["name"] == "panorama")["eax"] == 0

    checked_size = 0x8000
    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    if globals_after[:checked_size] != bytes(expected_globals[:checked_size]):
        mismatch = next(
            offset
            for offset, pair in enumerate(
                zip(globals_after, expected_globals, strict=True)
            )
            if pair[0] != pair[1]
        )
        raise AssertionError(
            f"{case['name']}: global {mismatch:#x}={globals_after[mismatch]:#x}, "
            f"expected {expected_globals[mismatch]:#x}; "
            f"secondary_count={globals_after[OFFSETS['secondary_marker_count']]:#x}, "
            f"calls={[call['name'] for call in calls]}"
        )
    assert bytes(cpu.mem_read(RECORDS, SEGMENT_SIZE)) == bytes(records_before)
    assert bytes(cpu.mem_read(INCOMING_DS, SEGMENT_SIZE)) == bytes(incoming_ds_before)
    assert bytes(cpu.mem_read(INCOMING_ES, SEGMENT_SIZE)) == bytes(incoming_es_before)
    assert bytes(cpu.mem_read(WORK, SEGMENT_SIZE)) == bytes(work_before)
    assert bytes(cpu.mem_read(BACK, SEGMENT_SIZE)) == bytes(back_before)
    assert bytes(cpu.mem_read(0, len(module))) == module
    for register, initial in initial_registers.items():
        if register in (UC_X86_REG_SP, UC_X86_REG_EFLAGS):
            continue
        assert cpu.reg_read(register) == initial, (
            case["name"],
            register,
            hex(cpu.reg_read(register)),
            hex(initial),
        )
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )

    noncopy_calls = [call for call in calls if call["name"] != "copy"]
    copy_bytes = b"".join(struct.pack("<HHH", *copy) for copy in copies)
    entity_states = []
    if "overlay" in actual_names:
        for entity in (5, 6, 1):
            raw = read8(globals_after, OFFSETS["entity_base"] + entity * 0x20)
            entity_states.append(
                {"entity": entity, "visible": bool(raw & 2), "active": bool(raw & 1)}
            )
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    return {
        "name": case["name"],
        "state_before": int(case["state"]),
        "state_after": read8(globals_after, OFFSETS["transition_step"]),
        "active": int(case["active"]),
        "current_location_selectable": True,
        "overlay_consumes_primary": bool(case.get("overlay_consumes_primary", False)),
        "calls": noncopy_calls,
        "entity_states": entity_states,
        "copy_count": len(copies),
        "copy_head": [list(copy) for copy in copies[:4]],
        "copy_tail": [list(copy) for copy in copies[-4:]],
        "copy_sha256": hashlib.sha256(copy_bytes).hexdigest(),
        "ui_active_after": bool(read8(globals_after, OFFSETS["ui_flags"]) & 4),
        "primary_pressed_after": bool(
            read8(globals_after, OFFSETS["primary_pressed"]) & 1
        ),
        "press_pending_after": bool(read8(globals_after, OFFSETS["press_pending"]) & 1),
        "selected_after": read16(globals_after, OFFSETS["selected"]),
        "deferred_after": read16(globals_after, OFFSETS["deferred"]),
        "panel_active_after": bool(read8(globals_after, OFFSETS["panel_active"]) & 1),
        "panel_phase_after": read8(globals_after, OFFSETS["panel_phase"]),
        "panel_scale_after": read8(globals_after, OFFSETS["panel_scale"]),
        "overview_active_after": bool(
            read8(globals_after, OFFSETS["overview_active"]) & 1
        ),
        "wipe_complete_after": bool(read8(globals_after, OFFSETS["wipe_complete"]) & 1),
        "palette_refresh_after": bool(
            read8(globals_after, OFFSETS["palette_refresh"]) & 1
        ),
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
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


def normalized_calls(calls: list[dict[str, object]]) -> list[dict[str, object]]:
    result = []
    for call in calls:
        if call["name"] == "overlay":
            continue
        normalized = dict(call)
        if normalized["name"] == "wipe":
            normalized["endpoint_offset"] = int(normalized["endpoint_offset"]) - 0x28E
        elif normalized["name"] == "dirty":
            rectangles = list(normalized["rectangles"])
            rectangles[1] = int(rectangles[1]) - 0x3D0
            normalized["rectangles"] = rectangles
        result.append(normalized)
    return result


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander_rows = json.loads(commander_path.read_text())
    assert len(rows) == 13 and len(commander_rows) == 12
    shared_fields = (
        "state_before",
        "state_after",
        "active",
        "copy_count",
        "copy_head",
        "copy_tail",
        "copy_sha256",
        "return",
    )
    for case, sequel, commander in zip(
        CASES[:12], rows[:12], commander_rows, strict=True
    ):
        assert str(case.get("commander_name", case["name"])) == commander["name"]
        for field in shared_fields:
            assert sequel[field] == commander[field], (sequel["name"], field)
        if str(case.get("interaction", "")) == "click_current":
            continue
        assert normalized_calls(sequel["calls"]) == commander["calls"], sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_8cce_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    actual = hashlib.sha256(executable[slice(*COORDINATOR)]).hexdigest()
    if actual != COORDINATOR_SHA256:
        raise SystemExit(
            f"native span {COORDINATOR[0]:#x}..{COORDINATOR[1]:#x} changed: {actual}"
        )
    assert executable[COORDINATOR[1] - 1] == 0xC3

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB navigation-camera cases")


if __name__ == "__main__":
    main()
