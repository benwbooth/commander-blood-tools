#!/usr/bin/env python3
"""Audit reachable FS/GS operations in a final linked DOS executable.

The linker map is authoritative for CODE-class ownership. Public entrypoints
from project objects and the MZ entrypoint seed a recursive disassembler, so
strings and jump tables inside CODE segments are not mistaken for code.
"""
from __future__ import annotations

import argparse
import csv
import importlib.util
import re
import sys
from collections import deque
from dataclasses import dataclass
from pathlib import Path

import capstone
from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location(
    "re_mzfile", ROOT / "re" / "tools" / "mzfile.py"
)
_module = importlib.util.module_from_spec(_spec)
sys.modules["re_mzfile"] = _module
_spec.loader.exec_module(_module)
MZ = _module.MZ

SEGMENT_ROW = re.compile(
    r"^(?P<name>\S+)\s+CODE\s+\S+\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})\s+"
    r"(?P<size>[0-9A-Fa-f]{8})$"
)
PUBLIC_ROW = re.compile(
    r"^(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})"
    r"(?:[*+]?)\s+(?P<symbol>\S+)$"
)


@dataclass(frozen=True)
class CodeSection:
    name: str
    segment: int
    offset: int
    size: int

    @property
    def linear_start(self) -> int:
        return self.segment * 16 + self.offset

    @property
    def linear_end(self) -> int:
        return self.linear_start + self.size

    @property
    def project_owned(self) -> bool:
        return self.name != "_TEXT"


def parse_link_map(path: Path) -> tuple[list[CodeSection], list[tuple[int, int]]]:
    sections: list[CodeSection] = []
    publics: list[tuple[int, int]] = []
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        stripped = line.strip()
        segment_match = SEGMENT_ROW.match(stripped)
        if segment_match:
            sections.append(
                CodeSection(
                    segment_match["name"],
                    int(segment_match["segment"], 16),
                    int(segment_match["offset"], 16),
                    int(segment_match["size"], 16),
                )
            )
            continue
        public_match = PUBLIC_ROW.match(stripped)
        if public_match:
            publics.append(
                (
                    int(public_match["segment"], 16),
                    int(public_match["offset"], 16),
                )
            )
    if not sections:
        raise SystemExit(f"{path}: no CODE segments found")
    return sections, publics


def containing_section(
    sections: list[CodeSection], linear: int
) -> CodeSection | None:
    for section in sections:
        if section.linear_start <= linear < section.linear_end:
            return section
    return None


def direct_target(insn: capstone.CsInsn) -> int | None:
    if len(insn.operands) != 1:
        return None
    operand = insn.operands[0]
    if operand.type != x86_const.X86_OP_IMM:
        return None
    return operand.imm & 0xFFFF


def segment_memory_override(insn: capstone.CsInsn) -> str | None:
    for prefix in insn.prefix:
        if prefix == 0x64:
            return "FS"
        if prefix == 0x65:
            return "GS"
    return None


def segment_register_writes(insn: capstone.CsInsn) -> tuple[str, ...]:
    writes: list[str] = []
    if insn.mnemonic == "mov" and insn.operands:
        destination = insn.operands[0]
        if destination.type == x86_const.X86_OP_REG:
            name = insn.reg_name(destination.reg).upper()
            if name in ("FS", "GS"):
                writes.append(name)
    elif insn.mnemonic in ("pop", "lfs", "lgs"):
        text = insn.op_str.upper()
        for name in ("FS", "GS"):
            if insn.mnemonic == "l" + name.lower() or text == name:
                writes.append(name)
    return tuple(writes)


def reachable_instructions(
    mz: MZ,
    sections: list[CodeSection],
    seeds: set[tuple[int, int]],
) -> dict[int, capstone.CsInsn]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    pending = deque(seeds)
    visited_blocks: set[int] = set()
    instructions: dict[int, capstone.CsInsn] = {}

    while pending:
        segment, offset = pending.popleft()
        linear = segment * 16 + offset
        if linear in visited_blocks:
            continue
        section = containing_section(sections, linear)
        if section is None:
            continue
        visited_blocks.add(linear)
        cursor = linear
        ip = offset

        while cursor < section.linear_end:
            if cursor in instructions:
                break
            file_offset = mz.header_size + cursor
            decoded = next(
                decoder.disasm(
                    mz.data[file_offset : mz.header_size + section.linear_end],
                    ip,
                    count=1,
                ),
                None,
            )
            if decoded is None:
                raise SystemExit(
                    f"cannot decode reachable instruction at "
                    f"{segment:04x}:{ip:04x} in {section.name}"
                )
            instructions[cursor] = decoded
            next_cursor = cursor + decoded.size
            next_ip = (ip + decoded.size) & 0xFFFF
            groups = set(decoded.groups)
            target = direct_target(decoded)

            if x86_const.X86_GRP_CALL in groups and target is not None:
                pending.append((segment, target))
            if x86_const.X86_GRP_JUMP in groups:
                if target is not None:
                    pending.append((segment, target))
                if decoded.mnemonic == "jmp":
                    break
            if x86_const.X86_GRP_RET in groups or decoded.mnemonic in (
                "iret",
                "iretd",
            ):
                break

            cursor = next_cursor
            ip = next_ip
    return instructions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--image",
        type=Path,
        default=ROOT / "output/recovered_dos_package/cd/BPRG_RE.EXE",
    )
    parser.add_argument(
        "--link-map",
        type=Path,
        default=ROOT
        / "output/recovered_dos_package/validation/bloodprg_runtime/final/link.map",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--allow-project-memory-prefixes",
        action="store_true",
        help="report project FS/GS memory accesses without failing",
    )
    args = parser.parse_args()

    mz = MZ(args.image.resolve())
    sections, publics = parse_link_map(args.link_map.resolve())
    seeds = {(mz.e_cs, mz.e_ip)}
    for segment, offset in publics:
        section = containing_section(sections, segment * 16 + offset)
        if section is not None and section.project_owned:
            seeds.add((segment, offset))

    instructions = reachable_instructions(mz, sections, seeds)
    rows: list[tuple[str, str, str, str, str, str, str]] = []
    project_memory_prefixes = 0
    for linear, insn in sorted(instructions.items()):
        section = containing_section(sections, linear)
        assert section is not None
        scope = "project" if section.project_owned else "runtime"
        segment = segment_memory_override(insn)
        if segment is not None:
            rows.append(
                (
                    scope,
                    section.name,
                    f"{section.segment:04x}:{insn.address:04x}",
                    f"0x{mz.header_size + linear:08x}",
                    "memory_override",
                    segment,
                    f"{insn.mnemonic} {insn.op_str}",
                )
            )
            if section.project_owned:
                project_memory_prefixes += 1
        for written in segment_register_writes(insn):
            rows.append(
                (
                    scope,
                    section.name,
                    f"{section.segment:04x}:{insn.address:04x}",
                    f"0x{mz.header_size + linear:08x}",
                    "register_write",
                    written,
                    f"{insn.mnemonic} {insn.op_str}",
                )
            )

    stream = (
        args.output.open("w", newline="", encoding="ascii")
        if args.output
        else sys.stdout
    )
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow(
        ("scope", "section", "address", "file_offset", "kind", "segment", "text")
    )
    writer.writerows(rows)
    if args.output:
        stream.close()

    memory_count = sum(row[4] == "memory_override" for row in rows)
    write_count = sum(row[4] == "register_write" for row in rows)
    print(
        f"{len(instructions)} reachable instructions audited across "
        f"{len(sections)} CODE segments; {memory_count} FS/GS memory overrides; "
        f"{write_count} FS/GS register writes"
    )
    if project_memory_prefixes and not args.allow_project_memory_prefixes:
        print(f"ERROR: {project_memory_prefixes} project FS/GS memory accesses")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
