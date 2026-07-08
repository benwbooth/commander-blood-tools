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

# --- spec: func_a744  (mov [0xD62],0 ; mov [0xD64],0xFFFF ; mov [0xD66],0xFFFF ; ret) ---
DS2 = 0x2000
spec2 = dict(name="func_a744", entry=0xA744, retf=False,
             out_regs=[], out_mem=[(DS2, 0xD62, 2), (DS2, 0xD64, 2), (DS2, 0xD66, 2)])
vecs2 = []
for _ in range(20):
    inp = dict(regs={"ds": DS2}, mem=[])
    o = run(spec2, inp)
    vecs2.append(dict(a=struct.unpack("<H", bytes(o["mem"][0][2]))[0],
                      b=struct.unpack("<H", bytes(o["mem"][1][2]))[0],
                      c=struct.unpack("<H", bytes(o["mem"][2][2]))[0]))
json.dump(vecs2, open("re/tools/oracle_vectors/func_a744.json", "w"))
print(f"wrote {len(vecs2)} func_a744 oracle vectors")

# --- spec: func_9f80  (bx=0x1FB5; add bx,ax x4 (=0x1FB5+4*ax); mov bx,[bx]; ret) ---
DS3 = 0x2000
spec3 = dict(name="func_9f80", entry=0x9F80, retf=False, out_regs=["bx"], out_mem=[])
vecs3 = []
for _ in range(300):
    ax = random.randint(0, 0xFFFF)
    bx = (0x1FB5 + 4 * ax) & 0xFFFF
    word = random.randint(0, 0xFFFF)
    inp = dict(regs={"ax": ax, "ds": DS3}, mem=[(DS3, bx, struct.pack("<H", word))])
    o = run(spec3, inp)
    vecs3.append(dict(ax=ax, word=word, bx_out=o["regs"]["bx"], flags=o["flags"]))
json.dump(vecs3, open("re/tools/oracle_vectors/func_9f80.json", "w"))
print(f"wrote {len(vecs3)} func_9f80 oracle vectors")

# --- spec: func_533c  (push bx; shl ax,3; mov bx,ax; mov eax,fs:[bx+4]; pop bx; retf) ---
# resource_get_field4: EAX = dword at fs:(ax*8 + 4). BX preserved. Flags from shl ax,3.
FS = 0x4000
spec4 = dict(name="func_533c", entry=0x533C, retf=True, out_regs=["eax", "bx"], out_mem=[])
vecs4 = []
random.seed(424242)
while len(vecs4) < 300:
    ax = random.randint(0, 0xFFFF)
    bx = random.randint(0, 0xFFFF)         # must be preserved
    shifted = (ax * 8) & 0xFFFF
    off = (shifted + 4) & 0xFFFF
    if off + 3 >= 0x10000:                  # skip 64K-boundary-crossing dword (offset-wrap edge)
        continue
    dword = random.randint(0, 0xFFFFFFFF)
    inp = dict(regs={"ax": ax, "bx": bx, "fs": FS},
               mem=[(FS, off, struct.pack("<I", dword))])
    o = run(spec4, inp)
    vecs4.append(dict(ax=ax, bx=bx, off=off, dword=dword,
                      eax_out=o["regs"]["eax"], bx_out=o["regs"]["bx"], flags=o["flags"]))
json.dump(vecs4, open("re/tools/oracle_vectors/func_533c.json", "w"))
print(f"wrote {len(vecs4)} func_533c oracle vectors")

# --- spec: func_a40b (cmp gs:[0xD5F],0; je; cmp gs:[0xD5F],1; ret) -- flags only ---
GS = 0x3000
spec5 = dict(name="func_a40b", entry=0xA40B, retf=False, out_regs=[], out_mem=[])
vecs5 = []
for byte in list(range(256)) + [random.randint(0, 255) for _ in range(44)]:
    o = run(spec5, dict(regs={"gs": GS}, mem=[(GS, 0xD5F, bytes([byte]))]))
    vecs5.append(dict(byte=byte, flags=o["flags"]))
json.dump(vecs5, open("re/tools/oracle_vectors/func_a40b.json", "w"))
print(f"wrote {len(vecs5)} func_a40b oracle vectors")

# --- spec: func_a634 (test byte [DS=GS:0xB17],1; ret) -- flags only ---
spec6 = dict(name="func_a634", entry=0xA634, retf=False, out_regs=[], out_mem=[])
vecs6 = []
for byte in list(range(256)):
    o = run(spec6, dict(regs={"gs": GS}, mem=[(GS, 0xB17, bytes([byte]))]))
    vecs6.append(dict(byte=byte, flags=o["flags"]))
json.dump(vecs6, open("re/tools/oracle_vectors/func_a634.json", "w"))
print(f"wrote {len(vecs6)} func_a634 oracle vectors")
