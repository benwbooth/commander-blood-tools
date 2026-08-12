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
import json
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_ROOT = REPO_ROOT / "re" / "source"
ROUTINE_INDEX = REPO_ROOT / "re" / "assembly" / "routine_index.tsv"

FORBIDDEN_SOURCE_TOKENS = [
    "read16_far",
    "write16_far",
    "cb_read",
    "cb_write",
    "machine_state",
    "register_state",
    "CbMachine",
]


def manifest_paths() -> list[Path]:
    return sorted(SOURCE_ROOT.glob("**/candidates/manifest.tsv"))


def default_module_for_manifest(manifest: Path) -> str:
    try:
        rel = manifest.relative_to(SOURCE_ROOT)
    except ValueError:
        return ""
    if rel.parts and rel.parts[0] == "bloodprg":
        return "bloodprg"
    return ""


def parse_entry_key(entry: str, default_module: str) -> tuple[str, int]:
    if ":" in entry:
        module, value = entry.split(":", 1)
    else:
        module, value = default_module, entry
    if not module:
        raise ValueError(f"{entry}: missing module prefix")
    return module, int(value, 16)


def load_manifests() -> list[tuple[Path, Path, list[dict[str, str]]]]:
    manifests: list[tuple[Path, Path, list[dict[str, str]]]] = []
    required = {"entry", "source", "asm_path", "function", "status", "notes"}

    for manifest in manifest_paths():
        with manifest.open(newline="") as fh:
            rows = list(csv.DictReader(fh, delimiter="\t"))
        missing = required.difference(rows[0].keys() if rows else set())
        if missing:
            raise SystemExit(f"{manifest}: missing columns: {', '.join(sorted(missing))}")
        manifests.append((manifest, manifest.parent, rows))

    if not manifests:
        raise SystemExit(f"{SOURCE_ROOT}: no candidate manifests found")

    return manifests


def load_routine_index() -> list[dict[str, str]]:
    with ROUTINE_INDEX.open(newline="") as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def function_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r"\b(?:void|int|unsigned|signed|cb_u8|cb_u16|cb_u32|cb_i8|cb_i16|cb_i32|char|short|long|[A-Za-z_][A-Za-z0-9_]*)\s+"
        r"(?:(?:CB_NEAR|CB_FAR|XDB_NEAR|XDB_FAR)\s+)?"
        + re.escape(name)
        + r"\s*\(",
        re.MULTILINE,
    )


def check_candidates(manifests: list[tuple[Path, Path, list[dict[str, str]]]]) -> int:
    errors: list[str] = []
    total_rows = 0
    candidate_roots: set[Path] = set()

    for manifest, candidate_root, rows in manifests:
        seen_entries: set[str] = set()
        manifest_sources: set[Path] = set()
        candidate_roots.add(candidate_root)
        total_rows += len(rows)

        for row in rows:
            entry = row["entry"].lower()
            source = candidate_root / row["source"]
            asm_path = REPO_ROOT / row["asm_path"]
            function = row["function"]

            if entry in seen_entries:
                errors.append(f"{manifest}: {entry}: duplicate manifest entry")
            seen_entries.add(entry)

            if source in manifest_sources:
                errors.append(f"{manifest}: {entry}: duplicate source {source}")
            manifest_sources.add(source)

            if not source.exists():
                errors.append(f"{manifest}: {entry}: missing source {source}")
                continue
            if not asm_path.exists():
                errors.append(f"{manifest}: {entry}: missing assembly {asm_path}")

            text = source.read_text(encoding="utf-8", errors="replace")
            if not function_pattern(function).search(text):
                errors.append(f"{manifest}: {entry}: function {function} not found")

        actual_sources = set(candidate_root.glob("**/*.c"))
        for source in sorted(actual_sources.difference(manifest_sources)):
            errors.append(f"{manifest}: unmanifested source {source}")

    for candidate_root in sorted(candidate_roots):
        for path in sorted(candidate_root.glob("**/*")):
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

    print(f"OK: {total_rows} natural-C candidate(s)")
    return 0


def candidate_keys(
    manifests: list[tuple[Path, Path, list[dict[str, str]]]]
) -> tuple[dict[tuple[str, int], dict[str, str]], list[str]]:
    keys: dict[tuple[str, int], dict[str, str]] = {}
    errors: list[str] = []
    for manifest, _candidate_root, rows in manifests:
        default_module = default_module_for_manifest(manifest)
        for row in rows:
            try:
                key = parse_entry_key(row["entry"].lower(), default_module)
            except ValueError as exc:
                errors.append(f"{manifest}: {exc}")
                continue
            if key in keys:
                errors.append(
                    f"{manifest}: {row['entry']}: duplicate candidate key "
                    f"{key[0]}:{key[1]:#08x}"
                )
                continue
            keys[key] = {
                "entry": row["entry"],
                "source": str((manifest.parent / row["source"]).relative_to(REPO_ROOT)),
                "asm_path": row["asm_path"],
                "function": row["function"],
                "status": row["status"],
            }
    return keys, errors


def coverage_report(
    manifests: list[tuple[Path, Path, list[dict[str, str]]]]
) -> int:
    rows = load_routine_index()
    candidates, errors = candidate_keys(manifests)
    indexed = {
        (row["module"], int(row["entry"], 16)): row
        for row in rows
    }

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    by_module: dict[str, dict[str, int]] = {}
    missing: list[dict[str, str]] = []
    for row in rows:
        module = row["module"]
        key = (module, int(row["entry"], 16))
        stats = by_module.setdefault(module, {"indexed": 0, "candidates": 0, "missing": 0})
        stats["indexed"] += 1
        if key in candidates:
            stats["candidates"] += 1
        else:
            stats["missing"] += 1
            missing.append(
                {
                    "module": module,
                    "entry": row["entry"],
                    "labels": row["labels"],
                    "boundary": row["boundary"],
                    "asm_path": row["asm_path"],
                }
            )

    unindexed = []
    for key, candidate in sorted(candidates.items()):
        if key not in indexed:
            module, entry = key
            unindexed.append(
                {
                    "module": module,
                    "entry": f"0x{entry:06x}",
                    **candidate,
                }
            )

    total_indexed = len(rows)
    total_candidates = total_indexed - len(missing)
    report = {
        "summary": {
            "indexed_routines": total_indexed,
            "candidate_routines": total_candidates,
            "missing_routines": len(missing),
            "coverage_ratio": round(total_candidates / total_indexed, 4)
            if total_indexed
            else 0.0,
        },
        "by_module": by_module,
        "missing": missing,
        "unindexed_candidates": unindexed,
    }
    print(json.dumps(report, indent=2))
    return 1 if unindexed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate candidate files")
    parser.add_argument("--coverage", action="store_true", help="emit candidate coverage JSON")
    args = parser.parse_args()

    manifests = load_manifests()

    if args.check:
        return check_candidates(manifests)
    if args.coverage:
        return coverage_report(manifests)

    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
