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
# NOT preceded by an alphanumeric: "320x200" contains the substring "0x200",
# so a plain `0x[0-9A-Fa-f]{3,6}` harvested a PHANTOM citation from every
# screen-dimension string in a doc. 11 ledger rows were provisionally ASM?
# on that basis alone -- evidenced-looking rows with no evidence.
ADDR = re.compile(r"(?<![0-9A-Za-z])0x([0-9A-Fa-f]{3,6})")
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


def is_real_boundary(mz, md, at):
    """Do decodes entered further back agree that `at` starts an instruction?"""
    agree = total = 0
    for back in range(6, 34, 4):
        anchor = max(0, at - back)
        total += 1
        starts = set()
        for insn in md.disasm(mz.data[anchor : at + 16], anchor):
            starts.add(insn.address)
            if insn.address > at:
                break
        if at in starts:
            agree += 1
    return total and agree * 2 > total


def is_operand_offset(mz, md, value):
    """Is `value` the file offset of some instruction's IMMEDIATE operand?

    A `*_IMMEDIATE` constant does not hold a value the game uses -- it holds the
    ADDRESS of one, so the port can patch or read it. `and bx,0xe` at 0x44B7 is
    `83 e3 0e`, so its imm8 lives at 0x44B9. Verifying that means decoding the
    instruction that CONTAINS the offset, not looking for the offset as a number.
    """
    for back in range(1, 9):
        start = value - back
        if start < 0:
            break
        for insn in md.disasm(mz.data[start : start + 16], start):
            if insn.address != start:
                break
            enc = getattr(insn, "encoding", None)
            if enc and enc.imm_offset and insn.address + enc.imm_offset == value:
                # Decoding from an arbitrary earlier byte resynchronises into
                # PHANTOM instructions: 0x44B9 "matched" a jcxz at 0x44B8 when the
                # real instruction is `and bx,0xe` at 0x44B7. Require the start to
                # be a boundary independent anchors agree on, as
                # check_opsize_mnemonics.py does.
                if not is_real_boundary(mz, md, start):
                    break
                return (
                    f"{insn.address:#07x}: {insn.mnemonic} {insn.op_str} "
                    f"(imm at {value:#07x})"
                )
            break
    return None


def negated_values(imm):
    """A constant can appear as its TWO'S COMPLEMENT.

    `LOCATION_PANEL_TINT_PERCENT` is 50, and the binary holds `mov ax,0xffce`
    (-50) at 0x90ED because the blend builder negates on entry (`neg ax` @0x22F1).
    Searching for 50 finds nothing at either address; searching for -50 finds it.

    ONLY the 16-bit form. Negating in 8 bits turns small constants into other small
    constants -- `OP_MAX` 0xFE becomes 0x02, `TALK_FIELD` 0x3A becomes 0xC6 -- and
    both then "matched" the first ordinary immediate nearby. 0xFFCE is distinctive;
    0x02 is not.
    """
    return {(-imm) & 0xFFFF}


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


skipped_overlay = []

# A module documenting an OVERLAY cites offsets into that overlay. The overlays
# are raw 386 images whose runtime cs maps 1:1 to file offsets (re/tools/dis_xdb.py),
# so the same immediate/shift/identity checks work -- they just need the right
# bytes. Skipping these files entirely (the earlier behaviour) left every constant
# in them permanently unverifiable.
OVERLAY_FOR = {
    "croolis.rs": ["croolis.xdb", "amer.xdb", "scrut.xdb"],
    "manu3.rs": ["manu3.xdb"],
    "manu3_hand.rs": ["manu3.xdb"],
}
OVERLAY_DIRS = [
    os.path.join("output", "_tmp_dat"),
    os.path.join("export_check", "_tmp_dat"),
]


class RawImage:
    """Minimal stand-in for MZ: an overlay is its own address space."""

    def __init__(self, data):
        self.data = data


def load_overlays(names):
    """First readable overlay image among `names`, or None."""
    for name in names:
        for d in OVERLAY_DIRS:
            path = os.path.join(d, name)
            if os.path.exists(path):
                return name, RawImage(open(path, "rb").read())
    return None, None


def constants():
    """Yield (path, line, name, value, [addresses])."""
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            # A module documenting an OVERLAY (`croolis.xdb`, `amer.xdb`, ...)
            # cites offsets into that overlay, not into BLOODPRG.EXE. Resolving
            # them here would make every match a coincidence and every miss
            # meaningless -- `ALIEN_POSITION_WRAP` cites method 0x999, which is
            # mid-instruction garbage in the main image. Skip the file; verifying
            # it needs re/tools/dis_xdb.py against the overlay itself.
            overlay_names = OVERLAY_FOR.get(f)
            overlay_name, overlay_img = (None, None)
            if overlay_names:
                overlay_name, overlay_img = load_overlays(overlay_names)
                if overlay_img is None:
                    skipped_overlay.append(path)
                    continue
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
                    # A number equal to the constant's OWN VALUE is the value
                    # restated, not an address -- `MENU_ANGLE_MASK = 0x0FFC` whose
                    # doc says "`0xFFC` = a 10-bit angle" was being reported as
                    # "not an immediate at 0x0FFC". Same tautology the removed
                    # value==address rule had on the output side.
                    addrs = [
                        int(a, 16)
                        for a in ADDR.findall(" ".join(doc))
                        if LOW <= int(a, 16) < HIGH and int(a, 16) != value
                    ]
                    if value is not None and addrs:
                        yield path, n, m.group(1), value, addrs, overlay_img, overlay_name
                # Attributes sit between a doc and its item (see audit-fixes #102).
                if st and not st.startswith("//") and not st.startswith("#["):
                    doc = []
    return


def main():
    mz = MZ()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    ok, bad, layout = [], [], {}
    overlay_checked = 0
    for path, line, name, value, addrs, overlay_img, overlay_name in constants():
        # Overlay-documented modules resolve against their own image.
        img = overlay_img if overlay_img is not None else mz
        if overlay_img is not None:
            overlay_checked += 1
        found = False
        for a in addrs:
            if a + 8 >= len(img.data):
                continue
            # NO "value == the cited address" rule. It looks like it recognises a
            # table-base constant, but what it actually matches is the constant's
            # own value appearing in its own doc comment -- `DLG_LINE_ASSET_NONE =
            # 0xFFFF` cleared itself because the doc calls 0xFFFF a sentinel. That
            # is the self-referential shape check_selfref_asserts.py exists for.
            # Real evidence for a table base is a `mov si,0x6212` in the code.
            near = immediates_near(img, md, a)
            if value in near:
                found = True
                break
            if negated_values(value) & near:
                found = True
                layout[name] = f"present NEGATED (two's complement) near {a:#07x}"
                break
        # LAYOUT IDENTITY: a table's length is the distance to the next table, so
        # the value is arithmetic on two cited addresses and appears as an
        # immediate nowhere. `DIALOGUE_FONT_ASCII_MAP_LEN = 176` is exactly
        # 0x14CD2 - 0x14C22, which is how the 128-vs-176 truncation was settled.
        # A `*_IMMEDIATE` constant names the file offset of an operand.
        if not found and value >= 0x400 and value < len(img.data):
            hit = is_operand_offset(img, md, value)
            if hit:
                found = True
                layout[name] = hit
        if not found:
            for i, a in enumerate(addrs):
                for b in addrs[i + 1:]:
                    if value and abs(a - b) == value:
                        found = True
                        layout[name] = (a, b)
                        break
                    # The SUM form: DS base + DS-relative offset is the file
                    # offset. `WORLD_ART_TABLE_FILE_OFFSET = 0xFFE7` is exactly
                    # 0xD420 + 0x2BC7, the file offset of DS:0x2BC7.
                    if value and a + b == value:
                        found = True
                        layout[name] = f"identity: {a:#07x} + {b:#07x}"
                        break
                if found:
                    break
        if found:
            if name in layout:
                v = layout[name]
                insn = (
                    v
                    if isinstance(v, str)
                    else f"layout identity: {max(v):#07x} - {min(v):#07x}"
                )
            else:
                insn = matching_insn(img, md, a, value)
            ok.append((path, line, name, value, a, insn))
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
    if overlay_checked:
        print(
            f"{overlay_checked} constant(s) resolved against .xdb OVERLAY images "
            "rather than BLOODPRG.EXE"
        )
    if skipped_overlay:
        print(
            f"skipped {len(skipped_overlay)} overlay-documented file(s) "
            f"({', '.join(os.path.basename(p) for p in skipped_overlay)}) -- no "
            "overlay image found under output/_tmp_dat"
        )
    print(
        f"{len(ok) + len(bad)} cited integer constant(s); {len(ok)} DIRECTLY "
        f"encoded at a cited address, {len(bad)} need reading (dispatch indices, "
        "derived values, wrong citations)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
