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
| manu3.rs | hand-overlay 3D machinery | **ASM+DATA (resolved)** | manu3 = *main* (French: hand); its LIVE role is the 3D hand cursor, decoded exact in manu3_hand.rs (see the hand-cursor row). manu3.rs holds the same overlay's cursor→pose camera pan (0x34..0x51, `menu_camera_pan` — the (x-160)*2/(y-100)*2 yaw/pitch), matrix-angle masking (0x270, `menu_pyramid_angles`), pose selector (0x181, `menu_item_handler`), and pose-tween processor (0x19B, `MenuTweenList`) — legacy "menu_" names from the retracted 3D-menu reading, each tested against the real overlay bytes. The "item sprites/RLE not decoded" gap was for the INVENTED 3D-pyramid OPTION screen, now REMOVED: the pyramid renderer + `option_active`/`option_item` machinery were unrouted (real OPTION = the gold CHOICE BOX, oracle-verified) — a fabricated surface deleted per the prime rule. No open manu3 gap remains |
| main.rs console_box row 2 | contact menu | **ASM (resolved)** | Was two transcribed labels; now built the way the game builds it. The bridge click at `0x86A4` dispatches through the per-row handler table `CS:0x0F29` (file `0x8709`, entry 2 = `0x0FDD` -> `0x87BD`), and that handler walks the 16-entry ship-slot array `DS:0x6D3E`: skip empty slots (`or ax,ax / je`), stop at `0xFFFF`, emit `record+4` — the object's INLINE NAME — into the menu list at `DS:0x2B13`. `DS:0x6D3E` is all zeros in the image because it is runtime state: the same array the insert/find/remove scans at `0x5FD8`/`0x5FF6`/`0x6008` walk with `mov cx,0x10`, already modelled as `VmMachine::ship_slots`. Ported as `ship_contact_menu_words`; the menu is whoever is aboard, and is empty when nobody is |
| main.rs OPTION menu | TEXT / MUSIC_OFF / SAVE / LOAD / QUIT + CANCEL | **DATA (resolved)** | Was six transcribed labels; now read from the game's own string table. Console row 4's handler `0x886C` does `mov si,0x2567` and calls the list widget at `0x8428`; `DS:0x2567` (file `0x0F987`) is a `0xFFFF`-terminated list of DS pointers to NUL-terminated labels at `DS:0x2573/0x2581/0x258B/0x2590/0x2595`. `CANCEL` is not in that list — it is the widget's shared trailing entry at `DS:0x0174` (`ship_3d_target_extra_label`). `MUSIC_ON` at `DS:0x2578` sits between `TEXT` and `MUSIC_OFF` and is deliberately absent from the pointer list: it is the toggle's other face, swapped in by state. Ported as `bloodprg::option_menu_labels` + `list_widget_cancel_label`, tested against the shipped binary. NOTE: a further list at `DS:0x259D` feeds a text-speed submenu (`VERY FAST`, ...) — not yet wired |
| engine.rs list menu x | concept/topic list label x | **ASM+ORACLE (corrected)** | The port centred each label using `0x857D` (`sub bx,[bp] / shr bx,1 / add bx,cx`), and a lib test asserted that. `concept_menu.ppm` disproves it: the real game puts all eleven measured rows at x=170, while the port put only the widest there. Both masks span x 170..280 IDENTICALLY at IoU 0.18 — correct band, wrong per-row placement. Now flush-left at the DERIVED `x0 + 10` (`x0 = anchor 0xE1 - (widest+20)/2`), which yields 170 for this set — NOT a return to the hardcoded 170 removed in #97. OPEN: which widget `0x857D` does centre |
| engine.rs bridge starfield | stars behind the panorama windows | **NOT A DIVERGENCE — claim WITHDRAWN** | An earlier row here claimed the bridge windows should show stars and render black, citing `nav_screen_opened.ppm` at mean_abs 102. That capture is NOT a bridge starfield: its top 135 rows are TWO colours, black and white, in per-pixel noise (mean run 1.87px) — the binary STATIC of the presentation/boot screen, with the console band below. The port's star layer plots 33 pixels because a 1000-point cloud plots ~1000 at most; the capture has 19855 white pixels, which no point cloud produces. The filename misled the comparison. See audit-fixes #115 |
| audio.rs mixing | simultaneous sounds | **WIRED (audible check pending)** | The game mixes by AVERAGING into a voice buffer, never by summing independent streams. Two paths reach a buffer: the loader OVERWRITES it (`int 21h`/`AH=3Fh` @`0x4049`) and the streamer AVERAGES a chunk in (`lodsb / add al,es:[di] / rcr al,1` @`0xBB6D`), so a lone sound is unattenuated and later ones layer at half weight each. `audio.rs` now opens ONE cpal stream and every `MusicPlayer` is a handle on a shared `AudioMixer` folding sources with `snd::mix_unsigned_pcm_layered`; the public API is unchanged, so all `main.rs` callers got this without edits. `AudioMixer::render` is device-free and tested (idle silence, lone source whole, pair averaged sample-by-sample, play-once reaped, loops not). Supporting decodes, all ported+tested: `stream_mix_span` (`0xBB2E..0xBB4E` ring spans), `mix_unsigned_pcm_half_rate` (`0xBB5B`), `snd_header_is_half_rate` (`0xBBFE`, the rate is DATA in the clip header), `SndStream` (two `0x4000` voices over the loaded file, `0xBBE4..0xBC2F`), and the driver vector map (static far pointers at `DS:0x0CD3`; `0xCF3` is vector 8 = the 8237 DMA current count, `DRV:0x01CA`). Full derivation in docs/audit-fixes.md #198-#201, #206, #213-#214. REMAINING: a device, to confirm it SOUNDS right -- not to know the mixing is right |
| vm.rs world destination commit | selecting a world target | **WIRED (ASM end-to-end)** | `world_click_select` ports `0xB20C..0xB27B` fully: `0` = nothing hit; `0xFFFF` = the back row, clearing the target; a target equal to `gs:0x251B` is already presented and is NOT rewritten (`cmp ax,[0x251b]` @`0xB21A`); anything else sets `gs:0x251B = target` and writes a C1 record `{0xC1, target, 0}` at `[0x6750]+0xA` (orxx), which the C1 ladder (`record_type_ladder` `0x5B38`) presents on a later frame. Nothing calls it — `world_target` is touched only by this function and its tests. `main.rs` instead calls `engine.visit_world(...)` directly, so the world appears but NO C1 record is written and the VM's presentation ladder never runs for it. Consequence: any script logic gated on that record cannot fire. WHY it is not wired, precisely: the VM commit takes a target RECORD, and the frontend's destination path has only a world NAME (`targeted_world_name`, picked by `compass_angle * n / 180` — frontend arithmetic, where the game hit-tests a nav-chart object). The game never needs a name->record mapping because it commits the object the player CLICKED; inventing one in the port would be a fabricated rule. The decoded pieces for the real route all exist — `build_nav_chart_list` (`0x721A`) for the navigable objects, `nav_chart_object_click`/`nav_chart_pick` (`0x92A3`) for the hit-test, `world_click_select` for the commit, `object_inline_name` for the art name — and `nav_chart_click` is already wired to the INFO PANEL (`location_panel_rows`). The task is routing the destination COMMIT through that same click, not bridging a name to a record **DECODE UPDATE (this session): the blocker's premise was WRONG.** The hit-test that feeds the commit is `ship_3d_target_record_select` (`0xB2BB`), and it is NOT a spatial nav-chart pick: it is the unified list widget (`0x71E:0xC48` -> `0x8428`) over the word list at `DS:0x250B`, whose entries are `RECORD+4` pointers to inline names. `sub ax,4` @`0xB33D` turns the selected row back into a record. So the game DOES have a name->record mapping and it is subtraction, not a table -- the exact inverse of the `add ax,4` @`0x87D5` that BUILDS such a list. Ported as `vm::ship_3d_target_record_select` with the fallback rule that proves the reading: when `DS:0x250B` is empty the widget is fed DS-relative names (`es=ds`, `DS:0x2537`), which are not inside records, so the code discards `sub 4` and returns `[0x251B]` -- the current target -- which `world_click_select` then rejects as unchanged. THE FALLBACK LIST CANNOT COMMIT A NEW DESTINATION. What remains open is only the frontend route: `main.rs` still reaches the world through `targeted_world_name()`/`visit_world`, so the C1 record is still not written at runtime BUILDER ALSO DECODED+PORTED: `entity_candidate_list` (`0x7259`) fills that list -- filter `flags & 0x98` @`0x727E`, `+2` byte bit 1 @`0x7284`, excluding `arche` (`gs:0x6752`) @`0x728B` -- and emits `add ax,4` @`0x7292`, which CONFIRMS the reader's `sub ax,4` from the opposite side (writer and reader disassembled independently, neither reading resting on the other). `vm::entity_candidate_list` + a round-trip test: build the list, select a row, commit the C1 record. The chain `build_nav_source_list` (`0x624B`) -> `entity_candidate_list` -> `ship_3d_target_record_select` -> `world_click_select` now exists end to end in the VM. NOW COMPOSED: `destination_candidate_records(target)`. The DI question is SETTLED -- `0x624B` preserves DI across its recursion (`0x6276 push di / mov di,ax / call 0x624b / 0x627D pop di`), so `0x7259`'s `mov ax,di` @`0x726F` is the caller's target, which is tested as candidate zero and normally dropped by the `arche` exclusion. `entity_candidate_list` still takes `first` explicitly so the routine stays modelled exactly. ROOT NOW DECODED TOO: `0x7259` is far-called from exactly two sites (`0xB0EE`, `0xB105`, found by searching the `0x4DA:0x1EB9` encoding), both in `ship_click_commit`, whose `0xB0EA mov di,[0x6752]` roots the chain at `arche` -- read, not assumed. `vm::ship_click_initial_target` ports `0xB0DC..0xB111`: the location's kind (`test es:[eax],0x140` @`0xB0FB`) picks the first CANDIDATE or the LOCATION object, and `add di,4` @`0xB10A` merely pre-compensates the shared `sub 4` @`0xB111`. **COMMIT NOW WIRED.** `main.rs::commit_world_destination` runs the decoded chain when the port enters a world: `ship_click_initial_target` (rooted at `arche` per `0xB0EA`) -> `world_click_select`, so the C1 record IS written at runtime and script logic gated on it can fire. `check_unrouted_rules.py` no longer flags `world_click_select`, its flagship decoded-but-unwired example. Every value comes from the VM's own records; the frontend supplies only the MOMENT. **CHOICE NOW WIRED TOO.** `vm::destination_rows` returns each candidate's RECORD and the NAME inside it -- no lookup table needed, because a list entry is `RECORD+4` (`0x7292`) and `object_inline_name` reads `object+4`, so the stored entry IS the string pointer. `main.rs` enters the world named by the chosen row and commits THAT record. `compass_angle` survives only as the no-DEB fallback, where there are no records to offer -- and `engine.rs` already recorded that the angle merely pans the view, so it was never the game's chooser. **ROW SELECTION NOW GOES THROUGH THE DECODED WIDGET.** The game runs the destination list through the unified list widget (`0x8428`), and the port already implements that widget's row hit-test (`console_box_click`, `div bl,0x0B` @`0x8508`) for other menus -- it simply was not used here. The world-entry key now OPENS the box (rows named by `destination_rows`, trailing CANCEL read from `DS:0x0174`) and a click selects the row, committing that row's record. The key-cycled cursor is gone. The click arm is placed before the chart handlers because an open list takes precedence in the game too (`0xB2DC` keeps the FSM in the list while it is up). NOTHING APPROX REMAINS ON THIS ROW: rows, order, names, hit-test, committed record and cancel label are all the game's |
| engine.rs console band | intro/tutorial pyramid band | CAPTURE | pixel-exact harvest from native DOSBox raws (static across times) |
| engine.rs hand cursor | pointing-hand 3D model | **ASM (laws) + APPROX (data)** | THE REAL 3D HAND, live as the cursor. DATA PROVENANCE CORRECTED (audit-fixes #429): the mesh (142v/216f) + UVs + texture and the node-tree state are RUNTIME CAPTURES, not shipped data -- neither `accuracy/manu3/manu3_ds.bin` nor `manu3_seg2_1b76.bin` appears in `manu3.xdb` (searched verbatim; the dump agrees with the overlay on 16192 of 62544 bytes). The row read ASM+DATA, which claimed a file provenance the blobs do not have. #430 then FOUND that provenance: the dump is manu3.xdb loaded at a fixed shift (`ds[i] == xdb[i + 0x1370]`, 52698/57568 bytes = 91%), the mesh region agreeing 98.8%, so the mesh is shipped data the port can read directly and only the node tree's pose state is genuinely runtime. THE LAWS remain exact: EXACT Q14 matrix build (trig ds:0x26), EXACT perspective projection (0x549), z-buffer visibility, top-left edge ownership; 16-segment SKELETON with 9 decoded POSES driven by the exact tween player (0x1DF/0x19B). Console fidelity 2.09 mean_abs (hand region excluded — a live 3D cursor is not pixel-comparable to one frozen pose). Atlas retired to a test-only reference. |
| engine.rs intro flow | logos/montage/credits | CAPTURE+DATA | DESCRIPT present record + real-args DOSBox captures (rows 69/79 credits, band rows 99..200) |
| engine.rs TV | broadcast channels | DATA | 7 self-identified Sequence records; chained clips+music+cues |
| engine.rs telephone/cryobox | console screens | DATA+**ORACLE** | savestate probes: TELEPHONE/CRYOBOX rows open contextual gold CHOICE BOXES (the console's universal interaction; CRYOBOX = {BOB_MORLOCK, CANCEL} tutorial-verified) -> the port routes row -> box -> item -> screen (bappel call) |
| engine.rs BOB_MORLOCK contact | CRYOBOX -> BOB screen | **ORACLE-CORRECTED (dual-run)** | cryobox_enter scenario (vs_003..007): choosing BOB_MORLOCK opens Bob's CONVERSATION screen — his talk-head video (pe/aabob.hnm, the red-face eye close-up; frigo.fd file-open traced) + console-position subtitles + his concept menu {BYE_BYE, BLACK_HOLE, BIG_BANG, BOB_MORLOCK, KANARY, MISSION, CORPO, GOOD_OL_BOB} at x=170 y=56 pitch 11 — NOT the cryo-chamber video the port had. Ported (render_bob_contact + bob_topic_click; BYE_BYE returns to the bridge). Residuals: the dark-teal border band's palette source; topic-click -> SCRIPT2 conversation-beat wiring; the engaged CRYOBOX row re-labels red CONTACT (vs_003) |
| engine.rs cyberspace | BIOXX minigame | **DATA (decoded routing)** | FIXED: cyberspace now routes through the world-visit system on the cyber.ext world (level index 36 'cyber', 1cyber*.lbm rooms, BIOXX = its entities via the list-driven engage; goal touch->BIONIUM). Same decoded model as the planets; verified the cyber world loads+activates. Residual (cosmetic): the exact cyber-room 3D vs 2D presentation + the per-visit playthrough pixel-confirm |
| engine.rs OPTION menu | choice box | **ORACLE** | savestate resume-probe (ring-corrected clicks — the console mouse-x is RING space, the reason earlier probes never dispatched): OPTION opens the measured gold choice box containing CANCEL; the invented 3D-pyramid OPTION screen is UNROUTED. MENU's {EXPLANATIONS, GAME} box same mechanism |
| engine.rs world visit | on-planet screens | **DATA (decoded)** | rooms/objects from decoded data; interaction is LIST-DRIVEN per the full traced chain (candidates 0x7259 -> box -> commit 0xB0F3 -> C1 0x5B75; entities STATIC, dirty-rect tracked not walking). Port matches; candidate labels = the script's distinct DEB-resolved actor names (the decoded 0x7259 entity list is the location's characters), host-label fallback — WIRED |
| engine.rs nav view | star chart + list | **CAPTURE+CLOSED** | CHART.FD bg + tablo2 toggle 0x886C verified; the invented compass steer (dead-zone/rate) was REMOVED — the real chart is static + target-list selection (regression test). No open steer constants |
| save.rs | port save format | n/a (port-own) | DOS interop via vm dos_save |
| progress.rs / entity.rs | progression FSM | DATA(partial) | entity records decoded; the REAL ending trigger is SCRIPT5's Bigbang-concert block (GUARD rec_103A==Bigbang && rec_1340==concert && active_actor==Migrator.talk → lpm*sc1 reels → LOADSTR fin.hnm — now wired via the VM LoadString path); all-visited remains only as a driver fallback. **CONCERT-WRITER DECODED:** rec_103A/rec_1340 are never written by literal script assignment (only GUARD-read, and only in SCRIPT5) — they are reference records of the same class as rec_0F4E (chapter location). rec_1340's value field (0x1346 = base+6) is written by **OP_C1 mode-0** (`write_c1_record_state_mode0`): SCRIPT5's three C1 sites `C1 46 13 {DC 0F / F4 0F / 0C 10}` write exactly 4060/4084/4108 — the same three values the rec_1340 guards check (lines 358/331/379). The port's C1 mode-0 write is real but owner-gated (target's owner object active + target field empty). rec_103A, by contrast, is NEVER written by any script instruction — no literal assign, no OP_C1/C0, no OP_CD (the 4 CD sites target 0x12DC/0x127C/0x10E4), not the init block — while its guard value 4024 is a record-IDENTITY literally assigned to *other* records (rec_07B2, rec_025A) and active_actor is a *separate* guard. So rec_103A is an ENGINE-MAINTAINED reference record, the same class as rec_13C2 (below): its exact engine write-instruction is the identically infrastructure-blocked trace, not a decodable script opcode. **SINGLE ROOT:** the C1 owner-gating is NOT an independent gap — every rec_1340-advancing C1 write lives INSIDE a block guarded by `rec_103A==4024` (e.g. the C1 at [1507] is in BLOCK [143D]: GUARD rec_103A==4024 && rec_1340==4060 && active_actor==Migrator). The whole SCRIPT5 concert FSM is a state-transition chain gated on rec_103A==4024, so it cannot advance in natural play until the engine sets rec_103A — collapsing the residual to ONE infrastructure-blocked root cause (rec_13C2-class), which is exactly what the all-visited fallback stands in for |
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

## 53 DECODED RULES HAVE NO RUNTIME CALLER (2026-07-25)

`tools/check_unrouted_rules.py` reports 111 `pub fn`s with no runtime caller, 53
of them carrying a binary citation. Each of those 53 is a rule decoded from the
game, tested, and executed by nothing — the class of defect #196 closed for the
world-destination commit, where `world_click_select` had sat correct and unused.

Found this session while reading `copy_ship_3d_plane_bands` (`0xB6DD`) against its
routine. The transcription is faithful, including the signed `jle` in its scroll
computation and the `0xA` scroll-mode hold — and the function is called only by
its own tests, so THE VGA PLANAR BAND COPY THAT DRAWS THE SHIP-3D DEPTH BANDS DOES
NOT RUN. Its `new_scroll_value` result, which the game writes to `DS:0x524F`, is
returned and dropped.

This is a different kind of open row from the rest of this file. Those record
things not yet decoded; this records things decoded CORRECTLY and not connected,
which no amount of further decoding will fix and which the accuracy ledger cannot
see — a settled ASM row and an unrouted one look identical there.

NOT a list to work blindly: some of the 53 are legitimately callable-but-unused
(test hooks like `special_slot_insert_pub`, alternates like
`ship_3d_target_record_select` whose caller supplies rows another way). Each needs
the judgement #240 applied to the nav destination list, where "unused duplicate"
turned out to be wrong. The number is recorded so the question gets asked.

## THE ALIEN OBJECTS ANIMATE BUT DO NOT MOVE OR CULL (2026-07-25)

Working #266's unrouted list. `engine.rs` drives `AlienObject::step()` once per
frame, which is the ANIMATION state machine (`0x16A4`): the timer counts down, the
PRNG picks a new state, the anim counter advances. That part is wired and correct.

Three sibling behaviours, decoded from the same overlay, are called by nothing:

  * `update_position` (`0x999`) — the per-frame position integration;
  * `proximity_visible` (`0xA30`) — the visibility/cull test, which computes a
    screen Y from the object's position plus the camera and rejects it outside
    `0..VISIBLE_SCREEN_Y_MAX`, then bounds world X to `±VISIBLE_WORLD_X_HALF`;
  * `reset` (`0x36A`) — the initializer that puts an object in its start pose.

And `AlienObject::dispatch` routes by method: only `AnimStateMachine` does
anything, `SubBehaviour(_)` returns `false` unconditionally.

CONSEQUENCE: the port's aliens cycle animation frames in place. They do not move,
and nothing culls them by proximity — the game's objects do both.

CAMERA NOW DECODED (#269), and it is not what the port's signature assumes.
`croolis.xdb 0xA70` adds `word [0x22ec]` for X and `0xA62` adds `word [0x22f0]`
for Y. `0x22EC` really is a word. `0x22F0` is the HIGH WORD OF A DWORD at
`0x22EE` (`mov ecx,dword ptr [0x22ee]` @`0x791`, `add dword ptr [0x22ee],eax`
@`0x1FD5`, and a dword there spans `0x22EE..0x22F1`).

So the camera Y is the integer part of a 32-bit fixed-point accumulator. The
port's `camera: [i16; 3]` cannot represent that: three independent words drop the
fractional motion, which is what makes the movement smooth. SIGNATURE FIXED (#270): `AlienCamera` now carries `x: i16` (`0x22EC`),
`y_fixed: i32` (the `0x22EE` accumulator) and exposes `y()` as its high word —
the value `0xA62` adds. `proximity_visible` and `update_position` take it instead
of `[i16; 3]`, and a test shows a third-of-a-unit step accumulating across three
frames before the integer part moves, which an `i16` camera would have rounded
away every frame.

STILL OPEN: the three behaviours have no runtime CALLER. What remains is the
alien view's per-frame update — who advances `y_fixed`, and with what step. That
is a further decode in `croolis.xdb` (`add dword ptr [0x22ee],eax` @`0x1FD5` is
where it is written; what computes `eax` there is the question), not a
signature problem any more.

## STAR-MAP NAV VIEW — the exact projection IS the live one (2026-07-25)

Checked while reading `ship3d.rs`'s remaining ASM? functions, because
`render_star_map_navview` is documented as "a VISUAL APPROXIMATION ... without the
exact recovered geometry/projection", which would be an APPROX row.

It is not the live path. `engine.rs` calls `render_star_map_navview_projected`,
which projects through `project_star_map_point` — the exact `0x9BBA` arithmetic,
verified instruction by instruction in audit-fixes #273 (dot products, `sar 7`,
the `0xa0`/`0x64` centres, and the depth's unsigned-vs-signed division split).

So the approximation is superseded code, reachable only from its own wrapper and
from tests. Marked as such in the source. No APPROX row is needed for the nav
view's geometry; the row that WOULD have been needed is closed by the projected
renderer already being wired.

## NAV DESTINATION LIST GEOMETRY — APPROX, replacement already written (2026-07-25)

`engine::NAV_DEST_X/Y/PITCH/W` (6, 22, 10, 150) place the choose-a-location list
at a fixed position. NOTHING cites them, and the game lays no list out that way:
the unified widget `0x8428` MEASURES its labels and derives the box (width =
widest + 20 @`0x84A1`, height = rows*pitch + 8 @`0x84A7`, x = anchor - width/2
@`0x84AD`), which the port already implements as
`ship3d::layout_ship_3d_target_list` and tests against the game's own strings
(audit-fixes #220).

So the port carries TWO list layouts: one decoded, one invented. The comment above
the draw called the invented one "the game's list-box nav".

STATUS (corrected, #240): labelled APPROX in the source. The "delete it as a
duplicate" plan was WRONG and was withdrawn after tracing what fills the list:
`main.rs` builds `nav_destinations` from the SCRIPT3..5 BUNDLES — label from a
bundle's first actor record, entries its parsed dialogue lines — so this is a
PORT-SIDE AFFORDANCE for reaching scenes, not a second rendering of a game
surface. The game's destination list comes from the DEB candidates (`0x7259`) and
is routed through `console_box` (#212), whose click arm sits BEFORE this one in
the event loop and therefore wins when a DEB is loaded.

So the defect is narrower than first written: not a duplicate layout, but an
invented layout that CLAIMED a game provenance. The claim is removed; the four
numbers stay, labelled, until someone decides whether the port should offer this
affordance at all. Deleting it would have removed scene access that the decoded
path does not provide when no DEB is loaded.

## RESOURCE DIRECTORY — a transcribed literal, and it is a PREFIX (2026-07-25)

Chasing the `.drv` loader (to map the driver vector slots) turned up the game's
own FILE MANIFEST: a 95-slot table of 16-byte NUL-padded filenames at `FS:0x0c04`
= file `0xCDF4`. The driver names are slots 1 (`nosound.drv`) and 25
(`dnsdb.drv`), which is why no immediate search ever found a reference to those
strings — the code indexes the table, it does not point at the names.

`levels::LEVEL_DIRECTORY` is 53 of those 95 slots, hand-copied into Rust source.
Two separate problems, both now measured rather than suspected:

1. It is a CONTENT-BEARING LITERAL restating game data — the defect class
   `CLAUDE.md` names first. `parse_level_directory` now reads the table from the
   image, and `level_directory_literal_matches_the_image` holds the literal to
   those bytes. The transcription turned out to be CORRECT for all 53 (a real
   check that could have failed, not a formality).
2. It is INCOMPLETE by 42 entries: further `.ext` worlds AND the entire
   script3/4/5 file sets (slots 76..90). The frontend already loads `SCRIPT3..5`
   by name, so the port has been reaching for resources its own directory does
   not list. `level_entry_from_image` reads any slot, tested at 76, 86 and 94.

STATUS: DONE. `init_level_directory(image)` installs the parsed 95-slot table in a
`OnceLock` and `directory()` backs both `entry()` and `primary_worlds()`, so every
caller now sees the real table; `main.rs` calls it at startup from whichever
BLOODPRG.EXE path exists. `LEVEL_DIRECTORY` remains only as the fallback for
contexts with no image, and `derived_directory_reproduces_the_literal` asserts the
parse equals it stem-for-stem, kind-for-kind over all 53 shared slots -- so the
literal is now checked BY the parse rather than trusted alongside it.

CONSEQUENCE, measured: `primary_worlds()` goes from 16 to 32. Sixteen top-level
`.ext` worlds were simply absent from the port's model. The nav map draws
`take(7)`, so the visible surface is unchanged, but anything enumerating worlds
was working from a little over half the set.

The stem convention (`.spr`/`.ext` stripped, other extensions kept) is the PORT'S,
not the game's -- the table stores full filenames -- and `level_stem` says so.

NOTE ON KINDS: the table stores FILENAMES ONLY. `LevelKind` is the port's own
classification by extension, and `level_entry_from_image` says so in its doc. The
names are the game's; the kinds are ours.

## THE TWO UNREAD CAPTURES — measured, and one now verified (2026-07-25)

Both were listed as "needing composite reproduction". They are measured now
(`re/tools/ppm_stats.py`, written for this so the numbers are reproducible rather
than impressions -- the #114 failure was reasoning from a capture's APPEARANCE):

* `bridge/script2_first_frame.ppm` — 320x200, 50 colours, mean horizontal run
  4.58px, and rows 0..39 are a SINGLE colour (`#080014`), which is also 42.2% of
  the frame. Nothing in the decode says "40": the panorama is full-screen
  (`PANORAMA_FRAME_PIXELS`), so that band is the frame's own CONTENT, not
  geometry. That makes it a falsifiable claim ABOUT THE ARCHIVE, and
  `some_panorama_frame_opens_with_a_flat_band_like_the_capture` now checks it:
  the port decodes `TB.BIG` from the game's file and some frame does open with
  40+ uniform rows. Direction preserved -- the decode produces the pixels, the
  capture only confirms them. STILL OPEN: matching the SPECIFIC frame index plus
  scene palette and console overlay, which is the full composite.

* `mission_briefing_eye.ppm` — 320x200, 173 colours, mean run 2.45px, no flat
  bands and no letterboxing. Those are the statistics of a DITHERED FULL-SCREEN
  VIDEO FRAME, i.e. an HNM still, not a UI surface. Reproducing it means
  identifying which DESCRIPT record names that clip and decoding the frame; the
  port already has the HNM decoder, so this is an identification task rather than
  a decoding one. NOT attempted here, and deliberately not guessed at from the
  imagery.

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

## rec_103A / rec_13C2 writer — EXHAUSTIVE static refutation; the SET is a runtime-computed native write (needs runtime observation)

**FINAL DISPOSITION for this item.** After a full static investigation, EVERY
concrete writer hypothesis has been refuted by direct checking — so rec_103A's set
to a plot-identity has NO static-decodable signature and requires runtime
observation (the oracle, blocked interactively) to pin. The five refuted hypotheses:
1. A SCRIPT5 bytecode opcode — NO. All 0x103A occurrences in SCRIPT5.COD are
   `AF`-prefixed 0x6946-family GUARD reads; no literal/6946-write, no OP_C1/CD.
2. The 0x5816 deferred-record drain — NO. It lands on arche+0x1C/+0x1E (field
   arithmetic), not arche+0x16 (= rec_103A).
3. A field-matrix selector write — NO. No selector maps to offset 0x16 for arche's
   kind (kind-1 offsets: 0x02/0x04/0x06/0x08/0x10/0x20).
4. A direct `mov [reg+0x16]` store — NO. The only two such sites in BLOODPRG write
   32-byte ENTITY records (entity_object_table @0x6212, "+0x14/+0x16 init backups"),
   not the arche object; the sole write to arche+0x16 is the CLEAR-to-0 @0x6b44.
5. A B8-family pair-write at 0x1038 (so second word = 0x103A) — NO. SCRIPT5 has no
   B8/B9/BD opcode targeting 0x1038.
6. A derivable actor→scene mapping (set rec_103A from the active actor) — NO. The
   SCRIPT5 guard co-occurrences give Bug_Deluxe→{2408,4024} and Yoko→{2930,4024}:
   the SAME actor appears in different rec_103A phases, so the value is NOT a function
   of the active actor. (Migrator→{4024} alone is clean, but a Migrator-only heuristic
   models the concert outcome, not the mechanism, and would not set the other phases.)
So the set is native, runtime-computed, from state (which object is active + its
record layout + plot history) that static analysis cannot pin — SIX distinct
approaches now refuted by direct checking.
BOUNDED ORACLE ATTEMPTS also fail (confirming the tooling frontier, not a quick win):
since arche+0x16 = rec_0F4E in SCRIPT2 (same native writer as SCRIPT5's rec_103A),
watching rec_0F4E should catch it — but (a) the hub_tour scenario on script2.state
writes rec_0F4E zero times (it reads 0 there; passive hover triggers no location
write), and (b) a 250M-step boot run with the pointer-relative BOOTWRITEWATCH caught
NO rec_0F4E write (only a timer IRQ; the block relocated at ~210M as the attract
cycled profiles). So observing the write needs driving the oracle to the specific
location/scene-transition MOMENT — the interactive-oracle tooling (decode BLOODPRG's
nav/story input handlers), not a bounded run. Both the static and the bounded-oracle
shortcuts are now exhausted; only the full tooling build remains.
DIAGNOSTIC — CORRECTED (my prior "broken block resolution" claim was WRONG; retracted
on verification, per the prime rule). The block resolves FINE: the profile word
gs:0x677E = 0x01 (= SCRIPT2, correct) and gs:[0x6724] is a stable real segment
(0x8681:0000). rec_0F4E reads 0 because script2.state is a PRE-LOCATION-SET state
(SCRIPT2 loaded, but before the arrival that writes rec_0F4E=3488 — the SCRIPT2
opening guard `rec_0F4E==3488` is not yet satisfied at this savestate). The "string
data" at 0x103A/0x1340 is simply SCRIPT2's DIFFERENT object layout (SCRIPT2's arche is
at 0xf38, so its arche+0x16 = 0xF4E; 0x103A is an unrelated SCRIPT2 object) — not
garbage, not a resolution bug. So the tooling task is NOT "fix block resolution" (it
works); it is "drive the oracle from script2.state THROUGH a location-set to observe
the rec_0F4E write" — which needs the story-flow/input handlers (the arrival is
triggered by gameplay progression the oracle can't yet drive). That is the real
interactive-oracle frontier, now correctly characterized. This CORRECTS the intervening
"deferred-record port-side wiring, not blocked" framing: that was a hopeful detour,
refuted by (2). WHAT IS DECODED and durable: 4024 = the "Bigbang" plot object;
rec_103A = arche+0x16 = a VM-maintained "current plot object" field that the 0xB8
handler CLEARS via the 0x6034 lookup match. The honest state matches rec_13C2:
assembly-derived model of the OUTCOME (the port plays correctly via the documented
all-visited fallback), with the exact engine SET-instruction pending runtime
verification — a legitimate prime-rule state (assembly-sourced, oracle-verification
tooling-blocked), NOT an incremental static gap. Reaching it needs the interactive
oracle (decode BLOODPRG's nav/examination input handlers) or a runtime record-directory
snapshot at the concert — the same tooling frontier documented for rec_13C2 and the
credit divergence.

**STRUCTURAL REFRAME (grounded, verified via both DEBs).** rec_103A is NOT a foreign
field — it is the same class the port ALREADY models. arche+0x16 is consistently the
"current location/plot reference" field: arche @ 0xf38 in SCRIPT2 → arche+0x16 = 0xF4E
= **rec_0F4E** (the location variable, driven by the port's `set_location`); arche @
0x1024 in SCRIPT5 → arche+0x16 = 0x103A = **rec_103A**. So rec_103A is SCRIPT5's
location/plot-reference field, the exact analogue of SCRIPT2's rec_0F4E. The gap is
narrower than "unknown native writer": the port drives arche+0x16 on ARRIVAL
(`set_location`, offset found by `location_var_offset`'s first-block wildcard-guard
scan), but (a) `location_var_offset` returns None for SCRIPT5 (its first block is
init-writes, not a wildcard guard, so the port never discovers 0x103A), and (b) the
port doesn't model SCRIPT5 ADVANCING arche+0x16 through wedding-plot phases (4024
Bigbang → …). A grounded port path therefore exists: teach the location-var discovery
to find SCRIPT5's arche+0x16 and drive it through the plot phases — still gated on
knowing the phase-transition triggers (the runtime-computed part), but now anchored to
a MODELED mechanism (set_location) rather than a from-scratch native decode.

--- (historical trail, superseded by the disposition above) ---

**CORRECTION (supersedes this section's earlier "deferred-drain writes rec_103A"
claim).** DEB decode + field-offset arithmetic falsified the clean hypothesis, and
per the prime rule the refutation is recorded rather than implemented on:
- CONFIRMED: **4024 = the "Bigbang" DEB object** (kind 1) — so `rec_103A==4024`
  means the plot-reference points at Bigbang (the wedding-concert entity). And
  **rec_103A (0x103A) sits inside the `arche` object** (arche @ 0x1024 → rec_103A =
  arche + 0x16).
- REFUTED: rec_103A is NOT the deferred-drain target. The 0x5816 deferred write
  lands on `arche + vm_field_offset(C9_RELATED=0x13, kind) [+2 related]`, whose
  values for arche are 0x102E (kind 1) / 0x1042 (kind 0x10) / 0x1030 (kind 0x200) —
  NONE equal 0x103A. So the arche+0x16 field that IS rec_103A is written by a
  DIFFERENT engine field-write path, not the C9_RELATED deferred drain. (Had this
  been implemented on the hypothesis it would have been a fabrication; the field
  math caught it — verification working as the prime rule intends.)
- STILL OPEN: which write path sets arche+0x16 to the Bigbang identity. ALSO
  REFUTED: it is not a standard field-selector write either — no selector in the
  field matrix (0x6D60) maps to offset 0x16 for arche's kind (kind-1 offsets are
  0x02/0x04/0x06/0x08/0x10/0x20, never 0x16). So rec_103A = arche+0x16 is written by
  a DIRECT structural store (a routine that handles the arche object with a hardcoded
  +0x16, or a {type,related,aux} record-entry triple whose related word lands at
  +0x16), OR the record base at the write site is not arche. The deferred-record
  mechanism below is real and the port genuinely lacks its setter, but it is NOT
  established to be rec_103A's writer.
- LOCATED (this pass): arche+0x16 IS manipulated by the VM opcode handlers. The
  0xB8/0xB9/0xBD handler (`vm_op_b8_record_readwrite` @0x6b06), after its 2-word
  pair write, computes `vm_record_lookup_by_threshold` (0x6034) and, if the result
  matches arche+0x16, CLEARS it (`mov es:[di+0x16],0` @0x6b44, di=gs:[0x6752]=arche).
  So rec_103A is a VM-maintained "current plot/scene object" field, cleared by the
  B8 family. The SET to a plot-identity (Bigbang=4024 etc.) is NOT a SCRIPT5 opcode
  (none of the twelve 4024-writes target 0x1038/0x103A) but NATIVE presentation-
  maintenance tied to the 0x6034 record-lookup. So the trail ends honestly here: no
  longer "computed write with no signature" but a LOCATED VM-internal field whose SET
  path is a bounded native RE task (decode 0x6034 + the presentation set-site). The
  port's B8-family handler (`is_pair_record_opcode`) does the pair write but NOT the
  arche+0x16 side-effect: a concrete divergence to close once the set-site is decoded.

SCRIPT5's Bigbang-concert ending is gated on `rec_103A==4024` (the concert FSM's
every edge — see the progression FSM row). rec_103A was investigated by THREE
independent static/data methods, no oracle involved:
1. SCRIPT analysis: NO bytecode writer in any of the 5 scripts — all 29 rec_103A
   occurrences are GUARD reads (0x6946-family query mode); no literal assign, no
   OP_C1/C0 (all 6 SCRIPT5 C1 sites target 0x1346=rec_1340), no OP_CD (targets
   0x12DC/0x127C/0x10E4), not the init block.
2. DATA: rec_103A's initial SCRIPT5.VAR value is 0, not 4024 — so it is genuinely
   runtime-written, not static table data. (4024 is a record-IDENTITY constant
   present at OTHER VAR slots 0xFF0/0x1008/0x1020 and literally assigned to
   rec_07B2/rec_025A.)
3. STATIC DISASSEMBLY: the offset 0x103A (`3A 10`) and its absolute gs address
   0x775E (`5E 77` — rec_103A = gs:[0x6724+0x103A]) appear as NO immediate anywhere
   in BLOODPRG.EXE; likewise rec_13C2's 0x13C2 (`C2 13`). The tool is sound (the
   record-table base 0x6724 is found at image 0xB269). So BOTH records are written
   only through COMPUTED addressing (record-base register + field offset).
WRITER MECHANISM — DECODED, and it is NOT infrastructure-blocked (corrects the
earlier "needs oracle / full decompile" framing). The post-VM scan **0x5816**
(`vm_post_exec_record_update`) DRAINS a DEFERRED RECORD (gs:0x6768 type / 0x676a
related / 0x676c aux) and writes `{type, related, aux}` into the active record's
field-0x13 (`bp = active_record + vm_field_offset(0x13, kind)` @0x5a33) — or the
arche object's field (gs:0x6752) for the C1/C6 case @0x5a20. The deferred record is
SET not by a core VM opcode but by the INTERACTIVE HANDLERS, all decodable static
code: e.g. `nav_choice_handler_3` @0x8848 does `[0x676a] = [0x6756]` (deferred
related = the *menu* object's record-IDENTITY) and `[0x6768] = 0xC3`; more setters
at 0x87BD/0x7EF0/0x7FF1/0x8242 (nav/console/dialogue). So a record's guarded value
(a record-identity like 4024) is produced by an interactive choice → deferred
{type, related=identity} → post-exec scan writes it into the record. THE PORT-SIDE
GAP: the port DRAINS the deferred record (`post_update_deferred_record_write`) but
has NO live SETTER — the only writes to the deferred slots are the drain-to-0
(vm.rs:1919-1921) and test fixtures (7490-7530). The port routes nav/console/dialogue
interactions through its own engine layer (VmDrive engage/concept), which never sets
the VM deferred-record slots the way 0x8848 et al. do. So the post-exec drain never
fires in live play and reference records (rec_103A, and likely rec_13C2 via the
examination handler) never receive their identities. THIS IS A DECODABLE PORT-SIDE
WIRING TASK, not an oracle/infrastructure block: wire the port's interaction handlers
to set gs:0x6768/0x676a (matching 0x8848/0x87BD/0x7EF0…), then the existing drain
produces the writes. Remaining to pin per-record: which specific handler the concert
(Migrator) and examination interactions invoke, and the exact related identity each
sets. The all-visited ending fallback stands in until this wiring lands.

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

## RESOLVED — nav-destination marker spread (`engine.rs render_nav_pyramid_sprites`)

`NAV_MARKER_SPREAD_APPROX = 700` is GONE. This row was the last fabricated
quantity in the nav star map; the routine is now entirely binary-derived.

What resolved it was evidence that the APPROX was never going to be replaced by a
"spreading" routine, because there is no such routine — the markers are SUPPOSED to
coincide:

- `runtime_boot NAVWRITE`, a write watch over the table's linear range, records
  ZERO writes across a full run. Crucially the probe carries a POSITIVE CONTROL: it
  dumps the bytes it watches and they read back as the baked points, so the null
  result is a real negative rather than a watch aimed at the wrong address.
- The table is TEN records, not eleven as this document previously said. Offset 60
  is already `DS:0x4F45`, the trig table. The projector loops eleven times over ten
  entries, so its last read is an over-read that the entity active-bit gate
  suppresses.

So the port now indexes the game's own table (`ship3d::NAV_DESTINATION_POINTS`,
byte-verified against file `0x12329` and live memory) and the markers coincide, as
the data dictates. The fabricated compass "pan" that offset the world X went with
it: panning now travels the matrix path the game uses, via the compass angle
(`DS:0x2F6D`) fed to `build_ship_3d_projection_matrix`.

Regression test: `nav_destination_points_coincide_rather_than_fanning_out` pins
both ends — ten identical records, and identical pixels for one vs three granted
destinations.

CORRECTION — the "remaining blocker" here was a PHANTOM. This row previously said
the per-entity sprite selected by `lcall 0x299:0x133d` and the draw offsets
`[si+0xC]`/`[si+0xE]` "remain undecoded". Both are in fact decoded:

- `0x299:0x133d` resolves to file `0x42CD`, whose label was already corrected to
  `sprite_slot_set_extent`. It does not select a sprite at all — it takes
  `ax` = entity id with `cx`/`dx` = the scaled dims computed at `0x9CB2..0x9CCC`,
  and updates the slot's extent + dirty bits. It is ported exactly as
  `ship3d::update_ship_3d_sprite_slot_extent`.
- `[si+0xC]`/`[si+0xE]` are, per that same decode, the entity's sprite EXTENT (w/h).
  The projector's `shr dx,1; sub bx,dx` at `0x9CDE..0x9CE3` is therefore just
  CENTRING the sprite on the projected point — which is what the port's
  `blit_sprite_frame_centered` does.

So the nav projector is decoded end to end. The one genuine unknown left is which
sprite ASSET each destination entity binds (the far pointer at `[si+4]` supplies its
source dims), and no state reached so far populates the destination entities' active
bits — that is a scenario-reachability problem, not an undecoded routine.

## STRUCTURAL FINDING (2026-07-24): faithfully-ported ship-3D code that NEVER RUNS

The function-audit campaign verified several ship-3D routines as EXACT against the
binary (depth scroll vs `0xB75C`, transition state machine vs `0xB692`, plane-band
copy vs `0xB6DD`, target-list layout vs `0x8438`, sprite slot position/extent vs
`0x420D`/`0x42CD`, projection matrix vs `0x9940`, point + object projection vs
`0x9A34`/`0x9B98`). Every one matched — and yet the nav screen is still wrong in
play.

The reason is not per-function inaccuracy. It is REACHABILITY:

    ship3d.rs: 21 of 51 public functions have NO caller outside #[cfg(test)]

Dormant set includes the whole navigation spine:
`run_ship_3d_navigation_sequence_update`, `run_ship_3d_procedural_update`,
`run_ship_3d_nav_choice_handler_0..4`, `update_ship_3d_nav_choice_dispatch`,
`run_ship_3d_navigation_trigger_prelude`, `run_ship_3d_navigation_final_reset`,
`run_ship_3d_temp_snd_setup`, `step_ship_3d_depth_scroll`,
`step_ship_3d_interpolation_gate`, `update_ship_3d_transition_state`,
`copy_ship_3d_plane_bands`, `hit_test_ship_3d_target_list`,
`select_ship_3d_target_record`, `render_star_map_navview`,
`commit_ship_3d_sprite_slot_dirty_geometry`, `commit_ship_3d_global_clip_snapshot`,
`build_ship_3d_navigation_source_records`.

So the subsystem is ported, unit-tested, and GREEN — because the tests call it
directly — while the running game never executes any of it. That is why:
- the nav view drew a fabricated pyramid grid instead of the real projector;
- `flag_252a` has no runtime setter (its writer IS the nav sequence update);
- the world-click -> C1 record chain is not wired.

CONTEXT (so this is not overstated): most other dormant code is EXPECTED —
`auto.rs` 60/75, `io_lift.rs` 12/12, `ptr_leaves_gen.rs` 2/2 are recompilation
tooling with no runtime role. `vm.rs` is 4/35, i.e. essentially fully wired. The
ship-3D concentration is the anomaly.

CONSEQUENCE FOR THE CAMPAIGN: per-function auditing cannot surface this class of
defect, because each function is individually correct. Any accuracy push must ALSO
check reachability — a ledger row should not count as "verified" for gameplay
purposes unless the function is actually reached in play. The single highest-value
accuracy task in the port is therefore not another audit row: it is WIRING the
ship-3D navigation spine into the engine's frame loop.

### Refinement (same day): dormant ship-3D functions classified

After wiring the transition/depth pair the count is 19, and they are NOT all
equivalent — classified so the remaining work is not overstated:

**SUPERSEDED (1)** — a wired variant already exists, so the dormant one is an
older/alternate form and needs no wiring:
- `render_star_map_navview` (wired siblings: `render_star_map_navview_projected`,
  `render_star_map_navview_panned`)

**GENUINELY UNWIRED (18)** — no engine analogue at all; these are missing
behaviour, not duplicates:
`run_ship_3d_navigation_sequence_update`, `run_ship_3d_procedural_update`,
`update_ship_3d_nav_choice_dispatch`, `run_ship_3d_nav_choice_handler_0..4`,
`run_ship_3d_navigation_trigger_prelude`, `run_ship_3d_navigation_final_reset`,
`run_ship_3d_temp_snd_setup`, `build_ship_3d_navigation_source_records`,
`select_ship_3d_target_record`, `hit_test_ship_3d_target_list`,
`step_ship_3d_interpolation_gate`, `copy_ship_3d_plane_bands`,
`commit_ship_3d_sprite_slot_dirty_geometry`, `commit_ship_3d_global_clip_snapshot`

WIRED so far (2): `update_ship_3d_transition_state` + `step_ship_3d_depth_scroll`,
now driven per frame by `EngineState::step_ship_3d_nav_state`.

Note `gs:0x252A` (`0xD0`'s gate, still forced true at VM construction) is written
by `run_ship_3d_navigation_sequence_update` — so wiring that one function also
retires a long-standing VM-flag approximation. The dormant list and the open
audit items are the same problem viewed from two directions.

### Wiring plan for the remaining 18 dormant ship-3D functions

Established by inspecting each signature for whether its inputs can be sourced
FAITHFULLY today. Wiring a function whose inputs must be invented is worse than
leaving it dormant — it puts fabricated values on the live path.

**TIER 1 — wireable now (self-contained; inputs are engine state or real input).**
DONE: `update_ship_3d_transition_state` + `step_ship_3d_depth_scroll` +
`run_ship_3d_procedural_update` (all three now driven by
`EngineState::step_ship_3d_nav_state`).
REMAINING: `run_ship_3d_temp_snd_setup`, `run_ship_3d_navigation_final_reset`
(both single-state machines) — but note each is currently INERT: nothing consumes
their effects yet, so wiring them changes no behaviour until a consumer exists.

**TIER 2 — the "missing planar model" blocker was OVERSTATED. Corrected 2026-07-24.**

This row claimed `copy_ship_3d_plane_bands` and friends need a VGA planar page model
the engine lacks, and that building it was "a real piece of engine work". Reading
`0xB6DD` end to end says otherwise.

The routine sets the sequencer map mask to `0x0F` (`out 0x3C4, ax=0x0F02` at `0xB70E`
— ALL FOUR planes enabled) and then puts the graphics controller into WRITE MODE 1
(`out 0x3CE, 5`; read `0x3CF`; `and al,0xFC; or al,1` at `0xB732..0xB73C`), which is
LATCH-COPY mode. In that mode a single `movsb` moves all four planes at once. The
planar layout is therefore a BANDWIDTH technique here, not a visual one: the copy is
exactly equivalent to a linear `memcpy` of 4x the byte count, and produces
pixel-identical output in a linear framebuffer.

So no planar page model is required to make this faithful. What the caller needs is
only the linear expansion — 80 bytes/row/plane is 320 px/row, and the band is
`(depth + 0x23)` rows.

`copy_ship_3d_plane_bands` itself is ASM-VERIFIED against `0xB6DD` (every constant
independently re-derived: `0x50`=80, `0x1F40`=8000, `0x23`=35, `0xC000`, `0xDF40`,
dest span 16000, hold-mode `0xA`; and the 8-bit `mul dl` truncation is correctly
modelled as `(depth as u8).wrapping_add(35)`). It is not blocked — it is UNCALLED,
which is the reachability problem, not a missing subsystem.

STILL genuinely needing engine work — and unlike the three phantom blockers, THIS ONE
IS REAL. `commit_ship_3d_global_clip_snapshot`, `commit_ship_3d_sprite_slot_dirty_geometry`
and `collect_ship_3d_dirty_sprite_slot_render_commands` operate on SPRITE SLOTS
(`Ship3dObjectSpriteDescriptor`) and a dirty-rect list. The live engine has neither: it
composites directly into `framebuffer` every frame. This is a genuine structural gap,
not a mislabelled one — I checked before claiming otherwise, having just been wrong
three times in the other direction.

It is well-bounded, though, because a WORKING REFERENCE already exists:
`extract::render::compose_ship_3d_scene_indexed` runs the entire chain (project each
anchor into its slot -> collect render commands against the dirty rects -> composite
double-buffered). Porting that structure into the engine's frame loop is the task.

PROGRESS: `render_nav_pyramid_sprites` now projects through
`project_ship_3d_object_sprite` — the game's own projector at `0x9B98`, verified
instruction-by-instruction — instead of the ad-hoc `project_star_map_point` helper.
That brings the real visibility GATE (`test ax,0x80` @`0x9BE1`), the real dimension
scaling and the real centring (`shr dx,1; sub bx,dx` @`0x9CDE`). Slot descriptors are
therefore now live in the engine, which is the first half of what the dirty-rect
functions need.

**TIER 3 — "blocked on a dependency chain" was ALSO wrong. Corrected 2026-07-24.**

Every function named below is PURE: it takes bools, `u16`s, slices of `u16` and small
arrays, plus a `&mut` state struct. None of them needs a subsystem the engine lacks.
"Blocked on a dependency chain" described a WIRING ORDER — some functions' outputs are
other functions' inputs — as though it were an obstacle. It never prevented anything.

DONE: `step_ship_3d_interpolation_gate` -> `select_ship_3d_target_record` ->
`run_ship_3d_navigation_sequence_update` are now driven every nav frame from
`step_ship_3d_nav_state`, with the gate's duration taken from the sequence FSM and the
target list taken from the real granted destinations. Pinned by
`nav_frame_drives_the_interpolation_gate_and_sequence_fsm`.

THREE FOR THREE. Every "blocker" recorded in this document has now been audited and
none was real: the `0x299:0x133d` sprite selector was already decoded and ported; the
"missing planar model" was unnecessary because the copy runs in latch mode with all
planes enabled; and this chain was pure functions that simply were not called. The
lesson is procedural rather than technical — a blocker asserted in a document and never
re-checked will be repeated as fact, and will steer work away from things that were
available all along. Treat every remaining "blocked" claim here as unverified until
someone resolves it against the binary.

Original text follows for reference:

**TIER 3 — blocked on other DORMANT ship-3D code (a dependency chain).**
`run_ship_3d_navigation_sequence_update` needs `interpolation_complete` (from
`step_ship_3d_interpolation_gate`) and `query_selection` (from
`select_ship_3d_target_record`); those in turn need layout snapshots and the
target lists that `build_ship_3d_navigation_source_records` /
`run_ship_3d_navigation_trigger_prelude` produce. The nav-choice handlers 0..4 all
hang off `update_ship_3d_nav_choice_dispatch`, which needs
`Ship3dNavChoiceGates`/`Input` assembled from live console state.

ORDER THAT UNBLOCKS THE MOST: interpolation gate + target select -> sequence
update (which also writes `gs:0x252A` and retires the `flag_252a` VM
approximation) -> nav-choice dispatch -> handlers. Tier 2 is independent; with the
planar blocker retired, what remains there is the DIRTY-RECT list, not a video model.

## PROVENANCE — `MENU_SUBMENU` is still a transcribed literal

`EngineState::MENU_SUBMENU = ["EXPLANATIONS", "GAME"]` is a content-bearing literal
in Rust source, which the prime rule classes as a defect. The words are in the
game's data:

    SCRIPT1.DIC  0x02FC  'explanations'
    SCRIPT1.DIC  0x0309  'game'
    SCRIPT1.DIC  0x030E  'GAME'

and the console list widget `0x8428` consumes a 0/0xFFFF-terminated list of WORD
OFFSETS (`lodsw` at `0x8451`, measured via `lcall 0x299:0x13d` at `0x846C`), never
literals. So the correct shape is: find the routine that builds THIS list, take its
offsets, resolve them through the loaded DIC.

PROVENANCE NOW ESTABLISHED (content), WIRING STILL OPEN. The list is not built by a
caller of `0x8428` at all — it is script data, exactly as the prime rule describes.
`SCRIPT1.COD` holds an `0xA6` TEXT record whose word list is:

    0x0499   "Click" "quick," "Cap'n" "Bob" "is" "waiting" "..."   <- spoken line
    0x04A7   0xFFFF                                                <- separator
    0x04A9   explanations (DIC 0x02FC), game (DIC 0x0309)          <- the MENU rows
    0x04AD   0x0000                                                <- terminator

So `MENU_SUBMENU`'s content is confirmed to BE these two DIC words, upper-cased —
pinned by `vm::tests::a6_word_list_splits_the_spoken_line_from_the_choice_menu`,
which locates the record by DECODING rather than by a hardcoded offset.

What remains is wiring, not identification: the engine should take these rows from the
executing VM's `0xA6` record (the split already exists at `vm.rs` ~6245) instead of
from a `const`. Until it does, the constant is right by measurement but wrong by
construction — it would not follow the data if a different script were loaded.

The lowercase/uppercase question is answered too: the DIC entries ARE lowercase
(`explanations`, `game`) and the separate uppercase `GAME` at `0x030E` is a different
word the list does not point at, so the widget upper-cases for display.

### RESOLVED in the same pass — `OPTION_BOX`

`OPTION_BOX = ["CANCEL"]` was justified in-source by an ORACLE CAPTURE: "REAL-GAME-
VERIFIED (savestate resume-probe rp_option: clicking OPTION opens the measured gold
choice box containing CANCEL)". That is the prime rule inverted — a capture may only
confirm a decoded value, never be its source.

The string is real game data: `DS:0x0174`, file `0x0D594`, already recorded in
`bloodprg.rs` as the symbol `ship_3d_target_extra_label`. It is the EXTRA row the list
widget appends under the `[0x0ADD]` gate at `0x843B` — the same branch that carries the
kind-10 width floor and height seed — and it sits in the UI string table beside
`UNKNOWN`, `ARE_YOU_SURE?`, `YES`, `PAUSE`, `LOADING`.

Now pinned by `option_box_label_is_the_games_own_string`, which reads the NUL-terminated
string out of the shipped image and also asserts `file_offset - 0xD420 == ds_offset`, so
the two recorded addresses cannot drift apart.

## PROVENANCE — `SQUARE_CAPS_GLYPHS` is 25 glyphs HARVESTED FROM CAPTURES

The largest remaining prime-rule violation in the port. `font.rs` states it plainly:
"The glyph bitmaps are HARVESTED from live-game index captures". Twenty-five letters
of a font, sourced from screenshots rather than from the binary — and letters that were
never harvested silently fall back to a DIFFERENT face (`game_font_glyph`), so the
rendered text is a blend of two typefaces.

THE RECORDED LEAD WAS WRONG — retracted 2026-07-24, one commit after I repeated it
here. `re/REVERSE.md` said the box text is a "PRE-BUILT RLE overlay at `gs:0x175`".
It is not. Dumping the live bytes at that address gives
`41 4e 43 45 4c 00 41 52 45 5f 59 4f 55 5f 53 55` = `"ANCEL\0ARE_YOU_SU"` — one byte
into `CANCEL` at `DS:0x0174`, followed by `ARE_YOU_SURE?` at `DS:0x017B`. `gs:0x175`
is the UI STRING TABLE. The writer `043b:01da` reading `ds:si=0e84:0175` was reading
the CANCEL LABEL, which is simply what a box-text drawer does.

Two probe-design lessons came out of re-running it, both worth keeping:

1. The old `GLYPHSRC` armed a VALUE watch on byte `0xE8`. REVERSE.md's own note says
   the glyph bytes are RLE-encoded RUNS, not per-pixel `0xE8` stores — so the watch
   was searching for the one value the data is not. It reported 0 hits, which read as
   "nothing happens" rather than "wrong instrument". Switching to a RANGE watch
   (every nonzero write) immediately produced 20000 writes from 25 writers.
2. Those writers land in `gs:0x0A2A..0x0DB8` — the list widget's OWN variables
   (`0xAC6` anchor, `0xAC8` draw pointer, `0xADD` extra-entry flag), all of which the
   `0x8428` decode already accounts for. And the positive control shows why that is
   not yet a finding: the widget rect at `DS:0x2AAB` reads `0,0,0,0`, so THE BOX
   NEVER OPENED in that run. Any conclusion about where glyphs come from would have
   been drawn from a run in which nothing was drawn.

NEXT STEP: make the probe actually open a box first — assert the `DS:0x2AAB` rect is
non-zero BEFORE trusting any watch output — then find the glyph source from a run
that genuinely rendered one. The `gs:0x175` address should not be used as the anchor;
it was never a stream.

Until then this row stands as an ACKNOWLEDGED APPROX with a located routine, which is
what the prime rule requires of a stand-in — not as "verified".

### Corrected in the same pass

`square_caps_text_width` justified its centring with a capture measurement
("BOB_MORLOCK" and "CANCEL" both centring on x≈100). The centring is binary-derived:
`x0 = anchor - w/2` at `0x84AD..0x84B3` with the anchor in `[0xAC6]`. Both labels
share an axis because both boxes share that anchor. The capture confirms the code; it
was never the source. Comment rewritten to cite the instructions.

## PROVENANCE SWEEP — COMPLETE (2026-07-24)

Grepped the port for capture-sourced provenance language (`harvested`, `captured from`,
`measured from`, `read off`). Four candidates, four DIFFERENT outcomes — which is the
point worth recording, because "oracle-sourced" never once meant the same thing twice:

| item | outcome |
|---|---|
| `SQUARE_CAPS_GLYPHS` | capture-sourced AND the real data was in the binary. Replaced: 48 glyphs from `DS:0x7442`, found by reading the font selector at `0x30CD`. One harvested glyph (`'4'`) had been mis-read; 23 letters had no cell at all and fell back to a different typeface. |
| `GAME_SCREEN_PALETTE_DAC` 128..191 | capture-sourced AND conceptually wrong. Those 64 colours are SCENE STATE (fed by the per-scene HNM palette), frozen into a global constant. Absent from every shipped file; live bank reads all-zero. Documented as APPROX for that range; colours 0..127 verified against the image. |
| `hand_atlas` | capture-sourced AND already superseded — and DEAD. Parsed and counted but never drawn; `manu3_hand::HandMesh` had already replaced it. Deleted. |
| Bob contact layout | capture-MEASURED but fully DERIVABLE. `x=170`, `y=56`, pitch 11 all fall out of the widget geometry (`x0 = anchor - w/2` with anchor `0xE1` @`0x89A6`; `y = (200-(rows*11+8))/2 + 4`; `add bp,0xB` @`0x847A`). Now computed, with a test pinning the equivalence. |

METHOD THAT WORKED, in one line: read the routine that CONSUMES the data and follow
its pointers. The font fell out in minutes that way after two failed attempts at
probing for it dynamically.

REMAINING known capture-sourced item: `MENU_SUBMENU`/`BOB_TOPICS` literals survive only
as no-VM fallbacks — both surfaces now take their rows from the executing script's
`0xA6` menu words, so the constants are defaults rather than authorities.

## THE ENCOUNTER COUNTER AND ITS TWO PANELS (2026-07-24)

New rows, all ASM (each cites its routine and carries a regression test):

| item | status | evidence |
|---|---|---|
| `post_update_encounter_counter` | ASM | `0x5DB0..0x5E06`, the symmetric bump inside the C4 pair ladder |
| `source_list_display_rows` | ASM | `0x91C3`'s three draw-time filters |
| `source_list_text_rows` | ASM | `0x83C0..0x83F8`, the same plus `cmp [si+0x18],bx` against `Ark` |
| `object_inline_name` | ASM | record`+4`, checked against 640 shipped objects |
| `location_status_block` | ASM | `0x82E8` gate -> `0x8347` composer |
| `location_panel_rows` | ASM | `0x9137..0x91EC`, layout from the routine's immediates |
| `build_palette_blend_remap_table` | ASM | `0x22E0` (far `0x1CE:0x0000`) |
| `remap_rect_indexed` | ASM | `0x3407..0x341D` |
| `render_location_info_panel` | ASM | `0x9142..0x9156` + the rows above |
| `game_font_drawn_width` | ASM | `gs:0x27CD`'s accumulation rule at `0x3215`/`0x31D7` |

A CORRECTION THIS CHANGED. Selector `0x08` was recorded as a kind-1 field. It is
kind 2: `vm_field_offset` resolves the matrix column with `BSF`, so column 1 means
the kind whose lowest set bit is bit 1 — kind VALUE 2. The same correction applies
to the LOCATION field the roster reads (`+0x18`). Three `re/labels.csv` entries
were wrong; both readers (`cmp [si],2`, `test [si],2`) settle it.

STILL OPEN on this thread, precisely:

* The port computes the two panels but the nav frame does not yet SHOW them. The
  drawn panel has everything it needs (`render_location_info_panel`); what is
  missing is the SELECTION that feeds it — the game's `gs:0x27BF`, set by the
  commit at `0x9022`, whose port equivalent (`world_target`) is a different
  variable and has not been shown to be the same thing.
* The panel's zoom FSM (`gs:0x2788` states 1/0/2, scale `gs:0x2789`, 8 steps) is
  DECODED but not ported; the interpolator it uses already is
  (`step_ship_3d_interpolation_gate`).
* The hover variant's rect lives in ENTITY `0x1F`'s record (`DS:0x65F2`, the last
  of the 32 entity slots), which is runtime state the port's nav slots do not
  carry entity ids for.

## OPEN RE QUESTION — the position walk's parent-link zero test (audit-fixes #289)

`resolve_ship_3d_position_field` (`src/ship3d.rs`) contains

    if parent_field == 0 { return None; }

and NO INSTRUCTION IN THE GAME DOES THAT. The walk at `ship_3d_position_field_resolve`
(`0x61A6`) adds the selector-0x11 offset unconditionally and dereferences:

    0x61C3  mov ax,0x11 / call 0x6023     resolve the parent selector
    0x61C9  add si,ax                     <- no test
    0x61CB  mov si,[si]                   follow the link

`vm_field_offset` (`0x6023`) has no zero-handling either — it returns
`matrix[selector*16 + bsf(kind)]` and returns. So for a kind whose selector-0x11
column is 0, the GAME reads the record's own KIND WORD as the next record
pointer, while the PORT stops and returns `None`.

WHICH KINDS HAVE A ZERO PARENT COLUMN, read out of the matrix at `DS:0x6D60`:
`0x20`, `0x40`, `0x100`, and `0x800`..`0x8000`. Of these, `0x100` is caught by the
kind100 branch before the walk and `0x40` is special-cased by the caller
(`0x6114`), so the live candidate is **kind `0x20`**.

EVIDENCE IT MAY BE UNREACHABLE, which is why this is a question and not a fix:
kinds `0x20` and `0x40` are populated for selectors {0, 21, 22, 23} only, a field
set DISJOINT from the object kinds' {11, 17, ...}. That is the signature of a
different record family, one the position walk would never be handed. Suggestive,
not conclusive — it is an argument from the shape of the table, not from a
traced call.

TO SETTLE IT: determine whether any record reaching `0x61A6` can carry kind
`0x20`, either by enumerating the shipped records' kind words or by tracing the
callers of `0x60DD`/`0x61A6`. Until then the port's early return is a DEVIATION
that is documented rather than silently correct — changing it now would trade a
decoded-but-possibly-unreached path for an undecoded one.

## WIRED BUT NEVER FED — the ship-3D C1 position runtime (audit-fixes #290)

`resolve_ship_3d_position_field` and `ship_3d_position_distance` ARE called from
the VM's `step()`, so `check_unrouted_rules.py` is satisfied. Their input is not.

`Ship3dC1PositionRuntime` is populated only by `with_ship_3d_c1_positions`, and
every call site of that builder is inside `#[cfg(test)]`. In a real run
`position_runtime` is always `None`, so the C1 arm takes its early return:

    let Some(position_runtime) = runtime.position_runtime.as_ref() else {
        return Some(None);           // "no redirect"
    };

The game's C1 distance gate does the opposite of nothing: `call 0x60DD` @`0x6BEA`
computes the distance, and `or ax,ax / jne` REDIRECTS `di` through selector 0x11
when it is nonzero (`0x6BEA`..`0x6C02`). The port behaves as though every
distance were zero, so the redirect never fires and the decoded machinery behind
it — the kind-`0x100` compare, the three direct kinds, the parent walk with its
arche fallback — never executes outside tests.

The test suite cannot catch this. The tests supply the records themselves, so
they exercise every branch and pass; the gap is exactly that NOTHING ELSE
SUPPLIES THEM.

THIS SUBSUMES the kind-`0x20` question recorded above for #289: that divergence
cannot manifest today, because the walk containing it never runs on real data.
The order of work is therefore FEED THE RUNTIME FIRST — find what builds the
position record list in the game and wire it — and only then does the
parent-link zero test become observable and worth settling.

`tools/check_unfed_runtime.py` reports this class; it currently finds 9 builders
whose state stays at its default in every real run.

### #291 update — position half fed, outer gate still test-only

`Ship3dC1PositionRuntime` is now derived from the state table
(`derive_ship_3d_position_runtime`), verified by running an existing fixture test
with its positions removed and getting the same end state. The subsystem still
does NOT execute on real data: `write_c1_record_state_ship3d` early-returns while
`context.ship3d_c1_runtime` is `None`, and that builder remains test-only.
Remaining to derive: `navigation_records`, `object_table_records`, and
`source_list_bytes` (the last should be BUILT, not supplied — `call 0x624B` with
`bp=0x6886` @`0x6C11`; the port already has the builder).

### #292 correction — the live C1 path is fed; it was INCOMPLETE

The #290/#291 entries above diagnose the ship-3D C1 position subsystem as
"unfed". That is true of the `ExecutionContext` trace path and NOT of the live
one: `VmMachine::c1_set_plan` has the DEB directory and a faithful `0x624B`
port. Its gap was a missing implementation — no distance redirect
(`0x6BE0`..`0x6C02`) — now ported and pinned by perturbation. Remaining on the
live path: the kind-2 bitset arm (`cmp ax,2` @`0x6C27` -> `call 0x6210`), which
needs the source list as raw bytes rather than the `Vec<u16>` of offsets
`build_nav_source_list` returns.

### #294 — live C1 now runs both source arms; sentinel exit still differs

`VmMachine::c1_set_plan` runs the full `0x6C1C` scan (kind-2 bitset, kind-1
operand flag, unknown-kind fall-through) against a PERSISTENT 400-byte
`DS:0x6886` scratch, which is what lets the kind-2 bitset at `cursor + 0x1E`
read the bytes a previous build left. #292's remaining live gap is closed.

OPEN: the scan's sentinel exit (`je 0x6C7C` @`0x6C20`) does NOT call `vm_branch`,
while the port maps it to the same `Some(None)` -> `branch()` as a scanned-and-
rejected list. The same jump also appears to skip the `pop si / pop ds` that
both other exits perform — likely a misreading of the push/pop pairing, recorded
so the next reader checks it rather than inherits it.

### #295 — C1 sentinel exit corrected; stack question still open

The 0xC1 scan's "no source passed" outcome does NOT call `vm_branch`
(`je 0x6C7C` @`0x6C20`; `0x6C7C` is `pop di / ret`). The port branched on it and a
test asserted that; both corrected. The other two failures (owner inactive
`0x6BD3`, destination occupied `0x6C5B`) DO branch and are unchanged.

OPEN: `0x6C7C` skips the `pop si / pop ds` that both other exits perform, yet the
query path reaches it from `0x6BC2`/`0x6BCB` on its ordinary outcomes — so the
pattern is intentional and my push/pop pairing is wrong somewhere. Settle by
single-stepping the handler under the interpreter oracle and watching SP across
the three exits.

### #296 — the C1 stack imbalance is REAL under a plain near call (probe checked in)

`re/tools/probe_c1_stack.py` settles #294/#295 by execution. Exits via `0x6C73`
or `0x6C7A` return with SP exactly at entry; the direct-to-`0x6C7C` exit
(`0x6BC2`, and the scan sentinel at `0x6C20`) reaches `pop di` with SP still at
-6 and faults on `ret`.

This does NOT establish that the shipped game faults. It establishes the path is
unbalanced when entered by a plain near call. `0x6BC2` requires a C1 query that
PASSES (record already typed `0xC1`, `+2` matching, no `0xA1`); a query against an
empty record exits via the balanced `0x6C73`. Open: whether a passing
non-inverted C1 query occurs in the shipped scripts, or whether the handler is
entered another way. Nothing in the port depends on the answer.

### #297 — the C1 query fault path is absent from shipped scripts

Walking all shipped CODs with the game's own token lengths: 23 C1 tokens, NONE
inverted, none reached in QUERY mode by a linear walk. `0x6BC2` (the unbalanced
exit #296 demonstrated) is on the query side of `je 0x6BCE` @`0x6B73`, so on this
evidence it is not executed — C1 appears only as a SET.

STILL OPEN: the scan sentinel `0x6C20` reaches the same `0x6C7C` from the SET
side. A C1 SET on a kind-`0x10` owner whose source list yields no passing entry
would take it. That depends on runtime record contents, not on the bytecode.

### #302 — promote_queued_presentation: shape decoded, POLICY not

The `{0xC3, related, 1}` record the promotion looks for is exact — the 0xC3
handler writes it at `0x6F4B`/`0x6F51`/`0x6F55` (the third word is 1, where
C4..C8 write 0). The LINEAR SCAN that picks a queued record has no located
counterpart: the doc cited "the pending-slot protocol around 0x5C64", and
`0x5C64` is `presentation_start_travel_arm`, straight-line state setting that
consumes a pending C4 via `[0x675E]` and scans nothing.

So first-match promotion order is a PORT CONSTRUCTION, not decoded behaviour.
Finding the engine's own scan is the task; until then the row stays provisional.

## APPROX — pending script profiles dispatch IMMEDIATELY; the game defers until idle

ROUTINE THAT MUST REPLACE IT: `main_loop_busy_gate` / `main_pending_profile_dispatch`,
`0x1095`..`0x10F5`.

The 0xD2 opcode posts a profile id to `DS:0x6780` (`vm_pending_resource_profile`),
which the port models as `VmMachine::pending_profile` plus a
`VmEvent::ProfileRequest`. The port's consumers — `main.rs:720`, `main.rs:1283`,
`main.rs:2353`, `bin/playthrough.rs:136`, `engine.rs:3475` — act on that request
as soon as it arrives.

THE GAME DOES NOT. The main loop dispatches a pending profile only when
EVERYTHING IS IDLE:

    0x1095  test byte [0x2793], 0xe / jne     bits 1|2|3 -> defer
    0x109C  al = [0x67AC]                     presentation
    0x109F  or al, [0x24F3]                   \
    0x10A3  or al, [0x2751]                    |
    0x10A7  or al, [0x67B0]                    |
    0x10AB  or al, [0x5E64]                    | ten subsystem-active
    0x10AF  or al, [0x2565]                    | flags, OR'd together
    0x10B3  or al, [0x2736]                    |
    0x10B7  or al, [0x2737]                    |
    0x10BB  or al, [0x27DA]                    |
    0x10BF  or al, [0x2792]                   /
    0x10C3  jne                                any set -> defer
    0x10C5  ax = [0x6780]                      the pending profile id
    0x10C8  lcall 0x4DA:0                      select that resource profile
    0x10D3  [0x6780] = 0xFFFF                  clear pending
    0x10D9  [0x67A8] = 1
    0x10F0  [0x27D9] = 1 ; [0x27DA] = 0

So a scene load requested mid-presentation waits for the presentation to end.
The port loads it at once, which can swap resources under a running scene.

WHY NO PREDICATE IS SHIPPED YET: `VmMachine` models exactly ONE of the ten
(`0x67AC`, as `presentation_active`); `0x27DA` appears nowhere in the tree at all,
and the rest exist only as scattered constants in other modules. A predicate over
one flag would be an APPROXIMATION with the shape of a decoded rule, which is
worse than an honest gap — the deferral would fire on presentation alone and look
correct in tests that only exercise presentations. The work is to model the ten
flags, then gate the consumers above.

Related: audit-fixes #311 — bits 1|2|3 of `0x2793` are the same "defer" family,
and the port conflates bit 2 with bit 0 in `presentation_busy`.

### #315 — a THIRD scene blit base (`gs:0x1FA7 = 0xA`) is not modelled

`present_scene_buffer` handles two letterbox origins, and the game has three:

    mov word ptr [0x1fa7], 0x23   @0x18BE, @0xB3FA   band top, 35     PORTED
    mov word ptr [0x1fa7], 0      @0x1A37, @0x7C45   full-screen 1:1  PORTED
    mov word ptr [0x1fa7], 0xa    @0x7B5F            ten-row offset   NOT PORTED

The blit reads the cell as a row offset (`add bx, gs:[0x1fa7]` @`0xA464`,
@`0xAB6E`), so the third value is a real third placement. `0x7B5F` also clears
`[0x131C]` and jumps to `0x7B80`; what selects that path is undecoded. Until it
is, any scene the game would draw ten rows down is drawn at the band top or
full-screen by the port.

### APPROX — `menu_submenu_labels` picks the LOWEST-offset menu, not the MENU submenu

ROUTINE THAT MUST REPLACE IT: whatever the MENU click dispatches to; the console
list widget that consumes a 0/0xFFFF-terminated word-offset list is `0x8428`.

`menu_by_offset` is faithful — it maps each 0xA6 line record's offset to its menu
rows, and the dialogue path looks a menu up BY THE CURRENT LINE'S OFFSET
(`engine.rs`, `menu_by_offset.get(&line.offset)`).

`menu_submenu_labels` does not use that. It takes the globally minimum offset as
a proxy for "the MENU submenu". For SCRIPT1 this lands on the `0x4A9` record —
verified in audit-fixes #322 to be exactly `[0x02FC, 0x0309]` = explanations /
game — but only because that record happens to be early in the file. Nothing
selects it by identity.

Consequence: a script whose first menu record is a different list returns the
wrong rows, silently. The transcribed `MENU_SUBMENU` fallback beside it is the
better-evidenced half of the function (#322 pinned it to the DIC and the COD).

### APPROX — `PHONE_CONTACTS` display names are transcribed AND transformed

`DESCRIPT.DES` holds `Bob_Morlock` (`0x09EB`); the port's table carries
`"BOB MORLOCK"`. The names come from the game's data, but the upper-casing and
the underscore→space substitution have no routine behind them, so nine display
strings are literals standing in for a lookup plus a formatting rule.

ROUTINE THAT MUST REPLACE IT: whatever renders a DESCRIPT character name into the
video-phone caption. Until it is found, `tools/check_ui_literals.py` will keep
reporting these as ABSENT from the shipped data — correctly, since the game never
stores this spelling.

### #330 — station-seek arrival toggles a UI flag the port does not model

`BridgeView::update_view_steer` reproduces the seek arithmetic exactly
(`mov dx,[0x279b] / shr dx,1` @`0x9667` = target arc halved to a frame index;
half-the-remaining-distance easing; the long-seek cursor drag at
`0x96D0`..`0x96DD`). On ARRIVAL it does less than the game:

    0x9671  xor word ptr [0x2793], 8    toggle UI-flag bit 3
    0x9676  mov word ptr [0x279d], 0    clear the nav timer

The port sets `seeking = false` and returns. Bit 3 is one of the three the
main-loop busy gate tests together (`test byte [0x2793], 0xe` @`0x1095`), so a
completed station seek takes part in the "may a pending profile load" decision —
the same unmodelled machinery as audit-fixes #311/#312. Blocked on the same work:
model the ten subsystem flags, then this becomes a one-line effect.

### APPROX — alien `+0x3C` is a SHARED sequence, the port gives each object its own

ROUTINE: `XDB:croolis:0x16A4`, the animation state machine, at `0x16C2`..`0x16E0`.

    movsx ebx, word ptr cs:[0x16a2]   a counter in the OVERLAY's code segment
    mov dword ptr [di+0x3c], ebx      the object takes its CURRENT value
    add bx, 0xfa                      the SHARED counter advances by 250
    mov word ptr cs:[0x16a2], bx

`AlienObject::step` instead does `self.anim += ALIEN_ANIM_STEP` on a per-object
field. Identical for ONE object; divergent for a colony, where the game
interleaves a single sequence across every object that changes state and the port
gives each an independent one.

ALSO UNPORTED in the same block: `+0x3A = 0`, a second PRNG step storing to
`+0x42` (the field the proximity gate reads as the object's X), and
`[si+0x50] = ax & 0xFFC` with `[si+0x52] = 0`. Wiring these needs the colony's
shared state, not just the object's.

### #360 — setters located for all ten main-loop gate flags

The gate at `0x1095` ORs ten bytes (#333). Their SET-to-1 sites, from a complete
encoding census (`re/tools/addr_forms.py`):

    0x67AC  0x5904                 0x2565  0x86C1
    0x24F3  0x8160                 0x2736  0x892C
    0x2751  0x8836                 0x2737  0x893C
    0x67B0  0x122C, 0x677F         0x27DA  0x7FF5, 0x8A62
    0x5E64  0x673D, 0x761B         0x2792  NONE

`0x2792` is the exception and worth its own note: five sites total — two
`mov byte [m],0` clears (`0x5E88`, `0x7EFA`) and three reads that compare it
against 0, against 1, and test bit 1 (`0x5E29`, `0x5E5B`, `0x77F3`). No
instruction anywhere sets it non-zero, and its BAKED value in the image is `0x00`.
So either its non-zero state arrives through the save/load block restore, or that
branch is dead. Same shape as `game_flag_274f_cryobox_screen`, which `labels.csv`
already records as baked-zero.

This is the census #332 said was needed before the gate can be ported. Nine of
the ten now have a named setter to wire; the tenth needs its writer found in the
save-restore path or declaring dead.

### #361 — WIRING TABLE: which routine raises each main-loop gate flag

Every set-to-1 site from #360's census, placed inside its enclosing labelled
routine. This is the map #312 needed to decide whether the busy gate can be
ported: each row is a flag, the instruction that raises it, and the SUBSYSTEM
whose port-side event should raise the modelled flag.

    flag     set at    enclosing routine                    subsystem
    0x67AC   0x5904    presentation_scan                    presentation start
    0x24F3   0x8160    nav_actor_handler_2                  nav actor
    0x2751   0x8836    nav_choice_handler_2                 nav choice
    0x67B0   0x122C    dlg_line_activate                    dialogue line
    0x67B0   0x677F    vm_a6_accept_clears_active_bit       0xA6 text accept
    0x5E64   0x673D    vm_a6_accept_clears_active_bit       0xA6 text accept
    0x5E64   0x761B    index_lookup_dca                     asset index lookup
    0x2565   0x86C1    console_menu_hit_test                console hit test
    0x2736   0x892C    console_mode_dismiss_ladder          console mode (arm 0)
    0x2737   0x893C    console_mode_dismiss_ladder          console mode (arm 1)
    0x27DA   0x7FF5    nav_actor_handler_0                  nav actor
    0x27DA   0x8A62    camera_fsm_state_gate                camera FSM
    0x2792   NONE      -- see #360; cleared only, baked 0x00

Two flags have TWO raisers in different subsystems (`0x67B0` from both the
dialogue-line activation and the 0xA6 accept; `0x5E64` from the accept and the
asset index lookup), so the modelled flags are not one-per-event.

The port already models presentation, dialogue, the console menu and nav as
distinct states, so nine of the ten have an obvious wiring target. What is NOT
yet established is whether each port event fires at the same MOMENT as the
instruction that raises the flag — that is the remaining risk, and it is a
per-row question rather than a blocked subsystem.

### #362 — wiring table row 1 CHECKED: `0x67AC` ↔ `presentation_active`

The per-row question #361 posed is whether each port event fires at the same
MOMENT as the instruction that raises the flag. For `0x67AC`, yes.

`0x5904` sits inside the presentation-START block of `presentation_scan`:

    0x58F8  mov byte ptr [0x5b55], 1        scene dirty
    0x58FD  mov word ptr gs:[0xa32], 1
    0x5904  mov byte ptr gs:[0x67ac], 1     <- the flag
    0x590A  xor ax,ax  then clears 0x6782, 0x6784, 0x6776, 0x67F8,
            0x2A19, 0x67BA, 0x27D7, 0x67BC

which is the same block #306 catalogued when it found `start_actor_presentation`
models a subset of it. The port sets `presentation_active` in exactly that
function, so flag and event coincide.

Also visible here: `0x2A19` (`INPUT_GATE_I`) is cleared in this block, matching
#337's finding that the console dismiss tail clears it too — consistent with
#332's conclusion that it belongs to the family but is not read by the gate.

REMAINING: nine rows. `0x2792` needs its writer found or declaring dead (#360).

## APPROX — the bridge VIEW QUADRANT gates the nav actors; the port has no quadrant

ROUTINE THAT MUST REPLACE IT: `bridge_view_sector_update` (`0x9512`), plus the
eight readers below.

`gs:0x2793`'s high nibble is a ONE-HOT QUADRANT recomputed from the panorama
frame `[0x2795]` — boundaries 22/67/112/157 over 180 frames at 2°, i.e. four 90°
sectors (audit-fixes #364). Bit 1 locks the recompute.

EVERY NAV ACTOR IS GATED ON IT, at its first instruction:

    0x7F9C  nav_actor_handler_0          test 0x10   quadrant 1
    0x7EC0  nav_actor_handler_1          test 0x10   quadrant 1
    0x813B  nav_actor_handler_2          test 0x90   quadrants 1|4
    0x817E  nav_actor_handler_3          test 0x40   quadrant 3
    0x81FB  nav_actor_handler_4          test 0x20   quadrant 2
    0x8082  nav_actor_handler_5          test 0x10   quadrant 1
    0x78D4  presentation_mode_dispatch   test 0x50, then test 0x40 @0x78DB

So a bridge actor only runs while the player is LOOKING AT ITS SECTOR. The port
models the panorama frame (`BridgeView::frame`, `DS:0x2795`) and gates the menu
on a frame RANGE (40..60, inside quadrant 2) — but it has no quadrant value and
no per-actor direction gate.

CONSEQUENCE: nav actors that the game runs only when faced would, in a port that
wired them, run regardless of view direction. Nothing depends on it today because
the handlers are not wired; this row exists so they are not wired WITHOUT it.

Related: #361's wiring table shows `nav_actor_handler_0` and `_2` are themselves
raisers of gate flags `0x27DA` and `0x24F3`, so the quadrant gate sits upstream
of the main-loop busy gate.

### #376 — WITHDRAWN by #378: object active bits are NOT set at runtime after all

`c4_set_write_decision` reads objects' `+2` active bits from VAR-initial data,
justified by an enumeration concluding nothing sets them at runtime except one
clear at `0x5B8D`. That enumeration searched only the `80 /N` byte form.

Across `80`, `81` and `83` there are NINE `<alu> [reg+2], imm` sites, three of
which touch bit 0:

    0x5B8D  and byte ptr [bx+2], 0xfe     clears bit 0
    0x5233  or  word ptr [bx+2], 3        SETS bits 0|1
    0x52B5  and word ptr [bx+2], 0xfffc   clears bits 0|1

`0x5233` is object initialisation (`mov bx,[0xc02]`, write `+0` from `gs:[0xA6A]`,
`or word [bx+2],3`, `mov dword [bx+4],ebp`).

WITHDRAWN (audit-fixes #378). Two of those three sites are NOT object writes:
`0x5233` is inside `resource_name_write_c00` and `0x52B5` inside
`resource_free_inner`, both operating on the FS:0xC00 RESOURCE DESCRIPTOR area,
whose `+2` is a resource flag word sharing an offset with the object record'''s.
`0x5B8D` remains the sole runtime writer of an OBJECT'''s active bit, so reading
VAR-initial bits is justified and there is no open question. What survives from
#376 is only that the ORIGINAL enumeration searched one encoding family.
