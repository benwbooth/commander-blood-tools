# VM source-language evidence

This document separates properties recovered from the shipped game from syntax
reconstructed by this project. The distinction is an acceptance requirement:
readability is not evidence of historical accuracy.

## Verified artifacts

Each of the five script profiles ships as a related bundle:

| File | Verified role |
| --- | --- |
| `SCRIPTn.COD` | Main executable VM token stream |
| `SCRIPTn.BAS` | Binary conversation and concept-menu program |
| `SCRIPTn.DEB` | 20-byte symbol records, including kind-2 named routine offsets |
| `SCRIPTn.DIC` | Null-terminated text and concept dictionary |
| `SCRIPTn.VAR` | Initial mutable object/state records |

## Recovered object vocabulary

The Big Bug Bang sequel retains Commander Blood's complete 21-by-16-byte
field-offset matrix and also retains the compiler's field-name table beside it.
The names map by selector index to `ETAT`, `POP`, `BASE`, `AGR`, `NRJ`,
`CONNAIS`, `PORTEE`, `EVL`, `RENCONTRE`, `POSITION1`, `POSITION2`, `POSITION`,
`UNIVERS1`, `UNIVERS2`, `UNIVERS`, `SUJET`, `RACE`, `LIEU`, `MESSAGE`, `ACTION`,
and `ICONE`. The sequel appends an `ATTAQUE` selector at character byte 72 and
expands its character record from 72 to 74 bytes; that sequel-only field is not
part of Commander Blood's schema. Its HUD independently reads character offsets 22, 50, 52, and 56
and labels them `POPULATION`, `AGRESSIVITE`, `ENERGIE`, and `EVOLUTION`.

The adjacent status table supplies both the applicable kind mask and stored
bit for `OK`, `CONNUS`, `CHEF`, `GUERRE`, `PRESENT`, `OCCUPE`, `PLEIN`,
`BLOQUE`, `EMET`, `AGIT`, `ACTIF`, and `PORTABLE`. BloodScript translates the
shipped subset to `active`, `known`, `leader`, `at_war`, `present`, `full`,
`acting`, `enabled`, and `portable`. The same executable retains the exact
16-value race table: `croolis_red`, `croolis_green`, `migrax`, `slimers`,
`izwals`, `sinox`, `waves`, `tromps`, `kam`, `tubular_brain`, `quizzers`,
`zen`, `scruters`, `robots`, `bob`, and `gluxx`.

BloodScript 8 uses those names in typed VAR records. Inline object names,
zeroed action records, known-object storage, reserved padding, and character BAS
roots are compiler-derived. Character BAS
roots come from the matching `selector NAME_choices` declaration; globals are
allocated in source declaration order. The all-five-profile byte gate verifies
the derived layout against every shipped VAR, BAS, and DEB byte.

The omitted padding has been investigated rather than assigned guessed field
names. Character byte 28 and bytes 64 through 67 are zero in all 593 character
records across the five Commander Blood profiles and the seven available sequel
profiles. Neither the original nor sequel field matrix addresses them, and the
complete BLOODPRG disassembly has no character-relative access to bytes 64 or
66. The XDB instructions that use displacements 64 and 66 operate on their own
94-byte overlay object records, not on VM VAR records.

The final `orxx` object is 36 bytes in both games. Its matrix-addressable fields
end at byte 17; bytes 18 through 35 are zero in every inspected profile and the
only two native consumers of the resolved `orxx` base use its position fields or
its six-byte action at byte 10. Every DEB places a kind-5 state symbol named
`tblood` at `orxx + 36`, followed by ordinary globals at `+38`. `tblood` starts
at zero, has no COD or BAS reference in either game, and is not the special
kind-5 symbol that BLOODPRG resolves (`vbio` is). It is therefore a separate,
unused compiler-injected state word, not part of the navigation controller and
not an editable gameplay variable.

The executable contains a 52-slot opcode dispatch table for byte values
`0xA0..0xD3`. Native handler recovery establishes a custom event/state VM with
dialogue, object-record operations, tests, control flow, yields, and script
profile transitions. The `.DEB` names establish that the source or compiler had
routine boundaries; `.DIC` and `.VAR` establish separate interned-text and
initial-state inputs.

All ten shipped `.COD` and `.BAS` images now round-trip byte exactly through
both `CBVM-ASM` and the program sections of `bloodscript 8`. The canonical
profiles contain no generic `OP` or `RAW` statements.

Three dispatch-table-proven shared handler families now have lossless typed
statements: `SHARED_STATE` (`0x6863`), `SHARED_BIT_STATE` (`0x6902`), and
`RECORD_WILDCARD` (`0x6946`). This removes 10,477 bytes from generic `OP`
coverage while retaining byte-exact output across all five COD images.

The `0x6863` family is now lifted one level further. Its assembly proves signed
query operators `F0..F5`, update operators `F5..F7`, and state-indirect RHS
modes `C0`/`C2`. The shipped images use family tag `B4` only for Bob Morlock's
aggressiveness field, `BF` only for kind-2 encounter counters, and `C0`
for script-global words. A whole-image assembly census found no reader of those
three family tags outside the dispatch table; the common handler does not read
the tag either. BloodScript therefore renders the shipped operations as `=`,
`+=`, `-=`, and signed `require` expressions and derives the original bytes
from the target category. The generated compiler output remains byte exact.

The shared-bit handler at native offset `0x6902` establishes the complete
shipped meaning of `AE` and `B0`. In query mode an optional `A1` inverts an
any-masked-bit test; in update mode it switches OR-mask to AND-complement-mask.
The outer opcode is never read after dispatch, and the same field, mask,
polarity, and effect occur under both opcode bytes in the shipped corpus. Native
consumers establish mask `0x0001` as object `active`, `0x0002` as `known`, and
`0x0020` as the C2 presentation eligibility bit (`portable`). BloodScript
therefore emits boolean property expressions. Ordinary assignments and
`require` select `AE`; the semantic synonyms `mark OBJECT as STATE` and
`check OBJECT is STATE` select `B0`. Both forms say what the operation does while
retaining the otherwise execution-identical opcode byte.

This alias result has been investigated to the boundary of the shipped
artifacts. `SCRIPT*.DEB` records only name, offset, and kind; `SCRIPT*.VAR`
records the same object kind and field storage used by both aliases. The four
shipped XDB overlays contain recovered runtime/minigame code and no BASIC,
compiler, parser, variable, keyword, or script-compiler strings. No shipped
module contains the offline source compiler. The historical reason its compiler
selected `AE` versus `B0` is therefore absent, while their execution semantics
are established and identical.

The direct-record handler at native offset `0x6946` establishes the shipped
`AF` and `BC` forms. In query mode `AF` compares a resolved field to an object
word and optional `A1` inverts equality. In update mode it assigns that relation
while maintaining the 16-entry aboard-object list. Operand `blood` maps to
`0xFFFF` before a query, and `0xFFFF` is the native aboard sentinel. Selector
`0x11` is consumed as a parent relation by the navigation tree: actor, arche,
and Orxx records use it as `current_location`; kind-`0x0400` inventory records
use it as `holder`.

`BC` adds one proven side effect: it publishes its raw RHS through
`gs:0x6782`. The presentation selector at `0x56FE` consumes that word to choose
a BAS case and stores it in the active actor's kind-2 selector-`0x0F` field.
Every shipped `BC` RHS resolves to a DIC word such as `talk`, `hello`, `rien`,
or `secrets`. BloodScript consequently names the field `topic` and interns the
quoted RHS through the profile dictionary. All 531 shipped `AF` operations
and 49 `BC` operations reproduce their original bytes.

Finally, opcode `A9` sets the native query bit just as an `A0` guard does. The
source formatter tracks that transition, so conditions immediately following
an `A9` are rendered as `require` expressions rather than updates. All 480
kind-2 DEB procedures begin at an `A9`: 420 carry flag byte `1` and 60 carry
flag byte `0`. BloodScript folds that state and skip target into
`proc name enabled|disabled { ... }`. Every non-final target equals the next
kind-2 entry, and all five final targets equal their script's sole `FF` halt.
The compiler derives those structural addresses; `until target` is retained
only as a lossless fallback for a non-structural A9. A top-level `A1`, when
present, becomes the procedure's `} then {` condition/body boundary. It is not
invented for procedures such as disabled `ERA`, where `A9` is followed by a
nested `A0` guard and has no separate root `A1`. Focused compiler tests pin both
shapes byte for byte.

The two RTC condition handlers are now fully lifted. `CA` reads an operator,
the otherwise ignored literal tag `C1`, and an hour; all 80 shipped instances
are ordinary `clock.hour` comparisons. `CB` reads an operator, day/month bytes,
and a year. Its four shipped operands decode to `1994-12-25`, `1994-01-01`, and
`1995-01-01`, matching their Christmas and New Year dialogue. Native routine
`0x6510` compares only day/month against `GS:0x0AA8/0x0AAA`; exhaustive native
reference search finds `GS:0x0AAC` written by the BIOS RTC loader but never read.
BloodScript therefore calls this value `annual_date` and still re-emits the
consumed source year exactly. The name records that the game compares only the
month and day; it does not attempt to repair the shipped behavior.

Seven control-flow encodings now have lossless typed statements: `GUARD_PUSH`
and `GUARD_POP` (`0xA0`/`0xA1`), `JUMP` (`0xA4`), `STATE_ARRAY_TEST` and
`STATE_ARRAY_SET` (the query/set forms of `0xA5`), `CONDITIONAL_BLOCK` (`0xA9`),
and the `0xCE`/`0xD0` context branches. This removes another 5,854 generic bytes.
Together the two lifts reduce generic coverage from 20,898 to 4,567 bytes, a
78.15 percent reduction, without changing any compiled COD byte.

All three context branches now have native domain names. Opcode `CE` tests
`GS:0x2793` bit 0, exactly the bit tested at the entry of the bridge renderer
`0x77E0`; contact and ship-presentation transitions clear the complete word and
their teardowns restore bit 0. Opcode `D0` tests `GS:0x252A`, whose only
set-to-one writer is the ship navigation coordinator at `0xB3F5`; completion,
record teardown, and HUD teardown clear it. Opcode `D1` tests `GS:0x274F`, whose
only set-to-one writer is the contact-scene transition at `0x18C4` after the
contact-menu handler has selected a target; the same transition clears it at
`0x1A48`. BloodScript consequently emits `during bridge`, `during travel`, and
`during contact`. The five shipped COD images contain 113, 224, and 65 of those
guards respectively, and all 402 recompile to their original one-byte opcodes.

`A5`'s state array is specifically a countdown bank for every shipped use.
Handler `0x65EB` sign-extends its byte index and addresses a word at
`GS:0x6ADE + index*2`. In update mode it consumes and stores the following
word. In query mode it consumes no value and calls the branch helper exactly
when the selected word is nonzero, so the source condition is expiry at zero.
The timer ISR at `0x0813` walks exactly 30 words from `GS:0x6ADE`, decrementing
only positive values when `GS:0x675A == 0`; zero and the negative class are
inert. Initialization at `0x53F6` fills the larger 256-word saved block with
`0xFFFF`. All 75 shipped `A5` instructions use indices `1..22`: 48 writes use a
positive count or `0xFFFF`, and 27 tests require zero. BloodScript therefore
uses `timer[n] = ticks`, `timer[n] = disabled`, and
`require timer[n] == 0`. No shipped profile requires a lower-level spelling.

`A7` is a one-topic offer, not an untyped presentation register. Handler
`0x67BA` consumes a word and, only while `GS:0x67AC & 1` marks a presentation
active, stores it at `GS:0x6770`. `vm_control_flow` calls collector `0x5AFD`
after the selected BAS body. That collector copies the current zero-terminated
`A3` menu into `GS:0x67F8`, appends the pending `0x6770` word when nonzero,
clears the pending slot, and writes the final terminator. The presentation
choice coordinator at `0x8963` consumes this resulting list. Every one of the
19 shipped A7 operands is an exact DIC offset (`sorceror`, `ekato`, `leisure`,
`gladis`, or `revelation` by text). Their surrounding dialogue uses them as the
additional selectable concepts. BloodScript emits `offer topic "word"` and
rejects an operand that does not resolve through the profile dictionary.

The final six native-handler families accounted for those remaining 4,567 bytes:
concept guards (`0xA3`), string loads (`0xA8`), procedure activation writes
(`0xAB`), sequence-slot bindings (`0xCC`), alternate-concept clears (`0xCF`),
and the contact-scene branch (`0xD1`). Every one of the 413 shipped `AB` writes
stores zero or one at exactly one byte after a named kind-2 procedure start,
which is the procedure's `A9` flag byte. The corpus contains 149 enables and 264
disables. BloodScript renders them as `procedure.enabled = true|false`; its
compiler restores the target address as `procedure + 1`. All 118,787 shipped COD bytes now compile from
typed statements with zero generic `OP` coverage.

Opcode `A3` is the choice condition used after dialogue menus. Native routine
`0x6596` compares its DIC operand with `GS:0x6762`, the clicked concept word, or
with the saved copy at `GS:0x6764` while `GS:0x67B1` bit 1 is active. An inline
`A1` before the operand inverts equality. The 239 direct and 80 inverted shipped
forms are therefore `require choice == "word"` and `require choice != "word"`.
Opcode `CF` at `0x64C0` clears the resume byte and saved choice, represented by
`choice = none`. The corpus contains 314 resets. Five guards intentionally lack
a procedure-local reset because their successful bodies end or hand off the
current presentation flow; the compiler does not add or remove resets.

Opcode `A8` has a complete resource-path interpretation. The handler at native
offset `0x67C8` copies its NUL-terminated operand to `SS:0x2120`, consumes one
pad byte, and conditionally stages presentation line 7 by writing
`GS:0x6788 = 7`, setting request bit `GS:0x67AA & 2`, and resetting the related
presentation fields. The static resource table at `DS:0x1FB5` stores a
four-byte `{descriptor,image_path}` entry for each line. Entry 7 points to
`DS:0x211B`, whose bytes begin `00 10 73 71 5C`: flags, variant, then `sq\`.
The mutable A8 destination starts at `DS:0x2120`, immediately after that prefix,
so line 7's complete resource filename is `sq\<A8 operand>`. The line dispatcher
marks line 7 for back-buffer drawing and passes it to `resource_load_sequence`.
The same A8 handler recognizes the exact lowercase prefix `fin.` and sets the
separate finale latch at `GS:0x67BD`.

Every one of the 89 shipped A8 operands is a basename ending in `.hnm`; there
are 36 unique names and the maximum length is 12 bytes. BloodScript therefore
uses `request sequence "name.hnm"` for the established high-level shape. Its
20-byte limit follows from the 21-byte writable region before the descriptor at
`DS:0x2135`, including the trailing NUL. Other possible A8 payloads remain
`load_string`, preserving their bytes without extending the proven semantics.

Opcode `CC` owns the six-slot DESCRIPT sequence playlist, not a generic
character table. Its handler copies a NUL-terminated operand into
`GS:0x6CDE + (slot-1)*16` and consumes one pad byte. The presentation-box driver
at `0x79E5` cycles selector `DS:0x27E3` through indices `0..5`; an empty slot
draws the noise fallback, while a nonempty slot is passed as the exact directory
key to `vm_c2_descript_lookup` at `0x7409`. That lookup opens `DESCRIPT.DES`,
matches the 16-byte directory name, and dispatches the record's media, subtitle,
sound, and music commands. It also places opcode-`0x0C` subtitle rows in a
128-byte page selected by the first character of slot 1, which proves that the
slot order is load-bearing rather than an unordered name set.

All 36 shipped CC instructions use slots `1..6`, contain at most 9 visible
bytes, and resolve to ten existing records; every referenced record has native
kind `Sequence`. BloodScript therefore emits
`sequence_slots[n] = "descriptor-name"`. The high-level form enforces the six
slots and the native 15-byte-plus-NUL capacity. An out-of-range slot or overlong
synthetic payload retains `character_slot` so no unproved shape gains playlist
semantics.

Selector `0x13` is the per-object action-record field. Native
`presentation_scan` at `0x5816` resolves that selector through the field matrix
for each active directory object, then passes any nonempty, nonnegative-valued
record to `record_c1_ship3d_action` at `0x5B38`. That routine dispatches on the
record's C1..CD kind, so `action` is established while a narrower name such as
`speaker` would be false for the non-presentation record kinds stored there.
The structured source has 115 unambiguous selector-`0x13` field aliases and now
names each one `object.action`.

C4, C3, and C9 establish one complete presentation lifecycle on those action
records. Every one of the 389 shipped C4 instructions runs while the VM query
bit is set, has no inline inversion, targets a selector-`0x13` field, and names
the built-in `blood` object as its related operand. Handler `0x6C7E` requires an
active owner and an exact `{C4,blood}` record, making
`require presentation == object` the direct source condition. All 35 C3
instructions run in update mode with the same field and related object; handler
`0x6EEE` writes `{C3,blood,1}`, which the post-VM presentation path consumes as
a scheduled presentation. BloodScript renders this as
`queue presentation object`.

All 371 C9 operands are selector-`0x13` action fields. Handler `0x6FB9` clears
the complete six-byte record. If its old kind was C4, it resolves selector
`0x13` on the related object and clears that reciprocal record too, while also
resetting sequence state. This is `end presentation object`. Eleven endings
occur in continuation procedures without a local C4 condition; their
presentations were established or queued earlier, so the syntax does not infer
an artificial local requirement. Nonmatching C3/C4/C9 forms retain their typed
low-level statements.

Opcode CD is a transfer rather than a three-word action assignment. In update
mode handler `0x69C7` threshold-resolves the first operand to its owning object,
uses the second operand as the moved object, and uses the third as its new
holder. It resolves selector `0x11` from the moved object's kind and writes the
new holder there. If the source owner is `blood`, it first removes the moved
object from the 16-word special-slot list. If the destination is `blood`, it
must insert the object into that list before writing the `0xFFFF` aboard
sentinel; a full list cancels the holder write. The first action-field operand
is not modified. A kind-`0x0400` transfer can additionally request active line
`0x2B` after a successful `descript.des` lookup and the native presentation
gates.

The corpus supplies a complete type proof for the readable form. All 182 CD
uses run in update mode without inversion and target selector-`0x13` action
fields. Every second operand resolves to a kind-`0x0400` DEB object, and every
source owner and destination resolves to either the built-in `blood` object or
a kind-2 character. Of these, 46 occur in COD and 136 in BAS. BloodScript maps
`blood` to the contextual holder name `aboard` and emits
`transfer ITEM from SOURCE to DESTINATION`; any other CD shape remains
`record_triple`.

C1, C2, and C6 have three separate proven source meanings in the shipped COD
corpus. All 20 C1 tokens run in update mode without inversion. Their first
operand is selector-`0x13` on the built-in kind-`0x0200` `orxx` object, and
their second operand is always a kind-`0x0080` sublocation. Handler `0x6B4C`
writes the C1 action; `record_c1_ship3d_action` at `0x5B38` consumes it by
changing `orxx`'s target and position and, while ship navigation is active,
selecting the new 3D target and presentation line 3. BloodScript therefore
emits `navigate to LOCATION`.

Both shipped C2 tokens also run in update mode without inversion. They target
`blood.action` and name a kind-2 character. Handler `0x6E34` requires the
character's dynamic portable bit, inserts it into the 16-word special-slot
list, writes `0xFFFF` to its selector-`0x11` current-location field, and stages
active line `0x27`. This is `bring CHARACTER aboard`; failure of a runtime gate
remains part of the native operation and does not change its source meaning.

The two C6 tokens are different: both run in query mode inside enabled A9
procedure blocks and compare `arche.action` with `{C6, Oddland, 0}`. Native
`nav_actor_handler_1` at `0x7EC0` is the producer: after the player completes
the kind-`0x0100` black-hole entry presentation it stages deferred type C6 and
the black-hole record. `presentation_scan` drains that state into the
kind-`0x0010` `arche.action` field, whose `0x5B38` consumer runs the multi-frame
camera transition. The script only observes that transition before requesting
the next profile, so the accurate form is `require travel through BLACK_HOLE`,
not an imperative travel command. Any C1/C2/C6 shape outside these exact
owner, field, kind, mode, and inversion constraints retains its typed low-level
form.

`B7` establishes a different, byte-addressed relation set. The handler at
`0x6AA7` adds `bit >> 3` to its base field and addresses the selected bit
high-bit-first. All three shipped tokens are non-inverted update-mode sets of
bit 2 on selector `0x05` of a kind-2 character: `Scruter_K` once and
`Bug_Deluxe` twice. In SCRIPT2 and SCRIPT3, DEB entry 2 is independently the
built-in kind-1 `blood` object. Helper `0x6210` scans that same 20-byte DEB
directory to turn an object offset into the bit index before testing selector
`0x05`/kind 2, proving that the bit number is an object-directory index rather
than an unnamed Boolean flag. Every initial kind-2 selector-`0x05` region in
all five VAR images is zero; these three statements are the only population
sites. BloodScript consequently emits `Character.known_objects += blood`. The
sequel executable independently retains the original compiler field name
`CONNAIS`, confirming the knowledge relation rather than merely suggesting it
from native behavior.

The two shipped pair records have a complete coordinate proof. Both are
opcode `BD`, both run in update mode, and both target `Kraner + 0x18`, which the
field matrix identifies as selector `0x0B` for initial kind `0x0010`. Native
`ship_3d_position_field_resolve` selects `0x0B` for direct kind-`0x0010`
positions; the distance, state-processing, HUD, and camera paths consume the
same two adjacent words as x/y. The values `(10, 10)` and `(100, 10)` bracket
the `krando20.hnm` race sequence and its completion. BloodScript renders these
as `Kraner.position = (10, 10)` and `(100, 10)`. The proof gate
does not lift query-mode pairs, `B8`/`B9`, another kind, or another selector.

The remaining 3,780 BAS bytes form 1,003 complete records: three one-topic menus,
19 offered-topic writes, three string loads, 37 `0xAA` yields, 321
`0xAC` yields, 321 linked selector nodes, and 299 shared state/record operations
and end markers. The sequential decoder now types all of them. Every node
selector resolves through its script's dictionary and every nonzero `next`
value points directly to another selector node. Runtime BASSTEP traces
independently establish `0xAC` as the selected response-body terminator. All
64,736 BAS bytes now compile with no `RAW` fallback.

The selector-node boundary is established statically, not inferred from the
corpus pattern. `vm_op_ac_yield` at executable file offset `0x685C` consumes no
operand. `vm_control_flow` starts the linked-list scan one byte after its saved
block pointer (`0x5715..0x5718`). `value_scan_match` at `0x577A` reads the node's
selector, assigns the second word directly to the scan cursor on mismatch, and
returns the node address plus four on match. Therefore `0xAC` and
`{selector,next}` are separate records and `next` is an ordinary zero-based BAS
offset.

The list roots are also recovered from data rather than guessed from layout.
`vm_cod_scan` resolves field selector 2 against each object's VAR kind and uses
the resulting word as a BAS offset. For kind `0x0002`, the executable's field
matrix maps selector 2 to object offset `+0x1A`. Restricting the DEB directory
to kind-1 object symbols yields 37 nonzero fields; all 37 point to the `AC`
immediately before one physical selector-list root, with no missing or interior
entry. Following those roots covers all 321 nodes exactly once.

BloodScript's structured BAS form therefore names lists after their owning DEB
objects and lists their cases in the exact linked order. `continues` marks a
case with a following node; the compiler derives the BAS next-node address and
the otherwise invisible `0xAC` prefix/terminator records. Ordinary `yield` and
`halt` remain visible because they change behavior. This is reconstructed source
notation, not a claim about the original token spelling, and all five BAS images
remain byte exact.

The whole ten-image corpus therefore compiles byte-for-byte from structured,
typed BloodScript 8. All 682 guards are balanced `when` regions, with only
behaviorally necessary nonlocal jumps and dialogue resume points left as named
labels.

Address correlation establishes three corpus-wide rules:

- all 480 kind-2 `.DEB` routine values are one-based COD addresses;
- 284 image-local distinct nonzero BAS targets are zero-based selector-node addresses;
- 1,054 image-local distinct explicit COD targets are zero-based COD addresses.

Every address resolves to a decoded token boundary under its respective rule.
The one-based rule applies only to `.DEB` routine values. Treating BAS `next`
values as one-based points at the preceding `0xAC`, not the node that the native
scanner reads.

BloodScript emits 480 named `proc` regions from the DEB names. Behaviorally
necessary COD jumps and dialogue resumes retain procedure-scoped role names;
internal guard and BAS selector addresses are compiler-derived and absent from
source. Its two-pass compiler derives addresses from source order without
exposing line offsets.

Kind-1 DEB entries also establish exact VAR object-base names. BloodScript 8
integrates those objects into the `state` section in DEB order. Logic uses their
derived identifiers for record relations and inventory transfers; source no
longer carries separate zero-byte `OBJECT` declarations or numeric offsets.

The native field-offset matrix plus each object's initial VAR kind establishes
367 unambiguous physical subrecord fields used by 1,880 operands. BloodScript
uses the sequel's retained field names directly as object properties; the
compiler derives each byte offset from the object's kind. The decompiler
requires one unique owner, excludes zero matrix entries, and gives exact object
bases priority over fields. Every shipped operand has a unique proven owner;
decompilation fails instead of emitting an unresolved numeric subrecord address.

Exact DIC string boundaries establish a second symbolic namespace. The corpus
uses 13,713 referenced offsets in 53,262 proven dictionary operands across COD
and BAS, including offered and actor topics. All shipped operands are readable
quoted literals. BloodScript 8 derives physical DIC order from `concepts`
followed by first use in `logic` and `conversations`. An inline
`dictionary blank after "word"` declaration preserves each otherwise invisible
empty entry. Dictionary literals are
still rejected in ordinary numeric or VAR-address operands.

The first structured COD pass recovers 7,010 basic blocks and 17,287 edges. It
models the native query bit and guard-target stack, direct jumps, text skips and
deferred frame resumes, and both possible states of self-modified `A9` block
flags. Every one of the 413 shipped `AB` writes targets an `A9` flag byte. All
branch-capable instructions resolve to a concrete guard target.
Exactly five block bodies are unreachable because their opener flag remains
zero; they are preserved rather than deleted from the source evidence.

The source-structuring pass converts all 682 `A0` guards into balanced
`when { ... } then { ... }` regions. Forty-four have the exact native if/else
shape: the final instruction of the true arm is `A4 <join>`, the false arm begins
at the `A0` target, and the forward join remains inside the same procedure. They
render as `} else {`, and the compiler recreates the original `A4`.

Five formerly rejected guards establish why CFG edges must remain visible even
inside structured source. `sort` retries through a backward jump. The `Corpo4`
and `big3` true arms perform navigation handoffs. `oto1` reaches the `oto2`
procedure boundary. The `tromp1` dialogue loop retains both its resume label and
its cross-boundary jump. Those labels and jumps remain explicit inside ordinary
`when` blocks. This preserves every nonlocal edge without exposing `GUARD_PUSH`
or `GUARD_POP`. The generated corpus has zero unstructured guards and all ten
sources reproduce the shipped bytes.

The earlier 443-region count was an analyzer artifact: destination membership
was tested against the destination basic-block leader instead of the exact
target instruction. A guard beginning in the middle of a block could therefore
appear to exit its own interval. CFG edges now retain both addresses; regression
tests distinguish this case from a real jump to an alternate exit.

## Historical boundary

The `.BAS` files are binary VM programs, not surviving BASIC text. No
QuickBASIC, BASCOM, BRUN, QBX, VBDOS, parser, compiler executable, source text,
or language manual is present in the shipped artifacts. The suffix proves only
that the original developers used the term `BAS`; it cannot prove historical
keywords or declaration spelling.

This is a provenance boundary, not an unresolved runtime behavior. Every
shipped COD and BAS token, operand, branch, procedure, state field, dictionary
entry, and directory record has a lossless BloodScript 8 representation. The
compiler accepts the reconstructed language only when all 25 generated VM
resources match the originals byte for byte. BloodScript syntax is deliberately
documented as reconstructed syntax rather than falsely presented as the exact
1994 source spelling.
