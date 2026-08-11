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

## Fingerprint Harness

`re/tools/toolchain_fingerprint.py` emits the comparison profile needed for that
next step:

```sh
python3 re/tools/toolchain_fingerprint.py \
  re/bin/BLOODPRG.EXE output/_tmp_iso/INSTALL.EXE \
  --sample-limit 8
```

Initial output confirms the main-game/installer split:

| Feature | `BLOODPRG.EXE` | `INSTALL.EXE` |
|---|---:|---:|
| MZ header size | `0x600` | `0x200` |
| relocations | 367 | 24 |
| relocation site order | monotonic | 2 backtracks |
| recovered segment bases | 12 | 7 |
| relocated far call/jump sites | 365 | 0 |
| distinct relocated far targets | 107 | 0 |
| `55 8B EC` byte-pattern hits | 0 | 7 |
| `66` operand-size prefix hits | 1007 | 237 |
| `67` address-size prefix hits | 647 | 104 |
| `FS` prefix hits | 201 | 240 |
| `GS` prefix hits | 1248 | 633 |

The marker-string section is deliberately raw. For example, `INSTALL.EXE`
contains `Microsoft` only because it prints `Microsoft compatible Mouse`; that
is not a Microsoft compiler signal. Treat marker hits as leads to inspect, not
as classifications.

## Pascal Hypothesis Check

Checked the Turbo/Borland Pascal hypothesis directly against `BLOODPRG.EXE`.

Commands used:

```sh
nix shell nixpkgs#detect-it-easy -c diec \
  re/bin/BLOODPRG.EXE output/_tmp_iso/INSTALL.EXE

python3 re/tools/toolchain_fingerprint.py \
  re/bin/BLOODPRG.EXE output/_tmp_iso/INSTALL.EXE \
  --sample-limit 4

python3 re/tools/indirect_dispatch_atlas.py --sample-limit 4
```

Results:

- Detect It Easy still reports `BLOODPRG.EXE` as `MSDOS / Unknown`; only
  `INSTALL.EXE` reports `Borland TLINK(5.0)`.
- `BLOODPRG.EXE` has no `Borland`, `Turbo`, `Pascal`, `TPU`, `TPL`,
  `Runtime error`, `Run-time error`, or classic Pascal runtime-error text.
- Raw `RTE` byte hits in `BLOODPRG.EXE` are false positives inside resource names
  such as `bcarte.spr` / `pterra.ext`, not runtime-error strings.
- The 308-entry atlas (`function_atlas` plus static dispatch targets from
  `indirect_dispatch_atlas`) has **zero aligned `ret imm` or `retf imm`
  terminals**. Known entries terminate as plain `ret`, plain `retf`, or tail
  `jmp`. That weakens a conventional Pascal callee-cleanup calling-convention
  hypothesis.
- Startup remains custom: entry `0x600` immediately sets `DS/SS/SP`, sets
  `GS/FS`, uses 386 registers, resizes/allocates DOS memory, probes devices, and
  starts the game. It does not look like a stock Turbo/Borland Pascal startup.

Current Pascal verdict: **no positive evidence for stock Turbo Pascal/Borland
Pascal runtime or compiler output in `BLOODPRG.EXE`**. This does not prove that
no Pascal-origin code was ever used, but it makes Pascal a lower-priority
candidate than Borland C/C++ with custom startup/runtime or a mostly custom
ASM/C engine.

## XDB Overlay Status

The `.xdb` files are executable runtime overlays, not ordinary asset banks.
They are raw code+data images, so `file` reports them as `data`, but Capstone
disassembles their entrypoints cleanly with `re/tools/dis_xdb.py`:

```sh
python3 re/tools/dis_xdb.py output/_tmp_dat/croolis.xdb 0 80
python3 re/tools/dis_xdb.py output/_tmp_dat/croolis.xdb 0xa3 90
```

Current extracted overlay sizes and SHA-256:

| Overlay | Size | SHA-256 |
|---|---:|---|
| `amer.xdb` | 266800 | `6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31` |
| `croolis.xdb` | 258832 | `13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31` |
| `scrut.xdb` | 258080 | `8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77` |
| `manu3.xdb` | 62544 | `d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31` |

The three alien/scrutinizer overlays (`amer`, `croolis`, `scrut`) share the same
custom entry ABI:

- save full 386 register state (`push eax/ebx/ecx/edx/esi/edi`, segment
  registers, `ebp`)
- derive `DS`/`FS` from `CS + cs:[delta]`
- patch segment words and a dispatch operand in the overlay image
- read a handle/selector through `les di,[bp]`
- call local body entry `0x00A3`
- restore registers and `retf`

The body entry is also structured subsystem code: it uploads a 768-byte VGA DAC
palette, initializes mouse ranges, clears VGA planes, runs init routines, walks
an object list at `fs:0x2308`, dispatches object methods through
`call word ptr fs:[bx+0x103a]`, and calls back through a runtime far pointer at
`[0x20]`.

`manu3.xdb` is a different overlay family. Its entry sets `FS/DS/ES` from
`cs:[0x136A]`, reads parameters from `[bp]`, and immediately runs 3D
matrix/projection math for the menu/manual overlay.

Current C/C++ verdict for XDBs: **unproven**. They contain native 16-bit/386
code and C-like object/table dispatch, but there are still no Borland/Turbo,
Watcom, Microsoft, Pascal, or runtime marker strings, and the entry/calling ABI
is custom rather than a stock C runtime shape. Treat them as raw overlay
modules that may contain compiler-generated C/C++-like routines mixed with
handwritten assembly, not as confirmed compiled C files.

## Function Atlas

`re/tools/function_atlas.py` emits a provenance-aware function report for the
bit-exact track:

```sh
python3 re/tools/function_atlas.py \
  --sample-limit 4 \
  --lift-queue-limit 8
```

Initial atlas output:

| Feature | Count |
|---|---:|
| relocation-proven direct far call sites | 365 |
| relocation-proven direct far targets | 107 |
| recursive graph functions from `re/func_graph.json` | 222 |
| recursive graph edges | 442 |
| recursive graph leaves | 112 |
| unresolved recursive graph indirect sites | 48 |
| far targets already present in recursive graph | 94 |
| far targets missing from recursive graph | 13 |
| graph entries without direct far incoming edge | 127 |
| atlas entry lower bound | 235 |
| entries beginning with `55 8B EC` | 0 |

The 13 missing far targets are not obvious junk. They include labeled service
and helper routines such as `rtc_time_read`, `get_rtc_date`, `poll_mouse`,
`strlen`, `file_open_wrapper`, `binary_u32_sqrt`, `gfx_draw_to_page`, and
`vm_lookup_prep`. That means the old 222-function graph is useful but not a
complete bit-exact lifting denominator.

This direct-call atlas is an interim denominator: **at least 235 entries before
decoding indirect dispatch tables**.

## Indirect Dispatch Atlas

`re/tools/indirect_dispatch_atlas.py` classifies the 48 indirect records in the
old graph:

```sh
python3 re/tools/indirect_dispatch_atlas.py --sample-limit 4
```

Initial output:

| Feature | Count |
|---|---:|
| old graph indirect records | 48 |
| unique indirect sites | 46 |
| classified records | 48 |
| unknown records | 0 |
| direct far calls to relative segment 0 misfiled as indirect | 9 |
| static internal dispatch records/sites | 6 |
| static-table distinct targets | 74 |
| static-table targets missing from old graph/direct-far denominator | 73 |
| lower bound after direct far atlas | 235 |
| lower bound after static-table decoding | 308 |

Static internal tables decoded by the atlas:

| Table | Dispatch site(s) | Entries | Distinct targets | Missing from old graph |
|---|---|---:|---:|---:|
| VM opcode handlers | `0x5627`, `0x56C4` | 52 | 37 | 36 |
| nav actor subdispatch | `0x7E09` | 6 | 6 | 6 |
| nav choice subdispatch | `0x8700` | 5 | 5 | 5 |
| sprite blitter candidates | `0x4506` | 8 | 8 | 8 |
| byte parser dispatch | `0x74E5` | 18 | 18 | 18 |

The remaining indirect categories are not all internal code:

- XMS driver vector `DS/GS:0x0A4A`: 18 records, external HIMEM/XMS boundary.
- Sound-driver vectors `DS:0x0CD3..0x0CF3`: 12 records, external
  `dnsdb.drv`/`nosound.drv` boundary.
- Presentation callback vector `DS:0x0A96`: 2 records, runtime callback that
  needs tracing.
- Input action dispatch `0x2137`: mechanism is proven, but the handler entries
  still require runtime trace or behavior matching. The xlat table at file
  `0x173E` has 159 live input bytes, 51 distinct action indices, max index 125.

Current denominator wording: **at least 308 internal `BLOODPRG.EXE` entries,
plus trace-resolved input-action/presentation-callback targets**. External XMS
and sound-driver vectors are runtime boundaries, not functions to decompile into
the game executable.

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
2. Repair the function denominator by merging the relocation-proven far-target
   atlas with the recursive graph, then resolving indirect dispatch tables.
3. Produce C-like per-function lifts whose emitted assembly is verified against
   the original function behavior first.
4. Only after enough codegen fingerprints are known, decide whether exact binary
   reproduction through a historical C/C++ compiler is realistic.

If no compiler match is found, the fallback bit-exact path is static
recompilation/lifting with an assembler/linker-controlled output, not idiomatic C.
