#!/usr/bin/env python3
"""Link one recovered XDB module into the game's raw loadable format.

The game copies an XDB image verbatim and far-calls offset zero.  This builder
therefore links the recovered C normally, supplies the original non-code data
as paragraph-aligned OMF segments, and emits a small assembly entry shim for
the host's BP/SS calling contract.  C routine offsets and sizes are free to
change; all references are resolved by the linker.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path
import re
import shutil
import struct
import subprocess

from xdb_data_layout_probe import Declaration, declarations, module_symbol


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
HEADER_DIR = MANIFEST.parent / "include"


class Module:
    __slots__ = (
        "api_symbol",
        "data_file_base",
        "data_delta_symbol",
        "data_segment_symbol",
        "directory_symbol",
        "directory_delta_offsets",
        "alien",
    )

    def __init__(
        self,
        api_symbol: str,
        data_file_base: int,
        data_delta_symbol: str,
        data_segment_symbol: str,
        directory_symbol: str,
        directory_delta_offsets: tuple[int, ...],
        alien: bool,
    ) -> None:
        self.api_symbol = api_symbol
        self.data_file_base = data_file_base
        self.data_delta_symbol = data_delta_symbol
        self.data_segment_symbol = data_segment_symbol
        self.directory_symbol = directory_symbol
        self.directory_delta_offsets = directory_delta_offsets
        self.alien = alien


MODULES = {
    "manu3": Module(
        "xdb_manu3_api_entry_",
        0x1370,
        "_xdb_manu3_data_segment_delta",
        "_xdb_manu3_data_segment",
        "_xdb_source_data_base",
        (0x0C, 0x0E, 0x10),
        False,
    ),
    "amer": Module(
        "xdb_amer_api_entry_",
        0x3280,
        "_xdb_amer_data_segment_delta",
        "_xdb_amer_data_segment",
        "_xdb_source_data_base",
        (0x0C, 0x0E, 0x10),
        True,
    ),
    "croolis": Module(
        "xdb_croolis_api_entry_",
        0x32F0,
        "_xdb_croolis_data_segment_delta",
        "_xdb_croolis_data_segment",
        "_xdb_source_data_base",
        (0x0C, 0x0E, 0x10),
        True,
    ),
    "scrut": Module(
        "xdb_scrut_api_entry_",
        0x33B0,
        "_xdb_scrut_data_segment_delta",
        "_xdb_scrut_data_segment",
        "_xdb_source_data_base",
        (0x0C, 0x0E, 0x10),
        True,
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--module", choices=tuple(MODULES), required=True)
    parser.add_argument("--object-dir", type=Path, required=True)
    parser.add_argument("--raw-xdb", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--wasm", default="wasm")
    parser.add_argument("--wlink", default="wlink")
    parser.add_argument("--library", action="append", default=["clibm"])
    return parser.parse_args()


def tool(name: str) -> str:
    resolved = shutil.which(name)
    if resolved is None:
        raise SystemExit(f"tool not found: {name}")
    return resolved


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    process = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise SystemExit(
            f"command failed: {' '.join(command)}\n"
            + process.stdout
            + process.stderr
        )
    return process


def write_bytes(lines: list[str], data: bytes) -> None:
    for start in range(0, len(data), 16):
        lines.append(
            "db " + ", ".join(f"0x{byte:02x}" for byte in data[start : start + 16])
        )


def emit_labeled_bytes(
    lines: list[str],
    data: bytes,
    labels: dict[int, list[str]],
    overrides: dict[int, tuple[int, str]] | None = None,
) -> None:
    overrides = overrides or {}
    points = sorted(set(labels) | set(overrides) | {0, len(data)})
    cursor = 0
    for point in points:
        if point < cursor or point > len(data):
            raise SystemExit(f"owner label/override outside byte interval: {point:#x}")
        write_bytes(lines, data[cursor:point])
        cursor = point
        for symbol in labels.get(point, ()):
            lines.append(f"{symbol} label byte")
        if point in overrides:
            width, directive = overrides[point]
            lines.append(directive)
            cursor += width
    write_bytes(lines, data[cursor:])


def segment_starts(image: bytes, module: Module) -> list[int]:
    starts = [module.data_file_base]
    cursor = module.data_file_base
    for offset in module.directory_delta_offsets:
        delta = struct.unpack_from("<H", image, module.data_file_base + offset)[0]
        cursor += delta * 16
        if cursor <= starts[-1] or cursor > len(image):
            raise SystemExit(
                f"invalid XDB segment delta at DS:{offset:#06x}: file {cursor:#x}"
            )
        starts.append(cursor)
    return starts


def entry_lines(module: Module) -> list[str]:
    api = module.api_symbol
    lines = [
        "XDB_ENTRY segment byte public use16 'CODE'",
        "public _xdb_overlay_entry",
        f"extrn {api}:near",
        "_xdb_overlay_entry proc far",
    ]
    if module.alien:
        lines.extend(
            [
                "push eax",
                "push ebx",
                "push ecx",
                "push edx",
                "push esi",
                "push edi",
                "push ds",
                "push es",
                "push fs",
                "push gs",
                "push ebp",
                "mov ax,bp",
                "mov dx,ss",
                "mov bx,cs",
                "push cs",
                f"call {api}",
                "pop ebp",
                "pop gs",
                "pop fs",
                "pop es",
                "pop ds",
                "pop edi",
                "pop esi",
                "pop edx",
                "pop ecx",
                "pop ebx",
                "pop eax",
            ]
        )
    else:
        lines.extend(
            [
                "push ds",
                "mov ax,cs",
                f"add ax,word ptr cs:{module.data_delta_symbol}",
                "mov ds,ax",
                "mov fs,ax",
                "mov es,ax",
                "mov ax,bp",
                "mov dx,ss",
                "mov bx,cs",
                "push cs",
                f"call {api}",
                "pop ds",
            ]
        )
    lines.extend(["retf", "_xdb_overlay_entry endp", "XDB_ENTRY ends", ""])
    return lines


def owner_assembly(
    path: Path,
    module_name: str,
    module: Module,
    image: bytes,
    declared: list[Declaration],
) -> None:
    code = sorted(
        (item for item in declared if item.segment == "_CODE"),
        key=lambda item: (item.offset, item.symbol),
    )
    data = sorted(
        (item for item in declared if item.segment == "XDB_DATA"),
        key=lambda item: (item.offset, item.symbol),
    )
    if not code or not data:
        raise SystemExit(f"{module_name}: missing code/data owner declarations")
    if module.data_delta_symbol not in {item.symbol for item in code}:
        raise SystemExit(f"{module_name}: missing data delta declaration")
    if module.data_segment_symbol not in {item.symbol for item in code}:
        raise SystemExit(f"{module_name}: missing data segment declaration")
    lines = [
        "; Generated source-linked XDB owner. Do not edit.",
        "; Original machine code is not included; only initialized state/payload bytes remain.",
        ".386",
        "",
        *entry_lines(module),
    ]

    code_start = min(item.offset for item in code)
    code_end = module.data_file_base
    code_labels: dict[int, list[str]] = {}
    for item in code:
        if not code_start <= item.offset < code_end:
            raise SystemExit(f"{item.symbol}: CS offset outside original code interval")
        code_labels.setdefault(item.offset - code_start, []).append(item.symbol)
    lines.append("_CODE segment byte public use16 'CODE'")
    for start in range(0, len(code), 8):
        lines.append("public " + ", ".join(item.symbol for item in code[start : start + 8]))
    overrides = {
        next(
            item.offset - code_start
            for item in code
            if item.symbol == module.data_delta_symbol
        ): (2, f"dw seg {module.directory_symbol}"),
        next(
            item.offset - code_start
            for item in code
            if item.symbol == module.data_segment_symbol
        ): (2, "dw 0"),
    }
    emit_labeled_bytes(
        lines,
        image[code_start:code_end],
        code_labels,
        overrides,
    )
    lines.extend(["_CODE ends", ""])

    starts = segment_starts(image, module)
    data_end = starts[1]
    data_labels: dict[int, list[str]] = {}
    for item in data:
        if item.offset >= data_end - module.data_file_base:
            raise SystemExit(
                f"{item.symbol}: DS offset {item.offset:#x} crosses first payload segment"
            )
        data_labels.setdefault(item.offset, []).append(item.symbol)
    lines.append("XDB_DATA segment para public use16 'FAR_DATA'")
    lines.append(f"public {module.directory_symbol}")
    for start in range(0, len(data), 8):
        lines.append("public " + ", ".join(item.symbol for item in data[start : start + 8]))
    data_labels.setdefault(0, []).insert(0, module.directory_symbol)
    emit_labeled_bytes(
        lines,
        image[module.data_file_base:data_end],
        data_labels,
    )
    lines.extend(["XDB_DATA ends", ""])

    # Keep every remaining byte initialized and split at real segment boundaries
    # and 64 KiB limits. All starts are paragraph aligned, as required by the
    # runtime's additive segment directory.
    payload_points = starts[1:] + [len(image)]
    part = 0
    for begin, end in zip(payload_points, payload_points[1:]):
        cursor = begin
        while cursor < end:
            remaining = end - cursor
            length = min(remaining, 0xFFF0)
            if length != remaining:
                length &= ~0x0F
            segment_name = f"XDB_PAYLOAD_{part:03d}"
            lines.append(
                f"{segment_name} segment para private use16 'XDB_PAYLOAD_{part:03d}'"
            )
            write_bytes(lines, image[cursor : cursor + length])
            lines.extend([f"{segment_name} ends", ""])
            cursor += length
            part += 1
    lines.append("end _xdb_overlay_entry")
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def manifest_sources(module: str) -> set[str]:
    prefix = f"xdb_{module}:"
    with MANIFEST.open(newline="", encoding="ascii") as handle:
        return {
            Path(row["source"]).stem.lower()
            for row in csv.DictReader(handle, delimiter="\t")
            if row["entry"].startswith(prefix)
        }


def candidate_objects(object_dir: Path, module: str) -> list[Path]:
    expected = manifest_sources(module)
    objects = sorted(object_dir.glob("*.OBJ"))
    actual = {path.stem.lower() for path in objects}
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        raise SystemExit(
            f"{module}: object set differs from manifest; missing={missing}, extra={extra}"
        )
    return objects


def mz_image_and_relocations(path: Path) -> tuple[bytes, list[int], int, int]:
    data = path.read_bytes()
    if data[:2] not in (b"MZ", b"ZM"):
        raise SystemExit(f"link output is not an MZ executable: {path}")
    last, pages, reloc_count, header_paragraphs = struct.unpack_from("<HHHH", data, 2)
    header_size = header_paragraphs * 16
    total = pages * 512 if last == 0 else (pages - 1) * 512 + last
    reloc_offset = struct.unpack_from("<H", data, 0x18)[0]
    ip, cs = struct.unpack_from("<HH", data, 0x14)
    relocations = []
    for index in range(reloc_count):
        offset, segment = struct.unpack_from("<HH", data, reloc_offset + index * 4)
        relocations.append(segment * 16 + offset)
    return data[header_size:total], relocations, cs, ip


def map_symbols(path: Path) -> dict[str, int]:
    symbols: dict[str, int] = {}
    pattern = re.compile(r"^([0-9A-Fa-f]{4}):([0-9A-Fa-f]{4})[+* ]+([A-Za-z_]\w*)$")
    for line in path.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line.rstrip())
        if match:
            address = int(match.group(1), 16) * 16 + int(match.group(2), 16)
            name = match.group(3)
            symbols[name] = address
            if name.endswith("_") and not name.startswith("_"):
                symbols["_" + name[:-1]] = address
    return symbols


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    args = parse_args()
    module = MODULES[args.module]
    raw_path = args.raw_xdb.resolve()
    object_dir = args.object_dir.resolve()
    output = args.output_dir.resolve()
    if not raw_path.is_file():
        raise SystemExit(f"missing original XDB: {raw_path}")
    if not object_dir.is_dir():
        raise SystemExit(f"missing object directory: {object_dir}")
    output.mkdir(parents=True, exist_ok=True)

    image = raw_path.read_bytes()
    known = declarations(HEADER_DIR, args.module)
    declared = sorted(
        (item for item in known.values() if module_symbol(item.symbol, args.module)),
        key=lambda item: (item.segment, item.offset, item.symbol),
    )
    owner_source = output / f"{args.module}_source_owner.asm"
    owner_assembly(owner_source, args.module, module, image, declared)
    owner_object = output / f"{args.module}_source_owner.obj"
    run([tool(args.wasm), "-q", f"-fo={owner_object}", str(owner_source)])

    objects = candidate_objects(object_dir, args.module)
    linked = output / f"{args.module}_source_link.exe"
    map_path = output / f"{args.module}_source_link.map"
    response = [
        "system dos",
        f"name {linked}",
        "option quiet",
        f"option map={map_path}",
        "option start=_xdb_overlay_entry",
        "option stack=0",
        f"file {owner_object}",
        *(f"file {path}" for path in objects),
        *(f"library {library}" for library in args.library),
    ]
    response_path = output / f"{args.module}_source_link.lnk"
    response_path.write_text("\n".join(response) + "\n", encoding="ascii")
    process = subprocess.run(
        [tool(args.wlink), f"@{response_path}"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    (output / "link.log").write_text(
        process.stdout + process.stderr,
        encoding="utf-8",
    )
    if process.returncode != 0 or not linked.is_file():
        raise SystemExit(
            f"{args.module}: source XDB link failed\n"
            + process.stdout
            + process.stderr
        )

    load_image, relocations, entry_cs, entry_ip = mz_image_and_relocations(linked)
    if (entry_cs, entry_ip) != (0, 0):
        raise SystemExit(
            f"{args.module}: linked entry is {entry_cs:04x}:{entry_ip:04x}, expected 0000:0000"
        )
    symbols = map_symbols(map_path)
    required = (
        "_xdb_overlay_entry",
        module.data_delta_symbol,
        module.data_segment_symbol,
        module.directory_symbol,
    )
    missing = [symbol for symbol in required if symbol not in symbols]
    if missing:
        raise SystemExit(f"{args.module}: symbols absent from map: {missing}")
    if symbols["_xdb_overlay_entry"] != 0:
        raise SystemExit(f"{args.module}: overlay entry is not at raw offset zero")

    data_start = symbols[module.directory_symbol]
    if data_start & 0x0F:
        raise SystemExit(f"{args.module}: linked data segment is not paragraph aligned")
    if symbols[module.data_segment_symbol] + 2 > data_start:
        raise SystemExit(f"{args.module}: code-resident state crossed into XDB data")
    delta_offset = symbols[module.data_delta_symbol]
    delta = struct.unpack_from("<H", load_image, delta_offset)[0]
    if delta != data_start // 16:
        raise SystemExit(
            f"{args.module}: linked data delta {delta:#x} != segment {data_start // 16:#x}"
        )
    unexpected_relocations = [offset for offset in relocations if offset != delta_offset]
    if unexpected_relocations:
        formatted = ", ".join(f"0x{offset:05x}" for offset in unexpected_relocations)
        raise SystemExit(
            f"{args.module}: raw image still needs runtime relocations at {formatted}"
        )

    original_payload = image[module.data_file_base :]
    rebuilt_payload = load_image[data_start : data_start + len(original_payload)]
    if rebuilt_payload != original_payload:
        mismatch = next(
            index
            for index, (left, right) in enumerate(zip(rebuilt_payload, original_payload))
            if left != right
        )
        raise SystemExit(
            f"{args.module}: non-code payload differs at DS/file-relative {mismatch:#x}"
        )
    raw = load_image[: data_start + len(original_payload)]
    destination = output / f"{args.module}.xdb"
    destination.write_bytes(raw)
    report = output / "build.tsv"
    report.write_text(
        "module\tentry\tcode_bytes\tdata_file_base\toriginal_bytes\trebuilt_bytes\t"
        "relocations\toriginal_sha256\trebuilt_sha256\n"
        f"{args.module}\t0x0000\t{data_start}\t0x{data_start:05x}\t{len(image)}\t"
        f"{len(raw)}\t{len(relocations)}\t{sha256(image)}\t{sha256(raw)}\n",
        encoding="ascii",
    )
    print(
        f"{args.module}: linked {len(objects)} C routine objects; "
        f"code={data_start} bytes, payload={len(original_payload)} bytes"
    )
    print(f"{args.module}: raw entry 0000:0000; relocations={len(relocations)} (data delta only)")
    print(f"wrote {destination}")
    print(f"wrote {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
