#!/usr/bin/env python3
"""Is there GAME TEXT hardcoded in the port's Rust source?

The prime rule names this defect outright: content that lives in the game's data
-- script bytecode, DIC words, DESCRIPT records, sprite and level tables -- must
be executed or parsed, never transcribed. A line of dialogue in a `.rs` file is
text copied off the running game, and it will not change when the data does.

`main.rs` carried

    "HONK! You worthless heap of wires... Are  \\nyou working?"

as a "no-VM fallback" for Bob's greeting, which SCRIPT2's bytecode already
provides (record 132 rel 40). Removed -- if the VM yields nothing, the contact
does not open, because inventing a line is worse than showing none.

What counts as PROSE here: a string literal of three or more words that reads
like sentence text -- it contains a space and either sentence punctuation or a
run of lowercase words. Format strings, paths, identifiers, log/diagnostic
messages and `expect`/`panic` text are excluded, since those are the port talking
to its developer, not the game talking to the player.

Test code is exempt: a test naming the line it expects is how the decode is
pinned.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

STRING = re.compile(r'"((?:[^"\\]|\\.){12,})"')
# A SHORTER scan, for UI labels only. The 12-character minimum above is right for
# prose but hid `"SHIP: "` (6) and `"PLANET: "` (8) -- two of the four headers
# transcribed out of DS:0x12E..0x14B in #139.
SHORT_STRING = re.compile(r'"((?:[^"\\]|\\.){3,20})"')
# Prose: has a space, and looks like words rather than a path/format/identifier.
WORDS = re.compile(r"^[A-Za-z0-9 ,.'!?;:()\-\\n]+$")
SENTENCE = re.compile(r"[.!?]|\b(?:you|your|the|and|are|is|not|this|that)\b", re.I)

# The port talking to its developer, not the game talking to the player.
DIAGNOSTIC = re.compile(
    r"(anyhow|bail|ensure|expect|panic|eprintln|println|format!|write!|writeln|"
    r"assert|debug|warn|error|info|todo!|unimplemented|unreachable)",
    re.I,
)
# Failure text reads like prose but is the port's own diagnostic vocabulary.
ERRORISH = re.compile(
    r"(not found|too small|exceeds|invalid|unsupported|missing|failed|"
    r"cannot|unable|no such|out of range|unexpected|\.(?:DES|EXE|BIG|DAT|SPR|HNM))",
    re.I,
)
# A comment-like string DESCRIBING an address (the DS-global label table in
# bloodprg.rs) is documentation, not game text.
DESCRIPTIVE = re.compile(r"0x[0-9A-Fa-f]{3,}")
# Prose ABOUT the code is not prose the player sees. The DS-global label table in
# bloodprg.rs is 200-odd descriptions like "nonzero byte blocks navigation-choice
# hit testing" -- documentation carried as data, which is the point of that table.
TECHNICAL = re.compile(
    r"\b(byte|word|flag|handler|offset|table|index|dispatch|pointer|buffer|"
    r"bank|slot|record|selector|opcode|struct|scanline|framebuffer|palette|"
    r"nonzero|set to|bit ?[0-7]|per-frame|hit test\w*|drawn|entry|counter|"
    r"cursor|toggle|sprite|glyph|stride|routine|state machine|VM|DS|CS|FS|GS)\b"
)
FIELD = re.compile(r"^(comment|kind|name|note|desc|description)\s*:\s*")
ARM = re.compile(r"=>\s*\"")
ARM_OPEN = re.compile(r"=>\s*\{\s*$")
SKIP_DIRS = (os.path.join("src", "recomp"), os.path.join("src", "bin"))


def prose(text):
    if "\\n" in text:
        text = text.replace("\\n", " ")
    if not WORDS.match(text):
        return False
    words = [w for w in text.split() if w]
    if len(words) < 3:
        return False
    lower = sum(1 for w in words if w[:1].islower())
    return bool(SENTENCE.search(text)) and lower >= 2


# UI LABEL LISTS. A menu label is not prose -- "BOB_MORLOCK" is one word -- so the
# prose test cannot see it, but a `vec![]` of SCREAMING_SNAKE strings is exactly the
# transcribed-menu shape CLAUDE.md calls out: conversation menus must come from the
# 0xA6 line records' 0xFFFF-separated word lists, executed by the VM.
LABEL = re.compile(r'"([A-Z][A-Z0-9_]{2,})"')
# SHORT UI STRINGS. The prose test needs 12+ characters and sentence shape, so
# `"PLANET: "`, `"SHIP: "`, `"BLACK HOLE: "` and `"LIFE SUPPORT:"` -- four headers
# transcribed out of DS:0x12E..0x14B into vm.rs -- were invisible to it. A short
# ALL-CAPS string ending in a colon is a label the game draws, not a diagnostic.
UI_LABEL = re.compile(r'^[A-Z][A-Z ]{2,20}:\s?$')
VEC_OPEN = re.compile(r"vec!\[")

# Sites known to be open, each tracked in docs/port-validation.md. Listed so the
# CLASS cannot grow silently while these are unfixed -- a new one fails the check.
# Empty: both label lists that lived here are now read from the game's own data --
# the contact menu from the ship-slot array (0x87BD) and the OPTION menu from the
# DS:0x2567 pointer list (0x8871). A NEW hardcoded list fails the check outright.
KNOWN_OPEN = set()


def label_lists():
    """Report `vec![]` blocks of SCREAMING_SNAKE labels in runtime code."""
    out = []
    for root, _, files in os.walk("src"):
        if any(root.startswith(d) for d in SKIP_DIRS):
            continue
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            lines = text.split("#[cfg(test)]")[0].splitlines()
            for n, line in enumerate(lines, 1):
                if not VEC_OPEN.search(line):
                    continue
                block = "\n".join(lines[n - 1 : n + 9])
                block = block[: block.find("]") + 1] if "]" in block else block
                labels = LABEL.findall(block)
                if len(labels) < 2:
                    continue
                if any(
                    path == kp and kl in labels for kp, kl in KNOWN_OPEN
                ):
                    continue
                out.append(
                    f"{path}:{n}: menu labels in source: {', '.join(labels[:6])} "
                    "-- menus come from the line records' word lists, via the VM"
                )
    return out


def main():
    problems = []
    scanned = 0
    for root, _, files in os.walk("src"):
        if any(root.startswith(d) for d in SKIP_DIRS):
            continue
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            body = text.split("#[cfg(test)]")[0]  # runtime code only
            prev_arm = False
            for n, line in enumerate(body.splitlines(), 1):
                st = line.strip()
                was_arm, prev_arm = prev_arm, ARM_OPEN.search(line) is not None
                if st.startswith("//"):
                    continue
                if DIAGNOSTIC.search(line):
                    continue
                # A `comment:`/`kind:`/`name:` field is DOCUMENTATION carried as
                # data -- bloodprg.rs's BinarySymbol table describes every DS global
                # that way. Excluding the field is structural and exact, where
                # guessing by vocabulary was neither.
                if FIELD.match(st):
                    continue
                # A match ARM yielding a string describes the thing it matched --
                # `0x008866 => "navigation choice handler reloads radio.snd"`.
                # Player-facing text is not produced by classifying an address.
                if ARM.search(line) or was_arm:
                    continue
                for m in SHORT_STRING.finditer(line):
                    lit = m.group(1)
                    if UI_LABEL.match(lit) and not ERRORISH.search(lit):
                        problems.append(
                            f"{path}:{n}: UI label in source: \"{lit}\" -- the game "
                            "draws it from its own DS strings"
                        )
                for m in STRING.finditer(line):
                    scanned += 1
                    lit = m.group(1)
                    if (
                        ERRORISH.search(lit)
                        or DESCRIPTIVE.search(lit)
                        or TECHNICAL.search(lit)
                    ):
                        continue
                    if UI_LABEL.match(lit):
                        problems.append(
                            f"{path}:{n}: UI label in source: \"{lit}\" -- the game "
                            "draws it from its own DS strings"
                        )
                        continue
                    if prose(lit):
                        problems.append(
                            f"{path}:{n}: game text in source: \"{lit[:60]}\""
                        )
    problems += label_lists()

    for p in problems:
        print("CONTENT " + p)
    print(
        f"{scanned} long string literal(s) in runtime code, "
        f"{len(problems)} reading as game text"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
