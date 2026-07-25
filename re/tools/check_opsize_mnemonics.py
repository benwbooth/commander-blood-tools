#!/usr/bin/env python3
"""Does a cited `cwde`/`cdq` match the operand size the BYTES actually encode?

Capstone prints opcode 0x98 as `cwde` and 0x99 as `cdq` in CS_MODE_16, where
without a 0x66 prefix they are architecturally CBW and CWD. The difference is
semantic, not cosmetic:

    cbw   sign-extends AL  into AX   -- AH is OVERWRITTEN
    cwde  sign-extends AX  into EAX  -- AX is left alone, AH survives

After `lodsb`, those two produce different values whenever AH is not already the
sign of AL, which is why the D2 profile handler at 0x64B8 stores the operand
under CBW and would store caller state under CWDE (audit-fixes #100).

The bytes settle it, and the tell is INSTRUCTION LENGTH: a bare 0x98 is one byte
(CBW), a prefixed `66 98` is two (CWDE). This walks forward from each cited
address to the first such instruction and compares its real size against the
mnemonic the citation used. Both directions are errors -- calling a genuine
32-bit `cwde` "cbw" is just as wrong, and src/manu3.rs's `shl eax,0x10; cdq`
IS a real CDQ.

Generated listings under src/recomp are exempt: they are verbatim capstone
output, and rewriting a mnemonic there would make the comment disagree with the
tool that produced it.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)

import capstone  # noqa: E402

sys.path.insert(0, _here)
from mzfile import MZ  # noqa: E402

# 16-bit forms and their 32-bit counterparts, by opcode.
PAIRS = {0x98: ("cbw", "cwde"), 0x99: ("cwd", "cdq")}
MNEMONICS = re.compile(r"\b(cbw|cwde|cwd|cdq)\b", re.I)
# A line SAYING the disassembler prints `cwde` is documenting the trap, not
# claiming the encoding. Flagging those would punish the only notes that warn the
# next reader -- the same mistake the cited-instruction guard made before it grew
# an alias table.
ABOUT_THE_TOOL = re.compile(
    r"(prints?|renders?|capstone|dis\.py|TOOLING TRAP|shows? it|mnemonic)", re.I
)
# NOT preceded by an alphanumeric: "320x200" contains the substring "0x200",
# so a plain `0x[0-9A-Fa-f]{3,6}` harvested a PHANTOM citation from every
# screen-dimension string in a doc. 11 ledger rows were provisionally ASM?
# on that basis alone -- evidenced-looking rows with no evidence.
ADDR = re.compile(r"(?<![0-9A-Za-z])0x([0-9A-Fa-f]{4,6})")
# How far past the cited address the instruction may sit. Citations name the
# routine or the neighbouring instruction, not the 0x98 itself.
WINDOW = 48

EXEMPT_DIRS = (os.path.join("src", "recomp"),)


def boundaries_from(mz, md, anchor, upto):
    """Instruction start offsets decoding from `anchor` until past `upto`."""
    out = set()
    for insn in md.disasm(mz.data[anchor : upto + 16], anchor):
        out.add(insn.address)
        if insn.address > upto:
            break
    return out


def is_real_boundary(mz, md, at, cited):
    """Do independent decode anchors agree that `at` starts an instruction?

    x86 is self-synchronizing, so streams entered at different points converge.
    A cited address that is itself mid-instruction produces a phantom stream:
    labels.csv had a `cdq` that was really the 0x99 inside `lcall 0x299:0x0ecb`.
    Requiring consensus from EARLIER anchors rejects those, because an earlier
    (correctly aligned) entry decodes the far call as one 5-byte instruction.
    """
    agree = total = 0
    for back in range(8, 40, 4):
        anchor = max(0, cited - back)
        total += 1
        if at in boundaries_from(mz, md, anchor, at):
            agree += 1
    return total and agree * 2 > total


def find_convert(mz, md, start):
    """First cbw/cwd-family instruction at or after `start`; (file_off, size)."""
    data = mz.data[start : start + WINDOW]
    for insn in md.disasm(data, start):
        if insn.bytes[-1] in PAIRS and insn.mnemonic in ("cbw", "cwde", "cwd", "cdq"):
            return insn.address, insn.size, insn.bytes[-1]
    return None


def citations():
    """Yield (source, line_no, address, mnemonic_as_written)."""
    path = os.path.join("re", "labels.csv")
    if os.path.exists(path):
        for n, line in enumerate(open(path, encoding="utf-8"), 1):
            m = MNEMONICS.search(line)
            if not m or ABOUT_THE_TOOL.search(line):
                continue
            a = ADDR.search(line)
            if a:
                yield path, n, int(a.group(1), 16), m.group(1).lower()

    for root, _, files in os.walk("src"):
        if any(root.startswith(d) for d in EXEMPT_DIRS):
            continue
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            p = os.path.join(root, f)
            for n, line in enumerate(open(p, encoding="utf-8", errors="replace"), 1):
                st = line.strip()
                if not (st.startswith("//") or st.startswith("///")):
                    continue
                m = MNEMONICS.search(st)
                if not m or ABOUT_THE_TOOL.search(st):
                    continue
                a = ADDR.search(st)
                if a:
                    yield p, n, int(a.group(1), 16), m.group(1).lower()


def main():
    mz = MZ()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    problems, phantom, checked, unresolved = [], [], 0, 0

    for src, line, addr, written in citations():
        hit = find_convert(mz, md, addr)
        if hit is None:
            # The 0x98 may be further off than WINDOW, or the citation may name a
            # data address. Not an error -- there is nothing to contradict.
            unresolved += 1
            continue
        at, size, opcode = hit
        if not is_real_boundary(mz, md, at, addr):
            # The byte is there but no instruction starts on it. The citation is
            # anchored mid-instruction -- a worse defect than a wrong mnemonic,
            # because everything else it claims about the routine is suspect too.
            phantom.append(
                f"{src}:{line}: `{written}` at {at:#07x} is not an instruction "
                f"boundary -- the citation at {addr:#07x} is misaligned"
            )
            continue
        checked += 1
        truth = PAIRS[opcode][1 if size > 1 else 0]
        if written != truth:
            problems.append(
                f"{src}:{line}: cites `{written}` for {at:#07x}, but the "
                f"encoding is {size} byte(s) -> `{truth}`"
            )

    for p in problems:
        print("OPSIZE " + p)
    for p in phantom:
        print("MISALIGNED " + p)
    print(
        f"{checked} cited convert instruction(s) resolved to bytes, "
        f"{len(problems)} mnemonic mismatch(es), {len(phantom)} misaligned "
        f"citation(s); {unresolved} citation(s) had no 0x98/0x99 within the window"
    )
    return 1 if (problems or phantom) else 0


if __name__ == "__main__":
    raise SystemExit(main())
