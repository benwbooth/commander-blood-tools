#!/usr/bin/env python3
"""Prove that Watcom ``modify exact`` declarations cover emitted clobbers.

Open Watcom uses these declarations at each separately compiled call site.  An
understated list lets a caller keep live state in a register that the callee
actually destroys.  This audit derives each recovered function's transitive
emitted register effects and fails closed on both understatements and effects
that cannot yet be proved.
"""

from __future__ import annotations

import argparse
import csv
from dataclasses import dataclass
import importlib.util
import io
import os
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]


def _load_register_audit():
    path = ROOT / "re/tools/audit_relinked_register_contracts.py"
    spec = importlib.util.spec_from_file_location(
        "watcom_modify_register_audit", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


REGISTERS = _load_register_audit()
REGISTER_TOKEN = re.compile(
    r"\b(?:e?[abcd]x|e?[sd]i|e?[bs]p|[abcd][lh]|[sd]i|[bs]p|[defg]s|ss)\b",
    re.IGNORECASE,
)
REGISTER_PARENT = {
    "AL": "AX", "AH": "AX", "EAX": "AX",
    "BL": "BX", "BH": "BX", "EBX": "BX",
    "CL": "CX", "CH": "CX", "ECX": "CX",
    "DL": "DX", "DH": "DX", "EDX": "DX",
    "ESI": "SI", "EDI": "DI", "EBP": "BP", "ESP": "SP",
}


@dataclass(frozen=True)
class ModifyContract:
    function: str
    modifies: frozenset[str]
    header: Path
    line: int


@dataclass(frozen=True)
class ContractResult:
    function: str
    routine: str
    status: str
    declared: str
    emitted: str
    underdeclared: str
    overdeclared: str
    source: str
    blockers: str


def _condition_value(expression: str, defines: frozenset[str]) -> bool:
    value = expression.strip()
    value = re.sub(
        r"!\s*defined\s*\(\s*(\w+)\s*\)",
        lambda match: str(match.group(1) not in defines),
        value,
    )
    value = re.sub(
        r"defined\s*\(\s*(\w+)\s*\)",
        lambda match: str(match.group(1) in defines),
        value,
    )
    value = re.sub(
        r"!\s*defined\s+(\w+)",
        lambda match: str(match.group(1) not in defines),
        value,
    )
    value = re.sub(
        r"defined\s+(\w+)",
        lambda match: str(match.group(1) in defines),
        value,
    )
    value = re.sub(
        r"\b[A-Za-z_]\w*\b",
        lambda match: (
            match.group(0)
            if match.group(0) in ("True", "False")
            else str(match.group(0) in defines)
        ),
        value,
    )
    value = value.replace("&&", " and ").replace("||", " or ")
    value = re.sub(r"!(?!=)", " not ", value)
    words = set(re.findall(r"[A-Za-z_]+", value))
    if words - {"True", "False", "and", "or", "not"} or re.search(
        r"[^A-Za-z0-9_()\s]", value
    ):
        raise ValueError(f"unsupported preprocessor condition: {expression}")
    return bool(eval(value, {"__builtins__": {}}, {}))


def active_logical_lines(
    path: Path, defines: frozenset[str]
) -> list[tuple[int, str]]:
    physical = path.read_text(encoding="ascii").splitlines()
    active = True
    # parent-active, prior-branch-taken, this-branch-active
    stack: list[tuple[bool, bool, bool]] = []
    logical: list[tuple[int, str]] = []
    index = 0
    while index < len(physical):
        first_line = index + 1
        line = physical[index]
        while line.rstrip().endswith("\\") and index + 1 < len(physical):
            line = line.rstrip()[:-1] + " " + physical[index + 1].lstrip()
            index += 1
        stripped = line.strip()
        directive = re.match(r"#\s*(\w+)\b(.*)", stripped)
        if directive is not None:
            kind = directive.group(1)
            argument = directive.group(2).strip()
            if kind in ("if", "ifdef", "ifndef"):
                parent = active
                if kind == "ifdef":
                    branch = argument in defines
                elif kind == "ifndef":
                    branch = argument not in defines
                else:
                    branch = _condition_value(argument, defines)
                current = parent and branch
                stack.append((parent, current, current))
                active = current
            elif kind == "elif":
                if not stack:
                    raise ValueError(f"{path}:{first_line}: unmatched #elif")
                parent, taken, _current = stack[-1]
                current = parent and not taken and _condition_value(argument, defines)
                stack[-1] = (parent, taken or current, current)
                active = current
            elif kind == "else":
                if not stack:
                    raise ValueError(f"{path}:{first_line}: unmatched #else")
                parent, taken, _current = stack[-1]
                current = parent and not taken
                stack[-1] = (parent, True, current)
                active = current
            elif kind == "endif":
                if not stack:
                    raise ValueError(f"{path}:{first_line}: unmatched #endif")
                parent, _taken, _current = stack.pop()
                active = parent
            elif active:
                logical.append((first_line, line))
        elif active:
            logical.append((first_line, line))
        index += 1
    if stack:
        raise ValueError(f"{path}: unterminated preprocessor condition")
    return logical


def normalize_registers(text: str) -> frozenset[str]:
    return frozenset(
        REGISTER_PARENT.get(token.upper(), token.upper())
        for token in REGISTER_TOKEN.findall(text)
    )


def parse_contracts(
    header_dir: Path,
    defines: frozenset[str] = frozenset(("__WATCOMC__", "BLOODPRG_RELINKED_RUNTIME")),
) -> dict[str, ModifyContract]:
    contracts: dict[str, ModifyContract] = {}
    for path in sorted(header_dir.glob("*.h")):
        for line_number, line in active_logical_lines(path, defines):
            pragma = re.match(r"\s*#\s*pragma\s+aux\s+(\w+)\b(.*)", line)
            if pragma is None:
                continue
            modify = re.search(
                r"\bmodify\s+exact\s*\[([^]]*)\]", pragma.group(2)
            )
            if modify is None:
                continue
            contract = ModifyContract(
                pragma.group(1), normalize_registers(modify.group(1)),
                path, line_number,
            )
            previous = contracts.get(contract.function)
            if (
                previous is not None
                and previous.modifies != contract.modifies
            ):
                raise ValueError(
                    f"conflicting modify contracts for {contract.function}: "
                    f"{previous.header}:{previous.line} and {path}:{line_number}"
                )
            contracts.setdefault(contract.function, contract)
    return contracts


def audit_contracts(
    contracts: dict[str, ModifyContract],
    manifest_rows: list[dict[str, str]],
    summaries: dict[str, object],
) -> list[ContractResult]:
    results: list[ContractResult] = []
    all_registers = set(REGISTERS.CONTRACT_REGISTERS)
    for row in manifest_rows:
        function = row["function"]
        contract = contracts.get(function)
        if contract is None:
            continue
        routine = Path(row["source"]).stem.lower()
        summary = summaries.get(routine)
        declared = set(contract.modifies)
        if summary is None:
            results.append(ContractResult(
                function, routine, "missing", ",".join(sorted(declared)),
                "", "", "", f"{contract.header}:{contract.line}",
                "emitted routine summary is missing",
            ))
            continue
        emitted = all_registers - set(summary.preserved)
        underdeclared = emitted - declared
        overdeclared = declared - emitted
        blockers = sorted(summary.blockers)
        if blockers:
            status = "unresolved"
        elif underdeclared:
            status = "underdeclared"
        else:
            status = "pass"
        results.append(ContractResult(
            function, routine, status, ",".join(sorted(declared)),
            ",".join(sorted(emitted)), ",".join(sorted(underdeclared)),
            ",".join(sorted(overdeclared)),
            f"{contract.header}:{contract.line}", " | ".join(blockers),
        ))
    return results


def render_tsv(results: list[ContractResult]) -> str:
    output = io.StringIO()
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "function", "routine", "status", "declared_modifies",
        "emitted_clobbers", "underdeclared", "overdeclared", "source",
        "blockers",
    ))
    for result in results:
        writer.writerow((
            result.function, result.routine, result.status, result.declared,
            result.emitted, result.underdeclared, result.overdeclared,
            result.source, result.blockers,
        ))
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest", type=Path,
        default=ROOT / "re/source/bloodprg/candidates/manifest.tsv",
    )
    parser.add_argument(
        "--header-dir", type=Path,
        default=ROOT / "re/source/bloodprg/candidates/include",
    )
    parser.add_argument(
        "--original-image", type=Path, default=ROOT / "re/bin/BLOODPRG.EXE"
    )
    parser.add_argument("--emitted-image", type=Path, required=True)
    parser.add_argument("--link-map", type=Path, required=True)
    parser.add_argument("--listing-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        contracts = parse_contracts(args.header_dir.resolve())
        manifest_rows = REGISTERS.read_manifest(args.manifest.resolve())
        _original, emitted, errors = REGISTERS.load_programs(
            args.manifest.resolve(), args.original_image.resolve(),
            args.emitted_image.resolve(), args.link_map.resolve(),
            args.listing_dir.resolve(),
        )
        summaries = REGISTERS.summarize_program(emitted) if not errors else {}
        results = audit_contracts(contracts, manifest_rows, summaries)
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    report = render_tsv(results)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")

    failures = [result for result in results if result.status != "pass"]
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    for result in failures:
        detail = result.blockers or result.underdeclared or result.status
        print(
            f"ERROR: {result.function}: {result.status}: {detail}",
            file=sys.stderr,
        )
    if errors or failures:
        return 1
    print(f"OK: {len(results)} Watcom modify-exact contracts cover emitted effects")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
