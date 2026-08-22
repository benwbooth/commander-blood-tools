#!/usr/bin/env python3
"""Guard rebuilt BLOODPRG invariants in a live DOSBox guest.

The watchdog derives DOS address zero from GAME_DATA:0000 and the live GS
value. It then verifies the final-link segment layout, the interrupt vector
table, and the conventional DOS memory-control-block chain while the game is
driven. A report is successful only after calibration and at least one guarded
sample.
"""
from __future__ import annotations

import os
import sys

_TOOL_DIRECTORY = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _TOOL_DIRECTORY:
    del sys.path[0]

import argparse
import ctypes
import hashlib
import importlib.util
import json
import re
import struct
import subprocess
import threading
import time
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GAME_DATA_ANCHOR = b"386 minimum !\0Not enough memory (570Ko min) !\0"
CONVENTIONAL_MEMORY_END = 0xA0000
GUEST_SNAPSHOT_SIZE = 0x100000
PTRACE_ATTACH = 16
PTRACE_DETACH = 17
TRANSIENT_INTERRUPT_VECTORS = frozenset((0x0F,))
VM_PROFILE_COUNT = 5
VM_RESOURCE_COUNT = 5
VM_RESOURCE_HANDLES_OFFSET = 0x6712
VM_RESOURCE_IMAGES_OFFSET = 0x671C
VM_RESOURCE_PROFILE_INDEX_OFFSET = 0x677E
VM_SCRIPT_PROFILE_REQUEST_OFFSET = 0x6780
VM_EXECUTION_ENABLED_OFFSET = 0x67A8
VM_RESOURCE_PROFILES_OFFSET = 0x11F4
VM_STATE_ARRAY_OFFSET = 0x6ADE
VM_TEXT_BUFFER_OFFSET = 0x0E18
VM_UI_FLAGS_OFFSET = 0x2793
VM_C2_PRESENTATION_GATE_OFFSET = 0x1FB2
VM_ACTIVE_LINE_OFFSET = 0x6788
VM_PRESENTATION_MODE_OFFSET = 0x27E0
VM_PRESENTATION_BOX_MODE_OFFSET = 0x27E1
LOAD_REQUEST_ACTIVE_OFFSET = 0x2737
SAVE_SLOT_MENU_PHASE_OFFSET = 0x2738
MOUSE_X_OFFSET = 0x0A2A
MOUSE_Y_OFFSET = 0x0A2C
MOUSE_PRIMARY_PRESSED_OFFSET = 0x0A3E
MOUSE_PRESS_PENDING_OFFSET = 0x0A40
TELEPHONE_CONSOLE_X = 230
TELEPHONE_CONSOLE_Y = 103
SCRIPT2_PROFILE = 1
SCRIPT2_RADIO_TARGET_VARIANT = 4
RADIO_BRIDGE_IDLE_SECONDS = 1.0
RADIO_CONSOLE_SETTLE_SECONDS = 1.0
RADIO_ORB_X = 125
RADIO_ORB_Y = 118
RADIO_ACCEPT_ORB_X = 160
RADIO_ACCEPT_ORB_Y = 150
RADIO_ACCEPT_IDLE_SECONDS = 0.5
SCRIPT2_RADIO_PROCEDURE_FLAGS = {
    "time": 0x2730,
    "radioscr": 0x2745,
    "sort": 0x2759,
    "radio1": 0x27D0,
}
SCRIPT2_SCRUTER_K_ACTION_OFFSET = 0x06FC

DIALOGUE_AUDIO_OFFSETS = {
    "voc_playback_enabled": (0x0ADE, "B"),
    "game_mode": (0x0ADF, "B"),
    "timer_hook_active": (0x0B21, "B"),
    "timer_tick": (0x0B29, "H"),
    "frame_delay": (0x0B2D, "H"),
    "dialogue_delay": (0x0B33, "H"),
    "dialogue_hold_countdown": (0x0B35, "H"),
    "bank_clip_count": (0x0BBB, "H"),
    "bank_dialogue_delay_base": (0x0BBD, "B"),
    "bank_dialogue_delay_limit": (0x0BBE, "B"),
    "last_clip": (0x0C4D, "H"),
    "streamed_clip_count": (0x0C53, "H"),
    "dialogue_seed": (0x0C55, "H"),
    "text_mode_seed": (0x0CF9, "B"),
    "text_mode_play": (0x0CFA, "B"),
    "text_voice_trigger": (0x0CFB, "B"),
}

PRESENTATION_FLOW_OFFSETS = {
    "mouse_x": (0x0A2A, "h"),
    "mouse_y": (0x0A2C, "h"),
    "nav_actor_presentation_state": (0x0A32, "H"),
    "mouse_primary_pressed": (0x0A3E, "B"),
    "mouse_secondary_pressed": (0x0A3F, "B"),
    "mouse_press_pending": (0x0A40, "B"),
    "list_file_handle": (0x0D5B, "H"),
    "list_state": (0x0D5F, "B"),
    "list_read_wrap_index": (0x0D60, "H"),
    "list_wrap_count": (0x0D62, "H"),
    "list_read_wrap_limit": (0x0D64, "H"),
    "list_secondary_wrap_limit": (0x0D66, "H"),
    "resource_source_offset": (0x0D84, "I"),
    "resource_source_remaining": (0x0D88, "I"),
    "list_head_offset": (0x0D8C, "H"),
    "list_head_segment": (0x0D8E, "H"),
    "list_tail_offset": (0x0D90, "H"),
    "list_tail_segment": (0x0D92, "H"),
    "list_active_offset": (0x0D94, "H"),
    "list_active_segment": (0x0D96, "H"),
    "list_buffer_end": (0x0D98, "H"),
    "list_queued_bytes": (0x0D9A, "H"),
    "list_iteration_count": (0x0DA0, "H"),
    "list_rollover_state": (0x0DAC, "B"),
    "list_entry_metric": (0x0DAF, "H"),
    "c2_presentation_gate": (0x1FB2, "B"),
    "ui_state": (0x2792, "H"),
    "presentation_mode": (0x27E0, "B"),
    "presentation_box_mode": (0x27E1, "B"),
    "presentation_box_phase": (0x2B93, "h"),
    "bridge_view_frame": (0x2795, "h"),
    "word_choice_active": (0x27D7, "B"),
    "text_display_active": (0x5E64, "B"),
    "text_reveal_phase": (0x5E65, "H"),
    "active_line": (0x6788, "H"),
    "displayed_line": (0x678A, "H"),
    "presentation_owner_offset": (0x679A, "H"),
    "nav_pending_record_link": (0x675A, "H"),
    "deferred_record_type": (0x6768, "H"),
    "deferred_record_related": (0x676A, "H"),
    "deferred_record_value": (0x676C, "H"),
    "presentation_request_flags": (0x67AA, "B"),
    "presentation_active": (0x67AC, "B"),
    "presentation_defer": (0x67B0, "B"),
    "presentation_start_lock": (0x67B7, "B"),
    "presentation_text_wait": (0x67BA, "B"),
    "dialogue_hold_complete": (0x67BB, "B"),
    "presentation_hold_ready": (0x67BC, "B"),
}

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

SEGMENT_ROW = re.compile(
    r"^(?P<name>GAME_DATA|FS_DATA)\s+\S+\s+\S+\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})\s+"
    r"(?P<size>[0-9A-Fa-f]{8})$"
)


class WatchdogError(RuntimeError):
    pass


class McbError(WatchdogError):
    pass


@dataclass(frozen=True)
class SegmentLayout:
    game_data: int
    fs_data: int


@dataclass(frozen=True)
class HostMapping:
    start: int
    end: int
    readable: bool


@dataclass(frozen=True)
class Mcb:
    segment: int
    kind: str
    owner: int
    paragraphs: int
    name: str

    @property
    def data_start(self) -> int:
        return self.segment + 1

    @property
    def data_end(self) -> int:
        return self.data_start + self.paragraphs

    def owns_segment(self, segment: int) -> bool:
        return self.data_start <= segment < self.data_end


@dataclass(frozen=True)
class ProfileState:
    profile: int
    request: int
    execution_enabled: int
    handles: tuple[int, ...]
    expected_handles: tuple[int, ...]
    images: tuple[tuple[int, int], ...]
    blockers: tuple[tuple[str, int], ...]

    @property
    def initialized(self) -> bool:
        return (
            0 <= self.profile < VM_PROFILE_COUNT
            and self.request == -1
            and self.handles == self.expected_handles
            and all(segment != 0 for _, segment in self.images)
        )

    def completed(self, target: int) -> bool:
        return (
            self.initialized
            and self.profile == target
            and self.execution_enabled == 1
        )

    @property
    def teleport_releaseable(self) -> bool:
        values = dict(self.blockers)
        return (
            values.get("vm_ui") == 4
            and all(
                value == 0
                for name, value in self.blockers
                if name != "vm_ui"
            )
        )


def parse_segment_layout(path: Path) -> SegmentLayout:
    placements: dict[str, tuple[int, int]] = {}
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = SEGMENT_ROW.match(line.strip())
        if match:
            placements[match["name"]] = (
                int(match["segment"], 16),
                int(match["offset"], 16),
            )
    missing = {"GAME_DATA", "FS_DATA"} - placements.keys()
    if missing:
        raise WatchdogError(
            f"{path}: missing segment(s): {', '.join(sorted(missing))}"
        )
    for name, (_, offset) in placements.items():
        if offset != 0:
            raise WatchdogError(
                f"{path}: {name} begins at offset {offset:#06x}, not zero"
            )
    return SegmentLayout(
        game_data=placements["GAME_DATA"][0],
        fs_data=placements["FS_DATA"][0],
    )


def ptrace_libc():
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [
        ctypes.c_long,
        ctypes.c_long,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    return libc


def locate_cpu_state(pid: int) -> dict[str, int] | None:
    executable = os.path.realpath(f"/proc/{pid}/exe")
    output = subprocess.run(
        ["nm", "-P", executable],
        text=True,
        capture_output=True,
        check=False,
    )
    if output.returncode != 0:
        return None
    symbols: dict[str, int] = {}
    symbol_sizes: dict[str, int] = {}
    for line in output.stdout.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
            if len(fields) >= 4:
                symbol_sizes[fields[0]] = int(fields[3], 16)
    if set(symbols) != {"Segs", "cpu_regs"}:
        return None

    image_base = None
    with open(f"/proc/{pid}/maps", encoding="ascii") as stream:
        for line in stream:
            fields = line.split()
            if len(fields) < 6:
                continue
            mapped = fields[-1].removesuffix(" (deleted)")
            if os.path.realpath(mapped) != executable:
                continue
            start = int(fields[0].split("-", 1)[0], 16)
            image_base = start - int(fields[2], 16)
            break
    if image_base is None:
        return None
    addresses = {
        name: image_base + offset for name, offset in symbols.items()
    }
    addresses["Segs_size"] = symbol_sizes.get("Segs", 0)
    return addresses


def read_cpu_state(mem, addresses: dict[str, int]) -> dict[str, int]:
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    segments = []
    if addresses.get("Segs_size") == 0x30:
        mem.seek(addresses["Segs"])
        segments = list(struct.unpack("<6H", mem.read(12)))
    else:
        for index in range(6):
            mem.seek(addresses["Segs"] + index * 8)
            segments.append(struct.unpack("<Q", mem.read(8))[0] & 0xFFFF)
    return {
        "es": segments[0],
        "cs": segments[1],
        "ss": segments[2],
        "ds": segments[3],
        "fs": segments[4],
        "gs": segments[5],
        "ip": ip & 0xFFFF,
        "ax": registers[0] & 0xFFFF,
        "cx": registers[1] & 0xFFFF,
        "dx": registers[2] & 0xFFFF,
        "bx": registers[3] & 0xFFFF,
        "sp": registers[4] & 0xFFFF,
        "bp": registers[5] & 0xFFFF,
        "si": registers[6] & 0xFFFF,
        "di": registers[7] & 0xFFFF,
    }


def write_json_report(path: Path, report: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def host_mappings(pid: int) -> list[HostMapping]:
    mappings = []
    with open(f"/proc/{pid}/maps", encoding="ascii") as stream:
        for line in stream:
            fields = line.split()
            start_text, end_text = fields[0].split("-", 1)
            mappings.append(
                HostMapping(
                    int(start_text, 16),
                    int(end_text, 16),
                    "r" in fields[1],
                )
            )
    return mappings


def exact_read(mem, address: int, size: int) -> bytes:
    mem.seek(address)
    data = mem.read(size)
    if len(data) != size:
        raise WatchdogError(
            f"short host-memory read at {address:#x}: {len(data)} of {size}"
        )
    return data


def exact_write(mem, address: int, data: bytes) -> None:
    mem.seek(address)
    written = mem.write(data)
    if written != len(data):
        raise WatchdogError(
            f"short host-memory write at {address:#x}: {written} of {len(data)}"
        )
    mem.flush()


def find_guest_base(pid: int, mem, game_segment: int) -> int | None:
    overlap = len(GAME_DATA_ANCHOR) - 1
    for mapping in host_mappings(pid):
        size = mapping.end - mapping.start
        if not mapping.readable or size < GUEST_SNAPSHOT_SIZE or size > 300_000_000:
            continue
        cursor = mapping.start
        tail = b""
        while cursor < mapping.end:
            chunk_size = min(2 * 1024 * 1024, mapping.end - cursor)
            try:
                chunk = exact_read(mem, cursor, chunk_size)
            except (OSError, WatchdogError):
                break
            haystack = tail + chunk
            search_from = 0
            while True:
                index = haystack.find(GAME_DATA_ANCHOR, search_from)
                if index < 0:
                    break
                anchor = cursor - len(tail) + index
                guest_base = anchor - game_segment * 16
                if (
                    mapping.start <= guest_base
                    and guest_base + GUEST_SNAPSHOT_SIZE <= mapping.end
                ):
                    snapshot = exact_read(mem, guest_base, GUEST_SNAPSHOT_SIZE)
                    if guest_memory_is_plausible(snapshot, game_segment):
                        return guest_base
                search_from = index + 1
            tail = haystack[-overlap:]
            cursor += chunk_size
    return None


def guest_memory_is_plausible(memory: bytes, game_segment: int) -> bool:
    anchor = game_segment * 16
    if memory[anchor : anchor + len(GAME_DATA_ANCHOR)] != GAME_DATA_ANCHOR:
        return False
    conventional_kib = struct.unpack_from("<H", memory, 0x0413)[0]
    if not 128 <= conventional_kib <= 640:
        return False
    int_21_offset, int_21_segment = struct.unpack_from("<HH", memory, 0x21 * 4)
    return (int_21_offset | int_21_segment) != 0


def parse_mcb_chain(
    memory: bytes,
    start_segment: int,
    required_segment: int,
) -> list[Mcb]:
    blocks = []
    segment = start_segment
    seen: set[int] = set()
    for _ in range(2048):
        if segment in seen:
            raise McbError(f"MCB cycle at {segment:#06x}")
        seen.add(segment)
        if not 0x0040 <= segment < 0xA000:
            raise McbError(f"MCB header outside conventional memory: {segment:#06x}")
        address = segment * 16
        if address + 16 > min(len(memory), CONVENTIONAL_MEMORY_END):
            raise McbError(f"truncated MCB header at {segment:#06x}")
        kind_byte = memory[address]
        if kind_byte not in (ord("M"), ord("Z")):
            raise McbError(
                f"invalid MCB type {kind_byte:#04x} at {segment:#06x}"
            )
        owner, paragraphs = struct.unpack_from("<HH", memory, address + 1)
        raw_name = memory[address + 8 : address + 16].rstrip(b"\0 ")
        name = "".join(
            chr(value) if 0x20 <= value < 0x7F else "." for value in raw_name
        )
        block = Mcb(segment, chr(kind_byte), owner, paragraphs, name)
        blocks.append(block)
        next_segment = block.data_end
        if not segment < next_segment <= 0xA000:
            raise McbError(
                f"MCB {segment:#06x} extends to invalid segment "
                f"{next_segment:#06x}"
            )
        if block.kind == "Z":
            if not any(entry.segment == required_segment for entry in blocks):
                raise McbError(
                    f"MCB chain omits required header {required_segment:#06x}"
                )
            return blocks
        segment = next_segment
    raise McbError("MCB chain exceeds 2048 blocks")


def discover_mcb_chain(memory: bytes, program_mcb: int, psp: int) -> list[Mcb]:
    address = program_mcb * 16
    if address + 5 > len(memory):
        raise McbError(f"program MCB {program_mcb:#06x} is outside guest memory")
    if memory[address] not in (ord("M"), ord("Z")):
        raise McbError(f"program MCB {program_mcb:#06x} has no M/Z signature")
    if struct.unpack_from("<H", memory, address + 1)[0] != psp:
        raise McbError(f"program MCB {program_mcb:#06x} is not owned by PSP {psp:#06x}")

    candidates = []
    for start in range(0x0040, program_mcb + 1):
        if memory[start * 16] != ord("M") and start != program_mcb:
            continue
        try:
            blocks = parse_mcb_chain(memory, start, program_mcb)
        except McbError:
            continue
        program = next(block for block in blocks if block.segment == program_mcb)
        if program.owner == psp:
            candidates.append(blocks)
    if not candidates:
        raise McbError(
            f"no complete MCB chain contains program header {program_mcb:#06x}"
        )
    return min(candidates, key=lambda blocks: blocks[0].segment)


def program_owned_block(
    blocks: list[Mcb], segment: int, psp: int
) -> Mcb | None:
    for block in blocks:
        if block.owner == psp and block.owns_segment(segment):
            return block
    return None


def game_is_ready(memory: bytes, game_segment: int) -> bool:
    base = game_segment * 16
    free_bytes = struct.unpack_from("<I", memory, base + 0x0A46)[0]
    crtc_port = struct.unpack_from("<H", memory, base + 0x0A9E)[0]
    timer_hook_active = memory[base + 0x0B21]
    return 0 < free_bytes <= 0x000A0000 and crtc_port == 0x03D4 and timer_hook_active == 1


def read_profile_state(
    memory: bytes, game_segment: int, fs_segment: int
) -> ProfileState:
    game = game_segment * 16
    fs = fs_segment * 16
    profile = struct.unpack_from(
        "<H", memory, game + VM_RESOURCE_PROFILE_INDEX_OFFSET
    )[0]
    request = struct.unpack_from(
        "<h", memory, game + VM_SCRIPT_PROFILE_REQUEST_OFFSET
    )[0]
    handles = struct.unpack_from(
        f"<{VM_RESOURCE_COUNT}H", memory, game + VM_RESOURCE_HANDLES_OFFSET
    )
    if 0 <= profile < VM_PROFILE_COUNT:
        expected_handles = struct.unpack_from(
            f"<{VM_RESOURCE_COUNT}H",
            memory,
            fs + VM_RESOURCE_PROFILES_OFFSET + profile * VM_RESOURCE_COUNT * 2,
        )
    else:
        expected_handles = ()
    images = tuple(
        struct.unpack_from(
            "<HH", memory, game + VM_RESOURCE_IMAGES_OFFSET + index * 4
        )
        for index in range(VM_RESOURCE_COUNT)
    )
    blockers = tuple(
        (name, memory[game + offset] & mask)
        for name, offset, mask in TELEPORT_BLOCKERS
    )
    return ProfileState(
        profile,
        request,
        memory[game + VM_EXECUTION_ENABLED_OFFSET],
        handles,
        expected_handles,
        images,
        blockers,
    )


def clear_presentation_ui_busy(flags: int) -> int:
    return flags & 0xFB


def write_primary_press(mem, game_address: int, pressed: bool) -> None:
    value = b"\x01" if pressed else b"\0"
    exact_write(mem, game_address + MOUSE_PRIMARY_PRESSED_OFFSET, value)
    exact_write(mem, game_address + MOUSE_PRESS_PENDING_OFFSET, value)


def write_script2_actor_prerequisites(
    mem,
    game_address: int,
    memory: bytes,
    game_offset: int,
) -> None:
    exact_write(mem, game_address + 0x2795, struct.pack("<h", 45))
    exact_write(mem, game_address + 0x279B, struct.pack("<H", 90))
    exact_write(
        mem,
        game_address + 0x2A33,
        bytes((memory[game_offset + 0x2A33] | 0x08,)),
    )
    exact_write(
        mem,
        game_address + VM_UI_FLAGS_OFFSET,
        bytes(((memory[game_offset + VM_UI_FLAGS_OFFSET] | 0x20) & 0xF7,)),
    )


def write_script2_variant(
    mem,
    guest_base: int,
    profile_state: ProfileState,
    variant: int,
    radio_enabled: bool,
) -> None:
    cod_offset, cod_segment = profile_state.images[0]
    record_offset, record_segment = profile_state.images[2]
    cod_address = guest_base + cod_segment * 16 + cod_offset
    record_address = guest_base + record_segment * 16 + record_offset
    exact_write(mem, record_address + 0x12C0, struct.pack("<H", variant))
    exact_write(
        mem,
        cod_address + SCRIPT2_RADIO_PROCEDURE_FLAGS["sort"],
        b"\0",
    )
    exact_write(
        mem,
        cod_address + SCRIPT2_RADIO_PROCEDURE_FLAGS["radio1"],
        b"\x01" if radio_enabled else b"\0",
    )


def send_mouse_button(display: str, pressed: bool) -> None:
    subprocess.run(
        ["xdotool", "mousedown" if pressed else "mouseup", "1"],
        env=dict(os.environ, DISPLAY=display),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def read_dialogue_audio_state(
    memory: bytes | bytearray,
    game_segment: int,
) -> dict[str, int]:
    game = game_segment * 16
    state = {}
    for name, (offset, field_type) in DIALOGUE_AUDIO_OFFSETS.items():
        state[name] = struct.unpack_from(
            "<" + field_type, memory, game + offset
        )[0]
    return state


def read_presentation_flow_state(
    memory: bytes | bytearray,
    game_segment: int,
) -> dict[str, int]:
    game = game_segment * 16
    state = {}
    for name, (offset, field_type) in PRESENTATION_FLOW_OFFSETS.items():
        state[name] = struct.unpack_from(
            "<" + field_type, memory, game + offset
        )[0]
    return state


def read_c_string(memory: bytes, address: int, limit: int = 160) -> str:
    end = min(address + limit, len(memory))
    raw = memory[address:end].split(b"\0", 1)[0]
    return "".join(
        chr(byte) if 0x20 <= byte < 0x7F else " " for byte in raw
    ).strip()


def read_vm_word_list(
    memory: bytes,
    words_address: int,
    dictionary_address: int,
    limit: int = 160,
) -> str:
    parts: list[str] = []
    cursor = words_address
    length = 0
    for _ in range(128):
        if cursor < 0 or cursor + 2 > len(memory):
            return ""
        dictionary_offset = struct.unpack_from("<H", memory, cursor)[0]
        cursor += 2
        if dictionary_offset in (0, 0xFFFF):
            break
        word = read_c_string(memory, dictionary_address + dictionary_offset)
        if not word:
            return ""
        separator = "" if word[0] in ",.?!:" or not parts else " "
        if length + len(separator) + len(word) > limit:
            break
        parts.append(separator + word)
        length += len(separator) + len(word)
    return "".join(parts)


def read_script2_radio_state(
    memory: bytes,
    game_segment: int,
    profile_state: ProfileState,
) -> dict[str, object] | None:
    if profile_state.profile != SCRIPT2_PROFILE:
        return None
    cod_offset, cod_segment = profile_state.images[0]
    record_offset, record_segment = profile_state.images[2]
    if cod_segment == 0 or record_segment == 0:
        return None
    cod = cod_segment * 16 + cod_offset
    records = record_segment * 16 + record_offset
    game = game_segment * 16
    action = memory[
        records + SCRIPT2_SCRUTER_K_ACTION_OFFSET:
        records + SCRIPT2_SCRUTER_K_ACTION_OFFSET + 6
    ]
    menu_words_offset, menu_words_segment = struct.unpack_from(
        "<HH", memory, game + 0x674A
    )
    dictionary_offset, dictionary_segment = struct.unpack_from(
        "<HH", memory, game + 0x6728
    )
    menu_subtitle = ""
    if menu_words_segment != 0 and dictionary_segment != 0:
        menu_subtitle = read_vm_word_list(
            memory,
            menu_words_segment * 16 + menu_words_offset,
            dictionary_segment * 16 + dictionary_offset,
        )
    buffered_subtitle = read_c_string(memory, game + VM_TEXT_BUFFER_OFFSET)
    if (memory[game + 0x67B0] & 1) != 0 and menu_subtitle:
        subtitle = menu_subtitle
    else:
        subtitle = buffered_subtitle
    return {
        "procedures": {
            name: memory[cod + offset]
            for name, offset in SCRIPT2_RADIO_PROCEDURE_FLAGS.items()
        },
        "timer_3": struct.unpack_from(
            "<H", memory, game + VM_STATE_ARRAY_OFFSET + 3 * 2
        )[0],
        "radio_variant": struct.unpack_from(
            "<H", memory, records + 0x12C0
        )[0],
        "scruter_k_action": action.hex(),
        "nav_pending_record_link": struct.unpack_from(
            "<H", memory, game + 0x675A
        )[0],
        "deferred_record": struct.unpack_from(
            "<HHH", memory, game + 0x6768
        ),
        "actor_slot_4": memory[game + 0x2A33:game + 0x2A33 + 24].hex(),
        "subtitle": subtitle,
        "buffered_subtitle": buffered_subtitle,
        "menu_subtitle": menu_subtitle,
        "menu_words": f"{menu_words_segment:04x}:{menu_words_offset:04x}",
        "dictionary": f"{dictionary_segment:04x}:{dictionary_offset:04x}",
    }


def dialogue_audio_stall_reason(state: dict[str, int]) -> str | None:
    selection_armed = (
        (state["voc_playback_enabled"] & 1) != 0
        and (state["game_mode"] & 1) == 0
        and (state["text_mode_play"] & 1) != 0
        and state["dialogue_delay"] == 0
    )
    if not selection_armed:
        return None

    clip_count = state["streamed_clip_count"]
    last_clip = state["last_clip"]
    if clip_count == 0 or (clip_count == 1 and last_clip == 0):
        return (
            "dialogue-clip-selection-no-candidates="
            f"count:{clip_count},last:{last_clip}"
        )
    return None


def profile_for_report(state: ProfileState) -> dict[str, object]:
    return {
        "profile": state.profile,
        "request": state.request,
        "execution_enabled": state.execution_enabled,
        "handles": list(state.handles),
        "expected_handles": list(state.expected_handles),
        "images": [
            f"{segment:04x}:{offset:04x}" for offset, segment in state.images
        ],
        "blockers": {name: value for name, value in state.blockers},
    }


def cpu_for_report(state: dict[str, int]) -> dict[str, str]:
    return {name: f"{value:#06x}" for name, value in state.items()}


def mcb_for_report(block: Mcb) -> dict[str, str | int]:
    return {
        "segment": f"{block.segment:#06x}",
        "kind": block.kind,
        "owner": f"{block.owner:#06x}",
        "paragraphs": block.paragraphs,
        "name": block.name,
    }


def changed_interrupt_vectors(before: bytes, after: bytes) -> list[dict[str, str]]:
    changes = []
    for vector in range(256):
        offset = vector * 4
        if before[offset : offset + 4] == after[offset : offset + 4]:
            continue
        before_offset, before_segment = struct.unpack_from("<HH", before, offset)
        after_offset, after_segment = struct.unpack_from("<HH", after, offset)
        changes.append(
            {
                "vector": f"{vector:#04x}",
                "before": f"{before_segment:04x}:{before_offset:04x}",
                "after": f"{after_segment:04x}:{after_offset:04x}",
            }
        )
    return changes


def load_action_driver():
    path = ROOT / "re" / "tools" / "capture_pterra_boundary.py"
    spec = importlib.util.spec_from_file_location("dosbox_action_driver", path)
    if spec is None or spec.loader is None:
        raise WatchdogError(f"cannot load action driver from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.run_driver


def default_link_map(cd_dir: Path) -> Path:
    return cd_dir.parent / "validation/bloodprg_runtime/final/link.map"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cd-dir", type=Path, required=True)
    parser.add_argument("--install-parent", type=Path, required=True)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--link-map", type=Path)
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--display", default=":83")
    parser.add_argument("--seconds", type=float, default=600.0)
    parser.add_argument("--calibration-timeout", type=float, default=30.0)
    parser.add_argument("--stable-samples", type=int, default=3)
    parser.add_argument("--poll-seconds", type=float, default=0.25)
    parser.add_argument(
        "--driver-delay",
        type=float,
        default=0.0,
        help="seconds to wait before the action driver begins window discovery",
    )
    parser.add_argument(
        "--actions",
        type=Path,
        help="input script in drive_real_game.sh vocabulary",
    )
    parser.add_argument("--report", type=Path)
    parser.add_argument(
        "--guest-snapshot",
        type=Path,
        help="write the 1 MiB guest-memory image at the radio release boundary",
    )
    parser.add_argument("--xvfb", action="store_true")
    parser.add_argument(
        "--teleport-profile",
        type=int,
        help="request one SCRIPT profile 0..4 from a fresh boot",
    )
    parser.add_argument(
        "--post-teleport-samples",
        type=int,
        default=4,
        help="guarded samples required after the last completed teleport",
    )
    parser.add_argument(
        "--script2-radio-probe",
        action="store_true",
        help=(
            "load GAME1.SAV and require the Scruter variant-4 call to advance "
            "through YOU DO THE COUNTING to Honk's report"
        ),
    )
    args = parser.parse_args()

    cd_dir = args.cd_dir.resolve()
    install_parent = args.install_parent.resolve()
    link_map = (args.link_map or default_link_map(cd_dir)).resolve()
    layout = parse_segment_layout(link_map)
    if args.stable_samples < 1:
        raise WatchdogError("--stable-samples must be positive")
    if args.post_teleport_samples < 1:
        raise WatchdogError("--post-teleport-samples must be positive")
    if (
        args.teleport_profile is not None
        and not 0 <= args.teleport_profile < VM_PROFILE_COUNT
    ):
        raise WatchdogError(
            f"teleport profile must be in 0..4: {args.teleport_profile}"
        )
    if args.script2_radio_probe:
        if args.teleport_profile is not None:
            raise WatchdogError(
                "--script2-radio-probe uses GAME1.SAV and cannot teleport"
            )
        save_path = install_parent / "cblood" / "GAME1.SAV"
        if not save_path.is_file():
            raise WatchdogError(
                f"--script2-radio-probe requires {save_path}"
            )

    report: dict[str, object] = {
        "verdict": "INCOMPLETE",
        "samples": 0,
        "guarded_samples": 0,
        "anomalies": [],
    }
    env = dict(os.environ, DISPLAY=args.display, SDL_VIDEODRIVER="x11")
    xvfb = None
    dosbox = None
    attached = False
    radio_physical_mouse_held = False
    driver_errors: list[str] = []
    libc = ptrace_libc()

    try:
        if args.xvfb:
            xvfb = subprocess.Popen(
                ["Xvfb", args.display, "-screen", "0", "800x600x24"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            time.sleep(2.0)

        dosbox_args = [
            args.dosbox,
            "--noprimaryconf",
            "--nolocalconf",
            "-set",
            "sdl output=surface",
            "-set",
            "sdl autolock=true",
            "-set",
            "cpu cycles=max",
            "-set",
            "cpu core=dynamic",
            "-set",
            "render frameskip=10",
            "-c",
            f"mount c {install_parent}",
            "-c",
            f"mount d {cd_dir} -t cdrom",
            "-c",
            "d:",
            "-c",
            f"{args.executable} AMR S162227 EMS WRIC:\\cblood\\",
            "-c",
            "exit",
        ]
        dosbox = subprocess.Popen(
            dosbox_args,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        if args.actions:
            actions = args.actions.read_text(encoding="utf-8").splitlines()
            run_driver = load_action_driver()

            def drive() -> None:
                if args.driver_delay > 0:
                    time.sleep(args.driver_delay)
                try:
                    run_driver(actions, args.display, args.executable)
                except Exception as error:  # propagated through driver_errors
                    driver_errors.append(f"{type(error).__name__}: {error}")

            threading.Thread(target=drive, daemon=True).start()

        started = time.monotonic()
        deadline = started + args.seconds
        calibration_deadline = started + args.calibration_timeout
        cpu_addresses = None
        guest_base = None
        calibration_key = None
        stable_samples = 0
        expected = None
        last_context = None
        teleport_queue = (
            [] if args.teleport_profile is None else [args.teleport_profile]
        )
        teleport_inflight = None
        teleport_last_completion = None
        radio_probe_phase = "wait-title-idle"
        radio_load_slot_pressed = False
        radio_intro_seen = False
        radio_input_held = False
        radio_console_pressed = False
        radio_bridge_idle_started = None
        radio_console_selected_at = None
        radio_post_montage_idle_started = None
        radio_orb_click_count = 0
        radio_snapshot_written = False
        radio_lines: list[str] = []

        while time.monotonic() < deadline:
            sample_delay = args.poll_seconds
            time.sleep(sample_delay)
            if driver_errors:
                report["verdict"] = "DRIVER-ERROR"
                report["error"] = driver_errors[0]
                break
            if dosbox.poll() is not None:
                report["exit_code"] = dosbox.returncode
                report["verdict"] = (
                    "CLEAN-EXIT"
                    if (
                        expected is not None
                        and dosbox.returncode == 0
                        and args.teleport_profile is None
                    )
                    else (
                        "GAME-EXIT"
                        if expected is not None
                        else "EXIT-BEFORE-CALIBRATION"
                    )
                )
                break
            if expected is None and time.monotonic() >= calibration_deadline:
                report["verdict"] = "CALIBRATION-TIMEOUT"
                break

            ctypes.set_errno(0)
            if libc.ptrace(PTRACE_ATTACH, dosbox.pid, None, None) != 0:
                continue
            os.waitpid(dosbox.pid, 0)
            attached = True
            try:
                with open(f"/proc/{dosbox.pid}/mem", "r+b", buffering=0) as mem:
                    if cpu_addresses is None:
                        cpu_addresses = locate_cpu_state(dosbox.pid)
                    if cpu_addresses is None:
                        continue
                    state = read_cpu_state(mem, cpu_addresses)
                    report["samples"] = int(report["samples"]) + 1

                    if expected is None:
                        if state["gs"] < layout.game_data:
                            continue
                        load_segment = state["gs"] - layout.game_data
                        expected_fs = load_segment + layout.fs_data
                        if expected_fs > 0xFFFF or state["fs"] != expected_fs:
                            continue
                        psp = load_segment - 0x10
                        if psp < 0x0050:
                            continue
                        if guest_base is None:
                            guest_base = find_guest_base(
                                dosbox.pid, mem, state["gs"]
                            )
                        if guest_base is None:
                            continue
                        memory = exact_read(mem, guest_base, GUEST_SNAPSHOT_SIZE)
                        if not game_is_ready(memory, state["gs"]):
                            continue
                        blocks = discover_mcb_chain(memory, psp - 1, psp)
                        ivt_hash = hashlib.sha256(memory[:0x400]).hexdigest()
                        key = (
                            guest_base,
                            state["gs"],
                            expected_fs,
                            psp,
                            blocks[0].segment,
                            ivt_hash,
                        )
                        if key == calibration_key:
                            stable_samples += 1
                        else:
                            calibration_key = key
                            stable_samples = 1
                        if stable_samples < args.stable_samples:
                            continue
                        expected = {
                            "guest_base": guest_base,
                            "load_segment": load_segment,
                            "game_segment": state["gs"],
                            "fs_segment": expected_fs,
                            "psp": psp,
                            "program_mcb": psp - 1,
                            "mcb_start": blocks[0].segment,
                            "ivt_sha256": ivt_hash,
                            "ivt_bytes": memory[:0x400],
                        }
                        report["calibrated"] = {
                            "load_segment": f"{load_segment:#06x}",
                            "game_segment": f"{state['gs']:#06x}",
                            "fs_segment": f"{expected_fs:#06x}",
                            "psp": f"{psp:#06x}",
                            "mcb_start": f"{blocks[0].segment:#06x}",
                            "mcb_count": len(blocks),
                            "mcb_chain": [mcb_for_report(block) for block in blocks],
                            "ivt_sha256": ivt_hash,
                            "program_mcb": mcb_for_report(
                                next(
                                    block
                                    for block in blocks
                                    if block.segment == psp - 1
                                )
                            ),
                        }
                        continue

                    memory = exact_read(
                        mem, int(expected["guest_base"]), GUEST_SNAPSHOT_SIZE
                    )
                    issues = []
                    diagnostics: dict[str, object] = {}
                    try:
                        blocks = parse_mcb_chain(
                            memory,
                            int(expected["mcb_start"]),
                            int(expected["program_mcb"]),
                        )
                        program = next(
                            block
                            for block in blocks
                            if block.segment == int(expected["program_mcb"])
                        )
                        if program.owner != int(expected["psp"]):
                            issues.append(
                                f"program-mcb-owner={program.owner:#06x} expected "
                                f"{int(expected['psp']):#06x}"
                            )
                    except McbError as error:
                        blocks = []
                        issues.append(f"mcb-chain: {error}")

                    if state["gs"] != int(expected["game_segment"]):
                        issues.append(
                            f"gs={state['gs']:#06x} expected "
                            f"{int(expected['game_segment']):#06x}"
                        )
                    fs_policy = "startup-table"
                    if state["fs"] != int(expected["fs_segment"]):
                        owner = program_owned_block(
                            blocks, state["fs"], int(expected["psp"])
                        )
                        if owner is None:
                            issues.append(
                                f"fs={state['fs']:#06x} is neither the startup "
                                "table nor game-owned overlay memory"
                            )
                            fs_policy = "invalid"
                        else:
                            fs_policy = f"game-owned-mcb-{owner.segment:#06x}"

                    ivt_hash = hashlib.sha256(memory[:0x400]).hexdigest()
                    if ivt_hash != expected["ivt_sha256"]:
                        changes = [
                            change
                            for change in changed_interrupt_vectors(
                                expected["ivt_bytes"], memory[:0x400]
                            )
                            if int(change["vector"], 16)
                            not in TRANSIENT_INTERRUPT_VECTORS
                        ]
                        if changes:
                            diagnostics["ivt_changes"] = changes
                            issues.append(
                                "ivt-vectors-changed="
                                + ",".join(
                                    change["vector"] for change in changes
                                )
                            )
                    if not guest_memory_is_plausible(
                        memory, int(expected["game_segment"])
                    ):
                        issues.append("guest-memory-anchor-invalid")

                    report["guarded_samples"] = int(report["guarded_samples"]) + 1
                    profile_state = read_profile_state(
                        memory,
                        int(expected["game_segment"]),
                        int(expected["fs_segment"]),
                    )
                    audio_state = read_dialogue_audio_state(
                        memory, int(expected["game_segment"])
                    )
                    presentation_state = read_presentation_flow_state(
                        memory, int(expected["game_segment"])
                    )
                    radio_state = read_script2_radio_state(
                        memory,
                        int(expected["game_segment"]),
                        profile_state,
                    )
                    report["last_runtime"] = {
                        "cpu": cpu_for_report(state),
                        "profile_state": profile_for_report(profile_state),
                        "audio_flow": audio_state,
                        "presentation_flow": presentation_state,
                        "radio_flow": radio_state,
                    }
                    runtime_samples = report.setdefault("runtime_samples", [])
                    assert isinstance(runtime_samples, list)
                    runtime_samples.append(
                        {
                            "sample": report["guarded_samples"],
                            "cpu": cpu_for_report(state),
                            "audio_flow": audio_state,
                            "presentation_flow": presentation_state,
                            "radio_flow": radio_state,
                        }
                    )
                    del runtime_samples[:-4096]
                    if args.report and (
                        args.poll_seconds >= 0.05
                        or int(report["guarded_samples"]) % 50 == 0
                    ):
                        write_json_report(args.report, report)
                    audio_stall = dialogue_audio_stall_reason(audio_state)
                    if audio_stall is not None:
                        issues.append(audio_stall)
                        diagnostics["audio_flow"] = audio_state
                    if teleport_inflight is not None:
                        if profile_state.completed(teleport_inflight):
                            teleports = report.setdefault("teleports", [])
                            assert isinstance(teleports, list) and teleports
                            teleports[-1]["completed_sample"] = report[
                                "guarded_samples"
                            ]
                            teleports[-1]["completed_state"] = profile_for_report(
                                profile_state
                            )
                            teleport_inflight = None
                            teleport_last_completion = int(
                                report["guarded_samples"]
                            )
                    elif (
                        teleport_queue
                        and profile_state.initialized
                        and profile_state.teleport_releaseable
                    ):
                        teleport_inflight = teleport_queue.pop(0)
                        game_address = (
                            int(expected["guest_base"])
                            + int(expected["game_segment"]) * 16
                        )
                        request_address = (
                            game_address + VM_SCRIPT_PROFILE_REQUEST_OFFSET
                        )
                        exact_write(
                            mem, request_address, struct.pack("<h", teleport_inflight)
                        )
                        blockers = dict(profile_state.blockers)
                        released_ui_busy = blockers["vm_ui"] == 4
                        if released_ui_busy:
                            raw_ui_flags = memory[
                                int(expected["game_segment"]) * 16 + 0x2793
                            ]
                            exact_write(
                                mem,
                                game_address + 0x2793,
                                bytes((clear_presentation_ui_busy(raw_ui_flags),)),
                            )
                        memory = exact_read(
                            mem,
                            int(expected["guest_base"]),
                            GUEST_SNAPSHOT_SIZE,
                        )
                        written_state = read_profile_state(
                            memory,
                            int(expected["game_segment"]),
                            int(expected["fs_segment"]),
                        )
                        if written_state.request != teleport_inflight:
                            issues.append(
                                "teleport-request-write-did-not-stick="
                                f"{written_state.request}"
                            )
                        teleports = report.setdefault("teleports", [])
                        assert isinstance(teleports, list)
                        teleports.append(
                            {
                                "target": teleport_inflight,
                                "requested_sample": report["guarded_samples"],
                                "released_ui_busy": released_ui_busy,
                                "request_state": profile_for_report(written_state),
                            }
                        )

                    if args.script2_radio_probe:
                        game_offset = int(expected["game_segment"]) * 16
                        game_address = int(expected["guest_base"]) + game_offset
                        blockers = dict(profile_state.blockers)
                        probe = report.setdefault(
                            "radio_probe",
                            {"phase": radio_probe_phase, "lines": radio_lines},
                        )
                        assert isinstance(probe, dict)

                        if (
                            radio_probe_phase == "wait-title-idle"
                            and profile_state.profile == 0
                            and profile_state.teleport_releaseable
                        ):
                            exact_write(
                                mem,
                                game_address + LOAD_REQUEST_ACTIVE_OFFSET,
                                b"\x01",
                            )
                            exact_write(
                                mem,
                                game_address + SAVE_SLOT_MENU_PHASE_OFFSET,
                                b"\x01",
                            )
                            radio_probe_phase = "wait-load-menu"
                            probe["load_menu_sample"] = report[
                                "guarded_samples"
                            ]
                        elif radio_probe_phase in (
                            "wait-load-menu",
                            "press-load-slot",
                        ):
                            if blockers.get("load", 0) != 0:
                                exact_write(
                                    mem,
                                    game_address + MOUSE_X_OFFSET,
                                    struct.pack("<h", 110),
                                )
                                exact_write(
                                    mem,
                                    game_address + MOUSE_Y_OFFSET,
                                    struct.pack("<h", 47),
                                )
                                exact_write(
                                    mem,
                                    game_address + MOUSE_PRIMARY_PRESSED_OFFSET,
                                    b"\x01",
                                )
                                radio_load_slot_pressed = True
                                radio_probe_phase = "press-load-slot"
                            elif (
                                radio_load_slot_pressed
                                and profile_state.completed(SCRIPT2_PROFILE)
                            ):
                                exact_write(
                                    mem,
                                    game_address + MOUSE_PRIMARY_PRESSED_OFFSET,
                                    b"\0",
                                )
                                radio_probe_phase = "wait-post-load-intro"
                                probe["save_loaded_sample"] = report[
                                    "guarded_samples"
                                ]
                        elif radio_probe_phase == "wait-post-load-intro":
                            if (
                                presentation_state["active_line"] == 2
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 1
                            ):
                                radio_intro_seen = True
                                radio_probe_phase = "dismiss-post-load-intro"
                        elif radio_probe_phase == "dismiss-post-load-intro":
                            if (
                                presentation_state["active_line"] == 2
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 1
                            ):
                                exact_write(
                                    mem,
                                    game_address + MOUSE_X_OFFSET,
                                    struct.pack("<h", 110),
                                )
                                exact_write(
                                    mem,
                                    game_address + MOUSE_Y_OFFSET,
                                    struct.pack("<h", 96),
                                )
                                write_primary_press(mem, game_address, True)
                                if not radio_physical_mouse_held:
                                    send_mouse_button(args.display, True)
                                    radio_physical_mouse_held = True
                                radio_bridge_idle_started = None
                            elif (
                                radio_intro_seen
                                and presentation_state["active_line"] == 0xFFFF
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 0
                                and all(value == 0 for value in blockers.values())
                            ):
                                write_primary_press(mem, game_address, False)
                                if radio_physical_mouse_held:
                                    send_mouse_button(args.display, False)
                                    radio_physical_mouse_held = False
                                if radio_bridge_idle_started is None:
                                    radio_bridge_idle_started = time.monotonic()
                                bridge_idle_seconds = (
                                    time.monotonic() - radio_bridge_idle_started
                                )
                                probe["bridge_idle_seconds"] = round(
                                    bridge_idle_seconds, 3
                                )
                                if bridge_idle_seconds >= RADIO_BRIDGE_IDLE_SECONDS:
                                    radio_probe_phase = "press-radio-console"
                                    probe["intro_dismissed_sample"] = report[
                                        "guarded_samples"
                                    ]
                                    if args.guest_snapshot:
                                        args.guest_snapshot.parent.mkdir(
                                            parents=True, exist_ok=True
                                        )
                                        args.guest_snapshot.write_bytes(memory)
                                        radio_snapshot_written = True
                            else:
                                write_primary_press(mem, game_address, False)
                                if radio_physical_mouse_held:
                                    send_mouse_button(args.display, False)
                                    radio_physical_mouse_held = False
                                radio_bridge_idle_started = None
                        elif radio_probe_phase == "press-radio-console":
                            exact_write(
                                mem,
                                game_address + MOUSE_X_OFFSET,
                                struct.pack("<h", TELEPHONE_CONSOLE_X),
                            )
                            exact_write(
                                mem,
                                game_address + MOUSE_Y_OFFSET,
                                struct.pack("<h", TELEPHONE_CONSOLE_Y),
                            )
                            write_primary_press(mem, game_address, True)
                            send_mouse_button(args.display, True)
                            radio_physical_mouse_held = True
                            radio_console_pressed = True
                            radio_probe_phase = "release-radio-console"
                        elif radio_probe_phase == "release-radio-console":
                            if radio_console_pressed:
                                write_primary_press(mem, game_address, False)
                            if radio_physical_mouse_held:
                                send_mouse_button(args.display, False)
                                radio_physical_mouse_held = False
                            radio_console_selected_at = time.monotonic()
                            radio_probe_phase = "wait-radio-orb"
                            probe["radio_console_selected_sample"] = report[
                                "guarded_samples"
                            ]
                        elif radio_probe_phase == "wait-radio-orb":
                            if (
                                radio_console_selected_at is not None
                                and time.monotonic() - radio_console_selected_at
                                >= RADIO_CONSOLE_SETTLE_SECONDS
                            ):
                                exact_write(
                                    mem,
                                    game_address + MOUSE_X_OFFSET,
                                    struct.pack("<h", RADIO_ORB_X),
                                )
                                exact_write(
                                    mem,
                                    game_address + MOUSE_Y_OFFSET,
                                    struct.pack("<h", RADIO_ORB_Y),
                                )
                                write_primary_press(mem, game_address, True)
                                send_mouse_button(args.display, True)
                                radio_physical_mouse_held = True
                                radio_probe_phase = "release-radio-orb"
                        elif radio_probe_phase == "release-radio-orb":
                            write_primary_press(mem, game_address, False)
                            if radio_physical_mouse_held:
                                send_mouse_button(args.display, False)
                                radio_physical_mouse_held = False
                            radio_orb_click_count = 1
                            radio_post_montage_idle_started = None
                            radio_probe_phase = "wait-first-radio"
                            probe["radio_orb_clicked_sample"] = report[
                                "guarded_samples"
                            ]
                        elif (
                            radio_probe_phase == "wait-first-radio"
                            and radio_orb_click_count == 1
                        ):
                            radio_idle = (
                                presentation_state[
                                    "c2_presentation_gate"
                                ] == 0
                                and presentation_state[
                                    "text_display_active"
                                ] == 0
                                and blockers.get("load", 0) == 0
                                and blockers.get("save", 0) == 0
                            )
                            if radio_idle:
                                if radio_post_montage_idle_started is None:
                                    radio_post_montage_idle_started = (
                                        time.monotonic()
                                    )
                                if (
                                    time.monotonic()
                                    - radio_post_montage_idle_started
                                    >= RADIO_ACCEPT_IDLE_SECONDS
                                ):
                                    exact_write(
                                        mem,
                                        game_address + MOUSE_X_OFFSET,
                                        struct.pack("<h", RADIO_ACCEPT_ORB_X),
                                    )
                                    exact_write(
                                        mem,
                                        game_address + MOUSE_Y_OFFSET,
                                        struct.pack("<h", RADIO_ACCEPT_ORB_Y),
                                    )
                                    write_primary_press(mem, game_address, True)
                                    send_mouse_button(args.display, True)
                                    radio_physical_mouse_held = True
                                    radio_probe_phase = "release-radio-accept"
                            else:
                                radio_post_montage_idle_started = None
                        elif radio_probe_phase == "release-radio-accept":
                            write_primary_press(mem, game_address, False)
                            if radio_physical_mouse_held:
                                send_mouse_button(args.display, False)
                                radio_physical_mouse_held = False
                            radio_orb_click_count = 2
                            write_script2_actor_prerequisites(
                                mem, game_address, memory, game_offset
                            )
                            write_script2_variant(
                                mem,
                                int(expected["guest_base"]),
                                profile_state,
                                SCRIPT2_RADIO_TARGET_VARIANT,
                                False,
                            )
                            radio_probe_phase = "wait-first-radio"
                            probe["radio_call_accepted_sample"] = report[
                                "guarded_samples"
                            ]

                        probe["phase"] = radio_probe_phase
                        probe["snapshot_written"] = radio_snapshot_written

                    if (
                        args.script2_radio_probe
                        and radio_probe_phase == "wait-first-radio"
                        and radio_state is not None
                    ):
                        subtitle = str(radio_state["subtitle"])
                        action = str(radio_state["scruter_k_action"])
                        if radio_orb_click_count == 2 and action.startswith("c3"):
                            write_script2_actor_prerequisites(
                                mem, game_address, memory, game_offset
                            )
                            write_script2_variant(
                                mem,
                                int(expected["guest_base"]),
                                profile_state,
                                SCRIPT2_RADIO_TARGET_VARIANT,
                                False,
                            )
                        if (
                            radio_orb_click_count == 2
                            and action.startswith("c4")
                            and presentation_state["presentation_active"] == 1
                            and audio_state["bank_clip_count"] >= 22
                        ):
                            write_script2_variant(
                                mem,
                                int(expected["guest_base"]),
                                profile_state,
                                SCRIPT2_RADIO_TARGET_VARIANT,
                                True,
                            )
                        if subtitle and (
                            not radio_lines or radio_lines[-1] != subtitle
                        ):
                            radio_lines.append(subtitle)
                            probe = report.setdefault("radio_probe", {})
                            assert isinstance(probe, dict)
                            probe["lines"] = radio_lines
                            line_states = probe.setdefault("line_states", [])
                            assert isinstance(line_states, list)
                            line_states.append(
                                {
                                    "sample": report["guarded_samples"],
                                    "subtitle": subtitle,
                                    "cpu": cpu_for_report(state),
                                    "audio_flow": audio_state,
                                    "presentation_flow": presentation_state,
                                    "radio_flow": radio_state,
                                }
                            )
                        normalized = subtitle.upper()
                        if (
                            radio_orb_click_count == 2
                            and "WAIT COMMANDER" in normalized
                            and presentation_state["c2_presentation_gate"] == 1
                        ):
                            if not radio_input_held:
                                write_primary_press(mem, game_address, True)
                                send_mouse_button(args.display, True)
                                radio_physical_mouse_held = True
                                radio_input_held = True
                        elif radio_input_held:
                            write_primary_press(mem, game_address, False)
                            if radio_physical_mouse_held:
                                send_mouse_button(args.display, False)
                                radio_physical_mouse_held = False
                            radio_input_held = False
                        elif "REPORT FROM HONK" in normalized:
                            report["verdict"] = "RADIO-PROBE-COMPLETE"
                            probe = report.setdefault("radio_probe", {})
                            assert isinstance(probe, dict)
                            probe["completed_sample"] = report[
                                "guarded_samples"
                            ]
                            break

                    context = (
                        state["cs"],
                        state["ds"],
                        state["fs"],
                        state["gs"],
                        fs_policy,
                    )
                    if context != last_context:
                        last_context = context
                        transitions = report.setdefault("contexts", [])
                        if isinstance(transitions, list) and len(transitions) < 100:
                            transitions.append(
                                {
                                    "sample": report["guarded_samples"],
                                    "cpu": cpu_for_report(state),
                                    "fs_policy": fs_policy,
                                }
                            )
                    if issues:
                        report["verdict"] = "ANOMALY"
                        diagnostics["profile_state"] = profile_for_report(
                            profile_state
                        )
                        anomalies = report["anomalies"]
                        assert isinstance(anomalies, list)
                        anomaly = {
                            "sample": report["guarded_samples"],
                            "cpu": cpu_for_report(state),
                            "issues": issues,
                        }
                        anomaly.update(diagnostics)
                        anomalies.append(anomaly)
                        break
                    if (
                        args.teleport_profile is not None
                        and not args.script2_radio_probe
                        and not teleport_queue
                        and teleport_inflight is None
                        and teleport_last_completion is not None
                        and int(report["guarded_samples"])
                        - teleport_last_completion
                        >= args.post_teleport_samples
                    ):
                        report["verdict"] = "TELEPORTS-COMPLETE"
                        break
            finally:
                if attached:
                    libc.ptrace(PTRACE_DETACH, dosbox.pid, None, None)
                    attached = False
        else:
            if args.script2_radio_probe:
                report["verdict"] = "RADIO-PROBE-TIMEOUT"
                probe = report.setdefault("radio_probe", {})
                if isinstance(probe, dict):
                    probe["phase"] = radio_probe_phase
            elif args.teleport_profile is not None:
                report["verdict"] = "TELEPORT-TIMEOUT"
                report["teleport_pending"] = (
                    [teleport_inflight] if teleport_inflight is not None else []
                ) + teleport_queue
            else:
                report["verdict"] = (
                    "TIMEOUT-NO-ANOMALY"
                    if expected is not None and int(report["guarded_samples"]) > 0
                    else "CALIBRATION-TIMEOUT"
                )
    except Exception as error:
        report["verdict"] = "WATCHDOG-ERROR"
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if radio_physical_mouse_held:
            send_mouse_button(args.display, False)
        if dosbox is not None and dosbox.poll() is None:
            if attached:
                libc.ptrace(PTRACE_DETACH, dosbox.pid, None, None)
            dosbox.kill()
            dosbox.wait()
        if xvfb is not None:
            xvfb.terminate()
            xvfb.wait()

    if args.report:
        write_json_report(args.report, report)
    print(
        json.dumps(
            {
                "verdict": report["verdict"],
                "samples": report["samples"],
                "guarded_samples": report["guarded_samples"],
                "anomalies": report["anomalies"],
            }
        )
    )
    return 0 if report["verdict"] in (
        "TIMEOUT-NO-ANOMALY",
        "CLEAN-EXIT",
        "TELEPORTS-COMPLETE",
        "RADIO-PROBE-COMPLETE",
    ) else 1


if __name__ == "__main__":
    raise SystemExit(main())
