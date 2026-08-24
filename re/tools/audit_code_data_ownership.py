#!/usr/bin/env python3
"""Derive and verify BLOODPRG code-segment data ownership.

The original executable stores a small number of constants, lookup tables, and
mutable cells in code segments. Watcom otherwise defaults file-local constants
to ``CONST2`` and accesses them through DS, which is wrong whenever the original
instruction used CS. This audit derives the original inventory from every
recovered BLOODPRG assembly routine and then checks the corresponding emitted
object listings. No symbol allowlist is used.
"""
from __future__ import annotations

import argparse
import re
from dataclasses import dataclass, field
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ORIGINAL_DIR = ROOT / "re/assembly/bloodprg"
DEFAULT_ORIGINAL_IMAGE = ROOT / "re/bin/BLOODPRG.EXE"

ORIGINAL_INSTRUCTION_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{4,8}):\s+"
    r"(?P<bytes>(?:[0-9A-Fa-f]{2}(?:\s+|$))+)(?P<text>.*?)\s*$"
)
LISTING_INSTRUCTION_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{4,8})\s+"
    r"(?P<bytes>(?:[0-9A-Fa-f]{2}(?:\s+|$))+)(?P<text>.*?)\s*$"
)
LABEL_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{4,8})\s+"
    r"(?P<label>[A-Za-z_$?][\w$?@]*):\s*$"
)
SEGMENT_ROW = re.compile(r"^Segment:\s+(?P<name>\S+)\s+")
FILE_OFFSET_ROW = re.compile(r"^; file_offset:\s+0x(?P<value>[0-9A-Fa-f]+)$")
SEG_OFF_ROW = re.compile(
    r"^; seg_off:\s+(?P<segment>[0-9A-Fa-f]+):(?P<offset>[0-9A-Fa-f]+)$"
)
GROUP_ROW = re.compile(r"^; group:\s+(?P<name>\S+)$")
ROUTINE_ID = re.compile(r"^func_(?P<offset>[0-9A-Fa-f]{6})_")
MEMORY_OPERAND = re.compile(
    r"(?:(?P<segment>cs|ds|es|fs|gs|ss):)?\[(?P<expr>[^]]+)\]",
    re.IGNORECASE,
)
SYMBOL = re.compile(
    r"(?<![\w$?@])(?:_[A-Za-z_$?][\w$?@]*|L\$\d+)", re.IGNORECASE
)
SEGMENT_SYMBOL = re.compile(
    r"\bseg\s+(?P<symbol>[A-Za-z_$?][\w$?@]*)", re.IGNORECASE
)
ADDRESS_SYMBOL = re.compile(
    r"\b(?:offset|seg)\s+(?P<symbol>[A-Za-z_$?][\w$?@]*)",
    re.IGNORECASE,
)
HEX_LITERAL = re.compile(r"(?<![\w])0x(?P<value>[0-9A-Fa-f]+)\b")

PREFIX_BYTES = {
    0x26, 0x2E, 0x36, 0x3E, 0x64, 0x65, 0x66, 0x67, 0xF0, 0xF2, 0xF3,
}
GENERAL_REGISTERS = {
    "ax", "bx", "cx", "dx", "si", "di", "bp", "sp",
    "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp",
}
SEGMENT_REGISTERS = {"cs", "ds", "es", "fs", "gs", "ss"}
WRITE_MNEMONICS = {
    "mov", "seta", "setae", "setb", "setbe", "sete", "setne", "setg",
    "setge", "setl", "setle", "inc", "dec", "not", "neg", "pop",
}
READ_WRITE_MNEMONICS = {
    "add", "adc", "sub", "sbb", "and", "or", "xor", "shl", "shr",
    "sal", "sar", "rol", "ror", "rcl", "rcr", "xchg",
}
INDIRECT_TRANSFER_MNEMONICS = {"call", "jmp", "lcall", "ljmp"}
KNOWN_MNEMONICS = {
    "aaa", "aad", "aam", "aas", "adc", "add", "and", "bound", "bsf",
    "bsr", "bswap", "bt", "btc", "btr", "bts", "call", "cbw", "cdq",
    "clc", "cld", "cli", "cmc", "cmp", "cmpsb", "cmpsd", "cmpsw",
    "cwd", "cwde", "daa", "das", "dec", "div", "enter", "hlt", "idiv",
    "imul", "in", "inc", "insb", "insd", "insw", "int", "into", "iret",
    "iretd", "ja", "jae", "jb", "jbe", "je", "jecxz", "jg", "jge", "jl",
    "jle", "jmp", "jne", "jno", "jnp", "jns", "jo", "jp", "js", "lahf",
    "lar", "lcall", "lds", "lea", "leave", "les", "lfs", "lgdt", "lgs",
    "lidt", "ljmp", "lodsb", "lodsd", "lodsw", "loop", "loope", "loopne",
    "lsl", "lss", "ltr", "mov", "movsb", "movsd", "movsw", "movsx",
    "movzx", "mul", "neg", "nop", "not", "or", "out", "outsb", "outsd",
    "outsw", "pop", "popa", "popad", "popf", "push", "pusha", "pushad",
    "pushf", "rcl", "rcr", "ret", "retf", "rol", "ror", "sahf", "sal",
    "sar", "sbb", "scasb", "scasd", "scasw", "seta", "setae", "setb",
    "setbe", "sete", "setg", "setge", "setl", "setle", "setne", "shl",
    "shld", "shr", "shrd", "stc", "std", "sti", "stosb", "stosd",
    "stosw", "sub", "test", "verr", "verw", "wait", "xchg", "xlat",
    "xlatb", "xor",
}


@dataclass(frozen=True)
class OriginalReference:
    routine_id: str
    routine_name: str
    group: str
    address: int
    target: int
    absolute_target: int
    width: int
    access: str
    indexed: bool
    kind: str
    text: str


@dataclass
class OriginalObject:
    group: str
    target: int
    absolute_target: int
    references: list[OriginalReference] = field(default_factory=list)

    @property
    def is_read_only(self) -> bool:
        return all(reference.access == "read" for reference in self.references)

    @property
    def is_table(self) -> bool:
        return self.is_read_only and any(
            reference.indexed or reference.kind in {"xlat", "indirect"}
            for reference in self.references
        )

    @property
    def width(self) -> int:
        return max(reference.width for reference in self.references)


@dataclass(frozen=True)
class ListingReference:
    listing_name: str
    routine_id: str
    symbol: str
    text: str
    memory: bool
    indexed: bool
    indirect: bool
    address_only: bool


@dataclass
class ListingInfo:
    name: str
    routine_id: str
    lines: list[str]
    labels: dict[str, str]
    data_labels: set[str]
    instructions: list[str]
    references: list[ListingReference]


def canonical_register(name: str) -> str:
    lowered = name.lower()
    if lowered in {"al", "ah"}:
        return "ax"
    if lowered in {"bl", "bh"}:
        return "bx"
    if lowered in {"cl", "ch"}:
        return "cx"
    if lowered in {"dl", "dh"}:
        return "dx"
    if lowered.startswith("e") and lowered[1:] in GENERAL_REGISTERS:
        return lowered[1:]
    return lowered


def operation(text: str) -> str:
    words = text.lower().split()
    while words and words[0] in {"rep", "repe", "repz", "repne", "repnz"}:
        words.pop(0)
    return words[0] if words else ""


def split_operands(text: str) -> tuple[str, ...]:
    normalized = text.strip()
    words = normalized.split(None, 1)
    while words and words[0].lower() in {
        "rep", "repe", "repz", "repne", "repnz",
    }:
        normalized = words[1] if len(words) == 2 else ""
        words = normalized.split(None, 1)
    if len(words) != 2:
        return ()
    return tuple(part.strip() for part in words[1].split(","))


def has_cs_override(data: bytes) -> bool:
    prefixes: list[int] = []
    for value in data:
        if value not in PREFIX_BYTES:
            break
        prefixes.append(value)
    return 0x2E in prefixes


def operand_width(text: str) -> int:
    lowered = text.lower()
    if "dword ptr" in lowered:
        return 4
    if "word ptr" in lowered:
        return 2
    return 1


def access_for_operand(mnemonic: str, operand_index: int) -> str:
    if mnemonic.startswith("cmp") or mnemonic in {"test", "call", "jmp", "lcall", "ljmp"}:
        return "read"
    if mnemonic.startswith("cmps") or mnemonic.startswith("scas"):
        return "read"
    if mnemonic.startswith("lods"):
        return "read"
    if mnemonic.startswith("stos"):
        return "write"
    if mnemonic.startswith("movs"):
        return "write" if operand_index == 0 else "read"
    if mnemonic in READ_WRITE_MNEMONICS:
        return "read_write" if operand_index == 0 or mnemonic == "xchg" else "read"
    if mnemonic in WRITE_MNEMONICS and operand_index == 0:
        return "write"
    return "read"


def expression_target(expr: str, constants: dict[str, int | None]) -> int | None:
    numeric = re.findall(
        r"(?<![\w])[-+]?\s*0x[0-9A-Fa-f]+|(?<![\w])[-+]?\s*\d+", expr
    )
    if numeric:
        total = 0
        for token in numeric:
            compact = token.replace(" ", "")
            sign = -1 if compact.startswith("-") else 1
            compact = compact.lstrip("+-")
            total += sign * int(compact, 0)
        return total & 0xFFFF
    registers = [
        canonical_register(token)
        for token in re.findall(
            r"\b(?:e?[abcd]x|e?[sd]i|e?[sb]p)\b", expr, re.IGNORECASE
        )
    ]
    known = [constants.get(register) for register in registers]
    if not known or all(value is None for value in known):
        return None
    return sum(value for value in known if value is not None) & 0xFFFF


def update_original_state(
    text: str,
    constants: dict[str, int | None],
    provenance: dict[str, str],
    segments: dict[str, str],
    stack: list[str],
) -> None:
    mnemonic = operation(text)
    operands = split_operands(text)
    if mnemonic in {"call", "lcall"}:
        # Original near/far helpers use mixed caller- and callee-cleaned ABIs.
        # A saved segment provenance cannot be carried across an unmodelled call.
        stack.clear()
        return
    if mnemonic == "push" and len(operands) == 1:
        source = canonical_register(operands[0])
        stack.append(segments.get(source, provenance.get(source, "unknown")))
        return
    if mnemonic == "pop" and len(operands) == 1:
        destination = canonical_register(operands[0])
        value = stack.pop() if stack else "unknown"
        if destination in SEGMENT_REGISTERS:
            segments[destination] = value
        else:
            provenance[destination] = value
            constants[destination] = None
        return
    if mnemonic == "mov" and len(operands) == 2:
        destination = canonical_register(operands[0])
        source = canonical_register(operands[1])
        if destination in GENERAL_REGISTERS:
            immediate = re.fullmatch(r"(?:0x[0-9A-Fa-f]+|\d+)", operands[1])
            constants[destination] = int(operands[1], 0) if immediate else None
            provenance[destination] = segments.get(
                source, provenance.get(source, "unknown")
            )
        elif destination in SEGMENT_REGISTERS:
            segments[destination] = segments.get(
                source, provenance.get(source, "unknown")
            )
        return
    if mnemonic == "xor" and len(operands) == 2:
        left = canonical_register(operands[0])
        right = canonical_register(operands[1])
        if left == right and left in GENERAL_REGISTERS:
            constants[left] = 0
            provenance[left] = "value"
            return
    if mnemonic in {"add", "sub"} and len(operands) == 2:
        destination = canonical_register(operands[0])
        immediate = re.fullmatch(r"(?:0x[0-9A-Fa-f]+|\d+)", operands[1])
        if (
            destination in GENERAL_REGISTERS
            and immediate
            and constants.get(destination) is not None
        ):
            delta = int(operands[1], 0)
            constants[destination] = (
                constants[destination] + (delta if mnemonic == "add" else -delta)
            ) & 0xFFFF
            return
    if mnemonic in {"inc", "dec"} and len(operands) == 1:
        destination = canonical_register(operands[0])
        if destination in GENERAL_REGISTERS and constants.get(destination) is not None:
            delta = 1 if mnemonic == "inc" else -1
            constants[destination] = (constants[destination] + delta) & 0xFFFF
            return
    if operands:
        destination = canonical_register(operands[0])
        if destination in GENERAL_REGISTERS and mnemonic not in {
            "cmp", "test", "call", "lcall", "jmp", "ljmp", "push",
        }:
            constants[destination] = None
            provenance[destination] = "unknown"


def parse_original_routine(path: Path) -> tuple[list[OriginalReference], list[str]]:
    match = ROUTINE_ID.match(path.name)
    if not match:
        return [], [f"{path}: cannot derive routine address from filename"]
    routine_id = match["offset"].lower()
    lines = path.read_text(encoding="ascii", errors="replace").splitlines()
    file_offset = None
    segment_offset = None
    group = None
    for line in lines:
        if found := FILE_OFFSET_ROW.match(line):
            file_offset = int(found["value"], 16)
        elif found := SEG_OFF_ROW.match(line):
            segment_offset = int(found["offset"], 16)
        elif found := GROUP_ROW.match(line):
            group = found["name"]
    if file_offset is None or segment_offset is None or group is None:
        return [], [f"{path.name}: incomplete file_offset/seg_off/group metadata"]
    segment_base = file_offset - segment_offset

    constants = {register: None for register in GENERAL_REGISTERS}
    provenance = {register: "unknown" for register in GENERAL_REGISTERS}
    segments = {
        "cs": "code", "ds": "unknown", "es": "unknown", "fs": "unknown",
        "gs": "unknown", "ss": "stack",
    }
    stack: list[str] = []
    references: list[OriginalReference] = []
    errors: list[str] = []

    for line in lines:
        row = ORIGINAL_INSTRUCTION_ROW.match(line)
        if not row:
            continue
        data = bytes.fromhex(row["bytes"])
        text = row["text"].strip()
        mnemonic = operation(text)
        operands = split_operands(text)
        address = int(row["offset"], 16)
        cs_override = has_cs_override(data)

        candidates: list[tuple[int | None, int, str, bool, str]] = []
        if mnemonic in {"xlat", "xlatb"} and cs_override:
            candidates.append((constants.get("bx"), 1, "read", True, "xlat"))
        else:
            memory_matches = list(MEMORY_OPERAND.finditer(text))
            for memory in memory_matches:
                explicit_segment = (memory["segment"] or "").lower()
                expr = memory["expr"]
                default_segment = (
                    "ss"
                    if re.search(r"\b(?:e?bp|e?sp)\b", expr, re.IGNORECASE)
                    else "ds"
                )
                effective_segment = explicit_segment or default_segment
                if not (cs_override or segments.get(effective_segment) == "code"):
                    continue
                target = expression_target(expr, constants)
                memory_text = memory.group(0).lower()
                operand_index = next(
                    (
                        index
                        for index, operand in enumerate(operands)
                        if memory_text in operand.lower()
                    ),
                    0,
                )
                access = access_for_operand(mnemonic, operand_index)
                indexed = bool(re.search(
                    r"\b(?:e?[abcd]x|e?[sd]i|e?[sb]p)\b", expr, re.IGNORECASE
                ))
                kind = (
                    "indirect"
                    if mnemonic in INDIRECT_TRANSFER_MNEMONICS
                    else "memory"
                )
                candidates.append((target, operand_width(text), access, indexed, kind))

        if cs_override and not candidates:
            errors.append(
                f"{path.name}:{address:06x}: unresolved CS-prefixed instruction: {text}"
            )
        for target, width, access, indexed, kind in candidates:
            if target is None:
                errors.append(
                    f"{path.name}:{address:06x}: unresolved CS-relative target: {text}"
                )
                continue
            references.append(OriginalReference(
                routine_id=routine_id,
                routine_name=path.name,
                group=group,
                address=address,
                target=target,
                absolute_target=segment_base + target,
                width=width,
                access=access,
                indexed=indexed,
                kind=kind,
                text=text,
            ))
        update_original_state(text, constants, provenance, segments, stack)
    return references, errors


def derive_original_inventory(
    original_dir: Path,
) -> tuple[dict[tuple[str, int], OriginalObject], list[str]]:
    if not original_dir.is_dir():
        return {}, [f"missing original BLOODPRG assembly directory: {original_dir}"]
    paths = sorted(original_dir.rglob("*.asm"))
    if not paths:
        return {}, [f"no original BLOODPRG assembly routines found in {original_dir}"]
    objects: dict[tuple[str, int], OriginalObject] = {}
    errors: list[str] = []
    seen_routines: set[str] = set()
    for path in paths:
        match = ROUTINE_ID.match(path.name)
        if match:
            routine_id = match["offset"].lower()
            if routine_id in seen_routines:
                errors.append(f"duplicate original BLOODPRG routine address: {routine_id}")
            seen_routines.add(routine_id)
        references, routine_errors = parse_original_routine(path)
        errors.extend(routine_errors)
        for reference in references:
            key = (reference.group, reference.target)
            item = objects.setdefault(key, OriginalObject(
                group=reference.group,
                target=reference.target,
                absolute_target=reference.absolute_target,
            ))
            if item.absolute_target != reference.absolute_target:
                errors.append(
                    f"{reference.group}:{reference.target:04x}: inconsistent file target "
                    f"{item.absolute_target:06x}/{reference.absolute_target:06x}"
                )
            item.references.append(reference)
    if not objects:
        errors.append("original BLOODPRG corpus yielded no CS-relative data objects")
    return objects, errors


def is_listing_instruction(text: str) -> bool:
    return operation(text) in KNOWN_MNEMONICS


def parse_listing(path: Path) -> ListingInfo:
    match = ROUTINE_ID.match(path.name)
    if not match:
        raise ValueError(f"{path.name}: cannot derive routine address from filename")
    routine_id = match["offset"].lower()
    lines = path.read_text(encoding="ascii", errors="replace").splitlines()
    section = ""
    labels: dict[str, str] = {}
    data_labels: set[str] = set()
    instructions: list[str] = []
    pending_label: str | None = None

    for line in lines:
        if found := SEGMENT_ROW.match(line):
            section = found["name"]
            pending_label = None
            continue
        if found := LABEL_ROW.match(line):
            pending_label = found["label"]
            labels[pending_label.lower()] = section
            if not section.endswith("_TEXT"):
                data_labels.add(pending_label.lower())
            continue
        if found := LISTING_INSTRUCTION_ROW.match(line):
            text = found["text"].strip()
            if is_listing_instruction(text):
                instructions.append(text.lower())
            elif pending_label is not None:
                data_labels.add(pending_label.lower())
            pending_label = None
            continue
        if line.strip():
            pending_label = None

    references: list[ListingReference] = []
    for text in instructions:
        mnemonic = operation(text)
        address_symbols = {
            match["symbol"].lower() for match in ADDRESS_SYMBOL.finditer(text)
        }
        for found in SYMBOL.finditer(text):
            symbol = found.group(0).lower()
            address_only = symbol in address_symbols
            direct_branch = (
                mnemonic in INDIRECT_TRANSFER_MNEMONICS
                and "ptr" not in text
                and "[" not in text
                and not re.search(
                    rf"(?:cs|ds|es|fs|gs|ss):{re.escape(symbol)}", text
                )
            )
            memory = not address_only and not direct_branch and (
                "ptr" in text
                or "[" in text
                or bool(re.search(
                    rf"(?:cs|ds|es|fs|gs|ss):{re.escape(symbol)}\b", text
                ))
            )
            if not memory and not address_only:
                continue
            indexed = bool(re.search(
                rf"{re.escape(symbol)}[^,]*\[(?:e?[abcd]x|e?[sd]i|e?[sb]p)",
                text,
            ))
            indirect = mnemonic in INDIRECT_TRANSFER_MNEMONICS and memory
            references.append(ListingReference(
                listing_name=path.name,
                routine_id=routine_id,
                symbol=symbol,
                text=text,
                memory=memory,
                indexed=indexed,
                indirect=indirect,
                address_only=address_only,
            ))
    return ListingInfo(
        name=path.name,
        routine_id=routine_id,
        lines=lines,
        labels=labels,
        data_labels=data_labels,
        instructions=instructions,
        references=references,
    )


def listing_inventory(
    listing_dir: Path,
) -> tuple[dict[str, ListingInfo], dict[str, list[tuple[str, str]]], list[str]]:
    if not listing_dir.is_dir():
        return {}, {}, [f"missing emitted listing directory: {listing_dir}"]
    listings: dict[str, ListingInfo] = {}
    definitions: dict[str, list[tuple[str, str]]] = {}
    errors: list[str] = []
    for path in sorted(listing_dir.glob("*.lst")):
        if not ROUTINE_ID.match(path.name):
            continue
        try:
            info = parse_listing(path)
        except ValueError as exc:
            errors.append(str(exc))
            continue
        if info.routine_id in listings:
            errors.append(f"duplicate emitted listing address: {info.routine_id}")
        listings[info.routine_id] = info
        for symbol, owner in info.labels.items():
            definitions.setdefault(symbol, []).append((info.name, owner))
    if not listings:
        errors.append(f"no emitted BLOODPRG listings found in {listing_dir}")
    return listings, definitions, errors


def resolve_owner(
    reference: ListingReference,
    listings: dict[str, ListingInfo],
    definitions: dict[str, list[tuple[str, str]]],
) -> str | None:
    local = listings[reference.routine_id].labels.get(reference.symbol)
    if local is not None:
        return local
    candidates = definitions.get(reference.symbol, [])
    owners = {owner for _name, owner in candidates}
    return next(iter(owners)) if len(owners) == 1 else None


def code_segment_literal_present(
    info: ListingInfo,
    definitions: dict[str, list[tuple[str, str]]],
) -> bool:
    for text in info.instructions:
        for found in SEGMENT_SYMBOL.finditer(text):
            symbol = found["symbol"].lower()
            local_owner = info.labels.get(symbol)
            owners = {owner for _name, owner in definitions.get(symbol, [])}
            loaded_owners = ({local_owner} if local_owner is not None else set()) | owners
            if any(owner.endswith("_TEXT") for owner in loaded_owners):
                return True
    return False


def immediate_values(texts: list[str]) -> set[int]:
    return {
        int(found["value"], 16)
        for text in texts
        for found in HEX_LITERAL.finditer(text)
    }


def scalar_materialized(data: bytes, texts: list[str]) -> bool:
    values = immediate_values(texts)
    if int.from_bytes(data, "little") in values:
        return True
    if len(data) == 4:
        return all(
            int.from_bytes(data[offset:offset + 2], "little") in values
            for offset in (0, 2)
        )
    if len(data) == 2:
        return int.from_bytes(data, "little") in values
    return data[0] in values


def audit(
    listing_dir: Path,
    original_dir: Path = DEFAULT_ORIGINAL_DIR,
    original_image: Path | None = DEFAULT_ORIGINAL_IMAGE,
) -> list[str]:
    objects, errors = derive_original_inventory(original_dir)
    listings, definitions, listing_errors = listing_inventory(listing_dir)
    errors.extend(listing_errors)
    if errors:
        return errors

    original_by_routine: dict[str, list[OriginalObject]] = {}
    original_table_targets: dict[str, set[tuple[str, int]]] = {}
    original_non_table_routines: set[str] = set()
    for key, item in objects.items():
        routine_ids = {reference.routine_id for reference in item.references}
        for routine_id in routine_ids:
            original_by_routine.setdefault(routine_id, []).append(item)
            if item.is_table and any(
                reference.routine_id == routine_id
                and (
                    reference.indexed
                    or reference.kind in {"xlat", "indirect"}
                )
                for reference in item.references
            ):
                original_table_targets.setdefault(routine_id, set()).add(key)
            else:
                original_non_table_routines.add(routine_id)

    for routine_id, items in sorted(original_by_routine.items()):
        if routine_id not in listings:
            sources = ", ".join(sorted({
                reference.routine_name
                for item in items
                for reference in item.references
                if reference.routine_id == routine_id
            }))
            errors.append(
                f"missing emitted counterpart for CS-owning routine {routine_id}: {sources}"
            )

    expected_code_references: dict[
        tuple[str, str], list[ListingReference]
    ] = {}
    table_candidates_by_routine: dict[str, set[str]] = {}
    for routine_id, info in listings.items():
        for reference in info.references:
            direct_cs = bool(re.search(
                rf"\bcs:{re.escape(reference.symbol)}\b", reference.text
            ))
            local_data = reference.symbol in info.data_labels
            owner = resolve_owner(reference, listings, definitions)
            globally_data = any(
                reference.symbol in candidate.data_labels
                for candidate in listings.values()
            )
            table_candidate = (
                reference.memory
                and (reference.indexed or reference.indirect)
                and (direct_cs or local_data or globally_data)
            )
            if table_candidate:
                table_candidates_by_routine.setdefault(routine_id, set()).add(
                    reference.symbol
                )
                expected_code_references.setdefault(
                    (routine_id, reference.symbol), []
                ).append(reference)
            elif direct_cs:
                expected_code_references.setdefault(
                    (routine_id, reference.symbol), []
                ).append(reference)
            elif (
                routine_id in original_non_table_routines
                and reference.address_only
                and globally_data
            ):
                expected_code_references.setdefault(
                    (routine_id, reference.symbol), []
                ).append(reference)
            elif owner is not None and owner.endswith("_TEXT") and reference.memory:
                expected_code_references.setdefault(
                    (routine_id, reference.symbol), []
                ).append(reference)

    for routine_id, expected_targets in sorted(original_table_targets.items()):
        if routine_id not in listings:
            continue
        candidates = table_candidates_by_routine.get(routine_id, set())
        if len(candidates) < len(expected_targets):
            errors.append(
                f"{listings[routine_id].name}: resolved {len(candidates)} emitted "
                f"code-table counterparts for {len(expected_targets)} original "
                "CS-relative tables"
            )

    checked_symbols: set[tuple[str, str]] = set()
    for (routine_id, symbol), references in sorted(expected_code_references.items()):
        info = listings[routine_id]
        owner = resolve_owner(references[0], listings, definitions)
        if owner is None:
            errors.append(f"{info.name}: unresolved emitted owner for {symbol}")
            continue
        if not owner.endswith("_TEXT"):
            errors.append(
                f"{info.name}: {symbol} is in {owner}, expected CODE from original CS ownership"
            )
            continue
        checked_symbols.add((routine_id, symbol))
        for reference in references:
            if reference.address_only:
                if not code_segment_literal_present(info, definitions):
                    errors.append(
                        f"{info.name}: address of {symbol} has no proven CODE segment load"
                    )
                continue
            if re.search(rf"\bcs:{re.escape(symbol)}\b", reference.text):
                continue
            segment_match = re.search(
                rf"\b(?P<segment>ds|es|fs|gs|ss):{re.escape(symbol)}\b",
                reference.text,
            )
            if segment_match and code_segment_literal_present(info, definitions):
                continue
            errors.append(
                f"{info.name}: non-CS access to code-owned {symbol}: {reference.text}"
            )

    image_data: bytes | None = None
    if original_image is not None and original_image.is_file():
        image_data = original_image.read_bytes()
    for item in sorted(objects.values(), key=lambda value: (value.group, value.target)):
        if not item.is_read_only or item.is_table:
            continue
        routine_ids = sorted({reference.routine_id for reference in item.references})
        has_candidate = any(
            routine_id in listings
            and any(key[0] == routine_id for key in checked_symbols)
            for routine_id in routine_ids
        )
        if has_candidate:
            continue
        if image_data is None:
            errors.append(
                f"{item.group}:{item.target:04x}: no original image available to "
                "prove an emitted scalar replacement"
            )
            continue
        end = item.absolute_target + item.width
        if item.absolute_target < 0 or end > len(image_data):
            errors.append(
                f"{item.group}:{item.target:04x}: original scalar lies outside image"
            )
            continue
        data = image_data[item.absolute_target:end]
        if not any(
            scalar_materialized(data, listings[routine_id].instructions)
            for routine_id in routine_ids
            if routine_id in listings
        ):
            errors.append(
                f"{item.group}:{item.target:04x}: unresolved ownership; no emitted "
                f"code object or exact {item.width}-byte scalar replacement"
            )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listing-dir", type=Path, required=True)
    parser.add_argument("--original-dir", type=Path, default=DEFAULT_ORIGINAL_DIR)
    parser.add_argument(
        "--original-image", type=Path, default=DEFAULT_ORIGINAL_IMAGE
    )
    args = parser.parse_args()
    errors = audit(args.listing_dir, args.original_dir, args.original_image)
    if errors:
        raise SystemExit("\n".join(errors))
    objects, _errors = derive_original_inventory(args.original_dir)
    references = sum(len(item.references) for item in objects.values())
    tables = sum(item.is_table for item in objects.values())
    print(
        "code-data ownership: "
        f"{references} original CS-relative references, "
        f"{len(objects)} objects, and {tables} tables verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
