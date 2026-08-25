# Rust port

The modern game is a separate workspace package at
`crates/commander-blood-game`. It is not a continuation of the retired
heuristic frontend.

## Boundaries

- `commander_blood_game::native` contains direct, typed Rust translations of
  recovered C routines. A routine is counted only after original-binary oracle
  vectors or an equivalent differential gate passes.
- `commander_blood_game::render` owns wgpu presentation and the future 3D
  pipeline. It does not decide game state.
- SDL owns the host window, input, timing, and future audio-device integration.
- `commander-blood-formats` owns lossless original-data parsers shared by the
  reverse-engineering tools and game.
- The existing root package remains the reverse-engineering oracle and
  BloodScript compiler. The new game does not depend on that package and may
  not call the retired heuristic `EngineState` frontend.

`re/rust-port/ported.tsv` is the positive coverage ledger. Its test validates
every row against the authoritative BLOODPRG and XDB C manifests. Unlisted
routines remain unported; there is no inferred or percentage-based credit.

## Source conventions

- Use names that describe game concepts, not registers, offsets, or decompiler
  temporaries. Retain the native address in documentation for provenance.
- Document every public module, type, field, and function. The game crate denies
  missing documentation at compile time.
- Give nontrivial numeric values named constants or enum variants. Use decimal
  for ordinary quantities and hexadecimal only for masks, packed values,
  addresses, and other binary-facing notation.
- Keep host adaptation in platform and rendering modules. Native game logic may
  not emulate segmented memory, register state, or DOS services.
- Use ordinary Rust ownership, references, slices, and typed indices. Near,
  far, and huge pointers; segment arithmetic; 16-bit address wrapping; and
  offset-addressed memory facades are prohibited in the game crate. Original
  addresses may appear only as routine provenance and oracle-fixture data.

## Current executable

`cargo run -p commander-blood-game --bin commander-blood` opens the original
`BLOOD.LBM` title art in an SDL window and presents it with wgpu. The renderer
uses nearest-neighbor sampling and aspect-correct letterboxing, preserving the
decoded source image while allowing arbitrary output resolution.

## MANU3 foundation

`commander-blood-formats::manu3` follows the original XDB's own initialized
section directory and decodes the authored skeletal hand directly from
`MANU3.XDB`: 16 nodes, 110 model vertices, 32 UV-seam aliases, 216 faces, the
indexed texture, the Q14 trigonometry table, and all 32 animation selectors.
No savestate or runtime memory dump is linked into the modern game.

`commander_blood_game::native::manu3` owns the flat-memory runtime. Six MANU3
routines are currently translated and oracle checked: animation selection,
tween construction/stepping, hierarchical matrix construction, and entity
projection. The typed model connects these stages end to end. GPU face
submission and the main MANU3 coordinator remain in progress and are not yet
counted as ported routines.

## Alien-overlay foundation

`commander-blood-formats::alien` decodes the initialized sections of
`AMER.XDB`, `CROOLIS.XDB`, and `SCRUT.XDB` into owned camera state, model
hierarchies, meshes, texture and palette data, trigonometry and raster tables,
and starfield parameters. Original section offsets are consumed only while
loading the files; no relocated pointers or segmented addresses survive in the
runtime model.

`commander_blood_game::native::alien` currently connects the translated camera
control, camera transform, primary-mesh projection, starfield generation,
hierarchy projection, and face-selection stages in their recovered frame
order. The three shipped overlays pass this typed frame pipeline. The recovered
face-activation decision then converts accepted faces into owned textured
triangles. SDL3 input drives the scene and wgpu renders the primary mesh,
palette-colored stars, and behavior models in recovered order at the host
resolution. Behavior method dispatch and original frame timing are still
pending, so the overlay main routine is not yet listed as ported.

The first behavior-method slice covers position wrapping, bounds and exit
updates, anchor selection, and species-specific state adjustment. It operates
directly on typed node poses and passes 96 original-overlay cases. The scene
loop will invoke behavior dispatch only after every method used by the shipped
models is translated; pending variants are not treated as no-ops.

Both cyclic sample-delta methods are now translated for all three overlays.
They advance a typed cosine-table phase and mutate the texture-U coordinates of
authored vertices in owned model poses; the wgpu triangle path consumes those
runtime coordinates. Thirty well-formed binary cases match exactly. Six
unaligned source states and six zero-count address-space walks from the oracle
suite are rejected at the typed boundary instead of being reproduced. The
architecture gate also rejects DOS pointer and real-mode address types from the
game crate.

The palette-animation method is translated for AMER, CROOLIS, and SCRUT. The
format layer decodes its authored 256-entry texture-index remap table, and the
native method updates a typed model transform, cycle direction/countdown,
species pulse levels, and bounded regions of an owned texture atlas. All 24
binary vectors match, including complete 64 KiB texture-result SHA-256 hashes.
It remains outside live behavior dispatch until the other active method kinds
are translated.
