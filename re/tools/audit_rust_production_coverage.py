#!/usr/bin/env python3
"""Correlate production Rust coverage with the recovered-routine port ledger."""
from __future__ import annotations

import argparse
import csv
import json
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Iterable

ComponentWitnesses = dict[str, tuple[str, str]]


def load_lcov(text: str, workspace: Path) -> dict[str, list[tuple[str, int]]]:
    functions: dict[str, list[tuple[str, int]]] = defaultdict(list)
    source: str | None = None
    for line in text.splitlines():
        if line.startswith("SF:"):
            path = Path(line[3:])
            try:
                source = path.resolve().relative_to(workspace.resolve()).as_posix()
            except ValueError:
                source = None
        elif line.startswith("FNDA:") and source is not None:
            count_text, name = line[5:].split(",", 1)
            functions[source].append((name, int(count_text)))
        elif line == "end_of_record":
            source = None
    return functions


def symbol_matches(mangled_name: str, rust_symbol: str) -> bool:
    position = 0
    for component in rust_symbol.split("::"):
        position = mangled_name.find(component, position)
        if position < 0:
            return False
        position += len(component)
    return True


def matching_functions(
    functions: dict[str, list[tuple[str, int]]],
    rust_path: str,
    rust_symbol: str,
) -> list[tuple[str, int]]:
    return [
        (name, count)
        for name, count in functions.get(rust_path, [])
        if symbol_matches(name, rust_symbol)
    ]


def audit_rows(
    rows: Iterable[dict[str, str]],
    functions: dict[str, list[tuple[str, int]]],
    component_witnesses: ComponentWitnesses | None = None,
) -> list[dict[str, object]]:
    component_witnesses = component_witnesses or {}
    witness_counts = {
        component: sum(
            count
            for _name, count in matching_functions(functions, rust_path, rust_symbol)
        )
        for component, (rust_path, rust_symbol) in component_witnesses.items()
    }
    audited = []
    for row in rows:
        matching = matching_functions(functions, row["rust_path"], row["rust_symbol"])
        raw_execution_count = sum(count for _name, count in matching)
        witness_count = witness_counts.get(row["component"])
        audited.append(
            {
                "component": row["component"],
                "entry": row["entry"],
                "native_function": row["function"],
                "rust_path": row["rust_path"],
                "rust_symbol": row["rust_symbol"],
                "execution_count": (
                    raw_execution_count
                    if witness_count is None or witness_count != 0
                    else 0
                ),
                "raw_execution_count": raw_execution_count,
                "component_witness_execution_count": witness_count,
                "instrumented_instances": len(matching),
            }
        )
    return audited


def llvm_lcov(llvm_cov: str, binary: Path, profile: Path) -> str:
    command = [
        llvm_cov,
        "export",
        str(binary),
        f"--instr-profile={profile}",
        "--format=lcov",
        "--skip-branches",
        "--skip-expansions",
    ]
    return subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout


def read_ledger(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as source:
        return list(csv.DictReader(source, delimiter="\t"))


def read_expected_coverage(path: Path) -> set[tuple[str, str]]:
    with path.open(encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    keys = {(row["component"], row["entry"]) for row in rows}
    if len(keys) != len(rows):
        raise ValueError(f"{path}: duplicate component/entry coverage row")
    return keys


def read_component_witnesses(path: Path) -> ComponentWitnesses:
    with path.open(encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    witnesses = {
        row["component"]: (row["rust_path"], row["rust_symbol"]) for row in rows
    }
    if len(witnesses) != len(rows):
        raise ValueError(f"{path}: duplicate component witness row")
    return witnesses


def missing_expected_routines(
    routines: Iterable[dict[str, object]],
    expected: set[tuple[str, str]],
) -> list[dict[str, object]]:
    return [
        row
        for row in routines
        if (str(row["component"]), str(row["entry"])) in expected
        and row["execution_count"] == 0
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--ledger", type=Path, default=Path("re/rust-port/ported.tsv"))
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--llvm-cov", default="llvm-cov")
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--expected-covered", type=Path)
    parser.add_argument(
        "--component-witnesses",
        type=Path,
        default=Path("re/rust-port/production-component-witnesses.tsv"),
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--summary-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace = args.workspace.resolve()
    ledger = args.ledger if args.ledger.is_absolute() else workspace / args.ledger
    functions = load_lcov(
        llvm_lcov(args.llvm_cov, args.binary, args.profile),
        workspace,
    )
    ledger_rows = read_ledger(ledger)
    witness_path = (
        args.component_witnesses
        if args.component_witnesses.is_absolute()
        else workspace / args.component_witnesses
    )
    component_witnesses = read_component_witnesses(witness_path)
    ledger_components = {row["component"] for row in ledger_rows}
    unknown_witnesses = sorted(component_witnesses.keys() - ledger_components)
    if unknown_witnesses:
        raise ValueError(
            f"{witness_path}: witnesses reference unknown ledger components: "
            f"{unknown_witnesses}"
        )
    routines = audit_rows(ledger_rows, functions, component_witnesses)
    covered = [row for row in routines if row["execution_count"] != 0]
    covered_symbols = {
        (row["rust_path"], row["rust_symbol"])
        for row in covered
    }
    report = {
        "schema": 1,
        "scenario": args.scenario,
        "routine_count": len(routines),
        "covered_routine_count": len(covered),
        "covered_symbol_count": len(covered_symbols),
        "uncovered_routine_count": len(routines) - len(covered),
        "component_witnesses": [
            {
                "component": component,
                "rust_path": rust_path,
                "rust_symbol": rust_symbol,
                "execution_count": sum(
                    count
                    for _name, count in matching_functions(
                        functions, rust_path, rust_symbol
                    )
                ),
            }
            for component, (rust_path, rust_symbol) in sorted(
                component_witnesses.items()
            )
        ],
        "routines": routines,
    }
    missing_expected: list[dict[str, object]] = []
    if args.expected_covered:
        expected_path = (
            args.expected_covered
            if args.expected_covered.is_absolute()
            else workspace / args.expected_covered
        )
        expected = read_expected_coverage(expected_path)
        ledger_keys = {
            (str(row["component"]), str(row["entry"])) for row in routines
        }
        unknown_expected = sorted(expected - ledger_keys)
        if unknown_expected:
            raise ValueError(
                f"{expected_path}: expected coverage rows are absent from the port ledger: "
                f"{unknown_expected}"
            )
        missing_expected = missing_expected_routines(routines, expected)
        report.update(
            {
                "expected_covered_routine_count": len(expected),
                "missing_expected_routine_count": len(missing_expected),
                "missing_expected_routines": missing_expected,
            }
        )
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    if args.summary_only:
        summary = {key: value for key, value in report.items() if key != "routines"}
        print(json.dumps(summary, indent=2))
    else:
        print(rendered, end="")
    return 1 if missing_expected else 0


if __name__ == "__main__":
    raise SystemExit(main())
