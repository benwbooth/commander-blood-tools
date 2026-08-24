#!/usr/bin/env python3
"""Prove recovered alien-XDB code preserves every original tail transfer."""

from __future__ import annotations

import os
from pathlib import Path
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import argparse
import csv
from dataclasses import dataclass
import io
import re


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ASSEMBLY_ROOT = ROOT / "re" / "assembly" / "xdb"
DEFAULT_SOURCE_XDB_ROOT = ROOT / "output" / "recovered_dos_package" / "validation" / "source_xdb"
MODULES = ("amer", "croolis", "scrut")

SLOT2_PREFIX = bytes.fromhex("8b751683c65ef74536ffff7403ff640e")
SLOT13_PREFIX = bytes.fromhex("8b5d360bdb7402ffe3")

# These dynamically targeted jumps cannot be resolved to one routine statically. Keep
# their existing exact machine-code proof alongside the derived direct-transfer audit.
DYNAMIC_DISPATCH_SYMBOLS = {
    "amer": (
        ("xdb_amer_method_slot_2_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_amer_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
    "croolis": (
        ("xdb_croolis_method_slot_2_4_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_croolis_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
    "scrut": (
        ("xdb_scrut_method_slot_2_4_dispatch_or_init_", SLOT2_PREFIX),
        ("xdb_scrut_method_slot_13_resume_or_init_", SLOT13_PREFIX),
    ),
}

MAP_SYMBOL = re.compile(
    r"^([0-9A-Fa-f]{4}):([0-9A-Fa-f]{4})\s+([A-Za-z_$?][\w$?@]*)\s*$",
    re.MULTILINE,
)
MAP_CODE_SEGMENT = re.compile(
    r"^(\S+)\s+CODE\s+\S+\s+([0-9A-Fa-f]{4}):([0-9A-Fa-f]{4})"
    r"\s+([0-9A-Fa-f]{8})\s*$",
    re.MULTILINE,
)
ROUTINE_ENTRY = re.compile(r"^; routine_entry: 0x([0-9A-Fa-f]+)\s*$", re.MULTILINE)
OVERLAY_OFFSET = re.compile(r"^; overlay_offset: 0x([0-9A-Fa-f]+)\s*$", re.MULTILINE)
BYTE_COUNT = re.compile(r"^; byte_count: ([0-9]+)\s*$", re.MULTILINE)
RAW_STOP = re.compile(r"^; raw stop: 0x([0-9A-Fa-f]+)\s*$", re.MULTILINE)
ASM_FILENAME = re.compile(r"^func_([0-9A-Fa-f]+)_(.+)\.asm$")
ASM_INSTRUCTION = re.compile(
    r"^\s*([0-9A-Fa-f]+):\s+((?:[0-9A-Fa-f]{2}\s+)+)"
    r"([A-Za-z][A-Za-z0-9]*)\s*(.*?)\s*$",
    re.MULTILINE,
)
DIRECT_TARGET = re.compile(r"^(?:short\s+)?(?:0x)?([0-9A-Fa-f]+)$", re.IGNORECASE)
LISTING_TRANSFER = re.compile(
    r"^\s*([0-9A-Fa-f]+)\s+((?:[0-9A-Fa-f]{2}\s+)+)"
    r"(call|jmp)\s+(.+?)\s*$",
    re.IGNORECASE,
)
LISTING_RETURN = re.compile(
    r"^\s*([0-9A-Fa-f]+)\s+(?:[0-9A-Fa-f]{2}\s+)+(retf?|iret)\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Routine:
    module: str
    path: Path
    entry: int
    stop: int
    symbol: str | None
    instructions: tuple[tuple[int, str, str], ...]


@dataclass(frozen=True)
class OriginalTransfer:
    module: str
    source_symbol: str
    target_symbol: str
    original_jump_offset: int
    original_target_offset: int


@dataclass(frozen=True)
class CodeSegment:
    name: str
    segment: int
    offset: int
    size: int


@dataclass(frozen=True)
class EmittedReference:
    mnemonic: str
    local_offset: int
    encoded: bytes


@dataclass(frozen=True)
class Result:
    kind: str
    module: str
    source_symbol: str
    target_symbol: str
    original_jump_offset: int | None
    original_target_offset: int | None
    emitted_offset: int | None
    actual: bytes
    status: str


def read_map_symbols(path: Path) -> dict[str, list[tuple[int, int]]]:
    symbols: dict[str, list[tuple[int, int]]] = {}
    text = path.read_text(encoding="ascii", errors="replace")
    for segment, offset, symbol in MAP_SYMBOL.findall(text):
        symbols.setdefault(symbol, []).append((int(segment, 16), int(offset, 16)))
    return symbols


def read_map_segments(path: Path) -> list[CodeSegment]:
    text = path.read_text(encoding="ascii", errors="replace")
    return [
        CodeSegment(name, int(segment, 16), int(offset, 16), int(size, 16))
        for name, segment, offset, size in MAP_CODE_SEGMENT.findall(text)
    ]


def routine_symbol(module: str, path: Path) -> str | None:
    match = ASM_FILENAME.match(path.name)
    if match is None or match.group(2) == "routine":
        return None
    return f"xdb_{module}_{match.group(2)}_"


def parse_original_routine(module: str, path: Path) -> tuple[Routine | None, list[str]]:
    text = path.read_text(encoding="ascii", errors="replace")
    errors: list[str] = []
    filename = ASM_FILENAME.match(path.name)
    if filename is None:
        return None, [f"{module}: cannot derive routine identity from {path}"]

    entry_match = ROUTINE_ENTRY.search(text) or OVERLAY_OFFSET.search(text)
    count_match = BYTE_COUNT.search(text)
    stop_match = RAW_STOP.search(text)
    if entry_match is None:
        errors.append(f"{module}: {path} has no routine entry metadata")
    if count_match is None and stop_match is None:
        errors.append(f"{module}: {path} has neither byte_count nor raw stop metadata")
    if errors:
        return None, errors

    assert entry_match is not None
    entry = int(entry_match.group(1), 16)
    filename_entry = int(filename.group(1), 16)
    if filename_entry != entry:
        errors.append(
            f"{module}: {path} names entry 0x{filename_entry:04x}, metadata says 0x{entry:04x}"
        )

    count = None if count_match is None else int(count_match.group(1))
    if stop_match is not None:
        stop = int(stop_match.group(1), 16)
    else:
        assert count is not None
        stop = entry + count
    if stop <= entry:
        errors.append(f"{module}: {path} has invalid routine range 0x{entry:04x}..0x{stop:04x}")
    if count is not None and stop != entry + count:
        errors.append(
            f"{module}: {path} byte_count ends at 0x{entry + count:04x}, raw stop is 0x{stop:04x}"
        )

    instructions = tuple(
        (int(address, 16), mnemonic.lower(), operands.strip())
        for address, _encoded, mnemonic, operands in ASM_INSTRUCTION.findall(text)
    )
    if not instructions:
        errors.append(f"{module}: {path} contains no parseable instructions")
    return Routine(module, path, entry, stop, routine_symbol(module, path), instructions), errors


def derive_original_transfers(
    assembly_root: Path, module: str
) -> tuple[list[OriginalTransfer], list[str]]:
    module_dir = assembly_root / module
    if not module_dir.is_dir():
        return [], [f"{module}: missing original assembly directory {module_dir}"]

    routines: list[Routine] = []
    errors: list[str] = []
    paths = sorted(module_dir.rglob("*.asm"))
    if not paths:
        return [], [f"{module}: no original assembly routines under {module_dir}"]
    for path in paths:
        routine, routine_errors = parse_original_routine(module, path)
        errors.extend(routine_errors)
        if routine is not None:
            routines.append(routine)

    by_entry: dict[int, Routine] = {}
    for routine in routines:
        if routine.entry in by_entry:
            errors.append(
                f"{module}: duplicate original routine entry 0x{routine.entry:04x}: "
                f"{by_entry[routine.entry].path} and {routine.path}"
            )
        else:
            by_entry[routine.entry] = routine

    transfers: list[OriginalTransfer] = []
    for source in routines:
        for address, mnemonic, operands in source.instructions:
            if mnemonic != "jmp":
                continue
            target_match = DIRECT_TARGET.fullmatch(operands)
            if target_match is None:
                continue
            target_offset = int(target_match.group(1), 16)
            if source.entry <= target_offset < source.stop:
                continue
            target = by_entry.get(target_offset)
            if target is None:
                containing_targets = [
                    routine
                    for routine in routines
                    if routine.entry < target_offset < routine.stop
                ]
                if len(containing_targets) == 1:
                    # The original binary also shares a few interior basic blocks.
                    # They are not routine-to-routine transfers and have no callable
                    # recovered target symbol.
                    continue
                errors.append(
                    f"{module}: unresolved original cross-routine jump at 0x{address:04x} "
                    f"from {source.path.name} to 0x{target_offset:04x}; target belongs "
                    f"to {len(containing_targets)} recovered routine ranges"
                )
                continue
            if source.symbol is None or target.symbol is None:
                errors.append(
                    f"{module}: cannot map original transfer 0x{address:04x} "
                    f"({source.path.name} -> {target.path.name}) to recovered symbols"
                )
                continue
            transfers.append(
                OriginalTransfer(
                    module,
                    source.symbol,
                    target.symbol,
                    address,
                    target_offset,
                )
            )

    transfers.sort(key=lambda item: item.original_jump_offset)
    if not transfers:
        errors.append(f"{module}: derived zero direct cross-routine tail transfers")
    return transfers, errors


def unique_symbol_location(
    module: str,
    symbol: str,
    symbols: dict[str, list[tuple[int, int]]],
    errors: list[str],
) -> tuple[int, int] | None:
    locations = symbols.get(symbol, [])
    if len(locations) != 1:
        errors.append(f"{module}: {symbol} has {len(locations)} map locations; expected one")
        return None
    segment, offset = locations[0]
    if segment != 0:
        errors.append(
            f"{module}: {symbol} is in segment 0x{segment:04x}; expected raw segment zero"
        )
        return None
    return segment, offset


def containing_code_segment(
    module: str,
    symbol: str,
    offset: int,
    segments: list[CodeSegment],
    errors: list[str],
) -> CodeSegment | None:
    matches = [
        segment
        for segment in segments
        if segment.segment == 0 and segment.offset <= offset < segment.offset + segment.size
    ]
    if len(matches) != 1:
        errors.append(
            f"{module}: {symbol} at 0x{offset:04x} belongs to "
            f"{len(matches)} code segments; expected one"
        )
        return None
    segment = matches[0]
    if not segment.name.startswith("func_") or not segment.name.endswith("_TEXT"):
        errors.append(
            f"{module}: {symbol} resolves to non-routine code segment {segment.name}"
        )
        return None
    return segment


def read_listing_references(
    path: Path,
    source_local_offset: int,
    target_symbol: str,
) -> tuple[list[EmittedReference], list[int]]:
    references: list[EmittedReference] = []
    returns: list[int] = []
    text = path.read_text(encoding="ascii", errors="replace")
    for line in text.splitlines():
        transfer = LISTING_TRANSFER.match(line)
        if transfer is not None:
            offset = int(transfer.group(1), 16)
            operand_symbols = re.findall(r"[A-Za-z_$?][\w$?@]*", transfer.group(4))
            if offset >= source_local_offset and target_symbol in operand_symbols:
                references.append(
                    EmittedReference(
                        transfer.group(3).lower(),
                        offset,
                        bytes.fromhex(transfer.group(2)),
                    )
                )
            continue
        returned = LISTING_RETURN.match(line)
        if returned is not None and int(returned.group(1), 16) >= source_local_offset:
            returns.append(int(returned.group(1), 16))
    return references, returns


def decode_linked_transfer(image: bytes, offset: int) -> tuple[str, int | None, bytes]:
    if offset >= len(image):
        return "truncated", None, b""
    opcode = image[offset]
    if opcode in (0xE8, 0xE9):
        actual = image[offset : offset + 3]
        if len(actual) != 3:
            return "truncated", None, actual
        displacement = int.from_bytes(actual[1:3], "little", signed=True)
        return ("call" if opcode == 0xE8 else "jmp"), (offset + 3 + displacement) & 0xFFFF, actual
    if opcode == 0xEB:
        actual = image[offset : offset + 2]
        if len(actual) != 2:
            return "truncated", None, actual
        displacement = int.from_bytes(actual[1:2], "little", signed=True)
        return "jmp", (offset + 2 + displacement) & 0xFFFF, actual
    return "other", None, image[offset : offset + 3]


def audit_direct_transfers(
    module_dir: Path,
    module: str,
    image: bytes,
    symbols: dict[str, list[tuple[int, int]]],
    segments: list[CodeSegment],
    transfers: list[OriginalTransfer],
) -> tuple[list[Result], list[str]]:
    errors: list[str] = []
    results: list[Result] = []
    grouped: dict[tuple[str, str], list[OriginalTransfer]] = {}
    for transfer in transfers:
        grouped.setdefault((transfer.source_symbol, transfer.target_symbol), []).append(transfer)

    for (source_symbol, target_symbol), original_edges in grouped.items():
        source_location = unique_symbol_location(module, source_symbol, symbols, errors)
        target_location = unique_symbol_location(module, target_symbol, symbols, errors)
        if source_location is None or target_location is None:
            for edge in original_edges:
                results.append(
                    Result(
                        "direct",
                        module,
                        source_symbol,
                        target_symbol,
                        edge.original_jump_offset,
                        edge.original_target_offset,
                        None,
                        b"",
                        "unresolved_emitted_symbol",
                    )
                )
            continue

        source_offset = source_location[1]
        target_offset = target_location[1]
        segment = containing_code_segment(
            module, source_symbol, source_offset, segments, errors
        )
        status = "pending"
        references: list[EmittedReference] = []
        returns: list[int] = []
        if segment is None:
            status = "unresolved_source_segment"
        else:
            listing_name = f"{segment.name[:-5]}.lst"
            listing_path = module_dir / "segment_contract_listings" / listing_name
            if not listing_path.is_file():
                errors.append(f"{module}: missing emitted listing {listing_path}")
                status = "missing_emitted_listing"
            else:
                references, returns = read_listing_references(
                    listing_path, source_offset - segment.offset, target_symbol
                )

        emitted_offset: int | None = None
        actual = b""
        if status == "pending":
            if not references:
                status = "missing_emitted_transfer"
                errors.append(
                    f"{module}: {source_symbol} has no emitted transfer to {target_symbol}"
                )
            else:
                linked: list[tuple[EmittedReference, int, str, int | None, bytes]] = []
                assert segment is not None
                for reference in references:
                    linked_offset = segment.offset + reference.local_offset
                    mnemonic, destination, encoded = decode_linked_transfer(image, linked_offset)
                    linked.append((reference, linked_offset, mnemonic, destination, encoded))

                emitted_offset = linked[0][1]
                actual = linked[0][4]
                invalid = [
                    item
                    for item in linked
                    if (
                        item[0].mnemonic != item[2]
                        or item[3] != target_offset
                        or (
                            item[0].mnemonic == "call"
                            and item[0].encoded[:1] != b"\xe8"
                        )
                        or (
                            item[0].mnemonic == "jmp"
                            and item[0].encoded[:1] not in (b"\xe9", b"\xeb")
                        )
                    )
                ]
                mnemonics = {item[0].mnemonic for item in linked}
                if invalid:
                    status = "linked_target_mismatch"
                    details = ", ".join(
                        f"0x{item[1]:04x}:{item[2]}->"
                        f"{'none' if item[3] is None else f'0x{item[3]:04x}'}"
                        for item in invalid
                    )
                    errors.append(
                        f"{module}: {source_symbol} emitted relocation(s) to {target_symbol} "
                        f"do not match linked bytes ({details})"
                    )
                elif mnemonics == {"jmp"}:
                    status = "tail_jump"
                elif mnemonics == {"call"}:
                    has_return = bool(returns)
                    status = "call_then_return" if has_return else "call_not_tail_jump"
                    errors.append(
                        f"{module}: {source_symbol} emits CALL"
                        f"{' + RET' if has_return else ''} to {target_symbol}; "
                        "original control flow requires a tail JMP"
                    )
                else:
                    status = "mixed_call_and_tail_jump"
                    errors.append(
                        f"{module}: {source_symbol} mixes CALL and JMP transfers to "
                        f"{target_symbol}; not every path preserves the original tail transfer"
                    )

        for edge in original_edges:
            results.append(
                Result(
                    "direct",
                    module,
                    source_symbol,
                    target_symbol,
                    edge.original_jump_offset,
                    edge.original_target_offset,
                    emitted_offset,
                    actual,
                    status,
                )
            )
    results.sort(key=lambda item: item.original_jump_offset or -1)
    return results, errors


def audit_dynamic_dispatches(
    module: str,
    image: bytes,
    symbols: dict[str, list[tuple[int, int]]],
) -> tuple[list[Result], list[str]]:
    errors: list[str] = []
    results: list[Result] = []
    for symbol, expected in DYNAMIC_DISPATCH_SYMBOLS[module]:
        location = unique_symbol_location(module, symbol, symbols, errors)
        if location is None:
            results.append(
                Result(
                    "dynamic",
                    module,
                    symbol,
                    "<callback>",
                    None,
                    None,
                    None,
                    b"",
                    "unresolved_emitted_symbol",
                )
            )
            continue
        offset = location[1]
        actual = image[offset : offset + len(expected)]
        status = "exact_tail_prefix" if actual == expected else "prefix_mismatch"
        results.append(
            Result(
                "dynamic",
                module,
                symbol,
                "<callback>",
                None,
                None,
                offset,
                actual,
                status,
            )
        )
        if actual != expected:
            errors.append(
                f"{module}: {symbol} at 0x{offset:04x} changes the dynamic tail "
                f"contract; expected {expected.hex()}, got {actual.hex()}"
            )
    return results, errors


def audit_module(
    assembly_root: Path,
    source_xdb_root: Path,
    module: str,
    *,
    include_dynamic: bool = True,
) -> tuple[list[Result], list[str]]:
    module_dir = source_xdb_root / module
    image_path = module_dir / f"{module}.xdb"
    map_path = module_dir / f"{module}_source_link.map"
    if not image_path.is_file():
        return [], [f"{module}: missing linked image {image_path}"]
    if not map_path.is_file():
        return [], [f"{module}: missing linked map {map_path}"]

    transfers, errors = derive_original_transfers(assembly_root, module)
    image = image_path.read_bytes()
    symbols = read_map_symbols(map_path)
    segments = read_map_segments(map_path)
    if not segments:
        errors.append(f"{module}: linked map {map_path} contains no code segments")

    direct_results, direct_errors = audit_direct_transfers(
        module_dir, module, image, symbols, segments, transfers
    )
    errors.extend(direct_errors)
    results = direct_results
    if include_dynamic:
        dynamic_results, dynamic_errors = audit_dynamic_dispatches(module, image, symbols)
        results.extend(dynamic_results)
        errors.extend(dynamic_errors)
    return results, errors


def render_tsv(results: list[Result]) -> str:
    output = io.StringIO()
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow(
        (
            "kind",
            "module",
            "source_symbol",
            "target_symbol",
            "original_jump_offset",
            "original_target_offset",
            "emitted_offset",
            "actual",
            "status",
        )
    )
    for result in results:
        writer.writerow(
            (
                result.kind,
                result.module,
                result.source_symbol,
                result.target_symbol,
                (
                    ""
                    if result.original_jump_offset is None
                    else f"0x{result.original_jump_offset:04x}"
                ),
                (
                    ""
                    if result.original_target_offset is None
                    else f"0x{result.original_target_offset:04x}"
                ),
                "" if result.emitted_offset is None else f"0x{result.emitted_offset:04x}",
                result.actual.hex(),
                result.status,
            )
        )
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="run the audit")
    parser.add_argument(
        "--assembly-root",
        type=Path,
        default=DEFAULT_ASSEMBLY_ROOT,
        help="directory containing original per-module assembly routines",
    )
    parser.add_argument(
        "--source-xdb-root",
        type=Path,
        default=DEFAULT_SOURCE_XDB_ROOT,
        help="directory containing per-module linked images, maps, and listings",
    )
    parser.add_argument("--output", type=Path, help="write a deterministic TSV report")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.check:
        raise SystemExit("--check is required")

    results: list[Result] = []
    errors: list[str] = []
    for module in MODULES:
        module_results, module_errors = audit_module(
            args.assembly_root.resolve(), args.source_xdb_root.resolve(), module
        )
        results.extend(module_results)
        errors.extend(module_errors)

    report = render_tsv(results)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    direct_count = sum(result.kind == "direct" for result in results)
    dynamic_count = sum(result.kind == "dynamic" for result in results)
    print(
        f"OK: {direct_count} derived direct tail transfer(s), "
        f"{dynamic_count} dynamic dispatch tail(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
