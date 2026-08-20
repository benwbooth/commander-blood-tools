#!/usr/bin/env python3
"""Load and call a source-linked raw XDB under DOSBox-X."""

from __future__ import annotations

from pathlib import Path
import sys

if sys.path and Path(sys.path[0]).resolve() == Path(__file__).resolve().parent:
    del sys.path[0]

import argparse
import csv
import os
import re
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
INCLUDE_DIR = ROOT / "re" / "source" / "xdb" / "candidates" / "include"
MANU3_HARNESS = ROOT / "re" / "integration" / "dos" / "source_xdb_manu3_loader.c"
ALIEN_HARNESS = ROOT / "re" / "integration" / "dos" / "source_xdb_alien_loader.c"
ALIEN_RENDER_OFFSETS = {
    "amer": (0x0944, 0x28D0),
    "croolis": (0x0946, 0x2940),
    "scrut": (0x0946, 0x2A00),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument(
        "--dump-raster",
        action="store_true",
        help="write the post-call 64 KiB raster segment to RASTER.BIN",
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


def map_symbols(path: Path) -> dict[str, int]:
    pattern = re.compile(
        r"^([0-9A-Fa-f]{4}):([0-9A-Fa-f]{4})[+* ]+([A-Za-z_]\w*)$"
    )
    result: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line.rstrip())
        if match:
            result[match.group(3)] = (
                int(match.group(1), 16) * 16 + int(match.group(2), 16)
            )
    return result


def main() -> int:
    args = parse_args()
    build_dir = args.build_dir.resolve()
    build_report = build_dir / "build.tsv"
    if not build_report.is_file():
        raise SystemExit(f"missing source-XDB build artifact: {build_report}")
    with build_report.open(newline="", encoding="ascii") as handle:
        row = next(csv.DictReader(handle, delimiter="\t"))
    module = row["module"]
    if module not in ("manu3", *ALIEN_RENDER_OFFSETS):
        raise SystemExit(f"unsupported source-XDB module: {module}")

    source_xdb = build_dir / f"{module}.xdb"
    link_map = build_dir / f"{module}_source_link.map"
    for path in (source_xdb, link_map):
        if not path.is_file():
            raise SystemExit(f"missing source-XDB build artifact: {path}")
    image_bytes = int(row["rebuilt_bytes"])
    data_paragraph = int(row["data_file_base"], 0) // 16
    symbols = map_symbols(link_map)
    state_offset = symbols.get(f"_xdb_{module}_data_segment")
    if state_offset is None or state_offset >= 0x10000:
        raise SystemExit(
            f"{module.upper()} data-segment state is absent from the linked code segment"
        )

    output = args.output_dir.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    alien = module != "manu3"
    shutil.copy2(source_xdb, output / ("ALIEN.XDB" if alien else "MANU3.XDB"))
    dos_executable = output / "XDBLOAD.EXE"
    module_defines = []
    harness = MANU3_HARNESS
    expected = "PASS source-linked MANU3 XDB"
    if alien:
        continuation_offset, mode_offset = ALIEN_RENDER_OFFSETS[module]
        module_defines = [
            f"-dXDB_RENDER_CONTINUATION_OFFSET=0x{continuation_offset:04x}U",
            f"-dXDB_RENDER_MODE_OFFSET=0x{mode_offset:04x}U",
        ]
        if args.dump_raster:
            raster_state_offset = symbols.get("_xdb_alien_raster_segment")
            if raster_state_offset is None or raster_state_offset >= 0x10000:
                raise SystemExit(
                    f"{module.upper()} raster-segment state is absent from linked code"
                )
            module_defines.extend(
                (
                    f"-dXDB_RASTER_STATE_OFFSET=0x{raster_state_offset:04x}U",
                    "-dXDB_DUMP_RASTER",
                )
            )
        harness = ALIEN_HARNESS
        expected = "PASS source-linked alien XDB"
    run(
        [
            executable(args.wcl),
            "-q",
            "-3",
            "-ox",
            "-mm",
            "-zdp",
            "-we",
            "-lr",
            f"-dXDB_IMAGE_BYTES={image_bytes}UL",
            f"-dXDB_DATA_PARAGRAPH=0x{data_paragraph:04x}U",
            f"-dXDB_DATA_STATE_OFFSET=0x{state_offset:04x}U",
            *module_defines,
            f"-i={INCLUDE_DIR}",
            f"-fe={dos_executable}",
            f"-fm={output / 'XDBLOAD.MAP'}",
            str(harness),
        ],
        output,
    )

    environment = os.environ.copy()
    environment["SDL_AUDIODRIVER"] = "dummy"
    environment["SDL_VIDEODRIVER"] = "offscreen"
    run(
        [
            executable(args.dosbox),
            "--noprimaryconf",
            "--nolocalconf",
            "--exit",
            "-silent",
            "-set",
            "sdl fullscreen=false",
            "-set",
            "sdl output=surface",
            "-c",
            f'mount c "{output}"',
            "-c",
            "c:",
            "-c",
            "XDBLOAD.EXE > CONSOLE.TXT",
        ],
        output,
        timeout=30,
    )
    result_path = output / "RESULT.TXT"
    if not result_path.is_file():
        raise SystemExit("source-XDB DOS loader did not produce RESULT.TXT")
    result = result_path.read_text(encoding="ascii").strip()
    if result != expected:
        console = output / "CONSOLE.TXT"
        diagnostics = (
            console.read_text(encoding="ascii", errors="replace")
            if console.is_file()
            else ""
        )
        raise SystemExit(f"source-XDB DOS failure: {result!r}\n{diagnostics}")
    print(f"{expected}: {module.upper()}, {image_bytes} byte raw overlay")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
