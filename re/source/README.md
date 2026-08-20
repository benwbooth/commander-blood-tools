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
4. Compile with candidate historical compilers and compare the generated
   assembly shape and bytes against the recovered routine.
5. Keep the source only when the comparison is close enough to justify it.

Generated routine length is not an ABI requirement for the normally linked
source build. Size and placement become acceptance constraints only for the
separate fixed-offset patch path. Linked routines are judged on calling
convention, pointer and struct layout, segment/global ownership, and behavior.

The assembly dumps under `re/assembly` remain the evidence. A missing `.c` file
means the routine has not yet cleared the natural-C evidence gate.

Natural C routines remain under the stable `bloodprg/candidates` and
`xdb/candidates` paths while their evidence matures. The manifest `status`
field is authoritative: `codegen_mismatch` remains under review, while
`codegen_accepted` has cleared the source-port gate. Keeping paths stable lets
the binary oracles, compiler corpus, and eventual translation-unit assembly
refer to the same one-to-one routine source without duplicating declarations.

Start with:

- `coverage.md` for current natural-C candidate coverage by module.
- `compiler_corpus.md` for the compiler/codegen comparison gate.
- `bloodprg/candidates/README.md` for pending natural-C candidate rules.
- `bloodprg/abi_observations.tsv` for current routine-level ABI facts.
- `routine_status.tsv` for routines that were rejected from the wrapper-style
  attempt and must be reworked from natural declarations.

Before accepting new recovered source, keep the assembly inventory closed:

```sh
python3 re/tools/split_xdb_slot3_assembly.py --check
python3 re/tools/split_xdb_resume_assembly.py --check
python3 re/tools/assembly_inventory.py --check
python3 re/tools/xdb_source_inventory.py --check
nix develop -c python3 re/tools/natural_candidate_oracle.py --check
nix develop -c python3 re/tools/xdb_candidate_oracle.py --check
```

The XDB source inventory cross-checks standardized callable owners against the
top-level routine index. Separate assembly artifacts retained for reviewed
internal branch labels are checked against `re/assembly/boundary_overrides.tsv`
and deliberately excluded from the C manifest. Older broad callback dumps that
still need a split audit remain visible as pending candidates rather than being
silently promoted to one-routine owners.
