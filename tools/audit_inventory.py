#!/usr/bin/env python3
"""Build the PORT AUDIT LEDGER: every function and struct in src/, each with its
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

ADDR = re.compile(r"0x[0-9A-Fa-f]{3,6}")
TEST_NAMES = set()

# Statuses the heuristics below can produce. Anything else in an existing ledger
# was set by hand and must survive a regeneration.
PROVISIONAL = {"ASM?", "ORACLE?", "DATA?", "INFRA?", "UNVERIFIED", ""}


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
        in_raw_string = False
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
            if stripped.startswith("#[cfg(test)]"):
                in_tests = True
            if stripped.startswith("///") or stripped.startswith("//!"):
                doc.append(stripped)
                continue
            m = re.match(
                r"\s*(?:pub(?:\([^)]*\))?\s+)?(fn|struct|enum)\s+([A-Za-z0-9_]+)", line
            )
            if not m:
                if stripped and not stripped.startswith("//"):
                    doc = []
                continue
            kind, name = m.group(1), m.group(2)
            if in_tests or name.startswith("test_"):
                doc = []
                continue
            doctext = " ".join(doc)[:400]
            addrs = ADDR.findall(doctext)
            origin = ",".join(dict.fromkeys(addrs))[:60]
            # provisional status
            low = doctext.lower()
            if any(k in low for k in ("oracle", "verified vs", "pixel-match", "capture")):
                status = "ORACLE?"
            elif addrs and any(
                k in low for k in ("exact", "transcri", "decoded", "asm", "0x")
            ):
                status = "ASM?"
            elif addrs:
                status = "ASM?"
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
