#!/usr/bin/env python3
"""Decode a Commander Blood .DRV sound driver's entry-vector table.

`BLOODPRG.EXE` never contains the sound driver: `snd_driver_call` (`0xBB9D`) does
`lcall [0xcdf]`, an indirect far call into code loaded from `dnsdb.drv` (Sound
Blaster) or `nosound.drv`. `re/dead_ends.md` recorded that as statically
unresolvable FROM THE EXECUTABLE, which is true and was mistaken for undecidable —
the drivers ship with the game.

Both files open with a run of `E9 rel16` near jumps: a fixed-order vector table,
the driver's ABI. This prints each vector's target so the host's far-pointer slots
(`gs:0x0CDB`, `0x0CDF`, `0x0CF3`, ...) can be matched to real code.

A vector whose target is the SAME as another's is not a bug: `nosound.drv`
implements most entries as one shared `ret`, which is exactly how a null driver
looks and is a useful control when reading the real one.

Usage:
    python3 re/tools/drv_vectors.py <file.drv> [more.drv ...]
"""

import os
import sys


def vectors(data):
    """The leading run of `E9 rel16` jumps, with resolved targets."""
    out = []
    off = 0
    while off + 3 <= len(data) and data[off] == 0xE9:
        rel = int.from_bytes(data[off + 1 : off + 3], "little", signed=True)
        target = (off + 3 + rel) & 0xFFFF
        out.append((off, target))
        off += 3
    return out


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 0

    for path in args:
        if not os.path.exists(path):
            print(f"{path}: missing")
            continue
        data = open(path, "rb").read()
        table = vectors(data)
        print(f"\n=== {path} ({len(data)} bytes) ===")
        print(f"{len(table)} vector(s), table occupies {len(table) * 3:#x} bytes")

        seen = {}
        for index, (off, target) in enumerate(table):
            shared = ""
            if target in seen:
                shared = f"  (same target as vector {seen[target]})"
            else:
                seen[target] = index
            inside = "" if target < len(data) else "  !! past end of file"
            print(f"  vector {index:>2} @{off:#05x} -> {target:#06x}{shared}{inside}")

        # A far-pointer slot 0x14 bytes after another is 5 vectors later, since
        # the host stores 4-byte far pointers; printing the stride helps match
        # DS:0x0CDF and DS:0x0CF3 to entries.
        print(f"  (host far-pointer slots are 4 bytes apart: 0x14 apart = 5 vectors)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
