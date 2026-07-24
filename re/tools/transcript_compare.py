#!/usr/bin/env python3
"""Score oracle vs port dialogue transcripts for one scenario: normalized
line-set overlap (whitespace/case-insensitive), reported as matched/oracle
and matched/port. The dual-run lane's line-level scorer."""
import sys, re

def norm(line):
    s = re.sub(r"\s+", " ", line.strip().lower())
    s = re.sub(r"\s+([?!,.:])", r"\1", s)  # renderer spacing, not behavior
    return s

def load(path):
    try:
        with open(path) as f:
            return [norm(l) for l in f if norm(l)]
    except FileNotFoundError:
        return []

oracle = load(sys.argv[1])
port = load(sys.argv[2])
oset, pset = set(oracle), set(port)
inter = oset & pset
print(f"oracle {len(oset)} lines, port {len(pset)} lines, matched {len(inter)}")
for l in sorted(oset - pset):
    print(f"  oracle-only: {l[:70]}")
for l in sorted(pset - oset):
    print(f"  port-only:   {l[:70]}")
