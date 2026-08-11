# Bit-Exact C/C++ Decompilation Restart

This branch starts a new track: identify the original toolchain, decompile
`BLOODPRG.EXE` to C/C++ in a form that can reproduce the shipped binary, then
use that as the source for a later Rust port.

## Artifact Identity

Canonical executable:

- `re/bin/BLOODPRG.EXE`
- size: `86680` bytes
- SHA-256:
  `7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823`

The local copies in `re/bin/`, `output/_tmp_iso/`, and
`commander-blood-audio/_tmp_iso/` hash identically.

## Executable Format

`BLOODPRG.EXE` is a plain DOS MZ executable, not a flat protected-mode DOS
extender image.

Header facts from `re/tools/mzfile.py`:

- header size: `0x600`
- load module: `0x600..0x15298`
- load size: `85144` bytes
- relocation count: `367`
- entry: `0x0000:0x0000` / file `0x600`
- stack: `SS:SP = 0x0ce2:0x7e78`
- trailing bytes: `0`

The entry code immediately does custom-looking process setup:

- `mov ax,0x0ce2; mov ds,ax`
- `cli; mov ss,ax; mov sp,0x7e78; sti`
- `mov gs,ds`
- `mov fs,0x0bbf`
- zeroes `edi/esi/ebp/ebx`
- resizes/allocates DOS memory through `int 21h`
- probes mouse, video mode, CD-ROM, EMS/XMS-style services, and starts the game

This is not the normal visible startup shape of a stock C runtime.

## Compiler Status

Current answer: **unknown for `BLOODPRG.EXE`**.

Positive/negative evidence so far:

- `file` identifies only `MS-DOS executable, MZ for MS-DOS`.
- Detect It Easy reports `BLOODPRG.EXE` as `MSDOS / Unknown`.
- No embedded strings were found for Borland/Turbo, Watcom, Microsoft C,
  QuickBASIC, BRUN, BASCOM, VBDOS, DOS4G, CauseWay, Phar Lap, DJGPP, or GO32.
- There is no DOS extender signature and no overlay/trailer after the MZ image.
- The program uses 80386 real-mode instructions and `FS`/`GS`.
- The program is far-linked across 12 recovered segment bases with 365 far
  call/jump sites.
- Common `push bp; mov bp,sp` C prologue bytes are absent as a pattern
  (`55 8B EC` count: `0`), while there are many register-save prologues and
  custom far routines.

Important nearby clue: Detect It Easy identifies `output/_tmp_iso/INSTALL.EXE`
as `Borland TLINK(5.0)`. That proves the installer was Borland-linked, but it is
not strong enough to claim that `BLOODPRG.EXE` was built by the same compiler or
linker.

Working hypothesis:

1. The main game was built with a custom startup and heavily custom runtime.
2. It may still contain compiler-generated C/C++ code, but the compiler is not
   identifiable yet from stock signatures.
3. A bit-exact recompile is therefore unlikely to start by selecting a compiler
   from metadata alone. We need codegen fingerprinting against historical
   compiler candidates.

## BASIC / VM Status

Current answer: the game uses a **custom compiled-BASIC-like script VM**, not a
recognized off-the-shelf QuickBASIC/BRUN interpreter.

Evidence:

- The CD ships `SCRIPT1..5.BAS`, `SCRIPT1..5.COD`, `SCRIPT1..5.VAR`,
  `SCRIPT1..5.DIC`, and `SCRIPT1..5.DEB`.
- The shipped `.BAS` files are binary token streams, not text BASIC source.
- `SCRIPT*.DIC` contains NUL-separated dictionary words.
- `SCRIPT*.DEB` contains fixed 20-byte symbol records.
- `SCRIPT*.VAR` contains object/runtime state records.
- `BLOODPRG.EXE` contains a VM opcode descriptor table at file `0x14338`
  (`DS:0x6F18`) and a handler table at file `0x142D0`.
- VM opcodes are biased by `0xA0`; real handlers cover `0xA0..0xD3`.
- `re/tools/vm_dispatch.py` reports 52 opcodes mapping to 37 distinct handlers.
- Most handlers are language primitives: branch, guard, assignment, arithmetic,
  record/object state, text line setup, and profile handoff. They are not
  QuickBASIC runtime calls.
- No `BRUN`, `BASCOM`, `QuickBASIC`, `QBX`, or VBDOS runtime files or strings are
  present in the CD root or executable.

The safest wording is "compiled BASIC-like game script VM". "BASIC" is useful
for human readability because the control flow and listings decompile naturally
to BASIC-ish blocks, guards, `SAY`, and `POKE`, but the interpreter itself is
game-specific.

## Next Compiler-Identification Work

Do not guess the compiler from date or installer metadata. The next pass should:

1. Build a small compiler-fingerprint corpus for candidate 1993-1995 DOS
   compilers: Borland C++/Turbo C++, Microsoft C/C++ 7.x/8.x, Watcom C/C++,
   Turbo Pascal/Borland Pascal only if codegen suggests it.
2. Compile tiny large-model real-mode programs with custom CRT disabled/minimal
   when possible.
3. Compare:
   - MZ header layout and relocation ordering
   - segment ordering
   - far call and far return conventions
   - prologue/epilogue shapes
   - 32-bit register use in 16-bit code
   - switch/jump-table lowering
   - division, shifts, `memcpy`/string-op idioms
   - stack cleanup convention (`ret`, `ret imm`, `retf`, `retf imm`)
4. Separately fingerprint `INSTALL.EXE` as a known Borland TLINK baseline, but
   do not transfer that conclusion to `BLOODPRG.EXE` without matching codegen.

## Practical Implication

The immediate decompilation path should be:

1. Treat `BLOODPRG.EXE` as the source of truth.
2. Recover function boundaries and call graph from the MZ/far-call structure.
3. Produce C-like per-function lifts whose emitted assembly is verified against
   the original function behavior first.
4. Only after enough codegen fingerprints are known, decide whether exact binary
   reproduction through a historical C/C++ compiler is realistic.

If no compiler match is found, the fallback bit-exact path is static
recompilation/lifting with an assembler/linker-controlled output, not idiomatic C.
