#!/usr/bin/env python3
"""Run fresh-boot DOS invariant and world-profile gates for a package."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
WATCHDOG = ROOT / "re" / "tools" / "runtime_watchdog.py"
SCRIPT_EXTENSIONS = ("COD", "BAS", "VAR", "DIC", "DEB")
PROFILE_COUNT = 5


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def stage_install_tree(
    package: Path, install_parent: Path, destination_parent: Path
) -> None:
    source = install_parent / "cblood"
    destination = destination_parent / "cblood"
    if not source.is_dir():
        raise SystemExit(f"missing installed game data: {source}")
    shutil.copytree(source, destination)
    for script in range(1, PROFILE_COUNT + 1):
        for extension in SCRIPT_EXTENSIONS:
            filename = f"SCRIPT{script}.{extension}"
            recovered = package / "cd" / filename
            if not recovered.is_file():
                raise SystemExit(f"missing recovered script resource: {recovered}")
            shutil.copy2(recovered, destination / filename)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-dir", required=True, type=Path)
    parser.add_argument("--install-parent", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--display-base", type=int, default=100)
    parser.add_argument("--seconds", type=float, default=60.0)
    parser.add_argument("--calibration-timeout", type=float, default=15.0)
    parser.add_argument("--poll-seconds", type=float, default=0.05)
    parser.add_argument("--post-teleport-samples", type=int, default=4)
    parser.add_argument(
        "--profile",
        type=int,
        action="append",
        help="profile 0..4 to test; defaults to all five",
    )
    args = parser.parse_args()

    package = args.package_dir.resolve()
    install_parent = args.install_parent.resolve()
    output = args.output_dir.resolve()
    executable = package / "cd" / args.executable
    link_map = package / "validation" / "bloodprg_runtime" / "final" / "link.map"
    if not executable.is_file():
        raise SystemExit(f"missing recovered DOS executable: {executable}")
    if not link_map.is_file():
        raise SystemExit(f"missing recovered DOS link map: {link_map}")
    executable_hash = sha256(executable)

    profiles = (
        args.profile if args.profile is not None else list(range(PROFILE_COUNT))
    )
    invalid = [profile for profile in profiles if not 0 <= profile < PROFILE_COUNT]
    if invalid:
        raise SystemExit(
            "profiles must be in 0..4: " + ", ".join(map(str, invalid))
        )
    if len(set(profiles)) != len(profiles):
        raise SystemExit("each profile may be requested only once")

    output.mkdir(parents=True, exist_ok=True)
    rows: list[tuple[int, str, str, str, str]] = []
    failures: list[int] = []
    for index, profile in enumerate(profiles):
        profile_dir = output / f"profile-{profile}"
        if profile_dir.exists():
            shutil.rmtree(profile_dir)
        profile_dir.mkdir(parents=True)
        cdrive = profile_dir / "cdrive"
        stage_install_tree(package, install_parent, cdrive)
        report = profile_dir / "watchdog.json"
        command = [
            sys.executable,
            "-P",
            str(WATCHDOG),
            "--cd-dir",
            str(package / "cd"),
            "--install-parent",
            str(cdrive),
            "--executable",
            args.executable,
            "--link-map",
            str(link_map),
            "--dosbox",
            args.dosbox,
            "--display",
            f":{args.display_base + index}",
            "--seconds",
            str(args.seconds),
            "--calibration-timeout",
            str(args.calibration_timeout),
            "--poll-seconds",
            str(args.poll_seconds),
            "--teleport-profile",
            str(profile),
            "--post-teleport-samples",
            str(args.post_teleport_samples),
            "--xvfb",
            "--report",
            str(report),
        ]
        process = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        (profile_dir / "watchdog.log").write_text(
            "$ " + shlex.join(command) + "\n" + process.stdout + process.stderr,
            encoding="utf-8",
        )
        result = (
            json.loads(report.read_text(encoding="utf-8"))
            if report.is_file()
            else {}
        )
        teleports = result.get("teleports", [])
        completed = teleports[0].get("completed_state", {}) if teleports else {}
        verdict = str(result.get("verdict", "NO-REPORT"))
        handles = completed.get("handles", [])
        expected_handles = completed.get("expected_handles", [])
        images = completed.get("images", [])
        blockers = completed.get("blockers", {})
        exact_completion = (
            len(teleports) == 1
            and teleports[0].get("target") == profile
            and completed.get("profile") == profile
            and completed.get("request") == -1
            and completed.get("execution_enabled") == 1
            and handles == expected_handles
            and len(images) == 5
            and all(not image.startswith("0000:") for image in images)
            and blockers
            and all(value == 0 for value in blockers.values())
            and not result.get("anomalies")
        )
        if (
            process.returncode == 0
            and verdict == "TELEPORTS-COMPLETE"
            and exact_completion
        ):
            status = "PASS"
        else:
            status = "FAIL"
            failures.append(profile)
        rows.append(
            (
                profile,
                status,
                verdict,
                ",".join(map(str, handles)),
                executable_hash,
            )
        )
        print(f"profile {profile}: {status} {verdict}")

    with (output / "result.tsv").open("w", encoding="ascii", newline="") as stream:
        writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
        writer.writerow(
            ("profile", "status", "verdict", "handles", "executable_sha256")
        )
        writer.writerows(rows)

    if failures:
        print("failed profiles: " + ", ".join(map(str, failures)), file=sys.stderr)
        return 1
    print(f"PASS fresh-boot DOS invariant gate for {len(profiles)} profile(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
