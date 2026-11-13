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
| SQ/EJECTORX, SQ/EJECTION (4/5), SQ/HYPER_00..07 (6), SQ/POLLUP (44) | ship presentation cycle, hyperjump, black-hole TravelReady | **open:** the static exporter can't yet build that ship state, so these are plain decodes (full screen, silent) and the plan says why |
| videos no record, script or executable path of the game names | none | plain decode, marked `played: false` (e.g. CB's ARCHEX*/TVRED*/CHANTRO*: only BBB records name them) |

Executable fixed films come from the asset path table (BLOODPRG 0x0F48B; BLOOD2PG's
line 0 is `sq\microfol.HNM`). The line numbers are the `presentation_catalog` line map.

## Open items

* Ship-cycle, hyperspace and black-hole films (11 per game) still need a static path into
  the ship presentation state.
* BBB actors' runtime places (D5 settlement) for on-planet lines.
* The opening and credits exports hold every HNM frame for two 68 ms waits: MIND runs
  35.6 s. The DOSBox-X capture cited in `platform.rs` puts MIND at about 18 s. This predates
  this work (the v5-era exporter gives the same result).
