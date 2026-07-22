#!/usr/bin/env python3
"""Dump the distinct typed-chunk tags across all frames of an HNM(1) file, to find non-video
chunks (e.g. embedded audio) the video decoder skips. Header format mirrors src/hnm.rs::open."""
import sys

def parse_palette_block(d, pos):
    while pos + 1 < len(d):
        start, count = d[pos], d[pos + 1]
        pos += 2
        if start == 0xFF and count == 0xFF:
            break
        n = 256 if count == 0 else count
        for _ in range(n):
            if pos + 2 >= len(d):
                return pos
            pos += 3
    return pos

def main(path):
    d = open(path, "rb").read()
    header_size = d[0] | (d[1] << 8)
    pos = parse_palette_block(d, 2)
    while pos < len(d) and d[pos] == 0xFF:
        pos += 1
    offsets = []
    while pos + 3 < header_size and pos + 3 < len(d):
        offsets.append(int.from_bytes(d[pos:pos+4], "little"))
        pos += 4
    nframes = len(offsets) - 1 if len(offsets) > 1 else len(offsets)
    tags = {}
    audio_frames = []
    for i in range(nframes):
        abs_off = header_size + offsets[i]
        if abs_off + 2 > len(d):
            continue
        sc_size = d[abs_off] | (d[abs_off + 1] << 8)
        cpos, sc_end = abs_off + 2, abs_off + sc_size
        while cpos + 4 <= sc_end and cpos + 4 <= len(d):
            t0, t1 = d[cpos], d[cpos + 1]
            csz = d[cpos + 2] | (d[cpos + 3] << 8)
            if 0x20 <= t0 < 0x7f and 0x20 <= t1 < 0x7f and csz >= 4:
                tag = chr(t0) + chr(t1)
                tags[tag] = tags.get(tag, 0) + 1
                if tag not in ("pl",) and not (t0 == ord('p') and t1 == ord('l')):
                    audio_frames.append((i, tag, csz))
                cpos += csz
            else:
                break
    print(f"{path}: header_size={header_size} frames={nframes}")
    print(f"  chunk tags (tag: frame-count): {tags}")
    non_pl = [t for t in tags if t != 'pl']
    print(f"  NON-palette/video chunk tags: {non_pl}")
    if audio_frames:
        print(f"  first non-pl chunks: {audio_frames[:8]}")

if __name__ == "__main__":
    for p in sys.argv[1:]:
        main(p)
