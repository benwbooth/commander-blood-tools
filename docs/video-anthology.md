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

The analyzer hashes the actual compiled COD and DIC images. The exporter requires
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

The actual batch binaries are archived in `native-sequences-v2/bin`. To re-verify
or resume these batches after rebuilding, pass their `offline-presentation` and
`video-catalog` paths through `--exporter` and `--catalog-binary`. Each chapter and
assembled movie retains its own hashes and native timing evidence. Final-build
reproduction of BBB's repeated-clip `39argent` record matched the batch's pixels,
PCM, endpoint, and runner report exactly.

## Tests

```sh
uv run python -m unittest discover -s tools -p test_dialogue_catalog.py
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
