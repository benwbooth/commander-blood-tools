#!/usr/bin/env python3
"""Capture first frames from the three shipped alien XDB overlays under DOS."""

from __future__ import annotations

from pathlib import Path
import sys

if sys.path and Path(sys.path[0]).resolve() == Path(__file__).resolve().parent:
    del sys.path[0]

import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "re" / "integration" / "dos" / "source_xdb_alien_loader.c"
INCLUDE_DIR = ROOT / "re" / "source" / "xdb" / "candidates" / "include"
FRAME_WIDTH = 320
FRAME_HEIGHT = 200
MOUSE_CENTER_X = 320
MOUSE_CENTER_Y = 512
PLANE_COUNT = 4
PLANE_ROW_BYTES = FRAME_WIDTH // PLANE_COUNT
PLANE_FRAME_BYTES = PLANE_ROW_BYTES * FRAME_HEIGHT
PALETTE_BYTES = 256 * 3
VGA_DAC_MAXIMUM = 63
EIGHT_BIT_MAXIMUM = 255
STANDARD_CAMPAIGN_FRAMES = (1, 2, 4, 8, 16, 32)
STANDARD_CAMPAIGN_TIMING_SCALE = 10

MODULES = {
    "amer": {
        "data_delta_field": 0x3275,
        "render_continuation": 0x0944,
        "render_mode": 0x28D0,
        "final_clear_call": 0x0204,
        "final_clear_displacement": 0x00E9,
        "callback_load": 0x019E,
    },
    "croolis": {
        "data_delta_field": 0x32E5,
        "render_continuation": 0x0946,
        "render_mode": 0x2940,
        "final_clear_call": 0x020B,
        "final_clear_displacement": 0x00F7,
        "callback_load": 0x01A5,
    },
    "scrut": {
        "data_delta_field": 0x33A5,
        "render_continuation": 0x0946,
        "render_mode": 0x2A00,
        "final_clear_call": 0x020B,
        "final_clear_displacement": 0x00F7,
        "callback_load": 0x01A5,
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--xdb-dir",
        type=Path,
        default=ROOT / "output" / "_tmp_dat",
        help="directory containing the original amer.xdb, croolis.xdb, and scrut.xdb",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "output" / "original_xdb_alien_frame_oracle",
    )
    parser.add_argument(
        "--fixture",
        type=Path,
        help=(
            "fixture destination; complete captures default to the committed oracle "
            "and diagnostic subsets default inside --output-dir"
        ),
    )
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument(
        "--module",
        choices=tuple(MODULES),
        action="append",
        help="capture only the selected module; may be repeated",
    )
    parser.add_argument(
        "--model-count",
        type=int,
        help="stop the original context list after this many behavior models",
    )
    parser.add_argument(
        "--frame-count",
        type=int,
        action="append",
        help="capture this one-based rendered frame; may be repeated",
    )
    parser.add_argument(
        "--timing-scale",
        type=int,
        default=7,
        help="native API timing-scale input word",
    )
    parser.add_argument(
        "--input-campaign",
        choices=("centered", "corners"),
        default="centered",
        help="deterministic mouse samples applied between rendered frames",
    )
    return parser.parse_args()


def executable(name: str) -> str:
    resolved = shutil.which(name)
    if resolved is None:
        raise SystemExit(f"executable not found: {name}")
    return resolved


def run(command: list[str], cwd: Path, timeout: int | None = None) -> None:
    try:
        process = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit(f"command timed out: {' '.join(command)}") from error
    if process.returncode != 0:
        raise SystemExit(
            f"command failed: {' '.join(command)}\n"
            + process.stdout
            + process.stderr
        )


def read_u16(data: bytes, offset: int) -> int:
    if offset < 0 or offset + 2 > len(data):
        raise SystemExit(f"word offset 0x{offset:04x} is outside XDB image")
    return struct.unpack_from("<H", data, offset)[0]


def deplanarize(planes: bytes) -> bytes:
    expected = PLANE_COUNT * PLANE_FRAME_BYTES
    if len(planes) != expected:
        raise SystemExit(
            f"FRAME.BIN has {len(planes)} bytes; expected {expected}"
        )
    indexed = bytearray(FRAME_WIDTH * FRAME_HEIGHT)
    for plane in range(PLANE_COUNT):
        plane_start = plane * PLANE_FRAME_BYTES
        for y in range(FRAME_HEIGHT):
            source = plane_start + y * PLANE_ROW_BYTES
            target = y * FRAME_WIDTH + plane
            indexed[target : target + FRAME_WIDTH : PLANE_COUNT] = planes[
                source : source + PLANE_ROW_BYTES
            ]
    return bytes(indexed)


def expand_palette(palette: bytes) -> bytes:
    if len(palette) != PALETTE_BYTES:
        raise SystemExit(
            f"PALETTE.BIN has {len(palette)} bytes; expected {PALETTE_BYTES}"
        )
    if any(component > VGA_DAC_MAXIMUM for component in palette):
        raise SystemExit("display palette contains a component above the VGA DAC range")
    return bytes(
        component * EIGHT_BIT_MAXIMUM // VGA_DAC_MAXIMUM for component in palette
    )


def rgba_frame(indexed: bytes, palette: bytes) -> bytes:
    rgba = bytearray(len(indexed) * 4)
    for pixel, index in enumerate(indexed):
        source = index * 3
        target = pixel * 4
        rgba[target : target + 3] = palette[source : source + 3]
        rgba[target + 3] = EIGHT_BIT_MAXIMUM
    return bytes(rgba)


def capture_module(
    module: str,
    config: dict[str, int],
    source: Path,
    output: Path,
    wcl: str,
    dosbox: str,
    model_count: int | None,
    frame_count: int,
    timing_scale: int,
    input_campaign: str,
) -> dict[str, object]:
    data = source.read_bytes()
    data_delta_field = config["data_delta_field"]
    data_paragraph = read_u16(data, data_delta_field)
    state_offset = data_delta_field + 2
    call_offset = config["final_clear_call"]
    expected_call = struct.pack(
        "<BH", 0xE8, config["final_clear_displacement"]
    )
    actual_call = data[call_offset : call_offset + len(expected_call)]
    if actual_call != expected_call:
        raise SystemExit(
            f"{module.upper()} cleanup call at 0x{call_offset:04x} is "
            f"{actual_call.hex()}, expected {expected_call.hex()}"
        )

    loaded_data = bytearray(data)
    if model_count is not None:
        if model_count < 1:
            raise SystemExit("--model-count must be positive")
        sentinel = data_paragraph * 16 + 0x2308 + model_count * 2
        if read_u16(data, sentinel) == 0:
            raise SystemExit(
                f"{module.upper()} has fewer than {model_count + 1} model contexts"
            )
        struct.pack_into("<H", loaded_data, sentinel, 0)

    object_delta = read_u16(data, data_paragraph * 16 + 0x000C)
    texture_delta = read_u16(data, data_paragraph * 16 + 0x000E)
    raster_delta = read_u16(data, data_paragraph * 16 + 0x0010)
    allocation_paragraphs = (
        data_paragraph + object_delta + texture_delta + raster_delta + 0x1000
    )

    work = output / module
    work.mkdir(parents=True)
    (work / "ALIEN.XDB").write_bytes(loaded_data)
    executable_path = work / "XDBLOAD.EXE"
    capture_defines = (
        ["-dXDB_PREQUEUE_ESCAPE"]
        if frame_count == 1 and timing_scale == 7
        else []
    )
    run(
        [
            wcl,
            "-q",
            "-3",
            "-ox",
            "-mm",
            "-zdp",
            "-we",
            "-lr",
            f"-dXDB_IMAGE_BYTES={len(data)}UL",
            f"-dXDB_ALLOCATION_PARAGRAPHS=0x{allocation_paragraphs:04x}U",
            f"-dXDB_DATA_PARAGRAPH=0x{data_paragraph:04x}U",
            f"-dXDB_DATA_STATE_OFFSET=0x{state_offset:04x}U",
            "-dXDB_DUMP_FRAME",
            f"-dXDB_FINAL_CLEAR_CALL_OFFSET=0x{call_offset:04x}U",
            "-dXDB_FINAL_CLEAR_CALL_DISPLACEMENT="
            f"0x{config['final_clear_displacement']:04x}U",
            f"-dXDB_CALLBACK_LOAD_OFFSET=0x{config['callback_load']:04x}U",
            *capture_defines,
            "-dXDB_RENDER_CONTINUATION_OFFSET="
            f"0x{config['render_continuation']:04x}U",
            f"-dXDB_RENDER_MODE_OFFSET=0x{config['render_mode']:04x}U",
            f"-i={INCLUDE_DIR}",
            f"-fe={executable_path}",
            f"-fm={work / 'XDBLOAD.MAP'}",
            str(HARNESS),
        ],
        work,
    )
    environment = os.environ.copy()
    environment["SDL_AUDIODRIVER"] = "dummy"
    environment["SDL_VIDEODRIVER"] = "offscreen"
    input_campaign_id = {"centered": 0, "corners": 1}[input_campaign]
    try:
        process = subprocess.run(
            [
                dosbox,
                "--noprimaryconf",
                "--nolocalconf",
                "--exit",
                "-silent",
                "-set",
                "sdl fullscreen=false",
                "-set",
                "sdl output=surface",
                "-c",
                f'mount c "{work}"',
                "-c",
                "c:",
                "-c",
                f"XDBLOAD.EXE {frame_count} {timing_scale} "
                f"{input_campaign_id} > CONSOLE.TXT",
            ],
            cwd=work,
            env=environment,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
            timeout=30,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit(f"{module.upper()} DOS frame capture timed out") from error
    if process.returncode != 0:
        raise SystemExit(process.stdout + process.stderr)

    result = work / "RESULT.TXT"
    if not result.is_file() or result.read_text(encoding="ascii").strip() != (
        "PASS source-linked alien XDB"
    ):
        diagnostics = work / "CONSOLE.TXT"
        detail = (
            diagnostics.read_text(encoding="ascii", errors="replace")
            if diagnostics.is_file()
            else ""
        )
        raise SystemExit(f"{module.upper()} frame capture failed\n{detail}")
    indexed = deplanarize((work / "FRAME.BIN").read_bytes())
    dac_palette = (work / "PALETTE.BIN").read_bytes()
    palette = expand_palette(dac_palette)
    rgba = rgba_frame(indexed, palette)
    (work / "FRAME.INDEXED").write_bytes(indexed)
    (work / "FRAME.RGBA").write_bytes(rgba)
    return {
        "module": module,
        "xdb_file": source.name,
        "xdb_bytes": len(data),
        "xdb_sha256": hashlib.sha256(data).hexdigest(),
        "indexed_sha256": hashlib.sha256(indexed).hexdigest(),
        "rgba_sha256": hashlib.sha256(rgba).hexdigest(),
        "dac_palette_sha256": hashlib.sha256(dac_palette).hexdigest(),
        "nonzero_pixels": sum(pixel != 0 for pixel in indexed),
        "model_count": model_count,
        "frame_count": frame_count,
        "timing_scale": timing_scale,
        "input_campaign": input_campaign,
    }


def main() -> int:
    args = parse_args()
    frame_counts = args.frame_count or [1]
    if any(frame_count < 1 for frame_count in frame_counts):
        raise SystemExit("--frame-count must be positive")
    if len(set(frame_counts)) != len(frame_counts):
        raise SystemExit("--frame-count values must be unique")
    if args.timing_scale < 0 or args.timing_scale > 0xFFFF:
        raise SystemExit("--timing-scale must fit an unsigned 16-bit word")
    xdb_dir = args.xdb_dir.resolve()
    output = args.output_dir.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    wcl = executable(args.wcl)
    dosbox = executable(args.dosbox)
    captures = []
    selected_modules = args.module or list(MODULES)
    for frame_count in frame_counts:
        capture_output = (
            output
            if len(frame_counts) == 1
            else output / f"frame-{frame_count:04d}"
        )
        for module in selected_modules:
            config = MODULES[module]
            source = xdb_dir / f"{module}.xdb"
            if not source.is_file():
                raise SystemExit(f"missing original XDB: {source}")
            capture = capture_module(
                module,
                config,
                source,
                capture_output,
                wcl,
                dosbox,
                args.model_count,
                frame_count,
                args.timing_scale,
                args.input_campaign,
            )
            captures.append(capture)
            print(
                f"{module.upper()} frame {frame_count}: "
                f"{capture['nonzero_pixels']} nonzero pixels, "
                f"RGBA {capture['rgba_sha256']}"
            )
    complete_first_frame = (
        args.module is None
        and args.model_count is None
        and frame_counts == [1]
        and args.timing_scale == 7
        and args.input_campaign == "centered"
    )
    complete_campaign = (
        args.module is None
        and args.model_count is None
        and tuple(frame_counts) == STANDARD_CAMPAIGN_FRAMES
        and args.timing_scale == STANDARD_CAMPAIGN_TIMING_SCALE
        and args.input_campaign == "corners"
    )
    fixture = {
        "format": (
            "commander-blood-original-alien-first-frame-v1"
            if complete_first_frame
            else "commander-blood-original-alien-frame-campaign-v1"
        ),
        "width": FRAME_WIDTH,
        "height": FRAME_HEIGHT,
        "timing_scale": args.timing_scale,
        "mouse_x": MOUSE_CENTER_X,
        "mouse_y": MOUSE_CENTER_Y,
        "model_count": args.model_count,
    }
    if complete_first_frame:
        fixture["frame_count"] = frame_counts[0]
    else:
        fixture["frame_counts"] = frame_counts
    fixture["input_campaign"] = args.input_campaign
    fixture["captures"] = captures
    fixture_path = args.fixture
    if fixture_path is None:
        if complete_first_frame:
            fixture_path = (
                ROOT
                / "re"
                / "tools"
                / "oracle_vectors"
                / "alien_first_frames.json"
            )
        elif complete_campaign:
            fixture_path = (
                ROOT
                / "re"
                / "tools"
                / "oracle_vectors"
                / "alien_frame_campaign.json"
            )
        else:
            fixture_path = output / "alien_frame_campaign.json"
    fixture_path.parent.mkdir(parents=True, exist_ok=True)
    fixture_path.write_text(
        json.dumps(fixture, indent=2) + "\n",
        encoding="ascii",
    )
    print(f"wrote {fixture_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
