#!/usr/bin/env python3
"""Check the claim behind 0x8389/0x83EA: an object record carries its NAME as a
NUL-terminated string at record+4.

Both status-list consumers copy from `si+4` (`mov si,bp / add si,4 / lodsb` at
0x8389, `add si,4` before the 0x299:0x202 draw at 0x91E1), which only makes sense
if +4 is inline text.  The DEB gives (name, offset, kind) per symbol, so the test
is: for every kind-1 DEB symbol, does VAR[offset+4:] start with that name?
"""

import os
import sys

ISO = sys.argv[1] if len(sys.argv) > 1 else "output/_tmp_iso"


def parse_deb(deb):
    """Mirror of src/script.rs parse_deb: 20-byte entries, name at +0, offset at
    +0x10, kind at +0x12."""
    out = []
    for base in range(0, len(deb) - 19, 20):
        raw = deb[base : base + 16]
        name = raw.split(b"\0")[0].decode("latin-1")
        off = int.from_bytes(deb[base + 0x10 : base + 0x12], "little")
        kind = int.from_bytes(deb[base + 0x12 : base + 0x14], "little")
        out.append((name, off, kind))
    return out


def main():
    total = hit = miss = 0
    for n in range(1, 6):
        deb_path = os.path.join(ISO, f"SCRIPT{n}.DEB")
        var_path = os.path.join(ISO, f"SCRIPT{n}.VAR")
        if not (os.path.exists(deb_path) and os.path.exists(var_path)):
            print(f"SCRIPT{n}: missing")
            continue
        deb = open(deb_path, "rb").read()
        var = open(var_path, "rb").read()
        n_hit = n_miss = 0
        examples = []
        for name, off, kind in parse_deb(deb):
            if kind != 1:
                continue
            total += 1
            at = var[off + 4 : off + 4 + 32].split(b"\0")[0].decode("latin-1")
            if at.lower() == name.lower():
                n_hit += 1
                if len(examples) < 4:
                    examples.append(f"{off:#06x}->{at!r}")
            else:
                n_miss += 1
                if n_miss <= 4:
                    examples.append(f"MISS {name!r} @{off:#06x} -> {at!r}")
        hit += n_hit
        miss += n_miss
        print(f"SCRIPT{n}: {n_hit} match / {n_miss} miss   {' '.join(examples)}")
    print(f"TOTAL kind-1 objects {total}: {hit} carry their name inline at +4, {miss} do not")


if __name__ == "__main__":
    main()
