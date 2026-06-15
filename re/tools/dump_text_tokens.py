#!/usr/bin/env python3
"""Parse 0xA6 TEXT tokens out of a SCRIPT*.COD and tabulate their 5-byte
parameter block, to expose which fields vary per dialogue line.

Token layout (from token_advance @0x62B6 and token_walker @0x73AF):
    A6  b1 b2 b3 b4 b5   w0 w1 ... 0x0000
    b1..b5 = 5 param bytes; b5 has bit 0x80 set (engine "active" flag).
    w* = u16 dictionary-word offsets, 0x0000-terminated.

Usage: python3 re/tools/dump_text_tokens.py <SCRIPT.COD> [--limit N]
"""
import collections
import sys


def main():
    path = sys.argv[1]
    limit = 100000
    if "--limit" in sys.argv:
        limit = int(sys.argv[sys.argv.index("--limit") + 1])
    d = open(path, "rb").read()
    print(f"# {path}  size={len(d)}")
    print(f"{'off':>7}  b1   b2   b3   b4   b5   nwords  words")
    i = 0
    rows = 0
    field_vals = [collections.Counter() for _ in range(5)]
    while i < len(d) and rows < limit:
        if d[i] != 0xA6:
            i += 1
            continue
        p = d[i + 1:i + 6]
        if len(p) < 5:
            break
        j = i + 6
        words = []
        while j + 1 < len(d):
            w = d[j] | (d[j + 1] << 8)
            j += 2
            if w == 0:
                break
            words.append(w)
        for k in range(5):
            field_vals[k][p[k]] += 1
        wstr = " ".join(f"{w:#06x}" for w in words[:8])
        print(f"{i:#07x}  {p[0]:#04x} {p[1]:#04x} {p[2]:#04x} "
              f"{p[3]:#04x} {p[4]:#04x}  {len(words):>5}  {wstr}")
        rows += 1
        i = j
    print(f"\n# {rows} text tokens")
    for k in range(5):
        common = field_vals[k].most_common(8)
        vals = " ".join(f"{v:#04x}x{c}" for v, c in common)
        print(f"# b{k+1} distinct={len(field_vals[k])}: {vals}")


if __name__ == "__main__":
    main()
