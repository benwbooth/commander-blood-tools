# Commander Blood Natural C Recovery

This tree is for reconstructing source that a 16-bit DOS C/C++ compiler could
plausibly lower into the recovered assembly. It is not a host compatibility
port and not an emulation layer.

Rules for recovered source:

- Write natural C/C++ over named globals, structs, arrays, and functions.
- Do not use byte-array memory APIs such as `read16_far` or `write16_far`.
- Do not model CPU registers, flags, segment registers, or machine state as C
  data structures.
- Do not add placeholder, no-op, or guessed routines.
- Keep a one-to-one routine mapping until translation-unit evidence proves a
  larger original source boundary.
- Mark a routine pending when the real ABI, global owner, segment mapping,
  struct layout, or carry/flag behavior is not understood.

For each routine, the expected workflow is:

1. Read the assembly and record the real inputs, outputs, globals, segment
   assumptions, and callees.
2. Recover the likely C-level declarations: structs, globals, arrays, enums,
   and function prototype.
3. Write the smallest natural source expression of that logic.
4. Compile with the candidate Borland/Turbo compiler and compare the generated
   assembly shape against the recovered routine.
5. Keep the source only when the comparison is close enough to justify it.

The assembly dumps under `re/assembly` remain the evidence. A missing `.c` file
means the routine has not yet cleared the natural-C evidence gate.

Natural C candidates may live under `bloodprg/candidates` before that compiler
gate is satisfied. They are useful for review and codegen experiments, but they
are not accepted replacement source until promoted out of the candidate tree.

Start with:

- `compiler_corpus.md` for the compiler/codegen comparison gate.
- `bloodprg/candidates/README.md` for pending natural-C candidate rules.
- `bloodprg/abi_observations.tsv` for current routine-level ABI facts.
- `routine_status.tsv` for routines that were rejected from the wrapper-style
  attempt and must be reworked from natural declarations.
