#!/usr/bin/env python3
"""Exercise the recovered DOS runtime through its rebuilt MANU3 overlay."""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
import re
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "re" / "tools" / "drive_real_game.sh"

SEEK_RE = re.compile(
    r"^(?P<pid>\d+)\s+lseek\((?P<fd>\d+)<(?P<path>[^>]*)>, "
    r"(?P<offset>\d+), SEEK_SET\) = (?P<result>\d+)"
)
READ_RE = re.compile(
    r"^(?P<pid>\d+)\s+read\((?P<fd>\d+)<(?P<path>[^>]*)>, .*?, "
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


def merged_read_coverage(
    trace: Path, filename: str, start: int, size: int
) -> int:
    positions: dict[tuple[int, int, str], int] = {}
    intervals: list[tuple[int, int]] = []
    end = start + size

    for line in trace.read_text(encoding="utf-8", errors="replace").splitlines():
        seek = SEEK_RE.match(line)
        if seek:
            if not seek["path"].endswith(f"/{filename}"):
                continue
            key = (int(seek["pid"]), int(seek["fd"]), seek["path"])
            positions[key] = int(seek["result"])
            continue
        read = READ_RE.match(line)
        if not read or not read["path"].endswith(f"/{filename}"):
            continue
        key = (int(read["pid"]), int(read["fd"]), read["path"])
        count = int(read["result"])
        if count <= 0:
            continue
        read_start = positions.get(key, 0)
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


def stage_install_tree(package: Path, install_parent: Path, output: Path) -> Path:
    source = install_parent / "cblood"
    destination_parent = output / "cdrive"
    destination = destination_parent / "cblood"
    if not source.is_dir():
        raise SystemExit(f"missing installed game data: {source}")
    shutil.copytree(source, destination, dirs_exist_ok=True)
    for script in range(1, 6):
        for extension in ("COD", "BAS", "VAR", "DIC", "DEB"):
            filename = f"SCRIPT{script}.{extension}"
            shutil.copy2(package / "cd" / filename, destination / filename)
    return destination_parent


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", required=True, type=Path)
    parser.add_argument("--install-parent", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--display", default=":98")
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument(
        "--through-script2",
        action="store_true",
        help="drive CRYOBOX and Bob's mission until SCRIPT2 is loaded",
    )
    args = parser.parse_args()

    package = args.package_dir.resolve()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    executable = package / "cd" / args.executable
    if not executable.is_file():
        raise SystemExit(f"missing recovered DOS executable: {executable}")

    trace = output / "blood_dat.strace"
    if args.through_script2:
        capture_names = (
            "bridge",
            "cryobox_hover",
            "cryobox_choice",
            "bob_hover",
            "mission_prompt",
            "mission",
            "script2",
        )
    else:
        capture_names = ("title", "bridge", "dispatched")
    capture_paths = tuple(output / f"{name}.png" for name in capture_names)
    for stale_path in (trace, *capture_paths, output / "result.tsv"):
        stale_path.unlink(missing_ok=True)
    if args.through_script2:
        actions = [
            "wait 6",
            "key Escape",
            "wait 2",
            "click 348 344",
            "wait 2",
            "shot bridge",
            "move_relative -300 0",
            "wait 4",
            "move_relative -300 0",
            "wait 3",
            "move_relative 100 -20",
            "wait 0.5",
            "move_relative 100 -20",
            "wait 0.5",
            "shot cryobox_hover",
            "mouse_button 1",
            "wait 1",
            "shot cryobox_choice",
            "move_relative -100 -20",
            "wait 0.5",
            "move_relative -100 -20",
            "wait 0.5",
            "shot bob_hover",
            "mouse_button 1",
            "fastforward 8",
            "wait 3",
            "shot mission_prompt",
            "move_relative 100 0",
            "wait 0.5",
            "mouse_button 1",
            "fastforward 5",
            "wait 1",
            "shot mission",
            "key_down space",
            "fastforward 60",
            "key_up space",
            "fastforward 15",
            "wait 2",
        ]
        actions.append("shot script2")
    else:
        actions = (
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
        )
    input_actions = "\n".join((*actions, ""))
    environment = os.environ.copy()
    environment["DOSBOX_TRACE_FILE"] = str(trace)
    runtime_install_parent = args.install_parent.resolve()
    if args.through_script2:
        runtime_install_parent = stage_install_tree(
            package, runtime_install_parent, output
        )
        trace_paths = [package / "cd" / "BLOOD.DAT"]
        trace_paths.extend(
            runtime_install_parent / "cblood" / f"SCRIPT2.{extension}"
            for extension in ("COD", "BAS", "VAR", "DIC", "DEB")
        )
        environment["DOSBOX_TRACE_PATHS"] = ":".join(map(str, trace_paths))
    subprocess.run(
        [
            "bash",
            str(DRIVER),
            str(package / "cd"),
            str(output),
            args.display,
            str(runtime_install_parent),
            args.executable,
        ],
        input=input_actions,
        text=True,
        check=True,
        env=environment,
    )

    capture_hashes = tuple(map(verify_png, capture_paths))
    if len(set(capture_hashes)) != len(capture_hashes):
        raise SystemExit("input-driven runtime captures did not change state")

    overlay_offset, overlay_size = manu3_archive_range(package)
    coverage = merged_read_coverage(
        trace, "BLOOD.DAT", overlay_offset, overlay_size
    )
    if coverage != overlay_size:
        raise SystemExit(
            "recovered runtime did not read the complete archived MANU3.XDB: "
            f"covered {coverage}/{overlay_size} bytes at 0x{overlay_offset:08x}"
        )

    result_rows = [
        ("runtime_state_changes", "PASS", ",".join(capture_hashes)),
        ("archived_manu3_read", "PASS", f"0x{overlay_offset:08x}+{overlay_size}"),
    ]
    if args.through_script2:
        for extension in ("COD", "BAS", "VAR", "DIC", "DEB"):
            filename = f"SCRIPT2.{extension}"
            file_size = (package / "cd" / filename).stat().st_size
            file_coverage = merged_read_coverage(trace, filename, 0, file_size)
            if file_coverage != file_size:
                raise SystemExit(
                    f"recovered runtime did not read complete {filename}: "
                    f"covered {file_coverage}/{file_size} bytes"
                )
            result_rows.append(
                (f"source_{filename.lower()}_read", "PASS", str(file_size))
            )

    report = output / "result.tsv"
    with report.open("w", encoding="ascii", newline="") as stream:
        writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
        writer.writerow(("gate", "status", "detail"))
        writer.writerows(result_rows)
    print(
        "PASS recovered DOS package: BPRG_RE.EXE changed interactive states and "
        f"read all {overlay_size} bytes of rebuilt MANU3.XDB"
        + (" and all five source-built SCRIPT2 resources" if args.through_script2 else "")
    )


if __name__ == "__main__":
    main()
