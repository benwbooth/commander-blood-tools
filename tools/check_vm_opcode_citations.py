#!/usr/bin/env python3
"""Does each VM opcode arm cite the handler the DISPATCH TABLE actually names?

`VmState::step` documents one handler address per opcode -- `// 0xA0 PUSH
(0x6559)`, `// 0xA2 (0x6588)`, and so on for the whole 0xA0..0xD2 range. Those
addresses were written by hand, one at a time, across many sessions. The image
contains the ground truth: the dispatch table the interpreter indexes with the
opcode byte, already decoded by `re/tools/dump_handler_table.py`.

So this compares the two directly. Every mismatch is either a wrong citation in
the port or a wrong reading of the table, and both are worth knowing.

Three outcomes per cited opcode:

  MISMATCH   the port cites an address the table does not map that opcode to.
  UNDISPATCHED  the port cites a handler for an opcode the table has no entry
             for. Not automatically wrong -- the table covers 0xA0..0xD2 and the
             port's comments may describe a token that is parsed but never
             dispatched (audit-fixes distinguishes the TOKEN bound from the
             DISPATCH bound at OP_MAX) -- but it should be a deliberate claim.
  ok         the cited address is exactly the table's handler.

Opcodes the table maps but the port never cites are reported as a count only:
a missing comment is a documentation gap, not an incorrect claim, and this tool
is about claims that are WRONG.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import re
import subprocess
import sys

VM = "src/vm.rs"
TABLE_TOOL = "re/tools/dump_handler_table.py"

# `0xa0    0x11b9    0x006559    <label> ...`
TABLE_ROW = re.compile(r"^(0x[0-9a-f]{2})\s+0x[0-9a-f]+\s+(0x[0-9a-f]+)\s")
# A step-arm comment: `// 0xA0 PUSH (0x6559): ...` / `// 0xA2 (0x6588): ...`
CITE = re.compile(r"//\s*(0x[0-9A-Fa-f]{2})\b[^(\n]*\((0x[0-9A-Fa-f]{4})\)")


def handler_table():
    out = subprocess.run(
        [sys.executable, TABLE_TOOL], capture_output=True, text=True, timeout=300
    ).stdout
    table = {}
    for line in out.splitlines():
        m = TABLE_ROW.match(line)
        if m:
            table[int(m.group(1), 16)] = int(m.group(2), 16)
    return table


def main():
    table = handler_table()
    if not table:
        print("could not read the dispatch table; nothing checked")
        return 1

    text = open(VM, encoding="utf-8", errors="replace").read()
    lines = text.splitlines()

    mismatch, undispatched, ok = [], [], 0
    cited = set()
    for i, line in enumerate(lines):
        m = CITE.search(line)
        if not m:
            continue
        op = int(m.group(1), 16)
        addr = int(m.group(2), 16)
        # Only opcode-looking values; the file cites plenty of other addresses.
        if not 0xA0 <= op <= 0xFF:
            continue
        cited.add(op)
        if op not in table:
            undispatched.append((i + 1, op, addr))
        elif table[op] != addr:
            mismatch.append((i + 1, op, addr, table[op]))
        else:
            ok += 1

    for ln, op, addr, want in mismatch:
        print(f"MISMATCH     {VM}:{ln}: {op:#04x} cites {addr:#07x}, table says {want:#07x}")
    for ln, op, addr in undispatched:
        print(f"UNDISPATCHED {VM}:{ln}: {op:#04x} cites {addr:#07x}, not in the dispatch table")

    # GROUPING. The port writes shared handlers as grouped arms --
    # `0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC => { .. }`. That is a claim
    # about the BINARY: those opcodes are one handler. The table can settle it,
    # and it is a stronger check than any per-opcode citation, because a group
    # that is wrong means the port merges behaviours the game keeps apart (or
    # splits ones it shares).
    ARM = re.compile(r"^\s*(0x[A-F0-9]{2}(?:\s*\|\s*0x[A-F0-9]{2})+)\s*=>")
    split_groups, merged_groups, good_groups = [], [], 0
    for i, line in enumerate(lines):
        m = ARM.match(line)
        if not m:
            continue
        ops = [int(x.strip(), 16) for x in m.group(1).split("|")]
        if not all(o in table for o in ops):
            continue
        handlers = {table[o] for o in ops}
        if len(handlers) > 1:
            split_groups.append((i + 1, ops, sorted(handlers)))
            continue
        handler = handlers.pop()
        # Everything the table maps to this handler must be in the arm, too.
        expected = {o for o, h in table.items() if h == handler}
        if expected != set(ops):
            merged_groups.append((i + 1, sorted(ops), sorted(expected)))
        else:
            good_groups += 1

    # GROUP-SPLIT is ADVISORY and off by default. Every one of the five instances
    # present when this check was written turned out legitimate, so printing them
    # as findings would be exactly the confidently-wrong tool this project keeps
    # catching. They fall into three benign shapes:
    #   * the arm is not a DISPATCH arm at all -- `0xCE | 0xD0 | 0xD1 => pc += 1`
    #     inside a script SCANNER groups by operand LENGTH, and all three really
    #     are one-byte opcodes;
    #   * the handlers are distinct addresses with identical bodies (0xAA/0xAC
    #     both just set the yield flag gs:[0x67b4]);
    #   * the arm groups for structure but discriminates inside, via `match op`
    #     (0xC5..0xC8, which have genuinely different per-opcode write guards).
    # GROUP-MEMBER below is the bucket that carries signal: an arm whose opcode
    # set does not equal the handler's opcode set is wrong either way.
    if "--splits" in sys.argv:
        for ln, ops, handlers in split_groups:
            names = " | ".join(f"{o:#04x}" for o in ops)
            hs = " ".join(f"{h:#07x}" for h in handlers)
            print(f"GROUP-SPLIT  {VM}:{ln}: {names} share an arm but {len(handlers)} handlers: {hs}")
    for ln, ops, expected in merged_groups:
        got = " ".join(f"{o:#04x}" for o in ops)
        want = " ".join(f"{o:#04x}" for o in expected)
        print(f"GROUP-MEMBER {VM}:{ln}: arm has [{got}], handler is shared by [{want}]")
    print(
        f"{good_groups} grouped arm(s) exactly match a shared handler's opcode set, "
        f"{len(split_groups)} span several handlers (advisory, --splits to list), "
        f"{len(merged_groups)} have the wrong members"
    )

    uncited = sorted(set(table) - cited)
    print(
        f"{ok} opcode citation(s) match the dispatch table, "
        f"{len(mismatch)} MISMATCH, {len(undispatched)} undispatched; "
        f"{len(uncited)} dispatched opcode(s) carry no citation "
        f"({' '.join(f'{o:#04x}' for o in uncited) if uncited else 'none'})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
