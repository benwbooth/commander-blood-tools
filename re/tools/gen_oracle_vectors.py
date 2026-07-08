"""Generate oracle vectors for a lifted function by running the REAL DOS function in Unicorn
over fuzzed inputs and dumping (input-state -> output-state) as JSON. The Rust test in
src/recomp replays these and asserts the lift is bit-exact. Requires: pip install unicorn.

Currently emits prng_2de2 vectors (game PRNG, far 0x1CE:0x0B02, file 0x2DE2)."""
from unicorn import *
from unicorn.x86_const import *
import struct, random, json, os, sys

EXE = open("re/bin/BLOODPRG.EXE", "rb").read()
CS = 0x1CE + 0x60          # whole file mapped at phys 0: CS*16+IP addresses into it
IP = 0xB02
BASE = 0x600 + 0x1CE * 16
SEED, A, B, CNT = BASE + 0xAEE, BASE + 0xAF0, BASE + 0xAF1, BASE + 0xAF2
RET_CS, RET_IP = 0x0020, 0x0000
SENTINEL = RET_CS * 16 + RET_IP

def run(seed, a, b, counter, modulus):
    mu = Uc(UC_ARCH_X86, UC_MODE_16)
    mu.mem_map(0, 0x300000)
    mu.mem_write(0, EXE + b"\x00" * (0x120000 - len(EXE)))
    mu.mem_write(SEED, struct.pack("<H", seed))
    mu.mem_write(A, bytes([a])); mu.mem_write(B, bytes([b])); mu.mem_write(CNT, bytes([counter]))
    ss, sp = 0x9000, 0xFFF0
    mu.reg_write(UC_X86_REG_SS, ss); mu.reg_write(UC_X86_REG_SP, sp - 4)
    mu.mem_write(ss * 16 + sp - 4, struct.pack("<HH", RET_IP, RET_CS))
    mu.reg_write(UC_X86_REG_CS, CS); mu.reg_write(UC_X86_REG_AX, modulus)
    mu.emu_start(CS * 16 + IP, SENTINEL, count=2000)
    return (mu.reg_read(UC_X86_REG_AX),
            struct.unpack("<H", mu.mem_read(SEED, 2))[0],
            mu.mem_read(A, 1)[0], mu.mem_read(B, 1)[0], mu.mem_read(CNT, 1)[0])

def main():
    random.seed(20260707)
    out = []
    for _ in range(int(sys.argv[1]) if len(sys.argv) > 1 else 300):
        s = random.randint(0, 0xFFFF); a = random.randint(0, 0xFF); b = random.randint(0, 0xFF)
        c = random.randint(0, 0xFF); m = random.randint(0, 0xFFFF)  # include modulus 0
        ax_out, seed_out, a_out, b_out, cnt_out = run(s, a, b, c, m)
        # this function must not modify the seed word; assert as a sanity check
        assert seed_out == s, "PRNG unexpectedly modified seed_word"
        out.append(dict(cs=CS, ax_in=m, seed=s, a=a, b=b, counter=c,
                        ax_out=ax_out, a_out=a_out, b_out=b_out, counter_out=cnt_out))
    os.makedirs("re/tools/oracle_vectors", exist_ok=True)
    json.dump(out, open("re/tools/oracle_vectors/prng_2de2.json", "w"))
    print(f"wrote {len(out)} prng_2de2 oracle vectors")

main()
