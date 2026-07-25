#!/usr/bin/env python3
"""Which port items cite an address that ALREADY HAS A LIFT?

A lifted function is oracle-verified ground truth sitting in the tree. Where the
port also has a hand-written implementation of the same address, the two can be
run side by side — the cheapest verification available here, because nothing new
has to be decoded or captured.

Six such differentials found one live defect, one latent divergence and one wrong
citation (docs/audit-fixes.md #80-#86); a seventh (#90) found the port's tint
builder had a lift all along, filed under a name for something else. That last
one is why this matches by ADDRESS: names drift, addresses do not.

Lifts are collected from every recomp module (`auto.rs`, `mod.rs`,
`ptr_leaves_gen.rs`, ...) as `fn func_<hex>`. Port citations come from the audit
ledger's origin column, so anything with a `0x...` in its doc is covered.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import csv
import os
import re
import sys

LEDGER = "docs/function-audit.tsv"
RECOMP = os.path.join("src", "recomp")
SETTLED = {"ORACLE", "ASM", "DATA", "INFRA", "TESTED"}


def lifted_addresses():
    out = {}
    for root, _, files in os.walk(RECOMP):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            for m in re.finditer(r"fn (func_([0-9a-f]+))\s*\(", text):
                out.setdefault(int(m.group(2), 16), []).append(f"{f}::{m.group(1)}")
    return out


def main():
    lifts = lifted_addresses()
    if not os.path.exists(LEDGER):
        print("no ledger")
        return 0
    rows = list(csv.DictReader(open(LEDGER), delimiter="\t"))

    pairs, done = [], []
    for r in rows:
        if r["file"].startswith(os.path.join("src", "recomp")):
            continue
# NOT preceded by an alphanumeric: "320x200" contains the substring "0x200",
# so a plain `0x[0-9A-Fa-f]{3,6}` harvested a PHANTOM citation from every
# screen-dimension string in a doc. 11 ledger rows were provisionally ASM?
# on that basis alone -- evidenced-looking rows with no evidence.
        for a in re.findall(r"(?<![0-9A-Za-z])0x([0-9A-Fa-f]{3,6})", r["origin"]):
            addr = int(a, 16)
            if addr not in lifts:
                continue
            entry = (r["item"], r["file"], addr, r["status"], ", ".join(lifts[addr]))
            (done if r["status"] == "ORACLE" else pairs).append(entry)
            break

    print(f"{len(lifts)} lifted addresses; {len(pairs) + len(done)} port items cite one\n")
    print(f"ALREADY DIFFERENTIALLED (status ORACLE): {len(done)}")
    for item, path, addr, _, lift in sorted(done):
        print(f"    {item:<38} {path:<22} {addr:#07x}  {lift}")
    print(f"\nCANDIDATES — a lift exists, the row is not ORACLE: {len(pairs)}")
    for item, path, addr, status, lift in sorted(pairs):
        print(f"    {item:<38} {path:<22} {addr:#07x}  {status:<7} {lift}")
    print(
        "\nEach candidate can be verified by running the lift beside the native "
        "implementation.\nNothing needs decoding or capturing first."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
