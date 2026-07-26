#!/usr/bin/env python3
"""Validate re/labels.csv against the binary.

labels.csv is what every future decode session reads first, so an error in it
propagates into work that never re-derives the claim. Three checks:

1. every flat `0xNNNNN` address is inside the image;
2. it decodes to a valid instruction (capstone yields one, and its length does not
   run past the image);
3. when the comment OPENS with a quoted instruction (`\"mov ax,...\"`, `cmp ...`),
   the mnemonic at that address matches.

DS:/GS:/FS: rows name data, not code, so only their range is checked.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import csv
import os
import collections
import re
import sys

# capstone BEFORE re/tools joins sys.path: that directory has a `dis.py` which
# shadows the stdlib `dis` capstone needs via `inspect`.
import capstone  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from mzfile import DEFAULT_BIN, LABELS_CSV  # noqa: E402

DS_BASE = 0xD420
MNEMONICS = {
    "mov", "cmp", "test", "jmp", "je", "jne", "ja", "jae", "jb", "jbe", "jg",
    "jge", "jl", "jle", "js", "jns", "jcxz", "call", "lcall", "ret", "retf",
    "push", "pop", "add", "sub", "adc", "sbb", "and", "or", "xor", "not", "neg",
    "inc", "dec", "shl", "shr", "sar", "rol", "ror", "mul", "imul", "div", "idiv",
    "lea", "les", "lds", "lfs", "lgs", "lodsb", "lodsw", "stosb", "stosw",
    "xlatb", "clc", "stc", "cwde", "loop", "int", "bsf", "btr", "xchg", "nop",
    # audit-fixes #350: `out`/`in` were absent, so a label quoting them was never
    # checked at all -- which is how #349's `out 0x42` (the PC SPEAKER) survived
    # over code that does `out 0x40` (the system TIMER).
    "out", "in", "cli", "sti", "setne", "sete", "movsx", "movzx", "cbw", "stosd",
    "lodsd", "movsd", "rep", "cwd", "cdq", "sar",
}

# PORT NUMBERS ARE OPERANDS, and mnemonic-only checking cannot see them: both
# `out 0x42,al` and `out 0x40,al` are `out`. #349 was exactly that -- one hex
# digit, timer vs speaker, in a label that read as authoritative. So when a
# comment names a port explicitly, compare it to the instruction's own operand.
PORT_CLAIM = re.compile(r"\b(out|in)\s+(?:al\s*,\s*)?(0x[0-9a-f]{1,4})", re.I)
# INT NUMBERS are the same class of operand claim as ports, and labels write them
# in both DOS conventions -- `int 21h` and `int 0x21` (audit-fixes #351).
INT_CLAIM = re.compile(r"\bint\s*(?:0x([0-9a-f]{1,2})\b|([0-9a-f]{1,2})h\b)", re.I)
# A comment only CLAIMS an instruction when the mnemonic is followed by something
# operand-shaped. "jmp target after early init" describes the address as a jump
# TARGET -- reading its first word as a quoted opcode reported a correct label as
# wrong.
LEAD = re.compile(
    r"^[`\"\']?([a-z]{2,6})\s+(?:byte|word|dword|ptr|[a-z]{2}\b|\[|0x|-?\d)"
)
# Flat rows that name DATA rather than code: they are not expected to decode.
# Comments also quote OTHER addresses inline -- "`inc word [eax+edi]` @0x5DCE",
# "0x91DB cmp word [si+0x36],0". Those are checkable too, and there are far more
# of them than there are comments that OPEN with an instruction.
# The instruction must be BACKTICK-QUOTED. Matching bare words next to an address
# reported 17 "problems", every one of them English: comments say "gs:[0x523B] and
# the clip", "[0x250B] or its fallback". `and`, `or`, `not`, `sub`, `add`, `test`,
# `in` and `int` are all ordinary words as well as mnemonics, so prose adjacency
# proves nothing.
INLINE = re.compile(r"`([a-z]{2,6})\s[^`]*`\s*@?\s*(0x[0-9A-Fa-f]{4,5})")
# The OTHER order, which this file uses at least as often: the address first, then
# the instruction -- "0x91DB cmp word [si+0x36],0", "the walker at 0x7DAB is
# `mov bp,0x2A1B`". Only 9 of 568 code labels were being checked because the
# instruction-then-address form is the rarer one.
#
# The mnemonic MUST still be backticked. Dropping that -- on the theory that a
# preceding address anchors it well enough -- reproduced precisely the false
# positives this file already warned about: four of five first-run "problems" were
# `and` in prose ("DS:0x2578 and the ..."), against DATA addresses that are not
# instructions at all. The address does not make the next word an opcode; the
# backticks are what mark a QUOTE.
INLINE_ADDR_FIRST = re.compile(
    r"(0x[0-9A-Fa-f]{4,5})\s+(?:is\s+)?`([a-z]{2,6})\s[^`]*`"
)
DATA_HINT = re.compile(
    r"\b(table|map|buffer|array|list|string|glyph|palette|record|font|data|vertices|"
    r"advances|offsets|entries)\b",
    re.I,
)


def main():
    data = open(DEFAULT_BIN, "rb").read()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    rows = list(csv.reader(open(LABELS_CSV, newline="")))
    code = data_rows = quoted = 0
    inline_checked = [0]
    port_checked = [0]
    int_checked = [0]
    problems = []
    seen_rows = []
    for line_no, row in enumerate(rows, 1):
        if not row or not row[0] or row[0].startswith("#"):
            continue
        addr_s, name = row[0].strip(), (row[1].strip() if len(row) > 1 else "")
        comment = row[2] if len(row) > 2 else ""
        seg = None
        if ":" in addr_s:
            seg, off = addr_s.split(":", 1)
            if seg in ("DS", "GS", "FS", "ES", "SS"):
                try:
                    off = int(off, 16)
                except ValueError:
                    continue
                data_rows += 1
                if not (0 <= DS_BASE + off < len(data)):
                    problems.append(f"{line_no}: {name} {addr_s} outside the image")
            continue
        try:
            addr = int(addr_s, 16)
        except ValueError:
            continue
        seen_rows.append((line_no, name, f"{addr:#07x}"))
        if addr >= len(data):
            problems.append(f"{line_no}: {name} {addr_s} outside the image")
            continue
        code += 1
        insn = next(md.disasm(data[addr:addr + 16], addr), None)
        if insn is None:
            if not (DATA_HINT.search(name) or DATA_HINT.search(comment)):
                problems.append(f"{line_no}: {name} {addr_s} does not decode")
            continue
        m = LEAD.match(comment.strip().lower())
        # `out`/`in` are EXEMPT from the opening-mnemonic check (audit-fixes
        # #350). A port write is nearly always a PAIR -- `mov ax,0xf02` then
        # `out dx,ax` -- and a label naturally describes the pair by its effect
        # ("out 0x3C4, ax=0x0F02") while pointing at the VALUE LOAD, which is the
        # instruction a reader needs to find. Adding these mnemonics immediately
        # flagged `ship_3d_plane_copy_mapmask_all`, whose label is exactly right.
        # The PORT-NUMBER check below still applies to them, and that is the part
        # that catches #349's timer-for-speaker error.
        if m and m.group(1) in MNEMONICS and m.group(1) not in ("out", "in"):
            quoted += 1
            want = m.group(1)
            got = insn.mnemonic
            if got != want and not (want in ("call", "jmp") and got in ("lcall", "ljmp")):
                problems.append(
                    f"{line_no}: {name} {addr_s} comment opens with `{want}` "
                    f"but the code is `{got} {insn.op_str}`"
                )
        # PORT-NUMBER claims: `out 0xNN` / `in al,0xNN` against the real operand.
        # SET MEMBERSHIP, not pairwise. A routine's label routinely names SEVERAL
        # ports -- `program_pit` cites both 0x43 (mode) and 0x40 (divisor), and
        # `cmos_rtc_read` both 0x70 (select) and 0x71 (read) -- so pairing the
        # first claim with the first I/O instruction reports correct labels as
        # wrong, which is what it did on its first run (audit-fixes #350).
        claims = {int(p, 16) for _, p in PORT_CLAIM.findall(comment.lower())}
        if claims:
            used = set()
            for i in md.disasm(data[addr:addr + 48], addr):
                if i.mnemonic in ("out", "in"):
                    for n in re.findall(r"0x[0-9a-f]+", i.op_str):
                        used.add(int(n, 16))
                if i.mnemonic in ("ret", "retf"):
                    break
            # Only judge when the window actually performs immediate-port I/O;
            # a DX-port form carries no immediate to compare against.
            if used:
                for c in sorted(claims):
                    port_checked[0] += 1
                    if c not in used:
                        problems.append(
                            f"{line_no}: {name} names port {c:#04x} but the routine's "
                            f"I/O uses {{{', '.join(f'{u:#04x}' for u in sorted(used))}}}"
                        )

        # INT-NUMBER claims, same set-membership rule as ports.
        int_claims = {
            int(a or b, 16) for a, b in INT_CLAIM.findall(comment.lower())
        }
        if int_claims:
            used_int = set()
            for i in md.disasm(data[addr:addr + 64], addr):
                if i.mnemonic == "int":
                    for n in re.findall(r"0x[0-9a-f]+|\b\d+\b", i.op_str):
                        used_int.add(int(n, 16) if n.startswith("0x") else int(n))
                if i.mnemonic in ("ret", "retf"):
                    break
            # A VECTOR NUMBER IS NOT AN ISSUED INTERRUPT. `install_timer_isr_hook`
            # and `program_pit` legitimately name "int 08h" while issuing only
            # int 21h, because they SET or GET that vector through DOS functions
            # 25h/35h. Suppress the claim whenever the routine loads AH with
            # either (audit-fixes #351) -- otherwise the check punishes the most
            # precise labels in the file.
            # BOTH ENCODINGS. `install_timer_isr_hook` writes `mov ah,0x25`;
            # `program_pit` writes `mov ax,0x2508`, packing the function AND the
            # vector number into one word load. Matching only the first form left
            # a correct label flagged (audit-fixes #351).
            vector_api = any(
                i.mnemonic == "mov"
                and re.match(r"^a[hx], 0x(25|35)([0-9a-f]{2})?$", i.op_str)
                for i in md.disasm(data[addr:addr + 64], addr)
            )
            if used_int and not vector_api:
                for c in sorted(int_claims):
                    int_checked[0] += 1
                    if c not in used_int:
                        problems.append(
                            f"{line_no}: {name} names int {c:#04x} but the routine "
                            f"issues {{{', '.join(f'{u:#04x}' for u in sorted(used_int))}}}"
                        )

        # Inline claims anywhere in the comment, in BOTH orders.
        inline_claims = [(mn, am) for mn, am in INLINE.findall(comment.lower())]
        inline_claims += [
            (mn, am) for am, mn in INLINE_ADDR_FIRST.findall(comment.lower())
        ]
        for mn, am in inline_claims:
            if mn not in MNEMONICS:
                continue
            a = int(am, 16)
            if a >= len(data):
                problems.append(f"{line_no}: {name} cites {am} which is outside the image")
                continue
            ins = next(md.disasm(data[a:a + 16], a), None)
            actual = ins.mnemonic if ins else "<undecodable>"
            inline_checked[0] += 1
            if actual != mn and not (mn in ("call", "jmp") and actual in ("lcall", "ljmp")):
                problems.append(
                    f"{line_no}: {name} says {am} is `{mn}` but it is `{actual}`"
                )

    # Two rows for ONE address, under different names. `0x008709` had
    # `nav_choice_subdispatch_table` and `console_row_handler_table`, added
    # independently for the same table -- the second while re-deriving what the
    # first already recorded (audit-fixes #128). A reader who finds one has no
    # reason to look for the other.
    by_addr = collections.defaultdict(list)
    for line_no, name, addr_s in seen_rows:
        by_addr[addr_s.lower()].append((line_no, name))
    duplicates = []
    for addr_s, rows in sorted(by_addr.items()):
        if len(rows) > 1:
            names = ", ".join(f"{n} (line {ln})" for ln, n in rows)
            duplicates.append(f"{addr_s} has {len(rows)} rows: {names}")
    # Reported, NOT failed. Some pairs record different facets of one address on
    # purpose (`resource_name_table` / `..._extent`); others are genuine
    # rediscovery, like the two names 0x008709 carried. 55 of these accumulated
    # over the campaign and resolving them is its own task, so this counts them
    # rather than blocking on them.
    for d in duplicates:
        print("DUPLICATE ADDRESS " + d)

    for p in problems:
        print("PROBLEM " + p)
    print(
        f"{code} code labels ({quoted} with a quoted opening instruction) and "
        f"{data_rows} data labels checked, {inline_checked[0]} inline address+mnemonic "
        f"claims verified, {len(problems)} problems, {len(duplicates)} duplicate address(es)"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
