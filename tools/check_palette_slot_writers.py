#!/usr/bin/env python3
"""Reserved palette slots (0xC0..0xFF) written by more than one port site.

The game fills `0xC0..0xFF` at RUNTIME; a scene's LBM/HNM palette leaves them
`[0,0,0]`, so the port installs its own RGB before drawing through them. #542 found
FOUR sites writing just two of those slots with different colours -- a cyberspace
reticle, a nav object marker, and the subtitle reveal helper -- under names
(`RETICLE`, `BAR`, `SUBTITLE_COLOR_REVEALED`) that give no hint they share a slot.
Whichever runs last wins, and the symptom surfaces in a different file from the
cause.

This lists every reserved index the port writes and where, flagging those with more
than one writer AND more than one distinct colour. Same-colour repeats are fine:
several screens installing the same subtitle white is the helper doing its job.

Usage: python3 tools/check_palette_slot_writers.py [--all]
"""
import os
import re
import sys
from collections import defaultdict

SRC = "src"
# `scene_palette[0xFE] = [245, 245, 160];` and `pal[IDX] = [...]`
# The index may be a LITERAL or a NAMED CONSTANT. The first version of this tool
# matched only literals and therefore missed `scene_palette[RETICLE as usize]` --
# the exact pair (#542) that motivated writing it. A guard that cannot see its own
# motivating case is the #527/#540 failure, so named indices are resolved below.
WRITE = re.compile(
    r"(\w*palette\w*|pal)\s*\[\s*([A-Za-z_][A-Za-z0-9_]*|0x[0-9A-Fa-f]{2}|\d{1,3})"
    r"(?:\s+as\s+usize)?\s*\]\s*=\s*\[([^\]]*)\]"
)
CONST_DEF = re.compile(
    r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*\w+\s*=\s*(0x[0-9A-Fa-f]+|\d+)\s*;"
)
RESERVED_MIN = 0xC0


def main():
    writers = defaultdict(list)
    for root, _, files in os.walk(SRC):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            consts = {m.group(1): int(m.group(2), 0) for m in CONST_DEF.finditer(text)}
            # SKIP TEST SECTIONS. A test installing `[1,2,3]` into a reserved slot to
            # prove the subtitle renderer reads it is not a conflicting writer, and
            # reporting it beside the real ones is the noise #528 removed elsewhere.
            body = text.split("#[cfg(test)]")[0]
            for n, line in enumerate(body.splitlines(), 1):
                m = WRITE.search(line)
                if not m:
                    continue
                token = m.group(2)
                try:
                    idx = int(token, 0)
                except ValueError:
                    if token not in consts:
                        continue
                    idx = consts[token]
                if idx < RESERVED_MIN:
                    continue
                rgb = " ".join(m.group(3).split())
                writers[idx].append((path, n, rgb))

    conflicts = {
        i: w for i, w in writers.items() if len({rgb for _, _, rgb in w}) > 1
    }
    print(f"{len(writers)} reserved slot(s) written by the port; "
          f"{len(conflicts)} written with MORE THAN ONE colour\n")
    show = writers if "--all" in sys.argv else conflicts
    for idx in sorted(show):
        print(f"  {idx:#04x}  {len(writers[idx])} writer(s)"
              + ("  <-- CONFLICT" if idx in conflicts else ""))
        for path, n, rgb in writers[idx]:
            print(f"      {path}:{n}: [{rgb}]")
    if conflicts:
        print("\nA slot with two colours means whichever site runs last wins.")
        print("That is fine if the screens never coexist -- and nothing here says so.")


if __name__ == "__main__":
    main()
