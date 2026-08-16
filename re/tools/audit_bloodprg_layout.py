#!/usr/bin/env python3
"""Audit natural-C BLOODPRG candidates against the original fixed layout.

The report is deliberately non-destructive.  It compiles each candidate in a
common code segment, verifies the assembly artifact's raw bytes against the
supplied executable, and reports generated spans that cannot fit at the
original segment-relative entry.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path
import re
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "re" / "source" / "bloodprg" / "candidates" / "manifest.tsv"
WCL_FLAGS = ("-q", "-c", "-3", "-ox", "-mm", "-we")
SEGMENT_RE = re.compile(r"^; seg_off:\s+([0-9A-Fa-f]+):([0-9A-Fa-f]+)\s*$", re.MULTILINE)
FILE_OFFSET_RE = re.compile(r"^; file_offset:\s+0x([0-9A-Fa-f]+)\s*$", re.MULTILINE)
BYTE_COUNT_RE = re.compile(r"^; byte_count:\s+(\d+)\s*$", re.MULTILINE)
SHA_RE = re.compile(r"^; routine_bytes_sha256:\s+([0-9a-fA-F]+)\s*$", re.MULTILINE)
CODE_SIZE_RE = re.compile(
    r"^Segment:\s+_CODE\s+\S+\s+USE16\s+([0-9A-Fa-f]+) bytes$",
    re.MULTILINE,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--wdis", default="wdis")
    return parser.parse_args()


def tool(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"tool not found: {value}")
    return resolved


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="ascii") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows:
        raise SystemExit(f"manifest has no candidates: {path}")
    return rows


def metadata(path: Path) -> dict[str, int | str]:
    text = path.read_text(encoding="ascii", errors="replace")
    patterns = {
        "seg": SEGMENT_RE,
        "file_offset": FILE_OFFSET_RE,
        "byte_count": BYTE_COUNT_RE,
        "sha256": SHA_RE,
    }
    matches = {name: pattern.search(text) for name, pattern in patterns.items()}
    missing = [name for name, match in matches.items() if match is None]
    if missing:
        raise SystemExit(f"{path} is missing metadata: {', '.join(missing)}")
    segment = matches["seg"]
    assert segment is not None
    return {
        "segment": int(segment.group(1), 16),
        "target": int(segment.group(2), 16),
        "file_offset": int(matches["file_offset"].group(1), 16),  # type: ignore[union-attr]
        "byte_count": int(matches["byte_count"].group(1), 10),  # type: ignore[union-attr]
        "sha256": matches["sha256"].group(1).lower(),  # type: ignore[union-attr]
    }


def compile_wrapper(wcl: str, source: Path, work: Path) -> Path:
    wrapper = work / f"{source.stem}_fixed_layout.c"
    wrapper.write_text(
        '#pragma code_seg("_CODE")\n'
        f'#include "{source.resolve()}"\n',
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


def code_layout(wdis: str, obj: Path, function: str) -> tuple[int, int]:
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
    segment = CODE_SIZE_RE.search(listing)
    symbol = re.search(
        rf"^([0-9A-Fa-f]+)\s+.*\b{re.escape(function)}_:\s*$",
        listing,
        re.MULTILINE,
    )
    if segment is None or symbol is None:
        raise SystemExit(f"could not locate _CODE/{function} in {obj}")
    return int(segment.group(1), 16), int(symbol.group(1), 16)


def write_report(path: Path, rows: list[dict[str, int | str]]) -> None:
    fields = (
        "entry",
        "function",
        "segment",
        "target_segment_offset",
        "target_file_offset",
        "raw_routine_size",
        "raw_routine_end",
        "raw_hash",
        "public_offset",
        "code_size",
        "generated_start",
        "generated_end",
        "status",
    )
    with path.open("w", encoding="ascii", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    args = parse_args()
    wcl = tool(args.wcl)
    wdis = tool(args.wdis)
    image_path = args.image.resolve()
    image = image_path.read_bytes()
    manifest = args.manifest.resolve()
    output = args.output_dir.resolve()
    work = output / "work"
    output.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    candidates = []
    for row in read_rows(manifest):
        asm = (ROOT / row["asm_path"]).resolve()
        source = (manifest.parent / row["source"]).resolve()
        info = metadata(asm)
        candidates.append((row, source, info))
    candidates.sort(key=lambda item: (item[2]["segment"], item[2]["target"]))
    targets_by_segment = {}
    for _row, _source, info in candidates:
        targets_by_segment.setdefault(info["segment"], []).append(info["target"])

    report = []
    failures = 0
    for row, source, info in candidates:
        try:
            obj = compile_wrapper(wcl, source, work)
            code_size, public_offset = code_layout(wdis, obj, row["function"])
        except SystemExit as error:
            print(str(error))
            failures += 1
            continue
        file_offset = info["file_offset"]
        raw_size = info["byte_count"]
        raw_bytes = image[file_offset : file_offset + raw_size]
        status = []
        if len(raw_bytes) != raw_size:
            status.append("raw_range_outside_image")
        actual_sha = hashlib.sha256(raw_bytes).hexdigest()
        if actual_sha != info["sha256"]:
            status.append("raw_hash_mismatch")
        target = info["target"]
        generated_start = target - public_offset
        generated_end = generated_start + code_size
        if generated_start < 0:
            status.append("negative_start")
        if generated_start < target:
            status.append("helper_before_entry")
        if generated_end > target + raw_size:
            status.append("exceeds_raw_routine")
        covered = [
            f"0x{other:04x}"
            for other in targets_by_segment[info["segment"]]
            if other != target and generated_start <= other < generated_end
        ]
        if covered:
            status.append("covers_fixed_entry:" + ",".join(covered))
        report.append(
            {
                "entry": row["entry"],
                "function": row["function"],
                "segment": f"0x{info['segment']:04x}",
                "target_segment_offset": f"0x{target:04x}",
                "target_file_offset": f"0x{file_offset:06x}",
                "raw_routine_size": f"0x{raw_size:04x}",
                "raw_routine_end": f"0x{target + raw_size:04x}",
                "raw_hash": "ok" if not any(item.startswith("raw_") for item in status) else actual_sha,
                "public_offset": f"0x{public_offset:04x}",
                "code_size": f"0x{code_size:04x}",
                "generated_start": f"0x{generated_start:04x}",
                "generated_end": f"0x{generated_end:04x}",
                "status": ";".join(status) if status else "ok",
            }
        )
    write_report(output / "placement.tsv", report)
    conflicts = [row for row in report if row["status"] != "ok"]
    print(f"audited {len(report)}/{len(candidates)} candidates")
    print(f"placement audit: {output / 'placement.tsv'}")
    print(f"layout conflicts: {len(conflicts)}")
    return 1 if failures or conflicts else 0


if __name__ == "__main__":
    raise SystemExit(main())
