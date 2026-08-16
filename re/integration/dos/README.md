# DOS Integration Gates

These programs link recovered natural-C routines into real-mode DOS
executables and run them under DOSBox-X. They are the bridge between the
per-routine Unicorn oracles and replacing routines in the original XDB
execution environment.

Compile every recovered XDB routine to an Open Watcom real-mode object with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_xdb_objects.py \
  --object-dir output/xdb_objects
```

The command emits one `.OBJ` per manifest entry and a hash manifest recording
the exact compiler command and output. These objects are source-build evidence,
not `.xdb` overlays: a loadable overlay still requires the original fixed code
and data layout, entry offsets, segment ownership, and cross-module symbols.
The integration gates below verify those boundaries individually while that
linker/layout work remains unresolved.

The same builder can compile the recovered BLOODPRG candidates:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_xdb_objects.py \
  --manifest re/source/bloodprg/candidates/manifest.tsv \
  --module-prefix '' \
  --output-label bloodprg \
  --object-dir output/bloodprg_objects
```

This creates a separate object for each manifest entry, but it is not a link
claim. The next executable gate must supply the canonical shared-data owners,
DOS/XMS/EMS adapters, cross-XDB calls, and the recovered startup boundary.

The BLOODPRG candidate headers intentionally omit the unverified `#pragma aux`
register contracts for the recovered VM, graphics, resource, and byte-parser
calls. Open Watcom rejects several of those contracts as illegal register
clobbers; the affected routines therefore compile with the normal C calling
convention. This proves source/object completeness only. It does not yet prove
that the original DOS entry ABI or cross-module data model has been reproduced.

To turn an object build into a measured aggregate-link attempt, pass a real DOS
harness object to the linker probe:

```sh
mkdir -p output/link_probe

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  wcl -q -c -3 -mm \
  -i=re/source/bloodprg/candidates/include \
  -fo=output/link_probe/startup_gate.obj \
  re/integration/dos/bloodprg_startup_options.c

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/link_recovered_objects.py \
  --main-object output/link_probe/startup_gate.obj \
  --object-dir output/bloodprg_objects \
  --object-dir output/xdb_objects \
  --extra-object output/link_probe/bloodprg_platform_adapters.obj
```

The probe writes `unresolved.tsv` and `link.log`, and exits nonzero until the
reported owners and platform boundaries are implemented. It never generates
dummy definitions or treats an unresolved link as a runnable game binary.

## Recovered hybrid package

The current full-package gate is a deliberately explicit hybrid. It compiles
the BloodScript sources, compiles every XDB C candidate, verifies the three
one-byte no-op candidates with `wdis`, and links small real-mode DOS probes for
the three mouse-position routines and the MANU3 entry. Those probes compare
the generated instruction shapes against the original fixed overlay offsets.
The builder then patches those fixed offsets in the three alien overlays and
rewrites the same-size XDB resources inside `BLOOD.DAT`. The generated
`SCRIPT1..5.COD/BAS` files are compared byte-for-byte and copied to the CD
root. `BLOODPRG.EXE` remains the shipped executable until its startup,
shared-data, DOS/XMS/EMS, and cross-XDB boundaries are recovered; the package
never pretends that the executable has been replaced by C.

Build the Rust VM compiler in the project shell, then run the package builder
with Open Watcom:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix develop --command cargo build --quiet --bin cbvm

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_recovered_package.py \
    --cbvm target/debug/cbvm \
    --output-dir output/recovered_dos_package
```

The builder emits `cd/` (a runnable CD tree), `scripts/`, `xdb/`,
`xdb_objects/`, `validation/` (including the DOS shape-probe binaries and
disassemblies), `package_manifest.tsv`, and `README.txt`.
The CD tree can be tested against the real launch path with:

```sh
nix develop --command bash re/tools/capture_real_game.sh \
  output/recovered_dos_package/cd \
  output/recovered_dos_package/captures :84 accuracy/cblood_install
```

This is a resource-integrity and runtime-smoke gate, not a claim of a full
C replacement. The next source milestone is recovering the startup/shared
data and cross-XDB owners required to replace `BLOODPRG.EXE` itself.

To classify the aggregate probe's data symbols against the recovered
BLOODPRG layout, generate the measurement-only layout object:

```sh
python3 re/tools/bloodprg_data_layout_probe.py \
  --unresolved output/link_probe/unresolved.tsv \
  --output-dir output/link_probe/data_layout

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  wasm -q output/link_probe/data_layout/bloodprg_data_layout_probe.asm \
  -fo=output/link_probe/data_layout/bloodprg_data_layout_probe.obj
```

The tool currently classifies the documented BLOODPRG declarations only. Its
assembler output is deliberately zero-filled and is a link-frontier probe,
not a runtime data owner. On the published object set it classified 718 of
875 unresolved symbols and reduced the measured frontier to 157. The
remaining symbols must be supplied by their real XDB module data segments,
DOS/XMS/EMS/audio services, or verified ABI thunks; they must not be resolved
by copying this probe into a production executable.

The first production adapter slice is compiled separately and can be added to
the probe's object directory:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  wcl -q -c -3 -mm -zdp -we \
  -i=re/source/bloodprg/candidates/include \
  -fo=output/link_probe/bloodprg_platform_adapters.obj \
  re/integration/dos/bloodprg_platform_adapters.c
```

This source implements the recovered DOS 21h file calls, DTA lookup, EMS
page-map call, allocation-failure dispatch, and the five verified far/near
source aliases. Pass its object with `--extra-object` as shown above. XMS and
sound-driver calls remain external far-call ABIs and are intentionally not
replaced with no-op bodies.

The aggregate measurement is not a valid XDB link model. Each alien overlay
uses the same `xdb_alien_*` source names for a different relocated data
segment, so AMER, CROOLIS, and SCRUT must be linked and loaded as separate
overlays. Generate a byte-backed layout owner for one overlay with:

```sh
python3 re/tools/xdb_data_layout_probe.py \
  --module croolis \
  --unresolved output/link_probe/unresolved.tsv \
  --image output/_tmp_dat/croolis.xdb \
  --data-file-base 0x32f0 \
  --output-dir output/link_probe/croolis_data_layout

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  wasm -q \
  -fo=output/link_probe/croolis_data_layout/croolis_data_layout_probe.obj \
  output/link_probe/croolis_data_layout/croolis_data_layout_probe.asm
```

The verified DS/FS/SS file bases are AMER `0x3280`, CROOLIS `0x32f0`, SCRUT
`0x33b0`, and MANU3 `0x1370`. The tool copies only the intervals between
recovered declarations and emits labels in `_CODE` for code-resident state
and `XDB_DATA` for relocated overlay data. It is still a layout owner/probe,
not an overlay entrypoint: it does not supply missing callbacks, external
XMS/audio services, or un-recovered declarations. The AMER base is derived
from the word at `CS:0x3275` (`0x0328`), CROOLIS from `CS:0x32e5`
(`0x032f`), and SCRUT from `CS:0x33a5` (`0x033b`); these are file mappings,
not guessed segment values.

For a module-scoped frontier, compile the empty link entrypoint and link only
one recovered module plus its owner:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  wcl -q -c -3 -mm -zdp -we \
  -i=re/source/xdb/candidates/include \
  -fo=output/link_probe/xdb_link_probe.obj \
  re/integration/dos/xdb_link_probe.c

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/link_recovered_objects.py \
  --main-object output/link_probe/xdb_link_probe.obj \
  --object-dir output/xdb_objects/xdb_manu3 \
  --object-dir output/link_probe/manu3_data_owner \
  --output-dir output/link_probe/manu3_module
```

With the current recovered object set and byte-backed owners, MANU3, AMER,
CROOLIS, and SCRUT each link without unresolved symbols. The CROOLIS and
SCRUT closure required explicit two-ABI slot-1 bridges, their slot-13 resume
state machines, and the module-local slot-2 update callbacks. Those links
prove module-level symbol ownership and DOS object compatibility; they do not
yet prove that the generated C matches every raw-overlay state transition.
The slot-2 ports therefore remain candidates for direct raw-vector validation
before they are used to construct production overlays.

The current callback recovery queue is:

| overlay | resume | slot-3 initial | slot-3 update | slot-2 update | slot-2 finish |
| --- | ---: | ---: | ---: | ---: | ---: |
| AMER | recovered `0x1c34` | `0x12b3` | recovered `0x1414` | recovered `0x1692` | recovered `0x1aa0` |
| CROOLIS | recovered `0x1b85` | `0x130b` | recovered `0x146c` | recovered `0x1727` | not referenced |
| SCRUT | recovered `0x1c45` | `0x12f9` | recovered `0x145a` | recovered `0x171b` | not referenced |

The slot-3 entries are embedded in the recovered method blocks rather than
being separate manifest entries. Their callback addresses come directly from
the `mov [state+0x0e], immediate` stores in the raw overlays, which is why
the module link probe can name the missing routines precisely.

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
