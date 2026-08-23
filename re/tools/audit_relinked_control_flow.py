#!/usr/bin/env python3
"""Verify evidence-backed branch polarity in emitted recovered routines."""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "segment_contract_audit", ROOT / "re/tools/audit_segment_contracts.py"
)
SEGMENTS = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = SEGMENTS
SPEC.loader.exec_module(SEGMENTS)


def audit_scene_dispatch(listing) -> list[str]:
    instructions = listing.instructions
    call_index = next(
        (
            index
            for index, item in enumerate(instructions)
            if re.search(
                r"\bcall\s+.*\blist_d8c_state_le_one_\b",
                item.text,
                re.IGNORECASE,
            )
        ),
        None,
    )
    if call_index is None:
        return ["scene dispatch: missing list_d8c_state_le_one call"]
    window = [
        " ".join(item.text.lower().split())
        for item in instructions[call_index + 1 : call_index + 4]
    ]
    if len(window) < 2 or not re.match(r"^(?:test|or)\s+ax,ax\b", window[0]):
        return [
            "scene dispatch: list state result is not tested directly in AX"
        ]
    if not re.match(r"^j(?:ne|nz)\s+", window[1]):
        return [
            "scene dispatch: normalized true list-state result must skip teardown, "
            f"emitted {window[1]!r}"
        ]
    return []


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listing-dir", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    path = (
        args.listing_dir.resolve()
        / "func_009d10_dlg_line_id_scene_dispatch.lst"
    )
    listing = SEGMENTS.parse_listing(
        path, path.read_text(encoding="ascii", errors="replace")
    )
    errors = audit_scene_dispatch(listing)
    if errors:
        raise SystemExit("\n".join(errors))
    print("relinked control flow: scene list-state branch polarity verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
