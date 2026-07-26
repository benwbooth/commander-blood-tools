#!/usr/bin/env python3
"""Flag assertions that cannot fail: `X.len() == THE_CONST_THAT_BUILT_X`.

This is the shape that hid the font truncation for a whole campaign:

    let ascii_map = slice(MAP_OFFSET, DIALOGUE_FONT_ASCII_MAP_LEN);   // 128
    ...
    assert_eq!(font.ascii_map.len(), DIALOGUE_FONT_ASCII_MAP_LEN);    // always true

The table is 176 entries. The extractor read 128, dropping every accented
character, and the test agreed with itself. Seven assertions of this shape were
found and re-grounded (against layout identities, code immediates, or the data's
own bounds); this stops an eighth appearing unnoticed.

A flagged assertion is CLEARED by another assertion in the same test that ties the
value to something independent -- an image read, a layout identity, a code
immediate. Heuristic and deliberately loud: it is a prompt to check, not a verdict.
"""

import os
import re
import sys

# The right-hand side must be a SINGLE constant. `W * H` is a dimensional identity
# ("the framebuffer is width times height"), not an extent claim about data read
# from the game, and flagging it was noise -- the first run reported four such.
LEN_EQ = re.compile(
    r"assert(?:_eq)?!\(\s*([A-Za-z0-9_.\[\]() &*]*?)\.len\(\)\s*,\s*"
    r"([A-Z][A-Z0-9_]{4,})\s*[,)]"
)
# Evidence, anywhere in the same test, that the value is pinned independently.
GROUNDED = re.compile(
    r"(BLOODPRG\.EXE|image_bytes|read_to_string|std::fs::read|imm16|imm8|"
    r"0x[0-9A-Fa-f]{4,6}\s*\+|layout|identity|must close|immediates?)"
)


# audit-fixes #371. A SECOND self-referential shape the length rule cannot see:
#
#     assert_eq!(EngineState::OPTION_BOX_LABEL, "CANCEL");
#
# where the constant's own definition IS `"CANCEL"`. Both sides are the same
# transcription, so the assertion cannot fail unless someone edits one of them,
# and it says nothing about the game (#370). Detectable mechanically: find the
# constant's definition and compare it to the literal it is asserted against.
CONST_DEF = re.compile(
    r'(?:pub\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*&\s*\'?static\s+str\s*=\s*("(?:[^"\\]|\\.)*")\s*;'
)
ASSERT_STR = re.compile(
    r'assert_eq!\s*\(\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Z][A-Z0-9_]*)\s*,\s*("(?:[^"\\]|\\.)*")'
)


def tests_in(text):
    """Yield (name, start_line, body) for each #[test] fn."""
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        if lines[i].strip().startswith("#[test]"):
            j = i
            while j < len(lines) and "fn " not in lines[j]:
                j += 1
            if j >= len(lines):
                break
            name = re.search(r"fn\s+(\w+)", lines[j])
            depth, k, body = 0, j, []
            started = False
            while k < len(lines):
                depth += lines[k].count("{") - lines[k].count("}")
                body.append(lines[k])
                if "{" in lines[k]:
                    started = True
                if started and depth <= 0:
                    break
                k += 1
            yield (name.group(1) if name else "?", j + 1, "\n".join(body))
            i = k + 1
            continue
        i += 1


def main():
    problems, checked = [], 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            # Grounding may live in a SIBLING test -- bloodsav pins its header
            # sizes to the writer's immediates in a separate function, which is
            # perfectly good evidence. Scope the search to the file.
            file_grounded = GROUNDED.search(text) is not None
            for name, line, body in tests_in(text):
                for m in LEN_EQ.finditer(body):
                    checked += 1
                    if not (GROUNDED.search(body) or file_grounded):
                        problems.append(
                            f"{path}:{line}: {name} asserts "
                            f"{m.group(1).strip()}.len() == {m.group(2)} with nothing "
                            "independent in the test"
                        )
    # STRING TAUTOLOGIES, gathered across the whole tree: a constant may be
    # defined in one file and asserted in another.
    consts, str_checked = {}, 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if f.endswith(".rs"):
                body = open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
                for m in CONST_DEF.finditer(body):
                    consts[m.group(1)] = m.group(2)
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            file_grounded = GROUNDED.search(text) is not None
            for name, line, body in tests_in(text):
                for m in ASSERT_STR.finditer(body):
                    if consts.get(m.group(1)) != m.group(2):
                        continue  # not comparing a constant to its own value
                    str_checked += 1
                    # UNCONDITIONAL, unlike the length rule. Grounding elsewhere
                    # does not rescue this shape: `CONST == "its own value"` is
                    # vacuous in itself, and the fix is to REPLACE it with a read
                    # of the source the constant claims to come from — which is
                    # what #370 did — not to leave it beside better evidence.
                    problems.append(
                        f"{path}:{line}: {name} asserts {m.group(1)} == "
                        f"{m.group(2)}, which IS its definition -- a tautology; "
                        "assert against the image/data the constant comes from"
                    )

    for p in problems:
        print("SELF-REF " + p)
    print(
        f"{checked} `len() == CONST` and {str_checked} `CONST == its own literal` "
        f"assertion(s), {len(problems)} ungrounded"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
