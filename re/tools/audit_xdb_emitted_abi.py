#!/usr/bin/env python3
"""Verify linked XDB callback and overlay ABIs in emitted machine code."""
from __future__ import annotations

import argparse
import csv
import importlib.util
import re
import sys
from collections import deque
from pathlib import Path

from capstone import CS_ARCH_X86, CS_GRP_RET, CS_MODE_16, Cs
from capstone.x86_const import X86_OP_IMM, X86_OP_REG


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "xdb_emitted_abi_core", ROOT / "re/tools/audit_segment_contracts.py"
)
CORE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CORE
SPEC.loader.exec_module(CORE)

MODULES = ("amer", "croolis", "scrut")
ALL_MODULES = MODULES + ("manu3",)
FORBIDDEN_SEGMENT_WRITES = frozenset(("ds", "fs", "gs", "ss"))
ALIEN_ENTRY_SEQUENCE = (
    ("push", "eax"), ("push", "ebx"), ("push", "ecx"),
    ("push", "edx"), ("push", "esi"), ("push", "edi"),
    ("push", "ds"), ("push", "es"), ("push", "fs"),
    ("push", "gs"), ("push", "ebp"), ("cld", ""),
    ("mov", "ax,bp"), ("mov", "dx,ss"), ("mov", "bx,cs"),
    ("push", "cs"), ("call", None), ("pop", "ebp"),
    ("pop", "gs"), ("pop", "fs"), ("pop", "es"),
    ("pop", "ds"), ("pop", "edi"), ("pop", "esi"),
    ("pop", "edx"), ("pop", "ecx"), ("pop", "ebx"),
    ("pop", "eax"), ("retf", ""),
)
MANU3_ENTRY_SEQUENCE = (
    ("push", "ds"), ("mov", "ax,cs"), ("add", None),
    ("mov", "ds,ax"), ("mov", "fs,ax"), ("mov", "es,ax"),
    ("cld", ""), ("mov", "ax,bp"), ("mov", "dx,ss"),
    ("mov", "bx,cs"), ("push", "cs"), ("call", None),
    ("pop", "ds"), ("retf", ""),
)


def decoder() -> Cs:
    result = Cs(CS_ARCH_X86, CS_MODE_16)
    result.detail = True
    return result


def normalized_operand(value: str) -> str:
    return re.sub(r"\s+", "", value.lower())


def validate_entry_image(
    image: bytes, expected_sequence=ALIEN_ENTRY_SEQUENCE
) -> list[str]:
    instructions = []
    for instruction in decoder().disasm(image[:128], 0):
        instructions.append(instruction)
        if instruction.mnemonic == "retf":
            break
    errors: list[str] = []
    if len(instructions) != len(expected_sequence):
        return [
            "overlay entry has "
            f"{len(instructions)} instructions through RETF; expected "
            f"{len(expected_sequence)}"
        ]
    for index, (instruction, expected) in enumerate(
        zip(instructions, expected_sequence)
    ):
        mnemonic, operand = expected
        actual_operand = normalized_operand(instruction.op_str)
        if instruction.mnemonic != mnemonic:
            errors.append(
                f"entry instruction {index}: {instruction.mnemonic} != {mnemonic}"
            )
        elif operand is not None and actual_operand != operand:
            errors.append(
                f"entry instruction {index}: {actual_operand or '<none>'} "
                f"!= {operand or '<none>'}"
            )
    return errors


def segment_writes(
    listing, allowed: frozenset[str] = frozenset()
) -> list[str]:
    errors: list[str] = []
    for item in listing.instructions:
        instruction = CORE.decode_instruction(item)
        try:
            _read, written = instruction.regs_access()
        except Exception:
            written = ()
        names = {
            CORE.canonical_register(instruction.reg_name(register))
            for register in written
        }
        clobbered = sorted((names & FORBIDDEN_SEGMENT_WRITES) - allowed)
        if clobbered:
            errors.append(
                f"0x{item.offset:04x} writes {','.join(clobbered)}: {item.text}"
            )
    return errors


def required_segments(item) -> frozenset[str]:
    lowered = item.text.lower()
    result = set(re.findall(r"\b(es|fs|gs|ds|ss|cs)\s*:", lowered))
    mnemonic = CORE.decode_instruction(item).mnemonic
    if mnemonic in ("stosb", "stosw", "stosd", "scasb", "scasw", "scasd", "insb", "insw", "insd"):
        result.add("es")
    if mnemonic in ("movsb", "movsw", "movsd", "cmpsb", "cmpsw", "cmpsd"):
        result.update(("ds", "es"))
    if mnemonic in ("lodsb", "lodsw", "lodsd", "outsb", "outsw", "outsd"):
        result.add("ds")
    return frozenset(result)


def segment_definitions(item) -> frozenset[str]:
    instruction = CORE.decode_instruction(item)
    try:
        _read, written = instruction.regs_access()
    except Exception:
        written = ()
    result = {
        name
        for register in written
        if (
            name := CORE.canonical_register(instruction.reg_name(register))
        ) in CORE.SEGMENT_REGISTERS
    }
    if instruction.mnemonic in ("lds", "les", "lfs", "lgs"):
        result.add(instruction.mnemonic[1:])
    return frozenset(result)


def far_segment_definition_errors(
    listing, initial: frozenset[str]
) -> tuple[int, list[str]]:
    by_offset = {item.offset: item for item in listing.instructions}
    edges = CORE.successors(listing)
    states: dict[int, frozenset[str]] = {
        entry: initial for entry in listing.entrypoints
    }
    pending = deque(listing.entrypoints)
    errors: set[str] = set()
    use_count = 0
    counted: set[tuple[int, str]] = set()
    while pending:
        offset = pending.popleft()
        item = by_offset[offset]
        state = states[offset]
        required = required_segments(item)
        for segment in required:
            key = (offset, segment)
            if key not in counted:
                counted.add(key)
                use_count += 1
            if segment not in state:
                errors.add(
                    f"0x{offset:04x}: {segment.upper()} use has no reaching "
                    f"selector definition: {item.text}"
                )
        outgoing = state | segment_definitions(item)
        instruction = CORE.decode_instruction(item)
        if instruction.mnemonic == "call":
            outgoing = outgoing - {"es"}
        for target in edges[offset]:
            previous = states.get(target)
            merged = outgoing if previous is None else previous & outgoing
            if previous != merged:
                states[target] = merged
                pending.append(target)
    return use_count, sorted(errors)


def stack_operand_size(instruction) -> int:
    if instruction.operands:
        return instruction.operands[0].size or 2
    return 2


def stack_transfer(item, state: tuple[int, int | None]) -> tuple[int, int | None]:
    sp, bp = state
    instruction = CORE.decode_instruction(item)
    mnemonic = instruction.mnemonic
    operands = instruction.operands
    if mnemonic in ("push", "pushf", "pushfd"):
        return sp - stack_operand_size(instruction), bp
    if mnemonic in ("pop", "popf", "popfd"):
        next_bp = bp
        if operands and operands[0].type == X86_OP_REG:
            if instruction.reg_name(operands[0].reg) in ("bp", "ebp"):
                next_bp = None
        return sp + stack_operand_size(instruction), next_bp
    if mnemonic in ("pusha", "pushaw", "pushad"):
        return sp - (32 if mnemonic == "pushad" else 16), bp
    if mnemonic in ("popa", "popaw", "popad"):
        return sp + (32 if mnemonic == "popad" else 16), None
    if mnemonic == "mov" and len(operands) == 2:
        destination, source = operands
        if destination.type == X86_OP_REG and source.type == X86_OP_REG:
            destination_name = instruction.reg_name(destination.reg)
            source_name = instruction.reg_name(source.reg)
            if destination_name in ("bp", "ebp") and source_name in ("sp", "esp"):
                return sp, sp
            if destination_name in ("sp", "esp") and source_name in ("bp", "ebp"):
                return (bp if bp is not None else sp), bp
    if mnemonic in ("add", "sub") and len(operands) == 2:
        destination, source = operands
        if (
            destination.type == X86_OP_REG
            and instruction.reg_name(destination.reg) in ("sp", "esp")
            and source.type == X86_OP_IMM
        ):
            amount = source.imm
            return sp + amount if mnemonic == "add" else sp - amount, bp
    if mnemonic == "leave":
        if bp is None:
            raise ValueError(f"0x{item.offset:04x}: LEAVE has unknown BP")
        return bp + 2, None
    return state


def stack_balance_errors(listing) -> list[str]:
    by_offset = {item.offset: item for item in listing.instructions}
    edges = CORE.successors(listing)
    states: dict[int, tuple[int, int | None]] = {
        entry: (0, None) for entry in listing.entrypoints
    }
    pending = deque(listing.entrypoints)
    errors: list[str] = []
    while pending:
        offset = pending.popleft()
        item = by_offset[offset]
        try:
            outgoing = stack_transfer(item, states[offset])
        except ValueError as exc:
            errors.append(str(exc))
            continue
        for target in edges[offset]:
            previous = states.get(target)
            if previous is not None and previous != outgoing:
                errors.append(
                    f"0x{target:04x}: incompatible stack states "
                    f"{previous} and {outgoing}"
                )
                continue
            if previous is None:
                states[target] = outgoing
                pending.append(target)

    for offset, state in sorted(states.items()):
        item = by_offset[offset]
        instruction = CORE.decode_instruction(item)
        if not instruction.group(CS_GRP_RET):
            continue
        if item.data[0] not in (0xC3, 0xCB):
            errors.append(f"0x{offset:04x}: callback uses RET with stack arguments")
        if state[0] != 0:
            errors.append(
                f"0x{offset:04x}: return has stack delta {state[0]}: {item.text}"
            )
        continue
    for offset, state in sorted(states.items()):
        if edges[offset] or state[0] == 0:
            continue
        item = by_offset[offset]
        errors.append(
            f"0x{offset:04x}: terminal transfer has stack delta "
            f"{state[0]}: {item.text}"
        )
    return sorted(set(errors))


def near_return_errors(listing) -> list[str]:
    returns = [
        item for item in listing.instructions
        if CORE.decode_instruction(item).group(CS_GRP_RET)
    ]
    if not returns:
        edges = CORE.successors(listing)
        terminals = [item for item in listing.instructions if not edges[item.offset]]
        if terminals and all(
            CORE.decode_instruction(item).mnemonic == "jmp"
            and item.data[0] in (0xE9, 0xEB)
            for item in terminals
        ):
            return []
        return ["callback has neither a near return nor a direct near tail jump"]
    errors = []
    for item in returns:
        if item.data[0] != 0xC3:
            errors.append(
                f"0x{item.offset:04x}: callback must use near RET: {item.text}"
            )
    return errors


def listing_for_source(root: Path, module: str, source: str):
    stem = Path(source).stem
    path = root / module / "segment_contract_listings" / f"{stem}.lst"
    if not path.is_file():
        raise ValueError(f"missing emitted listing {path}")
    return CORE.parse_listing(path, path.read_text(encoding="ascii", errors="replace"))


def require_pattern(items, pattern: str, label: str) -> list[str]:
    if any(re.search(pattern, item.text, re.IGNORECASE) for item in items):
        return []
    return [f"missing {label}: {pattern}"]


def dispatch_errors(root: Path, module: str) -> list[str]:
    errors: list[str] = []
    main = listing_for_source(root, module, f"func_0000a3_main.c")
    items = list(main.instructions)
    method_calls = [
        index for index, item in enumerate(items)
        if re.search(r"call\s+word ptr _xdb_alien_method_table\[bx\]", item.text)
    ]
    if len(method_calls) != 1:
        errors.append(f"main has {len(method_calls)} method-table calls; expected one")
    else:
        context = items[max(0, method_calls[0] - 6):method_calls[0]]
        errors += require_pattern(context, r"mov\s+bx,word ptr 0x34\[di\]", "method slot load")
        errors += require_pattern(context, r"mov\s+di,word ptr \[di\]", "method context load")

    frame_calls = [
        index for index, item in enumerate(items)
        if re.search(r"call\s+dword ptr _xdb_alien_frame_callback_ptr", item.text)
    ]
    if len(frame_calls) != 2:
        errors.append(f"main has {len(frame_calls)} frame-callback calls; expected two")
    for index in frame_calls:
        previous = [item.text.lower() for item in items[max(0, index - 3):index]]
        expected = (r"shl\s+edx,0x10", r"mov\s+dx,ax", r"mov\s+ax,bx")
        if len(previous) != 3 or any(
            re.search(pattern, text) is None
            for pattern, text in zip(expected, previous)
        ):
            errors.append(
                f"0x{items[index].offset:04x}: frame callback lacks AX/EDX setup"
            )

    slot3 = next(
        (path for path in (root / module / "segment_contract_listings").glob(
            "func_*_method_slot_3_update_or_init.lst"
        )),
        None,
    )
    if slot3 is None:
        errors.append("missing method-slot-3 emitted listing")
    else:
        listing = CORE.parse_listing(
            slot3, slot3.read_text(encoding="ascii", errors="replace")
        )
        slot_items = list(listing.instructions)
        calls = [
            index for index, item in enumerate(slot_items)
            if re.search(r"call\s+word ptr 0xe\[bx\]", item.text)
        ]
        if len(calls) != 1:
            errors.append(f"method slot 3 has {len(calls)} state calls; expected one")
        else:
            context = slot_items[max(0, calls[0] - 4):calls[0]]
            errors += require_pattern(context, r"mov\s+bx,si", "state callback BX alias")
            errors += require_pattern(context, r"mov\s+si,word ptr", "state callback SI")
    return errors


def frame_callback_errors(listing, assignment_source: Path) -> list[str]:
    errors = segment_writes(listing, frozenset(("ds",)))
    returns = [
        item for item in listing.instructions
        if CORE.decode_instruction(item).group(CS_GRP_RET)
    ]
    if not returns or any(item.data[0] != 0xCB for item in returns):
        errors.append("snd_play_clip must return with far RETF on every path")
    items = list(listing.instructions)
    if not items or re.fullmatch(
        r"push\s+ds", items[0].text.strip(), re.IGNORECASE
    ) is None:
        errors.append("snd_play_clip does not save incoming DS at entry")
    for item in returns:
        index = items.index(item)
        if index == 0 or not re.fullmatch(
            r"pop\s+ds", items[index - 1].text.strip(), re.IGNORECASE
        ):
            errors.append(
                f"0x{item.offset:04x}: snd_play_clip does not restore DS before RETF"
            )
    text = assignment_source.read_text(encoding="ascii", errors="replace")
    if re.search(r"snd_play_clip_callback\s*=\s*snd_play_clip\s*;", text) is None:
        errors.append("snd_play_clip_callback is not assigned to snd_play_clip")
    return sorted(set(errors))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source-xdb-root", type=Path,
        default=ROOT / "output/recovered_dos_package/validation/source_xdb",
    )
    parser.add_argument(
        "--callback-edges", type=Path,
        default=ROOT / "output/recovered_dos_package/validation/source_xdb/callback_edges.tsv",
    )
    parser.add_argument(
        "--bloodprg-listing-root", type=Path,
        default=ROOT / "output/recovered_dos_package/validation/bloodprg_runtime/final/segment_contract_listings",
    )
    parser.add_argument("--skip-frame-callback", action="store_true")
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source_root = args.source_xdb_root.resolve()
    rows: list[tuple[str, str, str, str, str]] = []
    errors: list[str] = []

    with args.callback_edges.open(newline="", encoding="ascii") as handle:
        callback_rows = list(csv.DictReader(handle, delimiter="\t"))
    targets: dict[tuple[str, str, str], set[str]] = {}
    for row in callback_rows:
        if row["status"] != "owned_pointer":
            continue
        module = row["module"].removeprefix("xdb_")
        key = (module, row["target_function"], row["source"])
        targets.setdefault(key, set()).add(row["field"])

    far_use_count = 0
    far_routine_count = 0
    for module in ALL_MODULES:
        entry_errors = validate_entry_image(
            (source_root / module / f"{module}.xdb").read_bytes(),
            ALIEN_ENTRY_SEQUENCE if module in MODULES else MANU3_ENTRY_SEQUENCE,
        )
        status = "ok" if not entry_errors else "error"
        rows.append((module, "overlay_entry", "_xdb_overlay_entry", status, "; ".join(entry_errors)))
        errors.extend(f"{module}: entry: {error}" for error in entry_errors)

        for path in sorted(
            (source_root / module / "segment_contract_listings").glob("func_*.lst")
        ):
            listing = CORE.parse_listing(
                path, path.read_text(encoding="ascii", errors="replace")
            )
            function = next(
                label.rstrip("_")
                for label in listing.labels
                if not label.startswith(("_", "L$"))
            )
            initial = {"cs", "ss"}
            if not (
                module in MODULES
                and path.stem in ("func_000000_api_entry", "func_0000a3_main")
            ):
                initial.update(("ds", "fs"))
            if module == "manu3" and path.stem == "func_000000_api_entry":
                initial.update(("ds", "es", "fs"))
            if function.endswith("_face_activate") or path.stem.endswith(
                "_face_activate"
            ):
                initial.add("es")
            use_count, far_errors = far_segment_definition_errors(
                listing, frozenset(initial)
            )
            far_use_count += use_count
            if use_count or far_errors:
                far_routine_count += 1
                status = "ok" if not far_errors else "error"
                rows.append((
                    module,
                    "far_segment_definition",
                    function,
                    status,
                    "; ".join(far_errors),
                ))
                errors.extend(
                    f"{module}: {function}: {error}" for error in far_errors
                )

        if module not in MODULES:
            continue
        module_dispatch_errors = dispatch_errors(source_root, module)
        status = "ok" if not module_dispatch_errors else "error"
        rows.append((module, "dispatch", "indirect_call_sites", status, "; ".join(module_dispatch_errors)))
        errors.extend(f"{module}: dispatch: {error}" for error in module_dispatch_errors)

        method_paths = sorted(
            (source_root / module / "segment_contract_listings").glob(
                "func_*_method_*.lst"
            )
        )
        if len(method_paths) != 12:
            errors.append(
                f"{module}: method table has {len(method_paths)} unique emitted "
                "targets; expected 12"
            )
        for path in method_paths:
            listing = CORE.parse_listing(
                path, path.read_text(encoding="ascii", errors="replace")
            )
            method_errors = near_return_errors(listing)
            method_errors += segment_writes(listing)
            method_errors += stack_balance_errors(listing)
            function = next(
                label.rstrip("_")
                for label in listing.labels
                if label.startswith(f"xdb_{module}_method_")
            )
            status = "ok" if not method_errors else "error"
            rows.append((module, "method_table", function, status, "; ".join(method_errors)))
            errors.extend(
                f"{module}: {function}: {error}" for error in method_errors
            )

    for (module, function, source), fields in sorted(targets.items()):
        try:
            listing = listing_for_source(source_root, module, source)
            target_errors = near_return_errors(listing)
            target_errors += segment_writes(listing)
            target_errors += stack_balance_errors(listing)
        except ValueError as exc:
            target_errors = [str(exc)]
        status = "ok" if not target_errors else "error"
        family = ",".join(sorted(fields))
        rows.append((module, family, function, status, "; ".join(target_errors)))
        errors.extend(f"{module}: {function}: {error}" for error in target_errors)

    if not args.skip_frame_callback:
        frame_path = args.bloodprg_listing_root / "func_00b8cd_snd_play_clip.lst"
        frame_listing = CORE.parse_listing(
            frame_path, frame_path.read_text(encoding="ascii", errors="replace")
        )
        frame_errors = frame_callback_errors(
            frame_listing,
            ROOT / "re/source/bloodprg/candidates/seg_0b1b/func_00b7b0_audio_param_init_cd5.c",
        )
        rows.append(("bloodprg", "frame_callback", "snd_play_clip", "ok" if not frame_errors else "error", "; ".join(frame_errors)))
        errors.extend(f"bloodprg: snd_play_clip: {error}" for error in frame_errors)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        stream = args.output.open("w", newline="", encoding="ascii")
    else:
        stream = sys.stdout
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow(("module", "contract", "function", "status", "detail"))
    writer.writerows(rows)
    if args.output:
        stream.close()

    if errors:
        for error in sorted(set(errors)):
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(
        f"OK: {len(rows)} emitted overlay/callback ABI contract(s); "
        f"{len(targets)} unique callback target(s); {far_use_count} explicit "
        f"far-segment use(s) in {far_routine_count} routine(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
