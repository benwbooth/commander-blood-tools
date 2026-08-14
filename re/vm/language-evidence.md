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
3. Use `.DEB` routine offsets and proven branch targets to construct a control
   flow graph.
4. Lift reducible graph regions into guards, blocks, and procedures while
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
