# Recovered Assembly Dumps

Each `.asm` file is rooted at a recovered routine entrypoint and includes
the provenance that made that entrypoint eligible. BLOODPRG routines are
grouped by recovered MZ relative segment. XDB routines are grouped by
entry/API seeds, method tables, manu3 labeled code seeds, and direct-call
discovery. These groups are not claimed to be original compiler
translation units unless future evidence proves that.

The indexed counts are the closed set of currently seeded routine owners, not
a claim that every function-pointer target has already been discovered. New
code addresses recovered from data writes are recorded in
`xdb/data_referenced_entries.tsv`; entries remain pending there until their
complete control-flow boundary is reviewed and an assembly routine dump can be
added without guessing.

These dumps are the evidence used by the handwritten C source recovery under
`re/source`. They are not generated C/C++ and no longer point at the retired
emulator-style translation scaffold.

`boundary_overrides.tsv` records reviewed corrections to recursive-graph seeds.
The inventory checker requires every merged entry to be absent from the index,
its owner to remain indexed, and its address to fall inside the owner's byte
range.

Routine counts:

- `bloodprg`: 318
- `xdb_amer`: 25
- `xdb_croolis`: 25
- `xdb_manu3`: 12
- `xdb_scrut`: 25

The pending data-referenced ledger is not included in these counts.
