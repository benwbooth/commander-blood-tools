# VM source-language evidence

This document separates properties recovered from the shipped game from syntax
reconstructed by this project. The distinction is an acceptance requirement:
readability is not evidence of historical accuracy.

## Verified artifacts

Each of the five script profiles ships as a related bundle:

| File | Verified role |
| --- | --- |
| `SCRIPTn.COD` | Main executable VM token stream |
| `SCRIPTn.BAS` | Binary conversation and concept-menu program |
| `SCRIPTn.DEB` | 20-byte symbol records, including kind-2 named routine offsets |
| `SCRIPTn.DIC` | Null-terminated text and concept dictionary |
| `SCRIPTn.VAR` | Initial mutable object/state records |

The executable contains a 52-slot opcode dispatch table for byte values
`0xA0..0xD3`. Native handler recovery establishes a custom event/state VM with
dialogue, object-record operations, tests, control flow, yields, and script
profile transitions. The `.DEB` names establish that the source or compiler had
routine boundaries; `.DIC` and `.VAR` establish separate interned-text and
initial-state inputs.

All ten shipped `.COD` and `.BAS` images now round-trip byte exactly through
both `CBVM-ASM` and `bloodscript-v5`. The current BloodScript manifest records
the unresolved generic-opcode and raw-byte totals rather than folding them into
semantic coverage.

Three dispatch-table-proven shared handler families now have lossless typed
statements: `SHARED_STATE` (`0x6863`), `SHARED_BIT_STATE` (`0x6902`), and
`RECORD_WILDCARD` (`0x6946`). This removes 10,477 bytes from generic `OP`
coverage while retaining byte-exact output across all five COD images.

The `0x6863` family is now lifted one level further. Its assembly proves signed
query operators `F0..F5`, update operators `F5..F7`, and state-indirect RHS
modes `C0`/`C2`. The shipped images use family tag `B4` only for Bob Morlock's
conversation-progress field, `BF` only for kind-2 encounter counters, and `C0`
for script-global words. A whole-image assembly census found no reader of those
three family tags outside the dispatch table; the common handler does not read
the tag either. BloodScript therefore renders the shipped operations as `=`,
`+=`, `-=`, and signed `require` expressions and derives the original bytes
from the target category. The generated compiler output remains byte exact.

The shared-bit handler at native offset `0x6902` establishes the complete
shipped meaning of `AE` and `B0`. In query mode an optional `A1` inverts an
any-masked-bit test; in update mode it switches OR-mask to AND-complement-mask.
The outer opcode is never read after dispatch, and the same field, mask,
polarity, and effect occur under both opcode bytes in the shipped corpus. Native
consumers establish mask `0x0001` as object `active`, `0x0002` as `in_play`, and
`0x0020` as the C2 presentation eligibility bit (`presentable`). BloodScript
therefore emits boolean property expressions. It preserves `B0` with the
non-semantic `using alternate_encoding` code-generation clause because no
runtime distinction exists from which that byte could be re-derived.

This alias result has been investigated to the boundary of the shipped
artifacts. `SCRIPT*.DEB` records only name, offset, and kind; `SCRIPT*.VAR`
records the same object kind and field storage used by both aliases. The four
shipped XDB overlays contain recovered runtime/minigame code and no BASIC,
compiler, parser, variable, keyword, or script-compiler strings. No shipped
module contains the offline source compiler. The historical reason its compiler
selected `AE` versus `B0` is therefore absent, while their execution semantics
are established and identical.

The direct-record handler at native offset `0x6946` establishes the shipped
`AF` and `BC` forms. In query mode `AF` compares a resolved field to an object
word and optional `A1` inverts equality. In update mode it assigns that relation
while maintaining the 16-entry aboard-object list. Operand `blood` maps to
`0xFFFF` before a query, and `0xFFFF` is the native aboard sentinel. Selector
`0x11` is consumed as a parent relation by the navigation tree: actor, arche,
and Orxx records use it as `current_location`; kind-`0x0400` inventory records
use it as `holder`.

`BC` adds one proven side effect: it publishes its raw RHS through
`gs:0x6782`. The presentation selector at `0x56FE` consumes that word to choose
a BAS case and stores it in the active actor's kind-2 selector-`0x0F` field.
Every shipped `BC` RHS resolves to a DIC word such as `talk`, `hello`, `rien`,
or `secrets`. BloodScript consequently names the field `topic` and interns the
quoted RHS through the companion dictionary. All 531 shipped `AF` operations
and 49 `BC` operations reproduce their original bytes.

Finally, opcode `A9` (`CONDITIONAL_BLOCK`) sets the native query bit just as an
`A0` guard does. The source formatter now tracks that transition, so conditions
immediately following an `A9` are rendered as `require` expressions rather than
updates. A focused compiler test pins the `A9` -> `B0` query -> `A1` -> `BC`
update sequence byte for byte.

Seven control-flow encodings now have lossless typed statements: `GUARD_PUSH`
and `GUARD_POP` (`0xA0`/`0xA1`), `JUMP` (`0xA4`), `STATE_ARRAY_TEST` and
`STATE_ARRAY_SET` (the query/set forms of `0xA5`), `CONDITIONAL_BLOCK` (`0xA9`),
and the `0xCE`/`0xD0` flag branches. This removes another 5,854 generic bytes.
Together the two lifts reduce generic coverage from 20,898 to 4,567 bytes, a
78.15 percent reduction, without changing any compiled COD byte.

The final six native-handler families account for those remaining 4,567 bytes:
concept guards (`0xA3`), string loads (`0xA8`), self-modifying byte writes
(`0xAB`), character-slot bindings (`0xCC`), alternate-concept clears (`0xCF`),
and the `0x274F` flag branch (`0xD1`). All 118,787 shipped COD bytes now compile
from typed statements with zero generic `OP` coverage.

The remaining 3,780 BAS bytes form 1,003 complete records: three one-topic menus,
19 presentation-register writes, three string loads, 37 `0xAA` yields, 321
`0xAC` yields, 321 linked selector nodes, and 299 shared state/record operations
and end markers. The sequential decoder now types all of them. Every node
selector resolves through its script's dictionary and every nonzero `next`
value points directly to another selector node. Runtime BASSTEP traces
independently establish `0xAC` as the selected response-body terminator. All
64,736 BAS bytes now compile with no `RAW` fallback.

The selector-node boundary is established statically, not inferred from the
corpus pattern. `vm_op_ac_yield` at executable file offset `0x685C` consumes no
operand. `vm_control_flow` starts the linked-list scan one byte after its saved
block pointer (`0x5715..0x5718`). `value_scan_match` at `0x577A` reads the node's
selector, assigns the second word directly to the scan cursor on mismatch, and
returns the node address plus four on match. Therefore `0xAC` and
`{selector,next}` are separate records and `next` is an ordinary zero-based BAS
offset.

The list roots are also recovered from data rather than guessed from layout.
`vm_cod_scan` resolves field selector 2 against each object's VAR kind and uses
the resulting word as a BAS offset. For kind `0x0002`, the executable's field
matrix maps selector 2 to object offset `+0x1A`. Restricting the DEB directory
to kind-1 object symbols yields 37 nonzero fields; all 37 point to the `AC`
immediately before one physical selector-list root, with no missing or interior
entry. Following those roots covers all 321 nodes exactly once.

BloodScript's structured BAS form therefore names lists after their owning DEB
objects and spells each linked node as `CASE selector next`. This is a
reconstructed source notation, not a claim about the original token spelling.
The compiler keeps the exact yield and menu records visible, validates the
proven list grammar, and reproduces all five BAS images byte for byte.

The whole ten-image corpus therefore compiles byte-for-byte from typed
BloodScript IR. This closes byte framing, not source structuring: record fields
and reducible control-flow blocks still need to be lifted above the exact IR.

Address correlation establishes three corpus-wide rules:

- all 480 kind-2 `.DEB` routine values are one-based COD addresses;
- 284 image-local distinct nonzero BAS targets are zero-based selector-node addresses;
- 1,054 image-local distinct explicit COD targets are zero-based COD addresses.

Every address resolves to a decoded token boundary under its respective rule.
The one-based rule applies only to `.DEB` routine values. Treating BAS `next`
values as one-based points at the preceding `0xAC`, not the node that the native
scanner reads.

BloodScript now emits 480 balanced `PROCEDURE`/`END_PROCEDURE` regions from the
DEB names, plus `LABEL` directives for other COD blocks and BAS selector nodes. Its
two-pass compiler resolves symbolic operands while retaining explicit source
offsets and exact statement order. Across the corpus this yields 1,059 distinct
COD symbols and 284 BAS labels without changing any output byte.

Kind-1 DEB entries also establish exact VAR object-base names. The structured
COD and BAS corpus contains 273 image-local declarations, including object
values recovered from record relations.
`OBJECT` declarations emit no bytes and retain the original numeric offset.

The native field-offset matrix plus each object's initial VAR kind establishes
367 unambiguous physical subrecord fields used by 1,880 operands. Each `FIELD`
declaration names an object and a wrapping byte delta; its comment records the
VAR kind and every matrix selector sharing that physical offset. The decompiler
requires one unique owner, excludes zero matrix entries, and gives exact object
bases priority over fields. Other numeric subrecord addresses remain hexadecimal
rather than being assigned to a nearby object.

Exact DIC string boundaries establish a second symbolic namespace. The
structured corpus uses 13,712 referenced offsets in 53,243 proven dictionary
operands across COD and BAS, including the typed actor topics. All shipped
operands are interned bare string
literals resolved through the companion DIC rather than routed through generated
declarations. The compiler chooses the lowest offset as the canonical identity
for duplicate text and retains `"text"@offset` only for a noncanonical physical
entry. It rejects dictionary references in ordinary numeric or VAR-address
operands, and the DIC companion source remains the owner of the actual string
bytes.

The first structured COD pass recovers 7,010 basic blocks and 17,287 edges. It
models the native query bit and guard-target stack, direct jumps, text skips and
deferred frame resumes, and both possible states of self-modified `A9` block
flags. Every one of the 413 shipped `POKE_BYTE` instructions targets an `A9`
flag byte. All branch-capable instructions resolve to a concrete guard target.
Exactly five block bodies are unreachable because their opener flag remains
zero; they are preserved rather than deleted from the source evidence.

The source-structuring pass converts 633 of the 682 `A0` guards into
balanced `WHEN`/`THEN`/`END_WHEN` regions. A region is accepted only when it is
forward, procedure-local, non-crossing, single-entry, and single-exit according
to the recovered CFG. The other 49 guards remain explicit low-level tokens.
Every retained guard is deterministically classified `alternate_exit`: at
least one recovered edge leaves its candidate interval somewhere other than
the guard's declared end. Each generated `GUARD_PUSH` records the reason in a
non-semantic comment, and the manifest reports the reason counts per image.
Both forms compile through the same exact backend, and all ten structured
sources reproduce the shipped bytes.

The earlier 443-region count was an analyzer artifact: destination membership
was tested against the destination basic-block leader instead of the exact
target instruction. A guard beginning in the middle of a block could therefore
appear to exit its own interval. CFG edges now retain both addresses; regression
tests distinguish this case from a real jump to an alternate exit.

## What is not established

The `.BAS` files are binary data, not surviving text source. No QuickBASIC,
BASCOM, BRUN, QBX, or VBDOS signature has been found in the ten program images.
The `.BAS` suffix is evidence that the developers used the term "BAS", but it
does not identify a Microsoft or Borland BASIC grammar, compiler, or runtime.

No source text, grammar, parser, compiler executable, or language manual has
been recovered. Consequently, exact keywords, expression syntax, declaration
syntax, and whether the historical implementation was called BASIC cannot be
recovered from the suffix alone.

## Defensible reconstruction

The available evidence supports a small, game-specific event language more
strongly than a general-purpose BASIC dialect. A useful reconstructed language
will need these constructs:

- named procedures from `.DEB` offsets;
- guards and conditional blocks from VM tests and control-flow tokens;
- object and global state reads, comparisons, and assignments;
- actor/background/resource selection and profile transitions;
- dialogue records with speaker, sound, presentation flags, and dictionary text;
- concept menus and responses from `.BAS` plus `.DIC`;
- explicit yield/resume points matching interpreter behavior.

A plausible high-level form might look like this:

```basic
PROCEDURE ScruterJo
  WHEN Scruter.state = Waiting
    ACTOR ScruterJo
    SAY voice 1, "SCANNING STRANGER..."
    YIELD
  END WHEN
END PROCEDURE
```

This example is proposed BloodScript syntax. It is not a decompilation of a
specific byte range and must never be labelled as the original 1994 source.

## Recovery path

1. Give each generic `OP` a typed statement only after its native handler and
   operand semantics are proven.
2. Decode the remaining BAS structures without changing a byte of the rebuilt
   images.
3. Use the recovered procedures and symbolic targets to construct typed basic
   blocks and per-procedure control-flow graphs. Complete for shipped COD.
4. Lift reducible graph regions into guards and conditional blocks while
   retaining symbolic labels and explicit rejection evidence for irreducible
   regions. The current guard lift proves 633 of 682 `A0` regions and classifies
   all 49 retained low-level guards.
5. Add symbolic object, field, and dictionary names without changing numeric
   identity in the compiler IR. Exact DEB object bases and referenced dictionary
   words are complete for the currently proven operand families. The first field
   pass names every direct operand with a unique nonzero DEB/VAR/matrix relation;
   unproven subrecord addresses remain numeric.
6. Compile the structured syntax back through the exact IR and require all ten
   images to remain byte exact.
7. Emit complete `.COD`, `.BAS`, `.DEB`, `.DIC`, and `.VAR` bundles and
   substitute them into the installed game for DOSBox scenario tests.

The historical spelling of the language may remain unknowable. Behavioral and
binary fidelity do not depend on guessing it: the reconstructed compiler is
accepted only when its output and the original VM agree.
