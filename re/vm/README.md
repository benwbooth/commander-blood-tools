# Commander Blood VM source recovery

Commander Blood loads two executable VM images for each of its five script
profiles:

- `SCRIPTn.COD` is the main state, object, presentation, and profile-control
  program.
- `SCRIPTn.BAS` is the conversation and concept-menu program. It is a binary
  image, not surviving text source, and it must not be parsed as if it were COD.

The other three files are inputs to those programs: `.VAR` is initial mutable
object state, `.DIC` is the text/concept dictionary, and `.DEB` is the symbol and
object directory. A complete source compiler will eventually need to emit all
five files as one bundle.

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

Compile one edited BloodScript IR image with:

```sh
cargo run --bin cbvm -- compile-bloodscript \
  re/vm/bloodscript/script1.cod.blood /tmp/SCRIPT1.COD
```

The typed files use named statements for established record, actor, dialogue,
menu, and profile operations. `OP` is an explicitly generic decoded opcode, not
a claim that its source-level meaning is understood. `RAW` retains bytes whose
instruction framing is not yet established. Both forms are deliberate
verification escapes and must be eliminated by evidence, not renamed guesses.

The recovered shared handlers at native offsets `0x6863`, `0x6902`, and
`0x6946` are represented as `SHARED_STATE`, `SHARED_BIT_STATE`, and
`RECORD_WILDCARD`. These statements preserve the opcode-family byte, optional
`A1` prefix, operator/mode bytes, and operands, so they are structured without
weakening the byte-exact compiler contract.

The native control-flow handlers at `0x6559`, `0x6572`, `0x65DB`, `0x65EB`,
`0x6830`, `0x6494`, and `0x64A0` are represented as guard push/pop, jump,
state-array test/set, conditional-block, and flag-branch statements. Their
numeric targets, flags, indices, and values remain explicit in source; the
compiler does not recalculate or normalize addresses.

`re/vm/source/manifest.tsv` records semantic and unresolved byte coverage for
all ten program images. BAS semantic coverage is intentionally conservative:
only dictionary-validated menu tables and text records are labelled today;
every other byte is retained as `RAW BAS structure`.

The initial corpus covers all 118,787 COD bytes with decoded token boundaries.
For BAS it labels 60,956 of 64,736 bytes (94.16 percent) as validated menu or
text spans and preserves the remaining 3,780 bytes raw. These percentages are
structural coverage, not a claim that every opcode's high-level meaning or the
historical source syntax is known.

The current BloodScript corpus recompiles all 183,523 input bytes exactly. It
contains 12,521 typed statements covering 179,743 bytes. Of that typed total,
1,233 statements and 4,567 bytes are still generic `OP` forms; the BAS images
retain 3,780 `RAW` bytes. See `bloodscript/manifest.tsv` for per-image counts and
[language-evidence.md](language-evidence.md) for the source-language inference.
