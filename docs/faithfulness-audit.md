# Commander Blood port — faithfulness audit

Systematic audit of the clean-room port (`src/engine.rs`, `src/main.rs`) against the game's own
ground truth (DESCRIPT/scripts = DATA; `BLOODPRG.EXE` disassembly = CODE). Method:
[re/REVERSE.md] + memory `commander-blood-faithfulness-method`. Ground truth is the game's data and
assembly, NOT self-referential tests (those green-lit the intro-music bug). Each finding names the
source to consult so a fix is grounded, not guessed. Status: **the port is progress, not faithful
everywhere.** Priority = how visible/audible + how confidently fixable.

## Fixed (data-grounded)
- **[DONE] Intro music timing** (commit 965). Bug: `blintr.voc` played from intro frame 0 (over
  the MINDSCAPE/Microfolie's logos). Truth (DESCRIPT `present` record): Music `blintr.voc` is bound
  to SequenceHnm `cliptoot.hnm` — the logos are SILENT, music starts with the cinematic. Fix: the
  engine carries per-clip music from the record; the logo reel is silent. Test:
  `intro_music_silent_over_logos_starts_with_cinematic`.

- **[DONE, mechanism] In-game cutscene player** (commit pending). Gap found this pass: the port
  parsed `RecordKind::Sequence` but had NO general player — only the boot intro + dialogue scenes
  ran, so the in-game cutscenes (IZWAL-TV `microkid`, `hatetv`, the `maledict` curse, …) never
  played with their data-defined music/tick-subtitles/HNMs. Fix: `EngineState::start_descript_
  cutscene(record, assets)` plays any Sequence record faithfully from its own data, reusing the
  intro path. Test: `descript_sequence_cutscene_plays_with_its_data` (maledict → maledict.hnm +
  klings.voc + the "CURSED" subtitles). REMAINING: wire the triggers (which script event fires
  each cutscene) — needs the script/asm event handler; the faithful PLAYER now exists.
- **[CLEARED] Intro/HNM subtitle tick scale** (risk #2, partial). The DESCRIPT subtitle ticks are
  HNM frame numbers (microkid→120, hatetv→200, cliptoot 1258 frames), so `FRAMES_PER_TICK = 1`
  (frame == tick) is faithful. The absolute HNM playback fps vs the game's tick rate is still worth
  confirming against the asm, but the cue TIMING relative to the video is correct.

## Audited FAITHFUL (decoded from the binary/data — no change needed)
- **Scene background music** — `extract::script_background_music` derives each scene's music from
  the DESCRIPT/character-context data. ✓
- **Per-line voice clips** — `vm::text_selector_voice_clip_index` is decoded from the A6 handler
  (file 0x661E/0x668D): one-based selector → clip (selector−1) when it requests voice. ✓
- **Subtitle chatter** — `sn/tb.snd` clip 0 fired once per fully-revealed line, decoded @0x94BA. ✓
- **Subtitle reveal rate** — `subtitle_reveal_chars_per_second` from the binary text-speed step. ✓
- **Intro credit cues** — timed by the DESCRIPT `present` subtitle ticks (data-driven). ✓ (but see
  the tick-scale risk below.)

## Open faithfulness risks (prioritized) — each needs its named source
1. **Intro STRUCTURE is wrong — DIAGNOSED from ground-truth captures** (`accuracy/captures/frame_*`,
   real game @1s intervals). The real intro is SHORT and the credits overlay the LIVE console:
     - 1s: MINDSCAPE logo (full screen) · 2s: Microfolie's logo · ~3-4s: space cinematic (pilot
       over a planet) — this whole run == the port's `mind.hnm` clip, which IS correct.
     - ~6s: the CONSOLE/BRIDGE is already active (3D pyramid menu + eye orb + a character over a
       seascape) with the subtitle "CRYO Interactive Entertainment 1995" OVERLAID on it.
     - ~9s: interactive gameplay (bridge + a talking character).
   So the "CRYO 1995" / "Commander BLOOD V 1.0" credits are SUBTITLES over the already-running
   console (the `present` record's subtitles overlaid on the bridge), NOT a separate full-screen
   cinematic — and the intro→gameplay is ~8 s. The port INSTEAD plays `cliptoot.hnm` as a separate
   ~69 s full-screen cinematic clip (1258 frames @ ~18 fps). FIX (grounded, but a restructure):
   after `mind.hnm`, go straight to the bridge/console and overlay the `present` credits there for
   their tick span, rather than playing cliptoot standalone. Confirm cliptoot's role by decoding a
   few of its frames (is it the seascape bridge background, or a distinct clip?). This is the top
   visible intro bug; needs care (it reshapes the intro→console handoff in main.rs/engine.rs).
   PROGRESS (commits 970/971): (a) the credit clip now ends with its cues (~124 frames ≈7s), not
   the full 1258 (~69s); (b) the spurious `logo_bl.hnm` fire-title clip was REMOVED — frame_07
   proves "Commander BLOOD V 1.0" is a SUBTITLE over the console (the present tick-30 cue), not a
   separate title screen. REMAINING: render the CRYO/title credits over the CONSOLE (as the real
   game does) rather than over the bare cliptoot clip — the last piece of the intro structure.
   DEEPER FINDING (frames 6-9): the "console" during the credits is the **SCRIPT1 console tutorial
   already running** — CREW characters (alien/mutant) on the console screen with the 3D pyramid
   menu + eye orb, credits overlaid on the first ~2s. The port instead goes post-intro to the
   BRIDGE PANORAMA (tb.big, purple organic) — a different screen. So the faithful post-intro state
   is SCRIPT1's console-tutorial view (crew talk-HNMs + pyramid menu), not the bridge panorama;
   the CRYO/title credits overlay its opening. This couples the intro fix to the console/SCRIPT1
   flow (risk #3) — a careful restructure, sourced to the captures + the SCRIPT1 console tutorial.
   PRECISE STRUCTURE (frames 6-9): the SCRIPT1 tutorial plays ON the console COMPOSITE — the crew
   talk-HNM (main view) + the 3D pyramid menu + eye orb (bottom) + the button panel (left) are
   shown TOGETHER, with the credits overlaid on the opening. The port renders these as SEPARATE
   screens: `render_bridge` (tb.big panorama + text menu) vs SCRIPT1 dialogue (crew HNM +
   subtitles). Two coupled changes needed: (a) FLOW — auto-play SCRIPT1 after the intro (matching
   the real game + the port's own line-348 comment; the code at main.rs:887 instead lands on the
   bridge and requires a HONK click), WITHOUT orphaning the bridge/console-function hub (HONK/
   TELEPHONE/… live on `bridge_active`, only re-entered post-intro + post-cyberspace); (b) RENDER —
   a console COMPOSITE view (crew HNM + pyramid menu + buttons) rather than bridge-or-dialogue.
   This is architecture work (a hub/composite model), not a one-line toggle — do it deliberately,
   verified frame-by-frame vs captures 1-9, not guessed.
   OPEN UNCERTAINTY (must resolve before building the composite — do NOT guess): frames 6-9 show a
   GRAY 3D-pyramid FLOOR + central eye-orb + a crew-in-viewscreen (top ~75%). But `render_bridge`
   draws the tb.big panorama + a GOLDEN choice-box menu, which memory records as PIXEL-VERIFIED vs
   the live game ([[commander-blood-bridge-panorama]], accuracy/captures/bridge/*). So the frames-
   6-9 view is a DIFFERENT console state/screen than the verified golden-menu bridge — not simply
   "the port is wrong". Resolve FIRST (deeper RE or interactive capture): is the gray-pyramid floor
   the OPTION/manu3 mesh, a decorative console floor, or the default nav console? How does the
   crew-in-viewscreen relate to the bridge vs the dialogue talk-HNM? Are 6-9 the intro-tutorial
   state specifically? Only after this is grounded can the composite be built faithfully. Sourced:
   render_bridge/render_option_menu (engine.rs) + captures 6-9 + the bridge/ capture set.
   RESOLVED (compared captures): frames 6-9 ARE a distinct screen from the bridge. The verified
   bridge (accuracy/captures/bridge/choice_box_bob_morlock.ppm, script2_first_frame.ppm) is the
   PURPLE ORGANIC PANORAMA + GOLDEN HONK/TELEPHONE menu + orange orb + blue hand — and SCRIPT2
   opens on it. Frames 6-9 (SCRIPT1 intro/tutorial) are a GRAY 3D-pyramid floor + eye-orb + a crew
   member in the viewscreen — no purple panorama, no golden menu. So the game has (≥)2 console
   screens: the bridge (nav/SCRIPT2+) and the SCRIPT1-tutorial pyramid-console. The port renders
   the bridge for both and lacks the tutorial console. STILL TO GROUND before building: exactly
   what the gray-pyramid floor is (OPTION/manu3 mesh? a distinct tutorial console?) and its draw
   path in the asm — do that RE first, then build; do not guess the composite.
2. **Intro/HNM playback RATE** (`INTRO_CREDIT_FRAMES_PER_TICK = 1`, one HNM frame per game step).
   A guess flagged "calibratable". Source: the HNM player's frame-advance timing in the asm (ticks
   per HNM frame) + the DESCRIPT tick unit. RISK: intro plays too fast/slow; subtitles mistimed.
3. **Top-level loop is a clean-room state machine, not an asm translation** (`engine.rs` step(),
   labelled a "faithful control-flow skeleton; for now"). The real 0x0FFB loop
   (`main_loop_is_presentation_coordinator`) does input-dispatch → HUD → per-frame update →
   busy-gate → profile-dispatch. RISK: flow/ordering divergences. Source: 0x0FFB + the labelled
   sub-calls (already partly mapped in labels.csv).
4. **Cyberspace mini-game interaction** (steer + arrive) is grounded interpretation, not decoded
   (`engine.rs` ~1290). Source: the hyper_*.hnm traversal + input handling in the asm.
5. **On-planet per-object CLICK semantics** undecoded (entity.rs has the flag state machine but not
   the per-object actions). Source: the on-planet interaction handler in the asm.
6. **DOS blood.sav byte format** undecoded (port has its own save; only matters for original-save
   interop). Low priority.

## Verification tooling built for this
- `runtime_boot INTROTRACE` (STEPS env) — real-game boot timeline: opened_files + SB audio-playback
  starts (`Runtime.sb_play_log`), step-stamped. LIMITS: blood.dat-internal assets don't show as
  opens; the interpreter doesn't reproduce intro audio — so it's a scene-FLOW oracle, not an
  audio-timing oracle. For audio timing, trust the DESCRIPT data.
- `re/tools/hnm_chunks.py` — dump HNM chunk tags (confirmed intro HNMs carry no embedded audio).
