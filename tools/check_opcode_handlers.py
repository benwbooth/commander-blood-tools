#!/usr/bin/env python3
"""Does each `OP_*` constant's cited HANDLER match the VM dispatch table?

An opcode constant cannot be checked the way other constants are: its value is a
table INDEX, so it appears nowhere in the handler's bytes, and
`check_cited_immediates.py` can only report it as needing reading. But the claim
those docs make is checkable — stronger, in fact, than an immediate match:

    /// `0xA4` unconditional JUMP (PC = operand). Handler 0x65db.
    pub const OP_JUMP: u8 = 0xA4;

says entry `0xA4` of the dispatch table points at `0x65DB`. The table is at file
`0x142D0` (`vm_opcode_handler_table_static`, copied to `GS:0x6EB0` at init): 52
near offsets into VM code segment `0x4DA`, for opcodes `0xA0..0xD3`. So

    handler_file_off = 0x600 + 0x4DA * 16 + table[opcode - 0xA0]

and a doc citing any other address for that opcode is simply wrong.

This resolves every `OP_<NAME> = 0x<hex>` constant whose doc cites an address,
and reports the ones whose cited handler is not the dispatched one. Opcodes
outside `0xA0..0xD3` are out of the table's range and are reported separately
rather than silently passed.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import os
import re
import sys

sys.path.insert(0, os.path.join("re", "tools"))

from mzfile import MZ  # noqa: E402

TABLE = 0x142D0
FIRST_OP = 0xA0
LAST_OP = 0xD3
CODE_SEG = 0x4DA
SEG_BASE = 0x600 + CODE_SEG * 16

OPCONST = re.compile(r"^\s*(?:pub\s+)?const\s+(OP_[A-Z0-9_]+)\s*:\s*u8\s*=\s*(0x[0-9A-Fa-f]+)\s*;")
# ONLY addresses introduced as a handler. Reading every address in the doc made
# `0xCE`'s citation of the game-flag words [0x2793]/[0x252a] look like a wrong
# handler claim -- the doc never claimed they were handlers.
HANDLER_CITE = re.compile(r"[Hh]andlers?\s+((?:0x[0-9A-Fa-f]{3,6}[/, ]*)+)")
ADDR = re.compile(r"0x([0-9A-Fa-f]{3,6})")


def handler_for(mz, opcode):
    if not (FIRST_OP <= opcode <= LAST_OP):
        return None
    off = TABLE + (opcode - FIRST_OP) * 2
    entry = int.from_bytes(mz.data[off : off + 2], "little")
    return SEG_BASE + entry


def constants(path="src/vm.rs"):
    """Yield (line, name, opcode, [cited addresses]) for OP_* constants."""
    doc = []
    for n, line in enumerate(open(path, encoding="utf-8").read().splitlines(), 1):
        st = line.strip()
        if st.startswith("///") or st.startswith("//"):
            doc.append(st)
            continue
        m = OPCONST.match(line)
        if m:
            blob = " ".join(doc)
            addrs = [
                int(a, 16)
                for run in HANDLER_CITE.findall(blob)
                for a in ADDR.findall(run)
            ]
            yield n, m.group(1), int(m.group(2), 16), addrs
            # Clear it. Letting a doc run carry across constants (to support
            # `0xAA`/`0xAC` sharing one) instead made every constant inherit the
            # GROUP comment above the block, so 18 of them "cited" the same five
            # addresses and every one looked wrong. A constant with no doc of its
            # own is reported as uncited, which is honest.
            doc = []
            continue
        if st and not st.startswith("#["):
            doc = []


def main():
    mz = MZ()
    checked = wrong = norange = uncited = 0
    problems = []

    for line, name, opcode, addrs in constants():
        handler = handler_for(mz, opcode)
        if handler is None:
            norange += 1
            continue
        if not addrs:
            uncited += 1
            continue
        checked += 1
        if handler not in addrs:
            wrong += 1
            cited = ",".join(f"{a:#07x}" for a in addrs[:5])
            problems.append(
                f"src/vm.rs:{line}: {name} = {opcode:#04x} dispatches to "
                f"{handler:#07x}, but the doc cites {cited}"
            )

    for p in problems:
        print("HANDLER " + p)
    print(
        f"{checked} opcode constant(s) resolved through the dispatch table, "
        f"{wrong} citing a handler the table does not dispatch; "
        f"{uncited} cite no address, {norange} are outside 0xA0..0xD3"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
