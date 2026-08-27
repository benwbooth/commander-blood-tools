# Commander Blood Tools

Rust tools for extracting and reverse-engineering Commander Blood media
combinations.

Longer term, this project is becoming a Rust reimplementation of Commander
Blood's DOS engine that runs the original English CD data files. The current
media exporter is the first vertical slice for that work: it exercises data-file
parsing, script recovery, rendering, audio, and real-game oracle comparison.

See [docs/decompilation-roadmap.md](docs/decompilation-roadmap.md) for the full
reverse-engineering and Rust reimplementation plan.

## Rust Game Port

The modern SDL3/wgpu executable imports the original installation into a
versioned loose-asset store on first launch. `BLOOD.DAT` is read only by this
one-time importer; subsequent game processes load ordinary files through
`assets-v1/manifest.json` and never open the archive.

```sh
nix develop --command cargo run -p commander-blood-game --bin commander-blood -- \
  --data /path/to/original/commander-blood
```

The default immutable asset cache is
`$XDG_DATA_HOME/commander-blood/assets-v1`, falling back to
`~/.local/share/commander-blood/assets-v1`. Set `CBLOOD_ASSET_CACHE` to select
another location. Saves remain separate and use `CBLOOD_WRITE_DATA` or the
parent Commander Blood user-data directory.

The manifest records every effective DOS resource, original archive or loose
origin, exact byte count, SHA-256 digest, and media kind. HNM, SND, and VOC files
remain lossless original resources in this first normalized format so palette,
frame-timing, subtitle, music, chatter, and effect behavior can still be checked
against the recovered decoders before standard media derivatives replace them.

## Commands

Run through the flake so `ffmpeg`, `7z`, `curl`, and Rust are all on `PATH`:

```sh
nix develop --command cargo run -- <output-dir>
nix develop --command cargo run -- inspect-bloodprg [re/bin/BLOODPRG.EXE]
nix develop --command cargo run -- inspect-vm /path/to/SCRIPT1.COD [/path/to/SCRIPT1.VAR]
nix develop --command cargo run --bin cbvm -- decompile-descript /path/to/DESCRIPT.DES re/descript/DESCRIPT.descript
nix develop --command cargo run --bin cbvm -- compile-descript re/descript/DESCRIPT.descript /tmp/DESCRIPT.DES /path/to/DESCRIPT.DES
nix develop --command cargo run --bin cbvm -- decompile-bundle /path/to/game re/vm/source
nix develop --command cargo run --bin cbvm -- decompile-unified /path/to/game re/vm/profiles
nix develop --command cargo run --bin cbvm -- compile-profile re/vm/profiles/script1.blood /tmp/script1
nix develop --command cargo run --bin cbvm -- compile-bundle re/vm/profiles /path/to/game /tmp/cbvm-bundle
nix develop --command cargo run --bin cbvm -- build-runtime-tree re/vm/profiles /path/to/extracted-cd /tmp/cblood-runtime
nix develop --command cargo run --bin cbvm -- analyze-contact-manifest /path/to/game re/vm/contact-manifest
nix develop --command python3 -P re/tools/runtime_scenario_matrix.py --cd-dir output/recovered_dos_package/cd --install-parent accuracy/cblood_install --all-contacts --jobs 4
nix develop --command python3 -P re/tools/runtime_watchdog.py --cd-dir output/recovered_dos_package/cd --install-parent accuracy/cblood_install --executable BPRG_RE.EXE --display :0 --seconds 900 --report output/manual-watchdog.json
nix develop --command python3 -P re/tools/focus_loss_probe.py --cd-dir output/recovered_dos_package/cd --install-parent accuracy/cblood_install --output output/focus-loss-probe.json
nix develop --command python3 -P re/tools/runtime_scenario_matrix.py --cd-dir output/recovered_dos_package/cd --install-parent accuracy/cblood_install --executable BLOODPRG.EXE --link-map re/bin/BLOODPRG.segments.map --all-contacts --jobs 4 --output-dir output/contact-matrix-original
nix develop --command python3 -P re/tools/compare_runtime_scenario_matrices.py --candidate output/contact-matrix-rebuilt/matrix.json --reference output/contact-matrix-original/matrix.json --reference-retry output/contact-matrix-original-retry/matrix.json --output output/contact-matrix-differential.json
nix develop --command cargo run -- inspect-descript /path/to/DESCRIPT.DES
nix develop --command cargo run -- inspect-scripts /path/to/extracted-iso
nix develop --command cargo run -- inspect-character-combinations /path/to/extracted-iso
```

`inspect-bloodprg` emits a Rust-validated map of the actual DOS MZ binary:
header math, known reverse-engineered symbols, the script VM opcode descriptor
table, the opcode handler table, a named opcode-family map, and the embedded
dialogue font tables.
`inspect-vm` emits the reverse-engineered compiled-BASIC token stream, plus
bounded interpreter line-state snapshots when a matching `SCRIPT*.VAR` is
provided.
`cbvm decompile-bundle` emits parseable lossless assembly for all five `.COD`
and five `.BAS` VM images, then assembles each result and requires byte equality.
`cbvm decompile-descript` emits readable typed source for the shared presentation
database and requires an internal byte-exact round trip. `cbvm compile-descript`
can independently compare the generated image with the shipped `DESCRIPT.DES`.
`cbvm decompile-unified` emits one BloodScript v8 profile for each five-resource
COD/BAS/DEB/DIC/VAR bundle and recompiles all five outputs byte-exactly before
writing source. See [re/vm/README.md](re/vm/README.md) for the verification
ladder and [re/vm/language-evidence.md](re/vm/language-evidence.md) for the
boundary between recovered facts and reconstructed source syntax.
`cbvm compile-bundle` compiles the five unified profiles into all 25 resources,
refusing any result that differs from the shipped resource. `cbvm
build-runtime-tree` installs that bundle into a hard-linked extracted-CD tree
for DOSBox substitution testing.
`cbvm analyze-contact-manifest` derives every COD contact procedure, its complete
entry predicate region, presentation object, activation state, exact subtitle
word-list offsets, and choices directly from COD/DEB/DIC plus the recovered CFG.
`runtime_watchdog.py` is the DOS guest crash recorder. It continuously checks
segment, IVT, MCB, VM, audio, presentation, input, and execution-liveness state;
captures DOSBox fatal diagnostics; and writes `guest.bin` plus `context.json`
beside the report on the first detected fault. Send `SIGUSR1` to the
`recorder.watchdog_pid` recorded in the live report to snapshot an ambiguous
manual freeze immediately.
`focus_loss_probe.py` runs that watchdog while moving X focus to an isolated
window and back. It requires the guest timer, watchdog sampling, mouse state,
and every memory-integrity gate to remain healthy before, during, and after the
focus transition.
`inspect-descript` emits typed JSON for `DESCRIPT.DES`.
`inspect-scripts` emits typed JSON for `SCRIPT*.DEB`, `SCRIPT*.VAR`,
`SCRIPT*.DIC`, and recovered speech bytecode events.
`inspect-character-combinations` emits the script-derived
foreground/background/music combinations as TSV.

## Current Findings

`DESCRIPT.DES` is parsed into 145 records: 64 locations, 35 characters, 35
objects, and 11 sequences. The parser currently has zero real unknown opcodes
against the English CD data. Opcode `0x08` appears once on every location record
as the constant two-byte value `0x0023`; it is preserved as metadata and does
not affect media selection.

The script parser recovers character contexts from `SCRIPT*.DEB` object symbols
plus the object location field in `SCRIPT*.VAR`. It also emits a
function-bounded `script-disassembly.tsv`, a branch-aware
`script-branch-trace.tsv`, an initial-state executed dialogue trace
`script-executed-dialogue.tsv`, branch-decision/coverage summaries
`script-text-flags.tsv`, `script-branch-decisions.tsv`,
`script-branch-coverage.tsv`, `script-branch-scenarios.tsv`,
branch-scenario executed dialogue manifests `script-branch-scenario-dialogue.tsv` and
`script-branch-scenario-dialogue-runs.tsv`, initial-state executed dialogue runs
`script-executed-dialogue-runs.tsv`, renderer scene-event streams
`script-scene-events.tsv`, `script-profile-scene-events.tsv`, and
`script-branch-scenario-scene-events.tsv`, VM-order `script-dialogue-runs.tsv`,
and every valid `0xa6` TEXT token in `SCRIPT*.COD` with the VM token walker by
following dictionary word offsets from `SCRIPT*.DIC`. Actor context is tracked
from the binary-sized `0xc4` actor/object tokens where those references match
DESCRIPT character talk slots.
The full export also emits `bloodprg-snd-call-sites.tsv`,
`bloodprg-render-call-sites.tsv`, and `bloodprg-sprite-blitters.tsv`,
binary-derived maps of direct audio/render call sites and the internal sprite
blitter dispatch modes now being ported into named engine behavior.
The run-level dialogue manifests append unresolved actor, background, and voice
counts so remaining presentation gaps are visible instead of hidden by fallback
combinations.
The scene-event manifests also emit explicit `unresolved_background`,
`unresolved_actor`, and `unresolved_voice` rows at the VM line where context is
missing; `0x00`/`0xff` voice selectors are treated as deliberate silent channels,
not unresolved clips.

The normal full exporter no longer emits per-character composites from the SND
pass. It exports branch-aware initial-state executed dialogue groups in VM
sequence order, including run-level composites that can switch actor voice banks
inside one scene. The old static `char_contents` table remains only as a direct
`--snd` fallback for manual inspection; the default export no longer writes the
legacy `script-dialogue-videos.tsv` per-character video manifest or fills
unresolved `character-combinations.tsv` backgrounds from that static table.

Character foreground HNM compositing uses a character-specific zero-clear decode
path. Zeros inside character update rectangles clear back to transparency, which
prevents stale frame-0/update pixels from sticking on the background while
leaving standalone HNM decoding unchanged.

MP4 output is encoded at 3x the original 320x200 game viewport using nearest
neighbor scaling, so generated videos are 960x600 while preserving hard pixel
edges.

Subtitle SFX follows the recovered line-complete dialogue state: after each
subtitle finishes revealing, the renderer mixes `sn/tb.snd` clip 0 once.
SND banks are parsed through the recovered `BLOODPRG.EXE` clip-player model in
`src/snd.rs`: AX selects the original clip index, the bank table resolves the
clip body, the 6-byte clip header is skipped, and the sample-rate byte controls
unsigned 8-bit PCM playback.
The renderer uses the custom dialogue bitmap font embedded in `BLOODPRG.EXE`:
ASCII map at file offset
`0x14c22`, glyph advances at `0x14cd2..0x14d27`, and 8-byte glyph bitmaps at
`0x14d28`. This matches the square-stroke subtitle font visible in game
screenshots.

`CHART.FD`, `ORX.FD`, `FRIGO.FD`, and `TB.BIG` are present in the CD root.
The `.FD` files are full-screen IFF PBM images, not font data. `BLOOD.DAT`
uses a fixed table of null-padded names, little-endian sizes and offsets, and
contains the `FD\*.LBM` static backgrounds plus `SN\TB.SND`.
