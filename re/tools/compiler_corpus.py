#!/usr/bin/env python3
"""Run natural-C compiler codegen probes for BLOODPRG source recovery.

The probes are not recovered game source. They are small C samples used to
compare candidate historical DOS compiler output against recovered assembly
shapes before accepting any natural-C routine.
"""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import csv
import json
import re
import subprocess
from collections import Counter
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
CORPUS_ROOT = REPO_ROOT / "re" / "compiler_corpus"
MANIFEST = CORPUS_ROOT / "manifest.tsv"
DEFAULT_OUT = CORPUS_ROOT / "out"

FORBIDDEN_SOURCE_TOKENS = [
    "read16_far",
    "write16_far",
    "cb_read",
    "cb_write",
    "machine_state",
    "register_state",
    "CbMachine",
]

ASM_SKIP_PREFIXES = (
    ".",
    "assume",
    "comment",
    "end",
    "ends",
    "extrn",
    "group",
    "include",
    "public",
    "segment",
)

INSN_RE = re.compile(
    r"^([0-9a-fA-F]{6}):\s+((?:[0-9a-fA-F]{2}\s+)+)\s*([A-Za-z][A-Za-z0-9]*)\s*(.*)$"
)

NUMERIC_RE = re.compile(
    r"(?<![A-Za-z0-9_])(?:0x[0-9a-f]+|[0-9a-f]+h|\d+)(?![A-Za-z0-9_])",
    re.IGNORECASE,
)


def load_manifest() -> list[dict[str, str]]:
    with MANIFEST.open(newline="") as fh:
        rows = list(csv.DictReader(fh, delimiter="\t"))
    required = {"sample", "source", "target_routine", "question", "candidate_source"}
    missing = required.difference(rows[0].keys() if rows else set())
    if missing:
        raise SystemExit(f"{MANIFEST}: missing columns: {', '.join(sorted(missing))}")
    return rows


def sample_path(row: dict[str, str]) -> Path:
    return CORPUS_ROOT / row["source"]


def asm_path_for_target(target: str) -> Path | None:
    needle = f"func_{int(target, 16):06x}_"
    matches = sorted((REPO_ROOT / "re" / "assembly" / "bloodprg").glob(f"**/{needle}*.asm"))
    return matches[0] if matches else None


def read_original_shape(path: Path) -> dict[str, object]:
    meta: dict[str, str] = {}
    insns: list[str] = []
    byte_lines: list[str] = []

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("; "):
            body = line[2:]
            if ": " in body:
                key, value = body.split(": ", 1)
                meta[key] = value
            continue
        match = INSN_RE.match(line)
        if not match:
            continue
        _, byte_text, mnemonic, op_str = match.groups()
        op_str = " ".join(op_str.split())
        insns.append(f"{mnemonic.lower()} {op_str}".strip())
        byte_lines.append(" ".join(byte_text.split()).lower())

    return {
        "byte_count": meta.get("byte_count", ""),
        "terminal": meta.get("terminal", ""),
        "direct_callees": meta.get("direct_callees", ""),
        "first_instructions": insns[:8],
        "first_bytes": byte_lines[:8],
    }


def read_original_routine(path: Path) -> dict[str, object]:
    meta: dict[str, str] = {}
    instructions: list[str] = []
    byte_lines: list[str] = []

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("; "):
            body = line[2:]
            if ": " in body:
                key, value = body.split(": ", 1)
                meta[key] = value
            continue
        match = INSN_RE.match(line)
        if not match:
            continue
        _, byte_text, mnemonic, op_str = match.groups()
        instructions.append(format_instruction(mnemonic, op_str))
        byte_lines.append(" ".join(byte_text.split()).lower())

    return {
        "meta": meta,
        "instructions": instructions,
        "mnemonics": [mnemonic_for(insn) for insn in instructions],
        "byte_lines": byte_lines,
    }


def format_instruction(mnemonic: str, op_str: str) -> str:
    op_str = " ".join(op_str.split())
    return f"{mnemonic.lower()} {op_str}".strip()


def mnemonic_for(instruction: str) -> str:
    return instruction.split(" ", 1)[0]


def canonicalize_instruction(instruction: str) -> str:
    instruction = instruction.split(";", 1)[0].strip().lower()
    instruction = re.sub(r"\s+", " ", instruction)
    instruction = NUMERIC_RE.sub("#", instruction)
    return instruction


def lcs_length(left: list[str], right: list[str]) -> int:
    if not left or not right:
        return 0
    previous = [0] * (len(right) + 1)
    for left_item in left:
        current = [0]
        for col, right_item in enumerate(right, start=1):
            if left_item == right_item:
                current.append(previous[col - 1] + 1)
            else:
                current.append(max(previous[col], current[-1]))
        previous = current
    return previous[-1]


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def normalize_asm_text(text: str) -> list[str]:
    out: list[str] = []
    for raw in text.splitlines():
        line = raw.split(";", 1)[0].strip().lower()
        if not line:
            continue
        if line.endswith(":"):
            continue
        if any(line.startswith(prefix) for prefix in ASM_SKIP_PREFIXES):
            continue
        line = re.sub(r"\s+", " ", line)
        out.append(line)
    return out


def read_compiler_asm(path: Path) -> dict[str, object]:
    text = path.read_text(encoding="utf-8", errors="replace")
    parsed: list[str] = []
    byte_lines: list[str] = []
    for line in text.splitlines():
        match = INSN_RE.match(line.strip())
        if not match:
            continue
        _, byte_text, mnemonic, op_str = match.groups()
        parsed.append(format_instruction(mnemonic, op_str))
        byte_lines.append(" ".join(byte_text.split()).lower())

    instructions = parsed if parsed else normalize_asm_text(text)
    return {
        "instructions": instructions,
        "mnemonics": [mnemonic_for(insn) for insn in instructions],
        "byte_lines": byte_lines,
    }


def expand_placeholders(value: str, row: dict[str, str], compiler: dict[str, object], outdir: Path) -> str:
    source = sample_path(row)
    return value.format(
        compiler=compiler["name"],
        outdir=str(outdir),
        sample=row["sample"],
        source=str(source),
        stem=source.stem,
    )


def command_for(command: object, row: dict[str, str], compiler: dict[str, object], outdir: Path) -> list[str]:
    if not isinstance(command, list) or not all(isinstance(x, str) for x in command):
        raise SystemExit(f"compiler {compiler.get('name')}: command entries must be string lists")
    return [expand_placeholders(arg, row, compiler, outdir) for arg in command]


def print_config_template() -> None:
    template = {
        "compilers": [
            {
                "name": "example-borland-tc",
                "workdir": ".",
                "commands": [
                    [
                        "dosbox-x",
                        "-conf",
                        "path/to/dosbox-compiler.conf",
                        "-c",
                        "REM mount compiler tree and compile {source} to {outdir}/{stem}.asm",
                    ]
                ],
                "asm_outputs": ["{outdir}/{stem}.asm"],
            }
        ]
    }
    print(json.dumps(template, indent=2))


def check_corpus(rows: list[dict[str, str]]) -> int:
    errors: list[str] = []
    seen: set[str] = set()
    for row in rows:
        sample = row["sample"]
        if sample in seen:
            errors.append(f"duplicate sample {sample}")
        seen.add(sample)

        src = sample_path(row)
        if not src.exists():
            errors.append(f"{sample}: missing source {src}")
            continue
        text = src.read_text(encoding="utf-8", errors="replace")
        for token in FORBIDDEN_SOURCE_TOKENS:
            if token in text:
                errors.append(f"{sample}: forbidden token {token}")

        asm_path = asm_path_for_target(row["target_routine"])
        if asm_path is None:
            errors.append(f"{sample}: no assembly file for {row['target_routine']}")

        candidate_source = row.get("candidate_source", "")
        if candidate_source and candidate_source != "-":
            candidate_path = REPO_ROOT / candidate_source
            if not candidate_path.exists():
                errors.append(f"{sample}: missing candidate source {candidate_path}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: {len(rows)} compiler-corpus sample(s)")
    return 0


def list_samples(rows: list[dict[str, str]]) -> None:
    fieldnames = list(rows[0].keys()) if rows else [
        "sample",
        "source",
        "target_routine",
        "question",
        "candidate_source",
    ]
    writer = csv.DictWriter(
        sys.stdout,
        fieldnames=fieldnames,
        delimiter="\t",
        lineterminator="\n",
    )
    writer.writeheader()
    writer.writerows(rows)


def original_shapes(rows: list[dict[str, str]]) -> None:
    out = []
    for row in rows:
        asm_path = asm_path_for_target(row["target_routine"])
        if asm_path is None:
            continue
        shape = read_original_shape(asm_path)
        out.append(
            {
                "sample": row["sample"],
                "target_routine": row["target_routine"],
                "asm_path": str(asm_path.relative_to(REPO_ROOT)),
                **shape,
            }
        )
    print(json.dumps(out, indent=2))


def run_compilers(args: argparse.Namespace, rows: list[dict[str, str]]) -> int:
    config = json.loads(Path(args.config).read_text())
    compilers = config.get("compilers", [])
    if not compilers:
        raise SystemExit(f"{args.config}: no compilers configured")

    selected = set(args.sample or [])
    rows_to_run = [row for row in rows if not selected or row["sample"] in selected]
    if selected and len(rows_to_run) != len(selected):
        known = {row["sample"] for row in rows}
        missing = sorted(selected.difference(known))
        raise SystemExit(f"unknown sample(s): {', '.join(missing)}")

    status = 0
    for compiler in compilers:
        if "name" not in compiler:
            raise SystemExit("compiler entry missing name")
        commands = compiler.get("commands", [])
        if not commands:
            raise SystemExit(f"compiler {compiler['name']}: no commands")

        for row in rows_to_run:
            outdir = Path(args.out_dir) / str(compiler["name"]) / row["sample"]
            if not args.dry_run:
                outdir.mkdir(parents=True, exist_ok=True)
            for command in commands:
                expanded = command_for(command, row, compiler, outdir)
                print("+ " + " ".join(expanded))
                if args.dry_run:
                    continue
                proc = subprocess.run(
                    expanded,
                    cwd=expand_placeholders(str(compiler.get("workdir", ".")), row, compiler, outdir),
                    check=False,
                )
                if proc.returncode != 0:
                    print(
                        f"ERROR: {compiler['name']} {row['sample']} command exited {proc.returncode}",
                        file=sys.stderr,
                    )
                    status = 1
                    break

            if args.dry_run:
                continue

            for asm_pattern in compiler.get("asm_outputs", []):
                asm_path = Path(expand_placeholders(str(asm_pattern), row, compiler, outdir))
                if asm_path.exists():
                    normalized = normalize_asm_text(
                        asm_path.read_text(encoding="utf-8", errors="replace")
                    )
                    norm_path = outdir / f"{asm_path.stem}.normalized.asm"
                    norm_path.write_text("\n".join(normalized) + "\n", encoding="utf-8")
                    print(f"normalized {asm_path} -> {norm_path}")
                elif not args.dry_run:
                    print(f"WARN: expected asm output missing: {asm_path}", file=sys.stderr)

    return status


def compare_sequences(original: list[str], compiled: list[str]) -> dict[str, object]:
    original_canon = [canonicalize_instruction(insn) for insn in original]
    compiled_canon = [canonicalize_instruction(insn) for insn in compiled]
    original_mnemonics = [mnemonic_for(insn) for insn in original]
    compiled_mnemonics = [mnemonic_for(insn) for insn in compiled]

    instruction_lcs = lcs_length(original_canon, compiled_canon)
    mnemonic_lcs = lcs_length(original_mnemonics, compiled_mnemonics)
    mnemonic_overlap = sum(
        (Counter(original_mnemonics) & Counter(compiled_mnemonics)).values()
    )
    original_len = len(original_canon)
    compiled_len = len(compiled_canon)

    return {
        "original_instruction_count": original_len,
        "compiled_instruction_count": compiled_len,
        "instruction_lcs": instruction_lcs,
        "instruction_lcs_ratio": round(instruction_lcs / original_len, 4)
        if original_len
        else 0.0,
        "mnemonic_lcs": mnemonic_lcs,
        "mnemonic_lcs_ratio": round(mnemonic_lcs / original_len, 4)
        if original_len
        else 0.0,
        "mnemonic_multiset_overlap": mnemonic_overlap,
        "mnemonic_multiset_overlap_ratio": round(mnemonic_overlap / original_len, 4)
        if original_len
        else 0.0,
    }


def compare_bytes(original: list[str], compiled: list[str]) -> dict[str, object]:
    byte_lcs = lcs_length(original, compiled)
    original_len = len(original)
    compiled_len = len(compiled)
    return {
        "original_byte_line_count": original_len,
        "compiled_byte_line_count": compiled_len,
        "byte_line_lcs": byte_lcs,
        "byte_line_lcs_ratio": round(byte_lcs / original_len, 4)
        if original_len
        else 0.0,
        "has_compiled_bytes": bool(compiled_len),
    }


def compare_outputs(args: argparse.Namespace, rows: list[dict[str, str]]) -> int:
    out_dir = Path(args.out_dir)
    selected_samples = set(args.sample or [])
    selected_compilers = set(args.compiler or [])
    results: list[dict[str, object]] = []

    if selected_samples:
        known = {row["sample"] for row in rows}
        missing = sorted(selected_samples.difference(known))
        if missing:
            raise SystemExit(f"unknown sample(s): {', '.join(missing)}")

    for row in rows:
        if selected_samples and row["sample"] not in selected_samples:
            continue
        asm_path = asm_path_for_target(row["target_routine"])
        if asm_path is None:
            continue
        original = read_original_routine(asm_path)
        sample_root = out_dir
        if not sample_root.exists():
            continue

        for compiler_dir in sorted(p for p in sample_root.iterdir() if p.is_dir()):
            compiler_name = compiler_dir.name
            if selected_compilers and compiler_name not in selected_compilers:
                continue
            sample_dir = compiler_dir / row["sample"]
            if not sample_dir.exists():
                continue
            asm_outputs = sorted(sample_dir.glob("*.normalized.asm"))
            if not asm_outputs:
                asm_outputs = sorted(sample_dir.glob("*.asm"))
            for compiled_path in asm_outputs:
                compiled = read_compiler_asm(compiled_path)
                seq_scores = compare_sequences(
                    original["instructions"], compiled["instructions"]
                )
                byte_scores = compare_bytes(
                    original["byte_lines"], compiled["byte_lines"]
                )
                results.append(
                    {
                        "sample": row["sample"],
                        "compiler": compiler_name,
                        "compiled_asm": display_path(compiled_path),
                        "target_routine": row["target_routine"],
                        "original_asm": display_path(asm_path),
                        **seq_scores,
                        **byte_scores,
                    }
                )

    print(json.dumps(results, indent=2))
    if not results:
        print(
            f"WARN: no compiler outputs found under {out_dir}",
            file=sys.stderr,
        )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate corpus files")
    parser.add_argument("--list", action="store_true", help="list manifest rows")
    parser.add_argument("--original-shapes", action="store_true", help="emit target routine shapes")
    parser.add_argument("--print-config-template", action="store_true")
    parser.add_argument("--config", help="JSON compiler runner config")
    parser.add_argument("--run", action="store_true", help="run configured compilers")
    parser.add_argument("--compare", action="store_true", help="compare compiler outputs with target routines")
    parser.add_argument("--dry-run", action="store_true", help="print configured compiler commands")
    parser.add_argument("--sample", action="append", help="run only this sample; repeatable")
    parser.add_argument("--compiler", action="append", help="compare only this compiler output directory; repeatable")
    parser.add_argument("--out-dir", default=str(DEFAULT_OUT))
    args = parser.parse_args()

    if args.print_config_template:
        print_config_template()
        return 0

    rows = load_manifest()

    ran_action = False
    if args.check:
        ran_action = True
        rc = check_corpus(rows)
        if rc:
            return rc
    if args.list:
        ran_action = True
        list_samples(rows)
    if args.original_shapes:
        ran_action = True
        original_shapes(rows)
    if args.run or args.dry_run:
        ran_action = True
        if not args.config:
            raise SystemExit("--config is required with --run or --dry-run")
        return run_compilers(args, rows)
    if args.compare:
        ran_action = True
        return compare_outputs(args, rows)

    if not ran_action:
        parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
