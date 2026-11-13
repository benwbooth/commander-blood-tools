# Video play plan: every HNM and every text line in the scene the game shows it in

`tools/video_play_plan.py` writes `output/anthology/video-play-plan/{cb,bbb}.json` (copies
here), plus the v7 render plan sets `text-plans-{cb,bbb}/` and `clip-plans-{cb,bbb}/`.
`tools/hnm_anthology_v7.py` joins the v7 renders into `all-hnm-{cb,bbb}-v7`.

## What was wrong before (v4..v6 renders)

* **Freezer everywhere.** The static exporter (`capture_static_text`) started every
  prepared line and clip with `request_scene_transition`, the contact transition. Its image
  is the fixed `FRIGO.FD` (path at DS:0x00F3, `SCENE_TRANSITION_IMAGE_RESOURCE`), the ship's
  freezer. The capture began at that transition's deferred-actor hold, so the place's own
  background never loaded. Bridge and on-planet lines, character clips and script
  sequences all appeared in the freezer, with its retained navigation music.
* **The hand.** The hand is MANU3, the game's 3D pointer. `RuntimePresentationHost` composited
  it into every frame, offscreen exports included. Offscreen targets exist only for
  offline exports, so it is now never drawn there (`shows_pointer`).
* **Colour "glitches".** In a freezer contact, the transition darkens scene colours
  128..191 (FRIGO.FD's colours) by 40 while the actor is in, and restores them when the
  actor leaves (`prepare_dark_palette_transition` / line 39, `prepare_live_palette_restore`
  / line 40). Character clips use palette indices 1..127 only, so they never recolour
  FRIGO. This dimming also appears in natively played freezer chapters. It is engine
  behaviour, not a palette load failure. Entry clips (`_cd`, line 39) play during that
  palette transition, so single frames can show mid-fade colours.

## Scenes, from the script that speaks the line

Each proc's `during` guard (BloodScript opcodes) decides the scene:

| guard | opcode | scene the engine shows | v7 render context |
| --- | --- | --- | --- |
| `during contact` | D1 (scene gate DS:0x274F) | contact transition: FRIGO.FD freezer. Opens with SQ/CRYOGEL (line 41, row 0), the actor enters on its right clip (39), talks and idles (8..38), leaves on its left clip (40), then SQ/CRYORAD (42, row 0). Native trace: `dialogue-bbb-prepared-contacts-v1/bbb/bbb-bob-brief-return`. | `source_default` (actor's own record only) |
| `during bridge` | CE | the bridge itself; the actor's presentation is deferred to C4 like an answered radio call (nav_actor_handler_4 0x81FB), with no transition image | `bridge_console` / `radio_call` |
| `during travel` | D0 | the commander's travel to the actor's place: `begin_chapter_travel` applies the planet and destination DESCRIPT records (backgrounds, navigation music), plays the place's PL film on line 3, and the actor talks over the place's background | new `travel` context |

The place is the holder chain in the line's own profile VAR (`object-locations.json`), as in
the earlier "placed" plans. It is not the first-profile chain play-order reports.

**BBB limitation.** BBB actors start at Trashlando, which has no DESCRIPT Location record
and can't be travelled to. At runtime, D5 settlement (`apply_sequel_settlement`,
BLOOD2PG 0x7367) moves them to places that depend on its simulation. Only the nine actors
whose place is known from verified native chapters get a travel scene. Other BBB travel
lines are planned in their contact scene, and each plan row says so.

## Videos, from the code path that presents them

| video | trigger | scene / render |
| --- | --- | --- |
| DESCRIPT Sequence record films (line 2) | sequence slots / panel | cut from the native `sequence:RECORD` renders (`native-sequences-v2`) |
| PL location films (line 3) | arrival at a Location record | native travel arrival, captured from departure to arrival (`clip_line` 3) |
| PE idle/talk clips (8..38) | Character record, `say … presentation=` | scene of the proc whose line uses the clip, else the actor's first line |
| PE right/left clips (39/40) | contact transition entry/exit | contact transition (freezer) |
| PE clips of characters with no verified line | Character record | that record applied in a contact scene (`clip_records`) |
| OB object clips (43) | `transfer X from Y to aboard` (C2 aboard transition) | scene of that proc |
| script `request sequence` films (7) | A8 (`vm_op_a8_load_string`): needs ship view or scene gate | requesting proc's scene; A8 is emulated: line 7 written to the ship presentation and the lifecycle, request bit 2 |
| SQ/MIND (CB) / SQ/MICROFOL (BBB), line 0 | opening reel | native `opening` export (silent) |
| SQ/THE_STAR, line 1 | credits | native `credits` export |
| SQ/CRYOGEL / SQ/CRYORAD, lines 41/42 | contact transition open/close | contact transition at row 0 |
| SQ/EJECTORX, SQ/EJECTION, lines 4/5 | an active ship presentation with no phase bit opens with this cycle, then the HUD (BLOODPRG 0x00AFA0) | bridge, ship presentation started natively; capture starts when the planned film is active |
| SQ/HYPER_00..07, line 6 | travel action's camera approach (camera_fsm_state_gate 0x8A4E) queues `hyper_0N` from its cycling index | bridge, then native `start_camera_transition` with that index |
| SQ/POLLUP, line 44 | script black-hole travel action, between its camera and relation phases (TravelReady) | **open:** the static exporter can't run the script travel action yet, so this is a plain decode (full screen, silent) and the plan says why |
| videos no record, script or executable path of the game names | none | plain decode, marked `played: false` (e.g. CB's ARCHEX*/TVRED*/CHANTRO*: only BBB records name them) |

Executable fixed films come from the asset path table (BLOODPRG 0x0F48B; BLOOD2PG's
line 0 is `sq\microfol.HNM`). The line numbers are the `presentation_catalog` line map.

## v7 results (2026-11-12)

* Text lines: all 12,374 previously verified lines re-rendered and verified in their scenes
  (CB 5,500, BBB 6,874): `static-text-batches/v7-{cb,bbb}-shard*` and `v7r2-cb`, merged in
  `static-text-batches/ledger-v7.json` (83 deferred sites carried over from v4).
* Masters: `output/anthology/all-hnm-cb-v7/all-hnm-videos.mkv`, 701 videos, 3333.4 s (528 engine
  clips, 47 sequence-panel cuts, opening, credits, 103 never-played plain decodes, 21 fallbacks);
  `all-hnm-bbb-v7`, 711 videos, 3680.8 s (500 engine clips, 128 sequence-panel cuts, opening,
  credits, 41 never-played plain decodes, 40 fallbacks). Each chapter title names its render method and scene.
* Fallbacks, i.e. planned engine clips without a verified render (decoded full screen):
  * AAHAN_CD/CG: never presented; talk entries 31/32 overwrite lines 39/40.
  * AAARCH10: the Ark has no DESCRIPT target.
  * BBB CROOL_3D/SSHARK10: their music (KUKANA.VOC, FLUOR.VOC) is missing from the archive.
  * The rest are talk, idle or object clips that never presented before the frame cap, even
    with their record applied: CB PION, AAISW, CRL01, ISWH..ISWX, KHRAN3_3/4_1/4_2, MG_SCR3,
    SCR12..21; BBB AAPORT, AATRO, AAZEN, DOC01..08, OMPA..L, ZEN01..12, ZENJ/K_TR.

## Open items

* **SQ/POLLUP** (one per game) needs the script travel action (C1 travel phases → TravelReady).
* **BBB on-planet places.** D5 `settle_descendants` belongs to BBB's timed population
  simulation (`population_growth`, `population_conflict`, timer-gated procs such as
  `glgest`). An actor's place depends on how long it has run, so there is no single static
  placement. Lines whose only place is Trashlando (no DESCRIPT Location) use their contact
  scene. The same applies to 88 CB lines at Trashlando.
* **Opening and credits pacing.** A queued frame is due when the software tick count has
  advanced by the stream threshold (`list_d8c_advance_due` 0x00A240). That threshold is 16
  for the startup reel (catalog variant; 12 after BLOODPRG 0x7C72). The port charges every
  blocking-loop iteration a fixed 68 ms (`PRESENTATION_FRAME_DURATION`, a capture-measured
  constant), and 68 ms gives fewer than 16 ticks. So every frame takes two iterations: MIND
  runs 35.6 s. The DOSBox-X capture shows about 18 s, but 16 ticks at the port's 182 ticks/s
  is already 88 ms (23 s). The tick rate, the startup delay and the original per-iteration
  cost can't all be right, and the original loop timing has to be recovered from the binary
  before the pacing is changed (prime rule: no tuning to captures). This predates this work.

## Paused (2026-11-12, at Ben's request: CPU)

All v8 renders were stopped. Nothing is rendering.

**Done and committed (main):** `tools/video_play_plan.py` places every `during travel`
line with evidence, so no on-planet line uses the freezer:
* the source-default place when it has a DESCRIPT Location; ships such as CB's Kraner count;
* otherwise the actor's explicit `actor.current_location = place` write in the scripts:
  same profile first, then the nearest earlier profile, then later ones;
* otherwise (BBB only) a race-mate's scripted place, because D5 `settle_descendants`
  moves a race group together;
* freezer fallbacks left: 0. CB travel lines: 1,928 source-default, 88 script-write.
  BBB: 542 source-default, 3,647 script-write, 74 race-mate.

The plan sets are `output/anthology/video-play-plan/text-plans-{cb,bbb}-v8` and
`clip-plans-{cb,bbb}-v8`. 119 CB lines, 3,721 BBB lines, 52 CB clips and 138 BBB clips
differ from v7. Each line's evidence is in its plan-set `reason`.

**Freezer audit:** `during contact` (D1, scene gate DS:0x274F) lines come from the contact
transition, which the console's CRYOBOX contacts start. Its image is FRIGO.FD (DS:0x00F3),
and the native cryobox chapters show the same. TELEPHONE calls are the radio path on the
bridge, not the freezer. Freezer-backed lines: CB 2,232 (2,218 contact + 14 unguarded
reply menus); BBB 1,037 (544 + 493). The 3,721 BBB and 119 CB fallbacks are gone.

**In progress (branch `wip/conversation-runs`, not merged):**
1. Frame-0 fix: the offline platform's `begin_capture_after_next_frame` drops the first
   captured interval, whose frame predates the prepared line or clip (stale actor or
   pose). Text and clip captures use it. It builds; there is no verified render yet.
2. Conversation-run capture: `OfflineStaticTextPlan.run_lines` presents a run's later
   lines inside one capture, so the scene and its music never restart.
   `tools/static_run_render.py plan|render` builds one plan per play-order
   `conversation_runs` entry (CB 662 captures, BBB 440) and writes `run.json` with each
   line's start and end. **Blocker:** publishing the second line fails with
   `static A6 did not publish: Gated(AlreadyShown)`. The run's lines are marked as shown
   once presented, so `publish_static_text` must reset that gate (or the actor's
   shown-line state) between lines. That's the next step.

**Next steps when resumed:**
1. Fix the AlreadyShown gate between run lines. Test the runs `cb run0000-0` (14 bridge
   lines), `cb run0004-0` (5 freezer lines) and `bbb run0070-0` (travel, placed by
   script write): music must be continuous, the first frame must not be stale, and the
   backgrounds must be real.
2. Render every run capture (CB 662, BBB 440) with the new exporter. Each run gets a
   sidecar with line times and placement evidence. Write ledger v8 from them.
3. Re-render all clips with the frame-0 fix (every v7 clip starts with one stale frame),
   then rebuild the all-HNM masters.
4. Ship-cycle fixes, POLLUP and opening pacing stay as listed under Open items.
