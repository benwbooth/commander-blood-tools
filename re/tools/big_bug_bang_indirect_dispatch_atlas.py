#!/usr/bin/env python3
"""Resolve the pinned Big Bug Bang executable's static indirect dispatches."""

from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import struct
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path
    for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]

from capstone.x86 import X86_OP_MEM  # noqa: E402


_SPEC = importlib.util.spec_from_file_location(
    "build_function_graph", _HERE / "build_function_graph.py"
)
assert _SPEC is not None and _SPEC.loader is not None
GRAPH = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = GRAPH
_SPEC.loader.exec_module(GRAPH)


EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
DIRECT_SEGMENT_ZERO = re.compile(r"^0,\s*0x[0-9a-f]+$", re.IGNORECASE)

TABLES = (
    {
        "name": "input_action_handlers",
        "description": "Keyboard action dispatch after the CS translation table",
        "dispatch_sites": (0x23CB,),
        "kind": "near",
        "table": 0x2382,
        "target_base": 0x0F70,
        "count": 16,
        "count_basis": "16 words end at owner routine 0x23a2",
    },
    {
        "name": "value_conversion_callbacks",
        "description": "Five far callbacks selected by (record kind - 8) * 4",
        "dispatch_sites": (0x2637,),
        "kind": "far",
        "table": 0x25CA,
        "count": 5,
        "count_basis": "five far pointers end at owner routine 0x25de",
    },
    {
        "name": "sprite_blitter_candidates",
        "description": "Eight candidates preceding the mutable blitter slot",
        "dispatch_sites": (0x4983,),
        "kind": "near",
        "table": 0x499F,
        "target_base": 0x3310,
        "count": 8,
        "mutable_slot": 0x49AF,
        "count_basis": "eight words end at the indirect call's mutable slot",
    },
    {
        "name": "vm_opcode_handlers",
        "description": "VM opcodes 0xa0 through 0xd7",
        "dispatch_sites": (0x5AD4, 0x5B7B),
        "kind": "near",
        "table": 0x16A78,
        "target_base": 0x5820,
        "count": 0x38,
        "selector_base": 0xA0,
        "count_basis": "complete pinned executable opcode domain 0xa0..0xd7",
    },
    {
        "name": "record_kind_handlers",
        "description": "Record-kind rendering/format dispatch",
        "dispatch_sites": (0x7D22,),
        "kind": "near",
        "table": 0x7C3B,
        "target_base": 0x5820,
        "count": 21,
        "count_basis": "dispatcher compares the selector with 0x15 at 0x7d39",
    },
    {
        "name": "byte_parser_handlers",
        "description": "One-based byte parser dispatch",
        "dispatch_sites": (0x8527,),
        "kind": "near",
        "table": 0x8560,
        "target_base": 0x5820,
        "count": 18,
        "count_basis": "18 words end at first handler 0x8584",
    },
    {
        "name": "navigation_actor_handlers",
        "description": "Bridge/navigation actor-row handlers",
        "dispatch_sites": (0x8EDD,),
        "kind": "near",
        "table": 0x8F88,
        "target_base": 0x8830,
        "count": 6,
        "count_basis": "dispatcher loads loop count 6 at 0x8e7c",
    },
    {
        "name": "navigation_choice_handlers",
        "description": "Bridge/navigation choice handlers",
        "dispatch_sites": (0x989A,),
        "kind": "near",
        "table": 0x98A3,
        "target_base": 0x8830,
        "count": 5,
        "count_basis": "dispatcher rejects choice indexes >= 5 at 0x9827",
    },
)

STATIC_SITE_TO_TABLE = {
    site: table["name"] for table in TABLES for site in table["dispatch_sites"]
}


def h(value: int) -> str:
    return f"0x{value:06x}"


def executable_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def direct_far_targets(mz) -> set[int]:
    targets = set()
    for site in range(mz.header_size, mz.image_total - 5):
        if mz.data[site] not in (0x9A, 0xEA):
            continue
        if mz.file_to_image(site + 3) not in mz.reloc_image_offsets:
            continue
        offset, segment = struct.unpack_from("<HH", mz.data, site + 1)
        targets.add(mz.segoff_to_file(segment, offset))
    return targets


def cs_operand_address(builder, site: int) -> int:
    owners = [
        entry
        for entry, instructions in builder.function_instructions.items()
        if site in instructions
    ]
    if len(owners) != 1:
        raise ValueError(f"expected one owner for {site:#x}, found {owners}")
    owner = owners[0]
    instruction = builder.one_instruction(site)
    memory = [operand for operand in instruction.operands if operand.type == X86_OP_MEM]
    if len(memory) != 1 or instruction.reg_name(memory[0].mem.segment) != "cs":
        raise ValueError(f"{site:#x} is not a single explicit-CS memory transfer")
    segment = builder.instruction_segments[(owner, site)]
    return builder.mz.header_size + segment * 16 + int(memory[0].mem.disp)


def table_entries(mz, table: dict[str, object]) -> tuple[list[dict[str, object]], set[int]]:
    start = int(table["table"])
    count = int(table["count"])
    entries = []
    targets = set()
    for index in range(count):
        if table["kind"] == "far":
            offset, segment = struct.unpack_from("<HH", mz.data, start + index * 4)
            target = mz.segoff_to_file(segment, offset)
            encoded: object = f"{segment:#06x}:{offset:#06x}"
        else:
            raw = struct.unpack_from("<H", mz.data, start + index * 2)[0]
            target = int(table["target_base"]) + raw
            encoded = f"{raw:#06x}"
        if not (mz.header_size <= target < mz.image_total):
            raise ValueError(
                f"{table['name']} index {index} target outside image: {target:#x}"
            )
        targets.add(target)
        selector = index + int(table.get("selector_base", 0))
        entries.append(
            {
                "index": index,
                "selector": f"0x{selector:02x}",
                "encoded": encoded,
                "target_file_offset": h(target),
            }
        )
    return entries, targets


def classify_indirect(records: list[list[str]]) -> list[dict[str, object]]:
    result = []
    for site_text, mnemonic, operand in records:
        site = int(site_text, 16)
        detail: dict[str, object] = {}
        if site in STATIC_SITE_TO_TABLE:
            category = "static_internal_dispatch_table"
            detail["table"] = STATIC_SITE_TO_TABLE[site]
        elif DIRECT_SEGMENT_ZERO.match(operand):
            category = "direct_far_segment0_runtime_call"
        elif "0xc42" in operand:
            category = "external_xms_driver_vector"
        elif "0xc8e" in operand:
            category = "dynamic_presentation_callback_vector"
        elif any(value in operand for value in ("0xf21", "0xf29", "0xf2d", "0xf39", "0xf41")):
            category = "external_sound_driver_vector"
        else:
            category = "unknown"
        result.append(
            {
                "site_file_offset": h(site),
                "mnemonic": mnemonic,
                "operand": operand,
                "category": category,
                "detail": detail,
            }
        )
    return result


def build_atlas(executable: Path, graph_path: Path) -> dict[str, object]:
    digest = executable_digest(executable)
    if digest != EXECUTABLE_SHA256:
        raise ValueError(f"unrecognized BLOOD2PG.EXE SHA-256 {digest}")

    mz = GRAPH.MZ(str(executable))
    builder = GRAPH.FunctionGraphBuilder(mz)
    generated_graph = builder.build()
    supplied_graph = json.loads(graph_path.read_text())
    if supplied_graph != generated_graph:
        raise ValueError("supplied function graph does not match the executable")

    functions = set(generated_graph["funcs"])
    static_targets: set[int] = set()
    tables = []
    for definition in TABLES:
        dispatch_sites = tuple(int(site) for site in definition["dispatch_sites"])
        if definition["name"] == "sprite_blitter_candidates":
            derived = cs_operand_address(builder, dispatch_sites[0])
            if derived != int(definition["mutable_slot"]):
                raise ValueError("changed mutable sprite dispatch slot")
        elif definition["name"] != "vm_opcode_handlers":
            derived = cs_operand_address(builder, dispatch_sites[0])
            if derived != int(definition["table"]):
                raise ValueError(
                    f"changed {definition['name']} table: {derived:#x}"
                )
        entries, targets = table_entries(mz, definition)
        static_targets.update(targets)
        table = {
            "name": definition["name"],
            "description": definition["description"],
            "dispatch_sites": [h(site) for site in dispatch_sites],
            "table_file_offset": h(int(definition["table"])),
            "entry_count": int(definition["count"]),
            "distinct_target_count": len(targets),
            "count_basis": definition["count_basis"],
            "targets_missing_from_recursive_graph": [
                h(target) for target in sorted(targets - functions)
            ],
            "entries": entries,
        }
        if "mutable_slot" in definition:
            table["mutable_slot_file_offset"] = h(int(definition["mutable_slot"]))
        tables.append(table)

    classified = classify_indirect(generated_graph["indirect"])
    category_counts = collections.Counter(row["category"] for row in classified)
    unknown = category_counts.get("unknown", 0)
    if unknown:
        raise ValueError(f"{unknown} indirect records remain unclassified")

    far_targets = direct_far_targets(mz)
    direct_denominator = functions | far_targets
    resolved_denominator = direct_denominator | static_targets
    return {
        "format": "big-bug-bang-indirect-dispatch-atlas-v1",
        "scope": (
            "Static lower-bound inventory; external vectors and dynamic callbacks "
            "still require runtime traces"
        ),
        "input": {
            "executable": os.fspath(executable),
            "executable_sha256": digest,
            "graph": os.fspath(graph_path),
        },
        "counts": {
            "recursive_graph_functions": len(functions),
            "relocation_proven_far_targets": len(far_targets),
            "lower_bound_after_direct_far": len(direct_denominator),
            "static_table_count": len(TABLES),
            "static_table_distinct_targets": len(static_targets),
            "static_targets_missing_from_direct_denominator": len(
                static_targets - direct_denominator
            ),
            "lower_bound_after_static_tables": len(resolved_denominator),
            "indirect_records": len(classified),
            "unique_indirect_sites": len(
                {row["site_file_offset"] for row in classified}
            ),
            "unclassified_records": unknown,
        },
        "category_record_counts": dict(sorted(category_counts.items())),
        "tables": tables,
        "classified_indirect_records": classified,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("graph", type=Path)
    parser.add_argument("output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    atlas = build_atlas(args.executable, args.graph)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(atlas, indent=2) + "\n")
    counts = atlas["counts"]
    print(
        f"{counts['lower_bound_after_static_tables']} native targets in the "
        f"static lower bound; {counts['unclassified_records']} unclassified records"
    )
    print(args.output)


if __name__ == "__main__":
    main()
