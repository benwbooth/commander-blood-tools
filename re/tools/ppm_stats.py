#!/usr/bin/env python3
"""Measure a capture before concluding anything about it.

audit-fixes #114 withdrew a defect claim that had been reasoned from a capture's
APPEARANCE. The arithmetic ruled it out afterwards: the surface was presentation
static, two colours with a mean run of 1.87 pixels, and the port's 33-pixel star
layer could not have produced 19855 white pixels either way.

So: measure first. This prints the shape of a PPM without interpreting it —
dimensions, distinct colours, the dominant ones, mean horizontal run length (low
means noise/dither, high means flat regions), and a coarse row profile so bands
and letterboxing are visible as numbers rather than impressions.

Usage:
    python3 re/tools/ppm_stats.py <file.ppm> [more.ppm ...]
"""

import collections
import sys


def read_ppm(path):
    with open(path, "rb") as fh:
        data = fh.read()
    if not data.startswith(b"P6"):
        raise ValueError(f"{path}: not a binary PPM")
    # Header: P6 <w> <h> <maxval>, whitespace-separated, # comments allowed.
    fields, pos = [], 2
    while len(fields) < 3:
        while pos < len(data) and data[pos : pos + 1].isspace():
            pos += 1
        if data[pos : pos + 1] == b"#":
            while pos < len(data) and data[pos] != 0x0A:
                pos += 1
            continue
        start = pos
        while pos < len(data) and not data[pos : pos + 1].isspace():
            pos += 1
        fields.append(int(data[start:pos]))
    pos += 1
    w, h, _maxval = fields
    return w, h, data[pos : pos + w * h * 3]


def main():
    paths = sys.argv[1:]
    if not paths:
        print(__doc__)
        return 0

    for path in paths:
        w, h, px = read_ppm(path)
        print(f"\n=== {path} ===")
        print(f"{w}x{h}, {len(px)} bytes of pixel data")

        counts = collections.Counter()
        for i in range(0, len(px), 3):
            counts[px[i : i + 3]] += 1
        total = sum(counts.values())
        print(f"{len(counts)} distinct colour(s)")
        for colour, n in counts.most_common(6):
            print(f"   #{colour.hex()}  {n:>7} px  {100.0 * n / total:5.1f}%")

        # Mean horizontal run: dither/noise ~= 1-2, flat art is much longer.
        runs = changes = 0
        for y in range(h):
            row = px[y * w * 3 : (y + 1) * w * 3]
            runs += 1
            for x in range(1, w):
                if row[x * 3 : x * 3 + 3] != row[(x - 1) * 3 : x * 3]:
                    changes += 1
                    runs += 1
        print(f"mean horizontal run: {(w * h) / max(runs, 1):.2f} px")

        # Row profile: how many distinct colours each band of rows uses.
        band = max(1, h // 10)
        print("row bands (rows: distinct colours, dominant share):")
        for top in range(0, h, band):
            bottom = min(h, top + band)
            c = collections.Counter()
            for y in range(top, bottom):
                row = px[y * w * 3 : (y + 1) * w * 3]
                for x in range(w):
                    c[row[x * 3 : x * 3 + 3]] += 1
            n = sum(c.values())
            share = 100.0 * c.most_common(1)[0][1] / n if n else 0.0
            print(f"   {top:>3}-{bottom - 1:<3} {len(c):>4} colours, dominant {share:5.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
