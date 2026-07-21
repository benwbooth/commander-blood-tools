# Commander Blood Rust Reimplementation Roadmap

## Objective

Fully reverse engineer the DOS version of Commander Blood enough to replace
`BLOODPRG.EXE` with a Rust implementation that runs the original English CD data
files. The Rust program should reproduce the original game's script behavior,
cutscenes, UI, rendering, audio, input, timing, and save-state behavior closely
enough that generated video and interactive play match real-game oracle captures.

This is broader than the current media exporter. The exporter remains useful as
the first vertical slice: it exercises the asset formats, dialogue VM,
presentation events, renderer, and audio mixer without needing the whole
interactive game loop first.

## Current Completion Status

The port (`src/*.rs` outside `src/recomp/`, driven by `EngineState` and
`run_engine_window`) is a **complete, faithful scene/cutscene player**. The
remaining gap to a full interactive-game replacement is dominated by subsystems
that are *not yet reverse-engineered* (undecoded runtime data and mechanics), not
by unwritten port code — so closing it is new RE work gated on oracle ground
truth, not transliteration. Fabricating those subsystems would violate the
"faithfully accurate" requirement, so they are left explicit rather than guessed.

Verified end to end by `engine::tests::full_playable_loop_end_to_end` (fast CI
test) and `src/bin/smoke.rs` (full five-scene headless playthrough): title →
intro (with the DESCRIPT-sourced CRYO publisher credit) → nav → every screen →
all five dialogue scenes play to completion (101/169/327/145/258 lines).

Against the Definition of Done below:

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 1 | Boot on original CD data | **Done** | loads DESCRIPT/SCRIPT/HNM/LBM/SPR/SND/VOC |
| 2 | Intro + HNM cutscenes (timing/palette/audio/subtitles) | **Done** | HNM carries its own palette; intro credit sourced from DESCRIPT; reveal timing + chatter decoded |
| 3 | Ship/nav UI + mouse/keyboard | **Partial** | compass steering + click + screen toggles work; nav *layout* + *palette* are approximations (RE-blocked, see below) |
| 4 | Execute compiled BASIC scripts | **Done** | `vm.rs` walk + `execute_trace`; A6 text/voice decoded |
| 5 | Dialogue scenes (bg/actor/voice/subtitle/sfx/music/timing) | **Done** | talk-HNM background, per-line voice, subtitle reveal + chatter, scene music; HUD strip approximate |
| 6 | Location navigation + interactive object flows | **Partial** | location visit is a faithful static room viewer with decoded `.ext` object positions; interaction semantics RE-blocked |
| 7 | Save/load state | **Absent** | downstream of the interactive-state layer (little decoded state to persist yet) |
| 8 | Oracle suite | **Partial** | per-behavior tests + smoke playthrough; no full frame-diff oracle suite |

Phase status: **Phase 1 (data layer) and Phase 2 (script VM/trace) complete;
Phase 3 (game-accurate presentation) substantially complete** (subtitle assembly,
reveal/chatter timing, voice indexing, talk-HNM behavior all decoded and ported);
**Phases 4–5 blocked** on the RE items below.

### RE-blocked remainder (needs decoding before it can be ported faithfully)

- **Ship-view / bridge / nav VGA palette** — the real palette is uploaded at
  runtime from an as-yet-unidentified resource (`re/REVERSE.md`: "identifying
  which resource sets the ship-view palette"; master buffer `gs:0x5B58`). The port
  substitutes a grey ramp so the indexed starfield/sprites read. `engine.rs`
  `render_bridge`/`render_ship_view`.
- **Star-map destination layout** — the 11 destinations live in runtime data
  `DS:0x4F09` not reproducible from the static binary; the port tiles an
  approximate grid (`engine.rs::render_nav_pyramid_sprites`, `ship3d.rs:1881-1888`).
- **HUD pyramid vertex→screen projection** — the recovered verts exist
  (`SHIP_3D_HUD_PYRAMID_VERTICES`) but the projecting routine is unlocated
  (`ship3d.rs:1911-1916`); the dialogue-mode nav strip uses a placeholder.
- **On-planet object interaction** — `entity.rs` models the decoded object record
  + flag state machine, but object population source, per-screen rendering, and
  click/interaction semantics are undecoded.
- **Comms/cyberspace mini-game logic** — presentation-only; input/goal logic
  undecoded (`engine.rs` note "the navigation minigame logic is undecoded").
- **Pyramid-menu UI** — `manu3.rs` provides the decoded interaction/animation math
  but the pyramid renderer depends on the unlocated projection above.

All of these need instruction-level oracle ground truth to decode, which is the
same DOSBox-differential bottleneck tracked elsewhere; none can be reproduced
faithfully by writing more port code against what is currently decoded.

## Ground Rules

- Original data files are the content source. The Rust code should load the
  user's Commander Blood data instead of embedding copyrighted assets.
- Reverse-engineering notes should describe behavior, data layouts, tables, and
  algorithms. Avoid copying large literal binary-derived blobs unless they are
  small compatibility tables already needed by the engine and their provenance is
  documented.
- Every claim about original behavior needs a source: binary address, data-file
  evidence, DOSBox oracle capture, or a repeatable tool output.
- Prefer runnable Rust modules over one-off notes once a subsystem is understood.
- Preserve the current `re/` workflow: labels and tools are the durable RE
  record; Rust tests and oracle comparisons are the durable correctness record.

## Definition of Done

The project is complete when a Rust executable can:

1. Boot using the original English CD data set.
2. Play the intro and full-HNM cutscenes with matching timing, palettes, audio,
   subtitles, and transitions.
3. Enter the ship/navigation UI and respond to mouse and keyboard input.
4. Execute the compiled BASIC scripts from `SCRIPT*.COD/VAR/DIC/DEB`.
5. Render dialogue scenes with the correct background, actor animation, voice
   clip, subtitle text, subtitle sound, music, HUD state, and timing.
6. Navigate between locations and run interactive dialogue/object flows.
7. Save and load enough state to match the original game's persisted behavior.
8. Pass a representative oracle suite captured from the real DOS game.

The short-term success criterion is narrower: one known dialogue scene should be
generated from original data through a VM/event-renderer path with no static
character/background/music guessing.

## Current Assets and Evidence

- `src/extract/` parses and exports many media formats already:
  `BLOOD.DAT`, HNM, SND/VOC, DESCRIPT, SCRIPT dictionaries, dialogue manifests,
  and video/audio composites.
- `src/vm.rs` contains the Rust implementation of the recovered token decoder,
  bounded state interpreter, and first branch-aware execution trace.
- `re/REVERSE.md` contains the active binary map, VM notes, renderer notes,
  subtitle timing, and known dead ends.
- `re/tools/` contains repeatable binary inspection helpers.
- `accuracy/` contains the DOSBox-X oracle harness and first capture notes.

## Architecture Target

Split the Rust code into a reusable core and thin frontends.

Core modules:

- `assets`: data-file discovery and archive loading.
- `formats`: DAT, HNM, LBM/PBM, SND/VOC, DESCRIPT, SCRIPT, fonts, sprites, save
  files, and config files.
- `vm`: compiled BASIC bytecode execution, state, functions, object table,
  dictionaries, events, and deterministic replay controls.
- `engine`: game state, scene manager, navigation, inventory, interaction, and
  script scheduling.
- `render`: software 320x200 indexed framebuffer, palette updates, HNM
  decode/blit, dialogue compositing, HUD drawing, text rendering, transitions,
  and scaling; plus a `wgpu` backend for the recovered 3D/minigame subsystem
  once its original runtime state, projection, geometry, and input semantics are
  decompiled.
- `ship3d`: recovered ship/procedural-3D transition state, planar page-band
  copy primitives, and eventually the original geometry/projection/input model
  that can drive the `wgpu` backend.
- `audio`: SND/VOC decoding, music, voice, UI chatter, mixing, looping, and
  timing.
- `input`: DOS mouse/keyboard semantics mapped to modern frontends.
- `oracle`: capture metadata, frame/audio comparison, and scenario definitions.

Frontends:

- `extract` / headless video export for cutscenes.
- Interactive desktop runtime for playing the game.
- CLI inspection tools for reverse engineering and regression checks.

## Reverse-Engineering Workflow

1. Pick one subsystem or one original function.
2. Locate it in `BLOODPRG.EXE` with `re/tools/dis.py`, `xref.py`, and
   `labels.csv`.
3. Record addresses, inputs, outputs, tables, and unresolved assumptions in
   `re/REVERSE.md`.
4. Add or update a Rust parser/interpreter/renderer module.
5. Add a focused test using original data where possible.
6. Validate against DOSBox oracle output once the behavior has a visible or
   audible surface.
7. Replace old heuristic paths only after the new path matches or improves the
   oracle evidence.

## Phase 1: Stabilize the Data Layer

Goal: all original data files are parsed by explicit, tested Rust structures.

Tasks:

- Promote exporter-local parsers into library modules where needed.
- Add typed structures for `BLOOD.DAT`, `DESCRIPT.DES`, `SCRIPT*.COD`,
  `SCRIPT*.VAR`, `SCRIPT*.DIC`, `SCRIPT*.DEB`, SND, VOC, HNM, and LBM/PBM.
- Keep binary-derived constants tied to source offsets.
- Add round-trip or fixture tests for parser boundaries, counts, offsets, and
  known records.
- Add an asset inventory command that reports missing, unknown, or unused files.

Exit criteria:

- The original English CD data set can be inventoried without heuristic file-name
  guessing.
- Parser tests cover all formats used by intro, dialogue, and one location.

## Phase 2: Script VM and Event Trace

Goal: execute compiled BASIC scripts well enough to produce the same presentation
events the game would produce for deterministic dialogue/cutscene paths.

Tasks:

- Finish opcode semantics for assignment, conditionals, branches, calls/returns,
  object references, random/state reads, and text calls.
- Model the `SCRIPT*.VAR` state area and object fields as typed accessors.
- Decode 0xA6 text call parameters completely: line id, voice selection, flags,
  loop target, subtitle behavior, and animation/audio routing.
- Produce an event trace:
  `SetScene`, `SetMusic`, `SetActor`, `PlayVoice`, `PlayHnm`, `ShowSubtitle`,
  `PlaySubtitleSfx`, `ClearSubtitle`, `Wait`, `Branch`, and `InputGate`.
- Preserve unresolved runtime choices explicitly instead of silently choosing a
  guessed branch.

Exit criteria:

- A known Bob/Izwalito dialogue trace can be generated from COD/VAR/DIC/DEB and
  DESCRIPT data without using `CHAR_CONTEXTS` or function-level grouping.
- The trace reports which events are proven, inferred, or unresolved.

## Phase 3: Game-Accurate Presentation

Goal: render VM events like the original game.

Tasks:

- Replace `words.join(" ")` subtitle text with the binary-derived text assembly
  rules.
- Match subtitle reveal timing and chatter sound scheduling.
- Match voice clip indexing and silence rules.
- Match talk HNM loop/reset behavior, transparency, band placement, palette
  behavior, and scene clearing.
- Recreate the HUD/navigation panel procedurally or from fully understood asset
  events.
- Locate and decompile the ship/procedural-3D minigame entrypoints and state
  variables before implementing the `wgpu` renderer for that path.
- Replace current background/music fallbacks with VM event state.

Exit criteria:

- The headless video exporter can reproduce one oracle-captured dialogue scene
  within defined frame/audio tolerances.
- Existing heuristic output paths are either removed or clearly marked as legacy
  inspection modes.

## Phase 4: Core Game Loop

Goal: run the non-cutscene game state in Rust.

Tasks:

- Implement a fixed-tick scheduler matching the original timing model.
- Implement scene transitions and location navigation.
- Implement input dispatch for mouse and keyboard.
- Implement object interaction, inventory, dialogue choice gates, and script
  scheduling.
- Implement save/load and config behavior as original-compatible structures.

Exit criteria:

- The Rust runtime can boot into the intro, proceed to the ship UI, accept input,
  and enter at least one scripted dialogue flow.

## Phase 5: Full Coverage and Cleanup

Goal: replace the original executable for the full game flow.

Tasks:

- Expand oracle scenarios to cover all major characters, location types,
  navigation states, menus, object interactions, and endings.
- Track unknown opcodes, unimplemented events, and mismatches as first-class
  reports.
- Remove legacy exporter heuristics after the runtime path covers the same use
  cases.
- Add developer documentation for adding labels, decoding handlers, and
  validating behavior.

Exit criteria:

- The Rust executable runs the game through representative playthrough paths.
- The oracle suite passes for visual, audio, and event-order checks.
- Remaining differences are documented as intentional or blocked by missing
  evidence.

## Immediate Milestones

1. Update the project docs to make the Rust reimplementation the explicit target.
2. Turn `src/vm.rs` into the canonical script-walking path for dialogue
   manifests.
3. Add a Rust `BLOODPRG.EXE` inspection layer so decompiled constants and data
   tables are validated against the actual DOS binary before being used by the
   renderer.
4. Add a VM event trace command for `SCRIPT*.COD/VAR/DIC/DEB`.
5. Pick one oracle dialogue target and capture it reproducibly.
6. Replace the current dialogue grouping/compositing path with trace-driven
   rendering for that target scene.
7. Use `accuracy/compare_oracle.py` to normalize DOSBox captures and generated
   MP4 frames to native 320x200, then promote one frame-aligned target to a
   thresholded oracle check.
8. Decompile the ship/procedural-3D path from `0x0A9A:0x0000` and its
   `inspect-bloodprg.presentation_3d_markers` before adding the `wgpu` runtime
   backend.
9. Replace the temporary `ship3d` primitive coverage with a full recovered
   ship/minigame state model once the object list, projection math, and input
   loop are decoded.

## Open Questions

- Which exact DOSBox-X capture path gives pixel-exact 320x200 frames plus
  synchronized audio on this machine?
- Which input route is most reliable for reaching target scenes: DOSBox-X
  `autotype`, X11 key injection, original debug scene selector, or scripted VM
  entry points?
- Which script opcodes must be fully executed for deterministic dialogue export,
  and which can remain as unresolved branch metadata until interactive gameplay
  work begins?
- Is the recovered ship/procedural-3D path a standalone minigame, a navigational
  presentation mode, or both, and which state variables feed its projection and
  object list?
- How closely must the final runtime match original DOS timing on modern systems:
  visual equivalence, frame-exact equivalence, or cycle-sensitive equivalence for
  specific subsystems?
