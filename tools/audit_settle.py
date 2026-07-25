#!/usr/bin/env python3
"""Settle audit-ledger rows by hand: `audit_settle.py STATUS item [item...]`.

The ledger's heuristics only ever produce the provisional `?` statuses; a settled
one is a claim that a human checked the row against the disassembly (ASM), a
differential run (ORACLE), a decoded data layout (DATA), or that it has no binary
counterpart (INFRA).  Writing them through this script rather than by hand keeps
the file's shape intact and REFUSES to settle a name that occurs more than once
in its file, which is exactly the case `audit_inventory.py` cannot carry forward.
"""
import csv
import sys
from collections import Counter

LEDGER = "docs/function-audit.tsv"
SETTLED = {"ASM", "ORACLE", "DATA", "INFRA", "TESTED"}
# UNVERIFIED is allowed so a mis-settled row can be put BACK — settling the wrong
# function is the failure mode this tool exists to prevent, and it needs an undo.
ALLOWED = SETTLED | {"UNVERIFIED"}


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in ALLOWED:
        print(
            f"usage: {sys.argv[0]} {{{'|'.join(sorted(ALLOWED))}}} item|file:item [...]\n"
            "  A bare NAME settles that name in EVERY file it appears in, which is\n"
            "  rarely what you want for common names like `run` or `main` — qualify\n"
            "  them as `src/gpu.rs:run` instead."
        )
        return 2
    status, wanted = sys.argv[1], set(sys.argv[2:])

    with open(LEDGER, newline="") as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        fields, rows = reader.fieldnames, list(reader)

    counts = Counter((r["item"], r["file"]) for r in rows)
    changed, refused, missing = [], [], set(wanted)
    for r in rows:
        keys = {r["item"], f"{r['file']}:{r['item']}"}
        hit = keys & wanted
        if not hit:
            continue
        missing -= keys
        if counts[(r["item"], r["file"])] > 1:
            refused.append(f"{r['item']}  {r['file']}  (name not unique in file)")
            continue
        if not r["origin"] and status == "ASM":
            refused.append(f"{r['item']}  {r['file']}  (ASM needs a cited address)")
            continue
        r["status"] = status
        changed.append(f"{r['item']}  {r['file']}:{r['line']}")

    with open(LEDGER, "w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)

    print(f"settled {len(changed)} row(s) as {status}")
    for line in changed:
        print(f"  {line}")
    for line in refused:
        print(f"  REFUSED {line}")
    for item in sorted(missing):
        print(f"  NOT IN LEDGER {item}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
