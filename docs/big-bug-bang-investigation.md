# Big Bug Bang Compatibility Investigation

Inspected 2026-09-05. This is a comparison of original disc contents, not a
claim that Big Bug Bang is playable in the Rust runtime.

## Conclusion

**Same engine family and extensive exact asset reuse, but not the same native
engine build.** A shared Rust engine is a reasonable direction. Loading the
sequel's scripts into the current Commander runtime is not sufficient: the
sequel adds native VM operations, changes native modules and object data, and
uses different executable-resident catalogs.

No download was necessary. The user already had the ISO locally. Neither
original images nor extracted game content is included in this commit.

## Inputs and Reproduction

- Big Bug Bang: `/home/ben/Downloads/BigBugB.iso`, 487114752 bytes, volume
  `BIGBUGBANG`, SHA-256
  `c608f2bc3a47eb74500fb7aa42d23eaffb33689b085555e223686bfac0c0c742`.
- Commander Blood: `output/CMDR_BLOOD.iso`, a symlink to
  `/home/ben/Music/commander-blood/CMDR_BLOOD.iso`, 498102272 bytes, volume
  `CMDR_BLOOD`, SHA-256
  `63dad9490200d627ce5b245c361ae95e3bb1193600958a24b55bd0162ebb320d`.
- Extracted inputs: `output/big-bug-bang/disc` and
  `output/big-bug-bang/commander-disc`.
- Detailed machine-readable findings: `output/big-bug-bang/comparison.json`.
- Locally generated script listings: `output/big-bug-bang/vm-probes`.

```sh
7z x /home/ben/Downloads/BigBugB.iso -ooutput/big-bug-bang/disc -aos
7z x output/CMDR_BLOOD.iso -ooutput/big-bug-bang/commander-disc -aos
nix develop -c cargo build --bin cbvm
python3 re/tools/compare_game_discs.py \
  output/big-bug-bang/commander-disc output/big-bug-bang/disc \
  output/big-bug-bang/comparison.json --cbvm target/debug/cbvm
python3 -m unittest discover -s re/tools -p test_compare_game_discs.py
```

Use original discs, **not** `output/recovered_dos_package/cd/BLOOD.DAT`:
the latter contains rebuilt C-port XDBs and would give misleading differences.
The original Commander archive has 974 directory records representing 945
unique names. Its duplicate names have identical payloads; the comparison
collapses these. It rejects conflicting duplicate payloads rather than silently
choosing one. Counts below describe unique named resources, not directory rows.

## Exact Reuse

| Comparison | Commander | Big Bug Bang | Same-name byte-identical |
| --- | ---: | ---: | ---: |
| Loose files, excluding BLOOD.DAT | 133 | 174 | 99 |
| Unique archive resources | 945 | 944 | 904 |
| DESCRIPT named records | 145 | 230 | Not compared by record |
| COD script files | 5 | 17 | 0 |

Of Big Bug Bang's archive resources, **95.8% match by name and bytes**. Including
renamed matches, 907 resources reuse original content: 421585127 of 479885815
payload bytes, **87.9% by bytes**. These are reuse measurements, not an engine
compatibility or port-completion percentage.

The 99 identical loose files consist of 50 `.EXT` environment files, 44 `.SPR`
sprite files, three `.FD` files, `TB.BIG`, and the small `BLOOD.EXE` launcher.
This establishes exact reuse of major environment, sprite and bridge assets.

Only four shared archive names have different content: `AMER.XDB`,
`CROOLIS.XDB`, `SQ\MICROFOL.HNM`, and `SQ\THE_STAR.HNM`.
There are 36 sequel-only names and 37 Commander-only names. The additions
include `FIN1.HNM` through `FIN14.HNM` and different music resources.
Three apparent additions are renamed copies:

- `OB\BARROW10.HNM` matches `PL\BARROW10.HNM`.
- `XXXXXXXX.XXXLBM` matches `FD\GLACIA1F.LBM`.
- `XXXXXXXX.XXXNM` matches `PE\TRMG_TR.HNM`.

Original archive hashes:

```text
Commander: 7ab01bb61d1a20dbc41afda6155fc1b49312074c3b5dbb27d57ac299d278cb81
Sequel: d5555d510f162590ad118de51221cf04a33e14d69dcd115fe3f183f6f2b93ad8
```

## Native Code

| Component | Commander bytes | Sequel bytes | Result |
| --- | ---: | ---: | --- |
| Main executable | 86680 | 98190 | BLOODPRG.EXE becomes BLOOD2PG.EXE; changed |
| MANU3.XDB | 62544 | 62544 | Entire file identical |
| AMER.XDB | 266800 | 266880 | Changed |
| CROOLIS.XDB | 258832 | 258832 | Changed despite identical length |
| SCRUT.XDB | 258080 | Absent | Not shipped in sequel archive |
| DNSDB.DRV | 2734 | 2734 | Identical |
| NOSOUND.DRV | 285 | 285 | Identical |

The changed XDBs differ in their code prefixes as well as their larger data
areas. File-size differences are not a count of changed instructions: shifting
code/data can produce many same-offset byte differences. A function-level
comparison of these two modules remains necessary.

Both main programs are DOS MZ executables. Commander has a 1536-byte header and
367 relocations; the sequel has a 2048-byte header and 436 relocations. The
sequel's examined VM paths use 16-bit addressing with 386 instructions and GS
accesses, consistent with the original architecture. This is not a compiler
identification or a complete executable audit.

### Confirmed VM Extensions

The sequel's dispatcher at file `0x5B65` loads table offset `0x7288`, then
dispatches through `GS:[BX+DI]` at `0x5B7B`. The table is at file `0x16A78`;
its near code offsets resolve relative to file `0x5820`. Independently, its
instruction-skip routine at `0x68C8` loads descriptor offset `0x72FA`, placing
the descriptor table at file `0x16AEA`.

The comparison tool guards these offsets by the executable's full SHA-256:
`4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834`.
Unknown builds are reported without a fixed-offset VM interpretation.

The existing A0-D2 descriptor pairs are unchanged. The sequel adds five real
handlers where Commander has a null table terminator and adjacent data:

| Opcode | Sequel handler file offset | Encoded bytes | Observed behavior |
| --- | --- | ---: | --- |
| `0xD3` | `0x7408` | 9 | Read variable, multiply and divide by immediate/indirect operands, write quotient |
| `0xD4` | `0x70CD` | 5 | Two-word operation selecting and updating object state; full semantics pending |
| `0xD5` | `0x7367` | 3 | One-word operation scanning object state; full semantics pending |
| `0xD6` | `0x728B` | 5 | Two-word operation with object-field clamps/arithmetic; full semantics pending |
| `0xD7` | `0x6E67` | 1 | Set native state byte at GS:0x6B73 to 1; downstream meaning pending |

The following D8 handler entry is null. Widths above agree with the native
skip descriptors and handler operand reads. **Do not infer that A0-D2 semantics
are unchanged just because their operand-length descriptors match.**

## Script and Scene Compatibility

The current `cbvm` executable was rebuilt before the final probe:

- Sixteen of the seventeen `.COD` files have zero raw spans under the existing
  Commander decoder and reassemble byte-for-byte.
- `SCRIPT2.COD` does not decode completely: 35477 of 40794 bytes remain raw.
  An extension sequence starts at `0x1458`; the Commander-specific D5 descriptor
  misreads it as a long operation and loses synchronization. The first raw span
  begins at `0x14C5`. This is not evidence that all remaining bytes are unknown
  instructions; the old dialect is simply the wrong decoder for this sequence.
- `SCRIPT2.BAS` round-trips by retaining 16229 of 19933 bytes as raw data. Its
  conversation structure needs separate analysis. `.BAS` is compiled binary
  data, not shipped BASIC source text.
- `DESCRIPT.DES` fully decompiles and recompiles byte-exactly: 230 records,
  2757 commands, 44676 bytes.

As a control, the same rebuilt tool decodes Commander's `SCRIPT2.COD` (39042
bytes) and `SCRIPT2.BAS` (22565 bytes) with zero raw spans and byte-exact
round trips. The comparison command also runs probes on all Commander profiles.

Even zero-raw-span disassembly is a syntax/encoding result, **not** proof of
correct branch targets, resource bindings, native effects or playability.
The raw-preserving listings are not a completed high-level decompilation.

Only `SCRIPT2.BAS` is present on this sequel disc, although the executable's
resource-name tables mention BAS files for all 17 profiles. Determine native
missing-file/retained-state behavior; do not invent 16 empty BAS resources or
assume shared BAS ownership without tracing the loader.

All `.DEB` sizes remain divisible by the familiar 20-byte symbol-record size.
Sequel `.VAR` files are 8368 bytes (16 files) or 8370 bytes (SCRIPT2), versus
4666-5428 bytes in Commander. Its symbol names include many more characters,
locations and game objects. Recover their record layouts and field mappings
from the sequel, not from Commander constants.

## English Localization

There are 13916 nonempty dictionary entries across 17 dictionaries, representing
7368 distinct byte strings. These are **not** 13916 independent sentences.
`DESCRIPT.DES` also contains ordinary French caption text, including the
intro's `Commander BLOOD Version 2.0` presentation. Native UI labels and any
text embedded in artwork/video need their own inventory.

Recommended approach: keep a byte-exact French baseline; extract complete
dialogue/caption messages into a UTF-8 localization catalog keyed by stable
game/profile/message identity. Translate full messages with speaker/context,
not words independently. Preserve object identifiers, dialogue-option identity,
resource names and VM references even when their visible text changes.
Support English layout and fonts at the rendering boundary. Reuse original
audio initially, with English subtitles; inventory intelligible speech before
making any dubbing promise. English translated scripts cannot retain original
French byte checksums; use separate source and localization manifests.

## Implementation Order

1. Add explicit game/version profiles for archive identity, script catalogs,
   native tables, field layouts and separate cache/save/checksum namespaces.
   Current `src/vm_bundle.rs` hardcodes five complete profiles;
   `crates/commander-blood-game/src/runtime.rs` decodes Commander-specific native
   catalogs. Do not silently reuse those offsets with BLOOD2PG.EXE.
2. Recover D3-D7 completely, compare inherited handlers, decode the new BAS
   structure and trace profile loading. Require semantic coverage plus
   byte-exact French script/scene rebuilds; raw-byte preservation is not a pass.
3. Diff AMER/CROOLIS by routine and data ownership. Compare the changed native
   game-state, simulation, presentation and UI paths. Reuse identical MANU3 and
   verified asset loaders, while validating new media inputs separately.
4. Establish original Big Bug Bang DOS captures/state traces for startup,
   conversation, travel and the added gameplay. Compare Rust event/state/frame
   sequences against those, not against Commander or screenshots alone.
5. Add the English catalog after the French behavior is verified. Test both
   games continuously so sequel features cannot change Commander behavior.

No Rust gameplay changes, sequel runtime launch, full native routine matching,
full French script recovery or translation were performed in this investigation.
