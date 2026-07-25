#!/usr/bin/env python3
"""Does the test a doc claims verifies it actually EXIST, and read real data?

Docs make claims like

    /// Verified byte-exact against the binary by `tests::angle_table_matches_binary`.

which is the strongest kind of provenance in this tree — if the test is real. A
named test that does not exist is worse than no claim: it reads as settled and
nothing runs. A named test that exists but only compares the port to itself is the
self-referential shape `check_selfref_asserts.py` was written for, one level up.

For each `name` a doc names as its verifier, this reports:

  MISSING   the test does not exist anywhere in src/ or tests/
  SELF      it exists but reads no game data -- no BLOODPRG.EXE, no asset file,
            no capture, so whatever it checks is not the original
  OK        it exists and opens something shipped by the game

OK is evidence for settling a ledger row, not a settling. Run with
PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

# `tests::foo`, `by `foo``, "see `foo`" -- the forms used in this tree.
# The KEYWORD is case-insensitive (docs open sentences with "Verified ..."), but
# the captured NAME must be lowercase. Making the whole pattern case-insensitive
# matched SCREAMING_CASE and reported the env var `CBLOOD_DATA` as a missing test;
# making the whole pattern case-SENSITIVE then dropped every capitalised sentence
# and the count fell from 5 to 1. Both halves need their own rule.
CLAIM = re.compile(
    r"(?:verified|checked|proven|pinned)[^.`]{0,40}`(?:tests::)?([a-z][a-z0-9_]{6,})`",
    re.I,
)
# A `func_<hex>` is a LIFT -- the original instruction stream transliterated and
# oracle-verified. Differentialling the port against one is the strongest evidence
# available here, so it is not "a test that reads no game data": the lift IS the
# game's code.
LIFT = re.compile(r"^func_[0-9a-f]{3,6}$")
# Reading something the GAME shipped, rather than the port's own output.
REAL_DATA = re.compile(
    r"(BLOODPRG\.EXE|\.xdb|\.DIC|\.BAS|\.BIG|\.SPR|\.HNM|\.LBM|\.DES|\.FD|"
    r"captures?/|iso_dir|fixture\(\)|image_bytes|_tmp_dat|_tmp_iso)",
    re.I,
)


def source_files():
    for base in ("src", "tests"):
        for root, _, files in os.walk(base):
            for f in sorted(files):
                if f.endswith(".rs"):
                    yield os.path.join(root, f)


def test_bodies():
    """name -> (path, body) for every `fn name(` in the tree."""
    out = {}
    for path in source_files():
        text = open(path, encoding="utf-8", errors="replace").read()
        for m in re.finditer(r"\bfn\s+([a-z][a-z0-9_]+)\s*\(", text):
            name = m.group(1)
            start = m.end()
            depth, i, started = 0, start, False
            while i < len(text) and i < start + 20000:
                if text[i] == "{":
                    depth += 1
                    started = True
                elif text[i] == "}":
                    depth -= 1
                    if started and depth == 0:
                        break
                i += 1
            out.setdefault(name, (path, text[start:i]))
    return out


def main():
    bodies = test_bodies()
    missing, selfref, ok = [], [], []
    for path in source_files():
        for n, line in enumerate(
            open(path, encoding="utf-8", errors="replace").read().splitlines(), 1
        ):
            st = line.strip()
            if not (st.startswith("///") or st.startswith("//!") or st.startswith("//")):
                continue
            for m in CLAIM.finditer(st):
                name = m.group(1)
                if not name.islower():
                    continue  # a CONSTANT or env var, not a function
                if name not in bodies:
                    missing.append(f"{path}:{n}: claims `{name}` verifies it — no such function")
                    continue
                if LIFT.match(name):
                    ok.append((path, n, name))
                    continue
                _, body = bodies[name]
                (ok if REAL_DATA.search(body) else selfref).append((path, n, name))

    for line in missing:
        print("MISSING " + line)
    for path, n, name in selfref:
        print(f"SELF {path}:{n}: `{name}` exists but reads no game data")
    print(
        f"{len(ok) + len(selfref) + len(missing)} doc(s) name a verifying test: "
        f"{len(ok)} read real game data, {len(selfref)} do not, {len(missing)} do not exist"
    )
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
