#!/usr/bin/env python3
"""Link recovered C candidates into one XDB code-segment layout probe.

This is deliberately a probe, not a production overlay builder.  It proves
that Open Watcom can place each candidate's public entry at its original
offset, and records any generated helper code that would overwrite bytes
before that entry.  A real overlay is not emitted unless every candidate's
footprint has been audited against the original image.
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
import re
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
WCL_FLAGS = ("-q", "-c", "-3", "-ox", "-mm", "-we")
SEGMENT_RE = re.compile(
    r"^Segment:\s+_CODE\s+\S+\s+USE16\s+([0-9A-Fa-f]+) bytes$",
    re.MULTILINE,
)
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--module", choices=("amer", "croolis", "manu3", "scrut"), required=True)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--wdis", default="wdis")
    parser.add_argument("--main-object", type=Path, required=True)
    parser.add_argument("--owner-object", type=Path, required=True)
    parser.add_argument("--raw-xdb", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--audit-only",
        action="store_true",
        help="compile all candidates and report independent footprint conflicts without linking",
    )
    parser.add_argument("--library", action="append", default=["clibh", "doslfnh"])
    return parser.parse_args()


def tool(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"tool not found: {value}")
    return resolved


def rows(path: Path, module: str) -> list[dict[str, str]]:
    with path.open(newline="", encoding="ascii") as handle:
        selected = [
            row for row in csv.DictReader(handle, delimiter="\t")
            if row["entry"].split(":", 1)[0] == f"xdb_{module}"
        ]
    if not selected:
        raise SystemExit(f"manifest has no candidates for {module}")
    return sorted(selected, key=lambda row: int(row["entry"].split(":", 1)[1], 16))


def compile_wrapper(
    wcl: str,
    source: Path,
    work: Path,
) -> Path:
    wrapper = work / f"{source.stem}_fixed_layout.c"
    wrapper.write_text(
        '#pragma code_seg("_CODE")\n'
        f'#include "{source}"\n',
        encoding="ascii",
    )
    obj = work / f"{source.stem}.OBJ"
    process = subprocess.run(
        [wcl, *WCL_FLAGS, f"-fo={obj}", str(wrapper)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0 or not obj.is_file():
        raise SystemExit(
            f"failed to compile fixed-layout wrapper {source}:\n"
            + process.stdout
            + process.stderr
        )
    return obj


def wdis_layout(wdis: str, obj: Path, function: str) -> tuple[int, int]:
    process = subprocess.run(
        [wdis, "-p", str(obj)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    listing = process.stdout + process.stderr
    if process.returncode != 0:
        raise SystemExit(f"failed to disassemble {obj}:\n{listing}")
    segment = SEGMENT_RE.search(listing)
    if segment is None:
        raise SystemExit(f"{obj} has no _CODE segment")
    symbol = re.search(
        rf"^([0-9A-Fa-f]+)\s+.*\b{re.escape(function)}_:\s*$",
        listing,
        re.MULTILINE,
    )
    if symbol is None:
        symbol = re.search(
            rf"^([0-9A-Fa-f]+)\s+{re.escape(function)}_:\s*$",
            listing,
            re.MULTILINE,
        )
    if symbol is None:
        raise SystemExit(f"{function} was not found in {obj}")
    return int(segment.group(1), 16), int(symbol.group(1), 16)


def write_anchor(path: Path, data: bytes) -> None:
    lines = [
        "; Fixed-layout padding generated from the original XDB image.",
        ".386",
        '_CODE segment byte public use16 \'CODE\'',
    ]
    for start in range(0, len(data), 16):
        lines.append(
            "db " + ", ".join(f"0x{byte:02x}" for byte in data[start : start + 16])
        )
    lines.extend(["_CODE ends", "end", ""])
    path.write_text("\n".join(lines), encoding="ascii")


def mz_image(path: Path) -> bytes:
    data = path.read_bytes()
    if data[:2] not in (b"MZ", b"ZM"):
        raise SystemExit(f"link output is not an MZ executable: {path}")
    header = int.from_bytes(data[8:10], "little") * 16
    pages = int.from_bytes(data[4:6], "little")
    last = int.from_bytes(data[2:4], "little")
    total = pages * 512 if last == 0 else (pages - 1) * 512 + last
    return data[header:total]


def write_placements(path: Path, placements: list[dict[str, str | int]]) -> None:
    with path.open("w", encoding="ascii", newline="") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=(
                "function",
                "target",
                "generated_start",
                "public_offset",
                "generated_end",
                "original_size",
                "status",
            ),
            delimiter="\t",
            lineterminator="\n",
        )
        writer.writeheader()
        for placement in placements:
            writer.writerow(
                {
                    key: f"0x{value:04x}" if isinstance(value, int) else value
                    for key, value in placement.items()
                }
            )


def audit_candidates(
    args: argparse.Namespace,
    wcl: str,
    wdis: str,
    selected: list[dict[str, str]],
    work: Path,
    output: Path,
    raw_size: int,
) -> int:
    targets = [int(row["entry"].split(":", 1)[1], 16) for row in selected]
    placements: list[dict[str, str | int]] = []
    failures = 0
    for row, target in zip(selected, targets):
        source = (args.manifest.resolve().parent / row["source"]).resolve()
        try:
            obj = compile_wrapper(wcl, source, work)
            code_size, public_offset = wdis_layout(wdis, obj, row["function"])
        except SystemExit as error:
            print(str(error))
            failures += 1
            continue
        start = target - public_offset
        end = start + code_size
        covered = [
            f"0x{other:04x}"
            for other in targets
            if other != target and start <= other < end
        ]
        status = "ok"
        if start < 0:
            status = "negative_start"
        elif start < target:
            status = "helper_before_entry"
        if covered:
            status = "covers_fixed_entry:" + ",".join(covered)
        if end > raw_size:
            status = "past_xdb:" + status
        placements.append(
            {
                "function": row["function"],
                "target": target,
                "generated_start": start,
                "public_offset": public_offset,
                "generated_end": end,
                "original_size": raw_size,
                "status": status,
            }
        )
    output.mkdir(parents=True, exist_ok=True)
    write_placements(output / "placement.tsv", placements)
    conflicts = [row for row in placements if row["status"] != "ok"]
    print(f"audited {len(placements)}/{len(selected)} candidates")
    print(f"placement audit: {output / 'placement.tsv'}")
    print(f"footprint conflicts: {len(conflicts)}")
    return 1 if failures or conflicts else 0


def main() -> int:
    args = parse_args()
    wcl = tool(args.wcl)
    wdis = tool(args.wdis)
    main_object = args.main_object.resolve()
    owner_object = args.owner_object.resolve()
    raw = args.raw_xdb.resolve().read_bytes()
    output = args.output_dir.resolve()
    work = output / "work"
    work.mkdir(parents=True, exist_ok=True)

    selected = rows(args.manifest.resolve(), args.module)
    if args.audit_only:
        return audit_candidates(args, wcl, wdis, selected, work, output, len(raw))

    placements: list[dict[str, str | int]] = []
    objects: list[Path] = []
    current = 0
    anchor_index = 0
    for row in selected:
        source = (args.manifest.resolve().parent / row["source"]).resolve()
        function = row["function"]
        target = int(row["entry"].split(":", 1)[1], 16)
        obj = compile_wrapper(wcl, source, work)
        code_size, public_offset = wdis_layout(wdis, obj, function)
        pad = target - current - public_offset
        if pad < 0:
            raise SystemExit(
                f"{function} cannot be placed at 0x{target:04x}: "
                f"current=0x{current:04x}, public=0x{public_offset:04x}"
            )
        if pad:
            anchor = work / f"anchor_{anchor_index:03d}.asm"
            gap = raw[current : current + pad]
            if len(gap) != pad:
                raise SystemExit(
                    f"{function} requires padding beyond {args.raw_xdb} "
                    f"at 0x{current + len(gap):04x}"
                )
            write_anchor(anchor, gap)
            anchor_obj = work / f"anchor_{anchor_index:03d}.OBJ"
            process = subprocess.run(
                [shutil.which("wasm") or "wasm", "-q", str(anchor)],
                cwd=work,
                text=True,
                capture_output=True,
                check=False,
            )
            if process.returncode != 0 or not anchor_obj.is_file():
                raise SystemExit(f"failed to assemble {anchor}:\n{process.stdout}{process.stderr}")
            objects.append(anchor_obj)
            current += pad
            anchor_index += 1
        start = current
        objects.append(obj)
        current += code_size
        placements.append(
            {
                "function": function,
                "target": target,
                "generated_start": start,
                "public_offset": public_offset,
                "generated_end": current,
                "original_size": len(raw),
                "status": "placed" if start <= target < current else "invalid",
            }
        )

    output.mkdir(parents=True, exist_ok=True)
    write_placements(output / "placement.tsv", placements)

    executable = output / "BLOODPRG_FIXED_LAYOUT_PROBE.EXE"
    map_file = output / "link.map"
    response = [
        "system dos",
        f"name {executable}",
        f"option map={map_file}",
        f"file {main_object}",
        *(f"file {path}" for path in objects),
        f"file {owner_object}",
        *(f"library {library}" for library in args.library),
    ]
    process = subprocess.run(
        [shutil.which("wlink") or "wlink"],
        cwd=ROOT,
        input="\n".join(response) + "\n",
        text=True,
        capture_output=True,
        check=False,
    )
    (output / "link.log").write_text(process.stdout + process.stderr, encoding="ascii")
    if process.returncode != 0 or not executable.is_file():
        raise SystemExit(f"fixed-layout link failed; see {output / 'link.log'}")
    (output / "linked_load_image.bin").write_bytes(mz_image(executable))
    print(f"linked fixed-layout probe: {executable}")
    print(f"wrote placement audit: {output / 'placement.tsv'}")
    print(f"generated code span: 0x{current:04x} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
