#!/usr/bin/env python3
"""Drive the game to Pterra and capture matched original/relinked state.

Launches dosbox-x exactly like BLOOD.BAT does. The authentic-save route opens
the game's own load menu, waits for GAME1.SAV and any resulting presentation to
finish, enables Pterra through SCRIPT2's recovered ``init`` predicate, and
selects Pterra through the native ship HUD. The HUD must publish an Orxx C1
travel command and the VM must consume it. From that boundary onward the
capture stops at the first interrupt-vector mutation, invalid instruction,
transition hang, or successful completion of SCRIPT2 ``proc pter``. The driver
advances each dialogue hold, selects ``exxos`` and ``teleport`` through the
game's list widget, then requires fault-free runtime after the presentation
closes. This avoids treating a synthetic destination event or the first
Pterra dialogue frame as a pass.

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
import hashlib
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
LIST_D8C_AUDIO_PHASE_OFFSET = 0x0C41
SND_CLIP_PLAYBACK_STATE_OFFSET = 0x0B39
SND_STREAM_CHANNEL_ACTIVE_OFFSET = 0x0BA3
SND_AUDIO_POSITION_CALLBACK_OFFSET = 0x0CF3
PRESENTATION_MODE_FLAG_27E0_OFFSET = 0x27E0
PRESENTATION_MODE_FLAG_27E1_OFFSET = 0x27E1
VM_RESOURCE_PROFILES_OFFSET = 0x11F4
VM_RESOURCE_COUNT = 5
SCRIPT2_PROFILE = 1
SCRIPT2_BLOOD_RECORD = 0x0028
SCRIPT2_SCRUTER_JO_RECORD = 0x070A
SCRIPT2_SCRUTER_JO_ACTION_OFFSET = 0x0744
SCRIPT2_PTERRA_RECORD = 0x0DA0
SCRIPT2_PTERRA_FLAGS_OFFSET = SCRIPT2_PTERRA_RECORD + 2
SCRIPT2_ARCHETYPE_CURRENT_LOCATION_OFFSET = 0x0F4E
SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET = 0x12C2
SCRIPT2_INIT_PROCEDURE_OFFSET = 0x3022
SCRIPT2_INIT_PROCEDURE_ENABLED = 0x01
SCRIPT2_IN_PLAY_FLAG = 0x0002
BLOODPRG_VM_RECORD_C4 = 0x00C4
BLOODPRG_VM_RECORD_C1 = 0x00C1
VM_SHIP_ACTIVE_FLAGS_OFFSET = 0x24F3
SHIP_3D_DIALOGUE_CYCLE_LINE_OFFSET = 0x24F5
SHIP_3D_PRESENTABLE_NAME_OFFSETS_OFFSET = 0x250B
# DS:250B ends immediately before the DS:251B current-target word.
SHIP_3D_PRESENTABLE_NAME_OFFSET_COUNT = 8
SHIP_3D_CURRENT_TARGET_OFFSET = 0x251B
SHIP_3D_HUD_INITIALIZED_OFFSET = 0x2529
SHIP_3D_TARGET_SELECT_PHASE_OFFSET = 0x252B
SHIP_3D_SCENE_DISPATCH_BLOCKED_OFFSET = 0x252D
SHIP_3D_DIALOGUE_PHASE_READY_OFFSET = 0x2534
SHIP_3D_HUD_INIT_PENDING_OFFSET = 0x2535
VM_SEQUENCE_ACTIVE_OFFSET = 0x252A
VM_RECORD_BASE_POINTER_OFFSET = 0x6724
VM_NAMED_ORXX_OBJECT_OFFSET = 0x6750
VM_NAMED_ARCHETYPE_OBJECT_OFFSET = 0x6752
VM_NAMED_ARK_OBJECT_OFFSET = 0x6758
NAV_DEFERRED_RECORD_LINK_OFFSET = 0x676A
SCENE_TRANSITION_FLAGS_OFFSET = 0x2751
VM_UI_FLAGS_OFFSET = 0x2793
VM_UI_STATE_OFFSET = 0x2792
BRIDGE_PANORAMA_FRAME_OFFSET = 0x2795
BRIDGE_SEEK_TARGET_ARC_OFFSET = 0x279B
BRIDGE_SEEK_INITIAL_DISTANCE_OFFSET = 0x279D
BRIDGE_TURN_DIRECTION_OFFSET = 0x27DB
NAV_CAMERA_VIEW_ACTIVE_OFFSET = 0x278A
NAV_CAMERA_VIEW_STATE_OFFSET = 0x278B
NAV_LOCATION_PANEL_TRANSITION_STATE_OFFSET = 0x2788
NAV_LOCATION_PANEL_SCALE_STEP_OFFSET = 0x2789
NAV_LOCATION_PANEL_ACTIVE_OFFSET = 0x278C
NAV_CENTER_WIPE_COMPLETE_OFFSET = 0x2791
NAV_SELECTED_LOCATION_RECORD_OFFSET = 0x27BF
NAV_CHART_OBJECT_COUNT_OFFSET = 0x27C1
NAV_CHART_OBJECT_OFFSETS_OFFSET = 0x2AD3
NAV_CHART_MAX_OBJECTS = 64
LOAD_REQUEST_ACTIVE_OFFSET = 0x2737
SAVE_SLOT_MENU_PHASE_OFFSET = 0x2738
MOUSE_X_OFFSET = 0x0A2A
MOUSE_Y_OFFSET = 0x0A2C
MOUSE_BUTTON_STATE_OFFSET = 0x0A2E
MOUSE_PREVIOUS_BUTTON_STATE_OFFSET = 0x0A30
MOUSE_LAST_X_OFFSET = 0x0A38
MOUSE_LAST_Y_OFFSET = 0x0A3A
MOUSE_PRIMARY_PRESSED_OFFSET = 0x0A3E
MOUSE_SECONDARY_PRESSED_OFFSET = 0x0A3F
MOUSE_PRESS_PENDING_OFFSET = 0x0A40
GRAPHICS_WORK_SURFACE_OFFSET = 0x0ABC
GRAPHICS_DRAW_FRAMEBUFFER_OFFSET = 0x5219
GRAPHICS_SCREEN_BUFFER_OFFSET = 0x521D
GRAPHICS_DISPLAY_BUFFER_OFFSET = 0x5221
GRAPHICS_BACK_BUFFER_OFFSET = 0x5229
GRAPHICS_SURFACE_BYTES = 320 * 200
GRAPHICS_POINTER_OFFSETS = (
    ("work_surface", GRAPHICS_WORK_SURFACE_OFFSET),
    ("draw_framebuffer", GRAPHICS_DRAW_FRAMEBUFFER_OFFSET),
    ("screen_buffer", GRAPHICS_SCREEN_BUFFER_OFFSET),
    ("display_buffer", GRAPHICS_DISPLAY_BUFFER_OFFSET),
    ("back_buffer", GRAPHICS_BACK_BUFFER_OFFSET),
)
VGA_GRAPHICS_POINTERS = frozenset((
    (0x0000, 0xA000),
    (0x4000, 0xA000),
    (0x8000, 0xA000),
    (0xC000, 0xA000),
))
PRESENTATION_CHOICE_RESULT_OFFSET = 0x0ACA
PRESENTATION_CHOICE_ACTIVE_OFFSET = 0x259B
PRESENTATION_CHOICE_PHASE_OFFSET = 0x259C
VM_WORD_CHOICE_ACTIVE_OFFSET = 0x27D7
VM_OPERAND_WORD_COUNT_OFFSET = 0x27CF
VM_TEXT_MENU_END_OFFSET = 0x27D3
VM_TEXT_REVEAL_CURSOR_OFFSET = 0x5E58
VM_TEXT_DISPLAY_ACTIVE_OFFSET = 0x5E64
VM_DISPLAYED_LINE_OFFSET = 0x678A
VM_PRESENTATION_OWNER_OFFSET = 0x679A
CHOICE_RECT_OFFSET = 0x2AAB
BRIDGE_STATIONS_OFFSET = 0x2A1B
BRIDGE_STATION_SIZE = 0x18
BRIDGE_STATION_COUNT = 6
ENTITY_TABLE_OFFSET = 0x6212
ENTITY_RECORD_SIZE = 0x20
CURRENT_LOCATION_ENTITY_INDEX = 31
VM_SCENE_GATE_OFFSET = 0x274F
RESOURCE_VERTICAL_OFFSET = 0x1FA7
RESOURCE_INDEX_OFFSET = 0x1FB5
RESOURCE_FRAME_PRESENTED_OFFSET = 0x0DB8
VM_PRESENTATION_SELECTED_WORD_OFFSET = 0x6796
VM_PRESENTATION_TEXT_WAIT_OFFSET = 0x67BA
VM_DIALOGUE_HOLD_COMPLETE_OFFSET = 0x67BB
VM_DIALOGUE_HOLD_COUNTDOWN_OFFSET = 0x0B35
VM_PRESENTATION_HOLD_READY_OFFSET = 0x67BC
VM_BLOCK_MATCH_VALUE_OFFSET = 0x6762
VM_PRESENTATION_DEFER_OFFSET = 0x67B0
VM_TEXT_DISPLAY_ACTIVE_OFFSET = 0x5E64
VM_PRESENTATION_WORD_BUFFER_OFFSET = 0x67F8
SCRIPT2_EXXOS_WORD = 0x0171
SCRIPT2_TELEPORT_WORD = 0x02A8
EXPECTED_PTERRA_CHOICES = (SCRIPT2_EXXOS_WORD, SCRIPT2_TELEPORT_WORD)
PTERRA_TRAVEL_MOVIE_TIMEOUT_SECONDS = 120.0
PTERRA_MAP_TRANSITION_TIMEOUT_SECONDS = 120.0
PTERRA_BRIDGE_ROTATION_TIMEOUT_SECONDS = 360.0
PTERRA_NAV_ACTIVATION_TIMEOUT_SECONDS = 30.0
PTERRA_NAV_CHART_MAX_REOPEN_ATTEMPTS = 3
PTERRA_NATIVE_INPUT_TIMEOUT_SECONDS = 10.0
PTERRA_NATIVE_INPUT_EDGE_INTERVAL_SECONDS = 0.25
PTERRA_NATIVE_INPUT_PULSE_SECONDS = 0.2
PTERRA_NATIVE_INPUT_TRIGGER_COUNTDOWN = 7
PTERRA_NATIVE_INPUT_MAX_EDGES = 6
PTERRA_TRANSITION_SAMPLE_INTERVAL_SECONDS = 1.0
PTERRA_TRANSITION_SAMPLE_LIMIT = 256
STARTUP_DOS_POOL_POINTER_OFFSET = 0x0A42
MANU3_ORIGINAL_PREFIX = bytes.fromhex("1e2e8b0e")
MANU3_ORIGINAL_DATA_SEGMENT_DELTA_OFFSET = 0x1368
MANU3_RECOVERED_PREFIX = bytes.fromhex("1e8cc82e")
MANU3_RECOVERED_DATA_SEGMENT_DELTA_OFFSET = 0x001B
MANU3_SEGMENT_DIRECTORY_SIZE = 0x0014
MANU3_CURRENT_STATE_OFFSET = 0x2248
MANU3_PROJECTION_REMAINING_OFFSET = 0x224A
MANU3_FACE_LIST_OFFSET = 0x2300
MANU3_FACE_COUNT_OFFSET = 0x2304
MANU3_ACTIVE_LIST_HEAD_OFFSET = 0x0964
MANU3_ACTIVE_LIST_MIDDLE_OFFSET = 0x09BE
MANU3_ACTIVE_LIST_TAIL_OFFSET = 0x0A18
MANU3_RASTER_POOL_OFFSET = 0x0A72
MANU3_RASTER_POOL_RECORD_SIZE = 0x005A
MANU3_RASTER_POOL_RECORD_COUNT = 0x00C8
MANU3_RASTER_STATE_OFFSETS = (
    0x067E, 0x0680, 0x0682, 0x0684, 0x0908, 0x0962, 0x0A28,
)
MANU3_RASTER_RECORD_OFFSETS = (0x0964, 0x09BE, 0x0A18)
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
    symbol_sizes = {}
    output = subprocess.check_output(
        ["nm", "-P", executable], text=True, stderr=subprocess.DEVNULL)
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
            if len(fields) >= 4:
                symbol_sizes[fields[0]] = int(fields[3], 16)
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
    addresses = {
        name: image_base + offset for name, offset in symbols.items()
    }
    addresses["Segs_size"] = symbol_sizes.get("Segs", 0)
    return addresses


def read_cpu_state(mem, addresses):
    if addresses is None:
        return None
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    if addresses.get("Segs_size") == 0x30:
        mem.seek(addresses["Segs"])
        segments = list(struct.unpack("<6H", mem.read(12)))
    else:
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


def read_graphics_pointer_state(mem, guest_base: int,
                                game_segment: int) -> dict[str, object]:
    game = game_segment * 16
    pointers: dict[str, object] = {}
    for name, pointer_offset in GRAPHICS_POINTER_OFFSETS:
        offset, segment = struct.unpack(
            "<HH", read_guest(
                mem, guest_base, game + pointer_offset, 4))
        pointers[name] = {
            "offset": offset,
            "segment": segment,
            "pointer": f"{segment:04x}:{offset:04x}",
            "linear": segment * 16 + offset,
        }
    return pointers


def graphics_pointer_errors(pointers: dict[str, object],
                            baseline: dict[str, object],
                            game_segment: int) -> list[str]:
    allowed = set(VGA_GRAPHICS_POINTERS)
    for entry in baseline.values():
        assert isinstance(entry, dict)
        allowed.add((int(entry["offset"]), int(entry["segment"])))

    errors = []
    for name, entry in pointers.items():
        assert isinstance(entry, dict)
        offset = int(entry["offset"])
        segment = int(entry["segment"])
        pointer = str(entry["pointer"])
        linear = int(entry["linear"])
        if offset == 0 and segment == 0:
            errors.append(f"{name} became null")
        elif segment == game_segment:
            errors.append(f"{name} points into DGROUP at {pointer}")
        elif linear + GRAPHICS_SURFACE_BYTES > 0x100000:
            errors.append(f"{name} exceeds real-mode memory at {pointer}")
        elif (offset, segment) not in allowed:
            errors.append(f"{name} selected unknown surface {pointer}")
    return errors


def read_manu3_runtime_state(mem, guest_base: int, game_segment: int,
                             cpu_state: dict[str, int]) \
        -> dict[str, object]:
    """Read the loaded MANU3 overlay's compact renderer state."""
    game = game_segment * 16
    image_offset, code_segment = struct.unpack(
        "<HH", read_guest(
            mem, guest_base,
            game + STARTUP_DOS_POOL_POINTER_OFFSET, 4))
    result: dict[str, object] = {
        "image_pointer": f"{code_segment:04x}:{image_offset:04x}",
        "code_segment": code_segment,
        "cpu_in_manu3": int(cpu_state["cs"]) == code_segment,
        "local_ip": (
            int(cpu_state["ip"])
            if int(cpu_state["cs"]) == code_segment else None),
    }
    if code_segment < 0x0050:
        result["loaded"] = False
        return result

    code = code_segment * 16 + image_offset
    code_prefix = read_guest(mem, guest_base, code, 4)
    if code_prefix == MANU3_ORIGINAL_PREFIX:
        image_layout = "original"
        data_segment_delta_offset = \
            MANU3_ORIGINAL_DATA_SEGMENT_DELTA_OFFSET
    elif code_prefix == MANU3_RECOVERED_PREFIX:
        image_layout = "recovered"
        data_segment_delta_offset = \
            MANU3_RECOVERED_DATA_SEGMENT_DELTA_OFFSET
    else:
        result.update({
            "loaded": False,
            "image_layout": "unknown",
            "code_prefix": code_prefix.hex(),
        })
        return result

    data_segment_delta = struct.unpack(
        "<H", read_guest(
            mem, guest_base,
            code + data_segment_delta_offset, 2))[0]
    data_segment = (code_segment + data_segment_delta) & 0xffff
    result.update({
        "loaded": True,
        "image_layout": image_layout,
        "data_segment_delta_offset": data_segment_delta_offset,
        "data_segment_delta": data_segment_delta,
        "data_segment": data_segment,
    })
    data = data_segment * 16
    segment_directory = struct.unpack(
        "<10H", read_guest(
            mem, guest_base, data, MANU3_SEGMENT_DIRECTORY_SIZE))
    raster_segment = segment_directory[3]

    def data_word(offset: int) -> int:
        return struct.unpack(
            "<H", read_guest(mem, guest_base, data + offset, 2))[0]

    result["renderer"] = {
        "segment_directory": list(segment_directory),
        "current_state": data_word(MANU3_CURRENT_STATE_OFFSET),
        "projection_remaining": data_word(
            MANU3_PROJECTION_REMAINING_OFFSET),
        "face_list": data_word(MANU3_FACE_LIST_OFFSET),
        "face_count": data_word(MANU3_FACE_COUNT_OFFSET),
    }
    if raster_segment < 0x0050:
        result["raster"] = {
            "segment": raster_segment,
            "loaded": False,
        }
        return result

    raster = raster_segment * 16
    raster_words = {
        f"{offset:04x}": struct.unpack(
            "<H", read_guest(mem, guest_base, raster + offset, 2))[0]
        for offset in MANU3_RASTER_STATE_OFFSETS
    }
    result["raster"] = {
        "segment": raster_segment,
        "loaded": True,
        "words": raster_words,
        "records": {
            f"{offset:04x}": read_guest(
                mem, guest_base, raster + offset, 16).hex()
            for offset in MANU3_RASTER_RECORD_OFFSETS
        },
        "boundary_chain": read_manu3_boundary_chain(
            mem, guest_base, raster_segment),
    }
    return result


def read_manu3_boundary_chain(mem, guest_base: int, raster_segment: int) \
        -> dict[str, object]:
    """Follow the renderer's offset-linked vertical boundary list."""
    raster = raster_segment * 16
    sentinels = {
        offset + boundary_offset
        for offset in (
            MANU3_ACTIVE_LIST_HEAD_OFFSET,
            MANU3_ACTIVE_LIST_MIDDLE_OFFSET,
            MANU3_ACTIVE_LIST_TAIL_OFFSET,
        )
        for boundary_offset in (0, 0x10)
    }
    pool_end = (
        MANU3_RASTER_POOL_OFFSET
        + MANU3_RASTER_POOL_RECORD_SIZE * MANU3_RASTER_POOL_RECORD_COUNT)

    def valid_boundary_offset(offset: int) -> bool:
        if offset in sentinels:
            return True
        if not MANU3_RASTER_POOL_OFFSET <= offset < pool_end:
            return False
        within_record = (
            (offset - MANU3_RASTER_POOL_OFFSET)
            % MANU3_RASTER_POOL_RECORD_SIZE)
        return within_record in (0, 0x10)

    offset = MANU3_ACTIVE_LIST_HEAD_OFFSET
    seen: dict[int, int] = {}
    nodes: list[dict[str, object]] = []
    termination = "limit"
    cycle_at = None
    invalid_at = None
    for _index in range(MANU3_RASTER_POOL_RECORD_COUNT + 3):
        if offset in seen:
            termination = "cycle"
            cycle_at = offset
            break
        if not valid_boundary_offset(offset):
            termination = "invalid-offset"
            invalid_at = offset
            break
        seen[offset] = len(seen)
        raw = read_guest(mem, guest_base, raster + offset, 12)
        field_000, flags, source_offset, next_offset, field_008, coordinate = \
            struct.unpack("<HHHHHh", raw)
        if len(nodes) < 32:
            nodes.append({
                "offset": offset,
                "field_000": field_000,
                "flags": flags,
                "source_offset": source_offset,
                "next_offset": next_offset,
                "field_008": field_008,
                "coordinate": coordinate,
            })
        if flags & 0x8000:
            termination = "terminal"
            break
        if next_offset == 0:
            termination = "null"
            break
        offset = next_offset

    return {
        "termination": termination,
        "visited_count": len(seen),
        "cycle_at": cycle_at,
        "invalid_at": invalid_at,
        "nodes": nodes,
    }


def write_guest(mem, guest_base: int, linear: int, data: bytes) -> None:
    mem.seek(guest_base + linear)
    if mem.write(data) != len(data):
        raise RuntimeError(f"short guest write at {linear:#x}")
    mem.flush()


def send_mouse_button(display: str, pressed: bool, button: int = 1) -> None:
    env = dict(os.environ, DISPLAY=display)
    result = subprocess.run(
        ["xdotool", "mousedown" if pressed else "mouseup", str(button)],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)
    if result.returncode != 0:
        action = "press" if pressed else "release"
        raise RuntimeError(f"could not {action} host mouse button {button}")


def inject_guest_primary_click(mem, guest_base: int, game_segment: int,
                               x: int, y: int) -> dict[str, object]:
    """Inject the canonical position and latches for a primary edge.

    The ship HUD restores the DOS mouse position from ``mouse_last_x/y``
    before polling INT 33. Updating both coordinate pairs lets that native
    path preserve the requested point until the selector consumes the edge.
    """
    game = game_segment * 16
    encoded_x = struct.pack("<h", x)
    encoded_y = struct.pack("<h", y)
    for offset in (MOUSE_X_OFFSET, MOUSE_LAST_X_OFFSET):
        write_guest(mem, guest_base, game + offset, encoded_x)
    for offset in (MOUSE_Y_OFFSET, MOUSE_LAST_Y_OFFSET):
        write_guest(mem, guest_base, game + offset, encoded_y)
    write_guest(
        mem, guest_base, game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
    write_guest(
        mem, guest_base, game + MOUSE_PRESS_PENDING_OFFSET, b"\x01")
    return {"adapter": "guest-primary-edge", "point": [x, y]}


def guest_mouse_point_is_valid(x: int, y: int) -> bool:
    return 0 <= x < 320 and 0 <= y < 200


def recapture_game_mouse(display: str, executable: str,
                         toggle_capture: bool = False) -> dict[str, object]:
    """Recenter the host pointer without injecting guest-relative motion."""
    env = dict(os.environ, DISPLAY=display)
    executable_stem = Path(executable).stem
    search = subprocess.run(
        ["xdotool", "search", "--name", executable_stem],
        env=env, capture_output=True, text=True, check=False)
    window_ids = [line.strip() for line in search.stdout.splitlines()
                  if line.strip()]
    if search.returncode != 0 or not window_ids:
        raise RuntimeError(
            f"could not locate the {executable_stem} DOSBox window")
    window_id = window_ids[0]
    geometry = subprocess.run(
        ["xdotool", "getwindowgeometry", "--shell", window_id],
        env=env, capture_output=True, text=True, check=False)
    if geometry.returncode != 0:
        raise RuntimeError(
            f"could not read DOSBox window geometry for {window_id}")
    fields = {}
    for line in geometry.stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator and value.lstrip("-").isdigit():
            fields[key] = int(value)
    width = fields.get("WIDTH", 640)
    height = fields.get("HEIGHT", 400)
    activate = subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)
    focus = subprocess.run(
        ["xdotool", "windowfocus", "--sync", window_id],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)
    if toggle_capture:
        released = subprocess.run(
            ["xdotool", "click", "2"], env=env,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            check=False)
        if released.returncode != 0:
            raise RuntimeError(
                f"could not release mouse capture in window {window_id}")
        time.sleep(0.05)
    moved = subprocess.run(
        ["xdotool", "mousemove", "--sync", "--window", window_id,
         str(width // 2), str(height // 2)],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)
    # windowactivate requires an EWMH window manager.  The isolated Xephyr
    # display deliberately has none, so direct X focus is the required gate.
    if focus.returncode != 0 or moved.returncode != 0:
        raise RuntimeError(
            f"could not recapture the DOSBox mouse in window {window_id}")
    if toggle_capture:
        captured = subprocess.run(
            ["xdotool", "click", "2"], env=env,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            check=False)
        if captured.returncode != 0:
            raise RuntimeError(
                f"could not restore mouse capture in window {window_id}")
        time.sleep(0.05)
    return {
        "window_id": window_id,
        "window_size": [width, height],
        "window_point": [width // 2, height // 2],
        "capture_toggled": toggle_capture,
        "window_activated": activate.returncode == 0,
    }


def dosbox_mouse_settings(executable: str) -> list[str]:
    """Return emulator-specific settings for deterministic relative input."""
    if Path(executable).name.lower() == "dosbox-x":
        return ["-set", "sdl autolock=true"]
    return [
        "-set", "mouse mouse_capture=onstart",
        "-set", "mouse mouse_raw_input=false",
    ]


def dosbox_needs_capture_toggle(executable: str) -> bool:
    return Path(executable).name.lower() != "dosbox-x"


def move_captured_game_mouse(display: str, current_x: int, current_y: int,
                             target_x: int, target_y: int) -> bool:
    delta_x = target_x - current_x
    delta_y = target_y - current_y
    if abs(delta_x) <= 2 and abs(delta_y) <= 2:
        return True

    # DOSBox reports relative motion while its mouse is captured. Small,
    # bounded steps let the next sampled guest position close the loop even
    # when SDL sensitivity or scaling differs between hosts.
    step_x = max(-32, min(32, delta_x))
    step_y = max(-32, min(32, delta_y))
    env = dict(os.environ, DISPLAY=display)
    moved = subprocess.run(
        ["xdotool", "mousemove_relative", "--sync", "--",
         str(step_x), str(step_y)],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        check=False)
    if moved.returncode != 0:
        raise RuntimeError(
            "could not move the captured host mouse toward game point "
            f"{target_x},{target_y}")
    return False


def choice_row_point(rect: tuple[int, int, int, int],
                     row_index: int) -> tuple[int, int]:
    x, y, width, _height = rect
    return x + width // 2, y + 4 + row_index * 11 + 5


def pterra_destination_ready(blockers: dict[str, int],
                             flow: dict[str, object]) -> bool:
    transition_blockers = (
        value for name, value in blockers.items() if name != "vm_ui")
    return (
        flow["c2_presentation_gate"] == 0
        and resource_pipeline_idle(flow)
        and all(value == 0 for value in transition_blockers)
    )


def pterra_ship_intro_waiting_for_input(
        blockers: dict[str, int], flow: dict[str, object],
        input_state: dict[str, object]) -> bool:
    """Recognize the final intro hold without spilling input into the HUD."""
    return (
        int(blockers["ship"]) & 6 != 0
        and int(input_state["ship_hud_initialized"]) == 0
        and int(input_state["ship_target_select_phase"]) == 0
        and (int(flow["active_line"]) == 5
             or int(flow["displayed_line"]) == 5)
        and int(flow["dialogue_hold_complete"]) & 1 != 0
        and int(flow["dialogue_hold_countdown"]) > 0
    )


def pterra_ship_intro_ready_for_edge(flow: dict[str, object]) -> bool:
    countdown = int(flow["dialogue_hold_countdown"])
    return 0 < countdown <= PTERRA_NATIVE_INPUT_TRIGGER_COUNTDOWN


def pterra_ship_intro_press_should_release(
        now: float, pressed_at: float | None) -> bool:
    return (
        pressed_at is not None
        and now - pressed_at >= PTERRA_NATIVE_INPUT_PULSE_SECONDS
    )


def pterra_ship_intro_input_action(
        pressing: bool, release_ready: bool, latch_active: bool,
        can_press: bool) -> str:
    if pressing:
        return "release" if release_ready else "hold"
    if latch_active:
        return "wait"
    return "press" if can_press else "wait"


def pterra_ship_intro_consumed_before_expiry(
        flow: dict[str, object], input_state: dict[str, object],
        lines_seen: list[int], edge_count: int, raw_seen: bool,
        latch_seen: bool) -> tuple[bool, str | None]:
    if edge_count == 0 or int(flow["dialogue_hold_countdown"]) <= 0:
        return False, None
    if int(flow["dialogue_hold_complete"]) == 0:
        return True, "hold-clear-observed"
    hud_handoff = (
        raw_seen
        and latch_seen
        and lines_seen[-2:] == [4, 5]
        and int(input_state["ship_hud_initialized"]) & 1 != 0
        and int(input_state["ship_target_select_phase"]) > 0
    )
    return (True, "guest-latched-hud-handoff") \
        if hud_handoff else (False, None)


def pterra_ship_intro_is_naturally_complete(
        blockers: dict[str, int], flow: dict[str, object],
        input_state: dict[str, object], lines_seen: list[int],
        edge_count: int) -> bool:
    """Recognize a complete countdown-to-HUD handoff with no input edge."""
    return (
        edge_count == 0
        and lines_seen[-2:] == [4, 5]
        and int(flow["dialogue_hold_complete"]) == 0
        and int(flow["dialogue_hold_countdown"]) == 0
        and int(blockers["ship"]) & 4 != 0
        and int(input_state["ship_hud_initialized"]) & 1 != 0
        and int(input_state["ship_target_select_phase"]) > 0
    )


def native_gameplay_control_ready(audio_flow: dict[str, object],
                                  blockers: dict[str, int],
                                  flow: dict[str, object]) -> bool:
    """Return true once a presentation has handed control to the bridge.

    The bridge can retain a queued ambient resource while it is interactive,
    so queue emptiness is not an ownership signal at this boundary.
    """
    return (
        int(audio_flow["presentation_mode_27e0"]) & 1 == 0
        and int(audio_flow["presentation_mode_27e1"]) & 1 == 0
        and int(flow["c2_presentation_gate"]) == 0
        and all(value == 0 for value in blockers.values())
    )


def bridge_navigation_timed_out(
        now: float, started_at: float, last_progress_at: float,
        first_click_at: float | None = None) -> bool:
    if first_click_at is not None:
        return now - first_click_at >= PTERRA_NAV_ACTIVATION_TIMEOUT_SECONDS
    return (
        now - last_progress_at >= PTERRA_MAP_TRANSITION_TIMEOUT_SECONDS
        or now - started_at >= PTERRA_BRIDGE_ROTATION_TIMEOUT_SECONDS
    )


def record_expected_pterra_choice(results: list[int],
                                   selected_word: int) -> bool:
    if (len(results) >= len(EXPECTED_PTERRA_CHOICES)
            or selected_word != EXPECTED_PTERRA_CHOICES[len(results)]):
        return False
    results.append(selected_word)
    return True


def pterra_encounter_idle(blockers: dict[str, int],
                          flow: dict[str, object],
                          choice_results: list[int]) -> bool:
    return (
        choice_results == list(EXPECTED_PTERRA_CHOICES)
        and blockers["presentation"] == 0
        and blockers["ship"] == 0
        and blockers["render"] == 0
        and int(flow["active_line"]) == 0xffff
        and int(flow["c2_presentation_gate"]) == 0
    )


def resource_pipeline_idle(flow: dict[str, object]) -> bool:
    return (
        int(flow["resource_source_remaining"]) == 0
        and int(flow["list_queued_bytes"]) == 0
        and flow["list_active"] == "0000:0000")


def read_script2_pterra_context(mem, guest_base: int,
                                game_segment: int) -> dict[str, object]:
    game = game_segment * 16
    cod_offset, cod_segment = struct.unpack(
        "<HH", read_guest(
            mem, guest_base, game + VM_RESOURCE_IMAGES_OFFSET, 4))
    if cod_segment < 0x0050:
        raise RuntimeError(
            "invalid VM code-image pointer "
            f"{cod_segment:04x}:{cod_offset:04x}")
    record_offset, record_segment = struct.unpack(
        "<HH", read_guest(
            mem, guest_base, game + VM_RECORD_BASE_POINTER_OFFSET, 4))
    if record_segment < 0x0050:
        raise RuntimeError(
            "invalid VM record-base pointer "
            f"{record_segment:04x}:{record_offset:04x}")
    cod_base = cod_segment * 16 + cod_offset
    record_base = record_segment * 16 + record_offset
    orxx_offset = struct.unpack(
        "<H", read_guest(
            mem, guest_base,
            game + VM_NAMED_ORXX_OBJECT_OFFSET, 2))[0]
    arche_offset = struct.unpack(
        "<H", read_guest(
            mem, guest_base,
            game + VM_NAMED_ARCHETYPE_OBJECT_OFFSET, 2))[0]
    ark_offset = struct.unpack(
        "<H", read_guest(
            mem, guest_base,
            game + VM_NAMED_ARK_OBJECT_OFFSET, 2))[0]
    return {
        "cod_base": f"{cod_segment:04x}:{cod_offset:04x}",
        "record_base": f"{record_segment:04x}:{record_offset:04x}",
        "record_segment": record_segment,
        "record_offset": record_offset,
        "orxx_offset": orxx_offset,
        "arche_offset": arche_offset,
        "ark_offset": ark_offset,
        "orxx_action": list(struct.unpack(
            "<HHH", read_guest(
                mem, guest_base, record_base + orxx_offset + 0x000a, 6))),
        "arche_action": list(struct.unpack(
            "<HHH", read_guest(
                mem, guest_base, record_base + arche_offset + 0x001c, 6))),
        "current_location": struct.unpack(
            "<H", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_ARCHETYPE_CURRENT_LOCATION_OFFSET,
                2))[0],
        "pterra_flags": struct.unpack(
            "<H", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_PTERRA_FLAGS_OFFSET, 2))[0],
        "pterra_access_count": struct.unpack(
            "<H", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_PTERRA_RECORD + 0x0014, 2))[0],
        "pterra_unlock_state": struct.unpack(
            "<H", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET, 2))[0],
        "init_procedure_enabled": read_guest(
            mem, guest_base,
            cod_base + SCRIPT2_INIT_PROCEDURE_OFFSET, 1)[0],
        "pterra_marker": list(struct.unpack(
            "<HH", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_PTERRA_RECORD + 0x0018, 4))),
        "scruter_action": list(struct.unpack(
            "<HHH", read_guest(
                mem, guest_base,
                record_base + SCRIPT2_SCRUTER_JO_ACTION_OFFSET, 6))),
        "sequence_flags": read_guest(
            mem, guest_base, game + VM_SEQUENCE_ACTIVE_OFFSET, 1)[0],
        "ship_active_flags": struct.unpack(
            "<H", read_guest(
                mem, guest_base, game + VM_SHIP_ACTIVE_FLAGS_OFFSET, 2))[0],
    }


def request_script2_pterra_unlock(mem, guest_base: int,
                                  game_segment: int) -> dict[str, object]:
    """Satisfy SCRIPT2.init and let the VM itself set Pterra in play."""
    context = read_script2_pterra_context(mem, guest_base, game_segment)
    if int(context["pterra_flags"]) & SCRIPT2_IN_PLAY_FLAG:
        return {
            "source": "save",
            "before": context,
        }
    if int(context["init_procedure_enabled"]) \
            != SCRIPT2_INIT_PROCEDURE_ENABLED:
        raise RuntimeError(
            "SCRIPT2 init procedure is disabled before Pterra is in play")
    if int(context["pterra_unlock_state"]) not in (0, 1):
        raise RuntimeError(
            "SCRIPT2 Pterra-unlock predicate has unexpected value "
            f"{int(context['pterra_unlock_state']):#06x}")

    record_segment = int(context["record_segment"])
    record_offset = int(context["record_offset"])
    state_linear = (record_segment * 16 + record_offset
                    + SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET)
    write_guest(mem, guest_base, state_linear, struct.pack("<H", 1))
    if struct.unpack(
            "<H", read_guest(mem, guest_base, state_linear, 2))[0] != 1:
        raise RuntimeError("SCRIPT2 Pterra-unlock predicate write did not persist")
    return {
        "source": "recovered-init-predicate",
        "before": context,
    }


def script2_pterra_unlock_completed(context: dict[str, object]) -> bool:
    return (
        int(context["pterra_flags"]) & SCRIPT2_IN_PLAY_FLAG != 0
        and int(context["init_procedure_enabled"]) == 0
    )


def prepare_native_nav_chart(mem, guest_base: int,
                             game_segment: int) -> dict[str, object]:
    """Validate the bridge boundary before physically opening the chart."""
    game = game_segment * 16
    context = read_script2_pterra_context(mem, guest_base, game_segment)
    if int(context["pterra_flags"]) & SCRIPT2_IN_PLAY_FLAG == 0:
        raise RuntimeError("Pterra is not enabled before nav-chart entry")
    view_active = read_guest(
        mem, guest_base, game + NAV_CAMERA_VIEW_ACTIVE_OFFSET, 1)[0]
    view_state = read_guest(
        mem, guest_base, game + NAV_CAMERA_VIEW_STATE_OFFSET, 1)[0]
    selected = struct.unpack(
        "<H", read_guest(
            mem, guest_base,
            game + NAV_SELECTED_LOCATION_RECORD_OFFSET, 2))[0]
    if view_active != 0 or view_state != 0 or selected != 0:
        raise RuntimeError(
            "nav chart is not at its bridge-idle boundary: "
            f"active={view_active} state={view_state} selected={selected:#06x}")
    return {
        "entry": "native-bridge-station",
        "bridge_station_index": 0,
        "view_active_before": view_active,
        "view_state_before": view_state,
        "pterra_marker": context["pterra_marker"],
    }


def selectable_rect_center(rect: tuple[int, int, int, int]) \
        -> tuple[int, int] | None:
    x, y, width, height = rect
    if x < 0 or y < 0 or width <= 0 or height <= 0:
        return None
    return x + width // 2, y + height // 2


def prepare_script2_orxx_descent(mem, guest_base: int,
                                 game_segment: int) -> dict[str, object]:
    """Validate the idle bridge boundary before native ship navigation."""
    game = game_segment * 16
    context = read_script2_pterra_context(mem, guest_base, game_segment)
    record_segment = int(context["record_segment"])
    record_offset = int(context["record_offset"])
    record_base = record_segment * 16 + record_offset
    orxx_offset = int(context["orxx_offset"])
    if orxx_offset == 0 or orxx_offset == 0xffff:
        raise RuntimeError(f"invalid named Orxx record {orxx_offset:#06x}")
    orxx_kind = struct.unpack(
        "<H", read_guest(
            mem, guest_base, record_base + orxx_offset, 2))[0]
    pterra_kind = struct.unpack(
        "<H", read_guest(
            mem, guest_base, record_base + SCRIPT2_PTERRA_RECORD, 2))[0]
    if orxx_kind != 0x0200:
        raise RuntimeError(
            f"named Orxx record has unexpected kind {orxx_kind:#06x}")
    if pterra_kind != 0x0008:
        raise RuntimeError(
            f"Pterra record has unexpected kind {pterra_kind:#06x}")
    if int(context["pterra_flags"]) & SCRIPT2_IN_PLAY_FLAG == 0:
        raise RuntimeError("Pterra is not present in the native ship target list")
    if int(context["current_location"]) != SCRIPT2_PTERRA_RECORD:
        raise RuntimeError(
            "native ship navigation is not at Pterra: "
            f"current={int(context['current_location']):#06x}")

    title_mode = read_guest(
        mem, guest_base, game + PRESENTATION_MODE_FLAG_27E0_OFFSET, 1)[0]
    presentation_active = read_guest(
        mem, guest_base, game + PRESENTATION_MODE_FLAG_27E1_OFFSET, 1)[0]
    if title_mode & 1 or presentation_active & 1:
        raise RuntimeError(
            "cannot submit Pterra travel outside native gameplay mode")

    action_linear = record_base + orxx_offset + 0x000a
    before_action = list(struct.unpack(
        "<HHH", read_guest(mem, guest_base, action_linear, 6)))
    if before_action != [0, 0, 0]:
        raise RuntimeError(
            f"Orxx action slot is not idle: {before_action!r}")

    ship_flags_before = struct.unpack(
        "<H", read_guest(
            mem, guest_base, game + VM_SHIP_ACTIVE_FLAGS_OFFSET, 2))[0]
    if ship_flags_before != 0:
        raise RuntimeError(
            "ship navigation is not at its bridge-idle boundary: "
            f"flags={ship_flags_before:#06x}")
    return {
        "entry": "native-current-location-entity",
        "entity_index": CURRENT_LOCATION_ENTITY_INDEX,
        "record_base": context["record_base"],
        "orxx_offset": orxx_offset,
        "orxx_action_before": before_action,
        "orxx_action_expected": [
            BLOODPRG_VM_RECORD_C1, SCRIPT2_PTERRA_RECORD, 0],
        "ship_active_flags_before": ship_flags_before,
        "pterra_access_count_before": context["pterra_access_count"],
        "title_mode": title_mode,
        "presentation_mode": presentation_active,
    }


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
    current_location_entity = game + ENTITY_TABLE_OFFSET \
        + CURRENT_LOCATION_ENTITY_INDEX * ENTITY_RECORD_SIZE
    current_location_entity_flags = struct.unpack(
        "<H", read_guest(
            mem, guest_base, current_location_entity, 2))[0]
    current_location_entity_rect = struct.unpack(
        "<hhhh", read_guest(
            mem, guest_base, current_location_entity + 8, 8))
    ship_target_name_offsets = []
    raw_ship_targets = struct.unpack(
        f"<{SHIP_3D_PRESENTABLE_NAME_OFFSET_COUNT}H",
        read_guest(
            mem, guest_base,
            game + SHIP_3D_PRESENTABLE_NAME_OFFSETS_OFFSET,
            SHIP_3D_PRESENTABLE_NAME_OFFSET_COUNT * 2))
    for target_name_offset in raw_ship_targets:
        if target_name_offset in (0, 0xffff):
            break
        ship_target_name_offsets.append(target_name_offset)
    nav_chart_object_count = struct.unpack(
        "<H", read_guest(
            mem, guest_base, game + NAV_CHART_OBJECT_COUNT_OFFSET, 2))[0]
    nav_chart_object_offsets = []
    if nav_chart_object_count <= NAV_CHART_MAX_OBJECTS:
        nav_chart_object_offsets = list(struct.unpack(
            f"<{nav_chart_object_count}H",
            read_guest(
                mem, guest_base,
                game + NAV_CHART_OBJECT_OFFSETS_OFFSET,
                nav_chart_object_count * 2)))
    list_head_offset, list_head_segment = struct.unpack(
        "<HH", read_guest(mem, guest_base, game + 0x0D8C, 4))
    list_tail_offset, list_tail_segment = struct.unpack(
        "<HH", read_guest(mem, guest_base, game + 0x0D90, 4))
    list_active_offset, list_active_segment = struct.unpack(
        "<HH", read_guest(mem, guest_base, game + 0x0D94, 4))
    list_buffer_end = struct.unpack(
        "<H", read_guest(mem, guest_base, game + 0x5233, 2))[0]
    list_wrap_limit = struct.unpack(
        "<H", read_guest(mem, guest_base, game + 0x0D98, 2))[0]
    list_tail_head = read_guest(
        mem, guest_base,
        list_tail_segment * 16 + list_tail_offset, 16).hex()
    list_head_context_offset = (list_head_offset - 2) & 0xffff
    list_head_context = read_guest(
        mem, guest_base,
        list_head_segment * 16 + list_head_context_offset, 16).hex()
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
        "graphics_pointers": read_graphics_pointer_state(
            mem, guest_base, game_segment),
        "input": {
            "mouse_x": mouse_x,
            "mouse_y": mouse_y,
            "mouse_button_state": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + MOUSE_BUTTON_STATE_OFFSET, 2))[0],
            "mouse_previous_button_state": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + MOUSE_PREVIOUS_BUTTON_STATE_OFFSET, 2))[0],
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
            "current_location_entity": {
                "index": CURRENT_LOCATION_ENTITY_INDEX,
                "flags": current_location_entity_flags,
                "hit_rect": list(current_location_entity_rect),
            },
            "bridge_panorama_frame": read_guest(
                mem, guest_base,
                game + BRIDGE_PANORAMA_FRAME_OFFSET, 1)[0],
            "bridge_ui_state": struct.unpack(
                "<H", read_guest(
                    mem, guest_base, game + VM_UI_FLAGS_OFFSET, 2))[0],
            "bridge_seek_target_arc": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + BRIDGE_SEEK_TARGET_ARC_OFFSET, 2))[0],
            "bridge_seek_initial_distance": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + BRIDGE_SEEK_INITIAL_DISTANCE_OFFSET, 2))[0],
            "bridge_turn_direction": read_guest(
                mem, guest_base,
                game + BRIDGE_TURN_DIRECTION_OFFSET, 1)[0],
            "ship_target_name_offsets": ship_target_name_offsets,
            "ship_current_target": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + SHIP_3D_CURRENT_TARGET_OFFSET, 2))[0],
            "ship_hud_initialized": read_guest(
                mem, guest_base,
                game + SHIP_3D_HUD_INITIALIZED_OFFSET, 1)[0],
            "ship_target_select_phase": read_guest(
                mem, guest_base,
                game + SHIP_3D_TARGET_SELECT_PHASE_OFFSET, 1)[0],
            "ship_scene_dispatch_blocked": read_guest(
                mem, guest_base,
                game + SHIP_3D_SCENE_DISPATCH_BLOCKED_OFFSET, 1)[0],
            "ship_dialogue_cycle_line": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + SHIP_3D_DIALOGUE_CYCLE_LINE_OFFSET, 2))[0],
            "ship_dialogue_phase_ready": read_guest(
                mem, guest_base,
                game + SHIP_3D_DIALOGUE_PHASE_READY_OFFSET, 1)[0],
            "ship_hud_init_pending": read_guest(
                mem, guest_base,
                game + SHIP_3D_HUD_INIT_PENDING_OFFSET, 1)[0],
            "nav_camera_view_active": read_guest(
                mem, guest_base,
                game + NAV_CAMERA_VIEW_ACTIVE_OFFSET, 1)[0],
            "nav_camera_view_state": read_guest(
                mem, guest_base,
                game + NAV_CAMERA_VIEW_STATE_OFFSET, 1)[0],
            "nav_center_wipe_complete": read_guest(
                mem, guest_base,
                game + NAV_CENTER_WIPE_COMPLETE_OFFSET, 1)[0],
            "nav_location_panel_active": read_guest(
                mem, guest_base,
                game + NAV_LOCATION_PANEL_ACTIVE_OFFSET, 1)[0],
            "nav_location_panel_transition_state": read_guest(
                mem, guest_base,
                game + NAV_LOCATION_PANEL_TRANSITION_STATE_OFFSET, 1)[0],
            "nav_location_panel_scale_step": read_guest(
                mem, guest_base,
                game + NAV_LOCATION_PANEL_SCALE_STEP_OFFSET, 1)[0],
            "nav_selected_location_record": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + NAV_SELECTED_LOCATION_RECORD_OFFSET, 2))[0],
            "nav_chart_object_count": nav_chart_object_count,
            "nav_chart_object_offsets": nav_chart_object_offsets,
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
            "list_head": f"{list_head_segment:04x}:{list_head_offset:04x}",
            "list_tail": f"{list_tail_segment:04x}:{list_tail_offset:04x}",
            "list_active": (
                f"{list_active_segment:04x}:{list_active_offset:04x}"),
            "list_buffer_end": list_buffer_end,
            "list_wrap_limit": list_wrap_limit,
            "list_tail_head": list_tail_head,
            "list_head_context": list_head_context,
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
            "text_display_active": read_guest(
                mem, guest_base,
                game + VM_TEXT_DISPLAY_ACTIVE_OFFSET, 1)[0],
            "text_reveal_cursor": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_TEXT_REVEAL_CURSOR_OFFSET, 2))[0],
            "displayed_line": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_DISPLAYED_LINE_OFFSET, 2))[0],
            "presentation_owner_offset": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_PRESENTATION_OWNER_OFFSET, 2))[0],
            "dialogue_hold_complete": read_guest(
                mem, guest_base,
                game + VM_DIALOGUE_HOLD_COMPLETE_OFFSET, 1)[0],
            "dialogue_hold_countdown": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + VM_DIALOGUE_HOLD_COUNTDOWN_OFFSET, 2))[0],
            "presentation_hold_ready": read_guest(
                mem, guest_base,
                game + VM_PRESENTATION_HOLD_READY_OFFSET, 1)[0],
            "deferred_record_type": struct.unpack(
                "<H", read_guest(mem, guest_base, game + 0x6768, 2))[0],
            "deferred_record_related": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + NAV_DEFERRED_RECORD_LINK_OFFSET, 2))[0],
            "deferred_record_value": struct.unpack(
                "<H", read_guest(mem, guest_base, game + 0x676C, 2))[0],
            "sequence_active": read_guest(
                mem, guest_base, game + VM_SEQUENCE_ACTIVE_OFFSET, 1)[0],
            "ship_current_target": struct.unpack(
                "<H", read_guest(mem, guest_base, game + 0x251B, 2))[0],
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
            "clip_playback_state": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + SND_CLIP_PLAYBACK_STATE_OFFSET, 2))[0],
            "stream_channel_active": read_guest(
                mem, guest_base,
                game + SND_STREAM_CHANNEL_ACTIVE_OFFSET, 1)[0],
            "presentation_mode_27e0": read_guest(
                mem, guest_base,
                game + PRESENTATION_MODE_FLAG_27E0_OFFSET, 1)[0],
            "presentation_mode_27e1": read_guest(
                mem, guest_base,
                game + PRESENTATION_MODE_FLAG_27E1_OFFSET, 1)[0],
            "list_audio_phase": struct.unpack(
                "<H", read_guest(
                    mem, guest_base,
                    game + LIST_D8C_AUDIO_PHASE_OFFSET, 2))[0],
            "position_callback": "%04x:%04x" % struct.unpack(
                "<HH", read_guest(
                    mem, guest_base,
                    game + SND_AUDIO_POSITION_CALLBACK_OFFSET, 4))[::-1],
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


def authentic_gameplay_start_actions() -> list[str]:
    """Enter the first native presentation without touching later input."""
    # DOSBox starts its captured game cursor at 160,150. The title orb is at
    # 160,161 in guest coordinates; an absolute X11 move is interpreted as a
    # large relative delta while capture is active and corrupts the cursor.
    return ["wait_title", "move_relative 0 11", "mouse_button 1"]


def title_transition_evidence(*, startup_presentation_line_seen: bool,
                              load_menu_requested: bool,
                              authentic_save_loaded: bool) -> list[str]:
    """Return durable observations proving that the title transition finished."""
    evidence = []
    if startup_presentation_line_seen:
        evidence.append("startup-presentation-line")
    if load_menu_requested:
        evidence.append("native-gameplay-load-boundary")
    if authentic_save_loaded:
        evidence.append("authentic-save-loaded")
    return evidence


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


def linear_surface_summary(data: bytes) -> dict[str, object]:
    if len(data) != 320 * 200:
        raise ValueError("a linear game surface must be exactly 320x200 bytes")
    rows = [data[row * 320:(row + 1) * 320] for row in range(200)]
    return {
        "sha256": hashlib.sha256(data).hexdigest(),
        "byte_count": len(data),
        "unique_byte_count": len(set(data)),
        "nonzero_row_count": sum(any(row) for row in rows),
        "row_sha256": [hashlib.sha256(row).hexdigest() for row in rows],
    }


def snapshot_linear_surfaces(mem, guest_base: int, game_segment: int,
                             capture_dir: Path | None = None) \
        -> dict[str, object]:
    surfaces: dict[str, object] = {}
    game = game_segment * 16
    for name, pointer_offset in (
        ("display", 0x5221),
        ("back_buffer", 0x5229),
    ):
        offset, segment = struct.unpack(
            "<HH", read_guest(mem, guest_base, game + pointer_offset, 4))
        linear = segment * 16 + offset
        entry: dict[str, object] = {
            "pointer": f"{segment:04x}:{offset:04x}",
            "linear": linear,
        }
        if segment != 0 and linear + 320 * 200 <= 0x100000:
            data = read_guest(mem, guest_base, linear, 320 * 200)
            entry.update(linear_surface_summary(data))
            if capture_dir is not None:
                capture_dir.mkdir(parents=True, exist_ok=True)
                path = capture_dir / f"pterra-marker-{name}.bin"
                path.write_bytes(data)
                entry["capture"] = str(path)
        else:
            entry["error"] = "surface pointer is outside conventional memory"
        surfaces[name] = entry
    return surfaces


def snapshot_guest(mem, guest_base: int, anchor: int,
                   cpu_addresses: dict[str, int], marker: Path | None,
                   profile: dict[str, object] | None,
                   surface_capture_dir: Path | None = None) \
        -> dict[str, object]:
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
        snapshot["linear_surfaces"] = snapshot_linear_surfaces(
            mem, guest_base, game_segment, surface_capture_dir)
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


def capture_state_pterra(db: subprocess.Popen[bytes], libc, marker: Path,
                         log_path: Path, timeout: float, display: str,
                         executable: str, manual: bool = False,
                         open_load_menu: bool = False,
                         trigger_pterra_after_load: bool = False,
                         drive_authentic_save: bool = False,
                         guest_snapshot: Path | None = None,
                         post_pter_seconds: float = 5.0,
                         toggle_mouse_capture: bool = False) \
        -> dict[str, object]:
    capture_started_at = time.monotonic()
    deadline = capture_started_at + timeout
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
    title_accept_sent = False
    title_accept_pressing = False
    title_pointer_recapture = None
    startup_presentation_line_seen = False
    startup_presentation_pressing = False
    post_load_pressing = False
    post_load_release_at = None
    post_load_dismiss_input_sent = False
    guest_snapshot_written = False
    pterra_unlock_requested = False
    pterra_unlock_completed = False
    pterra_unlock_setup = None
    pterra_nav_chart_started = False
    pterra_nav_chart_active = False
    pterra_nav_open_started_at = None
    pterra_bridge_panorama_frame = None
    pterra_nav_station_pressing = False
    pterra_nav_station_target_y = None
    pterra_nav_station_first_click_at = None
    pterra_nav_station_click_count = 0
    pterra_nav_chart_reopen_count = 0
    pterra_nav_pointer_last_position = None
    pterra_nav_pointer_last_changed_at = None
    pterra_nav_pointer_recapture_count = 0
    pterra_host_mouse_ready_at = None
    pterra_nav_chart_pressing = False
    pterra_nav_chart_selected = False
    pterra_nav_panel_close_pressing = False
    pterra_nav_panel_close_requested = False
    pterra_nav_panel_close_confirmed = False
    pterra_map_command_generated = False
    pterra_map_command_consumed = False
    pterra_map_destination_committed = False
    pterra_map_last_progress_at = None
    pterra_map_setup = None
    scruter_scene_requested = False
    scruter_scene_active_seen = False
    scruter_scene_completed = False
    scruter_sound_bank_loaded = False
    scruter_streamed_clip_count_before = 0
    scruter_streamed_clip_count = 0
    pter_semantic_checkpoints: list[dict[str, object]] = []
    pterra_arrival_last_progress_at = None
    pterra_travel_setup = None
    pterra_ship_navigation_started_at = None
    pterra_ship_navigation_activated = False
    pterra_ship_region_pressing = False
    pterra_ship_region_click_count = 0
    pterra_ship_region_next_press_at = 0.0
    pterra_ship_intro_lines_seen: list[int] = []
    pterra_ship_intro_started_at = None
    pterra_ship_intro_next_edge_at = 0.0
    pterra_ship_intro_capture_ready_at = None
    pterra_ship_intro_edge_count = 0
    pterra_ship_intro_input_evidence = None
    pterra_ship_intro_pressing = False
    pterra_ship_intro_press_started_at = None
    pterra_ship_intro_raw_seen = False
    pterra_ship_intro_latch_seen = False
    pterra_ship_intro_hold_observed = False
    pterra_ship_intro_dismissed = False
    pterra_ship_intro_completed_naturally = False
    pterra_ship_target_phases_seen: list[int] = []
    pterra_ship_hud_progress_key = None
    pterra_ship_hud_last_progress_at = None
    pterra_arrival_progress_key = None
    pterra_target_row = None
    pterra_target_pressing = False
    pterra_travel_command_generated = False
    pterra_travel_command_consumed = False
    destination_committed = False
    pter_reached = False
    pter_reached_at = None
    pter_completed_at = None
    pter_last_progress_at = None
    pter_last_semantic_key = None
    pter_sustained = False
    pter_choice_results: list[int] = []
    pter_input_pressed = None
    pter_next_input_at = 0.0
    marker_snapshot = None
    fault_snapshot = None
    dos_read_overflow_snapshot = None
    integrity_fault_snapshot = None
    hang_snapshot = None
    pter_snapshot = None
    post_pter_snapshot = None
    post_pter_cpu_samples: list[dict[str, int]] = []
    transition_cpu_samples: list[dict[str, object]] = []
    transition_next_sample_at = None
    ivt_baseline = None
    graphics_pointer_baseline = None
    graphics_pointer_faults: list[str] = []
    last_profile = None
    last_cpu_state = None
    last_profile_key = None
    log_offset = 0
    log_tail = b""

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
                graphics_pointers = last_profile["graphics_pointers"]
                assert isinstance(graphics_pointers, dict)
                if graphics_pointer_baseline is None:
                    graphics_pointer_baseline = graphics_pointers
                else:
                    graphics_pointer_faults = graphics_pointer_errors(
                        graphics_pointers,
                        graphics_pointer_baseline,
                        game_segment)
                    if graphics_pointer_faults:
                        integrity_fault_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        integrity_fault_snapshot["graphics_pointer_baseline"] = \
                            graphics_pointer_baseline
                        integrity_fault_snapshot["graphics_pointer_faults"] = \
                            graphics_pointer_faults
                        print(
                            "integrity: " + graphics_pointer_faults[0],
                            flush=True)
                        break
                scene_flow = last_profile["scene_flow"]
                audio_flow = last_profile["audio_flow"]
                assert isinstance(audio_flow, dict)
                profile_key = (
                    last_profile["profile"],
                    last_profile["request"],
                    tuple(last_profile["blockers"].items()),
                    audio_flow["presentation_mode_27e0"],
                    audio_flow["presentation_mode_27e1"],
                    scene_flow["active_line"],
                    scene_flow["c2_presentation_gate"],
                    scene_flow["text_display_active"],
                    scene_flow["text_reveal_cursor"],
                    scene_flow["displayed_line"],
                    scene_flow["presentation_owner_offset"],
                    scene_flow["list_d8c_state"],
                    (
                        last_profile["input"]["mouse_x"],
                        last_profile["input"]["mouse_y"],
                        last_profile["input"]["primary_pressed"],
                    ) if (last_profile["blockers"]["load"]
                          or pterra_nav_chart_active) else (),
                    last_profile["input"]["save_menu_phase"],
                    tuple(last_profile["input"]["choice_rect"]),
                    last_profile["input"]["word_choice_active"],
                    last_profile["input"]["word_choice_phase"],
                    last_profile["input"]["selected_word"],
                    last_profile["input"]["ship_hud_initialized"],
                    last_profile["input"]["ship_target_select_phase"],
                    last_profile["input"]["ship_dialogue_cycle_line"],
                    tuple(last_profile["input"][
                        "ship_target_name_offsets"]),
                    last_profile["input"]["nav_camera_view_active"],
                    last_profile["input"]["nav_camera_view_state"],
                    last_profile["input"]["nav_center_wipe_complete"],
                    last_profile["input"]["nav_location_panel_active"],
                    last_profile["input"][
                        "nav_location_panel_transition_state"],
                    last_profile["input"]["nav_location_panel_scale_step"],
                    last_profile["input"]["nav_selected_location_record"],
                    tuple(last_profile["input"][
                        "nav_chart_object_offsets"]),
                    last_profile["input"]["bridge_panorama_frame"],
                    last_profile["input"]["bridge_ui_state"],
                    last_profile["input"]["bridge_seek_target_arc"],
                    last_profile["input"]["bridge_seek_initial_distance"],
                    last_profile["input"]["bridge_turn_direction"],
                )
                if profile_key != last_profile_key:
                    last_profile_key = profile_key
                    print(
                        "state: "
                        f"profile={last_profile['profile']} "
                        f"request={last_profile['request']} "
                        f"blockers={last_profile['blockers']} "
                        "modes="
                        f"{audio_flow['presentation_mode_27e0']}:"
                        f"{audio_flow['presentation_mode_27e1']} "
                        f"flow={last_profile['scene_flow']} "
                        f"input={last_profile['input']}",
                        flush=True)

                blockers = last_profile["blockers"]
                assert isinstance(blockers, dict)
                game = game_segment * 16
                title_idle_ready = (
                    drive_authentic_save
                    and not title_accept_sent
                    and profile_releaseable(last_profile)
                    and int(audio_flow["presentation_mode_27e0"]) & 1 != 0
                    and int(audio_flow["presentation_mode_27e1"]) & 1 == 0
                    and int(scene_flow["active_line"]) == 0xffff
                    and int(scene_flow["c2_presentation_gate"]) == 0
                    and resource_pipeline_idle(scene_flow))
                if title_accept_pressing:
                    send_mouse_button(display, False, button=1)
                    title_accept_pressing = False
                elif title_idle_ready:
                    if title_pointer_recapture is None:
                        title_pointer_recapture = recapture_game_mouse(
                            display, executable, toggle_mouse_capture)
                    input_state = last_profile["input"]
                    assert isinstance(input_state, dict)
                    if move_captured_game_mouse(
                            display,
                            int(input_state["mouse_x"]),
                            int(input_state["mouse_y"]), 160, 161):
                        send_mouse_button(display, True, button=1)
                        title_accept_sent = True
                        title_accept_pressing = True
                        print(
                            "state: pressed the native title orb from the "
                            "guest title-idle boundary", flush=True)
                native_gameplay_idle = (
                    bool(last_profile["initialized"])
                    and int(last_profile["execution_enabled"]) == 1
                    and int(last_profile["request"]) == -1
                    and native_gameplay_control_ready(
                        audio_flow, blockers, scene_flow))
                if (drive_authentic_save
                        and not load_menu_requested
                        and int(audio_flow["presentation_mode_27e0"]) & 1 != 0
                        and int(audio_flow["presentation_mode_27e1"]) & 1 != 0
                        and int(scene_flow["active_line"]) == 2
                        and int(scene_flow["c2_presentation_gate"]) == 1):
                    startup_presentation_line_seen = True
                startup_presentation_ready = (
                    drive_authentic_save
                    and not load_menu_requested
                    and int(audio_flow["presentation_mode_27e0"]) & 1 != 0
                    and int(audio_flow["presentation_mode_27e1"]) & 1 != 0
                    and (startup_presentation_pressing
                         or (startup_presentation_line_seen
                             and int(scene_flow["active_line"]) == 0xffff
                             and int(scene_flow["c2_presentation_gate"]) == 0
                             and resource_pipeline_idle(scene_flow))))
                if startup_presentation_ready:
                    write_guest(
                        mem, guest_base, game + MOUSE_X_OFFSET,
                        struct.pack("<h", 110))
                    write_guest(
                        mem, guest_base, game + MOUSE_Y_OFFSET,
                        struct.pack("<h", 96))
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRESS_PENDING_OFFSET, b"\x01")
                    if not startup_presentation_pressing:
                        print(
                            "state: accepting the native startup "
                            "presentation", flush=True)
                    startup_presentation_pressing = True
                elif startup_presentation_pressing:
                    write_guest(
                        mem, guest_base,
                        game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x00")
                    startup_presentation_pressing = False
                if (manual and not load_menu_requested
                        and (profile_releaseable(last_profile)
                             if not trigger_pterra_after_load
                             else native_gameplay_idle)):
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

                flow = last_profile["scene_flow"]
                assert isinstance(flow, dict)
                now = time.monotonic()
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
                        print(
                            "state: pressed authentic save slot 1",
                            flush=True)
                    load_selection_started = True
                    load_slot_pressing = True
                elif load_slot_pressing:
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
                if post_load_pressing and post_load_release_at is not None:
                    if now >= post_load_release_at:
                        write_guest(
                            mem, guest_base,
                            game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x00")
                        post_load_pressing = False
                        post_load_release_at = None
                    else:
                        write_guest(
                            mem, guest_base, game + MOUSE_X_OFFSET,
                            struct.pack("<h", 110))
                        write_guest(
                            mem, guest_base, game + MOUSE_Y_OFFSET,
                            struct.pack("<h", 96))
                        write_guest(
                            mem, guest_base,
                            game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                elif (trigger_pterra_after_load
                        and load_selection_started
                        and not pterra_nav_chart_started
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
                    if (drive_authentic_save
                            and not post_load_dismiss_input_sent):
                        write_guest(
                            mem, guest_base, game + MOUSE_X_OFFSET,
                            struct.pack("<h", 110))
                        write_guest(
                            mem, guest_base, game + MOUSE_Y_OFFSET,
                            struct.pack("<h", 96))
                        write_guest(
                            mem, guest_base,
                            game + MOUSE_PRIMARY_PRESSED_OFFSET, b"\x01")
                        write_guest(
                            mem, guest_base,
                            game + MOUSE_PRESS_PENDING_OFFSET, b"\x01")
                        post_load_pressing = True
                        post_load_release_at = now + 0.25
                        post_load_dismiss_input_sent = True
                        print(
                            "state: dismissed post-load presentation",
                            flush=True)
                if (trigger_pterra_after_load
                        and authentic_save_loaded
                        and not pterra_unlock_requested
                        and pterra_destination_ready(blockers, flow)):
                    pterra_unlock_setup = request_script2_pterra_unlock(
                        mem, guest_base, game_segment)
                    pterra_unlock_requested = True
                    before = pterra_unlock_setup["before"]
                    assert isinstance(before, dict)
                    if script2_pterra_unlock_completed(before):
                        pterra_unlock_completed = True
                        pterra_unlock_setup["after"] = before
                        print(
                            "state: Pterra was already enabled by the save",
                            flush=True)
                    else:
                        print(
                            "state: submitted SCRIPT2 init predicate; "
                            "waiting for the VM to enable Pterra",
                            flush=True)
                elif (trigger_pterra_after_load
                        and pterra_unlock_requested
                        and not pterra_unlock_completed):
                    unlock_context = read_script2_pterra_context(
                        mem, guest_base, game_segment)
                    if script2_pterra_unlock_completed(unlock_context):
                        pterra_unlock_completed = True
                        assert isinstance(pterra_unlock_setup, dict)
                        pterra_unlock_setup["after"] = unlock_context
                        print(
                            "state: SCRIPT2 init enabled Pterra and disabled "
                            "itself", flush=True)
                if (trigger_pterra_after_load
                        and authentic_save_loaded
                        and pterra_unlock_completed
                        and not pterra_nav_chart_started
                        and pterra_destination_ready(blockers, flow)):
                    if (guest_snapshot is not None
                            and not guest_snapshot_written):
                        guest_snapshot.parent.mkdir(parents=True, exist_ok=True)
                        guest_snapshot.write_bytes(read_guest(
                            mem, guest_base, game, 0x10000))
                        guest_snapshot_written = True
                        print(
                            f"state: wrote matched post-load idle snapshot "
                            f"{guest_snapshot}", flush=True)
                    pterra_map_setup = prepare_native_nav_chart(
                        mem, guest_base, game_segment)
                    pterra_map_setup["pointer_recapture"] = (
                        recapture_game_mouse(
                            display, executable, toggle_mouse_capture))
                    pterra_nav_chart_started = True
                    pterra_nav_open_started_at = now
                    pterra_map_last_progress_at = now
                    ivt_baseline = read_guest(mem, guest_base, 0, 0x400)
                    print(
                        "state: driving the native bridge navigation station",
                        flush=True)

                input_state = last_profile["input"]
                assert isinstance(input_state, dict)
                if (trigger_pterra_after_load
                        and pterra_nav_chart_started
                        and len(transition_cpu_samples)
                        < PTERRA_TRANSITION_SAMPLE_LIMIT
                        and (transition_next_sample_at is None
                             or now >= transition_next_sample_at)):
                    transition_cpu_samples.append({
                        "elapsed_seconds": round(
                            now - capture_started_at, 3),
                        "phase": (
                            "ship-navigation"
                            if pterra_triggered else
                            "map-transition"
                            if pterra_map_command_consumed else
                            "nav-chart"),
                        "cpu": state.copy(),
                        "bridge": {
                            "panorama_frame": int(input_state[
                                "bridge_panorama_frame"]),
                            "ui_state": int(input_state[
                                "bridge_ui_state"]),
                            "seek_target_arc": int(input_state[
                                "bridge_seek_target_arc"]),
                            "seek_initial_distance": int(input_state[
                                "bridge_seek_initial_distance"]),
                            "turn_direction": int(input_state[
                                "bridge_turn_direction"]),
                        },
                        "timing": {
                            "timer_tick": int(audio_flow["timer_tick"]),
                            "frame_delay": int(audio_flow["frame_delay"]),
                        },
                        "graphics_pointers": graphics_pointers,
                        "manu3": read_manu3_runtime_state(
                            mem, guest_base, game_segment, state),
                    })
                    transition_next_sample_at = (
                        now + PTERRA_TRANSITION_SAMPLE_INTERVAL_SECONDS)
                if (trigger_pterra_after_load
                        and pterra_nav_chart_started
                        and not pterra_nav_chart_active
                        and int(input_state["nav_camera_view_active"]) == 0
                        and int(input_state["nav_camera_view_state"]) == 0):
                    panorama_frame = int(
                        input_state["bridge_panorama_frame"])
                    if pterra_bridge_panorama_frame is None:
                        pterra_bridge_panorama_frame = panorama_frame
                    elif panorama_frame != pterra_bridge_panorama_frame:
                        pterra_bridge_panorama_frame = panorama_frame
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["last_panorama_frame"] = \
                            panorama_frame
                    bridge_stations = input_state["bridge_stations"]
                    assert isinstance(bridge_stations, list)
                    station = bridge_stations[0]
                    assert isinstance(station, dict)
                    station_rect = tuple(station["hit_rect"])
                    station_center = selectable_rect_center(station_rect)
                    pointer_position = (
                        int(input_state["mouse_x"]),
                        int(input_state["mouse_y"]))
                    if pointer_position != pterra_nav_pointer_last_position:
                        pterra_nav_pointer_last_position = pointer_position
                        pterra_nav_pointer_last_changed_at = now
                    elif (pterra_nav_station_target_y is not None
                            and abs(pointer_position[1]
                                    - pterra_nav_station_target_y) > 2
                            and pterra_nav_pointer_last_changed_at is not None
                            and now - pterra_nav_pointer_last_changed_at
                            >= 2.0):
                        if pterra_nav_pointer_recapture_count >= 3:
                            hang_snapshot = snapshot_guest(
                                mem, guest_base, anchor, cpu_addresses,
                                hit, last_profile)
                            hang_snapshot["reason"] = (
                                "captured host pointer did not move the DOS "
                                "cursor toward the bridge station")
                            print(
                                "hang: captured pointer did not move toward "
                                "the bridge station", flush=True)
                            break
                        recapture = recapture_game_mouse(
                            display, executable, toggle_mouse_capture)
                        pterra_nav_pointer_recapture_count += 1
                        pterra_nav_pointer_last_changed_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup.setdefault(
                            "pointer_liveness_recaptures", []).append(
                                recapture)
                    if pterra_nav_station_pressing:
                        send_mouse_button(display, False, button=1)
                        pterra_nav_station_pressing = False
                    elif station_center is not None \
                            and int(station["flags"]) & 1:
                        current_x = int(input_state["mouse_x"])
                        current_y = int(input_state["mouse_y"])
                        pterra_nav_station_target_y = station_center[1]
                        if abs(current_y - station_center[1]) > 2:
                            move_captured_game_mouse(
                                display, current_x, current_y,
                                current_x, station_center[1])
                        else:
                            station_x = station_center[0]
                            close_enough = abs(current_x - station_x) <= 32
                            moved = move_captured_game_mouse(
                                display, current_x, current_y,
                                station_x, station_center[1])
                            if close_enough or moved:
                                send_mouse_button(display, True, button=1)
                                pterra_nav_station_pressing = True
                                pterra_nav_station_click_count += 1
                                if pterra_nav_station_first_click_at is None:
                                    pterra_nav_station_first_click_at = now
                                pterra_map_last_progress_at = now
                                assert isinstance(pterra_map_setup, dict)
                                pterra_map_setup["bridge_station_rect"] = \
                                    list(station_rect)
                                pterra_map_setup[
                                    "bridge_station_click_count"] = \
                                    pterra_nav_station_click_count
                                pterra_map_setup[
                                    "bridge_station_click_evidence"] = {
                                        "adapter": "host-primary-edge",
                                        "point": [
                                            station_x, station_center[1]],
                                    }
                                print(
                                    "state: pressed native bridge navigation "
                                    f"station at {station_x},"
                                    f"{station_center[1]}", flush=True)
                    else:
                        # Bring station zero into view through the bridge's
                        # own edge-pan behavior. Its handler owns the complete
                        # chart-open and chart-close actor state.
                        move_captured_game_mouse(
                            display,
                            int(input_state["mouse_x"]),
                            int(input_state["mouse_y"]), 2,
                            pterra_nav_station_target_y
                            if pterra_nav_station_target_y is not None
                            else 100)

                if (trigger_pterra_after_load
                        and pterra_nav_chart_started
                        and not pterra_nav_chart_active
                        and int(input_state["nav_camera_view_state"]) == 0
                        and int(input_state["nav_center_wipe_complete"]) & 1
                        and int(input_state["nav_camera_view_active"]) & 1
                        and SCRIPT2_PTERRA_RECORD in input_state[
                            "nav_chart_object_offsets"]):
                    pterra_nav_chart_active = True
                    # A station press can remain down for one sampled frame as
                    # the chart takes input ownership.  End that edge before
                    # moving; carrying it over a chart object can close the
                    # chart instead of selecting the object.
                    if pterra_nav_station_pressing:
                        send_mouse_button(display, False, button=1)
                        pterra_nav_station_pressing = False
                        print(
                            "state: released bridge-station press at the "
                            "nav-chart handoff", flush=True)
                    pterra_host_mouse_ready_at = now + 0.25
                    pterra_map_last_progress_at = now
                    assert isinstance(pterra_map_setup, dict)
                    pterra_map_setup["chart_object_offsets"] = list(
                        input_state["nav_chart_object_offsets"])
                    print(
                            "state: native nav chart is interactive and contains "
                            "Pterra", flush=True)

                if (trigger_pterra_after_load
                        and pterra_nav_chart_active
                        and not pterra_nav_chart_selected
                        and not pterra_nav_station_pressing
                        and not pterra_nav_chart_pressing
                        and int(input_state[
                            "nav_camera_view_active"]) == 0):
                    pterra_nav_chart_reopen_count += 1
                    if (pterra_nav_chart_reopen_count
                            > PTERRA_NAV_CHART_MAX_REOPEN_ATTEMPTS):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "native nav chart repeatedly closed during its "
                            "station-input handoff")
                        print(
                            "hang: nav chart repeatedly closed during "
                            "station-input handoff", flush=True)
                        break
                    pterra_nav_chart_active = False
                    pterra_nav_station_first_click_at = None
                    pterra_nav_open_started_at = now
                    pterra_map_last_progress_at = now
                    pterra_host_mouse_ready_at = None
                    assert isinstance(pterra_map_setup, dict)
                    pterra_map_setup["chart_reopen_count"] = \
                        pterra_nav_chart_reopen_count
                    print(
                        "state: reopening nav chart after station-input "
                        "handoff closed it", flush=True)

                if (trigger_pterra_after_load
                        and pterra_nav_chart_active
                        and not pterra_nav_chart_selected
                        and pterra_map_last_progress_at is not None
                        and now - pterra_map_last_progress_at
                        >= PTERRA_MAP_TRANSITION_TIMEOUT_SECONDS):
                    hang_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    hang_snapshot["reason"] = (
                        "native nav chart did not select Pterra after it "
                        "became interactive")
                    print(
                        "hang: interactive nav chart did not select Pterra",
                        flush=True)
                    break

                if (trigger_pterra_after_load
                        and pterra_nav_chart_started
                        and not pterra_nav_chart_active
                        and pterra_nav_open_started_at is not None
                        and pterra_map_last_progress_at is not None
                        and bridge_navigation_timed_out(
                            now, pterra_nav_open_started_at,
                            pterra_map_last_progress_at,
                            pterra_nav_station_first_click_at)):
                    hang_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    hang_snapshot["reason"] = (
                        "native bridge navigation station did not open the "
                        "chart")
                    print(
                        "hang: native bridge navigation station did not open "
                        "the chart", flush=True)
                    break

                if (trigger_pterra_after_load
                        and pterra_nav_chart_active
                        and not pterra_map_command_consumed):
                    map_context = read_script2_pterra_context(
                        mem, guest_base, game_segment)
                    marker_x, marker_y = map_context["pterra_marker"]
                    selected_location = int(
                        input_state["nav_selected_location_record"])
                    if pterra_nav_station_pressing:
                        raise RuntimeError(
                            "bridge-station press crossed the nav-chart "
                            "handoff without being released")
                    elif pterra_nav_chart_pressing:
                        send_mouse_button(display, False, button=1)
                        pterra_nav_chart_pressing = False
                    elif (not pterra_nav_chart_selected
                            and selected_location
                            == SCRIPT2_PTERRA_RECORD):
                        pterra_nav_chart_selected = True
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["selected_context"] = map_context
                        print(
                            "state: native nav chart selected Pterra",
                            flush=True)
                    elif (not pterra_nav_chart_selected
                            and not pterra_map_command_generated
                            and not pterra_nav_chart_pressing
                            and pterra_host_mouse_ready_at is not None
                            and now >= pterra_host_mouse_ready_at
                            and selected_location == 0
                            and int(input_state[
                                "nav_camera_view_active"]) & 1
                            and int(input_state[
                                "nav_camera_view_state"]) == 0):
                        if move_captured_game_mouse(
                                display,
                                int(input_state["mouse_x"]),
                                int(input_state["mouse_y"]),
                                marker_x, marker_y):
                            send_mouse_button(display, True, button=1)
                            pterra_nav_chart_pressing = True
                            assert isinstance(pterra_map_setup, dict)
                            pterra_map_setup["pterra_marker"] = [
                                marker_x, marker_y]
                            print(
                                "state: pressed Pterra at its native "
                                f"nav-chart marker {marker_x},{marker_y}",
                                flush=True)

                    if pterra_nav_panel_close_pressing:
                        send_mouse_button(display, False, button=1)
                        pterra_nav_panel_close_pressing = False
                    elif (pterra_nav_chart_selected
                            and not pterra_nav_panel_close_requested
                            and selected_location == SCRIPT2_PTERRA_RECORD
                            and int(input_state[
                                "nav_location_panel_active"]) & 1
                            and int(input_state[
                                "nav_location_panel_transition_state"]) == 0):
                        bridge_stations = input_state["bridge_stations"]
                        assert isinstance(bridge_stations, list)
                        travel_station = bridge_stations[5]
                        assert isinstance(travel_station, dict)
                        travel_rect = tuple(travel_station["hit_rect"])
                        travel_center = selectable_rect_center(travel_rect)
                        if (travel_center is not None
                                and int(travel_station["flags"]) & 1
                                and move_captured_game_mouse(
                                    display,
                                    int(input_state["mouse_x"]),
                                    int(input_state["mouse_y"]),
                                    travel_center[0], travel_center[1])):
                            send_mouse_button(display, True, button=1)
                            pterra_nav_panel_close_pressing = True
                            pterra_nav_panel_close_requested = True
                            pterra_map_last_progress_at = now
                            assert isinstance(pterra_map_setup, dict)
                            pterra_map_setup[
                                "travel_confirmation_station_index"] = 5
                            pterra_map_setup[
                                "travel_confirmation_station_rect"] = \
                                list(travel_rect)
                            print(
                                "state: confirmed Pterra through native "
                                "travel station 5", flush=True)
                    if (pterra_nav_panel_close_requested
                            and not pterra_nav_panel_close_confirmed
                            and int(input_state[
                                "nav_location_panel_active"]) == 0
                            and int(input_state[
                                "nav_location_panel_transition_state"]) == 0):
                        pterra_nav_panel_close_confirmed = True
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["panel_close_confirmed"] = True
                        print(
                            "state: native Pterra location panel closed",
                            flush=True)

                    expected_map_c1 = (
                        BLOODPRG_VM_RECORD_C1, SCRIPT2_PTERRA_RECORD, 0)
                    arche_action = tuple(map_context["arche_action"])
                    deferred_map_c1 = (
                        int(flow["deferred_record_type"]),
                        int(flow["deferred_record_related"]),
                        int(flow["deferred_record_value"]),
                    )
                    if (not pterra_map_command_generated
                            and (arche_action == expected_map_c1
                                 or deferred_map_c1 == expected_map_c1)):
                        pterra_map_command_generated = True
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["generated_context"] = map_context
                        pterra_map_setup["generated_via"] = (
                            "arche-action" if arche_action == expected_map_c1
                            else "deferred-record")
                        print(
                            "state: native nav chart generated the arche "
                            "Pterra C1 command", flush=True)
                    if (pterra_map_command_generated
                            and not pterra_map_command_consumed
                            and int(map_context["current_location"])
                            == SCRIPT2_PTERRA_RECORD
                            and arche_action[0] == 0
                            and int(flow["deferred_record_type"]) == 0):
                        pterra_map_command_consumed = True
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["consumed_context"] = map_context
                        print(
                            "state: native VM consumed the arche Pterra C1 "
                            "command", flush=True)

                if (trigger_pterra_after_load
                        and pterra_map_command_consumed
                        and not pterra_map_destination_committed):
                    map_context = read_script2_pterra_context(
                        mem, guest_base, game_segment)
                    if (int(map_context["current_location"])
                            == SCRIPT2_PTERRA_RECORD
                            and int(input_state[
                                "nav_camera_view_active"]) == 0
                            and int(input_state[
                                "nav_camera_view_state"]) == 0
                            and pterra_destination_ready(blockers, flow)):
                        pterra_map_destination_committed = True
                        pterra_map_last_progress_at = now
                        assert isinstance(pterra_map_setup, dict)
                        pterra_map_setup["arrival_context"] = map_context
                        print(
                            "state: native map travel returned to the bridge "
                            "at Pterra", flush=True)
                    elif (pterra_map_last_progress_at is not None
                            and now - pterra_map_last_progress_at
                            >= PTERRA_MAP_TRANSITION_TIMEOUT_SECONDS):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "native Pterra map transition made no semantic "
                            "progress")
                        print(
                            "hang: native Pterra map transition made no "
                            "semantic progress", flush=True)
                        break

                if (trigger_pterra_after_load
                        and pterra_map_destination_committed
                        and not pterra_triggered
                        and pterra_destination_ready(blockers, flow)):
                    audio_flow = last_profile["audio_flow"]
                    assert isinstance(audio_flow, dict)
                    scruter_streamed_clip_count_before = int(
                        audio_flow["streamed_clip_count"])
                    pterra_travel_setup = prepare_script2_orxx_descent(
                        mem, guest_base, game_segment)
                    intro_recapture = recapture_game_mouse(
                        display, executable, toggle_mouse_capture)
                    pterra_ship_intro_input_evidence = {
                        "adapter": "host-secondary-edge",
                        "recapture": intro_recapture,
                    }
                    pterra_ship_intro_capture_ready_at = now
                    pterra_travel_setup[
                        "intro_input_recapture"] = intro_recapture
                    pterra_triggered = True
                    pterra_ship_navigation_started_at = now
                    print(
                        "state: driving native ship navigation through the "
                        "current-location entity; waiting for the Pterra "
                        "target from Orxx record "
                        f"{pterra_travel_setup['orxx_offset']:#06x}",
                        flush=True)

                if (trigger_pterra_after_load
                        and pterra_triggered
                        and not pterra_ship_navigation_activated):
                    input_state = last_profile["input"]
                    assert isinstance(input_state, dict)
                    ship_active_flags = int(blockers["ship"])
                    if ship_active_flags != 0:
                        pterra_ship_navigation_activated = True
                        pterra_ship_hud_last_progress_at = now
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup["ship_active_flags_observed"] = \
                            ship_active_flags
                        pterra_travel_setup["entity_click_count"] = \
                            pterra_ship_region_click_count
                        print(
                            "state: native current-location interaction "
                            "activated the ship presentation",
                            flush=True)
                    else:
                        entity = input_state["current_location_entity"]
                        assert isinstance(entity, dict)
                        entity_rect = tuple(entity["hit_rect"])
                        entity_center = selectable_rect_center(entity_rect)
                        stations = input_state["bridge_stations"]
                        assert isinstance(stations, list)
                        station_flags = int(stations[3]["flags"])
                        if pterra_ship_region_pressing:
                            send_mouse_button(display, False, button=1)
                            pterra_ship_region_pressing = False
                            pterra_ship_region_next_press_at = now + 0.5
                        elif (now >= pterra_ship_region_next_press_at
                                and entity_center is not None
                                and int(entity["flags"]) & 1
                                and station_flags & 1
                                and move_captured_game_mouse(
                                    display,
                                    int(input_state["mouse_x"]),
                                    int(input_state["mouse_y"]),
                                    entity_center[0], entity_center[1])):
                            send_mouse_button(display, True, button=1)
                            pterra_ship_region_pressing = True
                            pterra_ship_region_click_count += 1
                            assert isinstance(pterra_travel_setup, dict)
                            pterra_travel_setup["entity_rect"] = \
                                list(entity_rect)
                            pterra_travel_setup["actor_slot_index"] = 3
                            print(
                                "state: pressed native current-location "
                                f"entity at {entity_center[0]},"
                                f"{entity_center[1]}", flush=True)
                        if (pterra_ship_navigation_started_at is not None
                                and now - pterra_ship_navigation_started_at
                                >= PTERRA_MAP_TRANSITION_TIMEOUT_SECONDS):
                            hang_snapshot = snapshot_guest(
                                mem, guest_base, anchor, cpu_addresses,
                                hit, last_profile)
                            hang_snapshot["reason"] = (
                                "native current-location interaction did not "
                                "activate ship navigation")
                            print(
                                "hang: native current-location interaction "
                                "did not activate ship navigation",
                                flush=True)
                            break

                if (trigger_pterra_after_load
                        and pterra_ship_navigation_activated):
                    active_line = int(flow["active_line"])
                    if (int(blockers["ship"]) & 2
                            and active_line in (4, 5)
                            and active_line not in pterra_ship_intro_lines_seen):
                        pterra_ship_intro_lines_seen.append(active_line)
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup["intro_lines_seen"] = list(
                            pterra_ship_intro_lines_seen)
                        print(
                            "state: native ship intro reached dialogue line "
                            f"{active_line}", flush=True)
                    intro_latch_active = bool(
                        int(input_state["secondary_pressed"]) & 1
                        or int(input_state["press_pending"]) & 1)
                    if intro_latch_active:
                        pterra_ship_intro_latch_seen = True
                    if int(input_state["mouse_button_state"]) & 2:
                        pterra_ship_intro_raw_seen = True

                    intro_waiting_for_input = (
                        pterra_ship_intro_waiting_for_input(
                            blockers, flow, input_state))
                    if intro_waiting_for_input:
                        pterra_ship_intro_hold_observed = True
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup["intro_hold_observed"] = True
                        if pterra_ship_intro_capture_ready_at is None:
                            recapture = recapture_game_mouse(
                                display, executable,
                                toggle_mouse_capture)
                            pterra_ship_intro_input_evidence = {
                                "adapter": "host-secondary-edge",
                                "recapture": recapture,
                            }
                            pterra_ship_intro_capture_ready_at = now
                            assert isinstance(pterra_travel_setup, dict)
                            pterra_travel_setup[
                                "intro_input_recapture"] = recapture

                    hold_consumed, hold_resolution = (
                        pterra_ship_intro_consumed_before_expiry(
                            flow, input_state,
                            pterra_ship_intro_lines_seen,
                            pterra_ship_intro_edge_count,
                            pterra_ship_intro_raw_seen,
                            pterra_ship_intro_latch_seen))
                    if not pterra_ship_intro_dismissed and hold_consumed:
                        pterra_ship_intro_dismissed = True
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup.update({
                            "intro_hold_dismissed": True,
                            "intro_hold_resolution": hold_resolution,
                            "intro_input_evidence":
                                pterra_ship_intro_input_evidence,
                            "intro_input_edges": pterra_ship_intro_edge_count,
                            "intro_raw_secondary_seen":
                                pterra_ship_intro_raw_seen,
                            "intro_guest_latch_seen":
                                pterra_ship_intro_latch_seen,
                            "intro_hold_countdown_after": int(
                                flow["dialogue_hold_countdown"]),
                        })
                        print(
                            "state: guest cleared the native ship-intro "
                            "hold before its countdown expired", flush=True)
                        if pterra_ship_intro_pressing:
                            send_mouse_button(display, False, button=3)
                            pterra_ship_intro_pressing = False
                            pterra_ship_intro_press_started_at = None
                    elif (not pterra_ship_intro_dismissed
                            and intro_waiting_for_input):
                        if (pterra_ship_intro_started_at is not None
                                and now - pterra_ship_intro_started_at
                                >= PTERRA_NATIVE_INPUT_TIMEOUT_SECONDS):
                            hang_snapshot = snapshot_guest(
                                mem, guest_base, anchor, cpu_addresses,
                                hit, last_profile)
                            hang_snapshot["reason"] = (
                                "guest did not consume bounded secondary "
                                "button edges before the ship-intro hold "
                                "countdown expired; latch_seen="
                                f"{pterra_ship_intro_latch_seen}, attempts="
                                f"{pterra_ship_intro_edge_count}")
                            print(
                                "hang: guest did not consume bounded ship-"
                                "intro secondary edges", flush=True)
                            break
                        if (int(flow["dialogue_hold_countdown"]) == 0
                                and not pterra_ship_intro_dismissed):
                            hang_snapshot = snapshot_guest(
                                mem, guest_base, anchor, cpu_addresses,
                                hit, last_profile)
                            hang_snapshot["reason"] = (
                                "ship-intro hold expired before a verified "
                                "secondary-button dismissal")
                            print(
                                "hang: ship-intro hold expired without a "
                                "verified secondary edge", flush=True)
                            break
                        release_ready = (
                            pterra_ship_intro_press_should_release(
                                now,
                                pterra_ship_intro_press_started_at))
                        can_press = (
                            pterra_ship_intro_edge_count
                            < PTERRA_NATIVE_INPUT_MAX_EDGES
                            and pterra_ship_intro_ready_for_edge(flow)
                            and pterra_ship_intro_capture_ready_at is not None
                            and now >= pterra_ship_intro_capture_ready_at
                            and now >= pterra_ship_intro_next_edge_at)
                        input_action = pterra_ship_intro_input_action(
                            pterra_ship_intro_pressing,
                            release_ready,
                            intro_latch_active,
                            can_press)
                        if input_action == "release":
                            send_mouse_button(display, False, button=3)
                            pterra_ship_intro_pressing = False
                            pterra_ship_intro_press_started_at = None
                            pterra_ship_intro_next_edge_at = (
                                now
                                + PTERRA_NATIVE_INPUT_EDGE_INTERVAL_SECONDS)
                        elif input_action == "press":
                            if pterra_ship_intro_started_at is None:
                                pterra_ship_intro_started_at = now
                            assert isinstance(pterra_travel_setup, dict)
                            pterra_travel_setup.setdefault(
                                "intro_hold_countdown_before",
                                int(flow["dialogue_hold_countdown"]))
                            send_mouse_button(display, True, button=3)
                            pterra_ship_intro_pressing = True
                            pterra_ship_intro_press_started_at = now
                            pterra_ship_intro_edge_count += 1
                            print(
                                "state: sent native ship-intro secondary "
                                f"edge {pterra_ship_intro_edge_count}",
                                flush=True)
                    if (not pterra_ship_intro_dismissed
                            and not pterra_ship_intro_completed_naturally
                            and not pterra_ship_intro_hold_observed
                            and pterra_ship_intro_is_naturally_complete(
                                blockers, flow, input_state,
                                pterra_ship_intro_lines_seen,
                                pterra_ship_intro_edge_count)):
                        pterra_ship_intro_completed_naturally = True
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup.update({
                            "intro_hold_observed": False,
                            "intro_completed_naturally": True,
                            "intro_input_edges": 0,
                            "intro_selector_phase_on_completion": int(
                                input_state["ship_target_select_phase"]),
                        })
                        print(
                            "state: native ship intro completed through its "
                            "countdown without an injected edge", flush=True)

                    intro_resolved = (
                        pterra_ship_intro_dismissed
                        or pterra_ship_intro_completed_naturally)
                    if (pterra_ship_intro_hold_observed
                            and not intro_resolved
                            and int(input_state[
                                "ship_hud_initialized"]) & 1):
                        if pterra_ship_intro_pressing:
                            send_mouse_button(display, False, button=3)
                            pterra_ship_intro_pressing = False
                            pterra_ship_intro_press_started_at = None
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "observed ship-intro hold reached the HUD without "
                            "a verified guest dismissal")
                        print(
                            "hang: observed ship-intro hold reached the HUD "
                            "without verified dismissal", flush=True)
                        break
                    if (intro_resolved
                            and int(blockers["ship"]) & 4
                            and int(input_state["ship_hud_initialized"]) & 1):
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup[
                            "hud_initialized_after_intro_dismissal"] = True
                        if int(flow["text_display_active"]) & 1:
                            pterra_travel_setup[
                                "hud_text_active_after_intro_dismissal"] = True

                    if intro_resolved:
                        target_phase = int(
                            input_state["ship_target_select_phase"])
                        if (not pterra_ship_target_phases_seen
                                or pterra_ship_target_phases_seen[-1]
                                != target_phase):
                            pterra_ship_target_phases_seen.append(target_phase)
                            assert isinstance(pterra_travel_setup, dict)
                            pterra_travel_setup[
                                "target_select_phases_seen"] = list(
                                    pterra_ship_target_phases_seen)

                    ship_hud_progress_key = (
                        int(flow["active_line"]),
                        int(flow["c2_presentation_gate"]),
                        int(flow["text_display_active"]),
                        int(flow["text_reveal_cursor"]),
                        int(flow["displayed_line"]),
                        int(flow["presentation_owner_offset"]),
                        int(flow["list_d8c_state"]),
                        int(flow["resource_source_remaining"]),
                        int(flow["list_queued_bytes"]),
                        int(flow["dialogue_hold_complete"]),
                        int(input_state["ship_hud_initialized"]),
                        int(input_state["ship_target_select_phase"]),
                        int(input_state["ship_scene_dispatch_blocked"]),
                    )
                    if ship_hud_progress_key != pterra_ship_hud_progress_key:
                        pterra_ship_hud_progress_key = ship_hud_progress_key
                        pterra_ship_hud_last_progress_at = now
                    elif (not pterra_travel_command_generated
                            and pterra_ship_hud_last_progress_at is not None
                            and now - pterra_ship_hud_last_progress_at
                            >= PTERRA_TRAVEL_MOVIE_TIMEOUT_SECONDS):
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "native ship HUD made no semantic progress before "
                            "target selection")
                        print(
                            "hang: native ship HUD made no semantic progress "
                            "before target selection", flush=True)
                        break

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

                if trigger_pterra_after_load and pterra_triggered:
                    travel_context = read_script2_pterra_context(
                        mem, guest_base, game_segment)
                    orxx_action = tuple(travel_context["orxx_action"])
                    input_state = last_profile["input"]
                    assert isinstance(input_state, dict)
                    target_name_offsets = input_state[
                        "ship_target_name_offsets"]
                    assert isinstance(target_name_offsets, list)
                    pterra_name_offset = SCRIPT2_PTERRA_RECORD + 4
                    if pterra_target_pressing:
                        send_mouse_button(display, False, button=1)
                        pterra_target_pressing = False
                    elif (not pterra_travel_command_generated
                            and pterra_name_offset in target_name_offsets
                            and int(input_state["ship_hud_initialized"]) & 1
                            and int(input_state[
                                "ship_target_select_phase"]) == 0):
                        row = target_name_offsets.index(pterra_name_offset)
                        rect = tuple(input_state["choice_rect"])
                        if len(rect) != 4 or int(rect[2]) <= 0 \
                                or int(rect[3]) <= 0:
                            raise RuntimeError(
                                "ship target list has no selectable rectangle")
                        x, y = choice_row_point(rect, row)
                        current_x = int(input_state["mouse_x"])
                        current_y = int(input_state["mouse_y"])
                        click_evidence = None
                        if not guest_mouse_point_is_valid(
                                current_x, current_y):
                            click_evidence = inject_guest_primary_click(
                                mem, guest_base, game_segment, x, y)
                        elif move_captured_game_mouse(
                                display, current_x, current_y, x, y):
                            send_mouse_button(display, True, button=1)
                            click_evidence = {
                                "adapter": "host-primary-edge",
                                "point": [x, y],
                            }
                        if click_evidence is not None:
                            if pterra_target_row is None:
                                pterra_target_row = row
                                assert isinstance(pterra_travel_setup, dict)
                                pterra_travel_setup["target_name_offsets"] = \
                                    list(target_name_offsets)
                                pterra_travel_setup[
                                    "pterra_target_row"] = row
                                pterra_travel_setup[
                                    "target_click_evidence"] = \
                                    click_evidence
                                print(
                                    "state: selected Pterra row "
                                    f"{row} in the native ship target list",
                                    flush=True)
                            pterra_target_pressing = True
                    expected_c1 = (
                        BLOODPRG_VM_RECORD_C1, SCRIPT2_PTERRA_RECORD, 0)
                    if (not pterra_travel_command_generated
                            and orxx_action == expected_c1):
                        pterra_travel_command_generated = True
                        pterra_arrival_last_progress_at = time.monotonic()
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup["generated_context"] = \
                            travel_context
                        pterra_travel_setup["pterra_target_row"] = \
                            pterra_target_row
                        print(
                            "state: native ship HUD generated the Orxx Pterra "
                            "C1 command", flush=True)
                    arrival_progress_key = (
                        int(blockers["render"]),
                        int(flow["active_line"]),
                        int(flow["c2_presentation_gate"]),
                        int(flow["deferred_record_type"]),
                        int(flow["deferred_record_related"]),
                        int(flow["deferred_record_value"]),
                        int(flow["sequence_active"]),
                        int(flow["ship_current_target"]),
                        int(blockers["ship"]),
                        *orxx_action,
                    )
                    if arrival_progress_key != pterra_arrival_progress_key:
                        pterra_arrival_progress_key = arrival_progress_key
                        pterra_arrival_last_progress_at = time.monotonic()
                    if (pterra_travel_command_generated
                            and not pterra_travel_command_consumed
                            and orxx_action[0] == 0):
                        pterra_travel_command_consumed = True
                        assert isinstance(pterra_travel_setup, dict)
                        pterra_travel_setup["consumed_context"] = travel_context
                        print(
                            "state: native VM scan consumed the Orxx Pterra C1 "
                            "command", flush=True)
                    if (not scruter_scene_requested
                            and flow["deferred_record_type"]
                            == BLOODPRG_VM_RECORD_C4
                            and flow["deferred_record_related"]
                            == SCRIPT2_SCRUTER_JO_RECORD):
                        scruter_scene_requested = True
                        print(
                            "state: native travel queued Scruter_Jo C4",
                            flush=True)

                if (trigger_pterra_after_load
                        and pterra_travel_command_consumed
                        and not destination_committed
                        and pterra_encounter_idle(
                            blockers, flow, pter_choice_results)):
                    assert isinstance(pterra_travel_setup, dict)
                    context = read_script2_pterra_context(
                        mem, guest_base, game_segment)
                    pterra_travel_setup["arrival_context"] = context
                    if context["current_location"] != SCRIPT2_PTERRA_RECORD:
                        hang_snapshot = snapshot_guest(
                            mem, guest_base, anchor, cpu_addresses,
                            hit, last_profile)
                        hang_snapshot["reason"] = (
                            "native Pterra travel returned idle without "
                            "committing arche.current_location")
                        print(
                            "hang: native Pterra travel did not commit the "
                            "destination", flush=True)
                        break
                    destination_committed = True
                    print(
                        "state: native Pterra arrival returned to idle with "
                        "the destination committed", flush=True)

                if (trigger_pterra_after_load
                        and pterra_travel_command_consumed
                        and not destination_committed
                        and not pter_reached
                        and pterra_arrival_last_progress_at is not None
                        and time.monotonic() - pterra_arrival_last_progress_at
                        >= (PTERRA_TRAVEL_MOVIE_TIMEOUT_SECONDS
                            if (flow["active_line"] == 3
                                and blockers["ship"] & 8)
                            else 30.0)):
                    stall_seconds = (
                        PTERRA_TRAVEL_MOVIE_TIMEOUT_SECONDS
                        if (flow["active_line"] == 3 and blockers["ship"] & 8)
                        else 30.0)
                    hang_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile)
                    hang_snapshot["reason"] = (
                        "Pterra arrival made no semantic progress for "
                        f"{stall_seconds:g} seconds")
                    print(
                        "hang: Pterra arrival made no semantic progress for "
                        f"{stall_seconds:g} seconds", flush=True)
                    break

                if (not pter_reached
                        and pterra_travel_command_consumed
                        and blockers["presentation"] != 0
                        and blockers["ship"] != 0
                        and flow["active_line"] != 0xffff):
                    pter_reached = True
                    scruter_scene_active_seen = True
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
                    current_clip_count = int(
                        audio_flow["streamed_clip_count"])
                    if (not scruter_sound_bank_loaded
                            and current_clip_count > 0
                            and current_clip_count
                            != scruter_streamed_clip_count_before):
                        scruter_streamed_clip_count = current_clip_count
                        scruter_sound_bank_loaded = True
                        scruter_scene_active_seen = True
                        print(
                            "state: Pterra actor transition loaded "
                            f"{scruter_streamed_clip_count} Scruter_Jo streamed "
                            "clips", flush=True)
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
                        assert pter_reached_at is not None
                        if len(pter_semantic_checkpoints) < 256:
                            pter_semantic_checkpoints.append({
                                "seconds_after_entry": round(
                                    now - pter_reached_at, 3),
                                "active_line": flow["active_line"],
                                "presentation": blockers["presentation"],
                                "presentation_defer": (
                                    blockers["presentation_defer"]),
                                "text": blockers["text"],
                                "word_choice_active": (
                                    input_state["word_choice_active"]),
                                "word_choice_phase": (
                                    input_state["word_choice_phase"]),
                                "selected_word": input_state["selected_word"],
                                "last_clip": audio_flow["last_clip"],
                            })

                    word_choice_active = bool(
                        input_state["word_choice_active"] & 1)
                    selected_word = int(input_state["selected_word"])
                    if record_expected_pterra_choice(
                            pter_choice_results, selected_word):
                        print(
                            "state: completed Pterra choice "
                            f"{len(pter_choice_results)} with dictionary "
                            f"word {selected_word:#06x}", flush=True)

                    if pter_input_pressed is not None:
                        if pter_input_pressed == "primary":
                            send_mouse_button(display, False, button=1)
                        pter_input_pressed = None
                    elif now >= pter_next_input_at:
                        if (word_choice_active
                                and (input_state["word_choice_phase"] & 7)
                                == 2):
                            target_rows = (4, 0)
                            target_row = target_rows[min(
                                len(pter_choice_results),
                                len(target_rows) - 1)]
                            point = choice_row_point(
                                tuple(input_state["choice_rect"]),
                                target_row)
                            current_x = int(input_state["mouse_x"])
                            current_y = int(input_state["mouse_y"])
                            input_adapter = None
                            if not guest_mouse_point_is_valid(
                                    current_x, current_y):
                                inject_guest_primary_click(
                                    mem, guest_base, game_segment,
                                    point[0], point[1])
                                input_adapter = "guest-primary"
                            elif move_captured_game_mouse(
                                    display, current_x, current_y,
                                    point[0], point[1]):
                                send_mouse_button(display, True, button=1)
                                input_adapter = "primary"
                            if input_adapter is not None:
                                pter_input_pressed = input_adapter
                                pter_next_input_at = now + 0.25
                                print(
                                    "state: selected Pterra choice row "
                                    f"{target_row + 1} at "
                                    f"{point[0]},{point[1]} via "
                                    f"{input_adapter}",
                                    flush=True)
                        elif (not word_choice_active
                              and blockers["presentation"] != 0):
                            send_mouse_button(display, True, button=1)
                            pter_input_pressed = "primary"
                            pter_next_input_at = now + 0.5

                    if (pter_completed_at is None
                            and len(pter_choice_results) >= 2
                            and destination_committed
                            and blockers["presentation"] == 0):
                        pter_completed_at = now
                        scruter_scene_completed = True
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

                if hit is not None and marker_snapshot is None:
                    marker_snapshot = snapshot_guest(
                        mem, guest_base, anchor, cpu_addresses,
                        hit, last_profile,
                        guest_snapshot.parent
                        if guest_snapshot is not None else None)
                    print(f"boundary marker: {hit.name}", flush=True)
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
        if (fault_snapshot is not None
                or dos_read_overflow_snapshot is not None
                or integrity_fault_snapshot is not None
                or hang_snapshot is not None
                or pter_sustained):
            break
        time.sleep(0.01)

    if pterra_ship_intro_pressing:
        send_mouse_button(display, False, button=3)
        pterra_ship_intro_pressing = False

    overall_timeout_reached = (
        time.monotonic() >= deadline
        and fault_snapshot is None
        and dos_read_overflow_snapshot is None
        and integrity_fault_snapshot is None
        and hang_snapshot is None
        and not pter_sustained
    )
    confirmed_title_evidence = title_transition_evidence(
        startup_presentation_line_seen=startup_presentation_line_seen,
        load_menu_requested=load_menu_requested if open_load_menu else False,
        authentic_save_loaded=authentic_save_loaded,
    )
    title_transition_confirmed = bool(confirmed_title_evidence)
    errors = []
    if overall_timeout_reached:
        errors.append(
            f"scenario exceeded its {timeout:g}-second overall timeout")
    if not manual and not request_written:
        errors.append("SCRIPT2 request boundary was never reached")
    if not manual and not profile_loaded:
        errors.append("SCRIPT2 did not finish loading")
    if open_load_menu and not load_menu_requested:
        errors.append("LOAD-menu request boundary was never reached")
    if drive_authentic_save and not title_transition_confirmed:
        errors.append("native title transition was never confirmed")
    if trigger_pterra_after_load and not authentic_save_loaded:
        errors.append("authentic GAME1.SAV load did not complete")
    if trigger_pterra_after_load and not pterra_unlock_requested:
        errors.append("SCRIPT2 Pterra unlock predicate was never submitted")
    if trigger_pterra_after_load and not pterra_unlock_completed:
        errors.append("SCRIPT2 VM never enabled Pterra through proc init")
    if trigger_pterra_after_load and not pterra_nav_chart_started:
        errors.append("native nav-chart opening was never started")
    if trigger_pterra_after_load and not pterra_nav_chart_active:
        errors.append("native nav chart never exposed Pterra")
    if trigger_pterra_after_load and not pterra_nav_chart_selected:
        errors.append("native nav chart never selected Pterra")
    if trigger_pterra_after_load and not pterra_nav_panel_close_confirmed:
        errors.append("native Pterra location panel never completed")
    if trigger_pterra_after_load and not pterra_map_command_generated:
        errors.append("native nav chart never generated the arche Pterra C1 command")
    if trigger_pterra_after_load and not pterra_map_command_consumed:
        errors.append("native VM never consumed the arche Pterra C1 command")
    if trigger_pterra_after_load and not pterra_map_destination_committed:
        errors.append("native map travel never returned to the bridge at Pterra")
    if trigger_pterra_after_load and not pterra_triggered:
        errors.append("native ship-navigation flow was never activated")
    if trigger_pterra_after_load and not pterra_ship_navigation_activated:
        errors.append(
            "native current-location interaction never activated ship navigation")
    if (trigger_pterra_after_load
            and pterra_travel_setup is not None
            and int(pterra_travel_setup.get("pterra_access_count_before", 0)) == 0
            and not (pterra_ship_intro_dismissed
                     or pterra_ship_intro_completed_naturally)):
        errors.append(
            "final first-visit ship intro neither consumed its observed hold "
            "nor completed without a hold")
    if trigger_pterra_after_load and not pterra_travel_command_generated:
        errors.append("native ship HUD never generated the Orxx Pterra C1 command")
    if trigger_pterra_after_load and not pterra_travel_command_consumed:
        errors.append("native VM scan never consumed the Orxx Pterra C1 command")
    if trigger_pterra_after_load and not scruter_scene_requested:
        errors.append("native travel never queued the Scruter_Jo C4 record")
    if (trigger_pterra_after_load and not scruter_sound_bank_loaded
            and hang_snapshot is None):
        errors.append("native Scruter_Jo streamed sound bank was never loaded")
    if (trigger_pterra_after_load and not scruter_scene_completed
            and hang_snapshot is None):
        errors.append("native Scruter_Jo Pterra lifecycle never completed")
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
        "title_accept_sent": title_accept_sent,
        "title_transition_confirmed": title_transition_confirmed,
        "title_transition_evidence": confirmed_title_evidence,
        "startup_presentation_line_seen": startup_presentation_line_seen,
        "load_menu_requested": load_menu_requested,
        "authentic_save_loaded": authentic_save_loaded,
        "title_pointer_recapture": title_pointer_recapture,
        "overall_timeout_seconds": timeout,
        "overall_timeout_reached": overall_timeout_reached,
        "log": str(log_path),
        "marker": marker_snapshot,
        "fault": fault_snapshot,
        "fault_detected": fault_snapshot is not None,
        "dos_read_overflow": dos_read_overflow_snapshot,
        "dos_read_overflow_detected": dos_read_overflow_snapshot is not None,
        "integrity_fault": integrity_fault_snapshot,
        "integrity_fault_detected": integrity_fault_snapshot is not None,
        "graphics_pointer_baseline": graphics_pointer_baseline,
        "graphics_pointer_faults": graphics_pointer_faults,
        "hang": hang_snapshot,
        "hang_detected": hang_snapshot is not None,
        "scruter_scene_requested": scruter_scene_requested,
        "scruter_scene_active_seen": scruter_scene_active_seen,
        "scruter_scene_completed": scruter_scene_completed,
        "scruter_sound_bank_loaded": scruter_sound_bank_loaded,
        "scruter_streamed_clip_count_before": (
            scruter_streamed_clip_count_before),
        "scruter_streamed_clip_count": scruter_streamed_clip_count,
        "pterra_target_row": pterra_target_row,
        "pterra_unlock_requested": pterra_unlock_requested,
        "pterra_unlock_completed": pterra_unlock_completed,
        "pterra_unlock_setup": pterra_unlock_setup,
        "pterra_nav_chart_started": pterra_nav_chart_started,
        "pterra_nav_chart_active": pterra_nav_chart_active,
        "pterra_nav_chart_selected": pterra_nav_chart_selected,
        "pterra_nav_panel_close_confirmed": (
            pterra_nav_panel_close_confirmed),
        "pterra_map_command_generated": pterra_map_command_generated,
        "pterra_map_command_consumed": pterra_map_command_consumed,
        "pterra_map_destination_committed": (
            pterra_map_destination_committed),
        "pterra_map_setup": pterra_map_setup,
        "pterra_travel_command_generated": pterra_travel_command_generated,
        "pterra_travel_command_consumed": pterra_travel_command_consumed,
        "pterra_ship_navigation_activated": (
            pterra_ship_navigation_activated),
        "pterra_travel_setup": pterra_travel_setup,
        "transition_cpu_samples": transition_cpu_samples,
        "pter_semantic_checkpoints": pter_semantic_checkpoints,
        "destination_committed": destination_committed,
        "pter": pter_snapshot,
        "pter_reached": pter_reached,
        "pter_completed": pter_completed_at is not None,
        "pter_choice_results": pter_choice_results,
        "pter_sustained": pter_sustained,
        "post_pter": post_pter_snapshot,
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
    parser.add_argument(
        "--dosbox", default="dosbox-x",
        help="DOSBox-X or DOSBox Staging executable (default: dosbox-x)")
    parser.add_argument("--timeout", type=float, default=900.0,
                        help="seconds to wait for the boundary")
    parser.add_argument(
        "--post-pter-seconds", type=float, default=5.0,
        help=("fault-free runtime required after completing proc pter "
              "(default: 5)"))
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
        help=("after the authentic save and its presentation complete, submit "
              "the native Orxx C1 travel command with Pterra as its target"))
    parser.add_argument(
        "--drive-authentic-save", action="store_true",
        help=("enter native gameplay, select save slot 1, and dismiss any "
              "resulting presentation through the game's input contract"))
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
            args.dosbox, "--noprimaryconf", "--nolocalconf",
            "-set", "sdl output=surface",
            *dosbox_mouse_settings(args.dosbox),
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
                toggle_mouse_capture=dosbox_needs_capture_toggle(
                    args.dosbox))
            args.output.write_text(json.dumps(snapshot, indent=1))
            print(f"wrote {args.output}")
            errors = snapshot.get("errors", [])
            if errors:
                raise RuntimeError("; ".join(str(error) for error in errors))
            return
        if args.drive:
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
