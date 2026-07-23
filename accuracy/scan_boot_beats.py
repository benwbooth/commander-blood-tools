"""Scan oracle boot frames for the WELCOME-ABOARD beat (green subtitle) and
cyan-pyramid content: print per-frame counts of subtitle-green and cyan pixels."""
import glob, sys

for path in sorted(glob.glob("boot_frames/boot_0*M.ppm")):
    data = open(path, "rb").read()
    # parse P6 header
    parts = data.split(b"\n", 3)
    px = parts[3]
    green = cyan = 0
    for i in range(0, min(len(px), 320 * 200 * 3), 3):
        r, g, b = px[i], px[i + 1], px[i + 2]
        if g > 180 and r < 120 and b < 120:
            green += 1
        if g > 140 and b > 140 and r < 110:
            cyan += 1
    if green > 200 or cyan > 2000:
        print(f"{path}: green={green} cyan={cyan}")
