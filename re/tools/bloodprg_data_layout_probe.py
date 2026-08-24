#!/usr/bin/env python3
"""Classify BLOODPRG link symbols and emit a layout-only OMF data probe."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
import re
import struct


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_HEADERS = ROOT / "re" / "source" / "bloodprg" / "candidates" / "include"
OFFSET_RE = re.compile(
    r"(?:GS|DS|SS|ES|FS|CS|game data|data)\s*:\s*(0x[0-9A-Fa-f]+)"
    r"|/\*\s*(0x[0-9A-Fa-f]+)\s*\*/",
    re.I,
)
SPAN_RE = re.compile(r"\bspan\s*:\s*(0x[0-9A-Fa-f]+|[0-9]+)\b", re.I)


class Declaration:
    __slots__ = ("symbol", "header", "segment", "offset", "span")

    def __init__(
        self,
        symbol: str,
        header: str,
        segment: str,
        offset: int,
        span: int = 1,
    ) -> None:
        self.symbol = symbol
        self.header = header
        self.segment = segment
        self.offset = offset
        self.span = span


class PointerRebinding:
    __slots__ = (
        "symbol",
        "segment",
        "offset",
        "original_file_offset",
        "original_offsets",
        "targets",
    )

    def __init__(
        self,
        symbol: str,
        segment: str,
        offset: int,
        original_file_offset: int,
        original_offsets: tuple[int, ...],
        targets: tuple[str, ...],
    ) -> None:
        self.symbol = symbol
        self.segment = segment
        self.offset = offset
        self.original_file_offset = original_file_offset
        self.original_offsets = original_offsets
        self.targets = targets


RUNTIME_POINTER_REBINDINGS = (
    PointerRebinding(
        "_nav_actor_handlers",
        "_CODE",
        0x06D4,
        0x07EB4,
        (0x07BC, 0x06E0, 0x095A, 0x099E, 0x0A1B, 0x08A2),
        tuple(f"nav_actor_handler_{index}_" for index in range(6)),
    ),
    PointerRebinding(
        "_nav_choice_handlers",
        "_CODE",
        0x0F29,
        0x08709,
        (0x0F33, 0x0F4C, 0x0FDD, 0x1068, 0x108C),
        tuple(f"nav_choice_handler_{index}_" for index in range(5)),
    ),
    PointerRebinding(
        "_input_action_handlers",
        "_CODE",
        0x123E,
        0x020EE,
        (
            0x1290, 0x12DD, 0x1351, 0x1352, 0x1353, 0x1359, 0x1374,
            0x139D, 0x1420, 0x135A, 0x135B, 0x135C, 0x135D, 0x1366,
            0x136F, 0x1402,
        ),
        (
            "input_action_move_previous_",
            "input_action_move_next_",
            "input_action_noop_2_",
            "input_action_noop_3_",
            "input_action_request_shutdown_",
            "input_action_noop_5_",
            "input_action_accept_",
            "input_action_cancel_",
            "input_action_latch_text_key_",
            "input_action_noop_9_",
            "input_action_noop_10_",
            "input_action_noop_11_",
            "input_action_noop_12_",
            "input_action_noop_13_",
            "input_action_noop_14_",
            "input_action_toggle_pause_",
        ),
    ),
    PointerRebinding(
        "_bloodprg_sprite_blitter_table",
        "_CODE",
        0x1592,
        0x04522,
        (0x15A6, 0x172C, 0x1C18, 0x1D46, 0x1FD2, 0x210A, 0x210B, 0x210C),
        (
            "sprite_blit_raw_transparent_",
            "sprite_blit_rle_transparent_",
            "sprite_blit_raw_opaque_",
            "sprite_blit_rle_opaque_",
            "sprite_blit_scaled_transparent_",
            "sprite_blitter_noop_5_",
            "sprite_blitter_noop_6_",
            "sprite_blitter_noop_7_",
        ),
    ),
    PointerRebinding(
        "_bloodprg_selected_sprite_blitter",
        "_CODE",
        0x15A2,
        0x04532,
        (0x0000,),
        ("0",),
    ),
    PointerRebinding(
        "_vm_opcode_handlers",
        "GAME_DATA",
        0x6EB0,
        0x142D0,
        (
            0x11B9, 0x11D2, 0x11E8, 0x11F6, 0x123B, 0x124B, 0x126C,
            0x141A, 0x1428, 0x1490, 0x14B5, 0x14AC, 0x14BC, 0x15A6,
            0x1562, 0x15A6, 0x1562, 0x14C3, 0x15A6, 0x15A6, 0x14C3,
            0x14C3, 0x14C3, 0x1707, 0x1766, 0x1766, 0x15A6, 0x15A6,
            0x15A6, 0x1766, 0x14C3, 0x14C3, 0x14C3, 0x17AC, 0x1A94,
            0x1B4E, 0x18DE, 0x1978, 0x19E0, 0x1A2F, 0x1BC2, 0x1C19,
            0x1145, 0x1170, 0x112E, 0x1627, 0x10F4, 0x1120, 0x1100,
            0x110C, 0x1118, 0x0000,
        ),
        (
            "vm_op_a0_push_", "vm_op_a1_pop_", "vm_op_a2_cond_call_",
            "vm_op_a3_block_", "vm_op_a4_jump_", "vm_op_a5_cond_state_array_",
            "vm_op_a6_text_", "vm_op_a7_set_if_presentation_",
            "vm_op_a8_load_string_", "vm_op_a9_cond_jump_", "vm_op_aa_yield_",
            "vm_op_ab_poke_byte_", "vm_op_ac_yield_",
            "vm_op_shared_record_wildcard_", "vm_op_shared_ae_b0_state_",
            "vm_op_shared_record_wildcard_", "vm_op_shared_ae_b0_state_",
            "vm_op_shared_state_marker_", "vm_op_shared_record_wildcard_",
            "vm_op_shared_record_wildcard_", "vm_op_shared_state_marker_",
            "vm_op_shared_state_marker_", "vm_op_shared_state_marker_",
            "vm_op_b7_record_op_", "vm_op_b8_record_readwrite_",
            "vm_op_b8_record_readwrite_", "vm_op_shared_record_wildcard_",
            "vm_op_shared_record_wildcard_", "vm_op_shared_record_wildcard_",
            "vm_op_b8_record_readwrite_", "vm_op_shared_state_marker_",
            "vm_op_shared_state_marker_", "vm_op_shared_state_marker_",
            "vm_op_c1_record_state_", "vm_op_c2_record_full_",
            "vm_op_c3_state_record_", "vm_op_c4_actor_", "vm_op_c5_record_match_",
            "vm_op_c6_record_match_", "vm_op_c7_record_match_",
            "vm_op_c8_record_match_", "vm_op_c9_clear_record_full_",
            "vm_op_ca_compare_var_", "vm_op_cb_compare_byte_",
            "vm_op_cc_set_record_byte_", "vm_op_cd_state_gated_",
            "vm_op_ce_cond_branch_", "vm_op_cf_clear_state_",
            "vm_op_d0_cond_branch_", "vm_op_d1_cond_branch_",
            "vm_op_d2_script_profile_request_", "vm_resource_profile_select_",
        ),
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unresolved", type=Path, required=True)
    parser.add_argument("--header-dir", type=Path, default=DEFAULT_HEADERS)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--image",
        type=Path,
        help="original BLOODPRG.EXE used to byte-back declaration intervals",
    )
    parser.add_argument("--code-file-base", type=lambda value: int(value, 0), default=0x600)
    parser.add_argument("--game-data-file-base", type=lambda value: int(value, 0), default=0xD420)
    parser.add_argument("--fs-data-file-base", type=lambda value: int(value, 0), default=0xC1F0)
    parser.add_argument(
        "--runtime-layout",
        action="store_true",
        help=(
            "place paragraph-aligned GAME_DATA first in DGROUP so original "
            "near offsets and based-segment accesses share one runtime owner"
        ),
    )
    return parser.parse_args()


def declarations(header_dir: Path) -> dict[str, Declaration]:
    result: dict[str, Declaration] = {}
    for path in sorted(header_dir.glob("*.h")):
        text = path.read_text(encoding="ascii")
        for match in re.finditer(r"\bextern\b(?P<body>.*?;)", text, re.S):
            body = match.group("body")
            following = text[match.end() :]
            # A comment after the semicolon belongs to this declaration only
            # when it starts on the same line.  Allowing ``\s*`` here consumed
            # the standalone ownership comment for the *next* declaration and
            # shifted runs of global offsets by one symbol.
            trailing_comment = re.match(
                r"[ \t]*(?:/\*.*?\*/|//[^\n]*)", following, re.S
            )
            local_comment = body
            if trailing_comment is not None:
                local_comment += trailing_comment.group(0)
            local_matches = list(OFFSET_RE.finditer(local_comment))
            preceding = text[max(0, match.start() - 600) : match.start()]
            comment = local_comment if local_matches else preceding + local_comment
            offset_matches = local_matches or list(OFFSET_RE.finditer(comment))
            offset_match = offset_matches[-1] if offset_matches else None
            if offset_match is None:
                continue
            declaration = " ".join(body.split())
            name_match = re.search(
                r"(?:\*\s*)?([A-Za-z_]\w*)\s*(?:\[[^]]*\])*\s*;\s*$",
                declaration,
            )
            if name_match is None:
                continue
            name = name_match.group(1)
            prefix = declaration[: name_match.start(1)]
            if "(" in prefix or re.search(r"\b" + re.escape(name) + r"\s*\(", declaration):
                continue
            if "CB_FS_DATA" in declaration or re.search(r"\bFS\s*:", comment, re.I):
                segment = "FS_DATA"
            elif "CB_CODE_DATA" in declaration or re.search(r"\bCS\s*:", comment, re.I):
                segment = "_CODE"
            else:
                segment = "GAME_DATA"
            span_match = SPAN_RE.search(comment)
            span = int(span_match.group(1), 0) if span_match else 1
            result.setdefault(
                "_" + name,
                Declaration(symbol="_" + name, header=path.name, segment=segment,
                            offset=int(offset_match.group(1) or offset_match.group(2), 16),
                            span=span),
            )
    return result


def read_symbols(path: Path) -> list[str]:
    with path.open(newline="", encoding="ascii") as handle:
        return [row["symbol"] for row in csv.DictReader(handle, delimiter="\t")]


def write_bytes(lines: list[str], data: bytes) -> None:
    for start in range(0, len(data), 16):
        lines.append(
            "db " + ", ".join(f"0x{byte:02x}" for byte in data[start : start + 16])
        )


def write_zeros(lines: list[str], length: int) -> None:
    for start in range(0, length, 16):
        lines.append("db " + ", ".join("0" for _ in range(min(16, length - start))))


def write_asm(
    path: Path,
    entries: list[Declaration],
    image: bytes | None,
    file_bases: dict[str, int],
    runtime_layout: bool,
    rebindings: tuple[PointerRebinding, ...],
) -> None:
    by_segment: dict[str, list[Declaration]] = {}
    for entry in entries:
        by_segment.setdefault(entry.segment, []).append(entry)

    lines = [
        "; Generated layout probe.",
        "; With --image, declaration intervals are copied from BLOODPRG.EXE.",
        "; This object still proves layout only; it is not a complete runtime owner.",
        ".386",
    ]
    targets = tuple(
        dict.fromkeys(
            target
            for rebinding in rebindings
            for target in rebinding.targets
            if target != "0"
        )
    )
    lines.extend(f"extrn {target}:near" for target in targets)
    if targets:
        lines.append("")
    rebinding_by_location = {
        (rebinding.segment, rebinding.offset): rebinding
        for rebinding in rebindings
    }
    for segment in ("_CODE", "GAME_DATA", "FS_DATA"):
        segment_entries = sorted(
            by_segment.get(segment, []), key=lambda item: (item.offset, item.symbol)
        )
        if not segment_entries:
            continue
        class_name = "CODE" if segment == "_CODE" else "FAR_DATA"
        alignment = "word"
        if runtime_layout and segment in ("GAME_DATA", "FS_DATA"):
            alignment = "para"
        lines.extend(
            [
                f"{segment} segment {alignment} public use16 '{class_name}'",
            ]
        )
        for start in range(0, len(segment_entries), 8):
            lines.append(
                "public "
                + ", ".join(
                    item.symbol for item in segment_entries[start : start + 8]
                )
            )
        current = 0
        index = 0
        while index < len(segment_entries):
            offset = segment_entries[index].offset
            if offset < current:
                raise ValueError(f"non-monotonic {segment} offset at {offset:#x}")
            lines.append(f"org {offset:#06x}")
            minimum_length = 1
            while index < len(segment_entries) and segment_entries[index].offset == offset:
                lines.append(f"{segment_entries[index].symbol} label byte")
                minimum_length = max(minimum_length, segment_entries[index].span)
                index += 1
            next_offset = (
                segment_entries[index].offset
                if index < len(segment_entries)
                else offset + minimum_length
            )
            length = next_offset - offset
            if length < minimum_length:
                raise ValueError(
                    f"{segment} declaration at {offset:#x} needs {minimum_length:#x} "
                    f"bytes but the next declaration starts at {next_offset:#x}"
                )
            rebinding = rebinding_by_location.get((segment, offset))
            rebound_bytes = 0
            if rebinding is not None:
                rebound_bytes = len(rebinding.targets) * 2
                if rebound_bytes > length:
                    raise ValueError(
                        f"{rebinding.symbol} pointer table crosses the next declaration"
                    )
                lines.extend(f"dw {target}" for target in rebinding.targets)
            remaining = length - rebound_bytes
            if remaining != 0:
                if image is None:
                    write_zeros(lines, remaining)
                else:
                    file_offset = file_bases[segment] + offset + rebound_bytes
                    data = image[file_offset : file_offset + remaining]
                    if len(data) != remaining:
                        raise ValueError(
                            f"{segment} offset {offset:#x} maps outside image "
                            f"at file offset {file_offset:#x}"
                        )
                    write_bytes(lines, data)
            current = next_offset
        lines.append(f"{segment} ends")
        if runtime_layout and segment == "GAME_DATA":
            lines.append("DGROUP group GAME_DATA")
        lines.append("")
    lines.append("end")
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def runtime_rebindings(
    entries: list[Declaration],
    image: bytes | None,
    runtime_layout: bool,
) -> tuple[PointerRebinding, ...]:
    if not runtime_layout:
        return ()
    if image is None:
        raise ValueError("runtime pointer rebindings require the original image")
    declarations_by_symbol = {entry.symbol: entry for entry in entries}
    result = []
    for rebinding in RUNTIME_POINTER_REBINDINGS:
        declaration = declarations_by_symbol.get(rebinding.symbol)
        if declaration is None:
            continue
        if (declaration.segment, declaration.offset) != (
            rebinding.segment,
            rebinding.offset,
        ):
            raise ValueError(
                f"{rebinding.symbol} declaration moved from "
                f"{rebinding.segment}:{rebinding.offset:#06x}"
            )
        if len(rebinding.original_offsets) != len(rebinding.targets):
            raise ValueError(f"{rebinding.symbol} has an invalid rebinding inventory")
        original = struct.unpack_from(
            f"<{len(rebinding.targets)}H",
            image,
            rebinding.original_file_offset,
        )
        if original != rebinding.original_offsets:
            raise ValueError(
                f"{rebinding.symbol} original pointer words changed: "
                f"{original!r} != {rebinding.original_offsets!r}"
            )
        result.append(rebinding)
    return tuple(result)


def write_rebinding_report(
    path: Path,
    rebindings: tuple[PointerRebinding, ...],
) -> None:
    with path.open("w", encoding="ascii", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("table", "index", "original_offset", "linked_symbol"))
        for rebinding in rebindings:
            for index, (original, target) in enumerate(
                zip(rebinding.original_offsets, rebinding.targets)
            ):
                writer.writerow(
                    (rebinding.symbol, index, f"0x{original:04x}", target)
                )


def main() -> int:
    args = parse_args()
    if args.image is not None and not args.image.is_file():
        raise SystemExit(f"BLOODPRG image does not exist: {args.image}")
    known = declarations(args.header_dir.resolve())
    symbols = read_symbols(args.unresolved.resolve())
    entries = [known[symbol] for symbol in symbols if symbol in known]
    unknown = [symbol for symbol in symbols if symbol not in known]
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    report = output_dir / "data_layout.tsv"
    with report.open("w", encoding="ascii", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("symbol", "status", "segment", "offset", "header"))
        for entry in sorted(entries, key=lambda item: (item.segment, item.offset, item.symbol)):
            writer.writerow((entry.symbol, "known", entry.segment, f"0x{entry.offset:04x}", entry.header))
        for symbol in sorted(unknown):
            writer.writerow((symbol, "unknown", "", "", ""))
    asm = output_dir / "bloodprg_data_layout_probe.asm"
    image = args.image.resolve().read_bytes() if args.image else None
    rebindings = runtime_rebindings(entries, image, args.runtime_layout)
    write_rebinding_report(output_dir / "pointer_rebindings.tsv", rebindings)
    write_asm(
        asm,
        entries,
        image,
        {
            "_CODE": args.code_file_base,
            "GAME_DATA": args.game_data_file_base,
            "FS_DATA": args.fs_data_file_base,
        },
        args.runtime_layout,
        rebindings,
    )
    known_count = len(entries)
    print(f"known data declarations: {known_count}/{len(symbols)}")
    print(f"unknown symbols: {len(unknown)}")
    print(
        "runtime pointer rebindings: "
        f"{sum(len(rebinding.targets) for rebinding in rebindings)}"
    )
    print(f"wrote {report}")
    print(f"wrote {asm}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
