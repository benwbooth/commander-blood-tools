# Port validation matrix — every module vs the original assembly/data

Standing directive: systematically validate each ported function/struct against
BLOODPRG.EXE's assembly and the game's data files. Status values:

- **ASM** — behavior derived from cited disassembly (address in the code/labels.csv).
- **DATA** — a faithful parser/interpreter of a game file format, cross-validated
  (decoded output matches known-good content, e.g. plays/renders correctly vs captures).
- **CAPTURE** — matched against DOSBox captures of the real game (screen-level truth).
- **APPROX** — reconstruction consistent with evidence but not derived from a specific
  routine; must not be presented as decoded. Listed with what would settle it.
- **UNVERIFIED** — porter invention or unchecked guess. Highest priority to fix/verify.

The matrix is maintained by hand as modules are audited; every status change needs the
evidence in the row. Re-audit pass 1: 2026-07-22..23.

| Module | What it is | Status | Evidence / gap |
|---|---|---|---|
| vm.rs `VmMachine` | script bytecode executor | **ASM** | every handler cited (dispatch 0x142D0; 0x6462/0x6830/0x65EB/0x6596/0x6588/0x6863/0x6946/0x6902/0x6B06/0x6AA7/0x64xx); flow verified vs live-oracle tutorial lines |
| vm.rs `decompile_script` | listings generator | ASM | same semantics as VmMachine; listings complete for SCRIPT1-5 |
| vm.rs walk/LineState | token scanner | ASM+DATA | descriptor table 0x6F18 transcribed; A6 layout decoded |
| vm.rs dos_save | blood.sav I/O | **ASM** | save path 0x1C3F / load 0x1CBD; block order+sizes cited; round-trip test. Tail work-buffer block written empty (rebuilt state) |
| bas_vm.rs / concept_menu.rs | conversation menus | DATA | 0xA3/0xA6 BAS blocks decoded; labels verified vs live captures (menu tree) |
| script.rs | speech-event assembly | DATA | offsets match VM Text events exactly; actor talk-ref +58 verified vs DEB names |
| descript.rs | DESCRIPT.DES records | DATA | drives intro/TV/music; verified against real-game behavior |
| hnm.rs | HNM video decoder | DATA+CAPTURE | frames match DOSBox captures (logos/montage checkpoints) |
| tbbig.rs | bridge panorama | CAPTURE | pixel test vs live game (mean_abs 2.58) |
| bridge.rs | bridge steering/stations | ASM | 0x9656 state machine decompiled; BRIDGEPROBE replays |
| font.rs GAME_FONT | proportional dialogue font | ASM | byte-identical to EXE tables 0x14C22/0x14CD2/0x14D28 (test) |
| font.rs BoldConsoleFont | subtitle/console font | ASM | tables 0x1451A/0x145CA; subtitle renderer 0x3630 uses it (decoded) |
| engine.rs subtitle draw | reveal + colors + phases | **ASM+ORACLE** | CORRECTED model (TUTORIAL4 calibration: settled=0xE0, revealing=0xFD..0xFF): while a line reveals it draws BOLD console font in the greens ('WELCOME ABOARD' mid-reveal frame); when complete it settles to THIN proportional white 0xE0 ('Today's fare:' frame). Phase-based, NOT per-speaker; rows 8/18 |
| engine.rs chatter | honk burble | ASM | 0xB898: tb.snd clip 7+rand(0..9), 4-tick throttle |
| palette.rs | baked game palette | DATA | extracted from file 0x12F78 |
| snd.rs / audio.rs | SND banks + playback | DATA | voices/clips play; clip-index mapping decoded (0x661E) |
| lbm.rs | LBM/PBM images | DATA | CHART.FD/FRIGO.FD/fd rooms decode correctly |
| ext.rs | world files | DATA(partial) | framing validated (magic/nodes/objects/payload refs); record semantics under study — needs the consumer load path |
| levels.rs | level manifest | ASM+DATA | filename table at 0xCF04 decoded |
| ship3d.rs nav projection | destination projection | ASM | 0x9B98 decompiled (matrix at 0x2F95) |
| ship3d.rs pyramid render | OPTION menu visuals | **APPROX** | flat-shaded stand-in; the real render path (manu3 3D + dither) not ported. Settle: decode the manu3 render loop or capture the real OPTION screen |
| manu3.rs | menu 3D data | DATA(partial) | camera-pan entries decoded; item sprites/RLE not |
| engine.rs console band | intro/tutorial pyramid band | CAPTURE | pixel-exact harvest from native DOSBox raws (static across times) |
| engine.rs hand cursor | pointing-hand 3D model | **DATA banked, projection APPROX** | mesh+UVs+texture ALL extracted (verts +0/+2 = UVs per the gradient-setup decode; 256x63 texture); cursor law + rest pose decoded; textured fill implemented — but the 0x270 matrix COMPOSITION (axis order/signs) isn't transcribed yet and the projection degenerates, so the LIVE-CAPTURE atlas remains the on-screen cursor until the composition matches. Progress: the 0x270 matrix build is now EXACTLY transcribed (real trig tables ds:0x26 {cos,sin} Q14, literal instruction-order composition — build_matrix in manu3_hand.rs) and the UV/texture assets are real; but the render still collapses (thin diagonal) => the +4..+8 "model coords" interpretation is wrong for the hand path (values hug the z-axis). **POSE PLAYER IMPLEMENTED** (exact 0x1DF/0x19B transcription: phased 8-byte groups, Q16
  interpolation, count-0 = sequence end, phase-mismatch = per-frame phase advance; sequence 0 =
  the null pose; test animates cells across selectors). Wiring the player to the live segment
  cells during clicks/idle = the remaining polish step alongside the span-exact raster.
  **ROW CLOSED — THE REAL 3D HAND IS LIVE**: exact projection transcribed (divide-by-depth,
  centres 252/110, y negated, row/T mapping +0x12=y/+0x1E=x/+0x2A=z) — the hand renders as a
  recognizable articulated textured hand and is now THE cursor (atlas retired to fallback).
  Residual refinements: per-frame tween poses + the span-exact rasterization order. Previously:
  SKELETAL ASSEMBLY LANDED: stride-0x5E segment records (16 segs = palm+finger joints, 110 verts + 32 aliases, live pose+translations extracted), exact per-segment matrices, alias resolution, fingertip anchoring — textured hand geometry now renders (visibly hand-material) but the projection SCALE/Z-base needs the exact 0x549 constants (units mismatch). Pinned: transcribe the projection math completely. Atlas remains on-screen until it matches | THE fix is porting manu3's real 3D render — the capture atlas (42 poses) is an interim stand-in, NOT the port target (user: "this is supposed to be an accurate port"). Verified footholds for the decompile: manu3.xdb starts with CODE (its own renderer); transform at file+0x477 = Q15 3x3 matrix at ds:0x2250 over dword vertices; trig tables at 0x17A3/0x1C94/0x2094/0xA482; TEXTURE pixel data (teal ramp d5/f5 runs) at ~0x6480+; rasterizer write-sites 0x2AF..0x13xx; shared 3D core also in amer/croolis/scrut.xdb. TEXTURING DECODED (0xC2A): column-oriented affine fill — u/v accumulators step per row, texel =
tex[v.hi*256+u.hi] (256x256, texture seg from the span record +0x54); span fields mapped
(+0x42 u/+0x44 v/+0x52 du/+0x54 dv+seg). EVERY mechanism now decoded; remaining = the Rust
transcription of the textured fill into manu3_hand (replacing flat shading). ARCHITECTURE FULLY MAPPED for transcription: API entries (0x0=hand+cursor law, 0x549=entity project), Euler matrix build (0x270), skeletal transform (0x477), 20B vertex records + alias pass, per-frame face buffer + Y-bucket sort (0x700), span S-BUFFER scanline renderer with depth-sorted spans (0x775+), data-driven tween animation system (0x181/0x19B, table 0x3E72). Remaining unknowns: the packed mesh location (via init sequences) + per-face texture params; then transcribe ~5KB wholesale |
| engine.rs intro flow | logos/montage/credits | CAPTURE+DATA | DESCRIPT present record + real-args DOSBox captures (rows 69/79 credits, band rows 99..200) |
| engine.rs TV | broadcast channels | DATA | 7 self-identified Sequence records; chained clips+music+cues |
| engine.rs telephone/cryobox | console screens | DATA+**ORACLE** | savestate probes: TELEPHONE/CRYOBOX rows open contextual gold CHOICE BOXES (the console's universal interaction; CRYOBOX = {BOB_MORLOCK, CANCEL} tutorial-verified) -> the port routes row -> box -> item -> screen (bappel call / cryo chamber) |
| engine.rs cyberspace | tunnel minigame | **APPROX** | presentation from real assets; goal decoded from SCRIPT2 text (BIOXX/BIONIUM) but the interaction logic is a stand-in. Settle: the cyber .ext consumer + input handler |
| engine.rs OPTION menu | choice box | **ORACLE** | savestate resume-probe (ring-corrected clicks — the console mouse-x is RING space, the reason earlier probes never dispatched): OPTION opens the measured gold choice box containing CANCEL; the invented 3D-pyramid OPTION screen is UNROUTED. MENU's {EXPLANATIONS, GAME} box same mechanism |
| engine.rs world visit | on-planet screens | DATA+APPROX | rooms/objects from decoded data; click=talk + room-step wiring is an interpretation. Settle: on-planet input handler in asm |
| engine.rs nav view | star chart + list | CAPTURE+**APPROX**(steer) | CHART.FD bg + tablo2 toggle 0x886C verified; the compass steer's dead-zone (8px) and rate (dx/20) are UNVERIFIED constants — the cited ~0x102/0x216 needs a proper aligned decode of the ship FSM (0xAFA0 segment) |
| save.rs | port save format | n/a (port-own) | DOS interop via vm dos_save |
| progress.rs / entity.rs | progression FSM | DATA(partial) | entity records decoded; the REAL ending trigger is SCRIPT5's Bigbang-concert block (GUARD rec_103A==Bigbang && rec_1340==concert && active_actor==Migrator.talk → lpm*sc1 reels → LOADSTR fin.hnm — now wired via the VM LoadString path); all-visited remains only as a driver fallback |
| recomp/* | interpreter runtime | oracle | separate: runs the real EXE for cross-checks |

## Findings log (evidence for open rows)
- SCRIPT1 contains ZERO script-driven presentation starts (C4 SET ops: S2=3, S3=3, S4=9, S5=2,
  S1=0) — its presentations are runtime-dispatched (console clicks), confirming the port's
  button routing. The 180-300s auto-chaining crew scenes in the no-input DOSBox run are the
  EXTENDED INTRO REEL (blood.dat-internal presentations — Bronko, Honk-in-iris, machine rooms,
  helmeted alien), not SCRIPT1 dialogue; the port's intro (mind+cliptoot) is a SHORT subset.
  Open: what enumerates the full reel — NOT the characters' pe/aa* idle HNMs (those are short
  10-13 frame dialogue talk-head idles, checked); the reel scenes exist only inside blood.dat
  (no file opens during the reel per INTROTRACE). Needs the blood.dat internal directory decode.

- blood.dat directory format CONFIRMED (16-byte path + u32 size + u32 offset + pad, 974 entries):
  60 files existed ONLY in the archive and were missing from the extracted assets — the complete
  talk-HNM sets for Rotator (g_gar*), Maziok/Fifi (omp*), Outrageor (r_pri*). Their dialogue
  scenes were silently video-less. Extracted; scenes now resolve.

- ORACLE console findings (settled where possible): the tutorial AUTO-CHAINS (hon ~52M steps,
  menus ~57M, across fresh boots with no dispatching clicks) — ported as the tutorial_chain.
  CLICKAT button injection does NOT dispatch console rows in the current harness (no file opens
  after TELEPHONE/CRYOBOX clicks; frames unchanged) — the earlier session's tut4-7 probe DID get
  dispatch (CRYOBOX -> {BOB_MORLOCK, CANCEL}); its click cadence needs recovering before the
  OPTION/TELEPHONE/CRYOBOX real screens can be captured. Port keeps the idle-dispatch gate
  (consistent with all observations).

- NAV FLOW PROBED (rp_nav4): after disengaging the menu clamp, ring-parking steers the view and
  the trail math matches the port EXACTLY (park ring 760 -> view frame 80 = target 95 minus the
  decoded 15-frame STEER_TRAIL_ARC; overshoot 880 -> frame 95). At the nav sector (purple
  pyramids + orb) the orb click opens the UNIVERSAL choice box (CANCEL at this story point;
  destinations populate when known) — validating the port's nav-sector destination box. The
  legitimate menu-disengage input (vs the diagnostic flag clear) still to find. The gray
  pyramid+orb viewscreen console (nav_screen_opened) = a further state, reachable from here.

- E2E after the choice-box refactor: smoke all-green; screen sweep healthy (note: export_screens'
  qa_option still exports the unrouted pyramid renderer — update the exporter to the choice box).
- Intro-reel name-convention hypothesis (G_* = wide shots) ruled out: G_ prefix is Rotator's talk
  set only. Reel enumeration remains with the boot-time auto-presentation driver (deep RE).

- VIEWSCREEN-CONSOLE CHAIN PORTED (this pass): pyramid-sector orb click with no destinations ->
  the viewscreen console (harvested band + static viewscreen per the oracle empty-nav state;
  destination choice box once granted); Esc -> bridge. Row closed to the evidence available.

- REEL TIMELINE MAPPED (from the 230 frames): logos ~0-25M -> ship/planet cinematic ~25-144M ->
  CRYO ~148-208M -> the STATIC VIEWSCREEN CONSOLE + tutorial voices from ~215M. The interpreter's
  no-input flow goes straight to the tutorial on the static console; the DOSBox run's extended
  crew scenes are the montage/presentations the interpreter renders differently — the two agree
  on the console-tutorial destination. PORTED: the SCRIPT1 tutorial screen now shows STATIC in
  the viewscreen between talk presentations (interpreter truth intro_215M), not black.
- REEL ENUMERATED (evidence closed): 230 frames at 1M-step intervals across the ENTIRE intro
  (INTROTRACE STEPS=230000000; archived accuracy/reel/, regenerable). The reel sequence is now
  frame-enumerated ground truth; matching scene boundaries to assets + extending the port's
  intro to the full reel = the remaining port-side work on this row.
- World-candidate labels: the box now carries the location's REAL character name (the nav
  destination label for the heading) instead of the generic TALK.

- MONTAGE RECONCILIATION (closed as analysis): three intros observed — DOSBox (full truth):
  logos -> cinematic -> CRYO -> cliptoot montage + crew presentations (blood.dat-internal reel)
  -> tutorial; INTERPRETER: same until CRYO then straight to the tutorial console (its known
  blood.dat presentation gap drops the montage — documented limitation, not game behavior);
  PORT: logos -> cinematic -> CRYO -> cliptoot + credits -> tutorial (the DESCRIPT-driven core,
  matching DOSBox through cliptoot). The DELTA: the crew-presentation reel between cliptoot and
  the tutorial — its enumeration (blood.dat-internal, no file opens, no DESCRIPT record found)
  remains THE open intro item, folded into the ext/overlay consumer-trace work.

- HIT SYSTEMS COMPLETE: both console hit paths fully decoded and confirmed — the region table
  (32x32B, ring-space {x,y,w,h}@+8, orb/zones; live-validated: presentation state = orb-only)
  and the station records ({flags, seek-arc, rect@+0xC} through mouse_hit_test 0x8269, menu/
  stations, auto-seek mechanism). The port's interaction model matches both.

## Active fix queue (from the matrix, user-reported first)
1. [x] Host crosshair removed; hand = the only cursor, all screens (this pass).
2. [x] Hand hotspot: oracle frames confirm fingertip = mouse position (arm extends down-left); the BRIDGEPROBE-derived atlas anchors encode this. Pose model (nearest-capture) remains APPROX vs the real 3D render.
3. [x] OPTION truth SETTLED via savestate resume-probe (RESUMEPROBE, ring-space mouse-x): the
   choice box with CANCEL; pyramid screen unrouted. The earlier "blocked" analysis was wrong on
   two counts (the savestate existed; the mouse-x model). TELEPHONE/CRYOBOX probed: both open
   choice boxes too (universal interaction) — ported (row -> box -> item -> screen).
4./5. [CLOSED to the decoded model] World/entity interaction is LIST-DRIVEN end to end (decoded
   chain: candidate list 0x7259 [flags-filtered entities -> [0x250B]] -> choice box -> commit
   0xB0F3 [[0x251B], FSM state 3] -> C1 presentation swap 0x5B75 -> script blocks; NO free-roam
   hit-test exists). PORTED: on-planet entity click opens the candidate box; choosing engages the
   dialogue — the same universal-box model as the console. Residual: per-world candidate labels
   (entity names) when multiple entities populate.
   [was: CONVERGED] Cyberspace + on-planet interaction are the SAME system: the cyber worlds are
   standard .ext worlds (initial entity id=1 kind=4 at a screen position, like every planet;
   fd/1cyber1*.lbm are their rooms). Both rows resolve with ONE trace: the EXE's world/entity
   runtime (entity_object_populate 0x40D0 + the entity click dispatch through entity_draw
   0x9240's hit path). Single documented target for the next deep session.
6. [ ] ext.rs record semantics via the consumer load path. NEGATIVE RESULT banked: walk-group
   counts do NOT correlate with room counts (VENUSIA 109 groups/3 rooms) — the payload runs are
   not per-room strips; per-node outlines or paths remain the candidates. Consumer trace stands
   as the only path.
6b. Entity stepper: watch infra CONFIRMED (Machine.watch_addr linear write-watch exists); the
   watch needs a game state with a MOVING entity — gated on story progression past the current
   savestates (a full-playthrough savestate at a location visit unlocks it). Plan complete.
7. [x] Nav compass steer REMOVED (the chart view is static in the real game — CHART.FD fixed
   image + target-list selection; the mouse-steered compass with dead-zone 8/rate dx/20 was an
   invention). compass_angle survives only as the explicit key-cycled world-target selector.
8. [ ] A8 LOADSTR scene reels: wired (SCRIPT5 finale films); verify other scripts' LOADSTR uses (explo3.hnm on SCRIPT2's third warning etc.) play at the right beats.
9. [ ] DOSBox interactive capture: injected clicks don't reach the game (window focus / SDL mouse capture) — fix with xdotool windowactivate + click-in-window before injection; needed for the real OPTION screen + hand hotspot.
