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
uv run tools/dialogue_catalog.py --out output/anthology/static-dialogue
```

This compiles the readable BloodScript files with the existing compiler, then
uses the typed COD/BAS decoders and control-flow analyzers. It does not execute
the VM, launch either game, use a display, or change saves. Default inputs are
all five CB profiles in `re/vm/profiles` and all 17 BBB profiles in
`re/vm/big-bug-bang-profiles`. The default CB sources are English and BBB sources
are French; this step does not translate or substitute the runtime localization.
`--cb-source`, `--bbb-source`, and `--analyzer` allow explicit alternate inputs.

The output contains an `index.md`, a hashed `catalog.json`, and per-profile:

- `source.blood`: the exact analyzed source, including named conditions.
- `graph.json`: every COD instruction, block, conditional edge, text site,
  deferred profile request, BAS selector list, and local menu-to-selector link.
- `dialogue.md`: readable lines grouped by COD procedure or BAS object/selector.
- `cod.dot` and, for CB, `bas.dot`: Graphviz graphs. Frame-resume edges are dashed
  and remain distinct from immediate control transfers.

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
source/analyzer/exporter hashes, and refuses to overwrite existing output.

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

Direct asset-composited videos and actual gameplay recordings must remain
separately labeled. The existing recorder still records explicit runtime routes;
the static scanner does not yet automatically turn every graph branch into a
video. Both catalogs leave unrecorded content unrecorded rather than equating
script/file presence with video coverage.

## Tests

```sh
uv run python -m unittest discover -s tools -p test_dialogue_catalog.py
nix develop -c cargo test --release -p commander-blood-script-compiler \
  --test dialogue_catalog
nix develop -c uv run --with av==18.1.0 python -m unittest discover \
  -s tools -p test_video_anthology.py
nix develop -c cargo test -p commander-blood-game --lib
nix develop -c cargo test -p commander-blood-game --lib \
  recording::tests::lossless_writer_preserves_frames_audio_and_shared_start_offset \
  -- --ignored --exact
```

PCM packet muxing uses [PyAV audio frames](https://pyav.org/docs/stable/api/audio.html)
and [containers](https://pyav.org/docs/stable/api/container.html); clock correction
uses FFmpeg's `aresample` filter. Captured media and imported game assets are not
checked into the repository.
