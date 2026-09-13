#!/usr/bin/env python3
"""Execute Big Bug Bang's complete contact scene-transition coordinator.

The original 0x1A17 routine executes unmodified. Established entity, scene,
DESCRIPT, image, renderer, bridge, alien, and ship-HUD boundaries are captured.
The resulting cases are compared with Commander Blood's 0x1855 oracle after
normalizing relocated storage, calls, and the one-byte FRIGO path shift.
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
    UC_X86_REG_BP,
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
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
COORDINATOR = (0x1A17, 0x1C55)
COORDINATOR_SHA256 = "75cfa0a8250bd5d6a34b7cc5791713ad60b36c359f076c8ff0d6503152583e2a"

GLOBALS = 0x30000
RECORDS = 0x50000
BACK_BUFFER = 0x60000
CALLER_ES = 0x71000
CALLER_FS = 0x72000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
RETURN_LINEAR = RETURN_IP
STACK_SENTINEL = bytes.fromhex("5aa5966987783cc3")
RECORD_OFFSET = 0x0240
DEFERRED_RECORD_OFFSET = 0x0380
BACK_BUFFER_OFFSET = 0x0100
FRIGO_PATH_OFFSET = 0x00F2

OFFSETS = {
    "temporary_state": 0x0C2A,
    "vertical_offset": 0x21F5,
    "selection_sentinel": 0x21F9,
    "c2_gate": 0x2200,
    "scene_record": 0x29DB,
    "scene_gate": 0x29DD,
    "phase": 0x29DF,
    "ui_state": 0x2A33,
    "redraw_pending": 0x2A79,
    "back_buffer_pointer": 0x55F9,
    "rect_top": 0x5609,
    "rect_bottom": 0x560B,
    "clip_snapshot_flags": 0x5619,
    "transition_increment": 0x561D,
    "transition_percent": 0x561F,
    "live_palette": 0x57A1,
    "target_palette": 0x5AA1,
    "source_palette": 0x5DA1,
    "palette_first": 0x5F21,
    "palette_last": 0x5F22,
    "palette_refresh": 0x5F23,
    "transparent_zero": 0x5F27,
    "subtitle_active": 0x6234,
    "record_pointer": 0x6AEC,
    "record_segment": 0x6AEE,
    "deferred_type": 0x6B3A,
    "deferred_record": 0x6B3C,
    "active_line": 0x6B5A,
    "request_flags": 0x6B80,
    "presentation_active": 0x6B82,
    "start_lock": 0x6B86,
    "text_wait": 0x6B90,
    "finale": 0x6B92,
}

# Commander storage copied into its BBB semantic counterpart before each run.
MAPPED_INITIAL_RANGES = (
    (0x0A32, OFFSETS["temporary_state"], 2),
    (0x1FA7, OFFSETS["vertical_offset"], 2),
    (0x1FAB, OFFSETS["selection_sentinel"], 2),
    (0x1FB2, OFFSETS["c2_gate"], 1),
    (0x274D, OFFSETS["scene_record"], 2),
    (0x274F, OFFSETS["scene_gate"], 1),
    (0x2751, OFFSETS["phase"], 1),
    (0x2793, OFFSETS["ui_state"], 2),
    (0x27D9, OFFSETS["redraw_pending"], 1),
    (0x5229, OFFSETS["back_buffer_pointer"], 4),
    (0x5239, OFFSETS["rect_top"], 2),
    (0x523B, OFFSETS["rect_bottom"], 2),
    (0x5249, OFFSETS["clip_snapshot_flags"], 2),
    (0x524D, OFFSETS["transition_increment"], 2),
    (0x524F, OFFSETS["transition_percent"], 2),
    (0x53D1, OFFSETS["live_palette"], 192),
    (0x56D1, OFFSETS["target_palette"], 192),
    (0x59D1, OFFSETS["source_palette"], 192),
    (0x5B51, OFFSETS["palette_first"], 1),
    (0x5B52, OFFSETS["palette_last"], 1),
    (0x5B53, OFFSETS["palette_refresh"], 1),
    (0x5B57, OFFSETS["transparent_zero"], 1),
    (0x5E64, OFFSETS["subtitle_active"], 1),
    (0x6724, OFFSETS["record_pointer"], 4),
    (0x6768, OFFSETS["deferred_type"], 2),
    (0x676A, OFFSETS["deferred_record"], 2),
    (0x6788, OFFSETS["active_line"], 2),
    (0x67AA, OFFSETS["request_flags"], 1),
    (0x67AC, OFFSETS["presentation_active"], 1),
    (0x67B0, OFFSETS["start_lock"], 1),
    (0x67BA, OFFSETS["text_wait"], 1),
    (0x67BC, OFFSETS["finale"], 1),
)

EXTERNALS = {
    0x464E: "entity_flag_state_transition",
    0xB4B0: "dlg_line_id_scene_dispatch",
    0x8450: "vm_c2_descript_lookup",
    0x2F83: "pbm_image_load_and_decode",
    0x42C3: "full_screen_blit",
    0x423C: "back_buffer_fill",
    0x9EA6: "ship_3d_hud_palette_snapshot_and_camera_reset",
    0xADF5: "bridge_steer_update",
    0xCD69: "alien_overlay_cycle",
}

CASES = (
    {"name": "inactive_bit_clear", "phase": 0xFE, "record_kind": 2},
    {"name": "active_initializes_record", "phase": 0x01, "record_kind": 2},
    {
        "name": "load_gate_after_dispatch",
        "phase": 0x03,
        "record_kind": 2,
        "c2_gate": 1,
    },
    {"name": "load_nonpresentation", "phase": 0x03, "record_kind": 1},
    {"name": "load_presentation_palette", "phase": 0x03, "record_kind": 2},
    {"name": "load_precedes_deferred", "phase": 0x07, "record_kind": 2},
    {
        "name": "deferred_gate_after_dispatch",
        "phase": 0x05,
        "record_kind": 2,
        "c2_gate": 1,
    },
    {"name": "deferred_record_armed", "phase": 0x05, "record_kind": 2},
    {
        "name": "bridge_nonpresentation_gate",
        "phase": 0x09,
        "record_kind": 1,
        "c2_gate": 1,
    },
    {"name": "bridge_nonpresentation_finishes", "phase": 0x09, "record_kind": 1},
    {
        "name": "bridge_callback_sets_blocked",
        "phase": 0x09,
        "record_kind": 2,
        "bridge_phase": 0x89,
    },
    {
        "name": "bridge_line_seven_sets_reload",
        "phase": 0x09,
        "record_kind": 2,
        "active_line": 7,
    },
    {"name": "bridge_reload", "phase": 0x49, "record_kind": 2},
    {
        "name": "bridge_callback_sets_reload",
        "phase": 0x09,
        "record_kind": 2,
        "bridge_phase": 0x49,
    },
    {
        "name": "bridge_alien_remains_active",
        "phase": 0x09,
        "record_kind": 2,
        "alien_active": 1,
    },
    {
        "name": "bridge_alien_sets_c2_gate",
        "phase": 0x09,
        "record_kind": 2,
        "alien_c2_gate": 1,
    },
    {"name": "bridge_palette_restore", "phase": 0x09, "record_kind": 2},
    {
        "name": "finish_gate_after_dispatch",
        "phase": 0x11,
        "record_kind": 2,
        "c2_gate": 1,
    },
    {"name": "finish_transition", "phase": 0x11, "record_kind": 2},
    {
        "name": "cleanup_gate_after_dispatch",
        "phase": 0x21,
        "record_kind": 2,
        "c2_gate": 1,
    },
    {"name": "cleanup_resets_presentation", "phase": 0x21, "record_kind": 2},
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


def seeded_segment(case_index: int) -> bytearray:
    return bytearray(
        (offset * 37 + (offset >> 8) * 13 + case_index * 29 + 0x41) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def initialize(
    case: dict[str, object], case_index: int
) -> tuple[bytearray, bytearray, bytearray, bytearray]:
    globals_before = seeded_segment(case_index)
    commander_seed = seeded_segment(case_index)
    for commander_offset, sequel_offset, size in MAPPED_INITIAL_RANGES:
        globals_before[sequel_offset : sequel_offset + size] = commander_seed[
            commander_offset : commander_offset + size
        ]

    globals_before[FRIGO_PATH_OFFSET : FRIGO_PATH_OFFSET + 9] = b"frigo.fd\0"
    write8(globals_before, OFFSETS["phase"], int(case["phase"]))
    write8(globals_before, OFFSETS["c2_gate"], int(case.get("c2_gate", 0)))
    write8(globals_before, OFFSETS["transparent_zero"], 0xA6)
    write16(globals_before, OFFSETS["scene_record"], RECORD_OFFSET)
    write16(globals_before, OFFSETS["ui_state"], 0x5A5A)
    write16(globals_before, OFFSETS["clip_snapshot_flags"], 0xA55A)
    write16(globals_before, OFFSETS["deferred_type"], 0x6B6B)
    write16(globals_before, OFFSETS["deferred_record"], DEFERRED_RECORD_OFFSET)
    write16(
        globals_before,
        OFFSETS["active_line"],
        int(case.get("active_line", 0x0033)),
    )
    write_far_pointer(
        globals_before,
        OFFSETS["record_pointer"],
        RECORDS // 16,
        RECORD_OFFSET,
    )
    write_far_pointer(
        globals_before,
        OFFSETS["back_buffer_pointer"],
        BACK_BUFFER // 16,
        BACK_BUFFER_OFFSET,
    )

    live_palette = bytes(
        (index * 17 + case_index * 11 + 3) & 0x3F for index in range(192)
    )
    target_palette = bytes(
        (index * 23 + case_index * 7 + 0x91) & 0xFF for index in range(192)
    )
    source_palette = bytes(
        (index * 31 + case_index * 5 + 0x53) & 0xFF for index in range(192)
    )
    for name, palette in (
        ("live_palette", live_palette),
        ("target_palette", target_palette),
        ("source_palette", source_palette),
    ):
        offset = OFFSETS[name]
        globals_before[offset : offset + len(palette)] = palette

    records_before = bytearray(
        (offset * 19 + case_index * 43 + 0x27) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    write16(records_before, RECORD_OFFSET, int(case["record_kind"]))
    back_buffer_before = bytearray(
        (offset * 7 + case_index) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    caller_es_before = bytearray(
        (offset * 11 + case_index * 17 + 0x35) & 0xFF for offset in range(SEGMENT_SIZE)
    )
    return globals_before, records_before, back_buffer_before, caller_es_before


def expected(
    case: dict[str, object],
    case_index: int,
    globals_before: bytearray,
) -> tuple[bytearray, list[dict[str, object]]]:
    phase = int(case["phase"])
    record_kind = int(case["record_kind"])
    c2_gate = int(case.get("c2_gate", 0))
    initial_eax = 0xA5A57E00 + case_index
    expected_globals = bytearray(globals_before)
    calls: list[dict[str, object]] = []

    def call(callee: str, return_address: int, **arguments: object) -> None:
        calls.append({"callee": callee, "return": [return_address, 0], **arguments})

    if phase & 1 == 0:
        return expected_globals, calls

    write16(expected_globals, OFFSETS["clip_snapshot_flags"], 1)
    if phase & 0xFE == 0:
        call("entity_flag_state_transition", 0x1A36, ax=4)
        call("entity_flag_state_transition", 0x1A3E, ax=31)
        write16(expected_globals, OFFSETS["ui_state"], 0)
        write8(expected_globals, OFFSETS["phase"], phase | 2)
        write16(expected_globals, OFFSETS["active_line"], 0x0029)
        write16(expected_globals, OFFSETS["scene_record"], DEFERRED_RECORD_OFFSET)
        call(
            "vm_c2_descript_lookup",
            0x1A64,
            es=RECORDS // 16,
            di=DEFERRED_RECORD_OFFSET + 4,
        )
        return expected_globals, calls

    call(
        "dlg_line_id_scene_dispatch",
        0x1A6C,
        ax=(initial_eax & 0xFF00) | phase,
        bp=(0x789A + case_index) & 0xFFFF,
    )
    if phase & 2:
        if c2_gate != 0:
            return expected_globals, calls
        write8(expected_globals, OFFSETS["phase"], 5)
        write16(expected_globals, OFFSETS["vertical_offset"], 0x0023)
        write8(expected_globals, OFFSETS["scene_gate"], 1)
        write8(expected_globals, OFFSETS["palette_refresh"], 1)
        write8(expected_globals, OFFSETS["transparent_zero"], 0)
        call(
            "pbm_image_load_and_decode",
            0x1AA1,
            path=[GLOBALS // 16, FRIGO_PATH_OFFSET],
            buffer=[BACK_BUFFER // 16, BACK_BUFFER_OFFSET],
            palette_refresh=1,
            transparent_zero=0,
        )
        call(
            "full_screen_blit",
            0x1AAB,
            source=[BACK_BUFFER // 16, BACK_BUFFER_OFFSET],
        )
        if record_kind != 2:
            write16(expected_globals, OFFSETS["temporary_state"], 0xFFFF)
            write16(expected_globals, OFFSETS["rect_top"], 0x0023)
            write16(expected_globals, OFFSETS["rect_bottom"], 0x00A5)
            call(
                "back_buffer_fill",
                0x1AD4,
                ax=0,
                top=0x0023,
                bottom=0x00A5,
            )
            write16(expected_globals, OFFSETS["rect_top"], 0)
            write16(expected_globals, OFFSETS["rect_bottom"], 200)
            write8(expected_globals, OFFSETS["phase"], 9)
            write16(expected_globals, OFFSETS["active_line"], 0x002B)
        else:
            live = OFFSETS["live_palette"]
            target = OFFSETS["target_palette"]
            source = OFFSETS["source_palette"]
            expected_globals[target : target + 192] = expected_globals[
                live : live + 192
            ]
            expected_globals[source : source + 192] = bytes(
                max(component - 40, 0)
                for component in expected_globals[live : live + 192]
            )
            write8(expected_globals, OFFSETS["palette_first"], 0x80)
            write8(expected_globals, OFFSETS["palette_last"], 0xBF)
            write16(expected_globals, OFFSETS["transition_increment"], 5)
            write16(expected_globals, OFFSETS["active_line"], 0x0027)
        return expected_globals, calls

    if phase & 4:
        if c2_gate == 0:
            write16(expected_globals, OFFSETS["deferred_type"], 0x00C4)
            write8(expected_globals, OFFSETS["phase"], 0x89)
            write16(expected_globals, OFFSETS["temporary_state"], 0)
        return expected_globals, calls

    if phase & 8:
        call("bridge_steer_update", 0x1B56)
        if "bridge_phase" in case:
            write8(expected_globals, OFFSETS["phase"], int(case["bridge_phase"]))
        if record_kind != 2:
            if c2_gate == 0:
                begin_cleanup(expected_globals)
            return expected_globals, calls
        if read8(expected_globals, OFFSETS["phase"]) & 0x80:
            return expected_globals, calls
        if read16(expected_globals, OFFSETS["active_line"]) == 7:
            write8(
                expected_globals,
                OFFSETS["phase"],
                read8(expected_globals, OFFSETS["phase"]) | 0x40,
            )
            return expected_globals, calls
        if read8(expected_globals, OFFSETS["phase"]) & 0x40:
            write8(
                expected_globals,
                OFFSETS["phase"],
                read8(expected_globals, OFFSETS["phase"]) & 0xBF,
            )
            write8(expected_globals, OFFSETS["palette_refresh"], 0)
            call(
                "pbm_image_load_and_decode",
                0x1B9E,
                path=[GLOBALS // 16, FRIGO_PATH_OFFSET],
                buffer=[BACK_BUFFER // 16, BACK_BUFFER_OFFSET],
                palette_refresh=0,
                transparent_zero=0xA6,
            )
            return expected_globals, calls

        call("alien_overlay_cycle", 0x1BA6)
        if "alien_active" in case:
            write8(
                expected_globals,
                OFFSETS["presentation_active"],
                int(case["alien_active"]),
            )
        if "alien_c2_gate" in case:
            write8(
                expected_globals,
                OFFSETS["c2_gate"],
                int(case["alien_c2_gate"]),
            )
        if (
            read8(expected_globals, OFFSETS["presentation_active"]) & 1 == 0
            and read8(expected_globals, OFFSETS["c2_gate"]) & 1 == 0
        ):
            write8(expected_globals, OFFSETS["phase"], 0x11)
            write16(expected_globals, OFFSETS["active_line"], 0x0028)
            live = OFFSETS["live_palette"]
            target = OFFSETS["target_palette"]
            source = OFFSETS["source_palette"]
            expected_globals[source : source + 192] = expected_globals[
                target : target + 192
            ]
            expected_globals[target : target + 192] = expected_globals[
                live : live + 192
            ]
            write16(expected_globals, OFFSETS["transition_percent"], 0)
        return expected_globals, calls

    if phase & 0x10:
        if c2_gate == 0:
            begin_cleanup(expected_globals)
        return expected_globals, calls

    if c2_gate != 0:
        return expected_globals, calls
    write16(expected_globals, OFFSETS["temporary_state"], 0)
    write8(expected_globals, OFFSETS["phase"], 0)
    write16(expected_globals, OFFSETS["ui_state"], 1)
    write16(expected_globals, OFFSETS["selection_sentinel"], 0xFFFF)
    write16(expected_globals, OFFSETS["active_line"], 0xFFFF)
    write8(expected_globals, OFFSETS["c2_gate"], 0)
    write8(expected_globals, OFFSETS["subtitle_active"], 0)
    write8(expected_globals, OFFSETS["start_lock"], 0)
    write8(expected_globals, OFFSETS["finale"], 0)
    write8(
        expected_globals,
        OFFSETS["request_flags"],
        read8(expected_globals, OFFSETS["request_flags"]) & 0xFC,
    )
    write8(expected_globals, OFFSETS["text_wait"], 0)
    write8(expected_globals, OFFSETS["redraw_pending"], 1)
    call("ship_3d_hud_palette_snapshot_and_camera_reset", 0x1C50)
    return expected_globals, calls


def begin_cleanup(globals_after: bytearray) -> None:
    write16(globals_after, OFFSETS["vertical_offset"], 0)
    write8(globals_after, OFFSETS["phase"], 0x21)
    write16(globals_after, OFFSETS["active_line"], 0x002A)
    write8(globals_after, OFFSETS["scene_gate"], 0)


def stack_word(cpu: Uc, index: int = 0) -> int:
    sp = cpu.reg_read(UC_X86_REG_SP)
    return struct.unpack("<H", cpu.mem_read(STACK + sp + index * 2, 2))[0]


def far_return(cpu: Uc) -> None:
    return_ip = stack_word(cpu)
    return_cs = stack_word(cpu, 1)
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def capture_external(cpu: Uc, callee: str) -> dict[str, object]:
    call: dict[str, object] = {
        "callee": callee,
        "return": [stack_word(cpu) + HEADER_SIZE, stack_word(cpu, 1)],
    }
    if callee == "entity_flag_state_transition":
        call["ax"] = cpu.reg_read(UC_X86_REG_AX)
    elif callee == "dlg_line_id_scene_dispatch":
        call["ax"] = cpu.reg_read(UC_X86_REG_AX)
        call["bp"] = cpu.reg_read(UC_X86_REG_BP)
    elif callee == "vm_c2_descript_lookup":
        call["es"] = cpu.reg_read(UC_X86_REG_ES)
        call["di"] = cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF
    elif callee == "pbm_image_load_and_decode":
        call["path"] = [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_SI)]
        call["buffer"] = [
            cpu.reg_read(UC_X86_REG_ES),
            cpu.reg_read(UC_X86_REG_EDI) & 0xFFFF,
        ]
        call["palette_refresh"] = cpu.mem_read(GLOBALS + OFFSETS["palette_refresh"], 1)[
            0
        ]
        call["transparent_zero"] = cpu.mem_read(
            GLOBALS + OFFSETS["transparent_zero"], 1
        )[0]
    elif callee == "full_screen_blit":
        call["source"] = [cpu.reg_read(UC_X86_REG_DS), cpu.reg_read(UC_X86_REG_SI)]
    elif callee == "back_buffer_fill":
        call["ax"] = cpu.reg_read(UC_X86_REG_AX)
        call["top"] = struct.unpack(
            "<H", cpu.mem_read(GLOBALS + OFFSETS["rect_top"], 2)
        )[0]
        call["bottom"] = struct.unpack(
            "<H", cpu.mem_read(GLOBALS + OFFSETS["rect_bottom"], 2)
        )[0]
    return call


def execute(
    executable: bytes, case: dict[str, object], case_index: int
) -> dict[str, object]:
    globals_before, records_before, back_before, caller_es_before = initialize(
        case, case_index
    )
    expected_globals, expected_calls = expected(case, case_index, globals_before)
    module = executable[HEADER_SIZE:]
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, module)
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(RECORDS, bytes(records_before))
    cpu.mem_write(BACK_BUFFER, bytes(back_before))
    cpu.mem_write(CALLER_ES, bytes(caller_es_before))
    cpu.mem_write(
        STACK + STACK_POINTER,
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL,
    )

    initial_registers = {
        UC_X86_REG_EAX: 0xA5A57E00 + case_index,
        UC_X86_REG_EBX: 0xB6B62345 + case_index,
        UC_X86_REG_ECX: 0xC7C73456 + case_index,
        UC_X86_REG_EDX: 0xD8D84567 + case_index,
        UC_X86_REG_ESI: 0xE9E95678 + case_index,
        UC_X86_REG_EDI: 0xFAFA6789 + case_index,
        UC_X86_REG_EBP: 0xABCD789A + case_index,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GLOBALS // 16,
        UC_X86_REG_ES: CALLER_ES // 16,
        UC_X86_REG_FS: CALLER_FS // 16,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    reached_return = False

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal reached_return
        if address == RETURN_LINEAR:
            reached_return = True
            machine.emu_stop()
            return
        file_offset = address + HEADER_SIZE
        callee = EXTERNALS.get(file_offset)
        if callee is not None:
            calls.append(capture_external(machine, callee))
            if callee == "bridge_steer_update" and "bridge_phase" in case:
                machine.mem_write(
                    GLOBALS + OFFSETS["phase"], bytes((int(case["bridge_phase"]),))
                )
            elif callee == "alien_overlay_cycle":
                if "alien_active" in case:
                    machine.mem_write(
                        GLOBALS + OFFSETS["presentation_active"],
                        bytes((int(case["alien_active"]),)),
                    )
                if "alien_c2_gate" in case:
                    machine.mem_write(
                        GLOBALS + OFFSETS["c2_gate"],
                        bytes((int(case["alien_c2_gate"]),)),
                    )
            far_return(machine)
            return
        assert (
            image_address(COORDINATOR[0])
            <= address
            < address + size
            <= image_address(COORDINATOR[1])
        ), (case["name"], hex(file_offset))

    def write_hook(_cpu, _access, address, size, _value, _context) -> None:
        assert any(
            start <= address < address + size <= start + SEGMENT_SIZE
            for start in (GLOBALS, STACK)
        ), (case["name"], hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(COORDINATOR[0]), 0, count=5000)
    assert reached_return, case["name"]
    assert calls == expected_calls, (case["name"], calls, expected_calls)

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
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
    assert bytes(cpu.mem_read(RECORDS, SEGMENT_SIZE)) == bytes(records_before)
    assert bytes(cpu.mem_read(BACK_BUFFER, SEGMENT_SIZE)) == bytes(back_before)
    assert bytes(cpu.mem_read(CALLER_ES, SEGMENT_SIZE)) == bytes(caller_es_before)
    assert bytes(cpu.mem_read(0, len(module))) == module

    palette_changed = (
        globals_after[OFFSETS["target_palette"] : OFFSETS["target_palette"] + 192]
        != globals_before[OFFSETS["target_palette"] : OFFSETS["target_palette"] + 192]
    )
    for register, initial in initial_registers.items():
        if register in (UC_X86_REG_SP, UC_X86_REG_EFLAGS):
            continue
        expected_register = initial
        if register == UC_X86_REG_ECX and palette_changed:
            expected_register &= 0xFFFF0000
        assert cpu.reg_read(register) == expected_register, (
            case["name"],
            register,
            hex(cpu.reg_read(register)),
            hex(expected_register),
        )
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_POINTER + 2
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )

    changed_offsets = [
        offset
        for offset, (before, after) in enumerate(
            zip(globals_before, globals_after, strict=True)
        )
        if before != after
    ]
    flags = cpu.reg_read(UC_X86_REG_EFLAGS)
    return {
        "name": case["name"],
        "phase_before": int(case["phase"]),
        "record_kind": int(case["record_kind"]),
        "c2_gate_before": int(case.get("c2_gate", 0)),
        "active_line_before": int(case.get("active_line", 0x0033)),
        "calls": calls,
        "phase_after": read8(globals_after, OFFSETS["phase"]),
        "clip_snapshot_flags": read16(globals_after, OFFSETS["clip_snapshot_flags"]),
        "scene_record_offset": read16(globals_after, OFFSETS["scene_record"]),
        "active_line_after": read16(globals_after, OFFSETS["active_line"]),
        "scene_gate_after": read8(globals_after, OFFSETS["scene_gate"]),
        "c2_gate_after": read8(globals_after, OFFSETS["c2_gate"]),
        "palette_refresh_after": read8(globals_after, OFFSETS["palette_refresh"]),
        "transparent_zero_after": read8(globals_after, OFFSETS["transparent_zero"]),
        "target_high_sha256": hashlib.sha256(
            globals_after[OFFSETS["target_palette"] : OFFSETS["target_palette"] + 192]
        ).hexdigest(),
        "source_high_sha256": hashlib.sha256(
            globals_after[OFFSETS["source_palette"] : OFFSETS["source_palette"] + 192]
        ).hexdigest(),
        "changed_byte_count": len(changed_offsets),
        "changed_offsets": changed_offsets,
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
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
        "return": "near",
    }


def normalized_call(call: dict[str, object]) -> dict[str, object]:
    normalized = {key: value for key, value in call.items() if key != "return"}
    if normalized["callee"] == "pbm_image_load_and_decode":
        path = list(normalized["path"])
        path[1] = 0x00F3
        normalized["path"] = path
    return normalized


def compare_commander(rows: list[dict[str, object]], commander_path: Path) -> None:
    commander_rows = json.loads(commander_path.read_text())
    assert len(rows) == len(commander_rows) == 21
    fields = (
        "name",
        "phase_before",
        "record_kind",
        "c2_gate_before",
        "active_line_before",
        "phase_after",
        "clip_snapshot_flags",
        "scene_record_offset",
        "active_line_after",
        "scene_gate_after",
        "c2_gate_after",
        "palette_refresh_after",
        "transparent_zero_after",
        "target_high_sha256",
        "source_high_sha256",
        "changed_byte_count",
        "return",
    )
    for sequel, commander in zip(rows, commander_rows, strict=True):
        for field in fields:
            assert sequel[field] == commander[field], (sequel["name"], field)
        assert [normalized_call(call) for call in sequel["calls"]] == [
            normalized_call(call) for call in commander["calls"]
        ], sequel["name"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--commander-vectors",
        type=Path,
        default=Path(__file__).parent / "oracle_vectors/func_1855_natural.json",
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
    print(f"verified {len(rows)} original BBB scene-transition cases")


if __name__ == "__main__":
    main()
