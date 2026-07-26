#!/usr/bin/env python3
"""What does this project ALREADY know about an address?

Written after audit-fixes #128. #109 decoded the console row-handler table from
the binary — `cs:[bx+0xF29]`, file `0x8709`, entries `0x8713`/`0x872C`/`0x87BD`/
`0x8848`/`0x886C` — and `ship3d.rs` already had all five as
`run_ship_3d_nav_choice_handler_0..4`, settled ASM. The values agreed, so the
rediscovery cost only time; the disagreement it exposed (#128) was worth more than
the table. But searching the BINARY before searching the SOURCE is backwards, and
one command fixes it.

Prints, for an address:

  * every `re/labels.csv` row naming it (its own row, and rows citing it)
  * every ledger row whose origin includes it, with status
  * every source line mentioning it

Usage:
    python3 re/tools/whatis.py 0x8709 [more addresses...]

Accepts bare or `0x` hex, and matches `DS:`-prefixed forms too.
"""

import csv
import os
import re
import sys

LEDGER = os.path.join("docs", "function-audit.tsv")
LABELS = os.path.join("re", "labels.csv")


def variants(value):
    """The spellings an address appears under in this tree."""
    out = set()
    for width in (3, 4, 5, 6):
        out.add(f"0x{value:0{width}X}")
        out.add(f"0x{value:0{width}x}")
    out.add(f"0x{value:X}")
    out.add(f"0x{value:x}")
    return out


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 0

    for arg in args:
        value = int(arg, 16)
        forms = variants(value)
        print(f"\n=== {value:#07x} ===")

        if os.path.exists(LABELS):
            rows = []
            for n, line in enumerate(open(LABELS, encoding="utf-8"), 1):
                if any(f in line for f in forms):
                    own = line.split(",", 1)[0].strip()
                    kind = "OWN ROW" if any(f == own or f in own for f in forms) else "cites"
                    rows.append((n, kind, line.strip()[:150]))
            print(f"labels.csv: {len(rows)} row(s)")
            for n, kind, text in rows[:6]:
                print(f"   {n:>4} [{kind}] {text}")
            if len(rows) > 6:
                # The count above is not enough on its own: a reader who sees six
                # lines assumes six unless told otherwise (audit-fixes #310).
                print(f"   ... {len(rows) - 6} more row(s) not shown")

        if os.path.exists(LEDGER):
            hits = []
            for r in csv.DictReader(open(LEDGER), delimiter="\t"):
                if any(f in r["origin"] for f in forms):
                    hits.append(r)
            print(f"ledger: {len(hits)} row(s) cite it")
            for r in hits[:8]:
                print(f"   {r['status']:<10} {r['item']:<38} {r['file']}:{r['line']}")

        src = []
        for root, _, files in os.walk("src"):
            for f in sorted(files):
                if not f.endswith(".rs"):
                    continue
                p = os.path.join(root, f)
                for n, line in enumerate(
                    open(p, encoding="utf-8", errors="replace").read().splitlines(), 1
                ):
                    if any(form in line for form in forms):
                        src.append(f"   {p}:{n}: {line.strip()[:110]}")
        print(f"source: {len(src)} line(s) mention it")
        for line in src[:8]:
            print(line)
        if len(src) > 8:
            print(f"   ... and {len(src) - 8} more")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
