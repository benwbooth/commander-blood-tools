#!/usr/bin/env python3
"""Find every short/near jump (and CALL) in the image whose computed target is a
given file offset.  Cheap complement to xref.py, which handles far refs and data
loads but not intra-segment relative branches.

    python3 re/tools/find_jump_target.py 0x83FF [-r START END]
"""

import os
import sys

RE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(RE_ROOT, "bin", "BLOODPRG.EXE")

SHORT = set([0xEB] + list(range(0x70, 0x80)))  # jmp rel8, jcc rel8
NEAR = {0xE8: "call", 0xE9: "jmp"}


def main():
    target = int(sys.argv[1], 16)
    lo, hi = 0, None
    if "-r" in sys.argv:
        i = sys.argv.index("-r")
        lo, hi = int(sys.argv[i + 1], 16), int(sys.argv[i + 2], 16)
    data = open(BIN, "rb").read()
    hi = hi or len(data)
    hits = []
    for off in range(lo, min(hi, len(data) - 6)):
        b = data[off]
        if b in SHORT:
            rel = data[off + 1]
            if rel > 127:
                rel -= 256
            if off + 2 + rel == target:
                hits.append((off, f"{b:02x} rel8"))
        elif b in NEAR:
            rel = int.from_bytes(data[off + 1 : off + 3], "little", signed=True)
            if off + 3 + rel == target:
                hits.append((off, f"{NEAR[b]} rel16"))
        elif b == 0x0F and 0x80 <= data[off + 1] <= 0x8F:
            rel = int.from_bytes(data[off + 2 : off + 4], "little", signed=True)
            if off + 4 + rel == target:
                hits.append((off, "jcc rel16"))
    print(f"{len(hits)} candidate branch(es) to {target:#x}")
    for off, kind in hits:
        print(f"  {off:#08x}  {kind}")


if __name__ == "__main__":
    main()
