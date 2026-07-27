#!/usr/bin/env python3
"""Which binary addresses are implemented by MORE THAN ONE port function?

A decoded rule with two copies is how verification stops covering the code that
uses it: the differential attaches to one function, the other drifts unwatched.
This session found three real instances that way —

  * `subtitle_draw_glyph` in both `font.rs` and `extract/render.rs`, the second
    carrying a 128-entry font map, Unicode-indexed lookups and a `'?'` fallback,
    all fixed in the first (audit-fixes #97);
  * two field-offset resolvers, only one swept against `func_6023` (#96);
  * two per-kind hit-box ladders and two marker box tests, only one verified
    against `func_92a3` (#96, #98).

Not every collision is duplication: a routine and its helper, or a caller and its
callee, legitimately cite the same address. The tool reports clusters for a human
to judge and flags the strongest signal — the SAME FUNCTION NAME in two files —
as an error, since that is duplication almost by definition.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import collections
import csv
import os
import re
import sys

LEDGER = "docs/function-audit.tsv"


def main():
    if not os.path.exists(LEDGER):
        print("no ledger")
        return 0
    rows = [
        r
        for r in csv.DictReader(open(LEDGER), delimiter="\t")
        if r["kind"] == "fn" and not r["file"].startswith(os.path.join("src", "recomp"))
    ]

    by_addr = collections.defaultdict(list)
    for r in rows:
# NOT preceded by an alphanumeric: "320x200" contains the substring "0x200",
# so a plain `0x[0-9A-Fa-f]{3,6}` harvested a PHANTOM citation from every
# screen-dimension string in a doc. 11 ledger rows were provisionally ASM?
# on that basis alone -- evidenced-looking rows with no evidence.
# An origin may name the OVERLAY space (`XDB:manu3:0x19B`). Key by SPACE plus
# address so an overlay offset can never compare equal to an image address at the
# same number -- the spaces are unrelated, and manu3.xdb's method entries sit
# exactly where small image offsets would (audit-fixes #485).
        m = re.search(
            r"(?:(XDB:[A-Za-z0-9_]+):)?(?<![0-9A-Za-z])0x([0-9A-Fa-f]{3,6})", r["origin"]
        )
        if m:
            key = (m.group(1) or "IMG", int(m.group(2), 16))
            by_addr[key].append((r["item"], r["file"], r["status"]))

    clusters = {a: v for a, v in by_addr.items() if len(v) > 1}
    # The strongest signal: one NAME implemented twice for one address. Two files
    # is the obvious case; the SAME file also counts, because Rust allows one name
    # per impl block and `owner_object_offset` was written out twice that way — in
    # `ExecutionContext` and in `VmMachine`, identical bodies, invisible to a
    # cross-file check.
    same_name = []
    for addr, items in clusters.items():
        names = collections.Counter(i[0] for i in items)
        for name, n in names.items():
            if n > 1:
                files = [f for i, f, _ in items if i == name]
                same_name.append((addr, name, files))

    def show(key):
        """`(space, addr)` -> a printable, space-qualified address."""
        space, addr = key
        return f"{addr:#07x}" if space == "IMG" else f"{space}:{addr:#05x}"

    print(f"{len(clusters)} addresses cited by more than one port function\n")
    for addr, items in sorted(clusters.items()):
        print(f"  {show(addr)}")
        for item, path, status in sorted(items):
            print(f"      {item:<42} {path:<24} {status}")

    if same_name:
        print("\nDUPLICATE NAMES — one name implemented twice for one address:")
        for addr, name, files in sorted(same_name):
            print(f"  {show(addr)}  {name}  in {', '.join(files)}")
        return 1
    print("\nNo same-name duplicates. Clusters above are for judgement: a routine")
    print("and its helper may share an address legitimately.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
