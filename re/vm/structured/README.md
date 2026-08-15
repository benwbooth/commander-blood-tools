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

The BAS pass uses `SELECTOR_LIST name` / `CASE selector next` /
`END_SELECTOR_LIST name`. List boundaries emit no bytes; `CASE` emits the same
four-byte `{selector,next}` header as low-level `SELECTOR_NODE`. The existing
`YIELD_B`, `MENU`, response/state operations, and terminal `YIELD` or `END`
remain explicit byte-owning statements. The compiler requires every list to
begin at one `YIELD_B`, every case body to begin with `MENU`, every nonterminal
case to end at the `YIELD_B` immediately before its declared next case, and the
terminal case to carry a zero `next` value.

List names come from the kind-1 DEB object whose selector-2 VAR field points to
that BAS root. All 37 object-owned lists and all 321 cases are structured; none
remain as low-level `SELECTOR_NODE` statements in this corpus.

A guard is lifted only when all of these are established:

- its target is forward and remains in the same DEB procedure;
- a unique, balanced `A1` divides its conditions from its body;
- its interval does not cross another candidate guard;
- no CFG edge enters its interior from outside;
- no CFG edge exits it except through its declared end.

The shipped COD corpus contains 682 `A0` guards. This pass structures 443
(65.0 percent) and deliberately leaves 239 as `GUARD_PUSH`/`GUARD_POP`.
Fallback is evidence that the region has not met the structural proof, not an
invitation to guess. Each retained `GUARD_PUSH` has an
`unstructured_guard=<reason>` comment, and `manifest.tsv` records the reason
counts per image. All 239 shipped fallbacks are `alternate_exit`: at least one
CFG edge leaves the candidate interval somewhere other than its declared end.
The analyzer also distinguishes non-forward targets, cross-procedure targets,
missing balanced pops, crossing regions, shared pops, and external entries when
they occur. Comments do not emit bytes.

All ten generated COD and BAS sources compile to the exact 183,523 shipped
bytes; per-image guard, rejection, list, and case counts are in `manifest.tsv`.

COD sources also declare exact kind-1 DEB object bases with zero-byte
`OBJECT name offset` directives. An object name is accepted only in an operand
position already established as a VAR address, and the compiler lowers it to
the declared `u16` without changing layout. The current corpus contains 104
used object declarations and 4,113 symbolic operands: 3,687 `TEXT` line owners,
389 actor relations, 35 record links, and two record-entry relations.

This pass aliases only an exact DEB object base. It does not assign a numeric
subrecord offset to the nearest object or invent field names. Such expressions
remain hexadecimal until the object's VAR kind and the native field-offset
matrix prove both ownership and field identity.

This syntax is reconstructed by this project. It is not claimed to be the
original 1994 source spelling.

## BloodData companions

The adjacent `script*.deb.blooddata`, `script*.dic.blooddata`, and
`script*.var.blooddata` files are the lossless source for the other fifteen VM
resources. Generate them with:

```sh
cargo run --bin cbvm -- decompile-data-bundle \
  /path/to/extracted-game re/vm/structured
```

DEB uses fixed-width `SYMBOL` records with the complete 16-byte name field plus
the two trailing words. DIC uses `STRING` records that own their terminating
NUL; `TAIL` is available for an unterminated final payload. VAR uses
little-endian `WORDS`, with `BYTES` reserved for an odd tail. Every statement
has an explicit output offset, and compilation rejects gaps, overlaps, malformed
field widths, and wrong directive kinds.

`data-manifest.tsv` records 14,676 statements covering 134,312 bytes. Every data
source recompiles byte-for-byte, and `compile-bundle` combines them with the ten
BloodScript images to rebuild all 25 VM resources from checked-in source.
