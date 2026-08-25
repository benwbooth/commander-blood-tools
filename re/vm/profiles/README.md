# Unified BloodScript profiles

These five `bloodscript 8` files are the canonical, editable source for all 25
shipped VM resources. Each `scriptN.blood` compiles to `SCRIPTN.COD`, `.BAS`,
`.DEB`, `.DIC`, and `.VAR`; no companion source file is required.

BloodScript derives binary layout from readable source order:

- `state` lists typed objects in DEB order. Fixed names, record storage, action
  slots, BAS roots, and proven reserved padding are compiler-derived.
- Editable object values use semantic names such as status, race, population,
  location, topic, position, universe, parent, holder, visits, and statistics.
- `global` declarations stay beside their logic and preserve both VAR allocation
  order and DEB symbol order without a separate globals table.
- `logic` procedures and labels retain the rest of DEB order automatically.
- `conversations` and `logic` intern dialogue and concept words on first use.
  Rare `dictionary blank after` declarations sit at that first use and preserve
  the original DIC padding without a dictionary table.
- Conversation cases appear in linked order. `continues` means another case
  follows; BAS node addresses and `0xAC` link terminators are compiler-derived.

Dialogue uses semantic controls. `presentation=bobb.hnm` names the exact talk-HNM
entry selected for that actor, while `presentation=text_only` selects the native
no-talk-HNM channel. The compiler derives the ordered table selector and its biased
active-line ID; numeric presentation IDs are rejected. Two HNM names occur twice
in their actor tables, so the later entries use an explicit `@2` suffix to retain
their distinct table positions. The hidden Ulikan/Cyberion Junior presentation has
semantic clock names such as `afternoon_signoff` and `early_morning_signoff` because
it has no DESCRIPT character record. `chatter`, `repeatable`, `chance=20%`,
`if_not_shown skip_next=N`,
`resume_at=LABEL`, and
`when aggressiveness == N` lower to the recovered TEXT control bits.
`if_not_shown` advances over the stated number of following VM statements only
when that line is rejected; `resume_at` names the continuation used after a
choice. The rare `when last_8_choices count WORD >= N` predicate counts the
line's choice word in the native eight-entry recent-choice ring.
`choices` represents the native phrase/choice separator. A `|` inside quoted
text preserves a no-space dictionary-token boundary when spelling alone would
be ambiguous.

Boolean state uses ordinary assignments and `require`. The equivalent
`mark OBJECT as STATE` and `check OBJECT is STATE` spellings preserve the
second execution-identical opcode emitted by the original compiler. Recurring
calendar tests use `annual_date`; the game compares month and day while the
compiler preserves the source year stored in the bytecode.

Object properties, variables, counters, and coordinates use decimal values.
CP437 text is written normally where representable, with a lossless `\xNN`
escape only for bytes that have no source character.

Generate canonical source from an extracted game:

```sh
cargo run --bin cbvm -- decompile-unified /path/to/game re/vm/profiles
```

Compile and compare the complete bundle against the shipped resources:

```sh
cargo run --bin cbvm -- compile-bundle \
  re/vm/profiles /path/to/game /tmp/cbvm-bundle
```

The decompiler recompiles its output internally and rejects any non-byte-exact
profile. The bundle compiler independently compares all 317,835 emitted bytes
against the installed game before writing its manifest.

The original field matrix and native consumers establish record offsets. The
sequel's retained compiler tables independently name the fields, flags, and 16
race values. Decompilation fails instead of emitting an opaque word when a
shipped record violates this typed schema.

Two invisible layout details are also evidence-backed. Character gaps at byte
28 and bytes 64 through 67 are reserved padding: they are outside the field
matrix, have no BLOODPRG consumer, and are zero in all 593 inspected character
records from Commander Blood and the available sequel profiles. The `orxx`
navigation controller is 36 bytes; bytes 18 through 35 are likewise inaccessible
zero padding. A separate zero word named `tblood` follows `orxx`. It is a
compiler-injected DEB state symbol in every inspected profile, but neither game's
VM program nor BLOODPRG reads it, so BloodScript derives it instead of exposing
an apparently meaningful editable variable.
