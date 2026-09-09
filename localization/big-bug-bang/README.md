# Big Bug Bang English Display Text

Latest SCRIPT4 coverage: Izwalito's earned Help conversation reaches and saves
the phone-number dialogue. Fresh recontact now displays its live population
in English after recovery of the inherited spoken-number path. Yes continues
through the Olga conversation to the rendered peace-treaty offer; No follows
the original authored Bob ending, not ordinary bridge navigation. The Help
prompt and Pierrette pronouns were corrected against the French source.
See `../../docs/big-bug-bang-port.md` for artifacts, native-handler evidence,
and the remaining unverified progression. This is not full-game completion.

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
`en/script5.json` supplies all 216 text sites in the fifth profile, covering
Eviscerator and Outrageor dialogue, threats and imprisonment, trades, gift
responses, and time-gate/crown instructions. This is an English editorial first
pass, not a live-scene fidelity claim. The runtime binds it only to the exact
SCRIPT5 COD/DIC hashes. Two population readouts preserve their state-number
sources, both GIVE sections preserve inventory generation, and all seven
concept-choice lists retain original dictionary IDs and positional ordering.

Source validation checks all 216 sites against the imported originals. The
real-resource test loads the initial persistent state before selecting SCRIPT5,
verifies the catalog route without modifying COD/DIC data, checks dynamic and
choice ownership, and renders all 214 static subtitles through the RGB UI font
path with nonblank-pixel and horizontal/vertical bounds assertions. Mismatched
COD/DIC resources and wrong profile tags are rejected. All ten localization
tests pass when explicitly including original-resource tests; the enabled game
library suite passes 964 tests (39 ignored), and game-package all-targets
checking passes. Live SCRIPT5 reachability, voice timing, scene composition,
and contextual review remain unverified.

`en/script17.json` supplies all 132 Cyberquizz/Bioquizz text sites, bound to
the original SCRIPT17 COD/DIC hashes. It preserves two live population values,
two inventory generators, and five choice lists, including the repeated yes/yes
and single no choices. The original-resource test selects the profile after
loading initial persistent state, checks unchanged code and dictionary bytes,
and renders all 130 static subtitles through the original font's RGB path,
checking nonblank output and horizontal/vertical bounds. All 11 localization
tests pass with original resources. This is an editorial first pass; live
SCRIPT17 progression, voice timing, and scene composition remain unverified.

`en/script15.json` supplies all 102 Scruter Mac/Jo/K text sites, with matching
original COD/DIC hashes. Three population readouts preserve their live state
offsets, and three inventory prompts retain the inventory generator without
static choice overrides. The original-resource runtime test loads SCRIPT15
after initial persistent state, verifies unchanged code/dictionary bytes, and
renders all 99 static subtitles through the original font's RGB path, checking
nonblank output and bounds. All 12 localization tests pass with original
resources. Live SCRIPT15 progression, scene composition, and voice timing are
not verified by these tests; this remains an editorial first pass.

`en/script7.json` supplies all 140 Betakam/Alphakam/Gammakam text sites,
including strike messages, mummy transfers, three live population readouts,
three inventory generators, and three concept-choice sections. Betakam's
single accept choice is preserved; the other two accept/refuse lists retain
their original ordered dictionary IDs. The original-resource test selects the
profile after initial state loading and renders all 137 static subtitles through
the RGB font path with nonblank and bounds checks. All 13 localization tests
pass with original resources. This is an editorial first pass, not a live
verification of Kam dialogue, mummy transfer, or subsequent progression.

`en/script9.json` supplies all 176 Bug Deluxe/Sinox text sites, including the
rescue, technological secrets, and time-reversal dialogue. Three population
readouts and three inventory generators remain live; three choice lists retain
their original ordered dictionary IDs. The original-resource runtime test loads
SCRIPT9 after initial state loading and renders all 173 static subtitles with
the original font's RGB path, checking nonblank output and bounds. All 14
localization tests pass with original resources. This editorial first pass does
not verify live rescue, attack, time-reversal, or subsequent gameplay branches.
Game-package all-targets checking and the executable build pass. The serial
game-library run passes 964 tests (43 ignored). The preceding default-parallel
run terminated with SIGSEGV without identifying a failing test; its cause remains
unresolved at that checkpoint. The serial pass does not establish parallel-suite
stability. Subsequent core analysis identified concurrent Vulkan driver unloading
and pipeline construction; see [GPU test isolation](../../docs/gpu-test-isolation.md)
for evidence and the test-harness mitigation.

`en/script16.json` supplies all 209 Tubular Brain text sites, including the
philosophy quiz, Bionium clairvoyance exchange, Croolis/Scruter dialogue, and
writing/culture trades. Seven choice lists retain their original ordered IDs;
the to-be/or/not-to-be prompt remains three choices. Three population readouts
and three inventory generators remain live. The original-resource runtime test
loads SCRIPT16 after initial state loading and renders all 206 static subtitles
through the original font's RGB path with nonblank and bounds checks. All 15
localization tests pass with original resources. This is an editorial first pass;
live quiz, trades, synthesis, voice timing, and progression remain unverified.

`en/script8.json` supplies all 288 robot text sites across the Toolbox, Khan,
and Oil contacts. Nine population readouts and nine inventory generators remain
live; three trade-choice lists preserve their ordered dictionary IDs. Morning_Oil's
authored decoder caption after the gift offer is retained explicitly, not silently
changed to match the offer. The original-resource runtime test selects SCRIPT8
after initial state loading and renders all 279 static subtitles with the original
font's RGB path, checking nonblank output and bounds. All 16 localization tests
pass with original resources. This editorial first pass does not verify live
robot conversations, trades, voice timing, or later gameplay progression.

`en/script12.json` supplies all 302 Migrator/Mig Burner text sites, including
illness and recovery, ship/address rewards, ship trades, concert dialogue, and
the Scruter-body sale. Eight choice lists preserve ordered dictionary IDs; two
population readouts and two inventory generators remain live. The original-resource
runtime test selects SCRIPT12 after initial state loading and renders all 300
static subtitles through the original font's RGB path, checking nonblank output
and bounds. All 17 localization tests pass with original resources. This is an
editorial first pass; live recovery/death branches, trades, concert progression,
and voice timing remain unverified.

`en/script10.json` supplies all 303 Blue Wave text sites, including the ring and
energy exchanges, cryobox transfer, and performance/replay choices. Eight concept
lists retain their original ordered dictionary IDs, and the inventory prompt
retains its live generator. The original-resource runtime test selects SCRIPT10
after initial state loading and renders all 303 static subtitles through the
original font's RGB path, checking nonblank output and bounds. All 18 localization
tests pass with original resources. This is an editorial first pass; live ring
and energy conditions, transfers, performances, and voice timing remain unverified.

`en/script14.json` supplies all 377 Trump/Tramp/Super Tromp text sites, including
the informer letter puzzle, migration hints, concert dialogue, magic spells,
and debugging requests. Ten choice lists preserve original ordered IDs. The
three selectable letter sequences remain verbatim, including duplicate letters;
they are puzzle inputs, not prose to translate. Three population readouts and
three inventory generators remain live. The original-resource runtime test
selects SCRIPT14 after initial state loading and renders all 374 static subtitles
through the original font's RGB path with nonblank and bounds checks. All 19
localization tests pass with original resources. This editorial first pass does
not verify live puzzle answers, migration, trades, debugging, or voice timing.

`en/script6.json` supplies all 543 green Crooli text sites for Chigraxx/Emasculator
and Sergeant Rotator. It includes nuclear-item trades, strike dialogue, jokes,
and gift reactions. Eighteen choice lists preserve ordered dictionary IDs;
population fields 244 and 392 and both inventory generators remain live.
Source validation passes all 543 sites. The original-resource runtime test loads
SCRIPT6 after initial state setup and renders all 541 static subtitles using the
original font's RGB path, checking nonblank output and bounds. All 21 localization
tests pass with the original resources. This is an English editorial first pass,
not live verification of its trades, destructive joke branches, or voice timing.

`en/script11.json` supplies all 633 Zen text sites, covering Super Zen, Kero Zen,
Ben Zen, cloning, medicine and guitar trades, and the ring quest. All 19 static
choice lists retain ordered dictionary IDs; population fields 2464, 2538, and
2612 and three inventory generators remain live. Source validation passes all
633 sites. The original-resource runtime test loads SCRIPT11 after initial state
setup and renders its 630 static subtitles through the original font's RGB path,
checking nonblank output and bounds. All 23 localization tests pass with original
resources. Live Zen progression, trades, telepathy, and voice timing remain
unverified; this is an editorial first pass.

The cave puzzle is localized from French IDEES to English IDEAS. Its third
letter's accented E is displayed as `e`, and the unaccented-E distractor becomes
`o` to avoid two visually identical choices. In the fourth list, the accepted E
is displayed as `a` and the two original A distractors become `e`. The test
`authentic_script11_english_ideas_matches_original_puzzle_guards` decodes the
original concept guards at COD offsets `0x19C9`, `0x1A28`, `0x1A8B`, `0x1AEC`,
and `0x1B3D`. It verifies exactly one matching English letter per list and that
each maps to the original accepted dictionary ID. Neither script bytes nor
branch logic change. This is source-bound branch verification, not a live puzzle
completion capture.

`en/script13.json` supplies all 775 Slimer text sites, covering Otto Von Smile,
Von Gluk, and Von Potato, the ship bargaining routes, gifts, and destructive
comedy branches. All 26 static choice lists retain their ordered dictionary IDs.
Population fields 3352, 3426, and 3500, the two live ship-price sites using field
7922, and three inventory generators remain live. Source validation passes all
775 sites. The original-resource runtime test loads SCRIPT13 after initial state
setup and renders its 770 static subtitles through the original font's RGB path,
checking nonblank output and bounds. All 24 localization tests pass with original
resources. The authored bad bargains, references to Von Smile in other Slimers'
gift responses, and original phone-number joke are retained. This is an editorial
first pass, not live verification of the ship trades or destructive branches.

English COD coverage is now all 17 profiles, or 6,921 of 6,921 text sites.
BAS text, most native UI,
object names outside the inventory chooser, and text embedded
in media remain untranslated.

## Location Captions and Separate BAS

The 75 opcode-05 DESCRIPT captions were inventoried separately from the timed
sequence cues. Most are proper names; three are blank. The display layer now
changes `Arche:` to `Ark:` and the French Ekatomb sentence to
`Ekatomb: we'll all end up here...`. Original lookup names, carriage returns,
and the rest of the text presentation state are preserved. Binding requires the
known DESCRIPT hash and exact source captions. An original-resource test renders
all 75 captions with the original RGB font, checks the two replacements and
unchanged presentation state, and rejects wrong-game, changed-file, unrelated-name,
and changed-caption matches. The BBB bootstrap test also applies both descriptions
through `RuntimeScriptBackend` with actual resource loaders. These are not live
captures of visiting Ekatomb.

`sequel_bas_catalog` inventories separate BAS files using the same typed decoder
as the runtime, keeping dictionary offsets and original source bytes. On the
imported BBB resources it fails at `SCRIPT2.BAS` byte 6: the first menu references
DIC offset `0x1F00`, which is inside `Private` (`0x1EFA..0x1F01`), not a word start.
The only separate BAS file is 19,933 bytes, SHA-256
`3e2b4a6d7c26aca6be2f88b3b539972b655ab1423907bbf75fb0931717bb5314`.
Its paired DIC is 15,369 bytes, SHA-256
`1666ae7bb0dead682f9c3fd64b5ea6b71999475beb77c85ad75e79a0ee71a5b0`.
Both match the import manifest and the extracted disc copies. The resource-backed diagnostic regression pins
that rejection and the raw word boundary. This does not prove the BAS file is
unused: native reachability and resource ownership still need investigation.
No empty BAS replacement, altered dictionary identity, or speculative BAS
translation was introduced.

The follow-up `big_bug_bang_bas_entry_oracle.py` executes the original handoff
gate at file offsets `0x5E0D..0x5E69`, including its unchanged field resolver at
`0x6633`, and stops before the BAS dispatcher call at `0x5E66`. Its 512 cases vary
the presentation gates, reciprocal actor actions, blocked flag, and zero/nonzero
entry offsets. The Rust handoff matches every case; a repeat capture is identical.
The original field matrix selects actor byte 26, and all 1,037 actor records in
the 17 shipped VAR profiles initialize that word to zero. Thus this gate does
not enter BAS from those initial records, even with the other gates open. This
does not yet prove that later script writes or loaded saves cannot change the
word, nor that other BAS-reading paths are unreachable. No runtime gate was
removed or weakened. The resource-enabled `sequel_` test selection passes all
79 tests, including the new gate and initial-record checks.

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

### Persistent Location Tracing

Semantic snapshots now include `persistent.object_locations`, resolved from the
same synchronized VAR state used for saving and hashing. Each entry records its
object ID/name/kind, raw holder word, and a typed target ID/name where resolvable.
Sentinels and unresolved words remain explicit rather than being inferred as a
location. The existing hashes retain their previous meaning.

`accuracy/scenarios/bbb_play_daddy_tempest_save_load.tsv` extends the ordinary
Daddy dialogue route with the options menu's Save and Load commands in a
disposable writable directory. Its intended assertion is that the Gluxx family's
later location survives the round trip; the scenario alone is not passing
evidence. In the initial live state, Daddy points to `Trashlando`, while
`Templand` points to the celestial object `Tempest`.

The replay completed in `output/big-bug-bang/daddy-tempest-save-load-01` with
`timed_out False` and `game_exit 0`. After action 27, all three Gluxx family
members point to `Templand` (raw VAR offset `0x12F0`). Action 38 saves slot 0
named `ab`; action 44 loads it. The final action 47 retains those three locations
in profile 2, with one completed save, one completed load, inactive save/load UI,
and all pointer locks clear. Its game-frame sequence is 9,191.

The resulting `GAME1.SAV` is 9,044 bytes with profile header 2, and `BLOOD.SAV`
is 320 bytes. Independently reading the saved VAR block after its 610-byte fixed
header confirms holder word `0x12F0` at actor records `0xC24`, `0xC6E`, and
`0xCB8`, matching Daddy, Mamy, and Papy in `SCRIPT3.DEB`.
This verifies a progressed gameplay save/load in the same process, not a fresh
process restore, original-DOS save compatibility, or travel/recontact on Tempest.

Verification for the trace change: 954 serial library tests passed (37 ignored),
the explicitly enabled real-asset profile-handoff test passed with named-location,
raw-holder, hash, and nonmutation assertions, and the game build and all-targets
check passed.

### Fresh Load and Tempest Navigation

The disposable captures `daddy-fresh-load-camera-01` through `03` copy only
`BLOOD.SAV` and `GAME1.SAV` from the progressed save above into a new writable
root before starting a new process. Ordinary Load changes the active profile
from 0 to 2 and restores all three Gluxx family holders to `Templand`.
The first two input probes did not open the chart: one parked at the wrong
station, and the other clicked above the camera's measured hit rectangle.

The corrected `daddy-fresh-load-camera-03` run parks at bridge frame 0, clicks
the camera at `(150,150)`, selects the Tempest chart marker at `(16,110)`, and
clicks the right-hand hyperjump control at `(250,125)`. It exits normally without
timeout. Arche's holder changes to Tempest (record 72, raw `0x12D2`); the
navigation status view renders Tempest and lists Daddy Gluxx there. This run
stops before planet entry or recontact. Its `screen-041.png` still shows the
source French labels, because it predates the display change below.

Navigation status and information panels now override the four exact decoded
BBB labels with `PLANET: `, `SHIP: `, `BLACK HOLE: `, and `LIFE FORMS:`. The
source executable bytes remain unchanged; other strings and Commander Blood
pass through unchanged. Both focused label tests pass, including a check against
the imported original executable. The serial library suite passes 955 tests
(38 ignored), and the game build and all-targets check pass.

`accuracy/scenarios/bbb_load_daddy_tempest.tsv` runs on the rebuilt binary in
`output/big-bug-bang/daddy-fresh-load-tempest-04` and exits without timeout.
`screen-036.png` visibly shows `PLANET: Tempest`, `LIFE FORMS:`, and Daddy Gluxx
in the chart information panel. The final status subtitle is
`PLANET: Tempest LIFE FORMS: Daddy_Gluxx`.
With the default Travel Off setting, the click inside the planet status region
produces `DestinationUnavailable` at live frame 2,851. No local ship-HUD
coordinator is created. Final action 21 remains at Tempest in profile 2 with
all pointer locks clear. This is evidence for navigation and English display,
not successful planet entry. The separate
`accuracy/scenarios/bbb_load_daddy_tempest_travel_on.tsv` uses the options menu
to enable the native unvisited-destination branch before hyperjumping.

That Travel On scenario completes in `daddy-fresh-load-tempest-05` without
timeout. Final action 23 has Travel enabled, an initialized ship-HUD coordinator
at Tempest, and presentable targets `[72, 65, 73]` (Tempest, Arche, Templand).
`PL\tempet10.hnm` is the active scene resource. `screen-046.png` shows the
planet view and all three choices plus Cancel. The selector has completed its
ten-step opening transition and owns the modal UI while awaiting a choice.
This verifies destination entry; selecting Templand and recontacting Daddy are
separate steps.

`accuracy/scenarios/bbb_load_daddy_templand.tsv` extends that route with a click
on Templand's measured third row at `(80,104)`. The capture
`daddy-fresh-load-templand-06` also exits normally without timeout. Final action
25 has local target 73 (Templand), active actor and description 44 (Daddy Gluxx),
the `tempet1*.lbm` backgrounds, and the English subtitle
`ha ha ha!... A murfalo!...`. `screen-058.png` visibly shows that caption over
the local scenery. The final game-frame sequence is 3,567 in profile 2.

This establishes scene entry, not completion of the renewed conversation.
At the final boundary, presentation line 7 is active, its presentation gate is
1, and no next choice is visible. The remaining dialogue continuation and
broader game progression still need verification; this is not a complete
playable-English release.

### Templand Interlude Completion

The extended baseline `daddy-fresh-load-templand-08` reproduced repeated
`SQ\\venus06.hnm` playback. Clearing the sequel lifecycle request alone was
insufficient: capture `output/fidelity/bbb-templand-dialogue-1788887781723898709-3431099-0`
still reopened the clip. At frame 5609 its source closed, but the ship active
line remained 7 and request flags remained 2; frame 5610 reopened the source.

The scene dispatcher clears the ship-owned line before navigation runs. The
navigation adapter had imported the older lifecycle line and text-owned request
flags, undoing completion. It now reads the ship-owned line and current lifecycle
request flags. The sequel completion handoff also publishes the native-cleared
secondary request and scene gate before resuming the VM. All 38 unchanged-binary
scene-completion vectors regenerate byte-identically; the Rust oracle test now
checks lifecycle request/gate ownership as well as VM resumption.

Production regression `templand_interlude_returns_to_dialogue_and_choices` in
`crates/commander-blood-game/tests/bbb_progression.rs` passed in 531.09 seconds.
Its retained capture is
`output/fidelity/bbb-templand-dialogue-1788888606946814893-3447994-0`.
At frame 3365, dialogue advances to "With that boiled-shank face"; subsequent
speech reaches "am I boring you, Commander? Are you in a hurry to finish?...".
The chooser reaches `Selecting` with `finish` and `no_hurry`. The test checks
this semantic progression, not visible pixels or successful choice selection.
`dialogue-continuation.png` visibly confirms later authored speech.
However, `choices.png` shows the question and hand **without visible choice
labels**. Choice rendering/interaction remains the next blocker; the trace field
`rendered_word_choices` actually reports retained labels, not proof of drawing.
The frame-ready gate around choice updates and transient UI clearing require
further verification. Neither menu usability nor the rest of this conversation
is established by this passing regression.

Run with a graphical display and imported BBB assets:

```sh
nix develop -c cargo test -p commander-blood-game --test bbb_progression \
  templand_interlude_returns_to_dialogue_and_choices -- --ignored --exact --nocapture
```

`BBB_ASSET_CACHE` and `BBB_PROGRESSED_SAVE_DIR` override the default local asset
and progressed-save paths. The test copies the seed saves to a fresh disposable
directory and retains input hashes, initial saves, logs, and frame traces.
Game-package all-targets checking and 963 enabled game-library tests pass (38
ignored). Workspace-wide all-targets checking still fails in the shared
script-compiler test imports; that is not a passing gate.

### Templand Choice Visibility and Selection

The missing labels in the preceding capture were not a font or color problem.
The native scheduler at file `0x12EA..0x131D` uses the retained queue entry/read
counters (`0x0FFD` / `0x0FAE`) to restart the character idle sequence after
speech. Ship-scene playback had not published those counters to the lifecycle,
leaving them both zero. No idle frame arrived, so the original frame-ready gate
stopped calling the chooser. Stored choice labels were therefore insufficient
evidence of rendering or interaction. Ship, panel, and contact-scene owners now
publish the same retained counters to the scheduler; the frame-ready gate is
unchanged.

The accompanying native audit also exposed a separate text-hold mistranslation:
file `0x124D` tests the queued-presentation byte `0x2200`, not contact-transition
activity at `0x29DD`. Rust now tests `c2_presentation_gate` before resuming the VM.
The older text-hold test incorrectly assigned the native queue field to the
contact flag; that mapping is corrected and the two flags are deliberately
opposed. `re/tools/big_bug_bang_idle_scheduler_oracle.py` runs unchanged
`BLOOD2PG.EXE` instructions at `0x11D2..0x1321` for 86 synthetic boundaries.
The new test reproduced the mismatch before the correction. All 86 cases now
match, including equal/wrapped queue counters and independent contact/queue
gates. A second capture is byte-identical. This is scheduler-boundary evidence,
not a native full-gameplay oracle.

The regression scenario now selects the second concept row at `(225,103)` after
the interlude. The first queue-transfer replay passed in 481.04 seconds, with
444 selecting frames containing both labels and none missing. After correcting
the text-hold gate, the final replay passed in 433.96 seconds:
`output/fidelity/bbb-templand-dialogue-1788890770084644379-3869719-0`.
Its captured/inspected `choices-visible.png` shows FINISH and NO HURRY with the
hand present; `selection-continuation.png` shows "LET'S CONTINUE, THEN...".
Trace analysis finds 328 selecting frames with both RGB text rows, zero missing
rows, first visibility at frame 3981, and the selected branch at frame 4342.
The final frame 5439 returns to profile 2 navigation with the Tempest life-form
readout, no active actor, and a closed chooser. The test now requires the exact
two labels, nonzero text pixels in each row throughout selection, and the
authored `no_hurry` continuation. Row geometry/pixel diagnostics supplement the
older misleadingly named `rendered_word_choices` label inventory.

The enabled game-library suite passes 964 tests (38 ignored), and game-package
all-targets checking passes. The prior workspace-wide script-compiler test
failure is still outstanding. The alternate FINISH branch, later gameplay,
remaining localization, and full-game fidelity are not established here.

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

## Inventory Chooser Labels

`en/inventory.json` provides English display names for the 25 inventory objects
and the runtime translates the verified `ANNULER` cancel label to `CANCEL`.
Substitution is limited to the recognized BBB executable and requires both the
original object record ID and exact source name bytes. Modified labels and
unmatched identities retain their text; Commander Blood receives no substitution.

The override runs after original inventory choices are constructed. It changes
only owned display labels, not object IDs, VAR bytes, dictionary operands,
ordering, inventory membership, selection results, or descriptor lookup.
The resource-backed test checks all 425 object records across 17 profiles,
unchanged identities, exact English text, RGB glyph output, and rejection of
changed source names, IDs, and executables. This is not yet live verification
of giving or cancelling an item with the translated chooser. The same test now
also exercises 400 translated list layouts: every starting object in cyclic
order at each supported roster size from 1 through 16, always including CANCEL.
It checks panel bounds, nonblank glyphs confined to their panel, all 3,800 row
clicks against expected item indices, and cancellation without item selection.
These use the original font and shared BBB list planner, not a live gameplay run.
Other object-name surfaces and BAS descriptions remain untranslated.

## Timed Sequence Captions

### Current-Build Templand Replay

After the full timed-caption catalog was added at `738715eb`, the production
`templand_interlude_returns_to_dialogue_and_choices` test passed in 435.88 seconds
using ordinary scenario input in a private X11 display and disposable save copies.
Artifacts: `output/fidelity/bbb-templand-dialogue-1788896504507189024-3968894-0`.
The executable SHA-256 is
`4286f1c7a838e76822dab2de6abc5ff962e3fda30185b41afead4f8efb679316`.

The 5,440-frame trace contains 59 `SQ\venus06.hnm` interlude frames and 328
FINISH/NO_HURRY selecting frames, with zero missing choice-label frames.
`choices-visible.png` and `selection-continuation.png` were captured from this
running process and visually inspected: both labels appear, and selecting
NO_HURRY displays "LET'S CONTINUE, THEN." Final frame 5439 returns to
`PLANET: Tempest LIFE FORMS: Daddy_Gluxx`, profile 2, with the chooser closed,
no active video, no active presentation screen, and an unblocked ship scene.
This revalidates the bounded Templand route on the current build, not later
progression or live playback of every translated sequence.

### Templand FINISH Is Terminal

The other Templand answer, FINISH, is an authored early exit, not a route back
to navigation or proof of winning the game. SCRIPT3 at `0x1466` says "it's over
for you", then `0x1478` says "as you wish, Commander" and `0x148C` requests
`fin.hnm`. Reading the subsequent COD text alone led to an incorrect initial
expectation that dialogue would resume. The original executable decides otherwise:

- A8 at file `0x6E7C` copies the basename and, at `0x6EA4`, sets GS:`0x6B93`
  when its first four bytes are lowercase `fin.`.
- Scene completion at `0xB6CE..0xB6D8` copies that flag's low bit into `0xD1D`.
- The main-loop gate at `0x1140` branches to cleanup at `0x13D8` when set.

`re/tools/big_bug_bang_finale_oracle.py` executes those original instruction
ranges without replaced callees. Five checked-in vectors distinguish `fin.hnm`
and `fin.other` from `FIN.HNM`, `affin.hnm`, and `venus06.hnm`; a repeated capture
is byte-identical. These probes stop at the cleanup branch, not inside DOS cleanup.
The Rust prefix test consumes the same vectors. No runtime behavior was changed.

The first live FINISH capture is
`output/fidelity/bbb-templand-finish-1788897090964201465-3975196-0`.
Its `choices-visible.png` and `finish-clip.png` capture the selection and clip;
the clip image was visually inspected. That run exited successfully after the
clip, but its original test assertion failed because it incorrectly required
continuation. The corrected regression is
`templand_finish_exits_after_authored_clip`; it requires visible choices, the
FINISH branch text, the clip, completed playback, successful process exit, and
no continuation or return to navigation.

The corrected test passed in 372.54 seconds. Its independent replay artifacts are
`output/fidelity/bbb-templand-finish-1788897794096234995-3984258-0`: 4,637 frames,
328 selecting frames with zero missing labels, and 263 `SQ\fin.hnm` frames.
Final frame 4636 has no active video and retains "as you wish, Commander...";
the process exits successfully. The broader game-library run passes 965 tests
with 49 ignored; the five native probe cases and all-targets check also pass.
NO_HURRY's final-navigation assertion was strengthened using the prior captured
endpoint, but that live branch was not rerun after this test-only refactor.

### Catalog Coverage

`en/sequences.json` covers all 706 authored subtitle cues across 54 sequences in
the verified DESCRIPT, including the original blank and number-only cues. These
numbers are present in the original resources; they are not substitute dialogue
invented by the port. The latest 377 cues cover the concert, remaining broadcasts,
and Bob's ending. All natural-language cues have an English first pass.
The modern renderer selects English only for Big Bug Bang with the matching
DESCRIPT SHA-256 hash and an exact match for the complete source cue stream.
The source database, video/audio selection, cue ordering, frame thresholds, and
playback state are not rewritten. Modified resources and unlisted cue streams
retain their original captions.

The loader rejects missing records, cue count/frame mismatches, changed blank
cues, non-ASCII text, and ambiguous translations of identical source streams.
The caption layout test uses the original line planner to check screen bounds.
The original-resource test also rasterizes each translated cue with the runtime's
8x8 caption glyphs and the executable's caption color, rejecting unexpectedly
blank output and preserving authored blanks. It also checks every original
subtitle-bearing record has a binding and number-only cues remain unchanged.
These checks do not establish
contextual translation quality or live rendering.
The catalog includes the nuclear-evolution threshold of 100 and the Izwalito
peace-treaty hint. Fragmented jokes retain their cue
boundaries; proper names and the authored `ICS` abbreviation stay intact.
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
the modern runtime binds English catalogs for BBB SCRIPT1 through SCRIPT10,
SCRIPT12, and SCRIPT14 through SCRIPT17
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

At that checkpoint, remaining work included BAS/UI translation, the other COD
profiles, contextual editorial review, and actual playable startup and
progression. See the current coverage above for subsequent work. No English
voice acting or whole-game localization is claimed.

The menu-prose integration regression run passed 942 library tests serially
(29 ignored), all three menu-reveal checks, four localization checks, and the
original-resource runtime binding test. All-targets checking passed. These do
not establish complete gameplay or live reachability of every translated menu.
