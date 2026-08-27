#!/usr/bin/env python3
"""Audit whether translated native routines reach the modern game executable.

The translation ledger proves that every recovered routine has documented Rust
source and oracle evidence.  It does not prove that production code calls that
source.  This audit checks the demangled symbols retained by a debug
``commander-blood`` executable, where rustc performs little incidental inlining
and the linker removes unreferenced function sections.

One Rust implementation can intentionally represent several sibling XDB
routines.  Each corresponding ledger row therefore checks the same retained
symbol.  Missing rows are findings, not automatic proof of a defect: a small
number can be forced inline or represented through a trait shim.  Every missing
row still requires an explicit source-level explanation before release.
"""

from __future__ import annotations

import argparse
import csv
import pathlib
import re
import subprocess
import sys
from dataclasses import dataclass


WORKSPACE_ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_LEDGER = WORKSPACE_ROOT / "re/rust-port/ported.tsv"
DEFAULT_BINARY = WORKSPACE_ROOT / "target/debug/commander-blood"
CRATES_PREFIX = pathlib.PurePosixPath("crates")


@dataclass(frozen=True)
class PortedRoutine:
    component: str
    entry: str
    source_path: pathlib.PurePosixPath
    rust_symbol: str

    @property
    def qualified_symbol(self) -> str:
        module = source_module(self.source_path)
        return f"{module}::{self.rust_symbol}"

    @property
    def function_name(self) -> str:
        return self.rust_symbol.rsplit("::", 1)[-1]


def source_module(source_path: pathlib.PurePosixPath) -> str:
    """Convert one workspace-crate source path to its demangled module prefix."""

    try:
        crate_relative = source_path.relative_to(CRATES_PREFIX)
    except ValueError as error:
        raise ValueError(f"Rust source is outside the workspace crates: {source_path}") from error

    crate_name, source_directory, *relative_parts = crate_relative.parts
    if source_directory != "src" or not relative_parts:
        raise ValueError(f"Rust source is outside a crate src directory: {source_path}")

    relative = pathlib.PurePosixPath(*relative_parts)
    parts = list(relative.with_suffix("").parts)
    if parts[-1] in {"lib", "main", "mod"}:
        parts.pop()
    return "::".join([crate_name.replace("-", "_"), *parts])


def read_ledger(path: pathlib.Path) -> list[PortedRoutine]:
    with path.open(encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    return [
        PortedRoutine(
            component=row["component"],
            entry=row["entry"],
            source_path=pathlib.PurePosixPath(row["rust_path"]),
            rust_symbol=row["rust_symbol"],
        )
        for row in rows
    ]


def demangled_symbols(binary: pathlib.Path) -> set[str]:
    completed = subprocess.run(
        ["nm", "-C", str(binary)],
        check=True,
        capture_output=True,
        text=True,
    )
    symbols: set[str] = set()
    for line in completed.stdout.splitlines():
        fields = line.split(maxsplit=2)
        if len(fields) == 3:
            symbols.add(fields[2])
    return symbols


def retained(qualified_symbol: str, symbols: set[str]) -> bool:
    """Recognize direct symbols and rustc's generic-instance suffixes."""

    direct = re.compile(rf"^{re.escape(qualified_symbol)}(?:$|::h[0-9a-f]+$|<)")
    if any(direct.search(symbol) for symbol in symbols):
        return True

    module, _, declared = qualified_symbol.rpartition("::")
    if "::" not in declared:
        method_form = re.compile(
            rf"^{re.escape(module)}::[^<]+::{re.escape(declared)}"
            rf"(?:$|::h[0-9a-f]+$|<)"
        )
        if any(method_form.search(symbol) for symbol in symbols):
            return True

    # Trait implementations demangle as ``<Type as Trait>::method`` rather than
    # ``Type::method``.  Keep the match anchored to the exact source module and
    # type so a same-named method in another module cannot hide an unwired row.
    if "::" not in qualified_symbol:
        return False
    owner, method = qualified_symbol.rsplit("::", 1)
    module, separator, type_name = owner.rpartition("::")
    if not separator or not type_name[:1].isupper():
        return False
    trait_form = re.compile(
        rf"^<{re.escape(module)}::{re.escape(type_name)} as [^>]+>::"
        rf"{re.escape(method)}(?:$|::h[0-9a-f]+$|<)"
    )
    return any(trait_form.search(symbol) for symbol in symbols)


def item_end(text: str, start: int) -> int:
    """Return the end of one brace item or semicolon statement."""

    brace = text.find("{", start)
    semicolon = text.find(";", start)
    if semicolon >= 0 and (brace < 0 or semicolon < brace):
        return semicolon + 1
    if brace < 0:
        return len(text)
    depth = 0
    for index in range(brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    return len(text)


def production_text(text: str) -> str:
    """Remove cfg-test items and public re-export statements."""

    while True:
        match = re.search(r"(?m)^\s*#\[cfg\(test\)\]", text)
        if match is None:
            break
        end = item_end(text, match.end())
        text = text[: match.start()] + " " * (end - match.start()) + text[end:]

    while True:
        match = re.search(r"(?m)^\s*pub\s+use\b", text)
        if match is None:
            break
        end = text.find(";", match.end())
        end = len(text) if end < 0 else end + 1
        text = text[: match.start()] + " " * (end - match.start()) + text[end:]
    return text


def without_function_body(text: str, name: str) -> str:
    """Remove each definition body so recursion cannot count as a caller."""

    pattern = re.compile(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+{re.escape(name)}\b")
    while True:
        match = pattern.search(text)
        if match is None:
            return text
        end = item_end(text, match.end())
        text = text[: match.start()] + " " * (end - match.start()) + text[end:]


def load_production_sources(root: pathlib.Path) -> dict[pathlib.PurePosixPath, str]:
    sources: dict[pathlib.PurePosixPath, str] = {}
    for crate in ("commander-blood-formats", "commander-blood-game"):
        source_root = root / "crates" / crate / "src"
        for path in source_root.rglob("*.rs"):
            relative = pathlib.PurePosixPath(path.relative_to(root).as_posix())
            sources[relative] = production_text(path.read_text(encoding="utf-8"))
    return sources


def source_routed(routine: PortedRoutine, sources: dict[pathlib.PurePosixPath, str]) -> bool:
    """Find a non-test reference outside every same-named function body."""

    name = routine.function_name
    definition = re.compile(
        rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+{re.escape(name)}\b"
    )
    definition_count = sum(len(definition.findall(text)) for text in sources.values())
    if definition_count != 1:
        return False
    reference = re.compile(rf"\b{re.escape(name)}\b")
    return any(reference.search(without_function_body(text, name)) for text in sources.values())


def parse_args(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=pathlib.Path, default=DEFAULT_LEDGER)
    parser.add_argument("--binary", type=pathlib.Path, default=DEFAULT_BINARY)
    parser.add_argument(
        "--strict",
        action="store_true",
        help="return failure when any translated routine lacks a retained symbol",
    )
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    options = parse_args(sys.argv[1:] if arguments is None else arguments)
    routines = read_ledger(options.ledger)
    symbols = demangled_symbols(options.binary)
    sources = load_production_sources(WORKSPACE_ROOT)
    binary_retained = [routine for routine in routines if retained(routine.qualified_symbol, symbols)]
    source_only = [
        routine
        for routine in routines
        if routine not in binary_retained and source_routed(routine, sources)
    ]
    missing = [
        routine
        for routine in routines
        if routine not in binary_retained and routine not in source_only
    ]

    for routine in missing:
        print(
            "UNROUTED-RUST "
            f"{routine.component}:{routine.entry} "
            f"{routine.qualified_symbol} "
            f"({routine.source_path})"
        )
    retained_count = len(binary_retained)
    print(
        f"{retained_count}/{len(routines)} translated routine rows retain a production "
        f"symbol; {len(source_only)} have a non-test source caller after inlining; "
        f"{len(missing)} have no production routing evidence"
    )
    return int(options.strict and bool(missing))


if __name__ == "__main__":
    raise SystemExit(main())
