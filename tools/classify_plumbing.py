#!/usr/bin/env python3
"""Which unsettled functions encode NO decoded rule at all?

The ledger's 1340 uncited rows are not uniformly hard. Some are plumbing: a getter
returning a field, or a `_pub` wrapper delegating to a private method with the
same arguments. Those carry no claim about the game — there is nothing in them to
verify against the binary — and leaving them as UNVERIFIED inflates the work queue
with items that can never be settled by decoding anything.

The test is deliberately strict, because a one-line function CAN encode a rule
(`entity_draw_scale` is `(3*scale >> 1) + 1`, decoded from a `mul`). A body
qualifies only when it is exactly:

    self.field                       a field read
    &self.field / self.field.clone() the same, borrowed or cloned
    Self::helper(args) / self.f(args) a delegation whose arguments are only
                                     plain identifiers -- no literals, no
                                     operators, no casts, no indexing

Anything with arithmetic, a constant, an index, or a conditional is NOT plumbing
and stays in the queue. Reports candidates for review; it does not settle them.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import csv
import os
import re
import sys

LEDGER = "docs/function-audit.tsv"
SETTLED = {"ASM", "ORACLE", "DATA", "INFRA", "TESTED"}

FIELD = re.compile(r"^&?(?:mut\s+)?self\.[A-Za-z_][A-Za-z0-9_]*(?:\.clone\(\))?$")
DELEGATE = re.compile(
    r"^(?:self|Self|crate::[A-Za-z0-9_:]+)"
    r"(?:\.|::)[A-Za-z_][A-Za-z0-9_]*"
    r"\(\s*([A-Za-z0-9_,&\s\.]*)\s*\)$"
)
# An argument list that is only identifiers (or self.field) is a pass-through.
# `true`/`false` and SCREAMING_CASE constants are NOT: a boolean literal selects
# behaviour, so `decode_frame` and `decode_character_frame` differ by a decoded
# MODE rather than by nothing. They matched the identifier pattern on the first
# run and had to be excluded explicitly.
ARG_OK = re.compile(r"^[&\s]*(?:self\.)?[A-Za-z_][A-Za-z0-9_]*$")
NOT_AN_ARG = re.compile(r"^(?:true|false|None|[A-Z][A-Z0-9_]{2,})$")


def body_of(lines, start):
    """The single-expression body of the fn starting at `start`, or None."""
    depth = 0
    collected = []
    for i in range(start, min(start + 12, len(lines))):
        line = lines[i]
        depth += line.count("{") - line.count("}")
        collected.append(line)
        if depth == 0 and "{" in "".join(collected):
            break
    else:
        return None
    text = "\n".join(collected)
    inner = text[text.index("{") + 1 : text.rindex("}")]
    stripped = [ln.strip() for ln in inner.splitlines() if ln.strip()]
    stripped = [ln for ln in stripped if not ln.startswith("//")]
    if len(stripped) != 1:
        return None
    return stripped[0].rstrip(";")


def is_plumbing(body):
    if body is None:
        return False
    if FIELD.match(body):
        return True
    m = DELEGATE.match(body)
    if not m:
        return False
    args = [a.strip() for a in m.group(1).split(",") if a.strip()]
    return all(ARG_OK.match(a) and not NOT_AN_ARG.match(a.lstrip("&").strip()) for a in args)


def main():
    rows = [
        r
        for r in csv.DictReader(open(LEDGER), delimiter="\t")
        if r["kind"] == "fn" and r["status"] not in SETTLED
    ]
    by_file = {}
    for r in rows:
        by_file.setdefault(r["file"], []).append(r)

    found = []
    for path, items in sorted(by_file.items()):
        if not os.path.exists(path):
            continue
        lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
        for r in items:
            idx = int(r["line"]) - 1
            if idx >= len(lines) or " fn " not in lines[idx] and not lines[idx].strip().startswith("fn "):
                continue
            body = body_of(lines, idx)
            if is_plumbing(body):
                found.append((path, r["line"], r["item"], body))

    for path, line, item, body in found:
        print(f"PLUMBING {path}:{line}: {item}  ->  {body}")
    print(f"{len(rows)} unsettled fn(s); {len(found)} are pure plumbing (no decoded rule)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
