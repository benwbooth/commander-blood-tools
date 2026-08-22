#!/usr/bin/env python3
"""Verify GAME_DATA and FS_DATA placement in the relinked DOS runtime."""
from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
INCLUDE_DIR = ROOT / "re/source/bloodprg/candidates/include"

SEGMENT_ROW = re.compile(
    r"^(?P<name>GAME_DATA|FS_DATA)\s+\S+\s+\S+\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})\s+"
    r"(?P<size>[0-9A-Fa-f]{8})$"
)
PUBLIC_ROW = re.compile(
    r"^(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})"
    r"(?:[*+]?)\s+(?P<symbol>\S+)$"
)
COMMENT_ADDRESS = re.compile(
    r"/\*\s*(?P<register>DS|GS|FS):0x(?P<address>[0-9A-Fa-f]+)\s*\*/"
)
NAME_BEFORE_SEMICOLON = re.compile(
    r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^]]*\])?\s*;"
)


@dataclass(frozen=True)
class SegmentPlacement:
    paragraph: int
    offset: int
    size: int


@dataclass(frozen=True)
class DocumentedSymbol:
    segment: str
    offset: int


def documented_name(line: str) -> tuple[str, DocumentedSymbol] | None:
    address_match = COMMENT_ADDRESS.search(line)
    if not address_match:
        return None
    code = line[: address_match.start()]
    if code.lstrip().startswith("#") or not code.strip():
        return None
    names = NAME_BEFORE_SEMICOLON.findall(code)
    if not names:
        return None
    register = address_match["register"]
    segment = "FS_DATA" if register == "FS" else "GAME_DATA"
    return (
        names[-1],
        DocumentedSymbol(segment, int(address_match["address"], 16)),
    )


def load_link_map(
    path: Path,
) -> tuple[dict[str, SegmentPlacement], dict[str, tuple[str, int]]]:
    placements: dict[str, SegmentPlacement] = {}
    raw_publics: list[tuple[int, int, str]] = []
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        stripped = line.strip()
        segment_match = SEGMENT_ROW.match(stripped)
        if segment_match:
            placements[segment_match["name"]] = SegmentPlacement(
                int(segment_match["segment"], 16),
                int(segment_match["offset"], 16),
                int(segment_match["size"], 16),
            )
            continue
        public_match = PUBLIC_ROW.match(stripped)
        if public_match:
            raw_publics.append(
                (
                    int(public_match["segment"], 16),
                    int(public_match["offset"], 16),
                    public_match["symbol"].lstrip("_"),
                )
            )
    missing_segments = {"GAME_DATA", "FS_DATA"} - placements.keys()
    if missing_segments:
        raise SystemExit(
            f"{path}: missing segment(s): {', '.join(sorted(missing_segments))}"
        )

    publics: dict[str, tuple[str, int]] = {}
    for paragraph, offset, symbol in raw_publics:
        for name, placement in placements.items():
            if paragraph != placement.paragraph:
                continue
            relative = offset - placement.offset
            if 0 <= relative < placement.size:
                publics[symbol] = (name, relative)
                break
    return placements, publics


def load_documented_addresses() -> dict[str, DocumentedSymbol]:
    documented: dict[str, DocumentedSymbol] = {}
    for header in sorted(INCLUDE_DIR.glob("*.h")):
        for line in header.read_text(
            encoding="ascii", errors="replace"
        ).splitlines():
            match = documented_name(line)
            if not match:
                continue
            name, location = match
            previous = documented.get(name)
            if previous is not None and previous != location:
                raise SystemExit(
                    f"conflicting placement for {name}: {previous} vs "
                    f"{location} in {header.name}"
                )
            documented[name] = location
    return documented


def linked_location(
    name: str, publics: dict[str, tuple[str, int]]
) -> tuple[str, int] | None:
    candidates = [name, name + "_gs"]
    if name.endswith("_gs"):
        candidates.append(name[:-3])
    if name.startswith("vm_ship_3d_"):
        candidates.append("ship_3d_" + name[len("vm_ship_3d_") :])
    for candidate in candidates:
        if candidate in publics:
            return publics[candidate]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--link-map",
        type=Path,
        default=ROOT
        / "output/recovered_dos_package/validation/bloodprg_runtime/final/link.map",
    )
    args = parser.parse_args()

    placements, linked = load_link_map(args.link_map.resolve())
    documented = load_documented_addresses()

    errors: list[str] = []
    for name, placement in placements.items():
        if placement.offset != 0:
            errors.append(
                f"{name} begins at {placement.paragraph:04x}:"
                f"{placement.offset:04x}; required offset is 0000"
            )

    checked = 0
    missing: list[tuple[str, DocumentedSymbol]] = []
    misplaced: list[
        tuple[str, DocumentedSymbol, tuple[str, int]]
    ] = []
    for name, expected in sorted(documented.items()):
        actual = linked_location(name, linked)
        if actual is None:
            missing.append((name, expected))
            continue
        checked += 1
        if actual != (expected.segment, expected.offset):
            misplaced.append((name, expected, actual))

    print(
        f"{checked} documented symbols verified; {len(missing)} absent; "
        f"{len(misplaced)} misplaced; {len(errors)} segment-base errors"
    )
    for error in errors:
        print(f"  SEGMENT {error}")
    for name, expected, actual in misplaced:
        print(
            f"  MISPLACED {name}: expected {expected.segment}:"
            f"{expected.offset:#06x}, linked at {actual[0]}:{actual[1]:#06x}"
        )
    for name, expected in missing[:20]:
        print(
            f"  ABSENT {name} (expected {expected.segment}:"
            f"{expected.offset:#06x})"
        )
    if len(missing) > 20:
        print(f"  ... and {len(missing) - 20} more absent symbols")
    return 1 if errors or misplaced or missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
