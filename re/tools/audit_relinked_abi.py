#!/usr/bin/env python3
"""Verify recovered ABI boundaries in the emitted Open Watcom objects."""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path

import capstone


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


def audit(sound_listing, critical_listing, adapter_listing, main_listing) -> list[str]:
    return [
        *audit_sound(sound_listing),
        *audit_critical_error(critical_listing),
        *audit_xms_allocate(adapter_listing),
        *audit_overlay_request_segment(adapter_listing),
        *audit_segment_install(main_listing),
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listing-dir", type=Path, required=True)
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
    adapter = SEGMENTS.listing_for_object(
        args.wdis, args.adapter_object, listing_dir
    )
    main_listing = SEGMENTS.listing_for_object(
        args.wdis, args.main_object, listing_dir
    )
    errors = [
        *audit(sound, critical, adapter, main_listing),
        *audit_startup_image(args.image.resolve(), args.link_map.resolve()),
    ]
    if errors:
        raise SystemExit("\n".join(errors))
    print(
        "relinked ABI: startup/overlay segments, foreign-DS sound, "
        "XMS AX/DX result, and INT 24h epilogue verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
