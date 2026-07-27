#!/usr/bin/env python3
"""Build the PORT AUDIT LEDGER: every function, struct and CONSTANT in src/, each with its
claimed binary origin (asm address cited in its doc comment / labels.csv) and its
verification status. The ledger is the driving worklist for the systematic
check-every-ported-item campaign:

  status meanings
    ORACLE   - differentially verified against the interpreter (bit/pixel/sequence)
    ASM      - literal transcription with the address cited, reviewed against disasm
    DATA     - loads/parses banked game data whose layout is decode-verified
    TESTED   - has a unit/regression test but no external ground truth
    INFRA    - port plumbing with no binary counterpart (windowing, GPU, CLI...)
    UNVERIFIED - no citation, no test: highest priority

Constants were originally left out, which hid the most directly checkable things
in the port: the TABLES copied byte-for-byte out of the binary. Four of them
(`GAME_FONT_GLYPHS`, `GAME_FONT_WIDTHS`, `NAV_DESTINATION_POINTS`,
`SHIP_3D_HUD_PYRAMID_VERTICES`) had no image comparison at all and no ledger row
to say so. Including them grows the denominator, which is the honest direction:
the ledger should enumerate the port's real surface.

Output: docs/function-audit.tsv (item, file, line, kind, origin, status, evidence)
Heuristics assign a PROVISIONAL status from doc comments; the campaign's job is to
upgrade every row to ORACLE/ASM/DATA/INFRA with real evidence, one by one.

CURATED STATUSES ARE PRESERVED. The heuristics can only ever produce the
provisional forms (`ASM?`, `ORACLE?`, `DATA?`, `INFRA?`, `UNVERIFIED`), so a row
carrying a SETTLED status (`ASM`, `ORACLE`, `DATA`, `INFRA`, `TESTED`) was
upgraded by hand after real evidence. Re-running this script used to overwrite
those with a fresh guess, silently throwing away the campaign's work -- a whole
pass of verification could vanish because someone regenerated the file, which
CLAUDE.md tells you to do after every pass. Now the previous file is read first
and any settled status is carried forward by (item, file).
"""
import collections
import os
import re
import csv

SRC = "src"
OUT = "docs/function-audit.tsv"

# NOT preceded by an alphanumeric: "320x200" contains the substring "0x200", so a
# plain pattern harvested a PHANTOM citation from every screen-dimension string in
# a doc, and 11 rows were provisionally ASM? on that basis alone -- rows that look
# evidenced and are not.
ADDR = re.compile(r"(?<![0-9A-Za-z])0x[0-9A-Fa-f]{3,6}")
# OVERLAY citations live in a different address space, where the >=0x600 MZ bound
# below is simply wrong: manu3.xdb's method entries are 0x000, 0x181, 0x19B, 0x1DF
# and its matrix build is 0x270..0x3DE. Filtering those as "prose numbers" made
# nine genuinely decoded overlay functions look uncited, and `audit_settle.py`
# then REFUSED to settle them for want of an address they had all along
# (audit-fixes #485). The qualified `XDB:<name>:0xNNN` form re/CLAUDE.md already
# defines is what distinguishes the spaces, so it is matched separately and kept
# WITH its prefix -- an overlay 0x19B must never compare equal to an image 0x19B.
XDB_ADDR = re.compile(r"XDB:[A-Za-z0-9_]+:0x[0-9A-Fa-f]{1,6}")


def cited(text):
    """Any citation, in either address space."""
    return XDB_ADDR.search(text) or ADDR.search(text)
TEST_NAMES = set()

# Statuses the heuristics below can produce. Anything else in an existing ledger
# was set by hand and must survive a regeneration.
PROVISIONAL = {"ASM?", "ORACLE?", "DATA?", "INFRA?", "CELL?", "UNVERIFIED", ""}

# audit-fixes #317. An address in a doc is NOT automatically a routine citation.
# Three rows in the #298 review cited DATA: "SCRIPT2: 0x0F4E" (a script record
# offset), `gs:0x1FA7` and `gs:0x6772` (DS cells). Disassembling the executable at
# any of them yields convincing phantoms, and `check_cited_instructions.py`
# correctly declines to check them -- so the row read as evidenced while nothing
# could ever verify it.
#
# The discriminator is a MNEMONIC beside the address, which is exactly what makes
# a citation checkable. Rows whose addresses never carry one become `CELL?`
# instead of `ASM?`: still provisional, but honestly labelled as "names a cell,
# not a routine" so the queue can be worked by kind.
MNEMONIC_NEAR = re.compile(
    r"(?<![0-9A-Za-z])0x[0-9A-Fa-f]{3,6}[^\n]{0,24}?\b(mov|cmp|test|jmp|je|jne|jb|jae|ja|jbe|jg|jl|"
    r"call|lcall|ret|retf|push|pop|add|sub|and|or|xor|inc|dec|shl|shr|sar|mul|imul|div|lea|les|lds|"
    r"lodsb|lodsw|lodsd|stosb|stosw|stosd|movsx|movzx|cbw|cwde|neg|not|bsf|rep|xchg)\b"
    r"|\b(mov|cmp|test|jmp|je|jne|jb|jae|ja|jbe|jg|jl|call|lcall|ret|retf|push|pop|add|sub|and|or|"
    r"xor|inc|dec|shl|shr|sar|mul|imul|div|lea|les|lds|lodsb|lodsw|lodsd|stosb|stosw|stosd|movsx|"
    r"movzx|cbw|cwde|neg|not|bsf|rep|xchg)\b[^\n]{0,24}?(?<![0-9A-Za-z])0x[0-9A-Fa-f]{3,6}",
    re.I,
)


def load_curated(path):
    """(item, file) -> settled status, from a previous run of this script.

    `(item, file)` is not a key: a name can occur twice in one file (a nested
    helper sharing the outer name, two `default` impls). Carrying a status onto an
    AMBIGUOUS name would credit the wrong function, so ambiguous names are dropped
    and fall back to the heuristic -- they are reported at the end so they can be
    re-settled by hand.
    """
    if not os.path.exists(path):
        return {}, {}, {}, set()
    seen = collections.Counter()
    settled = {}
    by_origin = {}
    by_ordinal = {}
    with open(path, newline="") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            key = (row["item"], row["file"])
            ordinal_key = key + (seen[key],)
            seen[key] += 1
            if (row.get("status") or "").strip() not in PROVISIONAL:
                by_ordinal[ordinal_key] = (row["status"].strip(), seen[key] - 1)
            status = (row.get("status") or "").strip()
            if status not in PROVISIONAL:
                settled[key] = status
                # The cited-address list can separate same-named siblings — but
                # ONLY when their origins differ. Two siblings that both cite
                # nothing (or the same address) are genuinely indistinguishable
                # here, and a collision poisons the entry to None so the caller
                # refuses to guess.
                ok = key + (row.get("origin", ""),)
                by_origin[ok] = None if ok in by_origin else status
    ambiguous = {k for k in settled if seen[k] > 1}
    return (
        {k: v for k, v in settled.items() if k not in ambiguous},
        by_origin,
        {"status": by_ordinal, "counts": dict(seen)},
        ambiguous,
    )


CURATED, CURATED_BY_ORIGIN, CURATED_BY_ORDINAL, AMBIGUOUS = load_curated(OUT)

rows = []
for root, _, files in os.walk(SRC):
    for f in sorted(files):
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        text = open(path, encoding="utf-8", errors="replace").read()
        lines = text.splitlines()
        in_tests = False
        test_depth = None
        in_raw_string = False
        # Brace depth INSIDE a function body, so local items can be skipped.
        fn_body_depth = 0
        pending_fn = False
        doc: list[str] = []
        for i, line in enumerate(lines, 1):
            stripped = line.strip()
            # Raw string literals hold WGSL shader source whose `fn vs`/`struct
            # VOut` the regex below happily counted as Rust items -- six phantom
            # rows in gpu.rs alone, inflating the ledger's denominator with
            # things that are not port code at all.
            if not in_raw_string and 'r#"' in line:
                in_raw_string = '"#' not in line.split('r#"', 1)[1]
                continue
            if in_raw_string:
                if '"#' in line:
                    in_raw_string = False
                continue
            # `in_tests` used to LATCH: once a file had a test module, every item
            # after it vanished from the ledger. src/font.rs keeps real code after
            # its test module -- BoldConsoleFont and all three SQUARE_CAPS tables
            # -- so those were invisible. Track the module's braces and clear the
            # flag when it closes.
            if stripped.startswith("#[cfg(test)]"):
                in_tests = True
                test_depth = None
            if in_tests:
                if test_depth is None:
                    if "{" in line:
                        test_depth = line.count("{") - line.count("}")
                else:
                    test_depth += line.count("{") - line.count("}")
                if test_depth is not None and test_depth <= 0:
                    in_tests = False
                    test_depth = None
                    # fall through: this line may itself declare an item
            # `//` counts too, not just `///`. Constants inside a function body are
            # documented with plain comments (a doc comment on a local item is
            # unidiomatic), so restricting to `///` left every in-function constant
            # with no origin and therefore permanently unsettleable -- the same
            # blind spot as attributes breaking the association.
            if (
                stripped.startswith("///")
                or stripped.startswith("//!")
                or stripped.startswith("//")
            ):
                doc.append(stripped)
                continue
            # Track whether we are inside a fn body BEFORE matching items, so a
            # local `const` is skipped and a following module-level one is not.
            if fn_body_depth > 0:
                fn_body_depth += line.count("{") - line.count("}")
                if fn_body_depth <= 0:
                    fn_body_depth = 0
            elif pending_fn:
                fn_body_depth += line.count("{") - line.count("}")
                if fn_body_depth > 0:
                    pending_fn = False
                elif ";" in line:
                    pending_fn = False  # a trait method signature, no body
            # `const fn e(...)` is a FUNCTION named `e`, not a const named `fn`.
            # The old pattern took the first keyword and then the next word, so
            # every `const fn` in the tree became a row literally called `fn`
            # (src/ship3d.rs:461 among them) -- an item that cannot be settled
            # because it does not exist.
            m = re.match(
                r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?"
                r"(fn|struct|enum|const|static)\s+([A-Za-z0-9_]+)",
                line,
            )
            if not m:
                # An ATTRIBUTE sits between the doc comment and the item it
                # documents (`#[allow(clippy::too_many_arguments)]`, `#[derive]`,
                # `#[inline]`). Clearing the doc here stripped those items of their
                # origin, so a fully cited function could never be settled -- the
                # same shape as constants being left out of the ledger entirely.
                # A BLANK LINE ends the run. Only a non-empty, non-comment line
                # used to clear it, so a note separated from the next item by a
                # blank line still attached to it -- #119's twin note sat above
                # DIALOGUE_FONT_GLYPH_HEIGHT and gave a font constant an origin of
                # 0x1B29/0x1B3D, the text-speed addresses. Rust doc comments must
                # be adjacent to their item, so nothing legitimate is lost.
                if not stripped:
                    doc = []
                elif not stripped.startswith("//") and not stripped.startswith("#["):
                    doc = []
                continue
            kind, name = m.group(1), m.group(2)
            # FUNCTION-LOCAL items are usually not port surface -- `render_star_
            # map_navview_projected` declares `const W`/`const H` as aliases of
            # already-settled screen constants, and three such pairs were separate
            # ledger rows making no independent claim.
            #
            # BUT a local item WITH A CITATION does make one. `engine.rs`'s
            # `const TEXT_SELECTED = 0xEF` sits inside a function and carries
            # `mov al,0xEF` @0x858B; dropping it silently deleted a SETTLED ASM row
            # from the ledger. The first cut of this rule did exactly that to
            # TEXT_SELECTED, TEXT_SELECTED_MOUSE, CREDIT_RECORD, NAV_CAMERA_ORIGIN
            # and others -- caught by diffing the item set before and after, which
            # is the check any inventory change needs.
            #
            # So: skip a local only when NEITHER its doc NOR its own declaration
            # carries a hex literal. `const PAL_DS: u32 = 0x5251` has no doc at all
            # but is a decoded address driving the recomp machine; `const W: isize
            # = SHIP_3D_PROJECTION_SCREEN_WIDTH as isize` is an alias and has no
            # hex anywhere. Erring toward KEEPING is deliberate: an extra row costs
            # a slightly larger denominator, while dropping a decoded value removes
            # port surface from the ledger silently.
            if (
                fn_body_depth > 0
                and kind != "fn"
                and not cited(" ".join(doc))
                and not re.search(r"0x[0-9A-Fa-f]+", line)
            ):
                continue
            if kind == "fn":
                # Arm body tracking; the brace may be on this line or the next.
                pending_fn = True
                fn_body_depth += line.count("{") - line.count("}")
                if fn_body_depth > 0:
                    pending_fn = False
            if in_tests or name.startswith("test_"):
                doc = []
                continue
            fulldoc = " ".join(doc)
            # A citation may live in the function BODY rather than the doc: 56 of
            # the 83 uncited-ASM functions in #141's queue had an address in a body
            # comment, which is a real citation placed where the ledger did not
            # look. Scan the body's COMMENT lines only -- a bare literal in code is
            # a VALUE, and treating it as an address is how "320x200" became a
            # citation in #123.
            if not cited(fulldoc):
                body_comments = []
                depth, started = 0, False
                for offset, probe in enumerate(lines[i - 1 : i + 80]):
                    depth += probe.count("{") - probe.count("}")
                    if "{" in probe:
                        started = True
                    # A BRACE-LESS ITEM HAS NO BODY TO SCAN. `const SILENCE: u8 =
                    # 0x80;` never opens a block, so `started` stayed false and
                    # this loop ran the full 80 lines forward, absorbing every
                    # comment it passed -- including the doc comments of later
                    # functions and, in SILENCE's case, a TEST comment citing
                    # 0x4049/0xBB6D. The constant was then filed ASM? with two
                    # addresses that do not contain it (audit-fixes #252).
                    #
                    # A real body opens within a line or two of the declaration,
                    # so if no brace has appeared by then there is nothing to scan.
                    if not started and offset > 2:
                        body_comments = []
                        break
                    st_probe = probe.strip()
                    if st_probe.startswith("//"):
                        body_comments.append(st_probe)
                    if started and depth <= 0:
                        break
                fulldoc = (fulldoc + " " + " ".join(body_comments)).strip()
            doctext = fulldoc[:400]  # evidence column stays readable
            # ...but addresses come from the WHOLE doc: a long transcription puts
            # its citations past 400 chars, and truncating them away made a
            # thoroughly cited function look uncited.
            # Take the qualified overlay citations first, then scrub them so the
            # bare-address pass cannot re-harvest their `0x...` tail as an image
            # address (and drop it, or worse, keep it unqualified).
            xdb_hits = XDB_ADDR.findall(fulldoc)
            addrs = ADDR.findall(XDB_ADDR.sub(" ", fulldoc))
            # DROP values below the 0x600 MZ header: they are not code addresses
            # in this image (audit-fixes #387 established the same bound for
            # labels.csv). Refreshing the ledger after a session of doc edits
            # made `0x0`, `0x100`, `0x181` and friends -- ordinary numbers in
            # prose -- look like citations, and the duplicate-rule test then
            # reported 65 addresses 'cited by more than one port function',
            # nearly all of them junk (audit-fixes #423).
            addrs = xdb_hits + [a for a in addrs if int(a, 16) >= 0x600]
            origin = ",".join(dict.fromkeys(addrs))[:60]
            # provisional status
            low = doctext.lower()
            if any(k in low for k in ("oracle", "verified vs", "pixel-match", "capture")):
                status = "ORACLE?"
            elif addrs and MNEMONIC_NEAR.search(fulldoc):
                # At least one address sits beside an instruction, so the guard
                # can check it and the row is a genuine decode claim.
                status = "ASM?"
            elif addrs:
                # Addresses but never an instruction: a DATA CELL reference
                # (`gs:0x1FA7`), a script-space offset, or a bare region pointer.
                status = "CELL?"
            elif any(k in low for k in ("banked", "dump", "extracted", "blood.dat", "lbm", "hnm", "descript")):
                status = "DATA?"
            elif f in ("main.rs", "gpu.rs") or "window" in low or "wgpu" in low or name.startswith("run_"):
                status = "INFRA?"
            else:
                status = "UNVERIFIED"
            rows.append(
                {
                    "item": name,
                    "file": path,
                    "line": i,
                    "kind": kind,
                    "origin": origin,
                    "status": status,
                    "evidence": doctext[:200],
                }
            )
            # Cleared after every item. Letting a doc run carry across CONSECUTIVE
            # items (so a group comment could cover `const TEXT` /
            # `const TEXT_SELECTED`) looked like a recall win -- 367 -> 594 rows
            # with an origin -- but most of that was bleed: a long run of
            # `pub const` lines in bloodprg.rs handed the font map's "176, NOT 128"
            # doc to unrelated sprite and ship-3D offsets. An origin asserts the row
            # IS evidenced, so a false one is worse than a missing one. Grouped
            # constants get their own comment instead.
            doc = []

# Carry forward hand-upgraded statuses, but only onto rows the key identifies
# UNAMBIGUOUSLY on BOTH sides: a name that now occurs twice in its file would
# otherwise credit the nested helper with the outer function's verification.
new_counts = collections.Counter((r["item"], r["file"]) for r in rows)
new_origin_counts = collections.Counter((r["item"], r["file"], r["origin"]) for r in rows)
ambiguous_now = {k for k in CURATED if new_counts[k] > 1}
applied = 0
recovered = set()
by_position = set()
unresolved = set()
occurrence = collections.Counter()
for r in rows:
    key = (r["item"], r["file"])
    origin_key = key + (r["origin"],)
    ordinal_key = key + (occurrence[key],)
    occurrence[key] += 1
    if key in CURATED and key not in ambiguous_now:
        r["status"] = CURATED[key]
        applied += 1
    elif (
        CURATED_BY_ORIGIN.get(origin_key) is not None
        and new_origin_counts[origin_key] == 1
    ):
        # Ambiguous by name, but the cited addresses pick the right sibling —
        # only usable when the siblings' origins actually DIFFER, which
        # load_curated has already checked.
        r["status"] = CURATED_BY_ORIGIN[origin_key]
        applied += 1
        recovered.add(key)
    elif (
        ordinal_key in CURATED_BY_ORDINAL["status"]
        and CURATED_BY_ORDINAL["counts"].get(key) == new_counts[key]
    ):
        # Last resort for names that cannot be told apart any other way (trait
        # `default` impls): the file still has the SAME NUMBER of them, so match
        # by position. Reordering breaks this, which is why the count must agree
        # and the recovery is reported.
        r["status"] = CURATED_BY_ORDINAL["status"][ordinal_key][0]
        applied += 1
        by_position.add(key)
    elif key in ambiguous_now or key in AMBIGUOUS:
        unresolved.add(key)
vanished = {k for k in CURATED if new_counts[k] == 0}

os.makedirs("docs", exist_ok=True)
with open(OUT, "w", newline="") as fh:
    w = csv.DictWriter(
        fh,
        fieldnames=["item", "file", "line", "kind", "origin", "status", "evidence"],
        delimiter="\t",
    )
    w.writeheader()
    for r in rows:
        w.writerow(r)

c = collections.Counter(r["status"] for r in rows)
print(f"{len(rows)} items -> {OUT}")
for k, v in sorted(c.items()):
    print(f"  {k:12} {v}")
print(f"  carried forward {applied} hand-settled status(es)")
for label, keys in (
    ("same-named siblings resolved by their cited addresses", recovered),
    ("same-named siblings resolved BY POSITION (count unchanged)", by_position),
    ("STILL ambiguous -- re-settle by hand", unresolved),
    ("VANISHED (renamed or deleted)", vanished),
):
    if keys:
        print(f"  !! {len(keys)} settled status(es) dropped -- {label}:")
        for item, path in sorted(keys):
            print(f"     {item}  {path}")
