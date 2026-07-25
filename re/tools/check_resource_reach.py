#!/usr/bin/env python3
"""Which shipped RESOURCES can we PROVE are reached?

The resource-name table at file 0xCDF4 holds 16-byte filename records indexed by
resource id. Its extent is fixed by a layout identity: 0xCDF4 + 95*16 = 0xD3E4,
exactly the script-profile table, so there are 95 records (ids 0..94) -- which
agrees with the highest id the world-art table uses (94, ondoya.ext).

THIS TOOL PROVES REACHABILITY, NEVER DEADNESS -- and that asymmetry is the point.
The UI-string sweep (check_ui_strings.py) can prove a string dead because a draw
site MUST load its offset as an immediate; there is nowhere else for the offset to
come from. Resource ids are different: they arrive in AX from DATA -- the .ext
object records feed entity_object_populate (0x40D0) with ids the executable never
mentions. So a zero here means "not reached by any route this tool models", not
"unused".

The counter-example that establishes it: id 16 is `borxx.spr`, the EYE ORB the
nav HUD draws every frame. It appears in no profile row, no world-art record, and
no `mov ax,16` -- and it is obviously live. Any list of "dead resources" built
from these three routes would have started with it.

Routes modelled:
  * `mov ax,<id>` (how immediate-driven loads are fed),
  * the SCRIPT-PROFILE table at file 0x0D3E4 (5 profiles x 5 slots),
  * the WORLD-ARTWORK table at DS:0x2BC7 (42 name -> id records).

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from mzfile import DEFAULT_BIN  # noqa: E402

NAME_TABLE = 0xCDF4
NAME_LEN = 16
# LAYOUT IDENTITY bounds the table: 0xCDF4 + 95*16 = 0xD3E4, exactly the
# script-profile table. So there are 95 records, ids 0..94 -- which also matches
# the highest id the world-art table uses (94, ondoya.ext). Walking past that
# reads the profile rows and the UI string table as though they were records.
NAME_COUNT = 95
PROFILES = 0x0D3E4
PROFILE_COUNT, PROFILE_SLOTS = 5, 5
ART_TABLE = 0xD420 + 0x2BC7
ART_REC = 0x16


def main():
    data = open(DEFAULT_BIN, "rb").read()

    assert NAME_TABLE + NAME_COUNT * NAME_LEN == PROFILES, "the table must close on the profiles"
    names = []
    for i in range(NAME_COUNT):
        rec = data[NAME_TABLE + i * NAME_LEN : NAME_TABLE + (i + 1) * NAME_LEN]
        names.append((i, rec.split(b"\0")[0].decode("latin-1")))

    profile_ids = set()
    for p in range(PROFILE_COUNT):
        row = data[PROFILES + p * PROFILE_SLOTS * 2 : PROFILES + (p + 1) * PROFILE_SLOTS * 2]
        profile_ids.update(struct.unpack("<5H", row))

    art_ids, i = set(), 0
    while True:
        rec = data[ART_TABLE + i * ART_REC : ART_TABLE + (i + 1) * ART_REC]
        if len(rec) < ART_REC or rec[0] == 0:
            break
        art_ids.add(struct.unpack("<H", rec[16:18])[0])
        i += 1

    def mov_ax_sites(value):
        lo, hi = value & 0xFF, (value >> 8) & 0xFF
        return sum(
            1
            for off in range(len(data) - 3)
            if data[off] == 0xB8 and data[off + 1] == lo and data[off + 2] == hi
        )

    print(f"{len(names)} resource-name records\n")
    dead = []
    for rid, text in names:
        if not text:
            continue
        movs = mov_ax_sites(rid)
        via = []
        if rid in profile_ids:
            via.append("profile")
        if rid in art_ids:
            via.append("world-art")
        if movs:
            via.append(f"mov ax x{movs}")
        if not via:
            dead.append((rid, text))
        print(f"  {'DEAD ' if not via else '     '}id {rid:3d}  {text:<16} {', '.join(via)}")

    print(f"\n{len(names) - len(dead)} of {len(names)} ids reached by a modelled route.")
    print(f"{len(dead)} NOT reached by any modelled route -- NOT a dead list:")
    for rid, text in dead:
        print(f"    id {rid:3d}  {text}")
    print(
        "\nRead that list as `route unknown`, not `unused`: id 16 borxx.spr is on it\n"
        "and is the eye orb the nav HUD draws every frame. Resource ids reach the\n"
        "loader from .ext object data, which this tool does not read. `mov ax,imm`\n"
        "counts also include coincidental matches for small ids, so a HIGH count is\n"
        "weak evidence too."
    )


if __name__ == "__main__":
    raise SystemExit(main())
