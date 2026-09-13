# Big Bug Bang Rust Port

## Objective

One Rust engine must run Commander Blood and Big Bug Bang, with English
localization for Big Bug Bang. Preserve each game's original behavior, keep
runtime memory flat and owned, reuse SDL3/wgpu and imported RGB assets, and
ship no external executable dependency. Test/oracle tools are separate from
runtime dependencies.

The objective is active and **not complete**. Verified production routes include
the opening, PLAY into SCRIPT2, initial Daddy dialogue in SCRIPT3, save/load in
a fresh process, travel to Tempest, and recontacting Daddy on Templand. The
renewed Templand conversation now passes its video interlude, displays both
choices, and continues through `no_hurry` back to Tempest navigation. Retained
screenshots and RGB-row checks verify the formerly missing labels. Other
conversation branches and broader progression remain unverified. These routes
do not establish a complete playthrough. A later earned-inventory route now
reaches Daddy's GIVE menu, transfers writing to him, receives the intelligence
acknowledgement, returns to navigation, and saves/restores that ownership in a
fresh process. Honk's accelerated mutation, the Internet League puzzle, and
saving/reloading the multiplexer reward, and creating/saving/reloading Super Zen
are now verified too. Super Zen's cryobox wake-up, second-contact teleport,
and fresh-load persistence of all three Zen destinations are verified. Izwal
creation, its immediate guild-code continuation, and save/load persistence of
Izwalito, Marakas and Tequila on Spiraland are also verified. Marakas's food
purchase, its one-credit cost, and fresh-load persistence are verified. Giving
Marakas optics now has verified first-migration and save/load coverage for
Izwalito and Tequila on Vulcland. Further migration, later trades, and endgame
remain unverified. English COD display
catalogs cover all 17 profiles and all 6,921 COD text sites. Timed sequence
captions, inventory labels, and DESCRIPT location captions also have English
display mappings, but live coverage of these surfaces is incomplete. The
separate SCRIPT2 BAS resource is preserved but is not reachable from legitimate
shipped state, as established below. Its dormant bytes have not been translated
or claimed as structured source. The latest route evidence is recorded in
`../localization/big-bug-bang/README.md`. The original-disc investigation is in
`big-bug-bang-investigation.md`; its initial decoder limitations describe the
state before the implementation below.

## Verified Implementation

### Source-First Completion Gate

The current priority is complete native-source and VM-logic coverage before
the user resumes manual testing. Route replays below are regression evidence,
not a substitute for this gate. The CB ledger's 521 recovered routines do not
constitute a BBB routine inventory or prove inherited handlers unchanged.

Outstanding coverage work includes BBB-specific native routine ownership and
comparison outside the VM dispatch. Every defined A0-D7 dispatch entry now has
a guarded original-BBB executable comparison against its typed Rust owner,
including the A6 publication state detailed below. That bounded opcode ledger
does not prove pre/post-frame orchestration, every malformed-input behavior, or
the wider simulation and presentation runtime. The changed AMER/CROOLIS overlay
images have the bounded native frame comparison described below. Modern typed
VM source, English text coverage, and the unreachable-BAS result do not close
those wider native obligations. The explicit BBB unified source path reproduces
all 68 active COD/DEB/DIC/VAR resources and the 44,676-byte DESCRIPT database,
as detailed below; the default CB walker and retired interpreter have not been
switched to BBB.

### Changed AMER and CROOLIS Overlays

The production alien decoder identifies Commander Blood and Big Bug Bang XDBs
from their exact 15-entry native method tables instead of accepting shifted
locations heuristically. Big Bug Bang AMER uses its changed section fields,
ring and resume state, callbacks, and initialized steering/finish callbacks.
Big Bug Bang CROOLIS uses its shifted resume state and retains its own revision
on the owned asset so runtime behavior can select the shipped semantics.

The native behavior translation preserves four observed BBB differences: the
AMER selection gate and exchanged lower X/Z bounds; CROOLIS pitch motion without
the Commander species-seed bias; CROOLIS's signed selection bounds, steering
bias and radial target; and the BBB reset routine's changed turn-score register.
Commander Blood and SCRUT retain their existing paths and oracle vectors.

`original_xdb_alien_frame_oracle.py --revision big-bug-bang` executes the
unmodified AMER and CROOLIS images under DOSBox-X, apart from the same bounded
frame-capture hooks used by the Commander oracle. Centered and corner-input
campaigns each match Rust's complete RGBA output at frames 1, 2, 4, 8, 16, 32,
and 64: 28 byte-exact frame comparisons. Focused tests also cover changed
branches that those two campaigns do not necessarily enter. This establishes
the decoded assets and those bounded overlay runs, not every possible input
sequence or an unbounded gameplay-parity claim.

The BBB dispatch table at file `0x16A78` maps A0-A4 to handlers
`0x6A75`, `0x6A8E`, `0x6AA4`, `0x6AB2`, and `0x6AF7`. The guarded
`big_bug_bang_control_flow_oracle.py` harness verifies those entries and
executes all five unmodified handlers, including A2's native PRNG and the real
failure helper at `0x697A..0x6993`. Its 152 cases cover guard depths and target
boundaries, eight random moduli across four complete PRNG states, both concept
slots and comparison polarities, and jump cleanup. The fixture regenerates
byte-identically. The Rust regression reconstructs BBB A0-A4 tokens from the
semantic inputs, decodes them with `ScriptDialect::BigBugBang`, and applies
every case through the existing shared `ScriptRuntime`; there is no sequel
control-flow runtime. This proves the well-formed typed domain exercised by the
fixture. A3's separate zero-word scan path is not exercised by this harness,
and A0-A4 coverage does not establish parity for later inherited handlers.

The inherited A5 handler at BBB `0x6B07..0x6B28` has the same instruction
structure as CB `0x65EB`, with relocated globals and timer storage. The
`big_bug_bang_timer_oracle.py` harness executes its original bytes together
with the failure helper at `0x697A..0x6993`. Its 3,072 cases cover every owned
timer slot, query-flag combinations and zero/signed-boundary state values,
with separate GS globals, SS state and DS script allocations. Instruction
and write-range assertions reject unexpected execution or state changes.
The Rust test compares BBB token decoding, cursor, full saved timer block,
query state and guard depth. Negative signed indices are outside the typed
timer domain and are not claimed as covered. This is one inherited-handler
comparison beyond the A0-A4 set, not complete VM parity.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_control_flow_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_control_flow.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_a0_a4_decode_into_the_shared_control_flow_runtime
```

BBB's A7, A9, AA, AB and AC entries likewise retain the shared sequence,
procedure and yield semantics. `big_bug_bang_sequence_procedure_oracle.py`
verifies their dispatch entries and executes the five unmodified handlers in
96 cases: 24 topic offers, 48 enabled/disabled procedure gates, 16 procedure
activation writes, and four initial latch values for each yield entry. The Rust
regression frames A7/A9/AA/AB as BBB tokens and applies the decoded values to
the same `SequencePresentationState`, `ScriptProcedureStates`, and
`ScriptRuntime` used by Commander. AC enters the same selector-yield method at
the BAS boundary. AB's typed state owns the low enabled bit consumed by A9; the
comparison covers that semantic bit for all tested byte values, not arbitrary
self-modifying COD bytes. A8 remains a separate comparison obligation because
it owns a variable-length basename and several presentation side effects.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_sequence_procedure_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_sequence_procedure.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_sequence_procedure_and_yield_handlers_use_shared_typed_state
```

That separate A8 comparison is now complete. The guarded
`big_bug_bang_sequence_request_oracle.py` harness executes BBB's unmodified
0x6E7C..0x6EE4 handler in 64 cases spanning empty, ordinary, finale-prefix,
case-sensitive, high-byte, and segment-wrapping basenames across pending,
ship, scene, and inactive gate combinations. The generated vectors prove the
copied basename and consumed pad, sticky `fin.` latch, preserved request bits,
line-7 selection, three presentation resets, and wrapped script cursor against
the same typed `load_sequence_request` path used by Commander.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_sequence_request_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_sequence_request.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_a8_uses_shared_sequence_request_state
```

BBB's AE and B0 dispatch entries both resolve to the inherited shared handler
at `0x750A..0x754E`. `big_bug_bang_shared_bit_oracle.py` executes that handler
and the real `0x697A..0x6993` failure helper in 480 cases across both opcodes,
all low query/inversion combinations, preserved high query bits, zero and
multi-bit masks, and boundary word values. The Rust regression decodes BBB
tokens and applies them through the same typed `ScriptState`,
`apply_shared_bit_operation`, and `ScriptRuntime` used by Commander. This
proves the shared masked-bit domain; it does not yet cover the later
shared-state or record-operation handler families.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_shared_bit_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_shared_bit.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_ae_b0_use_shared_masked_bit_state
```

BBB's AD, AF, B2, B3, BA, BB and BC entries all resolve to the inherited
`0x754E..0x75CD` direct-record handler. The guarded
`big_bug_bang_direct_record_oracle.py` harness executes that handler with the
real owner lookup, aboard-list removal and insertion, and guard-failure helpers
in 912 cases. The matrix covers every alias; odd and preserved nonzero query
bytes; every descriptor-valid inversion; object, aboard, special-player,
native-word and dictionary-topic values; matching and mismatched fields; empty,
duplicate and full aboard rosters; and BC's update-only publication side effect.
The Rust regression decodes BBB tokens against a complete BBB-sized DEB, DIC
and VAR fixture before applying the shared typed record state.

This comparison exposed an inherited update ordering requirement absent from
the older narrow fixture. Reassigning an already-aboard field to `aboard` or the
special player first removes its owner and then inserts it again. The shared
engine now preserves that roster membership and stores the typed aboard value;
it also retains the old field when a full roster rejects a new insertion.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_direct_record_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_direct_record.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_direct_record_aliases_use_shared_typed_state
```

The inherited B7 handler at BBB `0x76AD..0x770C` uses the same high-bit-first
byte addressing as Commander. `big_bug_bang_bit_flag_oracle.py` executes that
handler and the real guard-failure helper in 400 cases over query bytes,
inversion, five byte patterns, and bit indices spanning positions 0..7 and
multiple later bytes. The Rust regression decodes each BBB token and applies
it through the shared bounded `ScriptState` and `apply_bit_flag_operation`
path, checking the derived mask, byte mutation, control flow, guard depth, and
cursor. B8/B9/BD pair writes remain a separate helper-backed obligation.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_bit_flag_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_bit_flag.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_b7_uses_shared_high_bit_first_flag_state
```

BBB's B8, B9 and BD dispatch entries share the inherited
`0x770C..0x7752` adjacent-word handler. The guarded
`big_bug_bang_record_pair_oracle.py` harness executes that handler together
with the real `0x6644..0x665E` owner lookup and failure helper in 720 cases.
It covers exact and mismatched pairs, query bytes, all three aliases, and
owner-matching, other-owner, and empty active references. The Rust regression
uses a self-contained two-object BBB DEB/VAR layout and the shared
`apply_record_pair_operation`, proving both bounded pair mutation and
owner-scoped reference invalidation without original assets at test time.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_record_pair_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_record_pair.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_b8_b9_bd_use_shared_record_pair_state
```

BBB's CA and CB entries retain the inherited signed host-clock guards at
`0x6A01..0x6A75`. `big_bug_bang_clock_guard_oracle.py` executes both handlers
and the real failure helper in 1,764 cases spanning signed hour, month and day
boundaries, all three relation tags, ignored CA tag high bytes and CB year
words, and preserved nonzero query bytes. The Rust regression decodes BBB CA/CB
tokens and evaluates them through the shared `ScriptClock` and `ScriptRuntime`,
including guard failure, cursor movement and the ignored encoded year.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_clock_guard_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_clock_guard.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_ca_cb_use_shared_signed_host_clock_guards
```

BBB's CE, CF, D0, D1 and D2 entries resolve to the inherited environment
family at `0x69AC..0x69E6`. `big_bug_bang_environment_oracle.py` verifies all
five dispatch-table entries and executes the original handlers in 362 cases.
The 90 activity-guard cases cover all three native globals, clear and set low
bits, unrelated high bits, preserved nonzero query bytes, and one- and
two-level guard stacks through the real `0x697A..0x6993` failure helper. Sixteen
CF cases cover zero, low, high and mixed values in both cleared resume globals.
The remaining 256 cases execute D2 for every encoded signed operand and check
its one-byte cursor advance and exact zero-based request word.

The Rust regression frames each instruction with `ScriptDialect::BigBugBang`
and compares the vectors through the shared `ScriptEnvironmentActivity`,
`ScriptRuntime`, and `ScriptProfileRequestSlot`. D2 retains the previously
verified 17-profile BBB validation domain while Commander remains limited to
its own profile table. Raw high query bits have no independent typed owner;
the comparison preserves their observed low-bit semantics rather than claiming
an arbitrary byte-for-byte runtime representation.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_environment_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_environment.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_ce_d2_handlers_match_original_environment_vectors
```

BBB's CD entry resolves to the inherited `0x75CD..0x76AD` transfer handler.
`big_bug_bang_transfer_oracle.py` executes it with the real owner and field
lookups, aboard-list removal and insertion, guard-failure helper, DESCRIPT
parser, and entered descriptor helpers in 600 cases. The query matrix covers
exact and mismatched triples, both inversion states, preserved nonzero query
bytes, both possible source owners, and one- and two-level guard stacks. The
assignment matrix crosses source and destination ownership, actor, inventory
and location items, ignored active-flag values, empty, duplicate and full
rosters, and all interface, request and descriptor-result gates. DOS reads use
an owned synthetic descriptor database; no helper result or executable byte is
replaced.

The Rust regression reconstructs BBB-sized DEB and VAR records, decodes every
CD token with the sequel dialect, and compares the shared `apply_transfer`
state: record queries, holder changes, roster ordering, presentation requests,
guard control, request bits, and cursor movement. The earlier sequel inventory
transfer probes exercise a different selection procedure and did not establish
this opcode behavior.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_transfer_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_transfer.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_cd_uses_shared_typed_transfer_state
```

BBB's C1 dispatch entry retains the inherited `0x7752..0x7884`
navigation-record handler. `big_bug_bang_record_state_oracle.py` executes the
unmodified handler with its real owner, field, distance, position, source-list,
link-bit, square-root and guard-failure helpers in 624 cases. The matrix covers
direct and special-operand queries, ignored third words, inversion and nonzero
mode bytes; direct assignments; both special aliases; same and different
positions; wrong and navigation parents; empty and occupied destinations; and
unknown, player and actor source-list gates. The Rust regression reconstructs
the complete 15-object BBB-sized fixture, decodes each C1 token, and applies it
through the shared typed record and navigation state.

The executable also has a shipped epilogue defect: successful queries and
exhausted source scans jump to `0x7882` without restoring the handler's saved SI
and DS words. The oracle stops at that shared epilogue and records the bad frame
separately from the logical cursor. Rust intentionally preserves the observable
Continue result without corrupting its call frame.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_record_state_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_record_state.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_c1_uses_shared_typed_record_state
```

BBB's C3-C8 action-record handlers retain the inherited typed decision rules.
`big_bug_bang_action_record_oracle.py` executes all six original handlers with
their real owner lookup, field lookup and guard-failure helpers in 996 cases.
The matrix covers query inversion and ignored third words; owner, related and
reciprocal activity; C3 queue replacement; C4 player bypass and reciprocal
collision; C5 world-state validation; C6 unconditional travel replacement;
C7's empty-or-C4 destination rule; and C8's dormant zero marker. The Rust
regression constructs BBB-sized actor and location records and applies every
decoded token through the existing shared `ScriptActionRecords` functions.
C2 remains separate because its assignment path owns the aboard roster and
DESCRIPT-backed presentation effects.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_action_record_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_action_record.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_c3_c8_use_shared_typed_action_record_state
```

BBB's C2 dispatch entry retains the inherited `0x7A3A..0x7AF4` aboard-record
handler. `big_bug_bang_aboard_record_oracle.py` executes that handler with the
real roster insertion, owner and field lookups, guard-failure helper, DESCRIPT
parser, entered descriptor helpers, and DOS file boundaries in 684 cases. The
matrix covers all exact and mismatched query forms, inversion, nonzero mode
bytes, active and presentable gates, free, duplicate and full rosters, actor,
inventory and location holder layouts, both presentation gates, descriptor
presence, and unrelated request-bit preservation. The Rust regression decodes
BBB-sized records and C2 tokens before applying the shared typed aboard state.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_aboard_record_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_aboard_record.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_c2_uses_shared_aboard_record_state
```

### COD Source Recovery and Shared Arithmetic

The source-first pass found a production translation discrepancy that route
tests had missed. BBB's shared-state handler at `0x744B..0x750A` implements
operator F8 with unsigned MUL at `0x74DE..0x74EB` and F9 with unsigned DIV
at `0x74EB..0x74FE`. Rust previously classified both as `PreserveOrFail`.
The seven dispatch aliases B1/B4/B5/B6/BE/BF/C0 all enter this same handler.
The recovered corpus contains 57 multiply/divide uses across 12 profiles,
including the population words of Marakas, Izwalito, Tequila and Daddy.

The formats decoder now selects Multiply/Divide only for the BBB dialect.
The runtime keeps the low 16 bits of multiplication, performs unsigned
division, and preserves the target for a zero divisor. Both operations fail
in query mode without writing VAR. CB decoding retains its previous
PreserveOrFail classification for these bytes.
`big_bug_bang_shared_arithmetic_oracle.py` captures 1,728 original-executable
cases, with instruction/write guards and the real guard-failure helper.
They cover signed boundaries, zero, truncation, all ordinary comparisons and
updates, unsupported operators, immediate/indirect modes and aliased operands.
The Rust regression compares every case through each of the seven opcode
aliases (12,096 comparisons), checking the full VAR image, cursor, query flag
and guard depth. The fixture regenerates byte-identically.

The standalone compiler now has an explicit BBB source walker and named
statements for D3 multiply/divide, D4 population growth, D5 descendant
settlement, D6 conflict, D7 ending and A2 random guards. Shared-state aliases
retain their distinct encodings in readable expressions; F8/F9 use `*=` and
`/=`. Text uses `state_number(0xOFFSET)` and `inventory_choices` instead of
misclassifying these markers as dictionary words. In particular, a numeric
operand of zero does not terminate the line, and FFFF/FFFE operands are not
mistaken for choice markers. Generated `text_tokens` controls now compile
with their actual emitted grammar, and profile requests support SCRIPT1..17.

The BBB path now also uses each profile's real DEB symbols and VAR ownership to
recover procedures, structured guards, objects, and fields. CFG and guard
analysis select the BBB instruction walker without changing the Commander
Blood path. A2 random instructions create control-flow branches only in query
mode, matching their condition/update distinction. Names that start with a
digit, including SCRIPT5's `1fincro` and `2fincro`, receive a leading underscore
so that the recovered procedure remains an editable source identifier.

All 17 original COD files and their real CP437 dictionaries now pass structured
source decompilation/recompilation: **25,530 statements and 241,219 bytes**, with
zero raw bytes and zero generic opcode statements. The recovered structure
contains **312 procedures, 2,221 guards, 627 object aliases, 7,684 object-alias
uses, and 1,153 field aliases**, with zero rejected guards. The recovery command
loads each COD/DIC/DEB/VAR set and checks these gates and byte identity before
writing each local source file:

```sh
nix develop -c cargo run -p commander-blood-script-compiler --example recover_bbb_cod -- \
  output/big-bug-bang/imported-assets/resources \
  output/big-bug-bang/recovered-cod-source
```

The earlier standalone COD files remain useful intermediate output. The
canonical sources are now the unified profiles described next. Byte-identical
source compilation does not establish complete native behavior or a complete
playthrough.

Verification for the structured checkpoint: the real all-profile corpus test,
all five CFG tests, and all 34 BloodScript tests pass. The broader root-library
run had 689 passing tests and seven failures solely because its Commander Blood
fixtures were unavailable at `accuracy/cblood_install/cblood`; those failures
do not exercise this BBB source path. No manual gameplay session was requested
or launched for this source-recovery pass.

### Unified Profiles and Companion Rebuilding

`re/vm/big-bug-bang-profiles` contains 17 canonical `bloodscript 8` profiles.
An explicit `dialect big_bug_bang` declaration selects BBB's A0-D7 instruction
walker, eight-byte DIC prefix, 74-byte actor records and 26-byte location
records. The added actor word is exposed as `opponent`; the added location word
is exposed as `settler`. The remaining typed fields, reserved zero regions,
`tblood`, ordered globals, procedure addresses, directory sentinel, dictionary
word order and intentional empty entries are derived from readable source.

Each profile compiles to the four resources owned by shipped gameplay: COD,
DEB, DIC and VAR. The lone `SCRIPT2.BAS` remains preserved outside these
profiles because the ownership proof below establishes that it is unreachable
from legitimate shipped state. The compiler neither emits an invented empty
BAS nor binds that file to SCRIPT2's incompatible dictionary.

The separate artifact is nevertheless fully ported as editable source at
`re/vm/big-bug-bang-separate-bas/script2.bas.blood`. Its legacy BAS framing
recovers 593 text records, 122 menus, 122 linked selector nodes and 265 other
instructions with no raw or generic operations. All encoded word offsets remain
numeric because no shipped BBB dictionary resolves the stream; compiling the
source without a dictionary reproduces all 19,933 shipped bytes exactly. This
structural recovery does not alter the ownership or reachability conclusion.

Regeneration decompiles each original profile, recompiles it internally, and
rejects any byte difference before writing source:

```sh
nix develop -c cargo run -p commander-blood-script-compiler \
  --example recover_bbb_profiles -- \
  output/big-bug-bang/imported-assets/resources \
  re/vm/big-bug-bang-profiles
```

All 17 profiles reproduce **637,922 bytes across 68 resources** exactly. The
corpus gate is
`all_sequel_unified_profiles_rebuild_every_active_companion`; synthetic tests
independently cover resource ownership, extended record sizes, `opponent`,
`settler`, and the full CP437 byte codec. Numeric BBB presentation selectors
remain numeric because Commander Blood's symbolic presentation catalog is not
valid evidence for the sequel.

### Production BBB Source Verification

Production startup now selects editable script sources by game identity.
Commander Blood keeps its five profiles and DESCRIPT source; Big Bug Bang
selects the 17 profiles under `re/vm/big-bug-bang-profiles` plus its DESCRIPT
source under `re/descript/big-bug-bang`. A newer BBB source is compiled,
compared byte for byte with each original resource, and installed in the
existing per-game verified cache. Any difference aborts startup before the
compiled bytes can become a runtime override. BBB startup does not compile
Commander sources or emit BAS.

The package installs both profile sets and both DESCRIPT sources. An asset-free
layout test checks their separate counts, dialects and resource ownership. The
original-resource cache test compiles all 17 BBB profiles and its DESCRIPT
source through the startup artifact path and requires 68 named profile
overrides, 69 cache files, 18 rebuilt units, and no BAS cache file. The
production bootstrap test exercises the same game-identity selection while
continuing to verify that loaded COD bytes equal the shipped resource.

The existing DESCRIPT decompiler accepts the sequel database without a dialect
extension and rejects any non-exact round trip. The checked-in source covers all
230 records and 2,757 ordered commands and recompiles to the original 44,676
bytes (SHA-256
`3ffb0122c13ea951fa9a24df3ddbbeb383a5da91f645d8833a2de1ee760e913d`).

### Izwalito's Treaty Purchase

`bbb_izwalito_treaty.tsv` continues from the earned Help save, answers Yes,
accepts the treaty offer, cancels the GIVE menu and saves from the unblocked
bridge. `izwalito_treaty_purchase_survives_save_and_fresh_process_load` passed
in 193.96 seconds, requiring rendered Accept/Refuse and treaty inventory
labels, precisely one credit spent, retained prior inventory, and a fresh
process restoring the treaty without rewriting either save file.

The original SCRIPT4 subtracts the credit at `0x34CF` and transfers treaty
record `0x1E38` at `0x34E6`. No runtime behavior change was needed for this
route. Evidence is in
`output/fidelity/bbb-izwalito-treaty-save-1788923108216766608-1036958-0` and
`output/fidelity/bbb-izwalito-treaty-load-1788923285195158501-1036958-1`.
This verification preceded the source-first priority change; it does not
establish later treaty use or a complete playthrough.

### Izwalito Treaty Gift

`bbb_izwalito_treaty_gift.tsv` starts a fresh process from the purchased-treaty
checkpoint, returns to Izwalito on Vulcland, selects the visibly rendered
`treaty` row in GIVE, refuses the immediately authored replacement-treaty offer,
cancels the remaining four-item GIVE list, returns to the unblocked bridge, and
saves normally. A second fresh process loads that new save without rewriting
either save file.

Semantic traces now expose each BBB actor's aggressiveness, energy, encounter
count, and evolution from the same synchronized state already used for holder,
population, and simulation-flag diagnostics. The route requires Izwalito's
evolution to change from 90 to 140 and aggressiveness from 150 to 100 while the
treaty moves from aboard to Izwalito at state offset `0x065c`. These are the
authored SCRIPT4 effects at COD `0x3d8c..0x3dc4`: the English acknowledgement
is displayed, evolution increases by 50, aggressiveness is assigned 100, and
population increases by 400 before normal simulation continues. Guitar,
perfume, decoder, and energy remain aboard throughout.

`izwalito_treaty_gift_survives_save_and_fresh_process_load` passed in 146.72
seconds. Gift/save evidence is retained at
`output/fidelity/bbb-izwalito-treaty-gift-save-1789242995553521195-2838889-0`;
fresh-load evidence is at
`output/fidelity/bbb-izwalito-treaty-gift-load-1789243112928956812-2838889-1`.
The resulting `BLOOD.SAV` SHA-256 is
`f57ba4a59e8339b49e9229bf73b53417f9d468406ed144ac477fd16e715636df`;
`GAME1.SAV` is
`4b9b17a63606fd497e810b19295d8e319b3fec4233e19999473532b0077dd13a`.
The tested optimized runtime SHA-256 is
`c556c0079a12063c22421aec3ae94fce487ab3072dbc1db4c8fc08b1d0606346`.
Population continued through ordinary growth and conflict while the route ran;
this test does not establish another settlement stage, treaty effects on other
races, or the ending.

### Tequila Treaty Gift

`bbb_tequila_treaty_gift.tsv` begins again at the purchased-treaty checkpoint,
gives that treaty to Izwalito, accepts the authored one-credit replacement
offer, and cancels the rebuilt GIVE list. It then uses the normal chart and
hyperjump controls to reach Goan, enters Goanland, selects the visibly rendered
`treaty` row for Tequila, cancels her remaining four-item GIVE list, returns to
the bridge, and saves. A second executable process loads the resulting save
without rewriting either file.

The source-defined effects are independently visible in the synchronized state.
Izwalito's population changes from 1155 to 1555 while evolution changes from 90
to 140 and aggressiveness from 150 to 100. Tequila's population changes from
142 to 542 while evolution changes from 90 to 140 and aggressiveness from 150
to 0. The replacement costs exactly one credit, and the treaty ends at Tequila's
state offset `0x06a6`; guitar, perfume, decoder, and energy remain aboard. These
match SCRIPT4 COD `0x5580..0x55c2`, including the English inline acknowledgement
and the subsequent return to the four-item GIVE menu.

`tequila_treaty_gift_survives_save_and_fresh_process_load` passed in 220.08
seconds. Gift/save evidence is retained at
`output/fidelity/bbb-tequila-treaty-gift-save-1789244096370779124-2857000-0`;
fresh-load evidence is at
`output/fidelity/bbb-tequila-treaty-gift-load-1789244299754637295-2857000-1`.
The resulting `BLOOD.SAV` SHA-256 is
`f57ba4a59e8339b49e9229bf73b53417f9d468406ed144ac477fd16e715636df`;
`GAME1.SAV` is
`361ccb2938449e95719e94d296a6b49f567d0ef6b893be48a37b63a9cfe6fc57`.
The tested optimized runtime SHA-256 remains
`c556c0079a12063c22421aec3ae94fce487ab3072dbc1db4c8fc08b1d0606346`.
Later simulation continues to change population normally. This route does not
establish peace-treaty effects for the other races or the ending.

### Izwalito's Help Conversation

The first Vulcan visit after migration reveals Izwalito as a life form and
sets the descendants' discovery flags. A second approach reaches his SCRIPT4
conversation. `bbb_izwalito_contact.tsv` follows this route, selects Help,
continues through the `19 96 19 96` phone-number dialogue and saves normally.
`bbb_izwalito_help_reload.tsv` recontacts him after loading and answers No to
the subsequent yes/no question about calling his sweetheart.

The English prompt at `0x2D03` now correctly asks whether to help **him**:
Izwalito is asking for help expressing his feelings. Lines at `0x2F1B`,
`0x2F35`, and `0x30C9` now refer to Pierrette as **her**, consistent with the
surrounding original French dialogue. The Pierre/Pierrette joke is unchanged.
The original-resource SCRIPT4 test checks the corrected wrapped prompt along
with the catalog's existing live-number, inventory and RGB-rendering checks.

The `izwalito_help_save_reloads_into_authored_no_ending` regression requires
the rendered Help/Abandon choice with the corrected prompt, the phone-number
continuation, a save, and an unblocked bridge. Its fresh process must reach
the complete live English population and later Yes/No choice without repeating
Help, then reach the authored No ending without rewriting either save.
Izwalito's destination and remaining owned
inventory must be retained throughout. Tequila may continue from Vulcland
to Goanland as population grows.

The first two-process attempt exposed a real crash on fresh-load recontact:
`UnverifiedSpokenStateNumber` while executing the population line at SCRIPT4
`0x2B01`. The inherited spoken-mode latch makes this numeric path reachable
even though the instruction itself does not set the spoken flag. Its artifact
is `output/fidelity/bbb-izwalito-help-load-1788920784969375022-965183-1`.

`re/tools/big_bug_bang_spoken_number_oracle.py` now executes the original
`0x6D58..0x6DEA` string builder, dictionary-length helper, and signed decimal
formatter. Its 45 synthetic cases cover signed limits, digit lengths,
dictionary-interior operands, punctuation, and wrapping. The native Rust
translation matches their complete output bytes, including the original
cursor remaining on the numeric operand and subsequently reading it as a
dictionary suffix. The fixture regenerates byte-identically against the
pinned executable; production code has no executable or emulator dependency.

English subtitle overrides now receive read-only script state, so an inherited
spoken line displays the live signed value without the original accidental
dictionary suffix. The actual SCRIPT4 test checks the Izwalito population line
at values 0, 26, 32767, -32768, and -1 and verifies state is unchanged. Reads
outside an owned VAR field or dictionary suffix still fail explicitly; no
dictionary padding or invented backing bytes were added.

The first repaired runtime replay exited without a runtime error but exposed
two incorrect acceptance assumptions: Tequila had migrated to Goanland, and
No does not return to ordinary play. The original SCRIPT4 No branch writes
`VAR[0x1F04] = 2` at `0x30B6`. SCRIPT2 tests that value at `0x9F0C`, requests
sequence 6 of `28bob` at `0x9F14`, and executes D7 at `0x9F1D`. The trace reaches
the corresponding presentation assignment at `0x9F14`, ending latch, and
forward panel phase 7 before process exit. Adding more wait/park input cannot
prevent this authored ending; the test must verify it, not bypass it.
The migration exception is
specific, not a removal of destination checks: the original D5 handler also
moves Tequila from `0x1328` to `0x1360` while leaving Izwalito at `0x1328` when
the earned Help save is supplied with Izwalito's observed mature population
363. The saved population itself was 214; this is a controlled population
substitution, not a claim to have captured every live VAR byte. The result is
retained in `output/big-bug-bang/izwalito-goan-native-settlement.jsonl`; Rust
matches its full resulting VAR image. The complete English population line
was observed at frame 2851 of
`output/fidelity/bbb-izwalito-help-load-1788921800165281633-1002831-1` as
`There are 412 Izwals in this community...`, matching the live actor value.

The alternative Yes route is retained as `bbb_izwalito_yes.tsv`, seeded from
the earned Help save at
`output/fidelity/bbb-izwalito-help-save-1788922025625953790-1015705-0/writable`.
`output/big-bug-bang/izwalito-yes` completes that scenario and reaches the
English peace-treaty purchase prompt with rendered Accept/Refuse choices.
`validate_recorded_izwalito_yes_continuation` verifies the initial Yes/No choice,
the completed Olga conversation response, the final treaty prompt and choice
pixels, and absence of the ending latch throughout. Purchasing the treaty and
saving the continuation are not yet verified by this route.

The full two-process No regression passed in 254.06 seconds with artifacts
`output/fidelity/bbb-izwalito-help-save-1788922492594927215-1025035-0` and
`output/fidelity/bbb-izwalito-help-load-1788922664517641482-1025035-1`.
It verifies both save files remain byte-identical after fresh recontact.
The tested optimized binary SHA-256 is
`2713b033e4ea71f5a5580a8d20a132fd2d460455df1faed4a2090200ee891b8b`.
All 976 enabled game-library tests, all 24 localization tests including
original resources, and game-package all-targets checking also pass. These
results do not establish later trades, further quests, or a complete playthrough.

### Earned-State Settlement Oracle

The native D5 oracle now accepts `--save GAME1.SAV --directory SCRIPT1.DEB
--group 16`. This binds the original named objects from the directory and feeds
the saved VAR bytes to the unchanged original handler and its seven helpers.
It is a direct handler experiment, not proof that the script's D5 gate has
been satisfied. Generated results contain local game data and remain under
`output/`, outside the repository's committed fixtures.

On the earned food-purchase save, the original handler leaves Marakas on
Spiraland and moves Izwalito and Tequila to Vulcland. The new ignored regression
`earned_save_settlement_matches_original_executable`, supplied that output via
`BBB_SETTLEMENT_ORACLE`, verifies the entire resulting VAR byte-for-byte
against Rust and requires an actual state change. The 100 synthetic settlement
and 124 conflict cases regenerate unchanged after adding this input mode.

The normal gameplay gate is SCRIPT2 `0x1610`: counter `0x1FEC > 0` enables
group-16 settlement. Marakas's optics response at SCRIPT4 `0x1B61` increases
relief and sets this counter to one at `0x1B86`. This is an ordinary gift path,
not a direct simulation-state edit. His recontact food sequence includes native
CD transfers back to Marakas (`0x125C` or `0x14F2`), so food ownership may change
before the repeat purchase offer. Refusing that offer leaves the five-item GIVE
menu; giving optics opens another GIVE menu with four items and Cancel.

`bbb_izwal_migration.tsv` enables Travel, restores the forward view, approaches
the already-current Spiralus, refuses the repeat purchase, gives optics,
cancels GIVE, waits for ordinary settlement and saves. Clicking Spiralus on the
chart is intentionally ignored when it is the current planet; that behavior
must not be confused with a broken travel handler.

`izwal_migration_survives_save_and_fresh_process_load` passed in 143.16 seconds.
It requires rendered gift rows, optics ownership by Marakas before migration,
both descendants on the native-predicted Vulcland, an unblocked final bridge,
a completed save, immediate persistence in a fresh process, and byte-identical
save files after loading. The writing item is allowed to pass through the
authored Platon sequence to Marakas (`0x11DE`, `0x1277`); an initial test wrongly
treated its old Daddy ownership as permanent. No production change was needed.
The gift-only capture `output/big-bug-bang/izwal-optics-gift` is rejected for
not completing migration and returning to the bridge.

Passing save artifacts:
`output/fidelity/bbb-izwal-migration-save-1788919669277150183-937814-0`.
Fresh-load artifacts:
`output/fidelity/bbb-izwal-migration-load-1788919796220936127-937814-1`.
`BLOOD.SAV` SHA-256:
`f57ba4a59e8339b49e9229bf73b53417f9d468406ed144ac477fd16e715636df`.
`GAME1.SAV` SHA-256:
`07f03fa349766fb4caf8717efdb7de37774ae2a51ae47c8f071de26fedc2c3d8`.
Override the input checkpoint with `BBB_FOOD_SAVE_DIR`. Reproduce the separate
native comparison with:

```sh
nix develop -c python -P re/tools/big_bug_bang_settlement_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE output/big-bug-bang/earned-settlement.jsonl \
  --save output/fidelity/bbb-marakas-food-save-1788918507683083392-895506-0/writable/GAME1.SAV \
  --directory output/big-bug-bang/izwal-optics-gift/writable/SCRIPT1.DEB --group 16
BBB_SETTLEMENT_ORACLE=output/big-bug-bang/earned-settlement.jsonl \
  nix develop -c cargo test --release -p commander-blood-game --test bbb_progression \
  earned_save_settlement_matches_original_executable -- --ignored --exact
```

### Marakas Food Purchase

`bbb_marakas_food_trade.tsv` loads the earned Izwal checkpoint, waits for
ordinary population growth, travels to Spiralus, contacts Marakas, accepts his
food offer, cancels GIVE, and saves through the normal menu. It does not modify
VAR or bypass a story gate. SCRIPT4 at `0x152F` gates the offer on population
word `0x0628 >= 100` and food not already owned. The accepted branch decrements
credits at `0x1EF2` by one and transfers food to the cryobox.

`marakas_food_purchase_survives_save_and_fresh_process_load` checks rendered
English accept/refuse rows, food ownership after the offer, retention of the
other five items and Izwal destinations, return to an unblocked bridge, and a
completed save. It decodes the original save format to check the one-credit
cost, then launches a fresh process and verifies food ownership from the first
loaded frame and unchanged save files. The offer-only capture
`output/big-bug-bang/marakas-food-offer-AXfS9fgH` is rejected by the recorded
validator because no completed purchase or bridge return occurred. Its
`offer.png` visibly shows the English prompt and both choices.

The two-process live test passed in 199.77 seconds using runtime SHA-256
`6ab6e39f8630dbeacc2836e0805be6684b381df9a77edd716b8139318f9ea3c7`.
Purchase artifacts are at
`output/fidelity/bbb-marakas-food-save-1788918507683083392-895506-0`,
and reload artifacts at
`output/fidelity/bbb-marakas-food-load-1788918691390336278-895506-1`.
The resulting save has two credits, down from three. Its `BLOOD.SAV` SHA-256 is
`086b13ef2f2eda805fedafd68a8bcf6f807846ea39bfb3e658571eeab71833eb`;
`GAME1.SAV` is
`f1bc5b98c5335d74dcbd78dde6dd23d74ccfaff24f922d67f80a007daf55c2cb`.
The strengthened recorded validator also passes on the purchase trace. No
production behavior change was necessary for this trade after the save-order
fix below. Migration, later quests, and endgame are not established by it.

### Restore Before Executing BBB Scripts

Long bridge waits exposed a save-load ordering bug that the earlier ownership
and visible-UI checks missed. The loaded Izwal checkpoint retained a pending
Daddy_Gluxx call even though the visible bridge was idle. The timer context
suppresses script countdowns while a presentation owner is pending, leaving
Marakas's population fixed at 37 in the bridge-only replay.

Native BLOOD2PG loading selects the profile at file `0x1F42` (far call to
`0502:0000`, profile loader at file `0x5820`). It then restores timers,
sequences, VAR and procedure bytes at `0x1F4D..0x1FA3`. Only afterward does it
rebuild inventory (`0x1FAF`, `0502:01DF`) and execute the restored script
(`0x1FB4`, `0502:0233`, file `0x5A53`). The profile loader itself resets and
binds resources; it does not execute fresh SCRIPT2 before those reads.

Rust instead executed fresh SCRIPT2 before restoring the save. Its initialization
queued Daddy's opening call in transient action state, which restoring VAR did
not overwrite. The BBB restore path now executes only after restoring the save
blocks. Commander Blood's existing ordering is unchanged. No timer gate was
disabled and no pending owner is forcibly discarded after dispatch.

Live traces now expose `sequel_population` and `sequel_simulation_flags` for BBB
actors, directly from synchronized state. Non-actors expose nulls. The existing
live profile-handoff test checks the values and confirms tracing does not alter
state. `loaded_izwal_bridge_advances_population_without_a_stale_call` requires
an empty pending owner throughout the loaded bridge, an unblocked final profile,
and actual population growth through normal elapsed game time.

The old capture `output/big-bug-bang/izwal-growth-trace-ndQyQKae` is rejected by
the new validator for its leaked Daddy call. With the fix, the live regression
passed in 70.02 seconds at
`output/fidelity/bbb-loaded-izwal-growth-1788917525623232829-870455-0`:
Marakas grew from 37 to 108 and the pending owner remained null. Exact population
is not the test oracle because host-delivered elapsed time varies. Runtime
SHA-256: `6ab6e39f8630dbeacc2836e0805be6684b381df9a77edd716b8139318f9ea3c7`.

The existing Izwal creation/guild/save/fresh-load regression also passed with
the corrected load order in 134.04 seconds. Its retained captures are
`output/fidelity/bbb-izwal-mutation-save-1788917623255486087-874133-0` and
`output/fidelity/bbb-izwal-mutation-load-1788917740304573479-874133-1`.
The former provides a new earned checkpoint with the simulation no longer
stalled during the route.

The native D6 oracle was also regenerated: all 126 vectors matched the
checked-in results byte for byte. The game-library suite passed 975 tests with
59 ignored under `nix develop`; the ordinary BBB integration invocation passed
six support tests with 29 opt-in tests ignored. A direct test-binary invocation
outside Nix failed the bridge sprite raster test; its Nix rerun and full suite
passed. These checks do not establish migration or whole-game completion.

### Spiralus Travel and Marakas Contact

The earned Izwal checkpoint can enable travel in Options, select Spiralus at
chart coordinate `(77,49)`, approach it, and contact Marakas from the location
panel. SCRIPT4/profile 3 plays his first introduction and money request, then
opens GIVE with the five retained inventory items. Selecting Cancel returns to
an unblocked SCRIPT2/profile 1 bridge still targeting Spiralus, without moving
any of those items or the three Izwal actors.

`spiralus_marakas_contact_and_cancel_returns_to_bridge` passed in 74.14 seconds.
Its input is `accuracy/scenarios/bbb_spiralus_contact.tsv`; override its earned
seed with `BBB_IZWAL_SAVE_DIR`. The regression requires travel enabled, the
Spiralus target, Marakas ownership in profile 3, revealed English introduction
and money-request words, all five inventory rows plus Cancel with matching
text pixels, unchanged item ownership, and the unblocked final bridge.
Evidence is retained at
`output/fidelity/bbb-spiralus-marakas-contact-1788915865754010273-843456-0`.

The exploratory screenshot
`output/big-bug-bang/spiralus-contact-QyvuHefU/travel.png` visibly shows Marakas,
the GIVE list and hand. Despite its filename, this is not a travel-animation
screenshot. That first replay stopped at GIVE and is correctly rejected by the
completed-route validator. A corrected Cancel replay is retained at
`output/big-bug-bang/marakas-cancel-LrjtuTWP`.

An initial test incorrectly checked the general subtitle field for Marakas's
dialogue; that field still held another speaker's line. The corrected test
checks `inline_menu.revealed_words`, then independently checks chooser text
pixels. This test correction required no production runtime change. The normal
integration invocation passes six support tests with 27 opt-in tests ignored.
This establishes contact and cancellation, not a completed trade, migration,
save checkpoint after this visit, or original-game visual parity.

### Izwal Creation and Guild Continuation

From the earned Zen teleport save, the ordinary Metagluk route accepts `5`
and `GO` at the two Izwal prompts. SCRIPT2.COD's AF instructions at
`0x441F`, `0x4424` and `0x4429` place Tequila, Izwalito and Marakas at holder
`0x1478` (Spiraland, record 87). The next authored procedure immediately asks
for another code; selecting `code` completes the GAK membership dialogue and
returns to the bridge. No runtime changes or edited progression data were
needed for this route.

`izwal_mutation_survives_save_and_fresh_process_load` passed in 106.75 seconds
with the same optimized runtime binary recorded below. It checks the rendered
command/run/numeric/word menus, accepted Izwal code, guild continuation, all
three Spiraland destinations, retention of earlier mutation/Zen/Internet state,
an unblocked bridge, ordinary save, immediate restoration in a fresh process,
and unchanged save-file bytes after load. A subsequently tightened validator
also passed the retained trace with the full guild acceptance message required.
The ordinary integration suite passed six support tests; 25 asset/display tests
remain opt-in, not implicitly covered by that suite invocation.

Retained captures:

- `output/fidelity/bbb-izwal-mutation-save-1788915140825347076-835837-0`
- `output/fidelity/bbb-izwal-mutation-load-1788915231906564675-835837-1`

The first capture's `writable` directory is the next earned checkpoint.
BLOOD.SAV SHA-256:
`7cd0a6f7986325bf33f74511c546266141ecab6d178ee2e5dd1ded8f231a3b87`;
GAME1.SAV SHA-256:
`7b844398f96728584b08f73fb9ebc265c69298b572daa584b81c3ecd288be25d`.
The seed can be overridden with `BBB_ZEN_TELEPORT_SAVE_DIR`.

The initial `output/big-bug-bang/izwal-mutation-ADPIbzos` replay stopped at the
guild code prompt: all three actors had moved, but Save had not run. The
validator rejects that trace, and its original inputs are retained as
`scenario.tsv`. This was incomplete replay input, not an established runtime
failure. These checks do not establish migration, later quests, endgame, or
original-game visual parity.

### Super Zen Cryobox Teleport

The earned Super Zen checkpoint now continues through two normal cryobox
contacts. The first plays the wake-up dialogue and returns to the bridge. The
second renders YES/NO, accepts YES, displays `TELEPORTING SUPER ZEN TO
CRAZYSTONE...`, and returns with Super_Zen, Kero_Zen and Ben_Zen at Crazyland
(holder `0x1440`, record 85). SCRIPT11.COD's authored AF operands at
`0x0AAE..0x0ABB` select that surface destination, not the Crazystone planet
record. No runtime code change was needed for this route.

`zen_teleport_survives_save_and_fresh_process_load` passed in 84.38 seconds
using optimized binary SHA-256
`294f470519f58da64578b2b486e1e9de1d99d1bc91b46dd5fe5dff53685e699d`.
It checks the ordered wake/return/recontact flow, rendered choice text pixels,
teleport acknowledgement, retained earlier quest state, ordinary save, immediate
fresh-load destinations, unblocked main profile, and byte-identical save files
after loading. Retained artifacts:

- `output/fidelity/bbb-zen-teleport-save-1788914768113288261-831597-0`
- `output/fidelity/bbb-zen-teleport-load-1788914836928770448-831597-1`

The first directory's `writable` subdirectory is the next campaign checkpoint.
Its BLOOD.SAV SHA-256 is
`091d5bc577549f3c539bbd3a80225878186da850f9a409b51844a7d2725eb2a1`;
GAME1.SAV is
`8dcd3314650e1bd64ba630408dfac9ec03c662cbe858d02b3924115b65045759`.
Override the test seed with `BBB_ZEN_SAVE_DIR` for another earned pre-teleport
save. The retained-trace validator rejects the wake-only capture
`output/big-bug-bang/zen-contact-lwMORzaw` as expected.

This is trace and RGB text-pixel evidence, not a visual comparison with the
original game. Izwal creation, later Zen dialogue and endgame remain unverified.

### Resumed Concept-Slot Selection

The ZEN multiplexing path exposed a shared VM translation error. After selecting
`4` in the numeric prompt, Rust displayed `Enter ZEN GENETIC CODE:` but aborted
before its ten word choices appeared. Capture
`output/big-bug-bang/zen-mutation-11WhGpiE` records the failure, before the
scenario's next pointer input. Super Zen remained on Trashlando rather than
entering the cryobox.

Original BBB A3 (`0x6AB2`, slot selection at `0x6ACA`) chooses the primary or
alternate concept slot strictly using resume mask `0x02`. An empty selected slot
fails either guard polarity; it does not fall back to another slot. Rust used
`alternate_concept.or(selected_concept)`, so a stale primary `4` could satisfy
the inverted comparison against `GA` and enter the abort branch while the new
choice was still pending. `ScriptRuntime::concept_guard` now follows the
semantic resume phase, including when the selected slot is empty.

`re/tools/big_bug_bang_concept_slots_oracle.py` executes the original A3 handler
and guard-pop helper with both slots populated independently. Its 144 cases
cover four resume-byte values, empty/matching/nonmatching slots, two targets,
and both polarities. The original Rust implementation fails these vectors;
the corrected one matches all of them. Older Commander and Daddy oracle tests
now explicitly seed their recorded resume phases instead of assuming that
populating the alternate slot selects it. Game library tests pass 975 with
59 ignored, and the asset-dependent Daddy guard comparison also passes.

The full `zen_mutation_survives_save_and_fresh_process_load` graphical regression
passed in 89.88 seconds with optimized binary SHA-256
`294f470519f58da64578b2b486e1e9de1d99d1bc91b46dd5fe5dff53685e699d`.
It loads the earned Internet reward, selects `yes` from status, then `geranium`,
`run`, `4`, and `GA`. All command/code rows have matching rendered text pixels,
the acceptance message appears, Super Zen changes from Trashlando to player-owned,
and the ordinary save completes. A fresh process restores him immediately and
leaves both save files unchanged. The old aborted trace is rejected by the same
validator.

The new checkpoints and complete input manifests are retained in:

- `output/fidelity/bbb-zen-mutation-save-1788913743695723224-821950-0`
- `output/fidelity/bbb-zen-mutation-load-1788913817892165399-821950-1`

The first capture's `writable` directory is the reusable Super Zen checkpoint,
slot name `abhonkwmiz`.
BLOOD.SAV SHA-256 is
`b977c1234e75fd07d2db14cc7fec2e7efda7fb0e3313c68e7a99c485b8be80e1`;
GAME1.SAV is
`8c8bc94f9543d342dfd8bab99a96b873e69bf51ab709da6479433aaa5eab7afb`.
The input-only scenarios are `bbb_zen_mutation_save.tsv` and
`bbb_load_zen_checkpoint.tsv`; the test accepts `BBB_INTERNET_REWARD_SAVE_DIR`.
`bbb_multiplexer_program.tsv` is the shorter inspection route ending at the
numeric ZEN prompt.
This does not establish Super Zen's later dialogue, Izwal creation, or endgame.

The earlier three-process mutation/Internet regression was rerun with the same
corrected binary and passed in 206.65 seconds. Its captures are
`bbb-mutation-save-1788913858582822241-823242-0`,
`bbb-internet-reward-1788913931196421770-823242-1`, and
`bbb-internet-reward-load-1788914049685620992-823242-2` under `output/fidelity`.
This checks the shared guard change against the already-earned progression,
including both puzzles and fresh save loading, rather than relying only on
the new ZEN route.

### Internet Access and Mutation Puzzle

The actual Internet entry is in the second console row's remote-contact list:
open at logical `(230, 104)`, then select `internet` (record 3) at `(100, 94)`.
`bbb_mutation_remote_contacts.tsv` verifies the available contact from the
earned mutation checkpoint; its retained capture is
`output/big-bug-bang/mutation-remote-sOYaOMpy`.

First contact does not immediately offer the League puzzle. SCRIPT2's authored
visit-count branch at `0x4ED6` compares Internet's `0x80` field against one,
publishes `Wouldn't you prefer TV, Commander?`, clears the contact, grants the
decoder, and requests the advertisement sequence. The live capture
`internet-call-y6NnVh39` shows that branch and `SQ\\decodeur.hnm`, then returns
to the bridge. Its input file is retained as `scenario.tsv`.

Calling Internet again reaches the Interslimic Genetic Mutation League and
its six-row odd-one-out puzzle. `bbb_mutation_internet.tsv` captures both
calls without bypassing any script gate. All six English rows have matching
rendered text pixels; the inspected screenshot is
`output/big-bug-bang/internet-recontact-1hZtPaQd/mutation-puzzle.png`.
The fourth row is `mutant` at logical `(200, 104)`.

Selecting it publishes the reward acknowledgement and makes `optique`
(the mutation multiplexer) player-owned. This is not yet a safe save point:
the same call continues to Bernie's `honest`/`shady` riddle. Capture
`internet-multiplexer-xwHf9pw6` earned the multiplexer but ended at that
second choice; its attempted save was not accepted. The reward validator
rejects it because presentation ownership has not been released. The revised
`bbb_internet_multiplexer_save.tsv` answers `shady` before opening Save.

The full `mutation_and_internet_reward_survive_fresh_loads` graphical test
passed in 206.82 seconds with optimized binary
`a99d1efb4c47a6335ee73cf776ad441ff2d4207450e705cae643e7c96a50e499`.
It starts from the earned writing save, selects Honk's authored `mutation`
choice, saves, loads that new checkpoint in another process, earns both Internet
items, completes Bernie's riddle, saves, and loads again. The three captures are:

- `output/fidelity/bbb-mutation-save-1788912645186386474-809228-0`
- `output/fidelity/bbb-internet-reward-1788912717882326488-809228-1`
- `output/fidelity/bbb-internet-reward-load-1788912836210828259-809228-2`

The mutation validator requires the original destinations (Daddy on Tromaland,
Mamy on Loviland, Papy on Templand), writing retained by Daddy, and technology
and ship transferred to Cyberquizz. The reward validator requires the rendered
six-choice puzzle, acknowledgement, decoder and multiplexer ownership, and
an unblocked return. The final process restores the items immediately and
does not rewrite either save. The saved credits word at VAR `0x1EF2` is one.

The reusable reward checkpoint is the second capture's `writable` directory,
slot name `abhonkwmi`. BLOOD.SAV SHA-256 is
`ed5d779a342b9b4ba1cdb647d80585e6a11708a4866971057af88b20b56fdce7`;
GAME1.SAV is
`3fb4ae1bd1cb51722ecc5b83ddf68ec24cc34d965a4bed31650409a58627162f`.
The test accepts `BBB_WRITING_SAVE_DIR` for its earned starting checkpoint.
These input-only replays add campaign evidence without changing production
logic or claiming ordinary migration, later quests, or the ending.

### Numeric Status Menu Audio

Opening the ship's status menu from the earned mutation checkpoint previously
aborted with `sequel numeric chatter hashing has not been verified against the
native audio routine`. This was initially misidentified as opening the Internet;
the activated record is `menu`, and the first numeric line reports credits.

The original `BLOOD2PG.EXE` routine at file offset `0xCF73` hashes each encoded
word as a NUL-terminated dictionary suffix (`0xCFA6..0xCFD2`). Unlike the menu
renderer, it does not substitute decimal state values for the numeric marker
`1` and its following operand. Both are dictionary lookups for audio, including
positions inside a word. Rust now retains that behavior, signed-byte additions,
encoded-word count, and zero/FFFF termination. This is owned dictionary access,
not runtime executable or hardware emulation.

`re/tools/big_bug_bang_numeric_chatter_oracle.py` executes the original hash
with synthetic dictionary data and produces 23 vectors. The production resolver
and audio selector match every vector, including suffixes, signed bytes, empty
strings, and operand terminators. Invalid out-of-dictionary operands still fail.
Reproduce the oracle with:

```sh
nix develop -c python -P re/tools/big_bug_bang_numeric_chatter_oracle.py output/big-bug-bang/disc/BLOOD2PG.EXE re/tools/oracle_vectors/big_bug_bang_numeric_chatter.json
```

Live replay `output/big-bug-bang/numeric-chatter-fixed-ZWgEPlit` completed 4,143
frames with optimized binary SHA-256
`a99d1efb4c47a6335ee73cf776ad441ff2d4207450e705cae643e7c96a50e499`.
It restored Daddy on Tromaland, Mamy on Loviland, and Papy on Templand, revealed
`CREDITS ... ... ... ... 0 CREDIT`, played streamed dialogue, and reached
`Anything else, Commander?` with visible yes/no rows and the hand. The inspected
screenshot is `internet.png` in that capture; its historical filename does not
establish Internet access. Both input save hashes remained unchanged.

`bbb_mutation_status_menu.tsv` and `mutation_checkpoint_numeric_status_menu_continues`
cover this continuation. The retained-trace validator accepts the repaired
capture and rejects the earlier `mutation-internet-He2IIFf1` crash. The mutation
save is `output/big-bug-bang/honk-mutation-save-3VQKDoJ7/writable`, overridable
with `BBB_MUTATION_SAVE_DIR`. Actual Internet access, ordinary migration,
later trades, and endgame are not established by this regression.

The full optimized graphical test passed in 46.12 seconds, retaining its binary,
scenario, manifest, and initial-save hashes under
`output/fidelity/bbb-numeric-status-1788911984835081420-800828-0`.
Game library tests passed 974 with 59 ignored; format tests passed 116 with 10
ignored. A renderer fixture now clears its resource-owned RGB layer before
testing the indexed fallback, then installs RGB explicitly for the independent
palette-ownership assertion. No renderer production behavior changed here.

### Daddy Writing Gift and Persistence

The earned six-item Honk save now continues through Travel ON, Tempest's
destination selector, Templand, Daddy's interlude, and `no_hurry` to a visible
GIVE menu. Its six rows are technology, guitar, perfume, energy, writing, and
ship, followed by Cancel. Every row has matching rendered text pixels; the
menu was also visually inspected in `daddy-writing-kcDeMios/dialogue-choice.png`.

`bbb_honk_checkpoint_templand.tsv` completed with both builds:
`output/big-bug-bang/daddy-inventory-retry-LaKzWAJV` (debug) and
`output/big-bug-bang/release-daddy-inventory-qbThtEra` (optimized). Both pass
the same retained-trace validator. The earlier failed navigation capture
`daddy-inventory-fPvH6TkA` is rejected because Travel never became enabled.
Reopening the console explicitly after load corrected that input sequence;
no engine gate was bypassed.

Selecting writing at logical `(200, 109)` changes `ecriture` from player-owned
`FFFF` to Daddy's record `0xC24`. The dialogue confirms that writing has made
the Gluxx intelligent, the other five items remain owned, and SCRIPT2 resumes
with presentation ownership released. `daddy-writing-kcDeMios` verifies this
transfer without saving; its original input file is retained as `scenario.tsv`.

The extended `bbb_daddy_give_writing.tsv` then saves through the ordinary menu.
Capture `output/big-bug-bang/daddy-writing-save-lWm64gbJ` completed all 42
actions, naming slot zero `abhonkw`. BLOOD.SAV SHA-256 is
`8097ee17250080e1bd77ff35c17cf074396452bc327e6d7b49848fbbd209c0b0`;
GAME1.SAV is
`2df2a13ab0ccd23c7b2edaa8eeafaf073b6527f0999c650e8329b531a1b65cff`.
Inspection of the saved VAR shows `0x1EE4` changing from zero to one and bit
`0x10` becoming set for Daddy, Mamy, and Papy. These are observed side effects,
not a claim that their later migration branches have been exercised.

Fresh-process replay `output/big-bug-bang/writing-checkpoint-load-PbbXFdIP`
uses `bbb_load_writing_checkpoint.tsv`. Its first loaded SCRIPT2 frame restores
Daddy's writing ownership and the other five held items; its endpoint is
unblocked. Loading does not rewrite either save. Both gift captures and the
fresh-load capture pass their dedicated validators. The menu-only capture is
deliberately rejected by the gift validator for lacking transfer and the
intelligence acknowledgement.

The full `writing_gift_survives_save_and_fresh_process_load` integration test
also passed under the optimized build, including its two separate game
processes and save-byte comparison. Its independently retained artifacts are
`output/fidelity/bbb-daddy-writing-1788910446370331032-780857-0` and
`output/fidelity/bbb-writing-load-1788910544491255022-780857-1`; the harness
records binary/scenario/asset hashes and copies the initial saves. Normal
progression/support tests pass (six tests, fourteen resource/display-dependent
tests ignored by default).

The optimized production binary used here has SHA-256
`07a7108e2f1ee582b0b9d94d3aeb3e2f6dbfaf947872364cf11e89031236e7c7`.
It was built with `cargo build --release -p commander-blood-game --bin commander-blood`
from the current worktree, without changing production code or desktop launchers.
Its separate Honk fresh-load replay (`release-honk-load-GmBQAOFe`) passes the
existing six-item/phone-response validator and preserves both saves. In these
private software-rendered captures, the same inventory-menu scenario took
about 98 seconds optimized versus 763 seconds debug; this is a replay-specific
measurement, not a general hardware frame-rate guarantee.

### Honk Departure Block Comparison

`big_bug_bang_honk_departure_oracle.py` executes the original outer VM loop
over COD `0x3478..0x34BD`, stopping before presentation scans. It loads the
unchanged complete SCRIPT2 COD/DIC and SCRIPT1 VAR, with synthetic reciprocal
Honk/player conversation records, a count of five or six, and four text-gate
states. The resume boundary is explicitly seeded in both implementations.
Input asset hashes are pinned; the probe checks unchanged executable and
dictionary bytes, restricted VAR and COD mutations, and balanced stack use.

All eight cases agree with Rust on complete VAR hashes, complete COD hashes
after text-activation writes, yielded presentation count, VM enable state,
subtitle/menu flags, and presentation request bits. The Rust fixture uses the
same retained directory-prefix read alias as the production profile loader;
the separate BAS stream is never entered at this boundary.

With count six, original A6 at `0x3483` can publish text and yield, but the
outer loop continues to C9 at `0x34A6` in the same pass and clears both C4
records. With menu, subtitle, or already-shown gates, the text is suppressed
but the records are still cleared. Count five skips the block. No production
change is justified by this comparison. It explains why a missing departure
line alone is not evidence of a Rust-only fault; it does not prove the later
renderer/post-scan visual timing or full conversation parity.

### Honk Inventory Save and Fresh Load

The input-only `bbb_honk_inventory_save_followup.tsv` replay completed in
`output/big-bug-bang/honk-save-followup-jcsWFXhd` (exit zero, all 33 actions).
It acquired the same six hold items, saved slot zero through the ordinary menu,
and contacted Honk again. The slot name became `abhonk`: the typed name appended
to the fixture's existing `ab`. The follow-up produced "I'm searching..." and
ended with released presentation ownership. An intermediate two-item snapshot
was not a collection failure; all six were present before the save action.

A separate fresh process loaded a copy of that checkpoint using
`bbb_load_honk_checkpoint.tsv`, retained under
`output/big-bug-bang/honk-checkpoint-load-micR5onA` (exit zero, all 15 actions).
All six inventory records were owned in the first loaded SCRIPT2 frame and
remained owned through another phone command. Honk responded with
"I'm searching...", the radio bank was present, and the final presentation was
unblocked. `loaded-checkpoint.png` was inspected: the phone console and hand
rendered. Both runs used the same production binary SHA-256
`778bcb45c062ff7411b24533ce20cfd49817e8f94c77aed39c3a9e7ab968a507`.

The saved BLOOD.SAV SHA-256 is
`59c464db72415b001dbbcebd9762466e6e9bab40a9ff68000dc4126ce56e91ea`;
GAME1.SAV is
`9664e38b00b2da8ad78274e1efde9e41802c7e1bdbb6a9e04ab57bbf8347cf1f`.
Fresh loading and contacting Honk did not alter either copied file. The
original Daddy fixture remains unchanged.

The progression regression now chains acquisition/save and fresh-process load.
Its load validator requires startup, the correct loaded profile, all six items
immediately on load and throughout the trace, a new phone response with the
radio bank, and released final presentation. The equivalent CLI runs and shared
retained-trace validators were exercised; the new chained process wrapper was
not separately rerun. This establishes a reusable inventory checkpoint, not
later trade, migration, BAS, or endgame coverage.

### Honk Hold Inventory Pass

The longer `bbb_load_honk_inventory.tsv` input-only replay completed all actions
and exited successfully. Its capture is
`output/big-bug-bang/honk-inventory-Dq8uYgJi`, using binary SHA-256
`778bcb45c062ff7411b24533ce20cfd49817e8f94c77aed39c3a9e7ab968a507`.
At the final frame 4141, six previously unowned inventory records have holder
`FFFF`: `ecriture`, `vaisseau`, `technologie`, `guitare`, `energie`, and `parfum`.
SCRIPT2 remains loaded, the VM is enabled, and Honk's actor presentation,
screen, chooser, and dispatch block are cleared. `after-inventory.png` was
visually inspected. The source/copied saves were not rewritten by this run.

The new progression regression requires those six specific acquisitions to
remain present at the endpoint, as well as the bank/chatter checks from the
shorter replay and released presentation ownership. Its shared trace validator
passes this completed capture. The earlier short capture was deliberately
checked and rejected for incomplete inventory progression. The full-process
test wrapper was not separately replayed; the equivalent CLI scenario and its
shared validator were run. Normal progression/support tests pass (six tests;
seven resource/display-dependent tests ignored).

The script's counter comparison near COD `0x347B` is greater-than five, not a
five-item cap. Its nearby excuse line at `0x3483` did not appear in this capture;
the last revealed line was the perfume discovery before ownership cleared.
The regression does not assert that unproven visual detail. The block comparison
above now confirms the original same-pass text-gate/return semantics, while
post-scan visual timing remains outside its scope. The later save/load replay
above verifies a subsequent command
and saving these items; later quest progression remains unverified.

### Honk Radio Bank Ownership

The cryobox-dialogue crash below was traced to a missing BBB-specific menu
effect, not an optional sound asset. The unchanged first console handler at
file `0x98AD..0x98D0` tests phase bit zero, publishes Honk's C3 deferred record,
clears the panel phase, and calls the sound loader with mode one and
`SN\\RADIO.SND` (`DS:0xF64`). Commander's corresponding handler has no bank load.
The shared Rust Honk path previously implemented only Commander's writes.

`activate_horn_choice` now receives the active dialect and publishes the BBB
radio-bank effect through the same runtime backend used by radio/navigation
commands. The effect is applied before the deferred record enters script
dispatch. No audio invariant was removed, no missing-bank fallback was added,
and Commander retains its existing no-reload behavior.

`big_bug_bang_honk_bank_oracle.py` runs all 256 phase bytes with two source
records (512 cases), validates the original menu jump-table target, checks
unchanged executable/global memory outside the expected writes, and records
the real loader-call arguments at its boundary. It does not emulate the disk
loader. Rust checks those vectors plus Commander non-reload cases; existing
Commander horn/radio vectors also pass. Regular library tests: 973 passed,
58 ignored. The game-package all-target check passes.

The new `bbb_progression` Honk replay shares the normal copied-save/process
harness and requires the bank before Honk dispatch, the cryobox dialogue, and
a new streamed audio event after that line. Merely entering Honk or retaining
an older audio event cannot satisfy the continuation check.

Live replay `output/big-bug-bang/honk-bank-GiQyVySj` completed every scenario
action and exited successfully with binary SHA-256
`778bcb45c062ff7411b24533ce20cfd49817e8f94c77aed39c3a9e7ab968a507`.
The last frame, 2766, is still in SCRIPT2 with Honk presenting "A document on
technological inventions"; seven streamed-dialogue events have been emitted.
The inspected `after-cryobox.png` shows a later dialogue line on screen. Both
copied save hashes remained unchanged. The shared retained-trace validator
passes this completed capture and rejects the prior crash capture specifically
because Honk lacks `radio.snd`. The new full-process integration wrapper was
not separately replayed after adding it; the equivalent CLI replay and its
shared trace assertions were run. This fixes the demonstrated bank crash, not
the entire Honk conversation or later inventory/quest progression.

### F7 Conversation Abort and Escape

BBB now uses its own Escape and F7 bindings. Escape is inert, Space retains
media cancellation, and F7 queues a deferred C9 presentation-end record, requests
SCRIPT2 from later profiles, clears the sequence/menu-count/chooser-phase
globals, starts ship opening at depth six, and enables VM execution. Commander
retains its prior bindings. These are native semantic state writes, not a DOS
execution dependency in the game.

`re/tools/big_bug_bang_input_abort_oracle.py` executes the unchanged original
handler at file `0x24C8..0x2513` and verifies the key table against executable
SHA-256 `4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834`.
Its 204 cases cover every profile, three pending requests, absent/present actor
links, and both actionable/suppressed deferred records. The probe checks all
global writes, unchanged VAR/executable bytes, and restored registers/stack.
Rust compares those cases with all 17 actual loaded resource profiles and
checks deferred C9 serialization, player-slot draining, phase-only reset, and
BBB/Commander key ordering. The abstraction preserves the deferred auxiliary
word's actionable sign, not an arbitrary low-bit payload; these tests do not
claim general raw-word parity beyond the tested zero/FFFF values.

Verification: 972 regular library tests passed, 58 ignored; the separate
`sequel_ --include-ignored` run passed 85 tests before the final deferred-drain
test was added. The final three `sequel_f7` tests passed, including the
resource-backed oracle comparison. All-target game-package checking passed.
Live recovery uses `accuracy/scenarios/bbb_load_daddy_f7_phone.tsv`, which sends
F7 through the ordinary SDL key queue after loading a copied progressed save.

Live capture `output/big-bug-bang/f7-phone-jseEzSAA` used binary SHA-256
`dd51833d92dcaa63ef78fe67bb83ea9f77ac92dcae892bebca1e2e2f8e2f6ec1`.
It loaded SCRIPT3 at frame 1107, returned to SCRIPT2 after F7 at frame 1383,
and activated Honk at frame 2080 after the ordinary phone click. English
dialogue starts at frame 2100; `honk-dialogue.png` visibly confirms the next
line. However, this is a **failed full scenario**, not a successful conversation
completion: after frame 2392 ("I'll put them in the cryobox for you...") the
process exits with `dialogue chatter is armed without a streamed DESCRIPT sound
bank`. The last trace has chatter armed, a pending one-word menu, and no loaded
streamed bank. Investigate `ModernGameServices::process_runtime_audio_events`
and the original Honk menu/audio path before relaxing the invariant. Both
copied save hashes were unchanged by the run. This proves the F7 return and
subsequent dialogue entry, while exposing the next progression failure.

### Conversation Profile Return and D2 Domain

The progressed-save Honk probe (`bbb_load_daddy_phone.tsv`) exposed a stall:
Honk became the active presentation at frame 1432 but SCRIPT3 stayed loaded,
with no subtitle or video through the final frame 2108. The bridge remained
visible. The local capture is `output/big-bug-bang/progressed-phone-SiRTtR7G`.

The original C9 handler at file `0x7BBF..0x7C0D` supplies a missing return path.
For an old C4 actor record, it clears the reciprocal action, stops the sequence,
sets depth step six, re-enables VM execution, and, when the current profile is
greater than one, overwrites the pending profile with one (SCRIPT2). Rust now
publishes those extra sequel effects from the typed dispatcher using the loaded
profile's identity. Non-C4 clears and Commander behavior retain their old rules.
The standalone executable probe `big_bug_bang_record_clear_oracle.py` executes
the unchanged handler and field resolver for 204 cases; the dispatcher test
checks those cases and Commander non-regression cases. The native UI-abort path
at `0x24C8` has a related profile-return write, translated in the F7 work above.
The native key table at `0x2281` maps extended key `0xC1` (F7) to dispatch
entry 14 at `0x2382`; that entry resolves to `0x24C8`. BBB's Escape no-op and
Space media-cancel bindings now differ from Commander's shared defaults.
BBB's separate horizontal-arrow handlers at file `0x2495` and `0x24A3` are
now translated. They increment or decrement byte GS:`0x6B7D`, with wrapping,
only while either low bit of the field-inspector mode at GS:`0x6B7C` is set.
The mode initializes to zero and the closed static graph finds no explicit
writer, so retail gameplay continues to treat both horizontal arrows as inert.
This closes the input-table discrepancy without inventing a way to activate the
dormant diagnostic inspector.

`re/tools/big_bug_bang_diagnostic_field_selector_oracle.py` executes both
complete handlers in the unchanged executable and verifies their byte ranges,
register and stack preservation, write bounds, mode mask and wrapping behavior.
The 56 checked-in cases cover next/previous, all low-mode-bit combinations,
unrelated high bits, ordinary selectors and both byte boundaries. The Rust
translation matches every case.

```sh
nix develop -c python3 -P \
  re/tools/big_bug_bang_diagnostic_field_selector_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/diagnostic-field-selector.jsonl
cmp re/tools/oracle_vectors/big_bug_bang_diagnostic_field_selector.jsonl \
  output/big-bug-bang/diagnostic-field-selector.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_horizontal_arrows_match_original_diagnostic_selector_handlers
```

Verification for this input slice (2026-09-12): fresh oracle generation is
byte-identical to fixture SHA-256
`673b565753b002ac0fa292d49a83ad5ed4fa4292ce5ba6c799b4a05ff2ceca49`;
all five input-dispatch tests pass. The complete game library passes 1,002
tests with 61 ignored and zero failures, serially, and the game package passes
all-target checking. Python bytecode compilation, Ruff, Rust formatting and
`git diff --check` also pass.
This change does not rewrite old saves already captured in the wrong profile.

D2 also incorrectly validated every game against Commander's five-profile table.
BBB SCRIPT2 explicitly requests higher profiles, for example operand six at
COD byte `0x073A`. The dispatcher now supplies the active dialect to the request
slot, allowing all 17 BBB profiles without broadening Commander's domain.
All 256 encoded operands are checked in both dialects, preserving signed
one-based conversion and the no-request sentinel. This is not live verification
of all later character routes.

The combined-fix live capture
`output/fidelity/bbb-templand-dialogue-1788903699516844185-4191979-0`
contains 5,440 frames from binary SHA-256
`4f2e9010f89a0e80a693dd3a4011f70d74a9a912d5e5c20f48512246d1750c57`.
Both choice labels remain visible in all 328 selecting frames. After the
ordinary `no_hurry` click, SCRIPT2 is installed at frame 4838. The final frame
5439 has no active video, actor presentation, screen, or ship dispatch block.
Automatic ship clips run after the conversation return; this is not a verified
later quest or a new Honk conversation.

The initial live assertions failed because they required a fully revealed
Tempest caption at an idle endpoint. The second capture's next animation
interrupts that caption at `PLANET: Temp`. The shared trace validator now checks
the actual Tempest target (record 72), cleared actor owner, closed chooser,
unblocked screen, and SCRIPT2 return, rather than subtitle timing. Revalidating
the completed combined-fix capture with `BBB_PROGRESSION_TRACE` and
`validate_recorded_templand_continuation` passes. The full process replay was
not repeated after that assertion-only correction. Library verification:
969 passed, 57 ignored; the separate `sequel_ --include-ignored` run passed all
82 tests; all-target game-package checking passed. Both live runs used copied,
disposable saves, and the original save files were not rewritten.

### Explicit COD Destinations and BAS Entry

The resource-backed regression
`sequel_cod_explicit_destinations_do_not_overlap_actor_bas_entries` loads all
17 profiles through the production profile manager, including INITIAL before
SCRIPT2 so its adjacent DEB alias is bound. It checks all 25,513 decoded
instructions: 11,832 explicit state destinations, of which 2,930 belong to
actors. None overlaps actor bytes 26..28, the original BAS-entry field. Byte,
word, pair, and triple spans are checked, including query-mode destinations;
the instruction match is exhaustive so new variants require an audit decision.

Together with `sequel_shipped_actor_bas_entries_are_zero` and the 512-case
original-executable gate oracle, this rules out an explicit authored COD
destination changing the initially zero actor entry. It does not prove BAS
unreachable: implicit handler writes, loaded saves, selected-concept entry,
and other native BAS readers are outside this destination check. SCRIPT2.BAS
is not discarded or replaced with an empty stream. This is static resource
evidence, not another completed gameplay route.

### English Subtitle Raster Coverage

All 15 bundled COD catalogs now run the shared original-resource RGB subtitle
check, including the 2,725 display sites in SCRIPT1 through SCRIPT4. The check
loads profiles through the real profile manager, verifies COD/DIC identity,
draws static subtitles with the executable's original font and colors, and
checks nonblank output and display bounds. Authored empty text must remain empty;
dynamic-number sites retain their state operands instead of becoming static
subtitles. Choice IDs and inventory generators retain their original bindings.

The 21 localization tests pass with imported resources using
`cargo test -p commander-blood-game --lib runtime::localization::tests -- --include-ignored`.
This is headless text-rendering evidence, not live verification of every choice,
scene transition, translation meaning, or gameplay route.

### SCRIPT2 Load and Native Transition Gate

The retained-state loader now binds SCRIPT2's VAR-relative word 8368 to the first
two bytes of the actual SCRIPT2.DEB resource. This is a distinct read-only word
identity, not trailing VAR storage or a copied SCRIPT2.VAR default. It cannot be
written, widened into a record, or serialized into a save image. Profile loading
clears old aliases before binding the verified sequel layout; Commander and
other layouts do not receive this binding. The original-resource loader test
executes the decoded `time > 19` comparison, checks unchanged retained VAR bytes,
rejects a write, and checks repeat loading and removal on return to SCRIPT1.

The native main-loop gate at file `0x116D..0x118A` checks navigation choice,
save, load, navigation transition, and actor transition. It does not use
Commander's additional UI/presentation gates. The sequel now selects that rule
after a successful VM pass. Lifecycle-owned presentation flags explicitly
cleared by the native loader at `0x588F..0x5906` are also cleared before the new
VM pass, preventing the old lifecycle snapshot from restoring them. Countdown,
hold-ready/completion, and UI ownership retain their observed values. Commander
keeps its previous gate and reset behavior.

`re/tools/big_bug_bang_profile_gate_oracle.py` executes the unchanged native
gate for 512 cases, including high-bit blockers and unrelated globals filled
with zero or 255. It also captures five loader-reset cases. Regeneration is
byte-identical. Rust checks every gate case against 256 combinations of the
additional presentation/UI flags, for both dialects.

Live run `modern-honk-play-03` first loaded SCRIPT2 at frame 2401 and exited
normally after 5604 frame records. With the native gate/reset changes,
`modern-honk-play-04` loaded it at frame 1104 immediately after PLAY and exited
normally after 2010 records. Both retain SCRIPT1.VAR. The latter visibly reaches
SCRIPT2's French instruction to visit the cryobox (`screen-013.png`). Its cryobox
attempt ends during a static-TV presentation (`screen-020.png`), with semantic
subtitle `WAIT COMMANDER ...`; it does not establish reaching Bob or completing
the cryobox sequence. SCRIPT2 and the remaining profiles are still untranslated.
Post-load navigation/counter/HUD ordering also needs further native comparison.

Verification: 944 game-library tests and 116 format-library tests passed
serially (30 and 10 ignored respectively); the original-resource SCRIPT2 loader
test was run explicitly and passed; all-targets checking passed. Unrelated user
worktree edits were present but are not included in this change. The older
load-failure sections below describe superseded checkpoints, not a current
SCRIPT2 rejection. Full gameplay and English localization remain incomplete.

### Native PLAY Transition and Adjacent Directory Ownership

Two bounded original-executable runs now reach HONK's `JOUER` choice and
profile 1 (SCRIPT2) through private-X11 pointer input. The experiment reads
guest memory through the existing capture inspector; it does not write guest
state or force the profile. Both runs use the verified original executable,
DOSBox-X normal CPU core at 30,000 cycles, and separate writable directories.
The local experiment and raw captures remain ignored under
`output/big-bug-bang/`; they are evidence, not a packaged regression harness.

`native-play-02` first observed profile 1 at 348.068 seconds. The independent
`native-play-04` run first observed it at 110.989 seconds and retained it through
the 360.285-second final observation. Its 501 profile-1 observations all have
consistent resource bindings and the same ownership result:

- Retained SCRIPT1.VAR: handle 2, linear address 526960, allocation 8368 bytes.
- Active SCRIPT2.DEB: handle 38, linear address 535328, allocation 11168 bytes.
- VAR-relative byte 8368 therefore addresses SCRIPT2.DEB byte zero, outside VAR.
- The two bytes read as 24930 (`0x6162`), the beginning of the directory name
  `baby1`, not a separately initialized time variable.

The second run's `after-05.png` visibly shows `QUE DESIREZ-VOUS?` and
`JOUER` / `EXPLICATIONS`; its final image shows the forward bridge view after
the profile transition. Event-stream SHA-256:
`ca9824931bca045a7b3ff2d8653984394421c2de7b968a354fc5a363e085073f`.
Final guest-dump SHA-256:
`1d66443fa00d01c1f8a28da2ecddf6ab712fcf927c4d49552cd44cc9634e5841`.

This superseded the earlier lack of a live native SCRIPT2 ownership capture;
the capture alone did not change the modern loader or prove execution of the particular
COD condition. The binding described above derives the read from the actual adjacent
directory bytes, preserves VAR serialization, and avoids making directory data
writable script state. The original also transitions shortly after selecting
PLAY, whereas the earlier modern run continued a long HONK speech before
attempting the load. The native gate correction above removes that delay in
the repeated PLAY scenario. Full gameplay and English coverage are still incomplete.

### English Inline Prose and Live Transition Blocker

The matching opening catalog now supplies display-only inline menu prose as well
as subtitles and retained choice labels. Binding requires both the accepted A6
instruction and its complete authored word stream. The shared layout/reveal
routine accepts an optional display stream, leaving original dictionary words,
VAR operands, and chatter inputs untouched. English word count controls only
display reveal and completion timing. The original entry point supplies no
override and retains its native comparison coverage.

Trace snapshots retain the original word stream separately from displayed words
and do not assign dictionary IDs to translated prose. Raster reconstruction uses
the same bound display stream, and a changed stream cannot reuse a stale reveal
snapshot. All 89 English prose sections pass inline-font raster and screen-bound
checks; an original-resource runtime binding test covers Bob's first menu source
(`0x977`) and rejects mismatched words and profile-reset state. Those are isolated
checks, not live Bob/OLGA reachability.

The longer ordinary-pointer PLAY run in ignored local
`output/big-bug-bang/modern-honk-play-02` exited with game status 1 after 2,401
trace records. It selected PLAY, continued the opening English dialogue, and
attempted to load SCRIPT2. The actual load failed with `InvalidStateWord` at
COD byte `0x5A97`, VAR byte `8368`. Thus the shared-state blocker described below
is now reached through ordinary UI progression, not only an isolated loader call.
No extra VAR word or neighboring-resource value has been fabricated to bypass it.

A separate ordinary-pointer cryobox attempt (`modern-cryo-01`) opened an empty
contact list in the initial profile and exited normally after 1,773 records.
It did not reach Bob's recording, so the new prose path still lacks that live
visual check. Full English gameplay remains incomplete.

Verification: 942 library tests passed serially (29 ignored), all three menu
reveal tests passed including native numeric-renderer comparisons, the four
original-resource localization checks passed, the runtime menu-binding check
passed, and all-targets checking passed. Existing unrelated worktree edits were
present for the checks but are not part of this change.

### English Opening Choices

The retained dictionary-choice renderer now binds English labels to the accepted
A6 source instruction and exact ordered dictionary IDs. Layout measures the new
labels; completion still submits the original identity. The bundled opening
catalog supplies all three static choice sections. Inventory choices, unknown
sites, reordered word lists, and unmatched resources do not receive an override.
Profile reset clears the last published site; gated text does not replace it.

The ordinary-pointer run `output/big-bug-bang/modern-honk-play-01` reached the
opening menu with rendered labels `PLAY`, `INSTRUCTIONS` and original selector
words `JOUER`, `EXPLICATIONS`. Screenshot `screen-019.png` confirms the English
labels. Clicking logical position (185, 94) completed the first choice and
requested profile 1 (SCRIPT2), then continued English dialogue. The run exited
normally after 2,004 trace records, still during that dialogue with the profile
request pending. It did not establish a completed SCRIPT2 transition.

Verification: all 941 enabled library tests passed serially (28 ignored), the
four localization tests passed with original resources enabled, the isolated A6
dispatch comparison passed, and all-targets checking passed. Source tests check
all three choice sections and reject mismatched/reordered choices. Only the
opening menu has a live visual/selection check in this slice; menu prose and the
remaining profiles are still not translated at runtime.

### Text Hold Script Resume

The sequel coordinator at BLOOD2PG file offsets `0x11D2..0x1280` writes the
VM execution flag while a completed text reveal still has a nonzero hold timer.
It enables execution only when the scene gate is clear. The shared lifecycle
now selects this rule from the runtime's game dialect; Commander keeps its
previous behavior. Zero countdown and the active/ready early branch do not
perform this write.

`re/tools/big_bug_bang_text_lifecycle_oracle.py` runs the unmodified native
instruction range with 2,048 synthetic input combinations, checks bounded
execution and allowed writes, and verifies module immutability. Its committed
vectors cover both initial VM states, scene gates, secondary input, and timer
values 0, 1, 255, and 256. The Rust comparison checks the VM flag only, not the
subsequent presentation coordinator. This is native boundary evidence, not
proof of full gameplay reachability.

The private DOSBox capture helper now observes guest state while the mouse
button is held, releases the button even if observation fails, and supports
captured relative motion before scheduled clicks. Requested movement is not
reported as confirmed guest movement. A real capture observed primary button
state 1 during the press; large relative deltas were not consistently accepted
in full by the original runtime.

Verification: 941 game-library tests passed (28 ignored), 30 capture-helper
tests passed, all-targets checking passed, and regenerating the 2,048 native
vectors produced an identical file. Existing unrelated runtime edits were
present during these checks and remain outside this change.

The ordinary-pointer scripted run in local ignored
`output/big-bug-bang/modern-honk-navigation-05` advances HONK's first English
line to "What would you like?" and reaches retained word-choice phase
`Selecting`. Screenshot `screen-020.png` shows both menu entries, still French
(`JOUER`, `EXPLICATIONS`), and the final subtitle has 424/424 matching raster
pixels. The pre-fix run `modern-honk-navigation-04` remained on the first line.
The scenario uses the existing worktree's pointer-based `park` action, not VM
state injection. This verifies opening conversation progression, not the menu
branches, full translation, or complete playability.

### Game Identity and Loose-Asset Import

The game executable now has a library-only, import-only mode for either game:

```sh
nix develop -c cargo run -p commander-blood-game --bin commander-blood -- \
  --data output/big-bug-bang/disc \
  --import-assets output/big-bug-bang/imported-assets
```

This extracts original resources into ordinary files, verifies their checksums,
and exits without opening SDL or transcoding audio/video. It is preparation for
the shared runtime, **not a playable sequel mode**.

`GameVariant` owns each game's display name, storage namespace, executable/title
filenames, script dialect, and native resource/profile catalog selection. Sequel
catalog decoding requires the exact analyzed executable SHA-256. Importing a
different executable revision does not imply that its native tables are supported.

The importer detects the game from its main executable, rejecting a source with
both main executables. Both games use `BLOOD.DAT`, so the archive filename cannot
identify the game. Sequel companions are `BLOOD2PG.EXE`, `BLOOD2.LBM`, `TB.BIG`,
and `DESCRIPT.DES`. Neither the sequel disc nor its archive contains `BLOOD.SAV`;
the importer does not borrow or invent that template, or any missing BAS files.
Commander's existing companion requirements are preserved in this slice.

Manifest schema one gains an explicit `game` field. Existing manifests without
it remain Commander imports, with required Commander companions still checked.
This preserves existing media caches rather than forcing their regeneration.
Reusing an import now checks game identity, executable fingerprint, and available
source archive/loose-file fingerprints. Missing source archive data can still be
served by its completed cache, as before; that path is not a new source comparison.
A different game/build/content fails without replacing the destination. The game
identity check also applies to damaged caches. Source/destination overlap is
rejected before copying or replacing directories.

Default writable/cache namespaces are distinct: `commander-blood` and
`big-bug-bang`. Existing `CBLOOD_WRITE_DATA` and `CBLOOD_ASSET_CACHE` overrides retain
their exact Commander meaning; sequel defaults beneath those overrides use a
`big-bug-bang` child. Explicit path arguments remain exact caller-selected paths.
This is path/import isolation, not a completed sequel save-format implementation.
An explicit `CBLOOD_DATA` source is now resolved before default cached Commander
data, so a requested game is not silently replaced by the cached one.

At the import-only milestone, production path loading explicitly rejected a sequel manifest before calling
remaining Commander-only presentation decoders or starting media conversion. The new
native catalog selection is used by the existing Commander loader and by the
sequel imported-profile integration test. The rest of the sequel runtime still
needed connecting; rejecting it was not counted as game support. The startup
integration below supersedes that guard; earlier guard references in this log
describe their respective historical milestones.

The original-disc import integration test verifies every imported checksum, then
constructs an archive-free resource store and loads the initial sequel profile
through `ScriptProfileManager`. It verifies native COD ownership for missing BAS
and exact initial VAR bytes. The original-disc dependency is marked ignored by
default and was explicitly enabled when checking this slice.

Verification for this slice (2026-09-05): all 14 import tests passed with the
original-disc test explicitly enabled. The full game library passed 912 tests
with nine ignored, serially on a freshly allocated private Xvfb display. Game
all-targets checking passed. Checks ran with the existing unrelated Commander
runtime edits, which remain outside this change.

The CLI command above was also run with `DISPLAY` and `WAYLAND_DISPLAY` unset.
It imported 1118 resources from 944 archive entries plus loose files, with all
checksums verified. The resulting local manifest SHA-256 is
`1bb7199d23d840629eadfe4a7df656eaa04b8e3bc66a278509ba22c617408617`;
its archive hash matches the inspected original sequel disc. The output contains
only `resources`, `companions`, and `manifest.json`: no fabricated BAS/save data
or media transcodes. Original assets remain ignored by Git. Attempting a
production one-frame run against this imported tree returned the explicit
not-yet-integrated sequel runtime error before opening SDL. That negative check
verifies the guard, not gameplay.

### Sequel Fonts and Bridge Tables

Native references and direct original-binary comparisons establish the following
table locations. Game-specific decoders now read these into owned, flat tables;
the font tables feed the existing RGB glyph importer in integration tests.
This does **not** establish complete sequel UI behavior or enable its production
loader. All offsets below refer to the analyzed `BLOOD2PG.EXE` file;
its global-data base is file 0xF7F0.

| Table | Map/Start | Advances | Glyphs | Extent |
| --- | --- | --- | --- | --- |
| Bridge anchors | 0x14AC9 | n/a | n/a | 66 consumed bytes, identical to Commander |
| Bridge trigonometry | 0x14B05 | n/a | n/a | 180 four-byte samples, identical to Commander |
| Navigation actors | 0x124AB | n/a | n/a | Six 24-byte records with sequel resource IDs/flags |
| Small font | 0x1709E | n/a | 0x1711E | 128 map bytes, 42 five-row glyphs |
| Subtitle font | 0x171F0 | n/a | 0x172D8 | 232 map bytes, 66 eight-row glyphs |
| Square font | 0x174E8 | 0x175D0 | 0x17602 | 232 map bytes, 49 reachable glyphs |
| Main font | 0x179D6 | 0x17ABE | 0x17B16 | 232 map bytes, 87 reachable glyphs |

The bridge projector at file 0xB337 loads data offset 0x52D9 and a count of 11;
the 11th consumed anchor overlaps the start of the trigonometry data, as in
Commander. The matrix routine at 0xB058 loads trigonometry offset 0x5315.
The dual-font measurement routine at 0x344D references square map/advance offsets
0x7CF8/0x7DE0 and main offsets 0x81E6/0x82CE. The subtitle renderer at 0x39BE
references 0x7A00/0x7AE8; the small renderer at 0x3A78 references 0x78AE/0x792E.

The sequel map maxima, excluding sentinel 255, independently confirm reachable
glyph indices 41, 65, 48 and 86 respectively. Commander has only 176 entries in
its proportional maps and 55/48/86 subtitle/square/main glyphs. Simply relocating
its fixed arrays would omit sequel characters. The font audit also found changed
main-font advances. The decoder preserves their actual values rather than assuming
shared glyph bitmaps imply identical text layout. Main glyphs 69/71 advance by
5/8 respectively in the sequel, versus 8/5 in Commander. Variable-sized owned
font tables retain each game's exact dimensions; they neither truncate sequel
characters nor pad Commander tables. The small font remains identical.

Native text measurement indexes advances even for map sentinel 255 and subtracts
two with unsigned 16-bit wrapping. Import retains the original 256-byte lookup
region for each measurement face. This is serialized lookup data, not an emulated
memory space. Display advances remain separate, signed-byte glyph tables.

`re/tools/big_bug_bang_font_width_oracle.py` executes the **complete original**
procedure at file 0x344D through its far return at 0x3485, with original font
tables and synthetic strings. No native call is replaced. It guards the executable
SHA-256, execution range, register preservation, source/data immutability, and
stack-write range. Its 492 vectors cover both faces, every input byte 1 through
231, empty/NUL-terminated text, extended characters, and width overflow. The Rust
measurement test matches all 492 original results. The vectors contain synthetic
strings and measured widths, not original executable code or glyph bitmaps.

Separate RGB integration tests compare imported glyph coverage, subtitle reveal,
and channel masks against our **C-derived Rust raster functions** for all mapped
sequel characters. They include byte 225, beyond Commander's map, and all 89
mapped square-cap characters. These are not original-sequel framebuffer captures
and must not be presented as equivalent evidence. Native-table tests additionally
verify the bridge anchors, trigonometry and all six actor records against original
bytes. Actor resource IDs are 17, 13, 15, 16, 19 and 18; reusing Commander's IDs
would be incorrect even though the projection tables match.

The sequel planar square-cap routine at `0x37A8` adds a destination-selection
branch before the inherited text loop. At `0x37D0` it loads GS:`0x55E9`, tests
word GS:`0x6B94`, and selects GS:`0x55ED` when that word is zero. Commander has
only one unconditional buffer selection. This is now classified as a native
buffer-ownership difference, not an unported text-rendering path: the flat
runtime supplies the surface owned by the inventory or dictionary-choice
caller instead of preserving segment-valued framebuffer pointers. The sequel
planar-main entry is `0x38FC`.

`big_bug_bang_planar_square_caps_oracle.py` executes the complete unchanged
`0x37A8..0x38FC` routine. Its 12 cases include five draws to each native buffer
and two clipping exits that touch neither. They verify both destination
pointers, the GS:`0x6B94` selector, VGA port order, exact selected-buffer bytes,
unselected-buffer immutability, all four starting planes, signed advances,
source wrap, inherited backward direction, output and width wrapping, register
preservation, flags, and far-return stack discipline. The shared flat renderer
matches all ten bounded outputs; its two rejected address-wrap cases remain the
documented checked-memory boundary. The report at
`re/tools/oracle_vectors/big_bug_bang_planar_square_caps.json` has SHA-256
`3b4a25ace8e6d69d9ab43590bf0e0fedb378355275ecf25ef3ad7e748ec90359`.

```sh
nix develop -c python -P re/tools/big_bug_bang_planar_square_caps_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_planar_square_caps.json
```

`GameVariant` selects and fingerprint-checks both new decoders, and the Commander
runtime now accesses fonts/bridge tables through that same identity boundary.
The sequel production-loader guard remains in place until remaining presentation,
menu and host-state behavior is implemented from evidence.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_font_width_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/font-width-verified.jsonl
cmp re/tools/oracle_vectors/big_bug_bang_font_width.jsonl \
  output/big-bug-bang/font-width-verified.jsonl
nix develop -c cargo test -p commander-blood-formats --lib -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib native::bloodprg::font::tests -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib ui::tests -- --include-ignored
```

Original-disc font/bridge tests are explicitly ignored by default and were
enabled for this verification. Missing original assets are not counted as a pass.

Verification for this slice (2026-09-05): regenerated all 492 width vectors and
compared the output byte-for-byte with the saved fixture. All 119 formats library
tests passed with original corpus checks enabled. All ten native font tests and
eight UI-filtered tests passed with ignored checks explicitly enabled. The full
game library passed 912 tests, with 11 ignored, serially on a fresh private Xvfb
display; its server was reaped afterward. Game all-targets checking passed.
Existing unrelated Commander runtime edits were present during these checks but
are excluded from this checkpoint. No full-game sequel parity claim follows.

### A6 Condition Audit and Dynamic Inventory Choices

Direct assembly comparison found an inherited Rust translation omission:
both Commander at 0x636B and the sequel at 0x6B5A test the sign of the low control
byte before testing detail bit zero. Control flag 128 overrides equality and
requires signed `record > operand`. The recovered C `vm_condition_5` already
preserves that priority; Rust previously selected equality whenever detail bit
zero was set. `ScriptTextControl::uses_record_equality` now encodes the priority
and is used by the shared condition evaluator for both games.

`re/tools/text_record_condition_oracle.py` executes each original complete
condition procedure with only record-condition controls enabled: Commander
0x6339..0x6432 and sequel 0x6B28..0x6C44. It uses each executable's real field
matrix, guards both SHA-256 values, checks preserved registers and source/data
immutability, permits only the four stack-scratch bytes, and rejects execution
outside the procedure. No callee is substituted. The 288 vectors cross all four
equality/ordering flag combinations with signed boundary values. They do not
cover the unentered PRNG, history, or menu branches of these procedures.

The new Rust regression was run before the fix and failed at flags 388,
record zero, operand zero: Rust returned true; the original returned false.
This is a reproduced assembly/C-to-Rust discrepancy, not an inferred UI patch.
After the fix all 288 comparisons pass, along with the existing condition
vectors (five condition tests total). All 119 formats tests passed with corpus
checks enabled; the complete game library passed 913 tests with 11 ignored,
serially on a fresh private Xvfb display, which was reaped afterward. Game
all-targets checking passed. These checks include the same unrelated local
Commander runtime edits, not staged with this repair.

The following **unported** sequel paths were established by assembly inspection.
The condition and audio-gated transfer paths now have native component captures
described below; full presentation and authored execution remain unverified:

- A6 entry 0x6C89 saves its current selector-byte position to GS:0x6B4E.
  Its condition helper 0x6B28, on the resume/post-list path, recognizes the
  special word 65534 at 0x6C10 and calls 0x6C45. A zero count clears the yield
  and resume bytes and returns failure. A nonzero count copies the saved line
  position to GS:0x6B94 and returns success.
- The complete helper 0x6C45..0x6C88 scans GS:0x70E6 until word 65535, skips
  zero slots, tests the VAR record kind for mask 1024 (`InventoryItem`), and
  writes each selected record offset plus four to GS:0x6BDC. It terminates the
  result with zero and returns the count in AL. These are object-backed names,
  not DIC offsets and not ordinary actor records.
- The original GS:0x70E6 table contains sixteen zero slots plus a 65535
  terminator. Startup clears sixteen slots at 0x5890. The loader at 0x59FF
  populates it from active directory entries whose selector-17 field is 65535.
  Helpers 0x65E8 and 0x6606 remove or insert within those sixteen slots.
- The ordinary concept-consumption routine at 0x5C41 has a sequel-specific
  branch when resume bit two and GS:0x6B94 are both set. It clears that saved
  line and pending presentation state, marks the saved COD line, removes the
  chosen object from the sixteen-slot table, writes its selector-17 relationship
  from the saved line operand, and sets record flag 64. Later presentation calls
  in the same branch still need complete integration/oracle coverage.
- GS:0x6B94 also selects UI call order in the main loop (0x138B/0x13A3),
  background/text drawing paths (0x963E/0x970D/0x9754), and the planar text
  destination (0x37D5). The two page offsets are swapped at 0x43ED..0x43F7.
  Fixing only the font destination would omit the underlying interaction.

The typed `ScriptTextWord::InventoryChoices` now represents marker 65534 only in
the sequel dialect, alongside dictionary words, numeric operands and separators.
A6 expands it through the owned roster and object-selection state; UI routing
is still incomplete. Do not invent dictionary entries, substitute all actors for the
native candidate table, or enable sequel production loading with this flow
missing. Authored execution reachability and the later presentation calls remain
to be verified before calling the whole A6 path recovered.

#### Native Inventory Condition and Transfer Captures

`re/tools/big_bug_bang_inventory_condition_oracle.py` executes the complete
condition procedure 0x6B28..0x6C44 with authored controls `0x8030`, including
the real bytewise separator scanner 0x68A5 and inventory helper 0x6C45..0x6C88.
Its 22 vectors cover empty inventory, every one of the sixteen roster slots,
full and mixed rosters, raw kind-mask filtering, and duplicate entries.

The native result preserves slot order and duplicates, skips zeros, and returns
VAR record offsets plus four as choices. An empty result clears resume and
yield, returns carry clear, and **leaves the previous saved-line word intact**.
Spoken mode is enabled before that rejection and remains enabled. A nonempty
result sets yield and replaces the saved-line word. These are state effects,
not just candidate-list filtering.

Each valid inventory candidate then enters the complete selection procedure
0x5C41..0x5D5C with its original removal and field-matrix helpers. The 82 captures
exercise both native audio gates separately: global gate bit 0 at GS:0x2A33 and
dialogue gate bit 1 at GS:0x6B80. No helper is replaced or skipped by the harness;
these input gates naturally avoid the unresolved audio lookup at 0x8450.
Raw combined-kind candidates are excluded from transfer captures because the
field helper selects the least significant kind bit, unlike candidate filtering.

The transfer removes the first matching roster slot only, writes the saved A6
actor operand into selector 17 (inventory byte offset 20), sets object flag
`0x40`, and sets the high control byte's `0x80` enable bit in the saved COD line.
It clears selected and alternate concepts, saved line, resume and the pending
choice-list head. Yield and spoken mode remain set. Unchanged-memory guards
also establish that this branch does not append the selection to concept history.

Both executable SHA and entered-code ranges are guarded; all writes must fall
within explicit output or stack ranges, preserved registers are checked, and
source/state/global bytes outside those outputs must remain unchanged. Two
independent generator runs produced byte-identical fixtures:

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_condition_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_condition.jsonl \
  re/tools/oracle_vectors/big_bug_bang_inventory_selection.jsonl
```

`SequelInventoryState`, owned by each profile's selector state, now represents
saved A6/recipient identities and pending/selected objects without `ScriptWordId`.
Its condition component matches the 20 native cases representable by the strict
VAR decoder, and its gated transfer matches 80 cases. The two raw-kind condition
vectors and two associated transfers contain invalid or combined record kinds;
they remain native robustness references, not claimed Rust parity coverage.

Dispatch routes a pending inventory selection before ordinary DIC encoding,
history insertion and BAS processing. Transfer updates owned VAR and the derived
relationship-field view directly. It does not rebuild the roster: reconstruction
would compact holes and erase the native first-duplicate-only removal behavior.
A dispatch regression checks unchanged concept history, retained duplicate slot,
reactivated A6 state and exact VAR synchronization after transfer.

The ungated operation retains an explicit descriptor-lookup continuation and
the selected object, whose native clearing occurs after descriptor processing.
Dispatch now completes that lookup through the descriptor backend, as detailed
below; an absent backend remains an explicit error. Invalid selections, inactive resume,
missing instruction state and wrong dialect have explicit rejection checks.

Authored inventory A6 text now decodes and invokes the condition component after
the native gates. The extended oracle executes 0x6C89 through its return at
0x6E53 with all entered helpers intact: 27 synthetic captures cover publication,
empty choices and five rejection gates. Rust matches the 25 cases representable
by its strict VAR decoder. The two raw-kind cases remain native-only references.

Another 46 captures execute every original inventory A6 occurrence using pinned
COD/VAR/DEB/DIC images, with controlled actor action, shown flag and a one-item
roster. Rust matches subtitle hashes, VAR effects, resume state and publication
flags. These controlled handler checks do not prove gameplay reachability.

The dispatch regression now decodes and executes a synthetic A6 through a complete
frame before selection, checks duplicate choices, transfers the selected object,
then prepares another frame. Sequel field refresh preserves the canonical roster,
including holes and duplicates. Native pre-frame helper 0x6038 updates actor
relationships; direct calls to roster reconstruction 0x59FF occur at 0x11A8
(profile change) and 0x1FAF (load). Commander refresh behavior is unchanged.

The original handlers in both games also clear the alternate concept while arming
resume, before rejection gates; the shared Rust handler now does so. Runtime
choice readiness includes object choices without converting them to dictionary IDs.

Object-name display and UI selection/cancellation are now connected through the
shared chooser, as detailed below. Descriptor lookup is connected, but playback
completion and the profile lifecycle remain unfinished. Production sequel loading stays disabled until
the complete flow is integrated and verified.

Verification: all 123 formats tests pass with original-corpus tests enabled;
the game library passes 925 tests with 15 ignored under a private X server.
The authored inventory test (46 cases) and numeric-menu oracle test (59 cases)
also pass when explicitly enabled. Regenerated condition, transfer, synthetic A6
and authored A6 fixtures are byte-identical to the checked-in vectors.
No full-game inventory-playability or independent-review claim follows. Reproduce
the complete text captures after generating the corpus audit:

```sh
nix develop -c cargo run -p commander-blood-formats --example audit_sequel_text -- \
  output/big-bug-bang/imported-assets/resources > output/big-bug-bang/text-audit.json
nix develop -c python3 -P re/tools/big_bug_bang_inventory_condition_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/inventory-condition-oracle.jsonl \
  output/big-bug-bang/inventory-selection-oracle.jsonl \
  --text-output re/tools/oracle_vectors/big_bug_bang_inventory_text.jsonl \
  --resources output/big-bug-bang/imported-assets/resources \
  --audit output/big-bug-bang/text-audit.json \
  --authored-text-output re/tools/oracle_vectors/big_bug_bang_authored_inventory_text.jsonl
```

```sh
nix develop -c python3 -P re/tools/text_record_condition_oracle.py \
  re/bin/BLOODPRG.EXE output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/text_record_condition.jsonl
nix develop -c cargo test -p commander-blood-game --lib native::bloodprg::presentation::tests
```

#### Inventory Choice Panel

`PresentationChoiceId` distinguishes dictionary concepts from inventory objects
through panel opening, selection, closing and script publication. The runtime
resolves inventory labels from the offered VAR records' bounded name fields
(bytes 4..20), preserving order and duplicate entries. All 425 authored inventory
records across the seventeen profiles have valid names: 25 distinct labels,
including a CP437 accented name. These are not DEB symbol names or DIC words;
English localization must translate display names without renaming object IDs.

`big_bug_bang_inventory_choice_oracle.py` executes the complete original chooser
0x9B45..0x9C5D with list 0x958A, transition 0x20CE, width 0x344D, planar text
0x37A8 and background remap 0x3F13. Eleven sequences cover waiting, selection,
cancellation, wide/accented labels, full sixteen-item rosters and two ordinary
dictionary-choice controls. No callee is
patched. The original disabled-sound input gate naturally avoids device playback.
These captures verify control, layout and helper order, not planar VGA pixels or
audible playback. The separate direct `0x37A8` oracle above supplies the
selected-buffer and planar-byte evidence for that entered helper.

The opening layout has no cancel row. On later updates the inventory branch sets
the cancel flag; interactive layout adds the row with a minimum content width of
71, compared with Commander's 55. The current rectangle is recomputed for this
layout and retained for closing. The cancel label is decoded from the verified
sequel executable, not borrowed from Commander resources.

The sequel chooser pauses script execution when opening and re-enables it on
selection (before closing finishes). This also applies to ordinary sequel word
choices; Commander's chooser does not write the VM latch. Lifecycle imports now
refresh both this latch and the current modal UI bit when reopening a panel.

Rust matches every captured frame's rectangle, transition step and background
region, row text/position/color, phase and completion latches using original font
resources and the real translated transition helper. RGB text coverage is also
checked against the translated font rasterizer, including nonblank and panel-bound
checks. UI completion clears the pending choices but retains the saved line and
resume for an object selection; cancellation clears those and the alternate
concept without transferring an item. A subsequent frame dispatch test uses the
same typed completion operation before the native-referenced transfer.

Four additional native captures execute main-loop 0x1384..0x13AF and the complete
base-frame conversion helper 0x434B. Inventory choices run after base submission,
even without the ordinary frame-presented flag. Dictionary choices remain before
submission and retain that flag's gate. The shared lifecycle now follows this
ordering, and profile changes reset the cached inventory-line owner.

The modern UI binding is implemented, but live sequel startup remains guarded;
these tests do not establish end-to-end gameplay or playback completion.
The original eleven chooser/inventory tests pass with original-asset tests
enabled, and repeated native captures are byte-identical. Reproduce the native
panel and ordering captures with:

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_choice_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_choice.jsonl \
  --order-output re/tools/oracle_vectors/big_bug_bang_inventory_choice_order.jsonl
nix develop -c cargo test -p commander-blood-game --lib inventory -- --include-ignored
```

#### Pre-Frame Occupancy While Paused

The sequel calls 0x6038 before testing its VM enable bit at 0x5A9C. Commander
tests its enable bit before preparing state. The shared frame runner now respects
that distinction even for an empty COD program: a paused sequel frame prepares
state without executing instructions or post-scan handlers.

The sequel preparation pass first clears location flag 4 and the occupant word
at byte 24. It then normalizes actor position flags through the original holder
and coordinate semantics. Each actor with flag 4 and a nonnegative direct
location holder sets that location's flag 4 and occupant word. Nested holders
do not count as direct occupancy, and the last qualifying actor in directory
order wins. This is separate from the aboard-inventory roster, which this pass
does not rebuild. Commander retains its actor-only updates. Typed validation
failures leave both actor and location state unchanged.

`big_bug_bang_state_processor_oracle.py` executes the complete original helper
and unmodified 0x67B8/0x6633 callees in 21 synthetic cases. Seven enter the paused
frame at 0x5A99 after resource binding and execute its real return epilogue.
Rust compares complete VAR images, covering transient-flag gates, stale occupancy,
last-writer order, direct versus nested holders, zero/sentinel parents and
world/arche coordinate matching. The fixtures regenerate byte-for-byte. These
are component captures, not proof of initialized full-game state.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_state_processor_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_state_processor.jsonl
nix develop -c cargo test -p commander-blood-game --lib actor_position_state
```

#### Inventory Descriptor Continuation

The complete transfer at 0x5C41 now continues through DESCRIPT lookup at 0x8450
using the selected object's bounded VAR name. Successful lookup applies the
decoded commands, clears the scene queue gate, requests secondary
presentation line 43 and pauses the VM. A completed miss clears the selection
without requesting playback. A backend failure retains the continuation; retry
does not repeat the transfer or remove a second duplicate roster entry. A missing
host binding is an error, not a fabricated descriptor miss. Lifecycle publication
consumes the VM write once so late text updates cannot replay it over a UI write.

`big_bug_bang_inventory_descriptor_oracle.py` executes the complete transfer,
lookup and every entered helper unchanged. Its explicit INT 21 boundary handles
open/read/seek/close against owned database bytes. Twenty synthetic cases cover
success, case-sensitive misses, missing files, both native gates, empty records,
captions and accented names. Another 25 captures use all authored inventory
descriptors from the hash-pinned DESCRIPT database. No descriptor result is
substituted into native execution. Repeated captures are byte-identical.

Rust dispatch and the real typed DES parser match all 45 cases. An additional
original-asset test checks the concrete runtime backend's 25 object bindings,
selected clip names and resource availability. All these records select HNM
clips without loading an SND bank during lookup. The existing presentation
catalog retains the previous clip when a record supplies no new object-video
command; the per-record asset list is not the owner of that retained filename.

Correction to the initial descriptor integration: GS:0x2200 is the C2 scene
queue gate, not the handoff lock at GS:0x6B8D. Successful lookup clears only
the former and preserves the latter. The regenerated descriptor captures now
name and compare both fields separately; the previous test mislabeled 0x2200
as `start_locked` and therefore did not detect the wrong typed-state write.

Verification for this slice: the full game library suite passed 926 tests with
17 ignored; all 14 inventory tests passed with original-asset tests enabled.
The all-targets check passed, and all 45 regenerated native captures matched
the checked-in vectors byte for byte.

This is not initialized gameplay proof. Scene-completion VM writes are connected
as described below, but production startup and English localization remain
unfinished. The production guard remains in place.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_descriptor_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_descriptor.jsonl \
  --resources output/big-bug-bang/imported-assets/resources
nix develop -c cargo test -p commander-blood-game --lib inventory -- --include-ignored
```

#### Scene Completion and VM Resume

The sequel resumes the VM at 0xB68D after the queue reports neither open nor
draining, and at 0xB75A after explicit cancellation of an active scene. Ship,
bridge, camera, HUD and contact-transition dispatch now publish that write to
the owning lifecycle immediately. The panel publishes a consumed-once completion
output before subsequent UI work, so a later chooser pause cannot be overwritten
by replaying an old scene completion. Full presentation shutdown discards that
output. These new writes are gated by the loaded sequel dialect; Commander's
existing VM behavior is unchanged. Inactive sequel cancellation leaves scene and
VM state unchanged.

`big_bug_bang_scene_completion_oracle.py` records 38 cases from unpatched original
instructions. The active-scene continuation at 0xB67C executes the real empty-file
queue-service and source-status helpers, including blocked dispatch and status
values 0 through 3. The complete cancellation routine at 0xB731 executes its
real buffered/empty release helpers. Captures also cover palette reset, ship-depth
thresholds, non-owning flag bits and both initial VM states. Rust compares the
complete typed scene state and the lifecycle VM result against these captures.
This boundary does not decode HNM data, open a file, or exercise line-five display
clearing, and is not proof of a complete inventory interaction in initialized
sequel gameplay.

Verification for this slice: the full game library suite passed 928 tests with
17 ignored, all 14 inventory tests passed with original-asset tests enabled,
and the all-targets build check passed. All 38 regenerated captures matched the
checked-in vectors byte for byte. The one-shot output regression also verifies
that a subsequent UI pause survives another output-consumption call.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_scene_completion_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_scene_completion.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_scene_completion
```

#### A6 Outer-Loop Publication

The sequel's outer COD loop clears its handler yield byte at 0x5ACE before
each instruction. A6 publication returns signal 2 or 3; at 0x5B08..0x5B13,
the loop pauses the VM and sets the handoff lock. Signal 2 clears pending
skips, while signal 3 saves the continuation cursor. Traversal can continue
within the same pass before the ordinary post-scans. The typed dispatcher now
publishes the missing VM and lock writes, preserving the separate C2 gate.
The handoff-lock write is consumed before the actor scan so it cannot replay
over a later scan's release of that lock. Commander behavior is unchanged.

`big_bug_bang_vm_yield_oracle.py` executes the unmodified A6 handlers through
the original handler table and outer loop, from 0x5AA6 to the post-scan boundary
at 0x5B3D. Its 76 cases cover menu, subtitle and inventory text; active, shown,
record-kind, menu/subtitle and empty-inventory gates; both initial handoff-lock
states; and consecutive A6 handlers. The comparisons check full VAR at that
same pre-scan boundary, mutable COD activation flags, traversal end/cursor,
resume state, yield signals, request flags, VM state and both presentation gates.
They also compare the signed presentation selector, spoken/voice/chatter state,
subtitle bytes and reveal cursor, hold and pending-menu state, encoded menu-word
count, and the exact source words reached through the native menu far pointer.
No original handler is patched or replaced. Pre-frame preparation and the
subsequent selection/actor scans are outside this capture, so it does not prove
whole-frame or initialized startup parity. AA/AC's distinct signal-1 return
path is not covered by these A6 comparisons.

Verification for this slice: the full game library passed 929 tests with
17 ignored, all 14 inventory tests passed with original assets enabled, and
the all-targets check passed. The 76 outer-loop and 45 corrected descriptor
captures both regenerated byte-identically.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_vm_yield_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_vm_yield.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_a6_outer_loop
```

### Sequel Presentation Catalog

The production data loader now routes presentation-table decoding through
`GameVariant`. The sequel decoder reads its own 45 pointer pairs at data offset
0x2203, descriptor slots at 0x22B7..0x2745, and the eight effective unclamped-line
IDs at 0x100C. Its data base is file 0xF7F0. Native scene lookup at 0xB50A and
descriptor selector 0xB763 use that index; the first descriptor follows exactly
45 four-byte entries. The last descriptor ends before the ship-state word.
The scene's nine-byte scan excludes the final match when CX reaches zero, so
only the first eight entries are effective.

`big_bug_bang_presentation_catalog_oracle.py` executes the original selector
for every line without modifying its instructions or tables. Both synthetic
malformed-input tests and an original-executable test cover the typed decoder.
The existing runtime catalog keeps the authored flags, variant and dynamic
filename slots. The sequel opens with `sq\\microfol.HNM`, not Commander's
`sq\\mind.HNM`. All seven fixed names exist in the sequel resource store;
the 38 original dynamic names remain unresolved until their native owners write
them. The original-asset inventory test applies all 25 authored descriptions
through this catalog and loads the resulting line-43 resource bytes.

This is catalog and resource-binding verification, not HNM playback or startup
parity. The loader guard remains: the bridge-menu table and runtime/profile
transitions still need sequel-specific integration.
Scene-start policy, profile transitions and English localization are unfinished.

Verification: two native captures were byte-identical across all 45 entries.
All 125 format tests passed with original-asset checks enabled. All 14 inventory
tests passed with ignored checks enabled, including the catalog resource bindings.
The game library passed 929 tests with 17 ignored, serially on a fresh private
Xvfb display, which was reaped afterward. Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_presentation_catalog_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation_catalog.json
nix develop -c cargo test -p commander-blood-formats presentation_catalog -- --include-ignored
```

### Sequel Startup Visual Tables

The data loader routes the default palette, name-area effects and world-artwork
layout through game identity and the sequel executable fingerprint. Commander
keeps its original offsets. The sequel layouts are verified against these
original consumers:

- Screen reset at file 0xADA4 copies 192 dwords from data 0x5F28 to 0x5621.
  This identifies the full 256-entry six-bit palette at file 0x15718.
- Name-area selection at 0x9DD3 and 0x9DFA uses the pointer table at data 0x2A91
  (file 0x12281). The initial sequence plus nine random alternatives contain
  64 frames. Frame reads at 0x9E0D..0x9E2B supply the native origin and dimensions.
- World-artwork initialization at 0x801B walks data 0x2F97 (file 0x12787) in
  22-byte steps to its word terminator at data 0x3333. The 42 rows retain their
  names, resource IDs, entity IDs and initial flags. Selection at 0x80D8 and
  0x80E7 consumes the resource and entity fields at row offsets 16 and 18.

The offline `big_bug_bang_startup_tables_oracle.py` runs these bounded original
instruction ranges with guarded writes and immutable executable bytes. Its
captures cover every palette entry, effect selection/frame and artwork row.
The formats integration test checks both a metadata-derived fixture and the
original executable, plus malformed bounds, colors, pointers and terminators.
These are startup data contracts, not a complete native screen-reset execution
or proof of production sequel startup. The production guard remains enabled;
menu, navigation and profile integration are still pending.

Verification: both native captures were byte-identical. All 125 format unit
tests and three startup-table integration tests passed with original-asset checks
enabled. The sequel runtime test imported all artwork rows and compared every
RGBA pixel of a scaled rendering with the indexed reference, requiring nonblank
output; it also imported the sequel choice/dialogue fonts using this palette.
The game library passed 929 tests with 18 ignored on a private Xvfb display,
which was reaped afterward. The new ignored artwork test passed separately.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_startup_tables_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_startup_tables.json
nix develop -c cargo test -p commander-blood-formats --test sequel_startup_tables -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_startup_tables -- --include-ignored
```

### Sequel Writable Startup Catalog

The data loader now selects the writable-resource catalog by game identity.
The original sequel loop starts at file 0x190C with SI = data 0x2A0, visits each
16-byte filename, and stops when the next entry starts with NUL (0x1944..0x194C).
Its table is file 0xFA90..0x10410: 152 visits, including two `bappel.spr` entries,
with the final three entries belonging to profile 17. Commander retains its
125-entry table and existing decoder contract.

`big_bug_bang_writable_catalog_oracle.py` executes the unmodified traversal and
directory helper with guarded writes. Its DOS boundary reports that every
destination exists, so no copy code is entered. It captures all ordered names,
152 directory-helper entries and the single actual directory change. This is
an oracle for traversal, not a native copy or full startup oracle.

The catalog deliberately retains `blood.sav`, even though the sequel source has
no such file. Preparation must report that missing source locally, continue
with later entries, and never borrow Commander's save. The asset-backed test
uses the actual resource store and preparation coordinator with a filesystem
adapter and recorded graphics calls, not the still-guarded production startup
host. It checks copied bytes, source immutability, duplicate suppression,
preservation of an existing `script1.var`, and an idempotent second pass.

Verification: two native captures were byte-identical. All seven startup-focused
tests passed with original assets enabled. The real-resource case copied 149
files while preserving the preexisting state file, reported only missing
`blood.sav`, and copied nothing on its second pass. The game library passed
931 tests with 19 ignored on a private Xvfb display, which was reaped afterward.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_writable_catalog_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_writable_catalog.json
nix develop -c cargo test -p commander-blood-game --lib sequel_writable_catalog -- --include-ignored
```

### Sequel Navigation And Confirmation Tables

Game identity now selects confirmation hit regions, hyperspace names and
navigation labels/wipe geometry. The original sequel consumers establish:

- Confirmation calls at file 0x16E3 and 0x16F3 select data 0x27A7 and 0x27AF.
  Their rectangles are `[115,105,30,10]` and `[175,105,30,10]`, different from
  Commander's regions. The native pressed-pointer tester at 0x93F7..0x9424
  supplies 64 boundary cases, checked against the runtime's actual hit adapter.
- Hyperspace selection at 0x9D14..0x9D2F indexes eight 16-byte slots at data
  0x2170 (file 0x11960), masks the sequence counter by seven, increments it as
  a wrapping word, and copies the selected name into the presentation slot.
- Navigation label copies at 0x94E3 and 0x9504 consume the French prefixes at
  data 0x012D, 0x0137, 0x0142 and 0x014E. The wipe selector at 0xA028 addresses
  nine coordinate pairs at data 0x29E0 (file 0x121D0); the line routine reads
  both components at 0xAB10..0xAB16.

The guarded native capture records those hit results, four label byte strings,
nine endpoints and eleven travel selections including counter wrap. The
original-asset runtime test feeds the decoded clip names into the real camera
coordinator and presentation catalog, then loads every selected clip's bytes.
This checks selection and resource binding, not HNM playback, the complete
camera/navigation workflow, or the confirmation dialog's rendered labels.

The bridge-menu table cannot use Commander's five-command model.
The sequel list at data 0x27B9 has seven commands: `VITESSE`, `TEXTES`,
`VOYAGE_OFF`, `MUSIQUE_OFF`, `SAUVER`, `CHARGER`, `QUITTER`. Native selection at
0x9A6D..0x9B38 separately opens simulation-speed and text-speed controls and
toggles travel and music. The loader still rejects production sequel startup
until this model and the remaining runtime/profile contracts are integrated.
The French bytes are preserved source data, not completed English localization.

The table decoder now supports all seven sequel labels, both music and travel
toggle faces, the five subtitle speeds, the three simulation speeds, and the
shared cancel label. The game's fingerprint-guarded `decode_bridge_menu_text`
selects the correct data image; `OriginalGameData` no longer calls the
Commander-only menu decoder directly. Owned option labels have variable length,
and the label renderer replaces the music face at row 3 for BBB rather than
overwriting its text-speed row. Commander retains its five labels and row 1.

The subtitle pointer table is at data `0x281D`; the simulation pointer table
is at `0x282B`. The latter exposes its CS-authored `[100, 10, 1]` countdown
reloads alongside the labels. Initialized image values are text speed `2`,
simulation speed `1`, and travel low bit clear. These are decoded image values,
not a claim that all subsequent startup writes have been ported.

All 135 formats tests pass with original-asset tests explicitly enabled,
including direct checks of every menu label against the sequel executable,
malformed pointers/sentinels/terminators, and preserved speed values/flag bits.
Production sequel startup remains guarded until the command dispatcher and
its runtime owners are connected. This table change supplies data and correct label indexing, not an
interactive seven-command menu or an English localization.
The game all-targets check passes, as does the original-disc music-label test.
The full library passes 934 tests with 21 ignored on a private Xvfb display,
which was reaped after testing. The travel label reports the current flag
(`VOYAGE_ON` when set), unlike the music label's offered action.

Verification: repeated navigation captures were byte-identical, and regenerating
the earlier visual-table capture after the shared harness extension also produced
identical output. All 125 format unit tests and six format integration tests
passed with original-asset checks enabled. Both focused runtime tests passed,
including all 64 hit cases and 11 travel/resource selections. The game library
passed 932 tests with 20 ignored on a private Xvfb display, reaped afterward.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_navigation_tables_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_navigation_tables.json
nix develop -c cargo test -p commander-blood-formats --test sequel_navigation_tables -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_navigation_tables -- --include-ignored
```

### Seven-Command Options Handler

`update_sequel_option_menu` now implements the sequel's native command mapping
using the shared list-widget and panel-transition code. Commander Blood keeps
its existing five-command entry point and result type by default. The sequel
returns separate typed speed/travel commands and the five shared actions.

The command effects follow file `0x9A67..0x9B43`: simulation speed and text
speed set their activation and layout phases; travel toggles independently of
audio support; music is unchanged if support is absent; save/load activate
their shared panel and respective motion request; quit clears both pointer
button latches. Cancel closes without an action. A negative chooser result
leaves the menu open. Typed row resolution rejects the original ABI's
unreachable high-byte aliases.

The guarded original-instruction fixture has 80 cases: all seven rows,
cancel, two negative results, and every combination of music support, music
state and travel state. Rust tests compare both submenu phases, activation,
music/travel states, music label, save/load panel and motion flags, quit,
pointer latches, menu/modal ownership, and stream-start requests. Separate
tests check inactive ownership and transition waiting before action dispatch.

Music startup is deliberately captured at the original DOS far-call boundary
at `0x9AF9`; the post-driver branch resumes at `0x9B03` with the captured
globals. No machine code is patched or callback stubs installed. This proves
the request and menu effects, not DOS driver behavior or host audio playback.
The three focused tests pass, including the existing Commander options oracle,
and a repeated sequel capture is byte-identical.
The game all-targets check passes. The full library passes 936 tests with 21
ignored on a private Xvfb display, which was reaped after testing.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_options_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_options.json
nix develop -c cargo test -p commander-blood-game --lib options_
```

This is native handler verification. `RuntimeBridgeConsole` still uses the
Commander handler; it must exchange the sequel controls with the actual
simulation clock, camera/travel consumers, speed submenus and lifecycle before
production sequel startup can be enabled. English localization is also pending.

### Authored Text Corpus Audit

The offline `audit_sequel_text` formats example frames all seventeen COD files
with the existing lossless sequel parser, then reports A6 markers, flags, byte
positions and typed-decoder errors as structured JSON. It does not scan for raw
opcode bytes or claim that every framed instruction executes. BAS is outside
this audit's scope. It consumes the original loose resource import, not English
translations or generated substitute scripts.

```sh
nix develop -c cargo run -p commander-blood-formats --example audit_sequel_text -- \
  output/big-bug-bang/imported-assets/resources > output/big-bug-bang/text-audit.json
```

The original corpus has 6921 framed A6 tokens. The earlier dictionary-only typed
decoder accepted 6824 and rejected 97. Those numbers are a diagnostic snapshot,
**not a fidelity gate**: even accepted tokens can have wrong semantics.

There are 46 inventory markers in profiles 3 through 17. Every occurrence has
flags 32816 (`0x8030`), with marker 65534 at byte 12 relative to the A6 token.
Counts by profile, including the initial two zero-count profiles, are:
`0, 0, 4, 3, 2, 2, 3, 9, 3, 1, 3, 2, 3, 3, 3, 3, 2`.
The existing ordered sixteen-slot `AboardObjectRoster` is the appropriate
ownership model to investigate for this path, rather than a new global list.

The audit also found 58 marker-1 numeric substitutions. The sequel A6 spoken
path at 0x6D58 recognizes word 1, reads a word from the VAR image at the following
operand (0x6D66..0x6D6D), and calls signed decimal formatting at 0x2832..0x286A.
That helper checks the sign, emits a minus when needed and divides by ten.
The earlier dictionary-only decoder instead attempted DIC resolution for both
the marker and its operand. This explains the other 51 errors, but also hides
seven incorrect successes where the VAR operand coincides with a DIC entry:

| Profile | COD Token Byte | VAR Operand |
| --- | --- | --- |
| 4 | 16696 | 1724 |
| 4 | 16785 | 1724 |
| 6 | 11908 | 392 |
| 11 | 15221 | 2538 |
| 14 | 2683 | 466 |
| 14 | 7627 | 540 |
| 17 | 3629 | 1798 |

All numeric occurrences have flags 32768 except one with 32776; none explicitly
sets spoken flag 32. That is not proof that an inherited spoken-mode latch is
unreachable. The numeric menu comparison is described below.
The spoken path notably leaves SI at the VAR operand before dictionary-based
lookahead (0x6DA9), so it must not be rewritten as a guessed generic interpolation
loop. Runtime read ownership, spacing and cursor behavior must be verified.
Several numeric operands exceed their profile's DIC extent, while others land
inside unrelated strings. Do not pad the dictionary to make these reads appear
valid, and do not count current typed acceptance as recovery of numeric text.

The typed decoder now consumes marker 1 and its VAR operand together, only in
the sequel dialect. All 58 authored operands resolve to owned state words,
including the seven formerly incorrect successes above. The corpus test now
accepts all 6921 A6 tokens, including all 46 inventory-marker tokens. All 123
formats tests pass with original corpus checks enabled.

### Numeric Menu Renderer

`re/tools/big_bug_bang_menu_number_oracle.py` executes the original menu renderer
at 0x82C6 through 0x83E1, signed formatter, main-font drawing helper and width
helper without replacing callees. The checked-in 59 vectors compare text,
positions, widths, cursor, reveal counter, countdown, completion and DIC scratch
content against the Rust renderer. They cover signed extrema, zero, adjacent
numbers, punctuation, wrapping, partial reveal and retained scratch text.
This is a control/layout oracle, not a VGA-pixel oracle: planar hardware is not
emulated by this harness.

The renderer resolves each numeric operand from live VAR state and formats it as
signed decimal. Native reveal limits count encoded words, so marker plus operand
consume two positions. Lookahead to a number uses the previous scratch string
before formatting the next number; the Rust presentation owns that string and
the existing profile-change reset clears it. Normal menu publication retains it.

Spoken numeric text, numeric condition sections and numeric chatter hashing
return explicit errors until their distinct native paths are recovered. The
production sequel guard remains in place. These component checks do not prove
complete sequel dialogue, inventory selection, or playable startup.

Shared-engine verification: all-targets game check passes; the game-library
suite passes 913 tests with 12 ignored under a private X server. The ignored
numeric-menu test is run separately and passes all 59 native vectors.

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
GS:0x0CC4 at 0x5B46 after script/presentation processing. The clock ownership and
dispatch forwarding described below are now implemented; the interactive speed
control and remaining sequel runtime still need connecting. The handler must
not run independently at the renderer's presentation rate.

`SequelSimulationClock` now owns the two initialized words and exactly models
the original saturating decrement and zero-only reload. Forty unmodified-code
captures cover zero, boundary values and maximum words; a repeated capture is
byte-identical. `RuntimeGameLifecycleHost` advances the owned clock at the
start of each main-loop iteration, before input and pause gates, not from the
interrupt-timer tick count. `RuntimeScriptSystem` reloads after a successful
enabled pass, including a resume-boundary finish. Disabled and error returns
do not reload. Changing speed changes the next reload only.

The production script backend retains this clock across profile binding and
binds `Trashlando`, `arche`, `Arche`, and `Honk` from the current directory.
`ScriptExecutionService` now forwards simulation and settlement contexts to
D4-D6 instead of inheriting the absent-context default. Commander has no sequel
clock; missing sequel bindings still return absent context and dispatch errors.
An original-directory test checks all seventeen binding sets, clock changes,
disabled/enabled pass boundaries, and clearing stale identities. This verifies
the production backend/service boundary, not seventeen playable profiles.

The full-profile acceptance test was also attempted and **still fails** on the
transition from SCRIPT1 to SCRIPT2: `InvalidStateWord` at COD byte `0x5A97`,
VAR byte `8368`. The retained first VAR does not own SCRIPT2's `time` word;
the native allocation observations and remaining work below still apply.
The failing acceptance test is retained with an explicit known-incomplete
ignore reason. No zero extension, replacement VAR defaults, or fake state
binding was added. Production startup remains guarded.
The default game-library suite passes 937 tests with 23 ignored on a private
Xvfb display, reaped afterward; that ignored count includes the known failing
full-profile acceptance test. Game all-targets checking passes.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_simulation_clock_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_simulation_clock.json
nix develop -c cargo test -p commander-blood-game --lib sequel_clock_context_forwarding -- --include-ignored
# Acceptance gate, currently expected to expose the unresolved SCRIPT2 boundary:
nix develop -c cargo test -p commander-blood-game --lib sequel_simulation_clock_reaches_production_dispatch -- --include-ignored
```

The simulation-speed selection tail is now recovered and component-tested.
The menu at file `0x1D18` uses DS:`0x282B` for `LENT`, `NORMAL`, `RAPIDE`
and a final `0xFFFF` cancel pointer. Its CS-relative table at file `0x1D12`
contains `[100, 10, 1]`, written to DS:`0x0CC4` by the selected row. The
executable's initial value is `1`. These are countdown reload values, not
milliseconds or renderer frame rates. Cancel retains the old value and closes
the modal; a negative selection retains the value and leaves the modal open.

`PresentationChoiceItem::Value` allows the shared choice coordinator to publish
these explicit values while preserving its existing text-speed row mapping.
The guarded `big_bug_bang_speed_choice_oracle.py` runs original instructions
at `0x1D6F..0x1D90` with the original CS base and no patched callbacks. Its 90
cases cover every row, cancel, two negative selections, five previous values,
and three UI flag combinations. Tests compare result, activation, UI flags,
and the typed outcome. This captures only the selection tail, not the opening
transition or the simulation loop. **The seven-command menu and simulation
clock are still not connected to a production sequel host.**

```sh
nix develop -c python3 -P re/tools/big_bug_bang_speed_choice_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_speed_choice.json
nix develop -c cargo test -p commander-blood-game --lib presentation_choice
```

The five focused tests pass, including the original Commander Blood choice
vectors. A second speed capture and the existing startup-table capture are
byte-identical to their checked-in fixtures after the harness's CS-aware exit
check was added. These are component checks, not evidence of BBB playability.

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

### D4 Conflict

The D4 handler (file 0x70CD-0x724D) now has a typed Rust implementation and
production instruction dispatch. All 33 authored occurrences are in
SCRIPT2.COD. Its group mask and attack rate are unsigned immediate words.
The rate is retained even when the countdown suppresses execution or the
instruction runs as a query.

The recovered behavior is:

1. Select participating actors with the shared 0x706E helper. In query mode,
   any engaged actor succeeds; no engaged actor consumes the enclosing failed
   guard and clears query mode. Queries do not apply damage or acquire targets.
2. Unengaged actors require signed quantity at least 100, aggressiveness at
   least 200 and relief below 800. Search maximum-range locations in directory
   order, not nearest-first. A qualifying location's word at byte 24 identifies
   the opposing actor. Require opposing groups, in-play/participating flags and
   signed target quantity greater than 50. Link the opponents and mark engaged;
   an existing target back-reference is not overwritten.
3. Clamp relief and growth balance only from above. Compute aggression and
   damage with the original word wrapping, unsigned division and signed clamps.
   A word-DIV overflow reports an error with all earlier writes preserved,
   including completed updates to earlier actors.
4. Disengage when source quantity is at most ten, or damage lowers the target
   below ten. Clear the source's engagement and opponent link. If the target
   points back at the source, search for a replacement. With none, clear the
   target's engagement and relocate matching active descendants to the nearest
   free location within the current search range. Unlike settlement, retreat
   does not initialize quantities, balances or destination occupancy flags.

The replacement helper at 0x724E tests flag value 2, not Actor kind. Its byte-72
read on a non-actor record can alias a word in a following record. A native
oracle case establishes this behavior. Rust resolves that serialized position
to a checked owned state word; it neither invents an actor-only filter nor
emulates segmented memory. The extra actor word at byte 72 is now identified
as the opponent link by its reads and writes in this handler.

`re/tools/big_bug_bang_conflict_oracle.py` reuses the settlement runner and
executes the original handler and nine helper entries without replacing calls.
Its 124 synthetic cases include acquisition thresholds, existing back-references,
replacement candidates, non-actor aliasing, retreat range, both sides updating,
countdown gates, 18 failed guards and 13 divide errors. Rust compares the full
VAR buffer, attack rate, range override, query state and guard result. The
combined production-dispatch regression covers all 224 settlement/conflict
cases and rejects absent host bindings without silently substituting defaults.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_conflict_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_conflict.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_conflict -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

These are component-level native comparisons, not initialized-game captures.
They do not establish full gameplay parity or production sequel startup. In
particular, the original scratch candidate-list bounds and repeated updates
still need verification with authored state after script initialization.

Verification for the conflict slice (2026-09-05): all 114 formats tests passed
with original-corpus tests enabled; all nine sequel game tests passed with
original-table tests enabled. The full game library passed 889 tests with seven
ignored using `--test-threads=1`. Its first, concurrent run terminated with
SIGSEGV; the cause has not been established or fixed. Do not report that run as
passing. Game all-targets checking and workspace library/binary checking passed.
Both settlement and conflict fixtures regenerate byte-for-byte from the
original executable; the settlement fixture is unchanged. Tests used the
current worktree, including unrelated Commander runtime edits that remain
outside this commit.

### D7 Ending and Changed CC Selection

The sequel's D7 at file 0x6E67 sets the ending latch at GS:0x6B73, in both
normal and query mode. This is **not** the inherited A8 `fin.*` latch at
GS:0x6B93. D7 itself does not request media or immediate shutdown. All four
authored D7 instructions are in SCRIPT2.COD.

CC at 0x69E6 retains the bounded sequence-name copy but adds a zero-based
selection request at 0x69ED. Commander CC only copies the name. All 97 sequel
CC instructions decode into the existing six-slot domain. Both new controls
survive the profile reset at 0x588F-0x5903, whose cleared ranges exclude them.
Empty CC names now disable their slot in the shared owned representation,
matching both the native first-byte test and the existing save restore path.

The Rust dispatcher now handles D7 and publishes CC's additional request only
for the sequel dialect. A separate `SequelPresentationControl` carries this
state into the existing production panel and ready-actor handlers:

- At queued-scene entry (0x8C14), a pending CC choice takes precedence over D7
  and mouse input. Without a pending choice, D7 suppresses ordinary primary
  input. Empty record slots have a separate native input path and are not
  unconditionally locked by this flag.
- A pending choice is consumed at panel initialization or queued-scene entry,
  not during inactive/opening/closing phases. Initialization selects that
  channel directly. Queued-scene entry selects it before the ordinary input
  path, which increments it unless reverse-closing. This native ordering is
  preserved rather than treating both paths as a direct channel change.
- When the full scene list completes (0x8C75), reverse/startup mode returns to
  transition. Otherwise an ordinary list closes; D7 instead publishes the
  next-frame shutdown request. Finishing an intermediate scene is not enough.
- The ready panel actor (0x92E4) selects its hand animation then stops for D7
  when no CC request exists. An explicit CC selection bypasses that gate and
  the animation selection. Disabled/not-ready actor behavior remains intact.

CC consumption is published before media callbacks, avoiding a stale end-of-
frame snapshot overwriting a new request. Commander receives no sequel control
context and keeps its established panel behavior.

`re/tools/big_bug_bang_presentation_oracle.py` executes original CC and D7
instructions and the three decision blocks above. Its 66 synthetic cases
compare instruction effects, retained slot bytes, query preservation and
branch destinations. The decision probes deliberately stop before external
media calls; they are **not** full original panel/actor execution or playback
parity tests. The runner checks all global writes and rejects writes outside
that region; it uses the VM's shared SS/data layout for BP-relative globals.
Rust tests additionally cover typed production dispatch, profile-reset
retention, panel request consumption, multi-line completion and actor effects.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_presentation_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation.jsonl
nix develop -c cargo test -p commander-blood-formats --lib every_authored_sequel_panel_control_decodes -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

This is not a complete sequel presentation port. Initialized native captures
must verify authored ending lists, callback ordering and all media effects.
Sequel startup is still blocked by the unresolved loading work below, not
enabled by these component changes. Automatic activation is covered separately
below.

Verification for this slice (2026-09-05): all 115 formats tests passed with
original corpus tests enabled; all 13 sequel tests passed with original table
tests enabled. The full game library passed 893 tests with seven ignored,
serially under a private Xvfb display (`SDL_VIDEODRIVER=x11`, Wayland unset),
including SDL input and GPU tests without using the real desktop. Game
all-targets checking and workspace library/binary checking passed. The 66-case
native presentation fixture regenerates byte-for-byte. Tests used the current
worktree; unrelated Commander runtime edits remain outside this commit. The
earlier concurrent-suite SIGSEGV has not been diagnosed by a serial pass.

### Automatic Scripted Panel Activation

The complete helper at file 0x8A48-0x8A7C runs after presentation-mode updates
and before sprite geometry, hover and actor processing. It does not synthesize
pointer input or consume CC's pending request:

- Without a pending choice, or while the panel is already active, it does
  nothing.
- With the camera view active, it sets only the camera actor's auto-seek bit
  and clears the sequel simulation-overview flag. It does not arm the panel
  during this frame.
- Otherwise, unless the panel actor is already armed, it sets that actor's
  active and auto-seek bits and the shared redraw/modal bit.

The Rust bridge frame now calls this helper in that same position, guarded by
the active profile's sequel dialect. The camera handler also implements the
0x91F5 gate: a pending CC skips hand selection and primary-input clearing, but
still advances the camera line. After the camera closes, a later frame can arm
the panel; the request remains pending until the panel consumes it.

The new `re/tools/big_bug_bang_panel_activation_oracle.py` executes the entire
activation helper (no helper substitutes) for 384 combinations and the camera
gate for 12 combinations, stopping before its common line-playback call. It
checks retained global bytes as well as declared outputs; the two-byte stack
scratch used by the complete helper is accounted for explicitly. The 396-case
fixture SHA-256 is
`a30ba1c16df575adcec7d3b9fcc6992f56fb124a79bd86f67c73f7972de27cb6`.
Rust tests compare all these cases and exercise the camera-to-panel handoff
across updates with controlled line-completion feedback. They do not constitute
a full native media-playback or production sequel-startup test.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_panel_activation_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_panel_activation.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

The overview state at native 0x2A30 is **not** the existing planet-choice
panel. Its separate controller at 0xA286 and the camera-open secondary-input
write at 0x9252 were ported on 2026-09-12. `SequelOverviewState` owns the
active/show-all/category/status masks, and its typed controller retains the
native gates, secondary-edge open/close behavior, camera-slot writes, actor and
holder filters, category hit boxes, signed quantity comparison, status-color
priority, and draw order. The runtime decodes actor fields and relations from
the live sequel VAR, resolves marker positions through the shared navigation
resolver, and runs the overlay after interactive chart entities are published
but before chart-object picking. A consumed panel click therefore cannot also
open a location panel. Camera activation publishes the native synthetic
secondary edge, allowing the overview to open in that same bridge frame.

The flat renderer uses the already recovered fill, span, rectangle, and
square-caps primitives plus a typed translation of BLOOD2PG's line routine at
0x3B03. Diagonal lines retain endpoint ordering, both Bresenham branches, and
the excluded terminal endpoint; checked per-pixel clipping replaces the native
fixed-point edge clipper and segmented framebuffer writes. The native panel can
extend beyond x=319 when many categories exist. This overlay alone uses an
explicit clipped square-caps adapter so off-display glyph pixels are discarded
instead of reproducing VGA-aperture row wrapping. An original-resource test
loads all 17 profiles, decodes every actor, renders each authentic initial
state, then forces every authentic category eligible to exercise the complete
16-label layout and this clipping boundary.

`re/tools/big_bug_bang_overview_oracle.py` executes the complete original
0xA286 controller from SHA-256
`4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834`.
The roster builders at 0x6FF2 and 0x706E and position resolver at 0x67B8 run
unchanged; only established graphics-call boundaries are captured. Its 32
cases cover both early gates, inactive/open/close/empty paths, label boundary
hits and primary consumption, prior-hover versus selected ordering, actor and
holder filters, conflict/opponent and signed instability colors, and panel
layouts from one through sixteen categories. The fixture SHA-256 is
`63359073d942ae9de9d12ee279fa9b5257c0f8af921dc9d17badef2b6e12652d`.
This proves controller state and draw calls, not VGA pixel parity inside the
graphics operations.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_overview_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_overview.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_overview -- --nocapture
nix develop -c cargo test -p commander-blood-game --lib \
  every_authentic_profile_builds_and_renders_an_overview -- --ignored --nocapture
```

Verification for this overview slice (2026-09-12): all 32 original controller
cases and the new navigation-order and line-raster regressions pass. The
original-resource overview test passes across all 17 profiles and all 16 group
bits; both sequel font asset tests also pass. The complete game library passed
1,001 tests with 61 explicitly ignored, serially, and game all-targets plus
workspace library/binary checks passed. The workspace check retained unrelated
pre-existing warnings in the extraction binaries. The overview fixture
regenerated byte-for-byte and its generator passed Ruff.

Verification for this activation slice (2026-09-05): all 15 sequel tests passed
with original-table checks enabled; all 115 formats tests passed with original
corpus checks enabled. The full game library passed 895 tests with seven
ignored, serially on a private Xvfb display. Game all-targets checking and
workspace library/binary checking passed. Both the earlier 66-case fixture and
the new 396-case fixture regenerated byte-for-byte from the original binary.
These checks used the current worktree; unrelated Commander runtime edits are
not included in this slice. The previously documented concurrent-suite crash
and workspace all-targets test-import failures remain unresolved.

### Sequel Records and Profile Ownership

The formats crate now decodes sequel VAR records with an explicit dialect:
actors own 74 bytes and locations 26, versus Commander's 72 and 24. All 17
original VAR, DEB and DIC images round-trip exactly, with 184 objects per
profile. Their entire active-object directory prefix is identical, not just
the first few entries. Original field-table comparisons cover all 22 selector
rows and nine shipped object kinds. The inherited 21 rows match Commander;
the additional row selects the actor word at byte 72. The D4 recovery above
identifies that word as the opponent link; the generic format API retains its
field-index representation.

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
selection and dialect-aware runtime profile requests remain unwired. Missing
BAS ownership is now implemented below; entered unsupported dialogue still
produces a format error rather than silently skipping instructions.

Verification for the record/profile slice (2026-09-05): 108 formats tests passed
with ignored corpus tests enabled, all four sequel-specific game tests passed
with original-table tests enabled, and the full game library passed 884 tests
with seven ignored (including the two separately run sequel table tests).
`cargo check -p commander-blood-game --all-targets` passed. These commands ran
in the current working tree; unrelated in-progress runtime changes were not
included in this commit.

### Resource Binding and Initial Profile Loading

The complete resolver at file 0x5798 returns failure without changing DS:SI when
its handle is not resident. The VM binding loop at 0x5A64-0x5A97 resolves VAR,
DEB, COD, BAS and DIC in that order and publishes DS:SI without checking the
failure result. Consequently a missing BAS is bound to the preceding COD
resource, not to an empty BAS program or a universal SCRIPT2.BAS fallback.

`re/tools/big_bug_bang_profile_binding_oracle.py` executes that loop and the real
far-called resolver for all 32 resident-handle combinations. It also executes
the complete native allocator for eight three-allocation sequences, without
replacing helper calls. The latter establish exact paragraph rounding, including
an 8368-byte VAR allocation with no spare trailing word. These are synthetic
resident-table/allocation probes, **not a DOS filesystem or full startup oracle**.
Memory-write checks restrict each run to its outputs and call stack. The 40-case
fixture SHA-256 is
`a3f88f97e7b8e5280e6188081cd2a7be262add1f23dcc748b8fcc7ab0fb4bf57`.

Rust now loads sequel companions in native order, reports absent/empty BAS as
`Unavailable`, and binds dialogue ownership to the actual resident source ID.
`ScriptProfileDialogue` retains exact bytes and dictionary ownership; decoding
is cached on first dialogue use. Commander still requires valid BAS at profile
load. Missing essential sequel companions and I/O failures remain errors.
No synthetic resource is inserted into the cache or written to the game tree.

The actual initial sequel profile now loads through `ScriptProfileManager`,
with all COD instructions and records bound and VAR bytes unchanged. Its
dialogue image is exactly SCRIPT1.COD. Attempting to interpret that image as
the currently supported BAS grammar errors explicitly. Selector, menu, block
and object-text consumers validate only when their native path enters dialogue;
an idle frame does not force a BAS parse. This does not prove unused BAS data
is permanently unreachable or that the whole sequel can start in the UI.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_profile_binding_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_profile_binding.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

Verification for this binding/loading slice (2026-09-05): all 20 sequel tests
passed with original-disc checks enabled, including actual initial-profile
loading. The full game library passed 899 tests with eight ignored, serially
under private Xvfb; all 115 formats tests passed with corpus checks enabled.
Game all-targets checking and workspace library/binary checking passed. The
authored-media census and seven Commander campaign tests passed with required
accuracy data enabled; the separate process-driven morning-oil test was not
run. The 40-case native fixture regenerated byte-for-byte. Tests used the current
worktree, including unrelated Commander runtime edits excluded from this commit.
Prior concurrent-suite and workspace-wide test-import issues remain unresolved.

### Live Original Startup Allocation Capture

`re/tools/capture_big_bug_bang_startup.py` now runs the hash-checked original
sequel as its own DOSBox-X child and periodically reads its guest memory. Each
run allocates a new private Xvfb display with `-displayfd`, unsets Wayland,
disables joystick input, and uses dummy audio. It never writes guest memory or
moves a pointer. An optional, recorded primary button press is restricted to
that private display. The original disc directory is mounted as a CD-ROM; only
a fresh output-specific C drive is writable. Children are detached and reaped
on completion or error. Captures refuse to overwrite an existing directory.

The module locator checks independent global-data, VM-code and resource-name
anchors. Native loading uppercases filenames in the catalog in place, so that
anchor comparison permits ASCII case changes, not arbitrary different names.
The helper resolves symbols from the running child ELF, not the Nix wrapper,
and rejects unverified emulator layouts. It reports pre-allocation startup,
partial bindings and ambiguous module matches separately from a bound profile.
Register flags are stored/lazy emulator values, not materialized CPU EFLAGS.

```sh
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-reference --seconds 60
# Optional isolated input, without pointer motion:
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-click-reference \
  --seconds 85 --click-after 35
python3 -P -m unittest discover -s re/tools \
  -p test_capture_big_bug_bang_startup.py
```

The first successful no-input run (`output/big-bug-bang/startup-capture-03`)
observed the initial profile bound from approximately 28 seconds through the
60-second endpoint. A second run (`startup-capture-04`) sent one private click
at 35.321 seconds and sampled through 85 seconds. Its final screenshot shows
the bridge TV area with the hand visible and the video off. It did **not**
reach another profile. All 111 bound-profile samples in that run agreed on:

| Role | Handle | Owning Resource | Allocation Bytes |
| --- | ---: | --- | ---: |
| VAR | 2 | SCRIPT1.VAR | 8368 |
| DEB | 3 | SCRIPT1.DEB | 8912 |
| COD | 4 | SCRIPT1.COD | 4160 |
| BAS | 5 | SCRIPT1.COD, same pointer as COD | No BAS allocation |
| DIC | 6 | SCRIPT1.DIC | 2480 |

The word at VAR byte 8368 read 24930 and belonged to **offset zero of the
resident SCRIPT1.DEB allocation**. It was not VAR padding or a hidden initialized
timer field. The captured VAR allocation was byte-identical to SCRIPT1.VAR,
SHA-256 `b2e07ec2f1bdd3acbe7798fbc2eecc2ea596e7bf58d5c82aeecc0c58a28776ea`.
This validates real startup allocation/binding behavior against the earlier
synthetic native probes. It does not establish what `inter3` later reads after
profile changes, compaction, resource reloads or save restoration.

Local evidence hashes (original memory/media remain ignored, not committed):

- `startup-capture-03/capture.json`:
  `67755f6eda15d4214f12c48599cf73d10124b6239c9a0e3cba0c09fdbd7b1c4f`.
- `startup-capture-04/capture.json`:
  `7713353d5c16ba5f342d20d07afc7f6027971dccbbf91f270b5e9b719820030f`.
- `startup-capture-04/state-0059.bin`:
  `1e5bc8a05bcbc49f0a24250a74b768373effde7b5d4fa4537c04f29a427fac5c`.

DOSBox-X identified itself as 2026.05.02, commit 5817c64; its ELF SHA-256 is
`05f568c04cbedb12f82ea5b89b0912724011471d5c7619555f4e97e100fc7157`.
Runs used the normal core at 30000 cycles and
`AMR S162227 EMS WRIC:\cblood\`. These are recorded capture settings, **not a
recovered sequel installer command line**. Periodic ptrace stops also mean
these are not real-time timing or frame-rate reference measurements.

The 13 synthetic helper tests cover allocation ownership, retained VAR handles,
missing BAS aliasing, unowned adjacent bytes, incomplete bindings, independent
anchors, case conversion, ambiguous modules, changed VAR snapshots, truncated
reads, invalid durations, and private input routing. No production Rust files
change in this evidence slice; the full sequel port and localization remain
unfinished.

The final tool revision was rerun without input for 40 seconds in
`startup-capture-05`: 80 samples, including 25 bound-profile observations,
reproduced the same owners and adjacent word value. The run exited successfully
and its DOSBox/Xvfb children were reaped. Rust suites were not rerun for this
offline-tool-only change.

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
per-resource zero-result rejection. Only SCRIPT2.BAS is on the disc. The
missing-resource binding is established above. The shipped SCRIPT2.BAS first
menu fails current dictionary binding at BAS byte 6, dictionary offset 0x1F00;
it also fails typed decoding against every other shipped profile dictionary.

The call to the old-style conversation scanner at 0x5E66 is gated by actor
field selector 2 (byte 26), presentation context and object flags. A typed
authored-script audit found all 1,037 actor instances' initial byte-26 words zero
across the 17 profiles and no COD destination overlapping those fields. Do not
confuse selector 2 / byte 26 with dialogue-control selector 15 / byte 68.

`big_bug_bang_bas_ownership_audit.py` pins the executable hash and scans every
near call to the native field resolver at 0x6633. All 54 calls have classified
selector inputs. Only 0x5E59 and 0x8420 request selector 2, and both immediately
read it: the former gates the sole call to the BAS dispatcher at 0x5E66, while
the latter gates BAS text activation. The only three loads of GS:0x6AF8 are in
those dispatcher/text-scan chains. An over-approximating Capstone pass decodes
an instruction at every byte in the executable code image and inventories all
possible displacement-26 writes. Ten are starts inside other instructions; the
only real instruction, represented with and without its GS prefix, writes an
unrelated 32-byte sound-slot record at GS:0x65E2 from 0x481F. It does not address
the script-state allocation. A second all-byte pass inventories every
address-sized register assignment from literal 26, plus LEA displacement 26.
Six real instructions advance DOS DTA pointers to the file-size field after
INT 21h/2Fh, and one adjusts a DOS file seek past a 26-byte header. Six apparent
starts are the ModR/M byte of the preceding MOV SI,BX interpreted as a REP
prefix. No candidate forms a script-state field address.

The audit regenerates its checked-in report with:

```sh
nix develop -c python re/tools/big_bug_bang_bas_ownership_audit.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_bas_ownership.json
```

Initial resources and authored COD therefore provide no producer for a nonzero
selector-2 value, and the native audit finds no semantic mutation that can
introduce one. Bulk profile retention and ordinary save/load copy existing
state rather than synthesizing this field, so a legitimate new game and every
save derived from it remain zero by induction. The BAS dispatcher and text
scanner are unreachable for shipped gameplay. The resource stays preserved,
and a tampered or foreign save that injects a nonzero field remains outside this
conclusion; no empty BAS, dictionary substitution, or speculative interpreter
behavior was introduced.

SCRIPT2.VAR has an extra trailing word named `time` at byte 8368. The initial
VAR image is only 8368 bytes, yet the native loader retains it when selecting
noninitial profiles. The native allocator does not reserve that extra word.
The authored audit found one read, `time > 19` at SCRIPT2.COD byte 0x5A97 in
`inter3`, and no COD initialization. In the initial allocation order, bytes
after VAR belong to the next resource, DEB. Later compaction/residency can
change that neighbor; a synthetic initial allocation alone cannot prove the
value observed during the conversation. The live startup captures above confirm
that initial adjacency but do not reach `inter3`. Trace native writers and the
later profile transitions before deciding its flat-state representation.
Do not copy the second profile's defaults or silently zero-extend initial state.

### Recorded Reference Input

The startup capture accepts repeated, strictly increasing `--click-after` times
and an optional `--click-position X Y` on its private 800x600 X display. It saves
a screenshot before each click (`before-click.png`, `before-click-2.png`, etc.)
and records scheduled and actual request times. A requested pointer move is not
reported as verified movement. Repeated targets omit `xdotool mousemove --sync`:
the original `startup-capture-08` failed because that option waited for movement
when the pointer was already at the target.

`--relative-mouse` enables DOSBox-X autolock and, if needed, sends a separately
recorded acquisition click before the first intended click. It verifies the
child emulator's one-byte `mouselocked` symbol before proceeding. It does not
modify guest memory. The official
[DOSBox-X mouse guide](https://dosbox-x.com/wiki/Guide%3AMouse-support-in-DOSBox%E2%80%90X)
describes autolock; this build did not provide capture status in its window
title, and the tested Ctrl+F10 attempt did not set `mouselocked`. A failed capture
can have partially delivered input; `status: requested` is not evidence that
no input occurred.

```sh
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-relative-reference \
  --seconds 90 --click-after 35 --click-after 60 \
  --click-position 160 300 --relative-mouse
```

The above replay was run as `startup-capture-13`: mouse capture was verified,
both intended clicks were sent, and the 90-second screenshot still showed the
original introduction. Only profile 0 was observed, with the same extra-word
address owned by `script1.deb` and value 24930. Its `capture.json` SHA-256 is
`821c1be535a39f598571a242334f631e8c7440998574c876bcb8b25767f16b80`.
The uncaptured repeated-click replay `startup-capture-09` also completed, without
reaching SCRIPT2. These are input/capture checks, not gameplay acceptance.

A no-input extension, `startup-capture-14 --seconds 300 --interval 1`, completed
300 samples through 300.311 seconds. The final screenshot showed a later space
scene and narration, so the introduction was visibly advancing; it still had
only profile 0 and the same `script1.deb`-owned extra word. Its `capture.json`
SHA-256 is `56dd42db4fe1ad8f2a8f1366e52288665d953571adf4ad3f67c6de6677d8f0a9`.
This does not show the Honk `JOUER` choice or a native SCRIPT2 time-word read.

Samples now include `mouse_poll`, the shared words at global offsets
`0x0c22`, `0x0c24`, and `0x0c26`. The original module routine at `0x0709`
stores INT 33h CX, DX, and BX there. Later game code adjusts the same slots,
so periodic values are not a raw driver event trace or proof of a hit-tested
selection. The capture's 22 unit tests cover schedule validation, private-display
targeting, pointer bounds, capture-state validation, and the existing allocation
checks. Original normal-input navigation into SCRIPT2 remains unverified.

### Authored Translation Source

`sequel_text_catalog` now exports every typed A6 instruction from the 17 authored
COD resources, using the existing sequel decoder rather than scanning for text
bytes. The catalog has 6,921 message sites and retains each site's profile/COD
identity, line-record reference, presentation selector, control flags, resume
target, and record-condition operand. Explicit sections preserve condition/menu
boundaries. Their typed parts retain dictionary byte identities and original
word bytes, plus 58 state-number and 46 inventory substitutions. The joined
section `source` is a reading aid, not replacement executable text.

```sh
nix develop -c cargo run --bin sequel_text_catalog -- \
  output/big-bug-bang/imported-assets/resources \
  > output/big-bug-bang/authored-cod-text.json
nix develop -c cargo test --bin sequel_text_catalog -- --include-ignored
```

The export records SHA-256 hashes of each COD/DIC pair. Independent validation
matched its per-profile counts to the existing text audit and all 44,599 word
references to their original dictionary bytes. The synthetic structure test and
the opt-in original-corpus test both pass. Repeated exports are deterministic.
`cargo check --workspace` and `cargo check -p commander-blood-tools
-p commander-blood-game --all-targets` pass. Forcing `--workspace --all-targets`
also builds the script compiler's embedded shared-source tests, which fail with
17 unresolved-import/type errors. The same failure was reproduced in a clean
detached worktree at pre-export commit `1d3b90a6`; the compiler already declares
its unit tests owned by the tools crate. This change does not alter that setup.
There are 2,614 sites with the spoken-text flag and 2,111 distinct first-section
reading strings among those sites; these counts do not establish reachability
or complete contextual utterances across multiple instructions.

Five word occurrences in three messages contain byte `0xef`, outside the
repository CP437 decoder's supported range. They remain explicit `\\xef` escapes
with original bytes attached, not replacement characters or guessed corrections:
`bbb.script6.cod.000023bc`, `bbb.script6.cod.00003aa0`, and
`bbb.script12.cod.00001b9c`. An English editor must resolve these in context.

This is authored COD translation source, **not completed English localization**.
The separate BAS source has no recoverable companion dictionary, so its numeric
dialogue and choice operands cannot be translated from shipped evidence. Native
UI/object display names and media-embedded text are also explicitly excluded.
Runtime localization, translated choice labels that preserve concept IDs,
font/wrapping checks, and timing remain work.
Generated original-text catalogs stay under ignored `output/`, not in the repo.

The longer no-input reference, `startup-capture-15 --seconds 600 --interval 1`,
completed 600 samples through 600.625 seconds, still only in profile 0. Its final
screenshot returned to the TV narration about Terra also visible near startup;
waiting alone did not reach the Honk start conversation. The extra word remained
24930 in `script1.deb`. Capture JSON SHA-256:
`63aabc44a9c7977145ca7e38d27f0b28b7ba4eb1935d26b2163a875d575f696b`.
The next reference step is deliberate bridge navigation, not another longer
no-input run. SCRIPT2 acceptance remains unresolved.

## Production Loader and Native Save Defaults

The production path now selects the executable and title from game identity,
checks the analyzed sequel executable hash before media normalization, and uses
the sequel's writable namespace by default. The SDL window title also identifies
the selected game. Production source verification is game-specific: Commander
checks its DESCRIPT and five script profiles, while BBB checks its own DESCRIPT
and 17 COD/DEB/DIC/VAR profiles without compiling Commander sources or
inventing a BAS resource. Unchanged scripts continue to load the original disc
bytes.

With no writable `BLOOD.SAV`, BBB retains the ten native slot records embedded
at executable file offset `0x1206B` (GS:`0x287B`). This is not an invented empty
file: the optional native loader returns without changing that table on open
failure, and seven live startup snapshots match all 320 bytes. The modern
fallback stays in memory until an actual save-directory write is requested.
An existing user directory takes precedence; malformed user data remains an
error and is not overwritten. Complete sequel save/load behavior is unverified.

The real-asset bootstrap test loads the initial profile, MANU3, bridge panorama,
and startup resources, then checks native default slots, explicit persistence,
existing-data precedence, malformed-data preservation, and unchanged source COD.
The only startup-copy diagnostic is the absent `BLOOD.SAV` resource. Both the
bootstrap test and the native snapshot comparison pass when explicitly enabled.
The game library passes 940 tests with 28 ignored using `--test-threads=1`;
`cargo check --workspace` passes. The previously recorded parallel Vulkan crash
is not resolved by these serial results.

The captured secondary-click experiment (`startup-capture-17`, 65 seconds)
verified private mouse capture but still ended in profile 0 on the TV sequence.
It does not establish an exit from the opening or acceptance of SCRIPT2.

The first windowed run exposed a Commander-only builtin name: navigation-chart
setup required `Ark`, but BBB calls that object `Arche` (distinct from lowercase
`arche`). Native file `0x5995` resolves the capitalized name and stores its value
at GS:`0x6B28`. Binding is now dialect-specific. Five captured post-binding
startup snapshots contain the expected value 4624; the two pre-binding snapshots
still contain zero and are not used as completed-binding evidence.

All 711 HNM resources have generated verified lossless WebM derivatives. The
modern `modern-startup-04` smoke run exited successfully after 292 opening
presentation frames and five main-loop frames in profile 0. Screenshots from
the preceding run show changing opening-movie artwork. This is production
startup evidence, not sustained gameplay, full audiovisual parity, save/load,
or an English playthrough. The native builtin snapshot test is explicitly enabled
alongside the other original-asset tests.

The longer `modern-startup-05` run also exits successfully: 292 opening frames
plus 600 main-loop frames, still in profile 0. `screen-013.png` shows the active
character presentation and its `Hello!!!` caption. Its trace subtitle field only
contains empty text and `WAIT COMMANDER ...`, so this run does not establish
the English dialogue override in live play. Direct original/modern startup
sequence comparison and deliberate navigation remain necessary.

## Static Field Inspector Classification

The expanded native graph originally left all 11 targets in the 21-entry table
at `0x7C3B` looking like unexplained record-kind handlers. The selector was
misidentified: `0x7C65` skips selector zero and its `ETAT` label, then dispatches
selectors 1 through 21 alongside `POP` through sequel-only `ATTAQUE`. The
formatters only render typed object state for a diagnostic inspector. They do
not mutate records or advance gameplay.

`re/big_bug_bang_field_display_audit.json` binds that classification to the
known executable hash, the exact 21 labels, every field-matrix offset, all 11
formatter targets, and the owner call graph. GS:`0x6B7C` initializes to zero;
the closed static graph finds eight explicit reads and no explicit write. This
last result does not exclude an indirect write, so the report calls the path
statically dormant rather than unreachable. Rust's ignored original-asset test
independently checks the same matrix through
`script_field_offset_for_dialect`. Runtime semantic traces provide the useful
state inspection surface, but do not claim visual parity with this French
developer UI.

## Inherited Presentation AD Decoder

The expanded native audit left BBB `0xC0FE` structurally unmatched even though
its caller at `0xC016` maps to Commander Blood `0xA82C`. Direct execution now
classifies it as the sequel build of the already ported presentation AD decoder.
Its 422-byte body is bound by SHA-256
`3a37675c4b2da2f23fcf453b39a236a9278d700f0932bd8a08aca675976b9d70`;
the callee at `0xC2A4` is byte-identical to Commander Blood `0xAABC` across all
105 bytes.

```sh
nix develop -c python -P re/tools/big_bug_bang_presentation_ad_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation_ad.json
```

The oracle executes the complete unchanged BBB routine and helper against all
nine natural AD vectors previously derived from Commander Blood. It checks both
control layouts, both literal biases, fixed and variable runs, optional prefix
copying, 16-bit source wrapping with control refill, destination contents,
source immutability, self-modified helper immediates, registers, flags, stack
frames, and the original fixed-run overshoot. The BBB-specific report has
SHA-256 `269a84dfd3d84ebf805f0385766df447351879b899aca800546cad5cb433e9f3`.

The shared Rust `decode_presentation_ad` implementation matches the eight valid
vectors for both executables. It intentionally rejects the ninth case, where
the original writes four bytes despite declaring a three-byte destination
extent. This classifies one more entry from the 126-entry structural audit queue
as inherited behavior; it does not change the generated comparison count or
establish parity for unrelated presentation paths.

## Sequel C6 Travel Dispatcher

The expanded comparison leaves BBB's post-frame action dispatcher at `0x613F`
structurally unmatched. Its C6 arm at `0x6432..0x652D` is a 251-byte relocated
form of the Commander travel state machine, but the previous BBB travel oracle
only entered four downstream gates independently. It did not execute the C6
dispatcher or prove its record, relation, and position writes.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_script_travel_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_script_travel.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_c6_dispatch_matches_original_travel_vectors
```

The new oracle executes the complete unchanged `0x613F` routine through eight
C6 cases and lets the original `0x6633` field helper run. Only the established
camera-transition and ship-HUD far-call boundaries are captured. The cases
cover the actor gate, transition start, camera waits in phases one and two,
line 44 handoff, presentation blocking, action-record clearing, and both
black-hole relation/position branches. Exact global and VAR images, write
ownership, helper order and results, registers, segments, executable bytes,
and stack discipline are checked. The deterministic vector file has SHA-256
`17f2ec2e04dd01af007167f9833dbf3b2009503ae66a1eb60da8cebbfd5a6446`.

The direct comparison found one port defect: native C6 retains the action while
the camera countdown is nonzero in every nonzero phase, whereas Rust previously
applied that guard only while waiting for the camera. Rust now preserves a
phase-two action when the countdown is renewed and matches all eight native
vectors. This proves the C6 arm, not the dispatcher's separate C1-C4, C9, or CD
arms, and does not establish an end-to-end black-hole travel route.

## Sequel Presentation Scan

BBB's `0x5DD7..0x6037` post-frame presentation scan remained structurally
unmatched even though its Commander counterpart at `0x5816` is the source of
the shared typed `scan_script_presentations` implementation. The 609-byte BBB
body preserves the same coordinator, but uses relocated globals, different
record layouts, and an extra call to the already ported `0x6038` sequel state
processor after each active actor.

```sh
nix develop -c python3 -P \
  re/tools/big_bug_bang_presentation_scan_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation_scan.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  presentation_scan_accounts_for_both_original_native_vectors
```

The 16 direct original runs execute the complete unchanged scan and the real
`0x6633` field helper. They cover inactive entries; actor BAS handoff gates and
action dispatch; player presentation start, descriptor/effect setup and
teardown; complete and incomplete deferred records; navigation and world-state
actions; unknown active kinds; and the full-word next-directory-kind stop. The
callback order, presentation state, deferred triple, defined flags and
directory traversal agree with the equivalent Commander native vectors after
accounting for the sequel's field offsets and extra state-processor call. The
report also records exact changed bytes and full image hashes while the oracle
checks registers, segments, stack discipline and executable immutability. The
deterministic report has SHA-256
`736ec9d4bdd1bba7195336af53478c8d40174091d3afb1d0e7fda7fd0500571d`.

The shared Rust scan now consumes both original-game vector sets and matches
all 32 cases. BBB's BAS, action, resource, renderer, and state-processor calls
remain explicit subsystem boundaries with their own native coverage; this scan
does not re-prove those callees. It behaviorally classifies `0x5DD7`, but does
not establish end-to-end conversation playback or classify the remaining
uncovered arms of `0x613F`.

### Complete post-frame action dispatcher (`0x613F`)

The earlier C6 oracle covered only one arm of BBB's structurally unmatched
post-frame action dispatcher. The full guarded oracle now runs the complete
unchanged `0x613F..0x65E7` body together with the real `0x6633` field helper:

```sh
nix develop -c python3 -P \
  re/tools/big_bug_bang_script_action_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_script_action.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  original_oracle_separates_reachable_and_dead_record_arms
```

Its 33 cases retain all 32 Commander action-ladder scenarios and add the
sequel-only GS:`0x0CF1` gate that lets a C1 navigation action bypass the
inherited arche approach wait. The set covers C1 relinking, descriptor, HUD,
audio and position paths; C2 roster outcomes; C3 wildcard and radio handling;
C4 reciprocal and encounter-counter updates; all C6 phases; C9 clearing; CD
replacement; and an unknown record type. The two successful C2 cases stop
immediately before the shipped unmatched `POP ES` at `0x6343`, after verifying
the saved frame and every preceding mutation.

The oracle compares complete global and VAR images against an independent
model and checks changed bytes, native field results, callback arguments,
registers, segments, defined flags, stack discipline and executable
immutability. Its first 32 normalized callback traces match Commander, and the
extra case proves the BBB gate bypass. The deterministic JSONL has SHA-256
`4427e7e51bf4a5953dc4b42843199ee0e4b7142d48e12a41e939414a386b8f9e`.
The shared Rust classification test consumes both original fixtures. This
closes direct behavioral classification of `0x613F`; captured roster, audio,
renderer, nested-COD, camera and HUD callees retain their own proof boundaries,
and this result is not whole-game action parity.

## Sequel Presentation Scene Coordinator

BBB's `0xB4B0..0xB730` presentation-scene coordinator is the relocated sequel
counterpart of Commander Blood `0x9D10..0x9F52`, but its 641-byte body remained
structurally unmatched. The guarded oracle now executes the complete unchanged
BBB routine and captures its seven established image, back-buffer, resource,
palette, queue-state, queue-service and display-fill boundaries:

```sh
nix develop -c python3 -P \
  re/tools/big_bug_bang_presentation_scene_dispatch_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation_scene_dispatch.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  presentation_scene_dispatch
```

Eleven cases mirror the complete Commander coordinator fixture and retain the
same normalized callback order and typed result fields. They cover signed-line
exit, Scruter Jo overlay arming and handoff, changed and absent scene images,
shared and owned sources, all eight unclamped-line slots, blocked service,
line-five teardown, palette reset and ship depth opening. The oracle also
checks complete global and palette images, write ownership, registers, defined
flags, stack discipline and executable immutability. The routine body is bound
by SHA-256
`44bdf4ebff368c1f97fbb2aac01137939baa6c4d18938a7a96705cc0623b8213`;
the deterministic 13-row JSONL has SHA-256
`14582e8b9e0e38b7abb2406eeff127530cdac68bf697ff0c5bc5048605fd60c8`.

The two additional cases prove a sequel-only byte-exact resource-name branch.
When the basename starts with lowercase `fin` and its fourth byte is not `.`,
BBB forces vertical offset zero, unclamped rows, direct drawing off,
back-buffer presentation skipped and secondary request bit two set. `fin.HNM`
uses the ordinary line policy. The typed dispatcher now accepts that explicit
policy input, while the runtime derives it only for Big Bug Bang from the
selected resource basename after either DOS path separator. Focused tests cover
the native `finale.HNM` and `fin.HNM` cases plus case and directory boundaries.
This behaviorally classifies `0xB4B0`, not the seven captured callees or
end-to-end presentation parity.

## Sequel Contact Scene-Transition Coordinator

BBB's `0x1A17..0x1C55` contact scene-transition coordinator is the relocated
counterpart of Commander Blood `0x1855..0x1A93`. The guarded oracle executes
the complete unchanged 574-byte sequel body and captures its nine established
entity, scene-dispatch, DESCRIPT, image, renderer, bridge, alien and HUD
boundaries:

```sh
nix develop -c python3 -P \
  re/tools/big_bug_bang_scene_transition_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_scene_transition.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  coordinator_matches_both_original_scene_transition_fixtures
```

The 21 cases mirror Commander Blood's complete coordinator fixture. They cover
inactive and initialization paths; presentation and non-presentation image
loads; C2 gates; deferred-record arming; bridge callback blocking and reloads;
line-seven reloads; alien activity and C2 mutation; palette restoration;
finish; and final cleanup. After normalizing relocated returns and BBB's
one-byte FRIGO path shift, every shared result field and callback trace agrees.

The oracle independently checks complete global state, read-only record and
framebuffer regions, callback arguments and effects, changed bytes, registers,
segments, stack discipline and executable immutability, and records the defined
result flags. The routine body is bound by SHA-256
`75cfa0a8250bd5d6a34b7cc5791713ad60b36c359f076c8ff0d6503152583e2a`;
the deterministic 21-row JSONL has SHA-256
`14eb304162eb4a8d00b008f2763fbd0889dc1abe173578cf9c54df17329805ec`.
The shared typed coordinator already represented the observed behavior, so no
production semantic change was required; its regression now consumes both
original-game fixtures. This behaviorally classifies BBB `0x1A17`, not the nine
captured callees or end-to-end contact-scene parity.

## Remaining Completion Requirements

- Extend native comparison coverage beyond the now-complete A0-D7 opcode ledger
  into surrounding frame, runtime, presentation and conversation routines. The
  11 field-display targets are now classified as diagnostic inspector formatters,
  not unresolved gameplay handlers.
  Game-specific startup, profile switching, missing BAS ownership and SCRIPT2's
  adjacent read-only directory word are implemented, but those component results
  do not prove every cross-profile route.
- Port remaining changed native simulation, travel, interface and presentation
  behavior. The bounded AMER/CROOLIS asset and runtime frame comparison is
  complete as described above. Validate new media through the library-only
  import path and existing SDL3/wgpu rendering.
- Keep game selection, separate asset caches, save identities and source
  checksum manifests covered as packaging and runtime behavior evolve.
- Complete contextual and live coverage for English rendering, interaction and
  subtitle timing. Display catalogs now cover all 6,921 COD text sites across
  all 17 profiles plus the tracked non-COD layers, preserving logical IDs and
  source-hash fallback. This is broad source coverage, not a verified complete
  English playthrough; see `localization/big-bug-bang/README.md` for the remaining
  editorial and route limitations.
- Capture the original sequel in DOS and compare Rust behavior through startup,
  dialogue, travel, added gameplay and completion paths. Keep Commander regression
  coverage running alongside it. No whole-game parity claim from format tests.

Each item remains part of the full objective; completing the decoder or one
handler does not redefine the deliverable as a compatibility-only tool.

## PLAY Panel Comparison (2026-09-06)

The TV request after PLAY is present in the original, not evidence of a script
regression. In the local `native-play-05` capture, `after-07.bin` is still profile
0. The first profile-1 sample, `state-0194.bin` at 111.132 seconds, has pending
choice 2 (zero-based), panel actor flags 9, and sequence slot 3 named `1ppit`.
Samples remain at panorama frame 45 until 121.654 seconds, then turn to frame
88 and reach 90 at 122.155 seconds. The test pans back at 135 seconds; comparing
only its later snapshots incorrectly hides the automatic turn.

The Rust `modern-panel-request-01` capture similarly changes from no pending
choice and panel flags 1 to choice 2 and flags 9 on the SCRIPT2 load (frames
1311-1312), without a pointer press. A follow-up instrumented capture locates
the assignment at COD offset `0x9ED7`, outside query mode. Runtime traces now
include the last CC instruction offset and query mode; this provenance resets
on profile replacement while the session-owned pending choice is preserved.

The native UI word's `0x20` panorama-band bit is represented separately by
`PresentationBridgeMode::FirstBand` in Rust. Comparing that raw native word
against Rust's low UI bits alone is not a valid state comparison.

These observations do not establish cryobox interaction or whole-game parity.
The corrected ordinary-input comparison is retained as
`accuracy/scenarios/bbb_play_daddy.tsv`. The local `modern-play-daddy-01` run
completed 2834 recorded frames: it returned to panorama frame 45, rendered the
cryobox list with Daddy Gluxx, and closed the list after a click at (100, 100).
That click selected cancellation, not Daddy: the single contact occupies
y=89..99 and the cancel row begins at y=100. The native helper's nominal
(100, 100) motion actually settled at y=99. The retained scenario now uses
(100, 94), inside the contact row, and the boundary is covered by a focused
choice-list test. The earlier run's failure to start Daddy does not establish
a scene-handoff defect.

The corrected `modern-play-daddy-02` capture completed normally with 2834
recorded frames. Its final state has profile 2 (SCRIPT3) and active actor
`Daddy_Gluxx` (record 44). Screenshot `screen-026.png` visibly renders Daddy's
scene and the caption `Ageu rla... Mmeuh`. This verifies the ordinary-input
opening, PLAY, TV completion, return-pan and first cryobox-contact path. It
does not verify later dialogue, travel, completion, or the remaining English
translation.
