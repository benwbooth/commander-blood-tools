#!/usr/bin/env python3
"""Compare original and recovered MANU3 renderers on Pterra's real mesh."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import struct
import sys


ROOT = Path(__file__).resolve().parents[2]
ORIGINAL_MANU3_SHA256 = (
    "d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31"
)
ORIGINAL_DATA_DELTA_OFFSET = 0x1368
ORIGINAL_DATA_SEGMENT_OFFSET = 0x136A
RECOVERED_DATA_DELTA_OFFSET = 0x001B
RECOVERED_DATA_SEGMENT_OFFSET = 0x001D
ORIGINAL_API_ENTRY = 0x0000
ORIGINAL_RENDERER_ENTRY = 0x0700
ORIGINAL_RENDER_LINEAR_OFFSET = 0x0BD6
RENDER_CONTINUATION_OFFSET = 0x067E
FACE_LIST_OFFSET = 0x2300
FACE_COUNT_OFFSET = 0x2304
EXPECTED_FACE_LIST = 0x0B18
EXPECTED_FACE_COUNT = 216
CODE_SEGMENT = 0x1000
API_RETURN_SEGMENT = 0x2000
API_RETURN_ADDRESS = 0xF000
GLOBALS_SEGMENT = 0x3000
GEOMETRY_SEGMENT = 0x5000
RASTER_SEGMENT = 0x7000
TEXTURE_SEGMENT = 0x9000
FRAMEBUFFER_SEGMENT = 0xB000
STACK_SEGMENT = 0xC000
STACK_POINTER = 0xFF00
DIRECT_RETURN_ADDRESS = 0xF000
PTERRA_CURSOR_X = 224
PTERRA_CURSOR_Y = 0
MAX_INSTRUCTIONS = 5_000_000
REBUILT_RENDERER_SYMBOL = "xdb_manu3_face_bucket_sort_"
RASTER_SCRATCH_RANGES = (
    (0x061C, 0x0634),
    (0x0670, 0x067E),
    (0x0682, 0x0684),
)
RASTER_POOL_OFFSET = 0x0A72
RASTER_RECORD_SIZE = 0x005A
RASTER_RECORD_COUNT = 200
RASTER_SORT_NEXT_OFFSET = 0x0058
MAP_SYMBOL_RE = re.compile(
    r"^\s*([0-9a-fA-F]{4}):([0-9a-fA-F]{4,8})\s+(\S+)\s*$"
)


@dataclass(frozen=True)
class PterraRendererInput:
    globals_image: bytes
    geometry_image: bytes
    texture_image: bytes
    raster_image: bytes
    source_segments: tuple[int, int, int, int]


@dataclass(frozen=True)
class RendererResult:
    framebuffer: bytes
    globals_image: bytes
    geometry_image: bytes
    raster_image: bytes


def load_oracle_module():
    path = Path(__file__).with_name("xdb_candidate_oracle.py")
    spec = importlib.util.spec_from_file_location(
        "manu3_pterra_xdb_candidate_oracle", path
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load Unicorn oracle helper from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def require_prefix(image: bytes, prefix: bytes, label: str) -> None:
    if not image.startswith(prefix):
        raise ValueError(
            f"{label} MANU3 image starts with {image[:4].hex()}, "
            f"expected {prefix.hex()}"
        )


def prepare_original_image(image: bytes) \
        -> tuple[bytes, tuple[int, int, int, int]]:
    actual_hash = hashlib.sha256(image).hexdigest()
    if actual_hash != ORIGINAL_MANU3_SHA256:
        raise ValueError(
            f"original MANU3 sha256 {actual_hash} does not match "
            f"{ORIGINAL_MANU3_SHA256}"
        )
    require_prefix(image, bytes.fromhex("1e2e8b0e"), "original")
    patched = bytearray(image)
    data_delta = struct.unpack_from(
        "<H", patched, ORIGINAL_DATA_DELTA_OFFSET
    )[0]
    data_segment = (CODE_SEGMENT + data_delta) & 0xFFFF
    struct.pack_into(
        "<H", patched, ORIGINAL_DATA_SEGMENT_OFFSET, data_segment
    )
    data_file_offset = data_delta * 16
    work_delta_0, work_delta_1, work_delta_2 = struct.unpack_from(
        "<HHH", patched, data_file_offset + 0x000C
    )
    geometry_segment = (data_segment + work_delta_0) & 0xFFFF
    texture_segment = (geometry_segment + work_delta_1) & 0xFFFF
    raster_segment = (texture_segment + work_delta_2) & 0xFFFF
    struct.pack_into(
        "<HHH",
        patched,
        data_file_offset + 0x0002,
        geometry_segment,
        texture_segment,
        raster_segment,
    )
    raster_file_offset = (raster_segment - CODE_SEGMENT) * 16
    struct.pack_into(
        "<H",
        patched,
        raster_file_offset + RENDER_CONTINUATION_OFFSET,
        0x0AE0,
    )
    return bytes(patched), (
        data_segment,
        geometry_segment,
        texture_segment,
        raster_segment,
    )


def recovered_renderer_entry(link_map: str) -> int:
    matches: list[int] = []
    for line in link_map.splitlines():
        match = MAP_SYMBOL_RE.match(line)
        if match is None or match.group(3) != REBUILT_RENDERER_SYMBOL:
            continue
        segment = int(match.group(1), 16)
        if segment != 0:
            raise ValueError(
                f"{REBUILT_RENDERER_SYMBOL} is in unexpected segment "
                f"{segment:#x}"
            )
        matches.append(int(match.group(2), 16))
    if len(matches) != 1:
        raise ValueError(
            f"expected one {REBUILT_RENDERER_SYMBOL} map symbol, "
            f"found {len(matches)}"
        )
    return matches[0]


def capture_pterra_renderer_input(original_image: bytes, oracle) \
        -> PterraRendererInput:
    image, expected_segments = prepare_original_image(original_image)
    data_segment, geometry_segment, texture_segment, raster_segment = \
        expected_segments
    request_offset = 0x8000
    request = struct.pack(
        "<hhhh", PTERRA_CURSOR_X, PTERRA_CURSOR_Y, 0, 0
    )
    captured: dict[str, object] = {}

    def capture_at_renderer(machine, address, _size, _data) -> None:
        if address != CODE_SEGMENT * 16 + ORIGINAL_RENDERER_ENTRY or captured:
            return
        actual_segments = (
            machine.reg_read(oracle.UC_X86_REG_FS),
            machine.reg_read(oracle.UC_X86_REG_DS),
            struct.unpack(
                "<H",
                machine.mem_read(
                    machine.reg_read(oracle.UC_X86_REG_FS) * 16 + 4, 2
                ),
            )[0],
            machine.reg_read(oracle.UC_X86_REG_ES),
        )
        if actual_segments != expected_segments:
            raise RuntimeError(
                f"original API entered renderer with segments "
                f"{actual_segments!r}, expected {expected_segments!r}"
            )
        captured["segments"] = actual_segments
        captured["globals"] = bytes(
            machine.mem_read(data_segment * 16, 0x10000)
        )
        captured["geometry"] = bytes(
            machine.mem_read(geometry_segment * 16, 0x10000)
        )
        captured["texture"] = bytes(
            machine.mem_read(texture_segment * 16, 0x10000)
        )
        captured["raster"] = bytes(
            machine.mem_read(raster_segment * 16, 0x10000)
        )

    registers = {
        "eax": 0x1111,
        "ebx": 0x2222,
        "ecx": 0x3333,
        "edx": 0x4444,
        "esi": 0x5555,
        "edi": 0x6666,
        "ebp": request_offset,
        "sp": STACK_POINTER,
        "ds": 0x3000,
        "es": 0x4000,
        "fs": 0x5000,
        "gs": 0x6000,
        "ss": STACK_SEGMENT,
        "flags": 0x0202,
    }
    oracle.execute(
        image,
        ORIGINAL_API_ENTRY,
        API_RETURN_ADDRESS,
        registers,
        [
            (STACK_SEGMENT, request_offset, request),
            (
                STACK_SEGMENT,
                STACK_POINTER,
                struct.pack(
                    "<HH", API_RETURN_ADDRESS, API_RETURN_SEGMENT
                ) + b"PTERRA",
            ),
            (API_RETURN_SEGMENT, API_RETURN_ADDRESS, b"\xCC"),
            (FRAMEBUFFER_SEGMENT, 0, bytes(0x10000)),
        ],
        max_instructions=MAX_INSTRUCTIONS,
        output_handler=lambda *_args: None,
        code_handler=capture_at_renderer,
        code_segment=CODE_SEGMENT,
        return_segment=API_RETURN_SEGMENT,
    )
    if not captured:
        raise RuntimeError("original API never reached the MANU3 renderer")
    globals_image = bytes(captured["globals"])
    face_list = struct.unpack_from(
        "<H", globals_image, FACE_LIST_OFFSET
    )[0]
    face_count = struct.unpack_from(
        "<H", globals_image, FACE_COUNT_OFFSET
    )[0]
    if (face_list, face_count) != (EXPECTED_FACE_LIST, EXPECTED_FACE_COUNT):
        raise RuntimeError(
            f"captured mesh is face list {face_list:#06x}, count "
            f"{face_count}; expected {EXPECTED_FACE_LIST:#06x}, "
            f"{EXPECTED_FACE_COUNT}"
        )
    return PterraRendererInput(
        globals_image=globals_image,
        geometry_image=bytes(captured["geometry"]),
        texture_image=bytes(captured["texture"]),
        raster_image=bytes(captured["raster"]),
        source_segments=tuple(captured["segments"]),
    )


def relocated_input(renderer_input: PterraRendererInput) \
        -> tuple[bytes, bytes]:
    globals_image = bytearray(renderer_input.globals_image)
    raster_image = bytearray(renderer_input.raster_image)
    struct.pack_into(
        "<HHH",
        globals_image,
        0x0002,
        GEOMETRY_SEGMENT,
        TEXTURE_SEGMENT,
        RASTER_SEGMENT,
    )
    struct.pack_into("<H", globals_image, 0x0014, FRAMEBUFFER_SEGMENT)
    struct.pack_into("<H", globals_image, 0x0018, FRAMEBUFFER_SEGMENT)
    struct.pack_into(
        "<H",
        raster_image,
        RENDER_CONTINUATION_OFFSET,
        ORIGINAL_RENDER_LINEAR_OFFSET,
    )
    return bytes(globals_image), bytes(raster_image)


def run_renderer(
    image: bytes,
    entry: int,
    renderer_input: PterraRendererInput,
    oracle,
    *,
    recovered: bool,
    code_handler=None,
    max_instructions: int = MAX_INSTRUCTIONS,
) -> RendererResult:
    globals_image, raster_image = relocated_input(renderer_input)
    if recovered:
        require_prefix(image, bytes.fromhex("1e8cc82e"), "recovered")
        patched = bytearray(image)
        data_delta = struct.unpack_from(
            "<H", patched, RECOVERED_DATA_DELTA_OFFSET
        )[0]
        struct.pack_into(
            "<H",
            patched,
            RECOVERED_DATA_SEGMENT_OFFSET,
            (CODE_SEGMENT + data_delta) & 0xFFFF,
        )
        data_segment = GLOBALS_SEGMENT
        fs_segment = GLOBALS_SEGMENT
        eax = GEOMETRY_SEGMENT
        edx = RASTER_SEGMENT
    else:
        patched = bytearray(prepare_original_image(image)[0])
        data_segment = GEOMETRY_SEGMENT
        fs_segment = GLOBALS_SEGMENT
        eax = 0xA1A1BEEF
        edx = 0xD4D44567

    machine = oracle.execute(
        bytes(patched),
        entry,
        DIRECT_RETURN_ADDRESS,
        {
            "eax": eax,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": edx,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": STACK_POINTER,
            "ds": data_segment,
            "es": RASTER_SEGMENT,
            "fs": fs_segment,
            "gs": 0x2800,
            "ss": STACK_SEGMENT,
            "flags": 0x0202,
        },
        [
            (GLOBALS_SEGMENT, 0, globals_image),
            (GEOMETRY_SEGMENT, 0, renderer_input.geometry_image),
            (TEXTURE_SEGMENT, 0, renderer_input.texture_image),
            (RASTER_SEGMENT, 0, raster_image),
            (FRAMEBUFFER_SEGMENT, 0, bytes(0x10000)),
            (
                STACK_SEGMENT,
                STACK_POINTER,
                struct.pack("<H", DIRECT_RETURN_ADDRESS) + b"PTERRA",
            ),
        ],
        max_instructions=max_instructions,
        output_handler=lambda *_args: None,
        code_handler=code_handler,
        code_segment=CODE_SEGMENT,
        return_segment=CODE_SEGMENT,
    )
    return RendererResult(
        framebuffer=bytes(
            machine.mem_read(FRAMEBUFFER_SEGMENT * 16, 320 * 200)
        ),
        globals_image=bytes(
            machine.mem_read(GLOBALS_SEGMENT * 16, 0x10000)
        ),
        geometry_image=bytes(
            machine.mem_read(GEOMETRY_SEGMENT * 16, 0x10000)
        ),
        raster_image=bytes(
            machine.mem_read(RASTER_SEGMENT * 16, 0x10000)
        ),
    )


def region_differences(expected: bytes, actual: bytes) \
        -> list[dict[str, int]]:
    return [
        {"offset": offset, "original": old, "recovered": new}
        for offset, (old, new) in enumerate(zip(expected, actual))
        if old != new
    ][:16]


def normalize_raster_ephemeral(image: bytes) -> bytes:
    normalized = bytearray(image)
    for start, end in RASTER_SCRATCH_RANGES:
        normalized[start:end] = bytes(end - start)
    for index in range(RASTER_RECORD_COUNT):
        start = (
            RASTER_POOL_OFFSET
            + index * RASTER_RECORD_SIZE
            + RASTER_SORT_NEXT_OFFSET
        )
        normalized[start:start + 2] = b"\0\0"
    return bytes(normalized)


def result_report(
    original: RendererResult,
    recovered: RendererResult,
) -> dict[str, object]:
    regions = {}
    region_images = (
        ("framebuffer", original.framebuffer, recovered.framebuffer),
        (
            "globals_image",
            original.globals_image,
            recovered.globals_image,
        ),
        (
            "geometry_image",
            original.geometry_image,
            recovered.geometry_image,
        ),
        (
            "raster_semantic",
            normalize_raster_ephemeral(original.raster_image),
            normalize_raster_ephemeral(recovered.raster_image),
        ),
    )
    for name, expected, actual in region_images:
        regions[name] = {
            "match": expected == actual,
            "original_sha256": hashlib.sha256(expected).hexdigest(),
            "recovered_sha256": hashlib.sha256(actual).hexdigest(),
            "first_differences": region_differences(expected, actual),
        }
    raw_raster_differences = sum(
        old != new
        for old, new in zip(original.raster_image, recovered.raster_image)
    )
    return {
        "case": "pterra-real-mesh-x224-y0",
        "face_list": EXPECTED_FACE_LIST,
        "face_count": EXPECTED_FACE_COUNT,
        "regions": regions,
        "ignored_raster_ephemeral": {
            "ranges": [list(value) for value in RASTER_SCRATCH_RANGES],
            "record_sort_next": {
                "pool_offset": RASTER_POOL_OFFSET,
                "record_size": RASTER_RECORD_SIZE,
                "record_count": RASTER_RECORD_COUNT,
                "field_offset": RASTER_SORT_NEXT_OFFSET,
            },
            "difference_count": raw_raster_differences,
        },
        "passed": all(region["match"] for region in regions.values()),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--original",
        type=Path,
        default=ROOT / "output/_tmp_dat/manu3.xdb",
    )
    parser.add_argument(
        "--recovered",
        type=Path,
        default=ROOT / "output/recovered_dos_package/xdb/manu3.xdb",
    )
    parser.add_argument(
        "--recovered-map",
        type=Path,
        default=(
            ROOT
            / "output/recovered_dos_package/validation/source_xdb/manu3"
            / "manu3_source_link.map"
        ),
    )
    parser.add_argument("--report", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    for path in (args.original, args.recovered, args.recovered_map):
        if not path.is_file():
            raise SystemExit(f"missing required artifact: {path}")
    oracle = load_oracle_module()
    original_image = args.original.read_bytes()
    recovered_image = args.recovered.read_bytes()
    try:
        renderer_input = capture_pterra_renderer_input(
            original_image, oracle
        )
        original = run_renderer(
            original_image,
            ORIGINAL_RENDERER_ENTRY,
            renderer_input,
            oracle,
            recovered=False,
        )
        recovered = run_renderer(
            recovered_image,
            recovered_renderer_entry(args.recovered_map.read_text(
                encoding="ascii", errors="strict"
            )),
            renderer_input,
            oracle,
            recovered=True,
        )
    except (RuntimeError, ValueError) as error:
        raise SystemExit(f"FAIL MANU3 Pterra differential: {error}") from error

    report = result_report(original, recovered)
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="ascii",
        )
    if not report["passed"]:
        mismatches = [
            name
            for name, region in report["regions"].items()
            if not region["match"]
        ]
        raise SystemExit(
            "FAIL MANU3 Pterra differential: mismatched "
            + ", ".join(mismatches)
        )
    print(
        "PASS MANU3 Pterra differential: original and recovered renderers "
        "match for 216-face x=224 y=0 mesh"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
