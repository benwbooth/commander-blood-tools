# Recovered Assembly Dumps

Each `.asm` file is rooted at a recovered routine entrypoint and includes
the provenance that made that entrypoint eligible. BLOODPRG routines are
grouped by recovered MZ relative segment. XDB routines are grouped by
entry/API seeds, method tables, manu3 labeled code seeds, and direct-call
discovery. These groups are not claimed to be original compiler
translation units unless future evidence proves that.

These dumps are the evidence used by the handwritten C source recovery under
`re/source`. They are not generated C/C++ and no longer point at the retired
emulator-style translation scaffold.

Routine counts:

- `bloodprg`: 308
- `xdb_amer`: 25
- `xdb_croolis`: 25
- `xdb_manu3`: 18
- `xdb_scrut`: 25
