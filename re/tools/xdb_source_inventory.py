#!/usr/bin/env python3
"""Validate XDB C candidates, indexed owners, and reviewed internal labels."""

from __future__ import annotations

import os
from pathlib import Path
import sys

_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import argparse
import csv
import re


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
CANDIDATE_ROOT = MANIFEST.parent
ASSEMBLY_ROOT = ROOT / "re" / "assembly" / "xdb"
BOUNDARY_OVERRIDES = ROOT / "re" / "assembly" / "boundary_overrides.tsv"
ROUTINE_INDEX = ROOT / "re" / "assembly" / "routine_index.tsv"
MODULES = ("xdb_amer", "xdb_croolis", "xdb_manu3", "xdb_scrut")
ASM_ADDRESS_RE = re.compile(r"^\s*([0-9a-fA-F]{1,8}):", re.MULTILINE)
ASM_FILENAME_RE = re.compile(r"^func_([0-9a-fA-F]{6})_")


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def duplicate_values(rows: list[dict[str, str]], field: str) -> set[str]:
    seen: set[str] = set()
    duplicates: set[str] = set()
    for row in rows:
        value = row[field]
        if value in seen:
            duplicates.add(value)
        seen.add(value)
    return duplicates


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate the inventory")
    args = parser.parse_args()
    if not args.check:
        parser.print_help()
        return 0

    rows = read_tsv(MANIFEST)
    overrides = read_tsv(BOUNDARY_OVERRIDES)
    routine_rows = [
        row
        for row in read_tsv(ROUTINE_INDEX)
        if row["module"] in MODULES
    ]
    merged = {
        (row["module"], int(row["entry"], 16))
        for row in overrides
        if row["disposition"] == "merged_into_owner"
    }
    errors: list[str] = []

    routine_by_entry: dict[str, dict[str, str]] = {}
    for row in routine_rows:
        key = f"{row['module']}:{row['entry']}"
        if key in routine_by_entry:
            errors.append(f"duplicate routine-index entry: {key}")
        routine_by_entry[key] = row

    for field in ("entry", "source", "asm_path", "function"):
        for value in sorted(duplicate_values(rows, field)):
            errors.append(f"duplicate {field}: {value}")

    manifest_sources: set[Path] = set()
    manifest_assembly: set[Path] = set()
    pending_index: list[str] = []
    counts = {module: 0 for module in MODULES}
    for row in rows:
        try:
            module, entry_text = row["entry"].split(":", 1)
            entry = int(entry_text, 16)
        except ValueError:
            errors.append(f"invalid entry: {row['entry']}")
            continue
        if module not in counts:
            errors.append(f"unknown module: {module}")
            continue
        counts[module] += 1
        routine_row = routine_by_entry.get(row["entry"])
        if routine_row is not None and routine_row["asm_path"] != row["asm_path"]:
            errors.append(
                f"{row['entry']}: routine-index assembly path "
                f"{routine_row['asm_path']} differs from manifest {row['asm_path']}"
            )
        if (module, entry) in merged:
            errors.append(f"{row['entry']}: reviewed internal label has a C candidate")

        source = CANDIDATE_ROOT / row["source"]
        assembly = ROOT / row["asm_path"]
        manifest_sources.add(source)
        manifest_assembly.add(assembly)
        if not source.is_file():
            errors.append(f"{row['entry']}: missing source {row['source']}")
        else:
            source_text = source.read_text(encoding="utf-8", errors="replace")
            function_re = re.compile(rf"\b{re.escape(row['function'])}\s*\(")
            if function_re.search(source_text) is None:
                errors.append(
                    f"{row['entry']}: {row['function']} absent from {row['source']}"
                )
        if not assembly.is_file():
            errors.append(f"{row['entry']}: missing assembly {row['asm_path']}")
        else:
            assembly_text = assembly.read_text(encoding="utf-8", errors="replace")
            if routine_row is None:
                if "; routine_bytes_sha256: " in assembly_text:
                    errors.append(
                        f"{row['entry']}: standardized owner absent from "
                        "assembly routine index"
                    )
                else:
                    pending_index.append(row["entry"])
            addresses = {int(value, 16) for value in ASM_ADDRESS_RE.findall(assembly_text)}
            if entry not in addresses:
                errors.append(
                    f"{row['entry']}: entry instruction absent from {row['asm_path']}"
                )

    source_files = set(CANDIDATE_ROOT.glob("*/*.c"))
    for path in sorted(source_files - manifest_sources):
        errors.append(f"unmanifested C source: {path.relative_to(ROOT)}")
    for path in sorted(manifest_sources - source_files):
        errors.append(f"manifest source outside candidate set: {path.relative_to(ROOT)}")

    assembly_files = set(ASSEMBLY_ROOT.rglob("*.asm"))
    internal_artifacts = assembly_files - manifest_assembly
    for path in sorted(internal_artifacts):
        match = ASM_FILENAME_RE.match(path.name)
        module = f"xdb_{path.relative_to(ASSEMBLY_ROOT).parts[0]}"
        if match is None or (module, int(match.group(1), 16)) not in merged:
            errors.append(f"unclassified assembly artifact: {path.relative_to(ROOT)}")

    manifest_entries = {row["entry"] for row in rows}
    for entry in sorted(set(routine_by_entry) - manifest_entries):
        errors.append(f"routine-index XDB owner absent from C manifest: {entry}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(f"OK: {len(rows)} manifested XDB C candidate(s)")
    for module in MODULES:
        print(f"OK: {module}: {counts[module]} C candidate(s)")
    print(f"OK: {len(routine_rows)} standardized XDB assembly owner(s) indexed")
    print(
        f"OK: {len(pending_index)} legacy candidate boundary dump(s) remain "
        "pending standardized split audit"
    )
    print(f"OK: {len(internal_artifacts)} reviewed internal-label artifact(s) excluded")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
