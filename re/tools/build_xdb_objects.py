#!/usr/bin/env python3
"""Compile every recovered XDB C candidate to a real-mode DOS object file."""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
DEFAULT_OBJECT_DIR = ROOT / "output" / "xdb_objects"
DEFAULT_FLAGS = ("-q", "-c", "-3", "-ox", "-mm", "-zdp", "-we")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_candidates(module: str | None) -> list[dict[str, str]]:
    with MANIFEST.open(newline="", encoding="ascii") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if module is not None:
        manifest_module = f"xdb_{module}"
        rows = [
            row for row in rows if row["entry"].split(":", 1)[0] == manifest_module
        ]
    return rows


def command_text(command: list[str]) -> str:
    return " ".join(command)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--wcl",
        default="wcl",
        help="Open Watcom wcl executable or PATH name",
    )
    parser.add_argument(
        "--object-dir",
        type=Path,
        default=DEFAULT_OBJECT_DIR,
        help="directory receiving module/function .OBJ files and manifest.tsv",
    )
    parser.add_argument(
        "--module",
        choices=("amer", "croolis", "manu3", "scrut"),
        help="compile one XDB module instead of all four",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    wcl = shutil.which(args.wcl) or args.wcl
    if not Path(wcl).is_file() and shutil.which(wcl) is None:
        raise SystemExit(f"Open Watcom compiler not found: {args.wcl}")

    rows = read_candidates(args.module)
    if not rows:
        raise SystemExit("XDB candidate manifest selected no routines")

    object_dir = args.object_dir.resolve()
    object_dir.mkdir(parents=True, exist_ok=True)
    report_rows = [
        "entry\tmodule\tfunction\tsource\tstatus\tobject_bytes\t"
        "object_sha256\tcommand"
    ]
    failures = 0

    for row in rows:
        entry = row["entry"]
        module = entry.split(":", 1)[0]
        source = (MANIFEST.parent / row["source"]).resolve()
        stem = Path(row["source"]).stem
        module_dir = object_dir / module
        module_dir.mkdir(parents=True, exist_ok=True)
        output = module_dir / f"{stem}.OBJ"
        command = [str(wcl), *DEFAULT_FLAGS, f"-fo={output}", str(source)]
        process = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        diagnostics = "\n".join(
            part for part in (process.stdout, process.stderr) if part
        )
        if process.returncode != 0 or not output.is_file():
            failures += 1
            status = f"failed_{process.returncode}"
            object_bytes = "-"
            object_hash = "-"
            error_path = output.with_suffix(".err")
            error_path.write_text(
                f"$ {command_text(command)}\n{diagnostics}\n",
                encoding="utf-8",
            )
            print(f"FAIL {entry}: {error_path}", file=sys.stderr)
        else:
            status = "compiled"
            object_bytes = str(output.stat().st_size)
            object_hash = sha256(output)
            if diagnostics:
                output.with_suffix(".log").write_text(
                    f"$ {command_text(command)}\n{diagnostics}\n",
                    encoding="utf-8",
                )
            print(f"OK {entry}: {output} ({object_bytes} bytes)")

        report_rows.append(
            "\t".join(
                (
                    entry,
                    module,
                    row["function"],
                    row["source"],
                    status,
                    object_bytes,
                    object_hash,
                    command_text(command),
                )
            )
        )

    report = object_dir / "manifest.tsv"
    report.write_text("\n".join(report_rows) + "\n", encoding="ascii")
    compiled = len(rows) - failures
    print(f"wrote {report}")
    print(f"compiled {compiled}/{len(rows)} XDB candidate objects")
    if failures:
        print(f"{failures} candidate object build(s) failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
