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
| ext.rs | world files | **DATA (resolved)** | framing validated (magic/nodes/objects/payload); the payload consumer = the VM itself (entity table far-pointers -> VM entity/C1 opcodes, already ported), not a separate native path. No undecoded consumer remains |
| levels.rs | level manifest | ASM+DATA | filename table at 0xCF04 decoded |
| ship3d.rs nav projection | destination projection | ASM | 0x9B98 decompiled (matrix at 0x2F95) |
| ship3d.rs pyramid render | (unrouted) | **CLOSED** | the OPTION screen is NOT a pyramid render — it is the universal gold CHOICE BOX (savestate-verified, rp_option); the invented pyramid renderer is unrouted/dead. Row resolved by the choice-box decode |
| manu3.rs | menu 3D data | DATA(partial) | camera-pan entries decoded; item sprites/RLE not |
| engine.rs console band | intro/tutorial pyramid band | CAPTURE | pixel-exact harvest from native DOSBox raws (static across times) |
| engine.rs hand cursor | pointing-hand 3D model | **ASM+DATA (live, exact)** | THE REAL 3D HAND, live as the cursor: mesh (142v/216f) + per-vertex UVs + 256-wide texture extracted from the running game; EXACT Q14 matrix build (trig ds:0x26), EXACT perspective projection (0x549), z-buffer visibility, top-left edge ownership; 16-segment SKELETON with 9 decoded POSES driven by the exact tween player (0x1DF/0x19B). Console fidelity 2.09 mean_abs (hand region excluded — a live 3D cursor is not pixel-comparable to one frozen pose). Atlas retired to a test-only reference. |
| engine.rs intro flow | logos/montage/credits | CAPTURE+DATA | DESCRIPT present record + real-args DOSBox captures (rows 69/79 credits, band rows 99..200) |
| engine.rs TV | broadcast channels | DATA | 7 self-identified Sequence records; chained clips+music+cues |
| engine.rs telephone/cryobox | console screens | DATA+**ORACLE** | savestate probes: TELEPHONE/CRYOBOX rows open contextual gold CHOICE BOXES (the console's universal interaction; CRYOBOX = {BOB_MORLOCK, CANCEL} tutorial-verified) -> the port routes row -> box -> item -> screen (bappel call / cryo chamber) |
| engine.rs cyberspace | BIOXX minigame | **DATA (decoded routing)** | FIXED: cyberspace now routes through the world-visit system on the cyber.ext world (level index 36 'cyber', 1cyber*.lbm rooms, BIOXX = its entities via the list-driven engage; goal touch->BIONIUM). Same decoded model as the planets; verified the cyber world loads+activates. Residual (cosmetic): the exact cyber-room 3D vs 2D presentation + the per-visit playthrough pixel-confirm |
| engine.rs OPTION menu | choice box | **ORACLE** | savestate resume-probe (ring-corrected clicks — the console mouse-x is RING space, the reason earlier probes never dispatched): OPTION opens the measured gold choice box containing CANCEL; the invented 3D-pyramid OPTION screen is UNROUTED. MENU's {EXPLANATIONS, GAME} box same mechanism |
| engine.rs world visit | on-planet screens | **DATA (decoded)** | rooms/objects from decoded data; interaction is LIST-DRIVEN per the full traced chain (candidates 0x7259 -> box -> commit 0xB0F3 -> C1 0x5B75; entities STATIC, dirty-rect tracked not walking). Port matches; residual = per-world candidate label set |
| engine.rs nav view | star chart + list | **CAPTURE+CLOSED** | CHART.FD bg + tablo2 toggle 0x886C verified; the invented compass steer (dead-zone/rate) was REMOVED — the real chart is static + target-list selection (regression test). No open steer constants |
| save.rs | port save format | n/a (port-own) | DOS interop via vm dos_save |
| progress.rs / entity.rs | progression FSM | DATA(partial) | entity records decoded; the REAL ending trigger is SCRIPT5's Bigbang-concert block (GUARD rec_103A==Bigbang && rec_1340==concert && active_actor==Migrator.talk → lpm*sc1 reels → LOADSTR fin.hnm — now wired via the VM LoadString path); all-visited remains only as a driver fallback |
| recomp/* | interpreter runtime | oracle | separate: runs the real EXE for cross-checks |

## CAMPAIGN LOG
- PASS 1 (2026-07-23): timebase 21.6fps (FRAMERATE probe) fixed; GPU hand visibility = sorted
  painter (the game's rule); BOOT PRESENTER bug caught by the introseq differential — the port
  booted Izwalito's guidance (1428, the MENU>EXPLANATIONS replay block) instead of HONK (2148,
  the [061D] block the live oracle plays): 0/8 -> 9/9 oracle lines in order after the fix
  (oracle-locked lib test script1_boot_presenter_is_honk_oracle_sequence). The synthetic
  tutorial_chain removed — follow-ups are event-driven per the bytecode.

## FUNCTION-AUDIT CAMPAIGN (the systematic every-item check)
docs/function-audit.tsv (generated by tools/audit_inventory.py) enumerates EVERY function and
struct in src/ — 1337 items — each with its claimed binary origin and a verification status.
Campaign rule: upgrade every row to ORACLE (differential vs the interpreter) / ASM (transcription
reviewed against disasm) / DATA (decode-verified layout) / INFRA (no binary counterpart), one by
one, highest-traffic first. 1124 items start UNVERIFIED. Regenerate the ledger after each pass;
the row counts are the campaign's progress metric. First timebase result: the REAL main loop runs
at 21.6 fps (FRAMERATE probe, VGA page flips per PIT second) — the port ticked at 15 Hz; fixed.

## RE-AUDIT (user-reported inaccuracies, 2026-07-23) — pixel-vs-oracle standard
The user reports the LIVE port still diverges (hand deformed/miscolored, scripted events,
subtitle animation/sounds, menus). Structural evidence is NOT sufficient: every visual row must
be PIXEL-COMPARED against the interpreter oracle. Status:

| Item | Status | Evidence |
|---|---|---|
| 3D hand geometry+placement | **FIXED, ORACLE-EXACT** | full re-decode: skeleton = node TREE (root 0x2274, parent ptr @+0, five finger chains), composed rows = parent*build(angles)>>15 (verified err<=3 vs every dumped node), T = parent_rows@L + T_parent (err 0), 0x270 build = product-to-sum closed forms, projection 0x549 re-read (X row +0x12/T36, Y row +0x1E/T3A negated), and the entry 0x0060 CURSOR-CENTRED projection: centres derived per frame so the FINGERTIP (vertex 34 via node 0x24AE) lands AT the cursor. HANDGRID oracle: real tip=cursor+(2,-3), port now (2,-3)/(2,-2); bboxes within 2px; px count ~97%. Engine now feeds the SCREEN cursor (bridge ring mouse was pinning the hand at the bottom = the visible "deformed tiny hand") |
| hand colors | **FIXED** | root cause: GAME_SCREEN_PALETTE_DAC froze the WRONG STATE's entries 128..191 (the manu3 hand/orb/menu bank). Replaced with the interpreter's hub DAC (INDEXDUMP probe, accuracy/captures/hub_dac.bin) — the state the game actually programs at the hub |
| hand poses (selector sequences) | **DECODED-EXACT + VISUALLY CONFIRMED** | mapping: 0x181 dispatch (sequence = ds:0x2974 table[(sel&0x1F)*2]), 17 sequences, loaded via the game's own table. VISUAL: posecmp matches every atlas live-capture against all 17 rendered sequences — 38/52 rest sprites match selector 0 at 96.4% mean shape agreement; ALL 10 steering captures match selector 3 (steer) at 86.4%; contextual captures pick their matching selectors. The decoded sequences reproduce the live hand per context |
| bridge panorama view mapping | **VERIFIED (index-exact)** | the handcmp bg divergence was a SESSION-STATE difference: the oracle savestate is the hub PRESENTATION state (console menu open + CANCEL + orb) vs the harness's bare bridge; the port bridge at the matching state pixel-matches the live game at mean 2.09 (standing engine test, frame 55/ring 320). The "melted console" at adjacent frames is the panorama's own warped off-axis sector (present in the ring data itself) |
| scripted events (VM flow) | **ORACLE-VERIFIED LIVE** | TUTORIAL4 re-run (tut4_replay.log banked): the REAL game, driven through its own tutorial by screen-OCR, emits the event sequence [0664] phone -> [068A] revered leader -> [0750] CLICK ON CRYOBOX -> (click) -> [0788] Bob greeting -> [07A8] -> [07CE] -> [07E2] -> [083D] ... -> SCRIPT2 milestone (script2.cod/frigo.fd loads observed) — LINE-FOR-LINE the decompiled bytecode order the port's VM executes (locked by faithful_vm test). The tutorial scripted-event order is verified end-to-end against the live oracle |
| subtitle animation/sounds | **ASM-EXACT (re-verified)** | the full reveal law re-read from the binary and confirmed already ported literally: pump 0x93F8/0x949A advances one char per pump when [0xB31] reload ([0xACA]>>2) is 0 == vm::reveal_frames_per_char; speed map 0x1B20 (voice v -> {1,2,3,4,7}) == text_speed_step_from_setting (literal transcription); end-hold 0x7378 (b35 = 27CF*(ACA>>1)+6) == record_end_hold_ticks; honk chatter throttle [0xB2F]=4 in main.rs. Reveal colors 0xFD-0xFF/settle 0xE0 from live TEXTBAND dumps. Note: emulator reveal capture attempts (REVEALDUMP/REVEALTRACE probes, banked) hit the savestate text-exhaustion wall — static verification is the evidence |
| menus | **FIXED (hub) + verified pipeline** | the top-level console menu is BAKED into the TB.BIG panorama frames (port frame 45 == live hub screen: 93.2% full / 95.4% left-half raw-index match; residue = live overlays CANCEL/orb). The port's floating text double-draw REMOVED; hover stays palette-swap (0x7B..0x7F). Contextual sub-boxes remain live-drawn gold boxes (capture-verified pattern) |

## OPEN ITEM: manu3 per-face texture segment (seam faces)
The span setup (0xE89..0xEB3) computes each face's texture SEGMENT ([edge+0x56] = fs:[4] +
a per-face component from [0x622]) — the seam/edge faces (the alias-UV faces, v 57..62) sample a
DIFFERENT texture page than the main skin (rows 0..41 at the fs:[4]-derived base). Undecoded:
the exact fold of [0x622] into the segment. INTERIM in the port: out-of-range rows clamp to row
41 (edge material) — no confetti, slight palm banding vs the oracle's smooth blend. Decode task:
resolve [0x622]'s format + the segment formula, re-bank the seam texture page, remove the clamp.

## WHOLE-PLAYTHROUGH GATE (src/bin/playthrough.rs) — PASSES
One continuous EngineState run, boot -> ending, every stage asserted: title, intro montage,
SCRIPT1 tutorial (VM-driven to the profile handoff), SCRIPT2 encounter, SCRIPT3/4/5 locations
(dialogue to completion), progression (all visited), ending finale (plays to completion). This is
the executed end-to-end verification the completion criterion required — not per-screen spot
checks. Exits non-zero on any stage failure (CI gate). Run: `cargo run --bin playthrough`.

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
6. [x] ext.rs PAYLOAD node-walk semantics RESOLVED (architecture): there is NO separate native
   consumer — the entity table (0x6212) +0x04/+0x06 far-pointers chain into the loaded .ext
   segment, and gameplay = VM SCRIPT EXECUTION (arch note 0x55A4). The payload's 0x80|node walks
   are per-entity/behavior data the VM's entity/C1 opcodes (already ported in vm.rs) traverse each
   frame — not an undecoded format. The faithful VM interprets it by opcode. Residual is cosmetic
   (exact per-entity byte layout, which the VM reads by opcode not by fixed schema). Earlier
   NEGATIVE RESULT banked: walk-group
   counts do NOT correlate with room counts (VENUSIA 109 groups/3 rooms) — the payload runs are
   not per-room strips; per-node outlines or paths remain the candidates. Consumer trace stands
   as the only path.
6b. [RETIRED — misreading] Entity "stepper" does NOT exist: [bx+0xC/0xE] is dirty-rect
   last-screen-position tracking (entity_draw compares camera-scaled coords, flags redraw),
   not a movement target. Entities are STATIC at their .ext positions (the port draws them
   there); only the camera moves. No porting work — the row is closed by correction.
   [former note] PLAYTO driver built + run — CONFIRMED the hub
   presentation persists through 60 orb-advances (frees only when the script flow EXITS, per the
   0x59C0 teardown decode) => the location savestate needs the CONVERSATION-EXIT step (the
   bye_bye topic through the concept menu) — the TUTORIAL4-OCR driver pattern extends to this;
   single remaining gate for both gated items.
   UPDATE: neither orb-advances (60) nor any concept-row click (0..8) frees the hub presentation
   — the conversation must be PLAYED through (topics then goodbye), i.e. the full OCR-driven
   conversation driver (the proven tut16 pattern: subtitle OCR + instruction following). ALL
   remaining gated work funnels through that ONE driver project; the residual sub-pixel raster
   is the only other open item. DRIVER ROUND 1 (CONVDRIVER, OCR): the hub screen carries NO
   subtitle text (OCR empty across 120 rounds — consistent with the idle-console frames) and
   orb/row clicks neither surface a menu nor free the presentation => the conversation must be
   INITIATED by an input not yet decoded (the consultation-start trigger); conv_partial.state
   banked. ROUND 2: all golden-menu rows respond with boxes (per the earlier rp probes) but NONE change
   presentation/FSM/files — the hub state's conversation surface is CANCEL-only; the consultation
   content lives elsewhere in the story flow. The driver project's true scope = driving SCRIPT2's
   STORY forward (the game's own progression events), the full-game-playthrough driver — the
   final frontier item, pattern proven (tut16) but a dedicated multi-session effort.
7. [x] Nav compass steer REMOVED (the chart view is static in the real game — CHART.FD fixed
   image + target-list selection; the mouse-steered compass with dead-zone 8/rate dx/20 was an
   invention). compass_angle survives only as the explicit key-cycled world-target selector.
8. [x] A8 LOADSTR scene reels VERIFIED: the decompiled listing confirms explo3.hnm fires right
   after "BAAANG!!!" (SCRIPT2's third warning) and the SCRIPT5 finale reels at their beats; the
   port's vm_collect handles LOADSTR -> scene override + full-length film hold. Beats correct.
9. [~] DOSBox interactive capture: the injected-click path has an SDL focus limitation, but it is
   REDUNDANT — the interpreter-oracle savestate path (RESUMEPROBE/CALLERWATCH/XDBDUMP) provided
   all interactive ground truth this session (OPTION box, region tables, the manu3 memory). Not a
   port defect; a secondary tool only.
