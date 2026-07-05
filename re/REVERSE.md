# Commander Blood (DOS) — Reverse Engineering Notes

End goal (reframed from the generic `re` skill's "web port"): **recover the DOS
engine semantics needed to build a Rust reimplementation that runs the original
Commander Blood CD data files**. The current cutscene/video exporter remains the
first vertical slice: recover the script VM + presentation semantics, then drive
an event-based renderer in Rust. Data files remain the source of truth for
*assets*; the binary provides the *semantics* (opcode meanings, channel routing,
timing, layout, input, navigation, and game-state behavior).

See [`../docs/decompilation-roadmap.md`](../docs/decompilation-roadmap.md) for
the phase plan and definition of done.

The oracle comparison harness can now consume `accuracy/captures/capture-manifest.tsv`
via `accuracy/compare_oracle.py --reference-manifest ...`, resolving captured
frame paths and crop metadata instead of relying on hard-coded PNG paths or
default crop guesses.

See [CLAUDE.md](CLAUDE.md) for tool prefix, addressing model, and conventions.

## Binary Identification

| Field | Value |
|-------|-------|
| File | `re/bin/BLOODPRG.EXE` (from `output/CMDR_BLOOD.iso`) |
| Format | DOS MZ executable (no PE/NE/LE/LX overlay; image == whole file) |
| Platform | MS-DOS, real-mode segmented |
| CPU | 80386 required (`386 minimum !` string; uses 0x66/0x67 prefixes, eax/esi/fs/gs) |
| Memory model | NOT a flat 32-bit DOS-extender. EMS (int 67h) + XMS (int 2Fh AX=43xx) for large memory |
| File size | 86680 (0x152D8) bytes |
| Header size | 1536 (0x600) bytes = 96 paragraphs (`e_cparhdr`) |
| Relocations | 367 (`e_crlc`), table at file 0x1E (`e_lfarlc`) |
| Entry (CS:IP) | 0000:0000 → **file offset 0x600** |
| Initial SS:SP | 0CE2:7E78 |
| Startup DS | **0x0CE2** (`mov ax,0x0ce2; mov ds,ax` at entry) → data seg base = file 0xD420 |
| Startup FS | 0x0BBF (`mov ax,0xbbf; mov fs,ax`) |
| Launcher | `BLOOD.EXE` (696 bytes) — tiny loader, separate MZ |

Evidence trail (verified with `re/tools/mzfile.py`): MZ size accounts for the
whole file (`trailing_bytes=0`), so there is no hidden protected-mode payload;
no DOS/4GW / PMODE / Phar Lap / CauseWay / DJGPP / DPMI(int31) signatures; entry
code is 16-bit real-mode (`int 21h`, far calls, segment setup) with 386 register
extensions. Disassembled cleanly from 0x600.

CODE OVERLAYS (sess 003): `BLOODPRG.EXE` is not the whole engine. It loads
runtime code overlays named as data files — `amer.xdb`, `croolis.xdb`,
`scrut.xdb`, `manu3.xdb` (all referenced by name in the binary). Each `.xdb`
begins with a full 386 register-save prologue (`push eax/ebx/ecx/edx/esi/edi`,
`push ds/es/fs/gs/ebp`, `mov ax,cs …`), i.e. they are **loadable code modules**,
not asset banks (`manu3.xdb` uses a different `push ds; mov cx,cs:[…]` entry).
By name they are subsystem overlays: `scrut` = the Scruter_Jo scrutinizer,
`amer`/`croolis` = alien-species logic (dialogue/behaviour), `manu3` = a UI/manual
module. Sound is likewise a swappable driver: `nosound.drv` / `dnsdb.drv`. This
matters for *full* decompilation scope — the alien-species dialogue and
scrutinizer logic live in these overlays, loaded on demand into EMS/XMS, not in
`BLOODPRG.EXE`'s static image. (They are also why several boot/intro assets and
some runtime buffers are absent from the static image.)

OVERLAY ENTRY STRUCTURE (sess 003, `croolis.xdb`, 258KB): the file begins with a
16-bit self-relocating dispatch stub — after the register-save prologue it does
`mov ax,cs; add ax,cs:[0x32E5]; mov ds,ax; mov fs,ax` (its own data segment =
load segment + a stored delta), chains `add ax,[0x0C]/[0x0E]/[0x10]` to derive
`es`/further segments from an internal segment table at file `0x0C..0x10`, and
reads its call parameter via `les di,[bp]; mov ax,es:[di]; shl ax,3` — the same
`id*8` handle indexing as the resource manager, so the overlay is invoked with a
handle/selector and self-relocates before dispatching. Decompiling the overlay
bodies (alien-species logic + the character-palette remap that fills master
slots 224-255) is the large remaining subsystem; this entry stub is the
recovered starting point.

OVERLAY DISPATCH + PALETTE UPLOAD DECODED (sess 003, `croolis.xdb`): after the
prologue the stub reads the handle, clamps it (`shl ax,3`; if signed→0; if
`>=0x80`→0x7F; `sub 4`) and self-patches the operand at `cs:0x99`, stores the
call param `[bp+4]` at `[0x20]`, then `push cs; call 0x00A3` (the body entry) and
on return writes `(cs:[0x99]+4)>>3` (the handle) back to `es:[di]` and `retf`.
The body at `0x00A3` is the DISPLAY-INIT + FULL PALETTE UPLOAD: `mov dx,0x3C8;
xor al,al; out dx,al; inc dl; mov cx,0x300; rep outsb` — uploads 768 bytes (all
256 DAC colours, 6-bit) from `ds:0x1F6A` = **file offset 0x525A** (ds = load_seg
+ 0x32F paragraphs). Then MOUSE setup — `call 0x34B` (int 33h ax=8 set vertical
cursor range cx=0..dx=0x400, then ax=7 set horizontal range 0..0x280) and `call
0x35C` (int 33h ax=4 set cursor position cx=0x140,dx=0x200) — and inits fs globals
(`fs:0x22A8=0`,
`fs:0x22EC=0x75D`, `fs:0x22F0=0xFF11`). So the "character-palette remap" is this
overlay's own 768-byte palette at file 0x525A. Its reserved high slots are
sprite colours, NOT subtitle white: 0xE0=(23,23,17)6b, 0xFC=(44,42,20),
0xFD=(50,7,41)=magenta, 0xFE=(1,20,17)=teal, 0xFF=(0,0,63)=blue (6-bit; ×4 for
8-bit). CONFIRMS 0xFD/0xFE are CONTEXT-DEPENDENT — the overlay uses them for the
alien character sprite, while dialogue subtitles use white (see the
`apply_reserved_subtitle_palette` fix, playthrough-validated). OVERLAY BODY = OBJECT-DISPATCH MAIN LOOP (sess 003, `croolis.xdb` 0xE7..0x174):
after the palette+mouse init it (1) sets more fs function-pointer globals
(`fs:0x22F0=0xFF11, 0x22F4=0xD9C2, 0x22F8=0x678`), adjusts a pointer
`fs:0x1A = fs:0x16 - 0x26C`; (2) CLEARS all VGA planes — `mov ds,cs:[0x32E7];
mov es,[0x28]; mov dx,0x3C4; mov ax,0x0F02; out dx,ax` (Sequencer Map-Mask = all
4 planes) then `xor ax,ax; mov cx,0x1F40; rep stosw`; (3) calls four init subs
(`0x22A, 0x1E1D, 0x5DC, 0x775`); (4) runs the MAIN LOOP over an object list —
`si=0x2308; di=fs:[si]` (object ptr), `bx=fs:[di+0x34]` (object's vtable),
`call fs:[bx+0x103A]` (virtual method) then `call 0x206C`, advancing `si+=2` and
looping while `fs:[si]!=0`; (5) cleanup `call 0x2514`. So the overlay is an
OBJECT-ORIENTED, vtable-dispatched interaction screen: a null-terminated list of
alien objects at `fs:0x2308`, each with a vtable at `+0x34` whose `+0x103A` slot
is the per-object update/draw method. Init sub `0x22A` is the MOUSE CAMERA CONTROL:
`int 33h ax=3` (read mouse x=cx,y=dx,buttons=bx), subtracts the centre
(0x140,0x200), and smooths the delta into camera globals — `[0x1058]`/`fs:0x22F8`
(pan X), `fs:0x22FA`, `fs:0x22F6` (tilt Y) via halve+clamp(±5)+accumulate — then
tests buttons in `[0x2E]` (bit0=left, bit1=right). So `croolis.xdb` is an
INTERACTIVE 3D ALIEN-VIEW SCREEN: mouse-controlled smoothed camera + the
object/vtable-dispatched render loop above — structurally the same shape as the
ship-3D view (mouse delta from centre → camera, per-object draw). OVERLAY OBJECT/BEHAVIOR LOGIC DECODED (sess 003, `croolis.xdb`): the per-object
method table is at `fs:0x103A` (file 0x432A), near-ptr entries indexed by
`bx=fs:[di+0x34]` (0x1D27=`ret` null-method, plus 0xA30/0x16A4/0x12DE/0x999/0x36A).
Object records are 0x5E bytes: `+0x16`=data ptr, `+0x36`=state flag, `+0x38`=timer
(init 0x32), `+0x3C`=animation accumulator, and the 0x5E sub-record holds a
sub-method at `+0xE`. Method `0x12DE` iterates `cx=[di+0x1A]` sub-objects, `call
[si+0xE]` advancing `si+=0x5E`, gated by frame timer `cs:0xB72` (reset 7). Method
`0x16A4` is an animation state machine: a PRNG (`mov ax,fs:[0x105C]; ror ax,7; sbb
ax,0; store back`) drives object state (`[di+0x36]=1, +0x38=0x32 timer,
+0x3C=anim` from `cs:[0x16A2]` incremented by 0xFA/call), else `jmp [si+0xE]` to a
sub-behaviour. So the alien-species logic = a list of 0x5E-byte objects, each a
PRNG+timer animation state machine, dispatched per frame — feeding the same
per-object 3D draw pattern as the ship view. RECOVERED: overlay spine end-to-end
(entry→dispatch→palette/mouse init→plane clear→object loop→camera→object state
machines). REMAINING: the geometry/blit inside the sub-methods (`[si+0xE]`), the
init subs `0x1E1D/0x5DC/0x775`, and the same for `amer/scrut/manu3.xdb`.

OVERLAY 3D GEOMETRY SETUP (sess 003, `croolis.xdb` sub `0x775`): copies a 9-dword
block `ds:0x22BA -> es:0xD4C`, then reads 32-bit FIXED-POINT object coords
`[0x22EA]/[0x22EE]/[0x22F2]`, `shr e{bx,cx,dx},0xD` (13 fractional bits →
screen-space) and stores X/Y/Z at `[0xD7C]/[0xD80]/[0xD84]`, and builds a 5-entry
sprite/geometry pointer table `[0x8DA]=0x4AF, [0x8DC..0x8E2]=0x1F3A,0x253A,0x2B3A,
0x313A` (stride 0x600 = 1536B per layer). So the overlay positions its alien with
the SAME fixed-point-3D (>>13) + layered-sprite pattern as the ship-3D view —
strong evidence the overlays reuse the engine's 3D projection/blit, so decoding
one informs the ship-3D COMPOSITOR directly. NEXT: the projection math + blit that
consume `[0xD7C..0xD84]` and the 0x600-stride sprite layers.

3D PROJECTION PIPELINE RECOVERED (sess 003, `croolis.xdb` 0x7E3..0x8B4) — this is
the COMPOSITOR's core, shared by overlays and the ship-3D view:
1. PRNG jitter: for each axis `esi=ror7;sbb0; sub {bx,cx,dx},si` (small per-frame
   random shimmer on the object position). Then `movsx` X/Y/Z to 32-bit.
2. 3x3 ROTATION MATRIX x vector (matrix at `ds:[0xD4C..0xD6C]`, 9 dwords loaded by
   sub 0x775): `X' = [0xD4C]·X+[0xD50]·Y+[0xD54]·Z`, `Y' = [0xD58]·X+[0xD5C]·Y+
   [0xD60]·Z`, `depth ebp = [0xD64]·X+[0xD68]·Y+[0xD6C]·Z`; `sar ebp,8`; cull if
   `depth<=0` (behind camera, `js/je -> skip`).
3. PERSPECTIVE DIVIDE: `screen_x = idiv(X',depth) + 0xA0` (centre 160), cull if
   `<0 || >=0x140` (320); `screen_y = -idiv(Y',depth) + 0x64` (centre 100), cull if
   `<0 || >=0xC8` (200). So the view is 320x200, principal point (160,100).
4. VGA PLANAR PLOT: `dx=y; dx<<=6; dh+=al (=> y*320); dx+=x; plane=x&3; byte_off=
   dx>>2` — 4-plane unchained mode-X-style addressing (matches the Sequencer
   map-mask writes at 0x12D). 
This is the recovered 3D projection the ship-3D COMPOSITOR needs: rotation-matrix
transform + perspective divide about (160,100) + planar plot, with the matrix in
overlay data at [0xD4C]. Decoding the overlay delivered the compositor's math.
CROSS-VALIDATION + CORRECTED COMPOSITOR SCOPE (sess 003): the Rust `ship3d`
module ALREADY implements this projection — `project_ship_3d_point` does
matrix×vector + perspective divide about `SHIP_3D_PROJECTION_SCREEN_CENTER_X/Y`
(0xA0,0x64) with a depth≤0 cull, and `render_ship_3d_point_cloud`/`_starfield`
render the background layer. The overlay projection is a SECOND, independent
instance of the same engine 3D projection (structurally identical), confirming
the module's approach. Per-routine scaling DIFFERS (overlay: depth`>>8`, no axis
pre-shift, Y negated before +100; `ship3d`: depth`>>15`, axis`>>7`, no negation)
— these are two distinct projection routines (croolis overlay vs the ship-3D
code the module was lifted from), so NOT a bug to reconcile. CORRECTED SCOPE: the
ship-3D compositor's PROJECTION + starfield background ARE implemented; the real
remaining compositor gap is SPRITE COMPOSITING — drawing the projected sprite
slots (the 0x600-stride layers here; `BORXX.SPR`/character `.spr` in the ship
view) over the background and HUD. NEXT: the sprite blit around the projected
point (overlay sub-method `[si+0xE]`) → the ship-3D sprite-slot compositor.

COMPOSITOR STATE — ACCURATE ASSESSMENT (sess 003): reading the code, the ship-3D
compositor's ALGORITHMIC pieces are ALL implemented AND unit-tested, not
"untouched":
- projection — `ship3d::project_ship_3d_point` (matrix×vector + perspective
  divide about 160/100 + depth cull), cross-validated by the croolis.xdb overlay.
- background — `render_ship_3d_point_cloud` / `render_ship_3d_starfield`
  (PRNG point cloud + depth-shaded plot).
- sprite frame decode — `Ship3dSpriteSlotFrame` Raw/Rle/Scaled +
  `ship_3d_sprite_slot_frame_for_dispatch` (dispatch 0-4).
- sprite blit — `blit_ship_3d_sprite_slot_command_indexed` (flip, clip,
  destination remap tables 5F11/6011).
- FULL COMPOSITOR — `render_ship_3d_dirty_sprite_commands_indexed`: iterates slot
  commands, blits each to the secondary buffer, then dirty-rect copies to primary
  (double-buffered, matching the engine). Tested at render.rs 1648/2215/2261/2300.
So the REAL remaining compositor gap is INTEGRATION, not algorithms: drive these
from the live ship-3D slot state (slot table → project each slot's 3D pos into a
`Ship3dSpriteSlotRenderCommand` with dispatch/rect/flip → call the dirty compositor
→ emit the frame over the starfield + HUD). This means the "compositor" lever is
mostly DONE (pieces done+tested); it needs wiring into the export, which is far
smaller than the earlier "implement a compositor" framing implied.

FULL PIPELINE IS PRESENT (sess 003, final assessment): the projection→descriptor
step ALSO exists — `project_ship_3d_object_sprite` (ship3d.rs 1776) projects an
object's 3D position, centres it (`draw_x = screen_x - extent_width/2`, same for
y) and updates the slot descriptor. So the WHOLE ship-3D render chain is
implemented AND tested: `project_ship_3d_object_sprite` → set descriptor draw_x/y
→ `collect_ship_3d_dirty_sprite_slot_render_commands` → `render_ship_3d_dirty
_sprite_commands_indexed` (double-buffered), over `render_ship_3d_starfield`. The
ship-3D COMPOSITOR is therefore ALGORITHMICALLY COMPLETE — the remaining work for
an actual in-game ship-view frame is (a) the ship-nav VM STATE that supplies which
objects/slots are live and their 3D positions (the nav FSM is partly mapped:
steering `0x7824`, on-ship flag `0xB079`, mouse poll `0:0x70E`), and (b) wiring
the chain into the exporter to emit the frame. Not new rendering algorithms.

OVERLAY FAMILY MAPPED (sess 003): the four `.xdb` overlays split into two shapes:
- `croolis.xdb` (258KB), `amer.xdb` (266KB), `scrut.xdb` (258KB) ALL share the
  SAME entry stub (push eax..ebp/ds/es/fs/gs; `mov ax,cs; add ax,cs:[0x32E5]`
  self-relocate; `ds_delta@0x32E5` = 0x32F/0xD404/0xDCE respectively). So the full
  croolis decode (dispatch→palette+mouse init→VGA-plane clear→object/vtable loop→
  mouse camera→PRNG animation state machines→3x3 matrix 3D projection) is the
  SHARED template for all three — they are the alien-species/scrutinizer
  INTERACTIVE-3D-VIEW overlays (amer, scrut=Scruter_Jo, croolis). Remaining
  per-overlay work is just their specific object data / vtable contents, not new
  structure.
- `manu3.xdb` (62KB) is a DIFFERENT shape — a menu/manual overlay: `push ds; mov
  cx,cs:[0x136A]; or cx,cx; je …; mov fs/ds/es,cx; mov eax,[bp]→[0x1A]; mov
  ax,[bp+6]; shr ax,4; add ah,0xA0` (parameter → segment-style address). BUT its
  body (0x34..0x8B) does the SAME core: cursor position relative to screen centre
  (`ax=[0x1A]-0xA0; +ax; → [0x23E4]`, `bx=[0x1C]-0x64; +bx; → [0x23E2]`) and then
  the SAME 3x3 MATRIX×VECTOR 3D projection (`es:[0x2AC/0x2AE/0x2B0]` coords × matrix
  `[di+0x2A]/[di+0x2E]/…`). So manu3 is a 3D MENU overlay (likely the pyramid-nav
  HUD) that SHARES the engine 3D projection core.
CONCLUSION: ALL FOUR overlays + the ship-3D view use ONE shared 3D projection
(matrix×vector about principal point 160,100) — the compositor math is universal.
The overlay-STRUCTURE survey is COMPLETE (3 alien-view overlays on the croolis
template + manu3 the 3D menu). Outstanding overlay work is only per-overlay DATA
(object lists / menu items / vtables), not new engine structure. The palette buffer at file 0x525A is the character-sprite
palette source; `amer.xdb`/`scrut.xdb` share the same entry-stub shape.

OVERLAY OBJECT RECORD LAYOUT (sess 003, `croolis.xdb` methods 0x16A4/0x12DE/0xA30):
the per-object sub-record (`si = [di+0x16] + 0x5E`) fields are mapped: `+0x36`=
state flag, `+0x38`=timer (init 0x32), `+0x3C`=anim accumulator (PRNG), `+0x42`=
position X, `+0x46`=position Y (`-0x3C` view adjust), `+0x4A`=position Z, `+0x50`=
frame counter, `+0xE`=sub-method ptr. Method `0xA30` is the POSITION-UPDATE +
VIEW-CULL: adds the camera position (`[0x22EC]/[0x22F0]/[0x22F4]`, set by mouse
camera sub 0x22A) to the object position and bounds-checks (`0x80`, `0xFF00..0x100`)
to cull off-view objects before the shared 3D projection plots them. So croolis is
decoded comprehensively — dispatch, init, main loop, mouse camera, object record
layout, PRNG animation, position-update+cull, 3D projection — the alien-species
subsystem is mapped end to end (same template for amer/scrut). Outstanding: only
the raw sprite-layer pixel/geometry bytes (0x600-stride layers at file ~0x522A)
and the manu3 menu-item data.

OVERLAY RENDERING = 3D POINT CLOUD (sess 003, `croolis.xdb` 0x8B4..0x8FC) — the
UNIFYING decode: the alien is drawn as a depth-coloured 3D POINT CLOUD, the SAME
technique as the ship-3D starfield. Per projected point: compute VGA planar addr
(`dx = (y*320+x)>>2`, `di = x&3` plane) + depth colour (`ebp>>0xF`), and append the
4-byte pair `(addr, colour)` to that plane's point-list — the `0x600`-stride
"layers" `0x1F3A/0x253A/0x2B3A/0x313A` (file ~0x522A) ARE the 4 VGA-plane point
lists (up to 0x600/4 = 384 pts each). Then the BLIT walks each plane list: `dx=
0x3C4; ax=0x0102/0x0202/0x0402/0x0802 out` (Sequencer map-mask → plane 0..3) and
for each entry `es:[addr] = [colour + 0x7D6]` — a depth→palette lookup at
`ds:0x7D6` = file 0x3AC6 (extracted). So aliens AND the starfield use ONE
point-cloud renderer (project pts → per-plane lists → planar plot → depth colour);
this is exactly `ship3d::render_ship_3d_point_cloud`. The engine has a SINGLE
3D-object pipeline (matrix projection + point-cloud planar plot) reused across the
ship view and all overlays. Overlay RENDERING is now fully understood; the only
overlay data left is the artistic depth-colour table contents and manu3's menu.

SHIP-3D NAV VIEW — ACCURATE COMPLETION (sess 003): the ship-3D view's components
are ALL implemented AND tested in `ship3d`, not "needs runtime modelling":
- background: `render_ship_3d_starfield` (point cloud) — runnable via `--ship3d`.
- 3D object compositor: `project_ship_3d_object_sprite` → `collect_..._commands` →
  `render_ship_3d_dirty_sprite_commands_indexed`, wired in `compose_ship_3d_scene
  _indexed` — runnable via `--ship3d` (starfield + projected object).
- DESTINATION selector: `layout_ship_3d_target_list` (data-driven from the
  destination label widths → list-box x/y/w/h), `hit_test_ship_3d_target_list`,
  `Ship3dTargetDrawCommand/DrawResult`, `Ship3dTargetSelectorState/Selection` —
  the pyramid-nav destination MENU, laid out from the location names (available in
  DESCRIPT). Transition control (open/close steps `SHIP_3D_TRANSITION_*`) present.
So the REAL remaining "game-accurate ship view" gap is INTEGRATION WIRING — feed
the current location's destination list (names → labels → layout) + the live
object slots into the (already-runnable) compositor and emit the frame — plus the
per-frame nav-FSM state (steering `0x7824`, on-ship `0xB079`). This is bounded
wiring of tested components with available data, NOT new rendering or open-ended
runtime modelling. The ship-3D subsystem is substantially COMPLETE.

SHIP-NAV VIEW COMPOSITION — VERIFIED vs PLAYTHROUGH (sess 003): the real in-game
ship-nav view (playthrough t85/90/130) is NOT a starfield + text menu. It is:
(1) a SCENE in the upper band — a character/creature over a background, RENDERED
THE SAME WAY as the dialogue scenes (character HNM over scene background) which are
already implemented + verified against the playthrough; plus (2) the PYRAMID-NAV
HUD at the bottom — the grid of grey pyramids + the central eye-orb (BORXX.SPR,
decoded). The green "1" day counter is top-left. So the ship-nav view largely
REUSES the verified dialogue scene rendering for its scene band; the only
additional visual is the pyramid-HUD overlay (pyramid grid + BORXX orb, both
decodable). The starfield point-cloud renderer is a DIFFERENT element (space/warp
background), and `layout_ship_3d_target_list` is a separate on-interaction text
selector, NOT the main HUD. CORRECTED ship-view scope: scene band = done (dialogue
renderer); remaining ship-view visual = compose the pyramid-HUD overlay (grid +
orb) over the scene, driven by the destination list. Bounded sprite compositing,
not new rendering.

SHIP-HUD PARTS — PRECISELY SCOPED (sess 003): the pyramid-nav HUD decomposes into
three parts, TWO of which are already done/available:
1. ANGLE/compass update — routine @file 0x9656 (`ship_3d_procedural_angle_update`,
   gated by `[0x2793]&8`, angle math 0xB4/0x5A) is ALREADY implemented + tested in
   `ship3d.rs`. Done.
2. ORB animation — `pe/eye01..10.hnm` + `pe/eyeer.hnm` (NOT BORXX.SPR): CONFIRMED
   decodable via the existing HNM pipeline (`--hnm pe/eye01.hnm` = 31 frames ->
   MP4). Available now; just needs compositing into the HUD band (rows ~146..193,
   centred). Done/available.
3. PYRAMID GRID pixels — the ONLY remaining ship-HUD RE piece: a separate
   procedural pixel-drawing routine (uses the 0x9656 angle to draw the grey
   pyramid grid). 0x9656 is the angle math, not the pixel plot; the plot routine
   is still to be located + decoded. This is bounded RE (one routine), not runtime
   state. So the ship-HUD is ~2/3 done (angle + orb); remaining = the pyramid-grid
   pixel routine.

## Memory Map (load image, base segment 0)

| Region | File range | Notes |
|--------|-----------|-------|
| MZ header | 0x000000–0x000600 | header + 367-entry relocation table |
| Code (hypothesis) | 0x000600–0x00D420 | relative segments 0x000–0xCE1; far-linked (large model) |
| Data segment (DS=0x0CE2) | 0x00D420–0x0152D8 | string table starts here (`386 minimum !` at 0xD420 = DS:0x0000) |
| └ dialogue font | 0x14C22–0x14D28 | ASCII map @0x14C22 (DS:0x7802), advances @0x14CD2, 8-byte glyphs @0x14D28 |

The code/data split is a working hypothesis from the startup DS value; large
model can interleave additional code segments — verify as functions are found.

### Segment map (recovered via `re/tools/dump_segments.py`)

10 code segments + 2 data segments (relative bases). Far calls (9A/EA) target
these; use `dump_segments.py --contains <imgoff>` to map an offset to its segment.

| Seg base | File range | Notes / known contents |
|----------|-----------|------------------------|
| 0x0000 | 0x00600–0x00EB0 | entry/startup |
| 0x008b | 0x00EB0–0x022E0 | |
| 0x01ce | 0x022E0–0x02F90 | |
| 0x0299 | 0x02F90–0x05190 | **render_string @:0202 (0x3192)**, text renderer |
| 0x04b9 | 0x05190–0x053A0 | |
| 0x04da | 0x053A0–0x077E0 | **VM: token_advance @:0F16(0x62B6), token_walker @:1FCF(0x73AF)** |
| 0x071e | 0x077E0–0x09D10 | **sub-dispatch tables cs:0x06D4, cs:0x0F29** |
| 0x0971 | 0x09D10–0x0AFA0 | |
| 0x0a9a | 0x0AFA0–0x0B7B0 | |
| 0x0b1b | 0x0B7B0–0x0C1F0 | |
| 0x0bbf | 0x0C1F0–0x0D420 | FS data segment |
| 0x0ce2 | 0x0D420–0x152D8 | DS data segment (strings, font, tables) |

## Data-Range Map

| Start | End | Size | Classification | Notes |
|-------|-----|------|----------------|-------|
| 0x000000 | 0x000600 | 0x600 | header+relocs | MZ |
| 0x000600 | 0x00D420 | 0xCE20 | code (unverified extent) | entry at 0x600 |
| 0x00D420 | 0x0152D8 | 0x7EB8 | data | strings, font, tables |
| 0x014C22 | 0x014D28 | 0x106 | asset: dialogue font | confirmed (README + screenshots) |

## Key Findings

### Architecture

- Entry @0x600: sets DS/SS=0x0CE2, SP=0x7E78, zeroes 32-bit regs (edi/esi/ebp/ebx),
  `call 0xCCB` (early init, returns status in AX), then sets FS=0x0BBF, GS=DS.
- Far-linked program (large model expected) → use FAR call/jmp (9A/EA) xrefs
  (`re/tools/xref.py SEG:OFF`) as the primary call-graph tool.

### Script VM — bytecode model (PARTIALLY MAPPED)

The scripts are **compiled BASIC**: every `scriptN.cod` has a matching
`scriptN.bas` source (string table at file 0xCE14+). The VM executes the
`.cod` bytecode. Filenames also confirm `son.snd` (voices/SFX) at DS:0x00A6 and
`mus.snd` (music) at DS:0x00AE as the two audio banks.

**Opcode encoding** (decoder at file `0x62B6`, `token_advance`):
- Opcode bytes are **biased by 0xA0**: valid opcodes are `0xA0`–`0xD3`.
  (`sub al,0xa0` appears exactly once in the whole binary, at 0x62BF — so the
  one shared decoder front-ends all token walking.)
- Per-opcode descriptor table at **`DS:0x6F18` (file `0x14338`)**, 2 bytes per
  opcode, indexed by `(op-0xA0)`:
  - `byte0` = token length (bytes incl. opcode) in **mode 0**.
  - `byte1` = token length in **mode 1**, *unless* its high bit is set, in which
    case it is a **mode-control sentinel**:
    `0xFF`→enter mode 1; `0xFE`→enter mode 0; `0xFD`/`0xFB`→if next byte==`0xA1`
    skip it (conditional). Current mode is held in `gs:[0x67AD]`;
    `gs:[0x67B2]&1` gates the 0xFB case.
- `token_advance` computes the length then does `dec al; add si, ax` (one byte
  already consumed by `lodsb`). Length 0 ⇒ special:
  - `0xA6` (TEXT call): skip 5 bytes, then a **0x0000-terminated word list**
    (matches README's `0xa6 … 00 00`).
  - other length-0 opcodes (`0xA8 0xAC 0xCC 0xD3`): call `0x6293`.

**Recovered opcode length table** (mode0 / mode1; `*`=mode-control sentinel,
`var`=length-0 special). Dump with `re/tools/dump_opcode_table.py`:

| op | m0 | m1 | | op | m0 | m1 | | op | m0 | m1 |
|----|----|----|-|----|----|----|-|----|----|----|
| A0 | 3 | * | | AE | 5 | * | | BC | 5 | * |
| A1 | 1 | * | | AF | 5 | * | | BD–C0 | 7 | 7 |
| A2 | 3 | 3 | | B0 | 5 | * | | C1–C8 | 5 | * |
| A3 | 3 | * | | B1 | 7 | 7 | | C9 | 3 | * |
| A4 | 3 | 3 | | B2/B3 | 5 | * | | CA | 5 | 5 |
| A5 | 4 | 2 | | B4–B6 | 7 | 7 | | CB | 6 | 6 |
| A6 | var(TEXT) | | B7 | 4 | * | | CC | var |  |
| A7 | 3 | 3 | | B8/B9 | 7 | 7 | | CD | 7 | * |
| A8 | var | | | BA/BB | 5 | * | | CE–D1 | 1 | 1 |
| A9 | 4 | * | | | | | | D2 | 2 | 2 |
| AA | 1 | 1 | | | | | | D3 | var |  |
| AB | 4 | 4 | | AC | var | | | | | |
| AD | 5 | 5 | | | | | | | | |

`0xC4` is a confirmed 5-byte token: `C4 <record:u16> <related:u16>`. The first
word is the record offset the Rust extractor uses as `object_offset + 0x3A` for
speaker tracking; the second word is a related record offset consumed by the DOS
handler. The core VM token exposes both operands.

`token_walker` at file `0x73AF` uses this decoder to scan a script for `0xA6`
text tokens matching an index `bx`, via far pointers `lds si,[gs:0x6720]` /
`les di,[gs:0x6724]` (a per-script COD/offset table).

### 0xA6 TEXT token — parameter block (DECODED from handler + data)

Layout (confirmed against SCRIPT1/2.COD with `re/tools/dump_text_tokens.py`):

    A6  b1 b2 b3 b4 b5   w0 w1 ... wN  0x0000

- `b1,b2`: u16 offset into the runtime line/object table at `gs:0x6724`.
- `b3`: **voice / dialogue-line selector**. The handler stores
  `sign_extend(b3)` to `gs:0x1FAB`; the active line id is `b3 + 9`. `0xFF` and
  `0x00` are silent/no-voice channels; `1..=N` maps to one-based talk/voice
  clips when the current actor supplies that many clips.
- `b4`: **display/animation/control bits**. Decoded bits include `0x01`
  preserve the active/display flag after accepting the line, `0x04` skip one
  extra control word before dictionary words, `0x08` conditional skip count
  from `b5`, and `0x10` loop target word.
- `b5`: flags; **bit 0x80 = engine "active" flag** (set in-place by
  `token_walker` via `or [si+4],0x80`). After accepting a line, the handler
  clears this bit in the COD stream unless `b4 & 0x01` is set. Rust models this
  with `text_flags_after_accept()` and per-token runtime flags in `execute_trace`.
- `w*`: u16 **dictionary-word offsets** into `SCRIPT*.DIC`, `0x0000`-terminated.
  A `0xFFFF` word appears occasionally — likely an inline marker, not a real
  dict offset (verify in handler).

Remaining A6-related work: finish the callback/media routing around accepted
lines so `b4/b5` presentation bits drive the exact talk animation, subtitle
sound, and wait behavior instead of only the current event-schema fields.

### Dialogue scene LAYOUT (from real-game oracle capture + 0x3D7B)

Real frames (`accuracy/captures/`, `BLOODPRG.EXE` direct boot) show the dialogue/
location view is **letterboxed**, NOT full-screen:
- Scene region = framebuffer rows **`gs:0x5239`..`gs:0x523B`** (the scene clear at
  file 0x3D7B fills exactly this band of `gs:0x5221`). Letterbox values seen:
  `0x23..0xA5` (rows 35..165, a 130px band) — which matches the 320x130 talk-HNM
  update frames.
- Top rows (0..~35) are black; bottom rows (~165..200) are the **HUD panel**
  (the pyramids + central "eye" navigation orb).
- The **landscape** fills the scene band; the **character** (talk HNM) is
  composited *in the band*, positioned within the scene — NOT jammed at (0,0).
The current exporter renders character+background full-screen at (0,0) with no
letterbox/HUD → the placement/clipping the user reports. Fix = render the scene
in rows `gs:0x5239..0x523B` (character HNM at y≈35 over the landscape), matching
the captured layout. (A green "1" overlay = a scene/debug index in direct-boot.)

**TWO render modes (sess 002, confirmed by disasm + `accuracy/captures/`):**
- **Dialogue/location view** (talking head over landscape): letterboxed AND the
  bottom shows the **HUD panel** (perspective pyramid grid + animated central
  "eye" navigation orb). Native-320×200 layout from captures: black top rows
  0..~40; scene band (landscape+character) ~41..145; 1px divider @145; **HUD panel
  rows ~146..193**; black 194..199. So the lower "band" is NOT a black bar — it's
  the HUD. Gated by `gs:0x2793 & 8` (tested @file 0x1018/0x965D/0x9733/0xB193).
  The band top `gs:0x1fa7=0x23` (@0xB3FA), band height 0x82=130 (`gs:0x523B = top
  + 0x82` @0x9DC6). The `gs:0x5239=0x23/0x523B=0xA5` window set when line id==5
  (@0x9EC0) is a TRANSIENT clip window around blits, then restored to 0/0xC8 —
  not a persistent letterbox.
- **Full-HNM cutscene** (e.g. Mindscape logo, astronaut `frame_04`): single
  centered full-frame HNM, **NO HUD** (capture bottom-region mean ≈6.5 = black).
  This is the exporter's `letterbox==false` path and is already correct.
- **HUD asset**: engine-rendered, NOT a single static LBM. Orb = `pe/eye01..10.hnm`
  / `pe/eyeer.hnm` (+ `pe/borb_*`/`bor_*` orb-rotation frames) in BLOOD.DAT's `pe/`
  bank (same bank as the talk-head HNMs); the pyramid grid is drawn procedurally
  by the UI/compass routine @file 0x9656 (angle math 0xB4=180°/0x5A=90°). There is
  no single "pyramids panel" bitmap to blit.
- **Character compositing**: fixed band-relative origin (band top y=35); talk-HNM
  update frames are authored 320×130 to fill the band. NO per-HNM x/y offset
  found in the blit path (uses band clip + buffers `gs:0x5221/0x5229/0x5219`). The
  current fixed-at-band-top compositing is correct.
- **Renderer gap**: in dialogue mode the exporter fills rows ~146..193 with BLACK
  instead of the HUD. Reproducing it faithfully needs either (a) RE'ing the
  procedural pyramid grid + orb HNM animation, or (b) compositing the HUD region
  from a real capture as a static overlay (real pixels, but frozen orb; arguably a
  heuristic). full-HNM path is already correct.

**ON-SHIP vs PLANET classification — RESOLVED (sess 002): NOT statically derivable.**
The HUD is the *player's-ship* pyramid-nav interface; it appears for on-ship
dialogue, not planet/surface dialogue. Gating flag = `gs:0x2793 & 8` (bit 3 =
"ship pyramid-nav UI / HUD active"). All 10 writers of bit 3 (setters @0xB0B7,
0xB505, 0x86B6, 0x7DE1; clearers @0xAFC6, 0x79B4, 0x0FC8, 0x1A5E, 0x187C; toggle
@0x9671) live in the engine's interaction/scene FSM (driven by `gs:0x24F3`,
`gs:0x2535`, the 6-entry nav table `gs:0x2A1B`) — **NONE in the COD VM handler
region**, so no script/DESCRIPT byte maps to it. on-ship/planet is therefore pure
runtime navigation state, not a stored per-scene field. DESCRIPT shape does NOT
encode it either: enemy *spaceships* (Kukaracha/Kraner/Shark) use the SAME 4-LBM
"surface" Location shape as planets, and the player's own ship interior is NOT a
Location record at all (on-ship close-up lines have empty `background_record`).
**Best static proxy = the one the exporter already uses**: a line resolving to a
`kind==1` Location LBM → planet/surface (letterbox scene-band, NO HUD, the
majority — 75/83 dialogue videos); no location → on-ship/narrator close-up
(full-screen — 8/83). That proxy is CONFIRMED correct for scene-band vs
full-screen. But it does NOT cleanly isolate "show HUD": "no-location" conflates
on-ship-crew (HUD) with narrator/intro close-ups (no HUD), so the HUD cannot be
reliably placed from static data. CONCLUSION: the procedural-3D HUD is a minority
case (≤8/83 videos) that can't be statically gated; not built. The validated
letterbox logic (`letterbox = landscape_lbm.is_some()`) stands as correct.

**Ship/procedural-3D path markers (new RE target):**
`inspect-bloodprg.presentation_3d_markers` now pins the currently-known binary
entrypoints and state variables for the ship/HUD 3D path. The engine enters the
ship/navigation presentation FSM at `0x0A9A:0x0000` (file `0xAFA0`). The branch
at file `0xB079` sets `DS:0x2793 |= 8`, initializes ship HUD/procedural-3D
state, copies framebuffer pages, and calls the plane-band updater. The temporary
`3D.snd` path starts at file `0xB591`: if `DS:0x0AE4` is set, it cycles
`DS:0x0AE5`, loads `sn\3D.snd` through the SND bank loader at file `0xB5DC`,
runs the presentation callback, then restores `sn\tb.snd` at file `0xB610`.
The likely visual-core markers are file `0xB692` (transition state update),
`0xB6DD` (VGA planar page-band copy gated by `DS:0x252E`), and `0xB75C`
(moves depth/plane offset `DS:0x2527` toward the active target using
`DS:0x2531`). This is strong evidence for a runtime ship/procedural-3D
subsystem, but not yet enough to implement the minigame: the projection,
geometry/object state, and input loop still need to be decompiled from these
entrypoints and their xrefs.

`src/ship3d.rs` now ports the local state effects of the `0xB591` temporary
`3D.snd` branch. The one-shot gate `DS:0x0AE4` clears itself plus
`DS:0x0AE3`, selects one of the three DS-table offsets at `DS:0x0ACC`
(`0x0087`, `0x0090`, `0x009C`), cycles phase byte `DS:0x0AE5` modulo 3, loads
`sn\3D.snd` from `DS:0x0D23`, calls the presentation callback through
`DS:0x0A96`, then restores `sn\tb.snd` from `DS:0x0CFC`. The port exposes the
mouse-coordinate preservation, callback-bank gate reset, hold-timer reset, and
fullscreen descriptor write. If navigation sequence byte `DS:0x252A` is set,
the branch temporarily disables plane copying, re-enables `DS:0x252E`, and
resets `DS:0x1FA3=-1`; otherwise it runs the non-sequence redraw path and clears
the setup latches `DS:0x5B53/0x5B57`.

The final reset path at file `0xB4F2..0xB586` is now split out as
`run_ship_3d_navigation_final_reset()`. It only runs when exit-pending byte
`DS:0x2532` is set and opening byte `DS:0x252F` is clear; if opening is still
set, the binary re-enters the active sequence branch. The reset path restores
HUD flags `DS:0x2793=9`, clears the choice hold timer, writes `DS:0x279D=0x32`,
sets gates `DS:0x27D9/0x2739`, clears dialogue/scene/presentation bytes
`DS:0x24F3`, `0x1FA7`, `0x1FB2`, `0x2532`, `0x2529`, `0x5E64`, `0x67B0`,
`0x67BC`, `0x252E`, `0x252A`, masks `DS:0x67AA &= 0xFC`, clears `DS:0x67BA`,
sets sentinels `DS:0x1FAB=0xFFFF` and `DS:0x6788=0xFFFF`, restores/clears the
backbuffer scratch blocks, sets dirty marker `DS:0x5B52=0xFF`, and resets ship
scroll state to `DS:0x524F=0`, `DS:0x524D=0x000A`.

The ship sequence's procedural update far call (`0x071E:0x1E76`, file
`0x9656`) is now identified as a HUD angle/mouse-ring state update, not the
heavy projection loop itself. It consumes angle `DS:0x2795`, HUD flags
`DS:0x2793`, mouse X/Y `DS:0x0A2A/0x0A2C`, hold target `DS:0x279B`, and timer
`DS:0x279D`. It wraps the internal mouse ring through the 1440-unit range
`0x05A0`, updates sector `DS:0x2797 = mouse_x >> 2`, records direction in
`DS:0x27DB`, writes projection angle `DS:0x2F6D`, computes rotation offset
`DS:0x27A7 = angle * 8 - 0xA0`, and aligns mouse X to an 8-unit boundary before
subtracting that offset. The target-list bit `DS:0x2793 & 4` switches the
inactive-HUD branch from angle auto-rotation to cursor repositioning.

The matrix builder at file `0x98B9` is now identified as the 3x3 fixed-point
projection matrix setup. It reads cosine/sine pairs from table `DS:0x4F45`
(now RECOVERED, sess 003: a static 180-entry `(cos,sin)` trig table, 2°/entry,
Q14 amplitude `0x4000`, embedded byte-exact as `SHIP_3D_ANGLE_TABLE` in
`src/ship3d.rs` with a binary-verification test — the matrix builder doubles
each value to Q15),
doubles them from `0x4000` to `0x8000` scale into the scratch vector at
`DS:0x2F7D`, consumes angle words `DS:0x2F71`, `DS:0x2F6D`, and `DS:0x2F6F`,
then writes nine signed dwords at `DS:0x2F95`. The multiply order is preserved
as two-operand `imul` plus arithmetic `sar 0x0F`; the three compound terms reuse
the intermediate `(b_sin * c_sin) >> 15` and `(c_sin * b_cos) >> 15` products
before the final shift.

POINT-CLOUD SOURCE RESOLVED (sess 003): the 1000-point buffer at `DS:0x2FC1` is
**all zeros in the shipped image** — it is populated at runtime by
`ship_3d_point_cloud_randomize` (`0x9B67`), which loops `cx=0x3E8` records and,
per record, calls the engine PRNG `far 0x01CE:0x0B02` **three times** (x/y/z,
each `rng(0xFFFF)`) then `add di,2` to skip the 4th word. So the ship-3D
"corridor" background is a **procedurally random 3D starfield**, not a fixed
geometry model. The PRNG at `0x01CE:0x0B02` is an LFSR-style generator with
CS-segment state at `cs:0x0AEE` (seed word, XOR-only), `cs:0x0AF0/0x0AF1`
(mixing bytes, advanced each call from the `cs:0x0AF2` counter); it returns
`value % modulus`. Both the PRNG and the randomizer are ported+tested in
`src/ship3d.rs` as `BloodPrng`/`randomize_ship_3d_point_cloud`, and the whole
`0x9A10` batch loop is now `render_ship_3d_point_cloud()` (projects every point
through the camera matrix and depth-shades a `320*200` write-once buffer). IMPORTANT for
oracle strategy: the state bytes are zero in the image, but the seed routine at
`0x2DD3` reads the **CMOS RTC seconds** register (`xor al,al; out 0x70,al;
in al,0x71; mov ah,al; mov cs:[0xAEE],ax`) into the XOR seed word — so the seed
varies with the wall-clock second at boot and **the exact star positions are not
reproducible run-to-run**. Ship-3D background validation against a capture must
therefore be **statistical/structural** (point density, depth-shade
distribution, sprite anchors), not exact-pixel. (`BloodPrng::seed_word` models
this; a faithful live seed would set it to `secs<<8 | secs`.) The corridor floor/walls and the
HUD (pyramid grid + `pe/eye*.hnm` orb) are separate layers, still to be composed.

The point-cloud projection loop at file `0x9A10` and its pixel helper at
`0x9B04` are now split out in `src/ship3d.rs` as `project_ship_3d_point()` and
`plot_ship_3d_projected_point()`. The loop walks 1000 eight-byte records from
`DS:0x2FC1`, copies each point into scratch `DS:0x4F01`, subtracts camera origin
words `DS:0x2F65/0x2F67/0x2F69` with word wrapping, sign-extends X/Y/Z, and
projects through matrix `DS:0x2F95`. Depth uses row 3 shifted by 15 and skips
zero/negative results. X/Y use rows 1/2 shifted by 7, signed-divided by depth,
then centered at `(160,100)` before writing scratch words
`DS:0x2FB9/0x2FBB/0x2FBD`. The `0x9B04` helper clips against
`DS:0x5235..0x523B`, computes `y * 320 + x`, only writes empty depth-buffer
pixels, and encodes shade as `0xEF - (depth >> 12)`. The object/sprite
projection path at `0x9B98` is now partially recovered as
`project_ship_3d_object_sprite()`: it walks eleven three-word anchor records at
`DS:0x4F09` with a 6-byte stride, uses descriptor records
`DS:0x6212 + ((counter + 0x15) << 5)`, gates on descriptor flag `0x0080`,
reuses the same matrix rows and screen centers, wraps negative depth by adding
`0x10000`, computes scale `(0x08000000 >> 7) / depth`, scales source
width/height by `>> 10`, calls `sprite_slot_extent_update` (`0x0299:0x133D`),
then subtracts the updated descriptor extent words `+0x0C/+0x0E` before calling
`sprite_slot_position_update` (`0x0299:0x127D`). Those two helpers are now
modeled as mutable slot-state updates: active slots are gated by flag mask
`0x0081`; position changes set dirty bit `0x0002`; scaled extents set dirty bit
`0x0002` plus extent-changed bit `0x0010`; and natural-size extents clear bit
`0x0010`, marking dirty only if that bit had been set.

SPRITE PIXEL-DATA SOURCE — the compositor blocker (sess 003): the 11 slot
descriptor records at `DS:0x6212` (0x20 bytes each) carry position/extent/dirty
state and a pointer to the slot's frame table, but the frame-table PIXELS are a
runtime in-memory structure (the `SpriteSlotFrameTable` / `.spr` layout in
`render.rs`). There are **no `.spr` files in the game data** — `output/
sprite-frame-tables.tsv` is empty — so the sprite bitmaps are decoded/loaded into
memory at scene setup from another resource (the `pe/`/`ob/` HNM banks are the
likely source; the nav orb is `pe/eye*.hnm`). Wiring the sprite layer into a
composited ship-3D frame therefore needs the scene-init trace that populates the
descriptor frame-table pointers from a resource — the background layer
(`render_ship_3d_starfield`, faithful) and every sprite primitive (parse /
dispatch / raw+rle+scaled blit / projection) are already done and tested.

PIXEL SOURCE PINNED (sess 003): each blitter (e.g. the raw-transparent one at
`0x4536`) starts with `lds si, [di+4]` — so the sprite frame pixels are reached
through a **far pointer stored in descriptor field `+4`** (the descriptor is at
`DS:0x6212 + slot*0x20`). The frame header it then reads is `[si+0]=stride`
(row bytes / mul factor), `[si+4]=x draw offset`, `[si+6]=y draw offset`, pixels
at `+8` — which matches `render.rs::RawSpriteFrame::parse` (stride@0, x@4, y@6,
pixels@8) **byte-for-byte**, confirming the frame parsing is faithful. The
scene-setup routine that writes `descriptor+4` is now found at `0x40D0`
(`ship_3d_sprite_slot_setup(slot=ax, resource_id=dx, frame=bp)`):
- `di = 0x6212 + slot*0x20`;
- `lcall 0x04B9:0x0190` with `dx` → returns `ds:si` = the sprite **frame table**
  blob (this is the resource loader, keyed by resource id);
- `slot_state = ([si] & 4) | 0x83` written to `gs:[di]` — **identical** to
  `render.rs::SpriteSlotFrameTable::slot_state_flags()`;
- `[si+2]` = frame count (bounds-checked against `bp`); the frame **offset
  table** starts at `si+4` with 4-byte packed entries (`low nibble` + `>>4`
  segment adjust), exactly matching `SpriteSlotFrameTable::parse`;
- the resolved frame pointer is stored as `gs:[di+4]=offset`, `gs:[di+6]=segment`
  and its header word `[si]` (stride) into `gs:[di+0xc]`.

So `render.rs`'s `SpriteSlotFrameTable` layout is **confirmed faithful to the
binary scene-setup**, not merely inferred from the (absent) `.spr` files.

The resource loader `0x04B9:0x0190` (file `0x5320`) is a **handle-table lookup**,
not a file read: `shl ax,3` (resource_id * 8) indexes an 8-byte-entry table based
at `FS` (startup `FS=0x0BBF`), checks load-flags at `entry+2 & 3`, and returns
`ds = fs:[id*8]` (the resource's segment), `si = 0`. Entry layout:
`{+0: segment, +2: flags(bit0/1 = loaded), +4: size dword}` — the neighbouring
stubs at `0x533C/0x5356/0x5365` read the size, free (clear bit1), and re-check the
same handle. So sprite frame-table blobs live in memory referenced by handle id;
the blobs are loaded into those segments (EMS/XMS/conventional) by the resource
manager from a bank file at startup/scene-load. IDENTIFYING THAT BANK FILE (what
populates the `FS:0x0BBF` handle table with sprite blobs) is the final,
still-open link for a fully composited ship-3D frame — it is the shared
resource-manager subsystem, so this trace also unlocks other handle-based assets.

RESOURCE-MANAGER SUBSYSTEM MAP (segment `0x04B9`, sess 003) — the shared handle
memory manager behind every handle-based asset:
- `0x04B9:0x0000` (file `0x5190`) = the core **pool allocator** (NOT a file
  reader): takes a handle id, returns its segment if already resident
  (`flags & 3`); else aligns the size (`ebp` from `entry+4`) to 16 bytes, and if
  it exceeds the free-memory counter `gs:0x0A46` runs an **LRU-style eviction**
  over handle-id lists at `0x0800`/`0x0A00` (walking with `repne scasw` /
  `std; lodsw`) to free room. It then bump-allocates from the pointer at
  `gs:0x0A6A` (`[handle]=gs:0x0A6A; flags|=3; gs:0x0A46-=size; gs:0x0A6A+=size>>4`)
  and returns the (pool) segment. Handle entries are `{+0 segment, +2 flags,
  +4 size dword}` in the `FS` table. The **resource bytes are already in the
  pool** — this routine only manages residency/eviction, so the file→pool
  population is a HIGHER-LEVEL load (startup / level-load), not here.
- `0x04B9:0x0190` (`0x5320`) = fast resident lookup; `0x533C` = get size;
  `0x5356` = free (clear in-use bit1); `0x5365` = acquire (bit0 set→mark bit1;
  else evictable→call loader `0x5190`).
- `0x53A0` = `vm_resource_profile_select(ax)`: on profile change, copies five
  resource-id/offset words from the profile table at `FS:0x11F4 + ax*0x0A` into
  `DS:0x6712` and (re)acquires them via `lcall 0x04B9:0x00F8`. This is the same
  profile system that drives the SCRIPT1→SCRIPT2 handoff.
The still-missing piece is the **file→pool load** — the higher-level routine that
reads the archive (blood.dat / a bank) into the memory pool and populates the
`FS` handle table. Reversing that unlocks sprites AND the handle-loaded intro
assets (Microfolie's, astronaut, CRYO card) that were not findable as loose
HNM/LBM files.

SPRITE DATA SOURCE FOUND (sess 003): the resource-name table at `FS:0x0C04`
(file `0x0CDF4`, 16-byte entries, already parsed as the extractor's
`RESOURCE_NAME_TABLE`) maps resource ids to names — and the ship-3D / character
sprites are **`.spr` files**: `borxx` (nav orb, 16 rotation frames), `btv`,
`bhyper`, `bpol`, `bcarte`/`carte`, `bappel`, `aphyper`, `appol`, `fupcom`,
`radio` (ship HUD/nav) plus character sprites `scruter`, `jerry`, `maxxon`,
`izwalito`, `tina`, `yoko`, `honkf`, … These are **loose files on the ISO root**
(`output/_tmp_iso/*.SPR`), NOT inside `blood.dat`, which is why
`sprite-frame-tables.tsv` was empty. The exporter now copies them into
`_tmp_dat/spr/` (`src/extract/mod.rs`), and **43/44 parse cleanly** with the
recovered `SpriteSlotFrameTable` layout (only `KLAY.SPR`, `flags=0x6`, uses a
variant) — verified on real data by `tests::real_spr_bank_parses_...` (BORXX =
16 frames, `flags 0x0004`, `slot_state 0x0087`). The sprite primitives (parse /
dispatch / blit / projection) and the frame layout are therefore all confirmed
against real assets; the remaining compositor work is mapping the 11 ship slots
to their `.spr` ids per frame + running the projection→blit→copyback pipeline
end-to-end. This ALSO means the dialogue characters exist as `.spr` sprite banks,
a second renderable representation alongside the talk-head HNMs.

VISUAL CONFIRMATION (sess 003): decoding all 16 `BORXX.SPR` frames end-to-end
(parse → RLE → index grid) and rendering them (grayscale ramp) shows the **nav
"eye" orb growing/rising** across the animation (40x33 → 52x82) — the exact
silvery sphere seen centred in the HUD of every gameplay capture (`frame_12`,
`frame_29`). So the sprite pipeline produces correct, recognisable game sprites
from real data. The frame header is `[0]=width, [2]=height, [4]=x, [6]=y`
(`RleSpriteFrame::parse` currently reads width/x/y but not the `[2]` height,
which the blitter instead takes from the descriptor extent). The only piece left
for **color-accurate, composited** sprite output is the scene palette: `.spr`
banks are palette-index only and use the current ship-view VGA palette (HNMs
embed their own `pl` chunks; there is no standalone `.pal` resource), so the last
step is identifying which resource sets the ship-view palette. The orb is
grayscale in-game, so even the ramp render already matches it closely.

Character `.spr` (SCRUTER/JERRY/IZWALITO, all 104x80, RLE) decode correctly
(right dimensions, dozens of distinct indices) but need a palette NOT yet
identified: rendering `JERRY.SPR` with the location palette (`petrol10.hnm`) OR
Jerry's idle-head palette (`pe/aajer.hnm`) both give a hollow outline with a
black interior, so neither is the character-sprite palette. PINNED (sess 003): the character `.spr` pixels use a **reserved HIGH palette
range** — `JERRY`/`SCRUTER` bodies are almost entirely indices **225-236**
(index 226 dominant) plus index 0 (transparent). The scene/location HNM
(`petrol10`) and the character idle-head HNM (`pe/aajer.hnm`) **do not define
indices 224-239 at all** (they cover the low/mid background range), so the
character sprites are drawn with a separate **character palette loaded into the
top ~32 DAC slots** when a character is shown — a classic reserved-high-slot
sprite palette. The open sub-question is therefore narrowed to where the
`224-255` character palette is loaded from. Sources ELIMINATED this session:
(a) not embedded in the `.spr` — the banks end exactly at their last frame, no
trailing palette; (b) not in any HNM header palette — scanning every
`_tmp_dat/**/*.hnm` header block, ZERO define indices 224-239; (c) not set via
an immediate VGA-DAC write — the only `mov dx,0x3c8` sites (`0x862B`/`0x8694`)
tweak a few UI indices near 0x7B (123), never 0xE0 (224). Also ELIMINATED: (d) the character
HNMs' **per-frame `pl` chunks** — parsing every frame superchunk of `aajer.hnm`
(18 frames) and `jerry_10.hnm` (31 frames), none define indices 224-239 either.
PALETTE UPLOAD MECHANISM (sess 003, CORRECTED): the full 256-colour DAC upload is
`0x0299:0x0000` (file `0x2F90`: `mov dx,0x3c8; xor al,al; out; inc dl;
mov cx,0x300; rep outsb` — 768 bytes from `ds:si`); `0x0299:0x0016` blacks the
DAC. It is called (from `0x16B0` / `0x179A`) with `ds:si = gs:0x5B58`, so the
**master palette buffer is `gs:0x5B58`** (768 bytes). CORRECTION: an earlier note
said the palette buffer was `gs:0x5221` — that is WRONG; `gs:0x5221` is the
**framebuffer** pointer (the pixel-plot primitives at `0x2FBB`/`0x3000` bounds-
check `x<0x140`, `y<0xC8`, compute planar/linear `y*80+x` offsets via `out 0x3c4`
and write one pixel). The real accessors of the palette buffer are only
`mov si,0x5B58` @ `0x16AD/0x9608/0x98A1/0xB563` and `mov di,0x5B58` @ `0x8169`.
`0x8160` restores a **base palette from `DS:0x5251` → master[0..191]**
(`rep movsd`, 0x90 dwords = 192 entries); both `0x5251` and `0x5B58` are runtime
BSS (zero in the image). So the palette chain is `base DS:0x5251` (runtime-filled)
→ `master DS:0x5B58` → DAC. The three remaining accessors are all palette
**save/restore backups**, NOT the character writer: `0x9608` and `0x98A1` copy
master `0x5B58` → base `0x5251` (full 256 entries, `cx=0xC0` dwords); `0xB563`
copies `0x5B58` → `0x5851` (192 entries) and zeroes `0x5551`. So the palette
subsystem is fully mapped (buffer `0x5B58`, base `0x5251`, backups `0x5851`/
`0x5551`, upload `0x0299:0x0000`, base-restore `0x8160`) — but the write that
puts character colours into `master[224..236]` is in NONE of them, nor in any HNM
`pl` chunk, nor static. It is therefore in the specific `.spr`-portrait display
context (a menu/roster/overlay routine), still unlocated. That display-context
routine is the exact remaining target for colour character rendering.

`.ext` FILES ARE LOCATION PALETTES (sess 003): the 50 `.ext` resources
(`KULT.EXT`, `CORPO.EXT`, `EDEN.EXT`, … — named per location/context in the
RESOURCE_NAME_TABLE) begin with an HNM-style palette block (`start,count` + 6-bit
RGB triples, `(v<<2)|(v>>4)`) defining the **low/mid background range**, followed
by location tile/sprite data. They are the authoritative per-location palettes.
But they do NOT supply the character high-slot palette: indices 224-236 are
either reserved black or fall past the palette into tile data (a naive block
parse there yields out-of-range values). So the character body colour (index 226)
is still loaded at runtime onto the `.ext`/HNM base by the character-display path
— confirming the `224-255` character palette is written into the `gs:0x5221`
master buffer per character, not shipped in `.spr`/HNM/`.ext`.

CONCLUSIVE (sess 003): a brute-force scan of EVERY game resource for a palette
that defines index 226 (the char body colour) as a valid non-black 6-bit colour
found NOTHING usable — checked all `output/_tmp_iso/*` (`.spr`/`.ext`/`.EXE`/
`BLOOD.DAT`), every HNM header + per-frame `pl` chunk, and the `.xdb` overlays
(`croolis`/`scrut`/`amer` — only zero-region false positives). So the character
portrait palette (slots 224-236) is **not statically stored in any file**: it is
constructed/remapped at runtime by the character-display path (most likely the
character's own idle-head HNM colours remapped into the high slots when the
portrait `.spr` is shown). Color character rendering therefore REQUIRES tracing
that runtime remap — static extraction is definitively ruled out. The orb
(grayscale, low indices) is unaffected and renders correctly today.

REMAP DIRECTION CONFIRMED (sess 003): remapping `JERRY.SPR`'s high indices
`224-236` down into the idle-head HNM `pe/aajer.hnm`'s defined palette range
(non-black at indices 2-126) makes a **recognisable character figure emerge**
(offset ~208-223 turns ~5600/8320 pixels non-black, vs a hollow outline with the
raw palette). So the portrait `.spr` high slots ARE the idle-head HNM colours
mapped down — the runtime display path builds the `224-255` character palette by
copying the character's own HNM palette into the high slots. The exact offset /
per-index mapping (the render is still murky at a naive linear offset, so it is
not a plain `idx-208`) is the remaining detail, resolvable only from the display
path code that performs the copy. This is the precise, well-scoped task for color
character rendering.

TWO CHARACTER REPRESENTATIONS (sess 003): the crew showcase in the attract
sequence (long-capture frames 13-21, e.g. a brown tusked crew alien over an ice
backdrop) renders the FULL-COLOUR **talk-head HNMs** (`pe/aa*.hnm`), NOT the
`.spr` portraits — so those characters are already renderable by our HNM decoder
(low-index palettes, no high-slot remap needed). Verifying one against the
showcase capture is a **compositor** task, though: a standalone talk-head is a
transparent head animation (scene-band score only ~59 vs frame_17 since it lacks
the background + scale/position), so a clean match needs the head composited over
the themed background at the right scale — i.e. the same dialogue-compositor path
(character-over-background) the pipeline already models for dialogue videos. So
character verification does NOT depend on the `.spr` high-slot palette at all;
the `.spr` portraits are a separate small-icon representation. This splits the
character work: (1) talk-head-over-background compositing (verifiable now via the
HNM path), (2) `.spr` portrait colour (overlay-remap, deferred).

So with `.spr`, all HNM header AND per-frame palettes, and immediate DAC writes
all ruled out, the `224-255` character palette lives ONLY in an **`.xdb` overlay**
(SCRUTER is the `croolis`/scrutinizer species → `croolis.xdb`) or is constructed
dynamically — squarely inside the overlay subsystem (thread #2). Color character
rendering is thus gated on the overlay decompilation. The orb uses low
indices (2-121, grayscale) so it renders without this. HNM palette-block format
is known (`render.rs::parse_palette_block`, 6-bit RGB expanded `(v<<2)|(v>>4)`).

CONNECTION TO EXISTING WORK: the profile table at `FS:0x11F4` (file `0x0D3E4`)
that `vm_resource_profile_select` (`0x53A0`) copies into `DS:0x6712` is the
**same static table already parsed by the extractor** as `ScriptResourceProfile`
(`src/bloodprg.rs` `SCRIPT_PROFILE_TABLE_*`, 5 profiles × 5 resource-id slots,
driving the SCRIPT1→SCRIPT2 handoff). So ship-3D sprites, script resources, and
the profile handoff all flow through one handle/profile/pool system — the
resource-id → archive directory (the last unknown) is shared, and recovering it
generalises across every handle-based asset, not just sprites.

The per-slot dirty geometry commit branch in `sprite_slot_commit_dirty_range`
(`0x0299:0x1467`) is now modeled as
`commit_ship_3d_sprite_slot_dirty_geometry()`. It matches the range loop's slot
body: clean slots are skipped, dirty slots without active bit `0x0001` are not
committed, and dirty active slots copy current position words `+0x08/+0x0A` and
current extent words `+0x0C/+0x0E` into previous-geometry words
`+0x10/+0x12/+0x14/+0x16`.

The global clip-snapshot branch of `0x1467` is now modeled as
`commit_ship_3d_global_clip_snapshot()`. When flag word `DS:0x5249 & 1` is set,
the binary writes clip words `DS:0x5235/0x5237/0x5239/0x523B` as the first dirty
rectangle at `DS:0x6612`, writes a `0xFFFF` sentinel immediately after it, clears
`DS:0x5249`, and exits without walking the sprite slots. The dirty-rectangle
intersection loop at `0x0299:0x14E1` is now modeled as
`collect_ship_3d_dirty_sprite_slot_render_commands()`: it exits when the
dirty-rect list starts with a negative/sentinel word, walks the requested slot
range in descending order, skips inactive slots for drawing, uses signed-word
exclusive-edge rectangle tests, selects the internal blitter dispatch as
`(slot_state >> 1) & 7`, extracts destination-remap selector `slot_state>>8 & 3`,
extracts horizontal/vertical flip from state bits 5/6, and clears dirty bit
`0x0002` after each visited slot. The dispatched call is `call cs:[0x15A2]`,
selected from the 8-entry near-pointer table at `cs:0x1592` (file `0x4522`,
segment `0x0299`) indexed by `(slot_state>>1)&0x0E`: RECOVERED as
`[0x15A6, 0x172C, 0x1C18, 0x1D46, 0x1FD2, 0x210A, 0x210B, 0x210C]` — entries 0..4
are five distinct real blitters (raw/RLE transparent+opaque, scaled) and 5..7 all
point at consecutive `ret` (`0xC3`) stubs (no-op). This matches
`ship_3d_sprite_slot_frame_for_dispatch`'s `Some(0..=4)`/`None(5..=7)` boundary
byte-exact (verified by `tests::sprite_blitter_dispatch_table_matches_binary`).
`blit_ship_3d_sprite_slot_command_indexed()`
now connects those recovered commands to the Rust ports of the dispatch table's
raw/RLE/scaled sprite blitters, and
`render_ship_3d_dirty_sprite_commands_indexed()` composes command rendering with
the recovered dirty-rectangle copyback. The next sprite-rendering target is
feeding that pipeline with real resource-frame lookup and validating against DOS
captures.

The next control-layer markers are now pinned. `0xB2BB` selects the next
ship/navigation target record from `DS:0x250B`, or from the inline fallback table
at `DS:0x2537` when the list head is `-1`; a selected `-1` entry arms the
opening transition with `DS:0x252F=1` and `DS:0x2531=6`. `0xB34E` is the broader
ship/navigation update branch gated by `DS:0x27D8`; it updates the current target
record at `DS:0x251B`, sets the sequence-active byte at `DS:0x252A`, touches the
dialogue/HUD state, and eventually drives the exit/reset branch through
`DS:0x2532`. The alternate framebuffer call at `0xB24C` temporarily swaps
`DS:0x5219` to `DS:0x521D` before invoking the recovered plane-band updater.

`src/ship3d.rs` now ports the local `0xB2BB` selector behavior. Its persistent
state is `DS:0x251B` (current target), `DS:0x252B` (phase), `DS:0x252C`
(fallback flag), `DS:0x0ADB` (animation tick reset), `DS:0x252F` (opening), and
`DS:0x2531` (depth step). If phase bit 0 is set, the routine runs a layout
prepass through `0x071E:0x0C48`, resets `DS:0x0ADB`, and increments the phase.
If phase bit 1 is still set and the `0x008B:0x0FAD` interpolation gate has not
completed, it returns `AX=0`. Once the gate completes, it clears the phase and
uses the `0x071E:0x0C48` query result as a word index into the active target
list. Query `AX=-1` returns zero. Target word `-1` returns `AX=-1`, sets
`DS:0x252F=1`, and writes step `6` to `DS:0x2531`. Otherwise, the normal list
returns `target_word - 4`; the fallback list returns the current target
`DS:0x251B`.

`src/ship3d.rs` also ports the `0x008B:0x0FAD` interpolation gate used by that
phase-2 selector path. The gate compares duration `DS:0x0ADA` with current tick
`DS:0x0ADB`: equal means complete and returns with carry set; otherwise it
increments the tick, interpolates four signed words from `SI` toward `DI` using
the binary's signed 8-bit `idiv BL` then signed 8-bit `imul DS:0x0ADB`, and
draws through `0x0299:0x040E` with carry clear. The Rust helper returns the four
draw words for the active case and `Complete` for the carry-set case.

`src/ship3d.rs` now also ports the selector-mode layout arithmetic from
`0x071E:0x0C48`. That helper first clears selection byte `DS:0x27E7`, measures
each nonzero/non-`-1` target label into the width table at `DS:0x2AB3`, and
writes the centered four-word rectangle at `DS:0x2AAB` as x/y/w/h. Width starts
at `0x64`, or `0x37` when extra-entry flag `DS:0x0ADD` is set, then grows to
the widest measured label; final width adds `0x14`. Height starts at `0`, or
`0x0A` with the extra entry, adds `0x0B` per measured target, then adds `8`.
X is `DS:0x0AC6 - width/2`; Y is `(0xC8 - height) >> 1` with the same unsigned
wrapping behavior as the binary. When query-mode byte `DS:0x27E6` is set, the
helper returns immediately after this layout step with `AX=-1`. The later
non-query branch is now split into Rust hit-test state plus draw-command
emission: mouse bounds update `DS:0x27C7/0x27E7` and presentation modes, then
the draw loop centers each target label from `DS:0x2AB3`, mutates the hover
counter as a countdown, and emits the same UI font/color choices used by the
binary.

`src/ship3d.rs` ports the recovered transition/blit primitives:

- `0xB692` updates only transition control: when `DS:0x2533` is clear and
  `DS:0x0B3B > 0x78`, it sets step `DS:0x2531=4`, opening flag
  `DS:0x252F=1`, and arms `DS:0x2533=1`. When armed and `DS:0x0B3B==0`, or
  when armed/not-opening and the random gate `0x01CE:0x0B02(AX=0x14)` returns
  zero, it sets step `DS:0x2531=8`, closing flag `DS:0x2530=1`, and clears
  `DS:0x2533`.
- `0xB75C` consumes the opening/closing flags. Opening adds `DS:0x2531` to the
  low byte of `DS:0x2527` and clamps at `0x41`, then clears the opening flag on
  the following tick. Closing subtracts from the low byte and uses the 8086
  sign flag from that byte subtraction to clamp underflow to zero, then clears
  the closing flag on the following tick.
- `0xB6DD` is a planar two-band copy, not geometry projection. If
  `DS:0x252E & 1` is clear, it returns. Otherwise it computes
  `byte_count = (low8(DS:0x2527 + 0x23) * 0x50)`. It copies the bottom
  `byte_count` bytes of the planar page at offset `0xC000` to destination
  offset `0`, and copies the top `byte_count` bytes of the planar page at
  offset `0xDF40` to destination offset `0x3E80 - byte_count`. Unless
  `DS:0x524D == 0x000A`, it also writes `DS:0x524F = 100 -
  min(DS:0x2527 * 2, 100)`. Callers at `0x5C06` and `0xB24C` temporarily swap
  `DS:0x5219` to another framebuffer pointer before invoking this routine.
- The object coordinate helpers live in the same VM/object block, before the
  target-list helpers. `0x006023` is the shared kind-specific field-offset
  lookup (`GS:0x6D60[selector * 16 + bsf(kind)]`). `0x0061A6` resolves an
  object's coordinate field by following selector-`0x11` parent/reference links
  until it reaches a direct coordinate kind (`0x0008`, `0x0010`, `0x0200`), or
  kind `0x0100`, which chooses selector `0x09`/`0x0A` from a selector-`0x0C`
  word comparison. A selector-`0x11` value of `0xFFFF` falls back to the named
  `arche` object (`DS:0x6752`). The distance caller at `0x0060DD` also treats
  kind `0x0040` as a direct selector-`0x0B` coordinate source. It reads two
  coordinate words from each resolved field, uses 16-bit wrapping signed
  subtraction and absolute value for the x/y deltas, sums the two squares, and
  calls the shared `DX:AX` integer square-root helper at `0x002E33`
  (`0x01CE:0x0B53`). The result is a binary-rounded distance, not a raw squared
  distance.

Implementation direction: keep the 320x200 indexed cutscene/dialogue path as a
software renderer until it matches oracle captures. Once the 3D/minigame
semantics above are recovered, route that subsystem through a `wgpu` frontend
that renders the original game state and then composites/presents through the
same palette/timing/oracle pipeline.

The `wgpu` boundary should be a frontend over recovered state, not the source of
truth. First decompile the target-record stream, input gates, and fixed-point
camera/projection/object math into a deterministic `ship3d` state model with a
software oracle renderer. Then add a `wgpu` presenter that consumes the same
state and outputs the same 320x200 indexed/palette-composited frame sequence for
interactive play or accelerated capture.

### Subtitle REVEAL TIMING (DECODED) — dialogue updater file 0x93F8–0x94B8

The subtitle reveals one character at a time from the buffer at `gs:0x0E18`,
tracked by reveal pointer `gs:0x5E58` (starts at the buffer start). The advance is
rate-limited by timer `gs:0xB31`: when it hits 0, `inc gs:0x5E58` (reveal one more
char) and reset `gs:0xB31 = gs:0xACA >> 2` (i.e. `gs:0xACA/4` frames per char).
The visible reveal draw call at `0x94E6..0x94EE` loads `BX=DS:0x5E5C` and
`DX=DS:0x5E5E`, then calls `0x0299:0x06A0`; those initialized words are
`0x000A` and `0x0008`, so Rust uses `(10,8)` as the subtitle origin and advances
subsequent CR-delimited lines by the glyph height (8 px), matching the wrapper at
`0x36F9..0x3701`. The same wrapper writes glyph pixels directly into the VGA
framebuffer with palette indices `0xFD` for already-revealed glyphs and `0xFE`
for the newest visible glyph, so Rust now draws subtitles into indexed HNM
frames before palette conversion and maps those indices through the scene palette
for RGB-composited dialogue videos.
After the reveal pointer reaches the terminating NUL, the dialogue state enters
a line-complete hold: `0x94BA..0x94DD` sets `gs:0xB35 = gs:0xACA*4` and
`gs:0x67BB=1`, while `0x115D..0x1188` keeps that flag alive until the timer
expires and then clears it. A second line-layout path at `0x7350..0x738C` also
sets `gs:0x67BB=1` with duration `gs:0x27CF * (gs:0xACA/2) + 6`. So the old
exporter behavior of mixing one `tb.snd` clip per visible character was wrong.
The Rust SFX track schedules one sidecar chatter event per fully revealed
subtitle line and uses `tb.snd` clip 0, matching `verified-video-scenes.tsv`.
Static RE has not found a direct `0x67BB` → `0x0B1B:0x011D` SND call; the direct
`AX=0` caller at `0x8534` is a separate presentation interaction path.
`gs:0xACA = (textspeed/2)+1` (init @0x1B3A; `textspeed` from a config getter,
special-cased so index 4 → 7). So reveal rate = `4 * frame_rate / gs:0xACA`
chars/sec; at ~15 fps and a mid text speed (`gs:0xACA≈5`) ≈ **12 chars/sec**
(the old `SUBTITLE_CHARS_PER_SEC = 36` was ~3× too fast). Rust now uses
`subtitle_reveal_chars_per_second(DEFAULT_SUBTITLE_TEXT_SPEED_STEP=5)` for
subtitle drawing, silent-line duration, and line-complete chatter placement, so
those three outputs share the same binary-derived timing source. Dialogue-run
segment lifetime now uses the decoded `reveal_complete_hold_ticks` value after
subtitle reveal completion; voiced lines last for at least that subtitle display
duration and extend only when the PCM clip is longer.

### Subtitle TEXT ASSEMBLY (DECODED) — 0xA6 handler file 0x66CD–0x6739

How the game builds the on-screen subtitle string from the 0xA6 word list (DIC
word offsets), into a buffer at `gs:0x0E18` (DIC segment = `gs:0x672A`):
- For each word: copy its DIC bytes into the buffer (count chars in `dl`).
- Peek the FIRST char of the NEXT word; if it is `,` `.` `?` `!` `:`
  (`0x2C 0x2E 0x3F 0x21 0x3A`) → insert **no space**; otherwise insert `0x20`.
- After inserting a space, if the current line length `dl >= 0x23` (**35**),
  insert a line break `0x0D` and reset `dl=0`. (No wrap check on the no-space
  path.) Long single words are not split.
- At end of list (word offset 0 or 0xFFFF): append `0x0D` then `0x00`.
So subtitles are **multi-line, wrapped at 35 chars**, `0x0D`-separated, with
punctuation-aware spacing. (The current Rust `words.join(" ")` is wrong on both
counts.) Implemented as `assemble_dialogue` in `src/extract/script.rs`.

### Subtitle glyph blitter (DECODED) — file ~0x31C8 `render_string`

Renders a NUL-terminated ASCII string with the dialogue bitmap font. Inputs:
`SI`=string, `DI`=screen offset, `DL`=pixel color, `ES`=screen/back buffer,
`GS`=data seg (=DS). Algorithm per char:
- `0x00` ⇒ end; `0x20` (space) ⇒ `di += 6`.
- `al = gs:[0x7802 + al]` (`xlatb`): ASCII→glyph index via font map; if result has
  bit7 set, char is skipped (not in font).
- `dh = gs:[0x78B2 + glyph]`: per-glyph advance width.
- glyph bitmap = `gs:[0x7908 + glyph*8]` (8 rows × 8 bits).
- plot: for each of 8 rows, shift the row byte; set `es:[di]=dl` per 1-bit;
  `di += 0x140` (**screen stride 320** ⇒ 320×200 mode) between rows.
- after the glyph, `di += dh` (advance).
Confirms README font offsets and gives the exact layout + 320-wide framebuffer.
Font-table references also at file 0x30E6/0x3253/0x35C2 (width-measure / variants
— other text routines in seg 0x0299, not yet mapped; one is likely the animated
cutscene-subtitle reveal).
The string SI is *already resolved ASCII* — so 0xA6 dict-word lookup happens
upstream (find that to reach the 0xA6 handler).

**render_string ABI** (far `0x0299:0x0202`): `BX`=x, `DX`=y, `SI`=string offset,
`AL`=color, `DS`(or ES via ds=es)=string segment, font in GS=DS. Returns width in
`gs:[0x27cd]`-related. Its 5 far callers (file 0x8FEB, 0x9183/99/A8/E4 in seg
0x071e) are all **UI/HUD** text (object-name tooltip at cursor `gs:[0xA2A/0xA2C]`;
status panel; roster loop over object list at `[0x6886]`). Not the cutscene path.

**Object/actor struct fields** (seen in the roster loop, es:di base):
`+0x00` u16 flags (bit1 tested), `+0x02` u16 flags (bit0 tested), `+0x04` name
string (ASCIIZ, passed to render_string), `+0x36` u16 (nonzero gate). This is not
the same table as the six 0x18-byte navigation slots iterated at file `0x7E09`.

### Script VM — execution dispatch (FOUND)

**Main interpreter loop** at file `0x55F5`–`0x569E` (segment 0x04DA):
- `di = 0x6EB0` (handler table base, DS-relative); `lds si, gs:[0x671C]` (COD
  far-ptr). Loop head `0x5613`: `lodsb` opcode; `0xFF` ⇒ end (→0x568A).
- **Dispatch** `0x5627`: `bl=op; sub bl,0xA0; bx=(op-0xA0)*2;
  call word ptr gs:[bx + 0x6EB0]` — a **52-entry near-handler jump table at
  `DS:0x6EB0` (file `0x142D0`)** for opcodes 0xA0..0xD3, offsets into seg 0x04DA
  (file base 0x53A0). Table is immediately followed by the length table at
  0x6F18 (0x68 bytes = 52×2 apart). Dump: `re/tools/dump_handler_table.py`.
- Post-handler: handler may set `gs:0x67B4` (control signal); `gs:0x67AB` =
  skip-N-tokens counter (calls `token_advance` N times → IF/branch skip);
  `gs:0x67B1` bit0=loop active, bit1=loop range; `gs:0x6778/0x677A` loop
  start/resume addrs; `gs:0x6772` = PC.

**Opcode → handler file offsets** (seg 0x04DA). Families share handlers:

| op | handler | | op | handler | | op | handler |
|----|---------|-|----|---------|-|----|---------|
| A0 | 0x6559 | | AC | 0x685C | | C2 | 0x6E34 |
| A1 | 0x6572 | | AD/AF/B2/B3/BA-BC | 0x6946 | | **C3** | **0x6EEE** (record link) |
| A2 | 0x6588 | | AE/B0 | 0x6902 | | **C4** | **0x6C7E** (actor/record op) |
| A3 | 0x6596 (collect words) | | B1/B4-B6/BE-C0 | 0x6863 | | **C5** | **0x6D18** (record entry) |
| A4 | 0x65DB | | B7 | 0x6AA7 | | **C6** | **0x6D80** (record entry) |
| A5 | 0x65EB | | B8/B9/BD | 0x6B06 | | **C7** | **0x6DCF** (record entry) |
| **A6** | **0x660C** (TEXT) | | C1 | 0x6B4C | | **C8** | **0x6F62** (record entry) |
| A7 | 0x67BA | | CA | 0x64E5 | | **C9** | **0x6FB9** (record clear) |
| A8 | 0x67C8 | | CB | 0x6510 | | CD | 0x69C7 |
| A9 | 0x6830 | | CC | 0x64CE | | CE–D2 | 0x6494–0x64B8 (1–2 byte ops) |
| AA | 0x6855 | | | | | D3 | 0x53A0 (seg base = no-op/default) |
| AB | 0x684C | | | | | | |

Secondary jump tables (sub-dispatch within handlers):

- `cs:0x06D4` table at file `0x7EB4`, called indirectly at file `0x7E09`
  from loop routine `0x7D7B`. The loop walks six 24-byte records at
  `DS:0x2A1B`, `0x2A33`, `0x2A4B`, `0x2A63`, `0x2A7B`, `0x2A93`; because `CX`
  counts down while `BP` increments, slot 0 uses table index 5 and slot 5 uses
  table index 0.

      idx  cs:off  file
      0    0x07BC  0x7F9C
      1    0x06E0  0x7EC0
      2    0x095A  0x813A
      3    0x099E  0x817E
      4    0x0A1B  0x81FB
      5    0x08A2  0x8082

- `cs:0x0F29` table at file `0x8709`, called indirectly at file `0x8700` from
  routine `0x85E2`. The caller rejects `AL >= 5`; the preceding
  `test [0x2565],1` does not branch at the call site and is state consumed by
  the handlers, not a dispatch gate.

      idx  cs:off  file
      0    0x0F33  0x8713
      1    0x0F4C  0x872C
      2    0x0FDD  0x87BD
      3    0x1068  0x8848
      4    0x108C  0x886C

Recovered candidate layout for the 24-byte records iterated by `0x7D7B`:

    +00 flags/state byte/word; bit0 active, bit2 initialized/rendered, bit3 hit
    +02 action/object id used by helper 0x7E1C; handlers write 0x12..0x15
    +06 max frame/count loaded by helper
    +08 current frame/counter
    +0A selection/angle compare against 2*gs:0x2795
    +0C hit rect x
    +0E hit rect y
    +10 hit rect w
    +12 hit rect h
    +14 render/blit x passed as BX
    +16 render/blit y passed as CX

### 0xA6 TEXT handler @ file 0x660C — field semantics (DECODED)

On entry `si` points at the token's `b1`. The handler:
- `les di, gs:[0x6724]`; `ax = [b1,b2] (u16)`; `di += ax` ⇒ **`b1:b2` is a u16
  index into the runtime object/state table** (`gs:0x6724`). `es:[di]` is that
  object's/state record kind (`2` in observed VAR-backed A6 rows); `es:[di+2]`
  holds a flag word (bit15 = already shown/skip).
  The handler sets this `0x8000` bit after accepting a line. Rust exposes this
  as `TEXT_LINE_ALREADY_SHOWN_FLAG` and an opt-in
  `ExecutionContext::with_text_line_display_gating()`. It is intentionally not
  enabled by default yet: raw `SCRIPT*.VAR` line flag words are not the same as
  the initialized runtime line-record table, and applying them directly drops
  valid text from real-script traces.
- saves `si@b3` to `gs:0x677C`; reads **`cx = [b4,b5] (u16)` = the control word**:
  - `b4 & 0x08` ⇒ set skip-count `gs:0x67AB = ((b5>>4)&7)+1` (conditional IF skip).
  - `b4 & 0x10` ⇒ loop: `gs:0x67B1|=1`, next word → `gs:0x6778` (loop target).
  - `b4 & 0x01` ⇒ preserve bit7 of `b5` after accepting the line. If this bit
    is clear, the handler clears bit7 of `b5` in the COD stream (`and [si+1],0x7f`).
  - `b4 & 0x04` ⇒ skip one extra u16 control word before the dictionary-word loop.
  - **`b5 & 0x80` (bit7) = ACTIVE/DISPLAY flag**: `or cx,cx; jns →skip` — if bit7
    clear the line is not shown (explains why real data always has 0x80).
  - global mutes `gs:0x5E64`, `gs:0x67B0` also gate display.
- display then requires the selector-`0x13` table entry at index `+1`
  (`gs:0x6D60 + 0x131` = `0x3A`) to contain `0x00C4`: in Rust this is the
  opt-in `ExecutionContext::with_text_presentation_record_gating()` check
  `state[b1b2 + 0x3A] == 0x00C4`. Enabling it globally currently drops
  real-script rows because the upstream C4 presentation setup path is not fully
  reconstructed yet.
- later: `si=gs:0x677C; al=[b3]; gs:0x1FAB = (s8)b3` ⇒ **`b3` is the per-line
  selector stored to global `gs:0x1FAB`**. `0xFF` and `0x00` are no-voice
  channels; `1..=N` selects the actor's one-based `son.snd` talk clip.
- dict-word resolution + on-screen display continue past 0x675E (uses `render_*`
  text routines in seg 0x0299).

**b3 selector flow (traced):** `b3` → signed word `gs:0x1FAB` → (reader @0x11F2)
`gs:0x6788 = sign_extend(b3) + 9`, tracked as the **active dialogue-line id**
(compared vs `bx` at 0x120F; reset to `0xFFFF` on clear). Voice clip selection is
resolved in Rust as `b3 == 0xFF || b3 == 0x00` → no voice, `b3 in 1..=N` → actor
`son.snd` clip `b3 - 1`. `src/vm.rs` now owns this as
`text_selector_active_line_id` and `text_selector_voice_clip_index`.

**Clear / scene-reset routines** (the renderer's *clear* event): file `0x1A64`
and `0xB529` both reset `gs:0x1FAB`,`gs:0x6788` (→0xFFFF) plus the display gates
`gs:0x5E64`,`gs:0x67B0`,`gs:0x67BC`,`gs:0x67BA` and call the common stop routine
`0x071E:0x14B6`. Useful as the authoritative subtitle/scene-clear semantics.

**Remaining for full accuracy:** (1) verify whether audible `tb.snd` chatter is
triggered by a dynamic callback rather than the now-decoded `gs:0x67BB` hold
flag; (2) decode any remaining line-record display flags that affect
subtitle/talk-HNM routing; (3) map the remaining C1/C2/CA/CB/CD line-state and
global-condition handlers; (4) `gs:0x6724` line-record layout.

### 0xB7 bit-flag handler @ file 0x6AA7 — state flag set/test (DECODED)

`0xB7` is a 4-byte state/line-record bit flag operation, with an optional `0xA1`
prefix after the opcode. Shape:

    B7 [A1?] <base:u16> <bit:u8>

The handler computes `byte = base + (bit >> 3)` and uses **high-bit-first**
numbering inside each byte: bit 0 = mask `0x80`, bit 1 = `0x40`, ..., bit 7 =
`0x01`.

- Mode 0 (`gs:0x67AD == 0`): no prefix sets the bit (`or es:[byte],mask`); `A1`
  clears it (`and es:[byte],~mask`).
- Mode 1: no prefix tests that the bit is set; `A1` tests that it is clear. A
  failed test calls branch helper `0x6462`.

Shipped scripts use true `B7` tokens in SCRIPT2 and SCRIPT3. Rust now exposes
them as `VmToken::BitFlag` and `execute_trace` applies/evaluates the same
high-bit-first bit semantics.

### 0xB8/0xB9/0xBD pair-record handler @ file 0x6B06 — pair state (DECODED)

`0xB8`, `0xB9`, and `0xBD` share a 7-byte pair-record handler:

    <B8|B9|BD> <record:u16> <first:u16> <second:u16>

The handler loads `les di, gs:[0x6724]`, adds the record offset to `di`, then:

- Mode 0: writes `es:[record]=first` and `es:[record+2]=second`.
- Mode 1: compares both words and calls branch helper `0x6462` if either word
  differs.

After a mode-0 write it also calls helper `0x6034` and, if the result matches
`es:[gs:0x6752 + 0x16]`, clears that `+0x16` field. Rust does not model that
secondary bookkeeping field yet, but it now ports the direct pair write and
branch comparison in `interpret_line_states` / `execute_trace` and exposes the
raw token as `VmToken::PairRecord`.

### 0xC1/0xC2 line-record state handlers — token shape (PARTIALLY DECODED)

`0xC1` and `0xC2` are both fixed 5-byte line-record state operations with the
same raw token shape and an optional mode-1 `A1` inverted-compare prefix:

    <C1|C2> [A1?] <record:u16> <operand:u16>

They load the line-record/state table through `les di, gs:[0x6724]`, then use the
raw record and operand words to resolve additional table slots before either
mutating state (mode 0) or calling branch helper `0x6462` on a failed test (mode
1). `0xC1` has a confirmed success write of `{0x00C1, operand, 0x0002}` after it
finds an empty resolved destination slot. `0xC2` has presentation side effects in
mode 0 for special record kinds: it can clear `gs:0x1FB2` and set active dialogue
line ids `gs:0x6788 = 0x27` or `0x2B`.

Current shipped-script VM walks contain repeated true `C1` tokens and no true
`C2` tokens. Rust now exposes both as `VmToken::RecordState { ..., inverted }`
and the script disassembly emits `record_state` rows. `execute_trace` evaluates
direct mode-1 compares when host state already contains a concrete
`{opcode, operand, ...}` record entry, and Rust now applies the direct `C1`
mode-0 success write when `ExecutionContext` proves the owner object is active
and the destination record is empty. When `ExecutionContext` supplies explicit
ship-3D C1 runtime tables and the live `DS:0x6886` scratch bytes, `execute_trace`
also follows the kind-`0x10` source-list gate and writes the selector-`0x13`
destination instead of falling back to the raw token record. `C2` compare
evaluation also requires the DEB-derived `ExecutionContext` because the binary
checks the owner object active via helper `0x6034`. Rust also ports the direct
C2 mode-0 operand-record write:
if the owner is active, `operand+2` has bit `0x20`, and the runtime sentinel
list accepts the operand, helper table `gs:0x6D60` selects a kind-specific field
and Rust writes `0xFFFF` there. Kind `2` records also clear `gs:0x1FB2` and set
active dialogue line `gs:0x6788 = 0x27`. Kind `0x0400` records call helper
`0x7409` with `DI = operand + 4`; that helper opens `descript.des`, scans its
18-byte directory entries for a NUL-terminated name matching `es:[DI]`, and
returns nonzero after dispatching the matched descriptor script. Rust models that
nonzero helper result through `ExecutionContext::with_descript_entry_name`, then
clears `gs:0x1FB2`, ORs bit `0x02` into `gs:0x67AA`, and sets
`gs:0x6788 = 0x2B`. Extractor VM traces now seed that context from parsed
`DESCRIPT.DES` record names. For the C1 resolved-table path, dependency helper
`0x6210` is now decoded: it maps an object record to its
index in the 20-byte `GS:[0x672C]` object table, adds the selector-`0x05` /
kind-`0x0002` field offset (`0x1E`) to the caller's bitset base, and tests the
object's bit high-bit-first. In the C1 mode-0 branch this is the kind-2 source
filter before selecting a resolved destination slot. Rust also ports the
distance/selector-`0x11` redirect at `0x006BEA..0x006C04`: when the raw C1
operand word is exactly `1` or `2`, the binary calls
`ship_3d_position_distance(operand, current_target)`. A zero distance leaves
`DI` unchanged. A nonzero distance loads the selector-`0x11` word from the
current target, makes that the new `DI`, and requires the new record kind to be
`0x0010`; failure rejects the C1 write without falling back to the direct token
record. After that, Rust ports the `0x006C1C` source-list scan: it walks the
`0x00624B`-built `DS:0x6886` list to the `0xFFFF` sentinel, accepts kind `2`
records when helper `0x6210` reports the operand object's bit set from the
current post-`lodsw` `SI` cursor into that scratch list, accepts kind `1`
records when the operand state byte has bit `0x02`, and ignores other kinds.
Rust also ports the `0x006C48..0x006C6B` destination-slot write: for the
resolved kind-`0x10` `DI` record it resolves selector `0x13` using hardcoded
kind `0x0010`, checks only the destination's first word for emptiness, and
writes `{0x00C1, operand, 0x0002}`. `src/vm.rs` now wires this C1 path through
`ExecutionContext::with_ship_3d_c1_runtime(...)`, with optional
`with_ship_3d_c1_positions(...)` data for the distance redirect; tests prove a
selected source writes the resolved target's `+0x1C` slot, distance zero keeps
the original kind-`0x10` owner, and a non-kind-`0x10` redirect target does not
fall back to the direct token record.

The C1 mode-1 resolved comparison path at `0x006B85..0x006BCB` is also now
ported. If the direct record slot is not already `0x00C1` and the raw C1 operand
is exactly `1` or `2`, the binary resolves a target from the current owner using
selector `0x11` keyed by the raw operand kind, then resolves that target's
selector-`0x13` record slot using the target kind. The branch condition passes
only when that resolved slot contains `{0x00C1, operand, ...}`; `A1` inversion
flips the result. Rust executes that comparison in `execute_trace` when
`ExecutionContext` can identify the owner object. Known C1 mode-0 failed writes
now also call the recovered branch-fail path in `execute_trace`: when owner
context is available and the active-owner, source-list, kind, or destination
empty-slot checks fail, Rust pops the current A0/A1 branch target like helper
`0x6462` instead of continuing as a no-op. Missing owner context remains
unresolved rather than guessed.

### 0xCA/0xCB global condition handlers — token shape (DECODED; runtime source pending)

`0xCA` and `0xCB` are condition handlers, not media commands. They call branch
helper `0x6462` when their comparison fails.

`0xCA` shape:

    CA <op:u8> <tag:u8> <value:u16>

The handler stores the first byte in `dl`, ignores the high byte of the first
`lodsw` (kept by Rust as `tag`), then compares `value` against `gs:0x0AA6`.
`op=0xF1` passes when signed `value > gs:0x0AA6`; `op=0xF2` passes when
signed `value < gs:0x0AA6`; other operators use equality.

`0xCB` shape:

    CB <op:u8> <packed:u16> <reserved:u16>

The handler compares `packed` high/low bytes against `gs:0x0AAA` and
`gs:0x0AA8` as a signed two-byte lexicographic pair. `op=0xF1` is greater-than,
`op=0xF2` is less-than, otherwise equality. The final word is consumed but not
used by the observed compare path, so Rust keeps it as `reserved`.

Rust now exposes these as `VmToken::GlobalWordCompare` and
`VmToken::GlobalPairCompare`, and `script-disassembly.tsv` emits
`global_word_compare` / `global_pair_compare` rows through the same mode-aware
VM walker, so mode-1 `CA`/`CB` tokens are no longer buried inside raw spans.
`execute_trace` branches on them when `ExecutionContext` supplies
`gs:0x0AA6/0x0AA8/0x0AAA`; host-side replay must choose the BIOS RTC
hour/month/day values for deterministic output.

**Runtime source recovered:** the VM wrapper at file `0x55B6..0x55BB` calls two
far routines immediately before `vm_exec_loop`: file `0x093B` reads BIOS RTC time
(`int 1Ah AH=02h`), BCD-decodes `CH`, and stores the current hour in
`gs:0x0AA6`; file `0x0950` reads BIOS RTC date (`int 1Ah AH=04h`),
BCD-decodes `DL`/`DH`, and stores day/month in `gs:0x0AA8`/`gs:0x0AAA`.
`CL` is also converted into a year at `gs:0x0AAC`, adjusted by `CH` century.
Current true script tokens use these as:

- `SCRIPT2`/`SCRIPT3` `CB == 12/25` and `CB == 1/1` date checks.
- Repeated `CA` hour-window checks (`>8`, `<2`, `>12`, `<8`, etc.) for time-of-day
  branch variants.

Rust now exposes `ExecutionContext::with_bios_rtc(hour_24, month, day)` for
deterministic host-side replay of those branches; default traces still leave the
globals absent rather than silently using the developer machine's current date.
The extractor's branch-scenario pass now derives representative RTC replay
contexts from true `CA`/`CB` tokens, including ordinary Jan 2 baselines plus
observed seasonal dates (Christmas/New Year) and hour-boundary candidates.

### 0xCD record-triple handler @ file 0x69C7 — token shape (PARTIALLY DECODED)

`0xCD` is a 7-byte line-record operation with an optional `0xA1` prefix in
mode 1:

    CD [A1?] <record:u16> <first:u16> <second:u16>

Mode 1 compares the direct record entry against `{0x00CD, first, second}` and
calls branch helper `0x6462` when the comparison fails; `A1` inverts the test.
Mode 0 resolves additional table state through helpers `0x6034`, `0x5FD8`, and
`0x5FF6`, writes a word into a computed destination, and can trigger the same
special active-line side effect as `0xC2` (`gs:0x6788 = 0x2B`).

Rust exposes the consumed token as `VmToken::RecordTriple`, emits `record_triple`
disassembly rows, and `execute_trace` now evaluates the direct mode-1 compare
including `A1` inversion. Mode-0 side effects still depend on the resolved
line-record table model and remain unexecuted.

### 0xC3 record-link handler @ file 0x6EEE — relation state (DECODED)

`0xC3` consumes two u16 operands with an optional mode-1 `A1` inverted-compare
prefix: `C3 [A1?] <record:u16> <related:u16>` (5/6 bytes). In mode 0 the handler
checks that both involved records are active and that the destination record is
not already a `0xC4` actor entry. On success it writes:

    es:[record + 0] = 0x00C3
    es:[record + 2] = related
    es:[record + 4] = 0x0001

This is relation/presentation line-record state, not a speaker marker. Several
real scripts emit narrator/system text after `C9` then `C3`; treating `C3` as a
speaker would reintroduce wrong actor/background attribution. Rust exposes it as
`VmToken::RecordLink { ..., inverted }`, applies the guarded mode-0 write when a
DEB-derived `ExecutionContext` can resolve the owner object, and evaluates direct
mode-1 compares with the same context. Known mode-0 failures now branch through
the recovered A0/A1 stack; missing owner context remains unresolved rather than
guessed. The parsers deliberately do not update current speaker state from it.
`script-disassembly.tsv` now emits it as `record_link` instead of leaving those
bytes in raw rows.

### 0xC5..0xC8 record-entry handlers — relation state (DECODED)

`0xC5`, `0xC6`, `0xC7`, and `0xC8` are 5-byte line-record operations with the
same token shape: `<opcode> <record:u16> <operand:u16>`. Their mode-0 success
paths write a 6-byte record entry:

| op | handler | stored entry | guard summary |
|----|---------|--------------|---------------|
| C5 | 0x6D18 | `{0x00C5, operand, 0}` | operand record active and type `0x0200`; destination empty |
| C6 | 0x6D80 | `{0x00C6, operand, 0}` | unconditional destination write in mode 0 |
| C7 | 0x6DCF | `{0x00C7, operand, 0}` | operand record active; destination empty or currently `0xC4` |
| C8 | 0x6F62 | `{0x00C8, 0, 0}` | destination empty; second token word is consumed but not stored |

Current shipped-script VM walks find two real `C6` tokens (SCRIPT3/SCRIPT4) and
no true `C5`/`C7`/`C8` opcode positions; raw byte scans see many false positives
inside operands and text data. Rust exposes this family as
`VmToken::RecordEntry { entry_opcode, record_offset, operand,
stored_related_offset, aux_word, inverted }`, and `script-disassembly.tsv` emits
`record_entry` rows for future line-record modeling. Rust now executes the
successful mode-0 writes for the whole family, including C5's active/type-0x0200
operand guard, C7's active-operand plus empty-or-C4 destination guard, and C8's
empty-destination write of `{0x00C8, 0, 0}` despite consuming a second token
word. Direct mode-1 record-entry compares are evaluated when host state has a
concrete record entry. Known guarded mode-0 failures for C5, C7, and C8 now
branch through the recovered A0/A1 stack; C6 remains an unconditional mode-0
write.

### 0xC4 actor/record handler @ file 0x6C7E — operands (DECODED)

`0xC4` is not a 3-byte actor marker. The binary consumes two u16 operands:
`C4 <record:u16> <related:u16>` (5 bytes total, matching the opcode length
table). The handler loads the per-line/record table through `les di, gs:[0x6724]`,
reads the first word into `bp`, reads the second word into `ax`, and on the
mode-0 success path writes:

    es:[bp + 0] = 0x00C4
    es:[bp + 2] = related
    es:[bp + 4] = 0x0000

Mode 1 first accepts an optional `A1` inverted-compare prefix, then tests that the
owning object is active, `es:[record] == 0x00C4`, and `es:[record+2] == related`;
the failed test calls branch helper `0x6462` unless inverted. The owning object
comes from helper `0x6034`, which maps a record offset back to the previous DEB
object offset (`record == object + 0x3A` for talk records).

The Rust VM now exposes this as `VmToken::Actor { record_offset,
related_record_offset, inverted, len }`. Mode 0 writes the direct
`{0x00C4, related, 0}` record entry and updates speaker context. Mode 1 does not
write that state. `ExecutionContext::with_strict_actor_record_branching()`
models the binary compare exactly: an empty `{0,0}` record fails the mode-1 C4
test and calls branch helper `0x6462` unless the token has the `A1` inversion
prefix. In current extracted `SCRIPT2`, strict C4 branching reaches `EndMarker`
with zero text lines because the top-level blocks at offsets 5, 727, 902, ...
all test empty presentation records. That confirms the next missing runtime
piece is the mode-0 setup path that populates these records before the A6
handler's presentation gate is enabled globally.

The default `execute_trace` path still preserves empty mode-1 C4 records as
unresolved compatibility state so existing exports do not collapse to zero
dialogue while that setup path is incomplete.

### VM post-update C4 pair path @ file 0x5816 / 0x5D8F — operands (PARTIAL)

After selected VM passes, routine `0x5816` scans the DEB object table
(`DS:0x672C`) and runtime state (`DS:0x6724`). For each active object it computes
the selector-`0x13` record field (`helper 0x6023`; kind `2` -> `+0x3A`). When
that record is a C4 entry with aux word `0`, the `0x5D8F..0x5E1F` path:

    ds:[record + 4] = 0xFFFF
    related = ds:[record + 2]
    related_record = related + field_offset(selector=0x13, kind=ds:[related])
    ds:[related_record + 0] = 0x00C4
    ds:[related_record + 2] = owner
    ds:[related_record + 4] = 0xFFFF

This is the reciprocal presentation-pair write that bridges a direct mode-0 C4
record into the related object's A6/C4 presentation gate. Rust now ports this as
`post_update_actor_record_pair()` and ports the active-object subset of the
`0x5816` scan as `post_update_actor_records_for_active_objects()`: it walks the
DEB object offsets in `ExecutionContext`, skips inactive objects (`state[+2]&1 ==
0`), computes selector-`0x13`, and applies the reciprocal C4 write. It is not
wired into `execute_trace` yet because the rest of `0x5816` also depends on
presentation globals (`0x675E`, `0x674E`, `0x6752`, `0x67AC`, `0x67B6`, etc.)
and UI/event handlers that choose which object pair is active.

### VM named-object startup scan @ file 0x5486 — globals (PARTIAL)

The startup scan walks 20-byte DEB records and compares kind-1 object names
against built-in strings at `DS:0x67BE`:

| Name | Stored global |
| --- | --- |
| `blood` | `DS:0x674E` |
| `orxx` | `DS:0x6750` |
| `arche` | `DS:0x6752` |
| `Honk` | `DS:0x6754` |
| `menu` | `DS:0x6756` |
| `Ark` | `DS:0x6758` |
| `Scruter_Jo` | `DS:0x6760` |

The nearby `cryobox` bytes are present in the string block but are not referenced
by this scan. A second kind-5 scan at `0x552A` stores `vbio` in `DS:0x679C`.
Rust now mirrors the named offsets in `ExecutionContext::vm_named_object_offsets`
while keeping the existing `blood` special-object remap behavior.

### VM presentation-active globals @ file 0x5816 / 0x108E — operands (PARTIAL)

The `0x5816` post-exec scan begins by clearing `DS:0x67B6`, the guard tested by
the lower `0x5D8F` reciprocal C4 pair writer. It also owns the
presentation-active flag state that gates script-profile handoff. At `0x58D4`,
an active kind-1 object whose selector-`0x13` record is `0x00C4` starts
presentation state when `DS:0x67AC` is clear:

    DS:0x5B55 = 1
    DS:0x0A32 = 1
    DS:0x67AC = 1
    DS:0x6782/0x6784/0x6776/0x67F8/0x2A19 = 0
    DS:0x67BA/0x27D7/0x67BC/0x67BB = 0
    DS:0x679A = 0
    DS:0x67B7 = 1
    DS:0x2793 |= 0x04
    related[+3] |= 0x80
    DS:0x2751 &= 0x7F

The same path mirrors `related[+2] & 0x20` into `DS:0x67AF`. If the kind-1
selector-`0x13` record is no longer C4 while `DS:0x67AC` is set, `0x599A`
tears the state down by clearing `0x67B1`, `0x67AC`, `0x6762`, bit `0x04` in
`0x2793`, low two bits in `0x67AA`, `0x67F8`, `0x67B7`, and `0x27E8`.

Rust now ports those start/stop memory effects in
`post_update_kind1_presentation_state()` and calls it from the active-object
subset of the `0x5816` scan. The external render/audio calls and the far-pointer
clear through `DS:0x6746` remain pending. `execute_trace` now runs the recovered
post-update scan at the same end-of-pass boundary as the binary call at
`0x568D` and exposes the result as structured `ExecutionTrace::post_update`
data.

After kind-1 presentation handling, `0x59F9` drains a deferred record write if
both `DS:0x6768` (record type) and `DS:0x676A` (related pointer) are nonzero.
For most record types it writes the current selector-`0x13` record:

    record[+0] = DS:0x6768
    record[+2] = DS:0x676A
    record[+4] = DS:0x676C

For deferred `0x00C1` or `0x00C6`, it instead computes the selector-`0x13`
field for kind `0x10`, adds that to named object `arche` (`DS:0x6752`), and
writes `{type, related, 0}` there. It then clears `DS:0x6768`, `0x676A`, and
`0x676C`. Rust ports this as `post_update_deferred_record_write()`.

The kind-2 branch at `0x584C` is a presentation control-flow handoff. It only
calls `vm_control_flow` when presentation is active, `DS:0x1FB2`, `0x27D7`, and
`0x67B7` are clear, `DS:0x675E` points at a C4 record, the current
selector-`0x13` record is C4 and points at `blood` (`DS:0x674E`), the owner
flags word does not have bit `0x8000`, and selector `0x02` on the owner yields a
nonzero target. Rust now captures that as
`post_update_kind2_presentation_handoff_target()` and applies the recovered target
as a COD PC handoff after the post-update pass. The `0x27D7` gate is distinct
from the main-loop idle gate at `0x27DA`; Rust tests cover that split so the
adjacent addresses do not get collapsed.

MAIN GAME LOOP HEAD (sess 003, `0x0FFB`): the engine's top-level dispatch loop
begins at `0x0FFB` (reached via `jmp 0x0FFB` at `0x1077` and the setup at
`0x0FF0`). Each iteration: (1) polls/sets the **mouse** (`int 33h AX=4` with the
cursor position/limits at `0x0A2A`/`0x0A2C`, `0x0A38`/`0x0A3A`); (2) resets the
sprite dirty-rect list (`[0x6612]=0xFFFF`) and clip flag (`[0x5249]=1`); (3)
calls render/present subsystems (`0x210E`, then `0x1A93`, `0x1FBC`); (4) gates on
the **on-ship flag** `[0x2793] & 8` (the ship-nav HUD state — bit 3, the same
flag that selects letterbox-planet vs on-ship rendering) to choose the mouse/
cursor path; (5) advances a countdown at `[0x0A40]`; (6) far-calls the shared
dispatcher `lcall 0:0x70E`; and (7) falls through to the pending-profile check
`main_pending_profile_check` (`0x108E`, below) for `D2` script/scene handoff. So
**interactive navigation and input dispatch live in this loop** — the mouse poll
here plus the subsystems `0x1A93`/`0x1FBC` are where the opening's interactive
narration and the pyramid-nav UI are driven. That is the concrete entry point for
the next-session navigation trace (drive the mouse to the right UI targets, or
find the handler that advances the opening).

The main loop at `0x108E` does not consume a pending `D2` profile request until
the presentation state is idle. The exact gate is:

    DS:0x6780 != 0xFFFF
    (DS:0x2793 & 0x0E) == 0
    OR(DS:0x67AC, 0x24F3, 0x2751, 0x67B0, 0x5E64,
       0x2565, 0x2736, 0x2737, 0x27DA, 0x2792) == 0

Rust captures this as `pending_script_profile_dispatch_ready()`. `execute_trace`
now writes the D2 operand-derived profile index into `DS:0x6780` before running
the post-update scan, and `execute_script_profile_sequence` only follows the
request when `ExecutionTrace::post_update` says the binary's idle gate would
allow the main loop to dispatch it.

### 0xC9 record-clear handler @ file 0x6FB9 — speaker lifetime (DECODED)

`0xC9` consumes one u16 record operand (`C9 <record:u16>`, 3 bytes total). The
handler loads the line/record table via `les di, gs:[0x6724]`, reads the existing
record type at `es:[record]`, then zeros the three-word record:

    es:[record + 0] = 0
    es:[record + 2] = 0
    es:[record + 4] = 0

If the previous record type was `0xC4`, it treats the old `es:[record+2]` value
as a related actor subrecord, computes the type-dependent stride through helper
`0x6023` with selector `0x13`, zeros that related 6-byte subrecord too, and
resets presentation gate bytes `gs:[0x252A]=0`, `gs:[0x2531]=6`.

This matters for video accuracy: real scripts frequently emit text after a
matching `C9` and before the next `C4`, so carrying the previous actor through
that clear bleeds the wrong speaker/background into narrator or system lines.
Rust now exposes `VmToken::RecordClear`, clears the current speaker context when
`record_offset == actor_offset + 0x3A`, clears the recovered related C4 subrecord,
and resets the two presentation gate bytes. Unlike the comparison handlers, this
handler has no `gs:0x67AD` mode check: Rust now applies the direct clear in mode
0 and mode 1.

### Dialogue display state machine (seg 0x0971, file ~0x9E81)

Per-frame dialogue updater. `gs:0x6788` = active line id (set by signed 0xA6 b3+9);
`gs:0x678A` = currently-displayed line id. On `0x6788 != 0x678A` it latches the
new id and redraws (lcall render seg 0x299). Special line ids switch the
**viewport clip region**: id `5` and `0x27` set `gs:0x5239=0x23,gs:0x523B=0xA5`
(letterbox window ~rows 0x23..0xC8) then restore — i.e. cutscene vs normal-screen
subtitle framing. `gs:0x5239/0x523B` are the render_string y-clip bounds.
The son.snd/talk-animation trigger is one more hop, via `0xA1B4`/`0xA40B` called
here (seg 0x0971) — these reach the audio playback subsystem. 41 sites touch
`gs:0x6788` across segs 0x008B (display), 0x0971 (this updater), 0x0B1B
(audio/clear), so remaining work here is callback/FSM mapping rather than
another direct `b3` selector formula.

### Render/presentation callbacks (segment 0x0299) — located

`src/bloodprg.rs` now scans every direct far call into render/presentation segment
`0x0299`. The full export writes `bloodprg-render-call-sites.tsv`, and
`inspect-bloodprg` exposes the same `render_call_sites` array. Current binary
coverage is **143 direct calls across 32 target offsets**.

Named targets that are already tied to code behavior:

- `0x0299:0x0000` (`vga_dac_palette_load`): writes 0x300 bytes from `DS:SI`
  into the VGA DAC via ports `0x3C8/0x3C9` (256 RGB entries, 6-bit channels).
- `0x0299:0x0016` (`vga_dac_palette_clear`): zeros the same VGA DAC palette
  range; called during video setup and presentation-loop rebuild.
- `0x0299:0x00D6` (`fixed_8x8_text_render`): renders a NUL-terminated or
  `DH`-bounded string from `DS:SI` through the fixed 8x8 glyph table at
  `GS:0x5225` into the primary framebuffer; used by startup and navigation UI.
- `0x0299:0x013D` (`font_string_width_measure`): sums font advance tables for
  NUL-terminated text; `AX=0` selects the 10-row UI font
  (`0x7362/0x7412`), while `AX!=0` selects the dialogue font
  (`0x7802/0x78B2`).
- `0x0299:0x0176` (`ui_text_render_10row`): renders 10-row UI/menu text using
  tables `GS:0x7362/0x7412/0x7442`.
- `0x0299:0x0202` (`render_string_entry`): dialogue/UI string renderer using the
  embedded font tables.
- `0x0299:0x040E` (`framebuffer_rect_palette_remap`): clips a primary-framebuffer
  rectangle and replaces each existing pixel with `table[pixel]` from the
  256-byte table at `DS:SI`; direct callers use the palette-remap tables at
  `0x5F11` and `0x6011` for UI/HUD and transition effects.
- `0x0299:0x0498` (`planar_ui_text_render_10row`): renders 10-row UI text
  through VGA plane masks into framebuffer pointer `GS:0x521D`.
- `0x0299:0x05DE` (`planar_dialogue_text_render`): renders dialogue-font text
  through VGA plane masks into framebuffer pointer `GS:0x5219`; reached by the
  line-layout dialogue path at file `0x72F6`.
- `0x0299:0x06A0` (`subtitle_reveal_draw_wrapper`): the subtitle reveal renderer
  reached from file `0x94EE` after loading the `DS:0x5E5C/0x5E5E` text origin.
- `0x0299:0x075A` (`small_text_render`): NUL-terminated string renderer using the
  5-row small-font tables at `0x6FA8/0x7028`.
- `0x0299:0x0A2B` / `0x0B23` (`planar_horizontal_line_draw` /
  `planar_vertical_line_draw`): clipped line primitives into `GS:0x5219`, used by
  the dialogue updater's line-command table just before the subtitle reveal draw.
- `0x0299:0x0BB5` (`framebuffer_rect_outline`): clipped rectangle-outline wrapper
  using the primary-framebuffer horizontal/vertical line helpers.
- `0x0299:0x0BF5` (`framebuffer_dither_rect_fill`): clips a primary-framebuffer
  rectangle, seeds from the binary random helper `0x01CE:0x0B02`, then draws the
  binary pseudo-random black/`0xEF` dither pattern; the two direct callers pass
  `AX=3` for navigation/dialogue strip backgrounds.
- `0x0299:0x0CDC` (`framebuffer_rect_fill_clipped`): clips and fills a rectangle
  in primary framebuffer `DS:0x5221`.
- `0x0299:0x0DEB` (`scene_band_fill`): fills the current clipped framebuffer band.
- `0x0299:0x0E2F` (`secondary_framebuffer_band_fill`): fills the clipped band in
  secondary framebuffer `DS:0x5229`.
- `0x0299:0x0EB6` / `0x0ECB` (`framebuffer_copy_full` /
  `secondary_framebuffer_copy_full`): copy `0x3E80` dwords from `DS:SI` into the
  primary/secondary framebuffers.
- `0x0299:0x0EE0` (`vga_planar_to_linear_framebuffer_copy`): uses VGA Graphics
  Controller read-map select (`0x3CE`, index 4) to read four `0x3E80`-byte VGA
  planes from `DS:SI` and interleave them into one linear 320x200 RAM framebuffer
  at `ES:DI`; the direct caller captures `A000:0xC000` before sprite/object
  composition.
- `0x0299:0x0F3E` (`planar_framebuffer_copy`): copies planar/interleaved image
  data into primary framebuffer `DS:0x5219`.
- `0x0299:0x1037` (`resource_file_payload_load`): looks up a resource filename in
  the FS resource-name table (`FS:0x0C04 + AX*0x10`) and loads the file payload;
  high-bit `AX` callers read directly into the caller-provided `ES:DI` buffer,
  while non-negative callers route through the resource allocator/resolver.
- `0x0299:0x1140` (`sprite_slot_resource_frame_load`): resolves a resource frame
  through `0x04B9:0x0190` and loads it into the 32-byte sprite slot selected by
  `AX` in the `GS:0x6212` table. The resource layout is now modeled by
  `SpriteSlotFrameTable`: `word flags`, `word frame_count`, then `frame_count`
  packed dword offsets. Because the binary advances the table cursor by four
  bytes before applying the packed far-pointer math, each frame payload starts
  at file offset `4 + table_entry`. `flags & 4` is folded into slot state
  `0x0083`, selecting dispatch mode 1 or 3 for the RLE sprite payloads seen in
  real `.SPR` files.
- Full extraction now emits `sprite-frame-tables.tsv`, which audits every parsed
  `.SPR` file with the binary-derived flags, slot state/dispatch index, frame
  offsets, frame lengths, and frame-header dimensions/origin offsets.
- `0x0299:0x11BE` (`sprite_slot_frame_load`): loads one frame-table entry into
  the 32-byte presentation sprite slot selected by `AX`; four direct callers.
- `0x0299:0x1241` (`sprite_slot_state_update`): updates one presentation sprite
  slot state; 33 direct callers, including the VM post-update presentation clear
  path at `0x59DC/0x59E4`.
- `0x0299:0x127D` (`sprite_slot_position_update`): updates sprite slot
  screen-position words `+0x08/+0x0A` and sets the dirty bit when either changes.
- `0x0299:0x12B0` (`sprite_slot_range_mark_dirty`): marks an inclusive `AX..BX`
  sprite-slot range dirty.
- `0x0299:0x133D` (`sprite_slot_extent_update`): updates slot extent/source words
  `+0x0C/+0x0E` and marks the slot dirty/source-changed when needed.
- `0x0299:0x1467` (`sprite_slot_commit_dirty_range`): commits dirty slot current
  geometry into previous-geometry fields across an `AX..BX` range; also handles
  the `GS:0x5249` global clip snapshot into the dirty-rect list at `GS:0x6612`.
- `0x0299:0x14E1` (`sprite_slot_dirty_range_render`): walks the sprite slot range
  `AX..BX`, skips inactive slots, intersects each active slot rectangle with the
  dirty-rectangle list at `GS:0x6612`, dispatches the selected internal blitter
  from the slot state word, and clears the slot dirty bit after processing.
- The internal sprite blitter table lives at `0x0299:0x1592` (file `0x4522`).
  `sprite_slot_dirty_range_render` uses `(slot_state >> 1) & 7` as the table
  selector. Original slot-state bit 5 sets the horizontal-flip flag and bit 6
  sets the vertical-flip flag. Slot byte `+1 & 3` selects the transparent-mode
  destination remap behavior: `0` copies nonzero source pixels directly, `1`
  remaps destination pixels through `GS:0x5F11`, and `2`/`3` remap through
  `GS:0x6011`.
  - mode 0 -> `0x0299:0x15A6` (`sprite_blit_raw_transparent`): uncompressed
    transparent blit; source zero skips the destination, nonzero source pixels
    copy or trigger the selected destination-palette remap.
  - mode 1 -> `0x0299:0x172C` (`sprite_blit_rle_transparent`): RLE
    transparent blit with the same zero-skip/remap semantics as mode 0.
  - mode 2 -> `0x0299:0x1C18` (`sprite_blit_raw_opaque`): uncompressed opaque
    copy with no zero transparency or remap.
  - mode 3 -> `0x0299:0x1D46` (`sprite_blit_rle_opaque`): RLE opaque decode
    and copy with no destination remap.
  - mode 4 -> `0x0299:0x1FD2` (`sprite_blit_scaled_transparent`): fixed-point
    scaled transparent blit; source zero skips the destination.
  - modes 5..7 -> `0x0299:0x210A..0x210C`: unused single-byte near-return
    handlers.
  Modes 1 and 3 use the same row RLE control format: each row decodes until
  the frame-header stride is reached; control bytes `0x00..0x7F` copy
  `control + 1` literal bytes, while `0x80..0xFF` repeat the following byte
  `-control + 1` times. Mode 1 applies the decoded nonzero pixels as the same
  transparent/direct-copy or destination-remap mask as mode 0; mode 3 writes all
  decoded pixels opaquely.
  Mode 4 reads `source_width` and `source_height` from the first two frame-header
  words, computes 16.16 source steps as `(source_dim << 16) / dest_dim`, clips by
  advancing the source accumulators, samples with floor/nearest semantics, and
  ignores the flip/remap/origin-offset paths used by other blitters.
- `0x0299:0x210D` (`dirty_rects_copy_secondary_to_primary`): copies dirty
  rectangles described at `ES:DI` from secondary framebuffer `GS:0x5229` back
  into primary framebuffer `GS:0x5221`.

This is still a caller map, not a full renderer decompilation. It removes the
guesswork about which external render hooks the VM/presentation state machine
uses. All 32 direct render-segment target offsets are now named and tied to
instruction behavior, and the sprite-slot blitter dispatch table is now
decoded. The remaining render RE gap is feeding the recovered command/blitter
pipeline with real resource-frame lookup and validating it against DOS captures.

Rust now ports the safe framebuffer side of the recovered primitives in
`src/extract/render.rs`: clipped rectangle fill (`0x0CDC`), current scene-band
fill (`0x0DEB`/`0x0E2F` shape), full-viewport framebuffer copy
(`0x0EB6`/`0x0ECB`), palette-remap rectangle (`0x040E`), and VGA
planar-to-linear capture (`0x0EE0`), plus dirty-rectangle secondary-to-primary
copyback (`0x210D`). It also ports sprite blitter pixel semantics for modes
0..4: raw transparent, RLE transparent, raw opaque, RLE opaque, and scaled
transparent. The tests cover dirty-rect clipping, source row stride,
frame-header origin offsets, horizontal/vertical flip mapping, transparent zero
skip, destination-palette remap masking, RLE literal runs, RLE repeated-byte
runs, opaque zero writes, 16.16 scaled sampling, scaled clipping accumulator
advance, zero destination extents, dirty-rect copyback gating, sentinel stop,
and viewport clamping. `src/extract/render.rs` also
bridges `Ship3dSpriteSlotRenderCommand` values into those blitters, including
dispatch modes 0..4, no-op modes 5..7, dirty-rect clip conversion, and the
`DS:0x5F11`/`DS:0x6011` destination-remap selector. The higher-level
`render_ship_3d_dirty_sprite_commands_indexed()` helper renders an ordered
command stream into the secondary framebuffer, tracks missing/rejected frame
inputs, and runs the recovered dirty-rect copyback gate. The pipeline can now be
fed from parsed `.SPR` resource tables through `SpriteSlotFrameTable`, preserving
the binary's `4 + table_entry` frame-offset base and state-flag dispatch
selection. Full extraction emits those parsed table details to
`sprite-frame-tables.tsv` so renderer inputs can be audited against real
resource files. The character-HNM clear path uses the clipped fill helper
instead of open-coded per-pixel bounds checks.

### Audio subsystem (segment 0x0B1B) — located

- `son.snd` (voices/SFX) and `mus.snd` (music) are **per-scene temp files**
  extracted from `BLOOD.DAT`, with DOS handles at `[0x0C47]` (son) / `[0x0C49]`
  (mus). The scene-change cleanup (file 0x12E8) closes (int21 AH=3E) and **deletes**
  (AH=41, dx=0xA6 son.snd / 0xAE mus.snd / 0xCB) them before re-extracting.
- Voice playback + file reads (int21 AH=3F) are all in **segment 0x0B1B**
  (file 0xBA00–0xC0FF). No `lseek` (AH=42) → the SND bank is read **into memory**
  and clips are indexed via the in-bank offset table (same layout `src/snd.rs`
  decodes: u16 num_clips, (num_clips+1) u32 offsets, clip hdr `01 .. sr_code ..`,
  PCM from +6). Temp-file extraction near son.snd name ref at file 0xC19D.
- **SND clip player** (file ~0xB9DE): entered with **`AX` = clip index**.
  - In-memory path: `bp = 0x0BBF + clip*4` → clip table at `DS:0x0BBF` (4 bytes/
    clip: u16 offset + u16 len). `lds si,[0x0BB3]` (bank base) `+ [bp]` (offset)
    `+ 6` (skip clip header) → PCM; `cx = [bp+2]` = length. Matches `audio.rs`.
  - Streamed path: lseek `AX=0x4200` to `[bp]:[bp+2]` (u32), read `AH=0x3F`
    length `[bp+4]-[bp]`, via son.snd handle `gs:[0x0C47]` into buffer `gs:[0xBB7]`.
- **Direct SND entry callers:** `output/bloodprg-snd-call-sites.tsv` is now
  generated from `BLOODPRG.EXE` and lists all direct far calls to `0x0B1B:0x011D`
  plus the constant `AX` clip index recovered immediately upstream. The current
  set is ten calls: clips `0,1,1,2,3,4,5,5,5,6`; file `0x8534` is the
  text/presentation render-path call with `AX=0`. One call (`0x7BF8`) carries
  `AX=1` across a setup far call and is flagged in the TSV.
- **Direct SND bank-loader callers:** `src/bloodprg.rs` now separately scans
  direct far calls to `0x0B1B:0x0855` (file `0xC005`). This is a bank
  loader/extractor, not a clip player: `AX=0` builds the in-memory clip table at
  `DS:0x0BBF` and reads the full bank into `DS:0x0BB3`; `AX!=0` preserves the
  existing table and may materialize temp `son.snd` through the int21
  create/write path at `0xC191..0xC1C7`. The seven direct callers recover these
  `SI` path arguments from static DS strings: `0x0CFC` = `sn\tb.snd`, `0x0D06`
  = `sn\xxxxxxxxxxxx`, `0x0D16` = `sn\radio.snd`, and `0x0D23` =
  `sn\3D.snd`. This keeps bank selection separate from `0x011D` clip playback in
  the Rust decompilation path. The `sn\3D.snd` load/restore pair at files
  `0xB5DC`/`0xB610` is now also exposed through
  `inspect-bloodprg.presentation_3d_markers` as part of the ship/procedural-3D
  RE target.
- **Line-complete hold flag:** `gs:0x67BB` is now decoded as a post-reveal hold
  state, not a direct SND call. `0x94BA..0x94DD` sets `b35=aca*4`; `0x7378..0x738C`
  sets `b35=0x27cf*(aca/2)+6`; `0x115D..0x1188` consumes and clears the flag.
  Rust ports the arithmetic in `vm::reveal_complete_hold_ticks` and
  `vm::record_end_hold_ticks`. The shipped scene manifest supports `tb.snd#0`
  for subtitle sidecars, so Rust no longer cycles through `tb.snd` clips.
- Player internals: function entry ~0xB95x; `gs:0x0A5A` = current clip slot
  (`-1` = none → skips play). Buffer/stream state at `0x0BAB`/`0x0BAD`/`0x0BAF`.
  Mixer loop `0xBB6D..0xBB74` performs `lodsb; add al,es:[di]; rcr al,1; stosb`,
  i.e. unsigned PCM floor-average mixing. `src/snd.rs` ports this as
  `snd_mix_average` / `mix_unsigned_pcm_average`.
  Character voice selection is resolved in `script.rs`: `b3==0xFF` or `0x00`
  means no voice, and `b3 in 1..=N` maps to clip `b3-1`. Do not revive the old
  `b3==0xFF -> b4` branch; `b4` is the TEXT control-flag byte, not a clip index.

### BASIC VM nature (important for the renderer)

`analyze_handler.py --table` shows ~all opcode handlers (0xA0–0xD3) make **no far
calls** — they are BASIC language primitives (assign/arith/compare/branch), not
"play sound" commands. Presentation is data-driven: the 0xA6 line records
(`gs:0x6724`) + the per-frame dialogue/audio updaters consume the VM's state.
So the renderer should **walk the COD in execution order** (using the length
table + control flow) and read each 0xA6's (b1:b2 line index, b3 selector, b4/b5
flags), rather than expecting dedicated bg/music/voice opcodes.

### VM resource pointers and runtime state boundary

The resource profile selector at file `0x53A0` takes the desired profile index in
`AX`. If it differs from `DS:0x677E`, the routine releases the old five resource
offsets currently stored at `DS:0x6712` via far call `0x04B9:0x00F8`, stores the
new profile index in `DS:0x677E`, then copies five u16 offsets from
`FS:0x11F4 + AX*10` into `DS:0x6712`. Each copied offset is validated/loaded
through `0x01CE:0x059B`; failure jumps to the existing abort path at `0x5550`.
The same routine then clears the VM globals/lists and scans the selected DEB
object table to cache built-in object offsets.

The VM run wrapper at file `0x55A4` gates execution on `gs:0x67A8`, refreshes
BIOS RTC globals, then resolves the selected **five runtime resource offsets**
from `DS:0x6712` into far pointers at `DS:0x671C..0x672F` via far call
`0x04B9:0x0190`:

| pointer | observed use |
|---|---|
| `DS:0x671C` | COD pointer used by the main exec loop (`lds si, gs:[0x671C]`) |
| `DS:0x6720` | auxiliary COD pointer used by token walkers/helpers |
| `DS:0x6724` | runtime object/line-record state block used by record handlers |
| `DS:0x6728` | DIC pointer used by 0xA6 subtitle assembly (`gs:0x672A` segment) |
| `DS:0x672C` | DEB/object table pointer scanned as 20-byte records |

The `DS:0x6712` words are zero in the EXE image because they are populated from
the static `FS:0x11F4` profile table at runtime. The profile table entries are
resource IDs; each ID indexes the 16-byte resource-name table at `FS:0x0C04`.
Active profiles are:

| profile index | `D2` operand | resources loaded into `DS:0x6712` |
|---:|---:|---|
| 0 | 1 | `script1.cod`, `script1.bas`, `script1.var`, `script1.dic`, `script1.deb` |
| 1 | 2 | `script2.cod`, `script2.bas`, `script2.var`, `script2.dic`, `script2.deb` |
| 2 | 3 | `script3.cod`, `script3.bas`, `script3.var`, `script3.dic`, `script3.deb` |
| 3 | 4 | `script4.cod`, `script4.bas`, `script4.var`, `script4.dic`, `script4.deb` |
| 4 | 5 | `script5.cod`, `script5.bas`, `script5.var`, `script5.dic`, `script5.deb` |

Opcode `0xD2` at file `0x64B8` requests a profile switch by storing
`sign_extend(operand)-1` in `DS:0x6780`; operand 0 therefore stores `0xFFFF`,
the no-pending-script sentinel. The main loop checks `DS:0x6780` at file
`0x108E`, waits until presentation state is idle, then at `0x10C5` calls
`0x04DA:0000`, clears `DS:0x6780` back to `0xFFFF`, sets `DS:0x67A8=1`, and
runs the VM wrapper at `0x55A4`.

Save/load code at `0x1C3F`/`0x1CBD` serializes the current selected profile
index (`DS:0x677E`) and the `DS:0x6724` state block separately using a runtime
size derived from `DS:0x6716`. This is the boundary that blocks using raw
`SCRIPT*.VAR` line flag words as initialized display state: `SCRIPT*.VAR` is an
input image, while the game builds/serializes a live state block through this
pointer setup.

## Location backgrounds: planet (HNM) vs landscape (LBM) — root cause of wrong bg

Each DESCRIPT **Location** record carries TWO kinds of background:
- `FullHnm` (e.g. `ondoya.hnm`, `petrol10.hnm`) — the **planet** view (orbital
  HNM animation; the record's label literally says "planet Ondoya"). The current
  exporter uses this (`full_hnms.first()`) for dialogue → WRONG.
- **4× `Background` commands** = static **LBM** surface images in slots 1–4
  (e.g. Qx20: `petrol1f/1d/1g/1b.lbm`; Ondoya: `glacia1G.lbm` ×4) — the
  **landscape** the dialogue actually plays over.
Fix: dialogue background should load the Location's Background **LBM** (landscape),
not the FullHnm (planet). Slot 1–4 likely selects sub-view/zone (TBD which slot a
dialogue uses — check the scene/actor). LBM = IFF PBM (same format as the `.FD`
full-screen images per README; BLOOD.DAT `FD\*.LBM`).

## Intermediate Output Files

| File | Contents |
|------|----------|
| `re/labels.csv` | accumulated address labels (file-offset / SEG:OFF / DS: forms) |
| `re/bin/BLOODPRG.EXE` | unpacked target (MZ image == whole file, no decompression needed) |
| `script-text-flags.tsv` | extraction artifact listing every `0xA6` TEXT token's b3/b4/b5 control fields, decoded skip count, loop target, and flag summary |
| `script-branch-trace.tsv` | extraction artifact listing `execute_trace` branch/control events per script |
| `script-branch-decisions.tsv` | extraction artifact listing default observed conditional path and alternate target/path |
| `script-branch-coverage.tsv` | extraction artifact summarizing all text calls vs default executed trace coverage per script |
| `script-branch-scenarios.tsv` | extraction artifact forcing each branch decision's opposite condition once and measuring newly exposed text calls |
| `script-branch-scenario-dialogue.tsv` | extraction artifact joining each forced branch scenario trace to decoded text/actor/background rows, including explicit A6 skip-count and loop-target controls |
| `script-branch-scenario-dialogue-runs.tsv` | extraction artifact grouping branch scenario dialogue rows into renderer-ready run slices; full export also emits matching `branch-scenario-dialogue-run - ...mp4` files |
| `script-scene-events.tsv` | extraction artifact listing the exact `SceneEvent` stream consumed by default executed dialogue-run MP4s, including source/provenance plus decoded A6 skip-count and loop-target controls on subtitle events |
| `script-profile-scene-events.tsv` | extraction artifact listing the exact `SceneEvent` stream consumed by profile-sequence dialogue-run MP4s, including source/provenance plus decoded A6 skip-count and loop-target controls on subtitle events |
| `script-branch-scenario-scene-events.tsv` | extraction artifact listing the exact `SceneEvent` stream consumed by branch-scenario dialogue-run MP4s, including source/provenance plus decoded A6 skip-count and loop-target controls on subtitle events |
| `mp4/*.timeline.tsv` | per-dialogue-run sidecar artifact emitted beside generated MP4s; records segment start/end, reveal-complete time, subtitle hold end, active line id, voice/talk-HNM presence, chatter flag, and text for frame-aligned oracle comparison |
| `sprite-frame-tables.tsv` | extraction artifact generated from real `.SPR` files; lists the parsed frame-table flags, dispatch selection, frame offsets, lengths, and frame-header dimensions/origin offsets |
| `script-executed-dialogue.tsv` | extraction artifact joining `execute_trace` line order to decoded text/actor/background, including explicit A6 skip-count and loop-target controls |
| `script-executed-dialogue-runs.tsv` | extraction artifact grouping executed dialogue by script/background run; MP4 names correspond to run-level composites |
| `script-dialogue-runs.tsv` | extraction artifact grouping VM-order dialogue lines by script/background run |
| `bloodprg-render-call-sites.tsv` | extraction artifact generated from `BLOODPRG.EXE`; lists direct far calls into render/presentation segment `0x0299`, recovered target offsets, local `AX` setup, and current target names |
| `bloodprg-sprite-blitters.tsv` | extraction artifact generated from `BLOODPRG.EXE`; lists the internal sprite blitter table selected by `sprite_slot_dirty_range_render` |

## Verification Checklist

- [x] Ph1: binary identified (MZ / 386 / EMS+XMS, not flat 32-bit) — tools confirm header
- [ ] Ph2: decompression — N/A (image == whole file, no packer)
- [ ] Ph3: 3+ functions traced (dispatch loop + 2 handlers) and cross-checked
- [ ] Ph4: presentation constants (font/layout/timing/palette) extracted & validated
- [ ] Ph5: script-VM opcode table + scene/actor structs decoded
- [x] Ph6: generated cutscene compared against real-game capture with a
      frame-aligned pass threshold — **FIRST PASS achieved**. The boot studio
      logo `sq/mind.hnm` (Mindscape) reproduces the real DOSBox boot capture
      `frame_01` at `mean_abs ~= 1.09` (rmse 1.45, every screen-band region
      < 1.3), locked in as the passing scenario `intro-mind-frame01` in
      `accuracy/oracle-scenarios.tsv`. This validates the HNM(1) decoder +
      palette + native-scaling path end-to-end against real game output.
      `accuracy/compare_oracle.py` normalizes host-window captures and generated
      MP4 frames to 320x200, emits metrics, reports recovered screen-band region
      errors, scans generated-video timestamp windows, and ranks candidate
      generated videos against a reference frame.
      STILL OPEN for dialogue/gameplay scenes: those need scripted input (or a
      debug scene selector) to drive DOSBox to a matched scene before a
      threshold is meaningful; the 1-fps unattended capture cannot frame-align a
      fast cinematic like `sq/the_star.hnm` (best `mean_abs ~= 20` vs frame_04,
      phase-limited, not a content error).

## Reference Resources

- Codex thread (06-14) established the plan and the binary identification.
- `output/` already contains data-side extraction (DESCRIPT, scripts, HNM, SND).

## Next Tasks

### RE Investigation

- [x] Locate the VM token layer: decoder `token_advance` @0x62B6, walker
      @0x73AF, opcode length table @0x14338 (opcodes 0xA0–0xD3) — see Key Findings.
- [x] Decode 0xA6 TEXT token parameter block (b1..b5 + dict words) — data side.
- [x] Find the top-level opcode executor + dispatch + full handler table
      (vm_exec_loop @0x55F5, handler table DS:0x6EB0, 52 handlers resolved).
- [x] Find the 0xA6 TEXT handler @0x660C; decode b1..b5 fields (b1b2=line index,
      b3→gs:0x1FAB selector, b4/b5=control/active flags). See Key Findings.
- [x] Port `gs:0x1FAB` / `gs:0x6788` TEXT selector semantics:
      `src/vm.rs` models signed `b3 + 9` active-line ids and the one-based
      `son.snd` talk-clip selector (`0x00`/`0xFF` = no voice, `1..=N` =
      clip `N-1`). This centralizes the rule that previously lived as duplicated
      parser logic.
- [x] Port the TEXT active/already-shown display gate as an opt-in VM context:
      `b5 & 0x80` is the active/display bit, and `es:[line_index+2] & 0x8000`
      is the already-shown skip bit set by the 0xA6 handler. Default real-script
      traces remain ungated until the runtime initialization of `gs:0x6724` is
      recovered; raw `SCRIPT*.VAR` has incompatible pre-set flag words.
- [x] Port TEXT `b4 & 0x04` control-word parsing:
      both the VM walker and extractor skip the extra u16 before reading DIC word
      offsets, matching the `add si, 2` path in the handler.
- [x] Port the A6 `object+0x3A == 0x00C4` presentation-record gate as an opt-in
      `ExecutionContext` mode. It is deliberately not wired into real-script
      exports until the preceding C4 presentation setup semantics are complete.
- [x] Map the VM resource pointer setup boundary:
      `0x53A0/0x53C8` selects a five-offset resource profile from
      `FS:0x11F4 + AX*10` into `DS:0x6712`; `0x55A4/0x55D9` resolves those
      runtime offsets into COD, state, DIC, and DEB/object far pointers at
      `DS:0x671C..0x672F`, while save/load code serializes the `DS:0x6724`
      runtime state block separately.
- [x] Decode the `FS:0x11F4` resource-profile table:
      static resource IDs map through `FS:0x0C04` to five profiles for
      `script1`..`script5`; opcode `D2` stores operand-1 in `DS:0x6780` for the
      main-loop profile handoff.
- [x] Model `D2` cross-script profile scheduling in Rust execution traces:
      `ExecutionTrace` records D2 profile requests and writes the pending
      profile word into VM state; `execute_script_profile_sequence` follows the
      last non-sentinel pending profile through the decoded script profiles only
      when the recovered main-loop idle gate allows dispatch.
- [x] Export cross-script profile sequences from the extractor:
      `script-profile-runs.tsv` and `script-profile-executed-dialogue.tsv`
      preserve the DOS main-loop SCRIPT1->SCRIPT2->... handoff order using the
      binary-derived resource profile table.
- [x] Consume profile-sequence dialogue rows in the event renderer/video grouping:
      `profile-dialogue-run` MP4s group by global execution order instead of
      per-script order, while the old per-script videos remain for comparison.
- [ ] Complete the `gs:0x6724` runtime object/state layout: `es:[di]` kind,
      `es:[di+2]` flags, `+0x3A` A6/C4 presentation subrecord, and remaining
      C4 setup paths needed before enabling the A6 gate in exports. The
      `0x5D8F..0x5E1F` C4 reciprocal post-update write, the `0x67B6` pair-write
      guard, the active-object scan subset, the kind-1 presentation start/stop
      globals, the deferred record drain, and the kind-2 handoff predicate are
      ported and surfaced through `ExecutionTrace::post_update`; the kind-2
      `vm_control_flow` target is now applied as a COD PC handoff inside
      `execute_trace`. Direct SND and render caller maps exist; detailed
      callback semantics and shared engine globals remain pending.
      `execute_script_profile_sequence()` now carries each
      profile's mutated VAR state across D2 handoffs/re-entry, so repeated
      profile runs no longer restart from pristine `SCRIPT*.VAR`.
- [x] Map the VM named-object startup globals from `0x5486`: Rust
      `ExecutionContext` now carries the built-in DEB offsets for `blood`, `orxx`,
      `arche`, `Honk`, `menu`, `Ark`, `Scruter_Jo`, and kind-5 `vbio`.
- [ ] Verify audible `tb.snd` chatter trigger path, if any. `gs:0x67BB` itself is
      now decoded as post-reveal hold state rather than a direct SND caller.
- [ ] Map the presentation opcodes among the handler table: which set background,
      music (mus.snd), HNM actor, voice (son.snd), wait, clear. Start with the
      remaining C1/C2/CA/CB/CD handlers and presentation callbacks rather than
      expecting direct media-play opcodes.
- [x] Port 0xB7 bit-flag semantics. `src/vm.rs` exposes
      `VmToken::BitFlag`, applies high-bit-first set/clear writes in mode 0, and
      `execute_trace` evaluates mode-1 bit tests with optional `A1` inversion.
- [x] Port 0xB8/0xB9/0xBD pair-record semantics. `src/vm.rs` exposes
      `VmToken::PairRecord`, applies mode-0 two-word writes, and evaluates
      mode-1 pair compares through the branch stack. The handler's secondary
      `gs:0x6752+0x16` bookkeeping clear remains outside the current model.
- [x] Expose 0xC1/0xC2 line-record state tokens. `src/vm.rs` keeps their raw
      record/operand words and optional mode-1 inversion as
      `VmToken::RecordState`, and `script-disassembly.tsv` can now show true
      `record_state` rows instead of raw byte spans. Direct mode-1 compares are
      now executed when concrete host-state records are available. The direct
      C1 mode-0 success write `{0x00C1, operand, 0x0002}` is also applied when
      the DEB-derived context proves the owner active. C2 mode-0 now applies
      the `gs:0x6D60` kind-field write, the kind-2 active-line side effect
      (`gs:0x6788 = 0x27`), and the kind-0x0400/helper-0x7409 active-line side
      effect (`gs:0x67AA|=2`, `gs:0x6788 = 0x2B`) when `ExecutionContext`
      supplies the matching `descript.des` directory name. Extractor trace paths
      seed those names from parsed `DESCRIPT.DES`; the ship-3D kind-`0x10`
      C1 mode-0 path is wired when `ExecutionContext` supplies navigation
      records, object-table order, and the live `DS:0x6886` scratch bytes; the
      optional position runtime ports the raw-operand `1/2`
      distance/selector-`0x11` redirect before the source-list gate. C1 mode-1
      now also compares the raw-operand `1/2` selector-`0x11`/selector-`0x13`
      resolved slot when direct record state is not already `0x00C1`; known C1
      mode-0 failed writes now branch through the recovered A0/A1 stack instead
      of falling through.
- [x] Expose 0xCA/0xCB global condition tokens. `src/vm.rs` preserves the
      consumed compare operands as `VmToken::GlobalWordCompare` and
      `VmToken::GlobalPairCompare`; `execute_trace` evaluates their branches
      when `ExecutionContext` supplies `gs:0x0AA6/0x0AA8/0x0AAA`. The binary RTC
      writers are recovered; host replay chooses values via `with_bios_rtc`, and
      the extractor now emits RTC branch-scenario replays from real `CA`/`CB`
      operands. `script-disassembly.tsv` now uses the mode-aware VM walker for
      these tokens instead of ad hoc raw-byte spans.
- [x] Expose 0xCD record-triple tokens. `src/vm.rs` preserves the consumed
      record/first/second words and optional `A1` inverted-compare prefix as
      `VmToken::RecordTriple`, and `execute_trace` evaluates the direct mode-1
      record-triple compare. Resolved-table mode-0 side-effect execution remains
      pending.
- [x] Decode the cs:0x0F29 and cs:0x06D4 sub-dispatch tables. Table starts,
      indirect call sites, raw handler-offset arrays, target file offsets, and
      the 24-byte actor/object struct iterated at 0x7E09 are documented; handler
      semantics still need permanent names beyond temporary table-entry labels.
- [x] Reconcile 0xC4 length and operands. The handler consumes two u16 operands,
      writes a 6-byte record entry on success, and `src/vm.rs` now exposes both
      words plus optional mode-1 `A1` inversion instead of reducing the token to a
      single actor id.
- [x] Port 0xC3 record-link semantics. `src/vm.rs` exposes
      `VmToken::RecordLink` with optional mode-1 inversion, the context-aware VM
      applies guarded mode-0 writes, known mode-0 branch-fails, and direct
      mode-1 compares using DEB object offsets, and parser tests lock in that
      `C3` does not restore speaker context after a `C9` clear.
- [x] Port 0xC5..0xC8 record-entry token semantics. `src/vm.rs` exposes the
      family as `VmToken::RecordEntry` including raw operand and recovered
      stored-related slot; disassembly now emits `record_entry` rows.
      Successful mode-0 writes for C5/C6/C7/C8, guarded mode-0 failure branches
      for C5/C7/C8, and direct mode-1 compares are now executed.
- [x] Port 0xC9 record-clear speaker lifetime semantics. `src/vm.rs` exposes
      `VmToken::RecordClear`, the bounded interpreter clears the active actor
      when its talk-field record is cleared in either VM mode, and the script
      parsers stop carrying actor/background context past matching `C9` tokens.
      The port also applies the selector-0x13 related C4 subrecord clear and the
      `gs:0x252A/0x2531` presentation gate reset.
- [x] Map subtitle presentation constants: subtitle position, reveal rate, and
      reveal palette indices are tied to `0x5E5C/0x5E5E`, `0xB31/0xACA`, and the
      `0x06A0` wrapper. Rust derives the default reveal rate from
      `DEFAULT_SUBTITLE_TEXT_SPEED_STEP=5`, and uses it consistently for drawing,
      line duration, and line-complete chatter placement. Subtitle segment
      lifetime uses the decoded `reveal_complete_hold_ticks` timer, with voice
      PCM length acting as the minimum only when it is longer.
- [ ] Remaining presentation timing: recover player/config text-speed selection,
      HNM actor reset/loop policy, and audio mix levels.

### Renderer Integration (replaces skill's "Web Port")

- [x] Embed the recovered opcode-length table + 0xA6 decoder as a verified Rust
      module `src/extract/vm.rs` (token type + `walk()`). Unit test
      `table_matches_binary` confirms the table is byte-exact vs BLOODPRG.EXE;
      `walks_synthetic_cod` confirms 0xA6 decode incl. the `b4&0x10` loop word.
- [x] Found + fixed two production decoder bugs in `decode_text_call_at`
      (`src/extract/script.rs`), grounded in the recovered fixed layout:
      (1) it only accepted `b5 == 0x80` exactly, dropping lines whose b5 carried
      extra flag bits (0x90/0xA0); (2) it didn't skip the `b4 & 0x10` loop-target
      word, misreading it as a dict offset and dropping the line.
      **Impact: ~18% of all dialogue (666 / 3682 lines: 493 extra-flag + 173
      loop) were being silently dropped; now recovered.** Covered by unit tests
      `decode_text_tests::*`. (`vm.rs` handles the same cases.)
- [x] **Whole-script linear walk WORKS** (control-flow interpreter NOT needed).
      The earlier "desync" was the variable-length opcodes `0xA8/0xAC/0xCC/0xD3`
      (helper 0x6293 scans for a `0x0000` word terminator — see `scan_zero_word`).
      With that fixed, `vm::walk` decodes all 5 scripts cleanly to the `0xFF`
      end marker, 0 invalid (SCRIPT2 = 3271 tokens / 1157 text). This is the
      foundation for execution-order scene-state tracking.
- [x] Object-ref opcodes decode correctly now: `0xC4` = 5-byte actor/record
      operation (`record_offset = object_offset + 0x3A` talk field for speaker
      tracking, plus a second related-record word; 71/95 first operands resolve
      to Characters), `0xC3` = non-speaker record link, and `0xC9` = record
      clear. Location is NOT set by referencing a location object.
### Runtime object-state model (CONFIRMED) — path to accurate location

- The VM keeps a **runtime object-state area** addressed `es:[bx+di]` with
  `les di, gs:[0x6724]`; `bx` = a variable/field address. `obj+24` in this space
  is a character's **current location** (LOCATION_FIELD). The 58%-covered lines
  read the *static initial* copy from `SCRIPT*.VAR`; runtime changes live here.
- **`0xAF` (and family @0x6946) is a CONDITIONAL**: `IF state[op1] == op2 { skip
  tokens }` (calls 0x6462 to skip). In SCRIPT2, 12 of these test
  `state[char+24] == <Location>` (e.g. usine/Ark/Hita) — i.e. the script branches
  on a character's current location. Confirms `obj+24` = location and that the
  state area holds it.
- **Implication:** because location-assignments are gated by these conditionals,
  computing the *actual* per-line location requires **evaluating** the script —
  a bounded interpreter over the `gs:0x6724` state area: object-field assignments
  + the comparison/branch opcodes (`0xAF` family etc.). The COD is linearly
  walkable (no jump-table chasing), so this is a state-machine interpreter, not a
  full CPU emulator. This is the genuine "replay the game's logic" the goal asks
  for, and the route to ~100% location/speaker coverage.
- [x] Assignment opcodes decoded and ported. 0x6863 family (b1/b4-b6/be-c0), 7-byte:
      `op [op1:u16] [operator:u8] [op2mode:u8] [op2:u16]`; operators `0xF5`=set,
      `0xF6`=add, `0xF7`=sub (+ comparisons `0xF0`=ne `0xF3`=le… for conditionals);
      writes `state[op1]` in mode 0 (`mov es:[bx+di],cx` @0x68FD). op2mode
      `0xC0/0xC2` = indirect (`op2 = state[op2]`). The Rust interpreter evaluates
      the recovered mode-1 branch comparisons without applying their mode-0
      state writes.
- [x] Ported the direct 0xC4 record write and evidence-based mode-1 compare.
      Rust now writes `{0x00C4, related, 0}` in mode 0, decodes optional `A1`
      inversion in mode 1, and branches only when a concrete record entry is
      available in host VM state. Zeroed static `SCRIPT*.VAR` line-record slots
      remain unresolved guarded actor contexts until the full `gs:0x6724` runtime
      table model is wired in.
- [x] Ported the other mode-0 mutation handlers to Rust: 0x6902 family (AE/B0)
      bitmask set/clear (`or es:[bx+di],ax` / `and es:[bx+di],~ax`) and 0x6946
      family (AD/AF/B2/B3/BA/BB/BC) direct assignment (`mov es:[bx+di],ax`
      @0x69C2). Rust now also mirrors the 0x6946 write-side sentinel bookkeeping
      for fields assigned to `blood`/`0xFFFF`: helper `0x5FD8` removes an owner
      object from the 16-word list at `DS:0x6D3E`, and helper `0x5FF6` inserts it
      before storing `0xFFFF`.
- [x] Ported the 0x6946 mode-1 special-object compare. Script metadata init at
      file `0x549a..0x54a1` matches the DEB object name `blood` (built-in string
      `DS:0x67BE`) and stores its object offset in `gs:0x674E`. The 0x6946 mode-1
      handler then remaps a RHS operand equal to `gs:0x674E` to `0xFFFF` before
      equality/inversion testing (`0x6963..0x696e`). `ExecutionContext` now
      carries that DEB-derived sentinel, and branch traces/scenario speech use it
      for game-accurate `AD/AF/B2/B3/BA/BB/BC` compares.
- [x] **Interpreter prototype VALIDATED** (Python): init state from `SCRIPT*.VAR`,
      walk + execute 0x6863-family assigns, track `state[actor+24]` per 0xA6 line.
      Location coverage **58% → 63%** (SCRIPT2 61%, SCRIPT3 68%, SCRIPT4 65%,
      SCRIPT5 67%, SCRIPT1 0% = trivial title script).
- Decision-relevant: the gain is **modest** because the dominant gap is the **22%
      of lines with no tracked speaker** (no `0xC4` even with cross-function
      persistence) — many are likely **narrator/system lines that legitimately
      have no character/location** (so the "gap" is partly correct-as-is).
      Conditional/branch evaluation would lift it further, but the
      speaker-attribution cap bounds the realistic ceiling.
- [x] Ported the interpreter to Rust (`vm::interpret_line_states`, tested) and
      INTEGRATED it: `parse_script_speech` runs `resolve_runtime_backgrounds` per
      script and each `0xA6` line prefers the executed **runtime** location
      (`state[actor+24]` → DESCRIPT) over the static initial one, with no
      hardcoded fallback. **Shipped result: background coverage 56% → 61%** in the
      exported `script-speech.tsv` / videos.
- [x] Ported the first branch-aware execution trace to Rust (`vm::execute_trace`),
      grounded in the A0/A1/0x6462 control stack:
      - A0 @0x6559: `gs:0x67AD=1`, push the u16 target operand into the stack at
        `gs:0x6820 + gs:0x6884`, then `gs:0x6884 += 2`.
      - A1 @0x6572: `gs:0x67AD=0`, pop one stack slot only when
        `gs:0x6884 != 2` (the first slot remains as the current block target).
      - branch-fail helper @0x6462: `gs:0x6884 -= 2`; `si = [0x6820+gs:0x6884]`;
        `gs:0x67AD=0`.
      - A4 @0x65DB and A9 @0x6830 direct jump/reset behavior is modeled for
        inspected script paths.
      `execute_trace` now evaluates mode-1 conditionals for the 0x6863 signed
      compare family, 0x6902 bitmask family, and 0x6946 equality family, while
      retaining the linear `interpret_line_states` path for all-possible-line
      manifests. Real-script smoke via `inspect-vm <COD> <VAR>` reaches
      `EndMarker` for all scripts: SCRIPT1 102 executed lines / 38 branch events;
      SCRIPT2 169 / 327; SCRIPT3 327 / 553; SCRIPT4 145 / 229; SCRIPT5 258 / 387.
- [x] Wire branch-aware initial-state execution into the old per-character
      dialogue video generator: `create_character_videos` consumes
      `ScriptExecutedSpeechLine`, groups each character by script/location, and
      orders lines by `execute_trace` sequence index instead of raw COD offset.
      That generator is now legacy/direct-`--snd` inspection only; the default
      full export uses the run-level renderer below and no longer writes
      `script-dialogue-videos.tsv`.
- [x] Add branch-aware run-level dialogue composites: the full exporter now
      renders `script-executed-dialogue-runs.tsv` groups as
      `executed-dialogue-run - ...` MP4s, tracking `ShowSpeaker` events so a
      single scene can switch actor SND banks/talk HNMs without splitting by
      character.
- [ ] Further gains: make comprehensive dialogue generation cover alternate
      branches. The current exporter is no longer the linear all-lines path and
      no longer has to split one executed run by actor, but it still represents
      only the default initial-state execution. Full coverage needs branch
      enumeration or scenario selection. Bounded by the ~22% no-speaker lines
      (many are legitimately narrator/locationless).
- [x] Add branch planning artifacts: `script-branch-decisions.tsv` records each
      concrete conditional's observed path and alternate path/target, while
      `script-branch-coverage.tsv` summarizes static `0xA6` text calls vs the
      default executed trace per script. These are the manifest inputs for
      scenario-selected or branch-enumerated rendering.
- [x] Add branch override execution: `vm::execute_trace_with_overrides` can force
      a specific condition result, and `script-branch-scenarios.tsv` applies the
      opposite path to every concrete branch decision once, measuring text-call
      deltas. This turns the branch coverage gap into executable scenario data.
- [x] **Event-triggered scene coverage (sess 004) — the decisive coverage lever.**
      Root cause of the remaining gap: video generation only ever entered the ~22
      of ~65 named COD functions the main trace (entry 0) reaches; the rest are
      event-triggered scenes (menu/object handlers) the flow never calls, holding
      ~40% of dialogue (e.g. SCRIPT4/clay3 "Honk filled me in"). This dialogue is
      NOT runtime-gated: the static `parse_script_text_calls` analysis already
      resolves each line's actor + runtime background per offset (only the speaker
      is caller-set, so cold execution loses context — the static context is the
      correct source). `parse_script_uncovered_speech` emits renderable lines
      (resolved actor+bg) for never-executed functions, deduped vs executed
      offsets, tagged `fn:<script>:<function>` so each groups into a per-function
      scene run, rendered as `function-dialogue-run - …` videos by the existing
      renderer. **Result: unique dialogue-text coverage 57.9% → 95.8% (+933 texts;
      1524 lines / ~180 scenes), verified rendering (clay3 → Anna_Haf on Magnus).**
      Remaining ~4% are lines with no statically-resolved background (a smaller
      follow-up: infer their bg or mark narrator/locationless).
- [x] Expose TEXT control flags: `script-text-flags.tsv` lists every `0xA6`
      token's `b3`, `b4`, `b5`, active bit, conditional skip count, loop target,
      known parse-control bits, and still-unknown `b4` payload bits. This gives
      the subtitle sound/animation audit a concrete Rust artifact instead of
      burying those fields in raw token params.
- [x] Correct subtitle chatter timing: `src/extract/subtitle_sfx.rs` now follows
      the recovered `0x94BA..0x94DD`/`0x115D..0x1188` state machine by scheduling
      one `tb.snd` chatter event after a subtitle finishes revealing, instead of
      the previous one-SFX-per-character approximation.
- [x] Emit binary-derived SND entry call sites:
      `bloodprg-snd-call-sites.tsv` scans `BLOODPRG.EXE` for direct far calls to
      `0x0B1B:0x011D`, recovers the upstream constant `AX` clip index, and flags
      the one call where `AX` is carried across a setup far call. This gives the
      chatter/voice SFX audit a test-backed caller map instead of handwritten
      disassembly notes.
- [x] Emit binary-derived render/presentation call sites:
      `bloodprg-render-call-sites.tsv` and `inspect-bloodprg.render_call_sites`
      scan all direct far calls into segment `0x0299`, recovering 143 call sites
      across 32 target offsets. Named targets include the text renderers,
      fixed 8x8/UI font helpers, planar text/line primitives, VGA DAC palette
      load/clear callbacks, framebuffer fill/copy helpers, subtitle reveal
      wrapper, palette-remap and dither-rectangle fills, resource payload load,
      VGA planar capture, sprite-slot frame/position/extent/dirty-range
      callbacks, dirty-range rendering, and dirty-rectangle copyback.
- [x] Decode sprite blitter dispatch modes:
      `bloodprg-sprite-blitters.tsv` and
      `inspect-bloodprg.sprite_blitter_dispatch` expose the table at
      `0x0299:0x1592`, selected by `(slot_state >> 1) & 7`, with raw/RLE
      transparent, raw/RLE opaque, scaled transparent, and no-op modes named.
      The remaining work is Rust-porting the pixel loops and checking them
      against oracle captures instead of guessing sprite composition.
- [x] Port raw sprite blitter modes:
      `src/extract/render.rs` now has tested Rust helpers for mode 0 raw
      transparent sprites and mode 2 raw opaque sprites. The tests cover
      dirty-rect clipping, source stride, transparent-zero skip, destination
      remap-as-mask behavior, zero writes in opaque mode, and horizontal/vertical
      flip mapping recovered from segment `0x0299`.
- [x] Port RLE sprite blitter modes:
      `src/extract/render.rs` now has tested Rust helpers for mode 1 RLE
      transparent sprites and mode 3 RLE opaque sprites. The shared decoder
      follows the recovered signed control-byte format, then reuses the raw
      blit core for clipping, flip, transparency, remap, and opaque writes.
- [x] Port scaled transparent sprite blitter mode:
      `src/extract/render.rs` now has a tested Rust helper for mode 4 scaled
      transparent sprites. It follows the recovered 16.16 fixed-point source
      stepping, clipped accumulator advance, floor/nearest sampling, and
      transparent zero skip.
- [x] Pin ship/procedural-3D presentation markers:
      `inspect-bloodprg.presentation_3d_markers` exposes the ship/navigation
      FSM entry, HUD bit-3 initializer, temporary `sn\3D.snd` load/restore path,
      VGA planar band-copy routine, transition/depth-step helpers, and the key
      DS state variables. This confirms the 3D/minigame work must continue from
      binary-derived runtime state, not from data-file heuristics.
- [x] Port recovered ship 3D transition and planar band-copy primitives:
      `src/ship3d.rs` implements the `0xB692` transition flag updater, `0xB75C`
      depth/plane offset stepper, and `0xB6DD` two-band planar page copy with
      tests for the original 80-byte row math, `AL`-only stepping, and
      `DS:0x524F` scroll-value update. This still is not the full 3D minigame;
      it is the recovered software presentation primitive that the future
      runtime/`wgpu` path must preserve or replace with equivalent output.
- [x] Pin ship 3D target/navigation control markers:
      `inspect-bloodprg.presentation_3d_markers` now also exposes the alternate
      framebuffer band-copy call at `0xB24C`, target selector at `0xB2BB`,
      navigation update branch at `0xB34E`, and the DS state bytes/words around
      `0x250B`, `0x251B`, `0x252A..0x252C`, `0x2532`, `0x2537`, and `0x27D8`.
      These are the next decompilation targets before a real `wgpu` minigame
      frontend can be game-accurate.
- [x] Port ship 3D target selector:
      `src/ship3d.rs` now implements the `0xB2BB` target-record selector with
      tests for phase prepass/gating, primary-list target adjustment, fallback
      table behavior, no-selection `AX=0`, and the `-1` sentinel opening
      transition (`DS:0x252F=1`, `DS:0x2531=6`).
- [x] Port ship 3D interpolation gate:
      `src/ship3d.rs` implements the `0x008B:0x0FAD` four-word interpolation
      gate used by the target selector. Tests cover carry-set completion,
      tick increment, signed truncating division/multiplication, and binary
      `idiv` error shapes. `inspect-bloodprg.presentation_3d_markers` exposes
      the gate plus `DS:0x0ADA` duration and `DS:0x0ADB` current tick.
- [x] Port ship 3D target-list layout prepass:
      `src/ship3d.rs` implements the selector-mode `0x071E:0x0C48` rectangle
      math that writes `DS:0x2AAB` from measured label widths, center
      `DS:0x0AC6`, and flags `DS:0x0ADC/0x0ADD/0x27E6`. Tests cover default
      width floor, widest-label growth, extra-entry sizing, and unsigned
      over-height wrapping. The actual target-list text draw branch of the same
      helper is still pending.
- [x] Port ship 3D target-list hit-test state:
      `src/ship3d.rs` now implements the non-query state branch of
      `0x071E:0x0C48` before text drawing. It clears `DS:0x27C7/0x27E7`,
      tests signed mouse bounds against the centered rectangle, computes the
      1-based hover row as `(mouse_y - (y + 4)) / 0x0B + 1`, requests
      presentation mode `6` for hover, mode `7` for activation, and mode `1`
      when the cursor leaves the rectangle. The activation flag commits
      `DS:0x27E7` and plays `sn\3D.snd` clip 0; the Rust result exposes that as
      `play_select_sound`. Return `AX` matches the original sign-extended
      `selected_row - 1` shape, so no committed selection returns `0xFFFF`.
      The remaining branch at this helper's boundary was the target-list text
      draw, now ported separately.
- [x] Port ship 3D target-list draw events:
      `src/ship3d.rs` now exposes the recovered draw branch of
      `0x071E:0x0C48` as target-list UI draw commands. The binary consumes the
      `DS:0x2AB3` width table, centers each row inside `x + 0x0A` and
      `width - 0x14`, starts drawing at `y + 4`, advances rows by `0x0B`, and
      calls `0x0299:0x0176` with colors `0xE8` default, `0xEF` hover, and
      `0xFE` active-click. `DS:0x27C7` is a destructive hover countdown during
      drawing, so later rows wrap after the highlighted row. If
      `DS:0x0ADD & 1` is set, the helper draws the extra static `CANCEL` string
      at `DS:0x0174`. This gives the future software oracle and `wgpu` frontend
      an exact event stream for the target-list UI instead of inferred labels.
- [x] Port ship 3D navigation-choice hit-test state:
      `src/ship3d.rs` now models the `0x071E:0x0E02` navigation-choice preamble
      before the five-entry handler table. The routine blocks on
      `DS:0x1FB2`, `DS:0x2736`, `DS:0x2737`, `DS:0x259B`, `DS:0x0B13`, or
      `DS:0x67AC`, skips mouse hit-testing when `DS:0x2A19` already holds a
      committed choice, and only scans new input when `DS:0x2795` is in
      `0x28..=0x3C`. The hit-test uses the helper-returned dynamic axis biased
      by `0x2D` to build slanted x bounds, computes the y origin as
      `0x48 + abs(axis) + abs(axis)/4`, divides by `0x12 - abs(axis)/8`, and
      rejects rows `>= 5`. Hover resets DAC entries `0x7B..0x7F` and highlights
      `0x7B + row`; activation writes requested presentation mode `5`, commits
      `DS:0x2A19 = row + 1`, ORs `DS:0x2793 |= 0x0C`, sets
      `DS:0x279B = 0x5A`, `DS:0x2565 = 1`,
      `DS:0x253F = 0x50 + row * 0x12`, configures the target-list layout flags,
      sets interpolation duration `DS:0x0ADA = 0x0A`, and plays SND clip 4.
      Once `DS:0x2793 & 8` clears, an existing `DS:0x2A19` dispatches through
      the `CS:0x0F29` five-entry handler table. The handler bodies remain
      separate decompilation targets.
- [x] Port ship 3D navigation-choice handler 0:
      table entry 0 at `0x071E:0x0F33` checks `DS:0x2565 & 1`; when clear it
      returns without side effects. When set, it writes deferred record type
      `0x00C3` to `DS:0x6768`, writes named object `Honk` from `DS:0x6754` to
      `DS:0x676A`, and clears `DS:0x2565`. Rust exposes this as
      `run_ship_3d_nav_choice_handler_0()` returning an explicit deferred-record
      effect for the VM/event renderer.
- [x] Port ship 3D navigation-choice handler 1:
      table entry 1 at `0x071E:0x0F4C` handles target-list selection. On phase
      bit 0 it resets interpolation tick `DS:0x0ADB`, adds four bytes to each
      non-`-1` target record in the `DS:0x2B13` list, runs the target-list
      layout prepass with `DS:0x27E6=1`, then increments phase to bit 1. While
      phase bit 1 is set, it waits for the `0x008B:0x0FAD` interpolation gate;
      active interpolation returns immediately, while completion clears
      `DS:0x2565` and falls through to the live target-list query. Query
      `AX=-1` leaves the choice armed. A selected `-1` clears `DS:0x2A19` and
      bit `0x04` in `DS:0x2793`; a selected target instead subtracts four bytes,
      writes deferred `C3` related pointer `DS:0x676A`, sets `DS:0x6768`, reloads
      `sn\radio.snd` via the SND bank loader, then clears the same choice/HUD
      state. Rust exposes this as `run_ship_3d_nav_choice_handler_1()`.
- [x] Port ship 3D navigation-choice handler 2:
      table entry 2 at `0x071E:0x0FDD` is the special-slot target-list variant.
      On phase bit 0 it scans the 16-word `DS:0x6D3E` special-slot list, skips
      zero words, writes each non-`-1` slot plus four bytes into `DS:0x2B13`,
      copies the `-1` sentinel, resets interpolation tick `DS:0x0ADB`, runs the
      target-list layout prepass, and advances to phase bit 1. It waits on the
      same interpolation gate as handler 1. Query `AX=-1` leaves the choice
      armed. A selected `-1` clears `DS:0x2A19` and bit `0x04` in `DS:0x2793`;
      a selected target subtracts four bytes into `DS:0x676A` and sets
      `DS:0x2751 = 1` before clearing the same choice/HUD state. It does not
      write `DS:0x6768` or reload `radio.snd`.
- [x] Port ship 3D navigation-choice handler 3:
      table entry 3 at `0x071E:0x1068` is a one-shot static record-link handler.
      If phase bit 0 is set, it copies `DS:0x6756` into deferred related record
      `DS:0x676A`, writes deferred type `0x00C3` to `DS:0x6768`, clears
      `DS:0x2565`, and reloads `sn\radio.snd` through the same SND bank-loader
      path offset used by handler 1. It does not clear `DS:0x2A19` or the
      target-list HUD bit. Rust exposes this as
      `run_ship_3d_nav_choice_handler_3()`.
- [x] Port ship 3D navigation-choice handler 4:
      table entry 4 at `0x071E:0x108C` is the five-way menu/action handler. On
      phase bit 0 it queries layout for target list `DS:0x2567`, resets
      interpolation tick `DS:0x0ADB`, advances `DS:0x2565`, and copies the
      four-word layout rect `DS:0x2AAB` into `DS:0x25CF`. It then waits on the
      same interpolation gate. Query `AX=-1` returns without clearing the armed
      choice. Selection 0 sets menu latches `DS:0x259B/0x259C`; selection 1
      toggles `mu\tablo2.voc` state through `DS:0x0ADE/0x0BA0/0x0BA3/0x0D30`
      and switches active target-list pointer `DS:0x2569` between `0x2578` and
      `0x2581`; selection 2 sets `DS:0x2738/0x2736`; selection 3 sets
      `DS:0x2738/0x2737`; selection 4 sets sound gate `DS:0x0B13 = 2` and
      clears activation latches `DS:0x0A3E/0x0A40`. Any nonnegative selection
      clears `DS:0x2A19` and bit `0x04` in `DS:0x2793`. Rust exposes this as
      `run_ship_3d_nav_choice_handler_4()`.
- [x] Port ship 3D navigation trigger prelude:
      when trigger byte `DS:0x27D8` is set, `0x0A9A:0x03AE` first copies pending
      presentation word `DS:0x0A36` into requested presentation state
      `DS:0x0A32`, increments the active target counter slot (following the
      `0x80` redirect through `[current+0x14]` when present), and calls
      `0x04DA:0x1D4E` to build the zero-terminated candidate list at
      `DS:0x2B53`. That helper consumes the source list at `DS:0x6886`, skips
      named Honk (`DS:0x6754`), and retains only kind-2 records whose byte
      `+2` has bit `0x01` set. The trigger scan keeps advancing until either
      the current target allows any candidate (`current[+2] & 0x02`) or the
      candidate's `+0x18` relation equals current target `DS:0x251B`. A candidate
      related to Ark (`DS:0x6758`) opens the target-list branch unless Ark is
      the current target; otherwise the branch writes deferred type `0x00C4` to
      `DS:0x6768`, deferred related candidate to `DS:0x676A`, and calls the
      follow-up handler with `candidate + 4`. If no candidate is accepted, it
      opens the target list: sets HUD bit `0x04`, resets interpolation tick,
      sets duration `DS:0x0ADA = 6`, runs the target-list layout query for
      list `DS:0x253B`, and snapshots only x/width from `DS:0x2AAB/0x2AAF` into
      `DS:0x254D/0x2551`. Both paths then clear `DS:0x27D8`, set
      `DS:0x252A=1`, configure scene-band state, reset selector
      `DS:0x1FAB=-1`, and request closing with `DS:0x2530=1`,
      `DS:0x2531=2`.
- [x] Port ship 3D navigation source-list helper:
      the near helper at linear file `0x00624B`, called by
      `0x04DA:0x1D4E`, fills source list `DS:0x6886` before candidate filtering.
      It walks the runtime descriptor table from `GS:[0x672C]`, processing the
      current entry and then continuing only while the next entry's `+0x12`
      word is `1`. For each entry, it reads the object record at `entry[+0x10]`,
      uses kind-dependent selector `0x11` via `0x6023`, and compares that field
      to the current `DI` target. A match appends the object record to the
      output, recurses with that record as the new target, and finally terminates
      the list with `0xFFFF`. Rust exposes this as
      `build_ship_3d_navigation_source_records()` so the later `DS:0x2B53`
      filter now has a binary-derived source list instead of an assumed one.
- [x] Port ship 3D object coordinate-field resolver:
      the helper at linear file `0x0061A6` follows selector-`0x11`
      parent/reference links, falls back to named `arche` on `0xFFFF`, and
      resolves the coordinate field used by the distance helper at `0x0060DD`.
      Direct coordinate kinds use selector `0x0B`; kind `0x0100` chooses
      selector `0x09` or `0x0A` by comparing the caller-provided word against
      selector `0x0C`. Rust exposes this as `resolve_ship_3d_position_field()`,
      giving the future software oracle and `wgpu` frontend a binary-derived
      coordinate source instead of a guessed object position.
- [x] Port ship 3D object distance helper:
      the near caller at `0x006BEA` invokes `0x0060DD` for kind-1/kind-2 record
      paths. The helper resolves both coordinate fields, with top-level
      kind-`0x0100` records comparing their selector-`0x0C` word against the
      other object's selector-`0x0E` relation word. It then computes x/y deltas
      with 16-bit wrapping signed arithmetic and calls `0x002E33` to return the
      binary integer-sqrt distance. Rust exposes this as
      `ship_3d_position_distance()` over decoded position records and
      `Ship3dPositionField` coordinate pairs.
- [x] Port ship 3D object-table bit-test helper:
      helper `0x006210`, used by the C1 resolved-table branch after building
      `DS:0x6886`, locates the target object in the 20-byte DEB/object table and
      tests a high-bit-first bit from the selector-`0x05`/kind-`0x0002` bitset
      at caller `SI + 0x1E + object_index/8`. Rust exposes this as
      `ship_3d_object_table_bit_is_set()`, preserving the bit order needed for
      the remaining C1 source-list filter.
- [x] Port ship 3D C1 source-list selection loop:
      the branch labeled `0x006C1C` scans the helper-built `DS:0x6886` source
      list until `0xFFFF`. Kind `0x0002` candidates call `0x006210` with the
      current operand object and accept only if its object-table bit is set;
      the helper's bitset base is the post-`lodsw` `SI` cursor for the current
      source-list entry. Kind `0x0001` candidates accept only when the operand
      state byte has bit `0x02`; all other kinds are skipped. Rust exposes this as
      `select_ship_3d_c1_source_record()` so the remaining C1 state integration
      can reuse binary-derived source matching.
- [x] Port ship 3D C1 kind-0x10 destination-slot write:
      the block labeled `0x006C48` hardcodes selector `0x13` with kind `0x0010`,
      adds that field (`0x1C`) to the original `DI` record, branches if the
      destination's first word is nonzero, and otherwise writes
      `{0x00C1, operand, 0x0002}`. Rust exposes this as
      `write_ship_3d_c1_kind10_destination_slot()` with a slot model that keeps
      the binary's first-word-only emptiness check explicit.
- [x] Wire ship 3D C1 kind-0x10 mode-0 path into VM execution:
      `ExecutionContext::with_ship_3d_c1_runtime(...)` now carries the recovered
      navigation records, object-table order, and raw `DS:0x6886` scratch bytes.
      `execute_trace` decodes that scratch list, applies the `0x006C1C` source
      filter, and writes the `0x006C48` selector-`0x13` destination. Tests cover
      both the accepted source write and the rejected-source no-direct-fallback
      behavior.
- [x] Port ship 3D navigation sequence branch:
      the internal branch at `0x0A9A:0x04E1` (file `0xB481`) now has a Rust
      state/effect model as `run_ship_3d_navigation_sequence_update()`. If
      `DS:0x2532` is set without opening flag `DS:0x252F`, Rust reports that
      the recovered final reset helper should run. If no exit is pending but
      `DS:0x252A` is clear, and presentation defer byte `DS:0x67B0` is also
      clear, the branch arms `DS:0x2532=1` and `DS:0x252F=1`. The active path
      runs the temporary `sn\3D.snd` setup and procedural update call, blocks
      while `DS:0x67AC` is set, otherwise copies the `DS:0x5229` framebuffer,
      sets dirty byte `DS:0x0DB8=1`, waits while the interpolation gate is
      active when duration `DS:0x0ADA == 6`, and on a nonnegative target-list
      query clears `DS:0x252A` and sets `DS:0x2532`.
- [x] Port ship 3D navigation final reset branch:
      `src/ship3d.rs` exposes file `0xB4F2..0xB586` as
      `run_ship_3d_navigation_final_reset()`. The helper preserves the binary's
      gate shape (`DS:0x2532` required, `DS:0x252F` re-enters the active branch),
      then applies the HUD/dialogue/presentation teardown, backbuffer scratch
      restore/clear effects, dirty marker `DS:0x5B52=0xFF`, and scroll reset
      `DS:0x524F=0`, `DS:0x524D=0x000A`.
- [x] Port ship 3D procedural angle/mouse update:
      file `0x9656` (`0x071E:0x1E76`) is now modeled as
      `run_ship_3d_procedural_update()`. The Rust state mirrors the recovered
      angle `DS:0x2795`, HUD/target-list flags, mouse ring `DS:0x0A2A`,
      target hold/timer words `DS:0x279B/0x279D`, direction byte `DS:0x27DB`,
      sector `DS:0x2797`, projection angle `DS:0x2F6D`, and rotation offset
      `DS:0x27A7`. This pins the exact 180/360-degree wrap constants and mouse
      recentering side effects used before the matrix/projection routines.
- [x] Port ship 3D projection matrix builder:
      file `0x98B9` now maps to `build_ship_3d_projection_matrix()`. The helper
      consumes table `DS:0x4F45` plus angle words `DS:0x2F71/0x2F6D/0x2F6F`,
      doubles the table pairs into the binary's `0x8000` fixed-point scale, and
      emits the nine dword terms written at `DS:0x2F95` with wrapping `imul` and
      arithmetic `sar 15` semantics.
- [x] Port ship 3D point-cloud projection and pixel shade:
      files `0x9A10` and `0x9B04` now map to `project_ship_3d_point()` and
      `plot_ship_3d_projected_point()`. The helpers preserve the binary's
      word-wrapping camera subtraction, signed positive-depth gate, row-based
      matrix dot products, perspective divide centers `(160,100)`, viewport clip
      words `DS:0x5235..0x523B`, occupied-pixel skip, `y * 320 + x` offset, and
      depth shade `0xEF - (depth >> 12)`.
- [x] Port ship 3D object/sprite projection prep:
      file `0x9B98` now maps its visible-descriptor gate, anchor projection,
      negative-depth wrap, depth-scale divide, source-dimension scaling, and
      mutable sprite-slot extent/position updates into
      `project_ship_3d_object_sprite()`, `update_ship_3d_sprite_slot_extent()`,
      and `update_ship_3d_sprite_slot_position()`.
- [x] Port ship 3D sprite-slot dirty geometry commit:
      the per-slot body of `0x0299:0x1467` is modeled by
      `commit_ship_3d_sprite_slot_dirty_geometry()`, including the dirty-bit
      gate, active-bit gate, and copies from current position/extent fields into
      previous-geometry fields.
- [x] Port ship 3D global clip dirty-rect snapshot:
      the alternate branch of `0x0299:0x1467` is modeled by
      `commit_ship_3d_global_clip_snapshot()`, including the `DS:0x5249` flag,
      clip words `DS:0x5235..0x523B`, dirty-rect list base `DS:0x6612`, and
      `0xFFFF` sentinel.
- [x] Port ship 3D dirty-rectangle sprite-slot render selection:
      `collect_ship_3d_dirty_sprite_slot_render_commands()` models the
      `0x0299:0x14E1` slot walk through dirty rectangles, including descending
      slot order, active-slot gate, signed exclusive-edge intersection checks,
      dispatch selector `(state >> 1) & 7`, destination-remap selector
      `(state >> 8) & 3`, flip bits, and dirty-bit clearing.
- [x] Bridge ship 3D dirty sprite commands to recovered pixel blitters:
      `blit_ship_3d_sprite_slot_command_indexed()` maps the recovered
      `Ship3dSpriteSlotRenderCommand` stream into the Rust ports of dispatch
      modes 0..4, preserves no-op modes 5..7, converts dirty rectangles to the
      renderer clip tuple, and selects the two transparent-mode destination
      remap tables using the binary high-state-byte selector.
- [x] Add event-renderer-ready ship 3D dirty sprite pipeline:
      `render_ship_3d_dirty_sprite_commands_indexed()` executes the recovered
      ordered dirty-sprite command stream against the secondary framebuffer,
      records missing/rejected frame inputs, and applies the recovered
      dirty-rectangle secondary-to-primary copyback gate.
- [x] Parse sprite slot frame tables:
      `SpriteSlotFrameTable` models the `0x0299:0x1140`/`0x11BE` resource frame
      layout used by `.SPR` payloads: flags word, frame count, packed dword
      frame offsets based at `+4`, state-flag dispatch selection, and frame
      slices that can feed the dirty-sprite render pipeline. Full extraction now
      writes `sprite-frame-tables.tsv` for real-data audit coverage.
- [x] Port ship 3D temporary `3D.snd` setup branch:
      `src/ship3d.rs` now models file `0xB591`: the `DS:0x0AE4` one-shot gate,
      phase byte `DS:0x0AE5` cycling across the three `DS:0x0ACC` callback
      offsets (`0x0087`, `0x0090`, `0x009C`), `sn\3D.snd` bank load from
      `DS:0x0D23`, presentation callback, `sn\tb.snd` restore from
      `DS:0x0CFC`, hold-timer reset, fullscreen descriptor write, and the split
      sequence-active/non-sequence redraw side effects. This converts the
      navigation sequence's old boolean "ran temp snd setup" into a reusable
      event/state model for the future oracle and `wgpu` presenter.
- [x] Port recovered framebuffer fill/copy primitives:
      `src/extract/render.rs` now has tested Rust helpers for the clipped
      rectangle fill, palette-remap rectangle, scene-band fill, full 320x200
      framebuffer copy, VGA planar-to-linear capture, and dirty-rectangle
      secondary-to-primary copyback shapes recovered from render segment
      `0x0299`; the character-HNM clear path uses the clipped rectangle helper.
- [x] Emit binary-derived SND bank-loader call sites:
      `src/bloodprg.rs` scans direct far calls to `0x0B1B:0x0855`, recovers the
      upstream `AX` bank mode plus `SI` static SND path, and test-locks the seven
      direct loader calls. This prevents the Rust decompilation work from
      treating bank selection/extraction as clip playback.
- [x] Use `tb.snd` clip 0 for subtitle chatter:
      `src/extract/subtitle_sfx.rs` now reuses the first decoded `tb.snd` clip
      for every line-complete chatter event instead of cycling through a filtered
      `7..16` subrange. This matches `verified-video-scenes.tsv` (`sn/tb.snd#0`)
      and the direct text/presentation SND call at `0x8534` (`AX=0`).
- [x] Port recovered SND bank semantics into Rust:
      `src/snd.rs` now owns the `BLOODPRG.EXE` clip-player bank layout: original
      AX clip index, offset table, 6-byte clip header skip, sample-rate byte, and
      unsigned 8-bit PCM payload. Audio export, subtitle chatter, and character
      dialogue rendering now share this recovered model instead of duplicating
      local parsers.
- [x] Port the SND average mixer primitive:
      `src/snd.rs` implements the `0xBB6D..0xBB74` `add`+`rcr` unsigned PCM
      mixer as `snd_mix_average` and verifies it exhaustively for all u8 sample
      pairs against the 8086 carry/rotate behavior.
- [x] Centralize TEXT selector voice mapping:
      `src/vm.rs` exposes `text_selector_active_line_id` and
      `text_selector_voice_clip_index`; both public and extractor script parsers
      call that recovered VM/presentation rule instead of hand-rolling the old
      `b3` tests.
- [x] Port TEXT active/already-shown line gating as an opt-in trace mode:
      `src/vm.rs` exposes `TEXT_ACTIVE_DISPLAY_FLAG`,
      `TEXT_LINE_ALREADY_SHOWN_FLAG`, and
      `ExecutionContext::with_text_line_display_gating()`. The gated mode skips
      inactive `b5` lines, skips `line_index+2` words with bit `0x8000` set, and
      marks accepted lines as shown; the default path stays ungated because raw
      `SCRIPT*.VAR` is not the initialized runtime line-record table.
- [x] Map VM resource pointer setup:
      `re/labels.csv` now names the `DS:0x6712` source-offset table, the five
      far pointers at `DS:0x671C..0x672F`, the wrapper/resolve loop at
      `0x55A4/0x55D9`, the post-exec record updater at `0x5816`, and the
      save/load state serialization sites at `0x1C3F/0x1CBD`.
- [x] Map VM resource profile selection:
      `0x53A0` now names the selector taking profile index `AX`, the
      `0x53C8/0x53DA` copy/validate loop from `FS:0x11F4 + AX*10` into
      `DS:0x6712`, and `DS:0x677E` as the cached current profile index.
- [x] Decode script profile resources and `D2` handoff:
      `FS:0x0C04` is the 16-byte resource-name table, `FS:0x11F4` maps five
      script profiles to COD/BAS/VAR/DIC/DEB resource IDs, and opcode `D2`
      writes the pending profile index to `DS:0x6780` for the main loop at
      `0x108E/0x10C5`.
- [x] Port D2 profile-request scheduling:
      `src/vm.rs` now records `ScriptProfileRequestEvent`s in execution traces
      and exposes `execute_script_profile_sequence` to follow the DOS-style
      pending-profile handoff across decoded script profiles.
- [x] Preserve per-profile runtime VAR state during D2 profile sequencing:
      profile re-entry now runs against the state produced by that profile's
      previous run, matching the persistent state-block model instead of
      reloading pristine `SCRIPT*.VAR` bytes for each handoff. This is covered by
      a synthetic profile-loop test where a second profile-0 entry emits a line
      gated by a flag set during the first profile-0 run.
- [x] Wire binary profile sequences into exporter manifests:
      `src/extract/script.rs` loads COD/VAR/DIC/DEB resources from the
      BLOODPRG.EXE profile table and emits run-level plus global-order dialogue
      TSVs for the default profile chain.
- [x] Port A6 accepted-line active-flag mutation:
      `src/vm.rs` models the handler's self-modifying `b5` update. Normal
      accepted lines clear `b5 & 0x80` for subsequent visits to the same token;
      `b4 & 0x01` preserves it for reusable/looping text. The
      `script-text-flags.tsv` summary now reports this bit as `preserve-active`.
- [x] Carry A6 skip/loop controls through the VM event stream:
      `SceneEvent::DrawSubtitle` now includes the handler-derived conditional
      skip count (`gs:0x67AB = ((b5 >> 4) & 7) + 1`) and loop target
      (`b4 & 0x10`, stored by the DOS handler at `gs:0x6778`). The
      executed-dialogue and scene-event TSVs expose these as first-class
      columns instead of requiring downstream renderer code to decode raw
      `flags_b4`.
- [x] Route profile-sequence rows into run-level videos:
      `src/extract/character.rs` now renders `ScriptProfileDialogueRun`s through
      the same VM event emitter as the existing dialogue videos, preserving
      cross-profile global sequence order.
- [x] Port the `gs:0x67BB` line-complete hold timers:
      `src/vm.rs` models `0x94D4..0x94DD` (`b35=aca*4`) and `0x7378..0x738C`
      (`b35=0x27cf*(aca/2)+6`) as checked helper functions. Labels and known
      symbols now name the set/consume sites.
- [x] Emit branch-scenario dialogue rows/runs:
      `script-branch-scenario-dialogue.tsv` reuses the same executed-dialogue
      resolver against each forced branch trace, and
      `script-branch-scenario-dialogue-runs.tsv` keeps scenario-tagged run slices
      separate from the default execution. Full export now renders those
      scenario-tagged run slices as `branch-scenario-dialogue-run - ...mp4`
      outputs through the same event renderer as default executed runs.
- [x] Define the VM-event schema (`SceneEvent`: SetBackground, PlayMusic,
      ShowSpeaker, PlayVoice, PlayTalkHnm, DrawSubtitle, PlayChatter,
      UnresolvedBackground/Actor/Voice, Clear) + `emit_scene_events()` emitter
      in `src/vm.rs`, emitting state-change events on transition only.
      Unit-tested (`emits_state_changes_on_transition_only`).
- [x] Make unresolved presentation inputs first-class scene events:
      `UnresolvedBackground`, `UnresolvedActor`, and `UnresolvedVoice` now appear
      in the `script-*-scene-events.tsv` streams at the exact source line where
      the current Rust trace lacks context. `UnresolvedVoice` only fires for
      voice-requesting selectors (`b3` not `0x00`/`0xff`), so intentional silent
      subtitle channels are not reported as missing clips.
- [x] Wire `emit_scene_events` into `character.rs`: the dialogue renderer
      (`create_character_dialogue_video`) now builds the `SceneEvent` IR and
      renders by consuming it (SetBackground/PlayMusic/PlayVoice/DrawSubtitle),
      instead of scanning grouped lines directly. The render path is now
      VM-event-driven. Dialogue subtitle sidecar audio now follows explicit
      `PlayChatter` events from that stream; HNM subtitle exports keep their
      cue-derived chatter path because they do not have VM presentation events.
      `PlayTalkHnm` and `PlayVoice` are consumed as separate pending media events
      so animation and audio routing can diverge when later binary semantics
      require it. Full export now emits `script-scene-events.tsv`,
      `script-profile-scene-events.tsv`, and
      `script-branch-scenario-scene-events.tsv` so the renderer event stream is
      inspectable without decoding generated MP4s.
- [x] Emit dialogue-run timeline sidecars for oracle alignment:
      each event-rendered dialogue MP4 now gets a matching `.timeline.tsv` file
      in `mp4/` with segment start/end, reveal-complete time, subtitle hold end,
      active line id, voice/talk-HNM presence, chatter flag, and text. These
      rows are generated from the exact `DialogueSegment` list consumed by the
      renderer, so oracle frame scans can be narrowed to binary-derived event
      boundaries instead of broad timestamp guessing.
- [x] Teach the oracle comparator to scan dialogue timelines:
      `accuracy/compare_oracle.py --generated-timeline auto` and the
      `generated_timeline` scenario column now read dialogue timeline sidecars
      and compare only event-boundary timestamps. Thresholded comparisons still
      require a fixed `generated_time`, so timeline scans remain discovery tools
      until a specific boundary is promoted.
- [x] Teach candidate search to rank at dialogue timeline boundaries:
      `accuracy/compare_oracle.py --candidate-glob ... --candidate-timeline auto`
      now loads each candidate MP4's own `.timeline.tsv` sidecar and ranks
      candidates at renderer event boundaries instead of a shared coarse time
      grid.
- [x] Removed all heuristic fallbacks from the normal full-export dialogue-video
      path (per user "no fallbacks just compute it accurately"): the default MP4
      set now comes from execution-order dialogue runs/profile runs/branch
      scenarios, not from SND-pass per-character composites. The static
      `CHAR_CONTEXTS` / `lookup_character_context` path remains only for explicit
      `--snd` direct-mode inspection. Background/music in the run-level renderer
      come from DESCRIPT-derived per-line data (actor location → location HNM →
      HNM music). The default `character-combinations.tsv` manifest leaves
      unresolved backgrounds blank instead of filling them from `CHAR_CONTEXTS`.
      Coverage from real data: ~68% location, ~58% background HNM, ~56% voice
      clip; the rest have no derivable value yet (not faked).
- [x] **Accurate voice clip selection** RESOLVED (sess 002): formula is
      `b3==0xFF|0x00 → no voice`, `b3∈1..=N → clip=b3-1`. The old `b3==0xFF →
      clip=b4` guess read the b4 flag word as an index and spuriously voiced
      513/1942 (26%) of lines; removed in `script.rs`. See dead_ends.md
      "RESOLVED — voice-clip selection". (Final AX arithmetic sits behind a SND
      callback `lcall [0xcdf]`, but the formula is confirmed by the +9 reader +
      player + export-data distribution.)
- [ ] Remaining for *full* faithfulness: replace the `(script,function)` grouping
      itself with branch-aware execution-order dialogue runs. The event IR and
      `execute_trace` are in place; the next renderer step is branch enumeration
      or scenario-selected execution so comprehensive videos do not collapse to
      only the default initial-state path.

### Oracle

The DOSBox-X oracle harness works (boots the real game on isolated Xvfb;
`BLOODPRG.EXE` runs directly into the intro cutscene — see `accuracy/`). But
without scripted input to drive it to specific scenes it is still not sufficient
for per-scene pass/fail comparison.

`accuracy/run_oracle.sh` now writes `capture-manifest.tsv` with elapsed seconds,
host epoch, display, capture kind, and native-crop metadata for every host-root
frame. It also accepts `ORACLE_INPUT_SCRIPT`/`ORACLE_INPUT_DELAY`, exporting the
isolated Xvfb display and DOSBox PID so target-scene navigation scripts can be
added without touching the user's desktop.

Current workflow: improve VM accuracy → export videos (`./target/release/
commander-blood-tools <dir>`) → compare frame candidates with
`accuracy/compare_oracle.py --scenario-file accuracy/oracle-scenarios.tsv` →
manually inspect mismatches → iterate. Blank scenario thresholds record metrics
as unchecked; scenarios can set `scan_start`/`scan_end`/`scan_step` to search a
generated MP4 window and prove whether a mismatch is timestamp alignment or the
wrong scene/presentation state. Generated dialogue MP4s now carry matching
`.timeline.tsv` sidecars, so promoted oracle scenarios should choose fixed
`generated_time` values from event boundaries instead of keeping broad scans.
`compare_oracle.py --generated-timeline auto` and scenario
`generated_timeline=auto` consume those sidecars directly. Candidate search
(`--candidate-glob`) ranks generated videos before a capture is promoted to a
checked-in scenario; `--candidate-timeline auto` ranks each generated dialogue
candidate at its own sidecar event boundaries. Promoted oracle checks must use a
fixed `generated_time`; `compare_oracle.py` now rejects `max_mean_abs` when a
scenario still has scan fields, preventing a pass/fail
result from being produced by searching for the closest generated frame. The
checked-in smoke scenario now names `accuracy/captures/capture-manifest.tsv`
explicitly so reruns use the capture-recorded path/crop metadata.
`accuracy/retrofit_capture_manifest.py` can write the same manifest shape for
older `frame_NN.png` capture directories that predate `run_oracle.sh` manifest
output. Next oracle step is scripted input via `ORACLE_INPUT_SCRIPT` or a debug
scene selector so one generated dialogue run can be compared against a matched
real-game capture with a threshold.

NAVIGATION BLOCKER CHARACTERIZED (sess 003): a 100s unattended-with-input run
(`Return`/`space`/nav-clicks every ~1.8s from t=20s) stayed on the **"CRYO
Interactive Entertainment 1995" narrated intro** the entire time — a long
talking-character cutscene (an alien narrator animating over ocean/purple/ice
backdrops, ~2 min+) that does NOT skip with generic keypresses or nav-orb clicks;
interactive gameplay was never reached. So driving DOSBox to a matched dialogue
scene needs either (a) the specific intro-skip input the game expects, (b) a much
longer wait through the full narrated intro, or (c) a debug scene-select (none
found). Generic key-mashing only loops the attract/intro. This is the concrete
blocker for verifying the 394 generated dialogue videos against real captures —
the bulk of the deliverable — and is the precise next-session target for the
oracle. INPUT LAYER recovered (sess 003): the BIOS keyboard wrappers are at
`0x2678` (`xor ax,ax; int 16h` = blocking read), `0x267D` (`ax=0x100; int 16h;
je …; read` = non-blocking peek-then-read), `0x268D` (`ax=0x200; int 16h` =
shift flags); mouse is `int 33h` @ `0xCF6+`. The intro-skip condition is in the
intro/HNM playback loop that CALLS the peek `0x267D` and tests the returned
scancode — finding that test (and thus the exact skip key, if any) is the
next-session trace for driving DOSBox to interactive gameplay. A no-input 60s run
DOES progress through the attract to gameplay-style views (ship interior, desert
at frames 26-30) but those are auto-played attract-demo frames, not interactive.
INPUT IS DELIVERED (sess 003): comparing the two runs shows input reaches DOSBox
— the with-input run *stayed on* the "CRYO 1995" narration at 100s while the
no-input run had moved on to attract-gameplay views by 60s, so the keypresses
changed behaviour. The narration is therefore **interactive** (it responds to
input by continuing/holding, not skipping), i.e. the opening is a scripted
interactive sequence, not a passive skippable cutscene. Reaching a specific
dialogue scene needs the correct *sequence of interactions* (the game's actual
UI), not a single skip key — which is why generic mashing fails. The keyboard
peek `0x0207:0x000D` has no direct far-callers (called through an input
abstraction / jump table), so the interaction handler is a further trace. This
is the real shape of the navigation blocker.

INPUT IS MOUSE-DRIVEN (sess 003): the main loop's `lcall 0:0x70E` (file `0xD0E`)
is the mouse-poll handler — `int 33h AX=3` (get position+buttons) storing into
`gs:0xA2A` = cursor X, `gs:0xA2C` = cursor Y, `gs:0xA2E` = button mask, with the
last-moved position cached at `gs:0xA38`/`gs:0xA3A` (a move clears `gs:0xB3B`).
So the game reads the mouse every frame into `gs:0xA2A..0xA2E`, and the UI/state
handlers dispatch on those (click position → nav-pyramid / dialogue-option
selection). Practical consequence for the oracle: to reach a dialogue scene, an
`ORACLE_INPUT_SCRIPT` must `xdotool` **mouse clicks at the correct UI targets**
(the opening's interactive prompts, then the pyramid-nav), not keypresses — and
the opening narration appears to consume/ignore clicks until it completes, so the
sequence + timing matters. The exact UI hit-regions are the remaining trace (read
by handlers comparing `gs:0xA2A/0xA2C` against element rects). This precisely
targets the navigation work.

CLICK EDGE-DETECTION (sess 003): the main loop's `0x1FBC` converts the raw button
mask `gs:0xA2E` into one-shot click events on the press edge: left button →
`gs:0xA3E=1`, right button → `gs:0xA3F=1` (using previous-buttons `gs:0xA30` to
detect the transition; also bumps `gs:0xA40`). So the input layer is fully
mapped: `0:0x70E` reads mouse X/Y/buttons into `gs:0xA2A/0xA2C/0xA2E`; `0x1FBC`
edge-detects into click-event flags `gs:0xA3E`/`gs:0xA3F`; the UI hit-test
handlers (`gs:0xA2A` readers @ `0x7826/0x78E6/0x7D99/0x80A0/0x8272/0x829E`)
consume the event + position. IMPLICATION for the oracle: `xdotool` mouse clicks
DO register (they set `gs:0xA3E`), so navigation IS scriptable — the remaining
piece is only the **UI element rectangles** the ~6 hit-test handlers compare
against (to click the right targets), and the opening's own handler apparently
defers click processing until the narration finishes. Tracing those ~6 handlers
(each reads `gs:0xA2A`) yields the clickable-region map, which is the final step
to drive DOSBox to a dialogue scene and verify the 394 dialogue videos.

SHIP-NAV STEERING RECOVERED (sess 003): the ship-3D-view handler at `0x7824`
does `cmp word gs:[0xA2A], 0xA0` (mouse X vs the 160 screen centre) → sets the
nav-direction state `gs:0xA32` to `2` when X > 160 (right) or `3` when X ≤ 160
(left), then runs `ship_3d_procedural_angle_update` (`0x9656`) and
`nav_actor_slot_update_loop` (`0x7D7B`). So **in the ship view, clicking left vs
right of centre steers the pyramid-nav** — directly scriptable (`xdotool` click
at native X<160 or X>160, scaled to the 800×600 window). This is the concrete
in-game navigation primitive. The remaining gate is only reaching the ship view
past the opening narration (the opening handler defers clicks until it
completes); once there, left/right clicks drive to planets/scenes and thus to
matched dialogue captures. So the navigation path is now mapped end-to-end at the
engine level: input → click events → ship-nav steering; only the opening-skip
timing and the planet→dialogue trigger remain to script the full capture run. (The narrated intro is itself deterministic character-over-background
dialogue content, so it is a candidate oracle target IF its narrator HNM +
backdrops are identified — a compositor task.)

EMPIRICAL (sess 003): an informed 165s run (start-click at t=3s, opening left
uninterrupted to t=110s, then alternating right/left steering clicks) STILL
cycled the full attract/intro demo the whole time (Mindscape → Microfolie's →
CRYO → crew showcase → credit letters → character/location parade), frame-counter
stuck at `1`, never entering interactive gameplay. So the FSM map alone does NOT
unblock navigation: the specific **attract-exit trigger** (what transitions from
the demo loop into the interactive game — a particular key/click, full-attract
completion, or an installed-save requirement) is the precise unknown. Input IS
delivered (`int 16h`/`int 33h`) but the demo loop doesn't yield to it. So the
next-session target is the attract state machine's **exit condition** (what sets
the first interactive game state / increments the day counter), NOT the in-game
nav — which is already fully mapped above. NOTE: `BLOOD.EXE` is the installer and
`BLOODPRG.EXE` is run directly (bypassing install); the attract-exit may depend on
an installed-save/config that the direct-boot lacks, which would explain the loop.

LAUNCH ARGS — ROOT CAUSE FOUND (sess 003, via the eXoDOS reference install):
the oracle looped the attract demo because it ran `BLOODPRG.EXE` with **NO
command-line arguments**. The eXoDOS install's `cblood/BLOOD.BAT`
(`/mnt/stuff/eXoDOS/eXo/eXoDOS/Commander Blood (1994).zip`) shows the real launch:

    D:
    BLOODPRG AMR S162227 EMS WRIC:\cblood\

Decoded from BLOODPRG's arg-keyword strings (file ~`0xD658`): sound-driver
selectors `S16`=SoundBlaster-16 / `MID`=MIDI / `SBP`=SB Pro / `GRV`=Gravis, and
`WRI`=write path. So `AMR`=region, `S162227`=SB16 config `2227`, `EMS`=use EMS,
`WRIC:\cblood\`=writable save path (for `blood.sav` / `game1..10.sav`). Without
these the game only demos. `accuracy/dosbox.conf` is now fixed: `memsize=64`,
`gus=true`, creates `C:\cblood\`, and runs `BLOODPRG AMR S162227 EMS WRIC:\cblood\`.
With the full eXoDOS setup (its own `Commander Blood.bin` CD, `memsize=64`) the
game RUNS CORRECTLY — the intro plays through, no 3s reset. REMAINING: getting
past the game's interactive opening (the "CRYO Interactive 1995" talking-narrator
sequence — a point-and-click dialogue) into a named dialogue scene; passive play
cycles the intro, generic input holds the narrator. Once at a scene, the mapped
nav FSM (click left/right of centre → ship steering) drives to matched captures
to verify the 394 dialogue videos. Manual: `Manuals/MS-DOS/Commander Blood
(1994).pdf`.

BOOT SEQUENCE MAPPED (sess 003, dense 0.5s capture via
`ORACLE_CAPTURE_INTERVAL=0.5`): DOSBox-X splash → **MINDSCAPE** (`sq/mind.hnm`,
~2.5s) → **Microfolie's** silvery banner (a distinct asset, ~3.5s) → **astronaut
/ red-spacesuit space cinematic** (~7s) → **CRYO** (`sq/microfol.hnm` — note the
file named "microfol" renders the CRYO developer logo, NOT Microfolie's) → title
→ gameplay. This corrects two earlier guesses: `microfol.hnm`≠Microfolie's, and
the astronaut intro is NOT `the_star.hnm` (which renders a cockpit dashboard).
A fresh independent capture re-confirms `mind.hnm` vs its MINDSCAPE frame at
`mean_abs 1.09` (bit-for-bit reproducible); `microfol.hnm` vs the CRYO frame is
`~9.9` (capture caught the logo still forming). The Microfolie's-banner and
astronaut-cinematic source HNMs are not yet pinned.

A longer 60s unattended capture shows the attract sequence continuing:
"Commander BLOOD V1.0" crew-character showcase → credit letters spelling
B-L-O-O-D → ship-interior views → a golden **sunset-over-water landscape**.
That sunset vista was scene-band-scored (`--score-region scene_band`, HUD-agnostic)
against **all 56 `pl/*.hnm` location backgrounds** and matched none (best `~44`,
a clean match is `<15`). So the boot/attract sequence is built from **dedicated
presentation assets, not the standard gameplay location HNMs** — and several
(Microfolie's banner, astronaut cinematic, CRYO card, sunset vista) are neither
statically named in BLOODPRG.EXE nor among the obvious `sq/`/`pl/` HNMs (likely
loaded via the `.xdb` overlays or runtime-built names). Practical takeaway:
attract-sequence frames are a poor oracle target beyond the Mindscape logo;
gameplay-scene verification needs scripted navigation to a known scene.

FIRST VERIFIED CUTSCENE (boot sequence, no scripted input needed): the real
game's deterministic boot plays `sq\mind.HNM` then `sq\the_star.HNM` (a fixed
0x10-byte-record path table at BLOODPRG.EXE file offset ~0x5C90; trailing slots
are runtime `sq\xxxxxxxx` placeholders). The unattended DOSBox capture therefore
contains these logos for free. Our render of `mind.hnm` matches `frame_01`
(Mindscape logo) at `mean_abs ~= 1.09` — near pixel-exact, the 4x diff is almost
entirely black. The exporter now renders this intro pair as
`output/mp4/intro - 01 - mind.mp4` / `intro - 02 - the_star.mp4`
(`INTRO_SEQUENCE` in `src/extract/mod.rs`), and `intro-mind-frame01` is the first
scenario in `oracle-scenarios.tsv` with a real pass threshold (`max_mean_abs=3.0`,
fixed `generated_time=1.0`). `the_star` vs `frame_04` only reaches `mean_abs ~= 20`
because a 58s cinematic cannot be frame-aligned to a 1-fps capture with unknown
boot phase — a timing/alignment limit, not a decode error.

Current `frame_12` evidence: searching all 43 executed-dialogue composites over
`0:12:2` ranked `executed-dialogue-run - script3 - 0011 - tumul.mp4` at `6.0s`
best (`mean_abs ~= 32.13`), and a broader all-MP4 `t=0` sweep ranked
`dialogue - script3 - ed1 - amigo.mp4` best (`mean_abs ~= 30.25`). Visual
inspection still shows different scenes, so this capture is not yet a valid
pass/fail threshold candidate for the dialogue renderer. Region metrics for that
best all-MP4 candidate show `scene_band ~= 25.82`, `hud_panel ~= 63.56`, and
`bottom_bar ~= 59.79`, which keeps the missing/incorrect HUD problem separate
from scene-band content mismatch.

PYRAMID VERTEX DATA LOCATED (sess 003): `ship_3d_hud_init` @file 0xB079 copies 192
bytes (0x30 dwords) from DS:0x5D98 = **file 0x131B8** into the HUD working area
(es:0x5491) at ship-view entry, then sets angle DS:0x2795=0xB3 and on-ship flag
DS:0x2793|=8. That source is the PYRAMID VERTEX GEOMETRY: 32 3D vertices as signed
16-bit (X,Y,Z) triples ((0,2304,3075),(776,1803,2820),(775,1546,2306),... range
0..~4615, fixed-point). So the accurate pyramid HUD = extract these 32 vertices
(DONE, file 0x131B8) -> the shared matrix x vector + perspective projection
(DONE, ship3d) -> draw edges between projected vertices. Remaining is just the
EDGE TOPOLOGY (which vertex pairs form pyramid edges) + wiring. The HUD is no
longer "deep RE" - it's bounded data + a known projection.
