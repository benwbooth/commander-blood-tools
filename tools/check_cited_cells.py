#!/usr/bin/env python3
"""Does every DS cell the port cites in PROSE actually get touched by the game?

`check_cited_instructions.py` verifies an address only when a MNEMONIC is quoted
beside it (`` `mov ax, word ptr [0xa2a]` @0x8271 ``). Addresses named in prose --
"rebases `gs:0x2A2A` against `gs:0x27A7`" -- are unchecked, and audit-fixes #433
found one of them wrong by a single hex digit: the instruction reads `0x0A2A`, and
`0x2A2A` survived review because it looks like a plausible cell and sits beside
`0x2A19`/`0x2A1B`, two addresses that ARE real.

So this asks the weaker but automatable question: is the cited cell touched by ANY
instruction in the image? A DS offset that nothing reads or writes is not
necessarily wrong -- it may be runtime-only state, like the overlay tables in #406
that are zero in the file and filled at load -- but it is worth a look, and a
citation nothing corroborates is exactly what #433 was.

Reported as UNTOUCHED (no site found) with the file and line, so each can be
judged. Cells that ARE touched are counted only.

Uses `re/tools/addr_forms.py`'s census, which enumerates the direct-address
encodings (`A0..A3`, `80/81/83`, `C6/C7`, `F6/F7`, `88..8B`, `FF`) rather than
grepping for bytes -- audit-fixes #335/#359/#403 all record one-encoding scans
under-reporting.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "re", "tools"))

from addr_forms import census, reg_disp_census

BIN = os.path.join("re", "bin", "BLOODPRG.EXE")
# `gs:0x27A7`, `DS:0x5251`, `gs:[0x2793]` -- a SEGMENT-QUALIFIED cell reference.
# Bare `0x9722` is skipped: those are code addresses, a different question.
CELL = re.compile(r"\b(?:gs|ds|fs|es|DS|GS|FS|ES)\s*:\s*\[?\s*(0x[0-9A-Fa-f]{2,4})\b")


def main():
    if not os.path.exists(BIN):
        print("no image; nothing checked")
        return 0
    data = open(BIN, "rb").read()

    cited = {}
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            for i, line in enumerate(open(path, encoding="utf-8", errors="replace")):
                for m in CELL.finditer(line):
                    cited.setdefault(int(m.group(1), 16), []).append(f"{path}:{i+1}")

    touched, untouched = 0, []
    for addr, where in sorted(cited.items()):
        hits = census(data, addr)
        if not hits:
            # A cell reached through a base register (`[bx+0x6D60]`) has no
            # direct-address form.
            hits = reg_disp_census(data, addr) if addr <= 0xFFFF else {}
        if not hits:
            # reg_disp_census enumerates OPCODES, and missed `8A` (byte loads):
            # `65 8a 87 60 6d` -- `gs: mov al,[bx+0x6D60]` @0x6023, the
            # vm_field_offset matrix -- was reported UNTOUCHED. Rather than chase
            # opcodes one family at a time (the blind spot of #335/#359/#403),
            # match on the MODRM instead: any mod=10 byte immediately before the
            # little-endian address is a reg+disp16 access, whatever the opcode.
            #
            # The same applies to the DIRECT-address forms: `les di, gs:[0x6724]`
            # is `65 c4 3e 24 67`, and addr_forms' table has no C4/C5 (les/lds),
            # so the VM's record-table pointer -- read at 0x6B4D and a dozen other
            # places -- also came back UNTOUCHED. A direct address is modrm
            # mod=00, rm=110, whatever the opcode.
            le = addr.to_bytes(2, "little")
            at = data.find(le)
            while at > 0:
                modrm = data[at - 1]
                if modrm & 0xC0 == 0x80 or modrm & 0xC7 == 0x06:
                    kind = "reg+disp16" if modrm & 0xC0 == 0x80 else "direct"
                    hits = {at: (kind, "R", None)}
                    break
                at = data.find(le, at + 1)
        if hits:
            touched += 1
        else:
            untouched.append((addr, where))

    for addr, where in untouched:
        first = where[0]
        more = f" (+{len(where)-1} more)" if len(where) > 1 else ""
        print(f"UNTOUCHED {addr:#06x} cited at {first}{more}")

    print(
        f"{touched + len(untouched)} distinct segment-qualified cells cited; "
        f"{touched} are touched by an instruction, {len(untouched)} are not"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
