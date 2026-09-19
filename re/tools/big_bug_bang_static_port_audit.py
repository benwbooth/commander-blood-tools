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
ROUTINES = (
    (0x49B3, 0x4B33, 0x4536, "sprite"),
    (0x4B39, 0x5025, 0x46BC, "sprite"),
    (0x5025, 0x5153, 0x4BA8, "sprite"),
    (0x5153, 0x53DF, 0x4CD6, "sprite"),
    (0x53DF, 0x5517, 0x4F62, "sprite"),
    (0x8923, 0x8987, 0x78D0, "hover"),
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
    memory = SPRITE_MEMORY if kind == "sprite" else HOVER_MEMORY
    immediates = SPRITE_IMMEDIATES if kind == "sprite" else HOVER_IMMEDIATES
    instructions = decode(original, old)
    patches = []
    transformed = bytearray(original)
    for ins in instructions:
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
                "relocations. Relative branches, constants, register widths, memory "
                "addressing, clipping, traversal, and return are unchanged."
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
