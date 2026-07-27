#!/usr/bin/env python3
"""Every fixed encoding that references a DIRECT memory address, in one table.

Three times in one session an enumeration of "all the encodings" was missing a
family, and each time the omission changed a conclusion:

  #335  `a3` (mov [imm16],ax) missing -> a writer count came out 26 instead of 29
  #336  `89 /r` (mov [imm16],reg16) missing -> the same count needed a hand regex
  #358  `83 /N` (sign-extended imm8 on a WORD operand) missing -> a flag's bit 3
        showed six readers and NO writer, which is not a possible state

Twice that happened AFTER writing down the rule "a tool that searches a set must
report what is not in the set". The rule is correct and insufficient: what is
needed is not a reminder but a TABLE, written once and reused, so a caller cannot
express a partial search by accident.

x86-16 direct-address forms (`mod=00, r/m=110`, so modrm = `(reg << 3) | 6`):

    80 /N ib   byte  [imm16], imm8        N = add,or,adc,sbb,and,sub,xor,cmp
    81 /N iw   word  [imm16], imm16       same N
    83 /N ib   word  [imm16], imm8 SIGN-EXTENDED   <- the one most often missed
    C6 06 ib   mov   byte [imm16], imm8
    C7 06 iw   mov   word [imm16], imm16
    F6 06 ib   test  byte [imm16], imm8
    F7 06 iw   test  word [imm16], imm16
    88/89 /r   mov   [imm16], reg8/reg16
    8A/8B /r   mov   reg8/reg16, [imm16]
    A0/A1      mov   al/ax, [imm16]       accumulator short forms
    A2/A3      mov   [imm16], al/ax
    FF /N      inc/dec/push word [imm16]

Each also appears with a `65` (GS) segment prefix, and callers usually want the
prefixed site reported as the SAME site rather than a second one.

WHAT THIS CANNOT SEE, and it is a precise limit rather than a general one
(audit-fixes #364). These patterns find every instruction that NAMES the address.
They say nothing about what happens to the value afterwards:

    mov ax, [0x2793]      <- found
    and ax, 0xff0f        <- invisible: operates on a REGISTER
    test ax, 2            <- invisible
    or  ax, bx
    mov [0x2793], ax      <- found

A census over `0x2793` therefore reported "bit 1 is never referenced alone" and
"bits 4..7 are never OR-set", and BOTH were wrong — the reader and the writer of
those bits work through `ax`. Setter LOCATIONS are reliable (a store names its
address); conclusions about a bit's MEANING are not, unless you follow the
register.

Usage as a library:

    from encodings import address_forms
    for name, pattern, kind in address_forms(0x2793):
        ...   # pattern is a regex over the raw image; kind is R/W/SET/CLR/TOG
"""

import re

# (reg-field, mnemonic, kind) for the group-1 ALU opcodes 80/81/83.
_ALU = [
    (0, "add", "W"),
    (1, "or", "SET"),
    (2, "adc", "W"),
    (3, "sbb", "W"),
    (4, "and", "CLR"),
    (5, "sub", "W"),
    (6, "xor", "TOG"),
    (7, "cmp", "R"),
]
# (reg-field, mnemonic) for FF /N on a word operand.
_FF = [(0, "inc"), (1, "dec"), (6, "push")]


def _modrm(reg):
    """Direct-address modrm: mod=00, r/m=110, ESCAPED for use in a regex.

    `re.escape` is not optional here. modrm for reg=5 is `0x2E`, which as a byte
    IS the regex wildcard `.` — so an unescaped `sub` pattern matched EVERY modrm
    and silently reclassified `or`/`and`/`xor` sites as `sub` (audit-fixes #359).
    The address bytes need the same treatment: any address containing `0x2A`,
    `0x2B`, `0x3F`, `0x5B`, `0x5C`, `0x7C`, `0x5E` or `0x24` would do the same.
    """
    return re.escape(bytes([(reg << 3) | 0x06]))


# KNOWN GAPS, found by tools/check_cited_cells.py (audit-fixes #434) and left
# recorded rather than silently patched, because callers may already compensate:
#
#   * `address_forms` has no C4/C5 (`les`/`lds`). `les di, gs:[0x6724]` is
#     `65 c4 3e 24 67` -- the VM record-table pointer, read at 0x6B4D and a dozen
#     other sites -- and census() reports ZERO for 0x6724.
#   * `reg_disp_forms` enumerates word opcodes and omits `8A` (byte loads).
#     `gs: mov al,[bx+0x6D60]` @0x6023, the vm_field_offset matrix, is invisible.
#
# The robust alternative, used by check_cited_cells.py, is to match the MODRM
# rather than the opcode: mod=00/rm=110 is a direct address and mod=10 is a
# reg+disp16, whatever instruction carries it. Enumerating opcode families is what
# under-reported in #335, #359 and #403; this file was written to fix that and has
# the same shape of hole one level down.
def address_forms(addr):
    """[(name, compiled regex, kind)] for every direct reference to `addr`.

    `kind` is R (read/compare), W (whole write), SET (or), CLR (and), TOG (xor).
    Each pattern captures the immediate where the encoding has one, so a caller
    can classify by VALUE as well as by operation.
    """
    a = re.escape(bytes([addr & 0xFF, (addr >> 8) & 0xFF]))
    out = []

    def add(name, body, kind):
        # `(?:\x65)?` so a GS-prefixed instruction matches at the prefix, letting
        # the caller collapse it to one site instead of double-counting.
        out.append((name, re.compile(rb"(?:\x65)?" + body, re.S), kind))

    for reg, mn, kind in _ALU:
        add(f"{mn} byte [m],i8", b"\x80" + _modrm(reg) + a + b"(.)", kind)
        add(f"{mn} word [m],i16", b"\x81" + _modrm(reg) + a + b"(..)", kind)
        add(f"{mn} word [m],i8sx", b"\x83" + _modrm(reg) + a + b"(.)", kind)
    add("mov byte [m],i8", b"\xc6\x06" + a + b"(.)", "W")
    add("mov word [m],i16", b"\xc7\x06" + a + b"(..)", "W")
    add("test byte [m],i8", b"\xf6\x06" + a + b"(.)", "R")
    add("test word [m],i16", b"\xf7\x06" + a + b"(..)", "R")
    add("mov [m],al", b"\xa2" + a, "W")
    add("mov [m],ax", b"\xa3" + a, "W")
    add("mov al,[m]", b"\xa0" + a, "R")
    add("mov ax,[m]", b"\xa1" + a, "R")
    for reg in range(8):
        add("mov [m],reg8", b"\x88" + _modrm(reg) + a, "W")
        add("mov [m],reg16", b"\x89" + _modrm(reg) + a, "W")
        add("mov reg8,[m]", b"\x8a" + _modrm(reg) + a, "R")
        add("mov reg16,[m]", b"\x8b" + _modrm(reg) + a, "R")
    for reg, mn in _FF:
        add(f"{mn} word [m]", b"\xff" + _modrm(reg) + a, "W")
    return out


# Register-relative operands: `mod=01` (disp8) or `mod=10` (disp16), r/m = the
# base register. audit-fixes #376: a claim that "nothing sets [reg+2] bit 0 at
# runtime" was built by searching the `80 /N` BYTE form alone and missed three
# sites in the `81`/`83` families, one of which activates objects. The direct-
# address table above could not have helped -- it only knows `mod=00, r/m=110`.
RM_NAMES = ["bx+si", "bx+di", "bp+si", "bp+di", "si", "di", "bp", "bx"]
_ALU_BY_REG = {reg: (mn, kind) for reg, mn, kind in _ALU}


def reg_disp_forms(disp):
    """[(name, regex, kind)] for `<alu> byte|word [reg+disp], imm`, all bases.

    Covers `80` (byte, imm8), `81` (word, imm16) and `83` (word, imm8
    sign-extended) across every ALU op and both displacement widths. The
    immediate is captured so a caller can ask WHICH BITS a site touches, which is
    the question that matters for a flag byte.
    """
    out = []
    for opc, ilen in ((0x80, 1), (0x81, 2), (0x83, 1)):
        for reg, (mn, kind) in _ALU_BY_REG.items():
            for rm in range(8):
                for mod, dbytes in ((0x40, 1), (0x80, 2)):
                    if dbytes == 1 and not (0 <= disp <= 0xFF):
                        continue
                    d = (
                        bytes([disp & 0xFF])
                        if dbytes == 1
                        else bytes([disp & 0xFF, (disp >> 8) & 0xFF])
                    )
                    body = (
                        re.escape(bytes([opc, mod | (reg << 3) | rm]) + d)
                        + b"(" + (b"." if ilen == 1 else b"..") + b")"
                    )
                    out.append(
                        (
                            f"{mn} [{RM_NAMES[rm]}+{disp:#x}] ({'i8' if ilen == 1 else 'i16'})",
                            re.compile(b"(?:\x65|\x26|\x36|\x3e)?" + body, re.S),
                            kind,
                        )
                    )
    return out


def reg_disp_census(data, disp):
    """site -> (name, kind, immediate) for every `<alu> [reg+disp], imm`."""
    sites = {}
    for name, rx, kind in reg_disp_forms(disp):
        for m in rx.finditer(data):
            sites[m.start()] = (name, kind, int.from_bytes(m.group(1), "little"))
    return sites


def census(data, addr):
    """site -> (name, kind, immediate|None), one entry per distinct address."""
    sites = {}
    for name, rx, kind in address_forms(addr):
        for m in rx.finditer(data):
            imm = None
            if m.groups():
                imm = int.from_bytes(m.group(1), "little")
            sites[m.start()] = (name, kind, imm)
    return sites


if __name__ == "__main__":
    import collections
    import sys

    sys.path.insert(0, __file__.rsplit("/", 1)[0])
    from mzfile import MZ

    if len(sys.argv) < 2:
        print(__doc__)
        raise SystemExit(0)
    want = int(sys.argv[1], 16)
    sites = census(MZ().data, want)
    print(f"{len(sites)} distinct site(s) referencing {want:#06x}")
    by = collections.defaultdict(collections.Counter)
    for _, (name, kind, imm) in sorted(sites.items()):
        by[imm][kind] += 1
    for imm in sorted(by, key=lambda v: (v is None, v)):
        label = "(no immediate)" if imm is None else f"{imm:#06x}"
        print(f"  {label:>14}: {dict(by[imm])}")
