#!/usr/bin/env python3
"""Validate segment provenance for every symbolic data access in BLOODPRG.

The final executable's data placement audit proves where symbols were linked,
but it does not prove that the instruction accessing a symbol has the matching
segment register loaded.  This audit reads the Watcom object listings so that
relocation symbol names are still available, propagates segment provenance
through each routine's control-flow graph, and checks every data reference
against the canonical data-layout manifest.

Definite owner mismatches fail the audit.  Accesses whose segment provenance
cannot yet be proved are retained as explicit ``unproven`` findings instead of
being silently accepted.
"""
from __future__ import annotations

import argparse
import csv
import os
import re
import subprocess
import sys
import tempfile
import time
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, replace
from pathlib import Path

import capstone
from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]

SEGMENT_REGISTERS = ("cs", "ds", "es", "fs", "gs", "ss")
GENERAL_REGISTERS = (
    "ax", "bx", "cx", "dx", "si", "di", "bp", "sp",
    "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp",
)
OWNER_BY_SEGMENT_NAME = {
    "GAME_DATA": "GAME_DATA",
    "FS_DATA": "FS_DATA",
    "_CODE": "CODE",
    "STACK": "STACK",
}
UNKNOWN = "unknown"
ZERO = "zero"

INSTRUCTION_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{4,8})\s+"
    r"(?P<bytes>(?:[0-9A-Fa-f]{2}(?:\s+|$))+)(?P<text>.*?)\s*$"
)
INSTRUCTION_CONTINUATION_ROW = re.compile(r"^\s+(?P<text>\S.*?)\s*$")
LABEL_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{4,8})\s+(?P<label>[A-Za-z_$?][\w$?@]*):\s*$"
)
ROUTINE_SIZE_ROW = re.compile(
    r"^Routine Size:\s+(?P<size>\d+) bytes,\s+Routine Base:\s+"
    r"\S+\s+\+\s+(?P<offset>[0-9A-Fa-f]{4,8})\s*$"
)
SYMBOL_TOKEN = re.compile(r"_[A-Za-z_$?][\w$?@]*")
LOCAL_OPERAND = re.compile(
    r"(?:(?:byte|word|dword|qword)\s+ptr\s+)?"
    r"(?P<disp>-?0x[0-9a-f]+|[+-]?\d+)\[(?:e)?bp\]$",
    re.IGNORECASE,
)
SECTION_ROW = re.compile(
    r"^(?P<name>\S+)\s+(?P<class>\S+)\s+(?P<group>\S+)\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})\s+"
    r"(?P<size>[0-9A-Fa-f]{8})$"
)


@dataclass(frozen=True)
class ListingInstruction:
    offset: int
    data: bytes
    text: str


@dataclass(frozen=True)
class Listing:
    object_path: Path
    instructions: tuple[ListingInstruction, ...]
    labels: dict[str, int]
    jump_tables: dict[str, tuple[int, ...]]
    entrypoints: tuple[int, ...] = ()
    executable_ranges: tuple[tuple[int, int], ...] = ()


@dataclass(frozen=True)
class AbstractState:
    registers: tuple[tuple[str, str], ...]
    locals: tuple[tuple[int, str], ...] = ()
    stack: tuple[str, ...] = ()

    def register(self, name: str) -> str:
        canonical = canonical_register(name)
        return dict(self.registers).get(canonical, UNKNOWN)

    def local(self, offset: int) -> str:
        return dict(self.locals).get(offset, UNKNOWN)

    def with_register(self, name: str, value: str) -> AbstractState:
        canonical = canonical_register(name)
        values = dict(self.registers)
        values[canonical] = value
        return replace(self, registers=tuple(sorted(values.items())))

    def with_local(self, offset: int, value: str) -> AbstractState:
        values = dict(self.locals)
        values[offset] = value
        return replace(self, locals=tuple(sorted(values.items())))


@dataclass(frozen=True)
class Finding:
    routine: str
    offset: int
    status: str
    symbol: str
    expected_owner: str
    effective_segment: str
    proven_owner: str
    text: str


def canonical_register(name: str) -> str:
    lowered = name.lower().strip()
    if lowered.startswith("e") and lowered[1:] in GENERAL_REGISTERS:
        return lowered[1:]
    if lowered in ("al", "ah"):
        return "ax"
    if lowered in ("bl", "bh"):
        return "bx"
    if lowered in ("cl", "ch"):
        return "cx"
    if lowered in ("dl", "dh"):
        return "dx"
    return lowered


def initial_state() -> AbstractState:
    values = {name: UNKNOWN for name in GENERAL_REGISTERS}
    values.update({
        "cs": "CODE",
        "ds": "GAME_DATA",
        "es": UNKNOWN,
        "fs": "FS_DATA",
        "gs": "GAME_DATA",
        "ss": "STACK",
    })
    return AbstractState(tuple(sorted(
        (canonical_register(name), value) for name, value in values.items()
    )))


def merge_state(left: AbstractState, right: AbstractState) -> AbstractState:
    left_registers = dict(left.registers)
    right_registers = dict(right.registers)
    registers = {
        name: left_registers.get(name, UNKNOWN)
        if left_registers.get(name, UNKNOWN) == right_registers.get(name, UNKNOWN)
        else UNKNOWN
        for name in left_registers.keys() | right_registers.keys()
    }
    left_locals = dict(left.locals)
    right_locals = dict(right.locals)
    locals_ = {
        offset: left_locals.get(offset, UNKNOWN)
        if left_locals.get(offset, UNKNOWN) == right_locals.get(offset, UNKNOWN)
        else UNKNOWN
        for offset in left_locals.keys() | right_locals.keys()
    }
    stack = left.stack if left.stack == right.stack else ()
    return AbstractState(
        tuple(sorted(registers.items())), tuple(sorted(locals_.items())), stack
    )


def split_operands(text: str) -> tuple[str, ...]:
    parts = text.strip().split(None, 1)
    if len(parts) != 2:
        return ()
    return tuple(part.strip() for part in parts[1].split(","))


def mnemonic(text: str) -> str:
    return text.strip().split(None, 1)[0].lower() if text.strip() else ""


def local_offset(operand: str) -> int | None:
    match = LOCAL_OPERAND.search(operand.strip())
    if not match:
        return None
    return int(match["disp"], 0)


def source_provenance(operand: str, state: AbstractState,
                      owners: dict[str, str]) -> str:
    normalized = operand.strip().lower()
    if normalized.startswith("dgroup:"):
        return "GAME_DATA"
    seg_match = re.fullmatch(r"seg\s+(?P<symbol>_[\w$?@]+)", normalized)
    if seg_match:
        return owners.get(seg_match["symbol"], UNKNOWN)
    section_match = re.fullmatch(r"(?:seg\s+)?([A-Za-z_][\w$?@]*)", operand.strip())
    if section_match and section_match.group(1).upper() in OWNER_BY_SEGMENT_NAME:
        return OWNER_BY_SEGMENT_NAME[section_match.group(1).upper()]
    if canonical_register(normalized) in GENERAL_REGISTERS + SEGMENT_REGISTERS:
        return state.register(normalized)
    offset = local_offset(normalized)
    if offset is not None:
        return state.local(offset)
    if normalized in ("0", "0x0000", "0x0"):
        return ZERO
    return UNKNOWN


def decode_instruction(item: ListingInstruction) -> capstone.CsInsn:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    result = next(decoder.disasm(item.data, item.offset, count=1), None)
    if result is None or result.size != len(item.data):
        raise ValueError(
            f"cannot decode listing instruction {item.offset:04x}: {item.text}"
        )
    return result


def transfer(item: ListingInstruction, state: AbstractState,
             owners: dict[str, str]) -> AbstractState:
    insn = decode_instruction(item)
    result = state
    try:
        _read, written = insn.regs_access()
    except capstone.CsError:
        written = []
    for register_id in written:
        name = canonical_register(insn.reg_name(register_id))
        if name in GENERAL_REGISTERS + SEGMENT_REGISTERS:
            result = result.with_register(name, UNKNOWN)

    op = mnemonic(item.text)
    operands = split_operands(item.text)
    if op == "mov" and len(operands) == 2:
        destination, source = operands
        value = source_provenance(source, state, owners)
        destination_register = canonical_register(destination)
        if destination_register in GENERAL_REGISTERS + SEGMENT_REGISTERS:
            result = result.with_register(destination_register, value)
        else:
            offset = local_offset(destination)
            if offset is not None:
                result = result.with_local(offset, value)
    elif op == "xor" and len(operands) == 2:
        left = canonical_register(operands[0])
        right = canonical_register(operands[1])
        if left == right and left in GENERAL_REGISTERS:
            result = result.with_register(left, ZERO)
    elif op == "push" and len(operands) == 1:
        source = canonical_register(operands[0])
        if source in SEGMENT_REGISTERS:
            value = source_provenance(operands[0], state, owners)
            result = replace(result, stack=(value,) + result.stack[:63])
    elif op == "pop" and len(operands) == 1:
        destination = canonical_register(operands[0])
        if destination in SEGMENT_REGISTERS:
            value = result.stack[0] if result.stack else UNKNOWN
            result = replace(
                result, stack=result.stack[1:] if result.stack else ()
            )
            result = result.with_register(destination, value)
    elif op == "leave":
        result = result.with_register("bp", UNKNOWN)
    elif op in ("lds", "les", "lfs", "lgs"):
        segment = op[1:]
        result = result.with_register(segment, UNKNOWN)
        if operands:
            result = result.with_register(operands[0], UNKNOWN)
    return result


def parse_listing(path: Path, text: str) -> Listing:
    parsed_instructions: list[ListingInstruction] = []
    labels: dict[str, int] = {}
    table_entries: dict[int, str] = {}
    addressed_rows: list[tuple[int, bytes]] = []
    unparsed_rows: list[tuple[int, bytes, str]] = []
    routine_ranges: list[tuple[int, int]] = []
    pending: tuple[int, bytes] | None = None
    has_segments = any(line.startswith("Segment: ") for line in text.splitlines())
    in_code = not has_segments

    def add_instruction(offset: int, data: bytes, body: str) -> None:
        addressed_rows.append((offset, data))
        data_match = re.fullmatch(
            r"D[WBD]\s+offset\s+(\S+)", body, re.IGNORECASE
        )
        if data_match:
            table_entries[offset] = data_match.group(1)
            return
        if body.upper().startswith(("DB ", "DW ", "DD ", "DQ ")):
            return
        item = ListingInstruction(offset, data, body)
        try:
            decode_instruction(item)
        except ValueError:
            unparsed_rows.append((offset, data, body))
            return
        parsed_instructions.append(item)

    for raw_line in text.splitlines():
        if pending is not None:
            continuation = INSTRUCTION_CONTINUATION_ROW.match(raw_line)
            if continuation:
                offset, data = pending
                add_instruction(offset, data, continuation["text"].strip())
                pending = None
                continue
            offset, data = pending
            addressed_rows.append((offset, data))
            unparsed_rows.append((offset, data, "<missing continuation>"))
            pending = None

        if raw_line.startswith("Segment: "):
            in_code = bool(re.match(
                r"^Segment:\s+\S+_TEXT\s+BYTE\s+USE16\b", raw_line
            ))
            continue
        if not in_code:
            continue
        routine_match = ROUTINE_SIZE_ROW.match(raw_line)
        if routine_match:
            start = int(routine_match["offset"], 16)
            routine_ranges.append((start, start + int(routine_match["size"])))
            continue
        label_match = LABEL_ROW.match(raw_line)
        if label_match:
            labels[label_match["label"]] = int(label_match["offset"], 16)
            continue
        match = INSTRUCTION_ROW.match(raw_line)
        if not match:
            continue
        offset = int(match["offset"], 16)
        data = bytes.fromhex(match["bytes"])
        body = match["text"].strip()
        if not body:
            pending = (offset, data)
            continue
        add_instruction(offset, data, body)

    if pending is not None:
        offset, data = pending
        addressed_rows.append((offset, data))
        unparsed_rows.append((offset, data, "<missing continuation>"))

    function_labels = {
        offset
        for label, offset in labels.items()
        if not label.startswith(("_", "L$"))
    }
    if not function_labels:
        raise ValueError(f"{path}: no public function entry found")

    if routine_ranges:
        executable_ranges = tuple(sorted({
            (start, end)
            for start, end in routine_ranges
            if any(start <= entry < end for entry in function_labels)
        }))
        entrypoints = tuple(sorted(
            entry
            for entry in function_labels
            if any(start <= entry < end for start, end in executable_ranges)
        ))
        missing_spans = sorted(function_labels - set(entrypoints))
        if missing_spans:
            formatted = ", ".join(f"0x{offset:04x}" for offset in missing_spans)
            raise ValueError(
                f"{path}: public function entries have no routine span: {formatted}"
            )
    else:
        entrypoints = tuple(sorted(function_labels))
        if not addressed_rows:
            raise ValueError(f"{path}: no addressed rows found")
        executable_ranges = ((
            min(entrypoints),
            max(offset + len(data) for offset, data in addressed_rows),
        ),)

    def executable(offset: int, size: int = 1) -> bool:
        end = offset + size
        return any(start <= offset and end <= limit
                   for start, limit in executable_ranges)

    for offset, data, body in unparsed_rows:
        if executable(offset, len(data)):
            raise ValueError(
                f"{path}: unparsed executable row at 0x{offset:04x}: {body}"
            )

    covered: set[int] = set()
    for offset, data in addressed_rows:
        for position in range(offset, offset + len(data)):
            if any(start <= position < end for start, end in executable_ranges):
                covered.add(position)
    missing_bytes = [
        position
        for start, end in executable_ranges
        for position in range(start, end)
        if position not in covered
    ]
    if missing_bytes:
        formatted = ", ".join(
            f"0x{position:04x}" for position in missing_bytes[:8]
        )
        suffix = "..." if len(missing_bytes) > 8 else ""
        raise ValueError(
            f"{path}: uncovered executable byte(s): {formatted}{suffix}"
        )

    instructions = [
        item for item in parsed_instructions
        if executable(item.offset, len(item.data))
    ]

    jump_tables: dict[str, tuple[int, ...]] = {}
    for label, start in labels.items():
        targets: list[int] = []
        cursor = start
        while cursor in table_entries:
            target = labels.get(table_entries[cursor])
            if target is not None:
                targets.append(target)
            cursor += 2
        if targets:
            jump_tables[label] = tuple(targets)
    if not instructions:
        raise ValueError(f"{path}: no instructions found in Watcom listing")
    return Listing(
        path,
        tuple(instructions),
        labels,
        jump_tables,
        entrypoints,
        executable_ranges,
    )


def listing_for_object(wdis: Path, object_path: Path,
                       cache_dir: Path | None = None) -> Listing:
    cache_path = cache_dir / f"{object_path.stem}.lst" if cache_dir else None
    if (cache_path is not None and cache_path.is_file() and
            cache_path.stat().st_mtime_ns >= object_path.stat().st_mtime_ns):
        return parse_listing(
            object_path,
            cache_path.read_text(encoding="ascii", errors="replace"),
        )
    process = None
    listing_text = ""
    descriptor, raw_listing_path = tempfile.mkstemp(
        prefix="wd", suffix=".lst", dir="/tmp"
    )
    os.close(descriptor)
    listing_path = Path(raw_listing_path)
    listing_path.unlink()
    object_path = object_path.resolve()
    object_argument = object_path.name
    try:
        for attempt in range(3):
            time.sleep(0.12)
            process = subprocess.run(
                [str(wdis), "-p", f"-l={listing_path}", str(object_argument)],
                cwd=object_path.parent,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
            )
            if process.returncode == 0:
                listing_text = listing_path.read_text(
                    encoding="ascii", errors="replace"
                )
                break
            if listing_path.exists():
                listing_path.unlink()
            # The DOS-hosted Watcom binary can fault when many short-lived QEMU
            # processes start back-to-back.  A bounded retry avoids hiding a real
            # disassembly error while keeping whole-program audits deterministic.
            if process.returncode not in (-11, 139) or attempt == 2:
                break
            time.sleep(0.1 * (attempt + 1))
    finally:
        if listing_path.exists():
            listing_path.unlink()
    assert process is not None
    if process.returncode:
        raise RuntimeError(
            f"wdis failed for {object_path} ({process.returncode}):\n"
            f"{process.stderr}"
        )
    if cache_path is not None:
        cache_path.write_text(listing_text, encoding="ascii")
    return parse_listing(object_path, listing_text)


def successors(listing: Listing) -> dict[int, tuple[int, ...]]:
    offsets = {item.offset for item in listing.instructions}
    result: dict[int, tuple[int, ...]] = {}
    for item in listing.instructions:
        insn = decode_instruction(item)
        next_offset = item.offset + len(item.data)
        groups = set(insn.groups)
        candidates: list[int] = []
        immediate_target = None
        if (insn.operands and
                insn.operands[0].type == x86_const.X86_OP_IMM):
            immediate_target = insn.operands[0].imm & 0xFFFF
        if x86_const.X86_GRP_JUMP in groups:
            if immediate_target in offsets:
                candidates.append(immediate_target)
            else:
                table_match = re.search(r"cs:(?P<label>[\w$?@]+)",
                                        item.text, re.IGNORECASE)
                if table_match:
                    candidates.extend(
                        listing.jump_tables.get(table_match["label"], ())
                    )
            if insn.mnemonic != "jmp" and next_offset in offsets:
                candidates.append(next_offset)
        elif (x86_const.X86_GRP_RET not in groups and
              insn.mnemonic not in ("iret", "iretd") and
              next_offset in offsets):
            candidates.append(next_offset)
        result[item.offset] = tuple(dict.fromkeys(candidates))
    return result


def memory_symbols(text: str, owners: dict[str, str]) -> tuple[str, ...]:
    result: list[str] = []
    lowered = text.lower()
    for match in SYMBOL_TOKEN.finditer(lowered):
        symbol = match.group(0)
        if symbol not in owners:
            continue
        prefix = lowered[max(0, match.start() - 12):match.start()]
        if re.search(r"(?:seg|offset)\s+$", prefix):
            continue
        result.append(symbol)
    return tuple(dict.fromkeys(result))


def effective_segment(text: str, symbol: str) -> str:
    lowered = text.lower()
    position = lowered.find(symbol.lower())
    prefix = lowered[:position]
    override = re.search(r"\b(es|cs|ss|ds|fs|gs)\s*:[^,]*$", prefix)
    if override:
        return override.group(1)
    return "ds"


def analyze_listing(listing: Listing,
                    owners: dict[str, str]) -> tuple[list[Finding], int]:
    by_offset = {item.offset: item for item in listing.instructions}
    edges = successors(listing)
    entrypoints = listing.entrypoints or (min(by_offset),)
    missing_entries = sorted(set(entrypoints) - by_offset.keys())
    if missing_entries:
        formatted = ", ".join(f"0x{offset:04x}" for offset in missing_entries)
        raise ValueError(
            f"{listing.object_path}: function entry is not executable: {formatted}"
        )
    states: dict[int, AbstractState] = {
        entry: initial_state() for entry in entrypoints
    }
    pending = deque(entrypoints)
    visits = 0
    while pending:
        offset = pending.popleft()
        visits += 1
        if visits > len(by_offset) * 200:
            raise ValueError(f"{listing.object_path}: dataflow did not converge")
        outgoing = transfer(by_offset[offset], states[offset], owners)
        for target in edges[offset]:
            previous = states.get(target)
            merged = outgoing if previous is None else merge_state(previous, outgoing)
            if previous != merged:
                states[target] = merged
                pending.append(target)

    unreachable = sorted(by_offset.keys() - states.keys())
    if unreachable:
        formatted = ", ".join(f"0x{offset:04x}" for offset in unreachable[:8])
        suffix = "..." if len(unreachable) > 8 else ""
        raise ValueError(
            f"{listing.object_path}: disconnected executable instruction(s): "
            f"{formatted}{suffix}"
        )

    findings: list[Finding] = []
    for offset, state in sorted(states.items()):
        item = by_offset[offset]
        for symbol in memory_symbols(item.text, owners):
            segment = effective_segment(item.text, symbol)
            proven = state.register(segment)
            expected = owners[symbol]
            if proven == expected:
                status = "ok"
            elif proven == UNKNOWN:
                status = "unproven"
            else:
                status = "mismatch"
            findings.append(Finding(
                listing.object_path.stem,
                offset,
                status,
                symbol,
                expected,
                segment.upper(),
                proven,
                item.text,
            ))
    return findings, len(states)


def read_owners(path: Path) -> dict[str, str]:
    owners: dict[str, str] = {}
    with path.open(newline="", encoding="ascii") as handle:
        for row in csv.DictReader(handle, delimiter="\t"):
            if row["status"] != "known":
                continue
            owner = row["segment"]
            if owner not in ("GAME_DATA", "FS_DATA", "_CODE"):
                continue
            owners[row["symbol"].lower()] = (
                "CODE" if owner == "_CODE" else owner
            )
    if not owners:
        raise ValueError(f"{path}: no known data owners")
    return owners


def linked_project_stems(path: Path) -> set[str]:
    stems: set[str] = set()
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        stripped = line.strip()
        match = SECTION_ROW.match(stripped)
        if not match or match["class"] != "CODE":
            continue
        name = match["name"]
        if name.startswith("func_") and name.endswith("_TEXT"):
            stems.add(name[:-5].lower())
    if not stems:
        raise ValueError(f"{path}: no linked recovered routine sections")
    return stems


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--object-dir", type=Path,
        default=ROOT / "output/recovered_dos_package/bloodprg_objects/bloodprg",
    )
    parser.add_argument(
        "--link-map", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/"
            "final/link.map"
        ),
    )
    parser.add_argument(
        "--data-layout", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/"
            "data_owner/data_layout.tsv"
        ),
    )
    parser.add_argument("--wdis", type=Path, default=Path("wdis"))
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--listing-cache", type=Path,
        help="optional reusable directory for Watcom object listings",
    )
    parser.add_argument(
        "--fail-unproven", action="store_true",
        help="also fail when control-flow analysis cannot prove an owner",
    )
    parser.add_argument(
        "--jobs", type=int, default=1,
        help="wdis worker count; the DOS-hosted Watcom tool is safest at 1",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    owners = read_owners(args.data_layout.resolve())
    linked = linked_project_stems(args.link_map.resolve())
    objects = {
        path.stem.lower(): path
        for path in args.object_dir.resolve().glob("*.[Oo][Bb][Jj]")
    }
    missing = sorted(linked - objects.keys())
    if missing:
        raise SystemExit(
            "linked recovered routines have no object file: " + ", ".join(missing)
        )
    selected = [objects[stem] for stem in sorted(linked)]
    cache_dir = args.listing_cache.resolve() if args.listing_cache else None
    if cache_dir is not None:
        cache_dir.mkdir(parents=True, exist_ok=True)
    if args.jobs == 1:
        listings = [
            listing_for_object(args.wdis, path, cache_dir) for path in selected
        ]
    else:
        with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as executor:
            listings = list(executor.map(
                lambda path: listing_for_object(args.wdis, path, cache_dir), selected
            ))

    findings: list[Finding] = []
    reached = 0
    for listing in listings:
        routine_findings, routine_reached = analyze_listing(listing, owners)
        findings.extend(routine_findings)
        reached += routine_reached

    stream = (
        args.output.open("w", newline="", encoding="ascii")
        if args.output else sys.stdout
    )
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "routine", "object_offset", "status", "symbol", "expected_owner",
        "effective_segment", "proven_owner", "instruction",
    ))
    for finding in sorted(
        findings,
        key=lambda row: (row.status != "mismatch", row.status != "unproven",
                         row.routine, row.offset, row.symbol),
    ):
        writer.writerow((
            finding.routine,
            f"0x{finding.offset:04x}",
            finding.status,
            finding.symbol,
            finding.expected_owner,
            finding.effective_segment,
            finding.proven_owner,
            finding.text,
        ))
    if args.output:
        stream.close()

    counts = {
        status: sum(finding.status == status for finding in findings)
        for status in ("ok", "unproven", "mismatch")
    }
    print(
        f"{len(listings)} linked routines; {reached} reachable object "
        f"instructions; {len(findings)} symbolic data accesses; "
        f"{counts['ok']} proven; {counts['unproven']} unproven; "
        f"{counts['mismatch']} mismatches"
    )
    if counts["mismatch"] or (args.fail_unproven and counts["unproven"]):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
