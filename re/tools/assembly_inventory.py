#!/usr/bin/env python3
"""Validate recovered BLOODPRG assembly inventory consistency."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import csv
import hashlib
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
BLOODPRG = REPO_ROOT / "re" / "bin" / "BLOODPRG.EXE"
ROUTINE_INDEX = REPO_ROOT / "re" / "assembly" / "routine_index.tsv"
ASM_ROOT = REPO_ROOT / "re" / "assembly" / "bloodprg"

DIRECT_CALLEE_RE = re.compile(r"^; direct_callees: (.+)$", re.MULTILINE)
FILE_OFFSET_RE = re.compile(r"^; file_offset: 0x([0-9a-fA-F]+)$", re.MULTILINE)
BYTE_COUNT_RE = re.compile(r"^; byte_count: (\d+)$", re.MULTILINE)
SHA_RE = re.compile(r"^; routine_bytes_sha256: ([0-9a-f]+)$", re.MULTILINE)


def load_index() -> list[dict[str, str]]:
    with ROUTINE_INDEX.open(newline="") as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def bloodprg_entries(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    return [row for row in rows if row["module"] == "bloodprg"]


def indexed_entries(rows: list[dict[str, str]]) -> set[int]:
    return {int(row["entry"], 16) for row in bloodprg_entries(rows)}


def parse_metadata(path: Path) -> tuple[int, int, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    file_offset = FILE_OFFSET_RE.search(text)
    byte_count = BYTE_COUNT_RE.search(text)
    sha = SHA_RE.search(text)
    missing = []
    if file_offset is None:
        missing.append("file_offset")
    if byte_count is None:
        missing.append("byte_count")
    if sha is None:
        missing.append("routine_bytes_sha256")
    if missing:
        raise ValueError(f"missing metadata: {', '.join(missing)}")
    return int(file_offset.group(1), 16), int(byte_count.group(1)), sha.group(1)


def check_index_paths(rows: list[dict[str, str]]) -> list[str]:
    errors = []
    for row in bloodprg_entries(rows):
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            errors.append(f"{row['entry']}: missing asm path {row['asm_path']}")
    return errors


def check_hashes(rows: list[dict[str, str]]) -> list[str]:
    blob = BLOODPRG.read_bytes()
    errors = []
    for row in bloodprg_entries(rows):
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            continue
        try:
            file_offset, byte_count, expected = parse_metadata(path)
        except ValueError as exc:
            errors.append(f"{row['entry']} {row['asm_path']}: {exc}")
            continue
        got = hashlib.sha256(blob[file_offset:file_offset + byte_count]).hexdigest()
        if got != expected:
            errors.append(
                f"{row['entry']} {row['asm_path']}: sha mismatch {got} != {expected}"
            )
    return errors


def direct_callees_from_text(text: str) -> list[int]:
    out = []
    for match in DIRECT_CALLEE_RE.finditer(text):
        for value in match.group(1).split(","):
            value = value.strip()
            if not value or value == "none":
                continue
            out.append(int(value, 16))
    return out


def check_direct_callees(rows: list[dict[str, str]]) -> list[str]:
    indexed = indexed_entries(rows)
    errors = []
    for path in sorted(ASM_ROOT.glob("**/*.asm")):
        text = path.read_text(encoding="utf-8", errors="replace")
        for target in direct_callees_from_text(text):
            if target not in indexed:
                rel = path.relative_to(REPO_ROOT)
                errors.append(f"{rel}: direct callee {target:#08x} is not indexed")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="run all inventory checks")
    args = parser.parse_args()

    if not args.check:
        parser.print_help()
        return 0

    rows = load_index()
    errors = []
    errors.extend(check_index_paths(rows))
    errors.extend(check_hashes(rows))
    errors.extend(check_direct_callees(rows))

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(f"OK: {len(bloodprg_entries(rows))} BLOODPRG routine(s) indexed")
    print("OK: routine byte hashes match BLOODPRG.EXE")
    print("OK: direct callee targets are indexed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
