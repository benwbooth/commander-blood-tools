#!/usr/bin/env python3
"""Score the dual-run differential: boot_frames/vs_NNN.ppm (oracle) vs vp_NNN.ppm
(port) per step; writes a side-by-side contact sheet + a TSV scorecard."""
import glob
import os

import numpy as np
from PIL import Image

rows = []
sheet_rows = []
for vs in sorted(glob.glob("boot_frames/vs_*.ppm")):
    n = os.path.basename(vs)[3:6]
    vp = f"boot_frames/vp_{n}.ppm"
    if not os.path.exists(vp):
        continue
    a = np.asarray(Image.open(vs)).astype(int)
    b = np.asarray(Image.open(vp)).astype(int)
    mean = np.abs(a - b).mean()
    close = (np.abs(a - b).sum(axis=2) <= 24).mean()
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
