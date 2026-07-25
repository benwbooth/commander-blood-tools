#!/usr/bin/env python3
"""Which UNVERIFIED ledger rows already have their evidence sitting in the tree?

audit-fixes #211 found `TALK_FIELD` and `LOCATION_FIELD` listed as needing work
while `field_matrix_entries_match_the_constants` had been asserting both against
the image all along. The evidence existed; nothing had recorded it. At 976
UNVERIFIED rows, that cannot be found by reading.

This reports rows whose evidence is ALREADY present, split by what kind:

  CITED     the item's doc carries a binary address -- it makes a decoded claim,
            so it belongs in the ASM? queue rather than in UNVERIFIED
  TESTED    a test both names the item AND opens something the game shipped, so
            something checks it against real data
  BOTH      cited and tested: settle-able now, with the evidence named

It SUGGESTS, it does not settle. A row is settled by a person who has looked at
the evidence -- the point here is to stop plausible-looking noise from hiding the
items that need actual work.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import csv
import os
import re
import sys

LEDGER = "docs/function-audit.tsv"
# Reading something the GAME shipped, rather than the port's own output.
REAL_DATA = re.compile(
    r"(BLOODPRG\.EXE|\.xdb|TB\.BIG|\.DIC|\.BAS|\.BIG|\.SPR|\.HNM|\.LBM|\.DES|"
    r"\.snd|\.drv|_tmp_dat|_tmp_iso|captures?/|iso_dir|fixture\(\))",
    re.I,
)
ADDR = re.compile(r"0x[0-9A-Fa-f]{3,6}")
DEF = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:fn|const|static|struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def doc_above(lines, index):
    """The `///` block immediately preceding a definition line."""
    out, i = [], index - 1
    while i >= 0:
        stripped = lines[i].strip()
        if stripped.startswith("///") or stripped.startswith("//"):
            out.append(stripped)
            i -= 1
            continue
        if stripped.startswith("#["):  # attributes sit between doc and item
            i -= 1
            continue
        break
    return "\n".join(reversed(out))


def tested_names():
    """path -> names REFERENCED by a data-reading test IN THAT SAME FILE.

    Two deliberate narrowings, both from this tool's first run, which reported
    259 "tested" rows including `parse`, `summary` and `header_size` -- generic
    identifiers that appear in some data-reading test SOMEWHERE in the tree:

      * per-FILE, not global. A Rust unit test normally sits beside its item, so
        requiring the same file removes cross-module name collisions, which is
        most of the noise.
      * a REFERENCE, not a mention: `name(`, `::name`, `.name(`, `name {`, or a
        SCREAMING_CASE constant standing alone. A word appearing in a comment or
        as somebody else's local variable does not count.
    """
    per_file = {}
    for base in ("src", "tests"):
        for root, _, files in os.walk(base):
            for f in sorted(files):
                if not f.endswith(".rs"):
                    continue
                path = os.path.join(root, f)
                text = open(path, encoding="utf-8", errors="replace").read()
                bodies = []
                for m in re.finditer(r"\bfn\s+[a-z_][a-z0-9_]*\s*\(\s*\)\s*\{", text):
                    start = m.end()
                    depth, i = 1, start
                    while i < len(text) and depth:
                        if text[i] == "{":
                            depth += 1
                        elif text[i] == "}":
                            depth -= 1
                        i += 1
                    body = text[start:i]
                    if REAL_DATA.search(body):
                        bodies.append(body)
                per_file[path] = "\n".join(bodies)
    return per_file


def referenced(body, name):
    """Does `body` actually USE `name`, rather than merely contain the word?"""
    if not body:
        return False
    if name.isupper():
        return re.search(rf"\b{re.escape(name)}\b", body) is not None
    return re.search(
        rf"(?:\b|::|\.){re.escape(name)}\s*(?:\(|\{{|::|\b)", body
    ) is not None


def main():
    rows = [
        r
        for r in csv.DictReader(open(LEDGER), delimiter="\t")
        if r["status"] == "UNVERIFIED"
    ]
    exercised = tested_names()

    cited, tested, both = [], [], []
    cache = {}
    for r in rows:
        path = r["file"]
        if not os.path.exists(path):
            continue
        if path not in cache:
            cache[path] = open(path, encoding="utf-8", errors="replace").read().splitlines()
        lines = cache[path]
        idx = int(r["line"]) - 1
        if idx >= len(lines):
            continue
        # Confirm the ledger line still points at this item before trusting it.
        m = DEF.match(lines[idx])
        if not m or m.group(1) != r["item"]:
            continue
        has_addr = bool(ADDR.search(doc_above(lines, idx)))
        has_test = referenced(exercised.get(path, ""), r["item"])
        if has_addr and has_test:
            both.append(r)
        elif has_addr:
            cited.append(r)
        elif has_test:
            tested.append(r)

    # `--all` prints every row (for feeding a settle loop); the default caps the
    # listing so the SUMMARY line stays the thing you read.
    limit = 10_000 if "--all" in sys.argv else 40
    for label, group in (("BOTH", both), ("CITED", cited), ("TESTED", tested)):
        for r in group[:limit]:
            print(f"{label} {r['file']}:{r['line']}: {r['item']}")
        if len(group) > limit:
            print(f"   ... and {len(group) - limit} more {label} (--all to list)")

    print(
        f"{len(rows)} UNVERIFIED row(s): {len(both)} are BOTH cited and exercised "
        f"by a data test, {len(cited)} carry a citation only, {len(tested)} are "
        f"exercised only, {len(rows) - len(both) - len(cited) - len(tested)} have "
        "neither and are the real queue"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
