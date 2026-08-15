# Structured BloodScript corpus

This directory is the readable source layer above `re/vm/bloodscript`. Generate
it with:

```sh
cargo run --bin cbvm -- decompile-structured \
  accuracy/cblood_install/cblood re/vm/structured
```

The `WHEN target` / `THEN` / `END_WHEN target` syntax is a lossless structural
form of the native `A0 target` / `A1` guard protocol. `WHEN` and `THEN` emit
those original bytes. `END_WHEN` emits no bytes and must occur exactly at its
resolved target. The compiler validates nesting, matching names, procedure
boundaries, and target offsets.

A guard is lifted only when all of these are established:

- its target is forward and remains in the same DEB procedure;
- a unique, balanced `A1` divides its conditions from its body;
- its interval does not cross another candidate guard;
- no CFG edge enters its interior from outside;
- no CFG edge exits it except through its declared end.

The shipped COD corpus contains 682 `A0` guards. This pass structures 443
(65.0 percent) and deliberately leaves 239 as `GUARD_PUSH`/`GUARD_POP`.
Fallback is evidence that the region has not met the structural proof, not an
invitation to guess. All ten generated COD and BAS sources compile to the exact
183,523 shipped bytes; per-image counts are in `manifest.tsv`.

This syntax is reconstructed by this project. It is not claimed to be the
original 1994 source spelling.
