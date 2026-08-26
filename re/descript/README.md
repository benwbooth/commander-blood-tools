# DESCRIPT presentation source

`DESCRIPT.descript` is the canonical editable source for the shared
`DESCRIPT.DES` presentation database. It contains all 145 records and all 1,221
ordered commands in the shipped file.

The source exposes four semantic record types:

- `location` records define captions, four named-view LBM backgrounds, approach HNMs,
  vertical placement, and music.
- `character` records define talk and idle HNMs over `front`, `right`, `left`,
  `back`, or no background, plus right/left character videos, portrait sprites,
  and SND banks.
- `sequence` records define ordered HNMs, frame-timed subtitles, and music.
- `object` records define inventory and world-object HNMs.

Record and command order are significant. Directory offsets, record lengths,
16-byte name padding, binary opcodes, inter-record kind markers, and the final
database marker are compiler-derived and intentionally absent from source.
The source never exposes the binary's opaque background values 1 through 4.
Instead, each location binds the readable views `front`, `right`, `left`, and
`back` directly to its LBM files, and character clips select those views with
syntax such as `talk "homm_01.hnm" over right`. The compiler translates those
view names to the original byte values. Rows and subtitle frames remain decimal
because they are magnitudes rather than binary identities.

Generate canonical source and require an internal byte-exact round trip:

```sh
cargo run --bin cbvm -- decompile-descript \
  accuracy/cblood_install/cblood/DESCRIPT.DES \
  re/descript/DESCRIPT.descript
```

Compile it and compare it independently with the installed game:

```sh
cargo run --bin cbvm -- compile-descript \
  re/descript/DESCRIPT.descript \
  /tmp/DESCRIPT.DES \
  accuracy/cblood_install/cblood/DESCRIPT.DES
```

The canonical-source test checks both directions: source compilation must match
all 19,234 shipped bytes, and decompilation must reproduce the checked-in source
exactly.
