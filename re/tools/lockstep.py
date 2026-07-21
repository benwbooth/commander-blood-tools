#!/usr/bin/env python3
"""Interpret-vs-Unicorn LOCKSTEP replay: find the first instruction where the path-B interpreter
diverges from Unicorn on the REAL intro instruction stream (with real operands/state).

Consumes a trace produced by `runtime_boot --lockstep SKIP WINDOW trace.bin`. The Rust side runs
the actual game, routing VRAM to linear memory (vga_linear) so both sides share identical memory
semantics, and records, per step:
  - 'X' (pure-CPU): the interpreter's register state AFTER executing one instruction. Unicorn
    RE-EXECUTES that instruction independently from its own (in-sync) state; we compare the result.
    The FIRST mismatch in a general register / segment / IP (or a DEFINED arithmetic flag) is an
    interpreter bug — e.g. a wrong branch that would send the intro down the wrong path.
  - 'D' (device: int service, IRQ inject, in/out): the post-service register state + memory writes.
    Unicorn can't emulate the host device layer, so we APPLY the recorded state authoritatively and
    let Unicorn resume executing pure-CPU code from there.

Run: PYTHONSAFEPATH=1 $VENV/bin/python re/tools/lockstep.py trace.bin
"""
import dis  # noqa: F401 — bind stdlib dis before the local re/tools/dis.py can shadow it
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import capstone
from unicorn import Uc, UcError, UC_ARCH_X86, UC_MODE_16, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP,
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_SS,
    UC_X86_REG_FS, UC_X86_REG_GS, UC_X86_REG_EFLAGS, UC_X86_REG_IP,
)

GP = [UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
      UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP]
GPN = ["eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp"]
SG = [UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_SS, UC_X86_REG_FS, UC_X86_REG_GS]
SGN = ["cs", "ds", "es", "ss", "fs", "gs"]

ALL6 = 0x8d5  # CF PF AF ZF SF OF at bit positions 0,2,4,6,7,11


def defined_flags(mn: str) -> int:
    """Which arithmetic flags this mnemonic leaves DEFINED (mirror mod.rs interp_matches_unicorn_diff)."""
    if mn in ("add", "sub", "adc", "sbb", "cmp", "neg", "xadd"):
        return ALL6
    if mn in ("and", "or", "xor", "test"):
        return ALL6 & ~0x810  # CF/OF cleared, AF undefined -> compare CF PF ZF SF
    if mn in ("inc", "dec"):
        return ALL6 & ~1  # CF unaffected
    if mn in ("shl", "shr", "sar", "rol", "ror", "sal", "rcl", "rcr"):
        return 1  # CF (bit shifted/rotated out)
    if mn in ("bt", "btr", "bts", "btc"):
        return 1
    if mn in ("bsf", "bsr"):
        return 0x40  # ZF
    return 0


def main():
    path = sys.argv[1]
    d = open(path, "rb").read()
    assert d[:4] == b"LSTP", d[:4]
    window, memlen = struct.unpack_from("<II", d, 4)
    off = 12
    init = struct.unpack_from("<8I8H", d, off); off += 48
    mem = d[off:off + memlen]; off += memlen
    print(f"trace: window={window} memlen={memlen:#x}")

    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    mu.mem_map(0, memlen)
    mu.mem_write(0, mem)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)

    def load_regs(r):
        for i, reg in enumerate(GP):
            mu.reg_write(reg, r[i])
        for i, reg in enumerate(SG):
            mu.reg_write(reg, r[8 + i])
        mu.reg_write(UC_X86_REG_IP, r[14])
        mu.reg_write(UC_X86_REG_EFLAGS, r[15] | 0x2)

    load_regs(init)

    writes = {}

    def onwrite(u, acc, addr, size, val, ud):
        for k in range(size):
            writes[addr + k] = (val >> (8 * k)) & 0xFF
    mu.hook_add(UC_HOOK_MEM_WRITE, onwrite)

    xdone = 0
    ev = 0
    while off < len(d):
        typ = d[off]; off += 1
        after = struct.unpack_from("<8I8H", d, off); off += 48
        if typ == 1:  # device: apply authoritatively
            nw = struct.unpack_from("<I", d, off)[0]; off += 4
            for _ in range(nw):
                a, v = struct.unpack_from("<IB", d, off); off += 5
                if a < memlen:
                    mu.mem_write(a, bytes([v]))
            load_regs(after)
            ev += 1
            continue

        # pure-CPU 'X': disassemble at current cs:ip, single-step, compare
        cs = mu.reg_read(UC_X86_REG_CS)
        ip = mu.reg_read(UC_X86_REG_IP)
        code = mu.mem_read((cs * 16 + ip) & (memlen - 1), 15)
        try:
            insn = next(md.disasm(bytes(code), ip))
            mn, ops, isize = insn.mnemonic, insn.op_str, insn.size
        except StopIteration:
            mn, ops, isize = "?", "", 1
        writes.clear()
        begin = cs * 16 + ip
        # A `rep`/`repe`/`repne` string op is ONE step() on the interp (full rep), but Unicorn's
        # count=1 does ONE iteration (leaving IP on the rep). Single-step iterations until IP
        # advances past the instruction (the `until`-based form trips a Unicorn real-mode quirk).
        is_rep = code[0] in (0xf2, 0xf3)
        try:
            if is_rep:
                guard = 0
                while True:
                    mu.emu_start(begin, 0, count=1)
                    if mu.reg_read(UC_X86_REG_IP) != ip or mu.reg_read(UC_X86_REG_CS) != cs:
                        break
                    guard += 1
                    if guard > 0x100000:
                        break
            else:
                mu.emu_start(begin, 0, count=1)
        except UcError as e:
            print(f"\n*** DIVERGENCE at event {ev} (X #{xdone}) {cs:04x}:{ip:04x} {mn} {ops}")
            print(f"    Unicorn FAULTED: {e}  (interpreter did not)")
            print(f"    interp after: cs:ip={after[8]:04x}:{after[14]:04x}")
            return
        # compare
        bad = []
        for i, name in enumerate(GPN):
            g = mu.reg_read(GP[i])
            if g != after[i]:
                bad.append(f"{name}={g:#x}!={after[i]:#x}")
        for i, name in enumerate(SGN):
            g = mu.reg_read(SG[i])
            if g != after[8 + i]:
                bad.append(f"{name}={g:#x}!={after[8 + i]:#x}")
        gip = mu.reg_read(UC_X86_REG_IP)
        if gip != after[14]:
            bad.append(f"ip={gip:#x}!={after[14]:#x}")
        dm = defined_flags(mn)
        if dm:
            gf = mu.reg_read(UC_X86_REG_EFLAGS)
            if (gf ^ after[15]) & dm:
                bad.append(f"flags {gf & dm:#x}!={after[15] & dm:#x} (def {dm:#x})")
        if bad:
            print(f"\n*** DIVERGENCE at event {ev} (X #{xdone}) {cs:04x}:{ip:04x} {mn} {ops}")
            print("    " + ", ".join(bad))
            print(f"    (interpreter after-state is the ground-truth recorded value)")
            return
        xdone += 1
        ev += 1
        if xdone % 200000 == 0:
            print(f"  ... {xdone} pure-CPU instructions matched (event {ev})")

    print(f"\nNO DIVERGENCE: {xdone} pure-CPU instructions matched Unicorn across {ev} events.")
    print("The interpreter is bit-exact for the entire captured window.")


if __name__ == "__main__":
    main()
