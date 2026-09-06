# Big Bug Bang Rust Port

## Objective

One Rust engine must run Commander Blood and Big Bug Bang, with English
localization for Big Bug Bang. Preserve each game's original behavior, keep
runtime memory flat and owned, reuse SDL3/wgpu and imported RGB assets, and
ship no external executable dependency. Test/oracle tools are separate from
runtime dependencies.

The objective is active and **not complete**. The verified Big Bug Bang build
now plays its opening and loads SCRIPT2 after PLAY; complete gameplay is not
established. The original-disc investigation is in
`big-bug-bang-investigation.md`; its initial decoder limitations describe the
state before the implementation below.

## Verified Implementation

### SCRIPT2 Load and Native Transition Gate

The retained-state loader now binds SCRIPT2's VAR-relative word 8368 to the first
two bytes of the actual SCRIPT2.DEB resource. This is a distinct read-only word
identity, not trailing VAR storage or a copied SCRIPT2.VAR default. It cannot be
written, widened into a record, or serialized into a save image. Profile loading
clears old aliases before binding the verified sequel layout; Commander and
other layouts do not receive this binding. The original-resource loader test
executes the decoded `time > 19` comparison, checks unchanged retained VAR bytes,
rejects a write, and checks repeat loading and removal on return to SCRIPT1.

The native main-loop gate at file `0x116D..0x118A` checks navigation choice,
save, load, navigation transition, and actor transition. It does not use
Commander's additional UI/presentation gates. The sequel now selects that rule
after a successful VM pass. Lifecycle-owned presentation flags explicitly
cleared by the native loader at `0x588F..0x5906` are also cleared before the new
VM pass, preventing the old lifecycle snapshot from restoring them. Countdown,
hold-ready/completion, and UI ownership retain their observed values. Commander
keeps its previous gate and reset behavior.

`re/tools/big_bug_bang_profile_gate_oracle.py` executes the unchanged native
gate for 512 cases, including high-bit blockers and unrelated globals filled
with zero or 255. It also captures five loader-reset cases. Regeneration is
byte-identical. Rust checks every gate case against 256 combinations of the
additional presentation/UI flags, for both dialects.

Live run `modern-honk-play-03` first loaded SCRIPT2 at frame 2401 and exited
normally after 5604 frame records. With the native gate/reset changes,
`modern-honk-play-04` loaded it at frame 1104 immediately after PLAY and exited
normally after 2010 records. Both retain SCRIPT1.VAR. The latter visibly reaches
SCRIPT2's French instruction to visit the cryobox (`screen-013.png`). Its cryobox
attempt ends during a static-TV presentation (`screen-020.png`), with semantic
subtitle `WAIT COMMANDER ...`; it does not establish reaching Bob or completing
the cryobox sequence. SCRIPT2 and the remaining profiles are still untranslated.
Post-load navigation/counter/HUD ordering also needs further native comparison.

Verification: 944 game-library tests and 116 format-library tests passed
serially (30 and 10 ignored respectively); the original-resource SCRIPT2 loader
test was run explicitly and passed; all-targets checking passed. Unrelated user
worktree edits were present but are not included in this change. The older
load-failure sections below describe superseded checkpoints, not a current
SCRIPT2 rejection. Full gameplay and English localization remain incomplete.

### Native PLAY Transition and Adjacent Directory Ownership

Two bounded original-executable runs now reach HONK's `JOUER` choice and
profile 1 (SCRIPT2) through private-X11 pointer input. The experiment reads
guest memory through the existing capture inspector; it does not write guest
state or force the profile. Both runs use the verified original executable,
DOSBox-X normal CPU core at 30,000 cycles, and separate writable directories.
The local experiment and raw captures remain ignored under
`output/big-bug-bang/`; they are evidence, not a packaged regression harness.

`native-play-02` first observed profile 1 at 348.068 seconds. The independent
`native-play-04` run first observed it at 110.989 seconds and retained it through
the 360.285-second final observation. Its 501 profile-1 observations all have
consistent resource bindings and the same ownership result:

- Retained SCRIPT1.VAR: handle 2, linear address 526960, allocation 8368 bytes.
- Active SCRIPT2.DEB: handle 38, linear address 535328, allocation 11168 bytes.
- VAR-relative byte 8368 therefore addresses SCRIPT2.DEB byte zero, outside VAR.
- The two bytes read as 24930 (`0x6162`), the beginning of the directory name
  `baby1`, not a separately initialized time variable.

The second run's `after-05.png` visibly shows `QUE DESIREZ-VOUS?` and
`JOUER` / `EXPLICATIONS`; its final image shows the forward bridge view after
the profile transition. Event-stream SHA-256:
`ca9824931bca045a7b3ff2d8653984394421c2de7b968a354fc5a363e085073f`.
Final guest-dump SHA-256:
`1d66443fa00d01c1f8a28da2ecddf6ab712fcf927c4d49552cd44cc9634e5841`.

This superseded the earlier lack of a live native SCRIPT2 ownership capture;
the capture alone did not change the modern loader or prove execution of the particular
COD condition. The binding described above derives the read from the actual adjacent
directory bytes, preserves VAR serialization, and avoids making directory data
writable script state. The original also transitions shortly after selecting
PLAY, whereas the earlier modern run continued a long HONK speech before
attempting the load. The native gate correction above removes that delay in
the repeated PLAY scenario. Full gameplay and English coverage are still incomplete.

### English Inline Prose and Live Transition Blocker

The matching opening catalog now supplies display-only inline menu prose as well
as subtitles and retained choice labels. Binding requires both the accepted A6
instruction and its complete authored word stream. The shared layout/reveal
routine accepts an optional display stream, leaving original dictionary words,
VAR operands, and chatter inputs untouched. English word count controls only
display reveal and completion timing. The original entry point supplies no
override and retains its native comparison coverage.

Trace snapshots retain the original word stream separately from displayed words
and do not assign dictionary IDs to translated prose. Raster reconstruction uses
the same bound display stream, and a changed stream cannot reuse a stale reveal
snapshot. All 89 English prose sections pass inline-font raster and screen-bound
checks; an original-resource runtime binding test covers Bob's first menu source
(`0x977`) and rejects mismatched words and profile-reset state. Those are isolated
checks, not live Bob/OLGA reachability.

The longer ordinary-pointer PLAY run in ignored local
`output/big-bug-bang/modern-honk-play-02` exited with game status 1 after 2,401
trace records. It selected PLAY, continued the opening English dialogue, and
attempted to load SCRIPT2. The actual load failed with `InvalidStateWord` at
COD byte `0x5A97`, VAR byte `8368`. Thus the shared-state blocker described below
is now reached through ordinary UI progression, not only an isolated loader call.
No extra VAR word or neighboring-resource value has been fabricated to bypass it.

A separate ordinary-pointer cryobox attempt (`modern-cryo-01`) opened an empty
contact list in the initial profile and exited normally after 1,773 records.
It did not reach Bob's recording, so the new prose path still lacks that live
visual check. Full English gameplay remains incomplete.

Verification: 942 library tests passed serially (29 ignored), all three menu
reveal tests passed including native numeric-renderer comparisons, the four
original-resource localization checks passed, the runtime menu-binding check
passed, and all-targets checking passed. Existing unrelated worktree edits were
present for the checks but are not part of this change.

### English Opening Choices

The retained dictionary-choice renderer now binds English labels to the accepted
A6 source instruction and exact ordered dictionary IDs. Layout measures the new
labels; completion still submits the original identity. The bundled opening
catalog supplies all three static choice sections. Inventory choices, unknown
sites, reordered word lists, and unmatched resources do not receive an override.
Profile reset clears the last published site; gated text does not replace it.

The ordinary-pointer run `output/big-bug-bang/modern-honk-play-01` reached the
opening menu with rendered labels `PLAY`, `INSTRUCTIONS` and original selector
words `JOUER`, `EXPLICATIONS`. Screenshot `screen-019.png` confirms the English
labels. Clicking logical position (185, 94) completed the first choice and
requested profile 1 (SCRIPT2), then continued English dialogue. The run exited
normally after 2,004 trace records, still during that dialogue with the profile
request pending. It did not establish a completed SCRIPT2 transition.

Verification: all 941 enabled library tests passed serially (28 ignored), the
four localization tests passed with original resources enabled, the isolated A6
dispatch comparison passed, and all-targets checking passed. Source tests check
all three choice sections and reject mismatched/reordered choices. Only the
opening menu has a live visual/selection check in this slice; menu prose and the
remaining profiles are still not translated at runtime.

### Text Hold Script Resume

The sequel coordinator at BLOOD2PG file offsets `0x11D2..0x1280` writes the
VM execution flag while a completed text reveal still has a nonzero hold timer.
It enables execution only when the scene gate is clear. The shared lifecycle
now selects this rule from the runtime's game dialect; Commander keeps its
previous behavior. Zero countdown and the active/ready early branch do not
perform this write.

`re/tools/big_bug_bang_text_lifecycle_oracle.py` runs the unmodified native
instruction range with 2,048 synthetic input combinations, checks bounded
execution and allowed writes, and verifies module immutability. Its committed
vectors cover both initial VM states, scene gates, secondary input, and timer
values 0, 1, 255, and 256. The Rust comparison checks the VM flag only, not the
subsequent presentation coordinator. This is native boundary evidence, not
proof of full gameplay reachability.

The private DOSBox capture helper now observes guest state while the mouse
button is held, releases the button even if observation fails, and supports
captured relative motion before scheduled clicks. Requested movement is not
reported as confirmed guest movement. A real capture observed primary button
state 1 during the press; large relative deltas were not consistently accepted
in full by the original runtime.

Verification: 941 game-library tests passed (28 ignored), 30 capture-helper
tests passed, all-targets checking passed, and regenerating the 2,048 native
vectors produced an identical file. Existing unrelated runtime edits were
present during these checks and remain outside this change.

The ordinary-pointer scripted run in local ignored
`output/big-bug-bang/modern-honk-navigation-05` advances HONK's first English
line to "What would you like?" and reaches retained word-choice phase
`Selecting`. Screenshot `screen-020.png` shows both menu entries, still French
(`JOUER`, `EXPLICATIONS`), and the final subtitle has 424/424 matching raster
pixels. The pre-fix run `modern-honk-navigation-04` remained on the first line.
The scenario uses the existing worktree's pointer-based `park` action, not VM
state injection. This verifies opening conversation progression, not the menu
branches, full translation, or complete playability.

### Game Identity and Loose-Asset Import

The game executable now has a library-only, import-only mode for either game:

```sh
nix develop -c cargo run -p commander-blood-game --bin commander-blood -- \
  --data output/big-bug-bang/disc \
  --import-assets output/big-bug-bang/imported-assets
```

This extracts original resources into ordinary files, verifies their checksums,
and exits without opening SDL or transcoding audio/video. It is preparation for
the shared runtime, **not a playable sequel mode**.

`GameVariant` owns each game's display name, storage namespace, executable/title
filenames, script dialect, and native resource/profile catalog selection. Sequel
catalog decoding requires the exact analyzed executable SHA-256. Importing a
different executable revision does not imply that its native tables are supported.

The importer detects the game from its main executable, rejecting a source with
both main executables. Both games use `BLOOD.DAT`, so the archive filename cannot
identify the game. Sequel companions are `BLOOD2PG.EXE`, `BLOOD2.LBM`, `TB.BIG`,
and `DESCRIPT.DES`. Neither the sequel disc nor its archive contains `BLOOD.SAV`;
the importer does not borrow or invent that template, or any missing BAS files.
Commander's existing companion requirements are preserved in this slice.

Manifest schema one gains an explicit `game` field. Existing manifests without
it remain Commander imports, with required Commander companions still checked.
This preserves existing media caches rather than forcing their regeneration.
Reusing an import now checks game identity, executable fingerprint, and available
source archive/loose-file fingerprints. Missing source archive data can still be
served by its completed cache, as before; that path is not a new source comparison.
A different game/build/content fails without replacing the destination. The game
identity check also applies to damaged caches. Source/destination overlap is
rejected before copying or replacing directories.

Default writable/cache namespaces are distinct: `commander-blood` and
`big-bug-bang`. Existing `CBLOOD_WRITE_DATA` and `CBLOOD_ASSET_CACHE` overrides retain
their exact Commander meaning; sequel defaults beneath those overrides use a
`big-bug-bang` child. Explicit path arguments remain exact caller-selected paths.
This is path/import isolation, not a completed sequel save-format implementation.
An explicit `CBLOOD_DATA` source is now resolved before default cached Commander
data, so a requested game is not silently replaced by the cached one.

At the import-only milestone, production path loading explicitly rejected a sequel manifest before calling
remaining Commander-only presentation decoders or starting media conversion. The new
native catalog selection is used by the existing Commander loader and by the
sequel imported-profile integration test. The rest of the sequel runtime still
needed connecting; rejecting it was not counted as game support. The startup
integration below supersedes that guard; earlier guard references in this log
describe their respective historical milestones.

The original-disc import integration test verifies every imported checksum, then
constructs an archive-free resource store and loads the initial sequel profile
through `ScriptProfileManager`. It verifies native COD ownership for missing BAS
and exact initial VAR bytes. The original-disc dependency is marked ignored by
default and was explicitly enabled when checking this slice.

Verification for this slice (2026-09-05): all 14 import tests passed with the
original-disc test explicitly enabled. The full game library passed 912 tests
with nine ignored, serially on a freshly allocated private Xvfb display. Game
all-targets checking passed. Checks ran with the existing unrelated Commander
runtime edits, which remain outside this change.

The CLI command above was also run with `DISPLAY` and `WAYLAND_DISPLAY` unset.
It imported 1118 resources from 944 archive entries plus loose files, with all
checksums verified. The resulting local manifest SHA-256 is
`1bb7199d23d840629eadfe4a7df656eaa04b8e3bc66a278509ba22c617408617`;
its archive hash matches the inspected original sequel disc. The output contains
only `resources`, `companions`, and `manifest.json`: no fabricated BAS/save data
or media transcodes. Original assets remain ignored by Git. Attempting a
production one-frame run against this imported tree returned the explicit
not-yet-integrated sequel runtime error before opening SDL. That negative check
verifies the guard, not gameplay.

### Sequel Fonts and Bridge Tables

Native references and direct original-binary comparisons establish the following
table locations. Game-specific decoders now read these into owned, flat tables;
the font tables feed the existing RGB glyph importer in integration tests.
This does **not** establish complete sequel UI behavior or enable its production
loader. All offsets below refer to the analyzed `BLOOD2PG.EXE` file;
its global-data base is file 0xF7F0.

| Table | Map/Start | Advances | Glyphs | Extent |
| --- | --- | --- | --- | --- |
| Bridge anchors | 0x14AC9 | n/a | n/a | 66 consumed bytes, identical to Commander |
| Bridge trigonometry | 0x14B05 | n/a | n/a | 180 four-byte samples, identical to Commander |
| Navigation actors | 0x124AB | n/a | n/a | Six 24-byte records with sequel resource IDs/flags |
| Small font | 0x1709E | n/a | 0x1711E | 128 map bytes, 42 five-row glyphs |
| Subtitle font | 0x171F0 | n/a | 0x172D8 | 232 map bytes, 66 eight-row glyphs |
| Square font | 0x174E8 | 0x175D0 | 0x17602 | 232 map bytes, 49 reachable glyphs |
| Main font | 0x179D6 | 0x17ABE | 0x17B16 | 232 map bytes, 87 reachable glyphs |

The bridge projector at file 0xB337 loads data offset 0x52D9 and a count of 11;
the 11th consumed anchor overlaps the start of the trigonometry data, as in
Commander. The matrix routine at 0xB058 loads trigonometry offset 0x5315.
The dual-font measurement routine at 0x344D references square map/advance offsets
0x7CF8/0x7DE0 and main offsets 0x81E6/0x82CE. The subtitle renderer at 0x39BE
references 0x7A00/0x7AE8; the small renderer at 0x3A78 references 0x78AE/0x792E.

The sequel map maxima, excluding sentinel 255, independently confirm reachable
glyph indices 41, 65, 48 and 86 respectively. Commander has only 176 entries in
its proportional maps and 55/48/86 subtitle/square/main glyphs. Simply relocating
its fixed arrays would omit sequel characters. The font audit also found changed
main-font advances. The decoder preserves their actual values rather than assuming
shared glyph bitmaps imply identical text layout. Main glyphs 69/71 advance by
5/8 respectively in the sequel, versus 8/5 in Commander. Variable-sized owned
font tables retain each game's exact dimensions; they neither truncate sequel
characters nor pad Commander tables. The small font remains identical.

Native text measurement indexes advances even for map sentinel 255 and subtracts
two with unsigned 16-bit wrapping. Import retains the original 256-byte lookup
region for each measurement face. This is serialized lookup data, not an emulated
memory space. Display advances remain separate, signed-byte glyph tables.

`re/tools/big_bug_bang_font_width_oracle.py` executes the **complete original**
procedure at file 0x344D through its far return at 0x3485, with original font
tables and synthetic strings. No native call is replaced. It guards the executable
SHA-256, execution range, register preservation, source/data immutability, and
stack-write range. Its 492 vectors cover both faces, every input byte 1 through
231, empty/NUL-terminated text, extended characters, and width overflow. The Rust
measurement test matches all 492 original results. The vectors contain synthetic
strings and measured widths, not original executable code or glyph bitmaps.

Separate RGB integration tests compare imported glyph coverage, subtitle reveal,
and channel masks against our **C-derived Rust raster functions** for all mapped
sequel characters. They include byte 225, beyond Commander's map, and all 89
mapped square-cap characters. These are not original-sequel framebuffer captures
and must not be presented as equivalent evidence. Native-table tests additionally
verify the bridge anchors, trigonometry and all six actor records against original
bytes. Actor resource IDs are 17, 13, 15, 16, 19 and 18; reusing Commander's IDs
would be incorrect even though the projection tables match.

The assembly comparison also found an **unported destination-selection branch**
in the sequel planar square-cap routine (entry 0x37A8). At 0x37D0 it loads the
destination from GS:0x55E9, tests word GS:0x6B94, and, when zero, selects GS:0x55ED
instead. Commander has only one unconditional buffer selection. The sequel
selector is linked to its dynamic inventory-choice flow, traced below;
glyph/width tests do not validate that routing. The sequel planar-main entry
is 0x38FC.

`GameVariant` selects and fingerprint-checks both new decoders, and the Commander
runtime now accesses fonts/bridge tables through that same identity boundary.
The sequel production-loader guard remains in place until remaining presentation,
menu and host-state behavior is implemented from evidence.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_font_width_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/font-width-verified.jsonl
cmp re/tools/oracle_vectors/big_bug_bang_font_width.jsonl \
  output/big-bug-bang/font-width-verified.jsonl
nix develop -c cargo test -p commander-blood-formats --lib -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib native::bloodprg::font::tests -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib ui::tests -- --include-ignored
```

Original-disc font/bridge tests are explicitly ignored by default and were
enabled for this verification. Missing original assets are not counted as a pass.

Verification for this slice (2026-09-05): regenerated all 492 width vectors and
compared the output byte-for-byte with the saved fixture. All 119 formats library
tests passed with original corpus checks enabled. All ten native font tests and
eight UI-filtered tests passed with ignored checks explicitly enabled. The full
game library passed 912 tests, with 11 ignored, serially on a fresh private Xvfb
display; its server was reaped afterward. Game all-targets checking passed.
Existing unrelated Commander runtime edits were present during these checks but
are excluded from this checkpoint. No full-game sequel parity claim follows.

### A6 Condition Audit and Dynamic Inventory Choices

Direct assembly comparison found an inherited Rust translation omission:
both Commander at 0x636B and the sequel at 0x6B5A test the sign of the low control
byte before testing detail bit zero. Control flag 128 overrides equality and
requires signed `record > operand`. The recovered C `vm_condition_5` already
preserves that priority; Rust previously selected equality whenever detail bit
zero was set. `ScriptTextControl::uses_record_equality` now encodes the priority
and is used by the shared condition evaluator for both games.

`re/tools/text_record_condition_oracle.py` executes each original complete
condition procedure with only record-condition controls enabled: Commander
0x6339..0x6432 and sequel 0x6B28..0x6C44. It uses each executable's real field
matrix, guards both SHA-256 values, checks preserved registers and source/data
immutability, permits only the four stack-scratch bytes, and rejects execution
outside the procedure. No callee is substituted. The 288 vectors cross all four
equality/ordering flag combinations with signed boundary values. They do not
cover the unentered PRNG, history, or menu branches of these procedures.

The new Rust regression was run before the fix and failed at flags 388,
record zero, operand zero: Rust returned true; the original returned false.
This is a reproduced assembly/C-to-Rust discrepancy, not an inferred UI patch.
After the fix all 288 comparisons pass, along with the existing condition
vectors (five condition tests total). All 119 formats tests passed with corpus
checks enabled; the complete game library passed 913 tests with 11 ignored,
serially on a fresh private Xvfb display, which was reaped afterward. Game
all-targets checking passed. These checks include the same unrelated local
Commander runtime edits, not staged with this repair.

The following **unported** sequel paths were established by assembly inspection.
The condition and audio-gated transfer paths now have native component captures
described below; full presentation and authored execution remain unverified:

- A6 entry 0x6C89 saves its current selector-byte position to GS:0x6B4E.
  Its condition helper 0x6B28, on the resume/post-list path, recognizes the
  special word 65534 at 0x6C10 and calls 0x6C45. A zero count clears the yield
  and resume bytes and returns failure. A nonzero count copies the saved line
  position to GS:0x6B94 and returns success.
- The complete helper 0x6C45..0x6C88 scans GS:0x70E6 until word 65535, skips
  zero slots, tests the VAR record kind for mask 1024 (`InventoryItem`), and
  writes each selected record offset plus four to GS:0x6BDC. It terminates the
  result with zero and returns the count in AL. These are object-backed names,
  not DIC offsets and not ordinary actor records.
- The original GS:0x70E6 table contains sixteen zero slots plus a 65535
  terminator. Startup clears sixteen slots at 0x5890. The loader at 0x59FF
  populates it from active directory entries whose selector-17 field is 65535.
  Helpers 0x65E8 and 0x6606 remove or insert within those sixteen slots.
- The ordinary concept-consumption routine at 0x5C41 has a sequel-specific
  branch when resume bit two and GS:0x6B94 are both set. It clears that saved
  line and pending presentation state, marks the saved COD line, removes the
  chosen object from the sixteen-slot table, writes its selector-17 relationship
  from the saved line operand, and sets record flag 64. Later presentation calls
  in the same branch still need complete integration/oracle coverage.
- GS:0x6B94 also selects UI call order in the main loop (0x138B/0x13A3),
  background/text drawing paths (0x963E/0x970D/0x9754), and the planar text
  destination (0x37D5). The two page offsets are swapped at 0x43ED..0x43F7.
  Fixing only the font destination would omit the underlying interaction.

The typed `ScriptTextWord::InventoryChoices` now represents marker 65534 only in
the sequel dialect, alongside dictionary words, numeric operands and separators.
A6 expands it through the owned roster and object-selection state; UI routing
is still incomplete. Do not invent dictionary entries, substitute all actors for the
native candidate table, or enable sequel production loading with this flow
missing. Authored execution reachability and the later presentation calls remain
to be verified before calling the whole A6 path recovered.

#### Native Inventory Condition and Transfer Captures

`re/tools/big_bug_bang_inventory_condition_oracle.py` executes the complete
condition procedure 0x6B28..0x6C44 with authored controls `0x8030`, including
the real bytewise separator scanner 0x68A5 and inventory helper 0x6C45..0x6C88.
Its 22 vectors cover empty inventory, every one of the sixteen roster slots,
full and mixed rosters, raw kind-mask filtering, and duplicate entries.

The native result preserves slot order and duplicates, skips zeros, and returns
VAR record offsets plus four as choices. An empty result clears resume and
yield, returns carry clear, and **leaves the previous saved-line word intact**.
Spoken mode is enabled before that rejection and remains enabled. A nonempty
result sets yield and replaces the saved-line word. These are state effects,
not just candidate-list filtering.

Each valid inventory candidate then enters the complete selection procedure
0x5C41..0x5D5C with its original removal and field-matrix helpers. The 82 captures
exercise both native audio gates separately: global gate bit 0 at GS:0x2A33 and
dialogue gate bit 1 at GS:0x6B80. No helper is replaced or skipped by the harness;
these input gates naturally avoid the unresolved audio lookup at 0x8450.
Raw combined-kind candidates are excluded from transfer captures because the
field helper selects the least significant kind bit, unlike candidate filtering.

The transfer removes the first matching roster slot only, writes the saved A6
actor operand into selector 17 (inventory byte offset 20), sets object flag
`0x40`, and sets the high control byte's `0x80` enable bit in the saved COD line.
It clears selected and alternate concepts, saved line, resume and the pending
choice-list head. Yield and spoken mode remain set. Unchanged-memory guards
also establish that this branch does not append the selection to concept history.

Both executable SHA and entered-code ranges are guarded; all writes must fall
within explicit output or stack ranges, preserved registers are checked, and
source/state/global bytes outside those outputs must remain unchanged. Two
independent generator runs produced byte-identical fixtures:

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_condition_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_condition.jsonl \
  re/tools/oracle_vectors/big_bug_bang_inventory_selection.jsonl
```

`SequelInventoryState`, owned by each profile's selector state, now represents
saved A6/recipient identities and pending/selected objects without `ScriptWordId`.
Its condition component matches the 20 native cases representable by the strict
VAR decoder, and its gated transfer matches 80 cases. The two raw-kind condition
vectors and two associated transfers contain invalid or combined record kinds;
they remain native robustness references, not claimed Rust parity coverage.

Dispatch routes a pending inventory selection before ordinary DIC encoding,
history insertion and BAS processing. Transfer updates owned VAR and the derived
relationship-field view directly. It does not rebuild the roster: reconstruction
would compact holes and erase the native first-duplicate-only removal behavior.
A dispatch regression checks unchanged concept history, retained duplicate slot,
reactivated A6 state and exact VAR synchronization after transfer.

The ungated operation retains an explicit descriptor-lookup continuation and
the selected object, whose native clearing occurs after descriptor processing.
Dispatch now completes that lookup through the descriptor backend, as detailed
below; an absent backend remains an explicit error. Invalid selections, inactive resume,
missing instruction state and wrong dialect have explicit rejection checks.

Authored inventory A6 text now decodes and invokes the condition component after
the native gates. The extended oracle executes 0x6C89 through its return at
0x6E53 with all entered helpers intact: 27 synthetic captures cover publication,
empty choices and five rejection gates. Rust matches the 25 cases representable
by its strict VAR decoder. The two raw-kind cases remain native-only references.

Another 46 captures execute every original inventory A6 occurrence using pinned
COD/VAR/DEB/DIC images, with controlled actor action, shown flag and a one-item
roster. Rust matches subtitle hashes, VAR effects, resume state and publication
flags. These controlled handler checks do not prove gameplay reachability.

The dispatch regression now decodes and executes a synthetic A6 through a complete
frame before selection, checks duplicate choices, transfers the selected object,
then prepares another frame. Sequel field refresh preserves the canonical roster,
including holes and duplicates. Native pre-frame helper 0x6038 updates actor
relationships; direct calls to roster reconstruction 0x59FF occur at 0x11A8
(profile change) and 0x1FAF (load). Commander refresh behavior is unchanged.

The original handlers in both games also clear the alternate concept while arming
resume, before rejection gates; the shared Rust handler now does so. Runtime
choice readiness includes object choices without converting them to dictionary IDs.

Object-name display and UI selection/cancellation are now connected through the
shared chooser, as detailed below. Descriptor lookup is connected, but playback
completion and the profile lifecycle remain unfinished. Production sequel loading stays disabled until
the complete flow is integrated and verified.

Verification: all 123 formats tests pass with original-corpus tests enabled;
the game library passes 925 tests with 15 ignored under a private X server.
The authored inventory test (46 cases) and numeric-menu oracle test (59 cases)
also pass when explicitly enabled. Regenerated condition, transfer, synthetic A6
and authored A6 fixtures are byte-identical to the checked-in vectors.
No full-game inventory-playability or independent-review claim follows. Reproduce
the complete text captures after generating the corpus audit:

```sh
nix develop -c cargo run -p commander-blood-formats --example audit_sequel_text -- \
  output/big-bug-bang/imported-assets/resources > output/big-bug-bang/text-audit.json
nix develop -c python3 -P re/tools/big_bug_bang_inventory_condition_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  output/big-bug-bang/inventory-condition-oracle.jsonl \
  output/big-bug-bang/inventory-selection-oracle.jsonl \
  --text-output re/tools/oracle_vectors/big_bug_bang_inventory_text.jsonl \
  --resources output/big-bug-bang/imported-assets/resources \
  --audit output/big-bug-bang/text-audit.json \
  --authored-text-output re/tools/oracle_vectors/big_bug_bang_authored_inventory_text.jsonl
```

```sh
nix develop -c python3 -P re/tools/text_record_condition_oracle.py \
  re/bin/BLOODPRG.EXE output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/text_record_condition.jsonl
nix develop -c cargo test -p commander-blood-game --lib native::bloodprg::presentation::tests
```

#### Inventory Choice Panel

`PresentationChoiceId` distinguishes dictionary concepts from inventory objects
through panel opening, selection, closing and script publication. The runtime
resolves inventory labels from the offered VAR records' bounded name fields
(bytes 4..20), preserving order and duplicate entries. All 425 authored inventory
records across the seventeen profiles have valid names: 25 distinct labels,
including a CP437 accented name. These are not DEB symbol names or DIC words;
English localization must translate display names without renaming object IDs.

`big_bug_bang_inventory_choice_oracle.py` executes the complete original chooser
0x9B45..0x9C5D with list 0x958A, transition 0x20CE, width 0x344D, planar text
0x37A8 and background remap 0x3F13. Eleven sequences cover waiting, selection,
cancellation, wide/accented labels, full sixteen-item rosters and two ordinary
dictionary-choice controls. No callee is
patched. The original disabled-sound input gate naturally avoids device playback.
These captures verify control, layout and helper order, not planar VGA pixels or
audible playback.

The opening layout has no cancel row. On later updates the inventory branch sets
the cancel flag; interactive layout adds the row with a minimum content width of
71, compared with Commander's 55. The current rectangle is recomputed for this
layout and retained for closing. The cancel label is decoded from the verified
sequel executable, not borrowed from Commander resources.

The sequel chooser pauses script execution when opening and re-enables it on
selection (before closing finishes). This also applies to ordinary sequel word
choices; Commander's chooser does not write the VM latch. Lifecycle imports now
refresh both this latch and the current modal UI bit when reopening a panel.

Rust matches every captured frame's rectangle, transition step and background
region, row text/position/color, phase and completion latches using original font
resources and the real translated transition helper. RGB text coverage is also
checked against the translated font rasterizer, including nonblank and panel-bound
checks. UI completion clears the pending choices but retains the saved line and
resume for an object selection; cancellation clears those and the alternate
concept without transferring an item. A subsequent frame dispatch test uses the
same typed completion operation before the native-referenced transfer.

Four additional native captures execute main-loop 0x1384..0x13AF and the complete
base-frame conversion helper 0x434B. Inventory choices run after base submission,
even without the ordinary frame-presented flag. Dictionary choices remain before
submission and retain that flag's gate. The shared lifecycle now follows this
ordering, and profile changes reset the cached inventory-line owner.

The modern UI binding is implemented, but live sequel startup remains guarded;
these tests do not establish end-to-end gameplay or playback completion.
The original eleven chooser/inventory tests pass with original-asset tests
enabled, and repeated native captures are byte-identical. Reproduce the native
panel and ordering captures with:

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_choice_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_choice.jsonl \
  --order-output re/tools/oracle_vectors/big_bug_bang_inventory_choice_order.jsonl
nix develop -c cargo test -p commander-blood-game --lib inventory -- --include-ignored
```

#### Pre-Frame Occupancy While Paused

The sequel calls 0x6038 before testing its VM enable bit at 0x5A9C. Commander
tests its enable bit before preparing state. The shared frame runner now respects
that distinction even for an empty COD program: a paused sequel frame prepares
state without executing instructions or post-scan handlers.

The sequel preparation pass first clears location flag 4 and the occupant word
at byte 24. It then normalizes actor position flags through the original holder
and coordinate semantics. Each actor with flag 4 and a nonnegative direct
location holder sets that location's flag 4 and occupant word. Nested holders
do not count as direct occupancy, and the last qualifying actor in directory
order wins. This is separate from the aboard-inventory roster, which this pass
does not rebuild. Commander retains its actor-only updates. Typed validation
failures leave both actor and location state unchanged.

`big_bug_bang_state_processor_oracle.py` executes the complete original helper
and unmodified 0x67B8/0x6633 callees in 21 synthetic cases. Seven enter the paused
frame at 0x5A99 after resource binding and execute its real return epilogue.
Rust compares complete VAR images, covering transient-flag gates, stale occupancy,
last-writer order, direct versus nested holders, zero/sentinel parents and
world/arche coordinate matching. The fixtures regenerate byte-for-byte. These
are component captures, not proof of initialized full-game state.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_state_processor_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_state_processor.jsonl
nix develop -c cargo test -p commander-blood-game --lib actor_position_state
```

#### Inventory Descriptor Continuation

The complete transfer at 0x5C41 now continues through DESCRIPT lookup at 0x8450
using the selected object's bounded VAR name. Successful lookup applies the
decoded commands, clears the scene queue gate, requests secondary
presentation line 43 and pauses the VM. A completed miss clears the selection
without requesting playback. A backend failure retains the continuation; retry
does not repeat the transfer or remove a second duplicate roster entry. A missing
host binding is an error, not a fabricated descriptor miss. Lifecycle publication
consumes the VM write once so late text updates cannot replay it over a UI write.

`big_bug_bang_inventory_descriptor_oracle.py` executes the complete transfer,
lookup and every entered helper unchanged. Its explicit INT 21 boundary handles
open/read/seek/close against owned database bytes. Twenty synthetic cases cover
success, case-sensitive misses, missing files, both native gates, empty records,
captions and accented names. Another 25 captures use all authored inventory
descriptors from the hash-pinned DESCRIPT database. No descriptor result is
substituted into native execution. Repeated captures are byte-identical.

Rust dispatch and the real typed DES parser match all 45 cases. An additional
original-asset test checks the concrete runtime backend's 25 object bindings,
selected clip names and resource availability. All these records select HNM
clips without loading an SND bank during lookup. The existing presentation
catalog retains the previous clip when a record supplies no new object-video
command; the per-record asset list is not the owner of that retained filename.

Correction to the initial descriptor integration: GS:0x2200 is the C2 scene
queue gate, not the handoff lock at GS:0x6B8D. Successful lookup clears only
the former and preserves the latter. The regenerated descriptor captures now
name and compare both fields separately; the previous test mislabeled 0x2200
as `start_locked` and therefore did not detect the wrong typed-state write.

Verification for this slice: the full game library suite passed 926 tests with
17 ignored; all 14 inventory tests passed with original-asset tests enabled.
The all-targets check passed, and all 45 regenerated native captures matched
the checked-in vectors byte for byte.

This is not initialized gameplay proof. Scene-completion VM writes are connected
as described below, but production startup and English localization remain
unfinished. The production guard remains in place.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_inventory_descriptor_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_inventory_descriptor.jsonl \
  --resources output/big-bug-bang/imported-assets/resources
nix develop -c cargo test -p commander-blood-game --lib inventory -- --include-ignored
```

#### Scene Completion and VM Resume

The sequel resumes the VM at 0xB68D after the queue reports neither open nor
draining, and at 0xB75A after explicit cancellation of an active scene. Ship,
bridge, camera, HUD and contact-transition dispatch now publish that write to
the owning lifecycle immediately. The panel publishes a consumed-once completion
output before subsequent UI work, so a later chooser pause cannot be overwritten
by replaying an old scene completion. Full presentation shutdown discards that
output. These new writes are gated by the loaded sequel dialect; Commander's
existing VM behavior is unchanged. Inactive sequel cancellation leaves scene and
VM state unchanged.

`big_bug_bang_scene_completion_oracle.py` records 38 cases from unpatched original
instructions. The active-scene continuation at 0xB67C executes the real empty-file
queue-service and source-status helpers, including blocked dispatch and status
values 0 through 3. The complete cancellation routine at 0xB731 executes its
real buffered/empty release helpers. Captures also cover palette reset, ship-depth
thresholds, non-owning flag bits and both initial VM states. Rust compares the
complete typed scene state and the lifecycle VM result against these captures.
This boundary does not decode HNM data, open a file, or exercise line-five display
clearing, and is not proof of a complete inventory interaction in initialized
sequel gameplay.

Verification for this slice: the full game library suite passed 928 tests with
17 ignored, all 14 inventory tests passed with original-asset tests enabled,
and the all-targets build check passed. All 38 regenerated captures matched the
checked-in vectors byte for byte. The one-shot output regression also verifies
that a subsequent UI pause survives another output-consumption call.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_scene_completion_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_scene_completion.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_scene_completion
```

#### A6 Outer-Loop Publication

The sequel's outer COD loop clears its handler yield byte at 0x5ACE before
each instruction. A6 publication returns signal 2 or 3; at 0x5B08..0x5B13,
the loop pauses the VM and sets the handoff lock. Signal 2 clears pending
skips, while signal 3 saves the continuation cursor. Traversal can continue
within the same pass before the ordinary post-scans. The typed dispatcher now
publishes the missing VM and lock writes, preserving the separate C2 gate.
The handoff-lock write is consumed before the actor scan so it cannot replay
over a later scan's release of that lock. Commander behavior is unchanged.

`big_bug_bang_vm_yield_oracle.py` executes the unmodified A6 handlers through
the original handler table and outer loop, from 0x5AA6 to the post-scan boundary
at 0x5B3D. Its 76 cases cover menu, subtitle and inventory text; active, shown,
record-kind, menu/subtitle and empty-inventory gates; both initial handoff-lock
states; and consecutive A6 handlers. The comparisons check full VAR at that
same pre-scan boundary, mutable COD activation flags, traversal end/cursor,
resume state, yield signals, request flags, VM state and both presentation gates.
No original handler is patched or replaced. Pre-frame preparation and the
subsequent selection/actor scans are outside this capture, so it does not prove
whole-frame or initialized startup parity. AA/AC's distinct signal-1 return
path is not covered by these A6 comparisons.

Verification for this slice: the full game library passed 929 tests with
17 ignored, all 14 inventory tests passed with original assets enabled, and
the all-targets check passed. The 76 outer-loop and 45 corrected descriptor
captures both regenerated byte-identically.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_vm_yield_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_vm_yield.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_a6_outer_loop
```

### Sequel Presentation Catalog

The production data loader now routes presentation-table decoding through
`GameVariant`. The sequel decoder reads its own 45 pointer pairs at data offset
0x2203, descriptor slots at 0x22B7..0x2745, and the eight effective unclamped-line
IDs at 0x100C. Its data base is file 0xF7F0. Native scene lookup at 0xB50A and
descriptor selector 0xB763 use that index; the first descriptor follows exactly
45 four-byte entries. The last descriptor ends before the ship-state word.
The scene's nine-byte scan excludes the final match when CX reaches zero, so
only the first eight entries are effective.

`big_bug_bang_presentation_catalog_oracle.py` executes the original selector
for every line without modifying its instructions or tables. Both synthetic
malformed-input tests and an original-executable test cover the typed decoder.
The existing runtime catalog keeps the authored flags, variant and dynamic
filename slots. The sequel opens with `sq\\microfol.HNM`, not Commander's
`sq\\mind.HNM`. All seven fixed names exist in the sequel resource store;
the 38 original dynamic names remain unresolved until their native owners write
them. The original-asset inventory test applies all 25 authored descriptions
through this catalog and loads the resulting line-43 resource bytes.

This is catalog and resource-binding verification, not HNM playback or startup
parity. The loader guard remains: the bridge-menu table and runtime/profile
transitions still need sequel-specific integration.
Scene-start policy, profile transitions and English localization are unfinished.

Verification: two native captures were byte-identical across all 45 entries.
All 125 format tests passed with original-asset checks enabled. All 14 inventory
tests passed with ignored checks enabled, including the catalog resource bindings.
The game library passed 929 tests with 17 ignored, serially on a fresh private
Xvfb display, which was reaped afterward. Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_presentation_catalog_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation_catalog.json
nix develop -c cargo test -p commander-blood-formats presentation_catalog -- --include-ignored
```

### Sequel Startup Visual Tables

The data loader routes the default palette, name-area effects and world-artwork
layout through game identity and the sequel executable fingerprint. Commander
keeps its original offsets. The sequel layouts are verified against these
original consumers:

- Screen reset at file 0xADA4 copies 192 dwords from data 0x5F28 to 0x5621.
  This identifies the full 256-entry six-bit palette at file 0x15718.
- Name-area selection at 0x9DD3 and 0x9DFA uses the pointer table at data 0x2A91
  (file 0x12281). The initial sequence plus nine random alternatives contain
  64 frames. Frame reads at 0x9E0D..0x9E2B supply the native origin and dimensions.
- World-artwork initialization at 0x801B walks data 0x2F97 (file 0x12787) in
  22-byte steps to its word terminator at data 0x3333. The 42 rows retain their
  names, resource IDs, entity IDs and initial flags. Selection at 0x80D8 and
  0x80E7 consumes the resource and entity fields at row offsets 16 and 18.

The offline `big_bug_bang_startup_tables_oracle.py` runs these bounded original
instruction ranges with guarded writes and immutable executable bytes. Its
captures cover every palette entry, effect selection/frame and artwork row.
The formats integration test checks both a metadata-derived fixture and the
original executable, plus malformed bounds, colors, pointers and terminators.
These are startup data contracts, not a complete native screen-reset execution
or proof of production sequel startup. The production guard remains enabled;
menu, navigation and profile integration are still pending.

Verification: both native captures were byte-identical. All 125 format unit
tests and three startup-table integration tests passed with original-asset checks
enabled. The sequel runtime test imported all artwork rows and compared every
RGBA pixel of a scaled rendering with the indexed reference, requiring nonblank
output; it also imported the sequel choice/dialogue fonts using this palette.
The game library passed 929 tests with 18 ignored on a private Xvfb display,
which was reaped afterward. The new ignored artwork test passed separately.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_startup_tables_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_startup_tables.json
nix develop -c cargo test -p commander-blood-formats --test sequel_startup_tables -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_startup_tables -- --include-ignored
```

### Sequel Writable Startup Catalog

The data loader now selects the writable-resource catalog by game identity.
The original sequel loop starts at file 0x190C with SI = data 0x2A0, visits each
16-byte filename, and stops when the next entry starts with NUL (0x1944..0x194C).
Its table is file 0xFA90..0x10410: 152 visits, including two `bappel.spr` entries,
with the final three entries belonging to profile 17. Commander retains its
125-entry table and existing decoder contract.

`big_bug_bang_writable_catalog_oracle.py` executes the unmodified traversal and
directory helper with guarded writes. Its DOS boundary reports that every
destination exists, so no copy code is entered. It captures all ordered names,
152 directory-helper entries and the single actual directory change. This is
an oracle for traversal, not a native copy or full startup oracle.

The catalog deliberately retains `blood.sav`, even though the sequel source has
no such file. Preparation must report that missing source locally, continue
with later entries, and never borrow Commander's save. The asset-backed test
uses the actual resource store and preparation coordinator with a filesystem
adapter and recorded graphics calls, not the still-guarded production startup
host. It checks copied bytes, source immutability, duplicate suppression,
preservation of an existing `script1.var`, and an idempotent second pass.

Verification: two native captures were byte-identical. All seven startup-focused
tests passed with original assets enabled. The real-resource case copied 149
files while preserving the preexisting state file, reported only missing
`blood.sav`, and copied nothing on its second pass. The game library passed
931 tests with 19 ignored on a private Xvfb display, which was reaped afterward.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_writable_catalog_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_writable_catalog.json
nix develop -c cargo test -p commander-blood-game --lib sequel_writable_catalog -- --include-ignored
```

### Sequel Navigation And Confirmation Tables

Game identity now selects confirmation hit regions, hyperspace names and
navigation labels/wipe geometry. The original sequel consumers establish:

- Confirmation calls at file 0x16E3 and 0x16F3 select data 0x27A7 and 0x27AF.
  Their rectangles are `[115,105,30,10]` and `[175,105,30,10]`, different from
  Commander's regions. The native pressed-pointer tester at 0x93F7..0x9424
  supplies 64 boundary cases, checked against the runtime's actual hit adapter.
- Hyperspace selection at 0x9D14..0x9D2F indexes eight 16-byte slots at data
  0x2170 (file 0x11960), masks the sequence counter by seven, increments it as
  a wrapping word, and copies the selected name into the presentation slot.
- Navigation label copies at 0x94E3 and 0x9504 consume the French prefixes at
  data 0x012D, 0x0137, 0x0142 and 0x014E. The wipe selector at 0xA028 addresses
  nine coordinate pairs at data 0x29E0 (file 0x121D0); the line routine reads
  both components at 0xAB10..0xAB16.

The guarded native capture records those hit results, four label byte strings,
nine endpoints and eleven travel selections including counter wrap. The
original-asset runtime test feeds the decoded clip names into the real camera
coordinator and presentation catalog, then loads every selected clip's bytes.
This checks selection and resource binding, not HNM playback, the complete
camera/navigation workflow, or the confirmation dialog's rendered labels.

The bridge-menu table cannot use Commander's five-command model.
The sequel list at data 0x27B9 has seven commands: `VITESSE`, `TEXTES`,
`VOYAGE_OFF`, `MUSIQUE_OFF`, `SAUVER`, `CHARGER`, `QUITTER`. Native selection at
0x9A6D..0x9B38 separately opens simulation-speed and text-speed controls and
toggles travel and music. The loader still rejects production sequel startup
until this model and the remaining runtime/profile contracts are integrated.
The French bytes are preserved source data, not completed English localization.

The table decoder now supports all seven sequel labels, both music and travel
toggle faces, the five subtitle speeds, the three simulation speeds, and the
shared cancel label. The game's fingerprint-guarded `decode_bridge_menu_text`
selects the correct data image; `OriginalGameData` no longer calls the
Commander-only menu decoder directly. Owned option labels have variable length,
and the label renderer replaces the music face at row 3 for BBB rather than
overwriting its text-speed row. Commander retains its five labels and row 1.

The subtitle pointer table is at data `0x281D`; the simulation pointer table
is at `0x282B`. The latter exposes its CS-authored `[100, 10, 1]` countdown
reloads alongside the labels. Initialized image values are text speed `2`,
simulation speed `1`, and travel low bit clear. These are decoded image values,
not a claim that all subsequent startup writes have been ported.

All 135 formats tests pass with original-asset tests explicitly enabled,
including direct checks of every menu label against the sequel executable,
malformed pointers/sentinels/terminators, and preserved speed values/flag bits.
Production sequel startup remains guarded until the command dispatcher and
its runtime owners are connected. This table change supplies data and correct label indexing, not an
interactive seven-command menu or an English localization.
The game all-targets check passes, as does the original-disc music-label test.
The full library passes 934 tests with 21 ignored on a private Xvfb display,
which was reaped after testing. The travel label reports the current flag
(`VOYAGE_ON` when set), unlike the music label's offered action.

Verification: repeated navigation captures were byte-identical, and regenerating
the earlier visual-table capture after the shared harness extension also produced
identical output. All 125 format unit tests and six format integration tests
passed with original-asset checks enabled. Both focused runtime tests passed,
including all 64 hit cases and 11 travel/resource selections. The game library
passed 932 tests with 20 ignored on a private Xvfb display, reaped afterward.
Game all-targets checking passed.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_navigation_tables_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_navigation_tables.json
nix develop -c cargo test -p commander-blood-formats --test sequel_navigation_tables -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_navigation_tables -- --include-ignored
```

### Seven-Command Options Handler

`update_sequel_option_menu` now implements the sequel's native command mapping
using the shared list-widget and panel-transition code. Commander Blood keeps
its existing five-command entry point and result type by default. The sequel
returns separate typed speed/travel commands and the five shared actions.

The command effects follow file `0x9A67..0x9B43`: simulation speed and text
speed set their activation and layout phases; travel toggles independently of
audio support; music is unchanged if support is absent; save/load activate
their shared panel and respective motion request; quit clears both pointer
button latches. Cancel closes without an action. A negative chooser result
leaves the menu open. Typed row resolution rejects the original ABI's
unreachable high-byte aliases.

The guarded original-instruction fixture has 80 cases: all seven rows,
cancel, two negative results, and every combination of music support, music
state and travel state. Rust tests compare both submenu phases, activation,
music/travel states, music label, save/load panel and motion flags, quit,
pointer latches, menu/modal ownership, and stream-start requests. Separate
tests check inactive ownership and transition waiting before action dispatch.

Music startup is deliberately captured at the original DOS far-call boundary
at `0x9AF9`; the post-driver branch resumes at `0x9B03` with the captured
globals. No machine code is patched or callback stubs installed. This proves
the request and menu effects, not DOS driver behavior or host audio playback.
The three focused tests pass, including the existing Commander options oracle,
and a repeated sequel capture is byte-identical.
The game all-targets check passes. The full library passes 936 tests with 21
ignored on a private Xvfb display, which was reaped after testing.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_options_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_options.json
nix develop -c cargo test -p commander-blood-game --lib options_
```

This is native handler verification. `RuntimeBridgeConsole` still uses the
Commander handler; it must exchange the sequel controls with the actual
simulation clock, camera/travel consumers, speed submenus and lifecycle before
production sequel startup can be enabled. English localization is also pending.

### Authored Text Corpus Audit

The offline `audit_sequel_text` formats example frames all seventeen COD files
with the existing lossless sequel parser, then reports A6 markers, flags, byte
positions and typed-decoder errors as structured JSON. It does not scan for raw
opcode bytes or claim that every framed instruction executes. BAS is outside
this audit's scope. It consumes the original loose resource import, not English
translations or generated substitute scripts.

```sh
nix develop -c cargo run -p commander-blood-formats --example audit_sequel_text -- \
  output/big-bug-bang/imported-assets/resources > output/big-bug-bang/text-audit.json
```

The original corpus has 6921 framed A6 tokens. The earlier dictionary-only typed
decoder accepted 6824 and rejected 97. Those numbers are a diagnostic snapshot,
**not a fidelity gate**: even accepted tokens can have wrong semantics.

There are 46 inventory markers in profiles 3 through 17. Every occurrence has
flags 32816 (`0x8030`), with marker 65534 at byte 12 relative to the A6 token.
Counts by profile, including the initial two zero-count profiles, are:
`0, 0, 4, 3, 2, 2, 3, 9, 3, 1, 3, 2, 3, 3, 3, 3, 2`.
The existing ordered sixteen-slot `AboardObjectRoster` is the appropriate
ownership model to investigate for this path, rather than a new global list.

The audit also found 58 marker-1 numeric substitutions. The sequel A6 spoken
path at 0x6D58 recognizes word 1, reads a word from the VAR image at the following
operand (0x6D66..0x6D6D), and calls signed decimal formatting at 0x2832..0x286A.
That helper checks the sign, emits a minus when needed and divides by ten.
The earlier dictionary-only decoder instead attempted DIC resolution for both
the marker and its operand. This explains the other 51 errors, but also hides
seven incorrect successes where the VAR operand coincides with a DIC entry:

| Profile | COD Token Byte | VAR Operand |
| --- | --- | --- |
| 4 | 16696 | 1724 |
| 4 | 16785 | 1724 |
| 6 | 11908 | 392 |
| 11 | 15221 | 2538 |
| 14 | 2683 | 466 |
| 14 | 7627 | 540 |
| 17 | 3629 | 1798 |

All numeric occurrences have flags 32768 except one with 32776; none explicitly
sets spoken flag 32. That is not proof that an inherited spoken-mode latch is
unreachable. The numeric menu comparison is described below.
The spoken path notably leaves SI at the VAR operand before dictionary-based
lookahead (0x6DA9), so it must not be rewritten as a guessed generic interpolation
loop. Runtime read ownership, spacing and cursor behavior must be verified.
Several numeric operands exceed their profile's DIC extent, while others land
inside unrelated strings. Do not pad the dictionary to make these reads appear
valid, and do not count current typed acceptance as recovery of numeric text.

The typed decoder now consumes marker 1 and its VAR operand together, only in
the sequel dialect. All 58 authored operands resolve to owned state words,
including the seven formerly incorrect successes above. The corpus test now
accepts all 6921 A6 tokens, including all 46 inventory-marker tokens. All 123
formats tests pass with original corpus checks enabled.

### Numeric Menu Renderer

`re/tools/big_bug_bang_menu_number_oracle.py` executes the original menu renderer
at 0x82C6 through 0x83E1, signed formatter, main-font drawing helper and width
helper without replacing callees. The checked-in 59 vectors compare text,
positions, widths, cursor, reveal counter, countdown, completion and DIC scratch
content against the Rust renderer. They cover signed extrema, zero, adjacent
numbers, punctuation, wrapping, partial reveal and retained scratch text.
This is a control/layout oracle, not a VGA-pixel oracle: planar hardware is not
emulated by this harness.

The renderer resolves each numeric operand from live VAR state and formats it as
signed decimal. Native reveal limits count encoded words, so marker plus operand
consume two positions. Lookahead to a number uses the previous scratch string
before formatting the next number; the Rust presentation owns that string and
the existing profile-change reset clears it. Normal menu publication retains it.

Spoken numeric text, numeric condition sections and numeric chatter hashing
return explicit errors until their distinct native paths are recovered. The
production sequel guard remains in place. These component checks do not prove
complete sequel dialogue, inventory selection, or playable startup.

Shared-engine verification: all-targets game check passes; the game-library
suite passes 913 tests with 12 ignored under a private X server. The ignored
numeric-menu test is run separately and passes all 59 native vectors.

### Explicit COD Dialects

`commander-blood-formats::code::ScriptDialect` selects the recovered instruction
boundaries. Existing entry points default to Commander Blood unchanged.
`decode_script_code_for_dialect` exposes Big Bug Bang framing. Tokens retain
their dialect so an adjacent-data byte from Commander cannot become a sequel
instruction merely because the numeric opcode matches.

Big Bug Bang A0-D2 descriptor pairs match Commander. D3-D7 use lengths 9, 5,
3, 5, and 1 in both query and normal modes, verified against the sequel's
native table at file 0x16AEA. Adjacent-data interpretation after D7 remains
unsupported rather than borrowing Commander's unrelated executable data.
All 17 original sequel COD images frame without raw fallback and re-encode
byte-for-byte. All five Commander images retain their existing token counts
and exact round trips. This is **instruction framing**, not full semantic
recovery, high-level script compilation, or runtime parity.

### D3 Multiply/Divide

The original handler at BLOOD2PG.EXE 0x7408-0x744A has a typed instruction and
ordinary Rust implementation, wired into production instruction dispatch:

```text
target = (unsigned_32(target) * multiplier) / divisor
```

Operands with mode C0 or C2 read VAR words; other modes supply immediate words.
All reads precede the destination write. Query mode does not suppress the write
or branch. Division by zero and quotient overflow are errors that leave state
unchanged, corresponding to the original DIV exception rather than silently
wrapping or saturating.

`re/tools/big_bug_bang_vm_oracle.py` executes the original handler, guarded by
the executable SHA-256, to generate 114 synthetic reference cases. The Rust
test compares the entire state buffer with those results, including 41 native
divide errors, aliasing, unsigned boundary values, and both query modes. The
fixture contains input/output data, not original game machine-code bytes.
Unicorn is used only by this offline oracle; the Rust game does not emulate
registers or segmented memory.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_vm_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_multiply_divide.jsonl
nix develop -c cargo test -p commander-blood-formats code::tests -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib big_bug_bang_multiply_divide
```

The sequel corpus test is explicitly ignored unless requested because it
requires local original-disc assets. Its absence must not be counted as a pass.
The synthetic D3 reference test and dialect boundary tests run normally.

Verification for this slice (2026-09-05): all 106 formats library tests passed
with ignored tests explicitly enabled; game library tests passed 882 with five
unrelated platform/oracle tests ignored; `cargo check -p commander-blood-game
--all-targets` passed. These checks do not prove sequel playability.

`cargo check --workspace --all-targets` fails in the existing script-compiler
wrapper's test build: shared `src/vm.rs`, `ship3d.rs`, `font.rs` and `descript.rs`
tests import root tools modules such as `recomp` and `bridge` that the narrow
wrapper does not expose. Those source files and the wrapper are unchanged in
this slice. Keep this as a separate test-ownership repair; do not disable tests
or count the failed workspace-wide gate as passing.

### D6 Actor Growth

The native D6 handler (file 0x728B-0x7366) and its selection helper
(0x706E-0x70CC) now have a flat, typed Rust implementation. The decoder treats
both operands as immediate words: a group mask and a signed growth rate.
All 39 authored occurrences are in SCRIPT2.COD; the other 16 profiles have none.

The helper selects actors in directory order with intersecting group flags,
both in-play/participating flags, an active location, and a location other than
the specially bound `Trashlando`. The handler clamps aggressiveness even for
engaged actors, then skips their growth update. Other selected actors receive
the recovered pressure, growth-balance and quantity arithmetic. These names
describe the observed calculations, not recovered original source identifiers.

Important native details preserved by the Rust implementation:

- Query mode does not suppress updates or consume a branch.
- Pressure relief has an upper clamp but no lower clamp.
- The balance calculation wraps at 16 bits before its signed clamp.
- Negative balance halves the unsigned quantity. Nonnegative balance uses two
  low-32-bit signed products followed by division of a zero-extended numerator:
  the native code explicitly clears EDX before IDIV. Replacing this with ordinary
  signed mathematical division changes negative-rate behavior.
- Growth has a minimum increment of one, including when its rate is zero.
- Final quantity addition wraps at 16 bits before a signed minimum of five.
- A word-DIV overflow preserves earlier actor updates and preceding clamps on
  the faulting actor. It is not an all-or-nothing state transaction.

`re/tools/big_bug_bang_growth_oracle.py` executes the complete original handler
and helper without replacing calls. Its 126 synthetic input/output vectors
cover selection, inactive locations, the excluded location, engaged actors,
both query modes, countdown gating, integer boundaries, negative rates and
18 divide faults. Tests compare the full VAR buffer, including partial fault
effects. The oracle also checks that the directory and all seeded globals are
unchanged. Neither game machine code nor original authored state is included
in the committed vectors.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_growth_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_growth.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_growth -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_growth
```

Production typed dispatch now handles D6 but requires an explicit
`SequelSimulationContext` from its host. Missing context is an error, not a
synthetic zero countdown. Tests exercise that dispatch boundary, clock gating
and query-mode writes. **The production sequel host is not yet implemented.**
The native main loop decrements GS:0x0CC6 at 0x10CA and reloads it from
GS:0x0CC4 at 0x5B46 after script/presentation processing. The clock ownership and
dispatch forwarding described below are now implemented; the interactive speed
control and remaining sequel runtime still need connecting. The handler must
not run independently at the renderer's presentation rate.

`SequelSimulationClock` now owns the two initialized words and exactly models
the original saturating decrement and zero-only reload. Forty unmodified-code
captures cover zero, boundary values and maximum words; a repeated capture is
byte-identical. `RuntimeGameLifecycleHost` advances the owned clock at the
start of each main-loop iteration, before input and pause gates, not from the
interrupt-timer tick count. `RuntimeScriptSystem` reloads after a successful
enabled pass, including a resume-boundary finish. Disabled and error returns
do not reload. Changing speed changes the next reload only.

The production script backend retains this clock across profile binding and
binds `Trashlando`, `arche`, `Arche`, and `Honk` from the current directory.
`ScriptExecutionService` now forwards simulation and settlement contexts to
D4-D6 instead of inheriting the absent-context default. Commander has no sequel
clock; missing sequel bindings still return absent context and dispatch errors.
An original-directory test checks all seventeen binding sets, clock changes,
disabled/enabled pass boundaries, and clearing stale identities. This verifies
the production backend/service boundary, not seventeen playable profiles.

The full-profile acceptance test was also attempted and **still fails** on the
transition from SCRIPT1 to SCRIPT2: `InvalidStateWord` at COD byte `0x5A97`,
VAR byte `8368`. The retained first VAR does not own SCRIPT2's `time` word;
the native allocation observations and remaining work below still apply.
The failing acceptance test is retained with an explicit known-incomplete
ignore reason. No zero extension, replacement VAR defaults, or fake state
binding was added. Production startup remains guarded.
The default game-library suite passes 937 tests with 23 ignored on a private
Xvfb display, reaped afterward; that ignored count includes the known failing
full-profile acceptance test. Game all-targets checking passes.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_simulation_clock_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_simulation_clock.json
nix develop -c cargo test -p commander-blood-game --lib sequel_clock_context_forwarding -- --include-ignored
# Acceptance gate, currently expected to expose the unresolved SCRIPT2 boundary:
nix develop -c cargo test -p commander-blood-game --lib sequel_simulation_clock_reaches_production_dispatch -- --include-ignored
```

The simulation-speed selection tail is now recovered and component-tested.
The menu at file `0x1D18` uses DS:`0x282B` for `LENT`, `NORMAL`, `RAPIDE`
and a final `0xFFFF` cancel pointer. Its CS-relative table at file `0x1D12`
contains `[100, 10, 1]`, written to DS:`0x0CC4` by the selected row. The
executable's initial value is `1`. These are countdown reload values, not
milliseconds or renderer frame rates. Cancel retains the old value and closes
the modal; a negative selection retains the value and leaves the modal open.

`PresentationChoiceItem::Value` allows the shared choice coordinator to publish
these explicit values while preserving its existing text-speed row mapping.
The guarded `big_bug_bang_speed_choice_oracle.py` runs original instructions
at `0x1D6F..0x1D90` with the original CS base and no patched callbacks. Its 90
cases cover every row, cancel, two negative selections, five previous values,
and three UI flag combinations. Tests compare result, activation, UI flags,
and the typed outcome. This captures only the selection tail, not the opening
transition or the simulation loop. **The seven-command menu and simulation
clock are still not connected to a production sequel host.**

```sh
nix develop -c python3 -P re/tools/big_bug_bang_speed_choice_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_speed_choice.json
nix develop -c cargo test -p commander-blood-game --lib presentation_choice
```

The five focused tests pass, including the original Commander Blood choice
vectors. A second speed capture and the existing startup-table capture are
byte-identical to their checked-in fixtures after the harness's CS-aware exit
check was added. These are component checks, not evidence of BBB playability.

Verification for this slice (2026-09-05): 110 formats tests passed with original
corpus tests explicitly enabled; 886 game-library tests passed with seven
unrelated/platform and original-table tests ignored. Game all-targets checking
and workspace library/binary checking passed. The original-handler vectors were
regenerated and compared byte-for-byte. These checks used the current worktree;
unrelated runtime edits remain outside the commit. This is D6 component and
dispatch verification, not a sequel playthrough or timing-parity claim.

### D5 Settlement

The D5 handler (file 0x7367-0x7407) now has a typed decoder and Rust
implementation. All 30 authored occurrences are in SCRIPT2.COD. Its immediate
group mask filters both source actors and relocated descendants; it is not a
VAR reference and is not implicitly replaced by the source actor's group.

The recovered path is:

1. Skip while the shared simulation countdown is nonzero. Otherwise select
   participating actors using the same 0x706E helper as D6.
2. Enable the maximum-range override. Require signed source quantity at least
   300 and a current location record.
3. Search active locations within the native squared range of 250, excluding
   the capitalized `Arche`. Choose the closest unoccupied location; ties retain
   the first directory entry. Source position resolution uses lowercase `arche`
   as its sentinel fallback. Candidate body coordinates are direct reads.
4. Collect active actor descendants of the source location in depth-first
   directory order, excluding `Honk`. This reuses the existing translated
   navigation collector and position resolver, now covered against their
   sequel counterparts in the complete D5 oracle.
5. Move matching descendants except the source actor. Copy the source's relief,
   assign quantity 10 and growth balance 1000. Only the first moved actor gets
   the participation flag. Mark the destination occupied and write the source
   actor into its new word at byte 24, not the first moved actor.
6. Clear the range override after processing. Query mode still performs the
   updates. A nonzero countdown preserves the previous override state.

Distance subtraction/absolute value wraps at 16 bits and the summed squares
are compared as signed 32-bit values, preserving the native overflow case.
The temporary candidate lists are owned vectors of object identities, not DOS
scratch-buffer or register emulation.

`re/tools/big_bug_bang_settlement_oracle.py` executes the original handler and
all seven helper entries it reaches: 0x706E, 0x6F17, 0x6F52, 0x67B8, 0x6633,
0x8103 and 0x685D. None of the calls are replaced. Its 100 synthetic graph cases
cover nested descendants, masks, source thresholds, flags, exclusions, equal
distances, radius boundaries, signed overflow, query mode and countdowns.
The fixture captures full VAR results and the observable range override;
the oracle separately rejects unexpected global writes outside the recovered
scratch areas. Reaching each helper is not a claim of covering every branch
inside each helper.

The same vectors pass through the production typed-dispatch implementation,
including record refresh. Every vector also verifies that omitted settlement
bindings produce an error without changing state. The production sequel host
still needs to supply these bindings and the real main-loop countdown.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_settlement_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_settlement.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_settlement -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_settlement
```

An inspection of all 17 original initial VAR/DEB pairs found zero active
actor candidates below location records before script initialization. Thus
running D5 on those initial snapshots alone would be weak evidence. Native
captures after initialization remain necessary for real-game state coverage,
including candidate-list bounds and repeated simulation updates.

Verification for this slice (2026-09-05): all 112 formats tests passed with
original corpus tests enabled; game-library tests passed 888 with seven
platform/original-table tests ignored. Game all-targets checking passed. The
100 original-handler vectors regenerate byte-for-byte. Tests ran in the
current worktree; unrelated Commander runtime edits remain outside this commit.
These checks do not establish production sequel startup or initialized-game
simulation parity.

### D4 Conflict

The D4 handler (file 0x70CD-0x724D) now has a typed Rust implementation and
production instruction dispatch. All 33 authored occurrences are in
SCRIPT2.COD. Its group mask and attack rate are unsigned immediate words.
The rate is retained even when the countdown suppresses execution or the
instruction runs as a query.

The recovered behavior is:

1. Select participating actors with the shared 0x706E helper. In query mode,
   any engaged actor succeeds; no engaged actor consumes the enclosing failed
   guard and clears query mode. Queries do not apply damage or acquire targets.
2. Unengaged actors require signed quantity at least 100, aggressiveness at
   least 200 and relief below 800. Search maximum-range locations in directory
   order, not nearest-first. A qualifying location's word at byte 24 identifies
   the opposing actor. Require opposing groups, in-play/participating flags and
   signed target quantity greater than 50. Link the opponents and mark engaged;
   an existing target back-reference is not overwritten.
3. Clamp relief and growth balance only from above. Compute aggression and
   damage with the original word wrapping, unsigned division and signed clamps.
   A word-DIV overflow reports an error with all earlier writes preserved,
   including completed updates to earlier actors.
4. Disengage when source quantity is at most ten, or damage lowers the target
   below ten. Clear the source's engagement and opponent link. If the target
   points back at the source, search for a replacement. With none, clear the
   target's engagement and relocate matching active descendants to the nearest
   free location within the current search range. Unlike settlement, retreat
   does not initialize quantities, balances or destination occupancy flags.

The replacement helper at 0x724E tests flag value 2, not Actor kind. Its byte-72
read on a non-actor record can alias a word in a following record. A native
oracle case establishes this behavior. Rust resolves that serialized position
to a checked owned state word; it neither invents an actor-only filter nor
emulates segmented memory. The extra actor word at byte 72 is now identified
as the opponent link by its reads and writes in this handler.

`re/tools/big_bug_bang_conflict_oracle.py` reuses the settlement runner and
executes the original handler and nine helper entries without replacing calls.
Its 124 synthetic cases include acquisition thresholds, existing back-references,
replacement candidates, non-actor aliasing, retreat range, both sides updating,
countdown gates, 18 failed guards and 13 divide errors. Rust compares the full
VAR buffer, attack rate, range override, query state and guard result. The
combined production-dispatch regression covers all 224 settlement/conflict
cases and rejects absent host bindings without silently substituting defaults.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_conflict_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_conflict.jsonl
nix develop -c cargo test -p commander-blood-formats sequel_conflict -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

These are component-level native comparisons, not initialized-game captures.
They do not establish full gameplay parity or production sequel startup. In
particular, the original scratch candidate-list bounds and repeated updates
still need verification with authored state after script initialization.

Verification for the conflict slice (2026-09-05): all 114 formats tests passed
with original-corpus tests enabled; all nine sequel game tests passed with
original-table tests enabled. The full game library passed 889 tests with seven
ignored using `--test-threads=1`. Its first, concurrent run terminated with
SIGSEGV; the cause has not been established or fixed. Do not report that run as
passing. Game all-targets checking and workspace library/binary checking passed.
Both settlement and conflict fixtures regenerate byte-for-byte from the
original executable; the settlement fixture is unchanged. Tests used the
current worktree, including unrelated Commander runtime edits that remain
outside this commit.

### D7 Ending and Changed CC Selection

The sequel's D7 at file 0x6E67 sets the ending latch at GS:0x6B73, in both
normal and query mode. This is **not** the inherited A8 `fin.*` latch at
GS:0x6B93. D7 itself does not request media or immediate shutdown. All four
authored D7 instructions are in SCRIPT2.COD.

CC at 0x69E6 retains the bounded sequence-name copy but adds a zero-based
selection request at 0x69ED. Commander CC only copies the name. All 97 sequel
CC instructions decode into the existing six-slot domain. Both new controls
survive the profile reset at 0x588F-0x5903, whose cleared ranges exclude them.
Empty CC names now disable their slot in the shared owned representation,
matching both the native first-byte test and the existing save restore path.

The Rust dispatcher now handles D7 and publishes CC's additional request only
for the sequel dialect. A separate `SequelPresentationControl` carries this
state into the existing production panel and ready-actor handlers:

- At queued-scene entry (0x8C14), a pending CC choice takes precedence over D7
  and mouse input. Without a pending choice, D7 suppresses ordinary primary
  input. Empty record slots have a separate native input path and are not
  unconditionally locked by this flag.
- A pending choice is consumed at panel initialization or queued-scene entry,
  not during inactive/opening/closing phases. Initialization selects that
  channel directly. Queued-scene entry selects it before the ordinary input
  path, which increments it unless reverse-closing. This native ordering is
  preserved rather than treating both paths as a direct channel change.
- When the full scene list completes (0x8C75), reverse/startup mode returns to
  transition. Otherwise an ordinary list closes; D7 instead publishes the
  next-frame shutdown request. Finishing an intermediate scene is not enough.
- The ready panel actor (0x92E4) selects its hand animation then stops for D7
  when no CC request exists. An explicit CC selection bypasses that gate and
  the animation selection. Disabled/not-ready actor behavior remains intact.

CC consumption is published before media callbacks, avoiding a stale end-of-
frame snapshot overwriting a new request. Commander receives no sequel control
context and keeps its established panel behavior.

`re/tools/big_bug_bang_presentation_oracle.py` executes original CC and D7
instructions and the three decision blocks above. Its 66 synthetic cases
compare instruction effects, retained slot bytes, query preservation and
branch destinations. The decision probes deliberately stop before external
media calls; they are **not** full original panel/actor execution or playback
parity tests. The runner checks all global writes and rejects writes outside
that region; it uses the VM's shared SS/data layout for BP-relative globals.
Rust tests additionally cover typed production dispatch, profile-reset
retention, panel request consumption, multi-line completion and actor effects.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_presentation_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_presentation.jsonl
nix develop -c cargo test -p commander-blood-formats --lib every_authored_sequel_panel_control_decodes -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

This is not a complete sequel presentation port. Initialized native captures
must verify authored ending lists, callback ordering and all media effects.
Sequel startup is still blocked by the unresolved loading work below, not
enabled by these component changes. Automatic activation is covered separately
below.

Verification for this slice (2026-09-05): all 115 formats tests passed with
original corpus tests enabled; all 13 sequel tests passed with original table
tests enabled. The full game library passed 893 tests with seven ignored,
serially under a private Xvfb display (`SDL_VIDEODRIVER=x11`, Wayland unset),
including SDL input and GPU tests without using the real desktop. Game
all-targets checking and workspace library/binary checking passed. The 66-case
native presentation fixture regenerates byte-for-byte. Tests used the current
worktree; unrelated Commander runtime edits remain outside this commit. The
earlier concurrent-suite SIGSEGV has not been diagnosed by a serial pass.

### Automatic Scripted Panel Activation

The complete helper at file 0x8A48-0x8A7C runs after presentation-mode updates
and before sprite geometry, hover and actor processing. It does not synthesize
pointer input or consume CC's pending request:

- Without a pending choice, or while the panel is already active, it does
  nothing.
- With the camera view active, it sets only the camera actor's auto-seek bit
  and clears the sequel simulation-overview flag. It does not arm the panel
  during this frame.
- Otherwise, unless the panel actor is already armed, it sets that actor's
  active and auto-seek bits and the shared redraw/modal bit.

The Rust bridge frame now calls this helper in that same position, guarded by
the active profile's sequel dialect. The camera handler also implements the
0x91F5 gate: a pending CC skips hand selection and primary-input clearing, but
still advances the camera line. After the camera closes, a later frame can arm
the panel; the request remains pending until the panel consumes it.

The new `re/tools/big_bug_bang_panel_activation_oracle.py` executes the entire
activation helper (no helper substitutes) for 384 combinations and the camera
gate for 12 combinations, stopping before its common line-playback call. It
checks retained global bytes as well as declared outputs; the two-byte stack
scratch used by the complete helper is accounted for explicitly. The 396-case
fixture SHA-256 is
`a30ba1c16df575adcec7d3b9fcc6992f56fb124a79bd86f67c73f7972de27cb6`.
Rust tests compare all these cases and exercise the camera-to-panel handoff
across updates with controlled line-completion feedback. They do not constitute
a full native media-playback or production sequel-startup test.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_panel_activation_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_panel_activation.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

The new overview state at native 0x2A30 is **not** the existing planet-choice
panel. Its separate controller at 0xA286 toggles an actor/conflict overview and
builds an actor roster. That controller and renderer remain unported, along
with the overview camera/hover guards and the camera-open secondary-input
write at 0x9252. The native initial overview byte is zero; owning that initial
state in Rust does not implement those missing paths.

Verification for this activation slice (2026-09-05): all 15 sequel tests passed
with original-table checks enabled; all 115 formats tests passed with original
corpus checks enabled. The full game library passed 895 tests with seven
ignored, serially on a private Xvfb display. Game all-targets checking and
workspace library/binary checking passed. Both the earlier 66-case fixture and
the new 396-case fixture regenerated byte-for-byte from the original binary.
These checks used the current worktree; unrelated Commander runtime edits are
not included in this slice. The previously documented concurrent-suite crash
and workspace all-targets test-import failures remain unresolved.

### Sequel Records and Profile Ownership

The formats crate now decodes sequel VAR records with an explicit dialect:
actors own 74 bytes and locations 26, versus Commander's 72 and 24. All 17
original VAR, DEB and DIC images round-trip exactly, with 184 objects per
profile. Their entire active-object directory prefix is identical, not just
the first few entries. Original field-table comparisons cover all 22 selector
rows and nine shipped object kinds. The inherited 21 rows match Commander;
the additional row selects the actor word at byte 72. The D4 recovery above
identifies that word as the opponent link; the generic format API retains its
field-index representation.

The resource cache can decode the sequel's 155-name catalog. The profile
manager can decode all 17 native rows and carry the dialect into code and
state decoding. It retains synchronized live VAR and timers across noninitial
sequel switches, releases the four other companions, and reloads initial VAR
when returning to profile zero. Repeating the initial selection retains live
VAR but resets timers, matching the native release/cache/reset conditions.
An out-of-catalog identity is rejected before changing the active profile.
Retained state requires matching active-object directories; a mismatch errors
instead of rebinding objects under different identities.

Synthetic, well-formed companion files test the real manager's resource and
timer lifecycle, including modified live state and repeated selections. These
are isolated test fixtures, not substitutes for the sequel's missing files.
The corpus and native-table tests require local original assets and are
explicitly ignored unless requested:

```sh
nix develop -c cargo test -p commander-blood-formats --lib -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

This implements loader components, **not production sequel startup**. Game
selection and dialect-aware runtime profile requests remain unwired. Missing
BAS ownership is now implemented below; entered unsupported dialogue still
produces a format error rather than silently skipping instructions.

Verification for the record/profile slice (2026-09-05): 108 formats tests passed
with ignored corpus tests enabled, all four sequel-specific game tests passed
with original-table tests enabled, and the full game library passed 884 tests
with seven ignored (including the two separately run sequel table tests).
`cargo check -p commander-blood-game --all-targets` passed. These commands ran
in the current working tree; unrelated in-progress runtime changes were not
included in this commit.

### Resource Binding and Initial Profile Loading

The complete resolver at file 0x5798 returns failure without changing DS:SI when
its handle is not resident. The VM binding loop at 0x5A64-0x5A97 resolves VAR,
DEB, COD, BAS and DIC in that order and publishes DS:SI without checking the
failure result. Consequently a missing BAS is bound to the preceding COD
resource, not to an empty BAS program or a universal SCRIPT2.BAS fallback.

`re/tools/big_bug_bang_profile_binding_oracle.py` executes that loop and the real
far-called resolver for all 32 resident-handle combinations. It also executes
the complete native allocator for eight three-allocation sequences, without
replacing helper calls. The latter establish exact paragraph rounding, including
an 8368-byte VAR allocation with no spare trailing word. These are synthetic
resident-table/allocation probes, **not a DOS filesystem or full startup oracle**.
Memory-write checks restrict each run to its outputs and call stack. The 40-case
fixture SHA-256 is
`a3f88f97e7b8e5280e6188081cd2a7be262add1f23dcc748b8fcc7ab0fb4bf57`.

Rust now loads sequel companions in native order, reports absent/empty BAS as
`Unavailable`, and binds dialogue ownership to the actual resident source ID.
`ScriptProfileDialogue` retains exact bytes and dictionary ownership; decoding
is cached on first dialogue use. Commander still requires valid BAS at profile
load. Missing essential sequel companions and I/O failures remain errors.
No synthetic resource is inserted into the cache or written to the game tree.

The actual initial sequel profile now loads through `ScriptProfileManager`,
with all COD instructions and records bound and VAR bytes unchanged. Its
dialogue image is exactly SCRIPT1.COD. Attempting to interpret that image as
the currently supported BAS grammar errors explicitly. Selector, menu, block
and object-text consumers validate only when their native path enters dialogue;
an idle frame does not force a BAS parse. This does not prove unused BAS data
is permanently unreachable or that the whole sequel can start in the UI.

```sh
nix develop -c python3 -P re/tools/big_bug_bang_profile_binding_oracle.py \
  output/big-bug-bang/disc/BLOOD2PG.EXE \
  re/tools/oracle_vectors/big_bug_bang_profile_binding.jsonl
nix develop -c cargo test -p commander-blood-game --lib sequel_ -- --include-ignored
```

Verification for this binding/loading slice (2026-09-05): all 20 sequel tests
passed with original-disc checks enabled, including actual initial-profile
loading. The full game library passed 899 tests with eight ignored, serially
under private Xvfb; all 115 formats tests passed with corpus checks enabled.
Game all-targets checking and workspace library/binary checking passed. The
authored-media census and seven Commander campaign tests passed with required
accuracy data enabled; the separate process-driven morning-oil test was not
run. The 40-case native fixture regenerated byte-for-byte. Tests used the current
worktree, including unrelated Commander runtime edits excluded from this commit.
Prior concurrent-suite and workspace-wide test-import issues remain unresolved.

### Live Original Startup Allocation Capture

`re/tools/capture_big_bug_bang_startup.py` now runs the hash-checked original
sequel as its own DOSBox-X child and periodically reads its guest memory. Each
run allocates a new private Xvfb display with `-displayfd`, unsets Wayland,
disables joystick input, and uses dummy audio. It never writes guest memory or
moves a pointer. An optional, recorded primary button press is restricted to
that private display. The original disc directory is mounted as a CD-ROM; only
a fresh output-specific C drive is writable. Children are detached and reaped
on completion or error. Captures refuse to overwrite an existing directory.

The module locator checks independent global-data, VM-code and resource-name
anchors. Native loading uppercases filenames in the catalog in place, so that
anchor comparison permits ASCII case changes, not arbitrary different names.
The helper resolves symbols from the running child ELF, not the Nix wrapper,
and rejects unverified emulator layouts. It reports pre-allocation startup,
partial bindings and ambiguous module matches separately from a bound profile.
Register flags are stored/lazy emulator values, not materialized CPU EFLAGS.

```sh
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-reference --seconds 60
# Optional isolated input, without pointer motion:
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-click-reference \
  --seconds 85 --click-after 35
python3 -P -m unittest discover -s re/tools \
  -p test_capture_big_bug_bang_startup.py
```

The first successful no-input run (`output/big-bug-bang/startup-capture-03`)
observed the initial profile bound from approximately 28 seconds through the
60-second endpoint. A second run (`startup-capture-04`) sent one private click
at 35.321 seconds and sampled through 85 seconds. Its final screenshot shows
the bridge TV area with the hand visible and the video off. It did **not**
reach another profile. All 111 bound-profile samples in that run agreed on:

| Role | Handle | Owning Resource | Allocation Bytes |
| --- | ---: | --- | ---: |
| VAR | 2 | SCRIPT1.VAR | 8368 |
| DEB | 3 | SCRIPT1.DEB | 8912 |
| COD | 4 | SCRIPT1.COD | 4160 |
| BAS | 5 | SCRIPT1.COD, same pointer as COD | No BAS allocation |
| DIC | 6 | SCRIPT1.DIC | 2480 |

The word at VAR byte 8368 read 24930 and belonged to **offset zero of the
resident SCRIPT1.DEB allocation**. It was not VAR padding or a hidden initialized
timer field. The captured VAR allocation was byte-identical to SCRIPT1.VAR,
SHA-256 `b2e07ec2f1bdd3acbe7798fbc2eecc2ea596e7bf58d5c82aeecc0c58a28776ea`.
This validates real startup allocation/binding behavior against the earlier
synthetic native probes. It does not establish what `inter3` later reads after
profile changes, compaction, resource reloads or save restoration.

Local evidence hashes (original memory/media remain ignored, not committed):

- `startup-capture-03/capture.json`:
  `67755f6eda15d4214f12c48599cf73d10124b6239c9a0e3cba0c09fdbd7b1c4f`.
- `startup-capture-04/capture.json`:
  `7713353d5c16ba5f342d20d07afc7f6027971dccbbf91f270b5e9b719820030f`.
- `startup-capture-04/state-0059.bin`:
  `1e5bc8a05bcbc49f0a24250a74b768373effde7b5d4fa4537c04f29a427fac5c`.

DOSBox-X identified itself as 2026.05.02, commit 5817c64; its ELF SHA-256 is
`05f568c04cbedb12f82ea5b89b0912724011471d5c7619555f4e97e100fc7157`.
Runs used the normal core at 30000 cycles and
`AMR S162227 EMS WRIC:\cblood\`. These are recorded capture settings, **not a
recovered sequel installer command line**. Periodic ptrace stops also mean
these are not real-time timing or frame-rate reference measurements.

The 13 synthetic helper tests cover allocation ownership, retained VAR handles,
missing BAS aliasing, unowned adjacent bytes, incomplete bindings, independent
anchors, case conversion, ambiguous modules, changed VAR snapshots, truncated
reads, invalid durations, and private input routing. No production Rust files
change in this evidence slice; the full sequel port and localization remain
unfinished.

The final tool revision was rerun without input for 40 seconds in
`startup-capture-05`: 80 samples, including 25 bound-profile observations,
reproduced the same owners and adjacent word value. The run exited successfully
and its DOSBox/Xvfb children were reaped. Rust suites were not rerun for this
offline-tool-only change.

## Native Ownership Evidence and Open Questions

Inspection of the sequel loader at file 0x5820 established a different load
order: VAR, DEB, COD, BAS, DIC. The name catalog starts at file 0xED94;
the 17 profile rows start at file 0xF744 and hold five two-byte resource IDs.
The first row contains IDs 2-6; resource 2 names SCRIPT1.VAR, not COD.

The selector uses FS:0x15B4 at 0x5853 and scales the profile index by ten.
For a nonzero requested profile, 0x5867-0x586D skips the first resource and
loads four instead of five. Its release path at 0x582E-0x5842 likewise releases
four, except when selecting zero. Thus VAR ownership persists across noninitial
profile switches. Do not reuse Commander's wholesale state replacement.
The native timer/state-table reset at 0x587C-0x588C is also initial-profile-only.

At 0x5A7D-0x5A97, resolved pointers follow the same order: the main COD loop
loads from GS:0x6AF4 at 0x5AAF, and the old-style conversation scanner still
loads BAS from GS:0x6AF8 at 0x5BBA. The loader's resource loop lacks Commander's
per-resource zero-result rejection. Only SCRIPT2.BAS is on the disc. The
missing-resource binding is established above, but actual conversation-entry
reachability still needs native validation. Do not infer that the shipped
SCRIPT2.BAS is used with the current profile dictionary: its first menu fails
current dictionary binding at BAS byte 6, dictionary offset 0x1F00.

The call to the old-style conversation scanner at 0x5E66 is gated by actor
field selector 2 (byte 26), presentation context and object flags. Trace its
reachable callers and field writes before deciding which BAS resources matter;
initial field values alone cannot prove that conversations are unreachable.
A typed authored-script audit found all 61 actors' initial byte-26 words zero
in all 17 profiles and no COD writer to those fields. That constrains authored
behavior; native writers and retained live state still need checking. Do not
confuse selector 2 / byte 26 with dialogue-control selector 15 / byte 68.

SCRIPT2.VAR has an extra trailing word named `time` at byte 8368. The initial
VAR image is only 8368 bytes, yet the native loader retains it when selecting
noninitial profiles. The native allocator does not reserve that extra word.
The authored audit found one read, `time > 19` at SCRIPT2.COD byte 0x5A97 in
`inter3`, and no COD initialization. In the initial allocation order, bytes
after VAR belong to the next resource, DEB. Later compaction/residency can
change that neighbor; a synthetic initial allocation alone cannot prove the
value observed during the conversation. The live startup captures above confirm
that initial adjacency but do not reach `inter3`. Trace native writers and the
later profile transitions before deciding its flat-state representation.
Do not copy the second profile's defaults or silently zero-extend initial state.

### Recorded Reference Input

The startup capture accepts repeated, strictly increasing `--click-after` times
and an optional `--click-position X Y` on its private 800x600 X display. It saves
a screenshot before each click (`before-click.png`, `before-click-2.png`, etc.)
and records scheduled and actual request times. A requested pointer move is not
reported as verified movement. Repeated targets omit `xdotool mousemove --sync`:
the original `startup-capture-08` failed because that option waited for movement
when the pointer was already at the target.

`--relative-mouse` enables DOSBox-X autolock and, if needed, sends a separately
recorded acquisition click before the first intended click. It verifies the
child emulator's one-byte `mouselocked` symbol before proceeding. It does not
modify guest memory. The official
[DOSBox-X mouse guide](https://dosbox-x.com/wiki/Guide%3AMouse-support-in-DOSBox%E2%80%90X)
describes autolock; this build did not provide capture status in its window
title, and the tested Ctrl+F10 attempt did not set `mouselocked`. A failed capture
can have partially delivered input; `status: requested` is not evidence that
no input occurred.

```sh
nix develop -c python3 -P re/tools/capture_big_bug_bang_startup.py \
  output/big-bug-bang/disc output/big-bug-bang/startup-relative-reference \
  --seconds 90 --click-after 35 --click-after 60 \
  --click-position 160 300 --relative-mouse
```

The above replay was run as `startup-capture-13`: mouse capture was verified,
both intended clicks were sent, and the 90-second screenshot still showed the
original introduction. Only profile 0 was observed, with the same extra-word
address owned by `script1.deb` and value 24930. Its `capture.json` SHA-256 is
`821c1be535a39f598571a242334f631e8c7440998574c876bcb8b25767f16b80`.
The uncaptured repeated-click replay `startup-capture-09` also completed, without
reaching SCRIPT2. These are input/capture checks, not gameplay acceptance.

A no-input extension, `startup-capture-14 --seconds 300 --interval 1`, completed
300 samples through 300.311 seconds. The final screenshot showed a later space
scene and narration, so the introduction was visibly advancing; it still had
only profile 0 and the same `script1.deb`-owned extra word. Its `capture.json`
SHA-256 is `56dd42db4fe1ad8f2a8f1366e52288665d953571adf4ad3f67c6de6677d8f0a9`.
This does not show the Honk `JOUER` choice or a native SCRIPT2 time-word read.

Samples now include `mouse_poll`, the shared words at global offsets
`0x0c22`, `0x0c24`, and `0x0c26`. The original module routine at `0x0709`
stores INT 33h CX, DX, and BX there. Later game code adjusts the same slots,
so periodic values are not a raw driver event trace or proof of a hit-tested
selection. The capture's 22 unit tests cover schedule validation, private-display
targeting, pointer bounds, capture-state validation, and the existing allocation
checks. Original normal-input navigation into SCRIPT2 remains unverified.

### Authored Translation Source

`sequel_text_catalog` now exports every typed A6 instruction from the 17 authored
COD resources, using the existing sequel decoder rather than scanning for text
bytes. The catalog has 6,921 message sites and retains each site's profile/COD
identity, line-record reference, presentation selector, control flags, resume
target, and record-condition operand. Explicit sections preserve condition/menu
boundaries. Their typed parts retain dictionary byte identities and original
word bytes, plus 58 state-number and 46 inventory substitutions. The joined
section `source` is a reading aid, not replacement executable text.

```sh
nix develop -c cargo run --bin sequel_text_catalog -- \
  output/big-bug-bang/imported-assets/resources \
  > output/big-bug-bang/authored-cod-text.json
nix develop -c cargo test --bin sequel_text_catalog -- --include-ignored
```

The export records SHA-256 hashes of each COD/DIC pair. Independent validation
matched its per-profile counts to the existing text audit and all 44,599 word
references to their original dictionary bytes. The synthetic structure test and
the opt-in original-corpus test both pass. Repeated exports are deterministic.
`cargo check --workspace` and `cargo check -p commander-blood-tools
-p commander-blood-game --all-targets` pass. Forcing `--workspace --all-targets`
also builds the script compiler's embedded shared-source tests, which fail with
17 unresolved-import/type errors. The same failure was reproduced in a clean
detached worktree at pre-export commit `1d3b90a6`; the compiler already declares
its unit tests owned by the tools crate. This change does not alter that setup.
There are 2,614 sites with the spoken-text flag and 2,111 distinct first-section
reading strings among those sites; these counts do not establish reachability
or complete contextual utterances across multiple instructions.

Five word occurrences in three messages contain byte `0xef`, outside the
repository CP437 decoder's supported range. They remain explicit `\\xef` escapes
with original bytes attached, not replacement characters or guessed corrections:
`bbb.script6.cod.000023bc`, `bbb.script6.cod.00003aa0`, and
`bbb.script12.cod.00001b9c`. An English editor must resolve these in context.

This is authored COD translation source, **not completed English localization**.
BAS dialogue, native UI/object display names, and media-embedded text are
explicitly excluded in the output. Runtime localization, translated choice
labels that preserve concept IDs, font/wrapping checks, and timing remain work.
Generated original-text catalogs stay under ignored `output/`, not in the repo.

The longer no-input reference, `startup-capture-15 --seconds 600 --interval 1`,
completed 600 samples through 600.625 seconds, still only in profile 0. Its final
screenshot returned to the TV narration about Terra also visible near startup;
waiting alone did not reach the Honk start conversation. The extra word remained
24930 in `script1.deb`. Capture JSON SHA-256:
`63aabc44a9c7977145ca7e38d27f0b28b7ba4eb1935d26b2163a875d575f696b`.
The next reference step is deliberate bridge navigation, not another longer
no-input run. SCRIPT2 acceptance remains unresolved.

## Production Loader and Native Save Defaults

The production path now selects the executable and title from game identity,
checks the analyzed sequel executable hash before media normalization, and uses
the sequel's writable namespace by default. The SDL window title also identifies
the selected game. Commander keeps its editable, byte-verified script compiler;
BBB loads its original typed COD/DEB/DIC/VAR/DESCRIPT resources without compiling
Commander sources over them. Editable sequel source remains unfinished.

With no writable `BLOOD.SAV`, BBB retains the ten native slot records embedded
at executable file offset `0x1206B` (GS:`0x287B`). This is not an invented empty
file: the optional native loader returns without changing that table on open
failure, and seven live startup snapshots match all 320 bytes. The modern
fallback stays in memory until an actual save-directory write is requested.
An existing user directory takes precedence; malformed user data remains an
error and is not overwritten. Complete sequel save/load behavior is unverified.

The real-asset bootstrap test loads the initial profile, MANU3, bridge panorama,
and startup resources, then checks native default slots, explicit persistence,
existing-data precedence, malformed-data preservation, and unchanged source COD.
The only startup-copy diagnostic is the absent `BLOOD.SAV` resource. Both the
bootstrap test and the native snapshot comparison pass when explicitly enabled.
The game library passes 940 tests with 28 ignored using `--test-threads=1`;
`cargo check --workspace` passes. The previously recorded parallel Vulkan crash
is not resolved by these serial results.

The captured secondary-click experiment (`startup-capture-17`, 65 seconds)
verified private mouse capture but still ended in profile 0 on the TV sequence.
It does not establish an exit from the opening or acceptance of SCRIPT2.

The first windowed run exposed a Commander-only builtin name: navigation-chart
setup required `Ark`, but BBB calls that object `Arche` (distinct from lowercase
`arche`). Native file `0x5995` resolves the capitalized name and stores its value
at GS:`0x6B28`. Binding is now dialect-specific. Five captured post-binding
startup snapshots contain the expected value 4624; the two pre-binding snapshots
still contain zero and are not used as completed-binding evidence.

All 711 HNM resources have generated verified lossless WebM derivatives. The
modern `modern-startup-04` smoke run exited successfully after 292 opening
presentation frames and five main-loop frames in profile 0. Screenshots from
the preceding run show changing opening-movie artwork. This is production
startup evidence, not sustained gameplay, full audiovisual parity, save/load,
or an English playthrough. The native builtin snapshot test is explicitly enabled
alongside the other original-asset tests.

The longer `modern-startup-05` run also exits successfully: 292 opening frames
plus 600 main-loop frames, still in profile 0. `screen-013.png` shows the active
character presentation and its `Hello!!!` caption. Its trace subtitle field only
contains empty text and `WAIT COMMANDER ...`, so this run does not establish
the English dialogue override in live play. Direct original/modern startup
sequence comparison and deliberate navigation remain necessary.

## Remaining Completion Requirements

- Compare inherited VM handlers, including
  skip, state, presentation and conversation semantics. Integrate the native
  simulation countdown lifecycle required by D4-D6. Add native oracle coverage.
- Complete production startup and runtime profile changes using game identity
  and the recovered sequel catalogs/layouts; resolve missing-resource
  behavior and the extra SCRIPT2 state word before claiming complete loading.
- Recover the actual conversation representation and produce readable,
  hand-editable French source with byte-exact COD/BAS/DEB/DIC/VAR/DESCRIPT
  reproduction where those resources are active. No raw fallback as completion.
- Port changed native simulation, travel, interface and presentation behavior;
  compare AMER/CROOLIS routines and assets. Validate new media through the
  library-only import path and existing SDL3/wgpu rendering.
- Provide game selection and separate asset caches, save identities and source
  checksum manifests so the games cannot contaminate each other's state.
- Extract contextual complete messages and UI text into a stable localization
  catalog, translate French to English, preserve logical IDs, and verify English
  rendering, wrapping, interaction and subtitle timing. An English first pass now
  covers the opening profile's 89 A6 sites in
  `localization/big-bug-bang/en/script1.json`, with source-hash, site, section, and
  choice-count validation. The COD backend now substitutes English after accepted
  subtitle publication, preserving semantic/choice words and falling back on
  nonmatching resources. Menu-only prose and choice labels are not rendered in
  English yet; accepting BBB in the loader does not establish a playable English
  release. The other 16 COD profiles
  and the non-COD text remain untranslated. See
  `localization/big-bug-bang/README.md` for the validation command and editorial
  limitations.
- Capture the original sequel in DOS and compare Rust behavior through startup,
  dialogue, travel, added gameplay and completion paths. Keep Commander regression
  coverage running alongside it. No whole-game parity claim from format tests.

Each item remains part of the full objective; completing the decoder or one
handler does not redefine the deliverable as a compatibility-only tool.

## PLAY Panel Comparison (2026-09-06)

The TV request after PLAY is present in the original, not evidence of a script
regression. In the local `native-play-05` capture, `after-07.bin` is still profile
0. The first profile-1 sample, `state-0194.bin` at 111.132 seconds, has pending
choice 2 (zero-based), panel actor flags 9, and sequence slot 3 named `1ppit`.
Samples remain at panorama frame 45 until 121.654 seconds, then turn to frame
88 and reach 90 at 122.155 seconds. The test pans back at 135 seconds; comparing
only its later snapshots incorrectly hides the automatic turn.

The Rust `modern-panel-request-01` capture similarly changes from no pending
choice and panel flags 1 to choice 2 and flags 9 on the SCRIPT2 load (frames
1311-1312), without a pointer press. A follow-up instrumented capture locates
the assignment at COD offset `0x9ED7`, outside query mode. Runtime traces now
include the last CC instruction offset and query mode; this provenance resets
on profile replacement while the session-owned pending choice is preserved.

The native UI word's `0x20` panorama-band bit is represented separately by
`PresentationBridgeMode::FirstBand` in Rust. Comparing that raw native word
against Rust's low UI bits alone is not a valid state comparison.

These observations do not establish cryobox interaction or whole-game parity.
The corrected ordinary-input comparison is retained as
`accuracy/scenarios/bbb_play_daddy.tsv`. The local `modern-play-daddy-01` run
completed 2834 recorded frames: it returned to panorama frame 45, rendered the
cryobox list with Daddy Gluxx, and closed the list after a click at (100, 100).
That click selected cancellation, not Daddy: the single contact occupies
y=89..99 and the cancel row begins at y=100. The native helper's nominal
(100, 100) motion actually settled at y=99. The retained scenario now uses
(100, 94), inside the contact row, and the boundary is covered by a focused
choice-list test. The earlier run's failure to start Daddy does not establish
a scene-handoff defect.

The corrected `modern-play-daddy-02` capture completed normally with 2834
recorded frames. Its final state has profile 2 (SCRIPT3) and active actor
`Daddy_Gluxx` (record 44). Screenshot `screen-026.png` visibly renders Daddy's
scene and the caption `Ageu rla... Mmeuh`. This verifies the ordinary-input
opening, PLAY, TV completion, return-pan and first cryobox-contact path. It
does not verify later dialogue, travel, completion, or the remaining English
translation.
