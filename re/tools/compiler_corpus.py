#!/usr/bin/env python3
"""Run natural-C compiler codegen probes for BLOODPRG and XDB source recovery.

The probes are not recovered game source. They are small C samples used to
compare candidate historical DOS compiler output against recovered assembly
shapes before accepting any natural-C routine.
"""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import argparse
import csv
import json
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
CORPUS_ROOT = REPO_ROOT / "re" / "compiler_corpus"
MANIFEST = CORPUS_ROOT / "manifest.tsv"
DEFAULT_OUT = CORPUS_ROOT / "out"
DOS_PROBE_SOURCE = "PROBE.C"
DOS_PROBE_ASSEMBLY = "PROBE.ASM"
DOS_PROBE_LOG = "COMPILE.TXT"
DOS_PROBE_OBJECT = "PROBE.OBJ"

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
    r"^([0-9a-fA-F]{4,8}):?\s+((?:[0-9a-fA-F]{2}\s+)+)\s*([A-Za-z][A-Za-z0-9]*)\s*(.*)$"
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
    if ":" in target:
        module, address = target.split(":", 1)
        if not module.startswith("xdb_"):
            return None
        assembly_root = (
            REPO_ROOT / "re" / "assembly" / "xdb" / module.removeprefix("xdb_")
        )
    else:
        address = target
        assembly_root = REPO_ROOT / "re" / "assembly" / "bloodprg"

    try:
        needle = f"func_{int(address, 16):06x}_"
    except ValueError:
        return None
    matches = sorted(assembly_root.glob(f"**/{needle}*.asm"))
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
    cleaned = [raw.split(";", 1)[0].strip().lower() for raw in text.splitlines()]
    has_procedures = any(re.search(r"\bproc\b", line) for line in cleaned)
    in_procedure = not has_procedures
    far_procedure = False
    out: list[str] = []

    for line in cleaned:
        if not line:
            continue
        if re.search(r"\bproc\b", line):
            in_procedure = True
            far_procedure = bool(re.search(r"\bproc\s+far\b", line))
            continue
        if re.search(r"\bendp\b", line):
            in_procedure = False
            far_procedure = False
            continue
        if not in_procedure or line.endswith(":"):
            continue
        if any(line.startswith(prefix) for prefix in ASM_SKIP_PREFIXES):
            continue
        line = re.sub(r"\s+", " ", line)
        if far_procedure and line == "ret":
            line = "retf"
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


def expand_placeholders(
    value: str, row: dict[str, str], compiler: dict[str, object], outdir: Path
) -> str:
    source = sample_path(row)
    return value.format(
        compiler=compiler["name"],
        outdir=str(outdir),
        sample=row["sample"],
        source=str(source),
        stem=source.stem,
    )


def command_for(
    command: object, row: dict[str, str], compiler: dict[str, object], outdir: Path
) -> list[str]:
    if not isinstance(command, list) or not all(isinstance(x, str) for x in command):
        raise SystemExit(
            f"compiler {compiler.get('name')}: command entries must be string lists"
        )
    return [expand_placeholders(arg, row, compiler, outdir) for arg in command]


def parse_named_path(value: str, option: str) -> tuple[str, Path]:
    if "=" not in value:
        raise SystemExit(f"{option} must be LABEL=PATH")
    label, raw_path = value.split("=", 1)
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", label) or not raw_path:
        raise SystemExit(f"{option} must be LABEL=PATH with a filesystem-safe label")
    path = Path(raw_path).expanduser()
    return label, path


def selected_rows(
    rows: list[dict[str, str]], samples: list[str] | None
) -> list[dict[str, str]]:
    selected = set(samples or [])
    rows_to_run = [row for row in rows if not selected or row["sample"] in selected]
    if selected and len(rows_to_run) != len(selected):
        known = {row["sample"] for row in rows}
        missing = sorted(selected.difference(known))
        raise SystemExit(f"unknown sample(s): {', '.join(missing)}")
    return rows_to_run


def run_dosbox_turbo_c(
    dosbox: Path,
    toolchain: Path,
    source: Path,
    outdir: Path,
    flags: list[str],
) -> tuple[list[str], str]:
    tcc = toolchain / "TC" / "TCC.EXE"
    include = toolchain / "TC" / "INCLUDE"
    if not tcc.is_file() or not include.is_dir():
        raise SystemExit(
            f"Turbo C toolchain must contain TC/TCC.EXE and TC/INCLUDE: {toolchain}"
        )
    if not dosbox.is_file():
        resolved = shutil.which(str(dosbox))
        if resolved is None:
            raise SystemExit(f"DOSBox executable not found: {dosbox}")
        dosbox = Path(resolved)

    outdir.mkdir(parents=True, exist_ok=True)
    staged_source = outdir / DOS_PROBE_SOURCE
    assembly = outdir / DOS_PROBE_ASSEMBLY
    log = outdir / DOS_PROBE_LOG
    for stale in (staged_source, assembly, log):
        if stale.exists():
            stale.unlink()
    source_bytes = source.read_bytes()
    source_bytes = source_bytes.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    staged_source.write_bytes(source_bytes.replace(b"\n", b"\r\n"))

    dos_command = (
        " ".join([r"C:\TC\TCC.EXE", "-S", r"-IC:\TC\INCLUDE", *flags, DOS_PROBE_SOURCE])
        + f" > {DOS_PROBE_LOG}"
    )
    command = [
        str(dosbox),
        "--noprimaryconf",
        "--nolocalconf",
        "--exit",
        "-set",
        "sdl fullscreen=false",
        "-set",
        "sdl output=texture",
        "-c",
        f'mount c "{toolchain.resolve()}"',
        "-c",
        f'mount d "{outdir.resolve()}"',
        "-c",
        r"set PATH=C:\TC",
        "-c",
        "d:",
        "-c",
        dos_command,
    ]
    env = os.environ.copy()
    env["SDL_AUDIODRIVER"] = "dummy"
    env["SDL_VIDEODRIVER"] = "offscreen"
    proc = subprocess.run(command, check=False, capture_output=True, env=env)
    if proc.returncode != 0:
        raise RuntimeError(
            f"DOSBox exited {proc.returncode}: {proc.stderr.decode(errors='replace')}"
        )
    compiler_output = (
        log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    )
    if not assembly.is_file():
        raise RuntimeError(
            f"Turbo C did not create {assembly}; compiler output:\n{compiler_output}"
        )
    return command, compiler_output.replace("\r", "")


def run_turbo_c_compilers(args: argparse.Namespace, rows: list[dict[str, str]]) -> int:
    compiler_specs = args.turbo_c or []
    if not compiler_specs:
        raise SystemExit("--turbo-c LABEL=PATH is required with --run-turbo-c")
    flags = args.flag if args.flag is not None else ["-mh", "-O", "-Z"]
    for flag in flags:
        if not re.fullmatch(r"-[A-Za-z0-9+_.-]+", flag):
            raise SystemExit(f"unsafe or malformed Turbo C flag: {flag!r}")

    status = 0
    for label, toolchain in (
        parse_named_path(value, "--turbo-c") for value in compiler_specs
    ):
        for row in selected_rows(rows, args.sample):
            outdir = Path(args.out_dir) / label / row["sample"]
            print(
                f"+ {label} {row['sample']}: TCC.EXE -S {' '.join(flags)} {DOS_PROBE_SOURCE}"
            )
            if args.dry_run:
                continue
            try:
                _, compiler_output = run_dosbox_turbo_c(
                    Path(args.dosbox),
                    toolchain,
                    sample_path(row),
                    outdir,
                    flags,
                )
            except (OSError, RuntimeError) as error:
                print(f"ERROR: {label} {row['sample']}: {error}", file=sys.stderr)
                status = 1
                continue

            assembly = outdir / DOS_PROBE_ASSEMBLY
            normalized = normalize_asm_text(
                assembly.read_text(encoding="utf-8", errors="replace")
            )
            normalized_path = outdir / "PROBE.normalized.asm"
            normalized_path.write_text("\n".join(normalized) + "\n", encoding="utf-8")
            print(
                f"generated {display_path(assembly)}; "
                f"compiler log {len(compiler_output)} byte(s)"
            )
    return status


def resolve_watcom_tools(path: Path) -> tuple[Path, Path]:
    if path.is_dir():
        wcc = path / "wcc"
        wdis = path / "wdis"
    else:
        resolved = shutil.which(str(path))
        wcc = Path(resolved) if resolved is not None else path
        wdis = wcc.parent / "wdis"
    if not wcc.is_file() or not wdis.is_file():
        raise SystemExit(
            f"Watcom path must be a bin directory containing wcc and wdis, "
            f"or the wcc executable: {path}"
        )
    return wcc.resolve(), wdis.resolve()


def run_watcom_c(
    wcc: Path,
    wdis: Path,
    source: Path,
    outdir: Path,
    flags: list[str],
) -> tuple[list[str], str]:
    outdir.mkdir(parents=True, exist_ok=True)
    staged_source = outdir / DOS_PROBE_SOURCE
    assembly = outdir / DOS_PROBE_ASSEMBLY
    object_file = outdir / DOS_PROBE_OBJECT
    log = outdir / DOS_PROBE_LOG
    normalized = outdir / "PROBE.normalized.asm"
    for stale in (staged_source, assembly, object_file, log, normalized):
        if stale.exists():
            stale.unlink()
    shutil.copyfile(source, staged_source)

    compile_command = [
        str(wcc),
        *flags,
        f"-fo={DOS_PROBE_OBJECT}",
        DOS_PROBE_SOURCE,
    ]
    compile_proc = subprocess.run(
        compile_command,
        cwd=outdir,
        check=False,
        capture_output=True,
        text=True,
    )
    compiler_output = compile_proc.stdout + compile_proc.stderr
    log.write_text(compiler_output, encoding="utf-8")
    if compile_proc.returncode != 0 or not object_file.is_file():
        raise RuntimeError(
            f"wcc exited {compile_proc.returncode}; compiler output:\n{compiler_output}"
        )

    disassemble_command = [
        str(wdis),
        f"-l={DOS_PROBE_ASSEMBLY}",
        DOS_PROBE_OBJECT,
    ]
    disassemble_proc = subprocess.run(
        disassemble_command,
        cwd=outdir,
        check=False,
        capture_output=True,
        text=True,
    )
    if disassemble_proc.returncode != 0 or not assembly.is_file():
        output = disassemble_proc.stdout + disassemble_proc.stderr
        raise RuntimeError(
            f"wdis exited {disassemble_proc.returncode}; disassembler output:\n{output}"
        )
    return compile_command + ["&&"] + disassemble_command, compiler_output


def run_watcom_compilers(args: argparse.Namespace, rows: list[dict[str, str]]) -> int:
    compiler_specs = args.watcom or []
    if not compiler_specs:
        raise SystemExit("--watcom LABEL=PATH is required with --run-watcom")
    flags = args.flag if args.flag is not None else ["-3", "-ox", "-mh"]
    for flag in flags:
        if not re.fullmatch(r"-[A-Za-z0-9+=_.-]+", flag):
            raise SystemExit(f"unsafe or malformed Watcom C flag: {flag!r}")

    status = 0
    for label, path in (
        parse_named_path(value, "--watcom") for value in compiler_specs
    ):
        wcc, wdis = resolve_watcom_tools(path)
        for row in selected_rows(rows, args.sample):
            outdir = Path(args.out_dir) / label / row["sample"]
            print(
                f"+ {label} {row['sample']}: wcc {' '.join(flags)} "
                f"-fo={DOS_PROBE_OBJECT} {DOS_PROBE_SOURCE}; wdis"
            )
            if args.dry_run:
                continue
            try:
                _, compiler_output = run_watcom_c(
                    wcc,
                    wdis,
                    sample_path(row),
                    outdir,
                    flags,
                )
            except (OSError, RuntimeError) as error:
                print(f"ERROR: {label} {row['sample']}: {error}", file=sys.stderr)
                status = 1
                continue

            assembly = outdir / DOS_PROBE_ASSEMBLY
            compiled = read_compiler_asm(assembly)
            normalized_path = outdir / "PROBE.normalized.asm"
            normalized_path.write_text(
                "\n".join(compiled["instructions"]) + "\n",
                encoding="utf-8",
            )
            print(
                f"generated {display_path(assembly)} with object-code bytes; "
                f"compiler log {len(compiler_output)} byte(s)"
            )
    return status


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
    fieldnames = (
        list(rows[0].keys())
        if rows
        else [
            "sample",
            "source",
            "target_routine",
            "question",
            "candidate_source",
        ]
    )
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


def scan_library_routines(args: argparse.Namespace) -> int:
    routines: list[tuple[Path, bytes]] = []
    assembly_root = REPO_ROOT / "re" / "assembly" / "bloodprg"
    for asm_path in sorted(assembly_root.glob("**/func_*.asm")):
        routine = read_original_routine(asm_path)
        blob = b"".join(bytes.fromhex(byte_line) for byte_line in routine["byte_lines"])
        if len(blob) >= args.min_routine_bytes:
            routines.append((asm_path, blob))

    results: list[dict[str, object]] = []
    status = 0
    for value in args.scan_library:
        label, root = parse_named_path(value, "--scan-library")
        if not root.is_dir():
            print(f"ERROR: library root is not a directory: {root}", file=sys.stderr)
            status = 1
            continue
        files = sorted(path for path in root.rglob("*") if path.is_file())
        matches: list[dict[str, object]] = []
        for file_path in files:
            try:
                library_blob = file_path.read_bytes()
            except OSError as error:
                print(f"WARN: cannot read {file_path}: {error}", file=sys.stderr)
                continue
            for asm_path, routine_blob in routines:
                offset = library_blob.find(routine_blob)
                if offset >= 0:
                    matches.append(
                        {
                            "routine": asm_path.stem,
                            "routine_asm": display_path(asm_path),
                            "routine_bytes": len(routine_blob),
                            "library_file": str(file_path),
                            "library_offset": f"0x{offset:x}",
                        }
                    )
        results.append(
            {
                "label": label,
                "library_root": str(root),
                "files_scanned": len(files),
                "routines_scanned": len(routines),
                "minimum_routine_bytes": args.min_routine_bytes,
                "exact_match_count": len(matches),
                "matches": matches,
            }
        )
    print(json.dumps(results, indent=2))
    return status


def run_compilers(args: argparse.Namespace, rows: list[dict[str, str]]) -> int:
    config = json.loads(Path(args.config).read_text())
    compilers = config.get("compilers", [])
    if not compilers:
        raise SystemExit(f"{args.config}: no compilers configured")

    rows_to_run = selected_rows(rows, args.sample)

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
                    cwd=expand_placeholders(
                        str(compiler.get("workdir", ".")), row, compiler, outdir
                    ),
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
                asm_path = Path(
                    expand_placeholders(str(asm_pattern), row, compiler, outdir)
                )
                if asm_path.exists():
                    normalized = normalize_asm_text(
                        asm_path.read_text(encoding="utf-8", errors="replace")
                    )
                    norm_path = outdir / f"{asm_path.stem}.normalized.asm"
                    norm_path.write_text("\n".join(normalized) + "\n", encoding="utf-8")
                    print(f"normalized {asm_path} -> {norm_path}")
                elif not args.dry_run:
                    print(
                        f"WARN: expected asm output missing: {asm_path}",
                        file=sys.stderr,
                    )

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
        "instruction_count_delta": compiled_len - original_len,
        "mnemonic_sequence_exact": original_mnemonics == compiled_mnemonics,
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
            asm_outputs = sorted(
                path
                for path in sample_dir.iterdir()
                if path.is_file()
                and path.suffix.lower() == ".asm"
                and not path.name.endswith(".normalized.asm")
            )
            if not asm_outputs:
                asm_outputs = sorted(
                    path
                    for path in sample_dir.iterdir()
                    if path.is_file() and path.name.endswith(".normalized.asm")
                )
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
    parser.add_argument(
        "--original-shapes", action="store_true", help="emit target routine shapes"
    )
    parser.add_argument(
        "--scan-library",
        action="append",
        metavar="LABEL=PATH",
        help="find exact recovered BLOODPRG routine bytes under a compiler library tree",
    )
    parser.add_argument("--min-routine-bytes", type=int, default=8)
    parser.add_argument("--print-config-template", action="store_true")
    parser.add_argument("--config", help="JSON compiler runner config")
    parser.add_argument("--run", action="store_true", help="run configured compilers")
    parser.add_argument(
        "--run-turbo-c",
        action="store_true",
        help="run archived Turbo C directly through DOSBox",
    )
    parser.add_argument(
        "--turbo-c",
        action="append",
        metavar="LABEL=PATH",
        help="installed Turbo C tree; repeatable",
    )
    parser.add_argument(
        "--run-watcom",
        action="store_true",
        help="run native Open Watcom C16 and disassemble its OMF objects",
    )
    parser.add_argument(
        "--watcom",
        action="append",
        metavar="LABEL=PATH",
        help="Open Watcom bin directory or wcc executable; repeatable",
    )
    parser.add_argument("--dosbox", default="dosbox-staging")
    parser.add_argument(
        "--flag",
        action="append",
        help="compiler flag; repeatable (Turbo defaults: -mh -O -Z; Watcom: -3 -ox -mh)",
    )
    parser.add_argument(
        "--compare",
        action="store_true",
        help="compare compiler outputs with target routines",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="print configured compiler commands"
    )
    parser.add_argument(
        "--sample", action="append", help="run only this sample; repeatable"
    )
    parser.add_argument(
        "--compiler",
        action="append",
        help="compare only this compiler output directory; repeatable",
    )
    parser.add_argument("--out-dir", default=str(DEFAULT_OUT))
    args = parser.parse_args()

    runner_count = sum(
        (
            bool(args.config),
            bool(args.turbo_c) or args.run_turbo_c,
            bool(args.watcom) or args.run_watcom,
        )
    )
    if runner_count > 1:
        raise SystemExit("choose one configured, Turbo C, or Watcom compiler runner")

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
    if args.scan_library:
        ran_action = True
        if args.min_routine_bytes < 1:
            raise SystemExit("--min-routine-bytes must be positive")
        rc = scan_library_routines(args)
        if rc:
            return rc
    if args.run:
        ran_action = True
        if not args.config:
            raise SystemExit("--config is required with --run")
        return run_compilers(args, rows)
    if args.run_turbo_c:
        ran_action = True
        return run_turbo_c_compilers(args, rows)
    if args.run_watcom:
        ran_action = True
        return run_watcom_compilers(args, rows)
    if args.dry_run:
        ran_action = True
        if args.config:
            return run_compilers(args, rows)
        if args.turbo_c:
            return run_turbo_c_compilers(args, rows)
        if args.watcom:
            return run_watcom_compilers(args, rows)
        raise SystemExit("--config, --turbo-c, or --watcom is required with --dry-run")
    if args.compare:
        ran_action = True
        return compare_outputs(args, rows)

    if not ran_action:
        parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
