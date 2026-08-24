#!/usr/bin/env python3
"""Audit original alien-XDB callback stores and their recovered C ownership."""

from __future__ import annotations

import os
from pathlib import Path
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import argparse
from collections import Counter, defaultdict, deque
import csv
from dataclasses import dataclass
import hashlib
import io
import re

from capstone import CS_ARCH_X86, CS_GRP_CALL, CS_GRP_JUMP, CS_GRP_RET, CS_MODE_16, Cs
from capstone.x86_const import X86_OP_IMM, X86_OP_MEM


ROOT = Path(__file__).resolve().parents[2]
ROUTINE_INDEX = ROOT / "re" / "assembly" / "routine_index.tsv"
MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
XDB_DIR = ROOT / "output" / "_tmp_dat"

MODULE_SPECS = {
    "xdb_amer": ("amer.xdb", 0x3280),
    "xdb_croolis": ("croolis.xdb", 0x32F0),
    "xdb_scrut": ("scrut.xdb", 0x33B0),
}
MODULE_ORDER = {module: index for index, module in enumerate(MODULE_SPECS)}
MODULE_ROOTS = {
    "xdb_amer": frozenset(
        (
            0x0000, 0x0355, 0x0925, 0x0958, 0x09EF, 0x0B0F, 0x0B1F,
            0x1286, 0x12B3, 0x1414, 0x164C, 0x1692, 0x1B5F, 0x1B8F,
            0x1BEA, 0x1C34, 0x1DD6,
        )
    ),
    "xdb_croolis": frozenset(
        (
            0x0000, 0x036A, 0x0966, 0x0999, 0x0A30, 0x0B50, 0x0B60,
            0x12DE, 0x130B, 0x146C, 0x16A4, 0x1727, 0x1ACB, 0x1AFB,
            0x1B46, 0x1B85, 0x1D27,
        )
    ),
    "xdb_scrut": frozenset(
        (
            0x0000, 0x036A, 0x0966, 0x0999, 0x0A35, 0x0B55, 0x0B65,
            0x12CC, 0x12F9, 0x145A, 0x1692, 0x171B, 0x1B80, 0x1BB0,
            0x1BFB, 0x1C45, 0x1DE7,
        )
    ),
}

# context+0x36 is a union. These values are observed control states, not code.
CONTEXT_SCALAR_VALUES = frozenset((0x0000, 0x0001, 0x8001, 0xFFFF))
STATE_NULL_VALUES = frozenset((0x0000,))

METADATA_PATTERNS = {
    "module": re.compile(r"^; module: (\S+)$", re.MULTILINE),
    "artifact": re.compile(r"^; artifact: (.+)$", re.MULTILINE),
    "artifact_sha256": re.compile(
        r"^; artifact_sha256: ([0-9a-f]{64})$", re.MULTILINE
    ),
    "offset": re.compile(
        r"^; (?:overlay_offset|file_offset): 0x([0-9a-fA-F]+)$", re.MULTILINE
    ),
    "byte_count": re.compile(r"^; byte_count: (\d+)$", re.MULTILINE),
    "routine_sha256": re.compile(
        r"^; routine_bytes_sha256: ([0-9a-f]{64})$", re.MULTILINE
    ),
}
ROUTINE_ENTRY_PATTERN = re.compile(
    r"^; routine_entry: 0x([0-9a-fA-F]+)$", re.MULTILINE
)
ASM_ADDRESS_PATTERN = re.compile(r"^\s*([0-9a-fA-F]{1,8}):", re.MULTILINE)

TSV_FIELDS = (
    "module",
    "store_site",
    "writer_entry",
    "writer_function",
    "field",
    "classification",
    "original_value",
    "target_entry",
    "target_function",
    "asm_path",
    "source",
    "status",
)


@dataclass(frozen=True)
class AuditConfig:
    root: Path
    routine_index: Path
    manifest: Path
    xdb_dir: Path
    root_entries: dict[str, tuple[int, ...]] | None = None


@dataclass(frozen=True)
class AssemblyMetadata:
    start: int
    byte_count: int
    routine_entry: int

    @property
    def end(self) -> int:
        return self.start + self.byte_count


@dataclass(frozen=True)
class Owner:
    module: str
    entry: int
    row: dict[str, str]
    metadata: AssemblyMetadata
    instruction_addresses: tuple[int, ...]


@dataclass
class CallbackStore:
    module: str
    site: int
    field: str
    classification: str
    value: int
    writer_entry: int | None = None
    writer_function: str = ""
    target_function: str = ""
    asm_path: str = ""
    source: str = ""
    status: str = "unresolved"


@dataclass(frozen=True)
class AuditResult:
    stores: tuple[CallbackStore, ...]
    errors: tuple[str, ...]
    reachable_instruction_count: int


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_assembly_metadata(root: Path, path: Path) -> AssemblyMetadata:
    text = path.read_text(encoding="utf-8", errors="replace")
    values: dict[str, str] = {}
    for name, pattern in METADATA_PATTERNS.items():
        match = pattern.search(text)
        if match is None:
            raise ValueError(f"missing {name} metadata")
        values[name] = match.group(1)

    artifact = root / values["artifact"]
    if not artifact.is_file():
        raise ValueError(f"missing metadata artifact {values['artifact']}")
    blob = artifact.read_bytes()
    artifact_hash = sha256(blob)
    if artifact_hash != values["artifact_sha256"]:
        raise ValueError(
            f"artifact SHA-256 mismatch {artifact_hash} != {values['artifact_sha256']}"
        )

    start = int(values["offset"], 16)
    byte_count = int(values["byte_count"])
    routine_entry_match = ROUTINE_ENTRY_PATTERN.search(text)
    routine_entry = (
        int(routine_entry_match.group(1), 16)
        if routine_entry_match is not None
        else start
    )
    if start < 0 or byte_count <= 0 or start + byte_count > len(blob):
        raise ValueError(
            f"invalid byte interval 0x{start:06x}..0x{start + byte_count:06x}"
        )
    routine_hash = sha256(blob[start : start + byte_count])
    if routine_hash != values["routine_sha256"]:
        raise ValueError(
            f"routine SHA-256 mismatch {routine_hash} != {values['routine_sha256']}"
        )
    return AssemblyMetadata(start, byte_count, routine_entry)


def assembly_lists_entry(path: Path, entry: int) -> bool:
    return entry in assembly_instruction_addresses(path)


def assembly_instruction_addresses(path: Path) -> tuple[int, ...]:
    text = path.read_text(encoding="utf-8", errors="replace")
    return tuple(
        sorted({int(value, 16) for value in ASM_ADDRESS_PATTERN.findall(text)})
    )


def build_owner_inventory(
    config: AuditConfig,
    module: str,
    routine_rows: list[dict[str, str]],
) -> tuple[list[Owner], dict[int, list[dict[str, str]]], list[str]]:
    errors: list[str] = []
    by_entry: dict[int, list[dict[str, str]]] = defaultdict(list)
    owners: list[Owner] = []
    for row in routine_rows:
        if row["module"] != module:
            continue
        entry = int(row["entry"], 16)
        by_entry[entry].append(row)
        assembly = config.root / row["asm_path"]
        if not assembly.is_file():
            errors.append(
                f"{module}:0x{entry:06x}: missing assembly {row['asm_path']}"
            )
            continue
        try:
            metadata = parse_assembly_metadata(config.root, assembly)
        except ValueError as exc:
            errors.append(f"{module}:0x{entry:06x}: {row['asm_path']}: {exc}")
            continue
        instruction_addresses = assembly_instruction_addresses(assembly)
        if not metadata.start <= entry < metadata.end or entry not in instruction_addresses:
            errors.append(
                f"{module}:0x{entry:06x}: indexed entry is not listed inside the "
                f"assembly metadata interval 0x{metadata.start:06x}.."
                f"0x{metadata.end:06x}"
            )
            continue
        owners.append(Owner(module, entry, row, metadata, instruction_addresses))

    for entry, rows in by_entry.items():
        if len(rows) != 1:
            errors.append(
                f"{module}:0x{entry:06x}: {len(rows)} routine_index rows; expected one"
            )
    return owners, by_entry, errors


def direct_target(instruction) -> int | None:
    if len(instruction.operands) != 1:
        return None
    operand = instruction.operands[0]
    if operand.type != X86_OP_IMM:
        return None
    return operand.imm & 0xFFFF


def decode_reachable(
    image: bytes,
    roots: set[int],
    code_end: int,
) -> tuple[dict[int, object], list[str]]:
    decoder = Cs(CS_ARCH_X86, CS_MODE_16)
    decoder.detail = True
    queue = deque(sorted(root for root in roots if 0 <= root < code_end))
    instructions: dict[int, object] = {}
    errors: list[str] = []

    while queue:
        address = queue.popleft()
        if address in instructions:
            continue
        decoded = list(decoder.disasm(image[address : min(code_end, address + 15)], address, 1))
        if not decoded:
            errors.append(f"reachable address 0x{address:06x} does not decode")
            continue
        instruction = decoded[0]
        if instruction.address + instruction.size > code_end:
            errors.append(f"instruction at 0x{address:06x} crosses the code boundary")
            continue
        instructions[address] = instruction

        next_address = instruction.address + instruction.size
        is_ret = instruction.group(CS_GRP_RET) or instruction.mnemonic in (
            "iret",
            "iretd",
        )
        is_call = instruction.group(CS_GRP_CALL)
        is_jump = instruction.group(CS_GRP_JUMP)
        target = direct_target(instruction)

        if is_call:
            if target is not None and target < code_end:
                queue.append(target)
            if next_address < code_end:
                queue.append(next_address)
        elif is_jump:
            if target is not None and target < code_end:
                queue.append(target)
            if instruction.mnemonic not in ("jmp", "ljmp") and next_address < code_end:
                queue.append(next_address)
        elif not is_ret and next_address < code_end:
            queue.append(next_address)

    return instructions, errors


def immediate_callback_store(module: str, instruction) -> CallbackStore | None:
    if instruction.mnemonic != "mov" or len(instruction.operands) != 2:
        return None
    destination, source = instruction.operands
    if (
        destination.type != X86_OP_MEM
        or source.type != X86_OP_IMM
        or destination.size != 2
    ):
        return None

    displacement = destination.mem.disp & 0xFFFF
    value = source.imm & 0xFFFF
    if displacement == 0x000E:
        classification = "state_null" if value in STATE_NULL_VALUES else "pointer"
        return CallbackStore(module, instruction.address, "state+0x0e", classification, value)
    if displacement == 0x0036:
        classification = (
            "context_scalar" if value in CONTEXT_SCALAR_VALUES else "pointer"
        )
        return CallbackStore(
            module, instruction.address, "context+0x36", classification, value
        )
    return None


def discover_callback_closure(
    module: str,
    image: bytes,
    code_end: int,
    indexed_roots: set[int],
) -> tuple[list[CallbackStore], int, list[str]]:
    roots = set(indexed_roots)
    previous_roots: set[int] = set()
    final_instructions: dict[int, object] = {}
    errors: list[str] = []

    while roots != previous_roots:
        previous_roots = set(roots)
        instructions, decode_errors = decode_reachable(image, roots, code_end)
        final_instructions = instructions
        errors.extend(decode_errors)
        for instruction in instructions.values():
            store = immediate_callback_store(module, instruction)
            if store is None or store.classification != "pointer":
                continue
            if not 0 <= store.value < code_end:
                errors.append(
                    f"{module}:0x{store.site:06x}: unresolved immediate callback "
                    f"target 0x{store.value:04x} is outside code"
                )
                continue
            roots.add(store.value)

    stores = [
        store
        for instruction in final_instructions.values()
        if (store := immediate_callback_store(module, instruction)) is not None
    ]
    stores.sort(key=lambda store: (store.site, store.field, store.value))
    return stores, len(final_instructions), errors


def owner_for_site(owners: list[Owner], site: int) -> Owner | None:
    candidates = [
        owner for owner in owners if owner.metadata.start <= site < owner.metadata.end
    ]
    if not candidates:
        return None
    candidates.sort(key=lambda owner: (owner.metadata.start, -owner.metadata.byte_count))
    return candidates[-1]


def strip_c_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    return re.sub(r"//[^\n]*", "", text)


def c_function_defined(text: str, function: str) -> bool:
    pattern = re.compile(
        rf"\b{re.escape(function)}\s*\([^;{{}}]*\)\s*\{{", re.DOTALL
    )
    return pattern.search(strip_c_comments(text)) is not None


def c_callback_assignments(text: str) -> dict[str, Counter[str]]:
    source = strip_c_comments(text)
    result: dict[str, Counter[str]] = {
        "state+0x0e": Counter(),
        "context+0x36": Counter(),
    }
    for target in re.findall(r"->\s*callback\s*=\s*([A-Za-z_]\w*)\s*;", source):
        result["state+0x0e"][target] += 1
    for target in re.findall(
        r"->\s*control\s*\.\s*resume\s*=\s*([A-Za-z_]\w*)\s*;", source
    ):
        result["context+0x36"][target] += 1
    return result


def callback_abi_declared(header: str, function: str, field: str) -> bool:
    parameters = r"\[si\]\s+\[di\]" if field == "state+0x0e" else r"\[di\]"
    pattern = re.compile(
        rf"^\s*#pragma\s+aux\s+{re.escape(function)}\s+"
        rf"(?:\\\s*)?parm\s+{parameters}(?:\s|$)",
        re.MULTILINE,
    )
    return pattern.search(header) is not None


def resolve_target(
    config: AuditConfig,
    module: str,
    entry: int,
    index_by_entry: dict[int, list[dict[str, str]]],
    manifest_by_entry: dict[int, list[dict[str, str]]],
) -> tuple[dict[str, str] | None, list[str]]:
    errors: list[str] = []
    index_rows = index_by_entry.get(entry, [])
    if len(index_rows) != 1:
        errors.append(
            f"{module}:0x{entry:06x}: callback target has {len(index_rows)} "
            "exact routine_index owners; expected one"
        )
        return None, errors
    index_row = index_rows[0]
    assembly = config.root / index_row["asm_path"]
    if not assembly.is_file():
        errors.append(
            f"{module}:0x{entry:06x}: missing target assembly {index_row['asm_path']}"
        )
        return None, errors
    try:
        metadata = parse_assembly_metadata(config.root, assembly)
    except ValueError as exc:
        errors.append(f"{module}:0x{entry:06x}: target assembly metadata: {exc}")
        return None, errors
    if not metadata.start <= entry < metadata.end or not assembly_lists_entry(
        assembly, entry
    ):
        errors.append(
            f"{module}:0x{entry:06x}: callback target is not listed inside its "
            "hash-verified assembly metadata interval"
        )

    manifest_rows = manifest_by_entry.get(entry, [])
    if len(manifest_rows) != 1:
        errors.append(
            f"{module}:0x{entry:06x}: callback target has {len(manifest_rows)} "
            "exact XDB manifest owners; expected one"
        )
        return None, errors
    manifest_row = manifest_rows[0]
    if manifest_row["asm_path"] != index_row["asm_path"]:
        errors.append(
            f"{module}:0x{entry:06x}: manifest assembly {manifest_row['asm_path']} "
            f"differs from routine_index {index_row['asm_path']}"
        )
    source = config.manifest.parent / manifest_row["source"]
    if not source.is_file():
        errors.append(
            f"{module}:0x{entry:06x}: missing C source {manifest_row['source']}"
        )
    else:
        source_text = source.read_text(encoding="utf-8", errors="replace")
        if not c_function_defined(source_text, manifest_row["function"]):
            errors.append(
                f"{module}:0x{entry:06x}: C function {manifest_row['function']} "
                f"is not defined by {manifest_row['source']}"
            )
    return manifest_row, errors


def format_counter(counter: Counter[str]) -> str:
    return ", ".join(
        f"{name} x{count}" if count != 1 else name
        for name, count in sorted(counter.items())
    ) or "none"


def audit_module(config: AuditConfig, module: str) -> AuditResult:
    if module not in MODULE_SPECS:
        raise ValueError(f"unknown XDB module: {module}")
    filename, code_end = MODULE_SPECS[module]
    image_path = config.xdb_dir / filename
    if not image_path.is_file():
        return AuditResult((), (f"{module}: missing original XDB {image_path}",), 0)
    image = image_path.read_bytes()
    if len(image) < code_end:
        return AuditResult(
            (),
            (f"{module}: original XDB is shorter than code boundary 0x{code_end:x}",),
            0,
        )

    routine_rows = read_tsv(config.routine_index)
    manifest_rows = read_tsv(config.manifest)
    header_path = config.manifest.parent / "include" / "xdb_alien.h"
    if header_path.is_file():
        callback_abi_header = header_path.read_text(
            encoding="utf-8", errors="replace"
        )
        header_errors: list[str] = []
    else:
        callback_abi_header = ""
        header_errors = [f"{module}: missing callback ABI header {header_path}"]
    owners, index_by_entry, errors = build_owner_inventory(
        config, module, routine_rows
    )
    errors.extend(header_errors)
    manifest_by_entry: dict[int, list[dict[str, str]]] = defaultdict(list)
    prefix = module + ":"
    for row in manifest_rows:
        if row["entry"].startswith(prefix):
            manifest_by_entry[int(row["entry"].split(":", 1)[1], 16)].append(row)

    stores, reached, closure_errors = discover_callback_closure(
        module,
        image,
        code_end,
        set(
            config.root_entries[module]
            if config.root_entries is not None
            else MODULE_ROOTS[module]
        ),
    )
    errors.extend(closure_errors)
    target_cache: dict[int, dict[str, str] | None] = {}

    for store in stores:
        owner = owner_for_site(owners, store.site)
        if owner is None:
            errors.append(
                f"{module}:0x{store.site:06x}: callback store has no assembly owner"
            )
        else:
            store.writer_entry = owner.entry
            store.writer_function = owner.row["labels"]

        if store.classification != "pointer":
            store.status = store.classification
            continue
        if not 0 <= store.value < code_end:
            store.status = "unresolved_pointer"
            continue
        if store.value not in target_cache:
            target_row, target_errors = resolve_target(
                config,
                module,
                store.value,
                index_by_entry,
                manifest_by_entry,
            )
            target_cache[store.value] = target_row
            errors.extend(target_errors)
        target_row = target_cache[store.value]
        if target_row is None:
            store.status = "unresolved_pointer"
            continue
        store.target_function = target_row["function"]
        store.asm_path = target_row["asm_path"]
        store.source = target_row["source"]
        store.status = "owned_pointer"

    checked_abis: set[tuple[str, str]] = set()
    for store in stores:
        if store.status != "owned_pointer":
            continue
        key = (store.target_function, store.field)
        if key in checked_abis:
            continue
        checked_abis.add(key)
        if not callback_abi_declared(
            callback_abi_header, store.target_function, store.field
        ):
            errors.append(
                f"{module}: {store.target_function}: missing explicit "
                f"{store.field} register ABI pragma"
            )
            for matching in stores:
                if (
                    matching.target_function == store.target_function
                    and matching.field == store.field
                ):
                    matching.status = "callback_abi_missing"

    expected_by_writer: dict[tuple[int, str], Counter[str]] = defaultdict(Counter)
    stores_by_writer: dict[tuple[int, str], list[CallbackStore]] = defaultdict(list)
    for store in stores:
        if (
            store.classification == "pointer"
            and store.writer_entry is not None
            and store.target_function
        ):
            key = (store.writer_entry, store.field)
            expected_by_writer[key][store.target_function] += 1
            stores_by_writer[key].append(store)

    manifest_exact = {
        entry: rows[0] for entry, rows in manifest_by_entry.items() if len(rows) == 1
    }
    assignment_cache: dict[int, dict[str, Counter[str]]] = {}
    for (writer_entry, field), expected in sorted(expected_by_writer.items()):
        writer_row = manifest_exact.get(writer_entry)
        if writer_row is None:
            errors.append(
                f"{module}:0x{writer_entry:06x}: callback writer has no exact manifest owner"
            )
            continue
        if writer_entry not in assignment_cache:
            writer_source = config.manifest.parent / writer_row["source"]
            if not writer_source.is_file():
                errors.append(
                    f"{module}:0x{writer_entry:06x}: missing writer C source "
                    f"{writer_row['source']}"
                )
                continue
            writer_text = writer_source.read_text(encoding="utf-8", errors="replace")
            assignment_cache[writer_entry] = c_callback_assignments(writer_text)
        actual = assignment_cache[writer_entry][field]
        if actual != expected:
            errors.append(
                f"{module}:0x{writer_entry:06x}: {field} target mismatch; "
                f"original={format_counter(expected)}; C={format_counter(actual)}"
            )
            for store in stores_by_writer[(writer_entry, field)]:
                store.status = "source_target_mismatch"

    return AuditResult(
        tuple(stores),
        tuple(sorted(set(errors))),
        reached,
    )


def render_tsv(stores: list[CallbackStore] | tuple[CallbackStore, ...]) -> str:
    output = io.StringIO()
    writer = csv.DictWriter(
        output,
        fieldnames=TSV_FIELDS,
        delimiter="\t",
        lineterminator="\n",
    )
    writer.writeheader()
    ordered = sorted(
        stores,
        key=lambda store: (
            MODULE_ORDER.get(store.module, len(MODULE_ORDER)),
            store.site,
            store.field,
            store.value,
        ),
    )
    for store in ordered:
        writer.writerow(
            {
                "module": store.module,
                "store_site": f"0x{store.site:06x}",
                "writer_entry": (
                    f"0x{store.writer_entry:06x}"
                    if store.writer_entry is not None
                    else ""
                ),
                "writer_function": store.writer_function,
                "field": store.field,
                "classification": store.classification,
                "original_value": f"0x{store.value:04x}",
                "target_entry": (
                    f"0x{store.value:06x}" if store.classification == "pointer" else ""
                ),
                "target_function": store.target_function,
                "asm_path": store.asm_path,
                "source": store.source,
                "status": store.status,
            }
        )
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="run the audit")
    parser.add_argument(
        "--module",
        action="append",
        choices=("amer", "croolis", "scrut"),
        help="audit one module; repeatable; defaults to all three",
    )
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--routine-index", type=Path, default=ROUTINE_INDEX)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--xdb-dir", type=Path, default=XDB_DIR)
    parser.add_argument("--output", type=Path, help="write deterministic TSV here")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.check:
        raise SystemExit("--check is required")
    selected = args.module or ["amer", "croolis", "scrut"]
    modules = [f"xdb_{name}" for name in selected]
    config = AuditConfig(
        args.root.resolve(),
        args.routine_index.resolve(),
        args.manifest.resolve(),
        args.xdb_dir.resolve(),
    )

    results = [audit_module(config, module) for module in modules]
    stores = [store for result in results for store in result.stores]
    report = render_tsv(stores)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")

    errors = sorted({error for result in results for error in result.errors})
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    pointer_count = sum(store.classification == "pointer" for store in stores)
    reached = sum(result.reachable_instruction_count for result in results)
    print(
        f"OK: {len(stores)} immediate callback-field store(s), "
        f"{pointer_count} owned pointer edge(s), {reached} reachable instruction(s)",
        file=sys.stderr if args.output is None else sys.stdout,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
