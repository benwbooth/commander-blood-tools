#!/usr/bin/env python3
"""Verify recovered ABI boundaries in the emitted Open Watcom objects."""
from __future__ import annotations

import argparse
import csv
import importlib.util
import re
import sys
from collections import defaultdict, deque
from dataclasses import dataclass
from pathlib import Path

import capstone
from capstone import x86_const


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "segment_contract_audit", ROOT / "re/tools/audit_segment_contracts.py"
)
SEGMENTS = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = SEGMENTS
SPEC.loader.exec_module(SEGMENTS)

MZ_SPEC = importlib.util.spec_from_file_location(
    "relinked_abi_mzfile", ROOT / "re/tools/mzfile.py"
)
MZ_MODULE = importlib.util.module_from_spec(MZ_SPEC)
sys.modules[MZ_SPEC.name] = MZ_MODULE
MZ_SPEC.loader.exec_module(MZ_MODULE)
MZ = MZ_MODULE.MZ


DGROUP_ROW = re.compile(
    r"^DGROUP\s+(?P<segment>[0-9A-Fa-f]{4}):0000\b", re.MULTILINE
)
GAME_DATA_ROW = re.compile(
    r"^GAME_DATA\s+FAR_DATA\s+DGROUP\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):0000\b",
    re.MULTILINE,
)
MANIFEST = ROOT / "re/source/bloodprg/candidates/manifest.tsv"
ASM_SEGMENT_ROW = re.compile(
    r"^; seg_off:\s*(?P<segment>[0-9A-Fa-f]+):"
    r"(?P<offset>[0-9A-Fa-f]+)\s*$",
    re.MULTILINE,
)
ASM_INSTRUCTION_ROW = re.compile(
    r"^(?P<offset>[0-9A-Fa-f]{6}):\s{2}"
    r"(?P<bytes>[0-9A-Fa-f]{2}(?:\s+[0-9A-Fa-f]{2})*)\s{2,}"
    r"(?P<text>\S.*?)\s*$",
    re.MULTILINE,
)
DIRECT_NEAR_CALL = re.compile(
    r"^call\s+(?P<target>(?:0x)?[0-9a-f]+)$", re.IGNORECASE
)
DIRECT_FAR_CALL = re.compile(
    r"^lcall\s+(?P<segment>(?:0x)?[0-9a-f]+),\s*"
    r"(?P<offset>(?:0x)?[0-9a-f]+)$",
    re.IGNORECASE,
)
SYMBOLIC_CALL = re.compile(
    r"^call\s+(?:(?:near|far)\s+ptr\s+)?(?P<function>[A-Za-z_$?][\w$?@]*)_$",
    re.IGNORECASE,
)
RETURN_KINDS = {"ret": "near", "retf": "far", "iret": "interrupt"}
RESULT_REGISTERS = (
    "ax", "bx", "cx", "dx", "si", "di", "bp",
    "ds", "es", "fs", "gs", "flags",
)
REGISTER_PARTS = {
    "al": ("ax", 8), "ah": ("ax", 8), "ax": ("ax", 16),
    "eax": ("ax", 32),
    "bl": ("bx", 8), "bh": ("bx", 8), "bx": ("bx", 16),
    "ebx": ("bx", 32),
    "cl": ("cx", 8), "ch": ("cx", 8), "cx": ("cx", 16),
    "ecx": ("cx", 32),
    "dl": ("dx", 8), "dh": ("dx", 8), "dx": ("dx", 16),
    "edx": ("dx", 32),
    "si": ("si", 16), "esi": ("si", 32),
    "di": ("di", 16), "edi": ("di", 32),
    "bp": ("bp", 16), "ebp": ("bp", 32),
    "ds": ("ds", 16), "es": ("es", 16),
    "fs": ("fs", 16), "gs": ("gs", 16),
    "flags": ("flags", 1), "eflags": ("flags", 1),
}


@dataclass(frozen=True, order=True)
class ReturnSite:
    kind: str
    cleanup: int


@dataclass(frozen=True, order=True)
class ReturnCarrier:
    register: str
    width: int


@dataclass(frozen=True)
class RoutineAbi:
    returns: tuple[ReturnSite, ...]
    carriers: tuple[ReturnCarrier, ...]
    carrier_evidence: str
    hidden_result_width: int = 0


def parse_number(value: str) -> int:
    value = value.strip().lower()
    return int(value, 16 if value.startswith("0x") else 16)


def parse_original_listing(path: Path):
    """Parse one evidence-bearing assembly dump without redisassembling it."""
    text = path.read_text(encoding="ascii", errors="replace")
    instructions = []
    for match in ASM_INSTRUCTION_ROW.finditer(text):
        item = SEGMENTS.ListingInstruction(
            int(match["offset"], 16),
            bytes.fromhex(match["bytes"]),
            " ".join(match["text"].split()),
        )
        # A malformed or stale byte/text row must not become ABI evidence.
        SEGMENTS.decode_instruction(item)
        instructions.append(item)
    if not instructions:
        raise ValueError(f"{path}: no original instructions found")
    return SEGMENTS.Listing(
        path,
        tuple(instructions),
        {},
        {},
        (instructions[0].offset,),
        ((instructions[0].offset,
          instructions[-1].offset + len(instructions[-1].data)),),
    )


def routine_return_sites(instructions) -> tuple[ReturnSite, ...]:
    sites = set()
    for item in instructions:
        parts = normalized_text((item,))[0].split()
        if not parts or parts[0] not in RETURN_KINDS:
            continue
        cleanup = 0
        if len(parts) > 1:
            try:
                cleanup = int(parts[1], 0)
            except ValueError as error:
                raise ValueError(
                    f"cannot derive return cleanup at 0x{item.offset:04x}: "
                    f"{item.text}"
                ) from error
        sites.add(ReturnSite(RETURN_KINDS[parts[0]], cleanup))
    if not sites:
        return (ReturnSite("noreturn", 0),)
    return tuple(sorted(sites))


def register_accesses(item) -> tuple[dict[str, int], dict[str, int]]:
    """Return architectural register reads/writes with effective widths."""
    instruction = SEGMENTS.decode_instruction(item)
    reads: dict[str, int] = {}
    writes: dict[str, int] = {}
    try:
        read_ids, write_ids = instruction.regs_access()
    except capstone.CsError as error:
        raise ValueError(
            f"cannot derive register effects at 0x{item.offset:04x}: {item.text}"
        ) from error

    def add(target: dict[str, int], name: str) -> None:
        part = REGISTER_PARTS.get(name.lower())
        if part is not None and part[0] in RESULT_REGISTERS:
            target[part[0]] = max(target.get(part[0], 0), part[1])

    for register_id in read_ids:
        add(reads, instruction.reg_name(register_id))
    for register_id in write_ids:
        add(writes, instruction.reg_name(register_id))

    # Capstone omits the implicit DS/SS read from ordinary 16-bit memory
    # operands. Segment-valued returns such as DS:SI require that evidence.
    for operand in instruction.operands:
        if operand.type != x86_const.X86_OP_MEM:
            continue
        if operand.mem.segment:
            add(reads, instruction.reg_name(operand.mem.segment))
        else:
            base = (instruction.reg_name(operand.mem.base) or "").lower()
            add(reads, "ss" if base in ("bp", "sp", "ebp", "esp") else "ds")
    return reads, writes


def instruction_successors(listing, item) -> tuple[int, ...]:
    offsets = {candidate.offset for candidate in listing.instructions}
    instruction = SEGMENTS.decode_instruction(item)
    next_offset = item.offset + len(item.data)
    groups = set(instruction.groups)
    if instruction.mnemonic.lower() in ("ret", "retf", "iret"):
        return ()
    immediate = None
    if (instruction.operands and
            instruction.operands[0].type == x86_const.X86_OP_IMM):
        immediate = instruction.operands[0].imm & 0xFFFF_FFFF
    if x86_const.X86_GRP_JUMP in groups:
        result = []
        if immediate in offsets:
            result.append(immediate)
        if instruction.mnemonic.lower() != "jmp" and next_offset in offsets:
            result.append(next_offset)
        return tuple(result)
    return (next_offset,) if next_offset in offsets else ()


def carriers_consumed_after_call(listing, call_index: int) -> tuple[ReturnCarrier, ...]:
    """Find callee values read before being overwritten on every reachable path."""
    instructions = tuple(listing.instructions)
    by_offset = {item.offset: item for item in instructions}
    start = instructions[call_index].offset + len(instructions[call_index].data)
    if start not in by_offset:
        return ()
    pending = deque([(start, frozenset(RESULT_REGISTERS))])
    visited: set[tuple[int, frozenset[str]]] = set()
    consumed: dict[str, int] = {}
    while pending:
        offset, live = pending.popleft()
        state_key = (offset, live)
        if state_key in visited:
            continue
        visited.add(state_key)
        item = by_offset[offset]
        opcode = item.text.lower().split(None, 1)[0]
        if opcode in ("call", "lcall", "ret", "retf", "iret"):
            continue
        reads, writes = register_accesses(item)
        for register in live.intersection(reads):
            consumed[register] = max(consumed.get(register, 0), reads[register])
        remaining = frozenset(register for register in live if register not in writes)
        if not remaining:
            continue
        for successor in instruction_successors(listing, item):
            pending.append((successor, remaining))
    return tuple(sorted(
        ReturnCarrier(register, width) for register, width in consumed.items()
    ))


def locally_modified_carriers(listing) -> tuple[ReturnCarrier, ...]:
    """Conservative fallback for roots/callbacks without a direct caller."""
    writes: dict[str, int] = {}
    preserved: set[str] = set()
    pushed: set[str] = set()
    has_pusha = False
    has_popa = False
    has_frame_save = False
    has_leave = False
    explicit_carry = False
    for item in listing.instructions:
        text = normalized_text((item,))[0]
        opcode = text.split(None, 1)[0]
        operands = SEGMENTS.split_operands(text)
        _reads, item_writes = register_accesses(item)
        for register, width in item_writes.items():
            if register != "flags":
                writes[register] = max(writes.get(register, 0), width)
        if opcode == "push" and operands:
            part = REGISTER_PARTS.get(operands[0])
            if part is not None:
                pushed.add(part[0])
                if part[0] == "bp":
                    has_frame_save = True
        elif opcode == "pop" and operands:
            part = REGISTER_PARTS.get(operands[0])
            if part is not None and part[0] in pushed:
                preserved.add(part[0])
        elif opcode in ("pusha", "pushad"):
            has_pusha = True
        elif opcode in ("popa", "popad"):
            has_popa = True
        elif opcode == "leave":
            has_leave = True
        elif opcode in ("clc", "stc", "cmc"):
            explicit_carry = True
    if has_pusha and has_popa:
        preserved.update(("ax", "bx", "cx", "dx", "si", "di", "bp"))
    if has_frame_save and has_leave:
        preserved.add("bp")
    result = {
        register: width
        for register, width in writes.items()
        if register in RESULT_REGISTERS and register not in preserved
    }
    if explicit_carry:
        result["flags"] = 1
    return tuple(sorted(
        ReturnCarrier(register, width) for register, width in result.items()
    ))


def copied_result_width(listing) -> int:
    """Derive the largest compiler-emitted structure copy at a return path."""
    instructions = tuple(listing.instructions)
    largest = 0
    run = 0
    for index, item in enumerate(instructions):
        text = normalized_text((item,))[0]
        if text == "movsw":
            run += 2
            largest = max(largest, run)
            continue
        run = 0
        if text == "rep movsw":
            for previous in reversed(instructions[max(0, index - 8):index]):
                match = re.match(
                    r"^mov\s+cx,(?P<count>(?:0x)?[0-9a-f]+)$",
                    normalized_text((previous,))[0],
                )
                if match is not None:
                    largest = max(largest, parse_number(match["count"]) * 2)
                    break
    return largest


def caller_hidden_result_offset(listing, call_index: int) -> int | None:
    """Find a DS:SI stack-local buffer which the caller reads after return."""
    instructions = tuple(listing.instructions)
    lower = max(0, call_index - 12)
    local_offset = None
    for index in range(call_index - 1, lower - 1, -1):
        text = normalized_text((instructions[index],))[0]
        match = re.match(
            r"^lea\s+si,(?P<offset>-?(?:0x)?[0-9a-f]+)\[bp\]$", text
        )
        if match is not None:
            local_offset = int(match["offset"], 0)
            break
        _reads, writes = register_accesses(instructions[index])
        if "si" in writes:
            return None
    if local_offset is None:
        return None
    for item in instructions[call_index + 1:call_index + 25]:
        text = normalized_text((item,))[0]
        if text.split(None, 1)[0] in ("call", "lcall", "ret", "retf", "iret"):
            break
        for match in re.finditer(
                r"(?P<offset>-?(?:0x)?[0-9a-f]+)\[bp\]", text):
            candidate = int(match["offset"], 0)
            if local_offset <= candidate < local_offset + 0x20:
                reads, _writes = register_accesses(item)
                if reads:
                    return local_offset
    return None


def merge_carriers(values: list[tuple[ReturnCarrier, ...]]) -> tuple[ReturnCarrier, ...]:
    widths: dict[str, int] = {}
    for value in values:
        for carrier in value:
            widths[carrier.register] = max(widths.get(carrier.register, 0), carrier.width)
    return tuple(sorted(
        ReturnCarrier(register, width) for register, width in widths.items()
    ))


def observed_modified_carriers(
    observed: tuple[ReturnCarrier, ...],
    modified: tuple[ReturnCarrier, ...],
) -> tuple[ReturnCarrier, ...]:
    """Exclude reads of registers which the callee demonstrably preserves."""
    modified_names = {carrier.register for carrier in modified}
    return tuple(
        carrier for carrier in observed if carrier.register in modified_names
    )


def derive_corpus_abis(rows, original_listings, recovered_listings):
    near_targets = {int(row["entry"], 16): row["function"] for row in rows}
    far_targets = {}
    for row in rows:
        text = (ROOT / row["asm_path"]).read_text(
            encoding="ascii", errors="replace"
        )
        match = ASM_SEGMENT_ROW.search(text)
        if match is None:
            raise ValueError(f"{row['asm_path']}: missing seg_off metadata")
        key = (int(match["segment"], 16), int(match["offset"], 16))
        if key in far_targets:
            raise ValueError(f"duplicate original far entry {key[0]:04x}:{key[1]:04x}")
        far_targets[key] = row["function"]

    original_uses: dict[str, list[tuple[ReturnCarrier, ...]]] = defaultdict(list)
    recovered_uses: dict[str, list[tuple[ReturnCarrier, ...]]] = defaultdict(list)
    hidden_callers: set[str] = set()
    for listing in original_listings.values():
        for index, item in enumerate(listing.instructions):
            text = normalized_text((item,))[0]
            target = None
            match = DIRECT_NEAR_CALL.match(text)
            if match is not None:
                target = near_targets.get(parse_number(match["target"]))
            else:
                match = DIRECT_FAR_CALL.match(text)
                if match is not None:
                    target = far_targets.get((
                        parse_number(match["segment"]),
                        parse_number(match["offset"]),
                    ))
            if target is not None:
                original_uses[target].append(
                    carriers_consumed_after_call(listing, index)
                )
    functions = {row["function"] for row in rows}
    for listing in recovered_listings.values():
        for index, item in enumerate(listing.instructions):
            match = SYMBOLIC_CALL.match(normalized_text((item,))[0])
            if match is None or match["function"] not in functions:
                continue
            target = match["function"]
            recovered_uses[target].append(
                carriers_consumed_after_call(listing, index)
            )
            if caller_hidden_result_offset(listing, index) is not None:
                hidden_callers.add(target)

    result = {}
    for row in rows:
        function = row["function"]
        original = original_listings[function]
        recovered = recovered_listings[function]
        original_modified = locally_modified_carriers(original)
        recovered_modified = locally_modified_carriers(recovered)
        if original_uses[function]:
            original_carriers = observed_modified_carriers(
                merge_carriers(original_uses[function]), original_modified
            )
            original_evidence = "direct callers"
        else:
            original_carriers = original_modified
            original_evidence = "callee exits"
        if recovered_uses[function]:
            recovered_carriers = observed_modified_carriers(
                merge_carriers(recovered_uses[function]), recovered_modified
            )
            recovered_evidence = "direct callers"
        else:
            recovered_carriers = recovered_modified
            recovered_evidence = "callee exits"
        hidden_width = copied_result_width(recovered) if function in hidden_callers else 0
        if function in hidden_callers and not hidden_width:
            raise ValueError(
                f"{function}: hidden DS:SI result caller has no derivable callee copy width"
            )
        result[function] = (
            RoutineAbi(
                routine_return_sites(original.instructions),
                original_carriers,
                original_evidence,
            ),
            RoutineAbi(
                routine_return_sites(recovered.instructions),
                recovered_carriers,
                recovered_evidence,
                hidden_width,
            ),
        )
    return result


def format_carriers(abi: RoutineAbi) -> str:
    values = [f"{item.register}:{item.width}" for item in abi.carriers]
    if abi.hidden_result_width:
        values.append(f"hidden-ds:si-memory:{abi.hidden_result_width * 8}")
    return ",".join(values) if values else "void"


def compare_routine_abi(
    function: str, original: RoutineAbi, recovered: RoutineAbi
) -> list[str]:
    errors: list[str] = []
    if len(original.returns) != 1:
        errors.append(
            f"{function}: unresolved original return convention {original.returns}"
        )
    if len(recovered.returns) != 1:
        errors.append(
            f"{function}: unresolved recovered return convention {recovered.returns}"
        )
    if original.returns != recovered.returns:
        errors.append(
            f"{function}: return convention mismatch: original="
            f"{original.returns}, recovered={recovered.returns}"
        )
    if (original.carriers != recovered.carriers or
            recovered.hidden_result_width):
        errors.append(
            f"{function}: return carrier mismatch: original="
            f"{format_carriers(original)} ({original.carrier_evidence}), "
            f"recovered={format_carriers(recovered)} "
            f"({recovered.carrier_evidence})"
        )
    return errors


def audit_all_routine_abis(listing_dir: Path, manifest: Path = MANIFEST) -> tuple[list[str], int]:
    with manifest.open(newline="", encoding="ascii") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    required = {"entry", "source", "asm_path", "function"}
    if not rows or not required.issubset(rows[0]):
        missing = sorted(required.difference(rows[0] if rows else {}))
        raise ValueError(f"{manifest}: missing ABI manifest rows/columns: {missing}")

    original_listings = {}
    recovered_listings = {}
    errors: list[str] = []
    for row in rows:
        function = row["function"]
        if function in original_listings:
            errors.append(f"{function}: duplicate ABI manifest routine")
            continue
        asm = (ROOT / row["asm_path"]).resolve()
        emitted = listing_dir / f"{Path(row['source']).stem}.lst"
        try:
            original_listings[function] = parse_original_listing(asm)
            if not emitted.is_file():
                raise ValueError(f"missing recovered listing {emitted}")
            parsed = SEGMENTS.parse_listing(
                emitted, emitted.read_text(encoding="ascii", errors="replace")
            )
            if f"{function}_" not in parsed.labels:
                raise ValueError(f"{emitted}: missing recovered label {function}_")
            recovered_listings[function] = parsed
        except (OSError, ValueError) as error:
            errors.append(f"{function}: unresolved ABI evidence: {error}")
    if errors:
        return errors, len(rows)

    try:
        abis = derive_corpus_abis(rows, original_listings, recovered_listings)
    except ValueError as error:
        return [f"unresolved ABI evidence: {error}"], len(rows)
    for row in rows:
        function = row["function"]
        original, recovered = abis[function]
        errors.extend(compare_routine_abi(function, original, recovered))
    return errors, len(rows)


def routine_instructions(listing, label: str):
    start = listing.labels.get(label)
    if start is None:
        raise ValueError(f"{listing.object_path}: missing label {label}")
    public_starts = [
        offset
        for name, offset in listing.labels.items()
        if offset > start and not name.startswith("L$")
    ]
    end = min(public_starts, default=0x10000)
    return [item for item in listing.instructions if start <= item.offset < end]


def normalized_text(items) -> list[str]:
    return [" ".join(item.text.lower().split()) for item in items]


def audit_sound(listing) -> list[str]:
    errors: list[str] = []
    instructions = routine_instructions(listing, "snd_play_clip_")
    text = normalized_text(instructions)
    load_sequence = (
        r"^push\s+ds$",
        r"^mov\s+ax,dgroup(?::\w+)?$",
        r"^mov\s+ds,ax$",
    )
    cursor = 0
    for pattern in load_sequence:
        position = next(
            (
                index
                for index in range(cursor, min(len(text), 20))
                if re.search(pattern, text[index])
            ),
            None,
        )
        if position is None:
            errors.append(
                "snd_play_clip: entry does not restore DS from linked DGROUP "
                f"before foreign-XDB callers: missing {pattern}"
            )
            break
        cursor = position + 1
    for symbol in (
        "_snd_driver_pending_flag_gs",
        "_audio_position_callback_gs",
    ):
        accesses = [item.text.lower() for item in instructions if symbol in item.text.lower()]
        if not accesses:
            errors.append(f"snd_play_clip: missing access to {symbol}")
            continue
        for text in accesses:
            if not re.search(rf"\b(?:es|fs|gs):{re.escape(symbol)}\b", text):
                errors.append(
                    f"snd_play_clip: {symbol} can inherit caller DS: {text}"
                )
    return errors


def audit_critical_error(listing) -> list[str]:
    instructions = routine_instructions(
        listing, "bloodprg_critical_error_handler_"
    )
    text = normalized_text(instructions)
    errors: list[str] = []
    if any(item.split(maxsplit=1)[0] == "sti" for item in text):
        errors.append("INT 24h handler enables IRQs before its generated epilogue")
    if not text or text[-1].split(maxsplit=1)[0] != "iret":
        errors.append("INT 24h handler does not terminate directly with IRET")
    return errors


def find_instruction(items, start: int, pattern: str) -> int | None:
    regex = re.compile(pattern)
    for index in range(start, len(items)):
        if regex.search(items[index].text.lower()):
            return index
    return None


def audit_xms_allocate(listing) -> list[str]:
    instructions = routine_instructions(listing, "cb_xms_allocate_kb_")
    required = (
        r"^mov\s+ah,0x0*9\b",
        r"^call\s+.*_xms_driver_entry\b",
        r"^mov\s+cx,dx\b",
        r"^xor\s+dx,dx\b",
        r"^or\s+ax,ax\b",
        r"^j(?:e|z)\s+",
        r"^inc\s+dx\b",
        r"^mov\s+ax,cx\b",
        r"^mov\s+word ptr \[si\],ax\b",
        r"^test\s+dx,dx\b",
        r"^setne\s+al\b",
    )
    positions: list[int] = []
    cursor = 0
    for pattern in required:
        position = find_instruction(instructions, cursor, pattern)
        if position is None:
            return [
                "XMS allocate does not preserve AX=status and DX=handle: "
                f"missing emitted pattern {pattern}"
            ]
        positions.append(position)
        cursor = position + 1

    branch = instructions[positions[5]]
    target_name = branch.text.split()[-1]
    target = listing.labels.get(target_name)
    increment = instructions[positions[6]]
    handle_move = instructions[positions[7]]
    if target is None or not (increment.offset < target <= handle_move.offset):
        return [
            "XMS allocate success test does not skip the DX success increment "
            "when returned AX is zero"
        ]
    return []


def audit_segment_install(main_listing) -> list[str]:
    instructions = routine_instructions(main_listing, "main_")
    required = (
        r"^mov\s+dx,\s*ds\b",
        r"^mov\s+gs,\s*dx\b",
        r"^mov\s+fs,\s*ax\b",
    )
    cursor = 0
    for pattern in required:
        position = find_instruction(instructions, cursor, pattern)
        if position is None:
            return [f"main does not establish DS=GS and resource-table FS: {pattern}"]
        cursor = position + 1
    return []


def audit_overlay_request_segment(adapter_listing) -> list[str]:
    instructions = routine_instructions(
        adapter_listing, "cb_overlay_call_inherited_bp_"
    )
    required = (
        r"^mov\s+bp,\s*si\b",
        r"^call\s+dword ptr ss:\[bx\]$",
    )
    cursor = 0
    for pattern in required:
        position = find_instruction(instructions, cursor, pattern)
        if position is None:
            return [
                "overlay bridge no longer passes a DS-owned request offset "
                f"through inherited SS:BP: {pattern}"
            ]
        cursor = position + 1
    return []


def audit_vm_record_distance_call(caller_listing, callee_listing) -> list[str]:
    caller = normalized_text(
        routine_instructions(caller_listing, "vm_op_c1_record_state_")
    )
    callee = normalized_text(
        routine_instructions(callee_listing, "ship_3d_position_distance_")
    )
    errors: list[str] = []

    segment_load = next(
        (
            (index, match)
            for index, text in enumerate(caller)
            if (match := re.match(
                r"^mov\s+(?P<reg>[a-z]{2}),word ptr "
                r"(?:[a-z]{2}:)?_vm_record_base_gs\+(?:0x)?0*2$",
                text,
            ))
        ),
        None,
    )
    segment_slot = None
    if segment_load is not None:
        load_index, match = segment_load
        register = match["reg"]
        for text in caller[load_index + 1 : load_index + 6]:
            stored = re.match(
                rf"^mov\s+word ptr (?P<slot>[^,]+\[bp\]),{register}$",
                text,
            )
            if stored is not None:
                segment_slot = stored["slot"]
                break
    if segment_slot is None:
        errors.append(
            "vm C1 distance call does not retain the VM record-base segment"
        )

    call_index = next(
        (
            index
            for index, text in enumerate(caller)
            if re.match(
                r"^call\s+(?:near ptr )?ship_3d_position_distance_$", text
            )
        ),
        None,
    )
    if call_index is None:
        errors.append("vm C1 distance call is missing or no longer near")
    elif segment_slot is not None:
        window = caller[max(0, call_index - 8) : call_index]
        required = (
            rf"^mov\s+cx,word ptr {re.escape(segment_slot)}$",
            r"^mov\s+bx,si$",
            r"^mov\s+dx,cx$",
        )
        cursor = 0
        for pattern in required:
            position = next(
                (
                    index
                    for index in range(cursor, len(window))
                    if re.match(pattern, window[index])
                ),
                None,
            )
            if position is None:
                errors.append(
                    "vm C1 distance call does not pass the record segment in "
                    f"both far-pointer pairs: missing {pattern}"
                )
                break
            cursor = position + 1

    required_callee = (
        r"^mov\s+si,ax$",
        r"^mov\s+word ptr (?P<first>[^,]+\[bp\]),dx$",
        r"^mov\s+di,bx$",
        r"^mov\s+word ptr (?P<second>[^,]+\[bp\]),cx$",
        r"^mov\s+es,dx$",
    )
    cursor = 0
    second_slot = None
    for pattern in required_callee:
        match_index = next(
            (
                index
                for index in range(cursor, min(len(callee), 24))
                if re.match(pattern, callee[index])
            ),
            None,
        )
        if match_index is None:
            errors.append(
                "ship_3d_position_distance does not retain both far-pointer "
                f"segments: missing {pattern}"
            )
            break
        matched = re.match(pattern, callee[match_index])
        if matched is not None and "second" in matched.groupdict():
            second_slot = matched["second"]
        cursor = match_index + 1
    if second_slot is not None and not any(
        re.match(rf"^mov\s+es,word ptr {re.escape(second_slot)}$", text)
        for text in callee
    ):
        errors.append(
            "ship_3d_position_distance never selects the second record segment"
        )
    returns = [text for text in callee if re.match(r"^ret(?:\s|$)", text)]
    if not returns or any(
        re.match(r"^ret\s+(?:0x)?0*2$", text) is None for text in returns
    ):
        errors.append(
            "ship_3d_position_distance no longer pops its stacked compare word"
        )
    return errors


def audit_ship_target_transition_liveness(listing) -> list[str]:
    """Require the pre-call completion flag to survive the real interpolator."""
    instructions = routine_instructions(
        listing, "ship_3d_target_record_select_"
    )
    text = normalized_text(instructions)
    call_index = next(
        (
            index
            for index, item in enumerate(text)
            if re.match(
                r"^call\s+(?:far ptr )?"
                r"framebuffer_rect_interpolate_and_remap_step_$",
                item,
            )
        ),
        None,
    )
    if call_index is None:
        return ["ship target selector no longer calls the rectangle interpolator"]

    set_index = next(
        (
            index
            for index in range(call_index - 1, max(-1, call_index - 12), -1)
            if re.match(r"^sete\s+al$", text[index])
        ),
        None,
    )
    if set_index is None:
        return ["ship target selector does not snapshot transition completion"]

    preserved_operand = None
    for item in text[set_index + 1 : call_index]:
        register = re.match(
            r"^movzx\s+(?P<operand>bx|cx|dx|si|di|bp),al$", item
        )
        spill = re.match(
            r"^mov\s+byte ptr (?P<operand>[^,]*\[bp\]),al$", item
        )
        match = register or spill
        if match is not None:
            preserved_operand = match["operand"]
            break
    if preserved_operand is None:
        return [
            "ship target selector leaves transition completion in AX across "
            "the AX-clobbering rectangle interpolator"
        ]

    tested = any(
        re.match(
            rf"^(?:test\s+{re.escape(preserved_operand)},"
            rf"{re.escape(preserved_operand)}|cmp\s+{re.escape(preserved_operand)},0)$",
            item,
        )
        for item in text[call_index + 1 : call_index + 6]
    )
    if not tested:
        return [
            "ship target selector does not consume the preserved transition "
            "completion value after interpolation"
        ]
    return []


def startup_segment_rows(link_map: Path) -> tuple[int, int]:
    text = link_map.read_text(encoding="ascii", errors="replace")
    dgroup = DGROUP_ROW.search(text)
    game_data = GAME_DATA_ROW.search(text)
    if dgroup is None or game_data is None:
        raise ValueError(f"{link_map}: missing DGROUP or GAME_DATA placement")
    return int(dgroup["segment"], 16), int(game_data["segment"], 16)


def audit_startup_sequence(text: list[str], dgroup: int, game_data: int) -> list[str]:
    errors: list[str] = []
    if game_data != dgroup:
        errors.append(
            f"GAME_DATA {game_data:04x}:0000 does not begin at DGROUP "
            f"{dgroup:04x}:0000"
        )
        return errors

    immediate = rf"0x0*{dgroup:x}\b"
    required = (
        rf"^mov\s+cx,\s*{immediate}",
        r"^mov\s+es,\s*cx\b",
        r"^mov\s+ss,\s*cx\b",
        r"^mov\s+sp,\s*bx\b",
        rf"^mov\s+dx,\s*{immediate}",
        r"^mov\s+ds,\s*dx\b",
    )
    cursor = 0
    positions: list[int] = []
    for pattern in required:
        position = next(
            (index for index in range(cursor, len(text)) if re.search(pattern, text[index])),
            None,
        )
        if position is None:
            errors.append(
                "CRT startup does not establish SS=DS=GAME_DATA: "
                f"missing emitted pattern {pattern}"
            )
            return errors
        positions.append(position)
        cursor = position + 1

    if positions[3] != positions[2] + 1:
        errors.append("CRT startup does not load SP immediately after loading SS")
    return errors


def audit_startup_image(image: Path, link_map: Path) -> list[str]:
    dgroup, game_data = startup_segment_rows(link_map)
    mz = MZ(image)
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    instructions = list(
        decoder.disasm(mz.data[mz.entry_file : mz.entry_file + 0x240], mz.e_ip)
    )
    text = [
        " ".join(f"{item.mnemonic} {item.op_str}".lower().split())
        for item in instructions
    ]
    return audit_startup_sequence(text, dgroup, game_data)


def audit(
    sound_listing,
    critical_listing,
    adapter_listing,
    main_listing,
    vm_c1_listing,
    position_distance_listing,
    ship_target_listing,
) -> list[str]:
    return [
        *audit_sound(sound_listing),
        *audit_critical_error(critical_listing),
        *audit_xms_allocate(adapter_listing),
        *audit_overlay_request_segment(adapter_listing),
        *audit_segment_install(main_listing),
        *audit_vm_record_distance_call(
            vm_c1_listing, position_distance_listing
        ),
        *audit_ship_target_transition_liveness(ship_target_listing),
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listing-dir", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--adapter-object", type=Path, required=True)
    parser.add_argument("--main-object", type=Path, required=True)
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--link-map", type=Path, required=True)
    parser.add_argument("--wdis", type=Path, default=Path("wdis"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    listing_dir = args.listing_dir.resolve()

    def cached(name: str):
        path = listing_dir / name
        return SEGMENTS.parse_listing(
            path, path.read_text(encoding="ascii", errors="replace")
        )

    sound = cached("func_00b8cd_snd_play_clip.lst")
    critical = cached("func_000c1a_bloodprg_critical_error_handler.lst")
    vm_c1 = cached("func_006b4c_vm_op_c1_record_state.lst")
    position_distance = cached("func_0060dd_ship_3d_position_distance.lst")
    ship_target = cached("func_00b2bb_ship_3d_target_record_select.lst")
    adapter = SEGMENTS.listing_for_object(
        args.wdis, args.adapter_object, listing_dir
    )
    main_listing = SEGMENTS.listing_for_object(
        args.wdis, args.main_object, listing_dir
    )
    errors = [
        *audit(
            sound,
            critical,
            adapter,
            main_listing,
            vm_c1,
            position_distance,
            ship_target,
        ),
        *audit_startup_image(args.image.resolve(), args.link_map.resolve()),
    ]
    whole_program_errors, routine_count = audit_all_routine_abis(
        listing_dir, args.manifest.resolve()
    )
    errors.extend(whole_program_errors)
    if errors:
        raise SystemExit("\n".join(errors))
    print(
        f"relinked ABI: {routine_count} routine returns/carriers, "
        "startup/overlay segments, foreign-DS sound, "
        "VM-record far pointers, ship-transition liveness, XMS AX/DX result, "
        "and INT 24h epilogue verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
