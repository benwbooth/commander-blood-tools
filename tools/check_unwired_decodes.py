#!/usr/bin/env python3
"""Functions that carry a BINARY CITATION but that nothing outside tests calls.

A decoded routine with no live caller is not neutral. audit-fixes #610 found the
menu hit test ported TWICE -- `ship3d::hit_test_ship_3d_nav_choice`, decoded and
cited and reached only by its own tests, beside `bridge::menu_row_under_cursor`,
which is the one the running port uses. Two implementations, one live, nothing
comparing them. #608 found the same shape in `Ship3dNavChoiceGates`, whose six
booleans are only ever `Default::default()` in a real run.

Either way the ledger reads as though the behaviour is in the port. It is not: it is
in the tree. The next person to wire the dead copy up will find it plausible and will
not know a live one exists.

This lists functions whose doc comment cites an address (`0xNNNN`) and whose only
callers are inside `#[cfg(test)]` blocks. Two honest outcomes for each, and the tool
cannot tell them apart -- that is the reading:

  * a DUPLICATE of something already wired  -> hold them together with a
    differential test, as #610 did, and say which is live;
  * a decode that was never CONNECTED       -> wiring it is the task.

Usage:
    python3 tools/check_unwired_decodes.py [--all]
"""
import os
import re
import sys

SRC = "src"
FN = re.compile(r"^(?:pub(?:\([^)]*\))? )?fn ([a-z_][a-z0-9_]*)\s*[(<]", re.M)
ADDR = re.compile(r"0x[0-9A-Fa-f]{4,5}")


def split_test_blocks(text):
    """(non-test text, test text) by `#[cfg(test)]` and brace depth."""
    live, tests = [], []
    i = 0
    while True:
        m = text.find("#[cfg(test)]", i)
        if m < 0:
            live.append(text[i:])
            break
        live.append(text[i:m])
        # walk to the module's closing brace
        j = text.find("{", m)
        if j < 0:
            tests.append(text[m:])
            break
        depth, k = 0, j
        while k < len(text):
            if text[k] == "{":
                depth += 1
            elif text[k] == "}":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        tests.append(text[m:k])
        i = k
    return "".join(live), "".join(tests)


def main():
    files = {}
    for root, _, names in os.walk(SRC):
        for n in sorted(names):
            if n.endswith(".rs"):
                p = os.path.join(root, n)
                files[p] = open(p, encoding="utf-8", errors="replace").read()

    # Live code only: test modules removed, AND doc comments stripped. Counting
    # bare names picks up dispatch-table registrations (which are real uses) but
    # also intra-doc links like [`update_ship_3d_nav_choice_dispatch`], which are
    # not -- and that hid the exact function #610 was about.
    live_all = "\n".join(
        "\n".join(l for l in split_test_blocks(t)[0].splitlines() if not l.lstrip().startswith("///"))
        for t in files.values()
    )

    findings = []
    for path, text in files.items():
        live, _tests = split_test_blocks(text)
        lines = text.splitlines()
        for m in FN.finditer(live):
            name = m.group(1)
            line_no = text[: text.index(m.group(0))].count("\n") + 1 if m.group(0) in text else 0
            # the doc block above the declaration
            idx = line_no - 1
            j, doc = idx - 1, []
            while j >= 0 and (lines[j].lstrip().startswith("///") or lines[j].lstrip().startswith("#[")):
                doc.append(lines[j])
                j -= 1
            if not ADDR.search("\n".join(doc)):
                continue  # no citation: a different queue
            # Count call sites OUTSIDE test blocks ACROSS THE WHOLE TREE, excluding
            # the definition itself. The first version searched only the defining
            # file's live text, so anything called from another module -- e.g.
            # `font::game_font_drawn_width`, used in `vm.rs` -- came back "unwired".
            # 76 findings became 40.
            # ...and count BARE references too, not just calls. The lifted routines
            # in `recomp/` are registered in a dispatch table as
            # `("func_92a3", super::auto::func_92a3)` -- a mention with no parens --
            # so a call-only regex reported every one of them as unwired.
            uses = len(re.findall(rf"\b{re.escape(name)}\b", live_all)) - 1
            if uses <= 0:
                findings.append((path, line_no, name))

    print(f"{len(findings)} cited function(s) with no caller outside tests\n")
    limit = None if "--all" in sys.argv else 30
    for path, line_no, name in findings[:limit]:
        print(f"  {path}:{line_no} {name}")
    if limit and len(findings) > limit:
        print(f"\n  ... {len(findings) - limit} more (--all)")
    print("\nEach is EITHER a duplicate of something already wired (hold them")
    print("together with a differential test) OR a decode never connected (wire it).")
    print("This cannot tell which; reading the pair is the work.")


if __name__ == "__main__":
    main()
