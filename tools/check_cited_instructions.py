#!/usr/bin/env python3
"""Verify every `0xNNNN  <mnemonic>` the port QUOTES from the binary.

Doc comments across the port quote the disassembly in the shape

    ///   0x5DB4  mov ax,[si] / cmp ax,1 / jne 0x5DE3   owner kind == 1?

Nothing checked that the byte at `0x5DB4` really decodes to `mov`. A wrong address
in a comment is worse than no comment: it sends the next reader to the wrong
routine and it makes a claim look sourced when it is not. This disassembles each
cited address and compares the mnemonic.

Lines whose "mnemonic" is actually a register (`0x9016  bx = ...`) or prose are
skipped -- only real x86 mnemonics are checked, so the count reported is the
number of claims genuinely verified.
"""

import os
import re
import sys

# Import capstone BEFORE putting re/tools on the path: that directory holds a
# `dis.py`, which shadows the stdlib `dis` that capstone's `inspect` import needs.
# (re/tools/dis.py pops its own directory from sys.path for the same reason.)
import capstone  # noqa: E402

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "re", "tools"))
from mzfile import DEFAULT_BIN  # noqa: E402

MNEMONICS = {
    "mov", "movsb", "movsw", "cmp", "test", "jmp", "je", "jne", "jz", "jnz", "ja",
    "jae", "jb", "jbe", "jg", "jge", "jl", "jle", "js", "jns", "jcxz", "call",
    "lcall", "ret", "retf", "push", "pop", "add", "sub", "adc", "sbb", "and", "or",
    "xor", "not", "neg", "inc", "dec", "shl", "shr", "sal", "sar", "rol", "ror",
    "mul", "imul", "div", "idiv", "lea", "les", "lds", "lfs", "lgs", "lodsb",
    "lodsw", "stosb", "stosw", "stosd", "xlatb", "clc", "stc", "cwde", "cbw",
    "loop", "int", "bsf", "btr", "rep", "sete", "setb", "xchg", "nop", "in", "out",
}
# Aliases capstone prints differently from how a comment might spell them.
ALIAS = {
    "jz": "je", "jnz": "jne", "jc": "jb", "jnc": "jae", "sal": "shl",
    "lcall": "lcall", "call": "call",
}
DOC = re.compile(r"^\s*(?:///|//!)?\s*(0x[0-9A-Fa-f]{4,5})\s+([a-z]{2,7})\b")


def main():
    data = open(DEFAULT_BIN, "rb").read()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    checked = skipped = bad = 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            for i, ln in enumerate(open(path, encoding="utf-8", errors="replace"), 1):
                st = ln.strip()
                if not (st.startswith("///") or st.startswith("//!")):
                    continue
                m = DOC.match(st)
                if not m:
                    continue
                claimed = m.group(2).lower()
                if claimed not in MNEMONICS:
                    skipped += 1
                    continue
                addr = int(m.group(1), 16)
                if addr >= len(data):
                    bad += 1
                    print(f"OUT OF RANGE {path}:{i}: {addr:#x}")
                    continue
                got = next(md.disasm(data[addr:addr + 16], addr), None)
                actual = got.mnemonic if got else "<undecodable>"
                want = ALIAS.get(claimed, claimed)
                actual_norm = ALIAS.get(actual, actual)
                checked += 1
                if actual_norm != want and not (
                    # capstone prints far call/jmp as lcall/ljmp
                    want in ("call", "jmp") and actual_norm in ("lcall", "ljmp")
                ):
                    bad += 1
                    print(
                        f"MISMATCH {path}:{i}: doc says {addr:#07x} is `{claimed}`, "
                        f"disassembly says `{actual}`"
                    )
    print(f"{checked} cited instructions verified, {skipped} non-mnemonic lines skipped, {bad} wrong")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
