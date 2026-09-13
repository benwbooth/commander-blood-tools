#!/usr/bin/env python3
"""Execute Big Bug Bang's complete presentation-scene dispatcher.

The original 0xB4B0 routine executes unmodified. Established image, renderer,
resource, palette, queue, and display boundaries are captured. Eleven cases
mirror Commander Blood's 0x9D10 oracle and two isolate BBB's resource-name
policy branch.
"""

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
DISPATCHER = (0xB4B0, 0xB731)
DISPATCHER_SHA256 = "44bdf4ebff368c1f97fbb2aac01137939baa6c4d18938a7a96705cc0623b8213"
CODE_SEGMENT = 0x502
GLOBALS = 0x30000
CALLER_ES = 0x50000
OBJECTS = 0x60000
BACK_BUFFER = 0x70000
DECOY = 0x80000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
RETURN_LINEAR = CODE_SEGMENT * 16 + RETURN_IP
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
RECORD = 0x3456
BACK_BUFFER_OFFSET = 0x7100
LINK_TARGET = 0x4A40
PALETTE_SOURCE = bytes((index * 29 + 7) & 0xFF for index in range(0xC0))

OFFSETS = {
    "frame_presented": 0x1006,
    "draw_via_back_buffer": 0x1007,
    "skip_back_buffer_present": 0x1009,
    "source_is_banked": 0x100A,
    "unclamped_rows": 0x100B,
    "mode_ids": 0x100C,
    "entry_metric": 0x0FFD,
    "read_index": 0x0FAE,
    "xms_handle": 0x0C4E,
    "ems_handle": 0x0C50,
    "palette_force_directory": 0x0CEA,
    "alien_overlay_armed": 0x0CEC,
    "temporary_sound": 0x0CED,
    "navigation_choice_sound": 0x0D1D,
    "resource_name": 0x22F0,
    "image_table": 0x2203,
    "cached_image": 0x21F1,
    "vertical_offset": 0x21F5,
    "presentation_gate": 0x2200,
    "ship_flags": 0x2745,
    "sequence_active": 0x277C,
    "dispatch_blocked": 0x277F,
    "depth_opening": 0x2781,
    "depth_step": 0x2783,
    "bridge_redraw": 0x2A78,
    "scene_gate": 0x29DD,
    "image_palette_refresh": 0x5F23,
    "image_transparent_zero": 0x5F27,
    "back_buffer_pointer": 0x55F9,
    "rect_top": 0x5609,
    "rect_bottom": 0x560B,
    "transition_percent": 0x561F,
    "palette_source": 0x57A1,
    "palette_destination": 0x5DA1,
    "black_remap": 0x62E1,
    "record_segment": 0x6AEE,
    "primary_record": 0x6B30,
    "scruter_record": 0x6B32,
    "active_line": 0x6B5A,
    "displayed_line": 0x6B5C,
    "vm_enabled": 0x6B7E,
    "request_flags": 0x6B80,
    "finale": 0x6B93,
}

EXTERNALS = {
    0x2F83: ("pbm_image_load_and_decode", "far"),
    0x423C: ("back_buffer_fill", "far"),
    0xB942: ("resource_load_sequence", "near"),
    0x2660: ("palette_blend_remap_table_build", "far"),
    0xB997: ("ems_resource_flush", "near"),
    0xBBF5: ("list_d8c_state_le_one", "near"),
    0x41F8: ("blit_fill_row_5221", "far"),
}

CASES = (
    {"name": "negative_line_exits", "line": 0xFFFF},
    {
        "name": "scruter_jo_record_arms_overlay",
        "line": 0x001D,
        "scene_gate": 1,
        "record_related": 0x4567,
        "named_scruter": 0x4567,
        "displayed": 0x001D,
    },
    {"name": "armed_overlay_triggers_on_next_line", "line": 5, "alien_armed": 1},
    {
        "name": "new_image_loads_and_copies_palette",
        "line": 2,
        "image_path": 0x1234,
        "cached_path": 0x2222,
        "mode_slot": 0,
    },
    {
        "name": "missing_image_clears_back_buffer_band",
        "line": 0,
        "image_path": 0xFFFF,
        "cached_path": 0x2222,
    },
    {
        "name": "banked_line_builds_black_remap",
        "line": 8,
        "image_path": 0x1234,
        "cached_path": 0x1234,
        "xms_handle": 1,
        "ems_handle": 0xFFFF,
        "sequence_active": 1,
        "displayed": 7,
        "mode_slot": 8,
    },
    {
        "name": "unbanked_line_honors_first_eight_mode_slots",
        "line": 8,
        "image_path": 0x1234,
        "cached_path": 0x1234,
        "xms_handle": 0xFFFF,
        "ems_handle": 0xFFFF,
        "displayed": 8,
        "mode_slot": 7,
    },
    {
        "name": "active_dispatch_blocked",
        "line": 3,
        "presentation_gate": 1,
        "dispatch_blocked": 1,
    },
    {
        "name": "active_line_five_teardown",
        "line": 5,
        "presentation_gate": 1,
        "queue_ready": False,
        "ship_flags": 8,
        "alien_armed": 1,
        "finale": 1,
    },
    {
        "name": "ready_line_27_resets_transition",
        "line": 0x27,
        "presentation_gate": 1,
        "queue_ready": True,
        "entry_metric": 0x24,
        "read_index": 0x10,
    },
    {
        "name": "ready_ship_line_arms_depth_opening",
        "line": 3,
        "presentation_gate": 1,
        "queue_ready": True,
        "ship_flags": 8,
        "entry_metric": 0x18,
        "read_index": 0x10,
    },
    {
        "name": "sequel_finale_prefix_forces_unclamped_skip_policy",
        "line": 2,
        "resource_name": b"finale.HNM",
    },
    {
        "name": "sequel_fin_dot_uses_regular_line_policy",
        "line": 2,
        "resource_name": b"fin.HNM",
    },
)


def write8(memory: bytearray, offset: int, value: int) -> None:
    memory[offset] = value & 0xFF


def write16(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", memory, offset, value & 0xFFFF)


def write32(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", memory, offset, value & 0xFFFFFFFF)


def read8(memory: bytes | bytearray, offset: int) -> int:
    return memory[offset]


def read16(memory: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", memory, offset)[0]


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


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


def force_finale_policy(resource_name: bytes) -> bool:
    return resource_name.startswith(b"fin") and resource_name[3:4] != b"."


def initialize(case: dict[str, object], case_index: int):
    line = int(case["line"])
    globals_before = bytearray(
        (offset * 7 + (offset >> 8) * 11 + case_index * 17 + 0x21) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    caller_es_before = bytearray(
        (offset * 13 + (offset >> 8) * 19 + case_index * 23 + 0x43) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    objects_before = bytearray(
        (offset * 5 + (offset >> 8) * 29 + case_index * 31 + 0x65) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    back_buffer_before = bytes([0x6D]) * SEGMENT_SIZE
    decoy_before = bytes([0xCC]) * SEGMENT_SIZE
    stack_before = bytearray(
        (offset * 17 + case_index * 37 + 0x87) & 0xFF for offset in range(SEGMENT_SIZE)
    )

    write8(globals_before, OFFSETS["frame_presented"], 0x7A)
    write16(globals_before, OFFSETS["active_line"], line)
    write8(
        globals_before,
        OFFSETS["presentation_gate"],
        int(case.get("presentation_gate", 0)),
    )
    write8(globals_before, OFFSETS["scene_gate"], int(case.get("scene_gate", 0)))
    write8(
        globals_before, OFFSETS["alien_overlay_armed"], int(case.get("alien_armed", 0))
    )
    write8(globals_before, OFFSETS["temporary_sound"], 0x55)
    write16(globals_before, OFFSETS["record_segment"], OBJECTS // 16)
    write16(globals_before, OFFSETS["primary_record"], RECORD)
    write16(
        globals_before,
        OFFSETS["scruter_record"],
        int(case.get("named_scruter", 0x7777)),
    )
    write16(objects_before, RECORD + 2, int(case.get("record_related", 0x6666)))
    table_slot = OFFSETS["image_table"] + (line * 4 if line < 0x80 else 0)
    write16(globals_before, table_slot, 0x2468)
    if line < 0x80:
        write16(globals_before, table_slot + 2, int(case.get("image_path", 0x1234)))
    write16(
        globals_before,
        OFFSETS["cached_image"],
        int(case.get("cached_path", case.get("image_path", 0x1234))),
    )
    write8(globals_before, OFFSETS["image_palette_refresh"], 0x66)
    write8(globals_before, OFFSETS["image_transparent_zero"], 0x77)
    write32(
        globals_before,
        OFFSETS["back_buffer_pointer"],
        (BACK_BUFFER // 16 << 16) | BACK_BUFFER_OFFSET,
    )
    write8(globals_before, OFFSETS["palette_force_directory"], 0x88)
    write16(globals_before, OFFSETS["vertical_offset"], 0x0035)
    write16(globals_before, OFFSETS["rect_top"], 0x1111)
    write16(globals_before, OFFSETS["rect_bottom"], 0x2222)
    write8(globals_before, OFFSETS["draw_via_back_buffer"], 9)
    write8(globals_before, OFFSETS["skip_back_buffer_present"], 0x0B)
    write8(globals_before, OFFSETS["unclamped_rows"], 0x0D)
    write8(globals_before, OFFSETS["source_is_banked"], 0x0C)
    write8(globals_before, OFFSETS["request_flags"], 0xA3)
    write16(globals_before, OFFSETS["xms_handle"], int(case.get("xms_handle", 0xFFFF)))
    write16(globals_before, OFFSETS["ems_handle"], int(case.get("ems_handle", 0xFFFF)))
    write8(
        globals_before, OFFSETS["sequence_active"], int(case.get("sequence_active", 0))
    )
    write16(globals_before, OFFSETS["displayed_line"], int(case.get("displayed", line)))
    write8(
        globals_before,
        OFFSETS["dispatch_blocked"],
        int(case.get("dispatch_blocked", 0)),
    )
    write16(globals_before, OFFSETS["ship_flags"], int(case.get("ship_flags", 0)))
    write8(globals_before, OFFSETS["bridge_redraw"], 0x44)
    write8(globals_before, OFFSETS["finale"], int(case.get("finale", 0)))
    write8(globals_before, OFFSETS["navigation_choice_sound"], 0x33)
    write16(
        globals_before, OFFSETS["entry_metric"], int(case.get("entry_metric", 0x3333))
    )
    write16(globals_before, OFFSETS["read_index"], int(case.get("read_index", 0x1111)))
    write16(globals_before, OFFSETS["transition_percent"], 0x7777)
    write8(globals_before, OFFSETS["depth_opening"], 0x22)
    write8(globals_before, OFFSETS["depth_step"], 0x33)
    write8(globals_before, OFFSETS["vm_enabled"], 0xA6)
    globals_before[
        OFFSETS["palette_source"] : OFFSETS["palette_source"] + len(PALETTE_SOURCE)
    ] = PALETTE_SOURCE

    resource_name = bytes(case.get("resource_name", b"ordinary.HNM"))
    resource_field = resource_name[:12].ljust(13, b"\0")
    globals_before[
        OFFSETS["resource_name"] : OFFSETS["resource_name"] + len(resource_field)
    ] = resource_field
    caller_es_before[OFFSETS["mode_ids"] : OFFSETS["mode_ids"] + 9] = bytes(
        0xE0 + index for index in range(9)
    )
    mode_slot = case.get("mode_slot")
    if mode_slot is not None:
        caller_es_before[OFFSETS["mode_ids"] + int(mode_slot)] = line & 0xFF
    caller_es_before[
        OFFSETS["palette_destination"] : OFFSETS["palette_destination"]
        + len(PALETTE_SOURCE)
    ] = bytes([0xA5]) * len(PALETTE_SOURCE)
    stack_before[STACK_POINTER : STACK_POINTER + 4 + len(STACK_SENTINEL)] = (
        struct.pack("<HH", RETURN_IP, CODE_SEGMENT) + STACK_SENTINEL
    )
    return (
        globals_before,
        caller_es_before,
        objects_before,
        back_buffer_before,
        decoy_before,
        stack_before,
        resource_name,
    )


def expected(
    case: dict[str, object],
    globals_before: bytearray,
    caller_es_before: bytearray,
    resource_name: bytes,
):
    line = int(case["line"])
    presentation_gate = int(case.get("presentation_gate", 0))
    scene_gate = int(case.get("scene_gate", 0))
    alien_armed = int(case.get("alien_armed", 0))
    image_path = int(case.get("image_path", 0x1234))
    cached_path = int(case.get("cached_path", image_path))
    sequence_active = int(case.get("sequence_active", 0))
    displayed = int(case.get("displayed", line))
    ship_flags = int(case.get("ship_flags", 0))
    xms_handle = int(case.get("xms_handle", 0xFFFF))
    ems_handle = int(case.get("ems_handle", 0xFFFF))
    entry_metric = int(case.get("entry_metric", 0x3333))
    read_index = int(case.get("read_index", 0x1111))
    queue_ready = bool(case.get("queue_ready", True))
    expected_globals = bytearray(globals_before)
    expected_es = bytearray(caller_es_before)
    expected_calls: list[dict[str, object]] = []
    write8(expected_globals, OFFSETS["frame_presented"], 0)

    if line < 0x8000:
        if presentation_gate & 1 == 0:
            early_exit = False
            if line == 0x001D:
                armed = int(
                    int(case.get("record_related", 0x6666))
                    == int(case.get("named_scruter", 0x7777))
                )
                write8(expected_globals, OFFSETS["alien_overlay_armed"], armed)
            elif alien_armed & 1:
                write8(expected_globals, OFFSETS["temporary_sound"], 1)
                early_exit = True

            if not early_exit:
                if scene_gate & 1 == 0:
                    if image_path == 0xFFFF:
                        write16(expected_globals, OFFSETS["cached_image"], 0xFFFF)
                    elif image_path != cached_path:
                        write16(expected_globals, OFFSETS["cached_image"], image_path)
                        expected_calls.append(
                            {
                                "call": "pbm_image_load_and_decode",
                                "path_segment": GLOBALS // 16,
                                "path_offset": image_path,
                                "buffer_segment": BACK_BUFFER // 16,
                                "buffer_offset": BACK_BUFFER_OFFSET,
                                "palette_refresh": 1,
                                "transparent_zero": 1,
                                "force_directory": 1,
                            }
                        )
                        write8(expected_globals, OFFSETS["palette_force_directory"], 0)
                        write8(expected_globals, OFFSETS["image_palette_refresh"], 0)
                        write8(expected_globals, OFFSETS["image_transparent_zero"], 0)
                        expected_es[
                            OFFSETS["palette_destination"] : OFFSETS[
                                "palette_destination"
                            ]
                            + len(PALETTE_SOURCE)
                        ] = PALETTE_SOURCE
                    if read16(expected_globals, OFFSETS["cached_image"]) == 0xFFFF:
                        expected_calls.append(
                            {
                                "call": "back_buffer_fill",
                                "color": 0,
                                "top": 0x0035,
                                "bottom": 0x00B7,
                            }
                        )
                        write16(expected_globals, OFFSETS["rect_top"], 0)
                        write16(expected_globals, OFFSETS["rect_bottom"], 200)

                write8(expected_globals, OFFSETS["presentation_gate"], 1)
                write8(expected_globals, OFFSETS["draw_via_back_buffer"], 0)
                write8(expected_globals, OFFSETS["skip_back_buffer_present"], 0)
                write8(expected_globals, OFFSETS["unclamped_rows"], 0)
                special_finale = force_finale_policy(resource_name)
                if special_finale:
                    write8(expected_globals, OFFSETS["unclamped_rows"], 1)
                    write16(expected_globals, OFFSETS["vertical_offset"], 0)
                else:
                    mode_ids = caller_es_before[
                        OFFSETS["mode_ids"] : OFFSETS["mode_ids"] + 8
                    ]
                    if line & 0xFF in mode_ids:
                        write8(expected_globals, OFFSETS["unclamped_rows"], 1)
                        write8(expected_globals, OFFSETS["vm_enabled"], 0)

                if not special_finale and line in (2, 7):
                    write8(expected_globals, OFFSETS["draw_via_back_buffer"], 1)
                    write8(expected_globals, OFFSETS["vm_enabled"], 0)
                elif special_finale or line in (
                    0,
                    1,
                    3,
                    4,
                    5,
                    6,
                    0x29,
                    0x2A,
                    0x2B,
                    0x2C,
                ):
                    write8(
                        expected_globals,
                        OFFSETS["request_flags"],
                        read8(expected_globals, OFFSETS["request_flags"]) | 2,
                    )
                    write8(expected_globals, OFFSETS["skip_back_buffer_present"], 1)
                    write8(expected_globals, OFFSETS["vm_enabled"], 0)
                write8(expected_globals, OFFSETS["source_is_banked"], 0)
                if line == 8 and (xms_handle + ems_handle) & 0xFFFF != 0xFFFE:
                    write8(expected_globals, OFFSETS["source_is_banked"], 1)
                expected_calls.append({"call": "resource_load_sequence", "line": line})
                if (sequence_active | scene_gate) != 0 and displayed != line:
                    write16(expected_globals, OFFSETS["displayed_line"], line)
                    expected_calls.append(
                        {
                            "call": "palette_blend_remap_table_build",
                            "negative_percent": 0xFFCE,
                            "red": 0,
                            "green": 0,
                            "blue": 0,
                            "table_segment": CALLER_ES // 16,
                            "table_offset": OFFSETS["black_remap"],
                        }
                    )
        elif int(case.get("dispatch_blocked", 0)) & 1 == 0:
            expected_calls.extend(
                [
                    {"call": "ems_resource_flush", "link_target_offset": LINK_TARGET},
                    {"call": "list_d8c_state_le_one", "result": queue_ready},
                ]
            )
            if not queue_ready:
                write8(expected_globals, OFFSETS["vm_enabled"], 1)
                if ship_flags & 8:
                    write8(expected_globals, OFFSETS["bridge_redraw"], 1)
                if line == 5:
                    expected_calls.append(
                        {
                            "call": "blit_fill_row_5221",
                            "color": 0,
                            "top": 0x0023,
                            "bottom": 0x00A5,
                        }
                    )
                    write16(expected_globals, OFFSETS["rect_top"], 0)
                    write16(expected_globals, OFFSETS["rect_bottom"], 200)
                write8(
                    expected_globals,
                    OFFSETS["temporary_sound"],
                    int((alien_armed & 1) != 0),
                )
                write8(
                    expected_globals,
                    OFFSETS["navigation_choice_sound"],
                    int((int(case.get("finale", 0)) & 1) != 0),
                )
                write8(expected_globals, OFFSETS["presentation_gate"], 0)
                write16(expected_globals, OFFSETS["displayed_line"], line)
                write16(expected_globals, OFFSETS["active_line"], 0xFFFF)
                write8(
                    expected_globals,
                    OFFSETS["request_flags"],
                    read8(expected_globals, OFFSETS["request_flags"]) & 0xFD,
                )
            elif line == 0x27:
                if (entry_metric - read_index) & 0xFFFF == 0x14:
                    write16(expected_globals, OFFSETS["transition_percent"], 0)
            elif ship_flags & 8 and (entry_metric - read_index) & 0xFFFF == 8:
                write8(expected_globals, OFFSETS["depth_opening"], 1)
                write8(expected_globals, OFFSETS["depth_step"], 6)
    return expected_globals, expected_es, expected_calls


def stack_word(cpu: Uc, index: int = 0) -> int:
    sp = cpu.reg_read(UC_X86_REG_SP)
    return struct.unpack("<H", cpu.mem_read(STACK + sp + index * 2, 2))[0]


def capture_external(cpu: Uc, name: str, case: dict[str, object]) -> dict[str, object]:
    if name == "pbm_image_load_and_decode":
        return {
            "call": name,
            "path_segment": cpu.reg_read(UC_X86_REG_DS),
            "path_offset": cpu.reg_read(UC_X86_REG_ESI) & 0xFFFF,
            "buffer_segment": cpu.reg_read(UC_X86_REG_ES),
            "buffer_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
            "palette_refresh": cpu.mem_read(
                GLOBALS + OFFSETS["image_palette_refresh"], 1
            )[0],
            "transparent_zero": cpu.mem_read(
                GLOBALS + OFFSETS["image_transparent_zero"], 1
            )[0],
            "force_directory": cpu.mem_read(
                GLOBALS + OFFSETS["palette_force_directory"], 1
            )[0],
        }
    if name in ("back_buffer_fill", "blit_fill_row_5221"):
        return {
            "call": name,
            "color": cpu.reg_read(UC_X86_REG_AX),
            "top": struct.unpack("<H", cpu.mem_read(GLOBALS + OFFSETS["rect_top"], 2))[
                0
            ],
            "bottom": struct.unpack(
                "<H", cpu.mem_read(GLOBALS + OFFSETS["rect_bottom"], 2)
            )[0],
        }
    if name == "resource_load_sequence":
        return {"call": name, "line": cpu.reg_read(UC_X86_REG_AX)}
    if name == "palette_blend_remap_table_build":
        return {
            "call": name,
            "negative_percent": cpu.reg_read(UC_X86_REG_AX),
            "red": cpu.reg_read(UC_X86_REG_BX),
            "green": cpu.reg_read(UC_X86_REG_ECX) & 0xFFFF,
            "blue": cpu.reg_read(UC_X86_REG_EDX) & 0xFFFF,
            "table_segment": cpu.reg_read(UC_X86_REG_ES),
            "table_offset": cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
        }
    if name == "ems_resource_flush":
        return {
            "call": name,
            "link_target_offset": cpu.reg_read(UC_X86_REG_EBP) & 0xFFFF,
        }
    if name == "list_d8c_state_le_one":
        ready = bool(case.get("queue_ready", True))
        flags = cpu.reg_read(UC_X86_REG_EFLAGS)
        cpu.reg_write(UC_X86_REG_EFLAGS, flags | 0x40 if ready else flags & ~0x40)
        return {"call": name, "result": ready}
    raise AssertionError(name)


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    (
        globals_before,
        caller_es_before,
        objects_before,
        back_buffer_before,
        decoy_before,
        stack_before,
        resource_name,
    ) = initialize(case, case_index)
    for address, data in (
        (GLOBALS, globals_before),
        (CALLER_ES, caller_es_before),
        (OBJECTS, objects_before),
        (BACK_BUFFER, back_buffer_before),
        (DECOY, decoy_before),
        (STACK, stack_before),
    ):
        cpu.mem_write(address, bytes(data))

    initial_registers = {
        UC_X86_REG_EAX: 0xA5A51234,
        UC_X86_REG_EBX: 0xB6B62345,
        UC_X86_REG_ECX: 0xC7C73456,
        UC_X86_REG_EDX: 0xD8D84567,
        UC_X86_REG_ESI: 0xE9E95678,
        UC_X86_REG_EDI: 0xFAFA6789,
        UC_X86_REG_EBP: 0xABCD0000 | LINK_TARGET,
        UC_X86_REG_CS: CODE_SEGMENT,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: CALLER_ES // 16,
        UC_X86_REG_FS: 0xD000,
        UC_X86_REG_GS: DECOY // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    expected_globals, expected_es, expected_calls = expected(
        case, globals_before, caller_es_before, resource_name
    )
    calls: list[dict[str, object]] = []
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_LINEAR:
            reached_return = True
            machine.emu_stop()
            return
        file_offset = address + HEADER_SIZE
        external = EXTERNALS.get(file_offset)
        if external is not None:
            name, return_kind = external
            calls.append(capture_external(machine, name, case))
            (near_return if return_kind == "near" else far_return)(machine)
            return
        assert (
            image_address(DISPATCHER[0])
            <= address
            < address + size
            <= image_address(DISPATCHER[1])
        ), (
            case["name"],
            hex(file_offset),
        )

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert any(
            start <= address < address + size <= start + SEGMENT_SIZE
            for start in (GLOBALS, CALLER_ES, STACK)
        ), (case["name"], hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(DISPATCHER[0]), 0, count=3000)
    assert reached_return, case["name"]
    assert calls == expected_calls, (case["name"], calls, expected_calls)

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    caller_es_after = bytes(cpu.mem_read(CALLER_ES, SEGMENT_SIZE))
    if globals_after != bytes(expected_globals):
        mismatch = next(
            offset
            for offset, (actual, wanted) in enumerate(
                zip(globals_after, expected_globals, strict=True)
            )
            if actual != wanted
        )
        raise AssertionError(
            f"{case['name']}: global {mismatch:#x}="
            f"{globals_after[mismatch]:#x}, expected {expected_globals[mismatch]:#x}"
        )
    assert caller_es_after == bytes(expected_es), case["name"]
    assert bytes(cpu.mem_read(OBJECTS, SEGMENT_SIZE)) == bytes(objects_before)
    assert bytes(cpu.mem_read(BACK_BUFFER, SEGMENT_SIZE)) == back_buffer_before
    assert bytes(cpu.mem_read(DECOY, SEGMENT_SIZE)) == decoy_before
    assert bytes(cpu.mem_read(0, len(module))) == module
    for register, expected_value in initial_registers.items():
        if register in (UC_X86_REG_SP, UC_X86_REG_EFLAGS):
            continue
        assert cpu.reg_read(register) == expected_value, (
            case["name"],
            register,
            hex(cpu.reg_read(register)),
            hex(expected_value),
        )
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 4
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 4, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )

    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    return {
        "name": case["name"],
        "line": int(case["line"]),
        "presentation_gate": int(case.get("presentation_gate", 0)),
        "scene_gate": int(case.get("scene_gate", 0)),
        "resource_name": resource_name.decode("ascii"),
        "sequel_finale_policy": force_finale_policy(resource_name),
        "calls": calls,
        "result": {
            "active_line": read16(globals_after, OFFSETS["active_line"]),
            "displayed_line": read16(globals_after, OFFSETS["displayed_line"]),
            "presentation_gate": read8(globals_after, OFFSETS["presentation_gate"]),
            "alien_overlay_armed": read8(globals_after, OFFSETS["alien_overlay_armed"]),
            "temp_snd_trigger": read8(globals_after, OFFSETS["temporary_sound"]),
            "draw_via_back_buffer": read8(
                globals_after, OFFSETS["draw_via_back_buffer"]
            ),
            "skip_back_buffer_present": read8(
                globals_after, OFFSETS["skip_back_buffer_present"]
            ),
            "source_is_banked": read8(globals_after, OFFSETS["source_is_banked"]),
            "unclamped_row_count": read8(globals_after, OFFSETS["unclamped_rows"]),
            "request_flags": read8(globals_after, OFFSETS["request_flags"]),
            "vertical_offset": read16(globals_after, OFFSETS["vertical_offset"]),
            "vm_enabled": read8(globals_after, OFFSETS["vm_enabled"]),
            "depth_opening": read8(globals_after, OFFSETS["depth_opening"]),
            "depth_step": read8(globals_after, OFFSETS["depth_step"]),
        },
        "global_changes": changes(globals_before, globals_after),
        "caller_es_changes": changes(caller_es_before, caller_es_after),
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "caller_es_sha256": hashlib.sha256(caller_es_after).hexdigest(),
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


def normalized_calls(calls: list[dict[str, object]]) -> list[str]:
    return [str(call["call"]) for call in calls]


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander_rows = json.loads(commander_path.read_text())
    assert len(commander_rows) == 11
    for sequel, commander in zip(rows, commander_rows, strict=False):
        assert sequel["name"] == commander["name"]
        for field in (
            "active_line",
            "displayed_line",
            "presentation_gate",
            "alien_overlay_armed",
            "temp_snd_trigger",
            "draw_via_back_buffer",
            "skip_back_buffer_present",
            "source_is_banked",
            "unclamped_row_count",
            "request_flags",
            "depth_opening",
            "depth_step",
        ):
            assert sequel["result"][field] == commander["result"][field], (
                sequel["name"],
                field,
            )
        assert normalized_calls(sequel["calls"]) == normalized_calls(
            commander["calls"]
        ), sequel["name"]
    finale, fin_dot = rows[-2:]
    assert finale["sequel_finale_policy"] is True
    assert finale["result"]["draw_via_back_buffer"] == 0
    assert finale["result"]["skip_back_buffer_present"] == 1
    assert finale["result"]["unclamped_row_count"] == 1
    assert finale["result"]["vertical_offset"] == 0
    assert fin_dot["sequel_finale_policy"] is False
    assert fin_dot["result"]["draw_via_back_buffer"] == 1
    assert fin_dot["result"]["skip_back_buffer_present"] == 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_9d10_natural.json",
    )
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    actual = hashlib.sha256(executable[slice(*DISPATCHER)]).hexdigest()
    if actual != DISPATCHER_SHA256:
        raise SystemExit(
            f"native span {DISPATCHER[0]:#x}..{DISPATCHER[1]:#x} changed: {actual}"
        )
    assert executable[DISPATCHER[1] - 1] == 0xCB

    rows = [execute(executable, case, index) for index, case in enumerate(CASES)]
    compare_commander(rows, args.commander_vectors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original BBB presentation-scene cases")


if __name__ == "__main__":
    main()
