#!/usr/bin/env python3
"""Check reviewed BBB-to-Commander body equivalences, not fuzzy similarity."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path

from capstone import Cs, CS_ARCH_X86, CS_MODE_16
from capstone.x86 import X86_OP_IMM, X86_OP_MEM

ROOT = Path(__file__).resolve().parents[2]
BBB = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
COMMANDER = ROOT / "re/bin/BLOODPRG.EXE"
PORTED = ROOT / "re/rust-port/ported.tsv"
OUTPUT = ROOT / "re/big_bug_bang_static_port_audit.json"
HOVER = "crates/commander-blood-game/src/native/bloodprg/presentation_hover.rs"
SERVICES = "crates/commander-blood-game/src/runtime/services.rs"

# These are individual reviewed identities, never a blanket displacement delta.
# CS flip bytes are written by BBB 0x4944/0x494C; the other CS slots are local
# stride/run scratch. GS pointers become owned framebuffers and remap slices.
SPRITE_MEMORY = {
    ("cs", 0x14DF): (0x15DC, "horizontal_flip"),
    ("cs", 0x14E0): (0x15DD, "vertical_flip"),
    ("cs", 0x15A4): (0x16A1, "raw_source_row_stride"),
    ("cs", 0x1726): (0x1823, "rle_source_row_width"),
    ("cs", 0x1728): (0x1825, "rle_left_clip"),
    ("cs", 0x172A): (0x1827, "rle_right_clip"),
    ("gs", 0x5221): (0x55F1, "drawing_framebuffer"),
    ("gs", 0x524B): (0x561B, "selected_remap_table"),
}
SPRITE_IMMEDIATES = {
    ("mov", "bx", 0x5F11): (0x62E1, "first_remap_table"),
    ("mov", "bx", 0x6011): (0x63E1, "second_remap_table"),
}
HOVER_MEMORY = {
    ("", 0x2793): (0x2A33, "presentation_mode_bits"),
    ("", 0x0A2A): (0x0C22, "pointer_x"),
    ("", 0x0A2C): (0x0C24, "pointer_y"),
    ("", 0x27EA): (0x2A8B, "hover_latch"),
    ("", 0x0A32): (0x0C2A, "actor_state"),
    ("", 0x0A36): (0x0C2E, "previous_actor_state"),
}
HOVER_IMMEDIATES = {
    ("mov", "bp", 0x2A27): (0x2CC7, "primary_actor_rectangle"),
}
CD_MEMORY = {
    ("gs", 0x0AE6): (0x0CEF, "cd_audio_available"),
    ("gs", 0x01B9): (0x0205, "cd_drive_number"),
    ("", 0x01B9): (0x0205, "cd_drive_number"),
    ("gs", 0x0B6C): (0x0D76, "requested_physical_track"),
    ("", 0x0B6D): (0x0D77, "packed_track_start"),
    ("", 0x0B5E): (0x0D68, "packed_disc_leadout"),
}
CD_IMMEDIATES = {
    ("mov", "bx", 0x0B41): (0x0D4B, "cd_ioctl_request"),
    ("mov", "bx", 0x0B72): (0x0D7C, "cd_playback_request"),
}
COPY_MEMORY = {
    ("gs", 0x5219): (0x55E9, "back_framebuffer"),
    ("gs", 0x252E): (0x2780, "cropped_scene_gate"),
    ("gs", 0x2527): (0x2779, "ship_depth_crop"),
}
SUBTITLE_MEMORY = {
    ("", 0x0F18): (0x1166, "sequence_subtitle_cursor"),
    ("", 0x131C): (0x156A, "visible_video_frame"),
}
NOISE_MEMORY = {
    ("", 0x5235): (0x5605, "clip_left"),
    ("", 0x5237): (0x5607, "clip_right"),
    ("", 0x5239): (0x5609, "clip_top"),
    ("", 0x5221): (0x55F1, "drawing_framebuffer"),
}
MEMORY_MAPS = {
    "sprite": SPRITE_MEMORY,
    "hover": HOVER_MEMORY,
    "cd": CD_MEMORY,
    "copy": COPY_MEMORY,
    "subtitle": SUBTITLE_MEMORY,
    "noise": NOISE_MEMORY,
}
IMMEDIATE_MAPS = {
    "sprite": SPRITE_IMMEDIATES,
    "hover": HOVER_IMMEDIATES,
    "cd": CD_IMMEDIATES,
    "copy": {},
    "noise": {},
    "subtitle": {("mov", "bp", 0x0AF2): (0x0CFC, "subtitle_line_buffer")},
}
# Memory-destination immediates are approved at their exact instruction sites.
SITE_IMMEDIATES = {
    0x1362: (0x0B5B, 0x0D65, "disc_information_transfer"),
    0x136F: (0x0B6B, 0x0D75, "track_information_transfer"),
    0x1383: (0x0B62, 0x0D6C, "channel_mix_transfer"),
}
FAR_CALLS = {
    (0x299, 0x00D6): (0x2B1, 0x00D6, "draw_bios_font_line", 0x33E6),
    (0x1CE, 0x0B02): (0x1E6, 0x0B03, "random_word", 0x3163),
}
ROUTINES = (
    (0x1502, 0x1555, 0x1344, "cd"),
    (0x1582, 0x163D, 0x13C4, "cd"),
    (0x4002, 0x40E9, 0x3B85, "noise"),
    (0x434B, 0x43E4, 0x3ECE, "copy"),
    (0x49B3, 0x4B33, 0x4536, "sprite"),
    (0x4B39, 0x5025, 0x46BC, "sprite"),
    (0x5025, 0x5153, 0x4BA8, "sprite"),
    (0x5153, 0x53DF, 0x4CD6, "sprite"),
    (0x53DF, 0x5517, 0x4F62, "sprite"),
    (0x8923, 0x8987, 0x78D0, "hover"),
    (0x8DBC, 0x8E4F, 0x7CE8, "subtitle"),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def decode(body: bytes, address: int):
    decoder = Cs(CS_ARCH_X86, CS_MODE_16)
    decoder.detail = True
    instructions = list(decoder.disasm(body, address))
    if sum(item.size for item in instructions) != len(body):
        raise ValueError(f"incomplete instruction decode at {address:#x}")
    return instructions


def compare_body(original: bytes, sequel: bytes, old: int, new: int, kind: str):
    """Require byte identity after only specifically approved address patches."""
    memory = MEMORY_MAPS[kind]
    immediates = IMMEDIATE_MAPS[kind]
    instructions = decode(original, old)
    patches = []
    transformed = bytearray(original)
    for ins in instructions:
        if ins.mnemonic == "lcall":
            target = tuple(operand.imm for operand in ins.operands)
            if target not in FAR_CALLS:
                raise ValueError(f"unreviewed far call at {ins.address:#x}")
            segment, offset, identity, entry = FAR_CALLS[target]
            start = ins.address - old
            if ins.size != 5 or ins.bytes[0] != 0x9A:
                raise ValueError("unexpected far-call encoding")
            transformed[start + 1 : start + 5] = offset.to_bytes(
                2, "little"
            ) + segment.to_bytes(2, "little")
            patches.append(
                {
                    "commander_site": f"0x{ins.address:04x}",
                    "bbb_site": f"0x{new + ins.address - old:04x}",
                    "identity": identity,
                    "callee": f"0x{entry:04x}",
                }
            )
            continue
        for operand in ins.operands:
            replacement = None
            if operand.type == X86_OP_MEM:
                mem = operand.mem
                if not mem.base and not mem.index:
                    replacement = memory.get(
                        (ins.reg_name(mem.segment) or "", mem.disp)
                    )
                offset, size = ins.disp_offset, ins.disp_size
            elif operand.type == X86_OP_IMM and len(ins.operands) == 2:
                destination = ins.reg_name(ins.operands[0].reg)
                replacement = immediates.get((ins.mnemonic, destination, operand.imm))
                if ins.address in SITE_IMMEDIATES:
                    expected, value, identity = SITE_IMMEDIATES[ins.address]
                    if operand.imm != expected:
                        raise ValueError("unexpected transfer-pointer immediate")
                    replacement = (value, identity)
                offset, size = ins.imm_offset, ins.imm_size
            else:
                continue
            if replacement is None:
                continue
            value, identity = replacement
            if size != 2:
                raise ValueError(f"unexpected relocation width at {ins.address:#x}")
            start = ins.address - old + offset
            transformed[start : start + size] = value.to_bytes(size, "little")
            patches.append(
                {
                    "commander_site": f"0x{ins.address:04x}",
                    "bbb_site": f"0x{new + ins.address - old:04x}",
                    "identity": identity,
                }
            )
    if bytes(transformed) != sequel:
        raise ValueError(f"unreviewed instruction change at BBB {new:#x}")
    return instructions, patches


def build_report():
    bbb, commander = BBB.read_bytes(), COMMANDER.read_bytes()
    if (
        sha256(bbb)
        != "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
    ):
        raise ValueError("unsupported BBB executable")
    if (
        sha256(commander)
        != "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
    ):
        raise ValueError("unsupported Commander executable")
    with PORTED.open(newline="") as stream:
        ported = {
            int(row["entry"], 16): row
            for row in csv.DictReader(stream, delimiter="\t")
            if row["component"] == "bloodprg"
        }
    inputs = {
        str(path.relative_to(ROOT)): sha256(path.read_bytes())
        for path in (BBB, COMMANDER, PORTED, Path(__file__).resolve())
    }
    initializers = []
    for old_offset, new_offset, length, identity in (
        (0x0B41, 0x0D4B, 26, "cd_ioctl_request"),
        (0x0B5B, 0x0D65, 7, "disc_information_transfer"),
        (0x0B62, 0x0D6C, 9, "channel_mix_transfer"),
        (0x0B6B, 0x0D75, 7, "track_information_transfer"),
        (0x0B72, 0x0D7C, 22, "cd_playback_request"),
    ):
        # MZ entry loads DS=GS=0xCE2 / 0xEFF, after 0x600 / 0x800 headers.
        original = commander[0xD420 + old_offset : 0xD420 + old_offset + length]
        native = bbb[0xF7F0 + new_offset : 0xF7F0 + new_offset + length]
        if original != native:
            raise ValueError(f"changed initializer for {identity}")
        initializers.append({"identity": identity, "bytes": native.hex()})
    rows = []
    for entry, end, old, kind in ROUTINES:
        native = bbb[entry:end]
        original = commander[old : old + len(native) - (7 if kind == "hover" else 0)]
        source = ported[old]
        owner = {"path": source["rust_path"], "symbol": source["rust_symbol"]}
        notes = [source["documentation"]]
        if kind == "hover":
            # The only inserted instructions test overview bit 0 and jump to
            # the original pop-bp/ret. The complete remaining suffix is checked.
            if native[:8].hex() != "55f606302a01755a" or native[-2:] != b"\x5d\xc3":
                raise ValueError("BBB overview guard changed")
            if original[:1] != b"\x55":
                raise ValueError("Commander hover prologue changed")
            instructions, patches = compare_body(
                original[1:], native[8:], old + 1, entry + 8, kind
            )
            owner = {"path": HOVER, "symbol": "update_sequel_presentation_hover"}
            inputs[SERVICES] = sha256((ROOT / SERVICES).read_bytes())
            notes.append(
                "BBB overview guard returns without touching hover or actor state; "
                "runtime supplies sequel_overview.active(). Signed inclusive bounds, "
                "selection priority, actor 9, and previous-state restore are unchanged."
            )
        else:
            instructions, patches = compare_body(original, native, old, entry, kind)
            notes.append(
                "Complete body is byte-identical after only the named state/table "
                "and callee relocations. Relative branches, non-address constants, "
                "register widths, memory addressing, traversal, and return are unchanged."
            )
        if f"fn {owner['symbol']}" not in (ROOT / owner["path"]).read_text():
            raise ValueError(f"missing Rust owner {owner}")
        fixture = source["evidence"].split(":", 1)[0]
        inputs[owner["path"]] = sha256((ROOT / owner["path"]).read_bytes())
        inputs[fixture] = sha256((ROOT / fixture).read_bytes())
        rows.append(
            {
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "commander_entry": f"0x{old:04x}",
                "kind": kind,
                "body_sha256": sha256(native),
                "rust_owner": owner,
                "inherited_fixture": fixture,
                "instruction_count": len(instructions),
                "reviewed_relocations": patches,
                "notes": notes,
            }
        )
    return {
        "format": "big_bug_bang_static_port_audit_v1",
        "scope": "Reviewed native equivalence plus Rust ownership; not a new native "
        "execution claim or a proof of whole-game runtime wiring.",
        "inputs": inputs,
        "identical_initializers": initializers,
        "routines": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUTPUT)
    args = parser.parse_args()
    report = build_report()
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"audited {len(report['routines'])} BBB native bodies against typed Rust owners"
    )


if __name__ == "__main__":
    main()
