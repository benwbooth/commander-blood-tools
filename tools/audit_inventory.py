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
"""
import os
import re
import csv

SRC = "src"
OUT = "docs/function-audit.tsv"

ADDR = re.compile(r"0x[0-9A-Fa-f]{3,6}")
TEST_NAMES = set()

rows = []
for root, _, files in os.walk(SRC):
    for f in sorted(files):
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        text = open(path, encoding="utf-8", errors="replace").read()
        lines = text.splitlines()
        in_tests = False
        doc: list[str] = []
        for i, line in enumerate(lines, 1):
            stripped = line.strip()
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

from collections import Counter

c = Counter(r["status"] for r in rows)
print(f"{len(rows)} items -> {OUT}")
for k, v in sorted(c.items()):
    print(f"  {k:12} {v}")
