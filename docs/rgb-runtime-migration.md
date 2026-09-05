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
display-page artifacts. It does not migrate dialogue subtitles or panel effects.

## Remaining work (not migrated)

| Owner | Indexed dependency still present | Required replacement |
| --- | --- | --- |
| `runtime/state.rs`, `native/bloodprg/bridge_sprite*` | Shared front/back surfaces, sprite remaps and cached source indices | Imported RGBA sprites/backgrounds, explicit blend modes and dirty rectangles |
| `bridge_render.rs`, bridge scene inputs | Panorama/actor indices resolved with bridge colors | RGB panorama frames and sprite layers, direct RGB vertex/material colors |
| `runtime/video.rs`, `presentation_player.rs` | Live HNM decoding, index-backed retained pages and inherited color state | Context-complete imported RGB video layers plus coverage and timing metadata |
| `runtime/ship_navigation.rs`, `ship_hud.rs` | Legacy navigation dimming/depth-band and scene preparation | RGB backgrounds, masks, depth transitions, and explicit composition |
| `runtime/subtitles.rs`, panel effects in `presentation_screen.rs` | Dialogue subtitle raster and panel effects write indexed pixels | Imported RGB dialogue fonts, retained dialogue overlays, RGB noise/fades |
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
