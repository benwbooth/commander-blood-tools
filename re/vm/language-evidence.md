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
both `CBVM-ASM` and `bloodscript-v2`. The current BloodScript manifest records
the unresolved generic-opcode and raw-byte totals rather than folding them into
semantic coverage.

Three dispatch-table-proven shared handler families now have lossless typed
statements: `SHARED_STATE` (`0x6863`), `SHARED_BIT_STATE` (`0x6902`), and
`RECORD_WILDCARD` (`0x6946`). This removes 10,477 bytes from generic `OP`
coverage while retaining byte-exact output across all five COD images.

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
COD and BAS corpus contains 247 image-local declarations and 5,962 proven uses.
`OBJECT` declarations emit no bytes and retain the original numeric offset.

The native field-offset matrix plus each object's initial VAR kind establishes
367 unambiguous physical subrecord fields used by 1,880 operands. Each `FIELD`
declaration names an object and a wrapping byte delta; its comment records the
VAR kind and every matrix selector sharing that physical offset. The decompiler
requires one unique owner, excludes zero matrix entries, and gives exact object
bases priority over fields. Other numeric subrecord addresses remain hexadecimal
rather than being assigned to a nearby object.

Exact DIC string boundaries establish a second symbolic namespace. The
structured corpus uses 13,699 referenced offsets in 53,194 proven dictionary
operands across COD and BAS. Each is written inline as `"text"@offset` rather
than through a generated declaration. Offset suffixes keep equal text at
different addresses distinct, and the compiler rejects an inline dictionary
reference in an ordinary numeric or VAR-address operand. The DIC companion
source remains the owner of the actual string bytes.

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
