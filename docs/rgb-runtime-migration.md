# RGB runtime migration

The target is RGB/RGBA assets prepared during import/startup, not an indexed
runtime with an RGBA upload at the end. Legacy palette information belongs in
conversion tooling only. Preserve the recovered C's content, event ordering,
timing, geometry, and visible effects; do not preserve VGA hardware state.

## First migrated path: choice UI

- Executable square-cap font bitmaps and normal/hover/pressed text colors are
  converted to immutable RGBA glyph images when `OriginalGameData` loads.
- Contact, option, dialogue-word, and ship-target widgets use those images.
  Their backgrounds and opening/closing rectangles use an independent RGBA
  dimming layer, without modifying the scene framebuffer or its colors.
- The GPU composes this layer over the current artwork/video/bridge and below
  MANU3. Darkening blends RGB channel values in sRGB space. It does not search
  for a nearest palette entry, so original quantization artifacts are not kept.
- UI is rebuilt once per game frame and retained across render-only refreshes.
  Modern refresh/interpolation must not advance VM, movie, or audio state.
- Trace `rgb_ui` metrics and ship-target row pixel/color metrics now describe
  the RGBA overlay. Legacy video and palette trace fields are not final-output
  evidence.

The source contract is `func_008428_list_widget_layout_unified.c`: darken the
underlying image, draw the square-cap labels, then handle the pointer. The
dialogue word list uses center x=225; the contact list uses x=100. Both center
vertically at y=100. `func_008963_presentation_ready_gate.c` supplies the word
list's four-step collapsed-to-open rectangle animation.

## DESCRIPT sequence captions

The bridge `present` sequence in `re/descript/DESCRIPT.descript` supplies the
CLIPTOOT captions: the CRYO credit at frame 1, `Commander BLOOD  V 1.0` at
frame 30, and an empty cue at frame 100. These are not MIND video pixels.

The BIOS 8x8 font and authored subtitle color are now imported to immutable
RGBA glyphs at startup. The recovered `list_walk_f18` planner still advances
only when `screen_mode_update` reports a presented resource frame, including
its draw-before-advance order. The resulting caption is retained separately
and stamped onto the RGB UI every game frame, so decoder stalls and legacy
front/back-page replacement cannot erase it. Render-only refreshes reuse it
without advancing the cue clock. Blank cues, record/video replacement,
cancellation, closing, and sequence completion clear it.

This intentionally preserves authored content and timing, not transient DOS
display-page artifacts. Dialogue subtitles are covered below; panel effects
remain to be migrated.

## Location-panel artwork

The 42 world-artwork table entries now resolve their first sprite frames to
immutable RGBA images at startup. The native resource cache remains only as
geometry metadata for the location-panel entity. Loading that metadata no
longer publishes the planet's source colors into the game palette.

`func_009083_location_info_panel_dispatch.c` still controls opening, closing,
text, and draw order; `entity_draw_full` controls its position and extent. The
RGB sampler preserves the recovered scaler's truncated 16.16 coordinates and
transparent-zero coverage. Main-font styles 238 and 254 are imported once;
panel text preserves the C routine's space advance, skipped characters, and
draw-width accounting. The rectangle dims RGB directly instead of selecting
nearest palette colors. The adjacent transition entity remains a legacy
background operation; this is not a whole-navigation RGB migration.

## Travel ownership fixes

The manual trace `output/fidelity/manual-playtest-63QX9M/frames.jsonl` showed
line 6 decoding at frames 2574-2662 while the final indexed display hash stayed
constant. Rust's bridge preparation copied the bridge base onto the movie's
primary page after the camera FSM dispatched it. C's
`func_0078d0_presentation_mode_dispatch.c` only handles hover, not that copy.
Modern bridge layers may still be prepared, but an active camera/ship movie
now retains its primary page, including the previous pixels needed by deltas.

The same trace retained line 6 after camera return and replayed it at frame
3412 on Orxx. The camera adapter now preserves the scene dispatcher's completed
line clear instead of overwriting it with the pre-callback value.

At frame 4125 the planet target list was open, with scene dispatch blocked,
but a Rust full-screen-video latch suppressed all hand geometry. C's
`func_001610_manu3_hand_frame_dispatch.c` explicitly permits the hand during
that blocked-dispatch menu. The target menu now overrides video occlusion;
travel playback and subsequent full-screen dialogue retain their own gates.

Verification for these changes:

- Library suite: 881 passed, five optional tests ignored. The optional
  `real_services_run_the_complete_available_startup_slice` was then explicitly
  run in isolated SDL/wgpu and passed, including page preservation across bridge
  preparation and the completed camera line clear.
- All 42 world-art entries were compared pixel-for-pixel with the recovered
  scaled blitter at five sizes/positions, including clipping. Loading panel
  metadata under an adversarial all-red game palette leaves that palette alone.
  Main-font pixels/widths and RGB dimming also have focused regression tests.
- The complete lever/navigation scenario passed with 66 distinct display pages
  over 89 hyperspace game frames. The live trace, RGB planet-panel screenshot,
  and two visibly different hyperspace screenshots are retained in
  `output/fidelity/production-load-pterra-navigation.jsonl-1788585272963245012-1054908-0/`.
- The full Pterra ship-navigation scenario also passed: all 501 recorded target
  menu frames submitted hand geometry, and selecting Pterra reached Scruter's
  identity-code choices. Its trace and `planet-menu-hand.png` are retained in
  `output/fidelity/production-load-pterra-ship-navigation.jsonl-1788585490096845185-1061521-0/`.
- These checks exercise Rust production rendering with original assets and
  recovered-C contracts. They are not a new matched-frame original-DOS capture
  or proof of whole-game parity.

## Remaining work (not migrated)

| Owner | Indexed dependency still present | Required replacement |
| --- | --- | --- |
| `runtime/state.rs`, `native/bloodprg/bridge_sprite*` | Shared front/back surfaces, sprite remaps and cached source indices | Imported RGBA sprites/backgrounds, explicit blend modes and dirty rectangles |
| `bridge_render.rs`, bridge scene inputs | Panorama/actor indices resolved with bridge colors | RGB panorama frames and sprite layers, direct RGB vertex/material colors |
| `runtime/video.rs`, `presentation_player.rs` | Live HNM decoding, index-backed retained pages and inherited color state | Context-complete imported RGB video layers plus coverage and timing metadata |
| `runtime/ship_navigation.rs`, `ship_hud.rs` | Legacy navigation dimming/depth-band and scene preparation | RGB backgrounds, masks, depth transitions, and explicit composition |
| Panel effects in `presentation_screen.rs` | Panel rectangle/noise effects still write indexed pixels | RGB noise/fades; the text and channel masks are already independent RGB overlays |
| `runtime/palette_transition.rs` and screen effects | Color-range operations rather than visual layer effects | Explicit RGB layer fades, with the same C timing and ownership |

The existing `video-v1` cache is not a complete solution: production
`RuntimePresentationStream` still opens HNM sources. Its derivative metadata
also records context-dependent colors and an indexed companion stream. Moving
that companion stream into playback would recreate the problem. Resolve each
required scene/clip context at import, retain transparent/unwritten coverage,
and reject unaccounted context dependencies rather than guessing colors.

Migrate these owners in coherent groups with real call sites, not replacement
stubs. A decoder/library may retain indexed internals for conversion and oracle
tests; those internals must not become the production scene model.

## Verification and open reports

- Compare imported glyph coverage/advances against the independent C font
  raster tests. Changing game colors after import must leave RGBA UI unchanged.
- GPU readback must check arbitrary colored bases, opaque glyph colors,
  translucent dimming, letterboxing/resizing, and cleared UI. Compare actual
  composed pixels, not merely an upload hash or a nonempty draw list.
- Run existing production phone/Bob/Pterra scenarios to protect interaction
  ordering, but label their coverage honestly: scripted pacing bypasses the
  live scheduler, and state traces do not establish full visual parity.
- User reports of missing hyperspace and scene corruption are not resolved by
  these UI migrations. They require aligned RGB captures through their complete
  transitions. Sequence-caption persistence has a dedicated regression scenario;
  this is separate from a matched original-DOS video comparison.
- C6's `nav_actor_0_busy` is the slot-4 flag byte, despite the misleading name.
  Rechecking its table binding rejected an earlier suggestion to use slot 5.
  The added actor regression does not claim to fix Pterra.

### Verification recorded 2026-09-04

- Library: 865 passed, five explicitly ignored. The original-data glyph test
  was then run explicitly and passed for all 86 supported square-cap characters;
  the other four optional tests were not rerun in this slice.
- The 14 renderer tests include real GPU readback for RGB blending and resize.
- Production Bob/phone and Pterra-navigation scenarios passed. Retained runs:
  `output/fidelity/production-bob-first-contact.jsonl-1788566732154432925-4073436-0/`
  and `output/fidelity/production-load-pterra-ship-navigation.jsonl-1788566864736719701-4075665-0/`.
- Isolated Xvfb screenshots under `output/fidelity/rgb-ui-pterra-PfVYYe/`
  show colored scene content retained underneath darkened menu backgrounds.
  This is visual inspection, not a matched-frame original-DOS comparison.
- `frame-180.png` also shows stale green status text overlapping the white
  Scruter prompt. Those status/subtitle pixels are still on the indexed path;
  this observed defect remains open. The RGB choice panel itself is independent.
- The RGB UI hashes remain stable while Bob/Scruter idle video frames advance.
  Production tests use scripted pacing; live monitor-refresh interpolation was
  checked through clock/animation unit tests, not a new manual playtest.

### Sequence-caption follow-up, 2026-09-04

- Library: 868 passed, five ignored. New tests check all BIOS glyph bytes
  against the recovered font rasterizer, caption persistence over 120 UI
  refreshes without cue advancement, and blank/exit/replacement clearing.
- `production_runtime_retains_the_intro_caption_until_its_blank_cue` passed
  twice with the actual game assets in an isolated SDL/wgpu display. The first
  run retained 426 opaque title pixels with an identical RGBA hash across 12
  checkpoints at sequence indices 35 through 99; frame 108 had zero caption
  pixels, following the authored empty cue at frame 100.
- Captures and traces from the final implementation are retained at
  `output/fidelity/production-intro-caption.jsonl-1788568308106201159-35296-0/`.
  `title-1.png` through `title-5.png` show the title over changing video frames.
  The exact-white mask in the title rectangle (704x39 at 288,528 in the
  1280x960 capture) has the same image signature in all five captures:
  `22d3246cec7641b4c66b72b81d9022a833a44dfd9ec87a67612a7dd8ba1a4475`.
- These are Rust production captures and scripted timing checks, not matched
  original-DOS captures or proof of whole-game visual parity. The user's live
  game session was not restarted or interacted with.

### Intermittent-frame and camera follow-up, 2026-09-04

The user still observed one-frame caption gaps after that checkpoint-only
verification. The caption is now composed at the frame output boundary as well
as before diagnostic snapshots, even when the bridge coordinator bypasses the
panel update. A missing queue clock no longer becomes frame zero; a planner
`Waiting` result retains the old cue, matching C's no-draw return. Only a real
replacement/blank cue or the presentation owner's termination removes it.

Pointer steering also consults the startup and panel owners directly instead
of depending solely on transient shared UI flags. Automatic scripted seeks
remain permitted, including the initial turn to the TV. The camera scenario
drives both screen edges and checks that free steering returns after a click
closes the intro. The subtitle scenario now asserts that cancellation actually
stops the video, not merely that its already-blank cue has no pixels.

`--live-trace PATH.jsonl` records flushed semantic snapshots at every ordinary
game frame and blocking-video boundary, with monotonic frame/time metadata. It
does not enable scripted input or substitute the scenario clock. It also works
alongside `--scenario ... --trace ...` with separate output paths. Production
scenario tests now retain `frames.jsonl` in addition to action checkpoints.
Tracing adds CPU and file-I/O overhead and should be enabled for diagnosis,
not treated as a performance benchmark.

- Library after these changes: 875 passed, five ignored.
- Real-time SDL/wgpu run (no scenario driver):
  `output/fidelity/intro-realtime-OPATlw/`. Its 117 title-window game frames
  retained the same caption and composed-UI hash; all intro headings were 90.
  There were 27 nonzero mouse-motion frames, and all 27 captured screen images
  had identical exact-white title masks. This samples displayed images rather
  than claiming an exhaustive capture of every monitor refresh.
- That isolated real-time run entered pause through an injected P key, logged
  33 paused frames, and resumed through Escape. These inputs were sent only to
  a dedicated Xvfb display, not the user's desktop.
- Pterra navigation passed on the pre-follow-up build, with artifacts at
  `output/fidelity/production-load-pterra-ship-navigation.jsonl-1788569034301571011-149123-0/`.
  The user's reported Pterra freeze is not reproduced or declared fixed.
  `PAUSE` is the separate pause HUD, not a destination-list label; the trace is
  needed to determine why that state appeared during the user's run.
- Binary recheck rejected proposed navigation-list changes: original DS:2537
  is the single `GO` word plus sentinel; DS:253B is an empty trigger list.
  Those Rust lists already match the shipped executable. Pterra's inline VAR
  name also matches the directory label. No speculative menu changes were made.
- Final updated-build Pterra regression passed with continuous tracing at
  `output/fidelity/production-load-pterra-ship-navigation.jsonl-1788570158559742277-182269-0/`:
  4,117 recorded frames, 799 with `PL\\pterra10.hnm` active, zero paused frames.
  This does not reproduce the user's manual freeze. Intro camera/click-release
  and caption/blank/click-cancellation regressions also passed separately.

## Channel and dialogue overlays

The next manual run exposed a different text path from the DESCRIPT title:
the selected channel number, Honk's character-reveal subtitle, and Bob's
word-reveal dialogue still wrote to the indexed front page. An HNM EOF retains
its RGB image during scene dispatch, before the frame-tail text draw. The
retained-page refresh deliberately does not read the front page. Thus correct
indexed glyphs could coexist with missing displayed glyphs between looping
clips; indexed-buffer audits could not detect this.

These three UI paths now use precolored glyph/mask assets imported at startup,
then compose through the independent RGB UI texture. The native subtitle and
word planners still own timing, wrapping, reveal colors, completion, and hold
state. The channel mask is retained across decoder waits and skipped panel
updates; scene cancellation clears it. Subtitle corner darkening uses alpha
over the displayed RGB background instead of an indexed nearest-color remap.

References:

- `func_0079e5_screen_mode_update.c`: conditional frame-ready channel draw.
- `func_007cb4_selected_mask_overlay.c`: six channel masks and placement.
- `func_0093f5_subtitle_reveal_pump.c`: subtitle phases and draw-after-update.
- `func_003630_subtitle_reveal_draw_wrapper.c`: progressive glyph colors.
- Native `menu_reveal.rs` and `draw_planar_dialogue_text`: inline dialogue
  placements, signed advances, and exact main-font coverage.

The native indexed rasterizers remain independent reference implementations,
not production text destinations. Their audits now compare glyphs to the RGB
UI texture; channel-mask coverage is also exposed in `--live-trace`. The intro
and Bob/Honk scenarios assert pixel coverage on every logged game frame,
including frames with retained video ownership and no active stream. The GPU
UI test also reads back actual imported dialogue glyphs and the channel mask
over both artwork and bridge-like backgrounds. These are targeted regressions,
not a claim that every game path is accurate or palette-free yet.

Verification for this slice (2026-09-04):

- Library: 877 passed, five ignored, including actual-glyph GPU readback.
- Intro continuous-frame regression passed; 543 frames recorded, 375 with a
  visible channel mask, no channel-coverage mismatches. Artifacts:
  `output/fidelity/production-intro-caption.jsonl-1788582962196615119-754960-0/`.
- Bob/Honk regression passed through hang-up and restored bridge control.
  It checked 498 inline-dialogue frames, 120 progressive-subtitle frames, and
  159 text-bearing frames without an active video stream; no glyph mismatches.
  Artifacts:
  `output/fidelity/production-bob-first-contact.jsonl-1788582997799985026-771730-0/`.
- The SDL X11 window exposes the approved 256 by 256 blue-hand icon and
  `commander-blood` application class. Matching local desktop/icon entries were
  installed and KDE's application cache refreshed. Nix installation metadata
  was evaluated, but the full Nix package was not rebuilt in this slice.
