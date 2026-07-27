#!/usr/bin/env python3
"""Census of far calls into the RENDER DRIVER segment, and what cites them.

`src/bloodprg.rs` names ~25 `RENDER_*_OFFSET` constants. They are not values --
they are ENTRY POINTS in the driver segment the game reaches with
`lcall 0x299, <offset>` (encoded `9A off16 seg16`). So each one's evidence is a
real call site, and a constant with no call site anywhere in the image is a claim
with nothing behind it.

This scans every far call in BLOODPRG.EXE, keeps those whose segment matches, and
reports each offset with its call sites -- then cross-checks the constants in
bloodprg.rs against that census in both directions.

Usage: python3 re/tools/render_driver_calls.py [segment_hex]
"""
import re
import sys
import struct
from collections import defaultdict

EXE = "re/bin/BLOODPRG.EXE"
SRC = "src/bloodprg.rs"
DEFAULT_SEG = 0x299


def main():
    seg = int(sys.argv[1], 16) if len(sys.argv) > 1 else DEFAULT_SEG
    data = open(EXE, "rb").read()

    sites = defaultdict(list)
    for i in range(len(data) - 5):
        if data[i] != 0x9A:
            continue
        off, s = struct.unpack_from("<HH", data, i + 1)
        if s == seg:
            sites[off].append(i)
    print(f"far calls to segment {seg:#05x}: {len(sites)} distinct offset(s), "
          f"{sum(len(v) for v in sites.values())} call site(s)\n")

    # constants declared in the port
    consts = {}
    for m in re.finditer(
        r"pub const (RENDER_\w+_OFFSET): u16 = (0x[0-9a-fA-F]+|\d+);", open(SRC).read()
    ):
        consts[m.group(1)] = int(m.group(2), 0)

    named = {v: k for k, v in consts.items()}
    print(f"{len(consts)} RENDER_*_OFFSET constants in {SRC}\n")

    matched, unmatched = [], []
    for name, val in sorted(consts.items(), key=lambda kv: kv[1]):
        if val in sites:
            matched.append((name, val, sites[val]))
        else:
            unmatched.append((name, val))

    print(f"CITED BY A CALL SITE ({len(matched)}):")
    for name, val, where in matched:
        where_s = ", ".join(f"{w:#07x}" for w in where[:3])
        more = f" +{len(where) - 3} more" if len(where) > 3 else ""
        print(f"  {val:#06x}  {name:<46} {len(where):>2} site(s): {where_s}{more}")

    if unmatched:
        print(f"\nNO FAR-CALL SITE ({len(unmatched)}) -- reached another way, or wrong:")
        for name, val in unmatched:
            print(f"  {val:#06x}  {name}")

    extra = sorted(o for o in sites if o not in named)
    if extra:
        print(f"\nCALLED BUT UNNAMED ({len(extra)}): "
              + ", ".join(f"{o:#06x}" for o in extra[:20]))


if __name__ == "__main__":
    main()
