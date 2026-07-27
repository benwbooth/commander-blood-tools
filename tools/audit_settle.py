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
    status, wanted = sys.argv[1], set(a for a in sys.argv[2:] if a != "--no-verify")

    # SETTLE ONLY ON A GREEN SUITE (audit-fixes #537).
    #
    # Settling is a claim that a row was checked against the binary. Three times
    # (#423, #505, #536) I settled rows while the suite was RED -- twice by
    # chaining the settle onto the same shell command as the test run and reading
    # the output afterwards. Each time the fix was easy and the ordering was luck.
    # A claim recorded against a failing tree is not a claim about anything, so the
    # tool now refuses rather than trusting the operator to look first.
    #
    # `--no-verify` exists for the one legitimate case: putting a mis-settled row
    # BACK to UNVERIFIED, which must work even when something is broken.
    if "--no-verify" not in sys.argv and status != "UNVERIFIED":
        import subprocess

        probe = subprocess.run(
            ["cargo", "test", "--release", "--lib", "--bins", "--quiet"],
            capture_output=True,
            text=True,
        )
        if probe.returncode != 0:
            tail = [ln for ln in probe.stdout.splitlines() if "FAILED" in ln or "panicked" in ln]
            print("REFUSING TO SETTLE: the test suite is not green.")
            for ln in tail[:5]:
                print(f"  {ln}")
            print("\nFix the tree first. `--no-verify` is for reverting to UNVERIFIED only.")
            return 1

    with open(LEDGER, newline="") as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        fields, rows = reader.fieldnames, list(reader)

    counts = Counter((r["item"], r["file"]) for r in rows)
    changed, unchanged, refused, missing = [], [], [], set(wanted)
    for r in rows:
        # `file:line:item` disambiguates a name that appears more than once in a
        # file -- local constants inside different functions share names (three
        # `TEXT` colour constants in engine.rs), and without a line-qualified form
        # they were unsettleable no matter how well evidenced.
        keys = {
            r["item"],
            f"{r['file']}:{r['item']}",
            f"{r['file']}:{r['line']}:{r['item']}",
        }
        hit = keys & wanted
        if not hit:
            continue
        missing -= keys
        line_qualified = f"{r['file']}:{r['line']}:{r['item']}" in wanted
        if counts[(r["item"], r["file"])] > 1 and not line_qualified:
            refused.append(
                f"{r['item']}  {r['file']}  (name not unique in file -- qualify as "
                f"{r['file']}:{r['line']}:{r['item']})"
            )
            continue
        if not r["origin"] and status == "ASM":
            refused.append(f"{r['item']}  {r['file']}  (ASM needs a cited address)")
            continue
        if r["status"] == status:
            # Already at this status. Reporting it as "settled" makes a no-op look
            # like progress, which over a long campaign inflates the sense of
            # movement -- two rows were re-settled in #135 and the ledger total did
            # not move, because nothing had changed.
            unchanged.append(f"{r['item']}  {r['file']}:{r['line']}")
            continue
        r["status"] = status
        changed.append(f"{r['item']}  {r['file']}:{r['line']}")

    with open(LEDGER, "w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)

    print(f"settled {len(changed)} row(s) as {status}"
          + (f"; {len(unchanged)} already {status}" if unchanged else ""))
    for line in changed:
        print(f"  {line}")
    for line in refused:
        print(f"  REFUSED {line}")
    for item in sorted(missing):
        print(f"  NOT IN LEDGER {item}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
