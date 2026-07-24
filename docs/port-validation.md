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
| vm.rs dos_save | DOS save I/O | **ASM + LIVE ROUND-TRIP** | save path 0x1C3F / load 0x1CBD; block order+sizes cited. LIVE (save_option scenario): the REAL game, driven through OPTION->SAVE typing 'ab' + Enter, WROTE game1.sav (5887 B, profile=1 at the post-tutorial hub) + blood.sav (= the 10x32 slot-name DIRECTORY, slot 1 named 'ab'); both banked (accuracy/cdrive/cblood) and parsed by bloodsav.rs. Full slot-UI decode in re/REVERSE.md (edit state [0x2734]/[0x273B]/[0xB15], lowercase+digit filter, Enter commit 0x1DD8 -> int21 3C00 with slot filename) |
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
| engine.rs telephone/cryobox | console screens | DATA+**ORACLE** | savestate probes: TELEPHONE/CRYOBOX rows open contextual gold CHOICE BOXES (the console's universal interaction; CRYOBOX = {BOB_MORLOCK, CANCEL} tutorial-verified) -> the port routes row -> box -> item -> screen (bappel call) |
| engine.rs BOB_MORLOCK contact | CRYOBOX -> BOB screen | **ORACLE-CORRECTED (dual-run)** | cryobox_enter scenario (vs_003..007): choosing BOB_MORLOCK opens Bob's CONVERSATION screen — his talk-head video (pe/aabob.hnm, the red-face eye close-up; frigo.fd file-open traced) + console-position subtitles + his concept menu {BYE_BYE, BLACK_HOLE, BIG_BANG, BOB_MORLOCK, KANARY, MISSION, CORPO, GOOD_OL_BOB} at x=170 y=56 pitch 11 — NOT the cryo-chamber video the port had. Ported (render_bob_contact + bob_topic_click; BYE_BYE returns to the bridge). Residuals: the dark-teal border band's palette source; topic-click -> SCRIPT2 conversation-beat wiring; the engaged CRYOBOX row re-labels red CONTACT (vs_003) |
| engine.rs cyberspace | BIOXX minigame | **DATA (decoded routing)** | FIXED: cyberspace now routes through the world-visit system on the cyber.ext world (level index 36 'cyber', 1cyber*.lbm rooms, BIOXX = its entities via the list-driven engage; goal touch->BIONIUM). Same decoded model as the planets; verified the cyber world loads+activates. Residual (cosmetic): the exact cyber-room 3D vs 2D presentation + the per-visit playthrough pixel-confirm |
| engine.rs OPTION menu | choice box | **ORACLE** | savestate resume-probe (ring-corrected clicks — the console mouse-x is RING space, the reason earlier probes never dispatched): OPTION opens the measured gold choice box containing CANCEL; the invented 3D-pyramid OPTION screen is UNROUTED. MENU's {EXPLANATIONS, GAME} box same mechanism |
| engine.rs world visit | on-planet screens | **DATA (decoded)** | rooms/objects from decoded data; interaction is LIST-DRIVEN per the full traced chain (candidates 0x7259 -> box -> commit 0xB0F3 -> C1 0x5B75; entities STATIC, dirty-rect tracked not walking). Port matches; candidate labels = the script's distinct DEB-resolved actor names (the decoded 0x7259 entity list is the location's characters), host-label fallback — WIRED |
| engine.rs nav view | star chart + list | **CAPTURE+CLOSED** | CHART.FD bg + tablo2 toggle 0x886C verified; the invented compass steer (dead-zone/rate) was REMOVED — the real chart is static + target-list selection (regression test). No open steer constants |
| save.rs | port save format | n/a (port-own) | DOS interop via vm dos_save |
| progress.rs / entity.rs | progression FSM | DATA(partial) | entity records decoded; the REAL ending trigger is SCRIPT5's Bigbang-concert block (GUARD rec_103A==Bigbang && rec_1340==concert && active_actor==Migrator.talk → lpm*sc1 reels → LOADSTR fin.hnm — now wired via the VM LoadString path); all-visited remains only as a driver fallback |
| recomp/* | interpreter runtime | oracle | separate: runs the real EXE for cross-checks |

## DUAL-RUN DIFFERENTIAL HARNESS (the verification capability)
The port and the REAL game execute the SAME interaction scenario side-by-side:
- oracle side: runtime_boot VERIFYSCRIPT=<scenario.tsv> (resume hub, per-line actions
  move/click/key/wait with ring-corrected coords, settled frame per step -> boot_frames/vs_*)
- port side: verify_port <scenario.tsv> (same actions vs EngineState -> boot_frames/vp_*)
- scoring: tools/verify_compare.py -> accuracy/comparisons/verify/{scorecard.tsv, sheet.png}
Scenarios are TSVs under accuracy/scenarios/ — every new screen/interaction gets one; every
divergence is a scored, visually-reproducible work item. FIRST RESULT (hub_tour): initial
28.03 mean / 43.6% close exposed (a) the port harness steering while the oracle hub is
script-locked and (b) the missing live CANCEL overlay; after fixes: 2.22 mean / 95.6% close
across all 9 steps.

## BITCODE ROUND TRIP (user directive, 2026-07-23): decode -> re-encode -> byte-compare
vm::encode_token is the inverse of walk's decoding, from structured fields alone; the
token_model_round_trips_every_script test walks all five SCRIPT*.CODs, re-encodes every
structured token, and byte-compares against the original stream (contiguity asserted).
RESULT: **100% BYTE-EXACT** — all 10,349 tokens across the five scripts decode to the
structured IR and re-encode byte-identical (S1 214/214, S2 3271/3271, S3 3281/3281,
S4 1714/1714, S5 1869/1869; contiguous coverage asserted). The Op IR carries simple-op
operand bytes losslessly (standard compiler-IR design); their ASM semantics live in
VmMachine's handlers. The test asserts exact==total permanently — any future
mis-length or mis-parse of any script byte fails CI.

## CAPTURE-DERIVED DEFECTS — CORRECTIVE RE QUEUE (user directive, 2026-07-23)
Per CLAUDE.md's PRIME RULE, several recent conversation-wiring commits sourced their
constants from ORACLE CAPTURES rather than the assembly. These are APPROX until
re-derived from the code that produces them. Each row names the RE task.

| Capture-derived constant | Where | RE task (find in the binary) |
|---|---|---|
| in-window concept box geometry (x=175, y=39/83 split, pitch 11) | engine.rs render_bridge kind-3 | PARTIALLY RESOLVED: the unified list widget (0x8428) is the vertical-list source — pitch 11 (add bp,0xB @0x847A), row hit dy/11+1 (@0x8508), box w=max+20 / y=(200-h)/2 / top+4 now drive the CHOICE BOX draw (ported). RESOLVED: the anchors are code constants — hub 100 (@0x86D9), in-window 225 (@0x89A6, deriving x~175 and the y=39/83 split via the same law), world-candidate list 80 (@0xB0D1, inside ship_click_commit — ported for kind 10). The kind-3 draw + hit-test now compute from the widget law |
| Bob concept menu geometry (x=170, y=56, pitch 11) | engine.rs render_bob_contact | same render routine (the contact screen uses the same widget) |
| BOB_TOPICS label list | engine.rs BOB_TOPICS const | RESOLVED: Bob's topics now come from his prompt line's 0xFFFF-carried menu words (vm_collect out.2 -> engine.bob_topics; render + hit-test use the live list); the captured list remains only as the no-VM fallback |
| console-row -> actor-record map (HONK 2220 / BOB 132) | main.rs row dispatch | the click-dispatch code: which record each console row's hit-test starts (station-record / 0x5816 dispatch) — verify the operands are read from the decoded tables, not assumed |
| completion-hold bright-green timing | engine.rs draw_subtitle_revealed | ASM-CONFIRMED: the hold timers ([0xB31]=[0xACA]>>2 per char, [0xB35]=[0xACA]<<2 end-hold, [0x67BB] flag) read directly at 0x9480..0x94E0; the menu words then reveal word-at-a-time (0x7358 +2 stepping) |
| CONTACT re-label / red engaged row | engine.rs, bridge.rs | RESOLVED AS A CAPTURE MISREAD: no CONTACT label exists in any game file, and 0x8613's engaged path is a pure DAC swap of the baked label — the red capture text was CRYOBOX in red. The re-label overdraw is REMOVED; the ASM DAC model stands |

NOTE: capture-measured constants may stand in TEMPORARILY only while their row here
is APPROX and names the routine to decode. They are not evidence of correctness.

## ARCHITECTURE CORRECTION (user directive, 2026-07-23): NO hardcoded bytecode surfaces
The conversation wiring briefly drifted into transcribing oracle-captured menu labels and
trees into main.rs. CORRECTED: the menus are IN the bytecode — each 0xA6 line record
carries its concept menu after a 0xFFFF separator (the decompiled `SAY "... word_65535
talk remember bye_bye"`). script.rs now splits the marker into (display text, menu_labels);
vm_collect reports the emitted lines' carried menu; the kind-3 box + the HONK opener render
WHATEVER the VM emits — the trees, labels, and follow-up presentations all come from
executing the script (poked presentations start on the next frame). The oracle scenarios
are VERIFICATION ONLY. Remaining literals: the console row -> actor record map (2220/132,
the decoded click dispatch itself) and no-VM fallback labels. The Bob screen's topic
render still reads BOB_TOPICS — converting it to the line-carried menu is the follow-up.

## STORY-PROGRESSION MAP (bytecode-extracted, 2026-07-23) — the frontier's exact chain
From decompiled/SCRIPT2.bas + COD operand reads (assembly-first):
- **Scruter_Jo.talk = record 1860 rel 40** (C4 @0005) — his presentation explains
  CYBERSPACE ('you go get BIONIUM in CYBERSPACE of SCRUTER JO', @038C) and the
  BIOXX->Mantas->BIONIUM loop (@04B5..04C7).
- **vbio** = the BIONIUM counter variable: guards vbio==0/1/2 branch Bob's cryobox
  begging (@0BD3/0C40/0CA7); vbio>0 acknowledges 'You did get BIONIUM' (@0570/058E).
  Cyberspace play increments vbio — THE story gate.
- **rec_0722 == 65535** gates Bob's no-BIONIUM begging block (@0BCA).
- Driver chain for the outer ring, in order: start 1860 (Scruter Jo — EXECUTED, the
  script2_scruter_jo test locks his cyberspace block) -> his world binds via
  **SETCHAR slot 4 = "scrut"** (@004E, the 0xCC opcode — the entry citation) ->
  enter the cyber world (port: visit_world through the SETCHAR binding; tested by
  cyberspace_traversal + the playthrough gate) -> BIOXX touches raise vbio (WIRED:
  add_record(0x126C,1) on cyber arrival) -> Bob's vbio==0/1/2 blocks unlock ->
  nav destinations activate (entity flags 0x15..0x1F — the FSM decoded+ported in
  progress.rs/entity.rs; ORACLE-side verification chains behind driving the oracle
  through cyberspace) -> planets = concepts 3/4/5 -> D2 profiles 2/3/4 (STRUCTURE-
  LOCKED test @1269/@1284/@129F; the gate scr=rec 0x1276 is READ-ONLY in SCRIPT2's
  bytecode — its writer is runtime code or cross-script state, next trace: the
  Scruter-examination counter hypothesis + the C4-kind runtime paths) -> SCRIPT3/4/5 in-world dual-runs (port-side reference set BANKED:
  accuracy/comparisons/planets/*.ppm via examples/planetbank — FIXED: the cyan cast was the hand-draw's
  over-wide palette install (128..=255) clobbering the rooms' own 128..201 range —
  the hand texture occupies ONLY 202..=251 (verified over the whole seg4 texture);
  narrowed at both draw sites, worlds re-banked with correct palettes. Remaining
  question for the oracle pass: the green location header) ->
  the Bigbang-concert ending (fin.hnm trigger wired; dual-run pending).
Each arrow is one dual-run scenario + any needed decode of its dispatch site.
UPDATE (walk-fix era): the FULL bytecode is now decompiled (vm::walk covers every
stream byte; SCRIPT2 3636 lines) and the INTERCEPTION CHAIN IS PORTED, bytecode-
locked (script2_interception_arms_counts_down_and_queues): the shipped-enabled
@272F one-shot arms state[3]=10 -> the 0x8AA beat countdown (200Hz/25, idle-gated)
expires it -> @2744's OP_C3 queues the typed {0xC3,40,1} request at 0x6FC -> idle
promotion starts the presentation (C4, active actor). The frontend beats the
countdown and promotes queued requests. THE FULL SCRUT ENCOUNTER ARC now plays in the port (f387ad2: resume model +
departure test — arrival, repeat warnings, FINAL WARNING, departure radio, all
from shipped bytes; two escape routes decoded: stay/reprieve vs flee via the
rec_0F4E location write -> the Corpo unlock). THE INTERCEPTION NOW PLAYS through the
port's frame loop (script2_interception_plays_through_the_frame_loop: SCRIPT2
from load, frames + beats + serial queue promotion, SCRUT agent K's radio
warning emits @2DF5 after the TV-commercial presentation drains) — with the
0xAB POKE corrected to the COD self-modify the engine performs (0x684C).
**THE STORY PLAYS TO FIN.HNM IN THE PORT (b499bde)**: the single directed
test spans all five scripts, four profile handoffs, and the Bigbang-concert
ending — every beat from shipped bytes, hard-asserted. SCORING-PASS CALIBRATION ROOT CAUSE (recorded): the oracle resumes a frozen
savestate (script2.state) while the port loads fresh SCRIPT2 + exported
records — different start states. The fix is a shared fresh start (boot the
oracle to the hub, or reconstruct the port from the full machine state, not
just records); the dumprecords bridge is records-only. This is a lane-harness
calibration, not a VM-fidelity gap — the VM plays the beats identically once
started identically (the interception/wake/departure tests prove it).
VmDrive ADOPTION DISPOSITION: main.rs's script_vm wiring is what VmDrive was
EXTRACTED from — the policies are identical by construction (vm_collect = the
frame loop; the concept-click path = dispatch_concept; the idle promotion =
frame_idle). Full adoption (24 borrow sites) is DEDUPLICATION HYGIENE with no
fidelity delta; deferred as such, not as a parity gap.
THE MATCHED-DRIVE LANE'S SPEC (assessed): verify_port is the pre-VM-era
screen harness (static frame-45 hub, HARDCODED box literals — a no-transcription
violation to retire); the lane needs it rebuilt around the real VM loop
(main.rs's script_vm wiring: load_script, vm_collect, beats, promotions,
dispatch_concept) so the oracle's scenario files drive BOTH implementations'
full stacks — then BloodPrng seed-matching makes rolls agree and
tools/verify_compare.py scores line-and-screen parity per step. Remaining for FULL
parity per this ledger: the oracle-side dual-runs of these beats (the
interception drive exists; the rest follow the same scenario recipes), the
container-graph refinement behind OP_CD, the frontend presentation surfaces
for Acts 3-5 (the VM plays them; the screens render via the existing
presentation systems), and the per-act placement writes' replacement by their
own driven beats (each is cited to its stream operands today).
**THE SCRIPT2->SCRIPT3 HANDOFF RUNS IN THE PORT (66a45a8)**: the directed
drive plays stages 5/6, the gift, the verified cargo manifest, the customs
boarding and confiscations, and RUN PROFILE (pending profile 2 = SCRIPT3) —
end to end from shipped bytes, hard-asserted. The unlocking fix: the AE/B0
mask-guard polarity was inverted (every satisfied mask guard was skipped).
OP_CD (transfer/teleport) implemented as typed-record
query + marker + event (687fc00); the container-graph relink (field 0x11 +
special-slot list) is APPROX — the full inventory model is the replacing decode.
The customs SCRIPT2->SCRIPT3 handoff is a late-game loop-back (needs SCRIPT4's
rec_0332); the FIRST planet entry remains the scan gate (scr>5 via Honk's
script-select). NEXT: the scan drive (wake Scruter_Jo -> quiz -> examinations
writing the exam table) in both implementations; the interception dual-run.

## CAMPAIGN LOG
- PASS 7 (story_deep, 27 steps — the longest chain): deep-topic answers play from the
  bytecode (ORXX: 'living guided missiles...') with the persistent-menu + highlight
  laws holding throughout; AND an interaction law confirmed — an OPEN conversation
  holds input focus (console-row clicks mid-conversation stay in the conversation),
  matching the port's box-takes-clicks-first dispatch. All content VM-sourced.
- PASS 6 (story_cycle): a full conversation cycle live — psychotherapy in, topic
  acknowledged ('YOU GOT IT...'), and the menu STACK POPS back to the consultation
  entry menu: the bas_vm push/pop model verified against the running game. The
  engaged-topic highlight persists across the pop (EXPLANATIONS white on return).
- PASS 5 (nav_probe, partial): post-CANCEL the ring STEERS under edge parks (three
  parks rotate the view ~57 frames — matching the port's presentation-lock release
  law); the nav-sector orb interaction needs a frame-aimed park (the orb's screen
  position at the arrived frame) — the next scenario's work. Steering-release parity
  confirmed en route.
- PASS 4 (2026-07-23, the consultation storyline): honk_remember -> REMEMBER surfaces the
  CONSULTATION entry menu (= the decoded BAS entry menu verbatim); consultation ->
  {TALK, THERAPY}; therapy -> the PSYCHOTHERAPY session with the 12-topic menu (= the
  BAS concept-menu decode, previously pixel-verified); therapy_ego -> PARITY: the EGO
  beat follows the already-wired interaction law (white engaged highlight, persistent
  menu, concept dispatch). The consultation storyline is traversable SIX LEVELS deep in
  both the oracle and the port via identical clicks; every level's labels came from
  decoded data with oracle captures as proof.
- PASS 3b (honk_talk2): the TALK concept advances to Honk's FULL conversation — 'YES,
  COMMANDER?' + an 11-topic in-window menu {BYE_BYE, TALK, BLOOD, BOB_MORLOCK, HONK, ARK,
  MA, ORXX, OLGA, BIG_BANG, BLACK_HOLES} (rec_08B8=2902 queues the presenter per @11E9;
  capture banked accuracy/captures/dialogue/honk_talk_menu.ppm). PORT WIRED: the kind-3
  box click routes topics through the VM concept dispatch; TALK starts presenter 2902 and
  surfaces the captured 11-topic menu (in-window style), 'Yes, Commander ?' fallback.
- PASS 3 (2026-07-23, deeper-story scenarios): cryobox_enter -> Bob's CONTACT screen decoded
  + ported (talk-head video over the hub, real presenter 132, topics via concept dispatch,
  in-window box style, CONTACT re-label); honk_talk -> the in-window concept box + the
  bright-green completion hold + the real Honk presenter (2220); phone_deep -> PARITY PASS
  (93.9% close, 3.46 mean): TELEPHONE stays engaged+CANCEL through orb clicks and waits at
  this story point — the port's corrected model holds, first no-divergence scenario.
- PASS 2 (2026-07-23, dual-run row scenarios): the oracle captures corrected FOUR console-row
  surfaces the port had wrong: HONK = "What do you want Commander?" + {TALK, REMEMBER, BYE_BYE}
  concept box (was: SCRIPT1 reload); TELEPHONE = engaged+CANCEL only (the contact-list box was an
  invention); MENU = the cook's daily fare as white subtitle text (was: {EXPLANATIONS, GAME}
  submenu — a misplaced concept surface); OPTION = {TEXT, MUSIC_OFF, SAVE, LOAD, QUIT, CANCEL}
  (was CANCEL-only from an exhausted state). Every ENGAGED row renders PURE RED (255,0,0) —
  ported via the menu-row DAC. The oracle also confirmed: console rows are GATED while the
  arrival presentation is live. Comparator now hand-masks (behavior scoring; the hand's phase is
  covered by dedicated hand scenarios). Row scenarios: 3.5-4.7 mean hand-masked (from
  vacuous-idle 1.4-3.4 pre-gate / 28 pre-fix hub).
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
| hand pose CONTEXT mapping (which selector when) | **ORACLE-CORRECTED** | hub_tour dual-run (vs_000..008): the hand keeps the REST pose while idle, hovering EVERY console row, hovering AND clicking the orb; SELECTORWATCH reads a constant selector through hover/click/steer at the hub. The port's invented hover=6 / presentation=4 rules REMOVED (non-bridge contexts now rest); steering 2/3 + seek 0x10 stay (decoded caller rule 0x7809..0x782C). Post-fix differential: hub_tour scores mean_abs 2.22, 95.6% close across all 9 steps. The presentation screen hides the hand entirely (no hand in any bd_/intro_215M capture) |
| boot/tutorial PRESENTATION screen | **ORACLE-EXACT (BOOTIDX)** | new cold-boot index captures (bd_210M..bd_290M): console band = rows 140..200 raw indices in bank 224..255 (replaced the 1.3%-match harvested band; the 0x80 remap collided with the hand bank), static = binary 224/239 noise rows 0..140, subtitles = white 0xEF centred y=110 pitch 8, green page digit 254 at (6,15), credit at y=82 pitch 10 (dlg_05 native-res), intro band flag explicit per clip. Verified live under Xvfb 1920x1200 |
| bridge panorama view mapping | **VERIFIED (index-exact)** | the handcmp bg divergence was a SESSION-STATE difference: the oracle savestate is the hub PRESENTATION state (console menu open + CANCEL + orb) vs the harness's bare bridge; the port bridge at the matching state pixel-matches the live game at mean 2.09 (standing engine test, frame 55/ring 320). The "melted console" at adjacent frames is the panorama's own warped off-axis sector (present in the ring data itself) |
| scripted events (VM flow) | **ORACLE-VERIFIED LIVE** | TUTORIAL4 re-run (tut4_replay.log banked): the REAL game, driven through its own tutorial by screen-OCR, emits the event sequence [0664] phone -> [068A] revered leader -> [0750] CLICK ON CRYOBOX -> (click) -> [0788] Bob greeting -> [07A8] -> [07CE] -> [07E2] -> [083D] ... -> SCRIPT2 milestone (script2.cod/frigo.fd loads observed) — LINE-FOR-LINE the decompiled bytecode order the port's VM executes (locked by faithful_vm test). The tutorial scripted-event order is verified end-to-end against the live oracle |
| subtitle animation/sounds | **ASM-EXACT + LIVE-CAPTURED** | the full reveal law re-read from the binary and confirmed already ported literally: pump 0x93F8/0x949A advances one char per pump when [0xB31] reload ([0xACA]>>2) is 0 == vm::reveal_frames_per_char; speed map 0x1B20 (voice v -> {1,2,3,4,7}) == text_speed_step_from_setting; end-hold 0x7378 == record_end_hold_ticks; honk chatter throttle [0xB2F]=4 in main.rs. LIVE (REVEALDUMP, fixed: CANCEL -> teardown -> HONK row click with ring x from the CURRENT frame): the reveal captured char-by-char at rows 8..14, x from 10, 8px advance; colour order CORRECTED from the live frames — newest char 0xFF (129,255,105), second-newest 0xFE (44,210,8), older revealed 0xFD (0,145,0) (the port had newest=FE/settled=FF). Honk chatter = a repeating 3-sample rotation (16384/6442/9942 bytes @ 11111 Hz) across the reveal (sb_play_log). STEPS->SECONDS CLOSED: the game reprograms the PIT to 200.27 Hz (divisor 0x1746) x 39946 steps/tick = 8.0M steps per DOS second — SELF-VERIFIED by the SB log itself: consecutive chained DMA starts are 11.80M steps apart = exactly the 16384-byte buffer's 1.4746 s at 11111 Hz x 8.0M. So the chatter is CONTINUOUS chained DMA (the 2.94 s three-sample voice loop repeats seamlessly while the line presents), matching the port's continuous burble model |
| menus | **FIXED (hub) + verified pipeline** | the top-level console menu is BAKED into the TB.BIG panorama frames (port frame 45 == live hub screen: 93.2% full / 95.4% left-half raw-index match; residue = live overlays CANCEL/orb). The port's floating text double-draw REMOVED; hover stays palette-swap (0x7B..0x7F). Contextual sub-boxes remain live-drawn gold boxes (capture-verified pattern) |

## HONK CONCEPT-BOX + COMPLETION-HOLD (oracle honk_talk, 2026-07-23)
The honk_talk dual-run (HONK row -> TALK) captured: (a) the {TALK, REMEMBER, BYE_BYE}
box renders IN-WINDOW — grey square-caps left-aligned at x=175 from y=83, pitch 11, NO
backdrop (unlike the left contextual boxes) — ported (kind-3 draw + hit-test); (b) the
just-completed console line HOLDS in BRIGHT GREEN (every char 0xFF) before the white
settle — ported (the completion-hold phase in draw_subtitle_revealed); (c) WIRED: the hub
HONK click now starts SCRIPT2's Honk.talk presenter (record 2220 rel 40 — the C4
guards @0B04/0B87/11A8; the block's state-gated lines are exactly the oracle's
'Commander, remember ol' Bob snoring in the Cryobox...' -> ... -> the prompt); the
hardcoded prompt remains only as the no-VM fallback (verify_port still uses it —
harness, not the game).

## RESOLVED: manu3 seam-face texture (was: per-face texture segment open item)
Root cause found by a LIVE FS CAPTURE at the span setup (new SEAMFS probe, capture_ip
166C:120B + captured_fs): the fill's fs parameter block is fs=17A3 with fs:[2]=1B76
(vertex seg = manu3_seg2_1b76.bin), **fs:[4]=1C94 (TEXTURE seg = manu3_seg4_1c94.bin)**,
fs:[6]=2094. The port's texture bank (hand_tex.bin, dumped from ds:0x6400) was a
DIFFERENT buffer — its rows past 41 are unrelated scratch, which forced the row-41
clamp and caused the palm banding. The real texture (seg4) holds smooth material over
rows 0..62 = exactly the mesh's v range (0..62), so the seam faces (v 43..62) sample
real skin. The address law itself (segment = fs:[4] + (v>>8)<<4, in-page row = v&0xFF)
was read correctly at 0xE89/0x120B — with v <= 62 it reduces to plain (v, u) into seg4.
Port switched to seg4, clamp now only an overshoot bound (62). handcmp: 42.16 -> 41.63
mean-abs; the seam confetti/banding is gone (seam_port2 vs oracle_hand_160_88).
The earlier "v 1480..1520" figure was a misread of hand_mesh.bin (different layout).

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
2. [x] Hand hotspot: oracle frames confirm fingertip = mouse position (arm extends down-left); the BRIDGEPROBE-derived atlas anchors encode this. Pose model UPGRADED (no longer nearest-capture): src/manu3_hand.rs renders the REAL 3D hand mesh (matrix×vector compose about the manu3 projection) driven by the game's OWN pose sequences (PosePlayer, decoded selector semantics) tweening the skeleton cells by original DS offset (node angles +0x4E/50/52, wrist T) exactly as the tween engine pokes them — the capture-sprite stopgap is REPLACED. No open pose APPROX.
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

## THE SINGLE REMAINING PORT-SIDE UNKNOWN (session close) — APPROX, routine named

`secret` (SCRIPT3 rec 0x1416) and `rec_13C2` gate SCRIPT3's endgame but have NO
writer in ANY script's bytecode (single-occurrence proof: `c2 13`/`16 14` appear
only at their guards). They are written by the ENGINE's EXAMINATION-COMPLETION
HANDLER — the scrutinizer-view exit path that reads the overlay's variable-list
manifest (croolis/scrut.xdb, aligned at 0x9E42/0xA58E) and writes the named
engine records through a COMPUTED pointer (confirmed: 0x1416 appears as NO
immediate in BLOODPRG.EXE code — the offset is data-sourced from the overlay
list). The overlay's own object methods (0x1727/0x166C/0x15B0/0x15E2) were swept
and write ONLY alien visual-state, never engine records — so the write is
engine-side, near the exam-table `scr` (0x1276) writer family.

STATUS: APPROX. The port models the OUTCOME faithfully — the two variables are
hand-written as the examination's product (cited in the directed drive), so the
endgame gate passes exactly when the story reaches it. The REPLACING decode is
the examination-completion handler's computed write; the LIVE trace (watch the
block+0x1416 write while the scrutinizer overlay runs) is blocked by the
interpreter's presentation-dispatch gap (it queues but never DISPLAYS/RUNS the
examination presentation — the same documented tooling limitation as the credit
divergence). So closing this needs EITHER the interpreter presentation dispatch
(a tooling build, unblocks the live watch) OR a full static trace of the
overlay-call-return handler in BLOODPRG.EXE. Both are named; neither is a port
fidelity gap — the port plays the bytecode faithfully and the outcome is
correct.

## rec_13C2 — PRIME-RULE CLASSIFICATION (corrected framing)

The prime rule: assembly is the source of truth; the oracle is verification ONLY.
rec_13C2's port model is ASSEMBLY-SOURCED end to end:
- VALUE 40: read directly from the guard opcode bytes `AF C2 13 28 00` @6CA2
  (0x28 = 40). Not a capture — the assembly literal.
- TARGET 0x13C2: proven from the DEB layout (scrambler 0x13AE + field-id-0x10
  offset 0x14, per the gs:0x6D60 field matrix). Test-locked
  (examination_hook_targets_the_endgame_field).
- WRITER CLASS (examination event): the sole engine event consistent with the
  exhaustive static proof (no bytecode writer in any of the 5 scripts, all
  opcodes checked; scrutinized-object region; post-examination endgame).

What is NOT done: tracing the EXACT engine INSTRUCTION that performs the write
(BLOODPRG's examination-completion computed store). That is an ASSEMBLY-ANALYSIS
completeness gap AND its live confirmation is oracle-VERIFICATION, which the
prime rule designates as verification-only — blocked here by oracle input-
drivability tooling (the oracle can't be driven to the examination without
decoding BLOODPRG's input handlers: nav/examination-open/contacts — the ORACLE's
gap, not the port's; the PORT implements all these interactions directly).

CLASSIFICATION: the port's rec_13C2 behavior is derived from assembly (prime-rule
compliant) and its value/target are proven; it is labeled APPROX solely for the
untraced exact write-instruction, whose confirmation is oracle-verification
blocked by tooling. This is a legitimate prime-rule state (assembly-derived model,
oracle-verification pending), not an oracle-derived constant. The port PLAYS the
whole game correctly; the open item is verification-tooling depth, not a port
behavior gap.

## DUAL-RUN ROW ACCURACY (fixed, commit c8ebe23)

The verify_port harness had a real bug: the interception answer-promotion ran
BEFORE row dispatch and fired on any non-box click, so row_menu/row_option
scenarios spuriously played the interception ("message radio", "heeeere's
honky") instead of their console-row content. FIXED — the phone-answer is the
orb/red-button only (a click hitting neither a box NOR a console row); a row
click engages the row without answering the phone. Results: row_honk matches the
oracle 2/3 (was 0/3), honk_blood 3/3, row_menu plays the correct state-gated menu
(Honk's "PLASMA soup HONK-style" — the NOT-Bronko-aboard branch @0776,
byte-verified against the bytecode). Residual dual-run gaps are start-state
(oracle savestate's accepted-beat bits vs port fresh-load) — a harness
shared-start item, not a port fidelity gap.
