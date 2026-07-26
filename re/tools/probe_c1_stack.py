#!/usr/bin/env python3
"""Is the 0xC1 handler's stack balanced on the exits that jump straight to 0x6C7C?

audit-fixes #295 could not settle this by reading. The handler pushes `di`
(0x6B4C), then `ds` and `si` (0x6B71/0x6B72). Two of its exits pop all three:

    0x6C73  pop si / pop ds / call vm_branch / -> 0x6C7C
    0x6C7A  pop si / pop ds              -> 0x6C7C
    0x6C7C  pop di / ret

But three sites jump DIRECTLY to 0x6C7C, skipping the `pop si / pop ds`:
`0x6BC2` and `0x6BCB` on the query path, and the scan's sentinel at `0x6C20`.
Statically that reads as leaving two words on the stack — yet `0x6BC2` is the
ordinary "matched, not inverted" query outcome, which runs constantly, so the
game would break immediately. One of those two readings is wrong.

This settles it by EXECUTION rather than argument: run the handler in Unicorn
from its real entry, single-step, and record SP at entry, at every exit site,
and at the `ret`. A balanced routine returns with SP back at its entry value.

Not a behaviour oracle — it decides a question about the DECODE (does the
instruction sequence do what I read it as doing), which is exactly the kind of
thing the assembly is the authority on and my reading of it is not.

Run with PYTHONSAFEPATH=1 from the repo root.
"""

import sys

from unicorn import *
from unicorn.x86_const import *

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()

ENTRY = 0x6B4C
# The addresses this question is about.
WATCH = {
    0x6B4C: "entry (push di)",
    0x6B71: "push ds",
    0x6B72: "push si",
    0x6BC2: "query no-branch -> jmp 0x6C7C",
    0x6BCB: "query no-branch -> jmp 0x6C7C",
    0x6C20: "scan sentinel -> je 0x6C7C",
    0x6C73: "branch exit (pop si/ds, vm_branch)",
    0x6C7A: "write exit (pop si/ds)",
    0x6C7C: "pop di",
    0x6C7D: "ret",
}

STACK_SEG, STACK_TOP = 0x9000, 0xFF00
RET_MARKER = 0x00200000  # an address we can detect as "returned"


def probe(label, setup, limit=4000):
    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    mu.mem_map(0, 0x300000)
    mu.mem_write(0, EXE + b"\x00" * (0x140000 - len(EXE)))

    mu.reg_write(UC_X86_REG_SS, STACK_SEG)
    mu.reg_write(UC_X86_REG_SP, STACK_TOP)
    # Return address the handler will `ret` to; we watch for it.
    mu.mem_write(STACK_SEG * 16 + STACK_TOP, (0x2000).to_bytes(2, "little"))

    mu.reg_write(UC_X86_REG_CS, 0)
    setup(mu)

    entry_sp = mu.reg_read(UC_X86_REG_SP)
    log = []

    def hook(uc, address, size, _user):
        sp = uc.reg_read(UC_X86_REG_SP)
        if address in WATCH:
            log.append((address, sp))
        if address == 0x2000:  # returned
            log.append(("RET-TARGET", sp))
            uc.emu_stop()

    mu.hook_add(UC_HOOK_CODE, hook)
    try:
        mu.emu_start(ENTRY, 0x2000, count=limit)
    except UcError as exc:
        log.append((f"FAULT {exc}", mu.reg_read(UC_X86_REG_SP)))

    print(f"\n=== {label} ===")
    print(f"SP at entry: {entry_sp:#06x}")
    seen = set()
    for where, sp in log:
        if isinstance(where, int):
            if (where, sp) in seen:
                continue
            seen.add((where, sp))
            delta = sp - entry_sp
            print(f"  {where:#07x} SP={sp:#06x} ({delta:+d} vs entry)  {WATCH[where]}")
        else:
            delta = sp - entry_sp
            print(f"  {where} SP={sp:#06x} ({delta:+d} vs entry)")
    if not log:
        print("  (no watched address reached)")
    return log


def main():
    # Drive the handler down its QUERY path to 0x6BC2 -- the ordinary outcome.
    # gs:[0x67AD] bit 0 set selects query (`test byte gs:[0x67ad],1 / je` @0x6B73).
    def query(mu):
        gs = 0x0CE2  # the startup data segment (DS base file 0xD420)
        mu.reg_write(UC_X86_REG_GS, gs)
        mu.reg_write(UC_X86_REG_DS, gs)
        mu.reg_write(UC_X86_REG_ES, gs)
        mu.mem_write(gs * 16 + 0x67AD, b"\x01")
        # A COD stream for the handler to lodsw from: operand words, no 0xA1.
        # DS-RELATIVE -- `lodsw` reads DS:si, and DS is gs here. Writing this at
        # linear 0x4000 made the handler read EXE bytes instead, which is why the
        # first run of this probe never reached the exit it was written for.
        mu.reg_write(UC_X86_REG_SI, 0x4000)
        mu.mem_write(gs * 16 + 0x4000, bytes([0x10, 0x00, 0x20, 0x00, 0x30, 0x00]))

    probe("query path", query)

    def setpath(mu):
        gs = 0x0CE2
        mu.reg_write(UC_X86_REG_GS, gs)
        mu.reg_write(UC_X86_REG_DS, gs)
        mu.reg_write(UC_X86_REG_ES, gs)
        mu.mem_write(gs * 16 + 0x67AD, b"\x00")  # SET path
        mu.reg_write(UC_X86_REG_SI, 0x4000)
        mu.mem_write(gs * 16 + 0x4000, bytes([0x10, 0x00, 0x20, 0x00, 0x30, 0x00]))

    probe("set path", setpath)

    # The case the question is actually about: drive the QUERY path to 0x6BC2,
    # the "record matched, not inverted" outcome that jumps STRAIGHT to 0x6C7C.
    #
    #   0x6BB0  cmp cx,0xc1      cx = es:[bp], bp = the first operand
    #   0x6BB6  cmp ax,es:[bp+2] ax = the second operand
    #   0x6BBC  or dl,dl / jne   dl = the 0xA1-inversion flag, 0 here
    def query_match(mu):
        gs = 0x0CE2
        mu.reg_write(UC_X86_REG_GS, gs)
        mu.reg_write(UC_X86_REG_DS, gs)
        mu.reg_write(UC_X86_REG_ES, gs)
        mu.mem_write(gs * 16 + 0x67AD, b"\x01")  # query
        # ES comes from `les di,gs:[0x6724]` @0x6B4D -- the record-table far
        # pointer. Leaving it zero made `es:[bp]` read EXE bytes (cx=0xC700
        # instead of 0xC1), which is why the first attempts never reached the
        # match path. Point the table at this segment.
        mu.mem_write(gs * 16 + 0x6724, (0x0000).to_bytes(2, "little") + gs.to_bytes(2, "little"))
        # A record at 0x1000 typed 0xC1 whose +2 equals the second operand.
        mu.mem_write(gs * 16 + 0x1000, (0x00C1).to_bytes(2, "little"))
        mu.mem_write(gs * 16 + 0x1002, (0x1234).to_bytes(2, "little"))
        # gs:0x672c is a far pointer the lookup helper loads; aim it at zeros.
        mu.mem_write(gs * 16 + 0x672C, (0x0000).to_bytes(2, "little") + (0x3000).to_bytes(2, "little"))
        # 0x6034 scans `while ax > [si+0x10]` stepping 0x14. With a zeroed table
        # that never terminates, so give the FIRST entry a threshold above ax
        # (0x1000) and the helper returns immediately.
        mu.mem_write(0x3000 * 16 + 0x10, (0x2000).to_bytes(2, "little"))
        mu.reg_write(UC_X86_REG_SI, 0x4000)
        # operand1 = 0x1000 (the record), operand2 = 0x1234 (matches +2)
        mu.mem_write(gs * 16 + 0x4000, bytes([0x00, 0x10, 0x34, 0x12, 0x00, 0x00]))

    probe("query path forced to the 0x6BC2 no-branch exit", query_match)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
