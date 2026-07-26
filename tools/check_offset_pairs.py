#!/usr/bin/env python3
"""Cross-check every DS-offset / file-offset PAIR the port declares.

`DS` base is file `0xD420`, so any constant naming both must satisfy
`file == 0xD420 + ds`.  A pair that drifts is invisible to normal tests — each
half is individually plausible — which is why `OPTION_BOX_LABEL` has a hand-written
assertion for exactly this.  This does it for the whole tree.

Pairs are found two ways:
  * two constants whose names differ only by the `_DS_OFFSET` / `_FILE_OFFSET`
    (or `_DS` / `_FILE`) suffix;
  * a single doc comment mentioning both `DS:0xNNNN` and `file 0xNNNNN`.

Exit status is non-zero when a pair disagrees.
"""

import os
import re
import sys

DS_BASE = 0xD420
CONST = re.compile(
    r"pub const ([A-Z][A-Z0-9_]*)\s*:\s*(?:u16|u32|usize|i32)\s*=\s*(0x[0-9a-fA-F_]+|\d+)\s*;"
)
# The two halves must be ADJACENT. An 80-char window happily paired the DS offset
# of one item with the file offset of the NEXT one ("... at file A = DS:B, map at
# file C = DS:D" matched B with C) and reported two false mismatches in font.rs
# whose halves were both correct. Keep the window short and forbid another
# `DS:`/`file` in between; accept either order.
_GAP = r"[^\n]{0,24}?"
# Pairing prose positionally is a trap: an 80-char window paired one item's DS
# offset with the NEXT item's file offset and reported two false mismatches whose
# halves were both correct, and tightening the separator only traded that for lost
# coverage. So do not pair by position at all -- collect the DS and file tokens of
# a whole doc block and ask whether each file offset HAS a partner among the DS
# offsets. A block whose sets correspond is consistent however it is worded; only
# a block with exactly one of each and no correspondence is unambiguously wrong.
DS_TOK = re.compile(r"DS:(0x[0-9a-fA-F]{3,4})")
FILE_TOK = re.compile(r"file\s*`?(0x[0-9a-fA-F]{4,6})\b")

SUFFIXES = [("_DS_OFFSET", "_FILE_OFFSET"), ("_DS", "_FILE"), ("_DS_OFFSET", "_FILE")]


def load_image():
    for candidate in ("re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"):
        try:
            with open(candidate, "rb") as fh:
                return fh.read()
        except OSError:
            continue
    return b""


def main():
    image = load_image()
    grounded, empty = [], []
    consts = {}
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if f.endswith(".rs"):
                path = os.path.join(root, f)
                text = open(path, encoding="utf-8", errors="replace").read()
                for m in CONST.finditer(text):
                    consts.setdefault((path, m.group(1)), int(m.group(2).replace("_", ""), 0))

    checked = bad = 0
    by_file = {}
    for (path, name), val in consts.items():
        by_file.setdefault(path, {})[name] = val

    for path, names in sorted(by_file.items()):
        for name, val in sorted(names.items()):
            for ds_suf, file_suf in SUFFIXES:
                if not name.endswith(ds_suf):
                    continue
                twin = name[: -len(ds_suf)] + file_suf
                if twin not in names:
                    continue
                checked += 1
                if names[twin] != DS_BASE + val:
                    bad += 1
                    print(
                        f"MISMATCH {path}: {name}={val:#x} but {twin}={names[twin]:#x} "
                        f"(expected {DS_BASE + val:#x})"
                    )
                break

    # Doc-comment pairs. A doc comment is a RUN of consecutive `///`/`//!` lines;
    # matching per-line misses every pair that wraps, which is most of them.
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
            run, start = [], 0
            for i, line in enumerate(lines + [""], 1):
                st = line.strip()
                if st.startswith("///") or st.startswith("//!"):
                    if not run:
                        start = i
                    run.append(st.lstrip("/!").strip())
                    continue
                if run:
                    blob = " ".join(run)
                    ds_set = {int(x, 16) for x in DS_TOK.findall(blob)}
                    # A file offset BELOW the DS base is in the code segment (or
                    # another segment's data) and cannot be DS-relative at all --
                    # which is what `file 0x006BEA` and `FS:0x11F4 (file 0x0D3E4)`
                    # both are. Excluding them is strictly better than special-casing
                    # the segment prefix, since a code address rarely carries one.
                    file_set = {
                        v
                        for x in FILE_TOK.findall(blob)
                        if (v := int(x, 16)) >= DS_BASE
                    }
                    if not ds_set or not file_set:
                        run = []
                        continue
                    partnered = {f for f in file_set if f - DS_BASE in ds_set}
                    checked += len(partnered)
                    # WHAT THE ARITHMETIC CANNOT SEE. `file == 0xD420 + ds` holds
                    # whenever the two constants were written consistently -- if a
                    # pair names the wrong table, both drift together and this
                    # guard stays quiet (the same weakness audit-fixes #253 fixed
                    # in `parses_mz_header_and_address_conversions`, where two
                    # constants agreed with each other and neither with the image).
                    #
                    # An image check here is genuinely ambiguous, because many DS
                    # offsets are RUNTIME STATE and read as zeros in the shipped
                    # file -- `DS:0x6D3E`, the ship-slot array, is all zeros and
                    # correct. So this reports rather than judges: it says which
                    # pairs land on shipped DATA and which on an empty region, and
                    # leaves the reader to know which kind each should be.
                    for f in partnered:
                        window = image[f : f + 16] if image else b""
                        (grounded if any(window) else empty).append((path, f, f - DS_BASE))
                    orphan_files = file_set - partnered
                    orphan_ds = {d for d in ds_set if d + DS_BASE not in file_set}
                    # One of each and they do not correspond -> a real mismatch.
                    # Anything else (a file offset in the CODE segment with no DS
                    # twin, several items in one block) is not decidable here.
                    # A file offset attributed to ANOTHER segment (FS:/CS:/GS:)
                    # has no business pairing with a DS offset -- FS:0x11F4 is
                    # file 0x0D3E4 under a different base entirely. When a block
                    # names one, the lone-orphan inference is unsafe.
                    other_segment = re.search(r"\b(?:FS|CS|GS|ES|SS):0x", blob)
                    if len(orphan_files) == 1 and len(orphan_ds) == 1 and not other_segment:
                        checked += 1
                        bad += 1
                        d, f = orphan_ds.pop(), orphan_files.pop()
                        print(
                            f"MISMATCH {path}:{start}: doc says DS:{d:#06x} and file "
                            f"{f:#07x} (DS:{d:#06x} is file {DS_BASE + d:#07x})"
                        )
                    run = []

    # `checked` counts three different paths (a NAME-suffix pair, a doc-run pair,
    # and an inferred lone orphan); only the doc-run path is classified against
    # the image, so the classification is reported against ITS OWN total rather
    # than against `checked`. Saying "22 checked, 20 + 1 classified" would imply
    # one pair went missing.
    classified = len(grounded) + len(empty)
    print(
        f"{checked} DS/file pairs checked, {bad} mismatched; of the {classified} "
        f"named in a doc block, {len(grounded)} land on shipped data and "
        f"{len(empty)} on zeros (runtime state, or a pair pointing nowhere -- "
        "the arithmetic cannot tell which)"
    )
    for path, f, ds in sorted(empty):
        print(f"   ZERO-REGION {path}: DS:{ds:#06x} -> file {f:#07x}")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
