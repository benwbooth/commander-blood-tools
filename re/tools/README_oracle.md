# Per-function differential oracle (Unicorn) — proof of concept

Run a single DOS function from BLOODPRG.EXE in isolation on a scriptable 8086 core
(Unicorn Engine), capture its **return value + memory side effects + I/O**, and diff
against the Rust port. This is the gold-standard per-function verification.

## Status (PoC, 2026-07)
`diff_oracle_prng.py` verifies the game PRNG (far 0x1CE:0x0B02, file 0x2DE2) against the
Rust `BloodPrng`: **200/200 fuzz cases match bit-exact** on the return value AND all 4
state bytes (cs:0xAEE seed_word, 0xAF0 a, 0xAF1 b, 0xAF2 counter). Proves the harness +
that one function's port are bit-exact.

## How the harness works (template for any function)
- Load the whole EXE at physical 0; a relative segment S/offset O addresses file byte
  0x600 + S*16 + O, so set CS = S + 0x60 (the 60h-para header) and IP = O.
- Write the function's input registers (AX...) and any input MEMORY state.
- Push a far return sentinel (CS:IP = 0x20:0) and `emu_start(cs*16+ip, until=0x200)`.
- `UC_HOOK_MEM_WRITE` captures memory side effects; a port-I/O hook (UC_HOOK_INSN /
  IN/OUT) captures audio; writes filtered to A000: capture video; an interrupt hook
  captures int 21h/10h/... Read output registers + changed memory after the `ret(f)`.
- Compare all of it against the Rust counterpart. Fuzz the input domain; a clean sweep
  = that function is verified bit-exact vs the binary.

## Requires
`pip install unicorn` (not yet in the nix flake — add it there to make this a permanent
test, or run in a venv as the PoC does).

## IMPORTANT scope note (why per-function != whole-program)
This verifies LEAF functions. It does NOT prove the whole port runs correctly, because the
Rust port is an idiomatic REIMPLEMENTATION, not a 1-to-1 composition of ported DOS
functions on a shared memory image. A perfect (bit-exact) port needs a 1-to-1 static
RECOMPILATION (every function lifted, same call graph, shared 1MB image) - at which point
per-function fuzzing verifies each lift. Whole-program tracing in Unicorn additionally
needs a mini-DOS (stubs for int 21h/10h/16h/1Ah/2Fh/67h + VGA/PIT ports) since Unicorn is
a bare CPU. See re/PROGRESS.md.

## Whole-program tracing PoC (minidos_tracer.py) — 2026-07
A mini-DOS on Unicorn that MZ-loads BLOODPRG.EXE (367 relocs applied), sets up a PSP+stack,
boots from the real entry point, and stubs int 21h (file I/O against the real game files,
memory alloc, vectors, version), int 2Fh/67h (XMS/EMS absent), int 33h (mouse absent),
int 16h/1Ah. RESULT: the binary boots and executes its REAL startup (int 21h mem-alloc +
vector hook -> int 10h SET MODE 13h -> int 2Fh XMS/CD detect), matching the statically
decoded boot. It then STALLS in a vsync/tick wait loop at file 0xB5A:
  test gs:[0xB35],3; in al,dx; and al,8; xor al,ah; je (count vsync into [0xB12])
which exits only when [0xB35] is cleared by the game's TIMER ISR (int 08h @ cs:0x213). The
emulator never fires that ISR, so time never advances and it spins.
LESSON (reinforces the strategy note): whole-program execution needs PIT-timer emulation
(periodically invoking the hooked int-08h ISR) + vsync + full port I/O - i.e. progressively
MORE of a PC emulator. Bit-exact whole-program parity converges toward "build DOSBox". The
per-function oracle (diff_oracle_prng.py) needs none of this and is the tractable verifier.

## Timer-injection attempt (confirms the emulator-convergence) — 2026-07
Extended the tracer to capture the game's installed int-08h ISR vector (int 21h AH=25 AL=08 ->
cs:0x213, correctly captured) and inject a timer interrupt periodically (push flags/cs/ip, jump
to the ISR). It did NOT get the binary past boot: correct injection needs iret-return detection,
re-entrancy guards, PIC EOI (out 0x20), and a real PIT tick model - and even then the game polls
vsync + PIT + keyboard timing that all need faithful emulation. Each fix is another piece of PC
hardware. CONCLUSION (now empirically ironclad, not just argued): running the binary faithfully =
building a PC emulator (timer/PIC/vsync/ports). DOSBox already IS that. So:
- WHOLE-PROGRAM bit-exact parity -> use/extend an emulator (DOSBox), not a from-scratch Rust run.
- PER-FUNCTION verification -> the Unicorn oracle (diff_oracle_prng.py) is the tractable, proven
  tool; needs no hardware. It's the right verifier for a 1-to-1 static recompilation OR for
  bug-finding in the current idiomatic port.
DECISION for the maintainer: (A) idiomatic engine + oracle-driven leaf verification + accept
asymptotic-not-100 fidelity; or (B) 1-to-1 static recompilation verified per-function by the
oracle (large, non-idiomatic, provably 100%); or (C) treat DOSBox as the shipping runtime and the
Rust as tooling/asset pipeline. There is no idiomatic-AND-provably-100% middle.
