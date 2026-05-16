# Commander Blood Tools

Rust tools for extracting and reverse-engineering Commander Blood media
combinations.

## Commands

Run through the flake so `ffmpeg`, `7z`, `curl`, and Rust are all on `PATH`:

```sh
nix develop --command cargo run -- <output-dir>
nix develop --command cargo run -- inspect-descript /path/to/DESCRIPT.DES
nix develop --command cargo run -- inspect-scripts /path/to/extracted-iso
nix develop --command cargo run -- inspect-character-combinations /path/to/extracted-iso
```

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
function-bounded `script-disassembly.tsv` and decodes every valid
`0xa6 ... 0x80 ... 00 00` text call in `SCRIPT*.COD` by following dictionary
word offsets from `SCRIPT*.DIC`. Actor context is tracked from `0xc4 <u16>`
object references where those references match DESCRIPT character talk slots.

The normal full exporter no longer emits guessed all-clips character composites
when script speech data is available. It exports script-derived dialogue groups;
the old static `char_contents` table remains only as a direct `--snd` fallback
for manual inspection.

Character foreground HNM compositing uses a character-specific zero-clear decode
path. Zeros inside character update rectangles clear back to transparency, which
prevents stale frame-0/update pixels from sticking on the background while
leaving standalone HNM decoding unchanged.

MP4 output is encoded at 3x the original 320x200 game viewport using nearest
neighbor scaling, so generated videos are 960x600 while preserving hard pixel
edges.

Subtitle SFX is mixed during the animated text reveal using the short
`sn/tb.snd` UI bleep clips, rather than playing only one click at cue start.
The renderer uses the custom dialogue bitmap font embedded in `BLOODPRG.EXE`:
ASCII map at file offset
`0x14c22`, glyph advances at `0x14cd2..0x14d27`, and 8-byte glyph bitmaps at
`0x14d28`. This matches the square-stroke subtitle font visible in game
screenshots.

`CHART.FD`, `ORX.FD`, `FRIGO.FD`, and `TB.BIG` are present in the CD root.
The `.FD` files are full-screen IFF PBM images, not font data. `BLOOD.DAT`
uses a fixed table of null-padded names, little-endian sizes and offsets, and
contains the `FD\*.LBM` static backgrounds plus `SN\TB.SND`.
