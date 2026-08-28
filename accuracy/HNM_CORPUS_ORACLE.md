# HNM production-corpus oracle

The HNM decoder gate compares authored production payloads against the original
`BLOODPRG.EXE` routines under Unicorn. It is not a comparison against the
recovered C model and it is not a Rust self-round-trip.

The loose resource corpus currently contains:

- 701 HNM files
- 39,391 frame entries
- 2,740 AB-compressed payloads
- 36,320 AD-compressed payloads
- 21,721 AD payloads eligible for deferred transparent-rectangle presentation

Build the Rust side of the gate:

```sh
nix develop -c cargo build -p commander-blood-game --bin hnm-corpus-trace
```

Compare every payload and its compressed-source progress:

```sh
nix develop -c python re/tools/compare_hnm_decoder_corpus.py output/_tmp_dat --codec ab
nix develop -c python re/tools/compare_hnm_decoder_corpus.py output/_tmp_dat --codec ad
```

Compare transparent rectangle staging and complete seeded framebuffer state:

```sh
nix develop -c python re/tools/compare_hnm_decoder_corpus.py output/_tmp_dat --rect
```

The comparison repeats ordinary payload decoding with zeroed and patterned
destination memory. This detects authored streams whose output depends on
reusable 64 KiB destination history.

## Certified boundary

As of 2026-08-27, every AB payload, every AD payload, and every eligible
transparent rectangle matches the original executable. No shipped payload is
destination-history-sensitive under the two deterministic seeds.

This gate certifies decompression and isolated transparent rectangle
composition. It does **not** certify runtime HNM selection, queue ordering,
palette state at frame presentation, other overlays, timing, or final RGB
output. Those layers require a synchronized runtime frame oracle.
