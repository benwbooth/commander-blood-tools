#!/usr/bin/env python3
"""Recover the segment map of BLOODPRG.EXE from relocations + far-call targets.

A large-model DOS program is split into linker segments; far calls (9A) and far
jumps (EA) carry the target's *relative* segment base in a word that sits at a
relocation site. By collecting:
  (a) every segment value stored at a relocation site, and
  (b) every distinct segment used as a 9A/EA far-call target,
we recover the set of segment bases, which lets us map any file/image offset to
its containing segment (largest base <= offset/16).

Usage:
    python3 re/tools/dump_segments.py                 # list segment bases
    python3 re/tools/dump_segments.py --contains 0x2b92   # seg for an image off
    python3 re/tools/dump_segments.py --calltargets       # distinct far targets
"""
import collections
import os
import struct
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
sys.path.insert(0, _here)
from mzfile import MZ


def reloc_segment_values(mz):
    """Histogram of the relative-segment words stored at reloc sites."""
    hist = collections.Counter()
    for img_off in mz.reloc_image_offsets:
        if img_off + 1 < len(mz.image):
            hist[struct.unpack_from("<H", mz.image, img_off)[0]] += 1
    return hist


def far_call_targets(mz):
    """List (file_off, opcode, seg, off) for 9A/EA whose seg word is a reloc."""
    out = []
    data = mz.data
    for op, name in ((0x9A, "call"), (0xEA, "jmp")):
        i = mz.header_size
        while i < mz.image_total - 5:
            if data[i] == op:
                off, seg = struct.unpack_from("<HH", data, i + 1)
                seg_img = mz.file_to_image(i + 3)
                if seg_img in mz.reloc_image_offsets:
                    out.append((i, name, seg, off))
            i += 1
    return out


def segment_bases(mz):
    """Plausible code/data segment bases: union of reloc-stored segment values
    and far-call target segments, sorted."""
    bases = set(reloc_segment_values(mz).keys())
    for _, _, seg, _ in far_call_targets(mz):
        bases.add(seg)
    return sorted(bases)


def main():
    mz = MZ()
    args = sys.argv[1:]

    if args and args[0] == "--contains":
        img = int(args[1], 16)
        bases = segment_bases(mz)
        cand = [b for b in bases if b * 16 <= img]
        seg = max(cand) if cand else 0
        print(f"image {img:#06x} (file {mz.image_to_file(img):#08x}) "
              f"-> segment {seg:#06x}:{img - seg*16:#06x}")
        return

    if args and args[0] == "--calltargets":
        tgts = far_call_targets(mz)
        bycount = collections.Counter((seg, off) for _, _, seg, off in tgts)
        print(f"{len(tgts)} far call/jmp sites, "
              f"{len(bycount)} distinct targets:")
        for (seg, off), c in sorted(bycount.items()):
            print(f"  {seg:#06x}:{off:#06x}  x{c}  "
                  f"(file {mz.segoff_to_file(seg, off):#08x})")
        return

    hist = reloc_segment_values(mz)
    targets = far_call_targets(mz)
    tgt_segs = collections.Counter(seg for _, _, seg, _ in targets)
    print(f"{len(hist)} distinct segment values at {sum(hist.values())} reloc sites")
    print(f"{len(targets)} far call/jmp sites across "
          f"{len(tgt_segs)} distinct target segments\n")
    print("segment  reloc_uses  is_call_target  first_file_off")
    for seg in segment_bases(mz):
        r = hist.get(seg, 0)
        t = "CALL" if seg in tgt_segs else ""
        print(f"{seg:#06x}   {r:>6}      {t:<4}            "
              f"{mz.segoff_to_file(seg, 0):#08x}")


if __name__ == "__main__":
    main()
