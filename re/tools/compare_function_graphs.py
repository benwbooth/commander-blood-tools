#!/usr/bin/env python3
"""Compare reachable DOS routine bodies without treating similarity as parity.

Exact-body matches concatenate the bytes of every instruction reached by the
function's control-flow walk. Structural candidates additionally normalize
direct call destinations and address-like constants while preserving opcode,
register, memory-addressing, small-constant, and local-branch structure.

Structural matches are correspondence candidates, not behavioral proof. The
report leaves non-unique and changed routines unresolved for oracle work.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys

import capstone
from capstone.x86 import (
    X86_INS_CALL,
    X86_INS_JMP,
    X86_INS_LCALL,
    X86_INS_LJMP,
    X86_OP_IMM,
    X86_OP_MEM,
    X86_OP_REG,
)


_HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location(
    "build_function_graph", _HERE / "build_function_graph.py"
)
assert _SPEC is not None and _SPEC.loader is not None
GRAPH = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = GRAPH
_SPEC.loader.exec_module(GRAPH)


DIRECT_CALLS = {X86_INS_CALL, X86_INS_LCALL}
DIRECT_JUMPS = {X86_INS_JMP, X86_INS_LJMP}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def signed_immediate(value: int, size: int) -> int:
    bits = max(1, size) * 8
    mask = (1 << bits) - 1
    value &= mask
    sign = 1 << (bits - 1)
    return value - (1 << bits) if value & sign else value


def instruction_shape(instruction, function_entry: int) -> list[object]:
    is_jump = (
        instruction.id in DIRECT_JUMPS
        or capstone.CS_GRP_JUMP in instruction.groups
    )
    operands: list[object] = []
    for operand in instruction.operands:
        if operand.type == X86_OP_REG:
            operands.append(
                ["register", instruction.reg_name(operand.reg), operand.size]
            )
        elif operand.type == X86_OP_IMM:
            value = int(operand.imm)
            if instruction.id in DIRECT_CALLS:
                normalized: object = "call_target"
            elif is_jump:
                normalized = ["local_target", value - function_entry]
            else:
                signed = signed_immediate(value, operand.size)
                normalized = signed if -256 <= signed <= 256 else "large_constant"
            operands.append(["immediate", normalized, operand.size])
        elif operand.type == X86_OP_MEM:
            segment = (
                instruction.reg_name(operand.mem.segment)
                if operand.mem.segment
                else ""
            )
            base = (
                instruction.reg_name(operand.mem.base) if operand.mem.base else ""
            )
            index = (
                instruction.reg_name(operand.mem.index)
                if operand.mem.index
                else ""
            )
            displacement = int(operand.mem.disp)
            if base in {"bp", "ebp", "sp", "esp"} or -256 <= displacement <= 256:
                normalized_displacement: object = displacement
            else:
                normalized_displacement = "address"
            operands.append(
                [
                    "memory",
                    segment,
                    base,
                    index,
                    operand.mem.scale,
                    normalized_displacement,
                    operand.size,
                ]
            )
        else:
            operands.append(["other", operand.type, operand.size])
    return [
        instruction.id,
        instruction.mnemonic,
        list(instruction.prefix),
        instruction.addr_size,
        operands,
    ]


def body_bytes(builder, entry: int) -> bytes:
    return b"".join(
        bytes(builder.one_instruction(address).bytes)
        for address in sorted(builder.function_instructions[entry])
    )


def structural_key(builder, entry: int) -> str:
    shape = [
        instruction_shape(builder.one_instruction(address), entry)
        for address in sorted(builder.function_instructions[entry])
    ]
    return json.dumps(shape, separators=(",", ":"))


def index_functions(builder, functions: list[int], key) -> dict[object, list[int]]:
    result: dict[object, list[int]] = collections.defaultdict(list)
    for entry in functions:
        result[key(builder, entry)].append(entry)
    return result


def reverse_edges(graph: dict[str, object]) -> dict[int, set[int]]:
    result: dict[int, set[int]] = collections.defaultdict(set)
    for caller, callees in graph["callgraph"].items():
        for callee in callees:
            result[int(callee)].add(int(caller))
    return result


def function_summary(builder, graph: dict[str, object], reverse, entry: int):
    instructions = builder.function_instructions[entry]
    return {
        "entry": hex(entry),
        "instruction_count": len(instructions),
        "instruction_bytes": len(body_bytes(builder, entry)),
        "direct_callers": len(reverse.get(entry, ())),
        "direct_callees": len(graph["callgraph"][str(entry)]),
    }


def compare(left_path: Path, right_path: Path) -> dict[str, object]:
    left_builder = GRAPH.FunctionGraphBuilder(GRAPH.MZ(str(left_path)))
    right_builder = GRAPH.FunctionGraphBuilder(GRAPH.MZ(str(right_path)))
    left_graph = left_builder.build()
    right_graph = right_builder.build()
    left_functions = left_graph["funcs"]
    right_functions = right_graph["funcs"]

    left_shapes = index_functions(left_builder, left_functions, structural_key)
    right_shapes = index_functions(right_builder, right_functions, structural_key)
    left_bytes = index_functions(
        left_builder, left_functions, lambda builder, entry: body_bytes(builder, entry)
    )
    right_bytes = index_functions(
        right_builder, right_functions, lambda builder, entry: body_bytes(builder, entry)
    )

    mapping: dict[int, int] = {}
    ambiguous = []
    unresolved = []
    for right_entry in right_functions:
        shape = structural_key(right_builder, right_entry)
        left_candidates = left_shapes.get(shape, [])
        right_candidates = right_shapes[shape]
        if len(left_candidates) == 1 and len(right_candidates) == 1:
            mapping[right_entry] = left_candidates[0]
        elif left_candidates:
            ambiguous.append(
                {
                    "sequel_entry": hex(right_entry),
                    "commander_candidates": [hex(entry) for entry in left_candidates],
                    "sequel_signature_entries": [
                        hex(entry) for entry in right_candidates
                    ],
                }
            )
        else:
            unresolved.append(right_entry)

    left_reverse = reverse_edges(left_graph)
    right_reverse = reverse_edges(right_graph)
    matches = []
    exact_count = 0
    for right_entry, left_entry in sorted(mapping.items()):
        right_body = body_bytes(right_builder, right_entry)
        exact = (
            len(left_bytes[right_body]) == 1
            and len(right_bytes[right_body]) == 1
            and left_bytes[right_body][0] == left_entry
        )
        if exact:
            exact_count += 1
        matches.append(
            {
                "classification": (
                    "exact_body" if exact else "structural_candidate"
                ),
                "sequel": function_summary(
                    right_builder, right_graph, right_reverse, right_entry
                ),
                "commander": function_summary(
                    left_builder, left_graph, left_reverse, left_entry
                ),
            }
        )

    mapped_edges = 0
    preserved_edges = 0
    left_edges = {
        (int(caller), int(callee))
        for caller, callees in left_graph["callgraph"].items()
        for callee in callees
    }
    for right_caller, right_callees in right_graph["callgraph"].items():
        mapped_caller = mapping.get(int(right_caller))
        if mapped_caller is None:
            continue
        for right_callee in right_callees:
            mapped_callee = mapping.get(int(right_callee))
            if mapped_callee is None:
                continue
            mapped_edges += 1
            if (mapped_caller, mapped_callee) in left_edges:
                preserved_edges += 1

    mapped_left = set(mapping.values())
    return {
        "scope": (
            "Static correspondence candidates; structural matches are not "
            "behavioral parity proof"
        ),
        "inputs": {
            "commander": {
                "path": os.fspath(left_path),
                "sha256": sha256_file(left_path),
            },
            "sequel": {
                "path": os.fspath(right_path),
                "sha256": sha256_file(right_path),
            },
        },
        "summary": {
            "commander_functions": len(left_functions),
            "sequel_functions": len(right_functions),
            "unique_structural_correspondences": len(mapping),
            "exact_body_correspondences": exact_count,
            "relocation_tolerant_correspondences": len(mapping) - exact_count,
            "ambiguous_sequel_functions": len(ambiguous),
            "unresolved_sequel_functions": len(unresolved),
            "unresolved_commander_functions": len(left_functions) - len(mapped_left),
            "mapped_sequel_edges": mapped_edges,
            "mapped_edges_present_in_commander": preserved_edges,
        },
        "matches": matches,
        "ambiguous": ambiguous,
        "unresolved_sequel": [
            function_summary(right_builder, right_graph, right_reverse, entry)
            for entry in unresolved
        ],
        "unresolved_commander": [
            function_summary(left_builder, left_graph, left_reverse, entry)
            for entry in left_functions
            if entry not in mapped_left
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("commander", type=Path)
    parser.add_argument("sequel", type=Path)
    parser.add_argument("output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = compare(args.commander, args.sequel)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    summary = report["summary"]
    print(
        f"{summary['unique_structural_correspondences']} unique structural "
        f"correspondences ({summary['exact_body_correspondences']} exact); "
        f"{summary['unresolved_sequel_functions']} sequel functions unresolved"
    )
    print(args.output)


if __name__ == "__main__":
    main()
