#!/usr/bin/env python3
"""Audit BBB compiler-library helpers with no recovered reference path."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
GRAPH = ROOT / "re/big_bug_bang_expanded_func_graph.json"
DEFAULT_OUTPUT = ROOT / "re/big_bug_bang_unreferenced_library_audit.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
GRAPH_SHA256 = "15bb9d05325c8da221d2b3f2dcf33b33643223ea47bdf4367bf40ae51884a02b"
FAR_RETURN_OPCODE = 0xCB

ROUTINES = (
    {
        "entry": 0x28B2,
        "end": 0x28CC,
        "name": "format_byte_as_binary",
        "body_sha256": "f3408376215a164d58dac0837709312286acb30999efe0d120600f35dad605e0",
    },
    {
        "entry": 0x28CC,
        "end": 0x28E9,
        "name": "format_word_as_binary",
        "body_sha256": "d31b09bb829f6a555d1d683085fd73e797e0666f23d9e2ed2c8ebdb1c5592387",
    },
    {
        "entry": 0x28E9,
        "end": 0x2910,
        "name": "format_word_as_hex",
        "body_sha256": "83db563e61f3bc926fb65783c5227820d3dc868f19c72edf04f5faca78d9c4b4",
    },
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encoded_offset_hits(executable: bytes, entry: int) -> list[str]:
    needle = struct.pack("<H", entry)
    hits = []
    offset = 0
    while (match := executable.find(needle, offset)) >= 0:
        hits.append(f"0x{match:04x}")
        offset = match + 1
    return hits


def incoming_calls(graph: dict[str, Any], entry: int) -> list[str]:
    return [
        f"0x{int(caller):04x}"
        for caller, callees in graph["callgraph"].items()
        if entry in callees
    ]


def build_report() -> dict[str, Any]:
    executable = EXECUTABLE.read_bytes()
    graph_bytes = GRAPH.read_bytes()
    if (digest := sha256(executable)) != EXECUTABLE_SHA256:
        raise RuntimeError(f"unsupported BBB executable SHA-256 {digest}")
    if (digest := sha256(graph_bytes)) != GRAPH_SHA256:
        raise RuntimeError(f"unsupported BBB graph SHA-256 {digest}")
    graph = json.loads(graph_bytes)

    rows = []
    for routine in ROUTINES:
        entry = routine["entry"]
        end = routine["end"]
        body_digest = sha256(executable[entry:end])
        if body_digest != routine["body_sha256"]:
            raise RuntimeError(f"{entry:#x}: body changed to {body_digest}")
        if entry not in graph["funcs"] or entry not in graph["leaves"]:
            raise RuntimeError(f"{entry:#x}: no longer a static leaf")
        callers = incoming_calls(graph, entry)
        if callers:
            raise RuntimeError(f"{entry:#x}: acquired direct callers {callers}")
        callees = graph["callgraph"].get(str(entry), [])
        if callees:
            raise RuntimeError(f"{entry:#x}: acquired direct callees {callees}")
        address_hits = encoded_offset_hits(executable, entry)
        if address_hits:
            raise RuntimeError(
                f"{entry:#x}: acquired encoded offset references {address_hits}"
            )
        predecessor = executable[entry - 1]
        if predecessor != FAR_RETURN_OPCODE:
            raise RuntimeError(
                f"{entry:#x}: predecessor changed from far return to {predecessor:#x}"
            )
        rows.append(
            {
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "name": routine["name"],
                "body_sha256": body_digest,
                "instruction_bytes": end - entry,
                "predecessor_opcode": f"0x{predecessor:02x}",
                "direct_callers": callers,
                "direct_callees": callees,
                "encoded_offset_hits": address_hits,
            }
        )

    return {
        "format": "big_bug_bang_unreferenced_library_audit_v1",
        "scope": (
            "Static exclusion of isolated compiler-library formatter bodies with "
            "no recovered direct caller or encoded offset materialization; this "
            "does not generalize to computed indirect targets"
        ),
        "inputs": {
            str(EXECUTABLE.relative_to(ROOT)): EXECUTABLE_SHA256,
            str(GRAPH.relative_to(ROOT)): GRAPH_SHA256,
        },
        "routines": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    report = build_report()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"verified {len(report['routines'])} unreferenced BBB library routines")


if __name__ == "__main__":
    main()
