#!/usr/bin/env python3
"""Which literal tables in the port are actually IN the game's data?

audit-fixes #227: `SHIP_3D_TEMP_SND_CALLBACK_OFFSETS = [0x87, 0x90, 0x9c]` sat
beside `..._TABLE_OFFSET = 0x0acc`, which is that array's own DS address. The
values never needed transcribing — they could be read, and they matched. The word
past them was zero, which settled the COUNT from the data too.

That pattern generalises. A table of hex literals is one of:

  IN-IMAGE   its bytes appear in BLOODPRG.EXE (or an overlay), found by SEARCH.
             It is game data, it can be READ instead of trusted, and a test can
             pin it. A UNIQUE match is much stronger than several.
  AT-ADDR    too short to search for without chance matches, but a constant
             within a few lines names a DS address, and the bytes ARE there. This
             is #227's case exactly: six bytes, invisible to a search, readable
             the moment you know where to look.
  ABSENT     neither. NOT automatically wrong -- audit-fixes #229 found four
             absentees with four different good reasons (a WIDENED word table, a
             DERIVED conversion, a known APPROX, and one genuinely unexplained).
             Absence is a QUESTION for a reader, which is why this prints rather
             than fails. But it did find a real defect on its first run
             (`EXT_WORLD_MAGIC`, #228), so the questions are worth asking.

Encoding: values are packed little-endian at the array's element width (u8/u16/
u32/i16). A u16 table also gets searched as bytes, since the port sometimes widens
a byte table for convenience.

Small tables match by chance, so anything under MIN_BYTES is not SEARCHED for --
it goes through the AT-ADDR path instead, and is only skipped if no neighbouring
constant names an address to check.

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


# A scalar const that could be a DS offset, so a SHORT table beside it can be
# checked AT that address instead of searched for. This is #227's pattern
# generalised: `SHIP_3D_TEMP_SND_CALLBACK_OFFSETS` was six bytes -- under
# MIN_BYTES, invisible to the search -- but the constant on the previous line was
# its own address, which made it readable.
SCALAR = re.compile(
    r"(?:pub\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*(?:u16|usize|u32)\s*=\s*0x([0-9A-Fa-f]+)\s*;"
)
DS_BASE = 0xD420
NEAR_LINES = 6


def ds_candidates(text):
    """line -> (name, ds_offset) for plausible DS addresses."""
    out = {}
    for m in SCALAR.finditer(text):
        value = int(m.group(2), 16)
        if 0 < value < 0x8000:  # inside the data segment
            out[text[: m.start()].count("\n") + 1] = (m.group(1), value)
    return out


def main():
    imgs = images()
    if not imgs:
        print("no images found; nothing to check")
        return 0

    in_image, absent, at_addr, skipped = [], [], [], 0
    for root, _, files in os.walk("src"):
        for f in sorted(files):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            text = open(path, encoding="utf-8", errors="replace").read()
            for m in ARRAY.finditer(text):
                name, kind, _count, body = m.groups()
                packed = parse_values(body, kind)
                if not packed:
                    skipped += 1
                    continue
                line = text[: m.start()].count("\n") + 1
                if len(packed) < MIN_BYTES:
                    # Too short to search for, but maybe a neighbour names its
                    # address. Check there instead.
                    neighbours = ds_candidates(text)
                    exe = next((d for n, d in imgs if n.endswith(".EXE")), None)
                    hit_at = None
                    for nline, (nname, offset) in neighbours.items():
                        if abs(nline - line) > NEAR_LINES or exe is None:
                            continue
                        at = DS_BASE + offset
                        if exe[at : at + len(packed)] == packed:
                            hit_at = (nname, at)
                            break
                    if hit_at:
                        at_addr.append((path, line, name, len(packed), hit_at))
                    else:
                        skipped += 1
                    continue
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
    for path, line, name, size, (nname, at) in at_addr:
        print(
            f"AT-ADDR  {path}:{line}: {name} ({size} bytes) matches the image at "
            f"{at:#07x}, the address {nname} names"
        )
    for path, line, name, size in absent:
        print(f"ABSENT   {path}:{line}: {name} ({size} bytes) is in no shipped image")

    print(
        f"{len(in_image) + len(absent) + len(at_addr)} literal table(s) checked: "
        f"{len(in_image)} found by search, {len(at_addr)} confirmed AT an address "
        f"a neighbouring constant names, {len(absent)} absent; "
        f"{skipped} too small to search and with no address beside them"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
