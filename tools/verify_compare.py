#!/usr/bin/env python3
"""Score the dual-run differential: boot_frames/vs_NNN.ppm (oracle) vs vp_NNN.ppm
(port) per step; writes a side-by-side contact sheet + a TSV scorecard."""
import glob
import os
import sys

import numpy as np
from PIL import Image

# The scenario file gives each step's cursor -> mask the live-3D-hand region
# (the hand's phase/animation timing is compared by dedicated hand scenarios;
# these scenarios verify BEHAVIOR: surfaces, labels, palettes, boxes).
cursors = []
if len(sys.argv) > 1:
    cx, cy = 160, 100
    for line in open(sys.argv[1]):
        t = line.split()
        if not t or t[0].startswith("#"):
            continue
        if t[0] in ("move", "click"):
            cx, cy = int(t[1]), int(t[2])
        cursors.append((cx, cy))

rows = []
sheet_rows = []
for vs in sorted(glob.glob("boot_frames/vs_*.ppm")):
    n = os.path.basename(vs)[3:6]
    vp = f"boot_frames/vp_{n}.ppm"
    if not os.path.exists(vp):
        continue
    a = np.asarray(Image.open(vs)).astype(int)
    b = np.asarray(Image.open(vp)).astype(int)
    mask = np.ones((200, 320), dtype=bool)
    if cursors:
        cx, cy = cursors[min(int(n), len(cursors) - 1)]
        x0, x1 = max(0, cx - 95), min(320, cx + 35)
        y0, y1 = max(0, cy - 25), min(200, cy + 105)
        mask[y0:y1, x0:x1] = False
    d = np.abs(a - b)
    mean = d[mask].mean()
    close = (d.sum(axis=2)[mask] <= 24).mean()
    rows.append((n, mean, close))
    pair = Image.new("RGB", (640, 200))
    pair.paste(Image.fromarray(a.astype(np.uint8)), (0, 0))
    pair.paste(Image.fromarray(b.astype(np.uint8)), (320, 0))
    sheet_rows.append(pair)

os.makedirs("accuracy/comparisons/verify", exist_ok=True)
with open("accuracy/comparisons/verify/scorecard.tsv", "w") as f:
    f.write("step\tmean_abs\tpct_close\n")
    for n, m, c in rows:
        f.write(f"{n}\t{m:.2f}\t{c:.1%}\n")
if sheet_rows:
    sheet = Image.new("RGB", (640, 200 * len(sheet_rows)))
    for i, p in enumerate(sheet_rows):
        sheet.paste(p, (0, 200 * i))
    sheet.save("accuracy/comparisons/verify/sheet.png")
for n, m, c in rows:
    print(f"step {n}: mean_abs {m:6.2f}  close {c:6.1%}")
if rows:
    print(
        f"OVERALL: mean {np.mean([m for _, m, _ in rows]):.2f}"
        f"  close {np.mean([c for _, _, c in rows]):.1%}"
    )
