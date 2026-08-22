#!/usr/bin/env python3
"""Require byte-only framebuffer transfers in VGA write mode 1."""
from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]
AUDIT_PATH = ROOT / "re/tools/audit_segment_contracts.py"
SPEC = importlib.util.spec_from_file_location("segment_contract_audit", AUDIT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {AUDIT_PATH}")
audit = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = audit
SPEC.loader.exec_module(audit)


def based_es_access_widths(listing: audit.Listing) -> list[tuple[int, int, str]]:
    result = []
    for item in listing.instructions:
        instruction = audit.decode_instruction(item)
        for operand in instruction.operands:
            if operand.type != x86_const.X86_OP_MEM or not operand.mem.base:
                continue
            segment = instruction.reg_name(operand.mem.segment).lower() \
                if operand.mem.segment else ""
            base = instruction.reg_name(operand.mem.base).lower()
            if segment == "es" and base not in ("bp", "sp", "ebp", "esp"):
                result.append((item.offset, operand.size, item.text))
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--object", type=Path, required=True)
    parser.add_argument("--wdis", type=Path, default=Path("wdis"))
    parser.add_argument("--listing-cache", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    args.listing_cache.mkdir(parents=True, exist_ok=True)
    listing = audit.listing_for_object(
        args.wdis, args.object.resolve(), args.listing_cache.resolve()
    )
    accesses = based_es_access_widths(listing)
    wide = [access for access in accesses if access[1] != 1]
    if wide:
        for offset, width, text in wide:
            print(f"wide VGA access at {offset:04x}: width={width}: {text}")
        return 1
    if len(accesses) < 4:
        print(f"expected four VGA byte access sites, found {len(accesses)}")
        return 1
    print(f"{len(accesses)} based ES framebuffer accesses; all byte-wide")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
