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
        m = re.search(r"0x([0-9A-Fa-f]{3,6})", r["origin"])
        if m:
            by_addr[int(m.group(1), 16)].append((r["item"], r["file"], r["status"]))

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

    print(f"{len(clusters)} addresses cited by more than one port function\n")
    for addr, items in sorted(clusters.items()):
        print(f"  {addr:#07x}")
        for item, path, status in sorted(items):
            print(f"      {item:<42} {path:<24} {status}")

    if same_name:
        print("\nDUPLICATE NAMES — one name implemented twice for one address:")
        for addr, name, files in sorted(same_name):
            print(f"  {addr:#07x}  {name}  in {', '.join(files)}")
        return 1
    print("\nNo same-name duplicates. Clusters above are for judgement: a routine")
    print("and its helper may share an address legitimately.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
