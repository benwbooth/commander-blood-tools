#!/usr/bin/env python3
"""Compile the recovered BloodScript sources into a complete VM image bundle."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIR = ROOT / "re" / "vm" / "bloodscript"
DEFAULT_OUTPUT_DIR = ROOT / "output" / "recovered_scripts"
DEFAULT_REFERENCE_DIR = ROOT / "accuracy" / "cblood_install" / "cblood"

IMAGES = tuple(
    (script, extension)
    for script in range(1, 6)
    for extension in ("COD", "BAS")
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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
        description="Compile all ten recovered BloodScript VM images."
    )
    parser.add_argument(
        "--source-dir",
        type=Path,
        default=DEFAULT_SOURCE_DIR,
        help="directory containing script1.cod.blood through script5.bas.blood",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="directory receiving uppercase SCRIPTn.COD/BAS files and manifest.tsv",
    )
    parser.add_argument(
        "--reference-dir",
        type=Path,
        default=DEFAULT_REFERENCE_DIR,
        help="installed game directory used for byte-exact verification",
    )
    parser.add_argument(
        "--dictionary-dir",
        type=Path,
        help="directory containing SCRIPTn.DIC compiler dictionaries; defaults to --reference-dir",
    )
    parser.add_argument(
        "--no-reference",
        action="store_true",
        help="do not compare generated images with the installed game files",
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
    dictionary_dir = args.dictionary_dir.resolve() if args.dictionary_dir else None

    if args.no_reference:
        reference_dir = None
    elif not reference_dir.is_dir():
        print(
            f"warning: reference directory does not exist: {reference_dir}; "
            "generated images will be recorded without comparison",
            file=sys.stderr,
        )
        reference_dir = None

    if dictionary_dir is None and reference_dir is not None:
        dictionary_dir = reference_dir

    cbvm = args.cbvm.resolve() if args.cbvm else ROOT / "target" / "debug" / "cbvm"
    if args.cbvm is None or not cbvm.is_file():
        run_checked(["cargo", "build", "--quiet", "--bin", "cbvm"])
    if not cbvm.is_file():
        raise SystemExit(f"cbvm executable was not created: {cbvm}")

    output_dir.mkdir(parents=True, exist_ok=True)
    rows = [
        "script\timage\tsource_bytes\toutput_bytes\tsha256\t"
        "reference_sha256\tstatus"
    ]
    compared = 0

    for script, extension in IMAGES:
        source = source_dir / f"script{script}.{extension.lower()}.blood"
        output = output_dir / f"SCRIPT{script}.{extension}"
        if not source.is_file():
            raise SystemExit(f"missing BloodScript source: {source}")

        compile_command = [str(cbvm), "compile-bloodscript", str(source), str(output)]
        if dictionary_dir is not None:
            dictionary = dictionary_dir / f"SCRIPT{script}.DIC"
            if not dictionary.is_file():
                raise SystemExit(f"missing BloodScript dictionary: {dictionary}")
            compile_command.append(str(dictionary))
        run_checked(compile_command)
        generated_hash = sha256(output)
        reference_hash = "-"
        status = "generated"

        if reference_dir is not None:
            reference = reference_dir / output.name
            if not reference.is_file():
                raise SystemExit(f"missing reference image: {reference}")
            reference_hash = sha256(reference)
            if output.read_bytes() != reference.read_bytes():
                raise SystemExit(
                    f"byte mismatch for {output.name}: generated {generated_hash}, "
                    f"reference {reference_hash}"
                )
            status = "byte_exact"
            compared += 1

        rows.append(
            f"SCRIPT{script}\t{extension}\t{source.stat().st_size}\t"
            f"{output.stat().st_size}\t{generated_hash}\t{reference_hash}\t{status}"
        )
        print(f"{status}: {output.name} ({output.stat().st_size} bytes)")

    manifest = output_dir / "manifest.tsv"
    manifest.write_text("\n".join(rows) + "\n", encoding="ascii")
    if reference_dir is None:
        print(f"wrote {len(IMAGES)} VM images; reference comparison skipped")
    else:
        print(f"wrote {len(IMAGES)} VM images; {compared} byte-exact comparisons passed")
    print(f"wrote {manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
