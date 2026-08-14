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

`re/vm/source/manifest.tsv` records semantic and unresolved byte coverage for
all ten program images. BAS semantic coverage is intentionally conservative:
only dictionary-validated menu tables and text records are labelled today;
every other byte is retained as `RAW BAS structure`.

The initial corpus covers all 118,787 COD bytes with decoded token boundaries.
For BAS it labels 60,956 of 64,736 bytes (94.16 percent) as validated menu or
text spans and preserves the remaining 3,780 bytes raw. These percentages are
structural coverage, not a claim that every opcode's high-level meaning or the
historical source syntax is known.
