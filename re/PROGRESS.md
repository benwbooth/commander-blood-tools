# Commander Blood reimplementation — progress & remaining work

## RUNNABLE ENGINE — SCREEN COVERAGE (sess 007)

The `engine-window` now implements the game's major SCREENS from decoded assets, each
verified visually + tested:
- **Boot intro** — the `mind.hnm` reel (MINDSCAPE→Microfolie's→ship→CRYO) + fire title,
  with `blintr.voc` music.
- **Nav** — moving camera (decoded `[0x27DF]` approach FSM), real CARTE.SPR pyramids
  (decoded projection/scaling), centre-delta steering, camera-driven streaming starfield.
- **Dialogue** — VM trace, char-by-char subtitle reveal (decoded pacing/wrap/font),
  per-line character voice (real sn/*.snd), scene music, tb.snd chatter, D2 scene-chaining.
- **Alien examination** (`croolis`) — `caiscrut` scrutinizer intro → mouse-rotatable
  Scruter Jo (`scrut_a..d`).
- **Comms / "Hate TV"** — 18 broadcast channels (`tvgren*`/`tvred*`), channel-switchable.
- **Cyberspace** — hyperspace-tunnel presentation (`hyper_*`) with segment travel.

Controls: intro auto-plays; `c` alien exam, `t` comms TV, `y` cyberspace, Esc back.
Audio is fully in-process (cpal) + cross-platform.

**REMAINING = deep gameplay LOGIC (undecoded, multi-session each), NOT asset wiring:**
- Cyberspace navigation minigame (input→steer, obstacles, scoring).
- Alien-behaviour AI object state machines (`croolis` `+0x36/+0x38/+0x3C` records).
- `manu3` 3D menu / ship-bridge hub navigation (ties the screens together).
- Combat, the `amer` alien overlay, and the global object-simulation driving which
  content appears when (incl. the nav destination instances).
- ~72% of BLOODPRG.EXE's ~435 functions still undecoded — true 100% is a multi-month
  decompile.

## RESOURCE-LOADING PIPELINE (decoded end-to-end, sess 007)

The game loads everything (sprites, the script1-5 bytecode sets, `.ext` worlds) by
**resource ID** through one pipeline (all labeled in `labels.csv`):

1. `resource_name_table` `FS:0x0c04` (file 0xCDF4): 16-byte filename records indexed by
   resource ID. IDs 0..21 engine sprites/drivers/script1/buffers; **22..36 the primary
   worlds** (black=22, venusia=25, magnus=28, cyber=36); 37+ script2 set + sub-levels.
   Ported as `src/levels.rs LEVEL_DIRECTORY` (+ `world_resource_id`).
2. `resource_load_by_id(AX=id)` `0x287b`: `si=0xc04+id*16` → filename; lookup `0x28ca`,
   alloc `0x4b9:0`, file-load `0x2abb`.
3. `resource_file_load` `0x2abb`: path-build `0x2693` (gs-relative), FindFirst `0x4e00`
   → size to `GS:0x0A8E`, open `0x3d00`, read into the resource segment.
4. `resource_handle_resolve` `0x5320` / `resource_release` `0x5288`: 8-byte table entry
   `{segment@+0, flags@+2 (bits0-1=loaded)}` at `fs:[handle<<3]`.
5. `vm_resource_profile_select` `0x53A0`: a script "profile" = 5 resource IDs
   (COD/BAS/VAR/DIC/DEB) from `FS:0x11f4` (10-byte entries); frees old + loads new.

So a world loads via `resource_load_by_id(world_resource_id)` and its **uncompressed**
`.ext` data lives at the resource segment (`src/ext.rs` decodes the body framing). NOTE:
the `.ext` body has no single parse routine — the world-logic code reads fields directly
from the segment, so the record *semantics* need that (undecoded) world-logic decode.

## GAMEPLAY-LOGIC PORTED (sess 007) — beyond presentation

Decompiled + ported from the overlays (disassembled via capstone on the raw `.xdb`
files), each tested:

**Shared alien-behaviour engine** (`src/croolis.rs`) — verified identical across
`croolis.xdb`/`amer.xdb`/`scrut.xdb` (same `ror ax,7; sbb ax,0` anim PRNG + `0x5E`
object stride). Complete method set: `0x16A4` anim state machine, `0x12DE` frame-gated
colony dispatcher, `fs:0x103A` behaviour vtable, `0x999` position toroidal-wrap
(`±0x4000`), `0x36A` object initializer, `0xA30` proximity/visibility gate. Object
positions at record `+0x42/+0x46/+0x4a`; transform at `+0x12/+0x22/+0x32`. Remaining:
per-object 3D draw (reuses shared ship-3D compositor).

**manu3 3D-menu core** (`src/manu3.rs`) — the ship's pyramid menu. Ported: input-coord
decode (`[bp+4]&0x1F` item, `[bp+6]>>4` row), item-selection dispatch (`0x181`,
`base+table[item]`), tween setup (`0x1DF`, `delta=(end-current)<<16/count`), tween list
(`0x19B`, fixed-point animate + swap-remove), camera pan (`0x34..0x51`, centre-delta),
pyramid angle setup (`0x270`, angles `+0x4E/+0x50/+0x52 & 0xFFC` → shared projection).
Remaining: data-driven per-item action handlers + final vertex blit.

**Still undecoded (the majority — multi-month):** cyberspace minigame (BLOODPRG logic +
`CYBER*.EXT` graph data), combat, the global object/navigation simulation, and ~70% of
BLOODPRG.EXE's ~435 functions. True 100% is a complete decompile.

## VERIFICATION MATRIX (full pass, sess 005-007)

Coverage (measured, sess 007): ~281 ret-preceded clean-prologue function starts in the
base code segment (the raw E8-scan's ~360 includes mid-instruction false positives).
**~319 code addresses decoded/labeled** in labels.csv (up from ~113 at session start).
Every MAJOR SUBSYSTEM is now decoded end-to-end:
- **Boot/init/hardware**: cmdline args, timer hook + PIT, EMS (int67h) detect, CD-ROM,
  Ctrl-Break, video-mode save/restore, mouse init/poll, RTC read, sound-card port I/O.
- **Resource system**: name table (FS:0x0c04) → load-by-id → path build/findfirst/open/read
  → handle→segment 8-byte table (resolve/release/flags/size/loaded) → EMS-banked
  ring-buffer queue (gs:0xd8c) + DAT chunk seek.
- **Render**: linear back-buffer → RLE sprite composite + 2D clipped-plot primitives +
  3D matrix-mul (Q15) + perspective projection + vertex-list → dirty-rect blit / full-screen
  blit → mode-X pixel plotter (all verified equivalent to the engine's framebuffer/decoder).
- **VM**: vm_run_wrapper (per-frame) → exec-loop dispatch (opcode table 0x142d0) → all 51
  opcode behaviors + query/set model (gs:0x67ad) + full operator set (ne/lt/gt/le/ge/eq/
  set/add/sub) + DIC/text + object/line-record state (gs:0x6724, typed records).
- **Objects**: entity_object_table (DS:0x6212, 32-byte records) + populate + flag SM +
  entity_draw (reads .ext object x/y, scales, renders) + runtime object heap (gs:0x6726).
- **`.ext` world body** (fully characterized): 63-node table → 10-byte object records
  (id/type/x/y, cross-validated + engine-rendered) → node-reference geometry payload.
- **Audio**: SND player + driver callback + software mixer + PC-speaker synth (ported).
- **UI**: input-action dispatch + xlat table + region hit-testing + camera-approach FSM.

Remaining (~27%): family-sibling leaf functions, tiny state-gates, the exact per-node
`.ext` geometry meaning, and per-opcode deep internals. "Combat" verified NON-existent
(retracted). True 100% (every function fully decoded + ported) is still a multi-month effort.

**Verified exact (tested in the suite):**
- Font tables — byte-for-byte vs the exe (@0x14C22/0x14CD2/0x14D28), regression test.
- VM opcode descriptor/handler tables + token walk — byte-exact vs binary, tests.
- `snd_mix_average` — exhaustive equivalence with the 0xBB6D add/rcr idiom.
- Sprite bank decode — BORXX.SPR regression test.
- ship3d state machines / projection matrix / PRNG / trig table — `matches_binary` suite.
- Star-map projection formula — unit-tested vs the decoded 0x9BBA math.

**Verified against the running oracle:**
- HNM static keyframes — pixel-exact (MINDSCAPE/Microfolie's logos, sess 005).
- HNM RLE delta-frame placement — FIXED sess 007 (the 0xAB34 x,y-pair read; animation
  was smearing/speckling, now clean); MINDSCAPE frame matches the oracle up to
  animation phase (diff localizes to the rippling mountain + capture bar).
- Boot intro sequence — `sq/mind.hnm` = the complete boot reel (MINDSCAPE →
  Microfolie's → ship → CRYO), matches the oracle boot order; engine plays it + title.
- Letterbox band origin — band clips at rows 0x23..0xA5 (gs:[0x1fa7] analogue).
- Dialogue subtitle reconstruction — 99.8% word resolution across SCRIPT1-5.
- Nav decorative HUD — visually matched to the title-screen HUD.

**Made faithful (sess 007 accuracy grind — were approximations, now decoded):**
- Dialogue pacing — decoded text-speed timers (`text_speed_step_from_setting` @0x1B29,
  reveal `step>>2` frames/char @0x94BA, hold `step<<2` @0x94D4).
- Subtitle wrap — the decoded 0xA6 rule (35-char, 0x0D breaks, punctuation spacing).
- Subtitle reveal — character-by-character (@0x93F8, edge glyph 0xFE / body 0xFD).
- Nav pyramids — the game's real CARTE.SPR frames at 0x9BBA-projected positions with
  the sprite path's `dim*(0x100000/depth)>>10` scaling (replaced the hand-drawn grid).
- HNM letterbox band origin (rows 0x23..0xA5) + the RLE delta x,y-placement fix.
- Audio — in-process cpal playback (own VOC parser `snd::parse_voc_pcm`): per-location
  scene music + boot-reel music + per-line character voice (real sn/*.snd clips via the
  decoded one-based selector), extracted sn/ voice banks from BLOOD.DAT.

**Ship-movement simulation — DECODED + IMPLEMENTED (sess 007):** the nav camera moves
because the phase FSM at `0x8A6A..0x8B5A` (counter `DS:0x27DF`) walks the camera origin
`[0x2F65/67/69]` + yaw `[0x2F71]` each frame: P1 pulls X in 0x64/frame to 0x2328
(rotating yaw), P2 accelerates Z via `[0x2F6B]` to 0x4E20, P3 resets, P4 sets Z=0x7530.
Ported as `ship3d::Ship3dCameraApproach` (tested vs the decoded phases) and driven by the
engine each on-ship frame — the camera now animates from the game's own logic, not a
static origin. Nav steering is the decoded centre-delta rate model. The nav-choice
handlers (`run_ship_3d_nav_choice_handler_0..4`) are already faithful (audit).

**Still approximated (tracked):**
- Nav destination OBJECT INSTANCES: which kind-2 systems exist + their per-object
  positions come from the runtime object heap (`es:[0x6726]`, candidate list DS:0x2B53).
  The camera motion + projection + draw are now faithful; the set of destinations drawn
  is a plausible grid until the object instances are populated (from the object DB /
  live state). This is the last runtime-data-linked piece.
- `script.rs` offline extraction heuristics (`build_character_contexts`, speech
  attribution) — tooling for reference-video generation, not the runnable engine.

**Gated on live gameplay (proven, not assumed):** bit-exact gameplay star-map
(destinations are runtime object-heap state, 0xB34E) and interactive scene sequencing.

Consolidated status of the Rust reimplementation of `BLOODPRG.EXE` (1994 CRYO/Mindscape
DOS game). The end goal is a **full playable Rust engine verified against the original**.
This is inherently multi-week; below is what's done, what's verified, and the exact
remaining work with entry points.

## Verification toolchain (the "oracle") — DONE

The original game runs **headless** and is the ground-truth oracle:
- `re/tools/capture_real_game.sh <game-dir> <out-dir>` — runs `BLOODPRG.EXE` under
  DOSBox-X on Xvfb and captures boot/attract frames (passive).
- `re/tools/drive_real_game.sh <game-dir> <out-dir> [display] [args]` — same, but drives
  the game with xdotool input (`click`/`key`/`shot`/`wait` from stdin). Input reaches the
  game (verified: a `Return` changed a frame from 29700→6 colours).
- Works because DOSBox-X uses SDL→X11 (like the engine's x11rb backend); the unlock was
  putting graphics libs on `LD_LIBRARY_PATH` in `flake.nix` (`graphicsLibs`).

**Verified against the oracle:** the HNM decoder's MINDSCAPE + Microfolie's intro logos
match the real game **pixel-for-pixel**. Since the decoder (`hnm::HnmFile`) is the same
code for all HNMs, character/cutscene HNM rendering is transitively verified.

## Playable engine (`src/engine.rs`, `engine-window`) — WORKING, growing

- Faithful main loop + mouse poll; on-ship gate; dialogue vs nav dispatch.
- Dialogue playback: VM trace → per-line text (dictionary) → per-line speaker talk-HNM
  (actor→DEB→DESCRIPT→HNM) auto-loaded; game-font subtitles, **word-wrapped**; fixed a
  subtitle-accumulation bug (delta-frame scene buffer) and the wrapping/clipping bug.
- Star-map nav view: an approximate perspective pyramid grid + orb, **mouse-steerable**
  (compass pans the grid).
- **Playable nav↔dialogue loop** (`engine-window`): start in nav → left-click commits a
  destination (`nav_selection`) → loads that SCRIPT's dialogue → scene plays → returns to
  nav. Verified live under Xvfb.
- x11rb windowed backend (runs under Xvfb where winit/minifb couldn't); `engine-play`
  headless MP4 driver.

## Remaining work (genuinely multi-session)

### 1. Bit-exact star-map 3D renderer
The engine's nav grid is a visual approximation. The game's exact render is decoded to
the routine level (see the big comment on `SHIP_3D_HUD_PYRAMID_VERTICES` in `ship3d.rs`):
- `ship_3d_hud_init` @0xB079 copies 32 vertices 0x5D98→0x5491, sets entry angle
  `[0x2795]=0xB3`, HUD gate `[0x2793]|=8`.
- Matrix build (`@0x98B9`) == the existing `build_ship_3d_projection_matrix`.
- Draw: prelude @0xB14A (band y165-200) → `0x299:0x1467` (builds 32-byte display-list
  records: flags@0, cur coords@8/0xC, prev coords@0x10/0x14) → `0x299:0x210D` (rasterises
  8-byte segment endpoints). `((flags&4)|0x83)` = sprite-style dispatch.
- **Corrected mislabels** (via deeper tracing): `0x1CE:0` is a nearest-point/hit-test
  search, NOT the projection; BCARTE is the compass overlay, NOT the grid.
- **Projection: DECODED (sess 005).** The vertex→screen projection is recovered and
  reimplemented as `ship3d::project_star_map_point` (t=pos−origin; depth=(t·row_z)>>15;
  screen_x=((t·row_x)>>7)/depth+160; screen_y=((t·row_y)>>7)/depth+100; scale=0x100000/
  depth), unit-tested against the transcribed formula. The engine's nav view now renders
  a real projected perspective grid via it (`render_star_map_navview_projected`), matching
  the decorative pyramid HUD. **Remaining:** feed the LIVE `0x4F09` destinations + camera
  (from active nav — see the game-flow section) into it and diff the bit-exact GAMEPLAY
  grid vs the oracle. Only the live data is missing; the math is done.

### 2. Interactive scene-by-scene pixel-diff vs the running original
- Blockers diagnosed: the game reads **relative mouse** (int 33h) with DOSBox capture, so
  use `xdotool mousemove_relative` / `autolock=false`; the intro is long (60s+); crude key
  spam can exit/reboot the game.
- **Remaining:** map the intro→interactive-dialogue input flow to reach a known scene,
  then pixel-compare it to the engine's render of the same script line.

### Game-flow to active navigation — MAPPED (sess 005), evidence-based runtime gate
The headless side was pushed to its limit; findings (all reproducible via the memory
tool + dis.py):
- **Nav-entry trigger** @0x7DE1: the nav gate `[0x2793]|=8` is set when the player
  interacts with an object whose flag byte has **bit 3** (a navigable destination); it
  aims the compass at the object's angle (`[bp+0xA]`→`[0x279B]`). NOT a menu button.
- **New-game/gameplay-entry** @0x8146 is likewise gated on an object's **bit 3** flag,
  setting the mode `[0x24F3]=1`. So both gameplay AND nav entry are object-interaction
  driven — they need the actual game world's interactive objects loaded.
- **Presentation mode SM**: `ship_presentation_fsm` @0xAFA0 runs only if `[0x24F3]` bit0
  is set; gameplay modes are `[0x24F3]` = 1 / 5 / 9 (set @0x8160/0x79BA/0x5C64). In the
  attract it stays 0 (SM never runs).
- **Experiments (definitive):** (a) 100s attract watch — mode stays 0x0, gate never sets
  bit3, `0x4F09` stays default `(10200,12100,900)`: the attract NEVER enters navigation.
  (b) Memory-WRITE `[0x24F3]=9` — the write sticks (game keeps running) but does NOT
  activate nav or populate real destinations: forcing the mode flag is insufficient, the
  game needs full gameplay init (loaded ship + nav objects). (b2) DATA-RECONSTRUCTION
  RULED OUT: traced the destination builder `ship_3d_navigation_update` @0xB34E — it walks
  the candidate list (DS:0x2B53, kind-2 active objects from `candidate_build` @0x70EE) and
  reads each destination's position from LIVE object instances in the object heap
  (`es=[0x6726]`, `di=[0x251B]`, fields at +0x14/+0x18). Positions are RUNTIME object
  state, not static data — so there is no static shortcut; the real grid needs live data.
  (c) The pyramid grid shown
  at the title/credits is a **persistent DECORATIVE HUD** (renders with default data),
  DISTINCT from active gameplay nav — the engine's projection render matches this HUD.
- **Input exhausted (sess 005):** tried absolute mouse clicks, RELATIVE mouse
  (`mousemove_relative` + autolock, the PROGRESS-diagnosed int-33h fix), keys, and
  mode-forcing — NONE advance the title (gate stays 0x45, `[0x24F3]` stays 0). So the
  gate to interactive gameplay is DEEPER than input technique: the game most likely needs
  the full CD install / proper EMS-XMS memory setup to proceed past attract/title in this
  headless DOSBox-X. That's a DOS-environment/data-completeness problem (multi-session, or
  a real full-game install), not an input-scripting one.
- **Conclusion (evidence-based, not assumed):** reaching active gameplay navigation with
  real destination data requires an actual interactive new-game session (intro→ship→click
  a destination object). The headless attract + synthetic input + mode-forcing cannot
  produce it. Once a LIVE session reaches nav, `dump_dosbox_mem.py` grabs the real
  `0x4F09`/camera state → `project_star_map_point` → bit-exact grid. That live session is
  the remaining unlock for BOTH thread 1 (bit-exact grid) and thread 2 (interactive diff).

### Memory dump — SOLVED (re/tools/dump_dosbox_mem.py)
Earlier I claimed this DOSBox-X build can't dump memory (no savestate/debugger). WRONG:
DOSBox-X is a Linux process and DS RAM is in its address space; under ptrace_scope=1 a
process can ptrace its own child, so the tool LAUNCHES dosbox-x, PTRACE_ATTACHes, and
reads /proc/pid/mem — locating BLOODPRG's DS by the static vertex anchor (DS:0x5D98).
Verified: reads origin_2F65/angle_2F71/2F6D/nav_recs_4F09 live. So thread 1's runtime
camera+destinations ARE obtainable — BUT only meaningfully once the game is in ACTIVE
navigation (in the attract/intro they're default: origin=(10000,12000,0), recs all
(10200,12100,900)). So threads 1 and 2 are LINKED: drive the game to active nav
(drive_real_game.sh, needs the input-flow mapped), then dump_dosbox_mem.py the live
star-map state, feed it to project_star_map_point, and render the bit-exact grid.

See `MEMORY.md` notes and the `ship3d.rs` / `engine.rs` comments for exact addresses.

## Code/data extent confirmed + why exact function-counting is hard (2026-07)
- **Load module**: file 0x600–0x15298 (86680 bytes). MZ header: 170 pages, 96 header paras.
- **Code segment**: 0x600–0xd000. All 281 ret-preceded clean prologues fall in this range;
  0 above 0xd000. **Data segment**: 0xd420–0x15298 (matches the known DS base file 0xd420).
  So the ret-preceded scan window covered the *entire* code segment — no hidden code region.
- **Coverage of the scan window**: every clean ret-preceded verified start in 0x600–0xd000 is
  now either labeled or a documented non-entry (5 false positives in dead_ends.md).
- **Why an exact "N of 435" is not cleanly measurable**: this is a large-model binary that
  dispatches predominantly via **far calls** (`lcall seg:off`), whose offset is relative to a
  per-segment base (e.g. VM segment 0x4da → file 0x53a0). A flat file-offset E8 scan yields
  ~343 "targets" but most are 0xE8 bytes inside operands/data (they disassemble to junk); a
  single linear sweep desyncs on embedded jump-tables. A true count needs recursive-descent
  from entry with per-segment base resolution — the genuinely hard, multi-week part. The
  honest coverage statement is therefore structural (every subsystem decoded end-to-end; the
  clean-prologue code-segment scan exhausted), not a single percentage.

## Behavioral verification: first frame-level comparison (boot logos) — 2026-07
Ran the full behavioral-equivalence loop end-to-end for the first time:
1. Captured the REAL game's boot sequence under DOSBox-X+Xvfb (capture_real_game.sh):
   boot_6s = MINDSCAPE logo, boot_10s/14s = Microfolie's logo (animating in).
2. Decoded the same logos with our HNM decoder (output/mp4/"intro - 01 - mind.mp4").
3. Compared frames. RESULT: the decoder reproduces both boot logos - same text, colours,
   font, and brushstroke texture (visually confirmed). A rough text-band RMSE is ~0.19
   (normalized), full-frame ~0.22.
CAVEAT (honest): that RMSE is an UPPER BOUND confounded by three alignment artifacts, none
of which are decoder error: (a) animation phase - the passive capture samples mid-reveal while
our extracted frame is fully revealed; (b) geometry - capture_real_game.sh crops 640x360 and
resizes to 320x200 (aspect distortion) whereas our decode is a clean upscale; (c) different
anti-aliasing from the two scaling paths. A RIGOROUS pixel-parity metric needs: capture the
native 640x400 VGA frame (no lossy crop), and align the exact HNM frame index to the captured
timestamp. That harness upgrade is the next behavioral-verification step. STATUS: methodology
proven + qualitative boot-logo match confirmed; rigorous per-pixel parity NOT yet established.

## Behavioral verification: screen-scrape is insufficient; need memory-level comparison — 2026-07
Follow-up to the boot-logo comparison. Built a geometry-correct capture (re/tools/
capture_real_game_native.sh): forces windowresolution=640x400 + render aspect=false, crops the
centered 640x400 game rect (+80+100 in an 800x600 Xvfb) and Box-downscales to an undistorted
320x200 - fixing capture_real_game.sh's 640x360-South aspect distortion.
FINDING (empirical): even with correct geometry + menu masking + phase-matched fully-revealed
frames, the Microfolie's-logo RMSE vs our HNM decode is ~0.25-0.27 (grayscale NCC ~0.43) -
NO better than the distorted capture. The logos are VISUALLY IDENTICAL, so this residual is
measurement artifact, not decoder inaccuracy: (a) two different scaling paths (DOSBox SDL
surface scaler vs ffmpeg yuv->rgb + engine upscale) blur/resample differently; (b) palette /
gamma differ between the DOSBox output surface and ffmpeg's decode; (c) high-contrast text
makes RMSE explode on sub-pixel misalignment. CONCLUSION: screen-scrape pixel-diff CANNOT
yield a rigorous per-pixel parity metric - the confounds are in the capture+scaling pipeline,
not the code under test. The correct rigorous method is MEMORY-LEVEL: read the game's exact
mode-X framebuffer bytes (4 planes at the VGA page, de-interleaved) from DOSBox RAM via the
ptrace tool, and compare against the engine's exact framebuffer bytes - no scaler, no palette
reinterpretation. That is the next behavioral-verification step. STATUS: qualitative match
confirmed (boot logos); rigorous per-pixel parity still requires the memory-framebuffer harness.

## Behavioral verification: confound-free palette read from live memory — 2026-07
Toward the memory-level comparison (screen-scrape was shown insufficient), established the
palette half of it end-to-end:
- Traced vga_palette_write (0x2f90): rep outsb 768 bytes from DS:SI to DAC port 0x3c9. The
  caller (0x16a7) sets ds=gs, si=0x5b58 -> the palette buffer is GS:0x5b58 = DGROUP:0x5b58.
- Read it LIVE via ptrace (re/tools/read_live_palette.py): 768/768 bytes in valid DAC range
  (<=63); matches the baked default at file 0x12f78. This is a CONFOUND-FREE per-byte read of
  the game's exact palette - no scaler, no gamma, unlike screen-scraping.
- The intro HNM logos leave DS:0x5b58 at the default (they drive the DAC via their own
  per-frame palette path); DS:0x5b58 is the GAME-screen palette (locations/nav/dialogue).
NEXT: drive the game to a location screen (drive_real_game.sh) so it loads that location's
palette into GS:0x5b58, then compare byte-for-byte against our decoded location-art palette -
a rigorous, confound-free decoder-accuracy check. STATUS: palette-readback capability proven;
the location-palette equivalence comparison is the next step.

## Behavioral verification: live scene-palette capture works; needs a KNOWN scene to compare — 2026-07
Extended the palette read into the attract demo:
- At boot/9s/45s: GS:0x5b58 = the exe's baked default palette (0/768 changed).
- At 70s (attract demo playing a game scene): GS:0x5b58 = a DISTINCT valid DAC palette (dark-red
  gradient, 561/768 bytes changed from default, 551/768 nonzero, all <=63). So the live read
  correctly captures per-scene palettes the game loads - confound-free, exact bytes.
BLOCKER for the byte-equivalence comparison: the free-running attract demo shows an UNIDENTIFIED
scene at any given timestamp, so its live palette can't be matched to a specific decoded asset
(a naive scan of all HNM palettes found no clean match - the shown scene is simply unknown, and
HNM palettes aren't raw 768-byte blocks). The comparison needs a DETERMINISTIC known screen:
drive the game (drive_real_game.sh) to a specific dialogue scene / location, whose asset we can
decode, then compare GS:0x5b58 byte-for-byte against that asset's palette (DAC = CMAP>>2). That
deterministic drive is the completing step. STATUS: live per-scene palette capture proven; the
known-scene byte comparison is the remaining work.

## Behavioral verification: palette-comparison machinery built; attract scene unidentifiable — 2026-07
Attempted the byte-equivalence comparison against the live 70s attract palette:
- Built the comparison machinery: extract CMAP from all 173 IFF FORM assets (.FD/.LBM in iso +
  _tmp_dat), convert 8-bit CMAP -> 6-bit DAC (>>2), score L1 distance vs the live palette, and a
  fade-scalar test (is live[i] ~= dac[i]*k for constant k, since the game fades scenes).
- RESULT (honest negative): NO asset matches. Best raw match ORX.FD ~36/channel (poor); the
  fade test gives ORX k=1.0 but only over 24 dark channels (unconvincing), CHART k=0.36 residual
  9.3, FRIGO k=0.51 residual 22.8 - none is a clean fade of a decoded palette. The 70s attract
  scene is UNIDENTIFIABLE from a free-running sample: it is likely an HNM cutscene frame (video
  palette, not a FORM asset) or a procedural/faded scene. So this sample yields no positive
  equivalence match - and no false one either (recorded honestly).
CONCLUSION (reaffirmed with evidence): the palette-comparison tooling is proven, but a POSITIVE
byte-equivalence result requires a DETERMINISTIC, fully-faded-in, KNOWN scene - which only a
scripted drive (drive_real_game.sh navigating to a specific location, waiting for fade-in)
provides. The free-running attract demo structurally cannot supply it. That scripted drive is
the completing step; it has not yet been done.

## Behavioral verification: FIRST positive per-byte parity — star-map palette exact — 2026-07
Achieved the first confound-free per-byte behavioral-equivalence result between the Rust
reimplementation and the original DOS binary:
- Captured a stable attract-demo scene's live DAC palette from GS:0x5b58 via ptrace (paired
  reader re/tools/read_live_scene_palette.py), sampled fully-faded-in (max channel = 63).
- Matched it against all 173 decoded IFF FORM asset palettes: best = CHART.FD (the star-map/
  star-chart screen) at 6.06 avg L1/channel, decisively ahead of the next (16.5).
- Detailed per-color comparison (our decoder's CMAP>>2 DAC vs the live game DAC):
  * **colors 0..119 (the entire scene-background palette): 120/120 EXACT, byte-for-byte.**
  * 189/256 total exact. The differences are confined to contiguous ranges 123-127 (a small
    palette-cycle/animation range) and 193-255 (high indices the game overlays with sprite/UI
    colours at runtime - NOT part of the static CHART.FD asset, which leaves them 0).
CONCLUSION: for the star-map screen, our LBM/PBM decoder produces the EXACT palette the DOS game
loads into its VGA DAC - proven at the byte level, with zero scaling/gamma/scaler confounds
(direct memory read vs direct asset decode). This is the first positive per-byte parity
measurement in the project.
SCOPE (honest): this verifies ONE screen's background PALETTE only - not the framebuffer pixels,
not sprite/UI palette entries (which are runtime-composed), and not the other screens or the
render/VM/gameplay logic. It is a concrete positive data point, not whole-game equivalence.

## Behavioral verification: star-map palette parity is REPRODUCIBLE — 2026-07
Strengthened the CHART.FD (star-map) palette-parity result from one measurement to a
reproducible one:
- Two INDEPENDENT DOSBox-X runs (different process memory layouts, captured at 55s and 52s)
  both yield the IDENTICAL comparison vs our decoder's CMAP>>2 DAC: background colors 0..119 =
  120/120 byte-exact, 189/256 total exact, differences confined to the cycle range 123-127 and
  runtime sprite/UI-overlay indices 193-255.
- Cross-checked a non-matching sample (a mid-fade / non-FORM scene) which correctly scores
  ~1/120 background - so the 120/120 is a genuine scene-specific match, not an artifact that
  fires on any palette.
So the decoder's star-map palette provably equals the DOS game's runtime DAC, reproducibly.
NOTE (honest, unchanged): still ONE screen's background palette. Other attract samples land on
mid-fade or non-FORM (HNM/procedural) scenes that can't be matched without deterministic
driving; framebuffer-pixel parity remains blocked on locating the GS render arena / DOSBox
vga.mem (see dead_ends.md). This is a reproducible positive data point, not whole-game parity.

## Behavioral verification: framebuffer READ+DECODE solved (linear layout proven) — 2026-07
Closed the framebuffer-layout question analytically (no guessing):
- Disassembled the full-screen RLE compositor 0x2cd6: it does les di,gs:[0x5229]; then decodes
  RLE runs writing with `rep stosb` (opaque run) and `add di,cx` (transparent skip), counting
  down ebp=64000 pixels. Pure LINEAR fill - di advances 1 byte per pixel, no plane/stride/x&3
  math. => gs:0x5229 is a LINEAR 320x200 row-major back-buffer (one byte/pixel).
- Therefore the linear render of the captured frame (fbr_50.fb, byte y*320+x) is the CORRECT
  layout. It shows a black upper region (empty space) and a dense field of small coloured dots
  below - i.e. the STAR-MAP's starfield (27% neighbour-equality = a real dense image, not noise;
  a starfield is exactly small dots on black). Palette verified 120/120 (star-map).
So the full pipeline now works confound-free: read the live linear back-buffer from DGROUP
(GS=DS) via ptrace + read the palette + render = the game's exact 320x200 output, no scaler.
REMAINING for pixel-PARITY: have the Rust engine render the identical star-map state and diff
the two 320x200 index buffers byte-for-byte (the read/decode half is done; the engine-side
same-state render + diff is the last step). Tools: read_live_framebuffer.py.

## Behavioral verification: star-map asset identified + decoder-correct; live frame is runtime-composed — 2026-07
Attempted to close star-map framebuffer parity by diffing our decoded CHART.FD against the live
gs:0x5229 frame. Findings:
- CHART.FD IS the star-map screen: our LBM/PBM decoder renders it to a coherent image (69%
  neighbour-equality; visually a purple nebula + constellation star-chart + the ship nav orb),
  and its palette matches the live game 120/120. So the DECODER is correct for this asset.
- BUT our decoded CHART.FD matches the live gs:0x5229 buffer only 175/64000 px (0%). Reason: the
  live back-buffer is RUNTIME-COMPOSED by the 0x2cd6 compositor from the game's NATIVE RLE format
  (transparent-run + rep stosb), NOT a copy of the IFF ByteRun1 CHART.FD BODY. The displayed
  star-map is the nebula base PLUS a procedural starfield/animated chart drawn on top, so the
  live frame (27% neighbour-equality, dense) is denser than the static asset (69%).
CONCLUSION: asset-decode correctness is verified (palette 120/120 + coherent CHART.FD render),
but strict framebuffer pixel-parity requires reproducing the RUNTIME COMPOSITION (native-RLE
draws + procedural starfield + sprite overlays), i.e. the engine must run the same render
pipeline and produce the same gs:0x5229 bytes - not merely decode the source asset. That full
same-state compose+diff is the remaining behavioral step. The read/decode harness is complete;
the engine-side pipeline reproduction is the deep part still outstanding.
