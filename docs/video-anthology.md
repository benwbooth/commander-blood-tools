# Gameplay Video Anthologies

`tools/video_anthology.py` provides a first, scenario-driven export pipeline for
Commander Blood and Big Bug Bang. It records the Rust port's actual final GPU
output and SDL mixer submissions. It does **not** run the DOS executable or prove
DOS rendering/timing parity. Dialogue discovery uses a separate static script
scan; no game process or simulated clicks are needed for that scan.

## Static Dialogue Trees

```sh
nix develop -c cargo build --release -p commander-blood-script-compiler \
  --example dialogue_catalog
uv run tools/dialogue_catalog.py --out output/anthology/static-dialogue-en
```

This compiles the readable BloodScript files with the existing compiler, then
uses the typed COD/BAS decoders and control-flow analyzers. It does not execute
the VM, launch either game, use a display, or change saves. Default inputs are
all five CB profiles in `re/vm/profiles` and all 17 BBB profiles in
`re/vm/big-bug-bang-profiles`. BBB uses the existing English display catalogs by
default, just as the runtime does. The recovered French script remains the
authoritative source for IDs and conditions; it is not rewritten or retranslated.
`--cb-source`, `--bbb-source`, and `--analyzer` allow explicit alternate inputs.
`--bbb-english` selects the catalog directory (default
`localization/big-bug-bang/en`). `--bbb-language source` explicitly disables the
English overlay for source-language analysis.

The analyzer hashes the actual compiled COD, DIC, and optional BAS images. The exporter requires
both hashes, the profile tag, and the exact set of text-site IDs to match the
English catalog. It also checks section and choice counts, ordered live-number
operands, and inventory generators. Mismatches fail the export, rather than
silently assigning English text to different code. All 6,921 BBB text sites have
English display entries; this is catalog coverage, not a new editorial review.

The output contains an `index.md`, a hashed `catalog.json`, and per-profile:

- `source.blood`: the exact analyzed source, including named conditions.
- `graph.json`: every COD instruction, block, conditional edge, text site,
  deferred profile request, BAS selector list, and local menu-to-selector link.
- `dialogue.md`: readable lines grouped by COD procedure or BAS object/selector.
- `translation.json`: the exact bound English catalog for each BBB profile.
- `cod.dot` and, for CB, `bas.dot`: Graphviz graphs. Frame-resume edges are dashed
  and remain distinct from immediate control transfers.

Markdown and COD text/choice labels use English display fields. `graph.json`
retains the original `text`, `spoken_operands`, `choice_operands`, instructions,
and edges unchanged, with an additional per-site `display` object. Original
guard descriptions are not rewritten using translated choice labels. Translation
hashes and the selected language are included in export provenance.

Text sites retain dictionary operands, choice labels, presentation selectors,
chatter flags, conditional skips, progress/history predicates, and resume targets.
State-number and inventory operands remain symbolic. Identical strings at
different instruction offsets are distinct sites. Text-record ownership is
preserved without assuming it always identifies the audible speaker.

BAS choice links follow the first matching selector in the same object's linked
list. Shadowed duplicate selectors remain visible, and a missing local match is
explicit, not labeled unreachable. Inline TEXT choice operands are retained
separately from BAS menu-header links. COD branches retain both conditional
outcomes; conditions are not solved into feasible complete playthroughs. Cycles
stay as graph edges rather than becoming infinite DFS paths. The existing CFG's
`reachable` field is static control-flow reachability, not save-state feasibility.

The current census is **5,536 CB text sites** (3,687 COD + 1,849 BAS) and
**6,921 BBB text sites**, with 321 CB BAS selectors and 1,396 BAS menu rows.
Tests compare those sites against every authored `say`/`text_tokens` statement.
These counts include UI and repeated/conditional text, not just unique spoken
sentences. Static discovery is not video coverage or proof of runtime fidelity.
The separate BBB `SCRIPT2.BAS` artifact is not owned by the 17 active profiles
and is not included in this profile census.

The exporter publishes the directory only after every profile succeeds, retains
source/analyzer/exporter/translation hashes, and refuses to overwrite existing output.

## Build and Catalog

Requires the repository's Nix development shell, `uv`, and imported asset stores.
PyAV is pinned in the script's inline dependency declaration; `uv` manages it.

```sh
nix develop -c cargo build --release -p commander-blood-game --bins
nix develop -c uv run tools/video_anthology.py catalog \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --out output/anthology/catalog-cb.json
nix develop -c uv run tools/video_anthology.py catalog \
  --assets output/big-bug-bang/imported-assets \
  --out output/anthology/catalog-bbb.json
```

The catalog verifies every source resource against the import manifest, then
uses the typed DESCRIPT parser to associate HNM files with authored records,
captions, and roles. Categories are conversation, planetary, environment, object,
sequence, and unclassified. An asset can have several roles. Directory names are
hints, not proof of what a clip contains. No DESCRIPT reference does **not** mean
unreachable: scripts can refer to media independently. Intro, TV, credits, endings,
and story-specific labels need further authored-script classification.

## Record

```sh
nix develop -c uv run tools/video_anthology.py record \
  --jobs accuracy/anthology-pilot.json \
  --cb-assets "$HOME/.local/share/commander-blood/assets-v1" \
  --bbb-assets output/big-bug-bang/imported-assets \
  --out output/anthology/pilot --mp4
```

The checked-in pilot contains Bob's first-contact/goodbye route in CB and HONK's
initial PLAY conversation in BBB. Both include their startup/navigation prelude;
they are not edited, conversation-only cuts. Use repeatable `--job ID` options to
select jobs. `--bin-dir` selects a different build.

By default each job uses a private Xvfb display and SDL's dummy output device.
The real mixer still runs, but it does not play through the user's speakers.
`--visible` instead uses the current desktop and audio device. Every attempt has
a fresh `--write-data` directory; existing saves are never reused or changed.

Each `attempt-*` directory contains:

- `master.mkv`: final 30 fps FFV1 video plus timestamp-aligned FLAC audio.
- `viewing.mp4`: optional H.264/AAC viewing copy, not an archival source.
- `transcript.json` and `transcript.srt`: subtitles and spoken inline words.
- `chapters.ffmeta`: the route title and exact frame-based duration.
- `capture/video.mkv`, `audio.f32le`, and `timeline.jsonl`: original captured
  frames, untouched float mixer samples, and their monotonic timestamps.
- `capture/audio-timestamped.mka`: the same source PCM packets with callback
  timestamps, used by FFmpeg's asynchronous resampler to prevent audio drift.
- `capture/scenes.jsonl`: changes in profile, HNM, text, choices, and music.
- `runtime-trace.jsonl`, `scenario.tsv`, `game.log`, and `status.json`: action
  boundaries, exact route input, diagnostics, verification, and provenance.

The recorder retains native idle animation, chatter, music, progressive text,
and script waits. Recording enables real-time frame pacing even for scenarios,
which normally run accelerated. Scripted PIT ticks remain calibrated to scenario
frames. There is no artificial dialogue freeze-frame padding. The video is
sampled at 30 fps, so native frames may be repeated. Audio is measured at SDL
submission, not at the physical speaker; device latency is not calibrated.
The aligned master resamples audio to this common clock; the raw PCM remains
available for independent analysis. Final audio is padded/trimmed to the video
duration. GPU readback/encoding can still affect runtime speed on a slow machine.

Transcript timing marks a complete line at its first visible word, not individual
character reveal timing. Choice menus are retained separately in scene events.
This is not yet a speaker-attributed or static exhaustive script transcript.

## Jobs and Resume

Job manifests have `schema: 1` and a `jobs` array. Each job requires `id`, `game`
(`cb` or `bbb`), `category`, `title`, and `scenario` relative to the manifest.
Optional fields are `packed_second` (default 39), `timeout_seconds` (default 900),
`expected_resources` (DOS resource names), `expected_subtitles` (substrings), and
`expected_final_profile` (zero-based). These assertions validate a route's
observed content; they do not establish exhaustive branching coverage.

Rerunning an unchanged command verifies and reuses completed attempts. Resume
requires matching hashes of the game binary, runner, asset manifest and scenario,
matching job/options, intact output hashes, and a successful full media decode.
All source assets are revalidated. Changed inputs or damaged outputs produce a
new attempt; failed attempts remain for diagnosis. A job is complete only after
the final scripted action, its content assertions, mux, and decode checks pass.
Timeouts terminate the job process group, including its encoder.

## Assemble

Pass completed attempt directories in chapter order. Assemble the two games
separately; incompatible stream formats or corrupted captures are rejected.

```sh
nix develop -c uv run tools/video_anthology.py assemble \
  --attempt output/anthology/pilot/cb-bob-first-contact/attempt-REPLACE \
  --out output/anthology/cb-conversations.mkv
```

Repeat `--attempt` for additional chapters. Assembly stream-copies video and
losslessly re-encodes FLAC audio (to regenerate its whole-stream checksum),
offsets chapter boundaries by their exact frame durations, and writes a sibling
`.manifest.json` with source hashes. Existing output files are never overwritten.
The assembler does not deduplicate shared intros or trim routes automatically.

## Remaining Coverage Work

Use the static graph as the authored inventory for anthology planning. No click
traversal is needed to discover dialogue. The next export step is to select
finite graph segments, resolve their media and symbolic state variants, and
render alternate branches with explicit chapters. It must keep loops, deferred
profile changes, inventory, story phase, and prior-visit predicates explicit;
the graph alone does not choose one valid state for every line.

The requested full videos must use the game's presentation behavior, including
scene palettes, original subtitle fonts and reveal/hold timing, animation,
music, voices, and effects. A separate approximate compositor or silent preview
is not an acceptable substitute. The static scanner does not yet automatically
turn every graph branch into a video. Both catalogs leave unrecorded content
unrecorded rather than equating script/file presence with video coverage.

### Source-Ordered CB Contact Sweep

`tools/native_cb_contact_plans.py` walks the hashed CB COD/BAS catalog and all
65 contact procedures in profile/offset order:

```sh
uv run tools/native_cb_contact_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --out output/anthology/cb-all-contact-candidates-v2
```

The resulting `planning.json` records COD text/choice offsets for every contact,
351 simple BAS-topic candidates, and 29 procedures without a unique BAS list.
These are **not** 351 playable or verified chapters. COD story dialogue can close
a presentation before its actor's BAS menu opens; state and branch prerequisites
must be derived and tested before selecting a candidate for native capture.
SCRIPT3 Bronko's direct-contact `intelligence` candidate and SCRIPT4 Fifi's
`planet` candidate both failed the native check with "dialogue ended before all
planned choices." A SCRIPT4 Super Tromp probe navigated to Vista's `tombeau`
and selected Sinox, not Super Tromp. The source's `st1` procedure checks planet
Vista, so a second probe used the planet itself as the travel destination.
That native route verified all three simple Super Tromp BAS topics (painting,
culture, yolk) as 104.046 seconds of new capture in
`cb-super-tromp-vista-dialogue-native.mkv`. The three-chapter source batch is
`dialogue-cb-super-tromp-orbit-all-v1`. CB's combined v2 movie appends that
SCRIPT4 batch after the earlier SCRIPT1/2 dialogue captures; the earlier
standalone sequence block is not a reconstructed continuous gameplay route.
`cb-verified-anthology-v2.mkv` passed full decoded RGBA, PCM, timestamp, and
chapter verification: **78 chapters, 3,824.298 seconds, and 58,572 frames**.
The recomputed ledger `dialogue-coverage-cb-super-tromp-v1.json` has 275 CB
sites fully revealed in the native UI buffer (up from 261), with 5,254 still
uncovered. BBB remains at 787 fully revealed and 6,091 uncovered.

SCRIPT4 `bra1` begins on planet Vistar with Bratakas already there. Four
source-derived BAS topics (Vistar, planet, race, croolis) passed native capture
and complete chapter verification in `dialogue-cb-bratakas-vistar-v1`. Their
standalone movie `cb-bratakas-vistar-dialogue-native.mkv` has four chapters and
runs 174.946 seconds. The batch leaves three actor-list BAS sites unplanned;
one is the generic `talk` response and two use `leisure`, which is not offered
in this menu. The combined `cb-verified-anthology-v3.mkv` places Bratakas's
earlier SCRIPT4 procedure before Super Tromp's and passed full media checks:
82 chapters, 3,999.244 seconds.

`native_sequence_anthology.py assemble --dialogue-source-order` keeps the
authored sequence block first, then stable-sorts verified dialogue by profile
and source procedure offset where supplied, falling back to the earliest
required COD site. It checks each chapter's source-plan report hash before
sorting and records the ordering in the output manifest. This is script source
order within the captured branch set, not an inferred continuous playthrough;
the sequence records and mutually exclusive dialogue alternatives have no
single universal gameplay order.

Two SCRIPT4 `sin1` travel plans at Vista's `tombeau` now capture Sinox's candle
choice both ways. The accept and refuse plans bind COD 12056's authored words
and require their own response while excluding the other. They passed native
trace and media verification in `dialogue-cb-sinox-candle-v1`; the new batch
belongs after Super Tromp's SCRIPT4 `st1` procedure in source order. Their
standalone `cb-sinox-candle-native.mkv` passed full media verification with
two chapters and a 117.940-second duration.
The source-ordered CB assembly `cb-source-ordered-anthology-v2.mkv` passed
full media verification with 84 chapters and a 4,117.184-second duration.

BBB SCRIPT7's Betakam first-contact accept path passed as a 33.474-second
chapter in `dialogue-bbb-betakam-first-v3` and
`bbb-betakam-first-native.mkv`. COD 2182 is VM-published but not present at a
frame boundary before closure, so the plan requires publication but not a UI
draw. SCRIPT15 Scruter Mac's first-contact probe stopped at numeric chatter
dictionary position 2908. That site and chapter remain unverified; neither a
fabricated numeric voice nor a silent replacement is included.
The source-ordered BBB assembly `bbb-source-ordered-anthology-v1.mkv` passed
full media verification with 317 chapters and a 15,323.570-second duration;
the Betakam chapter occurs once in its manifest.

### Travel Audio Correction

The source-ordered CB v2 and BBB v1 assemblies above have an **audio defect**:
their offline travel entries skipped the ship-HUD music selection and retained
the startup bridge `TABLO2.VOC` stream. Their decoded PCM hashes prove only that
the captured mix survived encoding, not that the selected soundtrack was right.
Do not treat those full assemblies as audio-correct. Travel capture now applies
the planet and destination DESCRIPT selections before entry, restarts changed
music as the ship HUD does, and reports the selected `travel_music`. Dialogue
trace verification rejects a travel chapter whose selected music is not the
active navigation stream.

Freshly captured and fully verified samples are `cb-travel-audio-corrected-v3.mkv`
(Bratakas on `ITE2.VOC`, Sinox on `UMTHA2.VOC`; two chapters, 109.248 seconds)
and `bbb-travel-audio-corrected-v3.mkv` (Rotator on `TROMA.VOC`; one chapter,
74.794 seconds). The old full assemblies still need their travel chapters
re-captured and reassembled. A BBB Bug Deluxe probe with the current exporter
stopped at numeric chatter dictionary position 3944; it is not a corrected
chapter or evidence that every BBB branch can already be regenerated.

With Bratakas, Super Tromp, Sinox, and Betakam included, the recomputed
`dialogue-coverage-source-ordered-v1.json` ledger has **309 CB** and **794 BBB**
sites fully revealed in native UI buffers. It retains **5,220 CB** and **6,083
BBB** uncovered sites; publication without a raster remains separate. Script7
Alphakam and Gammakam first-contact probes stayed on their numeric presentation
line through 2,500 native frames and failed the bounded endpoint check. They are
not included in the ledger or any assembled movie.

### Offline Presentation Backend

`ModernGameServices::new_offline` now constructs the production services without
an SDL window or audio device. It uses the same GPU composition passes and the
same audio callback mixer as live playback:

- `read_offline_rgba` reads the fully composited frame after presentation. It
  rejects an unpresented or newly resized target instead of returning a blank
  frame. Main-viewport reconfiguration retains the offline target and its size.
- `render_offline_audio` consumes an explicit number of mono `f32` samples at
  `RuntimeAudioHost::output_sample_rate_hz()` (48,000 Hz). Native stream page
  refills, sound selection, and timers remain the caller's responsibility.
  Pulling samples from a live SDL host is rejected to prevent two consumers
  from advancing the same playback state.

This is a shared output backend, not a full-game export driver. It does not
choose dialogue branches, fabricate missing scene state, or change the native
schedule. The offline driver must still connect the static graph to those
services and advance the recovered clocks and stream refills at the correct
boundaries. The nominal 25 fps in normalized WebM files is not an authoritative
playback clock. No new complete anthology is claimed by these backend tests.

Tests check exact original-font pixels and palette colors for both executables
through the final GPU passes, output resizing and row padding, audio sample
equivalence with the SDL callback across stream refills and sound effects,
and device-free service initialization with real assets from both games.
These establish the tested output-backend behavior, not DOS whole-game parity.

### Offline Native Sequence Export

```sh
nix develop -c cargo run --release -p commander-blood-game \
  --bin offline-presentation -- \
  "$HOME/.local/share/commander-blood/assets-v1" opening \
  output/anthology/cb-opening
```

Use `credits` for presentation line one, `cinematic` for the complete startup
DESCRIPT sequence list, and the BBB imported asset directory for the sequel.
An optional final argument sets the frame cap (default 100,000).
The command requires FFmpeg/FFprobe and a headless wgpu adapter, but no SDL
device, window, mouse clicks, or wall-clock playback waits. Line zero is the
opening logo reel, not the subsequent scripted cinematic.

Both live and offline paths call the same native blocking presentation runner
and main lifecycle. The offline driver advances the production PIT accumulator
over each 46 ms game or 68 ms presentation wait and consumes exactly the
corresponding 48 kHz mixer samples.
Sound advances during the wait before the next completed GPU frame is exposed,
preserving the live runner's ordering. Capture includes native frame boundaries,
not the desktop host's extra render-only interpolated refreshes. It does not use
normalized WebM's 25 fps.

Each new output contains `master.mkv` (lossless full-range RGB VP9 and unmodified
float PCM), the separate `video.mkv` and `audio.f32le`, per-interval hashes and
sample offsets in `timeline.jsonl`, verified video timestamps, the source asset
manifest, private runtime files, and a provenance/accounting report. The earlier
v1/v2 logo and credits captures used lossless FFV1; the timestamped RGB encoder
was verified against the same decoded pixel stream. The final
GPU flip has zero duration at the capture endpoint; `endpoint.rgba` retains it
without inventing a hold. No audio resampling, padding, or trimming is performed.
Muxing assigns packet durations from the native timestamps and final wait;
WebM SimpleBlocks otherwise let FFmpeg infer a frame duration that can extend a
46 ms final wait to 68 ms. Both chapter export and assembly preserve the exact
endpoint, with decoded regression checks for variable and single-frame timelines.

Every imported source hash is checked before and after export. Full decoding of
the saved master must reproduce every captured pixel and audio sample exactly;
every encoded video timestamp and duration must match the native wait schedule.
Publication requires natural scene completion. A cap, codec failure, or mismatch
leaves only staging files. Existing output directories are never reused.

Verified captures on 2026-09-20: CB logos 262 frames / 17.816 s; CB credits
879 frames / 59.772 s; BBB logos 292 frames / 19.856 s; BBB credits 1,440 frames /
97.920 s. Both logo reels are silent; both credits exports contain native mixer
audio. These are standalone captures from fresh service state with the Initial
scene link, not a reconstruction of a preceding playthrough. Full dialogue,
other cinematic, travel, and branch coverage remain outstanding. Matching this native
Rust path and lossless encoding do not alone establish DOS timing/sound parity.

The `cinematic` target initializes the actual game lifecycle, executes the logo
reel during bootstrap, then captures from the first main-loop frame through the
first naturally completed DESCRIPT sequence list. It neither clicks through
scenes nor substitutes another playlist. Shutdown releases resources without
appending unrelated credits. CB's `present` sequence contains `cliptoot.hnm` and
`BLINTR.VOC`; BBB's contains thirteen clips and `CROOLRAP.VOC`. The checked
captures are 2,545 frames / 172.378 s (CB) and 1,784 frames / 120.630 s (BBB).
Both contain original-font captions; BBB uses the bound English display catalog.
`native-state.jsonl` retains each main-loop state at its pre-wait boundary.
The report also retains the selected sequence record, authored media order,
source caption bytes, displayed captions, cue frames, and fixed clock/seed.
Those cue frames belong to the game's sequence clock, not the encoded frame
number. The full library and original control-flow vectors test the shared
stepped lifecycle; exports still require their own decoded-media checks.

For a standalone authored chapter, use `sequence:RECORD`, for example
`sequence:maledict` (CB) or `sequence:48finbob` (BBB). The exporter validates the
name against the imported DESCRIPT database and rejects missing or non-sequence
records. After the initial profile is loaded, and before the panel actor opens,
it replaces only the first sequence-name slot. The unmodified native panel then
plays that record's complete ordered clip list, captions, and music through the
same lifecycle and PIT/mixer schedule. The short native bridge/panel prelude is
retained. This is explicit chapter selection, not a gameplay route or proof that
the record is reachable in that profile. The report records this distinction and
the exact selection time. No saved progress, script branch, or mouse click is used.

The databases contain 11 CB and 54 BBB sequence records, including `present`.
These are the authored multi-clip records, not all standalone HNM files or
dialogue-triggered sequences. Exporting them does not complete the dialogue,
travel, object, environment, and alternate-branch parts of the anthologies.

### Sequence Batches

```sh
nix develop -c uv run tools/native_sequence_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --out output/anthology/native-sequences/cb
nix develop -c uv run tools/native_sequence_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --out output/anthology/native-sequences/bbb
nix develop -c uv run tools/native_sequence_anthology.py assemble \
  --batch output/anthology/native-sequences/cb \
  --out output/anthology/cb-sequences.mkv
```

The batch reads the actual database with `video-catalog`, renders every sequence
record, and writes `selection.json` and `coverage.json`. Resume requires identical
source/exporter/catalog provenance and re-verifies complete chapter outputs;
changed inputs require a new output directory. Failures remain explicit and do
not prevent attempts at the remaining records. Native-state verification checks
the ordered clip occurrences, including repeated filenames, and actual caption
cue draws. Missing authored files are recorded without substituting other media.
In particular, CB's `year` references absent `SQ/PUVEN1.HNM`; this is distinct from
the existing `SQ/PUBVEN1.HNM` and follows the native unavailable-source path.

Assembly keeps every variable-rate video frame and concatenates the untouched
float PCM, with exact chapter boundaries. It stream-copies RGB VP9, then decodes
the entire assembled video/audio against each source interval's hashes and checks
every frame timestamp and chapter boundary before publishing. No frame-rate
conversion or audio resampling is used. The sibling manifest retains source and
output hashes, missing-resource dispositions, and caption coverage.

The shared authored subtitle clock must survive HNM switches. Original CB
`screen_mode_update` resets DS:0x131C at 0x7B65 only when starting a new record;
the loader and consumer increment it at 0xA18B and 0xA3F0. BBB performs the same
operations on DS:0x156A at 0x8C0C, 0xB96E, and 0xBBDA. The production player now
retains that counter across per-file queue ownership. Earlier multi-clip exports
reset it incorrectly and are superseded: lossless encoding alone did not detect
the resulting delayed or omitted captions. Per-file queue cursors still restart.

### Verified Sequence Outputs

The `output/anthology/native-sequences-v2` batches completed all 11 CB and 54 BBB
records, with no processing failures. Their assembled outputs are:

| Game | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| CB | `cb-authored-sequences-v2.mkv` | 11 | 608.352 s | 9,057 |
| BBB | `bbb-authored-sequences-v2.mkv` | 54 | 3,056.152 s | 45,485 |

The coverage reports explicitly retain CB's missing `year` resource and its two
unshown captions. One further CB cue and 15 BBB cues (including blank cues) are
timed beyond their captured sequence counters; no extra hold frames are invented
to show them. BBB has no missing authored sequence resources. These are sequence
anthologies, not the requested complete-game videos; the other content categories
and alternate dialogue branches remain to be rendered and verified.

### Native Dialogue Branches

`dialogue:PLAN.json` runs a source-bound COD/BAS branch through the same device-free
main lifecycle. It selects DIC words semantically, without pointer input. Plans
are under `accuracy/anthology-dialogue/`; that directory's README documents the
native skipped/preempted lines, radio/contact entry, and profile-setup limits.

```sh
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan accuracy/anthology-dialogue/cb-izwalito-game.json \
  --plan accuracy/anthology-dialogue/cb-izwalito-explanations.json \
  --out output/anthology/dialogue/cb
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --plan accuracy/anthology-dialogue/bbb-honk-play.json \
  --plan accuracy/anthology-dialogue/bbb-honk-instructions.json \
  --out output/anthology/dialogue/bbb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue/cb \
  --out output/anthology/cb-intro-dialogue-native.mkv
```

The batch verifies source/exporter provenance on resume, native publications and
choices, absence of pointer buttons, glyph-buffer audits, and complete decoded
RGBA/PCM/PTS equality. It records lines without full native UI reveal rather than
extending them or substituting subtitle cards. The assembly uses the sequence
assembler's lossless video copy and exact PCM concatenation.

`render --reuse-batch PATH` can reuse completed chapters from an earlier batch,
including a batch with other failed chapters. Plans, asset-manifest hashes, and
exporter hashes must match. Each selected chapter is fully decoded and its
source, trace, and media evidence compared with the saved coverage entry before
the new batch links it. Failed or altered entries cannot become successful by
being reused. The original batch and its failures are unchanged. Keep the source
capture directories: reused chapters are links, not independent copies. Resume
still verifies the linked captures. Different binaries require new captures.

The verified local `dialogue-native-v4` batch contains two CB chapters totaling
44.988 seconds and two BBB chapters totaling 75.026 seconds. Its chaptered movies
are `output/anthology/cb-intro-dialogue-native.mkv` and
`output/anthology/bbb-intro-dialogue-native.mkv`. They are introductory dialogue
branches only, not the completed full-game anthologies.

Four additional chapters render Bob's mission with both CB answers, BBB Bob's
recorded message, and BBB SCRIPT2 HONK's cryobox conversation. Contact chapters
include the native opening/closing transitions and embedded movie sequences.
No missing or skipped sequence is inserted by the exporter. These assembled
movies have passed full decoded-frame, PCM, timestamp, and chapter verification:

| Game | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| CB | `cb-bob-mission-dialogue-native.mkv` | 2 | 320.566 s | 4,776 |
| BBB | `bbb-mission-dialogue-native.mkv` | 2 | 128.068 s | 1,983 |

```sh
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan accuracy/anthology-dialogue/cb-bob-mission-yes.json \
  --plan accuracy/anthology-dialogue/cb-bob-mission-no.json \
  --out output/anthology/dialogue-contacts/cb
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --plan accuracy/anthology-dialogue/bbb-bob-recorded-mission.json \
  --plan accuracy/anthology-dialogue/bbb-honk-daddy-cryobox.json \
  --out output/anthology/dialogue-contacts/bbb
```

The CB assembled movie retains its `dialogue-contacts-v2/cb` inputs, and the BBB
movie uses `dialogue-contacts-v3/bbb`. The v3 exporter is archived at
`output/anthology/dialogue-contacts-v3/bin/offline-presentation` for reproduction.
Both CB chapters reproduced with that exporter in `dialogue-contacts-v3/cb`,
matching their v2 pixels, PCM, endpoints, and runner reports. The final-build
BBB SCRIPT2 reproduction in `dialogue-final-build/bbb` matched those same fields.
Frame inspection confirmed original contact artwork and fonts and BBB English
text in sampled encoded frames; this is not visual inspection of every line.

### Static BAS Topic Plans

```sh
uv run tools/native_bas_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile 2 --procedure 7244 \
  --out output/anthology/bas-plans/cb-bob-script2
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan-set output/anthology/bas-plans/cb-bob-script2/planning.json \
  --exporter output/anthology/dialogue-bas-bob-v1/bin/offline-presentation \
  --out output/anthology/dialogue-bas-bob-v1/cb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue-bas-bob-v1/cb \
  --out output/anthology/cb-bob-bas-topics-native.mkv
```

The planner follows the static graph's first-match menu links and emits finite
paths to simple positive-history topic replies. It selects a topic once for each
successive reply and closes through an authored `bye_bye` row. Random, record,
resume, and additional-history gates, unvisited nodes, and untargeted sites remain
explicit in `planning.json`; no UI traversal is used to discover them. Graph and
contact-manifest hashes bind the planning report. Generated plans are candidates,
not proof that a given COD contact procedure reaches that BAS menu.

The plans use explicit prepared contact state, validated against the CB contact
manifest. Capture retains all changed save bytes and before/after hashes; this
is not a continuous gameplay route. Required BAS publications are independently
source-bound and cannot alias COD offsets. The normal native text, menu, hand,
animation, audio, and closing-transition lifecycle remains active.

The verified Bob SCRIPT2 batch has **nine chapters, 408.562 seconds, and 6,105
native frames**. It targeted 38 BAS sites and published 39, including an extra
nested-menu-path line. All 39 have full native UI-buffer reveal evidence. Its
assembled movie passed full decoded RGBA, PCM, timestamp, and chapter checks.
Selected encoded frames were inspected for the original artwork and fonts,
including a nested cottage-topic reply; this is not inspection of every line.
Fourteen of this actor list's 53 BAS sites remain unrecorded. A separate Bronko
contact probe ended before its planned choices and is not counted as coverage.

### Prepared Travel Topics

```sh
uv run tools/native_bas_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile 2 --procedure 23683 \
  --travel-planet Moskito --travel-destination usine \
  --out output/anthology/bas-plans/cb-bronko-travel-script2-v2
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan-set output/anthology/bas-plans/cb-bronko-travel-script2-v2/planning.json \
  --exporter output/anthology/dialogue-travel-bronko-v4/bin/offline-presentation \
  --out output/anthology/dialogue-travel-bronko-v4/cb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue-travel-bronko-v4/cb \
  --out output/anthology/cb-bronko-bas-topics-native.mkv
```

The travel planner reads the hashed COD graph's outer travel guard rather than
the contact manifest. Runtime setup validates the actor and supported predicates,
records changed save bytes and hashes, then starts the native post-HUD travel
lifecycle. Arrival, automatic actor selection, conversation, and the completed
return to the bridge remain in each chapter. This is explicitly prepared state,
not a continuous route or proof that prior gameplay reached those conditions.

The Bronko batch verified **seven chapters, 183.094 seconds, and 2,788 native
frames**. It targeted 17 BAS sites and published 21, plus three COD sites; all
24 have full native UI-buffer reveal evidence. Nine of Bronko's 30 BAS sites
remain unrecorded. Buy and war each repeat the authored goodbye once after a
native menu reopening; the explicit bounded policy does not force closure or
change timing. Earlier failed contact/travel attempts are not counted.

The assembled video passed full decoded RGBA, PCM, timestamp, and chapter checks.
The checked-in energy fixture independently reproduced the generated chapter's
487 frames, pixels, PCM, and endpoint. A sampled frame from the assembled video
was inspected for original factory artwork, font, text, and hand; this is not
visual inspection of every line. No BBB travel chapter is claimed by this batch.

Three further actor batches use the same archived Bronko v4 exporter:

| Actor | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| Yoko | `cb-yoko-bas-topics-native.mkv` | 28 | 1,336.188 s | 20,733 |
| Daddy Gluxx | `cb-gluxx-bas-topics-native.mkv` | 5 | 408.960 s | 6,095 |
| Izwalito | `cb-izwalito-bas-topics-native.mkv` | 11 | 409.542 s | 6,430 |

Their capture batches are `dialogue-travel-yoko-v2/cb`,
`dialogue-travel-gluxx-v2/cb`, and `dialogue-travel-izwalito-v4/cb`. Yoko uses
SCRIPT2 procedure 32225 on Rondo at `pavillon`; Gluxx uses 34287 on Ekatomb;
Izwalito uses 21258 on Corpo with `--entry-menu 5915`. The latter is the BAS menu
selected by the COD `topic = "talk"` assignment, not a forced runtime setting.
Repeated topic names are distinguished by their actual menu paths.

The generated Gluxx treatment plan is replaced by the checked-in three-selection
plan to account for an unconditional intervening line. Izwalito's generic ideal
plan is replaced by both checked-in secret-answer branches. Each retains the
BAS goodbye that triggers the COD question and the later resumed-menu goodbye.
The accepted answer does not open the `know` submenu in this captured route;
those sites are still uncovered. No timing or script modification was needed.
The corrected Gluxx and Izwalito batches reuse their other verified chapters;
all earlier failed batches and diagnostic attempts remain separate.

Yoko publishes 70 BAS and six COD sites, Gluxx 15 BAS and 17 COD, and Izwalito
36 BAS and nine COD, all with full native UI-buffer reveals. Their respective
actor lists still have 34, six, and eleven BAS sites unrecorded. These 44 chapters
are additional prepared-state branches, not exhaustive actor dialogue or a
continuous playthrough. Sampled encoded frames were inspected; complete decoded
RGBA, PCM, timing, and chapter verification applies to each assembled movie.
The final-build reproduction of Izwalito's acceptance branch in
`dialogue-travel-actors-final-build/cb` matches the original capture's pixels,
PCM, endpoint, and complete runner report. Its binary is archived in the sibling
`bin` directory.

### BBB Prepared Contacts

The eleven checked-in SCRIPT2-5 prepared-contact plans produce
`output/anthology/bbb-cryobox-dialogue-native.mkv`: **266.934 seconds and 3,986
native frames**. They cover Bob's brief return, concert aftermath for Daddy,
Papy, Mamy, Marakas, Izwalito, Tequila, Eviscerator, and Outrageor, and Tequila's
outburst and ghost branches. The capture batch is
`output/anthology/dialogue-bbb-prepared-contacts-v1/bbb`; its exporter is archived
in the sibling `bin` directory.

BBB preparation reads the typed COD outer D1 guard and validates the selected
actor's action record. Only supported entry predicates are prepared; body
conditions are not promoted to entry state. Other outer contact procedures are
disabled. The report binds the COD hash and retains every changed save byte and
before/after hashes. CB continues to use its existing contact manifest. This is
explicit chapter setup, not proof of a gameplay route or contact-menu eligibility.

All 75 required COD publications occurred: 64 fully reveal in the native UI
buffer, and eleven terminal sites have no UI draw. Native CRYOGEL/CRYORAD
transitions, embedded movie sequences, English display text, fonts, timing, and
audio remain active. No artificial hold or missing-clip insertion was added.
The assembled movie passed full decoded RGBA, PCM, timestamp, and chapter checks.
Sampled encoded Bob and Tequila frames show the original artwork and subtitle
fonts with English text; this is not visual inspection of every line.
Two CB regression captures preserve prior pixels, PCM, and endpoints: Bob's
black-hole contact and Bronko's energy travel topic. These checks do not establish
original-executable parity or full-game coverage.

Ten later-profile chapters produce
`output/anthology/bbb-later-contacts-dialogue-native.mkv`: **274.948 seconds and
4,099 native frames**. They cover Rotator, Otto Von Smile, Otto Von Gluk, Trump,
Tramp, and Super Tromp after the concert; Bug Deluxe's future and farewell
conversations; and Cyberquizz's first visit and Christmas greeting. Their batch
is `dialogue-bbb-later-contacts-v1/bbb`, using the same archived prepared-contact
exporter as the preceding eleven chapters. All 74 required publications occur;
64 fully reveal in the native UI buffer, and ten terminal sites have no draw.
The movie passed full decoded RGBA, PCM, timestamp, and chapter verification.
Sampled encoded Bug Deluxe and Cyberquizz frames were inspected for native
artwork, fonts, and English text; not every line was visually inspected.

Cyberquizz's second visit is
`output/anthology/bbb-cyberquizz-second-visit-native.mkv`: **63.876 seconds and
951 native frames**, from `dialogue-bbb-encounters-v1/bbb`, whose new exporter is
archived in its sibling `bin` directory. `contact_encounter_guard: 2114` binds the
authored equality test on Cyberquizz's encounter counter within procedure 1748.
Setup stores one; native C4 entry increments it to two. Both visit plans assert
that the other branch's lines are absent. This is explicit prepared visit state,
not a replay of the first encounter. Unsupported or out-of-procedure guards and
arbitrary body assignments are rejected.

All 14 second-visit publications occur, 13 fully reveal in the UI buffer, and
terminal site 2504 has no draw. The native `PPIT07` sequence and contact closing
transition remain intact. Assembly passed full decoded media, timing, and
chapter checks; a sampled encoded second-visit frame was visually inspected.
The new exporter reproduces Cyberquizz's first visit, CB Bob's black-hole
contact, and CB Bronko's energy travel topic with identical pixels, PCM,
endpoints, and complete runner reports in `dialogue-bbb-encounter-regression`.

Cyberquizz and Bioquizz travel greetings produce
`output/anthology/bbb-quizz-travel-dialogue-native.mkv`: **66.400 seconds and
992 native frames**, from `dialogue-bbb-travel-quizz-v1/bbb`. The exporter is
archived in `dialogue-bbb-travel-probe-v2/bin`. Each chapter explicitly stages
the selected actor at Cyberock's Cyberland destination, including the native
navigation visibility flag. Cyberquizz also enables its source-bound companion
travel procedure. Native arrival, actor selection, dialogue, empty inventory
response, departure, and bridge return remain active. This is prepared chapter
state, not evidence of a normal gameplay route or canonical actor placement.

The two chapters publish twelve sites: ten fully reveal in the native UI buffer,
and terminal sites 4795 and 6046 have no draw. Assembly passed full decoded RGBA,
PCM, timestamp, and chapter checks. Encoded dialogue samples for both actors
were visually inspected; this is not an inspection of every line. The new
exporter reproduces CB Bob's black-hole contact, CB Bronko's energy travel
topic, and BBB Cyberquizz's first visit with identical pixels, PCM, endpoints,
and runner reports in `dialogue-bbb-travel-regression`.

The checked-in Bug Deluxe travel plan remains a failed probe in
`dialogue-bbb-travel-probe-v2/bbb`, not verified coverage. Numeric chatter tries
to hash dictionary offset 3944 while SCRIPT9.DIC contains only 2,612 bytes.
Original-executable boundary probes demonstrate that the hash depends on bytes
beyond that resource; the live native allocation contents are not recovered.
No zero-padding, substituted number text, or silent-audio workaround was added.

### BBB Inventory Branches

Object-backed inventory choices use their original VAR record offsets, not
dictionary words or translated labels. Optional starting inventory is explicitly
staged aboard and included in the setup's save-byte diff. The existing native
chooser, hand animation, transfer, descriptor continuation, and response scripts
remain responsible for the resulting scene. The verifier checks the actual
offered menu/item/recipient and the later ownership change. This does not prove
a normal gameplay acquisition route.

The static inventory planner binds the catalog graph, travel template, and
existing English inventory-label catalog. It requires each selected item's A6
and flat flag-guarded reaction in the native trace. It does not set item-transfer
flags or force reaction lines. Plans remain candidates until captures pass:

```sh
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --template accuracy/anthology-dialogue/bbb-cyberquizz-travel-greeting.json \
  --inventory-menu 4730 \
  --out output/anthology/plans-bbb-cyberquizz-inventory-v1
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --template accuracy/anthology-dialogue/bbb-bioquizz-travel-greeting.json \
  --inventory-menu 5981 \
  --out output/anthology/plans-bbb-bioquizz-inventory-v1
```

Each report plans 22 item branches. Nuclear reactions require additional
evolution predicates and remain deferred by this flat planner; laws and scruter have no simple
matching item-flag guard. Those are explicit gaps, not unreachable dialogue.
The checked-in technology and treaty plans exercise response requirements and
exact native identity independently of display-name encoding.

All 44 generated chapters completed with a verified native offer and ownership
transfer in each. The batches are `dialogue-bbb-inventory-v2/cyberquizz` and
`dialogue-bbb-inventory-v2/bioquizz`, with their exporter archived in the sibling
`bin` directory. Each assembled movie contains 22 chapters, **826.356 seconds
and 12,338 native frames**:

- `output/anthology/bbb-cyberquizz-inventory-dialogue-native.mkv`
- `output/anthology/bbb-bioquizz-inventory-dialogue-native.mkv`

Both passed full decoded RGBA, PCM, timestamp, and chapter checks. Each batch
publishes 29 unique COD sites: 28 fully reveal in the native UI buffer, and its
existing terminal site has no draw. Together they add 46 previously unrecorded
fully revealed sites, including the two inventory prompts and 44 reactions.
Encoded samples show the English treaty menu, native hand and actor artwork,
and guitar response. Not every encoded line was visually inspected. The earlier
string-identity prototype batch is excluded from the ledger; the final runner
uses VAR offsets, and its technology chapter preserves the prototype's pixels,
PCM, and endpoint.

Inventory chooser, transfer, and descriptor boundaries were rerun against the
original executable: 191 cases match the checked-in oracle vectors. These
boundary checks do not establish whole-conversation DOS parity. Four captures
in `dialogue-bbb-inventory-regression` preserve prior pixels, PCM, and endpoints:
CB Bob's black-hole topic, CB Bronko's energy topic, BBB HONK's instructions,
and Bioquizz's travel greeting. Runner reports also match except for the older
HONK report's legacy metadata; its native choices, publications, and timing match.

### BBB Nuclear Gifts

Six checked-in nuclear-gift plans add source-bound initial evolution guards to
the existing native inventory path. Cyberquizz guards 3669/3687/3752 and
Bioquizz guards 4920/4938/5003 derive starting values 0/101/501 using the
production signed comparison handler. The preparation records its guard,
before/after value, and save-byte diff; trace verification checks the actual
actor evolution before accepting the gift interaction. No item-transfer flag
or response line is forced.

The batch `output/anthology/dialogue-bbb-nuclear-v2/bbb` contains six verified
chapters, assembled as
`output/anthology/bbb-nuclear-gift-dialogue-native.mkv`: **224.232 seconds and
3,348 native frames**. The archived exporter is
`dialogue-bbb-nuclear-v1/bin/offline-presentation`. The initial v1 capture batch
failed only at encoding because it was launched without the Nix environment's
`ffmpeg`; it is excluded from coverage. Run captures inside `nix develop -c`.

Every chapter verifies the native item offer and ownership transfer. Middle
and high captures require their own response and exclude the other; low
captures exclude both spoken responses. The four new response sites fully
reveal in the UI buffer. Actor traces show the middle branch's population
1-to-0 and aggressiveness 0-to-50 changes, and the high branch's evolution
501-to-521 and energy 0-to-100 changes. Low captures are not evidence that no
global state changed. The authored 100 and 500 boundary gaps and signed-word
comparisons are covered by binding tests, not additional movie chapters.

The assembled movie passed full decoded RGBA, PCM, timestamp, and chapter
checks. Encoded samples of Cyberquizz's middle and Bioquizz's high response
show fully revealed English text; not every encoded frame was visually
inspected. The original executable's 1,728 shared-state arithmetic cases still
match the checked-in vectors. Three regression captures in
`dialogue-bbb-nuclear-regression` exactly preserve the prior runner, pixels,
audio, and endpoint for CB Bob's black-hole topic, Cyberquizz's technology
gift, and Bioquizz's travel greeting. These checks do not prove complete DOS
conversation parity or a normal gameplay acquisition/evolution route.

### BBB Paul Gifts

Three source-bound SCRIPT16 templates cover prepared gift visits to Mega Paul,
Sebasto Paul, and Inter Paul. They explicitly stage each actor at Cyberland,
enable the selected gift procedure, and leave native arrival, presentation,
inventory selection, transfer, and closure intact. This is not a reconstructed
route from their initial Trashlando placement. Mega Paul's separate story
procedures are not enabled by his gift template.

The flat planner produces 22 item candidates per actor. The templates are
`accuracy/anthology-dialogue/bbb-{mega,sebasto,inter}-paul-gift-visit.json`, with
inventory menus 5540, 6935, and 8328 respectively. The generated plan sets are
`output/anthology/plans-bbb-{mega,sebasto,inter}-paul-inventory-v1/planning.json`.
Nuclear responses and other evolution/story branches remain outside these
flat plan sets.

All 66 generated chapters passed native offer/transfer and lossless-media
verification in `output/anthology/dialogue-bbb-paul-inventory-v1`. The exporter
is the unchanged archived `dialogue-bbb-nuclear-v1/bin/offline-presentation`.
The three assembled outputs are:

| Movie under `output/anthology` | Chapters | Seconds | Native Frames |
| --- | ---: | ---: | ---: |
| `bbb-mega-paul-inventory-dialogue-native.mkv` | 22 | 707.632 | 10,660 |
| `bbb-sebasto-paul-inventory-dialogue-native.mkv` | 22 | 810.952 | 12,193 |
| `bbb-inter-paul-inventory-dialogue-native.mkv` | 22 | 810.952 | 12,193 |

All three assemblies passed full decoded RGBA, PCM, timestamp, and chapter
verification. The 69 captures including visits total 2,423.170 seconds and
36,455 native frames.

Together the batches publish 85 distinct COD sites: 82 fully reveal in the
native UI buffer and three terminal sites (5580, 6975, 8368) have no draw.
Encoded Mega/Inter guitar-response samples show the native artwork and fully
revealed English text. This is not a visual audit of every encoded line. The
three unstocked visit captures also passed, assembled as
`bbb-paul-gift-visits-native.mkv` (93.634 seconds, 1,409 frames) from
`dialogue-bbb-paul-gift-visits-v1/bbb`. They add chapters, not distinct response
coverage beyond the stocked branches.

The planner can also derive a gift-entry template directly from the hashed
catalog, without a hand-written visit fixture:

```sh
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile SCRIPT16 --inventory-menu 5540 \
  --planet Cyberock --destination Cyberland \
  --out output/anthology/plans-bbb-mega-paul-derived-inventory-v1
```

This mode accepts only an outer guard containing exactly D0 travel plus a
positive actor selection bound to the menu owner and player. Its report retains
the derived template and graph hash. Extra entry predicates are rejected; actor
kind, procedure, destination type, and planet relationship are independently
checked by the native exporter. The generated plans require the menu and flat
reaction, not unrelated introductory lines. Every actual publication is still
retained in the trace and ledger.

A static scan finds 963 flat candidates across 46 inventory menus. This is not
runtime coverage: one Daddy Gluxx menu has no flat candidates, complex reactions
remain deferred, and some actors still hit the unresolved numeric-chatter
dictionary-tail issue. The graph-derived Mega Paul guitar capture in
`dialogue-bbb-derived-entry-regression/bbb` matches the hand-written template's
RGBA, PCM, endpoint, and runner behavior (apart from the different required-site
plan). It is not counted as additional dialogue coverage. Regenerating Inter
Paul's original template-based plan set is byte-for-byte unchanged.

### BBB Eviscerator and Outrageor Gifts

SCRIPT5 has 23 flat item reactions for each actor, derived from source-bound
inventory menus 5431 (Eviscerator) and 7846 (Outrageor). Their checked-in nuclear
fixtures and generated plans explicitly stage the actor at Cyberland and the
item aboard; these are prepared branches, not acquisition or encounter routes.
The plan sets are `plans-bbb-eviscerator-inventory-v1` and
`plans-bbb-outrageor-inventory-v2` under `output/anthology`.

All 46 captures passed in `dialogue-bbb-mutant-inventory-v1/eviscerator` and
`dialogue-bbb-mutant-inventory-v2/outrageor`. Both assemblies passed full
decoded RGBA, PCM, timestamp, and chapter checks:

| Movie under `output/anthology` | Chapters | Seconds | Native Frames |
| --- | ---: | ---: | ---: |
| `bbb-eviscerator-inventory-dialogue-native.mkv` | 23 | 726.174 | 10,944 |
| `bbb-outrageor-inventory-dialogue-native.mkv` | 23 | 704.994 | 10,606 |

Together they add 1,431.168 seconds and 21,550 native frames. Eviscerator
publishes 28 distinct COD sites, 27 fully revealed in the native UI buffer;
Outrageor publishes 32, 31 fully revealed. Terminal sites 5471 and 7886 have
no draw. These counts include control text, not only spoken sentences.

Outrageor's nuclear reaction writes the item's holder back to aboard at COD
6712. The native inventory menu reopens. A new `inventory_cancel` choice binds
the same source menu but names neither an item nor a dictionary word. It
requests the chooser's existing final row and retains native hand animation,
Closing/Closed states, and script completion. The exporter cannot use it when
the native chooser has no cancel row. The trace verifier checks visible cancel
glyphs, matching menu/recipient, native closure, and aboard ownership of every
offered item through closure; the coverage ledger recomputes that evidence.
The flat planner adds cancellation when a reaction writes a known inventory
holder back to the aboard sentinel, still subject to native capture verification.

In the nuclear pilot, the item is offered at 23.512 seconds, transferred at
23.966, and returned before the reopened menu at 27.980. Cancellation enters
Closing at 28.048 and Closed at 28.366, retaining the nuclear item aboard.
The original executable's 11 inventory-chooser cases and four call-order cases
still match the checked-in vectors. This is not a complete original-executable
conversation comparison.

The Eviscerator batch uses the archived `dialogue-bbb-nuclear-v1` exporter.
Outrageor uses `dialogue-bbb-inventory-cancel-v1/bin/offline-presentation`.
Three captures under `dialogue-bbb-inventory-cancel-regression` preserve the
previous RGBA, PCM, endpoint, timing, and complete runner records for CB Bob's
black-hole topic, Cyberquizz's technology gift, and Eviscerator's nuclear gift.
Encoded samples show Eviscerator's guitar response, Outrageor's nuclear response,
and the returned-item CANCEL row. Not every encoded frame was visually audited.

The initial SCRIPT6 probes in `dialogue-bbb-mutant-inventory-pilot-v1` failed:
Emasculator's actor-only story procedure reaches a name/no question before the
gift menu; Rotator's first-visit story ends before gift selection. Each has 20
flat candidates and three deferred nested reactions. Those failed probes are
not counted as coverage; the following captures retain the story procedures
and prepare source-bound later encounters.

### BBB Emasculator and Rotator Visits

Thirty SCRIPT6 fixtures record both actors' first visits, selected later story
branches, and six nested gift reactions. All explicitly stage the actor at
Cyberland. First visits retain the native initial counter. Later visits use the
new BBB-only
`travel_setup.actor_encounter_guard`: a source offset inside an enabled
actor-only story procedure, whose outer guard must have no extra predicates.
Only a single positive equality against this actor's encounter field is
accepted. The exporter writes one less than the authored operand, then native
C4 entry increments it. The report retains the original/prepared values and
save hashes; the trace verifier checks preparation and the first presentation
of the selected actor. The ledger independently recomputes this evidence.
This is prepared visit state, not a replay of earlier encounters.

The first-visit batch is `dialogue-bbb-script6-first-visits-v2/bbb`; its three
chapters total 402.006 seconds and 6,050 native frames. It publishes 79 distinct
COD sites, 77 fully revealed in the UI buffer and two terminal sites without
a draw. The later-visit batch `dialogue-bbb-script6-later-visits-v1/bbb` adds
three chapters totaling 186.708 seconds and 2,802 frames: Emasculator's second
visit with the affirmative answer, Rotator's second visit, and his fourth
visit declining the offer. It publishes 46 sites, 43 fully revealed and three
without a draw. Rotator's second-visit site 12345 is not published; preceding
A6 12329 carries skip-next=1 when not shown. Its fixture records this native
omission instead of adding text or a hold.

The inventory planner now retains validated, same-actor COD dictionary
prerequisites before appending the item choice. Emasculator's template keeps
the second-visit affirmative answer, and Rotator's keeps the fourth-visit
decline. Native story, chooser, transfer, returned-item cancellation, and
closure still execute. Both actors have 20 static flat candidates. The planner
defers technology, culture, and writing, but separate source-bound fixtures
capture one native path through each of those six reactions. Emasculator's
captured technology path publishes COD 9578. COD 9495 requires the separate
`globals.A100 > 4` branch and remains unrecorded. Laws and scruter have no
matching simple item-flag guard.

Emasculator's painting candidate (VAR 7256) fails to publish required COD
9218/9244 in both second-visit and third-visit probes. The native trace shows
the item flag cleared before either publication. The source's final chatter
has skip-next=2 before three state mutations, including that clear, but the
cause and original-game behavior have not been established. The failed batch
and separate `dialogue-bbb-script6-painting-probe-v1/bbb` remain evidence of an
unresolved gap, not global unreachability. The final Emasculator plan set uses
`--exclude-item 7256`, preserving the exclusion in its planning report and
leaving the reaction uncovered.

The final batches and their assembled outputs are:

| Movie under `output/anthology` | Batch suffix | Chapters | Seconds | Native frames |
| --- | --- | ---: | ---: | ---: |
| `bbb-script6-first-visits-native.mkv` | `dialogue-bbb-script6-first-visits-v2/bbb` | 3 | 402.006 | 6,050 |
| `bbb-script6-later-visits-native.mkv` | `dialogue-bbb-script6-later-visits-v1/bbb` | 3 | 186.708 | 2,802 |
| `bbb-script6-story-dialogue-native.mkv` | `dialogue-bbb-script6-story-branches-v3/bbb` | 18 | 1,619.826 | 24,238 |
| `bbb-emasculator-inventory-dialogue-native.mkv` | `dialogue-bbb-script6-inventory-v2/emasculator` | 19 | 1,657.214 | 24,870 |
| `bbb-rotator-inventory-dialogue-native.mkv` | `dialogue-bbb-script6-inventory-v1/rotator` | 20 | 1,288.924 | 19,453 |
| `bbb-script6-nested-gifts-native.mkv` | `dialogue-bbb-script6-nested-gifts-v1/bbb` | 6 | 472.732 | 7,115 |

Each assembled movie passed full decoded RGBA, PCM, timestamp, and chapter
verification. First-visit, later-visit, Rotator painting, and Emasculator
fifth-visit encoded samples were visually inspected; this is not an audit of
every encoded line. Rotator's fourth-visit acceptance publishes COD 13395
without a frame-boundary UI draw. Its fixture requires the publication and
records that limitation.

First visits use the archived `dialogue-bbb-inventory-cancel-v1` exporter.
Later visits and gifts use `dialogue-bbb-travel-encounters-v1/bin/offline-presentation`.
Three captures under `dialogue-bbb-travel-encounters-regression` exactly retain
the previous pixels, PCM, endpoint, and runner records for CB Bob's black-hole
topic, Cyberquizz's second visit, and Outrageor's nuclear gift. All 33 original
BBB action-dispatch oracle cases still match the checked-in vectors. These
checks do not establish complete DOS conversation parity. Unrecorded answer
combinations, alternate nested reaction states, and global story states remain open.

### Whole-Catalog Dialogue Ledger

```sh
uv run tools/native_dialogue_coverage.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --batch output/anthology/dialogue-native-v4/cb \
  --batch output/anthology/dialogue-native-v4/bbb \
  --batch output/anthology/dialogue-contacts-v2/cb \
  --batch output/anthology/dialogue-contacts-v3/bbb \
  --batch output/anthology/dialogue-bas-bob-v1/cb \
  --batch output/anthology/dialogue-travel-bronko-v4/cb \
  --batch output/anthology/dialogue-travel-yoko-v2/cb \
  --batch output/anthology/dialogue-travel-gluxx-v2/cb \
  --batch output/anthology/dialogue-travel-izwalito-v4/cb \
  --batch output/anthology/dialogue-bbb-prepared-contacts-v1/bbb \
  --batch output/anthology/dialogue-bbb-later-contacts-v1/bbb \
  --batch output/anthology/dialogue-bbb-encounters-v1/bbb \
  --batch output/anthology/dialogue-bbb-travel-quizz-v1/bbb \
  --batch output/anthology/dialogue-bbb-inventory-v2/cyberquizz \
  --batch output/anthology/dialogue-bbb-inventory-v2/bioquizz \
  --batch output/anthology/dialogue-bbb-nuclear-v2/bbb \
  --batch output/anthology/dialogue-bbb-paul-gift-visits-v1/bbb \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/mega-paul \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/sebasto-paul \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/inter-paul \
  --batch output/anthology/dialogue-bbb-mutant-inventory-v1/eviscerator \
  --batch output/anthology/dialogue-bbb-mutant-inventory-v2/outrageor \
  --batch output/anthology/dialogue-bbb-script6-first-visits-v2/bbb \
  --batch output/anthology/dialogue-bbb-script6-later-visits-v1/bbb \
  --batch output/anthology/dialogue-bbb-script6-story-branches-v3/bbb \
  --batch output/anthology/dialogue-bbb-script6-inventory-v2/emasculator \
  --batch output/anthology/dialogue-bbb-script6-inventory-v1/rotator \
  --batch output/anthology/dialogue-bbb-script6-nested-gifts-v1/bbb \
  --out output/anthology/dialogue-coverage-bbb-script6-full.json
```

The ledger binds graph, chapter, trace, and media hashes, recomputes UI evidence
from the retained trace, and joins by game/profile/source-kind/offset. BAS
evidence requires the matching BAS hash. Repeated captures do not inflate the site census. A
site absent in one branch can still be published in another; absence is never
classified as global unreachability.

Across the 326 verified dialogue chapters, CB has 261 sites fully revealed in
the native UI buffer, six published without a UI draw, one absent from the
selected branches, and 5,268 uncovered. BBB has 787 fully revealed, 41
published without a UI draw, two absent from the selected branches, and 6,091
uncovered. Native UI-buffer evidence alone does not prove encoded glyph
visibility. These counts include empty/control text sites and do
not imply that every uncovered site is a unique spoken line. Neither full-game
anthology is complete; remaining profile branches, most CB BAS, state-dependent
conversations, objects, travel, and environments still need coverage.

### Combined Verified Movies

`native_sequence_anthology.py assemble` accepts repeated `--batch` arguments
in chapter order. It rejects incomplete or mixed-game batches and duplicate
captures. The two combined outputs join the 65 verified sequence chapters with
the 326 verified dialogue chapters. Each manifest lists its ordered
`source_batches`, source chapter hashes, chapter boundaries, and whole-file
RGBA/PCM hashes.

| Movie under `output/anthology` | Chapters | Seconds | Native frames |
| --- | ---: | ---: | ---: |
| `cb-verified-anthology-v1.mkv` | 75 | 3,720.252 | 56,962 |
| `bbb-verified-anthology-v1.mkv` | 316 | 15,290.096 | 229,684 |

Both combined movies passed full decoded pixel, float PCM, timestamp, and
chapter verification. They preserve the native chapter captures in order;
they are chaptered anthologies, not continuous playthroughs. The dialogue
ledger above still records thousands of uncovered sites. CB's authored `year`
sequence names `SQ/PUVEN1.HNM`, which is absent from the imported assets; the
native capture does not fabricate it. BBB's Bug Deluxe numeric-chatter travel
branch and Emasculator's painting reaction likewise remain unresolved. These
limitations prevent either movie from being called an exhaustive game video.

The sequence and initial dialogue batch binaries are archived in `native-sequences-v2/bin`. To re-verify
or resume these batches after rebuilding, pass their `offline-presentation` and
`video-catalog` paths through `--exporter` and `--catalog-binary`. Each chapter and
assembled movie retains its own hashes and native timing evidence. Final-build
reproduction of BBB's repeated-clip `39argent` record matched the batch's pixels,
PCM, endpoint, and runner report exactly.

## Tests

```sh
uv run python -m unittest discover -s tools -p test_dialogue_catalog.py
uv run python -m unittest discover -s tools -p 'test_native_*.py'
nix develop -c cargo test --release -p commander-blood-script-compiler \
  --test dialogue_catalog
nix develop -c uv run --with av==18.1.0 python -m unittest discover \
  -s tools -p test_video_anthology.py
nix develop -c cargo test -p commander-blood-game --lib
nix develop -c env CBLOOD_REQUIRE_ACCURACY_TESTS=1 \
  cargo test --release -p commander-blood-game --lib offline
nix develop -c cargo test --release -p commander-blood-game --lib \
  runtime::offline_export::tests::offline_export_preserves_variable_intervals_pixels_and_audio \
  -- --ignored --exact
nix develop -c cargo test -p commander-blood-game --lib \
  recording::tests::lossless_writer_preserves_frames_audio_and_shared_start_offset \
  -- --ignored --exact
```

PCM packet muxing uses [PyAV audio frames](https://pyav.org/docs/stable/api/audio.html)
and [containers](https://pyav.org/docs/stable/api/container.html); clock correction
uses FFmpeg's `aresample` filter. Captured media and imported game assets are not
checked into the repository.
