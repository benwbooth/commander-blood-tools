#!/usr/bin/env python3
"""The routine entry containing an address, found by CALL TARGETS not by guessing.

The heuristic this replaces scans backwards for a `ret`/`retf` followed by a
prologue-looking byte. It is wrong often enough to matter: in audit-fixes #568 it
placed a caller's entry at `project_tail_9bba`, a 12-instruction tail that
`check_label_alignment.py` already flags as misaligned and that nothing branches to.
A `ret` followed by `push` is extremely common inside a routine, so the scan finds
mid-routine positions and presents them as entries.

A real entry is a place something CALLS. This collects every call target in the
image -- near (`E8 rel16`, resolved against the instruction's own end) and far
(`9A off16 seg16`, resolved as `0x600 + seg*16 + off`) -- and reports the greatest
target at or below the address, with the calls that reach it.

That is still not a proof: a routine reached only by a jump table (the nav
subdispatch families of #494/#534) has no call site, and this will report the
previous called routine instead. So it prints the DISTANCE, and a large one is the
signal to distrust the answer rather than a licence to believe it.

Usage:
    python3 re/tools/find_entry.py <addr_hex> [more...]
"""
import struct
import sys
from collections import defaultdict

EXE = "re/bin/BLOODPRG.EXE"


# Bytes a real routine entry plausibly starts with: the push/save prologue this
# binary uses everywhere, plus the 0x66 operand-size prefix its 32-bit routines
# lead with. NOT a proof -- a filter (audit-fixes #570).
PROLOGUE = {
    0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57,  # push r16
    0x06, 0x0E, 0x16, 0x1E,                          # push es/cs/ss/ds
    0x60,                                            # pusha
    0x66,                                            # operand-size prefix
    0x9C,                                            # pushf
    0xC3, 0xCB,                                      # a bare ret/retf stub IS an entry
}


def call_targets(data):
    """target -> [call sites], for near and far calls.

    BYTE-SCANNING FOR `E8`/`9A` FINDS PHANTOM CALLS. `0xA8D1` in this image is the
    `E8` of `mov bp,ax` (`8B E8`) followed by `stc`, and reading it as a near call
    invents a target at `0x7ACD` -- an address that is not an instruction boundary,
    which is how the first version of this tool reported a phantom entry for the
    montage routine (audit-fixes #570). The same self-synchronisation trap that
    makes disassembling from an arbitrary address unsafe applies to scanning for
    opcodes.

    Mitigated, not solved: a target whose first byte is not a plausible prologue is
    dropped. A real entry can still be missed (one starting with something odd) and
    a phantom can still survive (landing on a `push` by chance), so the caller
    should treat a single-site target with more suspicion than an eight-site one.
    """
    out = defaultdict(list)
    for i in range(len(data) - 5):
        if data[i] == 0xE8:
            rel = struct.unpack_from("<h", data, i + 1)[0]
            t = i + 3 + rel
            if 0 <= t < len(data) and data[t] in PROLOGUE:
                out[t].append((i, "near"))
        elif data[i] == 0x9A:
            off, seg = struct.unpack_from("<HH", data, i + 1)
            t = 0x600 + seg * 16 + off
            if 0 <= t < len(data) and data[t] in PROLOGUE:
                out[t].append((i, "far"))
    return out


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    data = open(EXE, "rb").read()
    targets = call_targets(data)
    ordered = sorted(targets)

    for arg in sys.argv[1:]:
        addr = int(arg, 16)
        # greatest call target <= addr
        lo, hi, best = 0, len(ordered) - 1, None
        while lo <= hi:
            mid = (lo + hi) // 2
            if ordered[mid] <= addr:
                best = ordered[mid]
                lo = mid + 1
            else:
                hi = mid - 1
        print(f"\n{addr:#07x}:")
        if best is None:
            print("   no call target at or before this address")
            continue
        callers = targets[best]
        dist = addr - best
        warn = "  <-- SUSPICIOUS: far from the entry, may be jump-table reached" if dist > 0x400 else ""
        print(f"   entry {best:#07x}  ({dist} bytes before){warn}")
        kinds = ", ".join(f"{site:#07x} {kind}" for site, kind in callers[:5])
        print(f"   called from {len(callers)} site(s): {kinds}")


if __name__ == "__main__":
    main()
