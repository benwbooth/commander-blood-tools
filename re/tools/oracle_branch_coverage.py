#!/usr/bin/env python3
"""Branch-edge coverage reporting for direct 16-bit routine oracles."""

from __future__ import annotations

import csv
from dataclasses import dataclass
import hashlib
from pathlib import Path

import capstone
from capstone.x86_const import X86_OP_IMM


CONDITIONAL_BRANCHES = {
    "ja",
    "jae",
    "jb",
    "jbe",
    "jc",
    "jcxz",
    "jecxz",
    "je",
    "jg",
    "jge",
    "jl",
    "jle",
    "jna",
    "jnae",
    "jnb",
    "jnbe",
    "jnc",
    "jne",
    "jng",
    "jnge",
    "jnl",
    "jnle",
    "jno",
    "jnp",
    "jns",
    "jnz",
    "jo",
    "jp",
    "jpe",
    "jpo",
    "js",
    "jz",
    "loop",
    "loope",
    "loopne",
    "loopnz",
    "loopz",
}


@dataclass(frozen=True)
class Routine:
    module: str
    entry: int
    body_start: int
    function: str
    status: str
    asm_path: Path
    byte_count: int


class CoverageRecorder:
    """Aggregate normalized instruction and edge coverage across executions."""

    def __init__(
        self,
        image_sizes: dict[int, str],
        canonical_images: dict[str, bytes] | None = None,
    ) -> None:
        if len(image_sizes.values()) != len(set(image_sizes.values())):
            raise ValueError("each XDB module must have one unique image size")
        self._image_sizes = dict(image_sizes)
        self._canonical_images = canonical_images or {}
        self.instructions: dict[str, set[int]] = {
            module: set() for module in image_sizes.values()
        }
        self.edges: dict[str, set[tuple[int, int]]] = {
            module: set() for module in image_sizes.values()
        }

    def hook_for(
        self,
        image: bytes,
        code_segment: int,
        terminal_offset: int | None = None,
    ):
        try:
            module = self._image_sizes[len(image)]
        except KeyError as error:
            raise ValueError(
                f"cannot identify {len(image)}-byte XDB image for coverage"
            ) from error
        code_base = code_segment * 16
        previous: int | None = None

        def record(_machine, address: int, size: int, _data: object) -> None:
            nonlocal previous
            normalized = address - code_base
            if not 0 <= normalized < len(image):
                previous = None
                return
            if normalized == terminal_offset:
                if previous is not None:
                    self.edges[module].add((previous, normalized))
                previous = None
                return
            canonical = self._canonical_images.get(module)
            if (
                canonical is not None
                and image[normalized : normalized + size]
                != canonical[normalized : normalized + size]
            ):
                if previous is not None:
                    self.edges[module].add((previous, normalized))
                previous = None
                return
            self.instructions[module].add(normalized)
            if previous is not None:
                self.edges[module].add((previous, normalized))
            previous = normalized

        return record

    def record_trace(self, module: str, addresses: list[int]) -> None:
        """Record a normalized trace directly; used by focused tests."""
        previous: int | None = None
        for address in addresses:
            self.instructions[module].add(address)
            if previous is not None:
                self.edges[module].add((previous, address))
            previous = address


def _metadata_value(path: Path, *keys: str) -> str:
    prefixes = tuple(f"; {key}:" for key in keys)
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            for prefix in prefixes:
                if line.startswith(prefix):
                    return line[len(prefix) :].strip()
            if line and not line.startswith(";"):
                break
    raise ValueError(f"{path}: missing {' or '.join(keys)} metadata")


def load_routines(repo_root: Path, manifest: Path) -> list[Routine]:
    routines: list[Routine] = []
    with manifest.open(encoding="ascii", newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            module_name, entry_text = row["entry"].split(":", 1)
            asm_path = repo_root / row["asm_path"]
            entry = int(entry_text, 0)
            body_start = int(_metadata_value(asm_path, "overlay_offset"), 0)
            try:
                metadata_entry = int(_metadata_value(asm_path, "routine_entry"), 0)
            except ValueError:
                metadata_entry = body_start
            if metadata_entry != entry:
                raise ValueError(
                    f"{asm_path}: entry {metadata_entry:#x} does not match "
                    f"manifest {entry:#x}"
                )
            routines.append(
                Routine(
                    module=module_name.removeprefix("xdb_"),
                    entry=entry,
                    body_start=body_start,
                    function=row["function"],
                    status=row["status"],
                    asm_path=asm_path,
                    byte_count=int(_metadata_value(asm_path, "byte_count"), 0),
                )
            )
    return routines


def _decode_routine(image: bytes, routine: Routine) -> list[capstone.CsInsn]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    body = image[routine.body_start : routine.body_start + routine.byte_count]
    instructions = list(decoder.disasm(body, routine.body_start))
    if not instructions or instructions[0].address != routine.body_start:
        raise ValueError(f"{routine.asm_path}: routine body did not decode at its start")
    decoded_bytes = sum(instruction.size for instruction in instructions)
    if decoded_bytes != routine.byte_count:
        raise ValueError(
            f"{routine.asm_path}: decoded {decoded_bytes} of "
            f"{routine.byte_count} bytes"
        )
    return instructions


def _conditional_edges(
    instructions: list[capstone.CsInsn],
) -> list[tuple[int, int, int]]:
    branches: list[tuple[int, int, int]] = []
    for instruction in instructions:
        if instruction.mnemonic.lower() not in CONDITIONAL_BRANCHES:
            continue
        if (
            len(instruction.operands) != 1
            or instruction.operands[0].type != X86_OP_IMM
        ):
            raise ValueError(
                f"{instruction.address:#x}: conditional branch has no direct target"
            )
        branches.append(
            (
                instruction.address,
                instruction.operands[0].imm & 0xFFFF,
                instruction.address + instruction.size,
            )
        )
    return branches


REPORT_FIELDS = (
    "module",
    "entry",
    "function",
    "oracle_status",
    "coverage_status",
    "instruction_count",
    "executed_instruction_count",
    "conditional_branch_count",
    "branch_edge_count",
    "covered_branch_edge_count",
    "missing_branch_edges",
)


def build_report(
    repo_root: Path,
    manifest: Path,
    images: dict[str, bytes],
    coverage: CoverageRecorder,
) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for routine in load_routines(repo_root, manifest):
        instructions = _decode_routine(images[routine.module], routine)
        instruction_addresses = {instruction.address for instruction in instructions}
        executed = instruction_addresses & coverage.instructions[routine.module]
        branches = _conditional_edges(instructions)
        expected_edges = {
            (source, destination)
            for source, taken, fallthrough in branches
            for destination in (taken, fallthrough)
        }
        covered_edges = expected_edges & coverage.edges[routine.module]
        missing_edges = sorted(expected_edges - covered_edges)
        directly_verified = "oracle_verified" in routine.status
        if not directly_verified:
            coverage_status = "no_direct_original_execution"
        elif not executed:
            coverage_status = "not_executed"
        elif missing_edges:
            coverage_status = "branch_incomplete"
        else:
            coverage_status = "complete"
        rows.append(
            {
                "module": routine.module,
                "entry": f"0x{routine.entry:06x}",
                "function": routine.function,
                "oracle_status": routine.status,
                "coverage_status": coverage_status,
                "instruction_count": str(len(instructions)),
                "executed_instruction_count": str(len(executed)),
                "conditional_branch_count": str(len(branches)),
                "branch_edge_count": str(len(expected_edges)),
                "covered_branch_edge_count": str(len(covered_edges)),
                "missing_branch_edges": (
                    ",".join(
                        f"0x{source:04x}->0x{destination:04x}"
                        for source, destination in missing_edges
                    )
                    or "-"
                ),
            }
        )
    return rows


def encode_report(rows: list[dict[str, str]]) -> str:
    from io import StringIO

    output = StringIO()
    writer = csv.DictWriter(
        output, fieldnames=REPORT_FIELDS, delimiter="\t", lineterminator="\n"
    )
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue()


def update_report(path: Path, rows: list[dict[str, str]], check: bool) -> None:
    encoded = encode_report(rows)
    if check:
        if not path.is_file() or path.read_text(encoding="ascii") != encoded:
            raise SystemExit(f"{path}: stale or missing; regenerate without --check")
    else:
        path.write_text(encoded, encoding="ascii")


def require_complete_direct_coverage(rows: list[dict[str, str]]) -> None:
    incomplete = [
        row
        for row in rows
        if "oracle_verified" in row["oracle_status"]
        and row["coverage_status"] != "complete"
    ]
    if incomplete:
        examples = ", ".join(
            f"{row['module']}:{row['entry']}={row['coverage_status']}"
            for row in incomplete[:8]
        )
        raise SystemExit(
            f"{len(incomplete)} directly verified XDB routines have incomplete "
            f"branch coverage: {examples}"
        )


def require_reviewed_direct_coverage(
    rows: list[dict[str, str]], reviews_path: Path
) -> None:
    reviews: dict[tuple[str, str], dict[str, str]] = {}
    with reviews_path.open(encoding="ascii", newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            key = (row["module"], row["entry"])
            if key in reviews:
                raise SystemExit(
                    f"duplicate XDB branch coverage review: {key[0]}:{key[1]}"
                )
            reviews[key] = row

    incomplete = {
        (row["module"], row["entry"]): row
        for row in rows
        if "oracle_verified" in row["oracle_status"]
        and row["coverage_status"] == "branch_incomplete"
    }
    not_executed = [
        row
        for row in rows
        if "oracle_verified" in row["oracle_status"]
        and row["coverage_status"] == "not_executed"
    ]
    if not_executed:
        raise SystemExit(
            "directly verified XDB routines were not executed: "
            + ", ".join(
                f"{row['module']}:{row['entry']}" for row in not_executed
            )
        )

    missing_reviews = sorted(set(incomplete) - set(reviews))
    stale_reviews = sorted(set(reviews) - set(incomplete))
    invalid: list[str] = []
    for key in sorted(set(incomplete) & set(reviews)):
        row = incomplete[key]
        review = reviews[key]
        digest = hashlib.sha256(
            row["missing_branch_edges"].encode("ascii")
        ).hexdigest()
        if review["missing_edges_sha256"] != digest:
            invalid.append(f"{key[0]}:{key[1]}=edge-set-changed")
        if review["disposition"] != "directed_vectors_required":
            invalid.append(f"{key[0]}:{key[1]}=invalid-disposition")
        if not review["evidence"].strip():
            invalid.append(f"{key[0]}:{key[1]}=missing-evidence")
    if missing_reviews or stale_reviews or invalid:
        details = []
        if missing_reviews:
            details.append(
                "unreviewed="
                + ",".join(f"{module}:{entry}" for module, entry in missing_reviews)
            )
        if stale_reviews:
            details.append(
                "stale="
                + ",".join(f"{module}:{entry}" for module, entry in stale_reviews)
            )
        details.extend(invalid)
        raise SystemExit("XDB branch coverage review gate failed: " + "; ".join(details))
