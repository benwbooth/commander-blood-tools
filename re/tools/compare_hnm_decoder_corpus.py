#!/usr/bin/env python3
"""Compare every shipped HNM payload with the original BLOODPRG decoders."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import concurrent.futures
import hashlib
import json
import struct
import subprocess
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
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


REPO_ROOT = Path(__file__).resolve().parents[2]
ORIGINAL_EXECUTABLE_PATH = REPO_ROOT / "re/bin/BLOODPRG.EXE"
DEFAULT_RESOURCE_ROOT = REPO_ROOT / "output/_tmp_dat"
DEFAULT_RUST_TRACE = REPO_ROOT / "target/debug/hnm-corpus-trace"

AB_ENTRY = 0xA867
AD_ENTRY = 0xA914
RECT_AD_ENTRY = 0xAB25
PALETTE_ENTRY = 0xA0C3
AD_HELPER_ENTRY = 0xAABC
AD_LITERAL_BIAS_SITE_ONE = 0xAAED
AD_LITERAL_BIAS_SITE_TWO = 0xAB1D
AD_SEGMENT_RELATIVE_SITE_ONE = 0x0DDD
AD_SEGMENT_RELATIVE_SITE_TWO = 0x0E0D
RETURN_ADDRESS = 0x6F00
SOURCE_SEGMENT = 0x2000
DESTINATION_SEGMENT = 0x3800
FRAMEBUFFER_SEGMENT = 0x6000
STACK_SEGMENT = 0x7000
GAME_SEGMENT = 0x5000
STACK_OFFSET = 0xFF00
SEGMENT_BYTE_COUNT = 0x10000
MACHINE_BYTE_COUNT = 0x300000
EXECUTABLE_PADDED_BYTE_COUNT = 0x120000
MAXIMUM_INSTRUCTION_COUNT = 5_000_000
AD_FLAG_HIGH_LITERAL_BIAS = 0x40
AD_HIGH_LITERAL_BIAS = 0x80
GAME_RESOURCE_DECODE_MODE_OFFSET = 0x0AA0
GAME_RECT_STAGING_SEGMENT_OFFSET = 0x0ABE
GAME_RECT_RAW_WIDTH_OFFSET = 0x0DA4
GAME_RECT_RAW_ROWS_OFFSET = 0x0DA6
GAME_RECT_VERTICAL_OFFSET = 0x1FA7
GAME_RECT_FRAMEBUFFER_SEGMENT_OFFSET = 0x5223
GAME_PRESENTATION_FLAGS_OFFSET = 0x2751
GAME_ENTRY_METRIC_MODE_OFFSET = 0x0D60
GAME_LIVE_PALETTE_OFFSET = 0x5251
GAME_RENDER_PALETTE_SNAPSHOT_OFFSET = 0x5851
GAME_PALETTE_DIRTY_OFFSET = 0x5B55
PALETTE_BYTE_COUNT = 256 * 3
LIVE_PALETTE_PATTERN_STEP = 29
LIVE_PALETTE_PATTERN_SEED = 7
SNAPSHOT_PALETTE_PATTERN_STEP = 43
SNAPSHOT_PALETTE_PATTERN_SEED = 17
PROGRESS_INTERVAL = 1_000
DEFAULT_MAXIMUM_MISMATCHES = 20
DEFAULT_BATCH_SIZE = 256
DEFAULT_WORKER_COUNT = min(16, os.cpu_count() or 1)

ZERO_SEGMENT = bytes(SEGMENT_BYTE_COUNT)
PATTERNED_DESTINATION = bytes(
    (offset * 23 + (offset >> 8) * 11 + 31) & 0xFF
    for offset in range(SEGMENT_BYTE_COUNT)
)
PATTERNED_FRAMEBUFFER = bytes(
    (offset * 37 + (offset >> 8) * 13 + 19) & 0xFF
    for offset in range(SEGMENT_BYTE_COUNT)
)
PATTERNED_LIVE_PALETTE = bytes(
    (offset * LIVE_PALETTE_PATTERN_STEP + LIVE_PALETTE_PATTERN_SEED) & 0xFF
    for offset in range(PALETTE_BYTE_COUNT)
)
PATTERNED_RENDER_PALETTE_SNAPSHOT = bytes(
    (offset * SNAPSHOT_PALETTE_PATTERN_STEP + SNAPSHOT_PALETTE_PATTERN_SEED) & 0xFF
    for offset in range(PALETTE_BYTE_COUNT)
)


class OriginalPresentationDecoder:
    """Reusable Unicorn host for the original AB and AD routines."""

    def __init__(self, executable: bytes) -> None:
        if len(executable) > EXECUTABLE_PADDED_BYTE_COUNT:
            raise RuntimeError("BLOODPRG executable exceeds the oracle mapping")
        self.machine = Uc(UC_ARCH_X86, UC_MODE_16)
        self.machine.mem_map(0, MACHINE_BYTE_COUNT)
        self.machine.mem_write(
            0,
            executable + bytes(EXECUTABLE_PADDED_BYTE_COUNT - len(executable)),
        )
        self.machine.mem_write(RETURN_ADDRESS, b"\xcc")
        self.returned = False
        self.active_codec = ""
        self.literal_bias = 0
        self.machine.hook_add(UC_HOOK_CODE, self._code_hook)

    def _code_hook(
        self, machine: Uc, address: int, _size: int, _data: object
    ) -> None:
        if address == RETURN_ADDRESS:
            self.returned = True
            machine.emu_stop()
            return
        if self.active_codec in ("ad", "rect_ad") and address == AD_HELPER_ENTRY:
            encoded = bytes((self.literal_bias,))
            machine.mem_write(AD_LITERAL_BIAS_SITE_ONE, encoded)
            machine.mem_write(AD_LITERAL_BIAS_SITE_TWO, encoded)

    def decode(
        self,
        codec: str,
        payload: bytes,
        decoded_length: int,
        patterned_destination: bool,
    ) -> tuple[bytes, int]:
        if len(payload) > SEGMENT_BYTE_COUNT:
            raise RuntimeError(f"payload has {len(payload)} bytes")
        if decoded_length > SEGMENT_BYTE_COUNT:
            raise RuntimeError(f"decoded output has {decoded_length} bytes")

        self.active_codec = codec
        self.returned = False
        self.literal_bias = (
            AD_HIGH_LITERAL_BIAS
            if codec == "ad"
            and len(payload) >= 5
            and payload[4] & AD_FLAG_HIGH_LITERAL_BIAS
            else 0
        )
        source_address = SOURCE_SEGMENT * 16
        destination_address = DESTINATION_SEGMENT * 16
        stack_address = STACK_SEGMENT * 16 + STACK_OFFSET
        self.machine.mem_write(source_address, ZERO_SEGMENT)
        self.machine.mem_write(source_address, payload)
        self.machine.mem_write(
            destination_address,
            PATTERNED_DESTINATION if patterned_destination else ZERO_SEGMENT,
        )
        self.machine.mem_write(stack_address, struct.pack("<H", RETURN_ADDRESS))
        self.machine.mem_write(AD_SEGMENT_RELATIVE_SITE_ONE, b"\x5a")
        self.machine.mem_write(AD_SEGMENT_RELATIVE_SITE_TWO, b"\xa5")
        self.machine.mem_write(AD_LITERAL_BIAS_SITE_ONE, bytes((self.literal_bias,)))
        self.machine.mem_write(AD_LITERAL_BIAS_SITE_TWO, bytes((self.literal_bias,)))

        registers = {
            UC_X86_REG_EAX: 0xA1A11234,
            UC_X86_REG_EBX: 0xB2B20000,
            UC_X86_REG_ECX: 0xC3C33456,
            UC_X86_REG_EDX: 0xD4D44567,
            UC_X86_REG_ESI: 0xE5E50000,
            UC_X86_REG_EDI: 0xF6F60000,
            UC_X86_REG_EBP: 0x97972468,
            UC_X86_REG_SP: STACK_OFFSET,
            UC_X86_REG_CS: 0,
            UC_X86_REG_DS: SOURCE_SEGMENT,
            UC_X86_REG_ES: DESTINATION_SEGMENT,
            UC_X86_REG_FS: 0x1800,
            UC_X86_REG_GS: GAME_SEGMENT,
            UC_X86_REG_SS: STACK_SEGMENT,
            UC_X86_REG_EFLAGS: 0x0202,
        }
        for register, value in registers.items():
            self.machine.reg_write(register, value)

        entry = AB_ENTRY if codec == "ab" else AD_ENTRY
        try:
            self.machine.emu_start(
                entry,
                MACHINE_BYTE_COUNT - 16,
                count=MAXIMUM_INSTRUCTION_COUNT,
            )
        except UcError as error:
            raise RuntimeError(
                f"{codec} execution failed at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            ) from error
        if not self.returned:
            raise RuntimeError(
                f"{codec} did not return; stopped at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            )

        if codec == "ab":
            produced = self.machine.reg_read(UC_X86_REG_CX)
            consumed = self.machine.reg_read(UC_X86_REG_SI)
            if produced != decoded_length:
                raise RuntimeError(
                    f"AB produced {produced} bytes, Rust reports {decoded_length}"
                )
        else:
            consumed = self.machine.reg_read(UC_X86_REG_BX)
        decoded = bytes(self.machine.mem_read(destination_address, decoded_length))
        return decoded, consumed

    def decode_rectangle(
        self, payload: bytes, raw_width: int, raw_rows: int
    ) -> tuple[bytes, bytes, int]:
        if len(payload) > SEGMENT_BYTE_COUNT:
            raise RuntimeError(f"payload has {len(payload)} bytes")
        self.active_codec = "rect_ad"
        self.returned = False
        self.literal_bias = (
            AD_HIGH_LITERAL_BIAS
            if len(payload) >= 5 and payload[4] & AD_FLAG_HIGH_LITERAL_BIAS
            else 0
        )
        source_address = SOURCE_SEGMENT * 16
        staging_address = DESTINATION_SEGMENT * 16
        framebuffer_address = FRAMEBUFFER_SEGMENT * 16
        game_address = GAME_SEGMENT * 16
        stack_address = STACK_SEGMENT * 16 + STACK_OFFSET
        self.machine.mem_write(source_address, ZERO_SEGMENT)
        self.machine.mem_write(source_address, payload)
        self.machine.mem_write(staging_address, PATTERNED_DESTINATION)
        self.machine.mem_write(framebuffer_address, PATTERNED_FRAMEBUFFER)
        self.machine.mem_write(game_address, ZERO_SEGMENT)
        self.machine.mem_write(stack_address, struct.pack("<H", RETURN_ADDRESS))
        self.machine.mem_write(AD_SEGMENT_RELATIVE_SITE_ONE, b"\x5a")
        self.machine.mem_write(AD_SEGMENT_RELATIVE_SITE_TWO, b"\xa5")
        self.machine.mem_write(AD_LITERAL_BIAS_SITE_ONE, bytes((self.literal_bias,)))
        self.machine.mem_write(AD_LITERAL_BIAS_SITE_TWO, bytes((self.literal_bias,)))
        game_words = {
            GAME_RESOURCE_DECODE_MODE_OFFSET: 0,
            GAME_RECT_STAGING_SEGMENT_OFFSET: DESTINATION_SEGMENT,
            GAME_RECT_RAW_WIDTH_OFFSET: raw_width,
            GAME_RECT_RAW_ROWS_OFFSET: raw_rows,
            GAME_RECT_VERTICAL_OFFSET: 0,
            GAME_RECT_FRAMEBUFFER_SEGMENT_OFFSET: FRAMEBUFFER_SEGMENT,
        }
        for offset, value in game_words.items():
            self.machine.mem_write(game_address + offset, struct.pack("<H", value))

        registers = {
            UC_X86_REG_EAX: 0xA1A11234,
            UC_X86_REG_EBX: 0xB2B20000,
            UC_X86_REG_ECX: 0xC3C33456,
            UC_X86_REG_EDX: 0xD4D44567,
            UC_X86_REG_ESI: 0xE5E50000,
            UC_X86_REG_EDI: 0xF6F60000,
            UC_X86_REG_EBP: 0x97972468,
            UC_X86_REG_SP: STACK_OFFSET,
            UC_X86_REG_CS: 0,
            UC_X86_REG_DS: SOURCE_SEGMENT,
            UC_X86_REG_ES: 0x2C00,
            UC_X86_REG_FS: 0x1800,
            UC_X86_REG_GS: GAME_SEGMENT,
            UC_X86_REG_SS: STACK_SEGMENT,
            UC_X86_REG_EFLAGS: 0x0202,
        }
        for register, value in registers.items():
            self.machine.reg_write(register, value)
        try:
            self.machine.emu_start(
                RECT_AD_ENTRY,
                MACHINE_BYTE_COUNT - 16,
                count=MAXIMUM_INSTRUCTION_COUNT,
            )
        except UcError as error:
            raise RuntimeError(
                "rect_ad execution failed at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            ) from error
        if not self.returned:
            raise RuntimeError(
                "rect_ad did not return; stopped at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            )
        consumed = self.machine.reg_read(UC_X86_REG_BX)
        staging = bytes(self.machine.mem_read(staging_address, SEGMENT_BYTE_COUNT))
        framebuffer = bytes(
            self.machine.mem_read(framebuffer_address, SEGMENT_BYTE_COUNT)
        )
        return staging, framebuffer, consumed

    def apply_palette(self, payload: bytes) -> tuple[bytes, bytes, int, bool]:
        if len(payload) > SEGMENT_BYTE_COUNT:
            raise RuntimeError(f"palette payload has {len(payload)} bytes")
        self.active_codec = "palette"
        self.returned = False
        source_address = SOURCE_SEGMENT * 16
        game_address = GAME_SEGMENT * 16
        stack_address = STACK_SEGMENT * 16 + STACK_OFFSET
        self.machine.mem_write(source_address, ZERO_SEGMENT)
        self.machine.mem_write(source_address, payload)
        self.machine.mem_write(game_address, ZERO_SEGMENT)
        self.machine.mem_write(
            game_address + GAME_LIVE_PALETTE_OFFSET, PATTERNED_LIVE_PALETTE
        )
        self.machine.mem_write(
            game_address + GAME_RENDER_PALETTE_SNAPSHOT_OFFSET,
            PATTERNED_RENDER_PALETTE_SNAPSHOT,
        )
        self.machine.mem_write(
            game_address + GAME_PRESENTATION_FLAGS_OFFSET, b"\x00"
        )
        self.machine.mem_write(
            game_address + GAME_ENTRY_METRIC_MODE_OFFSET, b"\x01\x00"
        )
        self.machine.mem_write(
            game_address + GAME_PALETTE_DIRTY_OFFSET, b"\x00"
        )
        self.machine.mem_write(stack_address, struct.pack("<H", RETURN_ADDRESS))

        registers = {
            UC_X86_REG_EAX: 0xA1A11234,
            UC_X86_REG_EBX: 0xB2B20000,
            UC_X86_REG_ECX: 0xC3C33456,
            UC_X86_REG_EDX: 0xD4D44567,
            UC_X86_REG_ESI: 0xE5E50000,
            UC_X86_REG_EDI: 0xF6F60000,
            UC_X86_REG_EBP: 0x97972468,
            UC_X86_REG_SP: STACK_OFFSET,
            UC_X86_REG_CS: 0,
            UC_X86_REG_DS: GAME_SEGMENT,
            UC_X86_REG_ES: SOURCE_SEGMENT,
            UC_X86_REG_FS: 0x1800,
            UC_X86_REG_GS: GAME_SEGMENT,
            UC_X86_REG_SS: STACK_SEGMENT,
            UC_X86_REG_EFLAGS: 0x0202,
        }
        for register, value in registers.items():
            self.machine.reg_write(register, value)
        try:
            self.machine.emu_start(
                PALETTE_ENTRY,
                MACHINE_BYTE_COUNT - 16,
                count=MAXIMUM_INSTRUCTION_COUNT,
            )
        except UcError as error:
            raise RuntimeError(
                "palette execution failed at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            ) from error
        if not self.returned:
            raise RuntimeError(
                "palette did not return; stopped at "
                f"{self.machine.reg_read(UC_X86_REG_CS):#x}:"
                f"{self.machine.reg_read(UC_X86_REG_IP):#x}"
            )
        consumed = self.machine.reg_read(UC_X86_REG_SI)
        live = bytes(
            self.machine.mem_read(
                game_address + GAME_LIVE_PALETTE_OFFSET, PALETTE_BYTE_COUNT
            )
        )
        snapshot = bytes(
            self.machine.mem_read(
                game_address + GAME_RENDER_PALETTE_SNAPSHOT_OFFSET,
                PALETTE_BYTE_COUNT,
            )
        )
        dirty = bool(
            self.machine.mem_read(game_address + GAME_PALETTE_DIRTY_OFFSET, 1)[0]
        )
        return live, snapshot, consumed, dirty


_WORKER_DECODER: OriginalPresentationDecoder | None = None


def initialize_worker() -> None:
    global _WORKER_DECODER
    _WORKER_DECODER = OriginalPresentationDecoder(ORIGINAL_EXECUTABLE_PATH.read_bytes())


def compare_trace_in_worker(
    task: tuple[dict[str, object], str, bool]
) -> dict[str, object]:
    trace, resource_path, check_history = task
    if _WORKER_DECODER is None:
        raise RuntimeError("original decoder worker was not initialized")
    payload_offset = int(trace["payload_offset"])
    payload_length = int(trace["payload_length"])
    with Path(resource_path).open("rb") as source:
        source.seek(payload_offset)
        payload = source.read(payload_length)
    if len(payload) != payload_length:
        raise RuntimeError(
            f"{resource_path}: payload at {payload_offset} is truncated"
        )
    codec = str(trace["codec"])
    try:
        if codec == "palette":
            live, snapshot, consumed, dirty = _WORKER_DECODER.apply_palette(payload)
            return {
                "live_sha256": hashlib.sha256(live).hexdigest(),
                "render_snapshot_sha256": hashlib.sha256(snapshot).hexdigest(),
                "consumed_bytes": consumed,
                "dirty": dirty,
            }
        decoded_length = int(trace["decoded_length"])
        if codec == "rect_ad":
            staging, original_zero, consumed = _WORKER_DECODER.decode_rectangle(
                payload, int(trace["layout"]), int(trace["row_mode"])
            )
            original_patterned = None
            patterned_consumed = consumed
        else:
            staging = None
            original_zero, consumed = _WORKER_DECODER.decode(
                codec, payload, decoded_length, patterned_destination=False
            )
            original_patterned = None
            patterned_consumed = consumed
            if check_history:
                original_patterned, patterned_consumed = _WORKER_DECODER.decode(
                    codec, payload, decoded_length, patterned_destination=True
                )
    except Exception as error:
        return {"error": str(error)}
    return {
        "original_sha256": hashlib.sha256(original_zero).hexdigest(),
        "consumed_bytes": consumed,
        "history_changed": (
            original_patterned is not None and original_patterned != original_zero
        ),
        "patterned_consumed_bytes": patterned_consumed,
        "staging_sha256": (
            hashlib.sha256(staging).hexdigest() if staging is not None else None
        ),
    }


def resolve_resource(input_path: Path, resource: str) -> Path:
    base = input_path.parent if input_path.is_file() else input_path
    return base / Path(resource)


def comparison_record(
    trace: dict[str, object],
    reason: str,
) -> dict[str, object]:
    return {
        "resource": trace["resource"],
        "frame_index": trace["frame_index"],
        "codec": trace.get("codec", "palette"),
        "payload_offset": trace["payload_offset"],
        "reason": reason,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "input",
        nargs="?",
        type=Path,
        default=DEFAULT_RESOURCE_ROOT,
        help="loose-resource root or one HNM file",
    )
    parser.add_argument(
        "--rust-trace",
        type=Path,
        default=DEFAULT_RUST_TRACE,
        help="built hnm-corpus-trace executable",
    )
    parser.add_argument(
        "--skip-history-check",
        action="store_true",
        help="do not repeat original decoding with patterned destination history",
    )
    parser.add_argument(
        "--max-frames",
        type=int,
        help="stop after this many compressed frames",
    )
    parser.add_argument(
        "--max-mismatches",
        type=int,
        default=DEFAULT_MAXIMUM_MISMATCHES,
        help="stop after this many mismatches",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=DEFAULT_WORKER_COUNT,
        help="parallel original-decoder workers",
    )
    parser.add_argument(
        "--codec",
        choices=("ab", "ad"),
        help="compare only one compressed codec family",
    )
    parser.add_argument(
        "--rect",
        action="store_true",
        help="compare deferred transparent AD rectangle composition",
    )
    parser.add_argument(
        "--palette",
        action="store_true",
        help="compare bootstrap and effective per-frame palette records",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.jobs < 1:
        raise SystemExit("--jobs must be at least one")
    if args.max_frames is not None and args.max_frames < 1:
        raise SystemExit("--max-frames must be at least one")
    if args.max_mismatches < 1:
        raise SystemExit("--max-mismatches must be at least one")
    selected_modes = int(args.rect) + int(args.palette) + int(args.codec is not None)
    if selected_modes > 1:
        raise SystemExit("--rect, --palette, and --codec cannot be combined")
    input_path = args.input.resolve()
    trace_executable = args.rust_trace.resolve()
    if not input_path.exists():
        raise SystemExit(f"HNM input does not exist: {input_path}")
    if not trace_executable.is_file():
        raise SystemExit(
            f"Rust trace executable does not exist: {trace_executable}; "
            "build it with `cargo build -p commander-blood-game --bin hnm-corpus-trace`"
        )

    trace_command = [str(trace_executable), str(input_path)]
    if args.codec is not None:
        trace_command.extend(("--codec", args.codec))
    if args.rect:
        trace_command.append("--rect")
    if args.palette:
        trace_command.append("--palette")
    process = subprocess.Popen(
        trace_command,
        stdout=subprocess.PIPE,
        text=True,
    )
    if process.stdout is None:
        raise RuntimeError("Rust trace process has no stdout")

    frames = 0
    mismatches: list[dict[str, object]] = []
    history_sensitive = 0
    stopped_early = False

    def check_batch(
        batch: list[dict[str, object]],
        executor: concurrent.futures.ProcessPoolExecutor,
    ) -> bool:
        nonlocal frames, history_sensitive
        tasks = [
            (
                trace,
                str(resolve_resource(input_path, str(trace["resource"]))),
                not args.skip_history_check,
            )
            for trace in batch
        ]
        outcomes = executor.map(compare_trace_in_worker, tasks, chunksize=1)
        for trace, outcome in zip(batch, outcomes, strict=True):
            frames += 1
            if "error" in outcome:
                mismatch = comparison_record(
                    trace, f"original oracle execution failed: {outcome['error']}"
                )
                mismatches.append(mismatch)
                print(json.dumps(mismatch, sort_keys=True))
                continue
            consumed = int(outcome["consumed_bytes"])
            if args.palette:
                reason_parts = []
                if str(outcome["live_sha256"]) != str(trace["live_sha256"]):
                    reason_parts.append("live palette differs")
                if str(outcome["render_snapshot_sha256"]) != str(
                    trace["render_snapshot_sha256"]
                ):
                    reason_parts.append("render palette snapshot differs")
                if consumed != int(trace["consumed_bytes"]):
                    reason_parts.append(
                        f"source progress differs: Rust {trace['consumed_bytes']}, original {consumed}"
                    )
                if bool(outcome["dirty"]) != bool(trace["dirty"]):
                    reason_parts.append("palette dirty state differs")
                if reason_parts:
                    mismatch = comparison_record(trace, "; ".join(reason_parts))
                    mismatch["record_kind"] = trace["record_kind"]
                    mismatches.append(mismatch)
                    print(json.dumps(mismatch, sort_keys=True))
                if frames % PROGRESS_INTERVAL == 0:
                    print(
                        f"checked {frames} palette records; mismatches={len(mismatches)}",
                        file=sys.stderr,
                        flush=True,
                    )
                continue
            patterned_consumed = int(outcome["patterned_consumed_bytes"])
            if patterned_consumed != consumed:
                raise RuntimeError("destination history changed compressed-source progress")
            history_changed = bool(outcome["history_changed"])
            if history_changed:
                history_sensitive += 1
            rust_hash = str(trace["decoded_sha256"])
            original_hash = str(outcome["original_sha256"])
            original_staging_hash = outcome["staging_sha256"]
            rust_staging_hash = trace.get("staging_sha256")
            consumed_mismatch = consumed != int(trace["consumed_bytes"])
            staging_mismatch = original_staging_hash != rust_staging_hash
            if (
                original_hash != rust_hash
                or history_changed
                or consumed_mismatch
                or staging_mismatch
            ):
                reason_parts = []
                if original_hash != rust_hash:
                    reason_parts.append("decoded bytes differ")
                if history_changed:
                    reason_parts.append("original output depends on destination history")
                if consumed_mismatch:
                    reason_parts.append(
                        f"source progress differs: Rust {trace['consumed_bytes']}, original {consumed}"
                    )
                if staging_mismatch:
                    reason_parts.append("rectangular staging state differs")
                mismatch = comparison_record(trace, "; ".join(reason_parts))
                mismatch["rust_sha256"] = rust_hash
                mismatch["original_sha256"] = original_hash
                if staging_mismatch:
                    mismatch["rust_staging_sha256"] = rust_staging_hash
                    mismatch["original_staging_sha256"] = original_staging_hash
                mismatches.append(mismatch)
                print(json.dumps(mismatch, sort_keys=True))

            if frames % PROGRESS_INTERVAL == 0:
                print(
                    f"checked {frames} compressed frames; "
                    f"mismatches={len(mismatches)}; history_sensitive={history_sensitive}",
                    file=sys.stderr,
                    flush=True,
                )
        return len(mismatches) >= args.max_mismatches

    try:
        with concurrent.futures.ProcessPoolExecutor(
            max_workers=args.jobs, initializer=initialize_worker
        ) as executor:
            batch: list[dict[str, object]] = []
            for encoded in process.stdout:
                batch.append(json.loads(encoded))
                reached_frame_limit = (
                    args.max_frames is not None
                    and frames + len(batch) >= args.max_frames
                )
                if len(batch) < DEFAULT_BATCH_SIZE and not reached_frame_limit:
                    continue
                if check_batch(batch, executor):
                    stopped_early = True
                    break
                batch.clear()
                if reached_frame_limit:
                    stopped_early = True
                    break
            if batch and not stopped_early:
                if check_batch(batch, executor):
                    stopped_early = True
    except BaseException:
        process.terminate()
        process.wait()
        raise
    if stopped_early:
        process.terminate()
    return_code = process.wait()
    if not stopped_early and return_code != 0:
        raise RuntimeError(f"Rust trace process exited with status {return_code}")

    summary = {
        "palette_records_checked" if args.palette else "compressed_frames_checked": frames,
        "mismatches": len(mismatches),
        "complete": not stopped_early,
    }
    if not args.palette:
        summary["destination_history_sensitive_frames"] = history_sensitive
    print(json.dumps(summary, sort_keys=True))
    return int(bool(mismatches))


if __name__ == "__main__":
    raise SystemExit(main())
