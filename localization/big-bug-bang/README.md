# Big Bug Bang English Display Text

`en/script1.json` contains an English editorial first pass for all 89 A6 text
sites in the opening COD profile. It includes non-spoken sites, unchanged sound
effects, and the three choice sections. The COD runtime backend now binds its
subtitle sections, inline menu prose, and choice labels for the matching opening
profile. The normal game loader now
accepts the verified BBB build, but complete gameplay is not established, so this
is not yet a playable English release.
`en/script2.json` now supplies all 1,197 text sites in the second COD profile
(985 unique sections), including 11 live-number menus. The modern runtime binds
this catalog to matching SCRIPT2 resources. Source-aware validation and runtime
catalog tests pass. The ordinary startup/PLAY capture
`output/big-bug-bang/english-script2-play-02` reaches profile 1 and visibly renders
"Go and look in the cryobox. Old Daddy is waiting for you there..."
(`screen-028.png`). This verifies the first SCRIPT2 subtitle, not every site.
That capture's subsequent TV sequence displays French text. Its first news line is
the frame-10 sequence subtitle in DESCRIPT's `1ppit` record, not text baked into
the video. The separate DESCRIPT display path is described below. Contextual review and
the remaining localization layers are unfinished.
The integration run passed six localization tests with original resources,
game-package all-targets checking, and 948 serial game-library tests (31 ignored).
Workspace-wide all-targets checking failed in the script-compiler test target
on unresolved shared-module imports; it is not a passing workspace gate.
`en/script3.json` supplies all 779 text sites in the third profile (663 unique
sections). It is bound to matching resources, including three live population
readouts, four inventory prompts, and one intentionally empty text site.
Inventory sections preserve their generator marker and receive no static choice
override: the live inventory remains authoritative. Item-name localization is
still separate work. All eight localization tests pass with original resources;
Broader SCRIPT3 contextual review remains incomplete; the verified initial
exchange is described under Live Profile Handoff below.
The rebuilt runtime capture `output/big-bug-bang/english-script3-play-01`
completes every action in `accuracy/scenarios/bbb_play_daddy.tsv`, including the
final `wait 100` (action index 15), and exits normally without the 360-second
capture limit firing. The final trace reaches resource profile 2 / SCRIPT3.
`screen-050.png` visibly shows the Daddy scene and its unchanged vocalization
"Ageu rha... Mmmeuh". This verifies that bounded gameplay route, not English
SCRIPT3 prose, later dialogue choices, or full-game progression.
Game-package all-targets checking and 949 serial game-library tests pass
(32 ignored) after this integration.
`en/script4.json` supplies all 660 text sites in the fourth profile
and is bound to matching resources. Source validation and all nine
localization tests pass, including four live-number readouts and three inventory
prompts. Game-package all-targets checking and 949 serial game-library tests pass
(33 ignored). SCRIPT4 live English rendering
and contextual review remain unverified.
The other 13 COD profiles, BAS text, most native UI, object names, and text embedded
in media remain untranslated.

## Options and Saves

Production `runtime/bridge_console.rs` now selects the recovered seven-row
`update_sequel_option_menu` for BBB and retains the five-row Commander handler
for Commander Blood. The two speed lists share the existing presentation-choice
widget. Simulation choices publish the authored reload values 100, 10, and 1
without resetting the current countdown; text speed remains independent.
Travel state feeds the C1 dispatcher, destination activation, and hyperjump
completion paths described below, not just its label.

The options, both speed lists, and bridge-list Cancel labels have English
display overrides. They apply only to decoded sequel controls and exact known
source labels; other text passes through unchanged. Original menu row identities,
speed values, and executable-decoded strings remain intact. The bridge's baked-in
console artwork and other native UI are not translated by this change.

Before this fix, BBB's seven labels were sent through the five-row handler:
Save dispatched Quit, Load and Quit had no action, simulation speed opened text
speed, text speed toggled music, travel dispatched Save, and music dispatched Load.

`accuracy/scenarios/bbb_play_options.tsv` reaches the interactive list from
ordinary startup input (resource profile 0). The capture
`output/big-bug-bang/options-before-fix-01` exits normally after action 7,
`wait 100`; `screen-017.png` visibly contains all seven rows and Cancel.
`bbb_play_options_save.tsv` selects SAUVER at logical `(100, 104)` after the
list opens. In `output/big-bug-bang/options-save-before-fix-01`, action 8 closes
the list and `screen-022.png` visibly shows the Quit path's "ARE YOU SURE?"
confirmation. The scenario does not accept that confirmation and exits normally
after action 9, `wait 50`, without reaching the capture timeout. This is a
reproduction of the old defect, not a passing save-game test. Replaying the Save
route in `options-save-after-fix-01` instead opens the slot editor and exits
normally; `screen-024.png` shows the empty first slot and Cancel.

The first combined replay, `options-roundtrip-01`, wrote a save but failed while
loading it: `NonContiguousStateObject { expected: 146, actual: 148 }`. Restore was
still decoding BBB VAR with Commander record sizes. Save-header decoding also
retained Commander's five-profile domain. Both restore paths now take the game's
dialect explicitly; BBB's seventeen-profile domain does not relax Commander's
domain or the transactional malformed-state checks.

The real-resource codec test changes a flag, restores the captured save, and
recaptures byte-identical data for **all 17 BBB profiles**. It also rejects an
invalid object kind without changing the loaded state. This tests the port's
capture/restore codec with real layouts, not original-DOS save interoperability
or seventeen independently reached gameplay states.

`accuracy/scenarios/bbb_play_options_roundtrip.tsv` completes in
`output/big-bug-bang/options-roundtrip-02` without timeout (`game_exit 0`). It
writes slot 0 named `ab` to a disposable writable root: `GAME1.SAV` is 9,008 bytes,
and `BLOOD.SAV` is 320 bytes. After action 20 loads that slot, the trace reports one
completed save and one completed load. These counters advance only after I/O
and the production restore call succeed. Later actions verify simulation reload
100, travel enabled, text delay 1, and Quit followed by No. The final action 45
finishes at step 2,159 with VM enabled, profile 0, save/load inactive, and all
pointer locks clear. Screens 017, 041, 050, and 055 show the English options,
simulation list, text list, and Quit confirmation respectively. The native hand
partly occludes the speed lists; its placement is not fixed here.

This is a live startup-slot round trip, not proof of saving Daddy's later
location, cross-profile gameplay restoration, or complete travel progression.
The final build also synchronizes the submenu phase mirrors and initializes the
menu's travel mirror from the decoded default; the capture predates those two
state-bookkeeping changes (the captured BBB default is false).

Verification passed: 954 serial game-library tests (37 ignored), all 11 bridge
console tests including the original-executable label check, both new save
checks including all 17 real profiles, and the three Rust travel checks covering
all 46 native branch vectors. Game-package all-targets checking also passes.

```sh
nix develop -c cargo test -p commander-blood-game --lib sequel_save -- --include-ignored --test-threads=1
nix develop -c cargo test -p commander-blood-game --lib runtime::bridge_console -- --include-ignored --test-threads=1
nix develop -c cargo test -p commander-blood-game --lib sequel_travel -- --test-threads=1
```

### Native Travel Gates

The hash-locked probe
`re/tools/big_bug_bang_travel_option_oracle.py` executes unmodified original
instructions and supplies 46 branch cases in
`re/tools/oracle_vectors/big_bug_bang_travel_options.jsonl`:

- At file `0x616B`, travel off skips C1 resource dispatch only for the current
  navigation record while the phase byte is below four.
- At `0x89C4`, travel off with a zero resource word skips palette preparation,
  ORs UI bit four, and sets line bit eight only when line bit two is clear.
- At `0x90B5`, travel on publishes C1; a matching target takes the bridge-reset
  branch, while other cases set the camera countdown to eight.
- At `0x90E8`, travel on takes the reset branch, and travel off retains the
  deferred action for the following completion path.

Two fresh executions passed and produced byte-identical fixtures. Instruction
and write bounds reject unexpected behavior, and executable/object bytes are
checked unchanged. These are **branch-entry** observations, not a complete native
travel run: prerequisites are seeded and execution stops before resource work,
palette preparation, bridge reset, and external calls. The standalone probe does
not establish a complete runtime journey or save/load round trip.

```sh
nix develop -c python -P re/tools/big_bug_bang_travel_option_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE /tmp/bbb-travel-options.jsonl
cmp re/tools/oracle_vectors/big_bug_bang_travel_options.jsonl /tmp/bbb-travel-options.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_options -- --test-threads=1
```

The oracle refuses an existing output file. It requires the locally supplied,
matching original executable; neither executable bytes nor game assets are
included in the fixtures.

## Timed Sequence Captions

`en/sequences.json` supplies 215 cues across 23 sequences: `present`, `1ppit`,
`3ppit`, `4exploplane`, `5exploplane`, `7croolvent`, `8incanthom`,
`9scrutbox`, `10hachoir`, `11izwalexplo`, `14legscrut`, `15parfum`, `16ondobar`,
`17vtromp`, `19chapeau`, `20larvarc`, `21pubdecod`, `24bionium`, `25diplom`,
`26explocomb`, `27exploplane`, `28bob`, and `31explonebul`, including
the original blank cues. The verified DESCRIPT contains 706 sequence subtitle
cues in total; the other 491 are not covered by this catalog.
The modern renderer selects English only for Big Bug Bang with the matching
DESCRIPT SHA-256 hash and an exact match for the complete source cue stream.
The source database, video/audio selection, cue ordering, frame thresholds, and
playback state are not rewritten. Modified resources and unlisted cue streams
retain their original captions.

The loader rejects missing records, cue count/frame mismatches, changed blank
cues, non-ASCII text, and ambiguous translations of identical source streams.
The caption layout test uses the original line planner to check screen bounds.
Source validation and layout checks do not establish contextual translation
quality or live rendering.
The rebuilt ordinary-input capture
`output/big-bug-bang/english-sequences-daddy-01/screen-031.png` visibly renders
"Terrible news has just come in over our teleprinters" on the TV news screen.
This verifies the first `1ppit` cue in-game, not all catalog cues. The four
additional sequences' 30 cues pass source and layout checks but have not been
verified in-game. Both sequence
localization tests and all 13 presentation-screen tests pass; the serial
game-library run passes 950 tests (34 ignored), and game-package all-targets
checking passes.
The extended scenario `accuracy/scenarios/bbb_play_daddy_english.tsv` completes
all actions and exits normally, but its two additional clicks at `(160, 20)` do
not advance Daddy's initial message. At action 19, SCRIPT3 still has execution
disabled on line 8, with the fully revealed, localized but unchanged vocalization.
The trace reports `scene_gate_active=true` and no active streamed clip. This is
the profile-handoff defect corrected below, not proof of English SCRIPT3 prose
or a successful conversation in that earlier run.

```sh
nix develop -c cargo test -p commander-blood-game --lib runtime::sequence_localization -- --include-ignored
```

## Live Profile Handoff

The ordinary SCRIPT2-to-SCRIPT3 handoff was discarding the active contact
coordinator. Text reveal and its hold timer finished, but no contact scene
dispatcher remained to complete the presentation and resume the VM.
Live BBB handoffs now retain this coordinator when both profiles use the same
VAR resource, rebinding its typed records by native VAR offset and record kind.
Explicit profile loading, including save restoration, still clears it.
The original profile-reset oracle now explicitly checks that native contact
record bytes `DS:29DB..29DC`, scene gate `DS:29DD`, and phase `DS:29DF` are not
written. All 512 profile-gate, five reset, and 17 post-load probes pass.

`output/big-bug-bang/english-daddy-handoff-01` completes the same extended
ordinary-input scenario with a normal exit and no capture timeout.
`screen-049.png` visibly shows the English question "would you like to try
talking to him before teleporting him?" and the English `yes` / `no` choices.
The frame trace also reaches both preceding English explanatory lines.
At action 19, SCRIPT3's VM is enabled, the dialogue selector is active, and
original selector words `oui` / `non` remain distinct from displayed labels.
This verifies the initial dialogue and choice display, not choice selection,
the rest of SCRIPT3, or full-game progression.

The real-resource SDL test confirms that a live handoff preserves contact state
and record bindings while an explicit load clears them. Game-package all-targets
checking passes, as do 950 serial game-library tests (35 ignored).

## Daddy's Yes Branch

`accuracy/scenarios/bbb_play_daddy_yes.tsv` selects the first dialogue's `yes`
at logical `(185, 90)` and waits through the resulting conversation. The capture
`output/big-bug-bang/english-daddy-yes-01` completes action 17 (`wait 200`) and
exits normally without a timeout. Its trace reaches "something about a war and
a bug... I'll check..." and the subsequent Petit Pit prank explanation. These
follow the source SCRIPT3 `oui` branch at COD offset `0x0C61`.
`screen-053.png` shows the taunting TV character; `screen-060.png` shows the next
English question, "that's right, isn't it, Commander?...". The final frame
has the VM enabled and awaits a new choice.

That capture exposed the literal labels `good-that` / `not-that` at site
`bbb.script3.cod.00000d6b`. They are now displayed as `right` / `wrong`, retaining
the original two choice identities and order. All nine COD localization tests
and source validation for all 779 SCRIPT3 sites pass after this correction;
the new labels have not yet been visually rechecked. The separate 30-cue DESCRIPT
addition was also made after starting this capture and is not live-verified by it.
Later dialogue branches, teleportation, and whole-game progression remain open.

## Daddy's Right-Label Replay

`accuracy/scenarios/bbb_play_daddy_right.tsv` extends the yes replay with a
first-row click at `(185, 90)` and another 200-tick wait. The capture
`output/big-bug-bang/english-daddy-right-01` exits normally without a timeout.
`screen-059.png` shows the corrected menu (partly covered by the native hand);
the trace records displayed `right` / `wrong` with original selector IDs
281 / 283. Action 18 closes that selector. At action 19, frame 4635, the VM
is enabled in profile 2 and awaits `annoyed` / `cool` at the English question
"are you annoyed, Commander?..." (`screen-071.png`).

This proves progression after the first-row click, not whole-encounter parity.
The trace reaches "and what's this, then?..." (COD `0x0DB1`), but does not
reach the `bien_ça`-guarded line at `0x0D8C`. The source choice spelling and
guard identity were then compared with the original handler, as described below.
No selector or guard semantics were changed.

The additional 27 timed captions for `9scrutbox`, `10hachoir`, and
`11izwalexplo` pass authentic-resource binding and original line-layout checks.
They were added after this capture started and are not live-verified by it.
The rebuilt executable includes all 94 cues. The serial library suite passes
950 tests with 35 ignored. Teleportation and whole-game progression remain open.

## Authored Daddy Guard Mismatch

The matching original SCRIPT3.DIC has `bien-ça` at offset `0x06F7`, while
the guard expects the distinct `bien_ça` entry at `0x0707`. The second menu
choice is `pas-ça` at `0x0700`; the next inverted guard expects `_ça` at
`0x073A`. Neither displayed choice equals either guard identity.

`re/tools/big_bug_bang_dialogue_guard_oracle.py` verifies the executable,
COD, and DIC hashes and source operands, then executes the unmodified sequel
A3 handler at file offset `0x6AB2`. It stops at the native guard-failure
callee (`0x697A`) or normal return. Twenty cases cover both authored guards,
both selected/alternate slots, both displayed choices, both guard identities,
and the empty selection. An inactive-slot poison confirms which native slot
is read. No callee or instruction is patched.

Both displayed choices fail the positive `bien_ça` guard and pass the inverted
`_ça` guard. This explains the captured `0x0DB1` line without changing the
authored resource or normalizing dictionary spellings. Two fresh oracle runs
agree byte-for-byte; the real-dictionary Rust regression matches all 20 active
concept results. This is a handler-level check, not a full DOS dialogue replay.

```sh
nix develop -c python -P re/tools/big_bug_bang_dialogue_guard_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/imported-assets/resources /tmp/bbb-daddy-guards.jsonl
nix develop -c cargo test -p commander-blood-game --lib \
  sequel_daddy_authored_choice_mismatch_matches_original_guards -- --include-ignored
```

## Daddy Tempest Progression

`accuracy/scenarios/bbb_play_daddy_tempest.tsv` continues the ordinary PLAY
route through `cool`, `enough`, and `tell_me`, then selects the first planet
row, `Tempest`, at logical `(185, 74)`. The capture
`output/big-bug-bang/english-daddy-tempest-01` completes all 27 actions and
exits normally without a capture timeout. Its action-boundary trace records:

| Action | English Prompt or Result |
| --- | --- |
| 21 | shall I tell him again, Commander?... (`repeat` / `enough`) |
| 23 | would you like to know what he's saying?... (`tell_me` / `don't_care`) |
| 25 | SHALL WE TELEPORT DADDY GLUXX TO: (`Tempest`, `Vulcan`, `Troma`, `Lovia`, `refuse`) |
| 26 | The Tempest click closes the planet selector. |
| 27 | Contact has ended and the bridge has returned. |

The live trace reaches the complete English Tempest teleportation message;
`screen-109.png` shows its reveal, and `screen-117.png` shows the returned
bridge. Final frame 8709 has profile 2's VM enabled, no active line, no active
actor/object presentation, no contact screen, and no input locks. This verifies
the dialogue route and return to the bridge, not Daddy's persistent location,
travel to Tempest, another contact, save/load persistence, or full-game play.

The 121 new captions across 13 additional broadcasts include the decoder
advertisement, time-gate/crown instructions, and loss messages. All 215 catalog
cues pass original-resource binding and original renderer line-layout tests.
The rebuilt executable includes them; this capture used the preceding 94-cue
build, so it does not visually verify the new broadcasts. The serial library
suite passes 950 tests with 36 ignored; the new ignored real-dictionary guard
test passes when explicitly enabled. Targeted Rust formatting checks pass;
the workspace-wide check still reports an unrelated existing `runtime.rs`
formatting difference, which is unchanged here.

## COD Catalog Validation

Validate against the user's original resources:

```sh
nix develop -c cargo run --bin sequel_text_catalog -- \
  output/big-bug-bang/imported-assets/resources \
  --validate localization/big-bug-bang/en/script1.json
```

Each message key retains its COD instruction address. Array elements correspond
to the source catalog's sections in order. The first is prose; later sections
retain one space-separated display label per original choice in its original
position. `PLAY INSTRUCTIONS` labels `JOUER EXPLICATIONS`, without replacing the
underlying dictionary IDs. Never use English display words for conditions,
history, audio hashes, or dictionary lookups. Source COD and DIC hashes bind the
file to the specific authored resources, not to a similar-looking script.

V1 uses printable ASCII for the existing font path. Dynamic markers retain
their original order and spelling (`<state:N>` and `<inventory_choices>`).
The validator checks structural compatibility, not translation quality, choice
meaning, rendered width, reachability, or gameplay.

The runtime menu path now parses standalone `<state:N>` words into typed live
number references. It requires the same ordered references as the original
prose section, rejecting missing, added, reordered, or malformed markers. The
renderer reads each reached number from current VAR state as a signed 16-bit
value. Lookahead retains the native previous-number scratch value instead of
reading the next number early. Original words, word counts, and VAR are not
rewritten. Numeric overrides are restricted to non-spoken menu text; inventory
generators in the sole post-prose choice section remain live while their prompt
is translated. Mixed generator/static choice sections are not supported.

This support is used by the 11 numeric menu sites in SCRIPT2. Tests cover
signed limits, live changes, state preservation, marker-source validation, and
the existing original numeric-renderer vectors.

Editorial choices: Monsieur Bob becomes Mr. Bob; Biorédactrice becomes
bio-editor; proper names and invented terms such as GLUXX and BIONIUM remain.
The deliberately split `A DIEU` becomes `TO GOD` to preserve the farewell pun.
Bob's unusual phrase at `00000c2c` is translated literally pending contextual
review with the recording; it has not been silently corrected into a different
French sentence. English voice acting is not supplied.

## Subtitle and Menu Integration

The COD dispatcher requests a display override only after `SubtitlePublished`.
The backend substitutes section zero, wrapped at 34 columns with the existing
carriage-return line format. Native/reference hosts default to original text;
the modern runtime binds English catalogs for BBB SCRIPT1 through SCRIPT4
with matching COD and DIC SHA-256 hashes. Other profiles, modified resources,
and missing sites retain their original text. Binding another profile clears
the old translation. Original dictionary IDs and menu words are never replaced.

The retained choice renderer uses the last accepted A6 instruction as its display
source. English labels are bound to that instruction and the exact ordered source
dictionary IDs; missing sites, reordered choices, inventory lists, and unmatched
resources keep their original labels. Rejected A6 calls do not change the source,
and a profile reset clears it. Width measurement and drawing use the translated
labels, while choice completion retains the original dictionary identity.

There are 34 authored spoken-flag sites in the opening profile. Inline menu prose
(including Bob's recording and OLGA's dialogue) now uses English display words
when its complete authored word stream matches the accepted instruction. The
shared menu layout measures and wraps those words and completes at their own
word count. Authored words, number operands, dictionary IDs, and chatter inputs
remain untouched. Inventory and unmatched streams retain their original text.
The trace keeps original `words`/`word_ids` separately from `display_words`;
localized revealed words have null dictionary IDs rather than invented ones.

The three dictionary-choice sections are bound to English display labels. Only
the opening HONK menu has been reached and visually checked in the live runtime;
no live Bob/OLGA menu-prose reachability is claimed.

Verification commands:

```sh
nix develop -c cargo test -p commander-blood-game --lib runtime::localization -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib english_subtitle -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib english_inline_menu_binding -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib menu_reveal -- --include-ignored
```

These exercise source binding, hash-mismatch fallback, original-font rasterization
of all 89 translated prose sections, and reveal completion at the translated
length. An isolated call through the real A6 dispatcher compares original and
English outputs: only subtitle bytes differ; selector, choice, VAR, VM, and other
dispatch state remain identical. Gated and menu-only calls do not invoke the
subtitle hook; accepted menu prose binds separately at the renderer. All 89
English prose sections also pass complete inline-font raster and screen-bound
checks. A focused reveal test verifies translated completion timing and unchanged
source IDs; an original-resource binding test checks stale-stream/profile fallback.
The fixture explicitly prepares the actor-presentation gate; it is not evidence
of reaching Honk from a new game. The earlier ordinary-pointer PLAY run
`output/big-bug-bang/modern-honk-play-02` failed during the shared-VAR transition
at byte 8368. That loader rejection is now resolved using the native-captured
read-only SCRIPT2.DEB prefix binding, without extending VAR. The repeated run
`modern-honk-play-04` loads SCRIPT2 immediately after PLAY and reaches new French
dialogue. Translation bindings correctly stop at the profile boundary rather
than applying SCRIPT1 text to SCRIPT2 addresses. Starting SCRIPT2 without the
preceding persistent state is still rejected. The attempted cryobox sequence
has not yet reached Bob, so its translation remains without a live visual check.

The game library regression suite passed 940 tests (25 ignored) with
`--test-threads=1`. A parallel rerun terminated with SIGSEGV; its core dump placed
the crashing stack in Vulkan `loader_get_icd_and_device` /
`SetDebugUtilsObjectNameEXT` while
`render::tests::srgb_artwork_and_overlay_match_every_cpu_expanded_dac_level`
created a pipeline layout. The root cause is not established or fixed here;
the serial pass does not prove parallel GPU-test stability. Workspace checking
and the five targeted localization/dispatch checks passed.

Remaining work includes BAS/UI translation,
the other COD profiles, contextual editorial review, and actual playable startup
and progression. No English voice acting or whole-game localization is claimed.

The menu-prose integration regression run passed 942 library tests serially
(29 ignored), all three menu-reveal checks, four localization checks, and the
original-resource runtime binding test. All-targets checking passed. These do
not establish complete gameplay or live reachability of every translated menu.
