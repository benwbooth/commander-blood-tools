#!/usr/bin/env python3
"""Isolate the REAL hand from the HANDGRID oracle frames: per-pixel median across all
frames = hand-free background; per-frame |diff| > threshold = the hand mask. Reports the
hand bbox relative to the cursor position, its size, and its dominant colors."""
import glob
import re

import numpy as np
from PIL import Image

frames = {}
import sys
PREFIX = sys.argv[1] if len(sys.argv) > 1 else "boot_frames/hg"
for p in sorted(glob.glob(f"{PREFIX}_*.ppm")):
    m = re.match(r".*_(\d+)_(\d+)\.ppm", p)
    sx, sy = int(m.group(1)), int(m.group(2))
    frames[(sx, sy)] = np.asarray(Image.open(p)).astype(np.int16)

stack = np.stack(list(frames.values()))
bg = np.median(stack, axis=0).astype(np.int16)
Image.fromarray(bg.astype(np.uint8)).save(f"accuracy/comparisons/hand/{PREFIX.split(chr(47))[-1]}_bg.png")

print("cursor -> hand bbox (x0,y0,x1,y1), size WxH, tip offset from cursor")
report = []
for (sx, sy), fr in sorted(frames.items()):
    d = np.abs(fr - bg).sum(axis=2)
    mask = d > 40
    ys, xs = np.nonzero(mask)
    if len(xs) < 30:
        print(f"  ({sx:3},{sy:3}): no hand found")
        continue
    x0, x1, y0, y1 = xs.min(), xs.max(), ys.min(), ys.max()
    # tip = topmost hand pixel cluster
    tipx = int(np.median(xs[ys < y0 + 4]))
    report.append((sx, sy, x0, y0, x1, y1, tipx))
    print(
        f"  ({sx:3},{sy:3}): bbox=({x0:3},{y0:3})..({x1:3},{y1:3}) "
        f"{x1-x0+1:3}x{y1-y0+1:3}  tip=({tipx},{y0}) tip-cursor=({tipx-sx:+},{y0-sy:+}) "
        f"px={len(xs)}"
    )

# The hand's colors at one central position
fr = frames[(160, 88)]
d = np.abs(fr - bg).sum(axis=2)
mask = d > 40
pix = fr[mask]
print(f"\nhand pixel count @ (160,88): {mask.sum()}")
uniq, counts = np.unique(pix.reshape(-1, 3), axis=0, return_counts=True)
order = np.argsort(-counts)[:12]
print("dominant hand RGBs:")
for i in order:
    print(f"  {tuple(int(v) for v in uniq[i])} x{counts[i]}")

# Save the isolated hand for visual reference
iso = np.zeros_like(fr)
iso[mask] = fr[mask]
Image.fromarray(iso.astype(np.uint8)).resize((640, 400), Image.NEAREST).save(
    f"accuracy/comparisons/hand/{PREFIX.split(chr(47))[-1]}_hand_160_88.png"
)
