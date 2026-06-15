#!/usr/bin/env python3
"""Byte-pattern search over BLOODPRG.EXE.

Usage:
    python3 re/tools/search_bytes.py <hex> [--context N] [--disasm [N]] [--limit N]

`hex` may be spaced or not, and may use `??` as a single-byte wildcard:
    python3 re/tools/search_bytes.py "2e ff 2f"          # cs: jmp [bx] style
    python3 re/tools/search_bytes.py "b8 ?? ?? cd 21"

Prints file offsets (which double as disassembly addresses).
"""
import os
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
import re as _re

sys.path.insert(0, _here)
from mzfile import MZ


def build_regex(hexstr):
    toks = hexstr.replace(",", " ").split()
    if len(toks) == 1 and len(toks[0]) > 2 and "?" not in toks[0]:
        s = toks[0]
        toks = [s[i:i + 2] for i in range(0, len(s), 2)]
    pat = b""
    for t in toks:
        if t in ("??", "?"):
            pat += b"."
        else:
            pat += _re.escape(bytes([int(t, 16)]))
    return _re.compile(pat, _re.DOTALL)


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return
    context = 0
    disasm = 0
    limit = 200
    hex_parts = []
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--context":
            context = int(args[i + 1]); i += 2
        elif a == "--disasm":
            disasm = 8
            if i + 1 < len(args) and args[i + 1].isdigit():
                disasm = int(args[i + 1]); i += 1
            i += 1
        elif a == "--limit":
            limit = int(args[i + 1]); i += 2
        else:
            hex_parts.append(a); i += 1

    mz = MZ()
    rx = build_regex(" ".join(hex_parts))
    hits = list(rx.finditer(mz.data))
    print(f"{len(hits)} match(es) for pattern {' '.join(hex_parts)}")
    for m in hits[:limit]:
        off = m.start()
        in_image = mz.header_size <= off < mz.image_total
        loc = "header" if off < mz.header_size else ("image" if in_image else "trailer")
        line = f"{off:#08x} [{loc}] {m.group().hex()}"
        if context:
            lo = max(0, off - context)
            hi = off + len(m.group()) + context
            line += f"  ctx={mz.data[lo:hi].hex()}"
        print(line)
    if len(hits) > limit:
        print(f"... {len(hits) - limit} more (raise --limit)")

    if disasm and hits:
        print("\n-- disasm of first hits --")
        for m in hits[:min(8, len(hits))]:
            print(f"\n@ {m.start():#08x}")
            os.system(f"python3 {os.path.join(_here, 'dis.py')} {m.start():#x} {disasm}")


if __name__ == "__main__":
    main()
