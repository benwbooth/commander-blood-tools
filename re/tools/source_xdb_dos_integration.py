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
HARNESS = ROOT / "re" / "integration" / "dos" / "source_xdb_manu3_loader.c"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--dosbox", default="dosbox-x")
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
    source_xdb = build_dir / "manu3.xdb"
    build_report = build_dir / "build.tsv"
    link_map = build_dir / "manu3_source_link.map"
    for path in (source_xdb, build_report, link_map):
        if not path.is_file():
            raise SystemExit(f"missing source-XDB build artifact: {path}")

    with build_report.open(newline="", encoding="ascii") as handle:
        row = next(csv.DictReader(handle, delimiter="\t"))
    if row["module"] != "manu3":
        raise SystemExit("the current DOS loader gate requires a MANU3 build")
    image_bytes = int(row["rebuilt_bytes"])
    data_paragraph = int(row["data_file_base"], 0) // 16
    symbols = map_symbols(link_map)
    state_offset = symbols.get("_xdb_manu3_data_segment")
    if state_offset is None or state_offset >= 0x10000:
        raise SystemExit("MANU3 data-segment state is absent from the linked code segment")

    output = args.output_dir.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    shutil.copy2(source_xdb, output / "MANU3.XDB")
    dos_executable = output / "XDBLOAD.EXE"
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
            f"-i={INCLUDE_DIR}",
            f"-fe={dos_executable}",
            f"-fm={output / 'XDBLOAD.MAP'}",
            str(HARNESS),
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
    expected = "PASS source-linked MANU3 XDB"
    if result != expected:
        console = output / "CONSOLE.TXT"
        diagnostics = (
            console.read_text(encoding="ascii", errors="replace")
            if console.is_file()
            else ""
        )
        raise SystemExit(f"source-XDB DOS failure: {result!r}\n{diagnostics}")
    print(f"{expected}: {image_bytes} byte raw overlay")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
