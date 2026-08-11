#!/usr/bin/env python3
"""Export recovered native routines as assembly plus Borland C++ work files.

This is a decompilation workspace generator, not a decompiler.  It preserves a
one-to-one mapping between each currently recovered routine entry and:

* an assembly dump rooted at the recovered entrypoint
* a Borland C++ source file for the future faithful translation

The C++ side deliberately fails to compile for untranslated routines.  That is
intentional: a compile-clean no-op body would destroy the evidence trail and make
later DOSBox runs meaningless.
"""

from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
import os
import re
import shutil
import struct
import sys
from pathlib import Path
from typing import Iterable

_HERE_STR = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE_STR
]

from dataclasses import dataclass, field

import capstone
from capstone.x86_const import X86_OP_IMM

_HERE = Path(_HERE_STR)
sys.path.insert(0, str(_HERE))

from indirect_dispatch_atlas import STATIC_TABLES  # noqa: E402
from mzfile import MZ, load_labels  # noqa: E402

sys.path[:] = [
    path for path in sys.path if Path(path or os.curdir).resolve() != _HERE
]


RE_ROOT = _HERE.parent
PROJECT_ROOT = RE_ROOT.parent
DEFAULT_BLOODPRG = RE_ROOT / "bin" / "BLOODPRG.EXE"
DEFAULT_GRAPH = RE_ROOT / "func_graph.json"
DEFAULT_XDB_DIR = PROJECT_ROOT / "output" / "_tmp_dat"
DEFAULT_ASM_OUT = RE_ROOT / "assembly"
DEFAULT_CPP_OUT = RE_ROOT / "borland"
DEFAULT_MANIFEST = RE_ROOT / "routine_recovery_manifest.json"

RETURN_MNEMONICS = {"ret", "retf"}

ALIEN_XDBS = {"amer", "croolis", "scrut"}

MANU3_CODE_SEEDS = {
    0x0000: "manu3 external far-call API entry",
    0x0121: "manu3 self-relocation/init entry",
    0x0150: "manu3 no-cursor per-frame entry",
    0x017C: "manu3 selector wrapper entry",
    0x0181: "manu3 animation selector",
    0x019B: "manu3 tween stepper",
    0x01DF: "manu3 tween constructor",
    0x0270: "manu3 matrix builder",
    0x0477: "manu3 transform routine",
    0x0549: "manu3 entity projector",
    0x06F6: "manu3 face builder",
    0x0700: "manu3 face bucket sorter",
    0x0775: "manu3 span renderer init",
    0x0848: "manu3 span setup region",
    0x0849: "manu3 span insertion routine",
    0x0C2A: "manu3 affine fill routine",
    0x0D7D: "manu3 face activation routine",
    0x0D93: "manu3 gradient setup routine",
}


def h(n: int, width: int = 0) -> str:
    if width:
        return f"0x{n:0{width}x}"
    return f"0x{n:x}"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def slug(text: str, fallback: str = "routine", limit: int = 64) -> str:
    text = text.lower()
    text = re.sub(r"[^a-z0-9_]+", "_", text)
    text = re.sub(r"_+", "_", text).strip("_")
    if not text:
        text = fallback
    return text[:limit].strip("_") or fallback


def rel(path: Path) -> str:
    return str(path.relative_to(PROJECT_ROOT))


def make_md() -> capstone.Cs:
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True
    md.skipdata = False
    return md


def immediate_operand(insn: capstone.CsInsn) -> int | None:
    if not insn.operands:
        return None
    op = insn.operands[0]
    if op.type != X86_OP_IMM:
        return None
    return int(op.imm)


def load_graph(path: Path) -> dict[str, object]:
    if not path.exists():
        return {"funcs": [], "leaves": [], "callgraph": {}, "indirect": []}
    with path.open() as fh:
        return json.load(fh)


def direct_far_transfers(mz: MZ) -> list[dict[str, int | str]]:
    transfers: list[dict[str, int | str]] = []
    for opcode, kind in ((0x9A, "call"), (0xEA, "jmp")):
        i = mz.header_size
        while i < mz.image_total - 5:
            if mz.data[i] == opcode:
                off, seg = struct.unpack_from("<HH", mz.data, i + 1)
                seg_operand_image = mz.file_to_image(i + 3)
                if seg_operand_image in mz.reloc_image_offsets:
                    transfers.append(
                        {
                            "kind": kind,
                            "site": i,
                            "target_segment": seg,
                            "target_offset": off,
                            "target_file": mz.segoff_to_file(seg, off),
                        }
                    )
            i += 1
    return sorted(transfers, key=lambda x: (int(x["site"]), str(x["kind"])))


def segment_bases_from_relocs(mz: MZ, transfers: Iterable[dict[str, int | str]]) -> list[int]:
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


def static_table_targets(mz: MZ) -> dict[int, list[str]]:
    targets: dict[int, list[str]] = collections.defaultdict(list)
    for table in STATIC_TABLES:
        table_file = int(table["table_file_offset"])
        target_base = int(table["target_base_file_offset"])
        index_base = int(table["index_base"])
        index_prefix = str(table["index_prefix"])
        for idx in range(int(table["entry_count"])):
            raw = struct.unpack_from("<H", mz.data, table_file + idx * 2)[0]
            target = target_base + raw
            selector = index_base + idx
            if index_prefix in {"opcode", "byte"}:
                key = f"{index_prefix}_0x{selector:02x}"
            else:
                key = f"{index_prefix}_{selector}"
            targets[target].append(f"{table['name']}:{key}")
    return targets


@dataclass
class LabelInfo:
    name: str
    comment: str = ""


@dataclass
class Routine:
    module: str
    artifact_path: Path
    entry: int
    address_kind: str
    group: str
    provenance: set[str] = field(default_factory=set)
    labels: list[LabelInfo] = field(default_factory=list)
    incoming: list[str] = field(default_factory=list)
    seg_off: str | None = None
    instructions: list[capstone.CsInsn] = field(default_factory=list)
    direct_callees: set[int] = field(default_factory=set)
    indirect_calls: list[str] = field(default_factory=list)
    tail_jumps: set[int] = field(default_factory=set)
    boundary_reason: str = "not_decoded"
    terminal: str | None = None
    first_bytes: str = ""
    byte_count: int = 0
    cxx_status: str = "untranslated"
    cxx_reason: str = ""
    asm_path: Path | None = None
    cpp_path: Path | None = None

    @property
    def label_slug(self) -> str:
        if self.labels:
            return slug(self.labels[0].name)
        return "routine"

    @property
    def func_name(self) -> str:
        mod = slug(self.module)
        return f"cb_{mod}_{self.entry:06x}_{self.label_slug}"

    @property
    def file_stem(self) -> str:
        return f"func_{self.entry:06x}_{self.label_slug}"


def add_label(labels: list[LabelInfo], name: str, comment: str = "") -> None:
    if not name:
        return
    for label in labels:
        if label.name == name and label.comment == comment:
            return
    labels.append(LabelInfo(name=name, comment=comment))


def parse_xdb_labels(labels_csv: Path) -> dict[str, dict[int, list[LabelInfo]]]:
    by_module: dict[str, dict[int, list[LabelInfo]]] = collections.defaultdict(
        lambda: collections.defaultdict(list)
    )
    with labels_csv.open(newline="") as fh:
        for row in csv.reader(fh):
            if not row or row[0].strip().startswith("#"):
                continue
            addr = row[0].strip()
            if not addr.lower().startswith("xdb:"):
                continue
            name = row[1].strip() if len(row) > 1 else ""
            comment = row[2].strip() if len(row) > 2 else ""
            parts = addr.split(":")
            if len(parts) == 2:
                module = "manu3"
                off_s = parts[1]
            elif len(parts) == 3:
                module = parts[1].lower()
                off_s = parts[2]
            else:
                continue
            try:
                off = int(off_s, 16)
            except ValueError:
                continue
            by_module[module][off].append(LabelInfo(name=name, comment=comment))
    return {module: dict(offsets) for module, offsets in by_module.items()}


def decode_routine(
    routine: Routine,
    data: bytes,
    max_bytes: int,
    protected_entries: set[int] | None = None,
) -> None:
    if not (0 <= routine.entry < len(data)):
        routine.boundary_reason = "entry_out_of_range"
        return

    md = make_md()
    end = min(len(data), routine.entry + max_bytes)
    window = data[routine.entry:end]
    routine.first_bytes = window[:16].hex(" ")

    protected_entries = protected_entries or set()
    blocks = collections.deque([routine.entry])
    visited_blocks: set[int] = set()
    insn_by_addr: dict[int, capstone.CsInsn] = {}
    terminals: list[str] = []
    decode_stops: list[str] = []

    def enqueue(target: int) -> None:
        if not (routine.entry <= target < end):
            return
        if target in protected_entries and target != routine.entry:
            routine.tail_jumps.add(target)
            return
        if target not in visited_blocks:
            blocks.append(target)

    while blocks:
        block = blocks.popleft()
        if block in visited_blocks:
            continue
        visited_blocks.add(block)
        pos = block
        while pos < end:
            if pos in insn_by_addr:
                break
            decoded = list(md.disasm(data[pos:end], pos, count=1))
            if not decoded or decoded[0].address != pos:
                decode_stops.append(f"decode_stop_at_{h(pos)}")
                break
            insn = decoded[0]
            insn_by_addr[insn.address] = insn
            next_pos = insn.address + insn.size

            if insn.mnemonic == "call":
                target = immediate_operand(insn)
                if target is not None and 0 <= target < len(data):
                    routine.direct_callees.add(target)
                else:
                    routine.indirect_calls.append(
                        f"{h(insn.address)}: {insn.mnemonic} {insn.op_str}".strip()
                    )
            elif insn.mnemonic == "lcall":
                routine.indirect_calls.append(
                    f"{h(insn.address)}: {insn.mnemonic} {insn.op_str}".strip()
                )

            if insn.mnemonic in {"jmp", "ljmp"}:
                target = immediate_operand(insn)
                if target is not None and 0 <= target < len(data):
                    if routine.entry <= target < end and not (
                        target in protected_entries and target != routine.entry
                    ):
                        enqueue(target)
                    else:
                        routine.tail_jumps.add(target)
                terminals.append(f"{insn.mnemonic} {insn.op_str}".strip())
                break

            if insn.mnemonic in {"ret", "retf", "iret"}:
                terminals.append(insn.mnemonic)
                break

            if insn.group(capstone.CS_GRP_JUMP):
                target = immediate_operand(insn)
                if target is not None:
                    enqueue(target)
                enqueue(next_pos)
                break

            if next_pos in protected_entries and next_pos != routine.entry:
                break

            pos = next_pos

    routine.instructions = [insn_by_addr[addr] for addr in sorted(insn_by_addr)]
    if routine.instructions:
        last_end = max(insn.address + insn.size for insn in routine.instructions)
        routine.byte_count = max(0, last_end - routine.entry)
    else:
        routine.byte_count = 0

    terminal_counts = collections.Counter(terminals)
    if terminal_counts:
        routine.terminal = ", ".join(
            f"{term}:{count}" for term, count in sorted(terminal_counts.items())
        )
    else:
        routine.terminal = None
    if decode_stops:
        routine.boundary_reason = ",".join(sorted(set(decode_stops))[:4])
    elif blocks:
        routine.boundary_reason = f"cfg_incomplete_blocks_{len(visited_blocks)}"
    elif routine.byte_count >= max_bytes:
        routine.boundary_reason = f"max_bytes_{max_bytes}"
    else:
        routine.boundary_reason = (
            f"cfg_blocks_{len(visited_blocks)}_terminals_{sum(terminal_counts.values())}"
        )
    classify_cxx_translation(routine)


def classify_cxx_translation(routine: Routine) -> None:
    insns = routine.instructions
    if len(insns) == 1 and insns[0].mnemonic in RETURN_MNEMONICS:
        routine.cxx_status = "translated_empty_return"
        routine.cxx_reason = f"single {insns[0].mnemonic} instruction"
        return
    if not insns:
        routine.cxx_status = "blocked_decode"
        routine.cxx_reason = routine.boundary_reason
        return
    routine.cxx_status = "untranslated"
    routine.cxx_reason = "requires human/mechanical translation from assembly"


def bloodprg_routines(exe: Path, graph_path: Path) -> tuple[list[Routine], dict[str, object]]:
    mz = MZ(str(exe))
    graph = load_graph(graph_path)
    _, file_labels = load_labels()
    transfers = direct_far_transfers(mz)
    bases = segment_bases_from_relocs(mz, transfers)
    by_far_target: dict[int, list[dict[str, int | str]]] = collections.defaultdict(list)
    for transfer in transfers:
        by_far_target[int(transfer["target_file"])].append(transfer)

    static_targets = static_table_targets(mz)
    graph_funcs = {int(x) for x in graph.get("funcs", [])}
    far_targets = set(by_far_target)
    all_entries = sorted(graph_funcs | far_targets | set(static_targets) | {mz.entry_file})

    routines = []
    for entry in all_entries:
        if not (0 <= entry < len(mz.data)):
            continue
        seg, off = segment_for_file(mz, bases, entry)
        routine = Routine(
            module="bloodprg",
            artifact_path=exe,
            entry=entry,
            address_kind="file_offset",
            group=f"seg_{seg:04x}",
            seg_off=f"{seg:04x}:{off:04x}",
        )
        if entry == mz.entry_file:
            routine.provenance.add("mz_entry")
        if entry in graph_funcs:
            routine.provenance.add("recursive_graph")
        if entry in by_far_target:
            routine.provenance.add("relocation_proven_far_transfer_target")
            for transfer in by_far_target[entry]:
                routine.incoming.append(
                    f"{transfer['kind']}@{h(int(transfer['site']), 6)}"
                    f"->{int(transfer['target_segment']):04x}:{int(transfer['target_offset']):04x}"
                )
        if entry in static_targets:
            routine.provenance.add("static_dispatch_table_target")
            routine.incoming.extend(static_targets[entry])
        label = file_labels.get(entry)
        if label:
            add_label(routine.labels, label[0], label[1])
        routines.append(routine)

    metadata = {
        "path": rel(exe),
        "sha256": sha256_file(exe),
        "entry_count": len(routines),
        "grouping_evidence": (
            "Grouped by recovered MZ relative segment base. This is loader/linkage "
            "evidence, not proof of original object-file translation units."
        ),
    }
    return routines, metadata


def alien_delta_pointer(data: bytes) -> int | None:
    # Entry bytes: 8c c8; 2e 03 06 <disp16>; mov ds,ax ...
    pat = bytes.fromhex("8c c8 2e 03 06")
    idx = data.find(pat, 0, min(len(data), 0x40))
    if idx < 0 or idx + len(pat) + 2 > len(data):
        return None
    return struct.unpack_from("<H", data, idx + len(pat))[0]


def alien_method_table_entries(data: bytes) -> tuple[int | None, list[tuple[int, int, int]]]:
    ptr = alien_delta_pointer(data)
    if ptr is None or ptr + 2 > len(data):
        return None, []
    delta = struct.unpack_from("<H", data, ptr)[0]
    table = delta * 16 + 0x103A
    if table < 0 or table + 2 > len(data):
        return table, []
    entries: list[tuple[int, int, int]] = []
    for idx in range(64):
        off = table + idx * 2
        if off + 2 > len(data):
            break
        target = struct.unpack_from("<H", data, off)[0]
        if target in {0x0000, 0xFFFF}:
            if idx >= 15:
                break
            continue
        if 0 <= target < len(data):
            entries.append((target, idx, off))
    return table, entries


def xdb_seed_entries(module: str, data: bytes) -> dict[int, list[str]]:
    seeds: dict[int, list[str]] = collections.defaultdict(list)
    seeds[0x0000].append("overlay_entry_0")
    if module in ALIEN_XDBS:
        seeds[0x00A3].append("alien_body_entry_00a3")
        table, entries = alien_method_table_entries(data)
        for target, idx, slot_off in entries:
            if table is None:
                seeds[target].append("alien_method_table_103a")
            else:
                seeds[target].append(
                    f"alien_method_table_103a_slot_{idx}@{h(slot_off)}"
                )
    if module == "manu3":
        for off, reason in MANU3_CODE_SEEDS.items():
            seeds[off].append(reason)
    return dict(seeds)


def xdb_routines(xdb_paths: Iterable[Path], max_decode_bytes: int) -> tuple[list[Routine], dict[str, object]]:
    xdb_labels = parse_xdb_labels(RE_ROOT / "labels.csv")
    all_routines: list[Routine] = []
    metadata: dict[str, object] = {}

    for path in sorted(xdb_paths):
        module = path.stem.lower()
        data = path.read_bytes()
        labels = xdb_labels.get(module, {})
        seeds = xdb_seed_entries(module, data)

        for off, label_infos in labels.items():
            for label in label_infos:
                # Labels are attached to known code entries but do not create code
                # by themselves. Several XDB labels intentionally name data cells.
                if off in seeds:
                    seeds[off].append(f"label:{label.name}")

        queue = collections.deque(sorted(seeds))
        discovered: dict[int, set[str]] = {
            off: set(reasons) for off, reasons in seeds.items()
        }
        decoded_once: set[int] = set()

        while queue:
            entry = queue.popleft()
            if entry in decoded_once or not (0 <= entry < len(data)):
                continue
            decoded_once.add(entry)
            temp = Routine(
                module=f"xdb_{module}",
                artifact_path=path,
                entry=entry,
                address_kind="overlay_offset",
                group="pending",
            )
            decode_routine(temp, data, max_decode_bytes)
            for target in sorted(temp.direct_callees):
                if target not in discovered:
                    discovered[target] = {f"direct_call_from_{h(entry)}"}
                    queue.append(target)
                else:
                    discovered[target].add(f"direct_call_from_{h(entry)}")

        for entry in sorted(discovered):
            if not (0 <= entry < len(data)):
                continue
            provenance = discovered[entry]
            group = "direct_calls"
            if any("method_table" in p for p in provenance):
                group = "method_table_103a"
            if any("overlay_entry" in p or "body_entry" in p for p in provenance):
                group = "entry"
            if module == "manu3" and entry in MANU3_CODE_SEEDS:
                group = "manu3_labeled"
            routine = Routine(
                module=f"xdb_{module}",
                artifact_path=path,
                entry=entry,
                address_kind="overlay_offset",
                group=group,
                provenance=set(provenance),
            )
            for label in labels.get(entry, []):
                add_label(routine.labels, label.name, label.comment)
            all_routines.append(routine)

        metadata[module] = {
            "path": rel(path),
            "sha256": sha256_file(path),
            "entry_count": len(discovered),
            "grouping_evidence": (
                "Grouped by overlay entry/API seeds, alien method table entries, "
                "manu3 hand-labeled code seeds, and recursively discovered direct "
                "near calls. No original object-file translation-unit boundary has "
                "been proven for XDB overlays."
            ),
        }

    return all_routines, metadata


def decode_all(routines: Iterable[Routine], data_by_path: dict[Path, bytes], max_bytes: int) -> None:
    routines = list(routines)
    entries_by_path: dict[Path, set[int]] = collections.defaultdict(set)
    for routine in routines:
        entries_by_path[routine.artifact_path].add(routine.entry)
    for routine in routines:
        decode_routine(
            routine,
            data_by_path[routine.artifact_path],
            max_bytes,
            protected_entries=entries_by_path[routine.artifact_path],
        )


def routine_qualifier(routine: Routine) -> str:
    if any(insn.mnemonic == "retf" for insn in routine.instructions):
        return "CB_FAR"
    if "relocation_proven_far_transfer_target" in routine.provenance:
        return "CB_FAR"
    if routine.entry == 0 and routine.module.startswith("xdb_"):
        return "CB_FAR"
    return "CB_NEAR"


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def asm_text(routine: Routine, data: bytes) -> str:
    lines = [
        "; Commander Blood recovered routine assembly",
        f"; module: {routine.module}",
        f"; artifact: {rel(routine.artifact_path)}",
        f"; artifact_sha256: {sha256_file(routine.artifact_path)}",
        f"; {routine.address_kind}: {h(routine.entry, 6)}",
    ]
    if routine.seg_off:
        lines.append(f"; seg_off: {routine.seg_off}")
    lines.extend(
        [
            f"; group: {routine.group}",
            f"; provenance: {', '.join(sorted(routine.provenance)) or 'unknown'}",
        ]
    )
    if routine.labels:
        for label in routine.labels:
            lines.append(f"; label: {label.name}")
            if label.comment:
                lines.append(f"; label_comment: {label.comment}")
    if routine.incoming:
        for incoming in sorted(routine.incoming):
            lines.append(f"; incoming: {incoming}")
    lines.extend(
        [
            f"; byte_count: {routine.byte_count}",
            f"; boundary: {routine.boundary_reason}",
            f"; terminal: {routine.terminal or 'none'}",
            f"; direct_callees: {', '.join(h(x, 6) for x in sorted(routine.direct_callees)) or 'none'}",
            f"; indirect_calls: {len(routine.indirect_calls)}",
        ]
    )
    if routine.cpp_path:
        lines.append(f"; cxx_source: {rel(routine.cpp_path)}")
    if routine.byte_count:
        blob = data[routine.entry : routine.entry + routine.byte_count]
        lines.append(f"; routine_bytes_sha256: {sha256_bytes(blob)}")
    lines.append("")

    prev_end: int | None = None
    for insn in routine.instructions:
        if prev_end is not None and insn.address != prev_end:
            lines.append(f"; -- non-contiguous block: next {h(insn.address, 6)} --")
        byte_s = insn.bytes.hex(" ").upper()
        op_s = f" {insn.op_str}" if insn.op_str else ""
        lines.append(f"{insn.address:06X}:  {byte_s:<28} {insn.mnemonic:<8}{op_s}")
        prev_end = insn.address + insn.size
    if not routine.instructions:
        lines.append("; no instructions decoded")
    lines.append("")
    return "\n".join(lines)


def cpp_text(routine: Routine) -> str:
    asm_path = rel(routine.asm_path) if routine.asm_path else ""
    qualifier = routine_qualifier(routine)
    lines = [
        "// Commander Blood Borland C++ translation unit",
        f"// module: {routine.module}",
        f"// {routine.address_kind}: {h(routine.entry, 6)}",
        f"// assembly: {asm_path}",
        f"// provenance: {', '.join(sorted(routine.provenance)) or 'unknown'}",
        f"// status: {routine.cxx_status}",
        f"// reason: {routine.cxx_reason}",
        "",
        '#include "recovered.hpp"',
        "",
    ]
    for label in routine.labels:
        lines.append(f"// label: {label.name}")
    if routine.labels:
        lines.append("")

    lines.append(f'extern "C" void {qualifier} {routine.func_name}(void)')
    lines.append("{")
    if routine.cxx_status == "translated_empty_return":
        lines.append("    return;")
    else:
        lines.append(
            f'#error "Untranslated routine {routine.module}:{h(routine.entry, 6)}; see {asm_path}"'
        )
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def header_text() -> str:
    return """#ifndef CB_RECOVERED_HPP
#define CB_RECOVERED_HPP

#if defined(__BORLANDC__)
#define CB_NEAR near
#define CB_FAR far
#else
#define CB_NEAR
#define CB_FAR
#endif

typedef unsigned char cb_u8;
typedef signed char cb_i8;
typedef unsigned short cb_u16;
typedef signed short cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

#endif
"""


def readme_assembly_text(counts: dict[str, int]) -> str:
    lines = [
        "# Recovered Assembly Dumps",
        "",
        "Generated by `python3 re/tools/export_routine_sources.py --clean`.",
        "",
        "Each `.asm` file is rooted at a recovered routine entrypoint and includes",
        "the provenance that made that entrypoint eligible. BLOODPRG routines are",
        "grouped by recovered MZ relative segment. XDB routines are grouped by",
        "entry/API seeds, method tables, manu3 labeled code seeds, and direct-call",
        "discovery. These groups are not claimed to be original compiler",
        "translation units unless future evidence proves that.",
        "",
        "Routine counts:",
        "",
    ]
    for module, count in sorted(counts.items()):
        lines.append(f"- `{module}`: {count}")
    lines.append("")
    return "\n".join(lines)


def readme_borland_text(counts: dict[str, int], translated: int) -> str:
    return f"""# Borland C++ Translation Workspace

Generated by `python3 re/tools/export_routine_sources.py --clean`.

The current choice is Borland C++ source (`.cpp`) because the overlays have
C++-shaped method-table dispatch, while still using plain `extern "C"` function
names for controllable linkage. This is a working choice, not proof of the
original compiler.

There is one C++ source file per recovered assembly routine. Files marked
`translated_empty_return` contain real minimal C++ for routines that are exactly
a single `ret` or `retf`. Every other file deliberately contains `#error` until
that routine has been translated from its assembly dump. That stop gate prevents
untranslated routines from silently compiling as no-ops.

Recovered routine files: {sum(counts.values())}
Mechanically translated files: {translated}
"""


def clean_generated(path: Path) -> None:
    marker = path / ".generated_by_export_routine_sources"
    if not path.exists():
        return
    if marker.exists():
        shutil.rmtree(path)
        return
    if any(path.iterdir()):
        raise SystemExit(f"{path} exists and is not marked as generated; refusing to remove it")
    path.rmdir()


def prepare_output(path: Path, clean: bool) -> None:
    if clean:
        clean_generated(path)
    path.mkdir(parents=True, exist_ok=True)
    (path / ".generated_by_export_routine_sources").write_text(
        "generated by re/tools/export_routine_sources.py\n", encoding="utf-8"
    )


def write_outputs(
    routines: list[Routine],
    data_by_path: dict[Path, bytes],
    asm_out: Path,
    cpp_out: Path,
    manifest_path: Path,
    metadata: dict[str, object],
    clean: bool,
) -> dict[str, object]:
    prepare_output(asm_out, clean)
    prepare_output(cpp_out, clean)
    write_text(cpp_out / "include" / "recovered.hpp", header_text())

    counts = collections.Counter(r.module for r in routines)
    translated = 0
    index_rows = []
    manifest_entries = []

    for routine in sorted(routines, key=lambda r: (r.module, r.group, r.entry)):
        module_dir = "bloodprg" if routine.module == "bloodprg" else f"xdb/{routine.module[4:]}"
        asm_path = asm_out / module_dir / routine.group / f"{routine.file_stem}.asm"
        cpp_path = cpp_out / module_dir / routine.group / f"{routine.file_stem}.cpp"
        routine.asm_path = asm_path
        routine.cpp_path = cpp_path
        if routine.cxx_status.startswith("translated"):
            translated += 1

        write_text(asm_path, asm_text(routine, data_by_path[routine.artifact_path]))
        write_text(cpp_path, cpp_text(routine))

        index_rows.append(
            [
                routine.module,
                h(routine.entry, 6),
                routine.group,
                " | ".join(sorted(routine.provenance)),
                " | ".join(label.name for label in routine.labels),
                rel(asm_path),
                rel(cpp_path),
                routine.cxx_status,
                routine.boundary_reason,
            ]
        )
        manifest_entries.append(
            {
                "module": routine.module,
                "entry": h(routine.entry, 6),
                "address_kind": routine.address_kind,
                "seg_off": routine.seg_off,
                "group": routine.group,
                "provenance": sorted(routine.provenance),
                "labels": [
                    {"name": label.name, "comment": label.comment}
                    for label in routine.labels
                ],
                "incoming": sorted(routine.incoming),
                "byte_count": routine.byte_count,
                "boundary_reason": routine.boundary_reason,
                "terminal": routine.terminal,
                "direct_callees": [h(x, 6) for x in sorted(routine.direct_callees)],
                "indirect_call_count": len(routine.indirect_calls),
                "asm_path": rel(asm_path),
                "cpp_path": rel(cpp_path),
                "cxx_status": routine.cxx_status,
                "cxx_reason": routine.cxx_reason,
            }
        )

    for root in (asm_out, cpp_out):
        index_path = root / "routine_index.tsv"
        with index_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.writer(fh, delimiter="\t")
            writer.writerow(
                [
                    "module",
                    "entry",
                    "group",
                    "provenance",
                    "labels",
                    "asm_path",
                    "cpp_path",
                    "cxx_status",
                    "boundary",
                ]
            )
            writer.writerows(index_rows)

    write_text(asm_out / "README.md", readme_assembly_text(dict(counts)))
    write_text(cpp_out / "README.md", readme_borland_text(dict(counts), translated))

    manifest = {
        "generator": "re/tools/export_routine_sources.py",
        "routine_count": len(routines),
        "module_counts": dict(sorted(counts.items())),
        "translated_count": translated,
        "untranslated_count": len(routines) - translated,
        "metadata": metadata,
        "entries": manifest_entries,
    }
    write_text(manifest_path, json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bloodprg", type=Path, default=DEFAULT_BLOODPRG)
    parser.add_argument("--graph", type=Path, default=DEFAULT_GRAPH)
    parser.add_argument("--xdb-dir", type=Path, default=DEFAULT_XDB_DIR)
    parser.add_argument("--asm-out", type=Path, default=DEFAULT_ASM_OUT)
    parser.add_argument("--cpp-out", type=Path, default=DEFAULT_CPP_OUT)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--max-bytes", type=int, default=8192)
    parser.add_argument("--clean", action="store_true")
    args = parser.parse_args()

    xdb_paths = [args.xdb_dir / f"{name}.xdb" for name in ("amer", "croolis", "manu3", "scrut")]
    missing = [path for path in [args.bloodprg, args.graph, *xdb_paths] if not path.exists()]
    if missing:
        raise SystemExit("missing inputs:\n" + "\n".join(str(path) for path in missing))

    blood, blood_meta = bloodprg_routines(args.bloodprg, args.graph)
    xdb, xdb_meta = xdb_routines(xdb_paths, args.max_bytes)
    routines = blood + xdb
    data_by_path = {path: path.read_bytes() for path in {r.artifact_path for r in routines}}
    decode_all(routines, data_by_path, args.max_bytes)

    manifest = write_outputs(
        routines,
        data_by_path,
        args.asm_out,
        args.cpp_out,
        args.manifest,
        clean=args.clean,
        metadata={"bloodprg": blood_meta, "xdb": xdb_meta},
    )
    print(
        json.dumps(
            {
                "routine_count": manifest["routine_count"],
                "module_counts": manifest["module_counts"],
                "translated_count": manifest["translated_count"],
                "untranslated_count": manifest["untranslated_count"],
                "asm_out": str(args.asm_out),
                "cpp_out": str(args.cpp_out),
                "manifest": str(args.manifest),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
