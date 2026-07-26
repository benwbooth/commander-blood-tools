#!/usr/bin/env python3
"""Does each code label sit on an INSTRUCTION BOUNDARY?

audit-fixes #386 found `console_menu_hit_test` recorded at `0x8613`, which is the
last byte of the `jne 0x86F1` at `0x8610`. The block really starts at `0x8614`.
Decoding from the wrong address renders `add byte [bx+di+0x2795], ah`, a phantom
that swallows the `a1` of the real `mov ax,[0x2795]` -- so anyone checking a
citation there sees an instruction that does not exist. The error had propagated
to SEVEN citation sites in the port before anything noticed, because
`check_labels.py` verifies a label's CONTENT claims but never asks whether its
ADDRESS is decodable in the first place.

METHOD: for each code label, decode linearly from the nearest preceding code label
and see whether any instruction starts exactly at this one. Three outcomes:

  MISALIGNED  an instruction STRADDLES the label -- it starts before and ends
              after. This is the #386 shape and is the only bucket worth acting
              on directly.
  UNREACHED   the linear decode never got there (it desynced, hit an invalid
              byte, or ran through embedded DATA -- the dispatch table at 0x8709
              is exactly this). NOT evidence of an error; the decode simply
              cannot answer, and saying so is better than a confident guess.
  ok          an instruction starts exactly at the label.

The UNREACHED bucket is expected to be large: this binary interleaves jump tables
and record arrays with code, and a linear sweep has no way to know. A checker that
reported those as problems would be the "confident tool that is wrong" failure this
project keeps hitting, so they are counted and NOT listed unless asked.

Usage (PYTHONSAFEPATH=1, from the repo root):

    python3 re/tools/check_label_alignment.py            summary + MISALIGNED
    python3 re/tools/check_label_alignment.py --unreached  also list UNREACHED
"""

import csv
import sys

# NOTE: do NOT put re/tools on sys.path here. This directory contains dis.py, and
# inserting it shadows the STDLIB `dis` module that capstone imports (via
# inspect), which fails at import with a confusing AttributeError. Same shadowing
# trap as audit-fixes #359's encodings.py. Nothing in this tool needs mzfile.
from capstone import CS_ARCH_X86, CS_MODE_16, Cs

LABELS = "re/labels.csv"
BIN = "re/bin/BLOODPRG.EXE"
# Beyond this, a linear decode from the previous label has almost certainly
# desynced through data and its opinion is worthless.
MAX_SPAN = 4096


# Rows that name DATA rather than code are not expected to decode at all --
# `vm_opcode_lengths`, `script_basenames`, `scrut_var_lists` were all reported as
# MISALIGNED on the first run, which is meaningless for a table. Same heuristic
# check_labels.py already uses for the same reason.
DATA_HINT = __import__("re").compile(
    r"\b(table|map|buffer|array|list|string|glyph|palette|record|font|data|"
    r"vertices|advances|offsets|entries|names?|lengths)\b",
    __import__("re").I,
)


def code_labels():
    out = []
    with open(LABELS, newline="") as fh:
        for row in csv.reader(fh):
            if not row or not row[0].startswith("0x") or ":" in row[0]:
                continue
            try:
                addr = int(row[0], 16)
            except ValueError:
                continue
            name = row[1] if len(row) > 1 else "?"
            comment = row[2] if len(row) > 2 else ""
            if DATA_HINT.search(name) or DATA_HINT.search(comment[:120]):
                continue
            out.append((addr, name))
    return sorted(set(out))


def main():
    data = open(BIN, "rb").read()
    md = Cs(CS_ARCH_X86, CS_MODE_16)
    labels = code_labels()

    misaligned, unreached, ok = [], [], 0
    for i, (addr, name) in enumerate(labels):
        if i == 0:
            continue
        prev_addr, _ = labels[i - 1]
        span = addr - prev_addr
        if span <= 0 or span > MAX_SPAN or addr >= len(data):
            unreached.append((addr, name, "no usable predecessor"))
            continue

        starts = set()
        straddle = None
        for ins in md.disasm(data[prev_addr : addr + 16], prev_addr):
            starts.add(ins.address)
            if ins.address < addr < ins.address + ins.size:
                straddle = ins
                break
            if ins.address > addr:
                break
        if addr in starts:
            ok += 1
        elif straddle is not None:
            misaligned.append(
                (addr, name, f"{straddle.address:#07x} {straddle.mnemonic} spans it")
            )
        else:
            unreached.append((addr, name, "decode desynced or hit data"))

    # SECOND SIGNAL. A straddle alone does not distinguish a genuinely misplaced
    # label from a linear decode that desynced through data on the way. But an
    # address something BRANCHES TO must be an instruction boundary, whatever the
    # sweep thinks. So collect every relative-branch target in the image and use
    # it to rescue candidates. (Scanning relative branches over data invents
    # targets too, so a rescue is evidence of a desync, not proof of one -- hence
    # a separate bucket rather than a silent drop.)
    targets = set()
    for i in range(len(data) - 4):
        op = data[i]
        if op in (0xE8, 0xE9):  # call/jmp rel16
            rel = int.from_bytes(data[i + 1 : i + 3], "little", signed=True)
            targets.add(i + 3 + rel)
        elif op == 0xEB or 0x70 <= op <= 0x7F:  # jmp short / jcc rel8
            rel = int.from_bytes(data[i + 1 : i + 2], "little", signed=True)
            targets.add(i + 2 + rel)
        elif op == 0x0F and 0x80 <= data[i + 1] <= 0x8F:  # jcc rel16
            rel = int.from_bytes(data[i + 2 : i + 4], "little", signed=True)
            targets.add(i + 4 + rel)
        elif op in (0x9A, 0xEA) and i + 5 <= len(data):  # FAR call/jmp seg:off
            # Required, not optional: dlg_line_id_scene_dispatch (0x9D10) is
            # reached only by a far call from 0x1EDD, so a relative-branch-only
            # scan reports a correct label as MISALIGNED. Stored segments are
            # image-relative, so file = 0x600 + seg*16 + off (re/CLAUDE.md).
            off = int.from_bytes(data[i + 1 : i + 3], "little")
            seg = int.from_bytes(data[i + 3 : i + 5], "little")
            targets.add(0x600 + seg * 16 + off)

    rescued = [c for c in misaligned if c[0] in targets]
    misaligned = [c for c in misaligned if c[0] not in targets]

    for addr, name, why in misaligned:
        print(f"MISALIGNED {addr:#07x} {name} — {why}, and NOTHING branches to it")
    if "--rescued" in sys.argv:
        for addr, name, why in rescued:
            print(f"DESYNC?    {addr:#07x} {name} — {why}, but it IS a branch target")
    if "--unreached" in sys.argv:
        for addr, name, why in unreached:
            print(f"UNREACHED  {addr:#07x} {name} — {why}")

    print(
        f"{ok} label(s) land on an instruction boundary, "
        f"{len(misaligned)} MISALIGNED, {len(rescued)} straddled-but-branch-target, "
        f"{len(unreached)} unreachable by linear decode (data/desync — not errors)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
