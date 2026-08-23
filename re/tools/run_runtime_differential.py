#!/usr/bin/env python3
"""Run one action scenario against original and rebuilt BLOODPRG, then compare."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "compare_runtime_traces", ROOT / "re/tools/compare_runtime_traces.py"
)
COMPARE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARE
SPEC.loader.exec_module(COMPARE)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_side(
    name: str,
    runtime: Path,
    scenario: Path,
    state: Path,
    executable: str,
    c_root: Path,
    d_root: Path,
    output: Path,
    cpu_multiplier: int,
) -> Path:
    side_output = output / name
    trace = side_output / "semantic-trace.jsonl"
    side_output.mkdir(parents=True, exist_ok=True)
    c_root.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ)
    environment.update(
        {
            "VERIFYSCRIPT": str(scenario.resolve()),
            "VERIFYSTATE": str(state.resolve()),
            "VERIFYTRACE": str(trace.resolve()),
        }
    )
    command = [
        str(runtime.resolve()),
        "--c-root",
        str(c_root.resolve()),
        "--d-root",
        str(d_root.resolve()),
        "--executable",
        executable,
        "--out",
        str(side_output.resolve()),
        "--cpu-multiplier",
        str(cpu_multiplier),
    ]
    process = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    (side_output / "runtime.log").write_text(
        "$ " + " ".join(command) + "\n" + process.stdout + process.stderr,
        encoding="utf-8",
    )
    if process.returncode != 0:
        raise RuntimeError(
            f"{name} runtime exited {process.returncode}; see "
            f"{side_output / 'runtime.log'}"
        )
    if not trace.is_file():
        raise RuntimeError(f"{name} runtime did not produce {trace}")
    return trace


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", type=Path, required=True)
    parser.add_argument("--original-state", type=Path, required=True)
    parser.add_argument("--rebuilt-state", type=Path, required=True)
    parser.add_argument(
        "--runtime", type=Path, default=ROOT / "target/debug/runtime_boot"
    )
    parser.add_argument("--original-executable", default="BLOODPRG.EXE")
    parser.add_argument("--rebuilt-executable", default="BPRG_RE.EXE")
    parser.add_argument(
        "--original-d-root", type=Path, default=ROOT / "output/_tmp_iso"
    )
    parser.add_argument(
        "--rebuilt-d-root",
        type=Path,
        default=ROOT / "output/recovered_dos_package/cd",
    )
    parser.add_argument(
        "--output", type=Path, default=ROOT / "output/runtime-differential"
    )
    parser.add_argument("--original-cpu-multiplier", type=int, default=1)
    parser.add_argument("--rebuilt-cpu-multiplier", type=int, default=4)
    parser.add_argument("--allow-shared-state", action="store_true")
    parser.add_argument("--report-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    for path in (
        args.scenario,
        args.original_state,
        args.rebuilt_state,
        args.runtime,
    ):
        if not path.is_file():
            raise SystemExit(f"missing required file: {path}")
    if args.original_cpu_multiplier < 1:
        raise SystemExit("--original-cpu-multiplier must be positive")
    if args.rebuilt_cpu_multiplier < 1:
        raise SystemExit("--rebuilt-cpu-multiplier must be positive")
    original_state_hash = sha256(args.original_state)
    rebuilt_state_hash = sha256(args.rebuilt_state)
    if (
        original_state_hash == rebuilt_state_hash
        and not args.allow_shared_state
    ):
        raise SystemExit(
            "original and rebuilt savestates are byte-identical; savestates contain "
            "executable memory, so generate one state from each executable or pass "
            "--allow-shared-state only for a harness self-test"
        )

    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    original_trace = run_side(
        "original",
        args.runtime,
        args.scenario,
        args.original_state,
        args.original_executable,
        output / "c-original",
        args.original_d_root,
        output,
        args.original_cpu_multiplier,
    )
    rebuilt_trace = run_side(
        "rebuilt",
        args.runtime,
        args.scenario,
        args.rebuilt_state,
        args.rebuilt_executable,
        output / "c-rebuilt",
        args.rebuilt_d_root,
        output,
        args.rebuilt_cpu_multiplier,
    )
    report = COMPARE.compare_records(
        COMPARE.load_trace(original_trace),
        COMPARE.load_trace(rebuilt_trace),
    )
    report.update(
        {
            "scenario": str(args.scenario.resolve()),
            "original_state_sha256": original_state_hash,
            "rebuilt_state_sha256": rebuilt_state_hash,
            "original_trace": str(original_trace),
            "rebuilt_trace": str(rebuilt_trace),
        }
    )
    report_path = output / "comparison.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "equivalent" or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main())
