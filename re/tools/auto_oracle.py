#!/usr/bin/env python3
"""Generic oracle: fuzz ALL registers, run a linear DOS function in Unicorn, discover the memory
it reads (inputs) and writes (side effects) via hooks, and dump generic vectors. No per-function
spec. Pairs with lift.py (auto Rust) + a generic Rust verifier. Skips functions touching the
code/EXE region (<0x10000) to avoid the CS-segment ambiguity (those stay hand-lifted).
Requires unicorn; run with PYTHONSAFEPATH=1."""
from unicorn import *
from unicorn.x86_const import *
import struct, random, json, sys

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
GP = [("ax", UC_X86_REG_AX), ("bx", UC_X86_REG_BX), ("cx", UC_X86_REG_CX), ("dx", UC_X86_REG_DX),
      ("si", UC_X86_REG_SI), ("di", UC_X86_REG_DI), ("bp", UC_X86_REG_BP)]
OUT = [("eax", UC_X86_REG_EAX), ("ebx", UC_X86_REG_EBX), ("ecx", UC_X86_REG_ECX),
       ("edx", UC_X86_REG_EDX), ("esi", UC_X86_REG_ESI), ("edi", UC_X86_REG_EDI),
       ("ebp", UC_X86_REG_EBP)]
# fixed segment values (all high, above the EXE image, so data addrs are unambiguous)
SEGS = {"ds": 0x2000, "es": 0x2200, "fs": 0x2400, "gs": 0x2600, "ss": 0x9000}
RET_CS, RET_IP = 0x0020, 0x0000
SENT = RET_CS * 16 + RET_IP

def gen(entry, retf, n=250):
    vecs = []
    tries = 0
    while len(vecs) < n and tries < n * 6:
        tries += 1
        mu = Uc(UC_ARCH_X86, UC_MODE_16)
        mu.mem_map(0, 0x300000)
        mu.mem_write(0, EXE + b"\x00" * (0x120000 - len(EXE)))
        # randomize the data segments' low 64K so reads get varied values
        seed_bytes = {}
        for sname, sval in SEGS.items():
            data = bytes(random.randint(0, 255) for _ in range(0x1000))
            base = sval * 16
            mu.mem_write(base, data)
            seed_bytes[base] = data
        regs_in = {r: random.randint(0, 0xFFFF) for r, _ in GP}
        for r, uc in GP:
            mu.reg_write(uc, regs_in[r])
        for s, v in SEGS.items():
            mu.reg_write({"ds": UC_X86_REG_DS, "es": UC_X86_REG_ES, "fs": UC_X86_REG_FS,
                          "gs": UC_X86_REG_GS, "ss": UC_X86_REG_SS}[s], v)
        sp = 0xFFF0
        mu.reg_write(UC_X86_REG_SP, sp - (4 if retf else 2))
        if retf:
            mu.mem_write(SEGS["ss"] * 16 + sp - 4, struct.pack("<HH", RET_IP, RET_CS))
        else:
            mu.mem_write(SEGS["ss"] * 16 + sp - 2, struct.pack("<H", RET_IP))
        mu.reg_write(UC_X86_REG_CS, 0)
        reads, writes, bad = {}, {}, [False]
        def onread(u, acc, addr, size, val, ud):
            if addr < 0x10000: bad[0] = True   # code/EXE-region access -> skip (CS ambiguity)
            for k in range(size):
                a = addr + k
                if a not in reads and a not in writes:
                    reads[a] = u.mem_read(a, 1)[0]
        def onwrite(u, acc, addr, size, val, ud):
            if addr < 0x10000: bad[0] = True
            for k in range(size):
                writes[addr + k] = (val >> (8 * k)) & 0xFF
        mu.hook_add(UC_HOOK_MEM_READ, onread)
        mu.hook_add(UC_HOOK_MEM_WRITE, onwrite)
        try:
            mu.emu_start(entry, SENT if retf else RET_IP, count=5000)
        except UcError:
            continue
        if bad[0]:
            continue
        fl = mu.reg_read(UC_X86_REG_EFLAGS)
        vecs.append(dict(
            regs_in=regs_in, segs=SEGS,
            mem_in=[[a, b] for a, b in reads.items() if b != 0],
            regs_out={r: mu.reg_read(uc) for r, uc in OUT},
            mem_writes=[[a, b] for a, b in writes.items()],
            flags={"cf": bool(fl & 1), "pf": bool(fl & 4), "af": bool(fl & 0x10),
                   "zf": bool(fl & 0x40), "sf": bool(fl & 0x80), "of": bool(fl & 0x800)}))
    return vecs

if __name__ == "__main__":
    entry = int(sys.argv[1], 16); retf = sys.argv[2] == "retf"; name = sys.argv[3]
    v = gen(entry, retf)
    json.dump(v, open(f"re/tools/oracle_vectors/{name}_generic.json", "w"))
    print(f"wrote {len(v)} generic vectors for {name}")
