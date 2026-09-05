# Big Bug Bang Rust Port

## Objective

One Rust engine must run Commander Blood and Big Bug Bang, with English
localization for Big Bug Bang. Preserve each game's original behavior, keep
runtime memory flat and owned, reuse SDL3/wgpu and imported RGB assets, and
ship no external executable dependency. Test/oracle tools are separate from
runtime dependencies.

The objective is active and **not complete**. Big Bug Bang cannot yet be launched
through the production loader. The original-disc investigation is in
`big-bug-bang-investigation.md`; its initial decoder limitations describe the
state before the implementation below.

## Verified Implementation

### Explicit COD Dialects

`commander-blood-formats::code::ScriptDialect` selects the recovered instruction
boundaries. Existing entry points default to Commander Blood unchanged.
`decode_script_code_for_dialect` exposes Big Bug Bang framing. Tokens retain
their dialect so an adjacent-data byte from Commander cannot become a sequel
instruction merely because the numeric opcode matches.

Big Bug Bang A0-D2 descriptor pairs match Commander. D3-D7 use lengths 9, 5,
3, 5, and 1 in both query and normal modes, verified against the sequel's
native table at file 0x16AEA. Adjacent-data interpretation after D7 remains
unsupported rather than borrowing Commander's unrelated executable data.
All 17 original sequel COD images frame without raw fallback and re-encode
byte-for-byte. All five Commander images retain their existing token counts
and exact round trips. This is **instruction framing**, not full semantic
recovery, high-level script compilation, or runtime parity.

### D3 Multiply/Divide

The original handler at BLOOD2PG.EXE 0x7408-0x744A has a typed instruction and
ordinary Rust implementation, wired into production instruction dispatch:

```text
target = (unsigned_32(target) * multiplier) / divisor
```

Operands with mode C0 or C2 read VAR words; other modes supply immediate words.
All reads precede the destination write. Query mode does not suppress the write
or branch. Division by zero and quotient overflow are errors that leave state
unchanged, corresponding to the original DIV exception rather than silently
wrapping or saturating.

`re/tools/big_bug_bang_vm_oracle.py` executes the original handler, guarded by
the executable SHA-256, to generate 114 synthetic reference cases. The Rust
test compares the entire state buffer with those results, including 41 native
divide errors, aliasing, unsigned boundary values, and both query modes. The
fixture contains input/output data, not original game machine-code bytes.
Unicorn is used only by this offline oracle; the Rust game does not emulate
registers or segmented memory.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_vm_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_multiply_divide.jsonl
nix develop -c cargo test -p commander-blood-formats code::tests -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib big_bug_bang_multiply_divide
```

The sequel corpus test is explicitly ignored unless requested because it
requires local original-disc assets. Its absence must not be counted as a pass.
The synthetic D3 reference test and dialect boundary tests run normally.

Verification for this slice (2026-09-05): all 106 formats library tests passed
with ignored tests explicitly enabled; game library tests passed 882 with five
unrelated platform/oracle tests ignored; `cargo check -p commander-blood-game
--all-targets` passed. These checks do not prove sequel playability.

`cargo check --workspace --all-targets` fails in the existing script-compiler
wrapper's test build: shared `src/vm.rs`, `ship3d.rs`, `font.rs` and `descript.rs`
tests import root tools modules such as `recomp` and `bridge` that the narrow
wrapper does not expose. Those source files and the wrapper are unchanged in
this slice. Keep this as a separate test-ownership repair; do not disable tests
or count the failed workspace-wide gate as passing.

### D6 Actor Growth

The native D6 handler (file 0x728B-0x7366) and its selection helper
(0x706E-0x70CC) now have a flat, typed Rust implementation. The decoder treats
both operands as immediate words: a group mask and a signed growth rate.
All 39 authored occurrences are in SCRIPT2.COD; the other 16 profiles have none.

The helper selects actors in directory order with intersecting group flags,
both in-play/participating flags, an active location, and a location other than
the specially bound `Trashlando`. The handler clamps aggressiveness even for
engaged actors, then skips their growth update. Other selected actors receive
the recovered pressure, growth-balance and quantity arithmetic. These names
describe the observed calculations, not recovered original source identifiers.

Important native details preserved by the Rust implementation:

- Query mode does not suppress updates or consume a branch.
- Pressure relief has an upper clamp but no lower clamp.
- The balance calculation wraps at 16 bits before its signed clamp.
- Negative balance halves the unsigned quantity. Nonnegative balance uses two
  low-32-bit signed products followed by division of a zero-extended numerator:
  the native code explicitly clears EDX before IDIV. Replacing this with ordinary
  signed mathematical division changes negative-rate behavior.
- Growth has a minimum increment of one, including when its rate is zero.
- Final quantity addition wraps at 16 bits before a signed minimum of five.
- A word-DIV overflow preserves earlier actor updates and preceding clamps on
  the faulting actor. It is not an all-or-nothing state transaction.

`re/tools/big_bug_bang_growth_oracle.py` executes the complete original handler
and helper without replacing calls. Its 126 synthetic input/output vectors
cover selection, inactive locations, the excluded location, engaged actors,
both query modes, countdown gating, integer boundaries, negative rates and
18 divide faults. Tests compare the full VAR buffer, including partial fault
effects. The oracle also checks that the directory and all seeded globals are
unchanged. Neither game machine code nor original authored state is included
in the committed vectors.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_growth_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_growth.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_growth -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_growth
```

Production typed dispatch now handles D6 but requires an explicit
`SequelSimulationContext` from its host. Missing context is an error, not a
synthetic zero countdown. Tests exercise that dispatch boundary, clock gating
and query-mode writes. **The production sequel host is not yet implemented.**
The native main loop decrements GS:0x0CC6 at 0x10CA and reloads it from
GS:0x0CC4 at 0x5B46 after script/presentation processing. Those loop phases and
the speed-selection control still need integrating with the sequel runtime;
the handler must not run independently at the renderer's presentation rate.

Verification for this slice (2026-09-05): 110 formats tests passed with original
corpus tests explicitly enabled; 886 game-library tests passed with seven
unrelated/platform and original-table tests ignored. Game all-targets checking
and workspace library/binary checking passed. The original-handler vectors were
regenerated and compared byte-for-byte. These checks used the current worktree;
unrelated runtime edits remain outside the commit. This is D6 component and
dispatch verification, not a sequel playthrough or timing-parity claim.

### D5 Settlement

The D5 handler (file 0x7367-0x7407) now has a typed decoder and Rust
implementation. All 30 authored occurrences are in SCRIPT2.COD. Its immediate
group mask filters both source actors and relocated descendants; it is not a
VAR reference and is not implicitly replaced by the source actor's group.

The recovered path is:

1. Skip while the shared simulation countdown is nonzero. Otherwise select
   participating actors using the same 0x706E helper as D6.
2. Enable the maximum-range override. Require signed source quantity at least
   300 and a current location record.
3. Search active locations within the native squared range of 250, excluding
   the capitalized `Arche`. Choose the closest unoccupied location; ties retain
   the first directory entry. Source position resolution uses lowercase `arche`
   as its sentinel fallback. Candidate body coordinates are direct reads.
4. Collect active actor descendants of the source location in depth-first
   directory order, excluding `Honk`. This reuses the existing translated
   navigation collector and position resolver, now covered against their
   sequel counterparts in the complete D5 oracle.
5. Move matching descendants except the source actor. Copy the source's relief,
   assign quantity 10 and growth balance 1000. Only the first moved actor gets
   the participation flag. Mark the destination occupied and write the source
   actor into its new word at byte 24, not the first moved actor.
6. Clear the range override after processing. Query mode still performs the
   updates. A nonzero countdown preserves the previous override state.

Distance subtraction/absolute value wraps at 16 bits and the summed squares
are compared as signed 32-bit values, preserving the native overflow case.
The temporary candidate lists are owned vectors of object identities, not DOS
scratch-buffer or register emulation.

`re/tools/big_bug_bang_settlement_oracle.py` executes the original handler and
all seven helper entries it reaches: 0x706E, 0x6F17, 0x6F52, 0x67B8, 0x6633,
0x8103 and 0x685D. None of the calls are replaced. Its 100 synthetic graph cases
cover nested descendants, masks, source thresholds, flags, exclusions, equal
distances, radius boundaries, signed overflow, query mode and countdowns.
The fixture captures full VAR results and the observable range override;
the oracle separately rejects unexpected global writes outside the recovered
scratch areas. Reaching each helper is not a claim of covering every branch
inside each helper.

The same vectors pass through the production typed-dispatch implementation,
including record refresh. Every vector also verifies that omitted settlement
bindings produce an error without changing state. The production sequel host
still needs to supply these bindings and the real main-loop countdown.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_settlement_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_settlement.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_settlement -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_settlement
```

An inspection of all 17 original initial VAR/DEB pairs found zero active
actor candidates below location records before script initialization. Thus
running D5 on those initial snapshots alone would be weak evidence. Native
captures after initialization remain necessary for real-game state coverage,
including candidate-list bounds and repeated simulation updates.

Verification for this slice (2026-09-05): all 112 formats tests passed with
original corpus tests enabled; game-library tests passed 888 with seven
platform/original-table tests ignored. Game all-targets checking passed. The
100 original-handler vectors regenerate byte-for-byte. Tests ran in the
current worktree; unrelated Commander runtime edits remain outside this commit.
These checks do not establish production sequel startup or initialized-game
simulation parity.

### Sequel Records and Profile Ownership

The formats crate now decodes sequel VAR records with an explicit dialect:
actors own 74 bytes and locations 26, versus Commander's 72 and 24. All 17
original VAR, DEB and DIC images round-trip exactly, with 184 objects per
profile. Their entire active-object directory prefix is identical, not just
the first few entries. Original field-table comparisons cover all 22 selector
rows and nine shipped object kinds. The inherited 21 rows match Commander;
the additional row selects the actor word at byte 72. Its gameplay meaning
is not yet established, so the API does not invent a semantic name.

The resource cache can decode the sequel's 155-name catalog. The profile
manager can decode all 17 native rows and carry the dialect into code and
state decoding. It retains synchronized live VAR and timers across noninitial
sequel switches, releases the four other companions, and reloads initial VAR
when returning to profile zero. Repeating the initial selection retains live
VAR but resets timers, matching the native release/cache/reset conditions.
An out-of-catalog identity is rejected before changing the active profile.
Retained state requires matching active-object directories; a mismatch errors
instead of rebinding objects under different identities.

Synthetic, well-formed companion files test the real manager's resource and
timer lifecycle, including modified live state and repeated selections. These
are isolated test fixtures, not substitutes for the sequel's missing files.
The corpus and native-table tests require local original assets and are
explicitly ignored unless requested:

```sh
nix develop -c cargo test -p commander-blood-formats --lib -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

This implements loader components, **not production sequel startup**. Game
selection and dialect-aware runtime profile requests remain unwired, and the
strict BAS loader still rejects missing or unsupported dialogue data.

Verification for the record/profile slice (2026-09-05): 108 formats tests passed
with ignored corpus tests enabled, all four sequel-specific game tests passed
with original-table tests enabled, and the full game library passed 884 tests
with seven ignored (including the two separately run sequel table tests).
`cargo check -p commander-blood-game --all-targets` passed. These commands ran
in the current working tree; unrelated in-progress runtime changes were not
included in this commit.

## Native Ownership Evidence and Open Questions

Inspection of the sequel loader at file 0x5820 established a different load
order: VAR, DEB, COD, BAS, DIC. The name catalog starts at file 0xED94;
the 17 profile rows start at file 0xF744 and hold five two-byte resource IDs.
The first row contains IDs 2-6; resource 2 names SCRIPT1.VAR, not COD.

The selector uses FS:0x15B4 at 0x5853 and scales the profile index by ten.
For a nonzero requested profile, 0x5867-0x586D skips the first resource and
loads four instead of five. Its release path at 0x582E-0x5842 likewise releases
four, except when selecting zero. Thus VAR ownership persists across noninitial
profile switches. Do not reuse Commander's wholesale state replacement.
The native timer/state-table reset at 0x587C-0x588C is also initial-profile-only.

At 0x5A7D-0x5A97, resolved pointers follow the same order: the main COD loop
loads from GS:0x6AF4 at 0x5AAF, and the old-style conversation scanner still
loads BAS from GS:0x6AF8 at 0x5BBA. The loader's resource loop lacks Commander's
per-resource zero-result rejection. Only SCRIPT2.BAS is on the disc. Trace the
native failed-load and actual conversation-entry paths before defining the
meaning of missing BAS resources; do not synthesize empty files or assume that
the shipped SCRIPT2.BAS is used with the current profile dictionary.

The call to the old-style conversation scanner at 0x5E66 is gated by actor
field selector 2 (byte 26), presentation context and object flags. Trace its
reachable callers and field writes before deciding which BAS resources matter;
initial field values alone cannot prove that conversations are unreachable.

SCRIPT2.VAR has an extra trailing word named `time` at byte 8368. The initial
VAR image is only 8368 bytes, yet the native loader retains it when selecting
noninitial profiles. Allocation ownership and the first read/write of that
extra word remain unresolved. Do not copy the second profile's defaults or
silently zero-extend the initial state without native evidence.

## Remaining Completion Requirements

- Recover D4 and D7 effects and compare inherited VM handlers, including
  skip, state, presentation and conversation semantics. Integrate the native
  simulation countdown lifecycle required by D4-D6. Add native oracle coverage.
- Wire game/version identity and the recovered sequel catalogs/layouts into
  production startup and runtime profile changes; resolve missing-resource
  behavior and the extra SCRIPT2 state word before claiming complete loading.
- Recover the actual conversation representation and produce readable,
  hand-editable French source with byte-exact COD/BAS/DEB/DIC/VAR/DESCRIPT
  reproduction where those resources are active. No raw fallback as completion.
- Port changed native simulation, travel, interface and presentation behavior;
  compare AMER/CROOLIS routines and assets. Validate new media through the
  library-only import path and existing SDL3/wgpu rendering.
- Provide game selection and separate asset caches, save identities and source
  checksum manifests so the games cannot contaminate each other's state.
- Extract contextual complete messages and UI text into a stable localization
  catalog, translate French to English, preserve logical IDs, and verify English
  rendering, wrapping, interaction and subtitle timing. Translation has not begun.
- Capture the original sequel in DOS and compare Rust behavior through startup,
  dialogue, travel, added gameplay and completion paths. Keep Commander regression
  coverage running alongside it. No whole-game parity claim from format tests.

Each item remains part of the full objective; completing the decoder or one
handler does not redefine the deliverable as a compatibility-only tool.
