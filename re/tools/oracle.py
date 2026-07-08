"""General per-function differential oracle for the path-B static recompilation.

Given a function SPEC (entry file offset, ret type, which registers/memory are inputs and which
outputs to capture), fuzz the REAL DOS function in Unicorn and emit (input->output) vectors that
the Rust lift in src/recomp is tested against. Whole EXE mapped at phys 0, so file offset F is
addressed by CS:IP with CS = (F-0x600)//16-ish; we just run from linear F with CS=0.
Requires: pip install unicorn.  Run with PYTHONSAFEPATH=1 (re/tools shadows stdlib `dis`)."""
from unicorn import *
from unicorn.x86_const import *
import struct, random, json, os

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
RET_CS, RET_IP = 0x0020, 0x0000
SENTINEL = RET_CS * 16 + RET_IP
REG = {"ax": UC_X86_REG_AX, "bx": UC_X86_REG_BX, "cx": UC_X86_REG_CX, "dx": UC_X86_REG_DX,
       "si": UC_X86_REG_SI, "di": UC_X86_REG_DI, "bp": UC_X86_REG_BP,
       "ds": UC_X86_REG_DS, "es": UC_X86_REG_ES, "fs": UC_X86_REG_FS, "gs": UC_X86_REG_GS,
       "eax": UC_X86_REG_EAX, "ebx": UC_X86_REG_EBX, "ecx": UC_X86_REG_ECX, "edx": UC_X86_REG_EDX}

def run(spec, inp):
    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    mu.mem_map(0, 0x300000)
    mu.mem_write(0, EXE + b"\x00" * (0x120000 - len(EXE)))
    # inputs: registers
    for r, v in inp["regs"].items():
        mu.reg_write(REG[r], v)
    # inputs: memory (seg,off,bytes) -- seg is a fixed segment value from spec
    for (seg, off, data) in inp["mem"]:
        mu.mem_write(seg * 16 + off, data)
    ss, sp = 0x9000, 0xFFF0
    mu.reg_write(UC_X86_REG_SS, ss)
    depth = 4 if spec["retf"] else 2
    mu.reg_write(UC_X86_REG_SP, sp - depth)
    if spec["retf"]:
        mu.mem_write(ss * 16 + sp - 4, struct.pack("<HH", RET_IP, RET_CS))
    else:
        mu.mem_write(ss * 16 + sp - 2, struct.pack("<H", RET_IP))
    mu.reg_write(UC_X86_REG_CS, 0)  # run from linear = file offset
    entry = spec["entry"]
    if spec["retf"]:
        mu.emu_start(entry, SENTINEL, count=spec.get("count", 5000))
    else:
        mu.emu_start(entry, RET_IP, count=spec.get("count", 5000))
    fl = mu.reg_read(UC_X86_REG_EFLAGS)
    out = {"regs": {}, "mem": [],
           "flags": {"cf": bool(fl&1), "pf": bool(fl&4), "af": bool(fl&0x10),
                     "zf": bool(fl&0x40), "sf": bool(fl&0x80), "of": bool(fl&0x800)}}
    for r in spec["out_regs"]:
        out["regs"][r] = mu.reg_read(REG[r])
    for (seg, off, size) in spec["out_mem"]:
        out["mem"].append([seg, off, list(mu.mem_read(seg * 16 + off, size))])
    return out

# --- spec: func_a734  (add [0xD8C],ax ; add [0xD9A],ax ; clc ; ret) ---
DS = 0x2000
spec = dict(name="func_a734", entry=0xA734, retf=False,
            out_regs=["ax"], out_mem=[(DS, 0xD8C, 2), (DS, 0xD9A, 2)])
random.seed(20260707)
vecs = []
for _ in range(300):
    ax = random.randint(0, 0xFFFF)
    w1 = random.randint(0, 0xFFFF); w2 = random.randint(0, 0xFFFF)
    inp = dict(regs={"ax": ax, "ds": DS},
               mem=[(DS, 0xD8C, struct.pack("<H", w1)), (DS, 0xD9A, struct.pack("<H", w2))])
    o = run(spec, inp)
    vecs.append(dict(ax=ax, w1=w1, w2=w2,
                     ax_out=o["regs"]["ax"], flags=o["flags"],
                     w1_out=struct.unpack("<H", bytes(o["mem"][0][2]))[0],
                     w2_out=struct.unpack("<H", bytes(o["mem"][1][2]))[0]))
os.makedirs("re/tools/oracle_vectors", exist_ok=True)
json.dump(vecs, open("re/tools/oracle_vectors/func_a734.json", "w"))
print(f"wrote {len(vecs)} func_a734 oracle vectors")
