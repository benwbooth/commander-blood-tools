# Commander Blood reimplementation — progress & remaining work

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
- **Remaining:** locate the actual vertex→screen projection (buried, accessed via computed
  pointers), confirm the 0x5F11 origin, then reimplement and diff vs the oracle.

### 2. Interactive scene-by-scene pixel-diff vs the running original
- Blockers diagnosed: the game reads **relative mouse** (int 33h) with DOSBox capture, so
  use `xdotool mousemove_relative` / `autolock=false`; the intro is long (60s+); crude key
  spam can exit/reboot the game.
- **Remaining:** map the intro→interactive-dialogue input flow to reach a known scene,
  then pixel-compare it to the engine's render of the same script line.

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
