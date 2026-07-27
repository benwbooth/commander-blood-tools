#!/usr/bin/env python3
"""UNVERIFIED constants whose value is ALREADY explained in docs/audit-fixes.md.

#533 and #534 were both the same shape: a constant sitting uncited in one file
while the decode that explains it lived in the fix log, or in a doc comment in a
different file. #534 cost a 4096-base constraint solve to re-derive a segment
`bloodprg.rs` had already declared.

This finds them mechanically. For every UNVERIFIED constant in the ledger, take its
literal value and look for that value in `docs/audit-fixes.md` NEXT TO AN ADDRESS
(`@0x...`, `file 0x...`, `0x...`). A hit means the work is done and only the
citation is missing -- the cheapest possible ledger row to close.

Hits are LEADS, not citations. The fix log may be discussing a different cell that
happens to share a value, which is exactly the #501 trap; the address still has to
be read before it is written into a doc comment.

Usage:
    python3 tools/check_decoded_but_uncited.py [--all]
"""
import csv
import re
import sys

LEDGER = "docs/function-audit.tsv"
FIXES = "docs/audit-fixes.md"

# A constant's declared value, as written in the evidence column or its own line.
VALUE = re.compile(r"=\s*(0x[0-9A-Fa-f]{2,6}|\d{2,6})\s*;")
# An address mentioned in the fix log.
NEAR_ADDR = re.compile(r"(?:@|file\s+)`?(0x[0-9A-Fa-f]{4,6})")


def main():
    fixes = open(FIXES, encoding="utf-8", errors="replace").read()
    # index the fix log by line so a value and an address can be required to be close
    lines = fixes.splitlines()

    rows = [
        r
        for r in csv.DictReader(open(LEDGER, newline=""), delimiter="\t")
        if r["status"] == "UNVERIFIED" and r["kind"] == "const"
    ]

    src_cache = {}
    leads = []
    for r in rows:
        path = r["file"]
        if path not in src_cache:
            try:
                src_cache[path] = open(path, encoding="utf-8", errors="replace").read().splitlines()
            except OSError:
                src_cache[path] = []
        src = src_cache[path]
        idx = int(r["line"]) - 1
        if idx >= len(src):
            continue
        m = VALUE.search(src[idx])
        if not m:
            continue
        raw = m.group(1)
        # normalise: compare on the hex form, since the port writes some in decimal
        val = int(raw, 0)
        if val < 0x100:
            continue  # too small to attribute -- matches everything
        forms = {f"0x{val:x}", f"0x{val:X}", f"0x{val:04x}", f"0x{val:04X}", str(val)}

        for n, line in enumerate(lines):
            if not any(f in line for f in forms):
                continue
            # require an address nearby (same line or the two around it)
            window = "\n".join(lines[max(0, n - 2) : n + 3])
            if NEAR_ADDR.search(window):
                leads.append((path, r["line"], r["item"], raw, n + 1, line.strip()[:88]))
                break

    print(f"{len(rows)} UNVERIFIED constants; {len(leads)} have their value discussed "
          f"in {FIXES} beside an address\n")
    limit = None if "--all" in sys.argv else 25
    for path, line, item, raw, fixline, ctx in leads[:limit]:
        print(f"  {path}:{line} {item} = {raw}")
        print(f"      {FIXES}:{fixline}: {ctx}")
    if limit and len(leads) > limit:
        print(f"\n  ... {len(leads) - limit} more (--all)")
    print("\nLEADS, not citations: a shared value may be a different cell (#501).")
    print("Read the address before writing it into a doc comment.")


if __name__ == "__main__":
    main()
