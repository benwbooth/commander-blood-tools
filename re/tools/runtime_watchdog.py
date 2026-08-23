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
import signal
import struct
import subprocess
import threading
import time
from collections import Counter, deque
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GAME_DATA_ANCHOR = b"386 minimum !\0Not enough memory (570Ko min) !\0"
CONVENTIONAL_MEMORY_END = 0xA0000
GUEST_SNAPSHOT_SIZE = 0x100000
PTRACE_ATTACH = 16
PTRACE_DETACH = 17
TRANSIENT_INTERRUPT_VECTORS = frozenset((0x0F,))
DOSBOX_FAULT_PATTERNS = (
    (
        "illegal-interrupt",
        re.compile(
            rb"Illegal Unhandled Interrupt Called ([0-9]+)",
            re.IGNORECASE,
        ),
    ),
    (
        "fatal-error",
        re.compile(
            rb"(?:DOSBox-X fatal error|E_Exit:|Segmentation fault|"
            rb"Assertion .{0,160} failed)",
            re.IGNORECASE,
        ),
    ),
)
DOSBOX_LOG_CARRY_SIZE = 512
MAX_HOT_LOOP_IPS = 16
MAX_RUNTIME_SAMPLES = 512
SUCCESSFUL_VERDICTS = frozenset(
    (
        "TIMEOUT-NO-ANOMALY",
        "CLEAN-EXIT",
        "TELEPORTS-COMPLETE",
        "RADIO-PROBE-COMPLETE",
        "BOB-PROBE-COMPLETE",
        "CONTACT-PROBE-COMPLETE",
    )
)
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
SCRIPT2_RADIO_CHECKPOINTS = (
    (0x2B05, "MESSAGE RADIO:"),
    (0x2BB5, "OKAY OKAY, WISE GUY!"),
    (0x2BC9, "YOU DO THE COUNTING"),
    (0x2BDB, "CRUIIIIK!"),
    (None, "REPORT FROM HONK"),
)
SCRIPT1_PROFILE = 0
SCRIPT1_BOB_OBJECT_OFFSET = 0x004A
SCRIPT1_BOB_ACTION_OFFSET = 0x0084
SCRIPT1_BOB_PROCEDURE_FLAG_OFFSET = 0x077E
SCRIPT1_BOB_CHECKPOINTS = (
    (0x078E, "GOOD DAY COMMANDER. MY NAME IS BOB, BOB MORLOCK"),
    (0x07AE, "IF THE PHONE RINGS"),
    (0x07D4, "MY EARS ARE FRAGILE"),
    (0x07EA, "DO YOU WANT ME TO EXPLAIN YOUR MISSION"),
)
DEFAULT_CONTACT_MANIFEST = ROOT / "re/vm/contact-manifest/contact-manifest.json"

DIALOGUE_AUDIO_OFFSETS = {
    "voc_playback_enabled": (0x0ADE, "B"),
    "game_mode": (0x0ADF, "B"),
    "timer_hook_active": (0x0B21, "B"),
    "timer_tick": (0x0B29, "H"),
    "frame_delay": (0x0B2D, "H"),
    "dialogue_delay": (0x0B33, "H"),
    "dialogue_hold_countdown": (0x0B35, "H"),
    "clip_playback_state": (0x0B39, "H"),
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
    "mouse_button_state": (0x0A2E, "H"),
    "mouse_previous_button_state": (0x0A30, "H"),
    "nav_actor_presentation_state": (0x0A32, "H"),
    "nav_target_presentation_state": (0x0A34, "H"),
    "mouse_last_x": (0x0A38, "h"),
    "mouse_last_y": (0x0A3A, "h"),
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
    "ui_state": (0x2793, "H"),
    "presentation_mode": (0x27E0, "B"),
    "presentation_box_mode": (0x27E1, "B"),
    "presentation_box_phase": (0x2B93, "h"),
    "bridge_view_frame": (0x2795, "h"),
    "nav_target_hover_row": (0x27C7, "B"),
    "word_choice_active": (0x27D7, "B"),
    "nav_target_selection": (0x27E7, "B"),
    "choice_rect_x": (0x2AAB, "h"),
    "choice_rect_y": (0x2AAD, "h"),
    "choice_rect_width": (0x2AAF, "h"),
    "choice_rect_height": (0x2AB1, "h"),
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
CODE_SEGMENT_ROW = re.compile(
    r"^(?P<name>\S+)\s+CODE\s+\S+\s+"
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


@dataclass(frozen=True)
class ExecutionSample:
    sample: int
    cs: int
    ip: int
    ss: int
    sp: int
    bp: int
    progress: tuple[object, ...]
    waiting_for_input: bool
    game_owned_code: bool


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


def write_binary_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(data)
    temporary.replace(path)


def find_dosbox_fault(data: bytes) -> dict[str, object] | None:
    for kind, pattern in DOSBOX_FAULT_PATTERNS:
        match = pattern.search(data)
        if match is None:
            continue
        line_start = data.rfind(b"\n", 0, match.start()) + 1
        line_end = data.find(b"\n", match.end())
        if line_end < 0:
            line_end = len(data)
        fault: dict[str, object] = {
            "kind": kind,
            "message": data[line_start:line_end].decode(
                "utf-8", errors="replace"
            ),
        }
        if kind == "illegal-interrupt":
            fault["interrupt"] = int(match.group(1), 10)
        return fault
    return None


def scan_dosbox_log(
    path: Path,
    offset: int,
    carry: bytes,
) -> tuple[dict[str, object] | None, int, bytes]:
    if not path.exists():
        return None, offset, carry
    size = path.stat().st_size
    if size < offset:
        offset = 0
        carry = b""
    with path.open("rb") as stream:
        stream.seek(offset)
        chunk = stream.read()
    combined = carry + chunk
    return (
        find_dosbox_fault(combined),
        size,
        combined[-DOSBOX_LOG_CARRY_SIZE:],
    )


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
    return guest_memory_environment_is_plausible(memory)


def guest_memory_environment_is_plausible(memory: bytes) -> bool:
    conventional_kib = struct.unpack_from("<H", memory, 0x0413)[0]
    if not 128 <= conventional_kib <= 640:
        return False
    int_21_offset, int_21_segment = struct.unpack_from("<HH", memory, 0x21 * 4)
    return (int_21_offset | int_21_segment) != 0


def game_data_anchor_is_present(memory: bytes, game_segment: int) -> bool:
    anchor = game_segment * 16
    return memory[anchor : anchor + len(GAME_DATA_ANCHOR)] == GAME_DATA_ANCHOR


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


def load_contact_scenario(path: Path, selector: str) -> dict[str, object]:
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise WatchdogError(
            f"cannot read contact manifest {path}: {type(error).__name__}: {error}"
        ) from error
    if not isinstance(manifest, dict) or manifest.get("format_version") != 1:
        raise WatchdogError(f"unsupported contact manifest: {path}")
    try:
        script_name, procedure_selector = selector.split(":", 1)
    except ValueError as error:
        raise WatchdogError(
            "contact scenario must be SCRIPTn:procedure[@offset], "
            f"got {selector!r}"
        ) from error
    procedure_name, separator, offset_text = procedure_selector.rpartition("@")
    procedure_offset = None
    if separator:
        try:
            procedure_offset = int(offset_text, 16)
        except ValueError as error:
            raise WatchdogError(
                f"contact scenario has invalid procedure offset: {selector!r}"
            ) from error
    else:
        procedure_name = procedure_selector
    procedures = manifest.get("procedures")
    if not isinstance(procedures, list):
        raise WatchdogError("contact manifest has no procedure list")
    matches = [
        procedure
        for procedure in procedures
        if isinstance(procedure, dict)
        and procedure.get("script") == script_name
        and procedure.get("procedure") == procedure_name
        and (
            procedure_offset is None
            or procedure.get("procedure_offset") == procedure_offset
        )
    ]
    if len(matches) != 1:
        raise WatchdogError(
            f"contact scenario {selector!r} resolved to {len(matches)} procedures"
        )
    match = dict(matches[0])
    script_match = re.fullmatch(r"SCRIPT([1-5])", script_name)
    if script_match is None:
        raise WatchdogError(f"invalid contact script name: {script_name!r}")
    profile_procedures = [
        procedure
        for procedure in procedures
        if isinstance(procedure, dict) and procedure.get("script") == script_name
    ]
    texts = match.get("texts")
    if not isinstance(texts, list) or not texts:
        raise WatchdogError(f"contact scenario {selector!r} has no text tokens")
    word_offsets = [
        text.get("word_list_offset")
        for text in texts
        if isinstance(text, dict)
    ]
    if (
        len(word_offsets) != len(texts)
        or any(not isinstance(offset, int) for offset in word_offsets)
        or len(set(word_offsets)) != len(word_offsets)
    ):
        raise WatchdogError(
            f"contact scenario {selector!r} has invalid text word-list offsets"
        )
    match["selector"] = selector
    match["profile"] = int(script_match.group(1)) - 1
    match["profile_procedures"] = profile_procedures
    match["text_by_word_offset"] = {
        int(text["word_list_offset"]): text for text in texts
    }
    return match


def plan_contact_predicate_writes(
    scenario: dict[str, object], records: bytes | bytearray
) -> dict[str, object]:
    entry_tokens = scenario.get("entry_tokens")
    if not isinstance(entry_tokens, list):
        raise WatchdogError("contact scenario has no entry token list")
    values: dict[int, int] = {}
    reasons: dict[int, list[str]] = {}
    timers: set[int] = set()

    def current_word(offset: int) -> int:
        if offset < 0 or offset + 2 > len(records):
            raise WatchdogError(
                f"contact predicate record offset {offset:#06x} is outside VAR"
            )
        return values.get(offset, struct.unpack_from("<H", records, offset)[0])

    def set_word(offset: int, value: int, reason: str) -> None:
        current_word(offset)
        values[offset] = value & 0xFFFF
        reasons.setdefault(offset, []).append(reason)

    for entry in entry_tokens:
        if not isinstance(entry, dict):
            raise WatchdogError("contact entry token is not an object")
        kind = entry.get("kind")
        token_wrapper = entry.get("token")
        if not isinstance(token_wrapper, dict) or len(token_wrapper) != 1:
            raise WatchdogError("contact entry token has invalid typed payload")
        variant, token = next(iter(token_wrapper.items()))
        if not isinstance(token, dict):
            raise WatchdogError("contact entry token payload is not an object")
        if kind == "actor" and variant == "Actor":
            continue
        if kind == "guard_push" and variant == "GuardPush":
            continue
        if kind == "state_array" and variant == "StateArray":
            index = token.get("index")
            if not isinstance(index, int) or token.get("value") is not None:
                raise WatchdogError("unsupported contact state-array predicate")
            timers.add(index)
            continue
        if kind == "shared_state" and variant == "SharedState":
            if (
                token.get("opcode") not in (0xBF, 0xC0)
                or token.get("operator") != 0xF5
                or token.get("rhs_mode") != 0xC1
            ):
                raise WatchdogError("unsupported contact shared-state predicate")
            offset = token.get("field_offset")
            rhs = token.get("rhs")
            if not isinstance(offset, int) or not isinstance(rhs, int):
                raise WatchdogError("malformed contact shared-state predicate")
            set_word(offset, rhs, "shared_state_equal")
            continue
        if kind == "shared_bit_state" and variant == "SharedBitState":
            if token.get("opcode") not in (0xAE, 0xB0):
                raise WatchdogError("unsupported contact shared-bit predicate")
            offset = token.get("field_offset")
            mask = token.get("mask")
            inverted = token.get("inverted")
            if (
                not isinstance(offset, int)
                or not isinstance(mask, int)
                or not isinstance(inverted, bool)
            ):
                raise WatchdogError("malformed contact shared-bit predicate")
            value = current_word(offset)
            value = value & ~mask if inverted else value | mask
            set_word(offset, value, "shared_bit_clear" if inverted else "shared_bit_set")
            continue
        if kind == "record_wildcard" and variant == "RecordWildcard":
            if token.get("opcode") != 0xAF:
                raise WatchdogError("unsupported contact record predicate")
            offset = token.get("record_offset")
            expected = token.get("value")
            inverted = token.get("inverted")
            if (
                not isinstance(offset, int)
                or not isinstance(expected, int)
                or not isinstance(inverted, bool)
            ):
                raise WatchdogError("malformed contact record predicate")
            value = current_word(offset)
            if inverted and value == expected:
                value = 0 if expected != 0 else 0xFFFF
            elif not inverted:
                value = expected
            set_word(offset, value, "record_not_equal" if inverted else "record_equal")
            continue
        raise WatchdogError(
            f"unsupported contact entry token {kind!r}/{variant!r}"
        )

    for entry in entry_tokens:
        assert isinstance(entry, dict)
        token_wrapper = entry["token"]
        variant, token = next(iter(token_wrapper.items()))
        if variant == "SharedState":
            if current_word(token["field_offset"]) != token["rhs"]:
                raise WatchdogError("contact shared-state predicates conflict")
        elif variant == "SharedBitState":
            matched = current_word(token["field_offset"]) & token["mask"] != 0
            if matched == token["inverted"]:
                raise WatchdogError("contact shared-bit predicates conflict")
        elif variant == "RecordWildcard":
            matched = current_word(token["record_offset"]) == token["value"]
            if matched == token["inverted"]:
                raise WatchdogError("contact record predicates conflict")

    return {
        "record_writes": [
            {
                "offset": offset,
                "before": struct.unpack_from("<H", records, offset)[0],
                "after": value,
                "reasons": reasons[offset],
            }
            for offset, value in sorted(values.items())
        ],
        "timer_indices": sorted(timers),
    }


def apply_contact_scenario(
    mem,
    guest_base: int,
    game_segment: int,
    profile_state: ProfileState,
    scenario: dict[str, object],
    memory: bytes,
) -> dict[str, object]:
    if profile_state.profile != scenario["profile"]:
        raise WatchdogError("contact scenario profile is not loaded")
    cod_offset, cod_segment = profile_state.images[0]
    record_offset, record_segment = profile_state.images[2]
    if cod_segment == 0 or record_segment == 0:
        raise WatchdogError("contact scenario COD or VAR image is unavailable")
    cod_address = guest_base + cod_segment * 16 + cod_offset
    records_address = guest_base + record_segment * 16 + record_offset
    records_offset = record_segment * 16 + record_offset
    plan = plan_contact_predicate_writes(scenario, memory[records_offset:])

    profile_procedures = scenario["profile_procedures"]
    assert isinstance(profile_procedures, list)
    activation_writes = []
    for procedure in profile_procedures:
        assert isinstance(procedure, dict)
        procedure_offset = procedure.get("procedure_offset")
        if not isinstance(procedure_offset, int):
            raise WatchdogError("contact procedure has no activation offset")
        enabled = procedure_offset == scenario["procedure_offset"]
        exact_write(
            mem,
            cod_address + procedure_offset + 1,
            b"\x01" if enabled else b"\0",
        )
        activation_writes.append(
            {
                "procedure": procedure.get("procedure"),
                "offset": procedure_offset + 1,
                "enabled": enabled,
            }
        )

    for write in plan["record_writes"]:
        assert isinstance(write, dict)
        exact_write(
            mem,
            records_address + int(write["offset"]),
            struct.pack("<H", int(write["after"])),
        )

    contact_object = scenario.get("contact_object_offset")
    if not isinstance(contact_object, int):
        raise WatchdogError("contact scenario has no object offset")
    active_records = sorted({contact_object, 0x0028})
    planned_values = {
        int(write["offset"]): int(write["after"])
        for write in plan["record_writes"]
    }
    active_writes = []
    for object_offset in active_records:
        flags_offset = records_offset + object_offset + 2
        before = struct.unpack_from("<H", memory, flags_offset)[0]
        after = planned_values.get(object_offset + 2, before) | 1
        exact_write(
            mem,
            records_address + object_offset + 2,
            struct.pack("<H", after),
        )
        active_writes.append(
            {"object_offset": object_offset, "before": before, "after": after}
        )

    game_address = guest_base + game_segment * 16
    for index in plan["timer_indices"]:
        exact_write(
            mem,
            game_address + VM_STATE_ARRAY_OFFSET + int(index) * 2,
            b"\0\0",
        )
    exact_write(
        mem,
        game_address + 0x676A,
        struct.pack("<H", contact_object),
    )
    exact_write(mem, game_address + 0x2751, b"\x01")
    plan["activation_writes"] = activation_writes
    plan["active_writes"] = active_writes
    plan["selected_object"] = contact_object
    return plan


def read_contact_probe_state(
    memory: bytes,
    game_segment: int,
    profile_state: ProfileState,
    scenario: dict[str, object],
) -> dict[str, object] | None:
    if profile_state.profile != scenario["profile"]:
        return None
    cod_offset, cod_segment = profile_state.images[0]
    record_offset, record_segment = profile_state.images[2]
    if cod_segment == 0 or record_segment == 0:
        return None
    procedure_offset = scenario["procedure_offset"]
    contact_object = scenario["contact_object_offset"]
    assert isinstance(procedure_offset, int)
    assert isinstance(contact_object, int)
    cod = cod_segment * 16 + cod_offset
    records = record_segment * 16 + record_offset
    state = {
        "procedure_enabled": memory[cod + procedure_offset + 1],
        "contact_action": memory[
            records + contact_object + 0x3A:
            records + contact_object + 0x3A + 6
        ].hex(),
    }
    state.update(read_active_vm_subtitle(memory, game_segment))
    state["word_list_known"] = (
        state["menu_words_offset"] in scenario["text_by_word_offset"]
    )
    return state


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


def presentation_progress_key(state: dict[str, int]) -> tuple[int, ...]:
    """Fields that must change when an active word-choice consumes input."""
    return tuple(
        state[name]
        for name in (
            "word_choice_active",
            "presentation_text_wait",
            "active_line",
            "displayed_line",
            "nav_target_selection",
            "presentation_active",
            "presentation_defer",
            "text_display_active",
        )
    )


def word_choice_waiting_for_input(state: dict[str, int]) -> bool:
    return (
        (state["word_choice_active"] & 1) != 0
        and state["presentation_text_wait"] == 2
        and (state["mouse_primary_pressed"] & 1) == 0
        and (state["mouse_press_pending"] & 1) == 0
        and state["nav_target_selection"] == 0
    )


def word_choice_input_attempted(
    previous: dict[str, int], current: dict[str, int]
) -> bool:
    return word_choice_waiting_for_input(previous) and (
        (current["mouse_button_state"] & 1) != 0
        or (current["mouse_primary_pressed"] & 1) != 0
        or (current["mouse_press_pending"] & 1) != 0
    )


def word_choice_input_consumed(
    progress_key: tuple[int, ...], current: dict[str, int]
) -> bool:
    return presentation_progress_key(current) != progress_key


def active_presentation_progress_key(
    presentation: dict[str, int], audio: dict[str, int]
) -> tuple[int, ...]:
    """State that must eventually change while presentation work is active."""
    presentation_fields = (
        "list_state",
        "list_read_wrap_index",
        "list_wrap_count",
        "list_read_wrap_limit",
        "resource_source_offset",
        "resource_source_remaining",
        "list_head_offset",
        "list_tail_offset",
        "list_queued_bytes",
        "list_iteration_count",
        "list_entry_metric",
        "c2_presentation_gate",
        "presentation_box_phase",
        "text_reveal_phase",
        "active_line",
        "displayed_line",
        "presentation_request_flags",
        "presentation_active",
        "presentation_defer",
        "presentation_start_lock",
        "presentation_text_wait",
        "dialogue_hold_complete",
        "presentation_hold_ready",
    )
    audio_fields = (
        "dialogue_delay",
        "dialogue_hold_countdown",
        "clip_playback_state",
        "bank_clip_count",
        "last_clip",
        "streamed_clip_count",
        "text_mode_play",
        "text_voice_trigger",
    )
    return tuple(presentation[name] for name in presentation_fields) + tuple(
        audio[name] for name in audio_fields
    )


def presentation_work_is_active(
    presentation: dict[str, int], audio: dict[str, int]
) -> bool:
    if word_choice_waiting_for_input(presentation):
        return False
    return any(
        (
            presentation["c2_presentation_gate"] & 1,
            presentation["presentation_box_mode"] & 1,
            presentation["text_display_active"] & 1,
            presentation["presentation_active"] & 1,
            presentation["presentation_defer"] & 1,
            presentation["presentation_start_lock"] & 1,
            audio["clip_playback_state"],
        )
    )


def execution_progress_key(
    profile: ProfileState,
    presentation: dict[str, int],
    audio: dict[str, int],
) -> tuple[object, ...]:
    return (
        profile.profile,
        profile.request,
        profile.execution_enabled,
        profile.handles,
        profile.images,
        profile.blockers,
        active_presentation_progress_key(presentation, audio),
        presentation["ui_state"],
        presentation["presentation_mode"],
        presentation["presentation_box_mode"],
        presentation["bridge_view_frame"],
        presentation["nav_target_selection"],
        presentation["nav_pending_record_link"],
    )


def classify_execution_stall(
    samples: list[ExecutionSample] | deque[ExecutionSample],
    load_segment: int,
) -> dict[str, object] | None:
    """Recognize a frozen, non-main hot loop without flagging input waits."""
    if not samples:
        return None
    if any(sample.waiting_for_input for sample in samples):
        return None
    if not all(sample.game_owned_code for sample in samples):
        return None
    if any(sample.cs == load_segment for sample in samples):
        return None
    if len({sample.cs for sample in samples}) != 1:
        return None
    if len({(sample.ss, sample.sp, sample.bp) for sample in samples}) != 1:
        return None
    if len({sample.progress for sample in samples}) != 1:
        return None
    ip_counts = Counter(sample.ip for sample in samples)
    if len(ip_counts) > MAX_HOT_LOOP_IPS:
        return None
    first = samples[0]
    last = samples[-1]
    return {
        "reason": "game-owned execution hot loop with frozen runtime state",
        "first_sample": first.sample,
        "last_sample": last.sample,
        "sample_count": len(samples),
        "cs": f"{first.cs:#06x}",
        "ss": f"{first.ss:#06x}",
        "sp": f"{first.sp:#06x}",
        "bp": f"{first.bp:#06x}",
        "distinct_ips": [
            f"{ip:#06x}" for ip in sorted(ip_counts)
        ],
        "ip_histogram": {
            f"{ip:#06x}": count
            for ip, count in ip_counts.most_common()
        },
    }


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


def read_active_vm_subtitle(
    memory: bytes,
    game_segment: int,
) -> dict[str, object]:
    game = game_segment * 16
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
        "subtitle": subtitle,
        "buffered_subtitle": buffered_subtitle,
        "menu_subtitle": menu_subtitle,
        "menu_words": f"{menu_words_segment:04x}:{menu_words_offset:04x}",
        "menu_words_offset": menu_words_offset,
        "dictionary": f"{dictionary_segment:04x}:{dictionary_offset:04x}",
    }


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
    state = {
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
    }
    state.update(read_active_vm_subtitle(memory, game_segment))
    return state


def read_script1_bob_state(
    memory: bytes,
    game_segment: int,
    profile_state: ProfileState,
) -> dict[str, object] | None:
    if profile_state.profile != SCRIPT1_PROFILE:
        return None
    cod_offset, cod_segment = profile_state.images[0]
    record_offset, record_segment = profile_state.images[2]
    if cod_segment == 0 or record_segment == 0:
        return None
    cod = cod_segment * 16 + cod_offset
    records = record_segment * 16 + record_offset
    game = game_segment * 16
    state = {
        "bob1_enabled": memory[cod + SCRIPT1_BOB_PROCEDURE_FLAG_OFFSET],
        "bob_action": memory[
            records + SCRIPT1_BOB_ACTION_OFFSET:
            records + SCRIPT1_BOB_ACTION_OFFSET + 6
        ].hex(),
        "nav_pending_record_link": struct.unpack_from(
            "<H", memory, game + 0x675A
        )[0],
        "deferred_record": struct.unpack_from(
            "<HHH", memory, game + 0x6768
        ),
    }
    state.update(read_active_vm_subtitle(memory, game_segment))
    return state


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


def snapshot_guest(
    mem,
    guest_base: int,
    game_anchor: int,
    cpu_addresses: dict[str, int],
    marker: Path | None,
    profile: dict[str, object] | None,
) -> dict[str, object]:
    state = read_cpu_state(mem, cpu_addresses)
    game_segment = (game_anchor - guest_base) // 16
    memory = exact_read(mem, guest_base, GUEST_SNAPSHOT_SIZE)
    return snapshot_guest_memory(
        memory,
        state,
        game_segment,
        marker,
        profile,
    )


def snapshot_guest_memory(
    memory: bytes,
    state: dict[str, int],
    game_segment: int,
    marker: Path | None,
    profile: dict[str, object] | None,
) -> dict[str, object]:
    snapshot: dict[str, object] = {
        "cpu": cpu_for_report(state),
        "segments_minus_game_data": {
            name: state[name] - game_segment
            for name in ("cs", "ds", "es", "ss", "fs", "gs")
        },
    }

    def guest_bytes(linear: int, size: int) -> str | None:
        if linear < 0 or linear + size > len(memory):
            return None
        return memory[linear : linear + size].hex()

    code = guest_bytes(state["cs"] * 16 + state["ip"], 32)
    if code is not None:
        snapshot["code_at_cs_ip"] = code
    stack = guest_bytes(state["ss"] * 16 + state["sp"], 256)
    if stack is not None:
        snapshot["stack_at_ss_sp"] = stack
    bp_linear = state["ss"] * 16 + state["bp"]
    around_bp = guest_bytes(bp_linear - 64, 256)
    if around_bp is not None:
        snapshot["stack_around_ss_bp"] = {
            "start_offset": (state["bp"] - 64) & 0xFFFF,
            "bytes": around_bp,
        }
    snapshot["ivt"] = memory[:0x400].hex()
    game_anchor = game_segment * 16
    snapshot["resource_band"] = {
        f"{offset:#06x}": struct.unpack_from(
            "<H", memory, game_anchor + offset
        )[0]
        for offset in range(0x0A40, 0x0B00, 2)
    }
    snapshot["back_buffer_area"] = memory[
        game_anchor + 0x5219 : game_anchor + 0x5240
    ].hex()
    if marker is not None:
        snapshot["marker"] = str(marker)
    if profile is not None:
        snapshot["profile"] = profile
    return snapshot


def mcb_for_report(block: Mcb) -> dict[str, str | int]:
    return {
        "segment": f"{block.segment:#06x}",
        "kind": block.kind,
        "owner": f"{block.owner:#06x}",
        "paragraphs": block.paragraphs,
        "name": block.name,
    }


def code_region_at(path: Path, offset: int) -> dict[str, object] | None:
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = CODE_SEGMENT_ROW.match(line.strip())
        if match is None or int(match["segment"], 16) != 0:
            continue
        start = int(match["offset"], 16)
        size = int(match["size"], 16)
        if start <= offset < start + size:
            return {
                "name": match["name"],
                "start": f"{start:#06x}",
                "size": size,
                "offset_in_region": f"{offset - start:#06x}",
                "map": str(path),
            }
    return None


def mz_image_offset(path: Path, file_offset: int) -> int | None:
    header = path.read_bytes()[:0x1C]
    if len(header) < 0x1C or header[:2] != b"MZ":
        return None
    header_size = struct.unpack_from("<H", header, 8)[0] * 16
    if file_offset < header_size:
        return None
    return file_offset - header_size


def candidate_code_images(cd_dir: Path) -> list[Path]:
    package = cd_dir.parent
    roots = (
        cd_dir,
        package / "xdb",
        package / "validation/source_xdb",
        ROOT / "output/_tmp_dat",
    )
    paths: list[Path] = []
    seen: set[Path] = set()
    for root in roots:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path.suffix.lower() not in (".exe", ".xdb"):
                continue
            resolved = path.resolve()
            if resolved not in seen:
                seen.add(resolved)
                paths.append(resolved)
    return paths


def match_code_images(
    needle: bytes,
    cd_dir: Path,
    link_map: Path,
) -> list[dict[str, object]]:
    if len(needle) < 16:
        return []
    matches = []
    for path in candidate_code_images(cd_dir):
        data = path.read_bytes()
        offset = data.find(needle)
        if offset < 0:
            continue
        match: dict[str, object] = {
            "path": str(path),
            "file_offset": f"{offset:#08x}",
        }
        map_path = None
        logical_offset = offset
        if path.suffix.lower() == ".exe":
            logical_offset = mz_image_offset(path, offset)
            if path.name.upper() == "BPRG_RE.EXE":
                map_path = link_map
        else:
            module = path.stem.lower()
            candidate_map = (
                cd_dir.parent
                / "validation/source_xdb"
                / module
                / f"{module}_source_link.map"
            )
            if candidate_map.is_file():
                map_path = candidate_map
        if logical_offset is not None:
            match["image_offset"] = f"{logical_offset:#06x}"
            if map_path is not None:
                region = code_region_at(map_path, logical_offset)
                if region is not None:
                    match["code_region"] = region
        matches.append(match)
    return matches[:20]


def describe_execution_location(
    memory: bytes,
    state: dict[str, int],
    load_segment: int,
    blocks: list[Mcb],
    cd_dir: Path,
    link_map: Path,
) -> dict[str, object]:
    linear = state["cs"] * 16 + state["ip"]
    location: dict[str, object] = {
        "guest_address": f"{state['cs']:04x}:{state['ip']:04x}",
        "guest_linear": f"{linear:#07x}",
        "fs_minus_cs": state["fs"] - state["cs"],
    }
    owner = next(
        (block for block in blocks if block.owns_segment(state["cs"])),
        None,
    )
    if owner is not None:
        location["mcb_owner"] = mcb_for_report(owner)
    if state["cs"] == load_segment:
        region = code_region_at(link_map, state["ip"])
        if region is not None:
            location["code_region"] = region
    needle = memory[linear : linear + 24]
    location["binary_matches"] = match_code_images(
        needle, cd_dir, link_map
    )
    return location


def write_crash_bundle(
    directory: Path,
    memory: bytes,
    context: dict[str, object],
) -> dict[str, str]:
    guest_path = directory / "guest.bin"
    context_path = directory / "context.json"
    write_binary_atomic(guest_path, memory)
    write_json_report(context_path, context)
    return {
        "directory": str(directory.resolve()),
        "guest_memory": str(guest_path.resolve()),
        "context": str(context_path.resolve()),
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
    parser.add_argument(
        "--script1-bob-probe",
        action="store_true",
        help=(
            "load GAME1.SAV to enter the bridge, switch to SCRIPT1, select "
            "Bob through the normal scene transition, and require his first "
            "four dialogue checkpoints"
        ),
    )
    parser.add_argument(
        "--contact-probe",
        metavar="SCRIPT:PROCEDURE",
        help=(
            "load GAME1.SAV, switch to the manifest procedure's profile, "
            "satisfy its recovered entry predicates, and drive its normal contact transition"
        ),
    )
    parser.add_argument(
        "--contact-manifest",
        type=Path,
        default=DEFAULT_CONTACT_MANIFEST,
        help=f"binary-derived contact manifest (default: {DEFAULT_CONTACT_MANIFEST})",
    )
    parser.add_argument(
        "--contact-min-lines",
        type=int,
        default=4,
        help="valid procedure lines required before a contact probe completes (default: 4)",
    )
    parser.add_argument(
        "--input-liveness-samples",
        type=int,
        default=0,
        help=(
            "fail when a sampled click while a word-choice menu is waiting does "
            "not advance presentation state within this many guarded samples"
        ),
    )
    parser.add_argument(
        "--active-liveness-samples",
        type=int,
        default=0,
        help=(
            "fail when noninteractive presentation work makes no queue, text, "
            "VM, or audio progress for this many guarded samples"
        ),
    )
    parser.add_argument(
        "--hang-samples",
        type=int,
        default=120,
        help=(
            "capture a crash bundle when game-owned non-main code remains on "
            "one stack frame with frozen runtime state for this many samples "
            "(default: 120; zero disables)"
        ),
    )
    parser.add_argument(
        "--dosbox-log",
        type=Path,
        help="capture DOSBox output and treat fatal guest diagnostics as faults",
    )
    parser.add_argument(
        "--crash-dir",
        type=Path,
        help=(
            "crash bundle directory; defaults beside --report and contains "
            "guest.bin plus context.json"
        ),
    )
    args = parser.parse_args()

    cd_dir = args.cd_dir.resolve()
    install_parent = args.install_parent.resolve()
    link_map = (args.link_map or default_link_map(cd_dir)).resolve()
    layout = parse_segment_layout(link_map)
    if args.stable_samples < 1:
        raise WatchdogError("--stable-samples must be positive")
    if args.poll_seconds <= 0:
        raise WatchdogError("--poll-seconds must be positive")
    if args.post_teleport_samples < 1:
        raise WatchdogError("--post-teleport-samples must be positive")
    if args.input_liveness_samples < 0:
        raise WatchdogError("--input-liveness-samples cannot be negative")
    if args.active_liveness_samples < 0:
        raise WatchdogError("--active-liveness-samples cannot be negative")
    if args.hang_samples < 0:
        raise WatchdogError("--hang-samples cannot be negative")
    if args.contact_min_lines < 1:
        raise WatchdogError("--contact-min-lines must be positive")
    if (
        args.teleport_profile is not None
        and not 0 <= args.teleport_profile < VM_PROFILE_COUNT
    ):
        raise WatchdogError(
            f"teleport profile must be in 0..4: {args.teleport_profile}"
        )
    dialogue_probe_count = sum(
        bool(probe)
        for probe in (
            args.script2_radio_probe,
            args.script1_bob_probe,
            args.contact_probe,
        )
    )
    if dialogue_probe_count:
        if args.teleport_profile is not None:
            raise WatchdogError(
                "dialogue probes cannot be combined with --teleport-profile"
            )
    if dialogue_probe_count > 1:
        raise WatchdogError(
            "dialogue probe modes are mutually exclusive"
        )
    if dialogue_probe_count:
        save_path = install_parent / "cblood" / "GAME1.SAV"
        if not save_path.is_file():
            raise WatchdogError(
                f"dialogue probes require {save_path}"
            )
    contact_scenario = (
        None
        if args.contact_probe is None
        else load_contact_scenario(
            args.contact_manifest.resolve(), args.contact_probe
        )
    )
    dosbox_log = args.dosbox_log
    if dosbox_log is None and args.report is not None:
        dosbox_log = args.report.with_suffix(".dosbox.log")
    if dosbox_log is not None:
        dosbox_log = dosbox_log.resolve()
    crash_dir = args.crash_dir
    if crash_dir is None and args.report is not None:
        crash_dir = args.report.with_name(args.report.stem + "-crash")
    if crash_dir is not None:
        crash_dir = crash_dir.resolve()

    report: dict[str, object] = {
        "verdict": "INCOMPLETE",
        "samples": 0,
        "guarded_samples": 0,
        "anomalies": [],
        "recorder": {
            "watchdog_pid": os.getpid(),
            "manual_snapshot_signal": "SIGUSR1",
            "hang_samples": args.hang_samples,
            "dosbox_log": None if dosbox_log is None else str(dosbox_log),
            "crash_dir": None if crash_dir is None else str(crash_dir),
        },
    }
    env = dict(os.environ, DISPLAY=args.display, SDL_VIDEODRIVER="x11")
    xvfb = None
    dosbox = None
    log_stream = None
    attached = False
    radio_physical_mouse_held = False
    driver_errors: list[str] = []
    libc = ptrace_libc()
    snapshot_requested = threading.Event()
    previous_sigusr1 = signal.getsignal(signal.SIGUSR1)
    signal.signal(signal.SIGUSR1, lambda _signum, _frame: snapshot_requested.set())
    last_guest_memory: bytes | None = None
    last_guest_context: dict[str, object] | None = None
    last_guest_state: dict[str, int] | None = None
    last_guest_blocks: list[Mcb] = []
    pending_dosbox_fault: dict[str, object] | None = None
    log_offset = 0
    log_carry = b""

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
        if dosbox_log is not None:
            dosbox_log.parent.mkdir(parents=True, exist_ok=True)
            log_stream = dosbox_log.open("wb", buffering=0)
        dosbox = subprocess.Popen(
            dosbox_args,
            env=env,
            stdout=(subprocess.DEVNULL if log_stream is None else log_stream),
            stderr=(subprocess.DEVNULL if log_stream is None else subprocess.STDOUT),
        )
        report["recorder"]["dosbox_pid"] = dosbox.pid

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
        last_anchor_present = None
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
        radio_checkpoint_index = 0
        bob_probe_phase = "wait-title-idle"
        bob_load_slot_pressed = False
        bob_intro_seen = False
        bob_bridge_idle_started = None
        bob_profile_reload_requested = False
        bob_lines: list[str] = []
        bob_checkpoint_index = 0
        contact_probe_phase = "wait-title-idle"
        contact_load_slot_pressed = False
        contact_intro_seen = False
        contact_bridge_idle_started = None
        contact_profile_reload_requested = False
        contact_staging_profile = None
        contact_setup_applied = False
        contact_started = False
        contact_last_word_offset = None
        contact_checkpoints: list[dict[str, object]] = []
        previous_presentation_state = None
        input_attempt = None
        active_progress = None
        execution_samples: deque[ExecutionSample] = deque(
            maxlen=max(1, args.hang_samples)
        )
        report_checkpoint_samples = max(
            1, round(1.0 / args.poll_seconds)
        )

        while time.monotonic() < deadline:
            sample_delay = args.poll_seconds
            time.sleep(sample_delay)
            if dosbox_log is not None and pending_dosbox_fault is None:
                (
                    pending_dosbox_fault,
                    log_offset,
                    log_carry,
                ) = scan_dosbox_log(dosbox_log, log_offset, log_carry)
            if driver_errors:
                report["verdict"] = "DRIVER-ERROR"
                report["error"] = driver_errors[0]
                break
            if dosbox.poll() is not None:
                report["exit_code"] = dosbox.returncode
                if pending_dosbox_fault is not None:
                    report["verdict"] = "DOSBOX-FAULT"
                    if (
                        last_guest_context is not None
                        and last_guest_memory is not None
                        and last_guest_state is not None
                        and expected is not None
                    ):
                        last_guest_context["execution_location"] = (
                            describe_execution_location(
                                last_guest_memory,
                                last_guest_state,
                                int(expected["load_segment"]),
                                last_guest_blocks,
                                cd_dir,
                                link_map,
                            )
                        )
                    anomaly = {
                        "sample": report["guarded_samples"],
                        "issues": [
                            "dosbox-log-fault="
                            + str(pending_dosbox_fault["kind"])
                        ],
                        "dosbox_fault": pending_dosbox_fault,
                        "last_runtime": report.get("last_runtime"),
                        "guest_context": last_guest_context,
                    }
                    anomalies = report["anomalies"]
                    assert isinstance(anomalies, list)
                    anomalies.append(anomaly)
                    if (
                        crash_dir is not None
                        and last_guest_memory is not None
                        and last_guest_context is not None
                    ):
                        report["crash_artifacts"] = write_crash_bundle(
                            crash_dir,
                            last_guest_memory,
                            {
                                "verdict": report["verdict"],
                                "anomaly": anomaly,
                                "calibrated": report.get("calibrated"),
                                "last_runtime": report.get("last_runtime"),
                                "runtime_samples": report.get("runtime_samples"),
                                "dosbox_log": (
                                    None
                                    if dosbox_log is None
                                    else str(dosbox_log)
                                ),
                            },
                        )
                else:
                    report["verdict"] = (
                        "CLEAN-EXIT"
                        if (
                            expected is not None
                            and dosbox.returncode == 0
                            and args.teleport_profile is None
                            and not args.script2_radio_probe
                            and not args.script1_bob_probe
                            and not args.contact_probe
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
                        report["last_calibration_cpu"] = cpu_for_report(state)
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
                    if not guest_memory_environment_is_plausible(memory):
                        issues.append("guest-memory-environment-invalid")

                    anchor_present = game_data_anchor_is_present(
                        memory, int(expected["game_segment"])
                    )
                    if anchor_present != last_anchor_present:
                        last_anchor_present = anchor_present
                        anchor_transitions = report.setdefault(
                            "game_data_anchor_transitions", []
                        )
                        assert isinstance(anchor_transitions, list)
                        game = int(expected["game_segment"]) * 16
                        anchor_transitions.append(
                            {
                                "sample": int(report["guarded_samples"]) + 1,
                                "present": anchor_present,
                                "bytes": memory[
                                    game : game + len(GAME_DATA_ANCHOR)
                                ].hex(),
                            }
                        )

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
                    guarded_sample = int(report["guarded_samples"])
                    if args.input_liveness_samples > 0:
                        if input_attempt is not None:
                            attempt_sample, attempt_key = input_attempt
                            if word_choice_input_consumed(
                                attempt_key, presentation_state
                            ):
                                attempts = report.setdefault(
                                    "input_liveness", []
                                )
                                assert isinstance(attempts, list)
                                attempts.append(
                                    {
                                        "pressed_sample": attempt_sample,
                                        "consumed_sample": guarded_sample,
                                        "status": "consumed",
                                    }
                                )
                                input_attempt = None
                            elif (
                                guarded_sample - attempt_sample
                                >= args.input_liveness_samples
                            ):
                                issues.append(
                                    "word-choice-input-not-consumed="
                                    f"pressed:{attempt_sample},"
                                    f"current:{guarded_sample}"
                                )
                                diagnostics["presentation_flow"] = (
                                    presentation_state
                                )
                        if (
                            input_attempt is None
                            and previous_presentation_state is not None
                            and word_choice_input_attempted(
                                previous_presentation_state,
                                presentation_state,
                            )
                        ):
                            input_attempt = (
                                guarded_sample,
                                presentation_progress_key(
                                    previous_presentation_state
                                ),
                            )
                        previous_presentation_state = presentation_state
                    if args.active_liveness_samples > 0:
                        if presentation_work_is_active(
                            presentation_state, audio_state
                        ):
                            progress_key = active_presentation_progress_key(
                                presentation_state, audio_state
                            )
                            if (
                                active_progress is None
                                or active_progress[1] != progress_key
                            ):
                                active_progress = (guarded_sample, progress_key)
                            elif (
                                guarded_sample - active_progress[0]
                                >= args.active_liveness_samples
                            ):
                                issues.append(
                                    "active-presentation-stalled="
                                    f"start:{active_progress[0]},"
                                    f"current:{guarded_sample}"
                                )
                                diagnostics["audio_flow"] = audio_state
                                diagnostics["presentation_flow"] = (
                                    presentation_state
                                )
                        else:
                            active_progress = None
                    execution_stall = None
                    if args.hang_samples > 0:
                        owner = program_owned_block(
                            blocks, state["cs"], int(expected["psp"])
                        )
                        execution_samples.append(
                            ExecutionSample(
                                sample=guarded_sample,
                                cs=state["cs"],
                                ip=state["ip"],
                                ss=state["ss"],
                                sp=state["sp"],
                                bp=state["bp"],
                                progress=execution_progress_key(
                                    profile_state,
                                    presentation_state,
                                    audio_state,
                                ),
                                waiting_for_input=word_choice_waiting_for_input(
                                    presentation_state
                                ),
                                game_owned_code=owner is not None,
                            )
                        )
                        if len(execution_samples) == args.hang_samples:
                            execution_stall = classify_execution_stall(
                                execution_samples,
                                int(expected["load_segment"]),
                            )
                            if execution_stall is not None:
                                issues.append("execution-hot-loop-stalled")
                                diagnostics["execution_stall"] = execution_stall
                    if pending_dosbox_fault is not None:
                        issues.append(
                            "dosbox-log-fault="
                            + str(pending_dosbox_fault["kind"])
                        )
                        diagnostics["dosbox_fault"] = pending_dosbox_fault
                    if snapshot_requested.is_set():
                        issues.append("manual-snapshot-requested")
                        diagnostics["manual_snapshot"] = {
                            "signal": "SIGUSR1",
                            "watchdog_pid": os.getpid(),
                        }
                    radio_state = read_script2_radio_state(
                        memory,
                        int(expected["game_segment"]),
                        profile_state,
                    )
                    bob_state = read_script1_bob_state(
                        memory,
                        int(expected["game_segment"]),
                        profile_state,
                    )
                    contact_state = (
                        None
                        if contact_scenario is None
                        else read_contact_probe_state(
                            memory,
                            int(expected["game_segment"]),
                            profile_state,
                            contact_scenario,
                        )
                    )
                    report["last_runtime"] = {
                        "cpu": cpu_for_report(state),
                        "profile_state": profile_for_report(profile_state),
                        "audio_flow": audio_state,
                        "presentation_flow": presentation_state,
                        "radio_flow": radio_state,
                        "bob_flow": bob_state,
                        "contact_flow": contact_state,
                    }
                    last_guest_memory = memory
                    last_guest_state = dict(state)
                    last_guest_blocks = list(blocks)
                    last_guest_context = snapshot_guest_memory(
                        memory,
                        state,
                        int(expected["game_segment"]),
                        None,
                        profile_for_report(profile_state),
                    )
                    runtime_samples = report.setdefault("runtime_samples", [])
                    assert isinstance(runtime_samples, list)
                    runtime_samples.append(
                        {
                            "sample": report["guarded_samples"],
                            "cpu": cpu_for_report(state),
                            "profile_state": profile_for_report(profile_state),
                            "audio_flow": audio_state,
                            "presentation_flow": presentation_state,
                            "radio_flow": radio_state,
                            "bob_flow": bob_state,
                            "contact_flow": contact_state,
                        }
                    )
                    del runtime_samples[:-MAX_RUNTIME_SAMPLES]
                    if (
                        args.report
                        and int(report["guarded_samples"])
                        % report_checkpoint_samples
                        == 0
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
                            {
                                "phase": radio_probe_phase,
                                "lines": radio_lines,
                                "checkpoints": [],
                            },
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
                        if radio_checkpoint_index < len(
                            SCRIPT2_RADIO_CHECKPOINTS
                        ):
                            expected_offset, expected_text = (
                                SCRIPT2_RADIO_CHECKPOINTS[
                                    radio_checkpoint_index
                                ]
                            )
                            if (
                                (
                                    expected_offset is None
                                    or radio_state["menu_words_offset"]
                                    == expected_offset
                                )
                                and expected_text in subtitle.upper()
                            ):
                                checkpoints = probe.setdefault(
                                    "checkpoints", []
                                )
                                assert isinstance(checkpoints, list)
                                checkpoints.append(
                                    {
                                        "sample": report[
                                            "guarded_samples"
                                        ],
                                        "menu_words_offset": (
                                            radio_state["menu_words_offset"]
                                        ),
                                        "subtitle": subtitle,
                                    }
                                )
                                radio_checkpoint_index += 1
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
                        elif (
                            "REPORT FROM HONK" in normalized
                            and radio_checkpoint_index
                            == len(SCRIPT2_RADIO_CHECKPOINTS)
                        ):
                            report["verdict"] = "RADIO-PROBE-COMPLETE"
                            probe = report.setdefault("radio_probe", {})
                            assert isinstance(probe, dict)
                            probe["completed_sample"] = report[
                                "guarded_samples"
                            ]
                            break

                    if args.script1_bob_probe:
                        game_offset = int(expected["game_segment"]) * 16
                        game_address = int(expected["guest_base"]) + game_offset
                        blockers = dict(profile_state.blockers)
                        probe = report.setdefault(
                            "bob_probe",
                            {
                                "phase": bob_probe_phase,
                                "lines": bob_lines,
                                "checkpoints": [],
                            },
                        )
                        assert isinstance(probe, dict)

                        if (
                            bob_probe_phase == "wait-title-idle"
                            and profile_state.profile == SCRIPT1_PROFILE
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
                            bob_probe_phase = "wait-load-menu"
                            probe["load_menu_sample"] = report[
                                "guarded_samples"
                            ]
                        elif bob_probe_phase in (
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
                                bob_load_slot_pressed = True
                                bob_probe_phase = "press-load-slot"
                            elif (
                                bob_load_slot_pressed
                                and profile_state.completed(SCRIPT2_PROFILE)
                            ):
                                exact_write(
                                    mem,
                                    game_address + MOUSE_PRIMARY_PRESSED_OFFSET,
                                    b"\0",
                                )
                                bob_probe_phase = "wait-post-load-intro"
                                probe["save_loaded_sample"] = report[
                                    "guarded_samples"
                                ]
                        elif bob_probe_phase == "wait-post-load-intro":
                            if (
                                presentation_state["active_line"] == 2
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 1
                            ):
                                bob_intro_seen = True
                                bob_probe_phase = "dismiss-post-load-intro"
                        elif bob_probe_phase == "dismiss-post-load-intro":
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
                                bob_bridge_idle_started = None
                            elif (
                                bob_intro_seen
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
                                if bob_bridge_idle_started is None:
                                    bob_bridge_idle_started = time.monotonic()
                                if (
                                    time.monotonic() - bob_bridge_idle_started
                                    >= RADIO_BRIDGE_IDLE_SECONDS
                                ):
                                    exact_write(
                                        mem,
                                        game_address
                                        + VM_SCRIPT_PROFILE_REQUEST_OFFSET,
                                        struct.pack("<h", SCRIPT1_PROFILE),
                                    )
                                    bob_profile_reload_requested = True
                                    bob_probe_phase = "wait-profile-reload"
                                    probe["profile_reload_sample"] = report[
                                        "guarded_samples"
                                    ]
                            else:
                                write_primary_press(mem, game_address, False)
                                if radio_physical_mouse_held:
                                    send_mouse_button(args.display, False)
                                    radio_physical_mouse_held = False
                                bob_bridge_idle_started = None
                        elif (
                            bob_probe_phase == "wait-profile-reload"
                            and bob_profile_reload_requested
                            and profile_state.completed(SCRIPT1_PROFILE)
                            and all(value == 0 for value in blockers.values())
                            and presentation_state["presentation_mode"] == 0
                        ):
                            deferred = (
                                presentation_state["deferred_record_type"],
                                presentation_state["deferred_record_related"],
                                presentation_state["deferred_record_value"],
                            )
                            if deferred != (0, 0, 0):
                                issues.append(
                                    "bob-probe-deferred-record-not-idle="
                                    + ":".join(f"{value:#06x}" for value in deferred)
                                )
                            elif bob_state is None:
                                issues.append("bob-probe-script1-state-unavailable")
                            elif bob_state["bob1_enabled"] != 1:
                                issues.append(
                                    "bob-probe-bob1-disabled="
                                    f"{bob_state['bob1_enabled']!r}"
                                )
                            else:
                                exact_write(
                                    mem,
                                    game_address + 0x676A,
                                    struct.pack("<H", SCRIPT1_BOB_OBJECT_OFFSET),
                                )
                                exact_write(mem, game_address + 0x2751, b"\x01")
                                bob_probe_phase = "wait-first-contact"
                                probe["contact_selected_sample"] = report[
                                    "guarded_samples"
                                ]

                        if (
                            bob_probe_phase == "wait-first-contact"
                            and bob_state is not None
                        ):
                            subtitle = str(bob_state["subtitle"])
                            if subtitle and (
                                not bob_lines or bob_lines[-1] != subtitle
                            ):
                                bob_lines.append(subtitle)
                                probe["lines"] = bob_lines
                                line_states = probe.setdefault("line_states", [])
                                assert isinstance(line_states, list)
                                line_states.append(
                                    {
                                        "sample": report["guarded_samples"],
                                        "subtitle": subtitle,
                                        "cpu": cpu_for_report(state),
                                        "audio_flow": audio_state,
                                        "presentation_flow": presentation_state,
                                        "bob_flow": bob_state,
                                    }
                                )
                            if bob_checkpoint_index < len(
                                SCRIPT1_BOB_CHECKPOINTS
                            ):
                                expected_offset, expected_text = (
                                    SCRIPT1_BOB_CHECKPOINTS[
                                        bob_checkpoint_index
                                    ]
                                )
                                if (
                                    bob_state["menu_words_offset"]
                                    == expected_offset
                                    and expected_text in subtitle.upper()
                                ):
                                    checkpoints = probe.setdefault(
                                        "checkpoints", []
                                    )
                                    assert isinstance(checkpoints, list)
                                    checkpoints.append(
                                        {
                                            "sample": report[
                                                "guarded_samples"
                                            ],
                                            "menu_words_offset": (
                                                expected_offset
                                            ),
                                            "subtitle": subtitle,
                                        }
                                    )
                                    bob_checkpoint_index += 1
                            if bob_checkpoint_index == len(
                                SCRIPT1_BOB_CHECKPOINTS
                            ):
                                report["verdict"] = "BOB-PROBE-COMPLETE"
                                probe["completed_sample"] = report[
                                    "guarded_samples"
                                ]
                                break

                        probe["phase"] = bob_probe_phase

                    if contact_scenario is not None:
                        game_offset = int(expected["game_segment"]) * 16
                        game_address = int(expected["guest_base"]) + game_offset
                        blockers = dict(profile_state.blockers)
                        texts = contact_scenario["texts"]
                        text_by_word_offset = contact_scenario[
                            "text_by_word_offset"
                        ]
                        assert isinstance(texts, list)
                        assert isinstance(text_by_word_offset, dict)
                        target_line_count = min(args.contact_min_lines, len(texts))
                        probe = report.setdefault(
                            "contact_probe",
                            {
                                "phase": contact_probe_phase,
                                "selector": contact_scenario["selector"],
                                "script": contact_scenario["script"],
                                "procedure": contact_scenario["procedure"],
                                "procedure_offset": contact_scenario[
                                    "procedure_offset"
                                ],
                                "contact_object": contact_scenario[
                                    "contact_object"
                                ],
                                "contact_object_offset": contact_scenario[
                                    "contact_object_offset"
                                ],
                                "target_line_count": target_line_count,
                                "checkpoints": contact_checkpoints,
                            },
                        )
                        assert isinstance(probe, dict)

                        if (
                            contact_probe_phase == "wait-title-idle"
                            and profile_state.profile == SCRIPT1_PROFILE
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
                            contact_probe_phase = "wait-load-menu"
                            probe["load_menu_sample"] = report[
                                "guarded_samples"
                            ]
                        elif contact_probe_phase in (
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
                                contact_load_slot_pressed = True
                                contact_probe_phase = "press-load-slot"
                            elif (
                                contact_load_slot_pressed
                                and profile_state.completed(SCRIPT2_PROFILE)
                            ):
                                exact_write(
                                    mem,
                                    game_address + MOUSE_PRIMARY_PRESSED_OFFSET,
                                    b"\0",
                                )
                                contact_probe_phase = "wait-post-load-intro"
                                probe["save_loaded_sample"] = report[
                                    "guarded_samples"
                                ]
                        elif contact_probe_phase == "wait-post-load-intro":
                            if (
                                presentation_state["active_line"] == 2
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 1
                            ):
                                contact_intro_seen = True
                                contact_probe_phase = "dismiss-post-load-intro"
                        elif contact_probe_phase == "dismiss-post-load-intro":
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
                                contact_bridge_idle_started = None
                            elif (
                                contact_intro_seen
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
                                if contact_bridge_idle_started is None:
                                    contact_bridge_idle_started = time.monotonic()
                                if (
                                    time.monotonic()
                                    - contact_bridge_idle_started
                                    >= RADIO_BRIDGE_IDLE_SECONDS
                                ):
                                    target_profile = int(
                                        contact_scenario["profile"]
                                    )
                                    contact_staging_profile = (
                                        (target_profile + 1) % VM_PROFILE_COUNT
                                        if profile_state.profile == target_profile
                                        else None
                                    )
                                    requested_profile = (
                                        contact_staging_profile
                                        if contact_staging_profile is not None
                                        else target_profile
                                    )
                                    exact_write(
                                        mem,
                                        game_address
                                        + VM_SCRIPT_PROFILE_REQUEST_OFFSET,
                                        struct.pack(
                                            "<h", requested_profile
                                        ),
                                    )
                                    contact_profile_reload_requested = (
                                        contact_staging_profile is None
                                    )
                                    contact_probe_phase = (
                                        "wait-staging-profile"
                                        if contact_staging_profile is not None
                                        else "wait-profile-reload"
                                    )
                                    probe["profile_request_sample"] = report[
                                        "guarded_samples"
                                    ]
                            else:
                                write_primary_press(mem, game_address, False)
                                if radio_physical_mouse_held:
                                    send_mouse_button(args.display, False)
                                    radio_physical_mouse_held = False
                                contact_bridge_idle_started = None
                        elif (
                            contact_probe_phase == "wait-staging-profile"
                            and contact_staging_profile is not None
                            and profile_state.completed(contact_staging_profile)
                            and all(value == 0 for value in blockers.values())
                            and presentation_state["presentation_mode"] == 0
                        ):
                            exact_write(
                                mem,
                                game_address + VM_SCRIPT_PROFILE_REQUEST_OFFSET,
                                struct.pack(
                                    "<h", int(contact_scenario["profile"])
                                ),
                            )
                            contact_profile_reload_requested = True
                            contact_probe_phase = "wait-profile-reload"
                            probe["staging_profile"] = contact_staging_profile
                            probe["profile_reload_sample"] = report[
                                "guarded_samples"
                            ]
                        elif (
                            contact_probe_phase == "wait-profile-reload"
                            and contact_profile_reload_requested
                            and profile_state.completed(
                                int(contact_scenario["profile"])
                            )
                            and all(value == 0 for value in blockers.values())
                            and presentation_state["presentation_mode"] == 0
                        ):
                            deferred = (
                                presentation_state["deferred_record_type"],
                                presentation_state["deferred_record_related"],
                                presentation_state["deferred_record_value"],
                            )
                            if deferred != (0, 0, 0):
                                issues.append(
                                    "contact-probe-deferred-record-not-idle="
                                    + ":".join(
                                        f"{value:#06x}" for value in deferred
                                    )
                                )
                            else:
                                probe["setup"] = apply_contact_scenario(
                                    mem,
                                    int(expected["guest_base"]),
                                    int(expected["game_segment"]),
                                    profile_state,
                                    contact_scenario,
                                    memory,
                                )
                                contact_setup_applied = True
                                contact_probe_phase = "wait-contact"
                                deadline = time.monotonic() + args.seconds
                                probe["contact_runtime_started_sample"] = report[
                                    "guarded_samples"
                                ]
                                probe["contact_selected_sample"] = report[
                                    "guarded_samples"
                                ]

                        if (
                            contact_probe_phase == "wait-contact"
                            and contact_setup_applied
                            and contact_state is not None
                        ):
                            subtitle = str(contact_state["subtitle"])
                            word_offset = int(contact_state["menu_words_offset"])
                            expected_text = text_by_word_offset.get(word_offset)
                            if (
                                subtitle
                                and expected_text is not None
                                and word_offset != contact_last_word_offset
                            ):
                                assert isinstance(expected_text, dict)
                                checkpoint = {
                                    "sample": report["guarded_samples"],
                                    "menu_words_offset": word_offset,
                                    "subtitle": subtitle,
                                    "expected_subtitle": expected_text[
                                        "subtitle"
                                    ],
                                }
                                contact_checkpoints.append(checkpoint)
                                probe["checkpoints"] = contact_checkpoints
                                line_states = probe.setdefault("line_states", [])
                                assert isinstance(line_states, list)
                                line_states.append(
                                    {
                                        **checkpoint,
                                        "cpu": cpu_for_report(state),
                                        "audio_flow": audio_state,
                                        "presentation_flow": presentation_state,
                                        "contact_flow": contact_state,
                                    }
                                )
                                contact_started = True
                                contact_last_word_offset = word_offset
                            elif (
                                contact_started
                                and subtitle
                                and word_offset != contact_last_word_offset
                                and expected_text is None
                            ):
                                issues.append(
                                    "contact-probe-unexpected-word-list="
                                    f"{word_offset:#06x}"
                                )
                                diagnostics["contact_flow"] = contact_state

                            completion_reason = None
                            if len(contact_checkpoints) >= target_line_count:
                                completion_reason = "line-target"
                            elif contact_checkpoints and word_choice_waiting_for_input(
                                presentation_state
                            ):
                                completion_reason = "word-choice"
                            elif (
                                contact_started
                                and contact_checkpoints
                                and presentation_state["active_line"] == 0xFFFF
                                and presentation_state[
                                    "c2_presentation_gate"
                                ] == 0
                                and presentation_state[
                                    "presentation_active"
                                ] == 0
                                and presentation_state[
                                    "text_display_active"
                                ] == 0
                                and all(value == 0 for value in blockers.values())
                            ):
                                completion_reason = "presentation-complete"
                            if completion_reason is not None:
                                report["verdict"] = "CONTACT-PROBE-COMPLETE"
                                probe["completion_reason"] = completion_reason
                                probe["completed_sample"] = report[
                                    "guarded_samples"
                                ]
                                break

                        probe["phase"] = contact_probe_phase

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
                        if snapshot_requested.is_set():
                            report["verdict"] = "MANUAL-SNAPSHOT"
                        elif execution_stall is not None:
                            report["verdict"] = "EXECUTION-STALL"
                        elif pending_dosbox_fault is not None:
                            report["verdict"] = "DOSBOX-FAULT"
                        else:
                            report["verdict"] = "ANOMALY"
                        diagnostics["profile_state"] = profile_for_report(
                            profile_state
                        )
                        guest_context = snapshot_guest_memory(
                            memory,
                            state,
                            int(expected["game_segment"]),
                            None,
                            profile_for_report(profile_state),
                        )
                        guest_context["execution_location"] = (
                            describe_execution_location(
                                memory,
                                state,
                                int(expected["load_segment"]),
                                blocks,
                                cd_dir,
                                link_map,
                            )
                        )
                        diagnostics["guest_context"] = guest_context
                        anomalies = report["anomalies"]
                        assert isinstance(anomalies, list)
                        anomaly = {
                            "sample": report["guarded_samples"],
                            "cpu": cpu_for_report(state),
                            "issues": issues,
                        }
                        anomaly.update(diagnostics)
                        anomalies.append(anomaly)
                        if crash_dir is not None:
                            report["crash_artifacts"] = write_crash_bundle(
                                crash_dir,
                                memory,
                                {
                                    "verdict": report["verdict"],
                                    "anomaly": anomaly,
                                    "calibrated": report.get("calibrated"),
                                    "last_runtime": report.get("last_runtime"),
                                    "runtime_samples": runtime_samples,
                                    "dosbox_log": (
                                        None
                                        if dosbox_log is None
                                        else str(dosbox_log)
                                    ),
                                },
                            )
                        snapshot_requested.clear()
                        break
                    if (
                        args.teleport_profile is not None
                        and not args.script2_radio_probe
                        and not args.script1_bob_probe
                        and not args.contact_probe
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
            elif args.script1_bob_probe:
                report["verdict"] = "BOB-PROBE-TIMEOUT"
                probe = report.setdefault("bob_probe", {})
                if isinstance(probe, dict):
                    probe["phase"] = bob_probe_phase
            elif args.contact_probe:
                report["verdict"] = "CONTACT-PROBE-TIMEOUT"
                probe = report.setdefault("contact_probe", {})
                if isinstance(probe, dict):
                    probe["phase"] = contact_probe_phase
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
        if log_stream is not None:
            log_stream.close()
        if xvfb is not None:
            xvfb.terminate()
            xvfb.wait()
        signal.signal(signal.SIGUSR1, previous_sigusr1)

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
    return 0 if report["verdict"] in SUCCESSFUL_VERDICTS else 1


if __name__ == "__main__":
    raise SystemExit(main())
