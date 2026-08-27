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

## Video Gate

HNM is not treated as a simple sequence of opaque RGB frames. The runtime uses:

- indexed frame pixels and per-stream palette changes;
- delta updates that depend on prior decoder state;
- transparent character layers composited over another scene;
- authored frame cadence and subtitle synchronization.

A direct WebM transcode would flatten those semantics and could reintroduce the
same presentation inaccuracies the reverse engineering removed. Before runtime
video switches formats, the importer must classify each HNM use, serialize
palette and timing metadata, preserve alpha for composited layers, and compare
every normalized decoded frame against the existing HNM decoder.

For opaque video, a lossless Matroska/FFV1 master is the likely verification
format and WebM/AV1 is a possible playback derivative. Transparent and
palette-driven streams need an explicit sidecar or frame representation first.
