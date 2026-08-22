#!/usr/bin/env python3
"""Compare original and rebuilt dynamic-memory segment roles per routine.

Natural C changes register allocation and instruction counts substantially, so
raw assembly diffs are a poor detector for segmented-memory regressions.  This
tool instead propagates the provenance of segment values and compares accesses
through dynamic segments such as the VM record image, script image, resource
buffers, VGA memory, and far-pointer arguments.

Missing roles, rebuilt-only roles, and non-equivalent access shapes are package
gate failures unless a fingerprint-bound assembly-to-listing review classifies
the complete current routine comparison.  The caller-expanded report remains
diagnostic and helps establish or reject interprocedural equivalence.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import re
import struct
import sys
from collections import Counter, deque
from dataclasses import dataclass, replace
from pathlib import Path

import capstone
from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location(
    "segment_contract_audit", ROOT / "re/tools/audit_segment_contracts.py"
)
audit = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = audit
_spec.loader.exec_module(audit)

ORIGINAL_ROW = re.compile(
    r"^(?P<address>[0-9A-Fa-f]{6,8}):\s+"
    r"(?P<bytes>(?:[0-9A-Fa-f]{2}\s+)+)\s*(?P<text>.*?)\s*$"
)
SYMBOL_EXPRESSION = re.compile(
    r"(?P<symbol>_[A-Za-z_$?][\w$?@]*)"
    r"(?P<delta>[+-]0x[0-9A-Fa-f]+|[+-]\d+)?"
)
STATIC_ROLES = {"GAME_DATA", "FS_DATA", "STACK", "CODE"}
ORIGINAL_FIXED_SEGMENTS = {
    0x0CE2: "GAME_DATA",
    0x0BBF: "FS_DATA",
}
STATIC_DISPATCH_TABLES = (
    ((0x5627, 0x56C4), 0x142D0, 0x34, 0x053A0),
    ((0x7E09,), 0x07EB4, 6, 0x077E0),
    ((0x8700,), 0x08709, 5, 0x077E0),
    ((0x4506,), 0x04522, 8, 0x02F90),
    ((0x2137,), 0x020EE, 16, 0x00EB0),
    ((0x74E5,), 0x0751E, 18, 0x053A0),
)
ARGUMENT = "argument"
UNKNOWN = "unknown"
REVIEW_REQUIRED_STATUSES = frozenset((
    "missing_role", "extra_role", "shape_difference",
))


@dataclass(frozen=True)
class LayoutEntry:
    owner: str
    offset: int


@dataclass(frozen=True)
class RoleState:
    registers: tuple[tuple[str, str], ...]
    locals: tuple[tuple[int, str], ...] = ()
    saved_segments: tuple[str, ...] = ()
    pending_pushes: tuple[str, ...] = ()
    frame_sp: int | None = None
    deltas: tuple[tuple[str, int | None], ...] = ()

    def register(self, name: str) -> str:
        return dict(self.registers).get(audit.canonical_register(name), UNKNOWN)

    def local(self, offset: int) -> str:
        return dict(self.locals).get(offset, UNKNOWN)

    def with_register(self, name: str, value: str) -> RoleState:
        values = dict(self.registers)
        values[audit.canonical_register(name)] = value
        return replace(self, registers=tuple(sorted(values.items())))

    def with_local(self, offset: int, value: str) -> RoleState:
        values = dict(self.locals)
        values[offset] = value
        return replace(self, locals=tuple(sorted(values.items())))

    def delta(self, name: str) -> int | None:
        return dict(self.deltas).get(audit.canonical_register(name))

    def with_delta(self, name: str, value: int | None) -> RoleState:
        values = dict(self.deltas)
        values[audit.canonical_register(name)] = value
        return replace(self, deltas=tuple(sorted(values.items())))


@dataclass(frozen=True)
class Access:
    role: str
    mode: str
    width: int
    form: str
    displacement: int
    site: int = 0
    operand_index: int = 0

    def shape(self) -> str:
        return f"{self.mode}{self.width}:{self.form}:{self.displacement:+#x}"


@dataclass(frozen=True)
class Comparison:
    routine: str
    status: str
    role: str
    original_count: int
    rebuilt_count: int
    missing_shapes: str
    extra_shapes: str


@dataclass(frozen=True)
class Review:
    fingerprint: str
    classification: str
    rationale: str


def read_layout(path: Path) -> dict[str, LayoutEntry]:
    result: dict[str, LayoutEntry] = {}
    with path.open(newline="", encoding="ascii") as handle:
        for row in csv.DictReader(handle, delimiter="\t"):
            if row["status"] != "known" or not row["offset"]:
                continue
            owner = "CODE" if row["segment"] == "_CODE" else row["segment"]
            result[row["symbol"].lower()] = LayoutEntry(
                owner, int(row["offset"], 0)
            )
    if not result:
        raise ValueError(f"{path}: no layout entries")
    return result


def parse_original(path: Path) -> audit.Listing:
    instructions: list[audit.ListingInstruction] = []
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = ORIGINAL_ROW.match(line)
        if not match:
            continue
        instructions.append(audit.ListingInstruction(
            int(match["address"], 16),
            bytes.fromhex(match["bytes"]),
            match["text"].strip(),
        ))
    if not instructions:
        raise ValueError(f"{path}: no original instructions")
    return audit.Listing(path, tuple(instructions), {}, {})


def initial_state(listing: audit.Listing, original: bool) -> RoleState:
    registers = {
        audit.canonical_register(name): ARGUMENT
        for name in audit.GENERAL_REGISTERS
    }
    ds_role = "GAME_DATA"
    if original:
        has_gs = False
        has_explicit_ds_frame = False
        for item in listing.instructions:
            insn = audit.decode_instruction(item)
            for operand in insn.operands:
                if operand.type != x86_const.X86_OP_MEM:
                    continue
                segment = memory_segment(insn, operand)
                if segment == "gs":
                    has_gs = True
                base = insn.reg_name(operand.mem.base).lower() \
                    if operand.mem.base else ""
                if operand.mem.segment and segment == "ds" \
                        and base in ("bp", "ebp"):
                    has_explicit_ds_frame = True
        if has_gs and has_explicit_ds_frame:
            ds_role = ARGUMENT
    registers.update({
        "cs": "CODE",
        "ds": ds_role,
        "es": ARGUMENT,
        "fs": "FS_DATA",
        "gs": "GAME_DATA",
        "ss": "STACK",
    })
    deltas = {
        audit.canonical_register(name): 0
        for name in audit.GENERAL_REGISTERS
    }
    return RoleState(
        tuple(sorted(registers.items())), deltas=tuple(sorted(deltas.items()))
    )


def merge_state(left: RoleState, right: RoleState) -> RoleState:
    left_registers = dict(left.registers)
    right_registers = dict(right.registers)
    registers = {
        key: left_registers.get(key, UNKNOWN)
        if left_registers.get(key, UNKNOWN) == right_registers.get(key, UNKNOWN)
        else UNKNOWN
        for key in left_registers.keys() | right_registers.keys()
    }
    left_locals = dict(left.locals)
    right_locals = dict(right.locals)
    locals_ = {
        key: left_locals.get(key, UNKNOWN)
        if left_locals.get(key, UNKNOWN) == right_locals.get(key, UNKNOWN)
        else UNKNOWN
        for key in left_locals.keys() | right_locals.keys()
    }
    saved = (
        left.saved_segments
        if left.saved_segments == right.saved_segments else ()
    )
    pending_pushes = (
        left.pending_pushes
        if left.pending_pushes == right.pending_pushes else ()
    )
    frame_sp = left.frame_sp if left.frame_sp == right.frame_sp else None
    left_deltas = dict(left.deltas)
    right_deltas = dict(right.deltas)
    deltas = {
        key: left_deltas.get(key)
        if left_deltas.get(key) == right_deltas.get(key) else None
        for key in left_deltas.keys() | right_deltas.keys()
    }
    return RoleState(
        tuple(sorted(registers.items())), tuple(sorted(locals_.items())),
        saved, pending_pushes, frame_sp, tuple(sorted(deltas.items())),
    )


def memory_segment(insn: capstone.CsInsn, operand) -> str:
    if operand.mem.segment:
        return insn.reg_name(operand.mem.segment).lower()
    base = insn.reg_name(operand.mem.base).lower() if operand.mem.base else ""
    return "ss" if base in ("bp", "ebp", "sp", "esp") else "ds"


def symbol_location(expression: str,
                    layout: dict[str, LayoutEntry]) -> LayoutEntry | None:
    for match in SYMBOL_EXPRESSION.finditer(expression):
        symbol = match["symbol"].lower()
        entry = layout.get(symbol)
        if entry is None:
            continue
        prefix = expression[max(0, match.start() - 12):match.start()].lower()
        if re.search(r"(?:seg|offset)\s+$", prefix):
            continue
        delta = int(match["delta"], 0) if match["delta"] else 0
        return LayoutEntry(entry.owner, (entry.offset + delta) & 0xFFFF)
    return None


def memory_value_role(item: audit.ListingInstruction, insn: capstone.CsInsn,
                      operand_index: int, operand_text: str, state: RoleState,
                      layout: dict[str, LayoutEntry], original: bool,
                      segment_half: bool = False) -> str:
    local = audit.local_offset(operand_text)
    if local is not None:
        return state.local(local + (2 if segment_half else 0))
    location = None if original else symbol_location(operand_text, layout)
    operand = insn.operands[operand_index]
    if location is not None:
        offset = location.offset + (2 if segment_half else 0)
        return f"memseg:{location.owner}:{offset & 0xFFFF:04x}"
    if operand.type != x86_const.X86_OP_MEM:
        return UNKNOWN
    segment_role = state.register(memory_segment(insn, operand))
    if segment_role in STATIC_ROLES and not operand.mem.base and not operand.mem.index:
        offset = (operand.mem.disp + (2 if segment_half else 0)) & 0xFFFF
        return f"memseg:{segment_role}:{offset:04x}"
    return f"memseg:{segment_role}:indirect"


def source_role(item: audit.ListingInstruction, insn: capstone.CsInsn,
                operand_index: int, operand_text: str, state: RoleState,
                layout: dict[str, LayoutEntry], original: bool) -> str:
    normalized = operand_text.strip().lower()
    segment_symbol = re.fullmatch(r"seg\s+(?P<symbol>_[\w$?@]+)", normalized)
    if segment_symbol:
        entry = layout.get(segment_symbol["symbol"])
        return entry.owner if entry else UNKNOWN
    if normalized.startswith("dgroup:"):
        return "GAME_DATA"
    if normalized.upper() in audit.OWNER_BY_SEGMENT_NAME:
        return audit.OWNER_BY_SEGMENT_NAME[normalized.upper()]
    register = audit.canonical_register(normalized)
    if register in audit.GENERAL_REGISTERS + audit.SEGMENT_REGISTERS:
        return state.register(register)
    local = audit.local_offset(normalized)
    if local is not None:
        return state.local(local)
    operand = insn.operands[operand_index]
    if operand.type == x86_const.X86_OP_IMM:
        if original and (operand.imm & 0xFFFF) in ORIGINAL_FIXED_SEGMENTS:
            return ORIGINAL_FIXED_SEGMENTS[operand.imm & 0xFFFF]
        return f"constant:{operand.imm & 0xFFFF:04x}"
    if operand.type == x86_const.X86_OP_MEM:
        return memory_value_role(
            item, insn, operand_index, operand_text, state, layout, original
        )
    return UNKNOWN


def transfer(item: audit.ListingInstruction, state: RoleState,
             layout: dict[str, LayoutEntry], original: bool) -> RoleState:
    insn = audit.decode_instruction(item)
    result = state
    try:
        _read, written = insn.regs_access()
    except capstone.CsError:
        written = []
    for register_id in written:
        register = audit.canonical_register(insn.reg_name(register_id))
        if register in audit.GENERAL_REGISTERS + audit.SEGMENT_REGISTERS:
            result = result.with_register(register, UNKNOWN)
            if register in audit.GENERAL_REGISTERS:
                result = result.with_delta(register, None)

    op = audit.mnemonic(item.text)
    operands = audit.split_operands(item.text)
    if op == "mov" and len(operands) == 2 and len(insn.operands) >= 2:
        destination, source = operands
        value = source_role(
            item, insn, 1, source, state, layout, original
        )
        destination_register = audit.canonical_register(destination)
        if destination_register in audit.GENERAL_REGISTERS + audit.SEGMENT_REGISTERS:
            result = result.with_register(destination_register, value)
            source_register = audit.canonical_register(source)
            if destination_register in audit.GENERAL_REGISTERS:
                if source_register in audit.GENERAL_REGISTERS:
                    delta = state.delta(source_register)
                elif insn.operands[1].type == x86_const.X86_OP_IMM:
                    offset_symbol = re.fullmatch(
                        r"offset\s+(?P<symbol>_[\w$?@]+)",
                        source.strip(), re.IGNORECASE,
                    )
                    entry = layout.get(offset_symbol["symbol"].lower()) \
                        if offset_symbol else None
                    delta = (
                        entry.offset if entry is not None else
                        insn.operands[1].imm & 0xFFFF
                    )
                else:
                    delta = None
                result = result.with_delta(destination_register, delta)
            if destination_register == "bp" and source_register == "sp":
                result = replace(
                    result, pending_pushes=(), frame_sp=0
                )
            elif destination_register == "sp" and source_register == "bp":
                result = replace(result, frame_sp=0)
        else:
            local = audit.local_offset(destination)
            if local is not None:
                result = result.with_local(local, value)
    elif op == "xor" and len(operands) == 2:
        left = audit.canonical_register(operands[0])
        right = audit.canonical_register(operands[1])
        if left == right and left in audit.GENERAL_REGISTERS:
            result = result.with_register(left, "constant:0000")
            result = result.with_delta(left, 0)
    elif op == "xchg" and len(operands) == 2:
        left = audit.canonical_register(operands[0])
        right = audit.canonical_register(operands[1])
        if (left in audit.GENERAL_REGISTERS + audit.SEGMENT_REGISTERS and
                right in audit.GENERAL_REGISTERS + audit.SEGMENT_REGISTERS):
            result = result.with_register(left, state.register(right))
            result = result.with_register(right, state.register(left))
            result = result.with_delta(left, state.delta(right))
            result = result.with_delta(right, state.delta(left))
    elif (op in ("add", "sub") and len(operands) == 2 and
            audit.canonical_register(operands[0]) in audit.GENERAL_REGISTERS and
            audit.canonical_register(operands[0]) != "sp" and
            len(insn.operands) >= 2 and
            insn.operands[1].type == x86_const.X86_OP_IMM):
        register = audit.canonical_register(operands[0])
        delta = state.delta(register)
        if delta is not None:
            amount = insn.operands[1].imm
            result = result.with_delta(
                register, delta + amount if op == "add" else delta - amount
            )
    elif op in ("inc", "dec") and len(operands) == 1:
        register = audit.canonical_register(operands[0])
        delta = state.delta(register)
        if register in audit.GENERAL_REGISTERS and delta is not None:
            result = result.with_delta(
                register, delta + (1 if op == "inc" else -1)
            )
    elif op == "lea" and len(operands) == 2 and len(insn.operands) >= 2:
        destination = audit.canonical_register(operands[0])
        source_operand = insn.operands[1]
        if (destination in audit.GENERAL_REGISTERS and
                source_operand.type == x86_const.X86_OP_MEM and
                not source_operand.mem.index):
            base = insn.reg_name(source_operand.mem.base).lower() \
                if source_operand.mem.base else ""
            base_delta = state.delta(base) if base else 0
            result = result.with_delta(
                destination,
                None if base_delta is None else
                base_delta + source_operand.mem.disp,
            )
    elif op == "push" and len(operands) == 1:
        source = audit.canonical_register(operands[0])
        value = source_role(
            item, insn, 0, operands[0], state, layout, original
        )
        result = replace(
            result,
            pending_pushes=(value,) + result.pending_pushes[:63],
        )
        if state.frame_sp is not None:
            frame_sp = state.frame_sp - 2
            result = replace(result, frame_sp=frame_sp).with_local(
                frame_sp, value
            )
        if source in audit.SEGMENT_REGISTERS:
            result = replace(
                result,
                saved_segments=(state.register(source),) + result.saved_segments,
            )
    elif op == "pop" and len(operands) == 1:
        result = replace(
            result,
            pending_pushes=(
                result.pending_pushes[1:] if result.pending_pushes else ()
            ),
        )
        destination = audit.canonical_register(operands[0])
        if state.frame_sp is not None:
            value = state.local(state.frame_sp)
            result = replace(result, frame_sp=state.frame_sp + 2)
            if destination in audit.GENERAL_REGISTERS:
                result = result.with_register(destination, value)
        if destination in audit.SEGMENT_REGISTERS:
            value = (
                result.saved_segments[0] if result.saved_segments else UNKNOWN
            )
            result = replace(
                result,
                saved_segments=(
                    result.saved_segments[1:] if result.saved_segments else ()
                ),
            ).with_register(destination, value)
    elif op in ("lds", "les", "lfs", "lgs") and len(insn.operands) >= 2:
        segment = op[1:]
        role = memory_value_role(
            item, insn, 1, operands[1], state, layout, original,
            segment_half=True,
        )
        result = result.with_register(segment, role)
        offset_role = memory_value_role(
            item, insn, 1, operands[1], state, layout, original,
            segment_half=False,
        )
        result = result.with_register(operands[0], offset_role)
        result = result.with_delta(operands[0], 0)
    elif op.startswith("lods"):
        delta = state.delta("si")
        if delta is not None:
            width = next(
                (operand.size for operand in insn.operands
                 if operand.type == x86_const.X86_OP_MEM),
                1,
            )
            result = result.with_delta("si", delta + width)
    elif op.startswith("stos"):
        delta = state.delta("di")
        if delta is not None:
            width = next(
                (operand.size for operand in insn.operands
                 if operand.type == x86_const.X86_OP_MEM),
                1,
            )
            result = result.with_delta("di", delta + width)
    elif op.startswith("movs"):
        width = next(
            (operand.size for operand in insn.operands
             if operand.type == x86_const.X86_OP_MEM),
            1,
        )
        for register in ("si", "di"):
            delta = state.delta(register)
            if delta is not None:
                result = result.with_delta(register, delta + width)
    elif op in ("call", "lcall"):
        result = replace(result, pending_pushes=())
        call_target = operands[0].strip().lower() if operands else ""
        if not original and call_target.startswith("cb_"):
            for register in ("ax", "bx", "cx", "dx"):
                result = result.with_register(register, UNKNOWN)
                result = result.with_delta(register, None)
    elif (op in ("add", "sub") and len(operands) == 2 and
            audit.canonical_register(operands[0]) == "sp" and
            state.frame_sp is not None and len(insn.operands) >= 2 and
            insn.operands[1].type == x86_const.X86_OP_IMM):
        amount = insn.operands[1].imm
        result = replace(
            result,
            frame_sp=(
                state.frame_sp + amount
                if op == "add" else state.frame_sp - amount
            ),
        )
    elif op == "leave":
        result = replace(result, frame_sp=None)
    return result


def access_mode(operand) -> str:
    read = bool(operand.access & capstone.CS_AC_READ)
    write = bool(operand.access & capstone.CS_AC_WRITE)
    if read and write:
        return "rw"
    if write:
        return "w"
    return "r"


def instruction_accesses(item: audit.ListingInstruction, state: RoleState,
                         layout: dict[str, LayoutEntry], original: bool,
                         include_static: bool = False) \
        -> tuple[Access, ...]:
    insn = audit.decode_instruction(item)
    known_offsets = {
        (entry.owner, entry.offset) for entry in layout.values()
    }
    result: list[Access] = []
    for operand_index, operand in enumerate(insn.operands):
        if operand.type != x86_const.X86_OP_MEM:
            continue
        segment = memory_segment(insn, operand)
        role = state.register(segment)
        if (original and not operand.mem.base and not operand.mem.index and
              ("GAME_DATA", operand.mem.disp & 0xFFFF) in known_offsets):
            role = "GAME_DATA"
        if role in STATIC_ROLES and (
                not include_static or role in {"STACK", "CODE"}):
            continue
        form = "direct" if not operand.mem.base and not operand.mem.index else "based"
        location = None if original else symbol_location(item.text, layout)
        displacement = (
            location.offset
            if location is not None and location.owner == role
            else operand.mem.disp
        )
        if (location is None and normalize_role(role) != "dynamic" and
                operand.mem.base and not operand.mem.index):
            base = insn.reg_name(operand.mem.base).lower()
            delta = state.delta(base)
            if delta is not None:
                displacement += delta
        if 0 <= displacement <= 0xFFFF and displacement & 0x8000:
            displacement -= 0x10000
        result.append(Access(
            role, access_mode(operand), operand.size, form, displacement,
            item.offset, operand_index,
        ))
    return tuple(result)


def call_state(state: RoleState) -> RoleState:
    return RoleState(
        state.registers,
        pending_pushes=state.pending_pushes,
        deltas=state.deltas,
    )


def callee_state(state: RoleState, listing: audit.Listing) -> RoleState:
    far_return = any(
        audit.mnemonic(item.text) == "retf" for item in listing.instructions
    )
    pushes_before_bp = 0
    for item in listing.instructions:
        op = audit.mnemonic(item.text)
        operands = audit.split_operands(item.text)
        if (op == "mov" and len(operands) == 2 and
                audit.canonical_register(operands[0]) == "bp" and
                audit.canonical_register(operands[1]) == "sp"):
            break
        if (op == "push" and operands and
                audit.canonical_register(operands[0]) != "bp"):
            pushes_before_bp += 1
    return_bytes = 4 if far_return else 2
    first_argument = 2 + pushes_before_bp * 2 + return_bytes
    locals_ = {
        first_argument + index * 2: role
        for index, role in enumerate(state.pending_pushes)
    }
    return RoleState(
        state.registers, tuple(sorted(locals_.items())), deltas=state.deltas
    )


def analyze(listing: audit.Listing, layout: dict[str, LayoutEntry],
            original: bool, entry_state: RoleState | None = None,
            call_resolver=None, include_static: bool = False) \
        -> tuple[Counter[Access], dict[str, list[RoleState]]]:
    by_offset = {item.offset: item for item in listing.instructions}
    edges = audit.successors(listing)
    states: dict[int, RoleState] = {}
    unseeded = set(by_offset)
    first_component = True
    while unseeded:
        entry = min(unseeded)
        states[entry] = (
            entry_state
            if first_component and entry_state is not None
            else initial_state(listing, original)
        )
        first_component = False
        pending = deque([entry])
        while pending:
            offset = pending.popleft()
            unseeded.discard(offset)
            outgoing = transfer(by_offset[offset], states[offset], layout, original)
            for target in edges[offset]:
                previous = states.get(target)
                merged = outgoing if previous is None else merge_state(previous, outgoing)
                if previous != merged:
                    states[target] = merged
                    pending.append(target)
    accesses: Counter[Access] = Counter()
    calls: dict[str, list[RoleState]] = {}
    for offset, state in states.items():
        accesses.update(instruction_accesses(
            by_offset[offset], state, layout, original, include_static
        ))
        if call_resolver is not None:
            resolved = call_resolver(by_offset[offset])
            if resolved is not None:
                targets = (resolved,) if isinstance(resolved, str) else resolved
                for target in targets:
                    calls.setdefault(target, []).append(call_state(state))
    return accesses, calls


def merged_states(states: list[RoleState]) -> RoleState:
    result = states[0]
    for state in states[1:]:
        result = merge_state(result, state)
    return result


def interprocedural_accesses(
        listings: dict[str, audit.Listing],
        layout: dict[str, LayoutEntry],
        original: bool,
        call_resolver,
        include_static: bool = False) -> dict[str, Counter[Access]]:
    entries = {
        stem: initial_state(listing, original)
        for stem, listing in listings.items()
    }
    results: dict[str, Counter[Access]] = {}
    for _iteration in range(20):
        incoming: dict[str, list[RoleState]] = {}
        results = {}
        for stem, listing in listings.items():
            accesses, calls = analyze(
                listing, layout, original, entries[stem], call_resolver,
                include_static,
            )
            results[stem] = accesses
            for target, states in calls.items():
                if target in listings:
                    incoming.setdefault(target, []).extend(states)
        updated = dict(entries)
        for stem, states in incoming.items():
            updated[stem] = merged_states([
                callee_state(state, listings[stem]) for state in states
            ])
        if updated == entries:
            return results
        entries = updated
    raise ValueError("interprocedural segment provenance did not converge")


def static_dispatch_targets(entries: dict[int, str]) \
        -> dict[int, tuple[str, ...]]:
    image = (ROOT / "re/bin/BLOODPRG.EXE").read_bytes()
    result: dict[int, tuple[str, ...]] = {}
    for sites, table_offset, count, target_base in STATIC_DISPATCH_TABLES:
        targets = []
        for index in range(count):
            raw = struct.unpack_from("<H", image, table_offset + index * 2)[0]
            stem = entries.get(target_base + raw)
            if stem is not None and stem not in targets:
                targets.append(stem)
        for site in sites:
            result[site] = tuple(targets)
    return result


def normalize_role(role: str) -> str:
    if (role in (ARGUMENT, UNKNOWN) or
            "argument" in role or "unknown" in role or "STACK" in role or
            role.startswith("memseg:memseg:")):
        return "dynamic"
    return role


def comparison_signature(row: Comparison) -> str:
    return "\t".join((
        row.routine,
        row.status,
        row.role,
        str(row.original_count),
        str(row.rebuilt_count),
        row.missing_shapes,
        row.extra_shapes,
    ))


def routine_fingerprints(rows: list[Comparison]) -> dict[str, str]:
    grouped: dict[str, list[Comparison]] = {}
    for row in rows:
        grouped.setdefault(row.routine, []).append(row)
    return {
        routine: hashlib.sha256(
            "\n".join(
                comparison_signature(row)
                for row in sorted(routine_rows, key=lambda item: (
                    item.status, item.role, item.original_count,
                    item.rebuilt_count, item.missing_shapes, item.extra_shapes,
                ))
            ).encode("ascii")
        ).hexdigest()[:16]
        for routine, routine_rows in grouped.items()
    }


def read_reviews(path: Path | None) -> dict[tuple[str, str], Review]:
    if path is None or not path.is_file():
        return {}
    reviews: dict[tuple[str, str], Review] = {}
    with path.open(newline="", encoding="ascii") as handle:
        for row in csv.DictReader(handle, delimiter="\t"):
            key = (row["routine"], row["segment_role"])
            if key in reviews:
                raise ValueError(f"{path}: duplicate review for {key!r}")
            fingerprint = row["fingerprint"].strip().lower()
            rationale = row["rationale"].strip()
            if not re.fullmatch(r"[0-9a-f]{16}", fingerprint):
                raise ValueError(f"{path}: invalid review fingerprint for {key!r}")
            if not rationale:
                raise ValueError(f"{path}: empty review rationale for {key!r}")
            reviews[key] = Review(
                fingerprint, row["classification"], rationale
            )
    return reviews


def apply_reviews(rows: list[Comparison], reviews: dict[tuple[str, str], Review],
                  fingerprints: dict[str, str] | None = None) \
        -> list[Comparison]:
    if fingerprints is None:
        fingerprints = routine_fingerprints(rows)
    used: set[tuple[str, str]] = set()
    result: list[Comparison] = []
    for row in rows:
        key = (row.routine, row.role)
        review = reviews.get(key)
        review_key = key
        if review is None and row.status in REVIEW_REQUIRED_STATUSES:
            review_key = (row.routine, "*")
            review = reviews.get(review_key)
        if review is None:
            result.append(row)
            continue
        used.add(review_key)
        actual = fingerprints[row.routine]
        if review.fingerprint != actual:
            raise ValueError(
                f"stale segment-role review for {row.routine} {row.role}: "
                f"{review.fingerprint} != {actual}"
            )
        if review.classification == "equivalent":
            status = "reviewed_equivalent"
        elif review.classification == "bug":
            status = "confirmed_bug"
        else:
            raise ValueError(
                f"invalid review classification {review.classification!r} "
                f"for {row.routine} {row.role}"
            )
        result.append(replace(row, status=status))
    unused = sorted(reviews.keys() - used)
    if unused:
        raise ValueError(f"unused segment-role reviews: {unused!r}")
    return result


def byte_footprint(accesses: Counter[Access]) \
        -> frozenset[tuple[str, str, int]]:
    result: set[tuple[str, str, int]] = set()
    for access in accesses:
        modes = ("r", "w") if access.mode == "rw" else (access.mode,)
        for mode in modes:
            for byte in range(access.width):
                result.add((mode, access.form, access.displacement + byte))
    return frozenset(result)


def accesses_by_role(accesses: Counter[Access]) \
        -> dict[str, Counter[Access]]:
    result: dict[str, Counter[Access]] = {}
    for access, count in accesses.items():
        result.setdefault(normalize_role(access.role), Counter())[access] += count
    return result


def shape_counts(accesses: Counter[Access]) -> Counter[str]:
    result: Counter[str] = Counter()
    for access, count in accesses.items():
        result[access.shape()] += count
    return result


def compare(routine: str, original: Counter[Access], rebuilt: Counter[Access]) \
        -> list[Comparison]:
    original_accesses = accesses_by_role(original)
    rebuilt_accesses = accesses_by_role(rebuilt)
    rows: list[Comparison] = []
    for role in sorted(original_accesses.keys() | rebuilt_accesses.keys()):
        before_accesses = original_accesses.get(role, Counter())
        after_accesses = rebuilt_accesses.get(role, Counter())
        before = shape_counts(before_accesses)
        after = shape_counts(after_accesses)
        if before and not after:
            status = "missing_role"
        elif after and not before:
            status = "extra_role"
        elif before == after:
            status = "exact"
        elif byte_footprint(before_accesses) == byte_footprint(after_accesses):
            status = "footprint_equivalent"
        else:
            status = "shape_difference"
        missing = before - after
        extra = after - before
        rows.append(Comparison(
            routine,
            status,
            role,
            sum(before.values()),
            sum(after.values()),
            ",".join(f"{shape}x{count}" for shape, count in sorted(missing.items())),
            ",".join(f"{shape}x{count}" for shape, count in sorted(extra.items())),
        ))
    return rows


def context_covers_local_accesses(
        local: Counter[Access],
        own_context: Counter[Access],
        other_context: Counter[Access]) -> bool:
    own_by_site = {
        (access.site, access.operand_index): access
        for access in own_context
    }
    mapped: dict[str, Counter[Access]] = {}
    for access, count in local.items():
        contextual = own_by_site.get((access.site, access.operand_index))
        if contextual is None:
            return False
        role = normalize_role(contextual.role)
        mapped.setdefault(role, Counter())[contextual] += count
    if not mapped:
        return False
    other_by_role = accesses_by_role(other_context)
    return all(
        role in other_by_role and
        byte_footprint(accesses).issubset(
            byte_footprint(other_by_role[role])
        )
        for role, accesses in mapped.items()
    )


def add_context_evidence(local_rows: list[Comparison],
                         context_rows: list[Comparison],
                         original_local: Counter[Access] | None = None,
                         rebuilt_local: Counter[Access] | None = None,
                         original_context: Counter[Access] | None = None,
                         rebuilt_context: Counter[Access] | None = None) \
        -> list[Comparison]:
    context_is_equivalent = bool(context_rows) and all(
        row.status in ("exact", "footprint_equivalent")
        for row in context_rows
    )
    original_groups = accesses_by_role(original_local or Counter())
    rebuilt_groups = accesses_by_role(rebuilt_local or Counter())
    result: list[Comparison] = []
    for row in local_rows:
        equivalent = context_is_equivalent
        if (not equivalent and original_context is not None and
                rebuilt_context is not None):
            if row.status == "missing_role":
                equivalent = context_covers_local_accesses(
                    original_groups.get(row.role, Counter()),
                    original_context, rebuilt_context,
                )
            elif row.status == "extra_role":
                equivalent = context_covers_local_accesses(
                    rebuilt_groups.get(row.role, Counter()),
                    rebuilt_context, original_context,
                )
            elif row.status == "shape_difference":
                equivalent = (
                    context_covers_local_accesses(
                        original_groups.get(row.role, Counter()),
                        original_context, rebuilt_context,
                    ) and
                    context_covers_local_accesses(
                        rebuilt_groups.get(row.role, Counter()),
                        rebuilt_context, original_context,
                    )
                )
        if equivalent and row.status in (
                "missing_role", "extra_role", "shape_difference"):
            row = replace(row, status="interprocedural_equivalent")
        result.append(row)
    return result


def original_call_resolver(entries: dict[int, str]):
    dispatch = static_dispatch_targets(entries)

    def resolve(item: audit.ListingInstruction) -> str | tuple[str, ...] | None:
        insn = audit.decode_instruction(item)
        if x86_const.X86_GRP_CALL not in set(insn.groups):
            return None
        if item.offset in dispatch:
            return dispatch[item.offset]
        match = re.fullmatch(r"call\s+0x([0-9a-f]+)",
                             item.text.strip(), re.IGNORECASE)
        if not match:
            return None
        return entries.get(int(match.group(1), 16))
    return resolve


def rebuilt_call_resolver(functions: dict[str, str]):
    def resolve(item: audit.ListingInstruction) -> str | None:
        if audit.mnemonic(item.text) not in ("call", "lcall"):
            return None
        operands = audit.split_operands(item.text)
        if len(operands) != 1:
            return None
        symbol = operands[0].strip().lower()
        if symbol.endswith("_"):
            symbol = symbol[:-1]
        return functions.get(symbol)
    return resolve


def write_comparisons(stream, rows: list[Comparison]) -> None:
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "routine", "status", "segment_role", "original_accesses",
        "rebuilt_accesses", "missing_shapes", "extra_shapes",
    ))
    priority = {
        "missing_role": 0,
        "extra_role": 1,
        "shape_difference": 2,
        "footprint_equivalent": 3,
        "interprocedural_equivalent": 4,
        "reviewed_equivalent": 5,
        "confirmed_bug": 6,
        "exact": 7,
    }
    for row in sorted(rows, key=lambda item: (
        priority[item.status], item.routine, item.role
    )):
        writer.writerow((
            row.routine, row.status, row.role, row.original_count,
            row.rebuilt_count, row.missing_shapes, row.extra_shapes,
        ))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest", type=Path,
        default=ROOT / "re/source/bloodprg/candidates/manifest.tsv",
    )
    parser.add_argument(
        "--object-dir", type=Path,
        default=ROOT / "output/recovered_dos_package/bloodprg_objects/bloodprg",
    )
    parser.add_argument(
        "--link-map", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/"
            "final/link.map"
        ),
    )
    parser.add_argument(
        "--data-layout", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/"
            "data_owner/data_layout.tsv"
        ),
    )
    parser.add_argument(
        "--listing-cache", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/"
            "final/segment_contract_listings"
        ),
    )
    parser.add_argument("--wdis", type=Path, default=Path("wdis"))
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--context-output", type=Path,
        help="write the caller-resolved static and dynamic effect comparison",
    )
    parser.add_argument(
        "--reviews", type=Path,
        default=ROOT / "re/source/bloodprg/segment_role_reviews.tsv",
    )
    parser.add_argument(
        "--review-template", type=Path,
        help="write every unresolved memory-effect fingerprint",
    )
    parser.add_argument("--fail-unreviewed", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    layout = read_layout(args.data_layout.resolve())
    linked = audit.linked_project_stems(args.link_map.resolve())
    objects = {
        path.stem.lower(): path
        for path in args.object_dir.resolve().glob("*.[Oo][Bb][Jj]")
    }
    manifest: dict[str, dict[str, str]] = {}
    with args.manifest.resolve().open(newline="", encoding="ascii") as handle:
        for row in csv.DictReader(handle, delimiter="\t"):
            manifest[Path(row["source"]).stem.lower()] = row

    original_listings: dict[str, audit.Listing] = {}
    rebuilt_listings: dict[str, audit.Listing] = {}
    original_entries: dict[int, str] = {}
    function_stems: dict[str, str] = {}
    for stem in sorted(linked):
        row = manifest.get(stem)
        if row is None:
            raise SystemExit(f"manifest has no linked routine {stem}")
        original_listings[stem] = parse_original(ROOT / row["asm_path"])
        rebuilt_listings[stem] = audit.listing_for_object(
            args.wdis, objects[stem], args.listing_cache.resolve()
        )
        original_entries[int(row["entry"], 0)] = stem
        function_stems[row["function"].lower()] = stem

    original_context_accesses = interprocedural_accesses(
        original_listings,
        layout,
        original=True,
        call_resolver=original_call_resolver(original_entries),
        include_static=True,
    )
    rebuilt_context_accesses = interprocedural_accesses(
        rebuilt_listings,
        layout,
        original=False,
        call_resolver=rebuilt_call_resolver(function_stems),
        include_static=True,
    )
    original_accesses = {
        stem: analyze(listing, layout, original=True)[0]
        for stem, listing in original_listings.items()
    }
    rebuilt_accesses = {
        stem: analyze(listing, layout, original=False)[0]
        for stem, listing in rebuilt_listings.items()
    }
    rows: list[Comparison] = []
    all_context_rows: list[Comparison] = []
    for stem in sorted(linked):
        routine_rows = compare(
            stem,
            original_accesses[stem],
            rebuilt_accesses[stem],
        )
        context_rows = compare(
            stem,
            original_context_accesses[stem],
            rebuilt_context_accesses[stem],
        )
        all_context_rows.extend(context_rows)
        rows.extend(add_context_evidence(
            routine_rows,
            context_rows,
            original_accesses[stem],
            rebuilt_accesses[stem],
            original_context_accesses[stem],
            rebuilt_context_accesses[stem],
        ))

    fingerprints = routine_fingerprints(rows)
    reviews = read_reviews(args.reviews.resolve() if args.reviews else None)
    rows = apply_reviews(rows, reviews, fingerprints)
    if args.review_template:
        with args.review_template.open("w", newline="", encoding="ascii") as handle:
            writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
            writer.writerow((
                "routine", "segment_role", "fingerprint", "classification",
                "rationale",
            ))
            for row in rows:
                if row.status in REVIEW_REQUIRED_STATUSES:
                    writer.writerow((
                        row.routine, row.role,
                        fingerprints[row.routine], "", ""
                    ))

    stream = args.output.open("w", newline="", encoding="ascii") \
        if args.output else sys.stdout
    write_comparisons(stream, rows)
    if args.output:
        stream.close()
    if args.context_output:
        with args.context_output.open(
                "w", newline="", encoding="ascii") as context_stream:
            write_comparisons(context_stream, all_context_rows)
        context_counts = Counter(row.status for row in all_context_rows)
        print(
            f"caller context: {len(all_context_rows)} roles; "
            f"{context_counts['missing_role']} missing; "
            f"{context_counts['extra_role']} extra; "
            f"{context_counts['shape_difference']} shape differences; "
            f"{context_counts['footprint_equivalent']} footprint equivalents; "
            f"{context_counts['exact']} exact"
        )
    counts = Counter(row.status for row in rows)
    print(
        f"{len(linked)} routines; {len(rows)} dynamic segment roles; "
        f"{counts['missing_role']} missing; {counts['extra_role']} extra; "
        f"{counts['shape_difference']} shape differences; "
        f"{counts['footprint_equivalent']} footprint equivalents; "
        f"{counts['interprocedural_equivalent']} interprocedural equivalents; "
        f"{counts['reviewed_equivalent']} reviewed equivalents; "
        f"{counts['confirmed_bug']} confirmed bugs; "
        f"{counts['exact']} exact"
    )
    if args.fail_unreviewed and (
            any(counts[status] for status in REVIEW_REQUIRED_STATUSES) or
            counts["confirmed_bug"]):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
