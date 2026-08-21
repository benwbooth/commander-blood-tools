#!/usr/bin/env python3
"""Compile the complete recovered BloodScript and BloodData VM bundle."""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIR = ROOT / "re" / "vm" / "structured"
DEFAULT_OUTPUT_DIR = ROOT / "output" / "recovered_scripts"
DEFAULT_REFERENCE_DIR = ROOT / "accuracy" / "cblood_install" / "cblood"
SCRIPT_EXTENSIONS = ("COD", "BAS", "DEB", "DIC", "VAR")


def run_checked(command: list[str]) -> None:
    process = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if process.returncode == 0:
        return
    output = "\n".join(part for part in (process.stdout, process.stderr) if part)
    raise SystemExit(
        f"command failed with exit status {process.returncode}: "
        f"{' '.join(command)}\n{output}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compile all 25 recovered BloodScript/BloodData VM resources."
    )
    parser.add_argument(
        "--source-dir",
        type=Path,
        default=DEFAULT_SOURCE_DIR,
        help="directory containing the structured .blood and .blooddata sources",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="directory receiving uppercase SCRIPTn.COD/BAS/DEB/DIC/VAR files",
    )
    parser.add_argument(
        "--reference-dir",
        type=Path,
        default=DEFAULT_REFERENCE_DIR,
        help="game directory used for mandatory byte-exact verification",
    )
    parser.add_argument(
        "--cbvm",
        type=Path,
        help="existing cbvm executable; otherwise build target/debug/cbvm",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source_dir = args.source_dir.resolve()
    output_dir = args.output_dir.resolve()
    reference_dir = args.reference_dir.resolve()

    if not source_dir.is_dir():
        raise SystemExit(f"missing VM source directory: {source_dir}")
    if not reference_dir.is_dir():
        raise SystemExit(f"missing VM reference directory: {reference_dir}")

    cbvm = args.cbvm.resolve() if args.cbvm else ROOT / "target" / "debug" / "cbvm"
    if not cbvm.is_file():
        run_checked(["cargo", "build", "--quiet", "--bin", "cbvm"])
    if not cbvm.is_file():
        raise SystemExit(f"cbvm executable was not created: {cbvm}")

    run_checked(
        [
            str(cbvm),
            "compile-bundle",
            str(source_dir),
            str(reference_dir),
            str(output_dir),
        ]
    )

    expected = [
        output_dir / f"SCRIPT{script}.{extension}"
        for script in range(1, 6)
        for extension in SCRIPT_EXTENSIONS
    ]
    missing = [path.name for path in expected if not path.is_file()]
    if missing:
        raise SystemExit(f"compiled VM bundle is incomplete: {', '.join(missing)}")

    manifest = output_dir / "cbvm-bundle-manifest.tsv"
    if not manifest.is_file():
        raise SystemExit(f"compiled VM bundle manifest is missing: {manifest}")
    print(f"wrote {len(expected)} byte-exact VM resources to {output_dir}")
    print(f"wrote {manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
