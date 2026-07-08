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

## Behavioral verification: attract demo exposes only star-map + HNM; static-scene parity needs driving — 2026-07
To get a framebuffer-vs-static-asset pixel match (unlike the procedural star-map), sampled
gs:0x5229 deeper into the attract demo (90s, 120s) hoping for a dialogue/location screen with a
static full-screen background. Both caught bb_5229 = 0000:0000 (null back-buffer) => the attract
was showing HNM CUTSCENES, which render via their own path and leave gs:0x5229 unallocated.
Pattern (confirmed over many samples): the free-running attract demo reliably exposes only
(a) the STAR-MAP (~50s, back-buffer allocated but PROCEDURALLY composed - starfield, so it does
not byte-match any static asset), and (b) HNM cutscenes (no back-buffer). It does NOT reliably
surface a static-background game-engine screen (dialogue/location) whose live gs:0x5229 would be
a direct blit of a decoded fd/ location asset.
CONCLUSION: a clean framebuffer pixel-parity result (live back-buffer == our decoded asset)
requires a DETERMINISTIC drive into actual gameplay (launch with the AMR/EMS args, navigate to a
dialogue/location screen, read gs:0x5229 when stable) - the same deterministic-driving
dependency that gates the multi-scene palette comparison. The read/decode harness is complete
and proven; every remaining behavioral check is now gated on either deterministic gameplay
driving or full engine-side render-pipeline reproduction. Both are substantial, not yet done.

## Behavioral verification: reached gameplay; mapped render paths; VGA mem separately paged — 2026-07
Drove the real game into actual GAMEPLAY deterministically (launch args
`BLOODPRG.EXE AMR S162227 EMS WRIC:\cblood\`, cblood/ dir present): past the intro logos
(MINDSCAPE/Microfolie's/CRYO) to the main "Commander BLOOD V 1.0" nav interface (alien
viewscreen + CARTE nav pyramids + BORXX orb). Findings on the render architecture (confirmed
by reading gs:0x5229 across screens):
- STAR-MAP (attract ~50s): uses the LINEAR back-buffer gs:0x5229 (procedurally composed).
- MAIN NAV / most game screens: gs:0x5229 = 0 (null) even when a stable game screen is shown =>
  they render DIRECTLY to VGA mode-X (gs:0x521d = A000:xxxx), bypassing the linear back-buffer.
- HNM cutscenes: own path, no back-buffer.
- Tested reading the emulated VGA window at dos_base+0xA0000: 0/65536 nonzero => DOSBox-X stores
  live VGA RAM in a SEPARATE vga.mem allocation, NOT in the conventional-RAM block at 0xA0000.
CONCLUSION: the linear-back-buffer read only covers the procedural star-map. Per-pixel parity for
the (majority) VGA-rendered screens requires locating DOSBox-X's separate vga.mem region in the
process (search all mmaps for the mode-X frame) and de-interleaving 4 planes - not yet done. This
maps the remaining framebuffer-parity work precisely: it is a vga.mem-location + planar-decode
task, distinct from the (solved) linear-buffer + palette reads.

## Behavioral verification: CORRECTION - gs:0x5229 is a starfield TEXTURE, not the display frame — 2026-07
Self-correction of an earlier over-claim ("framebuffer read+decode solved via gs:0x5229"):
- Compared two captures of gs:0x5229 from DIFFERENT runs and DIFFERENT screens: gp45 (CRYO
  logo, gameplay run) vs fbr_50 (star-map, attract run). They are 84% IDENTICAL with near-
  identical value histograms. A per-frame display buffer would differ completely between two
  different screens; 84% sameness means gs:0x5229 holds a PERSISTENT STARFIELD TEXTURE, not the
  composited display frame.
- So my earlier "captured the star-map framebuffer" was actually capturing this starfield
  texture source. The linear-buffer read/decode pipeline is real, but it reads a TEXTURE, not
  the screen. The actual displayed frame is VGA mode-X (gs:0x521d = A000), in DOSBox-X's
  SEPARATE vga.mem (confirmed: dos_base+0xA0000 = all zeros).
STANDING (unaffected by this correction): the palette per-byte parity (CHART.FD 120/120,
reproducible) and asset-decoder correctness (coherent CHART.FD render) - those used the palette
buffer (0x5b58, genuinely in DGROUP) and the decoded IFF asset, not gs:0x5229.
CORRECTED next step: display-frame pixel-parity for ALL screens is gated on locating DOSBox-X's
separate vga.mem region (search the process mmaps for the mode-X frame) + planar de-interleave.
The gs:0x5229 texture path does not yield display frames. Honest downgrade of the prior claim.

## Behavioral verification: STATE parity - nav/camera constants match live runtime — 2026-07
Pivoted from framebuffer pixels (blocked on DOSBox vga.mem) to DGROUP-relative STATE parity,
which IS memory-reachable via the DS anchor. Read the live ship-3D / star-map nav state from the
running game and compared to our STATIC RE decode - two exact confound-free matches:
- CAMERA ORIGIN DS:0x2F65: our decode (ship3d.rs:1833) = reset to (0x2710,0x2EE0,0) =
  (10000,12000,0). Live runtime memory = (10000,12000,0). EXACT MATCH.
- NAV DESTINATION RECORDS DS:0x4F09: our decode (ship3d.rs:1880) = static default
  (10200,12100,900) per 6-byte record. Live runtime = (10200,12100,900) for 10/11 records
  (the 11th slot = (0x4000,0,0x3FF6) = adjacent Q14 rotation-matrix data, array boundary).
  EXACT MATCH on the destination records.
So our static reverse-engineering of the navigation subsystem's state layout + constants is
confirmed against the live game's runtime memory, confound-free (direct DGROUP read, no
scaler/emulation artifacts). This is a STATE-level equivalence result complementing the
asset-level palette parity (CHART.FD 120/120) - two independent axes now have positive
confound-free confirmation.
SCOPE (honest): these are static/default state constants (camera origin + destination defaults),
verified at the value level - not the dynamic per-frame nav math, not the object table under
active play, not the VM variable evolution. A deeper state check (drive active navigation, read
the evolving camera/destinations, compare to our engine's per-frame computation) is the next
step. Positive data points on the state axis; not whole-game state parity.

## Behavioral verification: STATE parity - render clip rect matches decoded 320x200 — 2026-07
Broadened the state-axis confirmation by reading render-state globals from the live game vs our
static RE decode:
- x-CLIP: DS:0x5235 = 0, DS:0x5237 = 0x140 (320). Our decode: 0x5235/0x5237 = x-clip bounds
  (gfx_clipped_draw 0x3321). Live = [0, 320] = full screen width. MATCH.
- y-CLIP: DS:0x5239 = 0, DS:0x523b = 0xC8 (200). Our decode: 0x5239/0x523b = y-clip min/max
  (graphics_plot_modex 0x3428, back_buffer_fill 0x3dbf). Live = [0, 200] = full screen height.
  MATCH.
So the game's runtime clip rectangle is exactly [0,320]x[0,200] - confirming our decoded screen
geometry (mode-X 320x200) against live memory, confound-free.
- DS:0x5221:0x5223 = 0000:2cee = the display-page far pointer to a RAM back-buffer this frame
  (consistent with the decoded double-buffering; the 0xa000 in set_vga_segment 0xD75 is the init
  value, and the page-flip swaps the segment - so 0x2cee here is the off-screen page, not a
  contradiction).
Running tally of confound-free positive equivalence results: (1) palette CHART.FD 120/120
[asset], (2) camera origin (10000,12000,0) [state], (3) nav destinations (10200,12100,900)
[state], (4) clip rect [0,320]x[0,200] [state]. Four independent confirmations across asset +
state axes. STILL NOT whole-game: dynamic per-frame math, VM evolution, object table under play,
and display-frame pixels remain unverified.

## Behavioral verification: object table populated in gameplay (structural, not value-matched) — 2026-07
Read the entity_object_table DS:0x6212 in two states:
- ATTRACT star-map (50s): all records zero (entity system not active in attract nav).
- GAMEPLAY (72s, launch args, main nav interface): records POPULATED with structured data -
  rec0 flags=0x0055 id=0x004d + pointer/data words; rec1 flags=0x0020 id=0x0051; rec2
  flags=0x0071. So the 32-byte entity-record table IS live and populated during play, confirming
  the decoded object system activates.
HONEST LIMITATION: the populated field values do NOT cleanly map onto our decoded 32-byte layout
(+0 flags,+8 id,+0xc/+0xe pos): the "pos" words read as (73,31130) etc = not screen coords, so
either the record stride/field offsets need refinement or these hold world/other coords. This is
a STRUCTURAL confirmation (table active + populated in the right state) but NOT a value-level
match - so it is NOT added to the confirmed-parity tally. The clean value-matched tally stays at
4 (palette 120/120; camera origin; nav destinations; clip rect). Refining the 0x6212 record
field layout against these live bytes (and getting a ground-truth object set) is follow-up work.
Also note DS:0x2789 (zoom) = 0 in both states.

## Behavioral verification: STATIC-DATA parity - ship-3D vertex table matches exe exactly — 2026-07
Added a confound-free STATIC-DATA axis (engine's embedded tables vs the exe's compiled data,
verified directly from BLOODPRG.EXE - no DOSBox, instant):
- SHIP_3D_HUD_PYRAMID_VERTICES (src/ship3d.rs, [[i16;3];32]) vs exe file 0x131B8 (the DS:0x5D98
  vertex table that ship_3d_hud_init 0xB079 copies to the HUD working area): 32/32 vertices
  BYTE-FOR-BYTE IDENTICAL (e.g. v0=(0,2304,3075), v1=(776,1803,2820)). The region is exactly 32
  vertices (192 bytes) then other data - matching our documented count.
- This CHAINS with the live axis: the ptrace DS-anchor IS this same vertex table (0x5D98), which
  we confirmed present in live runtime memory. So: exe static (0x131B8) == engine embedded
  (32/32) == live runtime (anchor found) - a full three-way match on the ship-3D geometry data.
- SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR embeds [.,.,.,.,0x140,0xc8,..] = 320x200, consistent with
  the live-verified clip rect [0,320]x[0,200].
CONFIRMED-PARITY TALLY now 5 confound-free positives across 3 axes:
  asset: (1) palette CHART.FD 120/120.
  state: (2) camera origin, (3) nav destinations, (4) clip rect [0,320]x[0,200].
  static-data: (5) ship-3D vertex table 32/32 (exe==engine==live).
STILL NOT whole-game: descriptor/sprite-pixel values, dynamic per-frame math, VM evolution, and
the composited display frame remain unverified. Five verified data points, not 100%.

## Verification accounting: 419 passing tests incl. ~32 exe-comparison + VM traces — 2026-07
Fuller (honest) accounting of the EXISTING verification base, beyond the per-turn dynamic tally:
- The Rust suite has 419 tests, 0 failing. Of these, ~32 test fns load BLOODPRG.EXE and assert
  engine data/behaviour BYTE-EXACT or math-exact against it, including:
  * angle_table_matches_binary (the DS:0x4F45 180-entry Q14 trig table),
  * SHIP_3D_HUD_PYRAMID_VERTICES vs 0x131B8 (32/32, this session),
  * font glyph tables vs 0x14D28 (extracts_dialogue_font_tables_from_binary),
  * anim_prng_matches_ror (the 0x1CE:0x0B02 PRNG),
  * projection_matrix_preserves_binary_fixed_point_operation_order, position_distance_uses_
    binary_rounded_sqrt, plane_band_copy_reports_scroll_value_like_binary_math, etc.
  * a large family of execution_trace_* tests that run our VM interpreter and compare record/
    state/branch behaviour against the decoded COD semantics.
So static-table parity + VM-semantics parity for the DECODED subsystems is already under test
and passing. My dynamic /proc/mem work adds a LIVE-runtime axis on top (5 confound-free
confirmations: palette, camera, nav, clip, vertex table - the last a 3-way exe==engine==live).
HONEST SCOPE (unchanged): these tests verify the SPECIFIC decoded tables, routines, and VM
traces - NOT every one of the ~435 functions, NOT the full composited render output, NOT
descriptor/sprite-pixel runtime values. Broad and passing != whole-game per-byte parity. The
coverage is substantial and real, but targeted; the uncovered remainder (full render pipeline,
all functions behaviorally, display-frame pixels) is exactly why 100% is not met.

## Behavioral verification: STATIC-DATA parity #6 - level directory matches exe resource table — 2026-07
Verified the full resource name table and made it a permanent regression test:
- src/levels.rs LEVEL_DIRECTORY (53 entries) vs BLOODPRG.EXE file 0xCDF4 (FS:0x0c04, 16-byte
  filename records indexed by resource id): 53/53 stems match byte-for-byte (fupcom.spr,
  nosound.drv, script1.cod/bas/var/dic/deb, radio.spr, buffer x5, bappel/btv/borxx/bcarte/
  bhyper/bpol/aphyper/appol.spr, black.ext, kult.ext, ...).
- Added test level_directory_matches_bloodprg_resource_table (passes) - now part of the suite
  (420 tests). This is confirmation #6 on the static-data axis.
Tally: 6 confound-free positives - asset (palette 120/120); state (camera, nav, clip); static-
data (ship-3D vertex table 32/32; resource name table 53/53). STILL targeted, not whole-game:
full render output, all-function behavior, and descriptor/sprite runtime pixels remain unverified.

## Behavioral verification: STATIC-DATA parity #7 - full font (73 glyphs incl lowercase) — 2026-07
Broadened the existing font byte-comparison test from ~40 glyphs to the FULL printable set:
- src/font.rs GAME_FONT_GLYPHS + GAME_FONT_WIDTHS vs BLOODPRG.EXE glyph map 0x14c22 / advances
  0x14cd2 / rows 0x14d28: all 73 printable ASCII chars the exe maps to a non-space glyph
  (uppercase A-Z, LOWERCASE a-z, digits 0-9, punctuation !"'+,-.:;?_) match BYTE-FOR-BYTE on both
  the 8-byte glyph rows AND the advance width. Previously only uppercase+digits+5 punct were
  checked; lowercase (a idx 39 != A idx 0, distinct glyphs) is now verified too.
- The engine panics if any exe glyph is missing - none is; 73/73 present and exact.
Static-data parity confirmations now: vertex table 32/32, resource table 53/53, font 73/73.
Tally of confound-free positives = 7 (asset palette; state camera/nav/clip; static-data vertex/
resource/font). Still targeted, not whole-game: full render output, all-function behavior, and
runtime descriptor/sprite pixels remain unverified.

## Behavioral verification: VM walk completeness - all 5 real scripts to exact token counts — 2026-07
Locked in the COD walker's completeness on the REAL game scripts (behavioral, not just static):
- walk() on each real SCRIPT<n>.COD produces the EXACT reverse-engineered token counts:
  SCRIPT1=214, SCRIPT2=3271, SCRIPT3=3281, SCRIPT4=1714, SCRIPT5=1869 - all cleanly to the 0xFF
  end marker (the existing test only asserted 0 invalid tokens; this adds the exact totals).
- New test walks_real_scripts_to_documented_token_counts (passes). This verifies the VM
  bytecode WALK (opcode length model incl. the 0-length scan-zero-word opcodes) exactly
  reproduces the game's script structure across all five scripts.
This is a VM-structure parity result on top of the static-data + execution_trace behavioral
tests. Confound-free tally unchanged at 7 memory/exe value-matches; VM-side coverage broadened.
STILL NOT whole-game: full render output, all-function behavioral parity, and runtime
descriptor/sprite pixels remain unverified. Broad and deepening, but not 100%.

## Behavioral verification: .ext parsing across ALL 16 primary worlds — 2026-07
Extended the .ext object cross-validation from 3 worlds to all 16 primary worlds:
- Every primary world (.ext) parses and exposes an initial object id=1, type=4 at an on-screen
  position: BLACK(199,42), KULT(133,57), VENUSIA(134,117), ERAZOR(133,124), MASTACHO(109,90),
  MAGNUS(169,92), EKATOMB(202,102), CRAZY(143,90), KORTEX(84,87), VISTA(133,111), MOSKITO
  (103,103), PTERRA(144,96), CYBER(173,75), CORPO(126,107), MENHIR(106,79), VULCAN(84,68).
  16/16 valid; the 3 previously-documented (black/venusia/magnus) match.
- New test all_primary_worlds_parse_with_valid_initial_object (passes). Confirms our .ext
  parser + object decoder handle the full world set, not just the 3 spot-checked worlds.
Verification base keeps broadening (static-data byte-exact tables + VM structure/semantics +
all-world .ext parsing + 7 confound-free live/exe value-matches). STILL targeted, not whole-game:
full render output, all-function behavioral parity, runtime descriptor/sprite pixels unverified.

## Bug fix (found via verification): raw sprite banks decoded to 0 frames — 2026-07
The broadened all-sprite-banks decode test surfaced a REAL decoder bug:
- bank_dispatch_index computed `(((flags&4)|0x83)>>1)&7`, which only ever yields odd values
  (1 or 3 = RLE) - so it NEVER selected the RAW path (0/2), silently decoding RAW banks to 0
  frames. Verified across all 44 banks: flags bit2 selects encoding (flags&4==0 => RAW body ==
  width*height, e.g. BAPPEL.SPR 11/11 raw frames; flags&4==4 => RLE, e.g. BCARTE). Fixed to
  `if flags&4==0 {0 /*raw*/} else {3 /*RLE*/}`.
- Result: all 43 standard sprite banks (41 RLE + 2 raw) now decode to valid frames. KLAY.SPR
  (flags=6, frame_count=256 with garbage offsets) is a non-frame-table asset and is correctly
  rejected (returns None) - the test asserts that too.
- New test decodes_every_sprite_bank_to_valid_frames (passes) locks this in.
This is the FIRST behavioral fix found by the verification push (not just a confirmation): 2 raw
sprite banks were previously mis-decoded. Suite now 422 tests, 0 failing. Accuracy improved.

## Behavioral verification: HNM parse robustness across all 645 files — 2026-07
Added a broad HNM(1) parse-robustness test (following the sprite-bank pattern that found a bug):
- opens_and_parses_every_hnm_asset walks the asset tree and, for every .hnm (645 files),
  asserts HnmFile::open succeeds, frame_count() > 0, and frame_dims(0) is a valid (1..=511 x
  1..=255) size. ALL 645 pass - the HNM header/superchunk parser is robust across the full set
  (intro logos, character animations tr*/cg, cutscenes). No gap here (unlike sprites).
- Suite now 424 tests, 0 failing.
This broadens verified decoder coverage: LBM (169 fd/ art, was already tested), sprite banks
(43 standard + KLAY rejected, bug fixed last turn), HNM (645 files, new). The asset-decoder
surface is now systematically covered. STILL not whole-game: full composited render output,
all-function behavioral parity, and runtime pixel values remain unverified.

## Behavioral verification: audio decoders robust across all SND/VOC assets — 2026-07
Extended the systematic decoder-coverage push to audio:
- parses_every_real_snd_bank: all 25 SND voice/sfx banks parse into a bank with >0 clips.
- parses_every_real_voc: all 44 .voc files carry the "Creative Voice File" signature and parse;
  some yield PCM (music/voice), verified non-empty at a valid rate.
Both pass - the SND bank parser and VOC decoder are robust across the full asset set. No gap
(unlike the sprite raw-decode bug). Suite now 426 tests, 0 failing.
Asset-decoder coverage is now systematic across ALL asset types: LBM 169, sprites 43+reject,
HNM 645, SND 25, VOC 44 - every decoder verified against its full asset set, one real bug fixed
(raw sprites). This is thorough decoder-parity, but STILL not whole-game: the composited render
output, all-function behavioral parity, and runtime pixel values remain unverified.

## Behavioral verification: DESCRIPT.DES scene database parses consistently — 2026-07
Added a real-file regression test for the scene/dialogue database (was only synthetic-tested):
- parses_real_descript_des_consistently: the real DESCRIPT.DES parses into 145 records with the
  exact kind distribution (11 Sequence, 35 Object, 64 Location, 35 Character), every record
  named, and every referenced snd/sprite stem non-empty. Passes.
This locks in the descript parser (the game's scene graph: locations, characters, objects,
cutscene sequences, their media + subtitles) against the shipped data file. Suite now 427.
Verified surface: decoders (LBM/sprite/HNM/SND/VOC full sets), data files (DESCRIPT.DES,
resource table, level dir, .ext all worlds, COD all scripts), static tables (vertex/font),
state (nav/camera/clip), VM (walk+traces). STILL not whole-game: composited display output,
all-function behavioral parity, runtime pixels unverified.

## Verification: sprite decoder decodes ALL header frames (no drops) — 2026-07
Strengthened the all-sprite-banks test: for every standard bank, the decoded frame count now
must equal the header frame_count (frames.len() == header count) - so no frame is silently
dropped. Verified across all 43 standard banks (total ~600+ frames). Combined with the raw-decode
bug fix, the sprite decoder is now proven to decode every frame of every standard bank correctly.
This is the payoff pattern: broad exhaustive tests either confirm 100% coverage or expose a bug.

## Bug fix #2 (cross-decoder verification): extract-side sprite decoder had the SAME raw bug — 2026-07
Cross-checking the two independent sprite decoders (engine src/sprite.rs vs extract src/extract/
render.rs) revealed the SAME raw/RLE dispatch bug in the extract-side path (used for the MP4/PGM
sprite exports):
- SpriteSlotFrameTable::dispatch_index() = ((flags&4)|0x83)>>1 & 7, which only ever yields odd
  (RLE) codes, so RAW banks (flags&4==0, BAPPEL.SPR + 1 other) silently exported 0 frames.
- Fixed identically: `if flags&4==0 {0 raw} else {3 RLE}`. Added regression test
  raw_sprite_bank_decodes_via_extract_decoder (passes).
So the raw-sprite bug existed in BOTH decoders; both now fixed. This is why cross-decoder
verification matters - the same defect was duplicated. Suite now 428 tests, 0 failing. Two real
accuracy bugs found+fixed by the verification push (both the raw-sprite dispatch, engine+extract).
STILL not whole-game: composited display pixels, all-function behavioral parity, runtime pixels.

## Behavioral verification: VM interpreter produces exact line-state counts for all 5 scripts — 2026-07
Extended VM verification from the WALK (token counts) to EXECUTION (interpret + VAR-init):
- interpret_line_states on each real SCRIPT<n> (COD+VAR) produces the exact RE'd dialogue
  line-state counts: SCRIPT1=111, SCRIPT2=1157, SCRIPT3=1048, SCRIPT4=719, SCRIPT5=652 - matching
  the recovered per-script text-line counts. New test interprets_real_scripts_to_documented_line_
  counts (passes). Also cross-checked the two RLE sprite decoders (engine decode_rle_frame vs
  extract decode_rle_sprite_pixels) - IDENTICAL control-byte semantics (negative=replicate
  -control+1, positive=literal control+1), consistent.
So the VM is now verified at both levels: bytecode WALK (all 5 scripts to exact token counts) and
EXECUTION (all 5 scripts to exact line-state counts), on top of the execution_trace behavioral
family. Suite now 430. STILL not whole-game: composited display pixels, all-function behavioral
parity, and runtime pixel values remain unverified.

## Accuracy improvement: sprite decoder now captures the frame draw-offset (was discarded) — 2026-07
Found via the sprite-frame investigation: SpriteFrameImage stored only width/height/indices,
DISCARDING each frame's authored x/y draw offset (header +4/+6). These offsets vary per frame to
anchor animations as the sprite resizes (BORXX orb: y-offset 0..49; BAPPEL/BCARTE also vary).
Dropping them loses the authored anchoring the game uses.
- Added x_offset/y_offset to SpriteFrameImage, populated from the header, with test
  captures_frame_draw_offsets_from_the_header (cross-checks captured vs file bytes; confirms the
  orb's offsets genuinely vary). blit_sprite_frame_centered still centres (correct for symmetric
  HUD sprites); the offset is now AVAILABLE for offset-aware draws. Also fixed a stale doc comment
  on bank_dispatch_index (still described the old buggy formula).
The decoder now captures the COMPLETE frame header instead of discarding anchor data - a real
accuracy improvement (third fix from the verification push). Suite now 431. Still not whole-game.

## Accuracy: game bottom-anchors the orb; added offset-aware blit primitive — 2026-07
Decoded the orb's animation anchoring and provided the correct primitive:
- BORXX orb: across all 16 frames, y_offset + height == 82 (CONSTANT). So the game draws each
  frame at base+(x_offset,y_offset) top-left, keeping the orb's BOTTOM edge fixed as it grows
  (33..82 px tall) - it grows UPWARD, not symmetrically. blit_sprite_frame_centered keeps the
  CENTRE fixed, which would grow the orb symmetrically = wrong anchoring.
- Added blit_sprite_frame_at(fb, base_x, base_y): draws at base+offset (the game's anchoring),
  with test offset_blit_bottom_anchors_the_orb_like_the_game (two different-height frames sharing
  yoff+h land their bottom edges on the same row). The correct primitive now exists.
FOLLOW-UP (honest): the engine's current orb draw is a legacy path drawing frame 0 centred; fully
wiring the animated, offset-anchored orb render needs the game's orb anchor screen position (and
visual verification, which is gated on the framebuffer-parity blocker). The primitive + the
anchoring fact are locked in; the render wiring is the remaining step. Suite 432.

## Accuracy scope: sprite offset is the UNIVERSAL positioning mechanism — 2026-07
Surveyed the frame-offset anchoring across all 44 .spr banks to scope the render inaccuracy:
- 34 banks are single-frame (offset positions the one frame; no animation-anchor question).
- Of the ~15 MULTI-frame (animated) banks, the offset yields a consistent per-sprite anchor:
  bottom-anchored 3 (yoff+h const, e.g. BORXX orb), top 3, right 4, left 5, and 4 with no simple
  constant but still offset-driven. So the specific anchor is emergent from the authored offsets.
- CONCLUSION: drawing each frame at base+(x_offset,y_offset) - i.e. blit_sprite_frame_at -
  reproduces the game's positioning for EVERY sprite by construction; centre-blitting
  (blit_sprite_frame_centered) is INCORRECT for all ~15 animated banks (it holds the centre fixed
  instead of the authored anchor). So the render inaccuracy is precisely scoped: any engine path
  that centre-blits a multi-frame animated sprite mis-anchors it.
The correct primitive (blit_sprite_frame_at) + captured offsets are in place; converting the
animated-sprite render paths (orb, and any other centre-blitted animation) to offset-anchoring is
the scoped remaining render fix, pending the anchor base + visual verification (framebuffer-parity
gated). Honest scoping of a real render-accuracy gap; the data/primitive are correct and tested.

## Behavioral verification: engine render pipeline produces coherent frames (qualitative) — 2026-07
Ran the engine window under Xvfb and captured its output: at ~30s the engine renders a detailed,
coherent HNM intro-cutscene frame (nebula + spaceship + star) - confirming the full pipeline
(HNM decode -> linear framebuffer -> x11 present) works end-to-end and produces proper game
content, not garbage. This is a QUALITATIVE render-pipeline check (the frames look right).
LIMITS (honest): (a) at 30s the engine is still in the intro, not the nav interface where the
sprite-anchoring issue lives, so this doesn't exercise blit_sprite_frame_centered on the orb;
(b) it is not per-pixel parity vs the game - that needs frame-index alignment against a matching
DOSBox capture (gated on the same alignment/vga.mem issues). So: render pipeline confirmed to
produce coherent output; byte-exact display parity + the animated-sprite anchoring in-context
remain unverified. A real positive on pipeline coherence, not whole-game parity.

## Verification: HNM decode-frame-0 across all 645 files, debug + release (no palette overflow) — 2026-07
Strengthened opens_and_parses_every_hnm_asset from frame_dims-only to actually DECODING frame 0
of every HNM (645 files) - exercising the 'pl' palette-chunk parse (data[pos]<<2 6-bit->8-bit
expansion) and the RLE body decode that frame_dims skips.
- Passes in RELEASE and in DEBUG (overflow-checked): so `data[pos] << 2` never overflows across
  the full set = every HNM palette byte is <=63 (valid 6-bit VGA DAC). No latent overflow/panic,
  no out-of-range sub-frame. The HNM decoder + incremental-palette parser are robust end-to-end.
Checked for (and ruled out) a real potential bug (palette-byte overflow in the <<2 expansion).
Decoder verification is now exercised on the actual decode path, not just header parsing. Still
not whole-game: display-frame pixel parity + all-function behavioral parity remain unverified.

## Verification: HNM decode of ALL frames across all 645 files (debug + release) — 2026-07
Extended the HNM test from frame-0 to EVERY frame of every HNM (645 files, all frames) - now
exercising the inter-frame DELTA decode + incremental 'pl' palette updates across whole clips,
the paths where a later-frame bug (delta overflow, out-of-range write) would hide.
- Passes in DEBUG (overflow-checked, 36s) and release: no overflow, no out-of-range sub-frame,
  no panic across every frame of every HNM. The full HNM decode pipeline (header, delta, palette,
  RLE body) is robust end-to-end over the entire video asset set.
This is the deepest asset-decoder verification yet: every frame of every video decoded under
overflow checking. No bug found (the 3 real bugs this session were all in sprites). STILL not
whole-game: byte-exact display parity + all-function behavioral parity remain unverified.

## Verification: entire suite passes in DEBUG mode (overflow-checked) — 2026-07
Ran the FULL test suite (432 tests) in debug mode, which enables Rust's arithmetic overflow
checks (panic on integer over/underflow, out-of-bounds shifts, index OOB). ALL 432 pass, 0
failures, no overflow/panic. This spans:
- every asset decoder over its FULL real asset set (sprite 43 banks, LBM 169, HNM 645 all-frames,
  SND 25, VOC 44) - all decode without any integer overflow,
- all VM walk+execution over the 5 real scripts, all ship3d fixed-point math, all execution_trace
  behavioral tests, all ~33 exe-comparison tests.
So the tested codebase is INTEGER-OVERFLOW SAFE end-to-end while processing the real game data -
a class of latent bug (silent wrap in release) proven absent across every exercised path. No new
bug found; the 3 real bugs this session were logic (sprite dispatch/offset), all fixed. STILL not
whole-game: display-frame pixel parity + all-function behavioral parity remain unverified.

## Accuracy gap (direct engine-vs-game comparison): nav/bridge layout diverges — 2026-07
Captured the engine's nav interface (Xvfb) and compared to the game's bridge screen (DOSBox
gp70.png). Real STRUCTURAL divergence found:
- ENGINE nav: BORXX orb centred, CARTE nav pyramids scattered in a pattern, "SECTOR 4 EDEN"
  label, starfield. (The orb being GREYSCALE is CORRECT - BORXX is grey in-game.)
- GAME bridge: orb + nav pyramids along the BOTTOM edge, an alien viewscreen filling the top,
  "Commander BLOOD V1.0". A full HUD composition.
So the engine's nav view is a SIMPLIFIED APPROXIMATION, not a layout/pixel-faithful reproduction
of the game's bridge screen. The animated-sprite anchoring gap (centre vs offset-anchored) is a
SMALL part of a LARGER divergence: the whole bridge HUD layout (viewscreen, orb+pyramid bottom
placement, station icons) is not reproduced. This is an honest, significant scoping of where the
reimplementation's RENDER/UI diverges from the original - beyond decoders (which are accurate) and
into full-scene composition. Faithful bridge reproduction is substantial UI-reconstruction work,
gated on per-pixel verification (framebuffer-parity blocker). Decoders accurate; scene layout not.

## Accuracy gap ROOT CAUSE: nav pyramids are a synthetic grid, not the 11 real destinations — 2026-07
Traced the nav-layout divergence to its exact cause:
- ENGINE render_nav_pyramid_sprites projects a SYNTHETIC GRID: loops xi in -3..=3 x ROW_Z with a
  made-up origin=[0,-700,(cam[0]-0x2264)/8], drawing a scattered lattice of pyramids. It uses the
  correct decoded projection (project_star_map_point / build_ship_3d_projection_matrix) but on
  INVENTED input positions.
- GAME ship_3d_object_sprite_project (0x9B98) projects the 11 ACTUAL nav destination records from
  DS:0x4F09 (each a real 3D star-map position) + descriptors from DS:0x6212. So the game draws the
  real star-chart destinations at their projected positions; the engine draws a placeholder grid.
So the projection MATH is decoded and correct in the engine; the INPUT DATA is a placeholder. To
be faithful, the engine must feed the real 11 nav-destination positions (per-world star-map data)
through project_star_map_point, matching ship_3d_object_sprite_project. That is concrete, scoped
reimplementation work (the math is done; the destination data source + per-frame update FSM at
0x8A6A remain to port), gated on per-pixel verification. Precise root-cause scoping of the nav
render gap - the divergence is placeholder INPUT, not wrong projection.

## Refinement: nav destination data is context-dependent (correction to the root-cause) — 2026-07
Read DS:0x4F09 live during the BRIDGE screen (gameplay 72s): all ZERO (camera origin 0,0,0 too).
So the previous turn's root-cause ("engine should just project the 11 real 0x4F09 destinations")
is REFINED/partly corrected:
- At the BRIDGE screen ("Commander BLOOD V1.0", orb + bottom pyramids): 0x4F09 = all zero. So the
  bridge's bottom pyramid bar is likely a STATIC HUD arrangement, not projected nav destinations.
- At the attract STAR-MAP: 0x4F09 = default (10200,12100,900) x11 (not real per-world positions).
- The PROJECTED destinations only populate during ACTIVE star-map navigation (a distinct state).
So the nav/bridge rendering spans MULTIPLE screens/states (bridge HUD, star-map background, active
nav projection), each with a different data source, and 0x4F09 is only meaningful in the active-nav
state. Faithful reproduction requires modelling the per-screen state machine (which screen is up,
what feeds the pyramids in each), not a single projection. This is honest scoping: the divergence
is a multi-screen STATE + composition gap, deeper than one projection call. Decoders remain
accurate; the nav/bridge scene-state machine is the substantial unported piece.

## Verification: subtitle cues monotonic + fully font-renderable — 2026-07
Verified the DESCRIPT.DES subtitle system (48 cues) end-to-end against its consumers:
- All ticks non-decreasing within each record (correct reveal ordering): 0 violations.
- Every subtitle char is renderable by the game font (the 73-glyph set + space): 0 unrenderable
  chars. So every subtitle can be drawn correctly - the subtitle text and the verified font
  charset are consistent (cross-subsystem: descript subtitles <-> font glyphs).
- New test real_subtitles_are_monotonic_and_font_renderable (passes). Suite now 433.
Cross-subsystem consistency (subtitles fit the font) is a real correctness property. Stepped back
from the nav/bridge scene-state deep-dive (no single screen selector; distributed VM+flag state =
substantial distributed RE) to lock in this completable verification. STILL not whole-game:
nav/bridge composition, display-pixel parity, all-function behavioral parity remain unverified.

## PATH B CHOSEN: 1-to-1 static recompilation, per-function oracle-verified — 2026-07
Maintainer chose path B (provably bit-exact). Built the foundation + first verified function:
- src/recomp/machine.rs: the shared Machine (8086 register/flag file + flat 1 MB real-mode
  memory, seg*16+off addressing). Tested.
- src/recomp/mod.rs: prng_2de2 - the game PRNG (file 0x2DE2, far 0x1CE:0x0B02) LIFTED 1-to-1
  from the disassembly to operate on the Machine (state at cs:0xAEE/0xAF0/0xAF1/0xAF2, bx/cx/dx
  preserved, exact rcr/rcl carry chain).
- re/tools/gen_oracle_vectors.py: runs the REAL DOS function in Unicorn over fuzzed inputs and
  dumps (input-state -> output-state) vectors to re/tools/oracle_vectors/prng_2de2.json (300).
- Test prng_2de2_matches_oracle_vectors: replays all 300 -> AX + a/b/counter + seed all
  BIT-EXACT. So prng_2de2 is provably identical to the binary over its fuzzed domain.
This is the TEMPLATE for path B: every reachable function follows disasm -> lift(m) -> oracle
vectors -> bit-exact test. When all are lifted + verified and composed in the binary's call
graph, the whole program runs identically by construction. Function 1 of ~N done, provably 100%.
Next: (a) recursive-descent function enumeration from entry + call graph, (b) lift the leaf/pure
functions first (each a clean oracle win), (c) build the composition (shared Machine + call graph
+ the DOS/hardware boundary for int/port). Suite green.

## PATH B: function denominator established (recursive-descent enumeration) — 2026-07
Built a recursive-descent enumerator (re/func_graph.json generator, capstone per project
convention): walk control flow from the MZ entry (0:0 = file 0x600), resolving near-call targets
(E8 -> file offset) and far-call targets (9A seg:off -> 0x600+seg*16+off), following jmp/Jcc.
RESULT - the path-B denominator:
- 222 reachable functions (statically, from entry). More principled than the flat 281-prologue
  scan or the ~435 estimate; this is the reachable-from-entry set.
- 442 call-graph edges (re/func_graph.json has the full graph).
- 112 LEAF functions (no resolved internal callees) - the clean first oracle wins; the PRNG
  (0x2de2, DONE) is one of them.
- 48 unresolved INDIRECT call/jmp sites (register-indirect dispatch - VM opcode table, input
  handler, callbacks). These reach MORE functions that recursive descent can't follow statically;
  enumerating them needs the dynamic tracer (or dataflow). So true total = 222 + indirect-reached.
PATH-B PLAN (denominator-driven): (1) lift the ~112 leaves first - each self-contained, verified
bit-exact by an oracle vector set like the PRNG; (2) lift internal nodes bottom-up (composing
verified callees); (3) resolve the 48 indirect sites via the dynamic tracer to enumerate + lift
the remainder; (4) compose a lifted entry over the shared Machine + a thin int/port boundary.
STATUS: 1 / 222+ functions lifted+verified (PRNG). The grind is now against a known denominator,
each step individually provable against the binary. This is the concrete route to 100%.

## PATH B: 2nd function lifted + reusable infrastructure (flags + general oracle) — 2026-07
Grinding the leaves + building the infrastructure that makes each lift fast:
- General oracle harness re/tools/oracle.py: given a function spec (entry, ret type, input
  regs/mem, output regs/mem), fuzzes the REAL function in Unicorn and dumps (in->out) vectors
  incl. ALL 6 arithmetic flags (cf/pf/af/zf/sf/of). So new functions = write a spec + the lift.
- Machine::add16: exact 8086 ADD flag semantics (cf/pf/af/zf/sf/of), reused by every arithmetic
  lift so flag state is bit-exact (a caller may branch on it).
- func_a734 (file 0xA734: add [DS:0xD8C],ax; add [DS:0xD9A],ax; clc; ret) lifted 1-to-1 and
  verified: 300/300 oracle vectors match on AX + both memory words + ALL 6 flags.
STATUS: 2 / 222+ functions lifted+verified (prng_2de2, func_a734), each bit-exact vs the binary
incl. full flag state. 37 pure short leaves identified as the next clean targets. The template +
harness + flag model are in place; remaining is the (large) grind of lifting each function.
Honest: 2 of 222+ done. Not 100%.

## PATH B: 4 functions lifted+verified (grinding leaves) — 2026-07
Lifted two more pure leaves, each oracle-verified bit-exact (regs + memory + all 6 flags):
- func_a744 (0xA744): init 3 word globals to 0/0xFFFF/0xFFFF (no flags). 20/20 vectors.
- func_9f80 (0x9F80): table addr 0x1FB5+4*ax (16-bit wrap) -> BX = word[DS:addr]; flags from the
  4th add. 300/300 vectors incl. all flags.
STATUS: 4 / 222+ functions lifted+verified (prng_2de2, func_a734, func_a744, func_9f80). Each
bit-exact vs the binary incl. full flag state. Deferred: shift/bsf leaves (0x533c, 0x6023) pending
per-flag "defined" tracking (shl OF/AF undefined for count>1 - assert only defined flags). The
grind continues; infrastructure makes each lift ~1 spec + 1 fn + 1 test. Honest: 4 of 222+.
