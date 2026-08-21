#!/usr/bin/env python3
"""Diff the boot-time game-data state of the relinked runtime against the original.

Runs re/tools/dump_dosbox_mem.py once per executable (original ISO tree vs
recovered package CD), parses the STARTUP_GLOBALS table each run prints, and
classifies every difference:

  OK            identical value
  ZERO-VS-SET   one side zero/negative-one sentinel, the other populated --
                the signature of an initializer the recovered startup omitted
  DIFFERENT     both populated but unequal -- expected for layout-dependent
                buffer segments and live handles; listed for review

Usage:
  python3 re/tools/diff_boot_state.py [--cd-dir DIR] [--iso-dir DIR]
      [--install-parent DIR] [--output FILE]

The heavy lifting (locating DS, reading guest memory) stays inside
dump_dosbox_mem.py; this wrapper only launches it twice and reconciles.
"""
from __future__ import annotations

import argparse
import csv
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DUMPER = ROOT / "re" / "tools" / "dump_dosbox_mem.py"

VALUE_RE = re.compile(r"^([a-z0-9_]+_0[0-9A-Fa-f]{3}): (.+)$")

SENTINELS = {"0", "-1", "(0, 0)", "(0, -1)", "(-1, -1)"}


def capture(cd_dir: Path, install_parent: Path, executable: str,
            wait_ticks: int | None, wait_seconds: int) -> dict[str, str]:
    environment = dict(os.environ)
    if wait_ticks is not None:
        # Matched GUEST time: hold the capture until the game's own tick
        # counter reaches the target. Wall-clock waits compare different
        # story points because the natural-C frame costs more instructions.
        environment["BLOODPRG_WAIT_GLOBAL"] = f"0xb29:2:{wait_ticks}"
        environment["BLOODPRG_WAIT_GLOBAL_TIMEOUT"] = "600"
    result = subprocess.run(
        [
            sys.executable,
            str(DUMPER),
            str(cd_dir),
            str(wait_seconds),
            str(install_parent),
            executable,
        ],
        capture_output=True,
        text=True,
        check=True,
        env=environment,
    )
    values: dict[str, str] = {}
    for line in result.stdout.splitlines():
        match = VALUE_RE.match(line.strip())
        if match:
            values[match.group(1)] = match.group(2)
    return values


def classify(name: str, original: str, rebuilt: str) -> str:
    if original == rebuilt:
        return "OK"
    if original in SENTINELS and rebuilt not in SENTINELS:
        return "MISSING-IN-REBUILT"
    if rebuilt in SENTINELS and original not in SENTINELS:
        return "LOST-BY-REBUILD"
    if "_handle_" in name or name.endswith("_0a84"):
        return "DIFFERENT-HANDLE"
    return "DIFFERENT"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--wait-ticks", type=int,
                        help="capture when guest tick_count 0xB29 reaches "
                             "this value (matched story points)")
    parser.add_argument("--wait-seconds", type=int, default=25)
    parser.add_argument("--cd-dir", type=Path,
                        default=ROOT / "output/recovered_dos_package/cd")
    parser.add_argument("--iso-dir", type=Path, default=ROOT / "output/_tmp_iso")
    parser.add_argument("--install-parent", type=Path,
                        default=Path("/tmp/opencode/user_play/install"))
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--original-executable", default="BLOODPRG.EXE")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    original = capture(args.iso_dir, args.install_parent,
                       args.original_executable, args.wait_ticks,
                       args.wait_seconds)
    rebuilt = capture(args.cd_dir, args.install_parent, args.executable,
                      args.wait_ticks, args.wait_seconds)

    rows = []
    for name in sorted(set(original) | set(rebuilt)):
        left = original.get(name, "<absent>")
        right = rebuilt.get(name, "<absent>")
        rows.append((name, left, right, classify(name, left, right)))

    writer_args: dict = {}
    stream = open(args.output, "w", newline="", encoding="ascii") \
        if args.output else sys.stdout
    writer = csv.writer(stream, delimiter="\t", **writer_args)
    writer.writerow(("global", "original", "relinked", "classification"))
    writer.writerows(rows)
    if args.output:
        stream.close()

    interesting = [r for r in rows if r[3] != "OK"]
    print(f"{len(rows)} globals compared, {len(interesting)} differ")
    for name, left, right, verdict in interesting:
        print(f"  {verdict:18} {name}: original={left} relinked={right}")


if __name__ == "__main__":
    main()
