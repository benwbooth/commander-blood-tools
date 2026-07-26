#!/usr/bin/env python3
"""Print the canonical status line, so it is never typed from memory.

Four times in one session an audit-fixes entry carried a wrong number — #295
said 585 citations for 583, #320 said 617 for 613, #329 said 617 for 620, #372
said 1075 confirmed as 1076. Every one came from writing the figure while
composing the entry and reading the tool afterwards, or not at all.

audit-fixes #319 already drew the conclusion for INSTRUCTION counts ("run the
tool, paste the number, do not predict it") and it kept not being applied to the
summary line, because the summary felt like prose rather than a measurement. It
is a measurement. This prints it.

The counting rule is the STRICT one (audit-fixes #286a): a row whose status ends
in `?` is PROVISIONAL and counts as OPEN, because the `?` statuses are heuristic
guesses that no one has checked. The lenient reading — counting them as settled —
is roughly eleven points higher and is not what this project reports.

Usage (from the repo root, with PYTHONSAFEPATH=1):

    python3 tools/audit_status.py            one line, ready to paste
    python3 tools/audit_status.py --verbose  plus the per-status breakdown
"""

import collections
import csv
import os
import subprocess
import sys

LEDGER = "docs/function-audit.tsv"


def ledger_counts():
    rows = list(csv.DictReader(open(LEDGER), delimiter="\t"))
    by = collections.Counter(r["status"] for r in rows)
    total = sum(by.values())
    provisional = sum(v for k, v in by.items() if k.endswith("?"))
    unverified = by.get("UNVERIFIED", 0)
    confirmed = total - provisional - unverified
    return total, confirmed, provisional, unverified, by


def citations():
    """(verified, wrong) from the citation guard, or None if it cannot run."""
    try:
        out = subprocess.run(
            [sys.executable, "tools/check_cited_instructions.py"],
            capture_output=True,
            text=True,
            timeout=300,
        ).stdout
    except Exception:
        return None
    for line in out.splitlines():
        if "cited instructions verified" in line:
            parts = line.split()
            verified = int(parts[0])
            wrong = int(parts[parts.index("wrong") - 1])
            return verified, wrong
    return None


def main():
    if not os.path.exists(LEDGER):
        print("no ledger found")
        return 1
    total, confirmed, provisional, unverified, by = ledger_counts()
    pct = 100.0 * confirmed / total if total else 0.0

    cites = citations()
    cite_text = ""
    if cites:
        verified, wrong = cites
        cite_text = f" {verified} citations verified, {wrong} wrong."

    print(
        f"{total} items, {confirmed} confirmed ({pct:.1f}%), "
        f"{provisional + unverified} open "
        f"({unverified} UNVERIFIED + {provisional} provisional).{cite_text}"
    )
    if "--verbose" in sys.argv:
        for status, n in sorted(by.items()):
            mark = "  open" if status.endswith("?") or status == "UNVERIFIED" else "  settled"
            print(f"  {status:<12} {n:>5}{mark}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
