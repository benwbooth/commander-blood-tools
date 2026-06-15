#!/usr/bin/env python3
"""Convert between SEG:OFF, file offset, image offset, and DS-relative offset.

Usage:
    python3 re/tools/seg_offset.py SEG:OFF        # e.g. 0ce2:7802
    python3 re/tools/seg_offset.py file 0x14c22   # file offset -> everything
    python3 re/tools/seg_offset.py img  0x14622   # image offset -> everything
    python3 re/tools/seg_offset.py ds   0x7802    # DS-relative (DS=startup data seg)

The startup data segment (the value loaded into DS at entry) is read from the
binary itself: the first instruction is `mov ax, imm16; mov ds, ax`.
"""
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
sys.path.insert(0, _here)
from mzfile import MZ


def startup_ds(mz):
    # Entry: b8 <lo> <hi> = mov ax, imm16 ; 8e d8 = mov ds, ax
    e = mz.entry_file
    if mz.data[e] == 0xB8 and mz.data[e + 3:e + 5] == b"\x8e\xd8":
        return mz.data[e + 1] | (mz.data[e + 2] << 8)
    return None


def report(mz, file_off, ds_seg):
    img = mz.file_to_image(file_off)
    seg = img // 16
    off = img % 16
    print(f"file offset : {file_off:#08x}")
    print(f"image offset: {img:#08x}")
    print(f"SEG:OFF     : {seg:#06x}:{off:#04x}  (relative segment, base 0)")
    if ds_seg is not None:
        ds_off = file_off - mz.segoff_to_file(ds_seg, 0)
        print(f"DS-relative : DS:{ds_off:#06x}   (DS={ds_seg:#06x})")


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return
    mz = MZ()
    ds_seg = startup_ds(mz)

    if ":" in args[0] and not args[0].lower().startswith("0x"):
        seg, off = args[0].split(":")
        report(mz, mz.segoff_to_file(int(seg, 16), int(off, 16)), ds_seg)
        return

    kind = args[0]
    val = int(args[1], 16)
    if kind == "file":
        report(mz, val, ds_seg)
    elif kind == "img":
        report(mz, mz.image_to_file(val), ds_seg)
    elif kind == "ds":
        if ds_seg is None:
            print("could not auto-detect startup DS")
            return
        report(mz, mz.segoff_to_file(ds_seg, val), ds_seg)
    else:
        print(f"unknown kind: {kind}")


if __name__ == "__main__":
    main()
