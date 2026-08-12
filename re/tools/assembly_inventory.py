#!/usr/bin/env python3
"""Validate recovered assembly inventory consistency."""

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
ROUTINE_INDEX = REPO_ROOT / "re" / "assembly" / "routine_index.tsv"
BOUNDARY_OVERRIDES = REPO_ROOT / "re" / "assembly" / "boundary_overrides.tsv"

DIRECT_CALLEE_RE = re.compile(r"^; direct_callees: (.+)$", re.MULTILINE)
ARTIFACT_RE = re.compile(r"^; artifact: (.+)$", re.MULTILINE)
ARTIFACT_SHA_RE = re.compile(r"^; artifact_sha256: ([0-9a-f]+)$", re.MULTILINE)
FILE_OFFSET_RE = re.compile(r"^; file_offset: 0x([0-9a-fA-F]+)$", re.MULTILINE)
OVERLAY_OFFSET_RE = re.compile(r"^; overlay_offset: 0x([0-9a-fA-F]+)$", re.MULTILINE)
BYTE_COUNT_RE = re.compile(r"^; byte_count: (\d+)$", re.MULTILINE)
SHA_RE = re.compile(r"^; routine_bytes_sha256: ([0-9a-f]+)$", re.MULTILINE)
ASM_LINE_RE = re.compile(r"^([0-9a-fA-F]{6,8}):  (.+)$")
BYTE_TOKEN_RE = re.compile(r"^[0-9a-fA-F]{2}$")


def load_index() -> list[dict[str, str]]:
    with ROUTINE_INDEX.open(newline="") as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def load_boundary_overrides() -> list[dict[str, str]]:
    with BOUNDARY_OVERRIDES.open(newline="") as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def indexed_entries(rows: list[dict[str, str]]) -> dict[str, set[int]]:
    indexed: dict[str, set[int]] = {}
    for row in rows:
        indexed.setdefault(row["module"], set()).add(int(row["entry"], 16))
    return indexed


def parse_metadata(path: Path) -> tuple[Path, str, int, int, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    artifact = ARTIFACT_RE.search(text)
    artifact_sha = ARTIFACT_SHA_RE.search(text)
    file_offset = FILE_OFFSET_RE.search(text)
    overlay_offset = OVERLAY_OFFSET_RE.search(text)
    byte_count = BYTE_COUNT_RE.search(text)
    sha = SHA_RE.search(text)
    missing = []
    if artifact is None:
        missing.append("artifact")
    if artifact_sha is None:
        missing.append("artifact_sha256")
    if file_offset is None and overlay_offset is None:
        missing.append("file_offset or overlay_offset")
    if byte_count is None:
        missing.append("byte_count")
    if sha is None:
        missing.append("routine_bytes_sha256")
    if missing:
        raise ValueError(f"missing metadata: {', '.join(missing)}")
    offset = file_offset or overlay_offset
    return (
        REPO_ROOT / artifact.group(1),
        artifact_sha.group(1),
        int(offset.group(1), 16),
        int(byte_count.group(1)),
        sha.group(1),
    )


def check_index_paths(rows: list[dict[str, str]]) -> list[str]:
    errors = []
    for row in rows:
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            errors.append(f"{row['entry']}: missing asm path {row['asm_path']}")
    return errors


def check_hashes(rows: list[dict[str, str]]) -> list[str]:
    errors = []
    artifact_cache: dict[Path, bytes] = {}
    artifact_hashes: dict[Path, str] = {}

    for row in rows:
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            continue
        try:
            artifact, expected_artifact_sha, file_offset, byte_count, expected = parse_metadata(path)
        except ValueError as exc:
            errors.append(f"{row['entry']} {row['asm_path']}: {exc}")
            continue
        if not artifact.exists():
            errors.append(f"{row['entry']} {row['asm_path']}: missing artifact {artifact}")
            continue
        if artifact not in artifact_cache:
            blob = artifact.read_bytes()
            artifact_cache[artifact] = blob
            artifact_hashes[artifact] = hashlib.sha256(blob).hexdigest()
        blob = artifact_cache[artifact]
        artifact_sha = artifact_hashes[artifact]
        if artifact_sha != expected_artifact_sha:
            errors.append(
                f"{row['entry']} {row['asm_path']}: artifact sha mismatch "
                f"{artifact_sha} != {expected_artifact_sha}"
            )
            continue
        got = hashlib.sha256(blob[file_offset:file_offset + byte_count]).hexdigest()
        if got != expected:
            errors.append(
                f"{row['entry']} {row['asm_path']}: sha mismatch {got} != {expected}"
            )
    return errors


def check_disassembly_bytes(
    rows: list[dict[str, str]],
) -> tuple[list[str], int]:
    errors = []
    checked = 0
    artifact_cache: dict[Path, bytes] = {}

    for row in rows:
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        artifact_match = ARTIFACT_RE.search(text)
        if artifact_match is None:
            continue
        artifact = REPO_ROOT / artifact_match.group(1)
        if not artifact.exists():
            continue
        if artifact not in artifact_cache:
            artifact_cache[artifact] = artifact.read_bytes()
        blob = artifact_cache[artifact]

        for line_number, line in enumerate(text.splitlines(), 1):
            match = ASM_LINE_RE.match(line)
            if match is None:
                continue
            byte_tokens = []
            for token in match.group(2).split():
                if BYTE_TOKEN_RE.fullmatch(token) is None:
                    break
                byte_tokens.append(token)
            if not byte_tokens:
                errors.append(f"{row['asm_path']}:{line_number}: missing byte listing")
                continue
            address = int(match.group(1), 16)
            expected = bytes.fromhex(" ".join(byte_tokens))
            actual = blob[address : address + len(expected)]
            if actual != expected:
                errors.append(
                    f"{row['asm_path']}:{line_number}: instruction bytes at "
                    f"{address:#08x} do not match {artifact.relative_to(REPO_ROOT)}"
                )
            checked += 1

    return errors, checked


def direct_callees_from_text(text: str) -> list[int]:
    out = []
    for match in DIRECT_CALLEE_RE.finditer(text):
        for value in match.group(1).split(","):
            value = value.strip()
            if not value or value == "none":
                continue
            out.append(int(value, 16))
    return out


def check_direct_callees(
    rows: list[dict[str, str]], overrides: list[dict[str, str]]
) -> list[str]:
    indexed = indexed_entries(rows)
    merged_entries: dict[str, set[int]] = {}
    for override in overrides:
        if override["disposition"] == "merged_into_owner":
            merged_entries.setdefault(override["module"], set()).add(
                int(override["entry"], 16)
            )
    errors = []
    for row in rows:
        path = REPO_ROOT / row["asm_path"]
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for target in direct_callees_from_text(text):
            if (
                target not in indexed.get(row["module"], set())
                and target not in merged_entries.get(row["module"], set())
            ):
                rel = path.relative_to(REPO_ROOT)
                errors.append(
                    f"{rel}: direct callee {target:#08x} is not indexed in {row['module']}"
                )
    return errors


def check_boundary_overrides(
    rows: list[dict[str, str]], overrides: list[dict[str, str]]
) -> list[str]:
    indexed = indexed_entries(rows)
    row_by_key = {
        (row["module"], int(row["entry"], 16)): row
        for row in rows
    }
    errors = []
    seen = set()

    for override in overrides:
        module = override["module"]
        entry = int(override["entry"], 16)
        owner = int(override["owner"], 16)
        key = (module, entry)
        if key in seen:
            errors.append(f"duplicate boundary override {module}:{entry:#08x}")
        seen.add(key)
        if override["disposition"] != "merged_into_owner":
            errors.append(
                f"{module}:{entry:#08x}: unknown boundary disposition "
                f"{override['disposition']}"
            )
        if entry in indexed.get(module, set()):
            errors.append(f"{module}:{entry:#08x}: merged entry remains indexed")
        owner_row = row_by_key.get((module, owner))
        if owner_row is None:
            errors.append(f"{module}:{entry:#08x}: owner {owner:#08x} is not indexed")
            continue
        owner_path = REPO_ROOT / owner_row["asm_path"]
        try:
            _artifact, _artifact_sha, start, byte_count, _sha = parse_metadata(owner_path)
        except ValueError as exc:
            errors.append(f"{module}:{entry:#08x}: invalid owner metadata: {exc}")
            continue
        if not start <= entry < start + byte_count:
            errors.append(
                f"{module}:{entry:#08x}: outside owner {owner:#08x} "
                f"range {start:#08x}..{start + byte_count:#08x}"
            )

    return errors


def module_counts(rows: list[dict[str, str]]) -> list[tuple[str, int]]:
    counts: dict[str, int] = {}
    for row in rows:
        counts[row["module"]] = counts.get(row["module"], 0) + 1
    return sorted(counts.items())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="run all inventory checks")
    args = parser.parse_args()

    if not args.check:
        parser.print_help()
        return 0

    rows = load_index()
    overrides = load_boundary_overrides()
    errors = []
    errors.extend(check_index_paths(rows))
    errors.extend(check_hashes(rows))
    disassembly_errors, checked_instructions = check_disassembly_bytes(rows)
    errors.extend(disassembly_errors)
    errors.extend(check_direct_callees(rows, overrides))
    errors.extend(check_boundary_overrides(rows, overrides))

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(f"OK: {len(rows)} routine(s) indexed")
    for module, count in module_counts(rows):
        print(f"OK: {module}: {count} routine(s)")
    print("OK: routine byte hashes match source artifacts")
    print(
        f"OK: {checked_instructions} listed instruction byte sequence(s) "
        "match source artifacts"
    )
    print("OK: direct callee targets are indexed or reviewed merged entries")
    print(f"OK: {len(overrides)} reviewed boundary override(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
