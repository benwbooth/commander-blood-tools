# Modern Rust port

This workspace ports recovered game behavior into typed Rust while retaining the
original data files as authoritative resources. `ported.tsv` is deliberately
conservative: a routine is listed only after its Rust behavior has direct binary
oracle coverage and documentation.

## Completion status

All 520 recovered native routines are accounted for by the checked coverage
ledgers: 470 routines have documented Rust implementations and 50 DOS memory,
hardware, or authored no-operation adapters have documented eliminations. The
coverage test rejects missing, duplicate, or unsupported mappings.

Routine counts alone do not prove cross-routine state fidelity. The recovered
BLOODPRG headers currently expose 71 semantically distinct address-alias
families after mechanical DS/GS views are collapsed. `shared-global-aliases.tsv`
is the checked review queue for those families. Sixty-three rows name a verified
canonical Rust owner or document why the native storage overlap can be split
after moving to flat typed state. The eight rows still marked `pending_review`
are not claims of behavioral parity.

The real-data acceptance suite restores an authentic save, reloads every script
profile and companion resource set, enters and completes an authored path for
all 65 recovered contacts, decodes every navigation world, and exercises the
production SDL3/wgpu Pterra HUD transition. These gates establish implementation
coverage and broad behavioral readiness; they do not replace a continuous
start-to-credits playthrough.

## Memory model

The runtime uses a flat address space. Skeleton parents, vertices, faces,
animation targets, and projection aliases are Rust indices, ranges, slices, and
owned collections. It has no near, far, or huge pointers, segment registers,
selectors, paragraph arithmetic, or emulated DOS memory.

XDB relocation deltas are interpreted only by the format decoder to locate file
sections. The decoder immediately produces typed owned data; native addresses do
not survive into runtime state.

## MANU3 status

Ten of the twelve MANU3 routines have direct Rust coverage. The current path
decodes all authored skeleton, animation, geometry, texture, trigonometry, and
raster-reciprocal data; runs recovered fixed-point animation and projection;
preserves face selection and activation decisions; and submits indexed textured
triangles to a depth-tested wgpu pipeline.

The DOS software scanline pool, linked raster records, Mode X plane writes, and
framebuffer segments are implementation details replaced by wgpu. The reserved
hand palette bank at indices 202 through 251 is decoded from `BLOODPRG.EXE` and
merged without overwriting scene-owned lower palette entries.

The two remaining routines are native-only adapters. Offsets `0x0121` and
`0x06f6` install relocated segments and must be classified as eliminated
flat-memory setup rather than recreated. `eliminated.tsv` records those mappings
separately from translated routines, and the coverage gate verifies that all
twelve recovered MANU3 entries are accounted for exactly once.

## Alien scene status

AMER, CROOLIS, and SCRUT are sibling instances of one interactive 3D engine.
The Rust port therefore uses a shared typed engine with explicit species policy
for the behavioral differences found in the overlays. The first shared routine,
mouse-driven camera control, is translated for all three overlays and checked
against twenty-four direct original-binary vectors. It retains the original
16-bit wrapping accumulator behavior without retaining DOS mouse interrupts,
register state, or segmented addresses.

The shared camera-transform routine is also translated for all three overlays.
Eighteen direct binary cases verify angle masking, fixed-point target-matrix
construction, matrix easing and roundoff, depth-motion integration, overflow,
and transformed view publication.

The shared hierarchy-transform and projection routine is translated for all
three overlays as well. Twenty-four direct binary cases verify typed parent
resolution, Q15 matrix composition, local and radial motion, wrapping overflow,
positive and nonpositive depth paths, every clipping edge, common-clip
rejection, and UV-alias projection copies. The runtime operates on model, node,
and vertex arrays; the original data and object segment split is eliminated.

Face selection and screen-column bucketing are translated for all three
overlays. Thirty direct binary cases verify common-clip rejection, cyclic
leftmost-vertex ordering, both tie branches, unsigned width limits, wrapped and
negative columns, LIFO bucket order, multi-model traversal, and typed
species-specific camera-plane signals. Raster-segment bucket offsets and
pointer-valued context latches do not enter the Rust model.

The camera-relative primary mesh coordinator is translated for all three
overlays. Twenty-seven direct binary cases verify retained authored raster
depth, negative and sub-byte depth rejection, modular matrix products, all clip
edges, whole-mesh rejection, face rotations and ties, width limits, and LIFO
buckets. The decoder now preserves the primary vertices' authored screen and
raster-depth fields rather than treating the entire vertex tail as scratch.

The shared 1,200-star generator is translated for all three overlays. Twenty-four
direct binary cases verify the complete random stream, logical camera cells,
modular fixed-point projection, negative and zero depth, all viewport edges,
visible-star order, shade selection, and palette lookup. The Rust output is a
flat vector of typed screen-space stars; VGA plane records and port writes are
eliminated presentation details.

The typed format layer now decodes all three original XDB images: the shared
primary mesh, 48 named behavior models, camera/root/node hierarchy, vertices,
projection aliases, faces, 256-by-512 indexed texture atlas, display palette,
trigonometry table, and 500-entry raster reciprocal table. It also resolves
method-table slots to semantic behavior kinds. File-relative relocation and
object offsets are loader inputs only and become validated Rust indices.

The alien face-activation rule is translated for all three overlays. Thirty-nine
direct vectors verify raster-capacity rejection, fixed-point orientation,
vertical-edge handling, backface and degenerate rejection, and width limits.
Accepted faces become owned textured triangles with unsigned 256-by-512 atlas
coordinates and recovered depth values.

The SDL3 executable now connects the typed alien frame to wgpu. It renders the
primary mesh, palette-colored starfield, and behavior models in recovered order,
with independent depth passes for the two mesh layers. The segmented scanline
pool, linked span records, Mode X planes, and framebuffer writes are classified
as eliminated presentation adapters. Offscreen GPU tests render all three
shipped overlays at wide and portrait output sizes.

Four shared behavior operations are translated for all three species. Ninety-six
direct binary cases verify camera-relative position wrapping, bounds and exit
updates, anchor publication, and the AMER/CROOLIS half-delta versus SCRUT fixed
state adjustment. Three malformed zero-count wrapping vectors are rejected by
the typed API instead of reproducing a 65,536-iteration address-space walk.
Every method-table variant selected by the shipped models is connected to the
live `AlienScene::step` dispatch.

The full-scale and one-sixteenth-scale cyclic sample methods are translated for
all three species as typed texture animation. Thirty well-formed binary cases
verify signed cosine sampling, phase wrapping, prior-sample differencing,
wrapping texture-U updates, multi-vertex traversal, and preservation of
texture-V. Six unaligned source states and six zero-count address-space walks
are deliberately rejected. Mutable UV arrays feed the wgpu triangle path
directly, with no retained source addresses or memory facade.

The palette-animation method is translated for all three species with 24
original-binary vectors. Tests cover root and first-node motion, every phase
exit, countdown reversal, CROOLIS/SCRUT pulse updates, both texture-page
regions, reversed intervals, and complete 64 KiB texture-result SHA-256 hashes.
The 256-entry remap table is decoded directly from each XDB. Runtime state uses
typed transforms, counters, and owned texture bytes only.

The wave method is translated for all three species with 33 original-binary
vectors. It initializes typed node and phase state, performs camera-relative
selection, decays accelerated phases, and applies both cyclic and
distance-weighted cosine motion to owned object-space vertex positions. The 30
active cases match complete reconstructed vertex-record SHA-256 hashes.
Hierarchy projection now consumes those mutable positions directly.

The slot-3 coordinator is translated for all three species with 24 valid
original-binary cases. Its fixed 128-entry motion-history ring uses ordinary
owned records and slot indices, while indirect routine addresses become a typed
callback enum and dispatch trait. Three malformed zero-count cases are rejected
instead of reproducing the original full-address-space walk. Its concrete
callbacks are connected to the live scene loop through typed dispatch.

Four supporting slot-3 callback operations are translated for every species:
course restart, resume-clear setup, resume capture, and timer-gated history
clearing. All 42 direct binary vectors match their represented state. Captured
nodes use `Option<usize>` and ring positions use checked slots; callback entry
addresses and byte cursors do not survive into runtime data. The initial-course
and follower-course callback bodies are translated, oracle-tested, and invoked
by the slot-3 coordinator.

The slot-13 resume coordinator is translated for all three species with 18
direct binary cases. Callback presence is a typed enum option, and paired or
resumed model state uses optional node indices. Initialization and indirect
dispatch therefore retain the native state-machine decision without retaining
routine addresses or model-record pointers.

The leading-node slot-3 course callback is translated for all three species
with 39 direct binary cases. It applies timer-gated ring motion, generates new
deterministic courses, and corrects every depth, lateral, and vertical scene
boundary through typed node and ring state. Oracle work corrected the recovered
C radial mask to 7 bits for AMER and 6 bits for CROOLIS/SCRUT.

Run the current interactive path with original assets:

```sh
nix develop -c cargo run -p commander-blood-game -- \
  --asset output/_tmp_dat/fd/pterra1f.lbm \
  --manu3 output/_tmp_dat/manu3.xdb
```

Run one recovered alien scene with:

```sh
nix develop -c cargo run -p commander-blood-game -- \
  --alien output/_tmp_dat/amer.xdb
```

The renderer never warps or moves the host pointer. SDL mouse events are mapped
into the letterboxed 320-by-200 game coordinate system.
