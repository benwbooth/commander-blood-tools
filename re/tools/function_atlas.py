#!/usr/bin/env python3
"""Emit a provenance-aware function atlas for BLOODPRG.EXE.

This is a bit-exact decompilation aid, not a decompiler. It separates hard
binary facts from derived analysis:

* relocation-proven direct far call/jump targets
* existing recursive-descent graph counts from re/func_graph.json
* graph/far-target coverage gaps
* per-entry byte-shape and first-terminal summaries

The output is JSON so it can be diffed, checked into notes, or consumed by later
batch lifters.
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
import hashlib
import json
import struct
from pathlib import Path

import capstone

sys.path.insert(0, _HERE)

from mzfile import MZ, load_labels  # noqa: E402

sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]


RE_ROOT = Path(_HERE).parent
DEFAULT_GRAPH = RE_ROOT / "func_graph.json"
DEFAULT_BIN = RE_ROOT / "bin" / "BLOODPRG.EXE"

TERMINAL_MNEMONICS = {"ret", "retf", "iret", "jmp", "ljmp"}


def h(n: int, width: int = 0) -> str:
    if width:
        return f"0x{n:0{width}x}"
    return f"0x{n:x}"


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_graph(path: Path) -> dict[str, object]:
    if not path.exists():
        return {"funcs": [], "leaves": [], "callgraph": {}, "indirect": []}
    with path.open() as fh:
        graph = json.load(fh)
    return {
        "funcs": sorted(int(x) for x in graph.get("funcs", [])),
        "leaves": sorted(int(x) for x in graph.get("leaves", [])),
        "callgraph": {
            int(k): sorted(int(x) for x in v)
            for k, v in graph.get("callgraph", {}).items()
        },
        "indirect": graph.get("indirect", []),
    }


def direct_far_transfers(mz: MZ) -> list[dict[str, object]]:
    """Return relocation-proven direct far transfer sites.

    A literal 9A/EA byte is not enough: the segment operand must sit at an MZ
    relocation site. That filter makes these much stronger evidence than raw
    opcode scanning.
    """
    out: list[dict[str, object]] = []
    for opcode, kind in ((0x9A, "call"), (0xEA, "jmp")):
        i = mz.header_size
        while i < mz.image_total - 5:
            if mz.data[i] == opcode:
                off, seg = struct.unpack_from("<HH", mz.data, i + 1)
                seg_operand_image = mz.file_to_image(i + 3)
                if seg_operand_image in mz.reloc_image_offsets:
                    out.append(
                        {
                            "site_file_offset": i,
                            "kind": kind,
                            "target_segment": seg,
                            "target_offset": off,
                            "target_file_offset": mz.segoff_to_file(seg, off),
                        }
                    )
            i += 1
    return sorted(out, key=lambda x: (int(x["site_file_offset"]), str(x["kind"])))


def segment_bases_from_relocs(mz: MZ, transfers: list[dict[str, object]]) -> list[int]:
    bases = set()
    for image_off in mz.reloc_image_offsets:
        if image_off + 1 < len(mz.image):
            bases.add(struct.unpack_from("<H", mz.image, image_off)[0])
    for transfer in transfers:
        bases.add(int(transfer["target_segment"]))
    return sorted(bases)


def segment_for_file(mz: MZ, bases: list[int], file_off: int) -> tuple[int, int]:
    image_off = mz.file_to_image(file_off)
    candidates = [seg for seg in bases if seg * 16 <= image_off]
    seg = max(candidates) if candidates else 0
    return seg, image_off - seg * 16


def label_for(labels: dict[int, tuple[str, str]], file_off: int) -> dict[str, str] | None:
    label = labels.get(file_off)
    if not label:
        return None
    name, comment = label
    out = {"name": name}
    if comment:
        out["comment"] = comment
    return out


def first_byte_class(byte: int | None) -> str:
    if byte is None:
        return "out_of_range"
    if 0x50 <= byte <= 0x57:
        return "push_reg"
    return {
        0x06: "push_es",
        0x0E: "push_cs",
        0x16: "push_ss",
        0x1E: "push_ds",
        0x55: "push_bp",
        0x60: "pusha",
        0x66: "operand_size_prefix",
        0x67: "address_size_prefix",
        0x9C: "pushf",
        0xB8: "mov_ax_imm16",
        0xC3: "near_ret_stub",
        0xCB: "far_ret_stub",
        0xE9: "near_jmp",
        0xEA: "far_jmp",
    }.get(byte, "other")


def linear_summary(mz: MZ, md: capstone.Cs, file_off: int, max_bytes: int) -> dict[str, object]:
    if not (0 <= file_off < len(mz.data)):
        return {
            "first_bytes": "",
            "first_byte_class": "out_of_range",
            "terminal": None,
            "instruction_count_to_terminal": 0,
        }

    end = min(len(mz.data), file_off + max_bytes)
    window = mz.data[file_off:end]
    first = window[0] if window else None
    first_insns: list[str] = []
    terminal = None
    count = 0

    for insn in md.disasm(window, file_off):
        count += 1
        if len(first_insns) < 6:
            first_insns.append(f"{insn.mnemonic} {insn.op_str}".strip())
        if insn.mnemonic in TERMINAL_MNEMONICS:
            terminal = {
                "file_offset": h(insn.address, 6),
                "mnemonic": insn.mnemonic,
                "op_str": insn.op_str,
                "bytes_from_entry": insn.address + insn.size - file_off,
            }
            break

    return {
        "first_bytes": window[:12].hex(" "),
        "first_byte_class": first_byte_class(first),
        "c_frame_prefix_55_8b_ec": window.startswith(bytes.fromhex("55 8b ec")),
        "first_instructions": first_insns,
        "terminal": terminal,
        "instruction_count_to_terminal": count,
    }


def transfer_site_sample(sites: list[dict[str, object]], sample_limit: int) -> list[dict[str, object]]:
    out = []
    for site in sites[:sample_limit]:
        out.append(
            {
                "site_file_offset": h(int(site["site_file_offset"]), 6),
                "kind": str(site["kind"]),
            }
        )
    return out


def build_atlas(args: argparse.Namespace) -> dict[str, object]:
    mz = MZ(str(args.exe))
    graph = load_graph(args.graph)
    _, labels = load_labels()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    transfers = direct_far_transfers(mz)
    bases = segment_bases_from_relocs(mz, transfers)
    by_target: dict[int, list[dict[str, object]]] = collections.defaultdict(list)
    by_segment: collections.Counter[int] = collections.Counter()
    by_kind: collections.Counter[str] = collections.Counter()
    for transfer in transfers:
        target = int(transfer["target_file_offset"])
        by_target[target].append(transfer)
        by_segment[int(transfer["target_segment"])] += 1
        by_kind[str(transfer["kind"])] += 1

    funcs = set(graph["funcs"])
    leaves = set(graph["leaves"])
    far_targets = set(by_target)
    all_entries = sorted(funcs | far_targets | {mz.entry_file})

    callgraph: dict[int, list[int]] = graph["callgraph"]  # type: ignore[assignment]
    reverse_edges: dict[int, list[int]] = collections.defaultdict(list)
    edge_count = 0
    for caller, callees in callgraph.items():
        edge_count += len(callees)
        for callee in callees:
            reverse_edges[callee].append(caller)

    entries = []
    first_byte_hist: collections.Counter[str] = collections.Counter()
    terminal_hist: collections.Counter[str] = collections.Counter()
    c_frame_count = 0

    for file_off in all_entries:
        seg, off = segment_for_file(mz, bases, file_off)
        summary = linear_summary(mz, md, file_off, args.max_bytes)
        first_byte_hist[str(summary["first_byte_class"])] += 1
        if summary["c_frame_prefix_55_8b_ec"]:
            c_frame_count += 1
        terminal = summary.get("terminal")
        terminal_key = "none"
        if isinstance(terminal, dict):
            terminal_key = str(terminal["mnemonic"])
        terminal_hist[terminal_key] += 1

        far_sites = by_target.get(file_off, [])
        graph_callers = sorted(reverse_edges.get(file_off, []))
        entry = {
            "file_offset": h(file_off, 6),
            "seg_off": f"{h(seg, 4)}:{h(off, 4)}",
            "in_recursive_graph": file_off in funcs,
            "is_graph_leaf": file_off in leaves,
            "is_mz_entry": file_off == mz.entry_file,
            "direct_far_incoming_count": len(far_sites),
            "graph_incoming_count": len(graph_callers),
            "graph_outgoing_count": len(callgraph.get(file_off, [])),
            "label": label_for(labels, file_off),
            "direct_far_site_sample": transfer_site_sample(far_sites, args.sample_limit),
            "graph_caller_sample": [h(x, 6) for x in graph_callers[: args.sample_limit]],
            "graph_callee_sample": [
                h(x, 6) for x in callgraph.get(file_off, [])[: args.sample_limit]
            ],
            "shape": summary,
        }
        entries.append(entry)

    missing_far_targets = sorted(far_targets - funcs)
    graph_only_entries = sorted(funcs - far_targets - {mz.entry_file})

    return {
        "input": {
            "exe": str(args.exe),
            "exe_sha256": sha256_file(args.exe),
            "graph": str(args.graph),
        },
        "counts": {
            "relocation_proven_direct_far_transfer_sites": len(transfers),
            "relocation_proven_direct_far_targets": len(far_targets),
            "recursive_graph_functions": len(funcs),
            "recursive_graph_edges": edge_count,
            "recursive_graph_leaves": len(leaves),
            "recursive_graph_indirect_sites": len(graph["indirect"]),
            "far_targets_in_recursive_graph": len(far_targets & funcs),
            "far_targets_missing_from_recursive_graph": len(missing_far_targets),
            "graph_entries_without_direct_far_incoming": len(graph_only_entries),
            "all_atlas_entries": len(all_entries),
            "c_frame_prefix_55_8b_ec_entries": c_frame_count,
        },
        "direct_far_transfer_kind_histogram": dict(sorted(by_kind.items())),
        "direct_far_target_segment_histogram": {
            h(seg, 4): count for seg, count in sorted(by_segment.items())
        },
        "entry_first_byte_class_histogram": dict(sorted(first_byte_hist.items())),
        "entry_terminal_histogram": dict(sorted(terminal_hist.items())),
        "far_targets_missing_from_recursive_graph": [
            {
                "file_offset": h(file_off, 6),
                "label": label_for(labels, file_off),
                "direct_far_incoming_count": len(by_target[file_off]),
                "direct_far_site_sample": transfer_site_sample(
                    by_target[file_off], args.sample_limit
                ),
                "shape": linear_summary(mz, md, file_off, args.max_bytes),
            }
            for file_off in missing_far_targets
        ],
        "graph_leaf_lift_queue_sample": [
            {
                "file_offset": h(file_off, 6),
                "label": label_for(labels, file_off),
                "graph_incoming_count": len(reverse_edges.get(file_off, [])),
                "shape": linear_summary(mz, md, file_off, args.max_bytes),
            }
            for file_off in sorted(leaves)[: args.lift_queue_limit]
        ],
        "entries": entries if args.include_entries else [],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=DEFAULT_BIN)
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--max-bytes", type=int, default=768)
    parser.add_argument("--sample-limit", type=int, default=8)
    parser.add_argument("--lift-queue-limit", type=int, default=24)
    parser.add_argument(
        "--include-entries",
        action="store_true",
        help="Include the full per-entry atlas; default emits counts and samples only",
    )
    args = parser.parse_args()

    atlas = build_atlas(args)
    print(json.dumps(atlas, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
