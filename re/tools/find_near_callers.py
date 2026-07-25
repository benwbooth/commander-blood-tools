#!/usr/bin/env python3
"""Find near-call (E8 rel16) sites targeting a given file offset.

Near calls stay within one code segment, so in file space the target is simply
site+3+rel16 (mod 64K within the segment). We search the whole image and keep
matches whose 16-bit wraparound lands on the target; callers in other segments
are impossible for E8, so any hit in a plausible code range is a real caller.

Usage: python3 re/tools/find_near_callers.py 0x981b [more offsets...]

Run from the REPO ROOT, like every other tool here. This used to open
`bin/BLOODPRG.EXE` as a bare relative path, so it worked only when invoked from
inside `re/` and raised FileNotFoundError everywhere else -- including from the
root, which is where CLAUDE.md says tools are run.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from mzfile import MZ

data = MZ().data

for arg in sys.argv[1:]:
    target = int(arg, 16)
    print(f'near callers of {target:#x}:')
    for pos in range(len(data) - 3):
        if data[pos] != 0xE8:
            continue
        rel = int.from_bytes(data[pos + 1:pos + 3], 'little', signed=True)
        if pos + 3 + rel == target:
            print(f'  {pos:#08x}')
