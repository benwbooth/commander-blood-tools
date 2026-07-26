#!/usr/bin/env python3
"""Which runtime state is WIRED but never FED?

`check_unrouted_rules.py` asks whether a decoded rule has a caller. That misses
a whole class of gap, found in audit-fixes #290.

`resolve_ship_3d_position_field` IS called — from the VM's `step()`, well inside
production code, so the unrouted checker is satisfied. But its input lives in
`Ship3dC1PositionRuntime`, and the only thing that ever populates it is the
builder `with_ship_3d_c1_positions`, whose every call site is inside
`#[cfg(test)]`. In a real run the field is always `None`, `step()` takes the
early-return arm, and an entire decoded subsystem — the kind-0x100 compare, the
direct kinds, the parent walk, the distance redirect — never executes.

A test suite cannot notice this. The tests supply the data themselves, so they
exercise the code and pass; the gap is precisely that NOTHING ELSE DOES.

So this looks for public builders/setters whose call sites are ALL inside the
test module. For each, it reports the field it writes, because that field is the
runtime state that stays at its default forever.

WHAT THIS DOES NOT CLAIM. A test-only builder is not automatically a defect —
some exist to construct fixtures for a subsystem fed by another path, and some
runtime state is legitimately optional. The output is a QUESTION per row: what,
in a real run, is supposed to call this? If the answer is "nothing yet", the
subsystem is decoded but inert, which is worth knowing and is not visible from
either the test results or the ledger.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

# A builder: `pub fn with_x(mut self, ...) -> Self` or a `pub fn set_x(&mut self`.
#
# The signature may WRAP. `with_ship_3d_c1_positions<I, J>(` puts `mut self` on
# the next line, and the first version of this regex — which required it on the
# same line — missed exactly the builder that motivated the tool (audit-fixes
# #290). So match the name here and look for the receiver over the following few
# lines instead.
BUILDER_NAME = re.compile(r"^\s*pub fn ((?:with|set)_[a-z0-9_]+)\s*(?:<[^>]*>)?\s*\(")
RECEIVER = re.compile(r"(?:^|\(|,)\s*(?:mut\s+self|&mut\s+self)\b")


def is_builder(lines, i):
    """Name of the builder defined at line `i`, or None.

    Looks for the receiver on the definition line and the three after it, which
    covers `pub fn f<I, J>(\\n    mut self,` without swallowing the next item.
    """
    m = BUILDER_NAME.match(lines[i])
    if not m:
        return None
    for line in lines[i : i + 4]:
        if RECEIVER.search(line):
            return m.group(1)
        if line.rstrip().endswith(")") or "->" in line:
            break
    return None


def test_module_start(lines):
    """Line index where `#[cfg(test)]` first opens the test module, or None."""
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            return i
    return None


def fields_written(lines, start):
    """Field names assigned inside a builder body, e.g. `self.foo = ...`."""
    out, depth, seen_open = set(), 0, False
    for line in lines[start : start + 60]:
        depth += line.count("{") - line.count("}")
        if "{" in line:
            seen_open = True
        for m in re.finditer(r"self\.([a-z_][a-z0-9_]*)", line):
            out.add(m.group(1))
        # also `runtime.position_runtime = Some(...)` style locals
        for m in re.finditer(r"\b([a-z_][a-z0-9_]*)\s*\.\s*([a-z_][a-z0-9_]*)\s*=", line):
            out.add(m.group(2))
        if seen_open and depth <= 0:
            break
    return out


def main():
    findings = []
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            cut = test_module_start(lines)
            if cut is None:
                cut = len(lines)

            for i, _line in enumerate(lines):
                if i >= cut:
                    break  # only consider builders DEFINED in production code
                name = is_builder(lines, i)
                if not name:
                    continue
                prod = test = 0
                # count call sites across the whole tree
                for r2, _, f2 in os.walk("src"):
                    for g in f2:
                        if not g.endswith(".rs"):
                            continue
                        p2 = os.path.join(r2, g)
                        l2 = open(p2, encoding="utf-8", errors="replace").read().splitlines()
                        cut2 = test_module_start(l2)
                        cut2 = len(l2) if cut2 is None else cut2
                        for j, line2 in enumerate(l2):
                            if j == i and p2 == path:
                                continue  # the definition itself
                            if re.search(rf"\.\s*{re.escape(name)}\s*\(", line2):
                                if j < cut2:
                                    prod += 1
                                else:
                                    test += 1
                if test and not prod:
                    findings.append((path, i + 1, name, test, fields_written(lines, i)))

    # DEAD SUBSYSTEMS: a public TYPE whose every constructor call is in tests.
    # AlienColony was exactly this -- the colony dispatcher, its frame gate and
    # its shared streams all decoded and ported, and nothing in the running game
    # ever builds one (audit-fixes #404). The builder scan above cannot see it,
    # because it looks for `with_`/`set_` state setters, not constructors, so a
    # whole subsystem can be unfed while every builder in it looks fine.
    types = {}
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            for i, line in enumerate(lines):
                m = re.match(r"^\s*pub struct ([A-Z][A-Za-z0-9]*)", line)
                if m:
                    types[m.group(1)] = (path, i + 1)

    dead_types = []
    for name, (path, line) in sorted(types.items()):
        prod = test = 0
        for r2, _, f2 in os.walk("src"):
            for g in sorted(f2):
                if not g.endswith(".rs"):
                    continue
                p2 = os.path.join(r2, g)
                l2 = open(p2, encoding="utf-8", errors="replace").read().splitlines()
                cut2 = test_module_start(l2)
                cut2 = len(l2) if cut2 is None else cut2
                for j, line2 in enumerate(l2):
                    # The DEFINITION `pub struct Name {` matches the construction
                    # pattern below, so the first run of this check reported zero
                    # dead types -- every type "constructed" itself. Comments
                    # mentioning `Name { .. }` are prose, not construction.
                    stripped = line2.lstrip()
                    if stripped.startswith(("//", "///", "//!", "*")):
                        continue
                    # `pub struct Name {`, `impl Name {`, `impl Trait for Name {`
                    # and `enum Name {` all match the construction pattern below.
                    # Excluding only `struct` left AlienColony invisible, because
                    # its `impl` block counted as a production construction site.
                    if re.match(
                        rf"^\s*((pub\s+)?(struct|enum|trait)\s+{re.escape(name)}\b"
                        rf"|impl\b.*\b{re.escape(name)}\b)",
                        line2,
                    ):
                        continue
                    # a CONSTRUCTION site: `Name::new(`, `Name {`, `Name::default(`
                    if re.search(rf"\b{re.escape(name)}\s*(::\s*\w+\s*\(|\{{)", line2):
                        if j < cut2:
                            prod += 1
                        else:
                            test += 1
        if test and not prod:
            dead_types.append((path, line, name, test))

    for path, line, name, test in dead_types:
        print(
            f"UNFED-TYPE {path}:{line}: {name} — built {test} time(s), ALL in tests; "
            "the whole type is dead in a real run"
        )

    for path, line, name, test, fields in findings:
        shown = ", ".join(sorted(fields)[:4]) or "(no field assignment found)"
        print(f"UNFED {path}:{line}: {name} — {test} call site(s), ALL in tests; writes {shown}")

    print(
        f"{len(findings)} builder(s) that only tests ever call: the state they "
        f"write stays at its default in every real run; {len(dead_types)} whole "
        "type(s) built only by tests"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
