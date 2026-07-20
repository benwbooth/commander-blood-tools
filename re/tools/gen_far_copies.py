#!/usr/bin/env python3
"""Emit the far-callee copy layout the deterministic oracle used, per det-vector function.

auto_oracle.gen_det mirrors a copy of each transitively-reached far callee at
`target - 0x600` (see _far_targets). The Rust interpreter corpus test must replay vectors
against the SAME memory layout, so this dumps {func_name: [target, ...]} to
oracle_vectors/far_copies.json. Regenerate whenever new *_det.json vectors are added.

Run: PYTHONSAFEPATH=1 $SCRATCHPAD/uvenv/bin/python re/tools/gen_far_copies.py
"""
import dis  # noqa: F401  — bind the STDLIB dis before re/tools/ (which has its own dis.py) is on the path
import glob
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
import auto_oracle  # noqa: E402  (loads the EXE + capstone)

out = {}
vec_dir = os.path.join("re", "tools", "oracle_vectors")
for p in sorted(glob.glob(os.path.join(vec_dir, "func_*_det.json"))):
    name = os.path.basename(p)[: -len("_det.json")]
    entry = int(name[len("func_") :], 16)
    targets = sorted(auto_oracle._far_targets(entry))
    if targets:
        out[name] = targets
with open(os.path.join(vec_dir, "far_copies.json"), "w") as f:
    json.dump(out, f, indent=0, sort_keys=True)
print(f"{len(out)} det functions with far callees")
