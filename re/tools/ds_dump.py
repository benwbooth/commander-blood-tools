#!/usr/bin/env python3
"""Dump the STATIC bytes behind a DS-relative address in BLOODPRG.EXE.

The image ships its initialised data segment, so a table the game reads at
`DS:0xNNNN` is already in the file at `0xD420 + 0xNNNN`. Several port constants
were documented as runtime PROBE dumps of such tables (`BRIDGEPROBE dump of
DS:0x2A1B`) when the same bytes can be read statically -- which is what the prime
rule asks for, since the binary is the source and the probe is verification.

Usage:
    python3 re/tools/ds_dump.py DS:0x2A1B 8 u16
    python3 re/tools/ds_dump.py 0xFE3B 16 u8      # a bare file offset works too

Formats: u8, u16, s16, u32, hex.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from mzfile import MZ

DS_BASE = 0xD420


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 0
    spec = args[0]
    count = int(args[1]) if len(args) > 1 else 8
    fmt = args[2] if len(args) > 2 else "u16"

    if spec.upper().startswith("DS:"):
        ds_off = int(spec[3:], 16)
        file_off = DS_BASE + ds_off
        origin = f"DS:{ds_off:#06x} -> file {file_off:#07x}"
    else:
        file_off = int(spec, 16)
        origin = f"file {file_off:#07x} (DS:{file_off - DS_BASE:#06x})"

    mz = MZ()
    print(origin)
    size = {"u8": 1, "u16": 2, "s16": 2, "u32": 4, "hex": 1}[fmt]
    for i in range(count):
        at = file_off + i * size
        raw = mz.data[at : at + size]
        if len(raw) < size:
            break
        value = int.from_bytes(raw, "little")
        if fmt == "s16" and value >= 0x8000:
            value -= 0x10000
        if fmt == "hex":
            print(f"  [{i:3}] {at:#07x}  {raw.hex()}")
        else:
            print(f"  [{i:3}] {at:#07x}  {value:#06x}  {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
