#!/usr/bin/env python3
"""Which `pub const` display strings does NOTHING in the crate actually use?

audit-fixes #385 deleted `Engine::CONSOLE_MENU`, a five-string array of the ship
console's row names whose stated source was "baked into the golden menu of the
TB.BIG panorama frames (verified against the live capture)" -- pixels, which the
prime rule forbids as a source. It had survived because it was never CALLED: a
`pub` item is exempt from rustc's `dead_code` lint, since the compiler assumes an
external consumer. This crate has no external consumer for these.

That combination is the dangerous one. A content-bearing literal that is USED will
show up in a test, a screenshot diff, or a wrong pixel. One that is used by nothing
is invisible: it asserts game content, cites a capture, and never gets checked
against anything, which is exactly how a transcription outlives the decode that
should have replaced it.

WHAT COUNTS: a `pub const` whose value contains a quoted string of >= 3 chars --
the shape of on-screen text rather than a tuning constant. Numeric consts are
skipped; a dead `pub const FOO: usize = 11` is untidy, not a faithfulness risk.

Reported in two buckets:

  DEAD        no reference anywhere in src/ outside the declaration. Deleting it
              loses nothing -- and if the content is real, the decode that
              produces it belongs in the port instead.
  TEST-ONLY   referenced only from `#[cfg(test)]` code. Weaker but still worth a
              look: a literal whose only consumer is the test asserting it is
              self-referential, the failure mode named in the faithfulness memo.

Deliberately NOT flagged: consts used by the runtime, however oddly named. This
asks one question -- is anything downstream of it -- not whether it is correct.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

DECL = re.compile(r"^\s*pub const ([A-Z][A-Z0-9_]*)\s*:")
STRINGY = re.compile(r'"[^"]{3,}"')


def rust_files():
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if f.endswith(".rs"):
                yield os.path.join(root, f)


def main():
    paths = list(rust_files())
    texts = {p: open(p, encoding="utf-8", errors="replace").read() for p in paths}

    # Where does test code begin in each file? Everything from the first
    # `#[cfg(test)]` on is test context.
    test_start = {}
    for p, t in texts.items():
        cut = t.find("#[cfg(test)]")
        test_start[p] = cut if cut >= 0 else len(t)

    dead, test_only = [], []
    for path, text in texts.items():
        lines = text.splitlines()
        for i, line in enumerate(lines):
            m = DECL.match(line)
            if not m:
                continue
            name = m.group(1)
            # The value may run past this line; take the declaration's statement.
            stmt = line
            j = i
            while stmt.count(";") == 0 and j + 1 < len(lines) and j - i < 40:
                j += 1
                stmt += lines[j]
            if not STRINGY.search(stmt):
                continue  # numeric / non-content const

            decl_start = sum(len(x) + 1 for x in lines[:i])
            decl_end = sum(len(x) + 1 for x in lines[: j + 1])

            prod_refs = test_refs = 0
            for p2, t2 in texts.items():
                for m2 in re.finditer(r"\b" + re.escape(name) + r"\b", t2):
                    if p2 == path and decl_start <= m2.start() < decl_end:
                        continue  # the declaration itself
                    if m2.start() >= test_start[p2]:
                        test_refs += 1
                    else:
                        prod_refs += 1
            if prod_refs == 0 and test_refs == 0:
                dead.append((path, i + 1, name))
            elif prod_refs == 0:
                test_only.append((path, i + 1, name, test_refs))

    for path, line, name in sorted(dead):
        print(f"DEAD      {path}:{line}: pub const {name} — no reference in src/")
    for path, line, name, n in sorted(test_only):
        print(f"TEST-ONLY {path}:{line}: pub const {name} — {n} ref(s), all in tests")

    print(
        f"{len(dead)} dead and {len(test_only)} test-only string-bearing pub const(s); "
        "a content literal nothing consumes is never checked against the game"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
