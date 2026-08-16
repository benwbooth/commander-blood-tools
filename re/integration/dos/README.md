# DOS Integration Gates

These programs link recovered natural-C routines into real-mode DOS
executables and run them under DOSBox-X. They are the bridge between the
per-routine Unicorn oracles and replacing routines in the original XDB
execution environment.

Run the current MANU3 and alien gates from the repository root:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py
```

Run the first BLOODPRG data-owner gate with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/bloodprg_startup_integration.py
```

This gate links the recovered startup command-line parser, option dispatcher,
and decimal parser against canonical C storage for the startup state. It runs
the executable under DOSBox-X and verifies the audio-option and write-directory
state mutations. It intentionally does not define unrelated game globals.

`manu3_renderer_empty.c` calls the recovered MANU3 `0x0700` renderer with one
fully clipped face. This is a complete original control-flow path: the sorter
rejects the face, initializes the 200-record raster free list, scans all 320
empty buckets, and returns. The test compares every byte of the raster arena
and verifies that the geometry arena was not modified.

`manu3_renderer_active.c` drives a real textured triangle through both recovered
`0x0700` and `0x0D7D`. It uses the shipped reciprocal-table values needed by
that triangle, the linear framebuffer continuation, and the same texture bytes
as the direct raw-overlay oracle. The DOS executable writes all 64,000 output
bytes; the host runner requires their SHA-256 to match the unmodified overlay.

`manu3_face_activate.c` independently drives the wide vertical-first-edge case
through `0x0D7D` and compares the complete 90-byte raster record against the
raw overlay. This specifically covers the 32-bit texture-delta operation order
that is otherwise easy to lose when expressing the routine in 16-bit C.

`alien_face_activate.c` drives the equivalent CROOLIS `0x2BDD` path through
the one-function natural-C activator. Its complete 90-byte record must match
the hash produced by direct execution of the shipped CROOLIS overlay.

`alien_starfield.c` links the recovered CROOLIS `0x0775` owner, generates its
1200-point balanced projection case, and writes the complete 64 KiB raster
workspace followed by the 64 KiB framebuffer. The combined file must match a
hash produced by direct execution of the shipped CROOLIS overlay.

`alien_main.c` links the recovered CROOLIS `0x00A3` far main-loop owner with
typed test callbacks at each external call boundary. A real BIOS-buffered
Escape key drives one complete frame. The DOS program verifies segment
restoration, initialization, two-context method dispatch, call order, timer
callback arguments and state, page rotation, keyboard publication, cleanup,
and the module-specific control-latch clear.

`alien_entry.c` links the recovered CROOLIS `0x0000` host entry, allocates a
paragraph-aligned data directory, passes a real far timing/callback request,
and verifies segment derivation, renderer continuation, callback publication,
the pre-main method delta, a typed main-boundary update, and timing writeback.

The recovered C makes the original implicit raster DS ownership explicit as a
segment argument from `0x0700` to `0x0D7D`. Raster records, reciprocal values,
free-list state, and active-list links remain direct typed far-memory accesses;
there is no register-state or instruction emulation layer.
