#!/usr/bin/env python3
"""Reject temporary far pointers that remain in registers across calls."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_ROOT = ROOT / "re/source/bloodprg/candidates"
DEFAULT_LISTING_DIR = (
    ROOT
    / "output/recovered_dos_package/validation/bloodprg_runtime/final"
    / "segment_contract_listings"
)

COMMENT_RE = re.compile(r"/\*.*?\*/|//[^\n]*", re.DOTALL)
POINTER_GLOBAL = (
    r"(?:graphics|bloodprg)_[A-Za-z0-9_]*"
    r"(?:buffer|framebuffer|surface)(?:_ds)?"
)
SAVE_RE = re.compile(
    rf"\b(?P<local>[A-Za-z_]\w*)\s*=\s*"
    rf"(?P<global>{POINTER_GLOBAL})\s*;"
)
CALL_RE = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
IGNORED_CALL_WORDS = frozenset(("if", "for", "while", "switch", "sizeof"))
TYPEDEF_DECL_RE = re.compile(
    r"\bbloodprg_graphics_buffer_ptr\b(?P<qualifiers>[^;\n]*)"
    r"\b(?P<name>[A-Za-z_]\w*)\s*;"
)
RAW_DECL_RE = re.compile(
    r"\bvolatile\s+cb_u8\s+CB_FAR\s*\*(?P<qualifiers>[^;\n]*)"
    r"\b(?P<name>[A-Za-z_]\w*)\s*;"
)
LISTING_RE = re.compile(
    r"^\s*(?P<offset>[0-9A-Fa-f]+)\s+"
    r"(?:(?:[0-9A-Fa-f]{2})\s+)+(?P<instruction>.*?)\s*$"
)
SYMBOL_RE = re.compile(
    rf"_(?P<symbol>{POINTER_GLOBAL})(?P<half>\+0x0*2)?\b",
    re.IGNORECASE,
)
REGISTER_RE = re.compile(
    r"^(?:[re]?(?:ax|bx|cx|dx|si|di|bp|sp)|"
    r"[abcd][lh]|[sd]il|[sb]pl)$",
    re.IGNORECASE,
)
REGISTER_FAMILY = {
    "al": "ax", "ah": "ax", "ax": "ax", "eax": "ax",
    "bl": "bx", "bh": "bx", "bx": "bx", "ebx": "bx",
    "cl": "cx", "ch": "cx", "cx": "cx", "ecx": "cx",
    "dl": "dx", "dh": "dx", "dx": "dx", "edx": "dx",
    "si": "si", "esi": "si", "sil": "si",
    "di": "di", "edi": "di", "dil": "di",
    "bp": "bp", "ebp": "bp", "bpl": "bp",
    "sp": "sp", "esp": "sp", "spl": "sp",
}


def strip_comments(text: str) -> str:
    return COMMENT_RE.sub(lambda match: "\n" * match.group(0).count("\n"), text)


def declaration_is_top_level_volatile(prefix: str, local: str) -> bool:
    declarations = list(TYPEDEF_DECL_RE.finditer(prefix))
    declarations.extend(RAW_DECL_RE.finditer(prefix))
    for declaration in declarations:
        if declaration.group("name") != local:
            continue
        return re.search(
            r"\bvolatile\b", declaration.group("qualifiers")
        ) is not None
    return False


def source_errors(path: Path) -> list[str]:
    text = strip_comments(path.read_text(encoding="ascii", errors="replace"))
    errors: list[str] = []
    seen: set[tuple[str, str]] = set()
    for save in SAVE_RE.finditer(text):
        local = save.group("local")
        global_name = save.group("global")
        restore = re.search(
            rf"\b{re.escape(global_name)}\s*=\s*{re.escape(local)}\s*;",
            text[save.end():],
        )
        if restore is None:
            continue
        between = text[save.end():save.end() + restore.start()]
        calls = [
            match.group(1) for match in CALL_RE.finditer(between)
            if match.group(1) not in IGNORED_CALL_WORDS
        ]
        if not calls:
            continue
        key = (local, global_name)
        if key in seen:
            continue
        seen.add(key)
        if declaration_is_top_level_volatile(text[:save.start()], local):
            continue
        line = text.count("\n", 0, save.start()) + 1
        errors.append(
            f"{path}:{line}: {local} saves {global_name} across "
            f"call {calls[0]} without top-level volatile"
        )
    return errors


def register_family(operand: str) -> str | None:
    value = operand.strip().lower()
    if REGISTER_RE.fullmatch(value) is None:
        return None
    return REGISTER_FAMILY.get(value)


def pointer_reference(operand: str) -> tuple[str, int] | None:
    match = SYMBOL_RE.search(operand)
    if match is None:
        return None
    return match.group("symbol").lower(), 2 if match.group("half") else 0


def listing_instructions(path: Path) -> list[tuple[int, str, list[str]]]:
    instructions: list[tuple[int, str, list[str]]] = []
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = LISTING_RE.match(line)
        if match is None:
            continue
        instruction = match.group("instruction").strip().lower()
        parts = instruction.split(None, 1)
        mnemonic = parts[0]
        operands = [] if len(parts) == 1 else [
            operand.strip() for operand in parts[1].split(",")
        ]
        instructions.append((int(match.group("offset"), 16), mnemonic, operands))
    return instructions


def listing_errors(path: Path) -> list[str]:
    provenance: dict[str, tuple[str, int, int, bool]] = {}
    errors: list[str] = []
    for offset, mnemonic, operands in listing_instructions(path):
        if mnemonic in ("call", "lcall"):
            provenance = {
                register: (symbol, half, loaded_at, True)
                for register, (symbol, half, loaded_at, _crossed) in provenance.items()
            }
            continue

        if mnemonic == "mov" and len(operands) == 2:
            destination, source = operands
            destination_register = register_family(destination)
            source_register = register_family(source)
            source_pointer = pointer_reference(source)
            destination_pointer = pointer_reference(destination)

            if destination_pointer is not None and source_register is not None:
                saved = provenance.get(source_register)
                if saved is not None:
                    symbol, half, loaded_at, crossed_call = saved
                    if crossed_call and (symbol, half) == destination_pointer:
                        errors.append(
                            f"{path}:{offset:04x}: restores _{symbol}"
                            f"{'+0x2' if half else ''} from {source_register} "
                            f"across a call (loaded at {loaded_at:04x})"
                        )

            if destination_register is not None:
                if source_pointer is not None:
                    provenance[destination_register] = (
                        source_pointer[0], source_pointer[1], offset, False
                    )
                elif source_register is not None and source_register in provenance:
                    provenance[destination_register] = provenance[source_register]
                else:
                    provenance.pop(destination_register, None)
            continue

        if mnemonic == "pop" and operands:
            destination_register = register_family(operands[0])
            if destination_register is not None:
                provenance.pop(destination_register, None)
            continue

        if operands and mnemonic not in (
            "cmp", "test", "push", "bt", "out", "int", "ret", "retf"
        ):
            destination_register = register_family(operands[0])
            if destination_register is not None:
                provenance.pop(destination_register, None)
        if mnemonic in ("mul", "imul", "div", "idiv"):
            provenance.pop("ax", None)
            provenance.pop("dx", None)
    return errors


def audit(source_root: Path, listing_dir: Path | None) -> list[str]:
    errors: list[str] = []
    for path in sorted(source_root.rglob("*.c")):
        errors.extend(source_errors(path))
    if listing_dir is not None:
        if not listing_dir.is_dir():
            errors.append(f"missing listing directory: {listing_dir}")
        else:
            for path in sorted(listing_dir.glob("*.lst")):
                errors.extend(listing_errors(path))
    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--listing-dir", type=Path, default=DEFAULT_LISTING_DIR)
    parser.add_argument(
        "--source-only", action="store_true",
        help="audit C declarations without requiring emitted listings",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    errors = audit(
        args.source_root.resolve(),
        None if args.source_only else args.listing_dir.resolve(),
    )
    if errors:
        print("far-pointer lifetime audit failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("far-pointer lifetime audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
