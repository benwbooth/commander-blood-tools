#!/usr/bin/env python3
"""Flag CAPTURE-SOURCED provenance claims in the port's runtime code.

The prime rule: behaviour comes from the assembly; the oracle only VERIFIES. A
comment saying a constant was "measured from" a capture is therefore either a
defect or a stale note left by the fix that removed it. This session found three
of each -- the choice box's colours, the list menu's x, the save UI's whole
layout were defects; the hand atlas, the square-caps advances and the viewscreen
band were notes outliving their code.

Test code is EXEMPT: comparing rendered output against a capture is exactly what
the oracle is for. Only `///`, `//!` and `//` comments outside `#[cfg(test)]`
count, and a line is cleared by naming a binary address near the claim (that is
the shape of "was measured, now derived, here is the routine").

Exit non-zero when an unexplained claim survives, so the class cannot come back
without someone seeing it.
"""

import os
import re
import sys

PHRASES = re.compile(
    r"(measured from|measured off|harvested (?:from|pixel|per)|captured from|"
    r"capture-matched|read off (?:a|the) (?:screenshot|capture)|oracle-measured)",
    re.I,
)
# A claim is EXPLAINED when the same comment run also cites a binary address or
# says the value is now derived.
EXPLAINED = re.compile(
    r"(0x[0-9A-Fa-f]{3,6}|DERIV|derived|no longer|used to|previously|rather than|"
    r"instead of|stale|was gone|not harvested|NOT a capture)",
)
# The ORACLE HARNESS is exempt: measuring the real game is what it is FOR. The
# prime rule constrains where the PORT's behaviour comes from, not what the probe
# binary observes. Everything else under src/ is port runtime code.
EXEMPT = {os.path.join("src", "bin", "runtime_boot.rs")}


def comment_runs(lines):
    """Yield (start_line, [lines]) for each run of consecutive comment lines."""
    run, start = [], 0
    for i, line in enumerate(lines + [""], 1):
        st = line.strip()
        if st.startswith("///") or st.startswith("//!") or st.startswith("//"):
            if not run:
                start = i
            run.append(st)
            continue
        if run:
            yield start, run
            run = []


def main():
    problems = []
    checked = 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            if path in EXEMPT:
                continue
            text = open(path, encoding="utf-8", errors="replace").read()
            body = text.split("#[cfg(test)]")[0]  # runtime code only
            lines = body.splitlines()
            for start, run in comment_runs(lines):
                blob = " ".join(run)
                if not PHRASES.search(blob):
                    continue
                checked += 1
                if not EXPLAINED.search(blob):
                    snippet = PHRASES.search(blob).group(0)
                    problems.append(f"{path}:{start}: unexplained '{snippet}' claim")
    for p in problems:
        print("PROVENANCE " + p)
    print(
        f"{checked} capture-provenance claim(s) in runtime code, "
        f"{len(problems)} without a citation or a 'no longer' note"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
