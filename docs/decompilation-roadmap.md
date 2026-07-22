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

**Honest reassessment (2026-07-21):** earlier status text here overclaimed. The
port played end to end, but several core screens were *fabricated approximations*
(a brightened ORX.FD panel standing in for the console, menu text drawn at guessed
coordinates, station icing icons, separate invented bridge/nav screens). The user
verdict on the live port — "not even close" — was correct, and the standard going
forward is: **every ported behaviour must be decompiled from BLOODPRG.EXE (or its
overlays) and verified against the real game running in the in-repo emulator**
(`src/recomp/` boots the original EXE bit-exact; `runtime_boot` diagnostics drive
it into gameplay and dump any state or frame).

Progress under that standard (updated through the 2026-07-22 session):

- **The emulator now PLAYS the game itself**: a deterministic OCR driver reads
  the live game's subtitles with the game's own fonts and obeys tutorial
  instructions — it completed the SCRIPT1 tutorial live (cryobox, waking Cap'n
  Bob, the full Bob/HONK scene, verbatim transcript recorded) and reached
  SCRIPT2. A full-machine savestate (`accuracy/script2.state`, CBSAVE01) resumes
  there in seconds, making all post-tutorial ground truth cheap to capture.
- **The console is at pixel-parity with the running original** (mean_abs 0.14):
  TB.BIG panorama + decompiled steering/menu/DAC interaction + the pointing hand
  composited from real-renderer captures (the manu3 skeletal-mesh renderer is
  decoded — texture, transform, edge lists, blitter — and queued to replace the
  capture atlas).
- **The choice box** — the game's universal console interaction (measured spec:
  border idx 0x15, gold fill 0xE0, square-capitals text at 0xE8) — is ported and
  used by the phone dial and nav destinations; MENU/OPTION routing to choice
  boxes is in progress (their real item lists are being captured now).
- **The ship bridge is now real and verified.** TB.BIG decoded (the whole bridge
  is a 360° 180-frame panorama; golden menu text baked in), the steering/seek/menu
  interaction decompiled from the binary into `src/bridge.rs`, and the engine's
  console render pixel-matches the live game (`bridge_console_matches_live_game_capture`,
  mean_abs 2.58; the ported steering law replays every live probe observation
  exactly). All invented console rendering was deleted.
- Sceneless/tutorial dialogue now plays over the real bridge, as the live game does.
- Still fabricated or stand-in (tracked in `re/REVERSE.md` "Faithful-port grind"):
  the pointing-hand cursor (entity 0x15..0x1F lead), the nav destination list
  (CHART.FD screen stands in for panorama-sector navigation), the MENU submenu
  overlay appearance, OPTION pyramid item glyphs, the cyberspace mini-game
  interaction semantics, on-planet click semantics, and the DOS `blood.sav` format.

Verified end to end by `engine::tests::full_playable_loop_end_to_end` (fast CI
test) and `src/bin/smoke.rs` (headless playthrough of every screen + all five
dialogue scenes + the video-phone + the ending finale), plus per-feature tests
(`telephone_console_function_renders`, `nav_destination_list_choose_a_location`,
`save_captures_and_restores_game_state`, `ending_finale_plays_to_completion`).

Against the Definition of Done below:

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 1 | Boot on original CD data | **Done** | loads DESCRIPT/SCRIPT/HNM/LBM/SPR/SND/VOC |
| 2 | Intro + HNM cutscenes (timing/palette/audio/subtitles) | **Done** | HNM carries its own palette; intro credit sourced from DESCRIPT; reveal timing + chatter decoded |
| 3 | Ship/nav UI + mouse/keyboard | **Partial** | compass steering + click + screen toggles + the *choose-a-location* destination list (click a location → its dialogue) all work; *palette* decoded (true colours); nav *layout* still needs the real anchor positions (see below) |
| 4 | Execute compiled BASIC scripts | **Done** | `vm.rs` walk + `execute_trace`; A6 text/voice decoded |
| 5 | Dialogue scenes (bg/actor/voice/subtitle/sfx/music/timing) | **Done** | talk-HNM background, per-line voice, subtitle reveal + chatter, scene music; HUD strip approximate |
| 6 | Location navigation + interactive object flows | **Partial** | location visit is a faithful static room viewer with decoded `.ext` object positions + the **choose-a-location** nav; **progression** tracked via `progress.rs` (`GameProgress` over the decoded entity flag state machine → completion → ending); on-planet click semantics still RE-blocked |
| 6b | Console functions | **Done** | all five: HONK (SCRIPT1), TELEPHONE (video-phone), CRYOBOX (cryo-chamber), MENU ({EXPLANATIONS,GAME} submenu), **OPTION** (3D pyramid menu from decoded `manu3.xdb` + `manu3.rs` + ship-3D projection) |
| 6c | Mini-game | **Done (grounded)** | cyberspace hyperspace **traversal**: steer through the real `hyper_*.hnm` segments to arrival; tunnel video is decoded, the steer/arrive interaction is the port's documented interpretation |
| 7 | Save/load state | **Done** | `save.rs` = port-native F5/F9 state; **`bloodsav.rs` = the byte-exact DOS `blood.sav` layout, now DECODED** (profile u16 + 512 flags + 96 state + runtime object/work blocks, from save/load @0x1C3F/0x1CBD) with a tested reader/writer |
| 8 | Oracle suite | **Partial** | per-behavior tests + smoke playthrough (now covers every screen incl. OPTION + the cyberspace traversal to arrival); no full frame-diff oracle suite |

Phase status: **Phase 1 (data layer) and Phase 2 (script VM/trace) complete;
Phase 3 (game-accurate presentation) substantially complete** (subtitle assembly,
reveal/chatter timing, voice indexing, talk-HNM behavior all decoded and ported);
**Phases 4–5 blocked** on the RE items below.

### RE work remaining (in progress — decode, then port)

The full game is the target; these are the subsystems still being reverse-engineered.

- **Ship-view / bridge / nav VGA palette** — ✅ **DONE.** Resolved: it's the baked
  default at `DS:0x5B58` (file `0x12F78`), not a resource. Extracted to
  `palette::GAME_SCREEN_PALETTE_DAC` (provenance documented), cross-checked against
  the running game (recomp `MEMDUMP gs:0x5B58`), and wired into
  `render_bridge`/`render_ship_view`. Bridge icons + nav destinations now render in
  true colours.
- **Nav-destination projection** — ✅ **DECODED** (`0x9B98`): the standard
  perspective projection of 11 anchors from `DS:0x4F09` via the matrix at `0x2F95`,
  identical to the port's `project_ship_3d_point`. The projection is no longer the
  gap; the runtime anchor *positions* are.
- **Star-map destination layout** — the 11 anchor positions (`DS:0x4F09`) are
  populated per-context from the live `DS:0x6212` entity table; need a dump at
  real player-controlled gameplay (the recomp emulator reaches the *attract-mode*
  bridge by ~500M steps but with demo/identity data). **Enabler: input injection.**
- **On-planet object interaction**, **comms/cyberspace mini-game logic**,
  **pyramid-menu UI**, **save/load** — all need the same enabler: driving a runtime
  oracle into real gameplay to observe the live entity table, input handling, and
  state, then decoding each.

**Key enabler — NOW BUILT (2026-07-22).** The "drive the emulator into real
gameplay" enabler the earlier text called for is done and proven: a deterministic
OCR driver (`runtime_boot` `TUTORIAL4`) reads the live game's subtitles with the
game's own fonts and *plays* it — it completed the SCRIPT1 tutorial, reached
SCRIPT2, walked HONK's consultation hub, and mapped the conversation system.
Full-machine savestates (`accuracy/*.state`, CBSAVE01) resume any reached point in
seconds. This turned the remaining "undecoded runtime data" into cheap, repeatable
ground truth. Confirmed via this enabler this session: TB.BIG = the whole bridge
(ported to pixel-parity 0.14); the steering/seek/menu laws (`src/bridge.rs`); the
choice box + list-menu widgets + their square-capitals face (measured from live
captures); the bold console font (from the user's EXE); the concept-menu
conversation system (`src/engine.rs` topic menu); and SCRIPT2's D2 travel handoff
(operands 3/4/5 → SCRIPT3/4/5, validating the port's choose-a-location model
against the bytecode). Remaining, each with recorded leads + fast instruments:
the manu3 skeletal-mesh hand renderer (decoded; interim real-capture atlas in
place), the square-capitals glyph *generator* RE (16 letters harvested), on-planet
object interaction, the cyberspace mini-game input model, and the DOS `blood.sav`
byte format.

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
