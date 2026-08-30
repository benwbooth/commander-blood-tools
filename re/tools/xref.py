#!/usr/bin/env python3
"""Cross-reference finder for BLOODPRG.EXE (relocation-aware).

DOS large/medium-model programs link functions with FAR calls (opcode 9A) and
far jumps (EA), whose segment word is patched by the MZ relocation table. This
tool finds those, plus pointer-loads of a segment and raw 16-bit immediate
references.

Usage:
    python3 re/tools/xref.py SEG:OFF            # far call/jmp to this seg:off
    python3 re/tools/xref.py --seg 0x1234       # relocations that load this segment
    python3 re/tools/xref.py --imm16 0x0a6      # raw LE16 immediate occurrences
    python3 re/tools/xref.py --callers SEG:OFF  # near (E8) callers within seg range
    python3 re/tools/xref.py --branches SEG:OFF [END_FILE]
                                                # every encoded relative branch

Far targets are matched against the *relative* segment (relocation pre-add
value), so SEG is the base-0 relative segment used by the other tools.
"""
import os
import struct
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
sys.path.insert(0, _here)
from mzfile import MZ


def far_refs(mz, seg, off):
    """Find 9A/EA far call/jmp ptr16:16 == seg:off (relative seg)."""
    data = mz.data
    hits = []
    target = struct.pack("<HH", off, seg)
    for opcode, name in ((0x9A, "call far"), (0xEA, "jmp far")):
        start = 0
        needle = bytes([opcode]) + target
        while True:
            idx = data.find(needle, start)
            if idx < 0:
                break
            # the segment word is at idx+3; confirm it is a relocation site
            img = mz.file_to_image(idx + 3)
            is_reloc = img in mz.reloc_image_offsets
            hits.append((idx, name, is_reloc))
            start = idx + 1
    return hits


def seg_pointer_loads(mz, seg):
    """Find image words (at relocation sites) whose relative segment == seg.
    These are places that load a far pointer to that segment."""
    hits = []
    for img_off in sorted(mz.reloc_image_offsets):
        w = struct.unpack_from("<H", mz.image, img_off)[0]
        if w == seg:
            hits.append(mz.image_to_file(img_off))
    return hits


def imm16_refs(mz, val):
    data = mz.data
    needle = struct.pack("<H", val & 0xFFFF)
    hits = []
    start = 0
    while True:
        idx = data.find(needle, start)
        if idx < 0:
            break
        hits.append(idx)
        start = idx + 1
    return hits


def near_callers(mz, seg, off, span=0x10000):
    """Scan a window around the target segment for E8 near calls that resolve
    to OFF within the same (assumed) segment. Heuristic: scans file bytes in the
    target segment's file range."""
    seg_base_file = mz.segoff_to_file(seg, 0)
    lo = max(0, seg_base_file - span)
    hi = min(len(mz.data), seg_base_file + span)
    data = mz.data
    hits = []
    i = lo
    while i < hi - 2:
        if data[i] == 0xE8:
            rel = struct.unpack_from("<h", data, i + 1)[0]
            ip_after = (i + 3) - seg_base_file  # IP relative to seg base
            tgt = (ip_after + rel) & 0xFFFF
            if tgt == off:
                hits.append(i)
        i += 1
    return hits


def relative_branches(mz, seg, off, end_file=None):
    """Find encoded near and short branches to OFF in one segment range.

    Scanning every byte is intentionally conservative: a hit may be embedded
    data and must be disassembled, while zero hits prove no ordinary relative
    transfer encoding exists. END_FILE can bound a reviewed code segment before
    the next linked segment begins.
    """
    base = mz.segoff_to_file(seg, 0)
    end = min(len(mz.data), base + 0x10000) if end_file is None else end_file
    if end < base or end > len(mz.data):
        raise ValueError(f"invalid branch scan end {end:#x}")
    hits = []

    for site in range(base, end):
        opcode = mz.data[site]
        if opcode in (0xE8, 0xE9) and site + 3 <= end:
            relative = struct.unpack_from("<h", mz.data, site + 1)[0]
            target = ((site - base) + 3 + relative) & 0xFFFF
            if target == off:
                hits.append((site, "call near" if opcode == 0xE8 else "jmp near"))
            continue
        if (
            opcode == 0xEB
            or 0x70 <= opcode <= 0x7F
            or 0xE0 <= opcode <= 0xE3
        ) and site + 2 <= end:
            relative = struct.unpack_from("<b", mz.data, site + 1)[0]
            target = ((site - base) + 2 + relative) & 0xFFFF
            if target == off:
                hits.append((site, "short branch"))
            continue
        if (
            opcode == 0x0F
            and site + 4 <= end
            and 0x80 <= mz.data[site + 1] <= 0x8F
        ):
            relative = struct.unpack_from("<h", mz.data, site + 2)[0]
            target = ((site - base) + 4 + relative) & 0xFFFF
            if target == off:
                hits.append((site, "conditional near"))
    return hits


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return
    mz = MZ()

    if args[0] == "--seg":
        seg = int(args[1], 16)
        hits = seg_pointer_loads(mz, seg)
        print(f"{len(hits)} pointer-load(s) of segment {seg:#06x}:")
        for h in hits:
            print(f"  {h:#08x}")
        return
    if args[0] == "--imm16":
        val = int(args[1], 16)
        hits = imm16_refs(mz, val)
        print(f"{len(hits)} raw LE16 occurrence(s) of {val:#06x} (incl. non-code)")
        for h in hits[:300]:
            loc = "image" if mz.header_size <= h < mz.image_total else "other"
            print(f"  {h:#08x} [{loc}]")
        return
    if args[0] == "--callers":
        seg, off = args[1].split(":")
        seg, off = int(seg, 16), int(off, 16)
        hits = near_callers(mz, seg, off)
        print(f"{len(hits)} near (E8) caller(s) of {seg:#06x}:{off:#06x}:")
        for h in hits:
            print(f"  {h:#08x}")
        return
    if args[0] == "--branches":
        seg, off = args[1].split(":")
        seg, off = int(seg, 16), int(off, 16)
        end_file = int(args[2], 16) if len(args) > 2 else None
        hits = relative_branches(mz, seg, off, end_file)
        print(f"{len(hits)} relative branch(es) to {seg:#06x}:{off:#06x}:")
        for file_offset, kind in hits:
            print(f"  {file_offset:#08x}  {kind}")
        return

    # default: SEG:OFF far refs
    seg, off = args[0].split(":")
    seg, off = int(seg, 16), int(off, 16)
    hits = far_refs(mz, seg, off)
    print(f"{len(hits)} far call/jmp(s) to {seg:#06x}:{off:#06x}:")
    for idx, name, is_reloc in hits:
        flag = "" if is_reloc else "  (NOT a reloc site -- likely false match)"
        print(f"  {idx:#08x}  {name}{flag}")


if __name__ == "__main__":
    main()
