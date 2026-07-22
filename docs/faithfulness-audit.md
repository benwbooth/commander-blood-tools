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
1. **Intro clip order/content is hardcoded + unverified** (`engine.rs` load_intro: mind→cliptoot→
   logo_bl). No single DESCRIPT record defines the boot sequence, so it is BOOT-CODE driven. Source
   to consult: the boot/intro sequencer in the disassembly (find the caller that plays the logo/
   cinematic HNMs in order; the DESCRIPT `present`/`microkid`/`hatetv` records are the pieces). RISK:
   wrong logos, wrong order, missing a clip. The oracle can't confirm — intro assets load from
   inside blood.dat (invisible to the file-open trace).
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
