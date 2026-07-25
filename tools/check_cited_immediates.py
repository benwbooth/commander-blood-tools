#!/usr/bin/env python3
"""Does a constant's cited address actually CONTAIN that constant?

Most of the port's constants claim provenance like

    /// `mov al,0x2C` @0x8642 -- the hand at rest
    pub const MENU_REST_FRAME: u8 = 0x2C;

which is checkable mechanically: disassemble the cited address and look for the
value among the instruction's immediates or displacements. A constant whose value
appears nowhere near the address it cites is either a wrong address, a wrong
value, or a capture-measured number wearing a citation -- all three are defects
the prime rule cares about.

This does NOT prove the constant means what the doc says it means; a matching
immediate can still be the wrong instruction's. It proves the weaker, still
valuable thing: the number is IN the binary where the doc says it is. Rows it
clears are candidates for settling, not settled by fiat.

It is a CLASSIFIER, not a guard, because plenty of correct constants are not
immediates at all:

  * an opcode constant cites its HANDLER -- the value is a dispatch-table index
    and appears nowhere in the handler's bytes (`OP_JUMP = 0xA4` at 0x65DB);
  * a stride can be encoded as a shift count (`DLG_ASSET_NAME_STRIDE = 0x10` is
    `shl ax,4` at 0x768E);
  * a table-base constant's value IS the address it cites -- but NOT recognised
    here, because matching it is indistinguishable from the constant's own value
    appearing in its own doc.

The shift-count case is recognised here. The first cannot be, so "not found directly" is
reported as NEEDS READING, never as a defect -- a tool that called those wrong
would be training the reader to ignore it.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

# Import capstone BEFORE putting re/tools on the path: that directory contains a
# `dis.py`, which shadows the stdlib `dis` module that capstone -> inspect needs.
import capstone  # noqa: E402

sys.path.insert(0, os.path.join("re", "tools"))

from mzfile import MZ  # noqa: E402

CONST = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*"
    r"(u8|u16|u32|usize|i8|i16|i32|isize)\s*=\s*([^;]+);"
)
ADDR = re.compile(r"0x([0-9A-Fa-f]{3,6})")
INT = re.compile(r"^(0x[0-9A-Fa-f_]+|[0-9_]+)$")
# Only file offsets inside the image are citable code addresses.
LOW, HIGH = 0x400, 0x160000
WINDOW = 10  # instructions decoded from the cited address


def parse_value(expr):
    expr = expr.strip().replace("_", "")
    for suffix in ("u8", "u16", "u32", "usize", "i8", "i16", "i32", "isize"):
        if expr.endswith(suffix):
            expr = expr[: -len(suffix)]
    if not INT.match(expr):
        return None
    return int(expr, 16) if expr.lower().startswith("0x") else int(expr)


def imm_values(imm):
    """The values an immediate can legitimately stand for.

    Masking every immediate to 8 and 16 bits cleared constants by COINCIDENCE:
    `GAME_FONT_WIDTH = 8` matched `add ax,0x808` and an entry count of 8 matched a
    `[di+8]` displacement. Only the immediate itself counts, plus its truncations
    when it is NEGATIVE, where capstone reports the sign-extended form of a byte
    the code really does contain.
    """
    if imm < 0:
        return {imm & 0xFFFFFFFF, imm & 0xFFFF, imm & 0xFF}
    return {imm}


def matching_insn(mz, md, addr, value):
    """The instruction at/after `addr` that encodes `value`, for eyeballing."""
    for insn in md.disasm(mz.data[addr : addr + WINDOW * 8], addr):
        vals = set()
        for op in insn.operands:
            if op.type == capstone.x86.X86_OP_IMM:
                vals |= imm_values(op.imm)
                if insn.mnemonic in ("shl", "sal", "shr", "sar") and 0 < op.imm < 16:
                    vals.add(1 << op.imm)
            elif op.type == capstone.x86.X86_OP_MEM and op.mem.disp:
                vals.add(op.mem.disp & 0xFFFF)
        if value in vals:
            return f"{insn.address:#07x}: {insn.mnemonic} {insn.op_str}"
    return "(no instruction found)"


def immediates_near(mz, md, addr):
    """Every value the instructions at/after `addr` encode.

    Includes shift counts expanded to the stride they mean: `shl ax,4` multiplies
    by 16, so a constant of 16 is genuinely encoded there even though no 0x10
    appears in the instruction.
    """
    out = set()
    data = mz.data[addr : addr + WINDOW * 8]
    for insn in md.disasm(data, addr):
        for op in insn.operands:
            if op.type == capstone.x86.X86_OP_IMM:
                out |= imm_values(op.imm)
                if insn.mnemonic in ("shl", "sal", "shr", "sar") and 0 < op.imm < 16:
                    out.add(1 << op.imm)
            elif op.type == capstone.x86.X86_OP_MEM:
                if op.mem.disp:
                    out.add(op.mem.disp & 0xFFFF)
        if len(out) > 400:
            break
    return out


def constants():
    """Yield (path, line, name, value, [addresses])."""
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            doc = []
            in_tests = False
            for n, line in enumerate(lines, 1):
                st = line.strip()
                if st.startswith("#[cfg(test)]"):
                    in_tests = True
                if st.startswith("///") or st.startswith("//!") or st.startswith("//"):
                    doc.append(st)
                    continue
                m = CONST.match(line)
                if m and not in_tests:
                    value = parse_value(m.group(3))
                    addrs = [
                        int(a, 16)
                        for a in ADDR.findall(" ".join(doc))
                        if LOW <= int(a, 16) < HIGH
                    ]
                    if value is not None and addrs:
                        yield path, n, m.group(1), value, addrs
                # Attributes sit between a doc and its item (see audit-fixes #102).
                if st and not st.startswith("//") and not st.startswith("#["):
                    doc = []
    return


def main():
    mz = MZ()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    ok, bad = [], []
    for path, line, name, value, addrs in constants():
        found = False
        for a in addrs:
            if a + 8 >= len(mz.data):
                continue
            # NO "value == the cited address" rule. It looks like it recognises a
            # table-base constant, but what it actually matches is the constant's
            # own value appearing in its own doc comment -- `DLG_LINE_ASSET_NONE =
            # 0xFFFF` cleared itself because the doc calls 0xFFFF a sentinel. That
            # is the self-referential shape check_selfref_asserts.py exists for.
            # Real evidence for a table base is a `mov si,0x6212` in the code.
            if value in immediates_near(mz, md, a):
                found = True
                break
        if found:
            ok.append((path, line, name, value, a, matching_insn(mz, md, a, value)))
        else:
            bad.append((path, line, name, value, addrs))

    if "--list" in sys.argv:
        print("DIRECTLY ENCODED -- value, and the instruction that encodes it:")
        for path, line, name, value, a, insn in sorted(ok, key=lambda r: r[2]):
            # A one-digit value matching some immediate is weak evidence: small
            # numbers are everywhere. Strong matches are what should be settled.
            weak = " WEAK" if value < 0x10 else ""
            print(f"  {name:<44} {value:#06x}  {insn}{weak}")
        print()

    for path, line, name, value, addrs in bad:
        where = ",".join(f"{a:#07x}" for a in addrs[:4])
        print(f"NEEDS-READING {path}:{line}: {name} = {value:#x} is not an "
              f"immediate at {where}")
    print(
        f"{len(ok) + len(bad)} cited integer constant(s); {len(ok)} DIRECTLY "
        f"encoded at a cited address, {len(bad)} need reading (dispatch indices, "
        "derived values, wrong citations)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
