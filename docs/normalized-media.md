# Normalized Media

The modern runtime uses two immutable, versioned layers below the asset cache.

## Canonical Assets

`assets-v1` is a lossless import of the effective DOS installation. It resolves
archive entries and same-name loose-file overrides once, records their hashes in
`manifest.json`, and stores every resource as an ordinary file. Runtime code
never opens `BLOOD.DAT`.

These bytes remain the fidelity oracle. A derivative may replace a runtime read
only after tests prove it preserves every behavior the game obtains from the
source format.

## Standard Audio

`assets-v1/media-v1` is generated atomically and keyed by the SHA-256 digest of
the canonical asset manifest. A stale, interrupted, or source-mismatched cache
is regenerated.

Every VOC resource becomes one RIFF/WAVE file. Every SND clip becomes one
RIFF/WAVE file below a directory named for its bank and zero-based clip index.
The conversion is lossless unsigned 8-bit mono PCM: decoded sample bytes and
source sample rates are validated against the generated files before the cache
is installed.

Production navigation music and standalone voice playback use WAVE derivatives.
SND bank headers still supply authored clip counts and dialogue delay bounds, so
the canonical loose SND resource remains the metadata source until those fields
move into the normalized manifest. The WAVE exports already cover every shipped
SND clip.

Ogg Vorbis or Opus is not the canonical audio format because both are lossy.
They may be optional distribution derivatives later, but they cannot replace the
exact WAVE oracle.

## Lossless Video

`assets-v1/media-v1/video-v1` contains verified VP9/WebM derivatives for every
HNM resource. HNM is not flattened into only an opaque RGB movie. Each source
produces:

- `NAME.webm`: lossless full-range `gbrp` RGB for viewing and future upscaling;
- `NAME.index.webm`: lossless palette indices repeated across the three planes;
- `NAME.mask.webm`: lossless ownership values, 255 for authored pixels and 0
  for pixels inherited from the underlying scene;
- `NAME.json`: six-bit palette updates, embedded sound records, service-call
  positions, queue metrics, dimensions, frame counts, and stream hashes.

The importer runs the recovered HNM presentation decoder twice. One trace starts
with the default palette and zero-filled framebuffers; the other starts with the
complement palette and 255-filled framebuffers. Values that converge were
authored by the stream. Values that remain different depend on prior scene state
and are excluded from RGB/index output by the ownership mask. This preserves
delta frames and transparent character overlays without filename heuristics.

Each WebM is encoded as lossless VP9 profile 1, decoded back to planar bytes by
FFmpeg, and compared with the exact pre-encode SHA-256 stream hash. The complete
cache is installed atomically only after all source hashes, output hashes,
metadata, and frame counts validate. WebM timestamps use a nominal 25 fps for
tool compatibility and are explicitly non-authoritative; the recorded game
service positions and runtime audio/software clocks remain the timing oracle.

Gameplay still reads canonical loose HNM resources. Switching playback to the
index and mask streams is a separate parity-gated step: the normalized reader
must reproduce framebuffers, palettes, sound records, queue metrics, and scene
transitions before HNM decoding can be removed from production.
