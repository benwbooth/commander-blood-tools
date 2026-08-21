#!/usr/bin/env python3
"""Exercise the recovered DOS runtime through its rebuilt MANU3 overlay."""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "re" / "tools" / "drive_real_game.sh"

SEEK_RE = re.compile(
    r"^(?P<pid>\d+)\s+lseek\((?P<fd>\d+)<[^>]*BLOOD\.DAT>, "
    r"(?P<offset>\d+), SEEK_SET\) = (?P<result>\d+)"
)
READ_RE = re.compile(
    r"^(?P<pid>\d+)\s+read\((?P<fd>\d+)<[^>]*BLOOD\.DAT>, .*?, "
    r"(?P<requested>\d+)\) = (?P<result>\d+)"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def manu3_archive_range(package: Path) -> tuple[int, int]:
    manifest = package / "package_manifest.tsv"
    with manifest.open(newline="", encoding="ascii") as stream:
        rows = csv.DictReader(stream, delimiter="\t")
        for row in rows:
            if (row["component"].lower() == "manu3.xdb"
                    and row["output"] == "cd/BLOOD.DAT"):
                _, new_offset = row["offset"].split("->", 1)
                overlay = package / "xdb" / "manu3.xdb"
                return int(new_offset, 16), overlay.stat().st_size
    raise SystemExit(f"{manifest}: archived MANU3.XDB record is missing")


def merged_read_coverage(trace: Path, start: int, size: int) -> int:
    positions: dict[tuple[int, int], int] = {}
    intervals: list[tuple[int, int]] = []
    end = start + size

    for line in trace.read_text(encoding="utf-8", errors="replace").splitlines():
        seek = SEEK_RE.match(line)
        if seek:
            key = (int(seek["pid"]), int(seek["fd"]))
            positions[key] = int(seek["result"])
            continue
        read = READ_RE.match(line)
        if not read:
            continue
        key = (int(read["pid"]), int(read["fd"]))
        count = int(read["result"])
        if count <= 0 or key not in positions:
            continue
        read_start = positions[key]
        read_end = read_start + count
        positions[key] = read_end
        overlap_start = max(start, read_start)
        overlap_end = min(end, read_end)
        if overlap_start < overlap_end:
            intervals.append((overlap_start, overlap_end))

    covered = 0
    cursor = start
    for interval_start, interval_end in sorted(intervals):
        if interval_end <= cursor:
            continue
        if interval_start > cursor:
            break
        cursor = max(cursor, interval_end)
        covered = cursor - start
        if cursor >= end:
            return size
    return covered


def verify_png(path: Path) -> str:
    if not path.is_file() or path.stat().st_size == 0:
        raise SystemExit(f"missing runtime capture: {path}")
    dimensions = subprocess.run(
        ["identify", "-format", "%wx%h", str(path)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if dimensions != "320x200":
        raise SystemExit(f"unexpected capture dimensions for {path}: {dimensions}")
    return sha256(path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", required=True, type=Path)
    parser.add_argument("--install-parent", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--display", default=":98")
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    args = parser.parse_args()

    package = args.package_dir.resolve()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    executable = package / "cd" / args.executable
    if not executable.is_file():
        raise SystemExit(f"missing recovered DOS executable: {executable}")

    trace = output / "blood_dat.strace"
    capture_paths = tuple(output / f"{name}.png" for name in (
        "title", "bridge", "dispatched"
    ))
    for stale_path in (trace, *capture_paths, output / "result.tsv"):
        stale_path.unlink(missing_ok=True)
    input_actions = "\n".join(
        (
            "wait 6",
            "key Escape",
            "wait 2",
            "shot title",
            "click 348 344",
            "wait 4",
            "shot bridge",
            "click 348 344",
            "wait 4",
            "shot dispatched",
            "",
        )
    )
    environment = os.environ.copy()
    environment["DOSBOX_TRACE_FILE"] = str(trace)
    subprocess.run(
        [
            "bash",
            str(DRIVER),
            str(package / "cd"),
            str(output),
            args.display,
            str(args.install_parent.resolve()),
            args.executable,
        ],
        input=input_actions,
        text=True,
        check=True,
        env=environment,
    )

    title_hash, bridge_hash, dispatched_hash = map(verify_png, capture_paths)
    if len({title_hash, bridge_hash, dispatched_hash}) != 3:
        raise SystemExit("input-driven runtime captures did not change state")

    overlay_offset, overlay_size = manu3_archive_range(package)
    coverage = merged_read_coverage(trace, overlay_offset, overlay_size)
    if coverage != overlay_size:
        raise SystemExit(
            "recovered runtime did not read the complete archived MANU3.XDB: "
            f"covered {coverage}/{overlay_size} bytes at 0x{overlay_offset:08x}"
        )

    report = output / "result.tsv"
    report.write_text(
        "gate\tstatus\tdetail\n"
        f"runtime_state_changes\tPASS\t{title_hash},{bridge_hash},{dispatched_hash}\n"
        f"archived_manu3_read\tPASS\t0x{overlay_offset:08x}+{overlay_size}\n",
        encoding="ascii",
    )
    print(
        "PASS recovered DOS package: BPRG_RE.EXE changed interactive states and "
        f"read all {overlay_size} bytes of rebuilt MANU3.XDB"
    )


if __name__ == "__main__":
    main()
