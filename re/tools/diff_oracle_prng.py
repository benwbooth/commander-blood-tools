"""Differential oracle PoC: run the DOS PRNG (far 0x1CE:0x0B02) in Unicorn, capture return
value + memory side effects, and compare against a port of the Rust BloodPrng logic."""
from unicorn import *
from unicorn.x86_const import *
import struct, random

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
# whole file mapped at physical 0 => file offset == physical addr.
# relative segment S, offset O -> physical 0x600 + S*16 + O  (0x600 = 60h-para header).
CS = 0x1CE + 0x60          # so CS*16 + IP addresses into the loaded file
IP = 0xB02
# state byte file offsets (cs:0xaee.. = 0x600 + 0x1CE*16 + off)
BASE = 0x600 + 0x1CE * 16
SEED, A, B, CNT = BASE + 0xaee, BASE + 0xaf0, BASE + 0xaf1, BASE + 0xaf2
RET_CS, RET_IP = 0x0020, 0x0000
SENTINEL = RET_CS*16 + RET_IP   # far return target (linear 0x200); stop here

def dos_prng(seed_word, a, b, counter, modulus):
    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    MEM = 0x300000
    mu.mem_map(0, MEM)
    mu.mem_write(0, EXE + b"\x00" * (0x120000 - len(EXE)))
    # state
    mu.mem_write(SEED, struct.pack("<H", seed_word))
    mu.mem_write(A, bytes([a])); mu.mem_write(B, bytes([b])); mu.mem_write(CNT, bytes([counter]))
    # stack + fake return addr
    ss, sp = 0x9000, 0xFFF0
    mu.reg_write(UC_X86_REG_SS, ss)
    mu.reg_write(UC_X86_REG_SP, sp - 4)
    mu.mem_write(ss * 16 + sp - 4, struct.pack("<HH", RET_IP, RET_CS))  # retf pops IP then CS
    mu.reg_write(UC_X86_REG_CS, CS)
    mu.reg_write(UC_X86_REG_AX, modulus)
    writes = []
    def on_write(u, access, addr, size, value, ud):
        if addr < 0x120000:  # ignore stack
            writes.append((addr, size, value))
    mu.hook_add(UC_HOOK_MEM_WRITE, on_write)
    mu.emu_start(CS * 16 + IP, SENTINEL, count=2000)
    out_ax = mu.reg_read(UC_X86_REG_AX)
    new = (struct.unpack("<H", mu.mem_read(SEED, 2))[0],
           mu.mem_read(A, 1)[0], mu.mem_read(B, 1)[0], mu.mem_read(CNT, 1)[0])
    return out_ax, new, writes

def rust_prng(seed_word, a, b, counter, modulus):
    bl, bh, ax, carry = a, b, 0, 0
    for _ in range(8):
        nc = bl & 1; bl = (((carry & 0xff) << 7) | (bl >> 1)) & 0xff; carry = nc
        nc = ax >> 15; ax = ((ax << 1) | carry) & 0xffff; carry = nc
        nc = bh >> 7; bh = ((bh << 1) | (carry & 0xff)) & 0xff; carry = nc
        nc = ax >> 15; ax = ((ax << 1) | carry) & 0xffff; carry = nc
    ax ^= seed_word
    counter = (counter + 1) & 0xff
    b2 = (b - counter) & 0xff
    a2 = (a ^ (((counter << 1) | (counter >> 7)) & 0xff)) & 0xff
    if modulus != 0:
        while ax >= modulus: ax = (ax - modulus) & 0xffff
    return ax, (seed_word, a2, b2, counter)

random.seed(1)
mism = 0
for i in range(200):
    s = random.randint(0, 0xFFFF); a = random.randint(0, 0xFF); b = random.randint(0, 0xFF)
    c = random.randint(0, 0xFF); m = random.randint(1, 0xFFFF)
    try:
        dos_ax, dos_state, _ = dos_prng(s, a, b, c, m)
    except UcError as e:
        print(f"[{i}] Unicorn error: {e}"); break
    r_ax, r_state = rust_prng(s, a, b, c, m)
    if dos_ax != r_ax or dos_state != r_state:
        mism += 1
        if mism <= 5:
            print(f"MISMATCH in=({s:#06x},{a:#04x},{b:#04x},{c:#04x},m={m:#06x})")
            print(f"   DOS  ax={dos_ax:#06x} state={dos_state}")
            print(f"   Rust ax={r_ax:#06x} state={r_state}")
print(f"200 fuzz cases: {200-mism} match, {mism} mismatch")
