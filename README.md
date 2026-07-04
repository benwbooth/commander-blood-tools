# Commander Blood Tools

Rust tools for extracting and reverse-engineering Commander Blood media
combinations.

Longer term, this project is becoming a Rust reimplementation of Commander
Blood's DOS engine that runs the original English CD data files. The current
media exporter is the first vertical slice for that work: it exercises data-file
parsing, script recovery, rendering, audio, and real-game oracle comparison.

See [docs/decompilation-roadmap.md](docs/decompilation-roadmap.md) for the full
reverse-engineering and Rust reimplementation plan.

## Commands

Run through the flake so `ffmpeg`, `7z`, `curl`, and Rust are all on `PATH`:

```sh
nix develop --command cargo run -- <output-dir>
nix develop --command cargo run -- inspect-bloodprg [re/bin/BLOODPRG.EXE]
nix develop --command cargo run -- inspect-vm /path/to/SCRIPT1.COD [/path/to/SCRIPT1.VAR]
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
