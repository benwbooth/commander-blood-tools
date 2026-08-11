# Commander Blood C Source Recovery

This tree is the source-reconstruction path. It replaces the retired
emulator-style semantic lift.

Rules for this tree:

- Read the assembly first and port the logic into real C data operations.
- Keep one recovered C function per original routine until call graph and
  translation-unit evidence prove a better boundary.
- Use routine addresses in names while signatures are still being recovered.
- Do not add placeholders, no-op implementations, fake stubs, or convenience
  behavior just to make a build pass.
- If flags, calling convention, segment ownership, or fall-through control flow
  are not understood, leave that routine out of this tree until they are.
- Inline assembly is allowed only for hardware/BIOS/DOS ABI edges that C cannot
  express.

The assembly dumps under `re/assembly` remain the source evidence. The C files
here are handwritten ports from those dumps and should compile as ordinary C89
with the compatibility macros in `include/cb_types.h`; Borland-specific ABI
wrappers can be added only when the surrounding caller/callee contracts are
known.
