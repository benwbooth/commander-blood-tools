#!/usr/bin/env python3
"""Classify indirect call sites from re/func_graph.json.

The recursive graph records 48 "indirect" sites. They are not all the same kind
of work:

* some are relocation-backed direct far calls to segment 0 that should be folded
  into the direct-call denominator
* some are static near-handler tables in BLOODPRG.EXE
* some are runtime vectors into XMS, the sound driver, or presentation callbacks
* the input action table contains 16 recovered near handlers selected through a
  256-byte key translation table

This tool makes that split reproducible and emits JSON.
"""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import collections
import json
import re
import struct
from pathlib import Path

sys.path.insert(0, _HERE)

from mzfile import MZ, load_labels  # noqa: E402

sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]


RE_ROOT = Path(_HERE).parent
DEFAULT_GRAPH = RE_ROOT / "func_graph.json"
DEFAULT_BIN = RE_ROOT / "bin" / "BLOODPRG.EXE"


STATIC_TABLES = [
    {
        "name": "vm_opcode_handlers",
        "description": "VM opcode dispatch, opcodes 0xA0..0xD3",
        "sites": [0x5627, 0x56C4],
        "table_file_offset": 0x142D0,
        "entry_count": 0x34,
        "target_base_file_offset": 0x053A0,
        "index_base": 0xA0,
        "index_prefix": "opcode",
    },
    {
        "name": "nav_actor_subdispatch",
        "description": "Bridge/nav actor-row handler table",
        "sites": [0x7E09],
        "table_file_offset": 0x07EB4,
        "entry_count": 6,
        "target_base_file_offset": 0x077E0,
        "index_base": 0,
        "index_prefix": "slot",
    },
    {
        "name": "nav_choice_subdispatch",
        "description": "Bridge/nav choice handler table",
        "sites": [0x8700],
        "table_file_offset": 0x08709,
        "entry_count": 5,
        "target_base_file_offset": 0x077E0,
        "index_base": 0,
        "index_prefix": "choice",
    },
    {
        "name": "sprite_blitter_candidates",
        "description": "Sprite blitter candidate table; call uses mutable slot at 0x4532",
        "sites": [0x4506],
        "table_file_offset": 0x04522,
        "entry_count": 8,
        "target_base_file_offset": 0x02F90,
        "index_base": 0,
        "index_prefix": "blit",
        "mutable_slot_file_offset": 0x04532,
    },
    {
        "name": "input_action_handlers",
        "description": "Keyboard action dispatch after CS key translation",
        "sites": [0x2137],
        "table_file_offset": 0x020EE,
        "entry_count": 16,
        "target_base_file_offset": 0x00EB0,
        "index_base": 0,
        "index_prefix": "action",
    },
    {
        "name": "byte_parser_dispatch_74e5",
        "description": "Byte-indexed parser dispatch table after 0x74E5",
        "sites": [0x74E5],
        "table_file_offset": 0x0751E,
        "entry_count": 18,
        "target_base_file_offset": 0x053A0,
        "index_base": 1,
        "index_prefix": "byte",
    },
]

STATIC_TABLE_BY_SITE = {
    site: table["name"] for table in STATIC_TABLES for site in table["sites"]
}

DIRECT_SEG0_RE = re.compile(r"^0,\s*0x([0-9a-fA-F]+)$")


def h(n: int, width: int = 0) -> str:
    if width:
        return f"0x{n:0{width}x}"
    return f"0x{n:x}"


def parse_site(site: str) -> int:
    return int(site, 16)


def load_graph(path: Path) -> dict[str, object]:
    with path.open() as fh:
        graph = json.load(fh)
    return graph


def label_for(labels: dict[int, tuple[str, str]], file_off: int) -> dict[str, str] | None:
    label = labels.get(file_off)
    if not label:
        return None
    name, comment = label
    out = {"name": name}
    if comment:
        out["comment"] = comment
    return out


def direct_far_targets(mz: MZ) -> set[int]:
    out = set()
    for opcode in (0x9A, 0xEA):
        i = mz.header_size
        while i < mz.image_total - 5:
            if mz.data[i] == opcode:
                off, seg = struct.unpack_from("<HH", mz.data, i + 1)
                if mz.file_to_image(i + 3) in mz.reloc_image_offsets:
                    out.add(mz.segoff_to_file(seg, off))
            i += 1
    return out


def table_entries(mz: MZ, labels: dict[int, tuple[str, str]], table: dict[str, object]) -> list[dict[str, object]]:
    table_file = int(table["table_file_offset"])
    target_base = int(table["target_base_file_offset"])
    index_base = int(table["index_base"])
    index_prefix = str(table["index_prefix"])
    entries = []
    for idx in range(int(table["entry_count"])):
        raw = struct.unpack_from("<H", mz.data, table_file + idx * 2)[0]
        target = target_base + raw
        selector = index_base + idx
        if index_prefix in {"opcode", "byte"}:
            key = f"{index_prefix}_0x{selector:02x}"
        else:
            key = f"{index_prefix}_{selector}"
        entries.append(
            {
                "index": key,
                "raw_near_offset": h(raw, 4),
                "target_file_offset": h(target, 6),
                "target_label": label_for(labels, target),
                "first_bytes": mz.data[target : target + 8].hex(" ")
                if 0 <= target < len(mz.data)
                else "",
            }
        )
    return entries


def input_dispatch_summary(mz: MZ) -> dict[str, object]:
    xlat = mz.data[0x1FEE : 0x1FEE + 256]
    live = [b for b in xlat if b < 0x80]
    distinct = sorted(set(live))
    return {
        "dispatch_site": h(0x2137, 6),
        "xlat_file_offset": h(0x1FEE, 6),
        "handler_table_operand": "cs:0x123e",
        "handler_table_file_offset": h(0x20EE, 6),
        "live_input_byte_count": len(live),
        "distinct_action_index_count": len(distinct),
        "max_action_index": max(distinct) if distinct else None,
        "action_indices": [h(x, 2) for x in distinct],
        "unmapped_handler_indices": [
            h(x, 2) for x in range(16) if x not in distinct
        ],
        "status": "dispatch table and all 16 handler targets recovered",
    }


def classify_indirect_records(
    records: list[list[str]], mz: MZ, labels: dict[int, tuple[str, str]]
) -> list[dict[str, object]]:
    classified = []
    for site_s, mnemonic, op_str in records:
        site = parse_site(site_s)
        category = "unknown"
        detail: dict[str, object] = {}

        direct_seg0 = DIRECT_SEG0_RE.match(op_str)
        if direct_seg0:
            off = int(direct_seg0.group(1), 16)
            target = mz.segoff_to_file(0, off)
            category = "direct_far_segment0_not_indirect"
            detail = {
                "target_file_offset": h(target, 6),
                "target_label": label_for(labels, target),
            }
        elif site in STATIC_TABLE_BY_SITE:
            category = "static_internal_dispatch_table"
            detail = {"table": STATIC_TABLE_BY_SITE[site]}
        elif "0xa4a" in op_str:
            category = "external_xms_driver_vector"
            detail = {"vector": "DS/GS:0x0A4A"}
        elif "0xa96" in op_str:
            category = "dynamic_presentation_callback_vector"
            detail = {"vector": "DS:0x0A96"}
        elif any(vec in op_str for vec in ("0xcd3", "0xcdb", "0xcdf", "0xceb", "0xcf3")):
            category = "external_sound_driver_vector"
            detail = {"vector_operand": op_str}
        classified.append(
            {
                "site_file_offset": h(site, 6),
                "mnemonic": mnemonic,
                "operand": op_str,
                "category": category,
                "detail": detail,
            }
        )
    return classified


def build_atlas(args: argparse.Namespace) -> dict[str, object]:
    mz = MZ(str(args.exe))
    graph = load_graph(args.graph)
    _, labels = load_labels()
    funcs = {int(x) for x in graph.get("funcs", [])}
    far_targets = direct_far_targets(mz)
    records = graph.get("indirect", [])
    classified = classify_indirect_records(records, mz, labels)

    static_tables = []
    static_targets = set()
    for table in STATIC_TABLES:
        entries = table_entries(mz, labels, table)
        targets = {int(entry["target_file_offset"], 16) for entry in entries}
        static_targets.update(targets)
        out = {
            "name": table["name"],
            "description": table["description"],
            "dispatch_sites": [h(site, 6) for site in table["sites"]],
            "table_file_offset": h(int(table["table_file_offset"]), 6),
            "entry_count": int(table["entry_count"]),
            "distinct_target_count": len(targets),
            "targets_missing_from_recursive_graph": [
                h(x, 6) for x in sorted(targets - funcs)
            ],
            "entries": entries if args.include_entries else entries[: args.sample_limit],
        }
        if "mutable_slot_file_offset" in table:
            slot = int(table["mutable_slot_file_offset"])
            out["mutable_slot_file_offset"] = h(slot, 6)
            out["mutable_slot_initial_word"] = h(struct.unpack_from("<H", mz.data, slot)[0], 4)
        static_tables.append(out)

    category_counts = collections.Counter(str(item["category"]) for item in classified)
    unique_site_counts = collections.defaultdict(set)
    for item in classified:
        unique_site_counts[str(item["category"])].add(str(item["site_file_offset"]))

    base_denominator = funcs | far_targets | {mz.entry_file}
    lower_bound_with_static = base_denominator | static_targets

    return {
        "input": {
            "exe": str(args.exe),
            "graph": str(args.graph),
        },
        "counts": {
            "indirect_records": len(records),
            "unique_indirect_sites": len({row[0] for row in records}),
            "classified_records": sum(category_counts.values()) - category_counts.get("unknown", 0),
            "unknown_records": category_counts.get("unknown", 0),
            "direct_far_segment0_records": category_counts.get(
                "direct_far_segment0_not_indirect", 0
            ),
            "static_table_dispatch_records": category_counts.get(
                "static_internal_dispatch_table", 0
            ),
            "static_table_dispatch_sites": sum(len(table["sites"]) for table in STATIC_TABLES),
            "static_table_distinct_targets": len(static_targets),
            "static_table_targets_missing_from_recursive_graph": len(static_targets - funcs),
            "static_table_targets_missing_from_direct_far_denominator": len(
                static_targets - base_denominator
            ),
            "lower_bound_after_direct_far": len(base_denominator),
            "lower_bound_after_static_tables": len(lower_bound_with_static),
        },
        "category_record_counts": dict(sorted(category_counts.items())),
        "category_unique_site_counts": {
            key: len(value) for key, value in sorted(unique_site_counts.items())
        },
        "static_tables": static_tables,
        "input_dispatch": input_dispatch_summary(mz),
        "classified_indirect_records": classified if args.include_records else [],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=DEFAULT_BIN)
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--sample-limit", type=int, default=8)
    parser.add_argument("--include-entries", action="store_true")
    parser.add_argument("--include-records", action="store_true")
    args = parser.parse_args()

    print(json.dumps(build_atlas(args), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
