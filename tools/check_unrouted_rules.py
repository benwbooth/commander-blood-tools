#!/usr/bin/env python3
"""Which decoded rules does nothing actually RUN?

Two real defects came from this shape, and neither was visible from the function
itself:

  * `bas_vm::parse_menu_block` walks a `0xA3` menu head's word list — decoded,
    correct, and called from nowhere, while `main.rs` hardcoded the menus it could
    have built (audit-fixes #108).
  * `vm::apply_operator` implements the `0x6863` operator ladder — decoded,
    correct, called only by its own tests, while the exec loop carried a SECOND
    inline copy that got the ladder's fallthrough wrong (#126). The correct
    implementation was the dead one.

So a function with a binary citation and no runtime caller is worth looking at: it
is either unwired (a feature the port has decoded but does not use) or duplicated
(the live copy is somewhere else, and may not agree).

A caller inside `#[cfg(test)]` does NOT count. That is the whole point — a rule
exercised only by its own tests is verified against itself and connected to
nothing.

Reported for judgement, not failed: a `pub` helper may legitimately exist for the
tools or for a future call site. Run with PYTHONSAFEPATH=1 from the repo root.
"""

import csv
import os
import re
import sys

LEDGER = "docs/function-audit.tsv"
SKIP_DIRS = (os.path.join("src", "recomp"),)
DEF = re.compile(r"^\s*pub(?:\([^)]*\))?\s+fn\s+([a-z_][a-z0-9_]*)\s*[(<]")


def runtime_text(path):
    """A file's source with every #[cfg(test)] module removed."""
    text = open(path, encoding="utf-8", errors="replace").read()
    out, i = [], 0
    while True:
        j = text.find("#[cfg(test)]", i)
        if j < 0:
            out.append(text[i:])
            break
        out.append(text[i:j])
        # skip to the end of the following item's brace block
        k = text.find("{", j)
        if k < 0:
            break
        depth, m = 0, k
        while m < len(text):
            if text[m] == "{":
                depth += 1
            elif text[m] == "}":
                depth -= 1
                if depth == 0:
                    break
            m += 1
        i = m + 1
    return "".join(out)


def main():
    origins = {}
    if os.path.exists(LEDGER):
        for r in csv.DictReader(open(LEDGER), delimiter="\t"):
            if r["kind"] == "fn" and r["origin"]:
                origins.setdefault((r["file"], r["item"]), r["origin"])

    files = []
    for root, _, names in os.walk("src"):
        if any(root.startswith(d) for d in SKIP_DIRS):
            continue
        for f in sorted(names):
            if f.endswith(".rs"):
                files.append(os.path.join(root, f))

    runtime = {p: runtime_text(p) for p in files}
    combined = "\n".join(runtime.values())

    unrouted = []
    for path, text in runtime.items():
        for line_no, line in enumerate(text.splitlines(), 1):
            m = DEF.match(line)
            if not m:
                continue
            name = m.group(1)
            # Count uses OUTSIDE the definition line itself.
            uses = len(re.findall(rf"\b{re.escape(name)}\s*[(:]", combined))
            defs = len(re.findall(rf"\bfn\s+{re.escape(name)}\s*[(<]", combined))
            if uses <= defs:
                cite = origins.get((path, name), "")
                unrouted.append((path, name, cite))

    cited = [u for u in unrouted if u[2]]
    for path, name, cite in sorted(cited):
        print(f"UNROUTED {path}: {name}  (cites {cite[:40]}) — no runtime caller")
    print(
        f"{len(unrouted)} pub fn(s) have no runtime caller; {len(cited)} of them "
        "carry a binary citation, so a decoded rule is either unwired or duplicated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
