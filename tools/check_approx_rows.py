#!/usr/bin/env python3
"""Every APPROX in the code must have a matrix row naming its replacement.

CLAUDE.md's rule is exact: a capture-measured constant "may stand in temporarily
only if the row in docs/port-validation.md explicitly labels it APPROX with the
binary routine that must replace it. Finding that routine is the actual task."

Nothing checked that. `PHONE_CONTACTS` carried an APPROX-shaped comment through
SIX audit entries (#326, #327, #328, #437, #438, #439) while the matrix graded its
screen `DATA+ORACLE` and never mentioned the literal (audit-fixes #440). Every
pass verified the ADDRESSES and none asked whether the table should exist.

So this pairs the two sides:

  UNPAIRED   an item whose doc calls itself APPROX / FABRICATED / a stand-in, with
             no row in docs/port-validation.md mentioning that item by name. This
             is the #440 shape and the bucket to act on.
  paired     the matrix names it; counted only.

Matching is by IDENTIFIER, not by prose: the item's own name (`PHONE_CONTACTS`,
`NAV_DEST_X`) must appear somewhere in port-validation.md. A row that gestures at
"the phone screen" without naming the literal does not count -- that is exactly
the gap #440 found, and a looser match would have hidden it.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

MATRIX = os.path.join("docs", "port-validation.md")
# A doc comment admitting the item is a stand-in.
#
# POLARITY MATTERS. The first version matched the word anywhere and reported
# `exit_query`, whose doc says the game's behaviour is "the same ... rather than an
# approximation of it" -- a NEGATION. Same trap check_labels.py documents for
# mnemonics in prose: the word is not the claim. These prefixes flip the meaning
# and are excluded.
NEGATED = re.compile(
    r"(?i)(not|never|no longer|rather than|instead of|isn't|is not|was|used to|"
    r"previously|no)\s+(an?\s+)?(APPROX\w*|FABRICATED|stand-?in\b|invented|transcrib\w*)"
)
# `stand-?in` without a trailing boundary matches "STANDING" -- it flagged
# FIELD_OFFSETS ("the port's standing ...") and DIALOGUE_FONT_ASCII_MAP_LEN
# ("left standing here"), neither of which admits anything. Third false-positive
# class in this one tool; the pattern is that an English word containing a
# keyword is not the keyword.
FLAG = re.compile(r"(?i)\b(APPROX|FABRICATED|stand-?in\b|invented|transcrib)")
# `pub const NAME`, `const NAME`, `pub fn name`, `fn name`, `pub struct Name`.
DECL = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static|fn|struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def main():
    if not os.path.exists(MATRIX):
        print("no validation matrix; nothing checked")
        return 0
    matrix = open(MATRIX, encoding="utf-8", errors="replace").read()

    unpaired, paired = [], 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            cut = next((i for i, l in enumerate(lines) if "#[cfg(test)]" in l), len(lines))
            doc = []
            for i, line in enumerate(lines[:cut]):
                stripped = line.lstrip()
                if stripped.startswith(("///", "//!", "//")):
                    doc.append(stripped)
                    continue
                m = DECL.match(line)
                if m and doc:
                    block = "\n".join(doc)
                    flagged = FLAG.search(block)
                    if flagged and not NEGATED.search(block):
                        name = m.group(1)
                        if re.search(rf"\b{re.escape(name)}\b", matrix):
                            paired += 1
                        else:
                            unpaired.append((path, i + 1, name))
                doc = []

    for path, line, name in unpaired:
        print(f"UNPAIRED {path}:{line}: {name} calls itself a stand-in; not named in {MATRIX}")

    print(
        f"{paired + len(unpaired)} item(s) whose doc admits a stand-in: "
        f"{paired} named in the matrix, {len(unpaired)} UNPAIRED"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
