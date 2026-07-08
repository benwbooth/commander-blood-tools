"""Scan all leaf functions: run lift_cfg on each, report which lift CLEAN (no TODO
markers). Prints a summary and writes re/tools/cfg_clean.json (sorted hex list) so the
generation step knows which leaves are ready for oracle verification."""
import json, sys, os, dis as _stdlib_dis  # load stdlib dis first so it's cached
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lift

leaves = json.load(open("re/func_graph.json"))["leaves"]
clean, blocked = [], {}
for off in sorted(set(leaves)):
    if not (0x600 <= off < 0xd000):
        continue
    name = f"func_{off:x}"
    try:
        src = lift.lift_cfg(off, name)
    except Exception as e:
        blocked[off] = f"exc:{type(e).__name__}:{e}"
        continue
    todos = [l.split("TODO(lifter): ")[1] for l in src.splitlines() if "TODO(lifter)" in l]
    if todos:
        blocked[off] = todos[0]
    else:
        clean.append(off)

json.dump([f"0x{a:x}" for a in sorted(clean)], open("re/tools/cfg_clean.json", "w"))
print(f"CLEAN leaves: {len(clean)} / {len([a for a in set(leaves) if 0x600<=a<0xd000])}")
print("clean:", " ".join(f"0x{a:x}" for a in sorted(clean)))
# histogram of first blocking reason
from collections import Counter
hist = Counter(v.split("(")[0].split(" size")[0].strip() for v in blocked.values())
print("\nTop blockers:")
for reason, n in hist.most_common(20):
    print(f"  {n:3d}  {reason}")
