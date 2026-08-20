#!/usr/bin/env python3
"""Classify one XDB module's symbols and emit a byte-backed OMF data probe."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_HEADERS = ROOT / "re" / "source" / "xdb" / "candidates" / "include"
DEFAULT_IMAGES = ROOT / "output" / "_tmp_dat"
OFFSET_RE = re.compile(
    r"(?:DS|SS|FS|CS|data|code)\s*:\s*(0x[0-9A-Fa-f]+)"
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
    parser.add_argument("--module", choices=("amer", "croolis", "scrut", "manu3"), required=True)
    parser.add_argument("--unresolved", type=Path, required=True)
    parser.add_argument("--header-dir", type=Path, default=DEFAULT_HEADERS)
    parser.add_argument(
        "--image",
        type=Path,
        help="raw XDB image used for byte backing; requires --data-file-base",
    )
    parser.add_argument(
        "--data-file-base",
        type=lambda value: int(value, 0),
        help="file offset of the module's DS/FS/SS offset zero",
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def selected_headers(header_dir: Path, module: str) -> list[Path]:
    names = {"xdb_common.h", "xdb_video.h", "xdb_mouse.h", "xdb_keyboard.h"}
    names.add("xdb_manu3.h" if module == "manu3" else "xdb_alien.h")
    return [header_dir / name for name in sorted(names)]


def module_symbol(symbol: str, module: str) -> bool:
    name = symbol[1:] if symbol.startswith("_") else symbol
    if module == "manu3":
        return name.startswith("xdb_manu3_") or name.startswith("xdb_video_")
    return (
        name.startswith("xdb_alien_")
        or name.startswith(f"xdb_{module}_")
        or name.startswith("xdb_video_")
    )


def declarations(header_dir: Path, module: str) -> dict[str, Declaration]:
    result: dict[str, Declaration] = {}
    for path in selected_headers(header_dir, module):
        text = path.read_text(encoding="ascii")
        for match in re.finditer(r"\bextern\b(?P<body>.*?;)", text, re.S):
            body = match.group("body")
            # An offset annotation belongs to this declaration only when it is
            # on the same physical line as the terminating semicolon. Looking
            # arbitrarily ahead can steal the next declaration's CS comment
            # and misclassify an ordinary DS object as code-resident state.
            trailing_line = text[match.end() :].split("\n", 1)[0]
            offset_match = OFFSET_RE.search(trailing_line)
            comment = trailing_line
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
            if "(" in prefix:
                continue
            segment = "_CODE" if "XDB_CODE_DATA" in declaration or re.search(
                r"\bCS\s*:", comment, re.I
            ) else "XDB_DATA"
            result.setdefault(
                "_" + name,
                Declaration(
                    symbol="_" + name,
                    header=path.name,
                    segment=segment,
                    offset=int(
                        offset_match.group(1)
                        or offset_match.group(2),
                        16,
                    ),
                ),
            )
    return result


def read_symbols(path: Path) -> list[str]:
    with path.open(newline="", encoding="ascii") as handle:
        return [row["symbol"] for row in csv.DictReader(handle, delimiter="\t")]


def image_bytes(
    image: Path | None,
    base: int | None,
    segment: str,
    offset: int,
    length: int,
) -> bytes | None:
    if image is None or base is None:
        return None
    file_offset = offset if segment == "_CODE" else base + offset
    data = image.read_bytes()
    if file_offset < 0 or file_offset >= len(data):
        raise ValueError(
            f"{segment} offset {offset:#x} maps outside {image} at {file_offset:#x}"
        )
    return data[file_offset : file_offset + length]


def write_bytes(lines: list[str], data: bytes, start: int) -> int:
    cursor = start
    for index in range(0, len(data), 16):
        chunk = data[index : index + 16]
        lines.append("db " + ", ".join(f"0x{byte:02x}" for byte in chunk))
        cursor += len(chunk)
    return cursor


def write_zeros(lines: list[str], length: int) -> None:
    for start in range(0, length, 16):
        count = min(16, length - start)
        lines.append("db " + ", ".join("0" for _ in range(count)))


def write_asm(
    path: Path,
    entries: list[Declaration],
    image: Path | None,
    data_file_base: int | None,
) -> None:
    by_segment: dict[str, list[Declaration]] = {}
    for entry in entries:
        by_segment.setdefault(entry.segment, []).append(entry)

    lines = [
        "; Generated per-XDB layout probe.",
        "; With --image, bytes are copied from the original XDB image.",
        "; This object still proves layout only; it is not an overlay entrypoint.",
        ".386",
    ]
    for segment in ("_CODE", "XDB_DATA"):
        segment_entries = sorted(
            by_segment.get(segment, []), key=lambda item: (item.offset, item.symbol)
        )
        if not segment_entries:
            continue
        lines.append(f"{segment} segment word public use16 'FAR_DATA'")
        for start in range(0, len(segment_entries), 8):
            lines.append(
                "public "
                + ", ".join(item.symbol for item in segment_entries[start : start + 8])
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
            if length:
                chunk = image_bytes(
                    image, data_file_base, segment, offset, length
                )
                if chunk is None or len(chunk) != length:
                    write_zeros(lines, length)
                else:
                    write_bytes(lines, chunk, offset)
            current = next_offset
        lines.extend([f"{segment} ends", ""])
    lines.append("end")
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def main() -> int:
    args = parse_args()
    if (args.image is None) != (args.data_file_base is None):
        raise SystemExit("--image and --data-file-base must be supplied together")
    known = declarations(args.header_dir.resolve(), args.module)
    symbols = read_symbols(args.unresolved.resolve())
    entries = [
        known[symbol]
        for symbol in symbols
        if symbol in known and module_symbol(symbol, args.module)
    ]
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
    asm = output_dir / f"{args.module}_data_layout_probe.asm"
    write_asm(asm, entries, args.image.resolve() if args.image else None, args.data_file_base)
    code_entries = sum(entry.segment == "_CODE" for entry in entries)
    data_entries = sum(entry.segment == "XDB_DATA" for entry in entries)
    print(
        f"{args.module}: known declarations: {len(entries)}/{len(symbols)} "
        f"(code={code_entries}, data={data_entries})"
    )
    print(f"{args.module}: unresolved symbols without declarations: {len(unknown)}")
    print(f"wrote {report}")
    print(f"wrote {asm}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
