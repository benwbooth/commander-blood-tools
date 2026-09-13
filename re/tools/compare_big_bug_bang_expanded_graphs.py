#!/usr/bin/env python3
"""Compare Commander and BBB graphs closed over known static dispatch tables."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import struct
import sys


_HERE = Path(__file__).resolve().parent


def load_tool(name: str):
    spec = importlib.util.spec_from_file_location(name, _HERE / f"{name}.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


GRAPH = load_tool("build_function_graph")
COMPARE = load_tool("compare_function_graphs")
COMMANDER_ATLAS = load_tool("indirect_dispatch_atlas")
BBB_ATLAS = load_tool("big_bug_bang_indirect_dispatch_atlas")


def relocation_far_roots(mz) -> dict[int, int]:
    roots = {}
    for site in range(mz.header_size, mz.image_total - 5):
        if mz.data[site] not in (0x9A, 0xEA):
            continue
        if mz.file_to_image(site + 3) not in mz.reloc_image_offsets:
            continue
        offset, segment = struct.unpack_from("<HH", mz.data, site + 1)
        roots[mz.segoff_to_file(segment, offset)] = segment
    return roots


def near_table_roots(
    mz, table: int, count: int, target_base: int
) -> dict[int, int]:
    segment = (target_base - mz.header_size) // 16
    if mz.header_size + segment * 16 != target_base:
        raise ValueError(f"unaligned target base {target_base:#x}")
    return {
        target_base + struct.unpack_from("<H", mz.data, table + index * 2)[0]: segment
        for index in range(count)
    }


def commander_roots(mz) -> dict[int, int]:
    roots = relocation_far_roots(mz)
    for table in COMMANDER_ATLAS.STATIC_TABLES:
        roots.update(
            near_table_roots(
                mz,
                int(table["table_file_offset"]),
                int(table["entry_count"]),
                int(table["target_base_file_offset"]),
            )
        )
    return roots


def bbb_table_roots(mz, table: dict[str, object]) -> dict[int, int]:
    start = int(table["table"])
    count = int(table["count"])
    if table["kind"] == "near":
        return near_table_roots(mz, start, count, int(table["target_base"]))
    roots = {}
    for index in range(count):
        offset, segment = struct.unpack_from("<HH", mz.data, start + index * 4)
        roots[mz.segoff_to_file(segment, offset)] = segment
    return roots


def bbb_roots(mz) -> dict[int, int]:
    roots = relocation_far_roots(mz)
    for table in BBB_ATLAS.TABLES:
        roots.update(bbb_table_roots(mz, table))
    return roots


def expanded_builder(path: Path, roots) -> tuple[object, dict[str, object]]:
    builder = GRAPH.FunctionGraphBuilder(GRAPH.MZ(str(path)))
    builder.build()
    for entry, segment in sorted(roots(builder.mz).items()):
        builder.walk_function(entry, segment)
    return builder, builder.graph()


def table_targets(mz, table: dict[str, object]) -> set[int]:
    return set(bbb_table_roots(mz, table))


def compare_expanded(
    commander: Path, sequel: Path, expected_bbb_graph: Path | None = None
) -> dict[str, object]:
    left_builder, left_graph = expanded_builder(commander, commander_roots)
    right_builder, right_graph = expanded_builder(sequel, bbb_roots)
    if expected_bbb_graph is not None:
        expected = json.loads(expected_bbb_graph.read_text())
        if expected != right_graph:
            raise ValueError("expanded BBB graph does not match its checked-in artifact")

    report = COMPARE.compare_builders(
        commander,
        sequel,
        left_builder,
        left_graph,
        right_builder,
        right_graph,
    )
    mapped = {
        int(row["sequel"]["entry"], 16): row["classification"]
        for row in report["matches"]
    }
    report["scope"] = (
        "Static correspondence candidates after known-table closure; structural "
        "matches are not behavioral parity proof"
    )
    report["closure"] = {
        "commander_root_count": len(commander_roots(left_builder.mz)),
        "sequel_root_count": len(bbb_roots(right_builder.mz)),
        "sequel_static_tables": [
            {
                "name": table["name"],
                "distinct_targets": len(targets),
                "unique_correspondences": sum(target in mapped for target in targets),
                "exact_body_correspondences": sum(
                    mapped.get(target) == "exact_body" for target in targets
                ),
                "unmapped_targets": [
                    hex(target) for target in sorted(targets) if target not in mapped
                ],
            }
            for table in BBB_ATLAS.TABLES
            for targets in (table_targets(right_builder.mz, table),)
        ],
    }
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("commander", type=Path)
    parser.add_argument("sequel", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--expected-bbb-graph", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = compare_expanded(
        args.commander, args.sequel, args.expected_bbb_graph
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    summary = report["summary"]
    print(
        f"{summary['unique_structural_correspondences']} expanded structural "
        f"correspondences ({summary['exact_body_correspondences']} exact); "
        f"{summary['unresolved_sequel_functions']} sequel functions unresolved"
    )
    print(args.output)


if __name__ == "__main__":
    main()
