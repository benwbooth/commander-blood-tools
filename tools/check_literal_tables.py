#!/usr/bin/env python3
"""Which literal tables in the port are actually IN the game's data?

audit-fixes #227: `SHIP_3D_TEMP_SND_CALLBACK_OFFSETS = [0x87, 0x90, 0x9c]` sat
beside `..._TABLE_OFFSET = 0x0acc`, which is that array's own DS address. The
values never needed transcribing — they could be read, and they matched. The word
past them was zero, which settled the COUNT from the data too.

That pattern generalises. A table of hex literals is either:

  IN-IMAGE   its bytes appear in BLOODPRG.EXE (or an overlay). It is game data,
             it can be READ instead of trusted, and a test can pin it.
  ABSENT     its bytes appear nowhere. That is not automatically wrong -- derived
             tables, port-side lookup tables and scaled copies are legitimate --
             but a CONTENT-bearing table that is absent is the defect class
             CLAUDE.md names first, so each one wants a reason in its doc.

Encoding: values are packed little-endian at the array's element width (u8/u16/
u32/i16). A u16 table also gets searched as bytes, since the port sometimes widens
a byte table for convenience.

Small tables match by chance, so anything under MIN_BYTES is skipped rather than
reported as a find. A UNIQUE match is much stronger evidence than several, and the
count is printed for that reason.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import struct
import sys

BIN = os.path.join("re", "bin", "BLOODPRG.EXE")
OVERLAY_DIR = os.path.join("output", "_tmp_dat")
# Below this a match is coincidence more often than not.
MIN_BYTES = 8

ARRAY = re.compile(
    r"(?:pub\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*\[\s*(u8|u16|u32|i16|i32)\s*;\s*"
    r"([0-9_]+)\s*\]\s*=\s*\[(.*?)\]\s*;",
    re.S,
)
VALUE = re.compile(r"(-?)(?:0x([0-9A-Fa-f]+)|([0-9]+))")
PACK = {"u8": "<B", "u16": "<H", "u32": "<I", "i16": "<h", "i32": "<i"}


def parse_values(body, kind):
    out = []
    for sign, hexpart, decpart in VALUE.findall(body):
        raw = int(hexpart, 16) if hexpart else int(decpart)
        if sign:
            raw = -raw
        try:
            out.append(struct.pack(PACK[kind], raw))
        except struct.error:
            return None  # a value the declared type cannot hold: not a plain table
    return b"".join(out)


def images():
    out = []
    if os.path.exists(BIN):
        out.append((os.path.basename(BIN), open(BIN, "rb").read()))
    if os.path.isdir(OVERLAY_DIR):
        for f in sorted(os.listdir(OVERLAY_DIR)):
            if f.endswith((".xdb", ".drv", ".big", ".BIG")):
                out.append((f, open(os.path.join(OVERLAY_DIR, f), "rb").read()))
    return out


def occurrences(haystack, needle, limit=4):
    found, at = [], haystack.find(needle)
    while at >= 0 and len(found) < limit:
        found.append(at)
        at = haystack.find(needle, at + 1)
    return found


def main():
    imgs = images()
    if not imgs:
        print("no images found; nothing to check")
        return 0

    in_image, absent, skipped = [], [], 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            for m in ARRAY.finditer(text):
                name, kind, _count, body = m.groups()
                packed = parse_values(body, kind)
                if not packed or len(packed) < MIN_BYTES:
                    skipped += 1
                    continue
                line = text[: m.start()].count("\n") + 1
                hit = None
                for label, data in imgs:
                    where = occurrences(data, packed)
                    if where:
                        hit = (label, where)
                        break
                if hit:
                    in_image.append((path, line, name, len(packed), hit))
                else:
                    absent.append((path, line, name, len(packed)))

    for path, line, name, size, (label, where) in in_image:
        spots = ", ".join(f"{w:#07x}" for w in where)
        unique = "UNIQUE" if len(where) == 1 else f"{len(where)}x"
        print(f"IN-IMAGE {path}:{line}: {name} ({size} bytes) in {label} at {spots} [{unique}]")
    for path, line, name, size in absent:
        print(f"ABSENT   {path}:{line}: {name} ({size} bytes) is in no shipped image")

    print(
        f"{len(in_image) + len(absent)} literal table(s) >= {MIN_BYTES} bytes: "
        f"{len(in_image)} found in a shipped image (readable, not transcribed), "
        f"{len(absent)} absent; {skipped} too small or not plain tables"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
