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
# The oracle resumes a frozen savestate whose subtitle buffer still holds
# SCRIPT1's last line ("...out for the count..."); it is a start-state
# artifact, not driven content. Drop known pre-state lines from BOTH so the
# score reflects the beats the scenario actually drives (calibration: the
# real fix is a shared fresh boot, banked; this scores driven content now).
PRESTATE = {"the old turkey's out for the count..."}
oracle = [l for l in oracle if l not in PRESTATE]
port = [l for l in port if l not in PRESTATE]
oset, pset = set(oracle), set(port)
inter = oset & pset
print(f"oracle {len(oset)} lines, port {len(pset)} lines, matched {len(inter)}")
for l in sorted(oset - pset):
    print(f"  oracle-only: {l[:70]}")
for l in sorted(pset - oset):
    print(f"  port-only:   {l[:70]}")
