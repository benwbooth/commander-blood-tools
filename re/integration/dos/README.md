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
  --define BLOODPRG_RELINKED_RUNTIME \
  --object-dir output/bloodprg_objects
```

This creates a separate object for each manifest entry. The package's
`--include-bloodprg-runtime` gate adds the recovered game entrypoint, the
paragraph-aligned byte-backed data owners, and the DOS/XMS/sound adapters. It
links those objects into `cd/BPRG_RE.EXE` with zero unresolved symbols. That
executable now reaches and renders the first opening image under DOSBox-X.
The complete opening sequence is not yet timing-accurate, and full-game
behavior plus cross-XDB execution remain later runtime gates.

The graphics integration gate links the recovered palette-transition and
chunky-to-planar routines into small real-mode DOS executables. It verifies the
register ABI between the two palette routines and all 64,000 bytes written to
the four Mode-X planes:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/bloodprg_graphics_integration.py
```

The `--include-bloodprg-fixed-patch` gate is stricter and smaller. It emits
`validation/bloodprg_fixed/BLOODPRG_C_PATCHED.EXE` and the DOS alias
`cd/BPRG_C.EXE` only after each selected C routine passes its fixed-layout
policy. The current proven set is the three sprite no-ops, four GS byte-parser
handlers, `list_d8c_init`, the runtime-verified `vm_special_slot_remove`,
`lookup_table_1fb5`, `matrix_table_clear_2a1b`, and
`presentation_queue_finish` replacements, the runtime-verified
`presentation_mode_dispatch`, `nav_chart_list_build`, and
`nav_kind2_target_list_build`, the ABI-bound
`ship_3d_navigation_candidate_build` and
`ship_3d_position_field_resolve`, the ABI-bound
`entity_flag_state_transition`, plus
`palette_upload_if_dirty`. The latter uses a fixed 36-byte
C body, restores its six original data/call operands, and moves the two MZ
relocation entries for the moved far calls; the builder verifies those old
entries before changing them.
The resulting executable is game-loadable, but it is intentionally
conservative and is not the full C replacement.

When the archived Turbo C 2.01 tree is available, add
`--turbo-c-toolchain /path/to/tc201`. The builder then compiles the five
Turbo-specific fixed-layout candidates with Borland C under DOSBox-X and
accepts their relocation-only differences. Run that form from a shell that
contains both Open Watcom and DOSBox-X.

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

## Recovered package

The package gate compiles the BloodScript sources and every XDB C candidate,
then links all 169 recovered routines into complete AMER, CROOLIS, MANU3, and
SCRUT overlays. Each linked image retains the original data payload apart from
verified callback-pointer rebindings and has one load-time data-segment delta.
Because the natural-C code is larger than the original assembly, the builder
replaces the four variable-size resources and updates all following
`BLOOD.DAT` directory offsets while preserving every unrelated payload byte.
The generated `SCRIPT1..5.COD/BAS` files are compared byte-for-byte and copied
to the CD root. `BLOODPRG.EXE` remains available as the shipped fallback. The
optional `--include-bloodprg-runtime` gate also builds every recovered BLOODPRG
candidate with the relink runtime contract, derives the byte-backed owner from
the actual unresolved report, adds the recovered entrypoint and platform
adapters, and emits `cd/BPRG_RE.EXE`. The relinked executable has passed an
opening-to-bridge semantic gate and a DOSBox path through an archive cinematic
back to the bridge, but not the complete opening or full-game parity gates.

Build the Rust VM compiler in the project shell, then run the package builder
with Open Watcom:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix develop --command cargo build --quiet --bin cbvm

NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_recovered_package.py \
    --cbvm target/debug/cbvm \
    --output-dir output/recovered_dos_package
```

To include the recovered C runtime and its data-owner synthesis:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_recovered_package.py \
    --cbvm target/debug/cbvm \
    --include-bloodprg-runtime \
    --output-dir output/recovered_dos_package
```

To emit the conservative game-loadable C-patched executable:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_recovered_package.py \
    --cbvm target/debug/cbvm \
    --include-bloodprg-fixed-patch \
    --output-dir output/recovered_dos_package
```

With the archived Turbo C tree:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/build_recovered_package.py \
    --cbvm target/debug/cbvm \
    --include-bloodprg-fixed-patch \
    --turbo-c-toolchain /path/to/tc201 \
    --output-dir output/recovered_dos_package
```

The builder emits `cd/` (a runnable CD tree), `scripts/`, `xdb/`,
`xdb_objects/`, `bloodprg_objects/`, `validation/` (including complete linked
XDB maps/reports and the optional BLOODPRG C runtime),
`package_manifest.tsv`, and `README.txt`. The optional runtime directory
contains `BPRG_RE.EXE`, `link.map`, and `unresolved.tsv`; the latter must
contain only its header. The same executable is copied to `cd/BPRG_RE.EXE`.
The fixed-patch directory contains the fixed-layout audit listing and patched
executable; `cd/BPRG_C.EXE` can be launched with the same arguments as the
original `BLOODPRG.EXE`.
The CD tree can be tested against the real launch path with:

```sh
nix develop --command bash re/tools/capture_real_game.sh \
  output/recovered_dos_package/cd \
  output/recovered_dos_package/captures :84 accuracy/cblood_install
```

The complete source-XDB package has passed this launch path with the shipped
`BLOODPRG.EXE`. A host I/O trace observed the game seek to the rebuilt
`MANU3.XDB` member at its updated `BLOOD.DAT` sector before continuing through
the rendered 3D manual/UI sequence. The four package-built loose overlays also
pass `source_xdb_dos_integration.py`; AMER, CROOLIS, and SCRUT still require
real-game navigation gates that select those character overlays.

For the relinked runtime, pass `BPRG_RE.EXE` as the final executable argument
to the same capture script. For the fixed-patch alias, pass `BPRG_C.EXE`.

The relinked runtime's first opening image passes a synchronized
original-versus-source-build content gate: the live 768-byte palette and
64,000-byte decoded display buffer match the original exactly. Run timing
comparisons with DOSBox-X at `cycles=max` and compare matching guest timer
ticks; wall-clock throughput is compiler-dependent and is not a reason to
replace natural C with instruction-level assembly. At 3.2 seconds the source
and original reached ticks 196 and 195 and had consumed 9 and 10 presentation
entries respectively, with identical display and palette data. At 6 seconds
they remained one entry apart (44 versus 45), and their second-scene palettes
matched. This is an opening content and progression gate, not a
completed-opening or full-game C parity claim. The next runtime gate is
following the original through the complete opening into cross-XDB gameplay.

The natural-C runtime can require several times as many guest instructions as
the original assembly. `runtime_boot --cpu-multiplier 4` models a faster CPU
while preserving PIT, VGA, and Sound Blaster time. With that setting, the
relinked runtime reaches the bridge semantic state with the same 768-byte
palette as the original. A separate DOSBox Staging `cycles=max` run reaches the
title, bridge, an archive-only character cinematic, and the bridge again. That
gate also exercises EMS archive-index mapping; function `44h` requires the
logical page in `BX` and EMS handle in `DX`.

The later opening checkpoint is still open. Both builds load byte-identical VM
script resources into the same handles, but the relinked runtime remains at
active line `0xFFFF` and UI state 65 while the original advances to line 2 and
UI state 69 to display the title. Integrated tracing exposed one real cause:
the nested VM block scanner preserved the floating script `DS` but addressed
its handler table, yield byte, and skip byte as ordinary `DS` data. Those
objects are now explicitly `GAME_DATA`-qualified in natural C, and the direct
oracles still pass. The remaining gate is an audit of every routine reachable
while `DS` points at script bytes.

Nonvisual diagnostics default to DOSBox-X `core=normal` and `frameskip=10` to
reduce host rendering work. `core=dynamic` was tested but rejected because it
made DOSBox-X exit with status `-11` before the VM checkpoint. Emulator tuning
can accelerate the comparison loop; it is not a substitute for correcting C
segment ownership.

The runtime data owner also verifies and rebinds 88 near-pointer words to the
final linked C offsets: six navigation actor handlers, five navigation choice
handlers, all 16 input-action handlers, eight sprite blitters plus their null
mutable slot, and all 52 VM opcode handlers.
`validation/bloodprg_runtime/data_owner/pointer_rebindings.tsv` records every
original word and linked symbol. The input handlers have 32 direct
original-binary vectors covering directory and save-slot movement, Enter,
Escape, pause, text-key latching, shutdown, and every semantic no-op.

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
assembler output is a link-frontier probe, not a complete runtime data owner.
To preserve the bytes between known declarations, pass the original image and
the verified file bases:

```sh
python3 re/tools/bloodprg_data_layout_probe.py \
  --unresolved output/link_probe/unresolved.tsv \
  --image re/bin/BLOODPRG.EXE \
  --output-dir output/link_probe/data_layout_bytes
```

The default bases are CS/file `0x600`, GS/DS/file `0xD420`, and FS/file
`0xC1F0`; override them when analyzing another executable revision. The
byte-backed output remains a layout owner/probe because unknown declarations,
startup relocation, and runtime segment ownership are still unresolved. In a
fresh BLOODPRG-only build, 322 current candidate objects produced 724
unresolved symbols before the owner was added; the byte-backed owner classified
718 of them and reduced the link frontier to six symbols. Those six are the
external sound-driver calls `cb_snd_clip_play`, `cb_snd_stream_play`,
`cb_snd_stream_service`, and the XMS calls `cb_xms_allocate_kb`,
`cb_xms_move`, `cb_xms_release`. `bloodprg_platform_adapters.c` now supplies
the recovered register-level wrappers: sound calls enter driver table slots
`GS:0xCDB`/`GS:0xCF3`, and XMS calls use the HIMEM entry with AH
`09h`/`0Ah`/`0Bh`. The fresh 323-object probe links with zero unresolved
symbols. Runtime still requires the real loaded sound driver and HIMEM entry;
this does not make the probe a replacement `BLOODPRG.EXE`.

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

## Source-linked raw XDB images

`build_source_xdb.py` performs the production link model rather than the
fixed-offset experiment. It places a small host-ABI shim at raw offset zero,
normally links every recovered C routine, derives the data-segment delta from
the linked layout, and retains the original initialized data/work payload.
The shim is the only assembly in this path: it converts the game's BP/SS
request convention to Open Watcom's register convention and far-returns to the
host. C routine lengths and addresses are otherwise unconstrained.

Build any recovered module with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/build_source_xdb.py \
    --module croolis \
    --object-dir output/xdb_objects/xdb_croolis \
    --raw-xdb output/_tmp_dat/croolis.xdb \
    --output-dir output/source_xdb/croolis
```

The builder rejects a nonzero entry offset, a non-paragraph data segment, any
runtime relocation other than the additive data-segment word, a manifest/object
mismatch, an unknown embedded near callback, or any unexplained changed byte in
the original non-code payload. For the alien overlays it verifies and rebinds
the 15-entry method table, initialized resume callbacks, and initialized slot-2
and slot-3 state callbacks. Every other payload byte must match the shipped
overlay.

The current clean builds are:

| overlay | C routines | linked code | retained payload | checked pointer rebindings | rebuilt bytes | shipped bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AMER | 55 | 31,344 | 253,872 | 88 | 285,216 | 266,800 |
| CROOLIS | 47 | 30,352 | 245,792 | 53 | 276,144 | 258,832 |
| MANU3 | 12 | 8,896 | 57,568 | 0 | 66,464 | 62,544 |
| SCRUT | 55 | 31,104 | 244,848 | 53 | 275,952 | 258,080 |

Each image has raw entry `0000:0000` and exactly one runtime relocation, the
additive data-segment word. Size changes are expected from normal C linking and
are not ABI failures.

Run the raw loader and initialization protocol under DOSBox-X with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/source_xdb_dos_integration.py \
    --build-dir output/source_xdb/croolis \
    --output-dir output/source_xdb_runtime/croolis
```

The DOS gate allocates the image, reads it across 64 KiB boundaries, far-calls
offset zero with the original BP/SS contract, and verifies the published data
segment, all three derived work segments, and the renderer continuation. It
also executes one complete frame for AMER, CROOLIS, and SCRUT and exits through
the original keyboard/callback protocol. All four modules pass this gate. Use
`--dump-raster` to retain the post-call 64 KiB raster segment for a byte-level
diagnostic comparison. Substitution into sustained game flow is the next gate.

To measure fixed-offset placement, use the layout probe:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/link_fixed_xdb_layout.py \
  --module croolis \
  --main-object output/link_probe/xdb_link_probe.obj \
  --owner-object output/link_probe/croolis_data_layout/croolis_data_layout_probe.obj \
  --raw-xdb output/_tmp_dat/croolis.xdb \
  --output-dir output/link_probe/croolis_fixed_layout
```

This compiles each candidate into a common `_CODE` segment, inserts the
original XDB bytes between candidates, and records each generated segment's
start, public entry offset, and end in `placement.tsv`. It fails when a
generated candidate would overlap an earlier fixed entry. That failure is
intentional: helper code emitted before a public entry, or a candidate that
ends before the raw routine's true control-flow boundary, must be recovered
and placed explicitly before any linked output can replace an XDB.

The audit also records the raw routine span from each assembly artifact and
marks generated code that exceeds that span. Most artifacts provide an exact
`byte_count`; callback disassemblies with an explicit raw-stop address use the
address range, and one legacy callback without a header uses its disassembly
extent. The `raw_size_basis` column makes that evidence level visible.

Use `--audit-only` to compile every candidate and report all independent
footprint conflicts without attempting the link. The current alien audit
exposes the same first conflict in each module: the natural-C API entry grows
to `0xbf`, while the original next entry is fixed at `0xa3`.

The current callback recovery queue is:

| overlay | resume | slot-3 initial | slot-3 update | slot-2 update | slot-2 finish |
| --- | ---: | ---: | ---: | ---: | ---: |
| AMER | recovered `0x1c34` | `0x12b3` | recovered `0x1414` | recovered `0x1692` | recovered `0x1aa0` |
| CROOLIS | recovered `0x1b85` | `0x130b` | recovered `0x146c` | recovered `0x1727` | not referenced |
| SCRUT | recovered `0x1c45` | `0x12f9` | recovered `0x145a` | recovered `0x171b` | not referenced |

## BLOODPRG fixed-layout audit

The aggregate DOS link is useful for symbol and runtime checks, but it cannot
replace the original primary executable: BLOODPRG has ten fixed code
segments, relocated DS/FS data, and an MZ entry contract. Audit the natural-C
candidates against that layout before attempting an image patch:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/audit_bloodprg_layout.py \
    --image re/bin/BLOODPRG.EXE \
    --output-dir output/link_probe/bloodprg_fixed_layout
```

The report verifies every assembly routine hash against the supplied image,
then records the original segment-relative span and the generated C span.
The current image audit covers all 321 manifest candidates with no raw hash
failures; 286 generated spans still exceed a routine or cover another fixed
entry. It is therefore a refusal report, not a patched executable.

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

`alien_amer_slot1_callbacks.c` links the split AMER `0x0BEA` callback head and
`0x0CAC` motion continuation. It verifies preserved high position words,
DS-owned 32-bit palette accumulators, selection and callback publication,
camera dispatch, and the distinct fixed-steering results produced by the four
staged distance adjustments. Run it with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py --case amer_slot1_callbacks
```

`alien_amer_slot2_callbacks.c` links the eleven recovered AMER slot-2 owners. It
verifies the reset seed and field ownership, countdown transfer through the
full common update, immediate selection callback invocation, the disabled
selection restart chain, the finish callback's in-bounds transition, and the
compiled but unreferenced steering sibling. Run it with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py --case amer_slot2_callbacks
```

`alien_croolis_slot2_callbacks.c` links the ten recovered CROOLIS slot-2
owners. It verifies countdown transfer, the heading ADC carry, low-word-only
position writes, fade timing, selected-state ownership, animation phase and
copy updates, camera reset, and the compiled but unreferenced steering sibling.
Run it with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py --case croolis_slot2_callbacks
```

`alien_scrut_slot2_callbacks.c` links the eighteen recovered SCRUT slot-2
owners. It verifies latch-return behavior, countdown and follower motion,
fade timing, the selection and damping chain, carry-return steering, finish
transition, visual reset, camera reset, and the unreferenced steering sibling.
Run it with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py --case scrut_slot2_callbacks
```

`alien_slot3_callbacks.c` links each alien overlay's generic slot-3 owner, its
four separately callable restart, resume, capture, and ring-zero callbacks,
and the six-routine slot-13 resume pipeline. The three DOS cases verify both
generic ring-flag dispatch paths, callback publication, queue ownership, frame
and resume countdown separation, position and motion resets, random-state
transition, timer gating, ring wrap, signed low-byte object motion, the three
independent resume-state words, low-word pair bounds and steering, timeout,
and final restart. Run only this family with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py \
    --case amer_slot3_callbacks \
    --case croolis_slot3_callbacks \
    --case scrut_slot3_callbacks
```

These are ordinary linked functions. Their generated byte lengths may differ
from the original owners; the DOS gate checks the recovered call contract and
behavior rather than fixed-offset patch suitability.

The recovered C makes the original implicit raster DS ownership explicit as a
segment argument from `0x0700` to `0x0D7D`. Raster records, reciprocal values,
free-list state, and active-list links remain direct typed far-memory accesses;
there is no register-state or instruction emulation layer.
