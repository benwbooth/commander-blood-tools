#!/usr/bin/env python3
"""Which SHORT display literals are in the shipped data, and which are not?

`check_content_literals.py` catches PROSE -- three or more words that read like a
sentence. That leaves a gap this session walked into four times (#227, #320,
#322, #325): the port is full of SHORT display strings -- `"LAST"`, `"PAUSE"`,
`"LOADING"`, `"EXPLANATIONS"` -- which are just as much game content, and every
one checked so far turned out to be sitting in the shipped data at an address the
using routine names.

So this asks the cheap question about each of them: IS IT IN THE DATA?

  IN-IMAGE   the exact bytes appear in BLOODPRG.EXE, usually in the DS string
             block. The literal can be PINNED by a test that reads it, which
             turns a transcription into a mirror that cannot drift.
  IN-DATA    found in a shipped script/dictionary file instead (DIC words,
             menus). Same conclusion, different file.
  ABSENT     in neither. NOT automatically wrong -- it may be a port-side label
             with no counterpart -- but it is the interesting half, because a
             display string the game does not contain is either invented or
             transcribed from a screenshot, and both are the defect the prime
             rule names.

WHAT COUNTS: an uppercase-ish display literal of 3..24 chars, letters/digits/
spaces only. Paths, format strings, identifiers, and anything with punctuation
that marks it as code are skipped -- those are the port talking to itself.

Test code is exempt: a test naming the string it expects is how a decode gets
pinned, which is the OUTCOME this tool is arguing for.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

BIN = os.path.join("re", "bin", "BLOODPRG.EXE")
DATA_DIRS = [os.path.join("output", "_tmp_iso"), os.path.join("output", "scripts")]
# Below this a byte match is coincidence more often than attribution.
MIN_ATTRIBUTABLE = 5

# A display string: quoted, 3..24 chars, letters/digits/space only, and carrying
# at least one uppercase letter or being all-caps -- the shape of on-screen text
# rather than an identifier or a key.
LITERAL = re.compile(r'"([A-Za-z0-9][A-Za-z0-9 \'.!?:-]{2,23})"')
# Words that mark a string as the port talking to a developer, not the player.
FILENAME = re.compile(r"^[A-Za-z0-9_]+\.[A-Za-z]{2,4}$")
# `pub const NAME`, `const NAME`, `pub fn name`, `pub struct Name` -- the item a
# literal belongs to, used to ask whether a test names it.
DECL = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static|fn|struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
NOISE = re.compile(
    r"(?i)\b(unwrap|expect|panic|todo|fixme|debug|trace|test|assert|err|error|"
    r"warn|failed|missing|invalid|usage|skip)\b"
)


def shipped_blobs():
    out = []
    if os.path.exists(BIN):
        out.append((os.path.basename(BIN), open(BIN, "rb").read()))
    for d in DATA_DIRS:
        if not os.path.isdir(d):
            continue
        for f in sorted(os.listdir(d)):
            p = os.path.join(d, f)
            if os.path.isfile(p) and os.path.getsize(p) < 8_000_000:
                out.append((f, open(p, "rb").read()))
    return out


def main():
    blobs = shipped_blobs()
    if not blobs:
        print("no shipped data found; nothing to check")
        return 0
    exe = next((b for n, b in blobs if n.upper().endswith(".EXE")), b"")

    in_image, in_data, absent, too_short = [], [], [], []
    seen = set()
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            # Everything from `mod tests` on is exempt.
            cut = text.find("#[cfg(test)]")
            body = text[:cut] if cut > 0 else text
            # The file's TEST section, kept so an already-pinned literal is not
            # reported as needing pinning (audit-fixes #466). "PIN IT" was appended
            # unconditionally, so WORLD_ART_DIRECTORY's 42 names -- held to the
            # image byte-for-byte by world_art_directory_matches_the_ds2bc7_table --
            # were flagged every run alongside genuinely loose ones. A checker whose
            # advice is mostly already taken teaches its reader to skip it.
            tests = text[cut:] if cut > 0 else ""
            # STRIP COMMENTS FIRST. A doc comment QUOTING on-screen text -- `///
            # The comms "Hate TV" screen` -- is documentation, not a literal, and
            # the first run of this tool reported four of them as suspect content.
            # A checker that sends a reader chasing prose in a comment is the same
            # defect this session found in three other tools.
            body = "\n".join(
                "" if ln.lstrip().startswith(("///", "//!", "//")) else ln
                for ln in body.splitlines()
            )
            for m in LITERAL.finditer(body):
                s = m.group(1)
                if NOISE.search(s) or not any(c.isupper() for c in s):
                    continue
                # FILENAMES are port-side paths, not game content. `TB.BIG`,
                # `HONKF.SPR`, `BLOODPRG.EXE` dominated the first ABSENT list and
                # buried the handful of real display strings under them.
                if FILENAME.match(s):
                    continue
                if s.strip() != s or s in seen:
                    continue
                seen.add(s)
                line = body[: m.start()].count("\n") + 1
                # TOO SHORT TO ATTRIBUTE. A 3-4 char string turns up in any
                # binary by chance -- the first run of this tool "found" 'DEB',
                # 'DIC', 'FORM' and 'ILBM' in shipped files and reported them as
                # game content, when they are parser magic and file extensions.
                # Same coincidence rule as check_literal_tables.py's MIN_BYTES
                # and #263's warning about matching constants by value.
                if len(s) < MIN_ATTRIBUTABLE:
                    too_short.append((path, line, s))
                    continue
                raw = s.encode("ascii", "ignore")
                # PINNED = the file's tests name the ENCLOSING ITEM, not the
                # string. Checking for the literal itself missed
                # WORLD_ART_DIRECTORY entirely: its test reads the image and
                # compares programmatically, so "Kortex" never appears in it.
                # A pinning test is precisely the kind that does NOT repeat the
                # value it pins (audit-fixes #466).
                owner = None
                # START AT `line`, NOT `line - 1`. A literal declared inline with
                # its constant -- `const EMS_DRIVER_SIGNATURE: &[u8; 8] =
                # b"EMMXXXX0";` -- has its owner on its OWN line, and beginning the
                # walk one line earlier skipped straight past it to whatever
                # declaration came before, so the literal read as unowned and its
                # pinning test could never be found (audit-fixes #527).
                for prev in range(line, max(0, line - 400), -1):
                    m2 = DECL.match(body.splitlines()[prev - 1]) if prev - 1 < len(body.splitlines()) else None
                    if m2:
                        owner = m2.group(1)
                        break
                pinned = (s in tests) or bool(owner and re.search(rf"\b{re.escape(owner)}\b", tests))
                if raw and raw in exe:
                    in_image.append((path, line, s, exe.find(raw), pinned))
                    continue
                where = next(
                    (n for n, b in blobs if raw and raw in b and not n.upper().endswith(".EXE")),
                    None,
                )
                # Try the lowercase form too: the DIC stores words lowercase and
                # the widget upper-cases for display (audit-fixes #322).
                if where is None:
                    low = s.lower().encode("ascii", "ignore")
                    where = next(
                        (n for n, b in blobs if low and low in b and not n.upper().endswith(".EXE")),
                        None,
                    )
                if where:
                    in_data.append((path, line, s, where, pinned))
                else:
                    absent.append((path, line, s))

    unpinned_image = [r for r in in_image if not r[4]]
    unpinned_data = [r for r in in_data if not r[4]]
    # A `.DIC` IS A WORD DICTIONARY (audit-fixes #528). The game builds subtitles
    # from it, so it contains most ordinary English words -- and a port literal
    # like "FRONT", "RIGHT" or "CLICK" therefore "appears in SCRIPT1.DIC" by pure
    # coincidence, exactly as a 4-letter string matches any binary. Reporting those
    # beside a real find (a character name in DESCRIPT.DES) buries the real one:
    # 42 entries of mostly-noise train the reader to skip the tool, which is the
    # failure #466 added the pin logic to prevent and #527 hit again.
    #
    # So DIC matches are reported SEPARATELY and are advisory. Record files
    # (.DES/.DEB/READ.ME) name specific things and a match there is real evidence.
    dict_hit = lambda where: str(where).upper().endswith(".DIC")
    unpinned_record = [r for r in unpinned_data if not dict_hit(r[3])]
    unpinned_dict = [r for r in unpinned_data if dict_hit(r[3])]
    for path, line, s, at, _ in sorted(unpinned_image):
        print(f"IN-IMAGE {path}:{line}: {s!r} at BLOODPRG.EXE {at:#07x} — PIN IT")
    for path, line, s, where, _ in sorted(unpinned_record):
        print(f"IN-DATA  {path}:{line}: {s!r} in {where} — PIN IT")
    if "--dict" in sys.argv:
        for path, line, s, where, _ in sorted(unpinned_dict):
            print(f"IN-DICT  {path}:{line}: {s!r} in {where} (a word list — likely coincidence)")
    if "--absent" in sys.argv:
        for path, line, s in sorted(absent):
            print(f"ABSENT   {path}:{line}: {s!r}")

    print(
        f"{len(in_image) + len(in_data) + len(absent)} display literal(s): "
        f"{len(in_image)} in the image ({len(unpinned_image)} unpinned), "
        f"{len(in_data)} in shipped data ({len(unpinned_record)} unpinned in RECORD "
        f"files, {len(unpinned_dict)} in .DIC word lists — --dict to list), "
        f"{len(absent)} in neither (--absent to list); "
        f"{len(too_short)} under {MIN_ATTRIBUTABLE} chars and NOT searched, "
        "because a short string matches any binary by chance"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
