#!/usr/bin/env python3
"""Compare original and relinked BLOODPRG register/FLAGS exit contracts.

The audit is intentionally fail closed.  It symbolically propagates entry
register values, FLAGS, saved stack values, and known callee summaries through
the complete control-flow graph of every recovered routine.  An original
register is part of the routine contract only when every reachable original
exit preserves it.  Every emitted exit must preserve that contract.

An indirect transfer is accepted only when an existing static table parser can
prove all of its targets.  Unknown indirect transfers, external call effects,
interrupt effects, disconnected instructions, incompatible stack joins, and
missing exits are blockers; none is silently treated as a harmless terminal.
"""
from __future__ import annotations

import os
from pathlib import Path
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path
    for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import argparse
import capstone
import csv
import importlib.util
import io
import re
from collections import deque
from dataclasses import dataclass, replace
from typing import Callable, Iterable

from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]


def _load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


CORE = _load_module(
    "relinked_register_segment_core", ROOT / "re/tools/audit_segment_contracts.py"
)
ROLES = _load_module(
    "relinked_register_role_core", ROOT / "re/tools/compare_segment_roles.py"
)
MAPS = _load_module(
    "relinked_register_map_core", ROOT / "re/tools/audit_xdb_tail_transfers.py"
)

GENERAL_REGISTERS = ("eax", "ebx", "ecx", "edx", "esi", "edi", "ebp")
SEGMENT_REGISTERS = ("ds", "es", "fs", "gs", "ss")
CONTRACT_REGISTERS = tuple(name[1:].upper() for name in GENERAL_REGISTERS) + (
    "SP", "DS", "ES", "FS", "GS", "SS",
)
UNKNOWN = "?"

REGISTER_SLICES = {
    "al": ("eax", 0, 1), "ah": ("eax", 1, 1),
    "ax": ("eax", 0, 2), "eax": ("eax", 0, 4),
    "bl": ("ebx", 0, 1), "bh": ("ebx", 1, 1),
    "bx": ("ebx", 0, 2), "ebx": ("ebx", 0, 4),
    "cl": ("ecx", 0, 1), "ch": ("ecx", 1, 1),
    "cx": ("ecx", 0, 2), "ecx": ("ecx", 0, 4),
    "dl": ("edx", 0, 1), "dh": ("edx", 1, 1),
    "dx": ("edx", 0, 2), "edx": ("edx", 0, 4),
    "si": ("esi", 0, 2), "esi": ("esi", 0, 4),
    "di": ("edi", 0, 2), "edi": ("edi", 0, 4),
    "bp": ("ebp", 0, 2), "ebp": ("ebp", 0, 4),
    "ds": ("ds", 0, 2), "es": ("es", 0, 2),
    "fs": ("fs", 0, 2), "gs": ("gs", 0, 2),
    "ss": ("ss", 0, 2),
}

WRITE_FLAGS_MASK = 0
for _constant in dir(x86_const):
    if _constant.startswith((
        "X86_EFLAGS_MODIFY_", "X86_EFLAGS_RESET_",
        "X86_EFLAGS_SET_", "X86_EFLAGS_UNDEFINED_",
    )):
        WRITE_FLAGS_MASK |= getattr(x86_const, _constant)

NUMERIC = r"(?:0x[0-9a-f]+|[0-9]+)"
FAR_TRANSFER = re.compile(
    rf"^(?:l?call|l?jmp)\s+(?P<segment>{NUMERIC})\s*,\s*"
    rf"(?P<offset>{NUMERIC})$",
    re.IGNORECASE,
)
MARKED_MAP_SYMBOL = re.compile(
    r"^\s*(?P<segment>[0-9a-f]{4}):(?P<offset>[0-9a-f]{4,8})[*+]?\s+"
    r"(?P<symbol>[A-Za-z_$?][\w$?@]*)\s*$",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class MachineState:
    registers: tuple[tuple[str, tuple[str, ...]], ...]
    flags: tuple[str, str]
    sp_delta: int | None = 0
    bp_delta: int | None = None
    stack: tuple[tuple[int, str], ...] = ()

    def register(self, name: str) -> tuple[str, ...]:
        parent, start, width = REGISTER_SLICES[name.lower()]
        return dict(self.registers)[parent][start:start + width]

    def with_register(self, name: str, value: tuple[str, ...]) -> "MachineState":
        parent, start, width = REGISTER_SLICES[name.lower()]
        if len(value) != width:
            raise ValueError(f"{name} needs {width} byte tokens, got {len(value)}")
        registers = dict(self.registers)
        current = list(registers[parent])
        current[start:start + width] = value
        registers[parent] = tuple(current)
        return replace(self, registers=tuple(sorted(registers.items())))

    def with_parent_unknown(self, parent: str) -> "MachineState":
        width = 2 if parent in SEGMENT_REGISTERS else 4
        registers = dict(self.registers)
        registers[parent] = (UNKNOWN,) * width
        return replace(self, registers=tuple(sorted(registers.items())))


@dataclass(frozen=True)
class Routine:
    key: str
    display_name: str
    listing: object
    edges: tuple[tuple[int, tuple[int, ...]], ...]
    calls: tuple[tuple[int, tuple[str, ...]], ...]
    tails: tuple[tuple[int, tuple[str, ...]], ...]
    exits: tuple[int, ...]
    effects: tuple[tuple[int, EffectContract], ...]
    opaque_effects: frozenset[int]
    stack_consuming_calls: frozenset[int]
    direct_blockers: frozenset[str]
    public_effect: EffectContract | None


@dataclass(frozen=True)
class ExitContract:
    offset: int
    preserved: frozenset[str]
    flags_preserved: bool
    cleanup: int


@dataclass(frozen=True)
class EffectContract:
    modifies: frozenset[str]
    flags_preserved: bool = False
    cleanup: int = 0


@dataclass(frozen=True)
class Summary:
    preserved: frozenset[str]
    flags_preserved: bool
    cleanup: int | None
    exits: tuple[ExitContract, ...]
    blockers: frozenset[str]


@dataclass(frozen=True)
class Comparison:
    routine: str
    status: str
    emitted_exit: str
    original_preserved: str
    emitted_clobbers: str
    original_flags: str
    emitted_flags: str
    original_cleanup: str
    emitted_cleanup: str
    blockers: str


TargetResolver = Callable[[object, str], tuple[str, ...] | None]
EffectResolver = Callable[[object], EffectContract | None]


def register_effect(*modifies: str) -> EffectContract:
    return EffectContract(frozenset(modifies))


PUBLIC_LINKED_EFFECTS = {
    "int86": register_effect("AX", "DX", "ES"),
    "int86x": register_effect("AX", "DX", "ES"),
    "_bios_keybrd": register_effect("AX"),
    "_dos_setdrive": register_effect("AX"),
    "chdir": register_effect("AX"),
}
LINKED_INDIRECT_EFFECTS = {
    "cb_overlay_call_inherited_bp": register_effect(
        "AX", "BX", "CX", "DX", "SI", "DI"
    ),
    "cb_xms_move": register_effect("AX", "BX", "CX", "DX"),
    "cb_xms_release": register_effect("AX", "BX", "CX", "DX"),
    "cb_xms_allocate_kb": register_effect("AX", "BX", "CX", "DX"),
    "cb_snd_stream_service": register_effect(
        "AX", "BX", "CX", "DX", "SI", "DI"
    ),
    "cb_snd_stream_play": register_effect(
        "AX", "BX", "CX", "DX", "SI", "DI"
    ),
    "cb_snd_clip_play": register_effect(
        "AX", "BX", "CX", "DX", "SI", "DI"
    ),
}
RECOVERED_INDIRECT_EFFECTS = (
    ("_audio_position_callback", register_effect("AX", "DX")),
    (
        "_snd_driver_entries",
        register_effect("AX", "BX", "CX", "DX", "SI", "DI", "ES"),
    ),
    (
        "_snd_driver_callback",
        register_effect("AX", "BX", "CX", "DX"),
    ),
)


def canonical_symbol(symbol: str) -> str:
    return symbol.lower().rstrip("_")


def recovered_effect_resolver(item) -> EffectContract | None:
    instruction = CORE.decode_instruction(item)
    if x86_const.X86_GRP_CALL not in set(instruction.groups):
        return None
    lowered = item.text.lower()
    for marker, effect in RECOVERED_INDIRECT_EFFECTS:
        if marker in lowered:
            return effect
    return None


def linked_effect_resolver(symbols: Iterable[str]) -> EffectResolver:
    names = {canonical_symbol(symbol) for symbol in symbols}
    indirect = next(
        (LINKED_INDIRECT_EFFECTS[name] for name in names
         if name in LINKED_INDIRECT_EFFECTS),
        None,
    )
    dos_find_first = "cb_dos_find_first" in names

    def resolve(item) -> EffectContract | None:
        instruction = CORE.decode_instruction(item)
        if instruction.mnemonic in ("int", "int1", "int3", "into"):
            target = immediate_target(instruction)
            if dos_find_first and target == 0x21:
                return register_effect("AX")
            return None
        if (
            indirect is not None
            and x86_const.X86_GRP_CALL in set(instruction.groups)
            and immediate_target(instruction) is None
            and far_target(item.text, 0) is None
        ):
            return indirect
        return None

    return resolve


def public_linked_effect(symbols: Iterable[str]) -> EffectContract | None:
    effects = {
        PUBLIC_LINKED_EFFECTS[name]
        for name in {canonical_symbol(symbol) for symbol in symbols}
        if name in PUBLIC_LINKED_EFFECTS
    }
    if len(effects) > 1:
        raise ValueError(f"linked aliases disagree on public ABI: {sorted(symbols)}")
    return next(iter(effects), None)


def initial_state() -> MachineState:
    registers: dict[str, tuple[str, ...]] = {}
    for name in GENERAL_REGISTERS:
        registers[name] = tuple(f"entry:{name}:{index}" for index in range(4))
    for name in SEGMENT_REGISTERS:
        registers[name] = tuple(f"entry:{name}:{index}" for index in range(2))
    return MachineState(
        tuple(sorted(registers.items())),
        ("entry:flags:0", "entry:flags:1"),
    )


INITIAL_STATE = initial_state()
INITIAL_REGISTERS = dict(INITIAL_STATE.registers)
INITIAL_FLAGS = INITIAL_STATE.flags


def instruction_is_conditional_jump(instruction) -> bool:
    return (
        x86_const.X86_GRP_JUMP in set(instruction.groups)
        and instruction.mnemonic not in ("jmp", "ljmp")
    )


def immediate_target(instruction) -> int | None:
    if not instruction.operands:
        return None
    operand = instruction.operands[0]
    if operand.type != x86_const.X86_OP_IMM:
        return None
    return operand.imm


def far_target(text: str, header_size: int) -> int | None:
    match = FAR_TRANSFER.fullmatch(text.strip())
    if match is None:
        return None
    segment = int(match["segment"], 0)
    offset = int(match["offset"], 0)
    return header_size + segment * 16 + offset


def emitted_symbol(item) -> str | None:
    instruction = CORE.decode_instruction(item)
    if (
        not instruction.operands
        or instruction.operands[0].type != x86_const.X86_OP_IMM
    ):
        return None
    operands = CORE.split_operands(item.text)
    if len(operands) != 1:
        return None
    value = operands[0].strip().lower()
    value = re.sub(r"^(?:near|far|word|dword)\s+ptr\s+", "", value)
    if "[" in value or re.fullmatch(NUMERIC, value, re.IGNORECASE):
        return None
    return value


def symbol_variants(symbol: str) -> tuple[str, ...]:
    lowered = symbol.lower()
    variants = [lowered]
    if lowered.endswith("_"):
        variants.append(lowered[:-1])
    else:
        variants.append(lowered + "_")
    return tuple(dict.fromkeys(variants))


def local_jump_table_targets(listing, item) -> tuple[int, ...]:
    match = re.search(r"\bcs:(?P<label>[\w$?@]+)", item.text, re.IGNORECASE)
    if match is None:
        return ()
    return listing.jump_tables.get(match["label"], ())


def build_routine(
    key: str,
    display_name: str,
    listing,
    resolver: TargetResolver,
    *,
    effect_resolver: EffectResolver | None = None,
    public_effect: EffectContract | None = None,
    interrupt_effects_unresolved: bool = True,
) -> Routine:
    by_offset = {item.offset: item for item in listing.instructions}
    offsets = set(by_offset)
    entries = listing.entrypoints or (min(offsets),)
    if len(entries) != 1:
        raise ValueError(
            f"{display_name}: expected one routine entry, found {len(entries)}"
        )

    edges: dict[int, tuple[int, ...]] = {}
    calls: dict[int, tuple[str, ...]] = {}
    tails: dict[int, tuple[str, ...]] = {}
    exits: list[int] = []
    effects: dict[int, EffectContract] = {}
    opaque_effects: set[int] = set()
    blockers: set[str] = set()

    for item in listing.instructions:
        instruction = CORE.decode_instruction(item)
        groups = set(instruction.groups)
        next_offset = item.offset + len(item.data)
        successors: list[int] = []

        if x86_const.X86_GRP_CALL in groups:
            targets = resolver(item, "call")
            if not targets:
                effect = (
                    effect_resolver(item) if effect_resolver is not None else None
                )
                if effect is not None:
                    effects[item.offset] = effect
                elif public_effect is None:
                    blockers.add(
                        f"{display_name}@0x{item.offset:x}: unresolved indirect or "
                        f"external call effect: {item.text.strip()}"
                    )
                    opaque_effects.add(item.offset)
                calls[item.offset] = ()
            else:
                calls[item.offset] = targets
            if next_offset in offsets:
                successors.append(next_offset)
            else:
                blockers.add(
                    f"{display_name}@0x{item.offset:x}: call has no in-routine "
                    "fallthrough"
                )
        elif x86_const.X86_GRP_JUMP in groups:
            target = immediate_target(instruction)
            if target is not None:
                target &= 0xFFFFFFFF
            symbolic_targets = (
                resolver(item, "jump") if emitted_symbol(item) is not None else None
            )
            table_targets = () if target is not None else local_jump_table_targets(
                listing, item
            )
            if symbolic_targets:
                tails[item.offset] = symbolic_targets
            elif target in offsets:
                successors.append(target)
            elif table_targets:
                missing = sorted(set(table_targets) - offsets)
                if missing:
                    blockers.add(
                        f"{display_name}@0x{item.offset:x}: jump table has "
                        f"out-of-routine targets {','.join(hex(x) for x in missing)}"
                    )
                successors.extend(target for target in table_targets if target in offsets)
            else:
                targets = resolver(item, "jump")
                if targets:
                    tails[item.offset] = targets
                else:
                    blockers.add(
                        f"{display_name}@0x{item.offset:x}: unresolved indirect or "
                        f"external jump: {item.text.strip()}"
                    )
            if instruction_is_conditional_jump(instruction):
                if next_offset in offsets:
                    successors.append(next_offset)
                else:
                    blockers.add(
                        f"{display_name}@0x{item.offset:x}: conditional jump has "
                        "no in-routine fallthrough"
                    )
        elif x86_const.X86_GRP_RET in groups or instruction.mnemonic in (
            "iret", "iretd",
        ):
            exits.append(item.offset)
        else:
            if instruction.mnemonic in ("int", "int1", "int3", "into"):
                effect = (
                    effect_resolver(item) if effect_resolver is not None else None
                )
                if effect is not None:
                    effects[item.offset] = effect
                elif public_effect is None:
                    opaque_effects.add(item.offset)
                if (
                    effect is None
                    and public_effect is None
                    and interrupt_effects_unresolved
                ):
                    blockers.add(
                        f"{display_name}@0x{item.offset:x}: interrupt register/FLAGS "
                        f"effect is unresolved: {item.text.strip()}"
                    )
            if next_offset in offsets:
                successors.append(next_offset)
            else:
                blockers.add(
                    f"{display_name}@0x{item.offset:x}: control falls out of routine: "
                    f"{item.text.strip()}"
                )
        edges[item.offset] = tuple(dict.fromkeys(successors))

    reachable = set(entries)
    pending = deque(entries)
    while pending:
        offset = pending.popleft()
        for target in edges[offset]:
            if target not in reachable:
                reachable.add(target)
                pending.append(target)
    disconnected = sorted(offsets - reachable)
    if disconnected:
        shown = ",".join(f"0x{x:x}" for x in disconnected[:8])
        suffix = "..." if len(disconnected) > 8 else ""
        blockers.add(
            f"{display_name}: disconnected executable instruction(s): {shown}{suffix}"
        )
    if not exits and not tails:
        blockers.add(f"{display_name}: no statically reachable routine exit")

    ordered = sorted(listing.instructions, key=lambda item: item.offset)
    stack_consuming_calls: set[int] = set()
    for previous, item in zip(ordered, ordered[1:]):
        instruction = CORE.decode_instruction(item)
        if x86_const.X86_GRP_CALL not in set(instruction.groups):
            continue
        if previous.offset + len(previous.data) != item.offset:
            continue
        if re.fullmatch(r"push\s+cs", previous.text.strip(), re.IGNORECASE):
            stack_consuming_calls.add(item.offset)

    return Routine(
        key,
        display_name,
        listing,
        tuple(sorted(edges.items())),
        tuple(sorted(calls.items())),
        tuple(sorted(tails.items())),
        tuple(sorted(exits)),
        tuple(sorted(effects.items())),
        frozenset(opaque_effects),
        frozenset(stack_consuming_calls),
        frozenset(blockers),
        public_effect,
    )


def unknown_bytes(width: int) -> tuple[str, ...]:
    return (UNKNOWN,) * width


def stack_address(instruction, operand, state: MachineState) -> int | None:
    if operand.type != x86_const.X86_OP_MEM or operand.mem.index:
        return None
    if operand.mem.segment:
        segment = instruction.reg_name(operand.mem.segment).lower()
        if segment != "ss":
            return None
    base = instruction.reg_name(operand.mem.base).lower() if operand.mem.base else ""
    if base in ("bp", "ebp"):
        base_delta = state.bp_delta
    elif base in ("sp", "esp"):
        base_delta = state.sp_delta
    else:
        return None
    return None if base_delta is None else base_delta + operand.mem.disp


def read_operand(instruction, operand, state: MachineState) -> tuple[str, ...]:
    width = operand.size or 2
    if operand.type == x86_const.X86_OP_REG:
        name = instruction.reg_name(operand.reg).lower()
        if name in REGISTER_SLICES:
            return state.register(name)
        return unknown_bytes(width)
    if operand.type == x86_const.X86_OP_MEM:
        address = stack_address(instruction, operand, state)
        if address is None:
            return unknown_bytes(width)
        stack = dict(state.stack)
        return tuple(stack.get(address + index, UNKNOWN) for index in range(width))
    return unknown_bytes(width)


def write_operand(
    instruction, operand, value: tuple[str, ...], state: MachineState
) -> MachineState:
    width = operand.size or len(value) or 2
    if operand.type == x86_const.X86_OP_REG:
        name = instruction.reg_name(operand.reg).lower()
        if name in ("sp", "esp"):
            return replace(state, sp_delta=None)
        if name in REGISTER_SLICES:
            result = state.with_register(name, value[:width])
            if name in ("bp", "ebp"):
                result = replace(result, bp_delta=None)
            return result
    if operand.type == x86_const.X86_OP_MEM:
        address = stack_address(instruction, operand, state)
        if address is not None:
            stack = dict(state.stack)
            for index, token in enumerate(value[:width]):
                stack[address + index] = token
            return replace(state, stack=tuple(sorted(stack.items())))
    return state


def push_value(state: MachineState, value: tuple[str, ...]) -> MachineState:
    if state.sp_delta is None:
        return replace(state, stack=())
    delta = state.sp_delta - len(value)
    stack = dict(state.stack)
    for index, token in enumerate(value):
        stack[delta + index] = token
    return replace(state, sp_delta=delta, stack=tuple(sorted(stack.items())))


def pop_value(state: MachineState, width: int) -> tuple[MachineState, tuple[str, ...]]:
    if state.sp_delta is None:
        return replace(state, stack=()), unknown_bytes(width)
    stack = dict(state.stack)
    value = tuple(stack.get(state.sp_delta + index, UNKNOWN) for index in range(width))
    for index in range(width):
        stack.pop(state.sp_delta + index, None)
    return (
        replace(
            state,
            sp_delta=state.sp_delta + width,
            stack=tuple(sorted(stack.items())),
        ),
        value,
    )


def adjust_stack(state: MachineState, amount: int) -> MachineState:
    if state.sp_delta is None:
        return state
    next_delta = state.sp_delta + amount
    stack = {
        address: token
        for address, token in state.stack
        if not (state.sp_delta <= address < next_delta)
    }
    return replace(
        state, sp_delta=next_delta, stack=tuple(sorted(stack.items()))
    )


def apply_callee(
    state: MachineState,
    summary: Summary | None,
    *,
    apply_cleanup: bool = True,
) -> MachineState:
    result = state
    preserved = summary.preserved if summary is not None else frozenset()
    for register in GENERAL_REGISTERS:
        if register[1:].upper() not in preserved:
            result = result.with_parent_unknown(register)
    for register in SEGMENT_REGISTERS:
        if register.upper() not in preserved:
            result = result.with_parent_unknown(register)
    if (
        summary is not None
        and not summary.blockers
        and "SP" not in preserved
    ):
        result = replace(result, sp_delta=None, stack=())
    if summary is None or not summary.flags_preserved:
        result = replace(result, flags=unknown_bytes(2))
    if apply_cleanup and summary is not None and summary.cleanup is not None:
        result = adjust_stack(result, summary.cleanup)
    return result


def effect_summary(effect: EffectContract) -> Summary:
    return Summary(
        frozenset(set(CONTRACT_REGISTERS) - set(effect.modifies)),
        effect.flags_preserved,
        effect.cleanup,
        (),
        frozenset(),
    )


def merge_states(left: MachineState, right: MachineState) -> tuple[MachineState, bool]:
    left_registers = dict(left.registers)
    right_registers = dict(right.registers)
    registers = {
        name: tuple(
            a if a == b else UNKNOWN
            for a, b in zip(left_registers[name], right_registers[name])
        )
        for name in left_registers
    }
    flags = tuple(a if a == b else UNKNOWN for a, b in zip(left.flags, right.flags))
    stack_conflict = left.sp_delta != right.sp_delta
    sp_delta = left.sp_delta if not stack_conflict else None
    bp_delta = left.bp_delta if left.bp_delta == right.bp_delta else None
    left_stack = dict(left.stack)
    right_stack = dict(right.stack)
    stack = {
        address: token
        for address, token in left_stack.items()
        if right_stack.get(address) == token
    }
    return (
        MachineState(
            tuple(sorted(registers.items())),
            flags,
            sp_delta,
            bp_delta,
            tuple(sorted(stack.items())),
        ),
        stack_conflict,
    )


def instruction_writes_flags(instruction) -> bool:
    return bool(instruction.eflags & WRITE_FLAGS_MASK)


def transfer_instruction(
    item,
    state: MachineState,
    callee: Summary | None = None,
    *,
    effect: EffectContract | None = None,
    opaque_effect: bool = False,
) -> MachineState:
    instruction = CORE.decode_instruction(item)
    mnemonic = instruction.mnemonic.lower()
    operands = instruction.operands
    result = state

    if effect is not None:
        result = apply_callee(result, effect_summary(effect))
    elif opaque_effect:
        result = apply_callee(result, None)
    elif x86_const.X86_GRP_CALL in set(instruction.groups):
        result = apply_callee(result, callee)
    elif mnemonic in ("pushf", "pushfd"):
        width = 4 if mnemonic == "pushfd" else 2
        value = result.flags + unknown_bytes(max(0, width - 2))
        result = push_value(result, value)
    elif mnemonic in ("popf", "popfd"):
        width = 4 if mnemonic == "popfd" else 2
        result, value = pop_value(result, width)
        result = replace(result, flags=value[:2])
    elif mnemonic in ("push",):
        width = operands[0].size or 2
        result = push_value(result, read_operand(instruction, operands[0], result)[:width])
    elif mnemonic in ("pop",):
        width = operands[0].size or 2
        result, value = pop_value(result, width)
        result = write_operand(instruction, operands[0], value, result)
    elif mnemonic in ("pusha", "pushaw", "pushad"):
        width = 4 if mnemonic == "pushad" else 2
        names = (
            ("eax", "ecx", "edx", "ebx", None, "ebp", "esi", "edi")
            if width == 4 else
            ("ax", "cx", "dx", "bx", None, "bp", "si", "di")
        )
        original_sp = unknown_bytes(width)
        for name in names:
            value = original_sp if name is None else result.register(name)
            result = push_value(result, value)
    elif mnemonic in ("popa", "popaw", "popad"):
        width = 4 if mnemonic == "popad" else 2
        names = (
            ("edi", "esi", "ebp", None, "ebx", "edx", "ecx", "eax")
            if width == 4 else
            ("di", "si", "bp", None, "bx", "dx", "cx", "ax")
        )
        for name in names:
            result, value = pop_value(result, width)
            if name is not None:
                parent, start, _ = REGISTER_SLICES[name]
                result = result.with_register(name, value)
                if parent == "ebp" and start == 0:
                    result = replace(result, bp_delta=None)
    elif mnemonic == "leave":
        result = replace(result, sp_delta=result.bp_delta)
        result, value = pop_value(result, 2)
        result = result.with_register("bp", value)
        result = replace(result, bp_delta=None)
    elif mnemonic == "enter":
        nesting = operands[1].imm if len(operands) > 1 else 0
        if nesting != 0:
            result = replace(result, sp_delta=None, bp_delta=None, stack=())
        else:
            result = push_value(result, result.register("bp"))
            result = result.with_register("bp", unknown_bytes(2))
            result = replace(result, bp_delta=result.sp_delta)
            allocation = operands[0].imm if operands else 0
            if result.sp_delta is not None:
                result = replace(result, sp_delta=result.sp_delta - allocation)
    elif mnemonic == "mov" and len(operands) == 2:
        destination, source = operands
        destination_name = (
            instruction.reg_name(destination.reg).lower()
            if destination.type == x86_const.X86_OP_REG else ""
        )
        source_name = (
            instruction.reg_name(source.reg).lower()
            if source.type == x86_const.X86_OP_REG else ""
        )
        if destination_name in ("bp", "ebp") and source_name in ("sp", "esp"):
            result = result.with_register(destination_name, unknown_bytes(destination.size))
            result = replace(result, bp_delta=result.sp_delta)
        elif destination_name in ("sp", "esp") and source_name in ("bp", "ebp"):
            result = replace(result, sp_delta=result.bp_delta)
        else:
            result = write_operand(
                instruction, destination, read_operand(instruction, source, result), result
            )
    elif mnemonic == "xchg" and len(operands) == 2:
        left = read_operand(instruction, operands[0], result)
        right = read_operand(instruction, operands[1], result)
        result = write_operand(instruction, operands[0], right, result)
        result = write_operand(instruction, operands[1], left, result)
    elif mnemonic in ("and", "or") and len(operands) == 2:
        left, right = operands
        same_register = (
            left.type == x86_const.X86_OP_REG
            and right.type == x86_const.X86_OP_REG
            and left.reg == right.reg
        )
        if not same_register:
            result = write_operand(
                instruction, left, unknown_bytes(left.size or 2), result
            )
    elif mnemonic in ("add", "sub") and len(operands) == 2:
        destination, source = operands
        destination_name = (
            instruction.reg_name(destination.reg).lower()
            if destination.type == x86_const.X86_OP_REG else ""
        )
        if destination_name in ("sp", "esp") and source.type == x86_const.X86_OP_IMM:
            if result.sp_delta is not None:
                amount = source.imm
                result = replace(
                    result,
                    sp_delta=(
                        result.sp_delta + amount
                        if mnemonic == "add" else result.sp_delta - amount
                    ),
                )
        elif source.type != x86_const.X86_OP_IMM or source.imm != 0:
            result = write_operand(
                instruction, destination, unknown_bytes(destination.size or 2), result
            )
    elif mnemonic == "lea" and operands:
        destination = operands[0]
        destination_name = (
            instruction.reg_name(destination.reg).lower()
            if destination.type == x86_const.X86_OP_REG else ""
        )
        result = write_operand(
            instruction, destination, unknown_bytes(destination.size or 2), result
        )
        if destination_name in ("bp", "ebp"):
            result = replace(result, bp_delta=None)
        if destination_name in ("sp", "esp"):
            result = replace(result, sp_delta=None)
    elif not (
        x86_const.X86_GRP_RET in set(instruction.groups)
        or mnemonic in ("iret", "iretd", "nop")
    ):
        try:
            _read, written = instruction.regs_access()
        except Exception as exc:
            raise ValueError(
                f"cannot derive register writes at 0x{item.offset:x}: {item.text}"
            ) from exc
        for register_id in written:
            name = instruction.reg_name(register_id).lower()
            if name in ("sp", "esp"):
                result = replace(result, sp_delta=None, stack=())
            elif name in REGISTER_SLICES:
                _parent, _start, width = REGISTER_SLICES[name]
                result = result.with_register(name, unknown_bytes(width))

    if instruction_writes_flags(instruction) and mnemonic not in ("popf", "popfd"):
        result = replace(result, flags=unknown_bytes(2))
    return result


def preserved_registers(state: MachineState) -> frozenset[str]:
    result = {
        name[1:].upper()
        for name in GENERAL_REGISTERS
        if dict(state.registers)[name][:2] == INITIAL_REGISTERS[name][:2]
    }
    result.update(
        name.upper()
        for name in SEGMENT_REGISTERS
        if dict(state.registers)[name] == INITIAL_REGISTERS[name]
    )
    if state.sp_delta == 0:
        result.add("SP")
    return frozenset(result)


def merged_target_summary(
    targets: Iterable[str], summaries: dict[str, Summary]
) -> Summary | None:
    target_summaries = [summaries.get(target) for target in targets]
    if not target_summaries or any(summary is None for summary in target_summaries):
        return None
    concrete = [summary for summary in target_summaries if summary is not None]
    preserved = set(CONTRACT_REGISTERS)
    for summary in concrete:
        preserved.intersection_update(summary.preserved)
    return Summary(
        frozenset(preserved),
        all(summary.flags_preserved for summary in concrete),
        (
            concrete[0].cleanup
            if all(summary.cleanup == concrete[0].cleanup for summary in concrete)
            else None
        ),
        (),
        frozenset().union(*(summary.blockers for summary in concrete)),
    )


def return_cleanup(instruction) -> int:
    if x86_const.X86_GRP_RET not in set(instruction.groups):
        return 0
    if not instruction.operands:
        return 0
    operand = instruction.operands[0]
    return operand.imm if operand.type == x86_const.X86_OP_IMM else 0


def analyze_routine(routine: Routine, summaries: dict[str, Summary]) -> Summary:
    listing = routine.listing
    by_offset = {item.offset: item for item in listing.instructions}
    if routine.public_effect is not None:
        effect = effect_summary(routine.public_effect)
        offset = (listing.entrypoints or (min(by_offset),))[0]
        contract = ExitContract(
            offset,
            effect.preserved,
            effect.flags_preserved,
            routine.public_effect.cleanup,
        )
        return replace(
            effect,
            exits=(contract,),
            blockers=routine.direct_blockers,
        )
    edges = dict(routine.edges)
    calls = dict(routine.calls)
    tails = dict(routine.tails)
    effects = dict(routine.effects)
    entry = (listing.entrypoints or (min(by_offset),))[0]
    entry_state = INITIAL_STATE
    if any(
        CORE.decode_instruction(by_offset[offset]).mnemonic in ("iret", "iretd")
        for offset in routine.exits
    ):
        entry_state = replace(entry_state, stack=(
            (4, INITIAL_FLAGS[0]), (5, INITIAL_FLAGS[1]),
        ))
    states = {entry: entry_state}
    pending = deque((entry,))
    blockers = set(routine.direct_blockers)
    visits = 0

    while pending:
        offset = pending.popleft()
        visits += 1
        if visits > len(by_offset) * 200:
            blockers.add(f"{routine.display_name}: register dataflow did not converge")
            break
        callee = None
        if offset in calls and calls[offset]:
            callee = merged_target_summary(calls[offset], summaries)
            if callee is not None and callee.cleanup is None:
                blockers.add(
                    f"{routine.display_name}@0x{offset:x}: call targets have "
                    "incompatible stack cleanup"
                )
        outgoing = transfer_instruction(
            by_offset[offset],
            states[offset],
            callee,
            effect=effects.get(offset),
            opaque_effect=offset in routine.opaque_effects,
        )
        if offset in routine.stack_consuming_calls:
            outgoing, _discarded_segment = pop_value(outgoing, 2)
        for target in edges[offset]:
            previous = states.get(target)
            if previous is None:
                states[target] = outgoing
                pending.append(target)
                continue
            merged, stack_conflict = merge_states(previous, outgoing)
            if stack_conflict:
                blockers.add(
                    f"{routine.display_name}@0x{target:x}: incompatible stack "
                    "deltas reach CFG join"
                )
            if merged != previous:
                states[target] = merged
                pending.append(target)

    exit_contracts: list[ExitContract] = []
    for offset in routine.exits:
        if offset not in states:
            continue
        state = states[offset]
        instruction = CORE.decode_instruction(by_offset[offset])
        flags = state.flags
        if instruction.mnemonic in ("iret", "iretd"):
            width = 4 if instruction.mnemonic == "iretd" else 2
            stack = dict(state.stack)
            flags_offset = 8 if width == 4 else 4
            flags = tuple(
                stack.get(
                    (state.sp_delta if state.sp_delta is not None else 0)
                    + flags_offset + index,
                    UNKNOWN,
                )
                for index in range(2)
            )
        preserved = preserved_registers(state)
        if "SP" not in preserved:
            blockers.add(
                f"{routine.display_name}@0x{offset:x}: return stack delta is "
                f"{state.sp_delta if state.sp_delta is not None else 'unknown'}"
            )
        exit_contracts.append(ExitContract(
            offset,
            preserved,
            flags == INITIAL_FLAGS,
            return_cleanup(instruction),
        ))

    for offset, targets in tails.items():
        if offset not in states:
            continue
        target_summary = merged_target_summary(targets, summaries)
        if target_summary is None:
            blockers.add(
                f"{routine.display_name}@0x{offset:x}: tail target contract unavailable"
            )
            state = apply_callee(states[offset], None, apply_cleanup=False)
            cleanup = 0
        else:
            state = apply_callee(
                states[offset], target_summary, apply_cleanup=False
            )
            cleanup = target_summary.cleanup
            if cleanup is None:
                blockers.add(
                    f"{routine.display_name}@0x{offset:x}: tail targets have "
                    "incompatible stack cleanup"
                )
                cleanup = 0
        exit_contracts.append(ExitContract(
            offset,
            preserved_registers(state),
            state.flags == INITIAL_FLAGS,
            cleanup,
        ))

    for targets in calls.values():
        for target in targets:
            summary = summaries.get(target)
            if summary is None:
                blockers.add(
                    f"{routine.display_name}: direct callee {target} has no emitted body"
                )
            else:
                if summary.blockers:
                    blockers.add(
                        f"{routine.display_name}: callee {target} contract unresolved"
                    )
    for targets in tails.values():
        for target in targets:
            summary = summaries.get(target)
            if summary is None:
                blockers.add(
                    f"{routine.display_name}: tail callee {target} has no emitted body"
                )
            else:
                if summary.blockers:
                    blockers.add(
                        f"{routine.display_name}: tail callee {target} contract unresolved"
                    )

    if not exit_contracts:
        blockers.add(f"{routine.display_name}: no reachable return or resolved tail exit")
        preserved = frozenset()
        flags_preserved = False
        cleanup = None
    else:
        preserved_set = set(CONTRACT_REGISTERS)
        for contract in exit_contracts:
            preserved_set.intersection_update(contract.preserved)
        preserved = frozenset(preserved_set)
        flags_preserved = all(contract.flags_preserved for contract in exit_contracts)
        cleanups = {contract.cleanup for contract in exit_contracts}
        cleanup = next(iter(cleanups)) if len(cleanups) == 1 else None
        if cleanup is None:
            blockers.add(
                f"{routine.display_name}: exits have incompatible stack cleanup "
                f"{sorted(cleanups)}"
            )
    return Summary(
        preserved,
        flags_preserved,
        cleanup,
        tuple(sorted(exit_contracts, key=lambda contract: contract.offset)),
        frozenset(blockers),
    )


def recursive_routines(routines: dict[str, Routine]) -> frozenset[str]:
    graph = {
        key: {
            target
            for _offset, targets in routine.calls + routine.tails
            for target in targets
            if target in routines
        }
        for key, routine in routines.items()
    }
    index = 0
    indices: dict[str, int] = {}
    lowlinks: dict[str, int] = {}
    stack: list[str] = []
    on_stack: set[str] = set()
    recursive: set[str] = set()

    def visit(node: str) -> None:
        nonlocal index
        indices[node] = index
        lowlinks[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)
        for target in graph[node]:
            if target not in indices:
                visit(target)
                lowlinks[node] = min(lowlinks[node], lowlinks[target])
            elif target in on_stack:
                lowlinks[node] = min(lowlinks[node], indices[target])
        if lowlinks[node] != indices[node]:
            return
        component: list[str] = []
        while True:
            member = stack.pop()
            on_stack.remove(member)
            component.append(member)
            if member == node:
                break
        if len(component) > 1 or node in graph[node]:
            recursive.update(component)

    for node in graph:
        if node not in indices:
            visit(node)
    return frozenset(recursive)


def summarize_program(routines: dict[str, Routine]) -> dict[str, Summary]:
    recursive = recursive_routines(routines)
    summaries = {
        key: Summary(
            (
                frozenset(CONTRACT_REGISTERS)
                if key in recursive else frozenset()
            ),
            key in recursive,
            0 if key in recursive else None,
            (),
            routine.direct_blockers,
        )
        for key, routine in routines.items()
    }
    callers: dict[str, set[str]] = {key: set() for key in routines}
    for caller, routine in routines.items():
        for _offset, targets in routine.calls + routine.tails:
            for target in targets:
                if target in callers:
                    callers[target].add(caller)
    pending = deque(sorted(routines))
    queued = set(pending)
    updates = 0
    limit = max(1, len(routines) * (len(CONTRACT_REGISTERS) + 4) * 8)
    while pending:
        key = pending.popleft()
        queued.remove(key)
        summary = analyze_routine(routines[key], summaries)
        if summary == summaries[key]:
            continue
        summaries[key] = summary
        updates += 1
        if updates > limit:
            raise ValueError("interprocedural register contracts did not converge")
        for caller in sorted(callers[key]):
            if caller not in queued:
                pending.append(caller)
                queued.add(caller)
    return summaries


def compare_programs(
    original: dict[str, Routine], emitted: dict[str, Routine]
) -> list[Comparison]:
    if not original.keys() <= emitted.keys():
        missing = sorted(original.keys() - emitted.keys())
        raise ValueError(f"emitted program is missing routines: {missing}")
    original_summaries = summarize_program(original)
    emitted_summaries = summarize_program(emitted)
    rows: list[Comparison] = []
    for key in sorted(original):
        before = original_summaries[key]
        after = emitted_summaries[key]
        required = ",".join(sorted(before.preserved))
        if before.blockers or after.blockers:
            if before.blockers and after.blockers:
                status = "unresolved_both"
            elif before.blockers:
                status = "unresolved_original"
            else:
                status = "unresolved_emitted"
            blocker_parts = []
            if before.blockers:
                blocker_parts.append(
                    "original: " + "; ".join(sorted(before.blockers))
                )
            if after.blockers:
                blocker_parts.append(
                    "emitted: " + "; ".join(sorted(after.blockers))
                )
            contracts = after.exits or (None,)
            for contract in contracts:
                rows.append(Comparison(
                    key,
                    status,
                    "" if contract is None else f"0x{contract.offset:x}",
                    required,
                    "",
                    "preserved" if before.flags_preserved else "clobbered",
                    "" if contract is None else (
                        "preserved" if contract.flags_preserved else "clobbered"
                    ),
                    "" if before.cleanup is None else str(before.cleanup),
                    "" if contract is None else str(contract.cleanup),
                    " | ".join(blocker_parts),
                ))
            continue
        if not after.exits:
            rows.append(Comparison(
                key, "missing_emitted_exit", "", required, "",
                "preserved" if before.flags_preserved else "clobbered",
                "", "" if before.cleanup is None else str(before.cleanup), "",
                "emitted routine has no reachable exit",
            ))
            continue
        for contract in after.exits:
            clobbers = sorted(before.preserved - contract.preserved)
            flags_mismatch = before.flags_preserved and not contract.flags_preserved
            cleanup_mismatch = before.cleanup != contract.cleanup
            if cleanup_mismatch:
                status = "stack_cleanup_mismatch"
            elif clobbers and flags_mismatch:
                status = "register_and_flags_mismatch"
            elif clobbers:
                status = "register_mismatch"
            elif flags_mismatch:
                status = "flags_mismatch"
            else:
                status = "pass"
            rows.append(Comparison(
                key,
                status,
                f"0x{contract.offset:x}",
                required,
                ",".join(clobbers),
                "preserved" if before.flags_preserved else "clobbered",
                "preserved" if contract.flags_preserved else "clobbered",
                "" if before.cleanup is None else str(before.cleanup),
                str(contract.cleanup),
                "",
            ))
    return rows


def original_resolver(
    entries: dict[int, str], header_size: int,
    indirect_targets: dict[int, tuple[str, ...]],
) -> TargetResolver:
    def resolve(item, _kind: str) -> tuple[str, ...] | None:
        instruction = CORE.decode_instruction(item)
        target = far_target(item.text, header_size)
        if target is None:
            target = immediate_target(instruction)
        if target is not None and target in entries:
            return (entries[target],)
        return indirect_targets.get(item.offset)
    return resolve


VM_DISPATCH_CALL_SITES = (0x5627, 0x56C4)


def emitted_resolver(
    functions: dict[str, str],
    indirect_targets: dict[int, tuple[str, ...]] | None = None,
) -> TargetResolver:
    def resolve(item, _kind: str) -> tuple[str, ...] | None:
        symbol = emitted_symbol(item)
        if symbol is None:
            if (
                indirect_targets is not None
                and "_vm_opcode_handlers" in item.text.lower()
            ):
                targets = {
                    target
                    for call_site in VM_DISPATCH_CALL_SITES
                    for target in indirect_targets.get(call_site, ())
                }
                return tuple(sorted(targets)) or None
            return None
        for variant in symbol_variants(symbol):
            if variant in functions:
                return (functions[variant],)
        return None
    return resolve


@dataclass(frozen=True)
class ExecutableImage:
    path: Path
    image: bytes
    header_size: int

    @classmethod
    def read(cls, path: Path) -> "ExecutableImage":
        data = path.read_bytes()
        if len(data) < 10 or data[:2] != b"MZ":
            raise ValueError(f"{path}: not an MZ executable")
        return cls(path, data, int.from_bytes(data[8:10], "little") * 16)

    def decode(self, linear: int):
        file_offset = self.header_size + linear
        if file_offset < self.header_size or file_offset >= len(self.image):
            raise ValueError(
                f"{self.path}: linked code address 0x{linear:x} is outside image"
            )
        decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
        decoder.detail = True
        instruction = next(
            decoder.disasm(self.image[file_offset:file_offset + 15], linear, count=1),
            None,
        )
        if instruction is None:
            raise ValueError(
                f"{self.path}: cannot decode linked instruction at 0x{linear:x}"
            )
        text = instruction.mnemonic
        if instruction.op_str:
            text += " " + instruction.op_str
        item = CORE.ListingInstruction(
            linear,
            self.image[file_offset:file_offset + instruction.size],
            text,
        )
        CORE.decode_instruction(item)
        return item


def direct_binary_target(item, segment_base: int) -> int | None:
    target = far_target(item.text, 0)
    if target is not None:
        return target
    target = immediate_target(CORE.decode_instruction(item))
    if target is None:
        return None
    return segment_base + ((target - segment_base) & 0xFFFF)


def map_symbol_addresses(path: Path) -> dict[str, tuple[int, ...]]:
    parsed = MAPS.read_map_symbols(path)
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = MARKED_MAP_SYMBOL.fullmatch(line)
        if match is None:
            continue
        location = (int(match["segment"], 16), int(match["offset"], 16))
        parsed.setdefault(match["symbol"], [])
        if location not in parsed[match["symbol"]]:
            parsed[match["symbol"]].append(location)
    result: dict[str, tuple[int, ...]] = {}
    for symbol, locations in parsed.items():
        addresses = tuple(sorted({segment * 16 + offset for segment, offset in locations}))
        for variant in symbol_variants(symbol):
            previous = result.get(variant)
            if previous is not None and previous != addresses:
                raise ValueError(
                    f"{path}: ambiguous map symbol variant {variant}: "
                    f"{previous} and {addresses}"
                )
            result[variant] = addresses
    return result


def unique_map_address(
    symbols: dict[str, tuple[int, ...]], symbol: str, map_path: Path
) -> int:
    candidates: set[int] = set()
    for variant in symbol_variants(symbol):
        candidates.update(symbols.get(variant, ()))
    if len(candidates) != 1:
        raise ValueError(
            f"{map_path}: direct symbol {symbol} has {len(candidates)} linked "
            "addresses; expected exactly one"
        )
    return next(iter(candidates))


def code_contains(segments, linear: int) -> bool:
    return any(
        segment.segment * 16 + segment.offset <= linear
        < segment.segment * 16 + segment.offset + segment.size
        for segment in segments
    )


def code_segment_base(segments, linear: int) -> int:
    matches = [
        segment.segment * 16
        for segment in segments
        if segment.segment * 16 + segment.offset <= linear
        < segment.segment * 16 + segment.offset + segment.size
    ]
    if not matches:
        raise ValueError(f"linked address 0x{linear:x} has no CODE segment")
    if len(set(matches)) != 1:
        raise ValueError(
            f"linked address 0x{linear:x} has ambiguous CODE segment bases "
            f"{sorted(set(matches))}"
        )
    return matches[0]


def discover_linked_listing(
    key: str,
    entry: int,
    executable: ExecutableImage,
    code_segments,
    address_keys: dict[int, str],
    allocate_call_target: Callable[[int], str | None],
) -> object:
    instructions: dict[int, object] = {}
    pending = deque((entry,))
    queued = {entry}
    while pending:
        offset = pending.popleft()
        queued.discard(offset)
        if offset in instructions:
            continue
        if not code_contains(code_segments, offset):
            raise ValueError(
                f"{key}: control reaches non-code address 0x{offset:x}"
            )
        if len(instructions) >= 8192:
            raise ValueError(f"{key}: linked routine exceeds 8192 instructions")
        item = executable.decode(offset)
        instructions[offset] = item
        instruction = CORE.decode_instruction(item)
        groups = set(instruction.groups)
        next_offset = offset + len(item.data)

        def queue_local(target: int) -> None:
            target_key = address_keys.get(target)
            if target_key is not None and target_key != key:
                return
            if target not in instructions and target not in queued:
                pending.append(target)
                queued.add(target)

        if x86_const.X86_GRP_CALL in groups:
            target = direct_binary_target(
                item, code_segment_base(code_segments, item.offset)
            )
            if target is not None:
                allocate_call_target(target)
            queue_local(next_offset)
        elif x86_const.X86_GRP_JUMP in groups:
            target = direct_binary_target(
                item, code_segment_base(code_segments, item.offset)
            )
            if target is not None:
                queue_local(target)
            if instruction_is_conditional_jump(instruction):
                queue_local(next_offset)
        elif x86_const.X86_GRP_RET in groups or instruction.mnemonic in (
            "iret", "iretd",
        ):
            continue
        else:
            queue_local(next_offset)
    return CORE.Listing(
        executable.path,
        tuple(instructions[offset] for offset in sorted(instructions)),
        {key: entry},
        {},
        (entry,),
        ((min(instructions), max(
            offset + len(item.data) for offset, item in instructions.items()
        )),),
    )


def discover_linked_dependencies(
    root_symbols: Iterable[str],
    recovered_functions: dict[str, str],
    recovered_addresses: dict[int, str],
    link_map: Path,
    emitted_image: Path,
) -> tuple[dict[str, Routine], dict[str, str]]:
    symbols = map_symbol_addresses(link_map)
    segments = MAPS.read_map_segments(link_map)
    executable = ExecutableImage.read(emitted_image)
    address_keys = dict(recovered_addresses)
    symbol_keys = dict(recovered_functions)
    entries: dict[str, int] = {}
    key_symbols: dict[str, set[str]] = {}
    pending: deque[str] = deque()

    def allocate(address: int) -> str | None:
        known = address_keys.get(address)
        if known is not None:
            return known
        if not code_contains(segments, address):
            return None
        key = f"linked_{address:05x}"
        address_keys[address] = key
        entries[key] = address
        pending.append(key)
        return key

    for symbol in sorted(set(root_symbols)):
        address = unique_map_address(symbols, symbol, link_map)
        key = allocate(address)
        if key is None:
            raise ValueError(
                f"{link_map}: direct symbol {symbol} resolves outside CODE at "
                f"0x{address:x}"
            )
        key_symbols.setdefault(key, set()).add(symbol)
        for variant in symbol_variants(symbol):
            symbol_keys[variant] = key

    listings: dict[str, object] = {}
    while pending:
        key = pending.popleft()
        if key in listings:
            continue
        listings[key] = discover_linked_listing(
            key,
            entries[key],
            executable,
            segments,
            address_keys,
            allocate,
        )

    def binary_resolver(item, _kind: str) -> tuple[str, ...] | None:
        target = direct_binary_target(
            item, code_segment_base(segments, item.offset)
        )
        if target is None:
            return None
        key = address_keys.get(target)
        return (key,) if key is not None else None

    routines = {}
    for key, listing in listings.items():
        names = key_symbols.get(key, set())
        routines[key] = build_routine(
            key,
            key + ":emitted-linked",
            listing,
            binary_resolver,
            effect_resolver=linked_effect_resolver(names),
            public_effect=public_linked_effect(names),
        )
    return routines, symbol_keys


def read_manifest(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="ascii") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    required = {"entry", "source", "asm_path", "function"}
    if not rows or not required.issubset(rows[0]):
        raise ValueError(f"{path}: incomplete BLOODPRG manifest")
    stems = [Path(row["source"]).stem.lower() for row in rows]
    if len(stems) != len(set(stems)):
        raise ValueError(f"{path}: duplicate recovered routine stem")
    return rows


def mz_header_size(path: Path) -> int:
    data = path.read_bytes()[:10]
    if len(data) < 10 or data[:2] != b"MZ":
        raise ValueError(f"{path}: not an MZ executable")
    return int.from_bytes(data[8:10], "little") * 16


def load_programs(
    manifest_path: Path,
    original_image: Path,
    emitted_image: Path,
    link_map: Path,
    listing_dir: Path,
) -> tuple[dict[str, Routine], dict[str, Routine], list[str]]:
    rows = read_manifest(manifest_path)
    stems = {Path(row["source"]).stem.lower() for row in rows}
    linked = CORE.linked_project_stems(link_map)
    errors: list[str] = []
    if linked != stems:
        errors.append(
            "manifest/link-map routine sets differ: "
            f"missing={sorted(stems - linked)}, extra={sorted(linked - stems)}"
        )

    entries = {
        int(row["entry"], 0): Path(row["source"]).stem.lower()
        for row in rows
    }
    functions: dict[str, str] = {}
    for row in rows:
        stem = Path(row["source"]).stem.lower()
        for variant in symbol_variants(row["function"]):
            previous = functions.setdefault(variant, stem)
            if previous != stem:
                raise ValueError(
                    f"ambiguous emitted symbol {variant}: {previous} and {stem}"
                )

    emitted_listings: dict[str, object] = {}
    for row in rows:
        stem = Path(row["source"]).stem.lower()
        listing_path = listing_dir / f"{stem}.lst"
        if not listing_path.is_file():
            errors.append(f"{stem}: missing emitted listing {listing_path}")
            continue
        emitted_listings[stem] = CORE.parse_listing(
            listing_path,
            listing_path.read_text(encoding="ascii", errors="replace"),
        )

    linked_symbols = map_symbol_addresses(link_map)
    recovered_addresses: dict[int, str] = {}
    for row in rows:
        stem = Path(row["source"]).stem.lower()
        address = unique_map_address(linked_symbols, row["function"], link_map)
        previous = recovered_addresses.setdefault(address, stem)
        if previous != stem:
            raise ValueError(
                f"{link_map}: recovered routines {previous} and {stem} share "
                f"entry 0x{address:x}"
            )

    external_symbols: set[str] = set()
    for listing in emitted_listings.values():
        local_labels = {label.lower() for label in listing.labels}
        for item in listing.instructions:
            instruction = CORE.decode_instruction(item)
            if not (
                x86_const.X86_GRP_CALL in set(instruction.groups)
                or x86_const.X86_GRP_JUMP in set(instruction.groups)
            ):
                continue
            symbol = emitted_symbol(item)
            if symbol is None:
                continue
            if symbol in local_labels:
                continue
            if not any(variant in functions for variant in symbol_variants(symbol)):
                external_symbols.add(symbol)

    linked_routines, emitted_functions = discover_linked_dependencies(
        external_symbols,
        functions,
        recovered_addresses,
        link_map,
        emitted_image,
    )

    dispatch = ROLES.static_dispatch_targets(entries)
    original_target_resolver = original_resolver(
        entries, mz_header_size(original_image), dispatch
    )
    emitted_target_resolver = emitted_resolver(emitted_functions, dispatch)
    original: dict[str, Routine] = {}
    emitted: dict[str, Routine] = {}
    for row in rows:
        stem = Path(row["source"]).stem.lower()
        asm_path = ROOT / row["asm_path"]
        if not asm_path.is_file():
            errors.append(f"{stem}: missing original assembly {asm_path}")
            continue
        original_listing = ROLES.parse_original(asm_path)
        original_listing = replace(
            original_listing,
            entrypoints=(int(row["entry"], 0),),
            executable_ranges=((
                min(item.offset for item in original_listing.instructions),
                max(
                    item.offset + len(item.data)
                    for item in original_listing.instructions
                ),
            ),),
        )
        original[stem] = build_routine(
            stem, stem + ":original", original_listing, original_target_resolver
        )

        emitted_listing = emitted_listings.get(stem)
        if emitted_listing is None:
            continue
        emitted[stem] = build_routine(
            stem,
            stem + ":emitted",
            emitted_listing,
            emitted_target_resolver,
            effect_resolver=recovered_effect_resolver,
        )
    emitted.update(linked_routines)
    missing_programs = sorted(stems - original.keys())
    missing_emitted = sorted(stems - emitted.keys())
    if missing_programs:
        errors.append("unloaded original routines: " + ",".join(missing_programs))
    if missing_emitted:
        errors.append("unloaded emitted routines: " + ",".join(missing_emitted))
    return original, emitted, errors


def render_tsv(rows: list[Comparison]) -> str:
    output = io.StringIO()
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow((
        "routine", "status", "emitted_exit", "original_preserved",
        "emitted_clobbers", "original_flags", "emitted_flags",
        "original_cleanup", "emitted_cleanup", "blockers",
    ))
    for row in rows:
        writer.writerow((
            row.routine, row.status, row.emitted_exit, row.original_preserved,
            row.emitted_clobbers, row.original_flags, row.emitted_flags,
            row.original_cleanup, row.emitted_cleanup, row.blockers,
        ))
    return output.getvalue()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest", type=Path,
        default=ROOT / "re/source/bloodprg/candidates/manifest.tsv",
    )
    parser.add_argument(
        "--original-image", type=Path,
        default=ROOT / "re/bin/BLOODPRG.EXE",
    )
    parser.add_argument(
        "--link-map", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/final/link.map"
        ),
    )
    parser.add_argument(
        "--emitted-image", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/final/"
            "BPRG_RE.EXE"
        ),
    )
    parser.add_argument(
        "--listing-dir", type=Path,
        default=ROOT / (
            "output/recovered_dos_package/validation/bloodprg_runtime/final/"
            "segment_contract_listings"
        ),
    )
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        original, emitted, errors = load_programs(
            args.manifest.resolve(),
            args.original_image.resolve(),
            args.emitted_image.resolve(),
            args.link_map.resolve(),
            args.listing_dir.resolve(),
        )
        rows = compare_programs(original, emitted) if not errors else []
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    report = render_tsv(rows)
    if args.output is None:
        sys.stdout.write(report)
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="ascii")
        print(f"wrote {args.output}")

    failures = [row for row in rows if row.status != "pass"]
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    for row in failures:
        detail = row.blockers or row.emitted_clobbers or row.status
        print(f"ERROR: {row.routine}: {row.status}: {detail}", file=sys.stderr)
    if errors or failures:
        return 1
    print(f"OK: {len(rows)} emitted exits satisfy original register/FLAGS contracts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
