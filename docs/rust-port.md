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

## Current executable

`cargo run -p commander-blood-game --bin commander-blood` opens the original
`BLOOD.LBM` title art in an SDL window and presents it with wgpu. The renderer
uses nearest-neighbor sampling and aspect-correct letterboxing, preserving the
decoded source image while allowing arbitrary output resolution.
