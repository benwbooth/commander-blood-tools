"""Analyze the BOOTIDX dialogue captures: index ranges by region, band stability
across beats, DAC values for the deck greys / subtitle / green digit."""
import sys

def load(m):
    idx = open(f"boot_frames/bd_{m:05d}M.idx", "rb").read()
    dac = open(f"boot_frames/bd_{m:05d}M.dac", "rb").read()
    return idx, dac

def region_hist(idx, x0, x1, y0, y1):
    h = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = idx[y * 320 + x]
            h[p] = h.get(p, 0) + 1
    return sorted(h.items(), key=lambda kv: -kv[1])

def dac_rgb(dac, i):
    r, g, b = dac[i * 3], dac[i * 3 + 1], dac[i * 3 + 2]
    return (r * 255 // 63, g * 255 // 63, b * 255 // 63)

for m in (218, 290):
    idx, dac = load(m)
    print(f"=== bd_{m}M ===")
    for name, (x0, x1, y0, y1) in {
        "deck (rows 150..200)": (0, 320, 150, 200),
        "orb (x140..180 y150..190)": (140, 180, 150, 190),
        "subtitle (rows 100..135, x60..260)": (60, 260, 100, 135),
        "digit (x0..16 y0..16)": (0, 16, 0, 16),
        "character (rows 20..90)": (0, 320, 20, 90),
    }.items():
        h = region_hist(idx, x0, x1, y0, y1)
        tops = ", ".join(f"{i}({c})#{dac_rgb(dac,i)}" for i, c in h[:8])
        print(f"  {name}: {tops}")

# Band stability: same deck indices across beats?
i218, _ = load(218)
i290, _ = load(290)
same = sum(
    1
    for y in range(140, 200)
    for x in range(320)
    if i218[y * 320 + x] == i290[y * 320 + x]
)
print(f"deck rows 140..200 identical 218M vs 290M: {same}/{60*320} = {same/(60*3.2):.1f}%")
# where does the static band actually START? scan rows for equality
for y in range(90, 145):
    row_same = sum(1 for x in range(320) if i218[y * 320 + x] == i290[y * 320 + x])
    if row_same > 300:
        print(f"first static row: {y} ({row_same}/320)")
        break
# DAC agreement between beats for the indices the deck uses
_, d218 = load(218)
_, d290 = load(290)
deck_idx = {i for i, _ in region_hist(i290, 0, 320, 150, 200)[:12]}
diffs = [i for i in deck_idx if d218[i*3:i*3+3] != d290[i*3:i*3+3]]
print(f"deck DAC entries differing between beats: {diffs}")
# compare vs port band harvest (console_band.bin rows 99..200)
band = open("accuracy/captures/console_band.bin", "rb").read()
rows = len(band) // 320
same_b = sum(
    1
    for y in range(rows)
    for x in range(320)
    if band[y * 320 + x] == i290[(99 + y) * 320 + x]
)
print(f"port console_band.bin vs oracle 290M rows 99..{99+rows}: {same_b}/{rows*320} = {same_b/(rows*3.2):.1f}%")
