#!/usr/bin/env python3
"""Drive the game to Pterra and capture matched original/relinked state.

Launches dosbox-x exactly like BLOOD.BAT does. The authentic-save route opens
the game's own load menu, waits for GAME1.SAV and its presentation to finish,
then emits the recovered Pterra destination event. From that boundary onward
the capture stops at the first interrupt-vector mutation, invalid instruction,
transition hang, or successful completion of SCRIPT2 ``proc pter``. The
driver advances each dialogue hold, selects ``exxos`` and ``teleport`` through
the game's list widget, then requires fault-free runtime after the presentation
closes. This avoids treating the first Pterra dialogue frame as a pass.

    { cpu: cs:ip + segments + registers,
      ivt: 1024 bytes of interrupt vectors,
      resource_band: words DS:0x0A40..0x0B00,
      back_buffer_area: bytes DS:0x5219..0x5240 }

Usage:
  python3 -P re/tools/capture_pterra_boundary.py \
      --cd-dir output/recovered_dos_package/cd --executable BPRG_RE.EXE \
      --install-parent /tmp/... --output state.json [--display :83]
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
import re
import signal
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOCATOR_ANCHOR = b"386 minimum !\0Not enough memory (570Ko min) !\0"
ADAPTER_TRACE_MAGIC = b"CBOPEN1\0"
PTRACE_ATTACH = 16
PTRACE_DETACH = 17
PTRACE_CONT = 7
VM_RESOURCE_HANDLES_OFFSET = 0x6712
VM_RESOURCE_IMAGES_OFFSET = 0x671C
VM_RESOURCE_PROFILE_INDEX_OFFSET = 0x677E
VM_SCRIPT_PROFILE_REQUEST_OFFSET = 0x6780
VM_EXECUTION_ENABLED_OFFSET = 0x67A8
VM_ACTIVE_LINE_OFFSET = 0x6788
VM_DISPLAYED_LINE_OFFSET = 0x678A
VM_C2_PRESENTATION_GATE_OFFSET = 0x1FB2
VM_PRESENTATION_REQUEST_FLAGS_OFFSET = 0x67AA
LIST_D8C_STATE_OFFSET = 0x0D5F
VM_RESOURCE_PROFILES_OFFSET = 0x11F4
VM_RESOURCE_COUNT = 5
SCRIPT2_PROFILE = 1
SCRIPT2_BLOOD_RECORD = 0x0028
SCRIPT2_SCRUTER_JO_ACTION_OFFSET = 0x0744
SCRIPT2_PTERRA_RECORD = 0x0DA0
SCRIPT2_ARCHETYPE_CURRENT_LOCATION_OFFSET = 0x0F4E
BLOODPRG_VM_RECORD_C4 = 0x00C4
VM_SHIP_ACTIVE_FLAGS_OFFSET = 0x24F3
VM_SEQUENCE_ACTIVE_OFFSET = 0x252A
VM_RECORD_BASE_POINTER_OFFSET = 0x6724
NAV_DEFERRED_RECORD_LINK_OFFSET = 0x676A
SCENE_TRANSITION_FLAGS_OFFSET = 0x2751
VM_UI_FLAGS_OFFSET = 0x2793
VM_UI_STATE_OFFSET = 0x2792
BRIDGE_PANORAMA_FRAME_OFFSET = 0x2795
LOAD_REQUEST_ACTIVE_OFFSET = 0x2737
SAVE_SLOT_MENU_PHASE_OFFSET = 0x2738
MOUSE_X_OFFSET = 0x0A2A
MOUSE_Y_OFFSET = 0x0A2C
MOUSE_PRIMARY_PRESSED_OFFSET = 0x0A3E
MOUSE_SECONDARY_PRESSED_OFFSET = 0x0A3F
MOUSE_PRESS_PENDING_OFFSET = 0x0A40
PRESENTATION_CHOICE_RESULT_OFFSET = 0x0ACA
PRESENTATION_CHOICE_ACTIVE_OFFSET = 0x259B
PRESENTATION_CHOICE_PHASE_OFFSET = 0x259C
VM_WORD_CHOICE_ACTIVE_OFFSET = 0x27D7
VM_OPERAND_WORD_COUNT_OFFSET = 0x27CF
VM_TEXT_MENU_END_OFFSET = 0x27D3
CHOICE_RECT_OFFSET = 0x2AAB
BRIDGE_STATIONS_OFFSET = 0x2A1B
BRIDGE_STATION_SIZE = 0x18
BRIDGE_STATION_COUNT = 4
VM_SCENE_GATE_OFFSET = 0x274F
RESOURCE_VERTICAL_OFFSET = 0x1FA7
RESOURCE_INDEX_OFFSET = 0x1FB5
RESOURCE_FRAME_PRESENTED_OFFSET = 0x0DB8
VM_PRESENTATION_SELECTED_WORD_OFFSET = 0x6796
VM_PRESENTATION_TEXT_WAIT_OFFSET = 0x67BA
VM_DIALOGUE_HOLD_COMPLETE_OFFSET = 0x67BB
VM_PRESENTATION_HOLD_READY_OFFSET = 0x67BC
VM_BLOCK_MATCH_VALUE_OFFSET = 0x6762
VM_PRESENTATION_DEFER_OFFSET = 0x67B0
VM_TEXT_DISPLAY_ACTIVE_OFFSET = 0x5E64
VM_PRESENTATION_WORD_BUFFER_OFFSET = 0x67F8
SCRIPT2_EXXOS_WORD = 0x0171
SCRIPT2_TELEPORT_WORD = 0x02A8
ILLEGAL_INTERRUPT_RE = re.compile(
    rb"Illegal Unhandled Interrupt Called ([0-9]+)", re.IGNORECASE)
DOS_READ_WARNING_RE = re.compile(
    rb"INT 21h READ warning: DX=([0-9a-f]+)h CX=([0-9a-f]+)h "
    rb"exceeds 64KB",
    re.IGNORECASE)
# Both BLOODPRG.EXE and BPRG_RE.EXE temporarily replace INT 0F while loading
# Pterra resources, then restore the DOSBox default vector.
TRANSIENT_INTERRUPT_VECTORS = frozenset((0x0F,))

TELEPORT_BLOCKERS = (
    ("vm_ui", 0x2793, 0x0E),
    ("ship", 0x24F3, 0xFF),
    ("render", 0x2751, 0xFF),
    ("presentation", 0x67AC, 0xFF),
    ("presentation_defer", 0x67B0, 0xFF),
    ("text", 0x5E64, 0xFF),
    ("nav_choice", 0x2565, 0xFF),
    ("save", 0x2736, 0xFF),
    ("load", 0x2737, 0xFF),
    ("nav_transition", 0x27DA, 0xFF),
    ("nav_actor_transition", 0x2792, 0xFF),
)


def libc_ptrace():
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long,
                            ctypes.c_void_p, ctypes.c_void_p]
    return libc


def locate_cpu_state(pid):
    executable = os.path.realpath(f"/proc/{pid}/exe")
    symbols = {}
    output = subprocess.check_output(
        ["nm", "-P", executable], text=True, stderr=subprocess.DEVNULL)
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
    if set(symbols) != {"Segs", "cpu_regs"}:
        return None
    image_base = None
    with open(f"/proc/{pid}/maps", encoding="ascii") as maps:
        for line in maps:
            fields = line.split()
            if len(fields) < 6:
                continue
            mapped = fields[-1].removesuffix(" (deleted)")
            if os.path.realpath(mapped) != executable:
                continue
            start = int(fields[0].split("-", 1)[0], 16)
            offset = int(fields[2], 16)
            image_base = start - offset
            break
    if image_base is None:
        return None
    return {name: image_base + offset for name, offset in symbols.items()}


def read_cpu_state(mem, addresses):
    if addresses is None:
        return None
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    segments = []
    for index in range(6):
        mem.seek(addresses["Segs"] + index * 8)
        segments.append(struct.unpack("<Q", mem.read(8))[0] & 0xffff)
    es, cs, ss, ds, fs, gs = segments
    return {"cs": cs, "ip": ip & 0xFFFF, "ds": ds, "es": es, "ss": ss,
            "fs": fs, "gs": gs,
            # dosbox cpu_regs stores AX,CX,DX,BX,SP,BP,SI,DI (x86 order)
            "ax": registers[0] & 0xFFFF, "cx": registers[1] & 0xFFFF,
            "dx": registers[2] & 0xFFFF, "bx": registers[3] & 0xFFFF,
            "sp": registers[4] & 0xFFFF, "bp": registers[5] & 0xFFFF,
            "si": registers[6] & 0xFFFF, "di": registers[7] & 0xFFFF}


def read_guest(mem, guest_base: int, linear: int, size: int) -> bytes:
    mem.seek(guest_base + linear)
    data = mem.read(size)
    if len(data) != size:
        raise RuntimeError(
            f"short guest read at {linear:#x}: {len(data)} of {size}")
    return data


def write_guest(mem, guest_base: int, linear: int, data: bytes) -> None:
    mem.seek(guest_base + linear)
    if mem.write(data) != len(data):
        raise RuntimeError(f"short guest write at {linear:#x}")
    mem.flush()


def send_mouse_button(display: str, pressed: bool) -> None:
    env = dict(os.environ, DISPLAY=display)
    subprocess.run(
        ["xdotool", "mousedown" if pressed else "mouseup", "1"],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)


def choice_row_point(rect: tuple[int, int, int, int],
                     row_index: int) -> tuple[int, int]:
    x, y, width, _height = rect
    return x + width // 2, y + 4 + row_index * 11 + 5


def activate_script2_pterra_procedure(mem, guest_base: int,
                                      game_segment: int) -> tuple[int, int]:
    game = game_segment * 16
    record_offset, record_segment = struct.unpack(
        "<HH", read_guest(
            mem, guest_base, game + VM_RECORD_BASE_POINTER_OFFSET, 4))
    if record_segment < 0x0050:
        raise RuntimeError(
            "invalid VM record-base pointer "
            f"{record_segment:04x}:{record_offset:04x}")
    location_linear = (record_segment * 16 + record_offset
                       + SCRIPT2_ARCHETYPE_CURRENT_LOCATION_OFFSET)
    encoded_location = struct.pack("<H", SCRIPT2_PTERRA_RECORD)
    write_guest(mem, guest_base, location_linear, encoded_location)
    if read_guest(mem, guest_base, location_linear, 2) != encoded_location:
        raise RuntimeError(
            "SCRIPT2 arche.current_location write did not persist")

    action_linear = (record_segment * 16 + record_offset
                     + SCRIPT2_SCRUTER_JO_ACTION_OFFSET)
    encoded_action = struct.pack(
        "<HHH", BLOODPRG_VM_RECORD_C4, SCRIPT2_BLOOD_RECORD, 0)
    write_guest(mem, guest_base, action_linear, encoded_action)
    if read_guest(mem, guest_base, action_linear, 6) != encoded_action:
        raise RuntimeError("SCRIPT2 Scruter_Jo.action write did not persist")

    sequence_linear = game + VM_SEQUENCE_ACTIVE_OFFSET
    sequence_flags = read_guest(mem, guest_base, sequence_linear, 1)[0]
    write_guest(mem, guest_base, sequence_linear,
                bytes((sequence_flags | 1,)))
    if read_guest(mem, guest_base, sequence_linear, 1)[0] & 1 == 0:
        raise RuntimeError("SCRIPT2 travel-context write did not persist")

    ship_state_linear = game + VM_SHIP_ACTIVE_FLAGS_OFFSET
    encoded_ship_state = struct.pack("<H", 1)
    write_guest(mem, guest_base, ship_state_linear, encoded_ship_state)
    if read_guest(mem, guest_base, ship_state_linear, 2) \
            != encoded_ship_state:
        raise RuntimeError("ship presentation state write did not persist")
    return record_segment, record_offset


def read_near_string(mem, guest_base: int, game: int, offset: int,
                     limit: int = 64) -> str:
    if offset == 0xffff:
        return "<none>"
    data = read_guest(mem, guest_base, game + offset, limit)
    return data.split(b"\0", 1)[0].decode("ascii", errors="replace")


def read_resource_flow(mem, guest_base: int,
                       game_segment: int) -> dict[str, object]:
    game = game_segment * 16

    def word(offset: int) -> int:
        return struct.unpack(
            "<H", read_guest(mem, guest_base, game + offset, 2))[0]

    def dword(offset: int) -> int:
        return struct.unpack(
            "<I", read_guest(mem, guest_base, game + offset, 4))[0]

    entries = {}
    for line in (0x29, 0x2A, 0x2B):
        descriptor, image_path = struct.unpack(
            "<HH", read_guest(
                mem, guest_base,
                game + RESOURCE_INDEX_OFFSET + line * 4, 4))
        entries[str(line)] = {
            "descriptor_offset": descriptor,
            "flags": read_guest(
                mem, guest_base, game + descriptor, 1)[0],
            "variant": read_guest(
                mem, guest_base, game + descriptor + 1, 1)[0],
            "filename": read_near_string(
                mem, guest_base, game, descriptor + 2),
            "image_path_offset": image_path,
            "image_path": read_near_string(
                mem, guest_base, game, image_path),
        }
    return {
        "file_handle": word(0x0D5B),
        "flags": word(0x0D76),
        "range_start": dword(0x0D6E),
        "range_remaining": dword(0x0D72),
        "index_start": dword(0x0D78),
        "index_remaining": dword(0x0D7C),
        "requested_id": word(0x0D80),
        "active_id": word(0x0D82),
        "source_offset": dword(0x0D84),
        "source_remaining": dword(0x0D88),
        "head_offset": word(0x0D8C),
        "head_segment": word(0x0D8E),
        "byte_count": word(0x0D9A),
        "iteration_count": word(0x0DA0),
        "entry_metric": word(0x0DAF),
        "path_is_embedded": read_guest(
            mem, guest_base, game + 0x0AE2, 1)[0],
        "source_is_banked": read_guest(
            mem, guest_base, game + 0x0DBC, 1)[0],
        "raw_d50_dc0": read_guest(
            mem, guest_base, game + 0x0D50, 0x70).hex(),
        "scene_entries": entries,
    }


def read_adapter_trace(mem, guest_base: int,
                       game_segment: int) -> dict[str, object] | None:
    game = game_segment * 16
    data = read_guest(mem, guest_base, game, 0x10000)
    offset = data.find(ADAPTER_TRACE_MAGIC)
    if offset < 0:
        return None
    fields = struct.unpack_from("<8H", data, offset + len(ADAPTER_TRACE_MAGIC))
    path = data[offset + 24:offset + 40].split(b"\0", 1)[0]
    return {
        "offset": offset,
        "open_call_count": fields[0],
        "path_offset": fields[1],
        "path_segment": fields[2],
        "handle_before": fields[3],
        "dos_ax": fields[4],
        "carry": fields[5],
        "success": fields[6],
        "handle_after": fields[7],
        "path": path.decode("ascii", errors="replace"),
    }


def read_profile_state(mem, guest_base: int, game_segment: int,
                       fs_segment: int) -> dict[str, object]:
    game = game_segment * 16
    fs = fs_segment * 16
    profile = struct.unpack(
        "<H", read_guest(mem, guest_base,
                          game + VM_RESOURCE_PROFILE_INDEX_OFFSET, 2))[0]
    request = struct.unpack(
        "<h", read_guest(mem, guest_base,
                          game + VM_SCRIPT_PROFILE_REQUEST_OFFSET, 2))[0]
    handles = struct.unpack(
        f"<{VM_RESOURCE_COUNT}H",
        read_guest(mem, guest_base, game + VM_RESOURCE_HANDLES_OFFSET,
                   VM_RESOURCE_COUNT * 2))
    expected = ()
    if 0 <= profile < 5:
        expected = struct.unpack(
            f"<{VM_RESOURCE_COUNT}H",
            read_guest(
                mem, guest_base,
                fs + VM_RESOURCE_PROFILES_OFFSET
                + profile * VM_RESOURCE_COUNT * 2,
                VM_RESOURCE_COUNT * 2))
    images = tuple(
        struct.unpack(
            "<HH",
            read_guest(mem, guest_base,
                       game + VM_RESOURCE_IMAGES_OFFSET + index * 4, 4))
        for index in range(VM_RESOURCE_COUNT)
    )
    blockers = {
        name: read_guest(mem, guest_base, game + offset, 1)[0] & mask
        for name, offset, mask in TELEPORT_BLOCKERS
    }
    mouse_x, mouse_y = struct.unpack(
        "<hh", read_guest(
            mem, guest_base, game + MOUSE_X_OFFSET, 4))
    choice_rect = struct.unpack(
        "<hhhh", read_guest(
            mem, guest_base, game + CHOICE_RECT_OFFSET, 8))
    bridge_stations = []
    for index in range(BRIDGE_STATION_COUNT):
        station = game + BRIDGE_STATIONS_OFFSET \
            + index * BRIDGE_STATION_SIZE
        flags = struct.unpack(
            "<H", read_guest(mem, guest_base, station, 2))[0]
        seek_target = struct.unpack(
            "<H", read_guest(mem, guest_base, station + 0x0A, 2))[0]
        hit_rect = struct.unpack(
            "<hhhh", read_guest(mem, guest_base, station + 0x0C, 8))
        bridge_stations.append({
            "index": index,
            "flags": flags,
            "seek_target": seek_target,
            "hit_rect": list(hit_rect),
        })
    return {
        "profile": profile,
        "request": request,
        "execution_enabled": read_guest(
            mem, guest_base, game + VM_EXECUTION_ENABLED_OFFSET, 1)[0],
        "handles": list(handles),
        "expected_handles": list(expected),
        "images": [f"{segment:04x}:{offset:04x}"
                   for offset, segment in images],
        "blockers": blockers,
        "input": {
            "mouse_x": mouse_x,
            "mouse_y": mouse_y,
            "primary_pressed": read_guest(
                mem, guest_base,
                game + MOUSE_PRIMARY_PRESSED_OFFSET, 1)[0],
            "secondary_pressed": read_guest(
                mem, guest_base,
                game + MOUSE_SECONDARY_PRESSED_OFFSET, 1)[0],
            "press_pending": read_guest(
                mem, guest_base,
                game + MOUSE_PRESS_PENDING_OFFSET, 1)[0],
            "save_menu_phase": read_guest(
                mem, guest_base,
                game + SAVE_SLOT_MENU_PHASE_OFFSET, 1)[0],
            "choice_rect": list(choice_rect),
            "choice_active": read_guest(
                mem, guest_base,
                game + PRESENTATION_CHOICE_ACTIVE_OFFSET, 1)[0],
            "choice_phase": read_guest(
                mem, guest_base,
                game + PRESENTATION_CHOICE_PHASE_OFFSET, 1)[0],
            "choice_result": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + PRESENTATION_CHOICE_RESULT_OFFSET, 2))[0],
            "word_choice_active": read_guest(
                mem, guest_base,
                game + VM_WORD_CHOICE_ACTIVE_OFFSET, 1)[0],
            "word_choice_phase": read_guest(
                mem, guest_base,
                game + VM_PRESENTATION_TEXT_WAIT_OFFSET, 1)[0],
            "selected_word": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_PRESENTATION_SELECTED_WORD_OFFSET, 2))[0],
            "bridge_stations": bridge_stations,
            "bridge_panorama_frame": read_guest(
                mem, guest_base,
                game + BRIDGE_PANORAMA_FRAME_OFFSET, 1)[0],
        },
        "initialized": (
            request == -1
            and handles == expected
            and bool(expected)
            and all(segment != 0 for _, segment in images)
        ),
        "scene_flow": {
            "active_line": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_ACTIVE_LINE_OFFSET, 2))[0],
            "c2_presentation_gate": read_guest(
                mem, guest_base,
                game + VM_C2_PRESENTATION_GATE_OFFSET, 1)[0],
            "list_d8c_state": read_guest(
                mem, guest_base, game + LIST_D8C_STATE_OFFSET, 1)[0],
            "list_file_handle": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D5B, 2))[0],
            "list_read_wrap_index": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D60, 2))[0],
            "list_wrap_count": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D62, 2))[0],
            "list_read_wrap_limit": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D64, 2))[0],
            "list_secondary_wrap_limit": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D66, 2))[0],
            "resource_source_offset": struct.unpack(
                "<I", read_guest(
                    mem, guest_base, game + 0x0D84, 4))[0],
            "resource_source_remaining": struct.unpack(
                "<I", read_guest(
                    mem, guest_base, game + 0x0D88, 4))[0],
            "list_head": "%04x:%04x" % struct.unpack(
                "<HH", read_guest(
                    mem, guest_base, game + 0x0D8C, 4))[::-1],
            "list_tail": "%04x:%04x" % struct.unpack(
                "<HH", read_guest(
                    mem, guest_base, game + 0x0D90, 4))[::-1],
            "list_active": "%04x:%04x" % struct.unpack(
                "<HH", read_guest(
                    mem, guest_base, game + 0x0D94, 4))[::-1],
            "list_buffer_end": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D98, 2))[0],
            "list_queued_bytes": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0D9A, 2))[0],
            "list_iteration_count": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0DA0, 2))[0],
            "list_rollover_state": read_guest(
                mem, guest_base, game + 0x0DAC, 1)[0],
            "list_entry_metric": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0DAF, 2))[0],
            "operand_word_count": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_OPERAND_WORD_COUNT_OFFSET, 2))[0],
            "text_menu_end": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_TEXT_MENU_END_OFFSET, 2))[0],
            "dialogue_hold_complete": read_guest(
                mem, guest_base,
                game + VM_DIALOGUE_HOLD_COMPLETE_OFFSET, 1)[0],
            "presentation_hold_ready": read_guest(
                mem, guest_base,
                game + VM_PRESENTATION_HOLD_READY_OFFSET, 1)[0],
        },
        "audio_flow": {
            "voc_playback_enabled": read_guest(
                mem, guest_base, game + 0x0ADE, 1)[0],
            "game_mode": read_guest(
                mem, guest_base, game + 0x0ADF, 1)[0],
            "timer_hook_active": read_guest(
                mem, guest_base, game + 0x0B21, 1)[0],
            "timer_tick": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0B29, 2))[0],
            "frame_delay": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0B2D, 2))[0],
            "dialogue_delay": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0B33, 2))[0],
            "dialogue_hold_countdown": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0B35, 2))[0],
            "bank_clip_count": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0BBB, 2))[0],
            "bank_dialogue_delay_base": read_guest(
                mem, guest_base, game + 0x0BBD, 1)[0],
            "bank_dialogue_delay_limit": read_guest(
                mem, guest_base, game + 0x0BBE, 1)[0],
            "last_clip": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0C4D, 2))[0],
            "streamed_clip_count": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0C53, 2))[0],
            "dialogue_seed": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + 0x0C55, 2))[0],
            "text_mode_seed": read_guest(
                mem, guest_base, game + 0x0CF9, 1)[0],
            "text_mode_play": read_guest(
                mem, guest_base, game + 0x0CFA, 1)[0],
            "text_voice_trigger": read_guest(
                mem, guest_base, game + 0x0CFB, 1)[0],
        },
        "resource_flow": read_resource_flow(
            mem, guest_base, game_segment),
        "adapter_trace": read_adapter_trace(
            mem, guest_base, game_segment),
    }


def profile_completed(state: dict[str, object], target: int) -> bool:
    return bool(
        state["initialized"]
        and state["profile"] == target
        and state["execution_enabled"] == 1)


def profile_releaseable(state: dict[str, object]) -> bool:
    blockers = state["blockers"]
    assert isinstance(blockers, dict)
    return (
        blockers.get("vm_ui") == 4
        and all(value == 0 for name, value in blockers.items()
                if name != "vm_ui")
    )


def game_is_ready(mem, anchor: int) -> bool:
    mem.seek(anchor + 0x0A46)
    free_bytes = struct.unpack("<I", mem.read(4))[0]
    mem.seek(anchor + 0x0A9E)
    crtc_port = struct.unpack("<H", mem.read(2))[0]
    mem.seek(anchor + 0x0B21)
    timer_hook_active = mem.read(1)
    return (
        0 < free_bytes <= 0x000A0000
        and crtc_port == 0x03D4
        and timer_hook_active == b"\x01"
    )


def find_ds_anchor(pid, mem, game_segment=None):
    overlap = len(LOCATOR_ANCHOR) - 1
    for line in open(f"/proc/{pid}/maps"):
        pr = line.split()
        if "r" not in pr[1] or "-" not in pr[0]:
            continue
        start, end = [int(x, 16) for x in pr[0].split("-")]
        size = end - start
        if size < 0x100000 or size > 300_000_000:
            continue
        cursor = start
        tail = b""
        while cursor < end:
            amount = min(2 * 1024 * 1024, end - cursor)
            try:
                mem.seek(cursor)
                chunk = mem.read(amount)
            except Exception:
                break
            haystack = tail + chunk
            search_from = 0
            while True:
                index = haystack.find(LOCATOR_ANCHOR, search_from)
                if index < 0:
                    break
                anchor = cursor - len(tail) + index
                guest_base = (
                    start if game_segment is None
                    else anchor - game_segment * 16)
                if not start <= guest_base or guest_base + 0x100000 > end:
                    search_from = index + 1
                    continue
                mem.seek(anchor + 0x0A46)
                free_bytes = struct.unpack("<I", mem.read(4))[0]
                mem.seek(anchor + 0x0A9E)
                crtc_port = struct.unpack("<H", mem.read(2))[0]
                mem.seek(guest_base + 0x0413)
                conventional_kib = struct.unpack("<H", mem.read(2))[0]
                mem.seek(guest_base + 0x21 * 4)
                int21 = struct.unpack("<I", mem.read(4))[0]
                if (0 < free_bytes <= 0x000A0000
                        and crtc_port in (0, 0x03D4)
                        and 128 <= conventional_kib <= 640
                        and int21 != 0):
                    return anchor, guest_base
                search_from = index + 1
            tail = haystack[-overlap:]
            cursor += amount
    return None, None


def bridge_prefix_actions() -> list[str]:
    """The proven gate prefix: logos -> title click -> CRYOBOX -> Bob."""
    return [
        "wait_title", "click 320 340", "wait 2",
        "move_relative -300 0", "wait 4",
        "move_relative -300 0", "wait 3",
        "move_relative 100 -20", "wait 0.5",
        "move_relative 100 -20", "wait 0.5",
        "mouse_button 1", "wait 1",
        "move_relative -100 -20", "wait 0.5",
        "move_relative -100 -20", "wait 0.5",
        "mouse_button 1", "fastforward 8", "wait 3",
        "move_relative 100 0", "wait 0.5", "mouse_button 1",
        "fastforward 5", "wait 1",
        "shot d_title",
        "key_down space", "fastforward 30", "key_up space",
        "fastforward 10", "wait 2", "shot d_bridge", "wait_bridge",
    ]


def rotation_lap(lap: int) -> list[str]:
    """One full rotation attempt: park right edge, center, orb click.

    The orb click coordinates come from accuracy/scenarios/nav_probe.tsv
    (125,118 in 320-space = 250,236 in the 640x400 window).
    """
    actions: list[str] = []
    for _ in range(6):
        actions += ["move 310 100", "wait 2"]
    actions += ["move 160 100", "wait 1"]
    for index in range(4):
        actions += [f"click {246 + 8 * index} 236", "wait 1"]
    actions += ["wait 3"]
    return actions


def run_driver(actions: list[str], display: str, executable: str) -> None:
    """Feed drive_real_game.sh's action vocabulary through xdotool directly.

    Re-implemented here (not via the shell driver) because the game is
    already running under this script's control.
    """
    env = dict(os.environ, DISPLAY=display, SDL_VIDEODRIVER="x11")
    window_id = ""
    executable_stem = Path(executable).stem
    for _ in range(40):
        output = subprocess.run(
            ["xdotool", "search", "--name", executable_stem],
            capture_output=True, text=True, env=env).stdout
        lines = [line for line in output.splitlines() if line.strip()]
        if lines:
            window_id = lines[0]
            break
        time.sleep(0.5)
    if not window_id:
        print("drive: game window not found")
        return
    # Focus + activate once, then use GLOBAL (XTEST) button events like
    # drive_real_game.sh does: per-window synthetic button events are
    # ignored by SDL's event pump on some windows.
    subprocess.run(["xdotool", "windowactivate", "--sync", window_id],
                   env=env)
    subprocess.run(["xdotool", "windowfocus", "--sync", window_id], env=env)

    def emit(action: str, a: str, b: str) -> None:
        if action == "click":
            subprocess.run(["xdotool", "mousemove", "--window", window_id,
                            a, b], env=env)
            time.sleep(0.3)
            subprocess.run(["xdotool", "mousedown", "1"], env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "mouseup", "1"], env=env)
        elif action == "move":
            subprocess.run(["xdotool", "mousemove", "--window", window_id,
                            str(int(a) * 2), str(int(b) * 2)], env=env)
        elif action == "move_relative":
            subprocess.run(["xdotool", "mousemove_relative", "--", a, b],
                           env=env)
        elif action == "mouse_button":
            subprocess.run(["xdotool", "mousedown", a], env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "mouseup", a], env=env)
        elif action == "key":
            subprocess.run(["xdotool", "keydown", "--window", window_id, a],
                           env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "keyup", "--window", window_id, a],
                           env=env)
        elif action == "key_down":
            subprocess.run(["xdotool", "keydown", "--window", window_id, a],
                           env=env)
        elif action == "key_up":
            subprocess.run(["xdotool", "keyup", "--window", window_id, a],
                           env=env)  # keys stay window-targeted (safe synth)

    for line in actions:
        parts = line.split()
        if not parts:
            continue
        verb = parts[0]
        arguments = parts[1:] + ["", ""]
        if verb == "wait" or verb == "fastforward":
            duration = float(arguments[0])
            if verb == "fastforward":
                subprocess.run(["xdotool", "keydown", "--window", window_id,
                                "Alt_L"], env=env)
                subprocess.run(["xdotool", "keydown", "--window", window_id,
                                "F12"], env=env)
                time.sleep(duration)
                subprocess.run(["xdotool", "keyup", "--window", window_id,
                                "F12"], env=env)
                subprocess.run(["xdotool", "keyup", "--window", window_id,
                                "Alt_L"], env=env)
            else:
                time.sleep(duration)
            continue
        elif verb == "wait_title":
            title_reached = False
            for attempt in range(120):
                probe = f"/tmp/opencode/driveshots/title_{attempt}.png"
                subprocess.run(["import", "-window", window_id, probe],
                               env=env)
                stats = subprocess.run(
                    ["magick", probe, "-crop", "100x100+270+290",
                     "-format", "%[fx:mean.r] %[fx:mean.g] %[fx:mean.b]",
                     "info:"],
                    capture_output=True, text=True, env=env).stdout
                values = [float(text) for text in stats.split()]
                if len(values) == 3 and values[0] > 0.5 \
                        and values[1] < 0.4 and values[2] < 0.2:
                    print(f"drive: title reached on attempt {attempt}")
                    title_reached = True
                    break
                time.sleep(0.2)
            if not title_reached:
                raise RuntimeError("title gate timed out")
        elif verb == "wait_bridge":
            # Observe the bridge gate without injecting input. A failed visual
            # classification can mean that a legitimate call is still playing;
            # Escape/title clicks at that point restart its resource stream.
            bridge_reached = False
            for attempt in range(8):
                probe = f"/tmp/opencode/driveshots/probe_{attempt}.png"
                subprocess.run(["import", "-window", window_id, probe],
                               env=env)
                stats = subprocess.run(
                    ["magick", probe, "-format",
                     "%[fx:mean.r] %[fx:mean.g] %[fx:mean.b] "
                     "%[fx:standard_deviation.g]", "info:"],
                    capture_output=True, text=True, env=env).stdout
                values = [float(text) for text in stats.split()]
                if len(values) == 4 and values[2] > 0.12 \
                        and values[0] < 0.15 and values[3] < 0.12:
                    print(f"drive: bridge reached on attempt {attempt}")
                    bridge_reached = True
                    break
                time.sleep(4)
            if not bridge_reached:
                print("drive: bridge classifier timed out; input left untouched")
        elif verb == "shot":
            out_dir = os.environ.get("DRIVE_SHOT_DIR", ".")
            subprocess.run(["import", "-window", window_id,
                            f"{out_dir}/{arguments[0]}.png"],
                           env=env)
        else:
            emit(verb, arguments[0], arguments[1])


def snapshot_guest(mem, guest_base: int, anchor: int,
                   cpu_addresses: dict[str, int], marker: Path | None,
                   profile: dict[str, object] | None) -> dict[str, object]:
    state = read_cpu_state(mem, cpu_addresses)
    game_segment = (anchor - guest_base) // 16
    snapshot: dict[str, object] = {
        "cpu": state,
        "segments_minus_game_data": {
            name: state[name] - game_segment
            for name in ("cs", "ds", "es", "ss", "fs", "gs")
        },
    }
    code_linear = state["cs"] * 16 + state["ip"]
    if code_linear + 32 <= 0x100000:
        snapshot["code_at_cs_ip"] = read_guest(
            mem, guest_base, code_linear, 32).hex()
    stack_linear = state["ss"] * 16 + state["sp"]
    if stack_linear + 256 <= 0x100000:
        snapshot["stack_at_ss_sp"] = read_guest(
            mem, guest_base, stack_linear, 256).hex()
    bp_linear = state["ss"] * 16 + state["bp"]
    if bp_linear >= 64 and bp_linear + 192 <= 0x100000:
        snapshot["stack_around_ss_bp"] = {
            "start_offset": (state["bp"] - 64) & 0xFFFF,
            "bytes": read_guest(
                mem, guest_base, bp_linear - 64, 256).hex(),
        }
    snapshot["ivt"] = read_guest(mem, guest_base, 0, 0x400).hex()
    band = {}
    for offset in range(0x0A40, 0x0B00, 2):
        band[f"{offset:#06x}"] = struct.unpack(
            "<H", read_guest(
                mem, guest_base, game_segment * 16 + offset, 2))[0]
    snapshot["resource_band"] = band
    snapshot["back_buffer_area"] = read_guest(
        mem, guest_base, game_segment * 16 + 0x5219,
        0x5240 - 0x5219).hex()
    if marker is not None:
        snapshot["marker"] = str(marker)
    if profile is not None:
        snapshot["profile"] = profile
    return snapshot


def changed_interrupt_vectors(before: bytes, after: bytes) \
        -> list[dict[str, object]]:
    if len(before) != 0x400 or len(after) != 0x400:
        raise ValueError("an interrupt table must contain exactly 1024 bytes")
    changes = []
    for vector in range(256):
        start = vector * 4
        old_offset, old_segment = struct.unpack_from("<HH", before, start)
        new_offset, new_segment = struct.unpack_from("<HH", after, start)
        if (old_offset, old_segment) != (new_offset, new_segment):
            changes.append({
                "vector": vector,
                "before": f"{old_segment:04x}:{old_offset:04x}",
                "after": f"{new_segment:04x}:{new_offset:04x}",
                "changed_byte_offsets": [
                    start + index
                    for index in range(4)
                    if before[start + index] != after[start + index]
                ],
            })
    return changes


def stable_scene_release_is_safe(mem, guest_base: int,
                                 state: dict[str, int], game_segment: int,
                                 executable: str) -> bool:
    """Only alter scene globals after the active handler has returned."""
    if executable.upper() != "BPRG_RE.EXE":
        return state["cs"] != 0xF000

    code_segment = (game_segment - 0x105B) & 0xFFFF
    runtime_segment = (code_segment + 0x0F9B) & 0xFFFF
    if state["cs"] == code_segment:
        ip = state["ip"]
        if 0xC6D1 <= ip < 0xD700 or 0xF6DD <= ip < 0xFA00:
            return False
    stack_linear = state["ss"] * 16 + state["sp"]
    stack = read_guest(mem, guest_base, stack_linear, 256)
    words = struct.unpack("<128H", stack)
    if any(
        0xC6D1 <= value < 0xD700 or 0xF6DD <= value < 0xFA00
        for value in words
    ):
        return False
    if state["cs"] in (0xF000, runtime_segment):
        return True
    return True


def capture_state_pterra(db: subprocess.Popen[bytes], libc, marker: Path,
                         log_path: Path, timeout: float, display: str,
                         executable: str, manual: bool = False,
                         open_load_menu: bool = False,
                         trigger_pterra_after_load: bool = False,
                         drive_authentic_save: bool = False,
                         guest_snapshot: Path | None = None,
                         post_pter_seconds: float = 5.0,
                         diagnostic_mute_pterra: bool = False) \
        -> dict[str, object]:
    deadline = time.monotonic() + timeout
    cpu_addresses = None
    anchor = None
    guest_base = None
    fs_segment = None
    request_written = manual
    profile_loaded = manual
    pterra_triggered = manual and not trigger_pterra_after_load
    load_menu_requested = not open_load_menu
    authentic_save_loaded = False
    post_load_presentation_seen = False
    load_selection_started = False
    load_slot_pressing = False
    post_load_pressing = False
    guest_snapshot_written = False
    destination_committed = False
    pter_reached = False
    pter_reached_at = None
    pter_completed_at = None
    pter_last_progress_at = None
    pter_last_semantic_key = None
    pter_sustained = False
    pter_choice_was_active = False
    pter_choice_results: list[int] = []
    pter_input_pressed = None
    pter_next_input_at = 0.0
    diagnostic_audio_muted = False
    marker_snapshot = None
    fault_snapshot = None
    dos_read_overflow_snapshot = None
    integrity_fault_snapshot = None
    hang_snapshot = None
    pter_snapshot = None
    post_pter_snapshot = None
    post_pter_cpu_samples: list[dict[str, int]] = []
    ivt_baseline = None
    last_profile = None
    last_cpu_state = None
    last_profile_key = None
    log_offset = 0
    log_tail = b""
    completed_scene_lines: list[dict[str, int]] = []
    completed_scene_line_states: set[tuple[int, int]] = set()
    natural_scene_active_key = None
    stable_scene_line_key = None
    stable_scene_line_started = None
    unsafe_scene_release_points: set[tuple[int, int]] = set()
    invalid_resource_handle_stall_started = None
    final_transition_stall_started = None
    final_transition_cpu_samples: list[dict[str, int]] = []

    while time.monotonic() < deadline:
        # Match the invariant watchdog's cadence so the short idle-frame profile
        # handoff is observable before the title sequence changes state again.
        time.sleep(0.05)
        if db.poll() is not None:
            raise RuntimeError(f"dosbox exited early with {db.returncode}")

        illegal_interrupt = None
        dos_read_warning = None
        if log_path.exists():
            with log_path.open("rb") as stream:
                stream.seek(log_offset)
                chunk = stream.read()
                log_offset += len(chunk)
            combined_log = log_tail + chunk
            illegal_interrupt = ILLEGAL_INTERRUPT_RE.search(combined_log)
            dos_read_warning = (
                DOS_READ_WARNING_RE.search(combined_log) if chunk else None)
            log_tail = combined_log[-max(
                len(ILLEGAL_INTERRUPT_RE.pattern),
                len(DOS_READ_WARNING_RE.pattern)):]

        hit = next(marker.glob("PTERRA1[DFG].LBM"), None)
        attached = False
        ctypes.set_errno(0)
        if libc.ptrace(PTRACE_ATTACH, db.pid, None, None) != 0:
            time.sleep(0.01)
            continue
        os.waitpid(db.pid, 0)
        attached = True
        try:
            with open(f"/proc/{db.pid}/mem", "r+b", buffering=0) as mem:
                if cpu_addresses is None:
                    cpu_addresses = locate_cpu_state(db.pid)
                if cpu_addresses is None:
                    continue
                state = read_cpu_state(mem, cpu_addresses)
                last_cpu_state = state.copy()
                if anchor is None or guest_base is None:
                    if state["gs"] < 0x0050:
                        continue
                    anchor, guest_base = find_ds_anchor(
                        db.pid, mem, state["gs"])
                    if anchor is None or guest_base is None:
                        continue
                if not game_is_ready(mem, anchor):
                    continue

                game_segment = (anchor - guest_base) // 16
                if fs_segment is None:
                    fs_segment = state["fs"]
                last_profile = read_profile_state(
                    mem, guest_base, game_segment, fs_segment)
                scene_flow = last_profile["scene_flow"]
                profile_key = (
                    last_profile["profile"],
                    last_profile["request"],
                    tuple(last_profile["blockers"].items()),
                    scene_flow["active_line"],
                    scene_flow["c2_presentation_gate"],
                    scene_flow["list_d8c_state"],
                    (
                        last_profile["input"]["mouse_x"],
                        last_profile["input"]["mouse_y"],
                        last_profile["input"]["primary_pressed"],
                    ) if last_profile["blockers"]["load"] else (),
                    last_profile["input"]["save_menu_phase"],
                    tuple(last_profile["input"]["choice_rect"]),
                    last_profile["input"]["word_choice_active"],
                    last_profile["input"]["word_choice_phase"],
                    last_profile["input"]["selected_word"],
                    last_profile["input"]["bridge_panorama_frame"],
                    tuple(
                        (station["flags"], tuple(station["hit_rect"]))
                        for station in last_profile["input"]["bridge_stations"]
                    ),
                )
                if profile_key != last_profile_key:
                    last_profile_key = profile_key
                    print(
                        "state: "
                        f"profile={last_profile['profile']} "
                        f"request={last_profile['request']} "
                        f"blockers={last_profile['blockers']} "
                        f"flow={last_profile['scene_flow']} "
                        f"input={last_profile['input']}",
                        flush=True)

                if (manual and not load_menu_requested
                        and profile_releaseable(last_profile)):
                    game = game_segment * 16
                    write_guest(
                        mem, guest_base,
                        game + LOAD_REQUEST_ACTIVE_OFFSET, b"\x01")
                    write_guest(
                        mem, guest_base,
                        game + SAVE_SLOT_MENU_PHASE_OFFSET, b"\x01")
                    load_menu_requested = True
                    print("state: opened the original LOAD menu", flush=True)
                elif (not manual and not request_written
                        and profile_releaseable(last_profile)):
                    write_guest(
                        mem, guest_base,
                        game_segment * 16 + VM_SCRIPT_PROFILE_REQUEST_OFFSET,
                        struct.pack("<h", SCRIPT2_PROFILE))
                    ui_linear = game_segment * 16 + VM_UI_FLAGS_OFFSET
                    ui_flags = read_guest(mem, guest_base, ui_linear, 1)[0]
                    write_guest(mem, guest_base, ui_linear,
                                bytes((ui_flags & 0xFB,)))
                    request_written = True
                    print("state: requested SCRIPT2 profile", flush=True)
                elif (not manual and request_written and not profile_loaded
                        and profile_completed(last_profile, SCRIPT2_PROFILE)):
                    profile_loaded = True
                    print("state: SCRIPT2 profile loaded", flush=True)

                if (not manual and profile_loaded and not pterra_triggered):
                    blockers = last_profile["blockers"]
                    assert isinstance(blockers, dict)
                    if all(value == 0 for value in blockers.values()):
                        write_guest(
                            mem, guest_base,
                            game_segment * 16
                            + NAV_DEFERRED_RECORD_LINK_OFFSET,
                            struct.pack("<H", SCRIPT2_PTERRA_RECORD))
                        write_guest(
                            mem, guest_base,
                            game_segment * 16
                            + SCENE_TRANSITION_FLAGS_OFFSET,
                            b"\x01")
                        pterra_triggered = True
                        ivt_baseline = read_guest(mem, guest_base, 0, 0x400)
                        print(
                            "state: selected SCRIPT2 Pterra record 0x0da0",
                            flush=True)

                blockers = last_profile["blockers"]
                flow = last_profile["scene_flow"]
                assert isinstance(blockers, dict)
                assert isinstance(flow, dict)
                game = game_segment * 16
                if (drive_authentic_save
                        and not authentic_save_loaded
                        and blockers["load"] != 0):
                    write_guest(
                        mem, guest_base, game + MOUSE_X_OFFSET,
                        struct.pack("<h", 110))
                    write_guest(
                        mem, guest_base, game + MOUSE_Y_OFFSET,
                        struct.pack("<h", 47))
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                    if not load_slot_pressing:
                        load_selection_started = True
                        load_slot_pressing = True
                        print("state: pressed authentic save slot 1", flush=True)
                elif load_slot_pressing and not authentic_save_loaded:
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x00")
                    load_slot_pressing = False
                if (trigger_pterra_after_load
                        and not authentic_save_loaded
                        and profile_completed(last_profile, SCRIPT2_PROFILE)
                        and blockers["load"] == 0):
                    authentic_save_loaded = True
                    print("state: authentic GAME1.SAV load completed",
                          flush=True)
                if (trigger_pterra_after_load
                        and load_selection_started
                        and blockers["load"] == 0
                        and flow["active_line"] == 2
                        and flow["c2_presentation_gate"] == 1):
                    post_load_presentation_seen = True
                    if (guest_snapshot is not None
                            and not guest_snapshot_written):
                        guest_snapshot.parent.mkdir(parents=True, exist_ok=True)
                        guest_snapshot.write_bytes(read_guest(
                            mem, guest_base, game, 0x10000))
                        guest_snapshot_written = True
                        print(
                            f"state: wrote matched presentation snapshot "
                            f"{guest_snapshot}", flush=True)
                    if drive_authentic_save:
                        write_guest(
                            mem, guest_base, game + MOUSE_X_OFFSET,
                            struct.pack("<h", 110))
                        write_guest(
                            mem, guest_base, game + MOUSE_Y_OFFSET,
                            struct.pack("<h", 96))
                        write_guest(
                            mem, guest_base,
                            game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                        if not post_load_pressing:
                            post_load_pressing = True
                            send_mouse_button(display, True)
                            print(
                                "state: dismissed post-load presentation",
                                flush=True)
                elif post_load_pressing:
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x00")
                    send_mouse_button(display, False)
                    post_load_pressing = False
                if (trigger_pterra_after_load
                        and authentic_save_loaded
                        and post_load_presentation_seen
                        and not pterra_triggered
                        and flow["active_line"] == 0xffff
                        and flow["c2_presentation_gate"] == 0
                        and all(value == 0 for value in blockers.values())):
                    game = game_segment * 16
                    write_guest(
                        mem, guest_base,
                        game + NAV_DEFERRED_RECORD_LINK_OFFSET,
                        struct.pack("<H", SCRIPT2_PTERRA_RECORD))
                    write_guest(
                        mem, guest_base,
                        game + SCENE_TRANSITION_FLAGS_OFFSET, b"\x01")
                    pterra_triggered = True
                    ivt_baseline = read_guest(mem, guest_base, 0, 0x400)
                    print(
                        "state: emitted saved-game Pterra destination event "
                        "for record 0x0da0", flush=True)

                if ivt_baseline is not None:
                    current_ivt = read_guest(mem, guest_base, 0, 0x400)
                    ivt_changes = [
                        change for change in changed_interrupt_vectors(
                            ivt_baseline, current_ivt)
                        if change["vector"] not in TRANSIENT_INTERRUPT_VECTORS
                    ]
                    if ivt_changes:
                        integrity_fault_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        integrity_fault_snapshot["baseline_ivt"] = \
                            ivt_baseline.hex()
                        integrity_fault_snapshot["changed_vectors"] = \
                            ivt_changes
                        first = ivt_changes[0]
                        print(
                            "integrity: interrupt vector "
                            f"{first['vector']:#04x} changed from "
                            f"{first['before']} to {first['after']}",
                            flush=True)
                        break

                release_key = (
                    int(blockers["render"]), int(flow["active_line"])
                )
                stable_release_ready = False
                if (trigger_pterra_after_load
                        and pterra_triggered
                        and hit is not None
                        and flow["c2_presentation_gate"] == 1
                        and blockers["presentation"] == 0
                        and blockers["presentation_defer"] == 0
                        and blockers["text"] == 0
                        and release_key in (
                            (3, 0x0029), (9, 0x002B), (0x21, 0x002A))):
                    if stable_scene_line_key != release_key:
                        stable_scene_line_key = release_key
                        stable_scene_line_started = time.monotonic()
                    elif (stable_scene_line_started is not None
                            and time.monotonic() - stable_scene_line_started
                            >= 10.0):
                        stable_release_ready = stable_scene_release_is_safe(
                            mem, guest_base, state, game_segment, executable)
                        release_point = (state["cs"], state["ip"])
                        if (not stable_release_ready
                                and release_point
                                not in unsafe_scene_release_points):
                            unsafe_scene_release_points.add(release_point)
                            if len(unsafe_scene_release_points) <= 16:
                                stack_linear = state["ss"] * 16 + state["sp"]
                                stack = read_guest(
                                    mem, guest_base, stack_linear, 64)
                                print(
                                    "state: deferred stable scene release at "
                                    f"{state['cs']:04x}:{state['ip']:04x} "
                                    f"ax={state['ax']:04x} "
                                    f"bx={state['bx']:04x} "
                                    f"cx={state['cx']:04x} "
                                    f"dx={state['dx']:04x} "
                                    f"ss:sp={state['ss']:04x}:"
                                    f"{state['sp']:04x} stack={stack.hex()}",
                                    flush=True)
                            elif len(unsafe_scene_release_points) == 17:
                                print(
                                    "state: suppressing additional unsafe "
                                    "scene-release samples", flush=True)
                elif stable_scene_line_key == release_key:
                    stable_scene_line_key = None
                    stable_scene_line_started = None
                if (
                    ((not trigger_pterra_after_load and not manual)
                     or stable_release_ready)
                    and pterra_triggered
                    and hit is not None
                    and flow["c2_presentation_gate"] == 1
                    and blockers["presentation"] == 0
                    and blockers["presentation_defer"] == 0
                    and blockers["text"] == 0
                    and release_key not in completed_scene_line_states
                ):
                    game = game_segment * 16
                    write_guest(
                        mem, guest_base, game + VM_DISPLAYED_LINE_OFFSET,
                        struct.pack("<H", release_key[1]))
                    write_guest(
                        mem, guest_base, game + VM_ACTIVE_LINE_OFFSET,
                        b"\xff\xff")
                    write_guest(
                        mem, guest_base,
                        game + VM_C2_PRESENTATION_GATE_OFFSET,
                        b"\x00")
                    request_flags = read_guest(
                        mem, guest_base,
                        game + VM_PRESENTATION_REQUEST_FLAGS_OFFSET, 1)[0]
                    write_guest(
                        mem, guest_base,
                        game + VM_PRESENTATION_REQUEST_FLAGS_OFFSET,
                        bytes((request_flags & 0xFD,)))
                    if release_key == (9, 0x002B):
                        # Exact non-presentation-record phase-9 tail from
                        # scene_transition_step(). The synthetic record trigger
                        # lacks the live queue callback that normally executes it.
                        write_guest(
                            mem, guest_base,
                            game + RESOURCE_VERTICAL_OFFSET, b"\x00\x00")
                        write_guest(
                            mem, guest_base,
                            game + SCENE_TRANSITION_FLAGS_OFFSET, b"\x21")
                        write_guest(
                            mem, guest_base,
                            game + VM_ACTIVE_LINE_OFFSET, b"\x2a\x00")
                        write_guest(
                            mem, guest_base,
                            game + VM_SCENE_GATE_OFFSET, b"\x00")
                    elif release_key == (0x21, 0x002A):
                        # Exact externally relevant final-reset state. This lets
                        # the ordinary main loop resume after line 0x2a closes.
                        write_guest(
                            mem, guest_base,
                            game + SCENE_TRANSITION_FLAGS_OFFSET, b"\x00")
                        write_guest(
                            mem, guest_base,
                            game + VM_UI_STATE_OFFSET, b"\x01\x00")
                    completed_scene_line_states.add(release_key)
                    completed_scene_lines.append({
                        "render": release_key[0],
                        "active_line": release_key[1],
                        "after_release": read_profile_state(
                            mem, guest_base, game_segment,
                            fs_segment)["resource_flow"],
                    })
                    print(
                        "state: completed stable scene line "
                        f"{release_key[1]} at render phase {release_key[0]}",
                        flush=True)

                natural_key = None
                if (trigger_pterra_after_load
                        and pterra_triggered
                        and hit is not None
                        and flow["c2_presentation_gate"] == 1
                        and release_key in (
                            (3, 0x0029), (9, 0x002B), (0x21, 0x002A))):
                    natural_key = release_key
                if (natural_scene_active_key is not None
                        and natural_key != natural_scene_active_key):
                    if (natural_scene_active_key
                            not in completed_scene_line_states):
                        completed_scene_line_states.add(
                            natural_scene_active_key)
                        completed_scene_lines.append({
                            "render": natural_scene_active_key[0],
                            "active_line": natural_scene_active_key[1],
                            "after_release": last_profile["resource_flow"],
                        })
                        print(
                            "state: game completed scene line "
                            f"{natural_scene_active_key[1]} at render phase "
                            f"{natural_scene_active_key[0]}", flush=True)
                    natural_scene_active_key = None
                if (natural_key is not None
                        and natural_key not in completed_scene_line_states):
                    natural_scene_active_key = natural_key

                if (trigger_pterra_after_load
                        and not destination_committed
                        and (0x21, 0x002A) in completed_scene_line_states
                        and blockers["render"] == 0
                        and flow["active_line"] == 0xffff
                        and flow["c2_presentation_gate"] == 0):
                    if diagnostic_mute_pterra:
                        # The synthetic destination event omits the queue
                        # callback which loads SN\SCRUT.SND. Disable playback
                        # before proc pter can enter audio_process_ade(); doing
                        # this after the first dialogue frame is too late.
                        write_guest(
                            mem, guest_base, game + 0x0ADE, b"\x00")
                        write_guest(
                            mem, guest_base,
                            game + RESOURCE_FRAME_PRESENTED_OFFSET, b"\x01")
                        diagnostic_audio_muted = True
                        print(
                            "diagnostic: muted Pterra audio before proc pter "
                            "to bypass the synthetic route's missing "
                            "SN\\SCRUT.SND callback",
                            flush=True)
                    record_segment, record_offset = \
                        activate_script2_pterra_procedure(
                            mem, guest_base, game_segment)
                    destination_committed = True
                    print(
                        "state: activated SCRIPT2 proc pter predicates after "
                        "the game's final transition reset through "
                        f"{record_segment:04x}:{record_offset:04x}",
                        flush=True)

                if (not pter_reached
                        and destination_committed
                        and blockers["presentation"] != 0
                        and blockers["ship"] != 0
                        and flow["active_line"] != 0xffff):
                    pter_reached = True
                    pter_reached_at = time.monotonic()
                    pter_last_progress_at = pter_reached_at
                    pter_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    print(
                        "state: entered SCRIPT2 proc pter; driving the full "
                        "dialogue and choice sequence",
                        flush=True)

                if pter_reached:
                    input_state = last_profile["input"]
                    audio_flow = last_profile["audio_flow"]
                    assert isinstance(input_state, dict)
                    assert isinstance(audio_flow, dict)
                    semantic_key = (
                        flow["active_line"],
                        flow["c2_presentation_gate"],
                        blockers["presentation"],
                        blockers["presentation_defer"],
                        blockers["text"],
                        input_state["word_choice_active"],
                        input_state["word_choice_phase"],
                        input_state["selected_word"],
                        flow["text_menu_end"],
                        flow["dialogue_hold_complete"],
                        flow["presentation_hold_ready"],
                        audio_flow["dialogue_hold_countdown"],
                    )
                    now = time.monotonic()
                    if semantic_key != pter_last_semantic_key:
                        pter_last_semantic_key = semantic_key
                        pter_last_progress_at = now

                    word_choice_active = bool(
                        input_state["word_choice_active"] & 1)
                    if diagnostic_mute_pterra and word_choice_active:
                        # The omitted renderer callback also republishes this
                        # bit while its choice frame is visible.
                        write_guest(
                            mem, guest_base,
                            game + RESOURCE_FRAME_PRESENTED_OFFSET, b"\x01")
                    if word_choice_active:
                        pter_choice_was_active = True
                    elif pter_choice_was_active:
                        selected_word = int(input_state["selected_word"])
                        pter_choice_results.append(selected_word)
                        pter_choice_was_active = False
                        print(
                            "state: completed Pterra choice "
                            f"{len(pter_choice_results)} with dictionary "
                            f"word {selected_word:#06x}", flush=True)

                    if pter_input_pressed is not None:
                        pressed_offset = (
                            MOUSE_PRIMARY_PRESSED_OFFSET
                            if pter_input_pressed == "primary"
                            else MOUSE_SECONDARY_PRESSED_OFFSET)
                        write_guest(
                            mem, guest_base, game + pressed_offset, b"\x00")
                        pter_input_pressed = None
                    elif now >= pter_next_input_at:
                        if diagnostic_mute_pterra and word_choice_active:
                            selected_word = (
                                SCRIPT2_EXXOS_WORD
                                if not pter_choice_results
                                else SCRIPT2_TELEPORT_WORD)
                            request_flags = read_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_REQUEST_FLAGS_OFFSET,
                                1)[0]
                            # Publish the exact terminal state of
                            # presentation_ready_gate(). The synthetic route
                            # has no renderer callback to open this widget.
                            write_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_SELECTED_WORD_OFFSET,
                                struct.pack("<H", selected_word))
                            write_guest(
                                mem, guest_base,
                                game + VM_BLOCK_MATCH_VALUE_OFFSET,
                                struct.pack("<H", selected_word))
                            write_guest(
                                mem, guest_base,
                                game + VM_WORD_CHOICE_ACTIVE_OFFSET, b"\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_DEFER_OFFSET, b"\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_TEXT_DISPLAY_ACTIVE_OFFSET, b"\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_DIALOGUE_HOLD_COMPLETE_OFFSET,
                                b"\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_TEXT_WAIT_OFFSET,
                                b"\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_WORD_BUFFER_OFFSET,
                                b"\x00\x00")
                            write_guest(
                                mem, guest_base,
                                game + VM_PRESENTATION_REQUEST_FLAGS_OFFSET,
                                bytes((request_flags & 0xFE,)))
                            pter_next_input_at = now + 0.5
                            print(
                                "diagnostic: published completed Pterra word "
                                f"choice {selected_word:#06x}", flush=True)
                        elif (word_choice_active
                                and (input_state["word_choice_phase"] & 7)
                                == 2):
                            target_rows = (4, 0)
                            target_row = target_rows[min(
                                len(pter_choice_results),
                                len(target_rows) - 1)]
                            point = choice_row_point(
                                tuple(input_state["choice_rect"]),
                                target_row)
                            write_guest(
                                mem, guest_base, game + MOUSE_X_OFFSET,
                                struct.pack("<h", point[0]))
                            write_guest(
                                mem, guest_base, game + MOUSE_Y_OFFSET,
                                struct.pack("<h", point[1]))
                            write_guest(
                                mem, guest_base,
                                game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                            write_guest(
                                mem, guest_base,
                                game + MOUSE_PRESS_PENDING_OFFSET, b"\x01")
                            pter_input_pressed = "primary"
                            pter_next_input_at = now + 0.25
                            print(
                                "state: selected Pterra choice row "
                                f"{target_row + 1} at {point[0]},{point[1]}",
                                flush=True)
                        elif (not word_choice_active
                              and blockers["presentation"] != 0):
                            write_guest(
                                mem, guest_base,
                                game + MOUSE_SECONDARY_PRESSED_OFFSET,
                                b"\x01")
                            write_guest(
                                mem, guest_base,
                                game + MOUSE_PRESS_PENDING_OFFSET, b"\x01")
                            pter_input_pressed = "secondary"
                            pter_next_input_at = now + 0.5

                    if (pter_completed_at is None
                            and len(pter_choice_results) >= 2
                            and blockers["presentation"] == 0):
                        pter_completed_at = now
                        post_pter_cpu_samples.clear()
                        print(
                            "state: completed SCRIPT2 proc pter; beginning "
                            f"{post_pter_seconds:g}s liveness gate",
                            flush=True)
                    if pter_completed_at is not None:
                        post_pter_cpu_samples.append(state.copy())
                    elif (pter_last_progress_at is not None
                          and now - pter_last_progress_at >= 15.0):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "Pterra encounter made no semantic progress for "
                            "15 seconds")
                        print(
                            "hang: Pterra encounter made no semantic progress "
                            "for 15 seconds", flush=True)
                        break

                resource_flow = last_profile["resource_flow"]
                scene_43 = resource_flow["scene_entries"]["43"]
                invalid_resource_handle = (
                    flow["active_line"] == 0x002b
                    and resource_flow["file_handle"]
                    == scene_43["descriptor_offset"] + 2
                    and resource_flow["iteration_count"] != 0)
                if invalid_resource_handle:
                    if invalid_resource_handle_stall_started is None:
                        invalid_resource_handle_stall_started = time.monotonic()
                    elif (time.monotonic()
                            - invalid_resource_handle_stall_started >= 1.0):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "resource read loop used descriptor filename "
                            "offset as DOS handle")
                        print(
                            "hang: resource line 43 read with descriptor "
                            f"offset {resource_flow['file_handle']:#06x} as "
                            "its DOS handle", flush=True)
                        break
                else:
                    invalid_resource_handle_stall_started = None

                if hit is not None and marker_snapshot is None:
                    marker_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    print(f"boundary marker: {hit.name}", flush=True)
                final_transition_stalled = (
                    trigger_pterra_after_load
                    and hit is not None
                    and not destination_committed
                    and release_key == (0x21, 0x002A)
                    and flow["c2_presentation_gate"] == 0
                    and blockers["presentation"] == 0
                    and blockers["presentation_defer"] == 0
                    and blockers["text"] == 0
                )
                if final_transition_stalled:
                    if final_transition_stall_started is None:
                        final_transition_stall_started = time.monotonic()
                        final_transition_cpu_samples.clear()
                    final_transition_cpu_samples.append(state.copy())
                    if (time.monotonic() - final_transition_stall_started
                            >= 5.0):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["cpu_samples"] = \
                            final_transition_cpu_samples[-64:]
                        print(
                            "hang: Pterra final transition did not publish "
                            "its completion gate", flush=True)
                        break
                else:
                    final_transition_stall_started = None
                    final_transition_cpu_samples.clear()
                if illegal_interrupt is not None:
                    fault_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    fault_snapshot["interrupt"] = int(
                        illegal_interrupt.group(1), 10)
                    fault_snapshot["message"] = illegal_interrupt.group(0).decode(
                        "ascii")
                    print(
                        "fault: illegal interrupt "
                        f"{fault_snapshot['interrupt']} at "
                        f"{fault_snapshot['cpu']['cs']:04x}:"
                        f"{fault_snapshot['cpu']['ip']:04x}", flush=True)
                    break
                if (dos_read_warning is not None
                        and last_profile["profile"] == SCRIPT2_PROFILE
                        and int(dos_read_warning.group(1), 16)
                        + int(dos_read_warning.group(2), 16) > 0x10000):
                    dos_read_overflow_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    dos_read_overflow_snapshot["dos_read"] = {
                        "offset": int(dos_read_warning.group(1), 16),
                        "byte_count": int(dos_read_warning.group(2), 16),
                        "message": dos_read_warning.group(0).decode("ascii"),
                    }
                    print(
                        "fault: overflowing DOS read at "
                        f"{dos_read_overflow_snapshot['cpu']['cs']:04x}:"
                        f"{dos_read_overflow_snapshot['cpu']['ip']:04x} "
                        f"dx={dos_read_overflow_snapshot['dos_read']['offset']:04x} "
                        f"cx={dos_read_overflow_snapshot['dos_read']['byte_count']:04x}",
                        flush=True)
                    break
                if (pter_completed_at is not None
                        and time.monotonic() - pter_completed_at
                        >= post_pter_seconds):
                    progress = {
                        (sample["cs"], sample["ip"], sample["sp"])
                        for sample in post_pter_cpu_samples
                    }
                    if len(progress) < 2:
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "Pterra post-encounter CPU state did not advance")
                        hang_snapshot["cpu_samples"] = post_pter_cpu_samples
                        print(
                            "hang: Pterra post-encounter CPU state did not "
                            "advance",
                            flush=True)
                    else:
                        pter_sustained = True
                        post_pter_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        post_pter_snapshot["duration_seconds"] = \
                            post_pter_seconds
                        post_pter_snapshot["distinct_cpu_states"] = len(progress)
                        print(
                            "success: completed SCRIPT2 proc pter and sustained "
                            "runtime for "
                            f"{post_pter_seconds:g}s across "
                            f"{len(progress)} CPU states",
                            flush=True)
                    break
        finally:
            if attached:
                libc.ptrace(PTRACE_DETACH, db.pid, None, None)

        if (fault_snapshot is not None
                or dos_read_overflow_snapshot is not None
                or integrity_fault_snapshot is not None
                or hang_snapshot is not None
                or pter_sustained):
            break
        time.sleep(0.01)

    errors = []
    if not manual and not request_written:
        errors.append("SCRIPT2 request boundary was never reached")
    if not manual and not profile_loaded:
        errors.append("SCRIPT2 did not finish loading")
    if open_load_menu and not load_menu_requested:
        errors.append("LOAD-menu request boundary was never reached")
    if trigger_pterra_after_load and not authentic_save_loaded:
        errors.append("authentic GAME1.SAV load did not complete")
    if trigger_pterra_after_load and not post_load_presentation_seen:
        errors.append("post-load presentation boundary was never observed")
    if trigger_pterra_after_load and not pterra_triggered:
        errors.append("saved-game Pterra destination event was never safe")
    if (trigger_pterra_after_load and not destination_committed
            and integrity_fault_snapshot is None
            and hang_snapshot is None):
        errors.append("saved-game Pterra location was never committed")
    if not manual and not pterra_triggered:
        errors.append("SCRIPT2 loaded, but the Pterra trigger was never safe")
    if (marker_snapshot is None
            and integrity_fault_snapshot is None
            and dos_read_overflow_snapshot is None):
        errors.append("Pterra trigger ran, but no Pterra image was created")
    if (pter_reached and not pter_sustained
            and fault_snapshot is None and hang_snapshot is None):
        errors.append("Pterra encounter completion gate did not complete")
    return {
        "mode": (
            "authentic-save-pterra" if trigger_pterra_after_load
            else "manual-pterra" if manual
            else "state-pterra"),
        "log": str(log_path),
        "marker": marker_snapshot,
        "fault": fault_snapshot,
        "fault_detected": fault_snapshot is not None,
        "dos_read_overflow": dos_read_overflow_snapshot,
        "dos_read_overflow_detected": dos_read_overflow_snapshot is not None,
        "integrity_fault": integrity_fault_snapshot,
        "integrity_fault_detected": integrity_fault_snapshot is not None,
        "hang": hang_snapshot,
        "hang_detected": hang_snapshot is not None,
        "destination_committed": destination_committed,
        "pter": pter_snapshot,
        "pter_reached": pter_reached,
        "pter_completed": pter_completed_at is not None,
        "pter_choice_results": pter_choice_results,
        "pter_sustained": pter_sustained,
        "diagnostic_audio_muted": diagnostic_audio_muted,
        "post_pter": post_pter_snapshot,
        "completed_scene_lines": completed_scene_lines,
        "last_profile": last_profile,
        "last_cpu": last_cpu_state,
        "errors": errors,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cd-dir", type=Path, required=True)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--install-parent", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--display", default=":83")
    parser.add_argument("--timeout", type=float, default=900.0,
                        help="seconds to wait for the boundary")
    parser.add_argument(
        "--post-pter-seconds", type=float, default=5.0,
        help=("fault-free runtime required after completing proc pter "
              "(default: 5)"))
    parser.add_argument(
        "--diagnostic-mute-pterra", action="store_true",
        help=("mute audio after proc pter starts; diagnostic only, because "
              "the synthetic transition omits the SN\\SCRUT.SND callback"))
    parser.add_argument("--drive", action="store_true",
                        help="script the navigation instead of a human")
    parser.add_argument("--display-for-drive", default=None,
                        help="X display holding the game when driving")
    parser.add_argument(
        "--state-pterra", action="store_true",
        help="load SCRIPT2 and select its Pterra record through recovered state")
    parser.add_argument(
        "--manual-pterra", action="store_true",
        help="monitor a manually played Pterra route through the first fault")
    parser.add_argument(
        "--open-load-menu", action="store_true",
        help="with --manual-pterra, open the original LOAD menu at title idle")
    parser.add_argument(
        "--trigger-pterra-after-load", action="store_true",
        help=("after the authentic save and its presentation complete, emit "
              "the destination selector's SCRIPT2 Pterra record event"))
    parser.add_argument(
        "--drive-authentic-save", action="store_true",
        help=("select save slot 1 and dismiss its presentation through the "
              "game's input-state contract"))
    parser.add_argument(
        "--guest-snapshot", type=Path,
        help=("write the 64 KiB game-data segment at the first matched "
              "post-load presentation frame"))
    parser.add_argument(
        "--dosbox-log", type=Path,
        help="DOSBox log used to stop at the first invalid instruction")
    args = parser.parse_args()
    if args.state_pterra and args.manual_pterra:
        parser.error("--state-pterra and --manual-pterra are mutually exclusive")
    if args.open_load_menu and not args.manual_pterra:
        parser.error("--open-load-menu requires --manual-pterra")
    if args.trigger_pterra_after_load \
            and not (args.manual_pterra and args.open_load_menu):
        parser.error(
            "--trigger-pterra-after-load requires --manual-pterra "
            "and --open-load-menu")
    if args.drive_authentic_save and not args.trigger_pterra_after_load:
        parser.error(
            "--drive-authentic-save requires --trigger-pterra-after-load")
    if args.post_pter_seconds <= 0:
        parser.error("--post-pter-seconds must be positive")

    env = dict(os.environ, DISPLAY=args.display, SDL_VIDEODRIVER="x11")
    xvfb = None
    db = None
    log_stream = None
    if not args.display.startswith(":0"):
        xvfb = subprocess.Popen(
            ["Xvfb", args.display, "-screen", "0", "800x600x24"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(2.0)
    libc = libc_ptrace()
    snapshot = None
    try:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        log_path = args.dosbox_log or args.output.with_suffix(".dosbox.log")
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_stream = log_path.open("wb", buffering=0)
        marker = args.install_parent / "cblood"
        if (args.trigger_pterra_after_load
                and not (marker / "GAME1.SAV").is_file()):
            raise RuntimeError(
                "authentic-save capture requires "
                f"{marker / 'GAME1.SAV'}")
        if args.state_pterra or args.manual_pterra:
            stale = list(marker.glob("PTERRA1[DFG].LBM"))
            if stale:
                names = ", ".join(path.name for path in stale)
                raise RuntimeError(
                    f"Pterra capture requires a clean drive; remove {names}")
        dosbox_args = [
            "dosbox-x", "--noprimaryconf", "--nolocalconf",
            "-set", "sdl output=surface",
            "-set", "sdl autolock=true",
            "-set", "cpu cycles=max",
            "-set", "cpu core=dynamic",
            "-set", "render frameskip=10",
            "-c", f"mount c {args.install_parent}",
            "-c", f"mount d {args.cd_dir} -t cdrom",
            "-c", "d:",
            "-c", f"{args.executable} AMR S162227 EMS WRIC:\\cblood\\",
        ]
        db = subprocess.Popen(dosbox_args, env=env,
                              stdout=log_stream,
                              stderr=subprocess.STDOUT)
        if args.state_pterra or args.manual_pterra:
            snapshot = capture_state_pterra(
                db, libc, marker, log_path, args.timeout,
                args.display, args.executable, manual=args.manual_pterra,
                open_load_menu=args.open_load_menu,
                trigger_pterra_after_load=args.trigger_pterra_after_load,
                drive_authentic_save=args.drive_authentic_save,
                guest_snapshot=args.guest_snapshot,
                post_pter_seconds=args.post_pter_seconds,
                diagnostic_mute_pterra=args.diagnostic_mute_pterra)
            args.output.write_text(json.dumps(snapshot, indent=1))
            print(f"wrote {args.output}")
            errors = snapshot.get("errors", [])
            if errors:
                raise RuntimeError("; ".join(str(error) for error in errors))
            return
        if args.drive:
            import threading
            drive_display = args.display_for_drive or args.display
            drive_actions = bridge_prefix_actions()
            for lap in range(6):
                drive_actions += [f"shot g_lap{lap}"] + rotation_lap(lap)

            def drive() -> None:
                time.sleep(3.0)  # let the window appear
                run_driver(drive_actions, drive_display, args.executable)

            threading.Thread(target=drive, daemon=True).start()
        deadline = time.time() + args.timeout
        hit = None
        while time.time() < deadline:
            if db.poll() is not None:
                print(f"dosbox exited early with {db.returncode}")
                break
            try:
                hit = next(marker.glob("PTERRA1[DFG].LBM"), None)
            except StopIteration:
                hit = None
            if hit is not None:
                break
            time.sleep(0.02)
        if hit is None:
            print("boundary never reached (no PTERRA file created)")
            return
        print(f"boundary marker: {hit.name}")
        time.sleep(0.05)  # let the creating instruction fully retire
        if libc.ptrace(PTRACE_ATTACH, db.pid, None, None) != 0:
            print("ptrace attach failed", ctypes.get_errno())
            return
        os.waitpid(db.pid, 0)
        with open(f"/proc/{db.pid}/mem", "rb") as mem:
            cpu_addresses = locate_cpu_state(db.pid)
            best, guest_base = find_ds_anchor(db.pid, mem)
            if not best:
                print("DS anchor not found")
                return
            snapshot = {}
            state = read_cpu_state(mem, cpu_addresses)
            if state:
                snapshot["cpu"] = state
                delta_segments = {
                    key: value - (best - guest_base) // 16
                    for key, value in state.items()
                    if key in ("ds", "es", "ss", "fs", "gs", "cs")
                }
                snapshot["segments_minus_ds_anchor"] = delta_segments
            mem.seek(guest_base)
            snapshot["ivt"] = mem.read(0x400).hex()
            band = {}
            for offset in range(0x0A40, 0x0B00, 2):
                mem.seek(best + offset)
                band[f"{offset:#06x}"] = struct.unpack(
                    "<H", mem.read(2))[0]
            snapshot["resource_band"] = band
            mem.seek(best + 0x5219)
            snapshot["back_buffer_area"] = mem.read(0x5240 - 0x5219).hex()
            snapshot["marker"] = str(hit)
        args.output.write_text(json.dumps(snapshot, indent=1))
        print(f"wrote {args.output}")
    finally:
        if db is not None:
            # keep or kill? kill: the relinked guest would storm anyway.
            try:
                libc.ptrace(PTRACE_DETACH, db.pid, None, None)
            except Exception:
                pass
            try:
                os.kill(db.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        if log_stream is not None:
            log_stream.close()
        if xvfb is not None:
            xvfb.terminate()


if __name__ == "__main__":
    main()
