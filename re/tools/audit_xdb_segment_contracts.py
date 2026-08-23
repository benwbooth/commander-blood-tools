#!/usr/bin/env python3
"""Prove symbolic segment ownership in every source-linked XDB C routine."""
from __future__ import annotations

import argparse
import csv
import importlib.util
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "xdb_segment_contract_core", ROOT / "re/tools/audit_segment_contracts.py"
)
CORE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CORE
SPEC.loader.exec_module(CORE)

MODULES = ("amer", "croolis", "manu3", "scrut")
ALIEN_MODULES = frozenset(("amer", "croolis", "scrut"))


def initial_segments(module: str, stem: str) -> dict[str, str]:
    segments = {
        "cs": "CODE",
        "ds": "XDB_DATA",
        "es": CORE.UNKNOWN,
        "fs": "XDB_DATA",
        "gs": CORE.UNKNOWN,
        "ss": "STACK",
    }
    if module in ALIEN_MODULES and stem in (
        "func_000000_api_entry",
        "func_0000a3_main",
    ):
        segments["ds"] = CORE.UNKNOWN
        segments["fs"] = CORE.UNKNOWN
    return segments


def module_findings(
    module: str,
    source_xdb_root: Path,
    object_root: Path,
    wdis: Path,
) -> tuple[list[object], int, int]:
    build_dir = source_xdb_root / module
    owners = CORE.read_owners(build_dir / "segment_owners.tsv")
    linked = CORE.linked_project_stems(build_dir / f"{module}_source_link.map")
    object_dir = object_root / f"xdb_{module}"
    objects = {
        path.stem.lower(): path
        for path in object_dir.glob("*.[Oo][Bb][Jj]")
    }
    missing = sorted(linked - objects.keys())
    if missing:
        raise ValueError(
            f"{module}: linked recovered routines have no object: "
            + ", ".join(missing)
        )
    extra = sorted(objects.keys() - linked)
    if extra:
        raise ValueError(
            f"{module}: unlinked recovered routine objects: " + ", ".join(extra)
        )

    cache_dir = build_dir / "segment_contract_listings"
    cache_dir.mkdir(parents=True, exist_ok=True)
    findings: list[object] = []
    reached = 0
    for stem in sorted(linked):
        listing = CORE.listing_for_object(wdis, objects[stem], cache_dir)
        routine_findings, routine_reached = CORE.analyze_listing(
            listing,
            owners,
            initial_segments(module, stem),
            {f"_xdb_{module}_data_segment": "XDB_DATA"},
        )
        findings.extend(routine_findings)
        reached += routine_reached
    return findings, reached, len(linked)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source-xdb-root",
        type=Path,
        default=ROOT / "output/recovered_dos_package/validation/source_xdb",
    )
    parser.add_argument(
        "--object-root",
        type=Path,
        default=ROOT / "output/recovered_dos_package/xdb_objects",
    )
    parser.add_argument("--module", action="append", choices=MODULES)
    parser.add_argument("--wdis", type=Path, default=Path("wdis"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fail-unproven", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source_xdb_root = args.source_xdb_root.resolve()
    object_root = args.object_root.resolve()
    modules = tuple(args.module or MODULES)
    all_findings: list[tuple[str, object]] = []
    total_reached = 0
    total_routines = 0
    for module in modules:
        findings, reached, routines = module_findings(
            module,
            source_xdb_root,
            object_root,
            args.wdis,
        )
        all_findings.extend((module, finding) for finding in findings)
        total_reached += reached
        total_routines += routines

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
    stream = (
        args.output.open("w", newline="", encoding="ascii")
        if args.output else sys.stdout
    )
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "module",
        "routine",
        "object_offset",
        "status",
        "symbol",
        "expected_owner",
        "effective_segment",
        "proven_owner",
        "instruction",
    ))
    for module, finding in sorted(
        all_findings,
        key=lambda item: (
            item[1].status != "mismatch",
            item[1].status != "unproven",
            item[0],
            item[1].routine,
            item[1].offset,
            item[1].symbol,
        ),
    ):
        writer.writerow((
            module,
            finding.routine,
            f"0x{finding.offset:04x}",
            finding.status,
            finding.symbol,
            finding.expected_owner,
            finding.effective_segment,
            finding.proven_owner,
            finding.text,
        ))
    if args.output:
        stream.close()

    mismatches = sum(
        finding.status == "mismatch" for _module, finding in all_findings
    )
    unproven = sum(
        finding.status == "unproven" for _module, finding in all_findings
    )
    proven = len(all_findings) - mismatches - unproven
    print(
        f"{len(modules)} XDB modules; {total_routines} linked C routines; "
        f"{total_reached} reachable object instructions; "
        f"{len(all_findings)} symbolic data accesses; {proven} proven; "
        f"{unproven} unproven; {mismatches} mismatches"
    )
    if mismatches or (args.fail_unproven and unproven):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
