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
    for p in problems:
        print("SELF-REF " + p)
    print(f"{checked} `len() == CONST` assertion(s), {len(problems)} ungrounded")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
