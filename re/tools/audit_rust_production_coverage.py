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


def audit_rows(
    rows: Iterable[dict[str, str]],
    functions: dict[str, list[tuple[str, int]]],
) -> list[dict[str, object]]:
    audited = []
    for row in rows:
        matching = [
            (name, count)
            for name, count in functions.get(row["rust_path"], [])
            if symbol_matches(name, row["rust_symbol"])
        ]
        audited.append(
            {
                "component": row["component"],
                "entry": row["entry"],
                "native_function": row["function"],
                "rust_path": row["rust_path"],
                "rust_symbol": row["rust_symbol"],
                "execution_count": sum(count for _name, count in matching),
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--ledger", type=Path, default=Path("re/rust-port/ported.tsv"))
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--llvm-cov", default="llvm-cov")
    parser.add_argument("--scenario", required=True)
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
    routines = audit_rows(read_ledger(ledger), functions)
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
        "routines": routines,
    }
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    if args.summary_only:
        summary = {key: value for key, value in report.items() if key != "routines"}
        print(json.dumps(summary, indent=2))
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
