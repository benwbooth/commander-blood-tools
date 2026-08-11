#!/usr/bin/env python3
"""Validate natural-C source candidates before compiler comparison."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import csv
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
CANDIDATE_ROOT = REPO_ROOT / "re" / "source" / "bloodprg" / "candidates"
MANIFEST = CANDIDATE_ROOT / "manifest.tsv"

FORBIDDEN_SOURCE_TOKENS = [
    "read16_far",
    "write16_far",
    "cb_read",
    "cb_write",
    "machine_state",
    "register_state",
    "CbMachine",
]


def load_manifest() -> list[dict[str, str]]:
    with MANIFEST.open(newline="") as fh:
        rows = list(csv.DictReader(fh, delimiter="\t"))
    required = {"entry", "source", "asm_path", "function", "status", "notes"}
    missing = required.difference(rows[0].keys() if rows else set())
    if missing:
        raise SystemExit(f"{MANIFEST}: missing columns: {', '.join(sorted(missing))}")
    return rows


def function_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r"\b(?:void|int|unsigned|signed|cb_u8|cb_u16|char|short|long)\s+"
        r"(?:CB_NEAR\s+)?"
        + re.escape(name)
        + r"\s*\(",
        re.MULTILINE,
    )


def check_candidates(rows: list[dict[str, str]]) -> int:
    errors: list[str] = []
    seen_entries: set[str] = set()
    manifest_sources: set[Path] = set()

    for row in rows:
        entry = row["entry"].lower()
        source = CANDIDATE_ROOT / row["source"]
        asm_path = REPO_ROOT / row["asm_path"]
        function = row["function"]

        if entry in seen_entries:
            errors.append(f"{entry}: duplicate manifest entry")
        seen_entries.add(entry)

        if source in manifest_sources:
            errors.append(f"{entry}: duplicate source {source}")
        manifest_sources.add(source)

        if not source.exists():
            errors.append(f"{entry}: missing source {source}")
            continue
        if not asm_path.exists():
            errors.append(f"{entry}: missing assembly {asm_path}")

        text = source.read_text(encoding="utf-8", errors="replace")
        if not function_pattern(function).search(text):
            errors.append(f"{entry}: function {function} not found")

    actual_sources = set(CANDIDATE_ROOT.glob("**/*.c"))
    for source in sorted(actual_sources.difference(manifest_sources)):
        errors.append(f"unmanifested source {source}")

    for path in sorted(CANDIDATE_ROOT.glob("**/*")):
        if path.suffix not in {".c", ".h"}:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for token in FORBIDDEN_SOURCE_TOKENS:
            if token in text:
                errors.append(f"{path}: forbidden token {token}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: {len(rows)} natural-C candidate(s)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate candidate files")
    args = parser.parse_args()

    rows = load_manifest()

    if args.check:
        return check_candidates(rows)

    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
