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
both `CBVM-ASM` and `bloodscript-ir-v1`. The current BloodScript manifest records
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

The remaining 3,780 BAS bytes form 682 complete records: three one-topic menus,
19 presentation-register writes, three string loads, 37 yields, 321 block links,
and 299 shared state/record operations and end markers. The sequential decoder
now types all of them. Every block-link selector resolves through its script's
dictionary and every continuation is zero or an in-image offset; runtime BASSTEP
traces independently establish `0xAC` as the response-block terminator. All
64,736 BAS bytes now compile with no `RAW` fallback.

The whole ten-image corpus therefore compiles byte-for-byte from typed
BloodScript IR. This closes byte framing, not source structuring: record fields
and reducible control-flow blocks still need to be lifted above the exact IR.

Address correlation establishes three corpus-wide rules:

- all 480 kind-2 `.DEB` routine values are one-based COD addresses;
- 284 image-local distinct nonzero BAS targets are one-based BAS addresses;
- 1,054 image-local distinct explicit COD targets are zero-based COD addresses.

Every address resolves to a decoded token boundary under its respective rule.
The contrary interpretation fails universally for the `.DEB` routine values and
BAS continuations: none of their raw encoded values is a token boundary.

BloodScript now emits 480 balanced `PROCEDURE`/`END_PROCEDURE` regions from the
DEB names, plus `LABEL` directives for other COD blocks and BAS responses. Its
two-pass compiler resolves symbolic operands while retaining explicit source
offsets and exact statement order. Across the corpus this yields 1,059 distinct
COD symbols and 284 BAS labels without changing any output byte.

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
   blocks and per-procedure control-flow graphs.
4. Lift reducible graph regions into guards and conditional blocks while
   retaining address labels for irreducible regions.
5. Add symbolic object, field, and dictionary names without changing numeric
   identity in the compiler IR.
6. Compile the structured syntax back through the exact IR and require all ten
   images to remain byte exact.
7. Emit complete `.COD`, `.BAS`, `.DEB`, `.DIC`, and `.VAR` bundles and
   substitute them into the installed game for DOSBox scenario tests.

The historical spelling of the language may remain unknowable. Behavioral and
binary fidelity do not depend on guessing it: the reconstructed compiler is
accepted only when its output and the original VM agree.
