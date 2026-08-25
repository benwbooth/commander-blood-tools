#!/usr/bin/env python3
"""Reject game logic hidden in recovered C inline assembly.

The native Rust port does not need DOS hardware and foreign-ABI shims, but it
does need every game decision to exist as typed C first.  This gate permits
only exact reviewed platform-bound ``#pragma aux`` instruction bodies.  ABI-
only pragmas do not emit code and are intentionally outside this inventory.
"""

from __future__ import annotations

import argparse
import csv
from dataclasses import dataclass
import io
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
PRODUCTION_PATHS = (
    ROOT / "re/source/bloodprg/candidates",
    ROOT / "re/source/xdb/candidates",
    ROOT / "re/integration/dos/bloodprg_platform_adapters.c",
    ROOT / "re/integration/dos/bloodprg_relinked_main.c",
)
PRAGMA_AUX = re.compile(
    r"\s*#\s*pragma\s+aux\s+([A-Za-z_]\w*)\b(.*)", re.IGNORECASE
)
DIRECT_ASM = re.compile(
    r"(^|[^A-Za-z0-9_])(?:__asm|_asm|asm)\s*(?:\{|\()|"
    r"\b(?:__emit|_emit)\s*\(",
    re.IGNORECASE,
)
CONTRACT_KEYWORDS = (" parm ", " value ", " modify ")


@dataclass(frozen=True)
class AllowedBlock:
    category: str
    instructions: tuple[str, ...]
    reason: str


@dataclass(frozen=True)
class AssemblyBlock:
    path: Path
    line: int
    function: str
    instructions: tuple[str, ...]

    @property
    def instruction_count(self) -> int:
        return sum(not item.endswith(":") for item in self.instructions)


ALLOWED_BLOCKS = {
    ("re/integration/dos/bloodprg_platform_adapters.c",
     "bloodprg_overlay_call_inherited_bp"): AllowedBlock(
        "overlay_abi",
        (
            "push bp", "push ds", "push es", "push fs", "mov bp,si",
            "push dx", "push ax", "mov bx,sp", "call dword ptr ss:[bx]",
            "add sp,4", "pop fs", "pop es", "pop ds", "pop bp",
        ),
        "Calls a loaded DOS overlay with its inherited SS:BP request ABI.",
    ),
    ("re/integration/dos/bloodprg_platform_adapters.c",
     "cb_platform_xms_move"): AllowedBlock(
        "xms_driver",
        ("mov ah,0bh", "call dword ptr xms_driver_entry"),
        "Calls the HIMEM.SYS move service through its register ABI.",
    ),
    ("re/integration/dos/bloodprg_platform_adapters.c",
     "cb_platform_xms_release"): AllowedBlock(
        "xms_driver",
        ("mov ah,0ah", "call dword ptr xms_driver_entry"),
        "Calls the HIMEM.SYS release service through its register ABI.",
    ),
    ("re/integration/dos/bloodprg_platform_adapters.c",
     "cb_platform_xms_allocate"): AllowedBlock(
        "xms_driver",
        (
            "mov ah,09h", "call dword ptr xms_driver_entry", "mov cx,dx",
            "xor dx,dx", "or ax,ax", "jz short cb_xms_allocate_done",
            "inc dx", "cb_xms_allocate_done:", "mov ax,cx",
        ),
        "Calls HIMEM.SYS and repacks its AX/DX result into a C return value.",
    ),
    ("re/integration/dos/bloodprg_relinked_main.c",
     "bloodprg_install_game_segments"): AllowedBlock(
        "segment_boundary",
        ("mov dx, ds", "mov gs, dx", "mov fs, ax"),
        "Installs the original DOS data-segment convention at startup.",
    ),
    ("re/source/bloodprg/candidates/include/bloodprg_hardware.h",
     "cb_flags_read"): AllowedBlock(
        "cpu_flags",
        ("pushf", "pop ax"),
        "Reads CPU flags for the recovered processor capability probe.",
    ),
    ("re/source/bloodprg/candidates/include/bloodprg_hardware.h",
     "cb_flags_write"): AllowedBlock(
        "cpu_flags",
        ("push ax", "popf"),
        "Writes CPU flags for the recovered processor capability probe.",
    ),
    ("re/source/xdb/candidates/include/xdb_alien.h",
     "xdb_alien_frame_callback_invoke"): AllowedBlock(
        "host_callback_abi",
        (
            "shl edx,16", "mov dx,ax", "mov ax,bx",
            "call dword ptr xdb_alien_frame_callback_ptr",
        ),
        "Adapts C arguments to the host game's alien-frame callback ABI.",
    ),
    ("re/source/xdb/candidates/include/xdb_alien.h",
     "xdb_alien_data_segments_install"): AllowedBlock(
        "segment_boundary",
        ("mov dx,ds", "mov ds,ax", "mov es,ax", "mov fs,ax"),
        "Installs an alien overlay's DOS data segments on entry.",
    ),
    ("re/source/xdb/candidates/include/xdb_alien.h",
     "xdb_alien_data_segment_restore"): AllowedBlock(
        "segment_boundary",
        ("mov ds,ax",),
        "Restores the host data segment when an alien overlay returns.",
    ),
}


def source_files(paths: tuple[Path, ...] = PRODUCTION_PATHS) -> list[Path]:
    files: set[Path] = set()
    for path in paths:
        if path.is_dir():
            files.update(path.rglob("*.c"))
            files.update(path.rglob("*.h"))
        elif path.is_file():
            files.add(path)
    return sorted(files)


def logical_lines(text: str) -> list[tuple[int, str]]:
    physical = text.splitlines()
    result: list[tuple[int, str]] = []
    index = 0
    while index < len(physical):
        line_number = index + 1
        line = physical[index]
        while line.rstrip().endswith("\\") and index + 1 < len(physical):
            line = line.rstrip()[:-1] + " " + physical[index + 1].lstrip()
            index += 1
        result.append((line_number, line))
        index += 1
    return result


def pragma_instructions(body: str) -> tuple[str, ...]:
    equals = body.find("=")
    if equals < 0:
        return ()
    boundaries = [
        body.find(keyword, equals)
        for keyword in CONTRACT_KEYWORDS
        if body.find(keyword, equals) >= 0
    ]
    end = min(boundaries) if boundaries else len(body)
    return tuple(re.findall(r'"([^"]*)"', body[equals + 1 : end]))


def scan_file(path: Path) -> list[AssemblyBlock]:
    blocks: list[AssemblyBlock] = []
    for line_number, line in logical_lines(path.read_text(encoding="ascii")):
        pragma = PRAGMA_AUX.match(line)
        if pragma is not None:
            instructions = pragma_instructions(pragma.group(2))
            if instructions:
                blocks.append(AssemblyBlock(
                    path, line_number, pragma.group(1), instructions
                ))
            continue
        if DIRECT_ASM.search(line):
            blocks.append(AssemblyBlock(
                path, line_number, "<direct-asm>", (line.strip(),)
            ))
    return blocks


def audit_blocks(
    blocks: list[AssemblyBlock],
    allowed: dict[tuple[str, str], AllowedBlock] = ALLOWED_BLOCKS,
) -> list[str]:
    errors: list[str] = []
    for block in blocks:
        relative = block.path.resolve().relative_to(ROOT).as_posix()
        key = (relative, block.function)
        expected = allowed.get(key)
        if expected is None:
            errors.append(
                f"{relative}:{block.line}: unreviewed code-emitting assembly "
                f"in {block.function}"
            )
        elif block.instructions != expected.instructions:
            errors.append(
                f"{relative}:{block.line}: reviewed platform assembly in "
                f"{block.function} changed; expected {expected.instructions!r}, "
                f"got {block.instructions!r}"
            )
    return errors


def render_tsv(blocks: list[AssemblyBlock]) -> str:
    output = io.StringIO()
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "source", "line", "function", "category", "instruction_count",
        "reason",
    ))
    for block in blocks:
        relative = block.path.resolve().relative_to(ROOT).as_posix()
        allowed = ALLOWED_BLOCKS.get((relative, block.function))
        writer.writerow((
            relative,
            block.line,
            block.function,
            "unreviewed" if allowed is None else allowed.category,
            block.instruction_count,
            "" if allowed is None else allowed.reason,
        ))
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail on game-logic assembly")
    parser.add_argument("--output", type=Path, help="write the reviewed inventory TSV")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.check:
        raise SystemExit("--check is required")
    blocks = [block for path in source_files() for block in scan_file(path)]
    errors = audit_blocks(blocks)
    report = render_tsv(blocks)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    instructions = sum(block.instruction_count for block in blocks)
    print(
        f"OK: {len(blocks)} reviewed platform assembly block(s), "
        f"{instructions} instruction(s), zero game-logic blocks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
