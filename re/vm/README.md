# Commander Blood VM source recovery

Commander Blood loads two executable VM images for each of its five script
profiles:

- `SCRIPTn.COD` is the main state, object, presentation, and profile-control
  program.
- `SCRIPTn.BAS` is the conversation and concept-menu program. It is a binary
  image, not surviving text source, and it must not be parsed as if it were COD.

The other three files are inputs to those programs: `.VAR` is initial mutable
object state, `.DIC` is the text/concept dictionary, and `.DEB` is the symbol and
object directory. One unified `scriptN.blood` profile owns all five resources,
so symbols and offsets are resolved across the complete bundle rather than
copied between companion source files.

## Source-language evidence

The executable and CD establish a custom VM, not QuickBASIC, BASCOM, BRUN, QBX,
or VBDOS. The `.BAS` extension is real evidence that the original toolchain used
BASIC terminology, but the shipped `.BAS` files are tokenized binary programs.
No original text grammar or compiler has been recovered.

The most defensible reconstruction is therefore a game-specific BASIC-like
language, provisionally called **BloodScript**, with statements for blocks,
guards, state assignment, object/record operations, dialogue, concept menus,
yielding, and profile transitions. Its syntax will be our reconstruction; it
must not be represented as the exact historical syntax unless new evidence is
found.

## Verification ladder

1. BloodScript 8 combines structured COD/BAS programs and typed VAR object
   records in one profile source. DEB and DIC order are derived from declarations
   and first use instead of exposed as binary-shaped tables.
2. Compiling all five profiles must reproduce every byte of all 25 shipped VM
   resources before generated output is accepted.
3. Rebuilt bundles are substituted into the installed DOS game and exercised in
   DOSBox/oracle scenarios. Byte equality proves the compiler; game execution
   proves that structural rewrites remain behaviorally compatible.

Recover the typed COD control-flow graphs with:

```sh
cargo run --bin cbvm -- analyze-control-flow \
  accuracy/cblood_install/cblood re/vm/control-flow
```

Generate the binary-derived contact census with:

```sh
cargo run --bin cbvm -- analyze-contact-manifest \
  accuracy/cblood_install/cblood re/vm/contact-manifest
```

This pass finds all 65 procedures gated by native opcode `D1` (`during
contact`) and retains the whole activation predicate region regardless of
whether D1 appears before, between, or after the other requirements. The
shipped split is `1/15/16/19/14` procedures by profile, with 29 direct and 36
state-conditioned entries containing 661 TEXT tokens. Sixty-four procedures
have a C4 presentation predicate. `SCRIPT2.Cryomorn2` is the sole deliberate
exception: its Morning Oil entry is selected by location, state, and inventory
holder requirements instead. Five contact procedures begin disabled. JSON
retains exact typed predicate operands; TSV is the compact review index.

Run every recovered entry through a fresh isolated DOS guest with:

```sh
python3 -P re/tools/runtime_scenario_matrix.py \
  --cd-dir output/recovered_dos_package/cd \
  --install-parent accuracy/cblood_install \
  --all-contacts --jobs 4
```

The generic contact probe reloads the requested profile through the native
loader, disables competing D1 procedures, satisfies the manifest's supported
VAR/timer predicates, and submits the recovered object through the normal
contact transition. It accepts only word-list offsets owned by that procedure
and keeps the segment, IVT, MCB, input, audio, and active-presentation liveness
guards running until four valid lines, the first word choice, or clean dialogue
completion. Contact setup and post-selection dialogue each have a separately
bounded runtime window, so a slow native save load cannot consume the dialogue
observation period.

Run the same matrix against the shipped executable using its verified segment
layout, then compare the reports semantically:

```sh
python3 -P re/tools/runtime_scenario_matrix.py \
  --cd-dir output/recovered_dos_package/cd \
  --install-parent accuracy/cblood_install \
  --executable BLOODPRG.EXE \
  --link-map re/bin/BLOODPRG.segments.map \
  --all-contacts --jobs 4 \
  --output-dir output/contact-matrix-original

python3 -P re/tools/compare_runtime_scenario_matrices.py \
  --candidate output/contact-matrix-rebuilt/matrix.json \
  --reference output/contact-matrix-original/matrix.json \
  --reference-retry output/contact-matrix-original-retry/matrix.json \
  --output output/contact-matrix-differential.json
```

For successful probes, the comparator requires the same completion reason and
ordered recovered word-list offsets and subtitles; it does not compare the
arbitrary in-progress queue and chatter snapshot taken when the stop condition
fires. For failures, it additionally compares stable terminal presentation,
audio, and anomaly state while removing host sample counts, timer ticks,
volatile countdowns, and allocated segment values. A failure shared by both
binaries is reported as coverage-inconclusive rather than accepted as proof of
recovered-code parity. Focused retry matrices may cover any subset of the base
matrix. All attempts remain in the differential report, and a retry proves a
match only when both sides have a successful attempt with an identical semantic
signature.

Generate the five unified profiles with:

```sh
cargo run --bin cbvm -- decompile-unified \
  accuracy/cblood_install/cblood re/vm/profiles
```

Compile one complete edited profile with:

```sh
cargo run --bin cbvm -- compile-profile \
  re/vm/profiles/script1.blood /tmp/script1
```

Generated `bloodscript 8` source is intended for editing rather than for reading
as decorated disassembly. Its `state`, `logic`, and `conversations` sections own
all information required to emit the five binary images. DEB object order comes
from state declarations; procedures, globals, and labels retain the remaining DEB order;
DIC words are interned in logic-then-conversation first-use order.

Dialogue uses `say object presentation=N [modifiers] : "sentence"`. Named modifiers
include `chatter`, `repeatable`, `chance=20%`, `if_not_shown skip_next=N`,
`resume_at=LABEL`, and the
recovered aggressiveness predicate. A `|` inside a sentence forces a
no-space dictionary-token boundary only where combined and split DIC spellings
would otherwise select different bytes. `choices` represents the shipped
`0xFFFF` dialogue/concept separator. All generated dialogue statements rebuild
byte exactly.

Compile the complete 25-resource VM bundle and compare it with the installed
game:

```sh
python3 re/tools/compile_bloodscript_bundle.py \
  --source-dir re/vm/profiles \
  --output-dir output/recovered_scripts \
  --reference-dir accuracy/cblood_install/cblood
```

This builds `cbvm` once, emits uppercase `SCRIPTn.COD`, `.BAS`, `.DEB`, `.DIC`,
and `.VAR` files, and writes a bundle manifest. Every generated resource must
match the corresponding shipped image byte-for-byte.

The unified source keeps proven structures explicit. Typed object declarations
derive fixed names, record padding, runtime action slots, and BAS selector
roots; named `global` declarations replace the old globals table and state-symbol
exports. DIC order, padding entries, CP437 bytes, VAR object boundaries, and
the final DEB sentinel remain exact.

The canonical profiles use named statements for every shipped record, actor,
dialogue, menu, control-flow, and profile operation. They contain no generic
opcode, raw-byte, numeric object-address, or numeric field-address fallback.
Decompilation fails when a shipped construct cannot be assigned proven syntax.

The shared-word handler at native offset `0x6863` is represented as ordinary
assignments and signed `require` comparisons. Script-global words use recovered
names such as `globals.vbio`; proven kind-2 fields are named `encounter_count`,
`aggressiveness`, and `current_location`. The compiler reconstructs the
original family, operator, and RHS-mode bytes from these expressions.

RTC conditions are also source-level requirements. Opcode `CA` becomes
`require clock.hour <|>|== HOUR`; all 80 shipped forms use the literal tag
`C1`. Opcode `CB` becomes
`require annual_date <|>|== YYYY-MM-DD`. Its four shipped
operands are Christmas or New Year's Day date literals with encoded years 1994
or 1995. `annual_date` explicitly means that the game compares month and day;
the native handler consumes but does not compare the encoded year word and never
reads the RTC year stored at `GS:0x0AAC`. Compilation retains that source year
exactly.

The shared-bit handler at `0x6902` is represented as boolean properties:
`active`, `known`, and `portable`. These are native masks `0x0001`,
`0x0002`, and `0x0020` in each object's selector-`0x00` status word. Native
opcodes `AE` and `B0` are execution-identical aliases, including occurrences on
the same field with the same effect. BloodScript uses ordinary property
assignments and `require` for `AE`. The equally readable `mark OBJECT as STATE`
and `check OBJECT is STATE` forms select `B0`, retaining the original byte
without inventing a runtime distinction.

The direct-record handler at `0x6946` is represented as assignments and
`require` equality tests. Selector `0x11` is `current_location` on actor, ship,
and Orxx records, and `holder` on kind-`0x0400` inventory records. The native
`0xFFFF` ship-slot value is spelled `aboard`. Opcode `BC` publishes a
dictionary-backed actor `topic`, so source such as `Eviscerator.topic =
"secrets"` compiles to the original DIC offset and `BC` byte.

The native control-flow handlers at `0x6559`, `0x6572`, `0x65DB`, `0x65EB`,
`0x6830`, `0x6494`, and `0x64A0` lower from structured conditions, jumps,
timers, procedure headers, and scene-context guards. `A5` indexes the saved word
array at `GS:0x6ADE`; the timer ISR decrements exactly its first 30 words when
they are positive and no presentation is active. All 75 shipped `A5` uses lie
in that range. BloodScript renders their 48 writes as `timer[n] = ticks` or
`disabled` and their 27 zero tests as `require timer[n] == 0`. Every one
of the 480 kind-2 DEB procedures begins with an `A9` activation header, folded
into `proc name enabled|disabled { ... }`. Its hidden target is the next
procedure entry, or the sole final `halt`; all 480 shipped headers obey that
invariant. All 413 shipped `AB` byte writes target that same flag byte at a named
procedure's start plus one, so they are rendered as
`procedure.enabled = true|false`. The compiler derives the exact opcode, flag
byte, and address from these forms. Branch destinations are symbolic labels or procedure names; the
compiler resolves them without reordering statements or changing layout.

Opcode `A3` compares a DIC word against the logical current menu choice. The
native handler selects either the newly clicked word at `GS:0x6762` or its
resume copy at `GS:0x6764`; an inline `A1` inverts equality. BloodScript renders
the 239 direct and 80 inverted shipped forms as `require choice ==|!= "word"`.
Opcode `CF` clears the resume bit and saved word, represented by the 314 explicit
`choice = none` statements. Resets are not inferred or inserted, because five
shipped choice guards deliberately have no matching `CF` in their procedure.

Opcode `A7` conditionally offers one additional topic. Its handler writes the
DIC operand to `GS:0x6770` only while a presentation is active. After executing
the selected BAS body, native collector `0x5AFD` appends that pending word to
the current `A3` menu in `GS:0x67F8`, clears `0x6770`, and terminates the list.
All 19 shipped operands resolve through their profile's DIC, so BloodScript uses
`offer topic "word"`. A value absent from DIC retains `presentation_register`.

Opcode `A8` requests an HNM presentation sequence rather than merely loading an
arbitrary string. Its handler at `0x67C8` copies the operand to `SS:0x2120`,
selects presentation line 7, and raises request bit `GS:0x67AA & 2` when the
native presentation gates permit it. Resource table slot 7 at `DS:0x1FD1`
points to the descriptor at `DS:0x211B`; that descriptor's filename begins with
`sq\` at `DS:0x211D`, exactly three bytes before the mutable A8 buffer. The
resource loader consequently sees `sq\<operand>`. A case-sensitive `fin.`
prefix also raises the finale latch at `GS:0x67BD`.

All 89 shipped A8 operands are one of 36 basename-only `.hnm` names, and the
longest is 12 bytes. BloodScript renders them as
`request sequence "name.hnm"`. The high-level form allows at most 20 filename
bytes, which is the space from `DS:0x2120` through the byte before the next
descriptor at `DS:0x2135`, leaving room for the terminating NUL. A non-HNM or
otherwise nonconforming A8 operand retains the exact low-level `load_string`
fallback rather than being assigned sequence semantics.

Opcode `CC` assigns one of six rotating DESCRIPT sequence slots. The native
presentation-box driver cycles those slots, draws noise for an empty one, and
looks up a nonempty name in `DESCRIPT.DES` before dispatching its HNM, subtitle,
sound, and music commands. All 36 shipped assignments name existing
kind-`Sequence` records and fit the native 16-byte entries, so BloodScript uses
`sequence_slots[n] = "name"`. Invalid slot numbers and overlong synthetic names
retain the explicit `character_slot` fallback.

Selector `0x13` is now named `action` rather than the opaque `s13`. The native
post-VM scan resolves that selector for every active object and dispatches its
six-byte typed record through `record_c1_ship3d_action`. Presentation pairing is
one proven subset of that action protocol. All 389 shipped C4 operations execute
in query mode and test whether the named object's action is a C4 pair with the
built-in `blood` object; BloodScript spells these as
`require presentation == object`. All 35 C3 operations execute in update mode,
write `{C3,blood,1}`, and schedule a subsequent presentation, represented as
`queue presentation object`. All 371 C9 operations clear an `action`; when it
contains C4 the native handler also clears the reciprocal `blood.action` pair,
represented as `end presentation object`.

Other C3/C4 operand shapes retain `record_link` or `actor`, and a C9 target that
cannot be proven to be an action retains `record_clear`. String-bearing opcodes
are lifted only when they match the shipped
printable-ASCII plus `00 00` representation; other payload shapes retain the
generic lossless fallback.

Opcode `CD` is an inventory transfer. Its first operand identifies the source
owner through an `action` field, its second operand is the moved object, and its
third operand is the destination holder. The handler writes selector `0x11` on
the moved object and synchronizes the 16-word special-slot list when the source
or destination is `blood`; BloodScript names that special holder `aboard` and
renders the proven form as `transfer ITEM from SOURCE to DESTINATION`. All 182
shipped uses are non-inverted updates with a kind-`0x0400` item and a source and
destination restricted to `blood` or kind-2 characters. There are 46 such
instructions in COD and 136 in BAS. Other possible shapes retain
no inferred meaning; the canonical decompiler accepts the shipped shape only.

`re/vm/source/manifest.tsv` records semantic and unresolved byte coverage for
all ten program images. The BAS decoder now walks the recovered sequential
grammar: dictionary-validated menus, text records, both yield opcodes,
four-byte linked selector nodes, offered-topic writes, and the shared
record/state operations used by both image kinds.

All 118,787 COD bytes and all 64,736 BAS bytes now have decoded token boundaries.
Each of the 321 `0xAC` bytes is a one-byte yield followed by a distinct selector
node `{selector:u16, next:u16}`. Native `value_scan_match` at `0x577A` compares
the selector, follows `next` directly on mismatch, and returns the body at node
offset `+4` on match. Runtime tracing additionally establishes that `0xAC`
terminates the selected response body.

The selector-list CFG pass additionally resolves the owning object through its
kind-1 `.DEB` symbol and selector-2 `.VAR` field. The 37 nonzero object fields
match all 37 physical list roots exactly (`1/10/12/10/4` by profile), and their
linked chains own all 321 nodes once. The generated graphs contain 963 match,
mismatch, miss-exit, and body-termination edges with no unresolved entrypoint.
See `bas-control-flow/manifest.tsv` and its README for the field derivation.
The structured corpus now renders these as 37 named `selector` regions and 321
`case` headers. The compiler derives node links and terminator yields from case
order and compiles back to all 64,736 BAS bytes exactly.

`cbvm compile-bundle` turns the five unified profiles into a complete 25-file
VM resource set, refusing any result that differs from the shipped file. `cbvm
build-runtime-tree` installs the result into an extracted-CD asset tree without
retaining any original script resource. The original DOS executable boots and
runs that tree in DOSBox-X; see [runtime-validation.md](runtime-validation.md),
`profiles/manifest.tsv`, and `bundle-manifest.tsv`.

The shipped address conventions are now enforced rather than inferred during
display. All 480 kind-2 `.DEB` routine values are one-based: subtracting one
lands on a COD token boundary, while the encoded value never does. The 284
image-local distinct nonzero BAS `next` values and the 1,054 image-local
distinct explicit COD branch destinations are zero-based offsets. Every BAS
`next` value lands on the first byte of a selector node; the former one-based
reading resulted from incorrectly grouping the preceding `0xAC` with that node.
An unaligned address in any of these sets makes decompilation fail.

Brace-delimited `proc` blocks delimit the 480 named COD routines. Label colons
remain only for behaviorally necessary nonlocal jumps and dialogue resume
points; internal guard and BAS selector addresses are compiler-derived. These
structures emit no bytes, but symbolic operands are resolved and range-checked
by the two-pass BloodScript compiler.

The COD and BAS sections additionally use 302 image-local object aliases. Their
6,781 uses replace direct operands, record relation values, and transfer
endpoints. Each alias resolves against a named object in the same profile's
`state` section, so no global VAR offset is duplicated in source.

The remaining shipped action records are also lifted without guessing. Twenty
C1 updates on the built-in `orxx` object are `navigate to LOCATION`; two C2
updates on `blood` are `bring CHARACTER aboard`; and two mode-1 C6 comparisons
on `arche.action` are `require travel through BLACK_HOLE`. The C6 wording is a
condition deliberately: native navigation code produces the deferred C6 record
after black-hole entry, and the script observes it before requesting the next
profile. These forms require exact DEB/VAR owner and operand kinds plus the
native mode and inversion; a nonmatching shipped shape would fail decompilation.

The final five rare COD state operations have similarly bounded domain forms.
Two update-mode `BD` writes to kind-`0x0010` selector `0x0B` are
`Kraner.position = (x, y)`. Three `B7` sets address selector `0x05` on kind-2
characters at DEB bit index 2; because entry 2 is the built-in `blood` object
and native helper `0x6210` proves the object-index mapping, they are
`Character.known_objects += blood`. The sequel's retained `CONNAIS` field name
independently confirms that this bitmap stores known directory objects.

The native field matrix and each object's kind resolve 367 properties used by
1,880 direct VAR operands. BloodScript writes the retained compiler field names
as ordinary object properties and derives their offsets. Equal textual
proximity, zero matrix entries, ambiguous owners, and unmatched addresses do
not produce an alias; a shipped unresolved operand fails decompilation.

The structured COD and BAS sources intern 13,713 distinct referenced DIC offsets
as readable string operands, with 53,262 uses in dialogue, concept, menu,
selector, offered-topic, and actor-topic positions. They are bare quoted
literals interned by first use after the profile's explicit `concepts` seeds.
Rare inline `dictionary blank after` statements preserve empty DIC entries at the
point where they occur. Dictionary literals are accepted only in
dictionary-typed operand positions and lower to the exact original `u16`.

The current BloodScript corpus recompiles all 183,523 program bytes exactly. It
contains 13,524 typed statements covering every byte with no shipped generic
`OP` or `RAW` fallback. Unified source semantics cover all 134,312 companion
bytes, making the complete 25-resource, 317,835-byte VM bundle
source-reproducible. In BloodScript 8, directory and dictionary bytes are derived
from the ordered `state`, `logic`, and `conversations` sections rather than
stored in standalone source tables. Both instruction streams are fully framed
and typed. The COD pass now recovers 7,010 basic blocks and 17,287 typed
edges across all 480 DEB procedures, with no unresolved guard target. Five
disabled block bodies are retained as unreachable evidence. The structured pass
recovers all 682 `A0` guard regions. Forty-four are proven if/else forms whose
true arm ends in `A4 <join>`; the five other formerly rejected regions are
`sort`, `Corpo4`, `oto1`, `tromp1`, and `big3`. Their retry, navigation,
procedure-boundary, and dialogue-resume edges remain visible as labels or jumps
inside their structured regions. No shipped `guard_push`, `guard_pop`, or
standalone `activation` statement remains, and no shipped procedure exposes an
`until` target. See `profiles/manifest.tsv` for all 25 byte-exact images,
`control-flow/manifest.tsv` for graph counts, `bas-control-flow/manifest.tsv`
for selector-list graphs, and
[language-evidence.md](language-evidence.md) for the source-language
inference.
