#!/usr/bin/env python3
"""Differential single-instruction fuzzer: the Rust interpreter vs Unicorn (M1b).

Harvests the game's OWN instruction inventory (linear sweep of BLOODPRG.EXE's code region +
optional extra code dumps like the SND driver), then executes each unique instruction under
N randomized register states in Unicorn with a deterministic position-dependent memory fill,
recording the full outcome. The Rust side (`recomp::tests::interp_matches_unicorn_diff`)
replays every vector through `interp::Cpu::step` and asserts registers, IP, and memory writes.

Determinism: memory byte at linear address A is `fill(A) = (A * 2654435761 >> 16) & 0xFF`,
regs come from a seeded PRNG — the Rust test reconstructs everything from the JSON.

Run: PYTHONSAFEPATH=1 $VENV/bin/python re/tools/diff_fuzz.py [extra_code.bin ...]
Writes re/tools/oracle_vectors/diff_fuzz.json
"""
import dis  # noqa: F401  — bind stdlib dis before re/tools (local dis.py) hits sys.path
import glob
import json
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

import capstone
from unicorn import Uc, UcError, UC_ARCH_X86, UC_MODE_16, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP,
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_SS,
    UC_X86_REG_FS, UC_X86_REG_GS, UC_X86_REG_EFLAGS, UC_X86_REG_IP,
)

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
CODE_END = 0xCA00  # known code region

GP = [("eax", UC_X86_REG_EAX), ("ebx", UC_X86_REG_EBX), ("ecx", UC_X86_REG_ECX),
      ("edx", UC_X86_REG_EDX), ("esi", UC_X86_REG_ESI), ("edi", UC_X86_REG_EDI),
      ("ebp", UC_X86_REG_EBP), ("esp", UC_X86_REG_ESP)]
SEGS = [("ds", UC_X86_REG_DS), ("es", UC_X86_REG_ES), ("ss", UC_X86_REG_SS),
        ("fs", UC_X86_REG_FS), ("gs", UC_X86_REG_GS)]

# Excluded: far transfers, interrupts, port I/O, halt, iret (host-boundary or needs frames),
# and x87 (int-emulated by the game).
SKIP_MN = {"lcall", "ljmp", "int", "int1", "int3", "into", "iret", "iretd", "hlt",
           "in", "out", "insb", "insw", "insd", "outsb", "outsw", "outsd",
           # x87 FPU is int-emulated by the game (Borland), never real opcodes at runtime;
           # `bound`/`arpl`-class are misaligned-decode junk from the linear sweep.
           "bound", "arpl", "wait", "fwait"}

def _skip(mn: str) -> bool:
    base = mn.split()[-1] if " " in mn else mn
    if mn in SKIP_MN or base in SKIP_MN:
        return True
    # any x87 mnemonic (f...), but not the flag ops (nop-free): fclex etc. all start 'f'
    if mn.startswith("f") and mn not in ("fs", "for"):
        return True
    return False

CS_BASE = 0x10000  # instruction placed at CS_BASE (CS = 0x1000, IP = 0)
MEM_TOP = 0x110000


def fill(a: int) -> int:
    return ((a * 2654435761) >> 16) & 0xFF


FILL_IMG = bytes(fill(a) for a in range(MEM_TOP))


def harvest():
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    seen = {}
    blobs = [EXE[0x600:CODE_END]]
    for extra in sys.argv[1:]:
        blobs.append(open(extra, "rb").read())
    for blob in blobs:
        # linear sweep from several offsets to catch misaligned starts
        for start in (0, 1, 2, 3):
            for i in md.disasm(blob[start:], 0):
                if _skip(i.mnemonic):
                    continue
                b = bytes(i.bytes)
                if b not in seen:
                    seen[b] = f"{i.mnemonic} {i.op_str}".strip()
    return seen


def run_one(code: bytes, rng: random.Random):
    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    mu.mem_map(0, MEM_TOP)
    mu.mem_write(0, FILL_IMG)
    mu.mem_write(CS_BASE, code)
    regs_in = {}
    for name, ucreg in GP:
        v = rng.getrandbits(32) if rng.random() < 0.3 else rng.getrandbits(16)
        if name == "esp":
            v = 0x8000 | (v & 0x7FF0)  # sane stack inside the segment
        regs_in[name] = v
        mu.reg_write(ucreg, v)
    segs_in = {}
    for name, ucreg in SEGS:
        v = rng.randrange(0x0000, 0xF000) & 0xFFFF
        segs_in[name] = v
        mu.reg_write(ucreg, v)
    mu.reg_write(UC_X86_REG_CS, CS_BASE >> 4)
    mu.reg_write(UC_X86_REG_IP, 0)
    flags_in = 0x0002 | (rng.getrandbits(1) << 0) | (rng.getrandbits(1) << 6) \
        | (rng.getrandbits(1) << 7) | (rng.getrandbits(1) << 10) | (rng.getrandbits(1) << 11) \
        | (rng.getrandbits(1) << 2) | (rng.getrandbits(1) << 4)
    mu.reg_write(UC_X86_REG_EFLAGS, flags_in)
    writes = {}

    def onwrite(u, acc, addr, size, val, ud):
        for k in range(size):
            writes[addr + k] = (val >> (8 * k)) & 0xFF

    mu.hook_add(UC_HOOK_MEM_WRITE, onwrite)
    try:
        mu.emu_start(CS_BASE, 0, count=1)
    except UcError:
        return None
    ip_out = mu.reg_read(UC_X86_REG_IP)
    if ip_out == 0:  # didn't advance (e.g. faulted quietly)
        return None
    fl = mu.reg_read(UC_X86_REG_EFLAGS)
    return dict(
        code=code.hex(),
        regs_in=regs_in, segs=segs_in, flags_in=flags_in,
        regs_out={n: mu.reg_read(r) for n, r in GP},
        segs_out={n: mu.reg_read(r) for n, r in SEGS},
        ip_out=ip_out,
        flags_out=fl,
        mem_writes=sorted((a, v) for a, v in writes.items()),
    )


def main():
    inv = harvest()
    print(f"instruction inventory: {len(inv)} unique encodings")
    rng = random.Random(0xB100D)
    vecs = []
    per = 3
    for code, txt in sorted(inv.items()):
        for _ in range(per):
            v = run_one(code, rng)
            if v is not None:
                v["asm"] = txt
                vecs.append(v)
    out = os.path.join("re", "tools", "oracle_vectors", "diff_fuzz.json")
    with open(out, "w") as f:
        json.dump(vecs, f)
    print(f"{len(vecs)} vectors -> {out}")


if __name__ == "__main__":
    main()
