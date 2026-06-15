#!/usr/bin/env python3
"""ASCII string scanner for BLOODPRG.EXE.

Usage:
    python3 re/tools/strings_dump.py [-n MIN] [-g REGEX] [-r START END] [-i]

Options:
    -n MIN      minimum run length (default 4)
    -g REGEX    only print strings matching this (case-insensitive) regex
    -r A B      restrict to file offset range [A, B) (hex ok with 0x)
    -i          also show the SEG:OFF and DS-relative offset of each hit

Prints `file_off  text`.
"""
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
import re as _re

sys.path.insert(0, _here)
from mzfile import MZ
from seg_offset import startup_ds


def main():
    args = sys.argv[1:]
    minlen = 4
    grep = None
    start, end = None, None
    show_addr = False
    i = 0
    while i < len(args):
        a = args[i]
        if a == "-n":
            minlen = int(args[i + 1]); i += 2
        elif a == "-g":
            grep = _re.compile(args[i + 1], _re.IGNORECASE); i += 2
        elif a == "-r":
            start = int(args[i + 1], 0); end = int(args[i + 2], 0); i += 3
        elif a == "-i":
            show_addr = True; i += 1
        else:
            i += 1

    mz = MZ()
    ds_seg = startup_ds(mz)
    data = mz.data
    lo = start if start is not None else 0
    hi = end if end is not None else len(data)

    rx = _re.compile(rb"[\x20-\x7e]{%d,}" % minlen)
    for m in rx.finditer(data[lo:hi]):
        off = lo + m.start()
        text = m.group().decode("ascii", "replace")
        if grep and not grep.search(text):
            continue
        if show_addr:
            img = mz.file_to_image(off)
            seg, o = img // 16, img % 16
            ds_off = off - mz.segoff_to_file(ds_seg, 0) if ds_seg is not None else None
            extra = f"  ({seg:#06x}:{o:#04x}"
            if ds_off is not None:
                extra += f"  DS:{ds_off:#06x}"
            extra += ")"
        else:
            extra = ""
        print(f"{off:#08x}{extra}  {text}")


if __name__ == "__main__":
    main()
