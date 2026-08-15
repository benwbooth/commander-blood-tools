# Structured BloodScript corpus

This directory is the readable source layer above `re/vm/bloodscript`. Generate
it with:

```sh
cargo run --bin cbvm -- decompile-structured \
  accuracy/cblood_install/cblood re/vm/structured
```

`bloodscript-v5` has no address column or opcode-style uppercase syntax. The
compiler lays out statements first, then resolves every label, procedure,
branch, selector link, and guard target. Generated source uses four-space
indentation for procedure, guard, selector, and case bodies. Redundant
disassembly comments are omitted; comments remain only for recovery evidence
that is not expressed by syntax.

The shared-word handler is lifted to ordinary state expressions. Script-global
words use `state[address]`; the two proven kind-2 fields use
`object.encounter_count` and `object.conversation_progress`. Updates use `=`,
`+=`, or `-=`, while query-mode comparisons use `require` with the signed
operators `!=`, `<`, `>`, `<=`, `>=`, and `==`. The compiler derives the
original `B4`/`BF`/`C0` family byte, `F0..F7` operator byte, and immediate or
state-backed RHS mode from the expression and reproduces the original seven
bytes. Kind-2 selector `0x11` is also named `current_location`, based on its
native consumers. Other selector names remain explicit until their roles are
similarly established.

The `0x6902` bit family is lifted to boolean properties on each object's
selector-`0x00` `flags` word. The corpus has 177 uses of `active`, `in_play`, or
`presentable`, corresponding exactly to masks `0x0001`, `0x0002`, and `0x0020`.
The `AE` and `B0` bytes reach one handler and that handler never reads the
opcode; both bytes also encode identical operations on identical fields in the
shipped scripts. `using alternate_encoding` retains a shipped `B0` where byte
identity requires it and deliberately carries no runtime meaning.

The separate `B7` single-bit handler now has a narrow object-link form. All
three shipped uses are mode-0 sets of bit 2 in selector `0x05` on a kind-2
character. DEB entry 2 is the built-in `blood` object in both affected
profiles, and helper `0x6210` independently proves that this field is indexed by
the high-bit-first DEB object number. They therefore render as
`Character.links += blood`. `links` is intentionally a structural name: the
only native consumer tests membership during the dormant kind-`0x10` C1 source
scan, whose shipped call site reads the persistent `DS:0x6886` scratch buffer
rather than the character field. Other indices or field kinds retain
`bit_flag` instead of receiving a guessed gameplay meaning.

Both shipped `BD` pair writes now render as ordinary coordinate assignments:
`Kraner.position = (x, y)`. Their exact field is selector `0x0B` on a
kind-`0x0010` record, the same two-word x/y field consumed by the native
position resolver, distance helper, state processor, HUD path, and camera
code. The lift requires update mode, opcode `BD`, that exact kind, and that
exact selector. Query-mode pairs, sibling `B8`/`B9` encodings, and other fields
remain `pair_record`.

All 75 shipped `A5` operations address the 30-word range serviced by the timer
ISR. The 48 update forms are `timer[n] = ticks` or `timer[n] = disabled` for the
native `0xFFFF` sentinel; the 27 query forms are `require timer[n] == 0`. The
ISR decrements only positive entries, and the query handler succeeds only at
zero. An index outside `0..29` or another negative-class value retains
`state_array_set`/`state_array_test` so the high-level syntax cannot imply a
countdown the binary does not perform.

The 19 shipped BAS `A7` operations are `offer topic "word"`. While a
presentation is active, the handler places its DIC operand in a one-word
pending slot. The menu collector appends that word to the selectable topic
list and clears the slot. Every shipped operand resolves to its profile's DIC;
an unresolved value retains `presentation_register`.

The `0x6946` record family is lifted to 531 location/holder assignments and
equality requirements plus 49 actor-topic assignments. Selector `0x11` is
`current_location` for kind `0x0002`, `0x0010`, and `0x0200`, while the same
parent relation is `holder` for kind-`0x0400` inventory objects. `aboard` is the
native `0xFFFF` ship-slot sentinel. Kind-2 selector `0x0F` is `topic`; its `BC`
assignment accepts an interned DIC string and reproduces the exact dictionary
offset.

Menu choices use ordinary conditions. Opcode `A3` becomes `require choice ==
"word"`; its inline inversion byte becomes `!=`. The native handler reads the
newly selected DIC word or its saved resume copy, so both storage paths are one
logical `choice` in source. Opcode `CF` remains the explicit `choice = none`
statement that clears the saved choice and resume state. The corpus has 319
choice requirements and 314 resets; the compiler preserves that difference
instead of pairing them heuristically.

HNM cutscene selections use `request sequence "name.hnm"`. Opcode `A8` writes
the basename at `DS:0x2120`, which is the mutable suffix of resource slot 7's
`sq\` descriptor, then conditionally stages presentation line 7. All 89 shipped
uses are `.hnm` basenames (36 unique, at most 12 bytes), and each recompiles to
the original padded string instruction. The compiler limits this high-level
form to 20 bytes so its NUL cannot cross into the next descriptor. An unusual
A8 payload that does not meet those established constraints is rendered as the
explicit `load_string` fallback.

Presentation relationships are also source-level. The native post-VM object
scan resolves selector `0x13` as a six-byte typed action record, so the 115
unambiguous field declarations now use `object.action` instead of `object.s13`.
The narrower C3/C4/C9 lifecycle is rendered without exposing that storage:
`require presentation == Actor` tests the active C4 pair,
`queue presentation Actor` writes the C3 scheduled form, and
`end presentation Actor` clears the action and any reciprocal C4 pair. The
shipped corpus has 389 requirements, 35 queues, and 371 endings; every C3/C4
related operand is the built-in `blood` object.

The three other shipped action-record forms now use their proven domain
semantics. All 20 C1 updates target the built-in kind-`0x0200` `orxx.action`
field and name a kind-`0x0080` sublocation, so they render as `navigate to
LOCATION`. Both C2 updates target `blood.action` and name a kind-2 character;
the native handler inserts that character into the Ark's special-slot list,
writes the `0xFFFF` aboard sentinel, and requests presentation line `0x27`, so
they render as `bring CHARACTER aboard`. The two C6 tokens are mode-1 guards,
not transition requests: the navigation actor stages C6 after the player enters
a kind-`0x0100` black hole, and the script tests the resulting `arche.action`
record before changing profiles. They render as `require travel through
BLACK_HOLE`. Any owner, field, kind, mode, or inversion outside these exact
shapes retains `record_state` or `record_entry`.

Inventory movement uses `transfer ITEM from SOURCE to DESTINATION`. Native
opcode `CD` derives `SOURCE` from the owner of its first action-field operand,
writes `DESTINATION` to the kind-specific selector-`0x11` holder field of
`ITEM`, and maintains the 16-word special-slot list when either endpoint is the
built-in `blood` object. BloodScript calls that endpoint `aboard`; entering it
writes the native `0xFFFF` holder sentinel, while leaving it removes the item
from the slot list. All 182 shipped updates (46 COD and 136 BAS) have this exact
shape: a non-inverted action field, a kind-`0x0400` item, and a destination that
is either `blood` or a kind-2 character. Nonmatching or query-mode `CD` tokens
retain the exact `record_triple` fallback.

Each kind-2 procedure begins with an `activation enabled|disabled until target`
header backed by its native `A9` flag byte. Writes to those bytes are named
assignments such as `dialogue.enabled = false`. All 413 shipped writes target a
named procedure exactly; arbitrary byte writes retain `poke_byte` as a lossless
fallback.

The `when target { ... } then { ... }` syntax is a lossless structural form of
the native `A0 target` / `A1` guard protocol. `when` and `then` emit those
original bytes. The closing brace emits no bytes and must occur exactly at the
resolved target. The compiler validates brace nesting, procedure boundaries,
and derived target offsets.

The BAS pass uses brace-delimited `selector name { ... }` and `case selector ->
next { ... }` blocks. List and case braces emit no bytes; `case` emits the same four-byte
`{selector,next}` header as low-level `selector_node`. The existing `yield_b`,
`menu`, response/state operations, and terminal `yield` or `halt`
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

The shipped COD corpus contains 682 `A0` guards. This pass structures 633
(92.8 percent) and deliberately leaves 49 as `GUARD_PUSH`/`GUARD_POP`.
Fallback is evidence that the region has not met the structural proof, not an
invitation to guess. Each retained `GUARD_PUSH` has an
`unstructured_guard=<reason>` comment, and `manifest.tsv` records the reason
counts per image. All 49 shipped fallbacks are `alternate_exit`: at least one
CFG edge leaves the candidate interval somewhere other than its declared end.
The analyzer also distinguishes non-forward targets, cross-procedure targets,
missing balanced pops, crossing regions, shared pops, and external entries when
they occur. Comments do not emit bytes.

All ten generated COD and BAS sources compile to the exact 183,523 shipped
bytes; per-image guard, rejection, list, and case counts are in `manifest.tsv`.

Every dialogue record is a `say` statement with named control fields and one
sentence literal. BloodScript uses the companion DIC as the sentence lexicon;
it inserts exact dictionary offsets rather than storing duplicate strings in
the program image. `choices` denotes the native `0xFFFF` separator. The rare
`|` character forces a token boundary where the DIC contains both a combined
punctuated spelling and its split form.

COD and BAS sources also declare exact kind-1 DEB object bases with zero-byte
`object name = offset` directives. An object name is accepted only in an operand
position already established as a VAR address, and the compiler lowers it to
the declared `u16` without changing layout. The current corpus contains 302
image-local object declarations with 6,781 uses; record-value aliases cover
locations, holders, inventory transfers, and other referenced objects as well
as direct operands.

Subrecord addresses use zero-byte `field name = object + delta` declarations only
when the address has exactly one owner under the native field-offset matrix and
that object's initial VAR kind. The current corpus contains 367 fields and 1,880
uses. The compiler computes the wrapping base-plus-delta address; comments retain
the VAR kind and matrix selectors. Zero matrix entries, ambiguous owners, and
unmatched addresses remain hexadecimal rather than using nearest-object guesses.

COD and BAS sources intern exact DIC references as string operands. The current
corpus has 13,713 distinct referenced offsets and 53,262 uses in dictionary-typed
dialogue, concept, menu, selector, offered-topic, and actor-topic operands. All
shipped references are bare quoted literals resolved through the companion DIC;
there are no generated dictionary declarations or address suffixes in the
corpus. Equal text at multiple physical offsets uses the lowest offset as its
canonical interned value, with `"text"@offset` retained as a lossless escape for
noncanonical references. The quoted portion does not duplicate or replace the
separately compiled DIC string; edit `script*.dic.blooddata` to change text. The
compiler accepts these references only in dictionary-typed positions.

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
