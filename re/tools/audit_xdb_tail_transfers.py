#!/usr/bin/env python3
"""Reject source-linked alien XDB dispatchers that mutate SP before tail jumps."""

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
from dataclasses import dataclass
import io
import re


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_XDB_ROOT = ROOT / "output" / "recovered_dos_package" / "validation" / "source_xdb"

SLOT2_PREFIX = bytes.fromhex("8b751683c65ef74536ffff7403ff640e")
SLOT13_PREFIX = bytes.fromhex("8b5d360bdb7402ffe3")

MODULE_SYMBOLS = {
    "amer": (
        ("xdb_amer_method_slot_2_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_amer_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
    "croolis": (
        ("xdb_croolis_method_slot_2_4_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_croolis_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
    "scrut": (
        ("xdb_scrut_method_slot_2_4_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_scrut_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
}

MAP_SYMBOL = re.compile(
    r"^([0-9A-Fa-f]{4}):([0-9A-Fa-f]{4})\s+([A-Za-z_$?][\w$?@]*)\s*$",
    re.MULTILINE,
)


@dataclass(frozen=True)
class Result:
    module: str
    symbol: str
    offset: int | None
    expected: bytes
    actual: bytes
    status: str


def read_map_symbols(path: Path) -> dict[str, list[tuple[int, int]]]:
    symbols: dict[str, list[tuple[int, int]]] = {}
    text = path.read_text(encoding="ascii", errors="replace")
    for segment, offset, symbol in MAP_SYMBOL.findall(text):
        symbols.setdefault(symbol, []).append((int(segment, 16), int(offset, 16)))
    return symbols


def audit_module(root: Path, module: str) -> tuple[list[Result], list[str]]:
    module_dir = root / module
    image_path = module_dir / f"{module}.xdb"
    map_path = module_dir / f"{module}_source_link.map"
    errors: list[str] = []
    results: list[Result] = []

    if not image_path.is_file():
        return [], [f"{module}: missing linked image {image_path}"]
    if not map_path.is_file():
        return [], [f"{module}: missing linked map {map_path}"]

    image = image_path.read_bytes()
    symbols = read_map_symbols(map_path)
    for symbol, expected in MODULE_SYMBOLS[module]:
        locations = symbols.get(symbol, [])
        if len(locations) != 1:
            errors.append(
                f"{module}: {symbol} has {len(locations)} map locations; expected one"
            )
            results.append(Result(module, symbol, None, expected, b"", "missing_symbol"))
            continue
        segment, offset = locations[0]
        if segment != 0:
            errors.append(
                f"{module}: {symbol} is in segment 0x{segment:04x}; expected raw segment zero"
            )
            results.append(Result(module, symbol, offset, expected, b"", "wrong_segment"))
            continue
        actual = image[offset : offset + len(expected)]
        status = "exact_tail_prefix" if actual == expected else "prefix_mismatch"
        results.append(Result(module, symbol, offset, expected, actual, status))
        if actual != expected:
            errors.append(
                f"{module}: {symbol} at 0x{offset:04x} changes the callback tail "
                f"contract; expected {expected.hex()}, got {actual.hex()}"
            )
    return results, errors


def render_tsv(results: list[Result]) -> str:
    output = io.StringIO()
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow(("module", "symbol", "offset", "expected_prefix", "actual_prefix", "status"))
    for result in results:
        writer.writerow(
            (
                result.module,
                result.symbol,
                "" if result.offset is None else f"0x{result.offset:04x}",
                result.expected.hex(),
                result.actual.hex(),
                result.status,
            )
        )
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="run the audit")
    parser.add_argument(
        "--source-xdb-root",
        type=Path,
        default=DEFAULT_SOURCE_XDB_ROOT,
        help="directory containing per-module linked images and maps",
    )
    parser.add_argument("--output", type=Path, help="write a deterministic TSV report")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.check:
        raise SystemExit("--check is required")

    results: list[Result] = []
    errors: list[str] = []
    for module in MODULE_SYMBOLS:
        module_results, module_errors = audit_module(args.source_xdb_root.resolve(), module)
        results.extend(module_results)
        errors.extend(module_errors)

    report = render_tsv(results)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"OK: {len(results)} linked callback tail-transfer prefix(es)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
