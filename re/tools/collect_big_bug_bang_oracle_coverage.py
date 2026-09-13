#!/usr/bin/env python3
"""Run every BBB native oracle and aggregate executed entrypoint coverage."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "re/tools"
COLLECTOR = Path(__file__).resolve()
RUNNER = TOOLS / "run_big_bug_bang_oracle_with_coverage.py"
SEQUEL = REPO_ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DISC = SEQUEL.parent
GRAPH = REPO_ROOT / "re/big_bug_bang_expanded_func_graph.json"
COMPARISON = REPO_ROOT / "re/big_bug_bang_expanded_function_comparison.json"
SELECTION_FIXTURE = TOOLS / "oracle_vectors/big_bug_bang_inventory_selection.jsonl"
DYNAMIC_ENTRYPOINTS = {0xE0ED}
FIXTURE_STEM_OVERRIDES = {
    "big_bug_bang_travel_option": "big_bug_bang_travel_options",
    "big_bug_bang_vm": "big_bug_bang_multiply_divide",
}
FIXTURE_RELATION_OVERRIDES = {
    "big_bug_bang_inventory_descriptor_oracle": "prefix",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def oracle_arguments(oracle: Path, output: Path) -> list[str]:
    stem = oracle.stem
    if stem == "big_bug_bang_dialogue_guard_oracle":
        return [str(SEQUEL), str(DISC), str(output)]
    if stem == "big_bug_bang_honk_departure_oracle":
        return [str(DISC), str(output)]
    if stem == "big_bug_bang_inventory_condition_oracle":
        return [str(SEQUEL), str(output), str(SELECTION_FIXTURE)]
    return [str(SEQUEL), str(output)]


def oracle_fixture(oracle: Path) -> Path | None:
    stem = oracle.stem.removesuffix("_oracle")
    stem = FIXTURE_STEM_OVERRIDES.get(stem, stem)
    matches = sorted((TOOLS / "oracle_vectors").glob(f"{stem}.json*"))
    if not matches:
        return None
    if len(matches) != 1:
        raise RuntimeError(f"{oracle.name} has ambiguous fixtures: {matches}")
    return matches[0]


def run_oracle(oracle: Path, directory: Path) -> dict[str, Any]:
    coverage = directory / f"{oracle.stem}.coverage.json"
    output = directory / f"{oracle.stem}.fixture"
    command = [
        sys.executable,
        str(RUNNER),
        str(oracle),
        str(coverage),
        *oracle_arguments(oracle, output),
    ]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        transcript = (result.stdout + result.stderr)[-4_000:]
        raise RuntimeError(f"{oracle.name} failed:\n{transcript}")
    report = json.loads(coverage.read_text())
    output_lines = result.stdout.strip().splitlines()
    report["summary"] = output_lines[-1] if output_lines else ""
    fixture = oracle_fixture(oracle)
    if fixture is None:
        report["fixture"] = None
        report["fixture_sha256"] = None
        report["fixture_relation"] = None
    else:
        relation = FIXTURE_RELATION_OVERRIDES.get(oracle.stem, "exact")
        actual = output.read_bytes()
        expected = fixture.read_bytes()
        matches = (
            actual == expected if relation == "exact" else expected.startswith(actual)
        )
        if not matches:
            raise RuntimeError(
                f"{oracle.name} output does not satisfy {relation} relation with "
                f"{fixture.relative_to(REPO_ROOT)}"
            )
        report["fixture"] = str(fixture.relative_to(REPO_ROOT))
        report["fixture_sha256"] = sha256(fixture)
        report["fixture_relation"] = relation
    return report


def comparison_classes(report: dict[str, Any]) -> dict[int, dict[str, Any]]:
    rows: dict[int, dict[str, Any]] = {}
    for match in report["matches"]:
        sequel = int(match["sequel"]["entry"], 16)
        rows[sequel] = {
            "comparison": match["classification"],
            "commander_entry": match["commander"]["entry"],
        }
    for match in report["ambiguous"]:
        rows[int(match["sequel_entry"], 16)] = {
            "comparison": "ambiguous",
            "commander_candidates": match["commander_candidates"],
        }
    for match in report["unresolved_sequel"]:
        rows[int(match["entry"], 16)] = {"comparison": "unresolved"}
    return rows


def aggregate(reports: list[dict[str, Any]]) -> dict[str, Any]:
    graph = json.loads(GRAPH.read_text())
    static_entries = set(graph["funcs"])
    comparison = comparison_classes(json.loads(COMPARISON.read_text()))
    evidence: dict[int, list[str]] = {}
    for report in reports:
        for encoded in report["entered_entrypoints"]:
            evidence.setdefault(int(encoded, 16), []).append(report["oracle"])

    rows = []
    for entry in sorted(static_entries | DYNAMIC_ENTRYPOINTS):
        row = {
            "entry": f"0x{entry:04x}",
            "origin": "static_graph" if entry in static_entries else "runtime_vector",
            "entered": entry in evidence,
            "oracles": sorted(evidence.get(entry, [])),
        }
        if entry in comparison:
            row.update(comparison[entry])
        else:
            row["comparison"] = "outside_static_comparison"
        rows.append(row)

    entered_static = static_entries.intersection(evidence)
    entered_dynamic = DYNAMIC_ENTRYPOINTS.intersection(evidence)
    class_counts: dict[str, dict[str, int]] = {}
    for row in rows:
        key = row["comparison"]
        counts = class_counts.setdefault(key, {"entered": 0, "unentered": 0})
        counts["entered" if row["entered"] else "unentered"] += 1
    return {
        "format": "big_bug_bang_oracle_entrypoint_coverage_v3",
        "scope": (
            "Measured original-executable instruction entrypoints reached by the "
            "checked-in BBB oracle scenarios; entries count only direct starts and "
            "non-return transfers from original code, while absence is an audit "
            "queue and not proof of missing behavior"
        ),
        "inputs": {
            "sequel": {
                "path": str(SEQUEL.relative_to(REPO_ROOT)),
                "sha256": sha256(SEQUEL),
            },
            "static_graph": {
                "path": str(GRAPH.relative_to(REPO_ROOT)),
                "sha256": sha256(GRAPH),
            },
            "comparison": {
                "path": str(COMPARISON.relative_to(REPO_ROOT)),
                "sha256": sha256(COMPARISON),
            },
            "runner": {
                "path": str(RUNNER.relative_to(REPO_ROOT)),
                "sha256": sha256(RUNNER),
            },
            "collector": {
                "path": str(COLLECTOR.relative_to(REPO_ROOT)),
                "sha256": sha256(COLLECTOR),
            },
        },
        "summary": {
            "oracle_count": len(reports),
            "oracle_fixture_count": sum(
                report["fixture"] is not None for report in reports
            ),
            "fixture_relation_counts": {
                relation: sum(
                    report["fixture_relation"] == relation for report in reports
                )
                for relation in ("exact", "prefix")
            },
            "static_entrypoint_count": len(static_entries),
            "dynamic_entrypoint_count": len(DYNAMIC_ENTRYPOINTS),
            "known_entrypoint_count": len(static_entries | DYNAMIC_ENTRYPOINTS),
            "entered_static_entrypoints": len(entered_static),
            "unentered_static_entrypoints": len(static_entries - entered_static),
            "entered_dynamic_entrypoints": len(entered_dynamic),
            "entered_known_entrypoints": len(entered_static | entered_dynamic),
            "comparison_counts": class_counts,
        },
        "oracles": sorted(reports, key=lambda report: report["oracle"]),
        "entrypoints": rows,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--jobs", type=int, default=6)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    oracles = sorted(TOOLS.glob("big_bug_bang_*_oracle.py"))
    with tempfile.TemporaryDirectory(prefix="bbb-oracle-coverage-") as temporary:
        directory = Path(temporary)
        reports = []
        with ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = {
                executor.submit(run_oracle, oracle, directory): oracle
                for oracle in oracles
            }
            for future in as_completed(futures):
                reports.append(future.result())

    result = aggregate(reports)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    summary = result["summary"]
    print(
        f"verified {summary['oracle_count']} BBB oracle programs; "
        f"entered {summary['entered_static_entrypoints']}/"
        f"{summary['static_entrypoint_count']} static entrypoints and "
        f"{summary['entered_dynamic_entrypoints']}/"
        f"{summary['dynamic_entrypoint_count']} known dynamic entrypoints"
    )


if __name__ == "__main__":
    main()
