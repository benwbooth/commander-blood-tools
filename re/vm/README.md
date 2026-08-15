# Commander Blood VM source recovery

Commander Blood loads two executable VM images for each of its five script
profiles:

- `SCRIPTn.COD` is the main state, object, presentation, and profile-control
  program.
- `SCRIPTn.BAS` is the conversation and concept-menu program. It is a binary
  image, not surviving text source, and it must not be parsed as if it were COD.

The other three files are inputs to those programs: `.VAR` is initial mutable
object state, `.DIC` is the text/concept dictionary, and `.DEB` is the symbol and
object directory. The lossless BloodData source layer now emits those three
companions, so the bundle compiler rebuilds all five files for every profile.

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

1. `CBVM-ASM` is the lossless textual layer. Every line owns explicit bytes and
   offsets; comments are non-semantic. Disassembly followed by assembly must
   reproduce every `.COD` and `.BAS` byte exactly.
2. Proven instructions replace raw spans in the typed IR without changing the
   assembled bytes.
3. BloodScript statements lower to that typed IR. Any not-yet-proven construct
   remains an explicit low-level operation or byte span rather than a guess.
4. Rebuilt bundles are substituted into the installed DOS game and exercised in
   DOSBox/oracle scenarios. Byte equality proves the compiler; game execution
   proves that structural rewrites remain behaviorally compatible.

Run the current lossless pass with:

```sh
cargo run --bin cbvm -- decompile-bundle \
  accuracy/cblood_install/cblood re/vm/source
```

Generate the typed BloodScript IR with:

```sh
cargo run --bin cbvm -- decompile-bloodscript \
  accuracy/cblood_install/cblood re/vm/bloodscript
```

Recover the typed COD control-flow graphs with:

```sh
cargo run --bin cbvm -- analyze-control-flow \
  accuracy/cblood_install/cblood re/vm/control-flow
```

Generate the first proven structured source view with:

```sh
cargo run --bin cbvm -- decompile-structured \
  accuracy/cblood_install/cblood re/vm/structured
```

Generate lossless source for the DEB/DIC/VAR data companions with:

```sh
cargo run --bin cbvm -- decompile-data-bundle \
  /path/to/extracted-game re/vm/structured
```

Compile one edited BloodScript IR image with:

```sh
cargo run --bin cbvm -- compile-bloodscript \
  re/vm/structured/script1.cod.blood /tmp/SCRIPT1.COD \
  /path/to/SCRIPT1.DIC
```

Generated `bloodscript-v5` source is intended for editing rather than for
reading as a decorated disassembly. It uses lowercase statements, `0x` numeric
literals, `none` for absent optional values, declaration expressions, label
colons, concise DEB-derived names, and four-space indentation. A layout pass
derives label and procedure offsets before encoding.

Dialogue uses `say object voice=... flags=... display=... loop=...
control=... : "sentence"`. The compiler tokenizes the sentence through the
companion DIC and emits the exact original word offsets. A `|` inside a sentence
forces a dictionary-token boundary only where combined and split DIC spellings
would otherwise select different bytes. `choices` represents the shipped
`0xFFFF` dialogue/concept separator. All 5,536 generated dialogue statements use
this form and rebuild byte exactly.

BloodData keeps the known physical structures explicit without inventing
unrecovered field names. DEB source has one exact 20-byte `SYMBOL` record per
line, DIC source has one NUL-owning `STRING` entry per dictionary offset, and
VAR source has offset-checked little-endian `WORDS` plus an optional odd-byte
tail. Full DEB name fields are retained rather than normalized, so padding and
CP437 bytes round-trip exactly.

The typed files use named statements for established record, actor, dialogue,
menu, and profile operations. `OP` is an explicitly generic decoded opcode, not
a claim that its source-level meaning is understood. `RAW` retains bytes whose
instruction framing is not yet established. Both forms are deliberate
verification escapes and must be eliminated by evidence, not renamed guesses.

The shared-word handler at native offset `0x6863` is represented as ordinary
assignments and signed `require` comparisons. Script-global words use
`state[address]`; the proven kind-2 fields are named `encounter_count`,
`conversation_progress`, and `current_location`. The compiler reconstructs the
original family, operator, and RHS-mode bytes from these expressions.

The shared-bit handler at `0x6902` is represented as boolean properties:
`active`, `in_play`, and `presentable`. These are native masks `0x0001`,
`0x0002`, and `0x0020` in each object's selector-`0x00` `flags` word. Native
opcodes `AE` and `B0` are execution-identical aliases, including occurrences on
the same field with the same effect. BloodScript uses the ordinary property form
for `AE` and appends `using alternate_encoding` for `B0`; that clause preserves
the original byte without claiming a behavior the handler does not have.

The direct-record handler at `0x6946` is represented as assignments and
`require` equality tests. Selector `0x11` is `current_location` on actor, ship,
and Orxx records, and `holder` on kind-`0x0400` inventory records. The native
`0xFFFF` ship-slot value is spelled `aboard`. Opcode `BC` publishes a
dictionary-backed actor `topic`, so source such as `Eviscerator.topic =
"secrets"` compiles to the original DIC offset and `BC` byte.

The native control-flow handlers at `0x6559`, `0x6572`, `0x65DB`, `0x65EB`,
`0x6830`, `0x6494`, and `0x64A0` are represented as guard push/pop, jump,
state-array test/set, conditional-block, and flag-branch statements. Their
flags, indices, and values remain explicit in source. Branch destinations are
now symbolic labels or procedure names; the compiler resolves them without
reordering statements or changing layout.

The remaining native handlers provide typed concept guards, presentation-name
loads, self-modifying COD byte writes, character-slot bindings, alternate
concept clears, and the `0x274F` flag branch. String-bearing opcodes are lifted
only when they match the shipped printable-ASCII plus `00 00` representation;
other payload shapes retain the generic lossless fallback.

`re/vm/source/manifest.tsv` records semantic and unresolved byte coverage for
all ten program images. The BAS decoder now walks the recovered sequential
grammar: dictionary-validated menus, text records, both yield opcodes,
four-byte linked selector nodes, presentation-register writes, and the shared
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
`case` headers. Those directives retain the explicit native yield bytes and
compile back to all 64,736 BAS bytes exactly.

`cbvm compile-bundle` turns the structured corpus into a complete 25-file VM
resource set. It compiles every COD/BAS BloodScript image and every DEB/DIC/VAR
BloodData image, refusing any result that differs from the shipped file. `cbvm
build-runtime-tree` installs the result into an extracted-CD asset tree without
retaining any original script resource. The original DOS executable boots and
runs that tree in DOSBox-X; see [runtime-validation.md](runtime-validation.md),
`structured/data-manifest.tsv`, and `bundle-manifest.tsv`.

The shipped address conventions are now enforced rather than inferred during
display. All 480 kind-2 `.DEB` routine values are one-based: subtracting one
lands on a COD token boundary, while the encoded value never does. The 284
image-local distinct nonzero BAS `next` values and the 1,054 image-local
distinct explicit COD branch destinations are zero-based offsets. Every BAS
`next` value lands on the first byte of a selector node; the former one-based
reading resulted from incorrectly grouping the preceding `0xAC` with that node.
An unaligned address in any of these sets makes decompilation fail.

Brace-delimited `proc` blocks delimit the 480 named COD routines. Label colons
name remaining COD blocks and BAS selector nodes. These structures
emit no bytes, but symbolic operands are resolved and range-checked by the
two-pass BloodScript compiler. The generated corpus contains 1,059 distinct COD
symbols and 284 BAS selector labels while retaining exact layout.

The structured COD and BAS sources additionally use 273 image-local zero-byte
`OBJECT` declarations recovered from exact kind-1 DEB offsets. They replace
direct operands and record relation values while retaining the original numeric
offset in each declaration.

A first subrecord pass adds 367 zero-byte `field name = object + delta`
declarations and replaces 1,880 direct VAR operands. A field is emitted only
when its address equals exactly one DEB object base plus a nonzero entry selected
from the native field-offset matrix by that object's initial VAR kind. Equal
textual proximity, zero matrix entries, ambiguous owners, and unmatched
addresses do not produce an alias. The compiler resolves the declared wrapping
base-plus-delta expression back to the original `u16` address.

The structured COD and BAS sources intern 13,712 distinct referenced DIC offsets
as readable string operands, with 53,243 uses in dialogue, concept, menu,
selector, and actor-topic positions. They are bare quoted
literals resolved through the companion DIC image; the shipped corpus needs no
generated dictionary declarations or address suffixes. If equal text exists at
multiple offsets, the lowest offset is canonical and a noncanonical reference
uses the lossless `"text"@offset` escape. Dictionary literals are accepted only
in dictionary-typed operand positions and lower to the exact original `u16`;
the DIC companion source remains the owner of the string bytes.

The current BloodScript corpus recompiles all 183,523 program bytes exactly. It
contains 13,524 typed statements covering every byte with no shipped generic
`OP` or `RAW` fallback. The BloodData corpus adds 14,676 offset-checked
statements for all 134,312 companion bytes, making the complete 25-resource,
317,835-byte VM bundle source-reproducible. Both instruction streams are fully
framed and typed. The COD pass now recovers 7,010 basic blocks and 17,287 typed
edges across all 480 DEB procedures, with no unresolved guard target. Five
disabled block bodies are retained as unreachable evidence. The structured pass
proves 633 of 682 `A0` guard regions and classifies the remaining 49 explicit
low-level guards as `alternate_exit`: at least one CFG edge leaves the candidate
interval somewhere other than its declared end. Each generated fallback records
that reason as a non-semantic comment. See `bloodscript/manifest.tsv` for
per-image byte coverage,
`control-flow/manifest.tsv` for graph counts, `structured/manifest.tsv` for
source-lift and rejection counts, `bas-control-flow/manifest.tsv` for
selector-list graphs,
and
[language-evidence.md](language-evidence.md) for the source-language
inference.
