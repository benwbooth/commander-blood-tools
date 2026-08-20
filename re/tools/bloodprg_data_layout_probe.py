#!/usr/bin/env python3
"""Classify BLOODPRG link symbols and emit a layout-only OMF data probe."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_HEADERS = ROOT / "re" / "source" / "bloodprg" / "candidates" / "include"
OFFSET_RE = re.compile(
    r"(?:GS|DS|SS|ES|FS|CS|game data|data)\s*:\s*(0x[0-9A-Fa-f]+)"
    r"|/\*\s*(0x[0-9A-Fa-f]+)\s*\*/",
    re.I,
)


class Declaration:
    __slots__ = ("symbol", "header", "segment", "offset")

    def __init__(self, symbol: str, header: str, segment: str, offset: int) -> None:
        self.symbol = symbol
        self.header = header
        self.segment = segment
        self.offset = offset


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unresolved", type=Path, required=True)
    parser.add_argument("--header-dir", type=Path, default=DEFAULT_HEADERS)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--image",
        type=Path,
        help="original BLOODPRG.EXE used to byte-back declaration intervals",
    )
    parser.add_argument("--code-file-base", type=lambda value: int(value, 0), default=0x600)
    parser.add_argument("--game-data-file-base", type=lambda value: int(value, 0), default=0xD420)
    parser.add_argument("--fs-data-file-base", type=lambda value: int(value, 0), default=0xC1F0)
    parser.add_argument(
        "--runtime-layout",
        action="store_true",
        help=(
            "place paragraph-aligned GAME_DATA first in DGROUP so original "
            "near offsets and based-segment accesses share one runtime owner"
        ),
    )
    return parser.parse_args()


def declarations(header_dir: Path) -> dict[str, Declaration]:
    result: dict[str, Declaration] = {}
    for path in sorted(header_dir.glob("*.h")):
        text = path.read_text(encoding="ascii")
        for match in re.finditer(r"\bextern\b(?P<body>.*?;)", text, re.S):
            body = match.group("body")
            following = text[match.end() :]
            trailing_comment = re.match(
                r"\s*(?:/\*.*?\*/|//[^\n]*)", following, re.S
            )
            local_comment = body
            if trailing_comment is not None:
                local_comment += trailing_comment.group(0)
            local_matches = list(OFFSET_RE.finditer(local_comment))
            preceding = text[max(0, match.start() - 600) : match.start()]
            comment = local_comment if local_matches else preceding + local_comment
            offset_matches = local_matches or list(OFFSET_RE.finditer(comment))
            offset_match = offset_matches[-1] if offset_matches else None
            if offset_match is None:
                continue
            declaration = " ".join(body.split())
            name_match = re.search(
                r"(?:\*\s*)?([A-Za-z_]\w*)\s*(?:\[[^]]*\])*\s*;\s*$",
                declaration,
            )
            if name_match is None:
                continue
            name = name_match.group(1)
            prefix = declaration[: name_match.start(1)]
            if "(" in prefix or re.search(r"\b" + re.escape(name) + r"\s*\(", declaration):
                continue
            if "CB_FS_DATA" in declaration or re.search(r"\bFS\s*:", comment, re.I):
                segment = "FS_DATA"
            elif "CB_CODE_DATA" in declaration or re.search(r"\bCS\s*:", comment, re.I):
                segment = "_CODE"
            else:
                segment = "GAME_DATA"
            result.setdefault(
                "_" + name,
                Declaration(symbol="_" + name, header=path.name, segment=segment,
                            offset=int(offset_match.group(1) or offset_match.group(2), 16)),
            )
    return result


def read_symbols(path: Path) -> list[str]:
    with path.open(newline="", encoding="ascii") as handle:
        return [row["symbol"] for row in csv.DictReader(handle, delimiter="\t")]


def write_bytes(lines: list[str], data: bytes) -> None:
    for start in range(0, len(data), 16):
        lines.append(
            "db " + ", ".join(f"0x{byte:02x}" for byte in data[start : start + 16])
        )


def write_zeros(lines: list[str], length: int) -> None:
    for start in range(0, length, 16):
        lines.append("db " + ", ".join("0" for _ in range(min(16, length - start))))


def write_asm(
    path: Path,
    entries: list[Declaration],
    image: bytes | None,
    file_bases: dict[str, int],
    runtime_layout: bool,
) -> None:
    by_segment: dict[str, list[Declaration]] = {}
    for entry in entries:
        by_segment.setdefault(entry.segment, []).append(entry)

    lines = [
        "; Generated layout probe.",
        "; With --image, declaration intervals are copied from BLOODPRG.EXE.",
        "; This object still proves layout only; it is not a complete runtime owner.",
        ".386",
    ]
    for segment in ("_CODE", "GAME_DATA", "FS_DATA"):
        segment_entries = sorted(
            by_segment.get(segment, []), key=lambda item: (item.offset, item.symbol)
        )
        if not segment_entries:
            continue
        class_name = "CODE" if segment == "_CODE" else "FAR_DATA"
        alignment = "word"
        if runtime_layout and segment == "GAME_DATA":
            alignment = "para"
        lines.extend(
            [
                f"{segment} segment {alignment} public use16 '{class_name}'",
            ]
        )
        for start in range(0, len(segment_entries), 8):
            lines.append(
                "public "
                + ", ".join(
                    item.symbol for item in segment_entries[start : start + 8]
                )
            )
        current = 0
        index = 0
        while index < len(segment_entries):
            offset = segment_entries[index].offset
            if offset < current:
                raise ValueError(f"non-monotonic {segment} offset at {offset:#x}")
            lines.append(f"org {offset:#06x}")
            while index < len(segment_entries) and segment_entries[index].offset == offset:
                lines.append(f"{segment_entries[index].symbol} label byte")
                index += 1
            next_offset = (
                segment_entries[index].offset
                if index < len(segment_entries)
                else offset + 1
            )
            length = next_offset - offset
            if image is None:
                write_zeros(lines, length)
            else:
                file_offset = file_bases[segment] + offset
                data = image[file_offset : file_offset + length]
                if len(data) != length:
                    raise ValueError(
                        f"{segment} offset {offset:#x} maps outside image "
                        f"at file offset {file_offset:#x}"
                    )
                write_bytes(lines, data)
            current = next_offset
        lines.append(f"{segment} ends")
        if runtime_layout and segment == "GAME_DATA":
            lines.append("DGROUP group GAME_DATA")
        lines.append("")
    lines.append("end")
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def main() -> int:
    args = parse_args()
    if args.image is not None and not args.image.is_file():
        raise SystemExit(f"BLOODPRG image does not exist: {args.image}")
    known = declarations(args.header_dir.resolve())
    symbols = read_symbols(args.unresolved.resolve())
    entries = [known[symbol] for symbol in symbols if symbol in known]
    unknown = [symbol for symbol in symbols if symbol not in known]
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    report = output_dir / "data_layout.tsv"
    with report.open("w", encoding="ascii", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("symbol", "status", "segment", "offset", "header"))
        for entry in sorted(entries, key=lambda item: (item.segment, item.offset, item.symbol)):
            writer.writerow((entry.symbol, "known", entry.segment, f"0x{entry.offset:04x}", entry.header))
        for symbol in sorted(unknown):
            writer.writerow((symbol, "unknown", "", "", ""))
    asm = output_dir / "bloodprg_data_layout_probe.asm"
    image = args.image.resolve().read_bytes() if args.image else None
    write_asm(
        asm,
        entries,
        image,
        {
            "_CODE": args.code_file_base,
            "GAME_DATA": args.game_data_file_base,
            "FS_DATA": args.fs_data_file_base,
        },
        args.runtime_layout,
    )
    known_count = len(entries)
    print(f"known data declarations: {known_count}/{len(symbols)}")
    print(f"unknown symbols: {len(unknown)}")
    print(f"wrote {report}")
    print(f"wrote {asm}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
