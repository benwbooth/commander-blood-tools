# Gameplay Video Anthologies

`tools/video_anthology.py` provides a first, scenario-driven export pipeline for
Commander Blood and Big Bug Bang. It records the Rust port's actual final GPU
output and SDL mixer submissions. It does **not** run the DOS executable or prove
DOS rendering/timing parity. Dialogue discovery uses a separate static script
scan; no game process or simulated clicks are needed for that scan.

## Normal-Playthrough Flow Work

The requested final scope is a successful playthrough of each game plus distinct
alternatives reached through legitimate choices, including every optional
conversation topic reachable through normal play. Shared material should appear
once; optional dialogue must not be silently dropped to meet a duration target.
The final runtime remains to be measured. This is not the Cartesian product of
actors, destinations, inventory, visit counts, and evolution values. The existing
selected-branch anthology masters are **not** complete normal-playthrough videos.

The order of work is flow discovery and validation first, rendering second.
`tools/native_game_flow.py` runs the production native executable with only
ordinary input scenarios, a fresh private writable directory, and no recording
option. It concurrently consumes the native frame and action traces through
FIFOs, retaining compact scene changes and object-state deltas rather than
writing raw per-frame traces, video, or PCM.

```sh
nix develop -c cargo build --release -p commander-blood-game \
  --bin commander-blood --bin big-bug-bang
nix develop -c uv run tools/native_game_flow.py --game cb \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --scenario accuracy/scenarios/production_bob_first_contact.tsv \
  --out output/game-flows/cb-first-contact
```

Each attempt retains `flow.md`, `flow.json`, `events.jsonl`, `actions.jsonl`,
the exact scenario, and the game log. The manifest hashes the native executable,
asset manifest, recorder, scripts, and output evidence. Actual assets are checked
against the import manifest before the run. A failed route stays failed; there
is no fallback to forced contacts or prepared inventory.

Normal-flow scenarios reject `contact`, `teleport`, and `alien` injection commands.
They also reject observed save loads without predecessor validation and the CB
script cheat menu. `choose LABEL` waits for a uniquely matching, rendered,
selectable menu row and performs an ordinary pointer click on that row. It does
not assign the script choice variable or invoke an inventory transfer directly.

Use repeated `--then fragment.tsv` arguments to append input fragments in order.
The combined scenario is replayed in one process with carried state; each
fragment's content hash is retained. The final observation includes the actual
object locations/inventory and bridge/VM readiness flags for endpoint checks.
Action records also retain menu/navigation state and bridge hit regions. An
input script reaching its last action does not establish its named quest result;
check the actor, destination, item holders, and presented scenes explicitly.
New traces also retain named script-global word changes, with source offsets
and raw/signed values. The recorder stores only changes and the final snapshot;
older traces without these fields do not claim global-state evidence. This is
read-only observation of the synchronized native VAR state.
New recordings also retain ordered `SQ\\` sequence runs and the maximum decoder
counter observed before each source closes. These counters survive event
compaction but are not total frame counts or durations: a source can decode its
last frame and clear its counter in the same native tick. Replaced/restarted
sources are distinguished from closed sources; no per-frame media is retained.
Inputs completed inside a synchronous minigame are queued until a bridge
snapshot is available. Each retains its actual completion clock and explicitly
marks a deferred semantic observation; those snapshots are not minigame-frame
state evidence.
Action trace schema 2 retains each input and its clocks, but replaces an exactly
repeated `scene`, `bridge`, and `state_array_hash` snapshot with `snapshot_ref`.
Its `record_index` is a zero-based JSONL record index pointing directly to the
full snapshot, never another reference. Resolve those three fields from that
record; all other fields belong to the current input. Unchanged deferred
minigame observations therefore do not duplicate a full bridge snapshot for
every flight input.

`alien-first-frame-trace ... navigate` is an offline input planner for Jo's
collection minigame. It reads scene geometry to steer toward a Bioxx and then
back into a Manta, emitting only logical `move` and `key` actions. It never
changes inventory, score, quest flags, or native flight behavior. The first
neutral frame corresponds to `await-alien`; the emitted actions begin on the
next frame and end with Escape when ten bionium are reached or the frame limit
expires. `navigate:TARGET` selects another positive total collection target;
the trace records that target and the actual returned total separately. It does
not grant any doses or change the game runtime. Planning is not a normal-flow
witness or original-executable parity
proof: replay the emitted actions through ordinary Jo entry and inspect the
resulting dialogue, global `vbio`, inventory, and saved checkpoint.
The planner's timing argument must match the witnessed starting `vbio` value.
Reusing a zero-start plan at two doses caused an early minigame exit in
`bbb-jo-paul-resupply-v1`; that interrupted attempt is excluded. Always check the
planner's actual returned total, which can be below its requested target, before
replaying the generated inputs.

```sh
target/release/alien-first-frame-trace amer \
  output/big-bug-bang/imported-assets/resources/AMER.XDB \
  /dev/null full 0 100000 navigate > output/game-flows/oracles/jo-input-plan.json
jq -r '.input_actions[]' output/game-flows/oracles/jo-input-plan.json \
  > output/game-flows/oracles/jo-input-plan.tsv
```

`--resume-from path/to/flow.json --slot 0` permits one normal UI load from a
recorded predecessor. The preceding run must have witnessed a successful save
in that slot. Its evidence, save-directory record, save bytes, source profiles,
asset manifest and executable hashes must still match. Earlier lineage is
validated recursively. Only the witnessed save and its native slot directory
are copied into the new private writable directory; the scenario still has to
select the correct slot through the normal load menu. Failed predecessors,
arbitrary imported checkpoints, changed saves, different binaries, missing
save events, and an additional unvalidated load are rejected.

A verified runtime repair can use `--runtime-update "reason for the repair"`
to permit an executable change explicitly. The continuation records both binary
hashes and the reason. All save bytes, source profiles, assets, and predecessor
evidence must still match; the save is loaded only through the normal game menu.
Historical runtime changes are validated against their recorded hashes rather
than silently treating the entire lineage as one build.

The chapter-five Cyberquizz attempt exposed a skipped-token defect: an A9
procedure header's descriptor still enters query mode even when its handler
is skipped. Without that mode change, the following exam-score comparison
became an assignment and the opening line repeated. Four new original
`BLOODPRG.EXE` `0x62B6` vectors verify both header flags and both incoming modes.
The port's added A9 skip test failed before the repair; afterward the game
library passed 1,115 tests (78 ignored), the format library passed 118 tests
(10 ignored), and all 21 token-advance oracle vectors matched.

Two later normal-route failures exposed presentation edge cases. BBB's map
must exclude aboard actors even when their simulation flags include 4 or 16;
the original `BLOOD2PG.EXE` roster builders exclude both cases before using the
holder. The overview oracle now executes 34 cases, including both aboard
variants and an opponent whose position resolves to the Ark. The initialized
profile regression failed before the runtime adapter was repaired.

CB's Big Band location deliberately has an empty DESCRIPT caption. Two added
`BLOODPRG.EXE` `0x93F5` probes show that terminal empty text retains the frame
and arms the ordinary hold unless the ship gate is set. The native subtitle
test reproduced `EmptyText` before the fix; an empty caption now emits an
empty line with that timing, without adding text or skipping the frame.
All 13 CB and 12 BBB reveal vectors pass. The game library passes 1,115 tests
(78 ignored), both asset-backed overview tests pass, and the flow-recorder
suite passes 21 tests. These repairs do not establish that either complete
route or its alternatives have finished.

BBB endings can terminate the executable during the final passive wait.
`--expect-bbb-ending 0x9f14`, for example, requires the native SCRIPT2 ending
assignment at that exact code offset, a drained video source, a clean process
exit, and all preceding actions completed. No pending choice or navigation
action may be skipped. Such a run is an `observed_ending`, not a successful
story-completion claim or a resumable route checkpoint. Without this explicit
expectation, incomplete action lists remain errors.

The flow keeps subtitle and inline-menu text separate. A fully revealed text
site requires the native glyph-raster audit to match, not just a published VM
offset. It also requires a live actor presentation: navigation captions can
retain an old COD offset after the dialogue closes. Older manifests without
`text_site_attribution` predate that guard; their raw events must be re-audited
before using the summary counts for coverage, without rewriting their hashed
lineage. Loaded DESCRIPT bindings and videos with observed decoded frames are
separate evidence. Frame boundaries and wall-clock observation times are not
video presentation timestamps. A native route witness is not a DOS-parity proof.
Route-exit credits do not establish that the story ending was reached.

All current manifests deliberately retain `full_game_complete: false` and
`all_normal_branches_complete: false`. Initial route probes are not the complete
flows. Both successful endings are now witnessed below; alternate-branch
coverage remains incomplete, so the new final videos have not been rendered.

`tools/native_flow_coverage.py` consolidates those witnesses without replaying
games or generating media. It hashes every included flow artifact, checks
ordinary action completion, validates predecessor manifests, saves, assets,
scripts, and explicit runtime changes, and rechecks the successful-ending gates.
It reads the compact events again rather than trusting old text-site totals.
Full dialogue evidence requires a live actor, lifecycle activity, matching glyph
pixels, and rendered text matching the source site's spoken text. Inline text
uses the actual dictionary-token spacing; English numeric markers may match
displayed integers, but that match does not verify their values against DOS.

```sh
uv run tools/native_flow_coverage.py \
  --flows output/game-flows \
  --catalog output/anthology/static-dialogue-en-v2 \
  --cb-success output/game-flows/cb-wedding-concert-v2/flow.json \
  --bbb-success output/game-flows/bbb-concert-ending-v2/flow.json \
  --out output/game-flows/audit-v2
```

The output directory must be new. Its `coverage.json`, `cb.md`, and `bbb.md`
retain 254 audited witnesses and order the successful lineages into 72 CB and
99 BBB segments. Each continuation starts at its observed normal load; a parent
segment stops at the exact save used by the child, excluding later actions.
These are evidence boundaries, not movie timestamps or duration estimates.
Nine early probes lack the lifecycle/load observations needed by this audit
and are listed separately, as are 94 failed or unfinished attempts.

| Text-site evidence in `audit-v2` | CB | BBB |
| --- | ---: | ---: |
| Witnessed on the successful route | 1,569 | 3,013 |
| Witnessed on other normal routes | 267 | 175 |
| Unobserved site with identical witnessed wording | 404 | 421 |
| Unobserved empty/control text | 129 | 610 |
| Other unobserved wording or dynamic site | 3,167 | 2,702 |

Counts include UI/control text and repeated wording; they are not unique spoken
scene counts. The audit rejects two retained-offset misattributions: BBB
SCRIPT4 COD 8426 displayed the answer labels instead of the attributed sentence,
and CB SCRIPT5 COD 4285 displayed a different waiting message. Their original
hashed witnesses remain unchanged. Equal wording elsewhere never marks an
unobserved site reached, equivalent in visuals, or globally unreachable.
The ledger preserves recorded sequence runs without claiming source-frame or
caption completeness. It deliberately remains `render_ready: false`.

The original BBB startup diagnostic now accepts bounded `--key-after SECONDS KEY`
events on its own isolated X display. Each event identifies the child window,
records a read-only held-key observation, and releases the key even when that
observation fails. Only `F7`, `Return`, `Escape`, and `space` are accepted; no
guest-memory writes or desktop input are added. All 35 capture-tool tests pass.
`startup-capture-24` pressed F7 while the television was open;
`startup-capture-25` first closed it with an ordinary click, then pressed F7.
Both remain at profile zero. These diagnostics neither resolve the numeric
chatter blocker nor establish a normal original-game route beyond startup.
The native F7 handler's profile request is gated to current profiles above one;
it cannot be assumed to bypass the initial profile.
The follow-up movement support records `--move-after SECONDS DX DY` separately
from clicks and requires a confirmed private mouse capture. Read-only snapshots
include the original bridge steering fields checked by the existing `0xADF5`
oracle. All forty capture-tool tests pass. `startup-capture-26`'s single large
movement did not change frame 90; `startup-capture-27`'s smaller movements reached
frame 79, confirmed in both guest state and the screenshot, but still profile
zero. Neither a sent movement nor a captured-host flag proves the intended
in-game camera position or contact was reached.
`startup-capture-28` tested private host-pointer recentering without advancing
the camera; that experimental option was removed. Its recorder snapshot is
retained with the diagnostic. Captures 29 and 30 used an explicitly hashed,
process-local classic SDL2 library, with its mapping verified and version
2.30.6 reported by DOSBox-X. Neither normal relative mode nor requested SDL2
warp mode progressed beyond startup. No system library was replaced.
Capture 31 adds read-only emulator cursor fields: both capture flags were true,
while the last motion's host X coordinate reached the window edge. The camera
reached only frame 79. This localizes the input problem without establishing
its cause. The tool records its own hash, accepts the library/mouse-mode
overrides explicitly, and passes all 45 focused tests.

The optional `--recapture-after SECONDS X Y` diagnostic uses DOSBox's normal
Ctrl-F10 shortcut to release capture, move the pointer on the private display,
and recapture. It checks each lock transition, restores capture if pointer
movement fails, and never writes guest memory. Captures 32 and 33 both stopped
because mouse release was not confirmed, before recentering or later inputs.
Capture 33 held the shortcut for 150 ms rather than sending an instantaneous
key sequence; neither attempt progressed beyond profile zero. The exact
earlier recorder is retained in `startup-capture-32/recorder.py`. All 51
capture-tool tests pass, but these attempts did not verify live recapture.
This does not resolve the numeric-chatter issue.
Capture 34 repeats the held-shortcut diagnostic with the explicitly hashed
classic SDL2 2.30.6 override. It also fails to confirm release before any
recenter operation and remains at profile zero. The library change is not a
solution or evidence of an original normal-playthrough continuation.

Capture 35 resolves each shortcut key to its base-level X keycode on the
private display and verifies the keycode-to-symbol round trip before sending
input. It uses only the Xlib mapped by the child emulator. Both scheduled
Ctrl-F10 release/recapture operations now pass their lock-transition checks,
using keycodes 37 and 76 on this display. The original bridge camera moves
from frame 90 to 26 and then 1, independently visible in the final screenshot.
This verifies the input helper, not progression: the game remains at profile
zero, and the numeric-chatter question is unresolved. No guest memory or
system libraries are changed. All 55 capture-tool tests pass, including
rejection of missing/ambiguous Xlib, failed display connections, and incorrect
keycode round trips.

Capture 36 confirms another release/recapture but lands at frame 163 rather
than the console. Capture 37 splits an ordinary -600 horizontal motion into
100 small packets and moves the original camera from frame 90 to 70. Capture
38 instead uses the bounded `--steer-after 36 45` controller: read the bridge,
send small relative packets, and verify both camera and centered cursor. It
reaches frame 45 with cursor arc 89 after 21 observations, and the screenshot
shows the radio console. Intermediate feedback stores compact state, not a
new RAM dump for every iteration. No state is written to the guest; unbound
feedback, changed profiles, or an exhausted iteration limit cannot report
success. All 65 capture-tool tests pass. This is still startup input evidence,
not a later original-game route or a resolution of numeric chatter.

Capture 39 uses bounded `--point-after` relative movements within the current
bridge view, then ordinary clicks on the console and Honk row. Its final
screenshot shows the original `JOUER / EXPLICATIONS` menu. Capture 40 follows
that route and clicks `JOUER`: the original executable changes from profile
zero to profile one (SCRIPT2) at the 109.601-second observation. The diagnostic
does not write guest memory or count as anthology coverage. The `time` address
still resolves to the first word of the live DEB allocation, not VAR; numeric
chatter remains unresolved. Pointer feedback rejects changed camera/profile
identity, invalid observations, and exhausted bounds. All 71 capture-tool
tests pass. Captures now preserve the exact hashed recorder source at startup;
capture 39 also retains its hash-matching recorder snapshot.

Captures 41-44 introduce an explicitly cross-runtime save diagnostic. The
recorder checks every predecessor, all 17 script sources, the imported archive
and loose-file hashes, and the witnessed save/directory bytes before staging
only those save files in its private writable drive. Capture 44 selects Options,
Load, and the displayed saved slot through ordinary mouse input. The original
binds profile one, matching the save's zero-based header, after the last click.
This is not an original-playthrough witness or complete save-state parity proof.
The metadata deliberately does not infer load verification from staged bytes.
DESCRIPT's mutable sprite-name catalog slot is now parsed from live memory;
fixed slots still anchor module identification. Unit pointer gain also prevents
the menu oscillation seen in capture 43. All 81 capture-tool tests pass. The
numeric-chatter allocation question remains unresolved.

Captures 46 and 47 subsequently load the earned
`bbb-scruter-concert-offer-v2` save through the original Options/Load menu and
travel normally to Kortland. Both bind SCRIPT15 and reach Scruter Mac's numeric
greeting and GIVE menu. These are cross-runtime diagnostics, not complete DOS
playthroughs. Capture 47 also verifies the recorder's bounded terminal JSON
commands: only ordinary pointer/key input, snapshots, and finish are accepted;
all inputs and preceding screenshots are recorded. All 84 recorder tests pass.

`re/tools/big_bug_bang_retained_pool_oracle.py` replays the original relocated
release and allocation instructions in a private Unicorn copy of those
captured states. Releasing the old companions in native DEB/COD/BAS/DIC order
leaves the vacated pool tail intact. Loading SCRIPT15 DEB/COD/DIC then reproduces
all 6,024 observed bytes from DIC+976 through DIC+6999, independently in both
captures. The numeric operand 2908 reads `2d 55 a5 00`, retained from
SCRIPT2.BAS offset 17484, not a zero byte beyond the current 973-byte dictionary.
The checked fixture records source/capture hashes and the short probe.
Controlled executions of the original audio hasher yield seed 125 with an
empty DIC+1 number buffer and 131 with `20` in that buffer. These two oracle
inputs establish the hash behavior, not the exact original call timing.

```sh
uv run --with unicorn python -P re/tools/big_bug_bang_retained_pool_oracle.py \
  output/big-bug-bang/startup-capture-46 \
  output/big-bug-bang/imported-assets/resources \
  re/tools/oracle_vectors/big_bug_bang_retained_pool.json \
  --before state-0067.bin --after state-0072.bin \
  --offset 2908 --extent 7000 --numeric-sample state-0076.bin
```

The runtime now retains checked script-companion pool bytes, uses the original
release order, and hashes the renderer's current numeric scratch only when the
audio coordinator actually has a pending menu. Mutable COD/DEB/VAR bytes and
unknown allocation padding cannot supply guessed values. Flat artwork-cache
entries represent different lifetimes, including fixed original buffers, and
must not be inserted into the script-companion pool. An intermediate generic
cache model completed `bbb-kortland-arrival-v4` but was exposed by v5's new
read-only hash trace: it used unrelated artwork bytes and seed 139. Neither
intermediate run verifies the numeric audio behavior.

`bbb-kortland-arrival-v6` uses the corrected pool ownership. Its recorded hash
at boundary 2412 contains the exact original retained suffix and seed 125,
with an empty current numeric scratch. It passes the greeting, cancels the
offered GIVE menu normally, and saves at boundary 4694, SHA-256
`9a3f99c3f6a53a9e93126b9465c9badb5e4552af2edd911b32fbd8b92f49c0ee`.
The run has 4,830 frames, 1,484 events, seven decoded resources, and five
source-matched SCRIPT15 sites. The game-library suite passes 1,119 tests
(79 ignored); the separate original-resource loader test and 62 flow/catalog
tests also pass. This clears the witnessed Mac greeting failure; the remaining
numeric actors and full optional-conversation scope still require validation.

Captures 48 and 49 extend the original normal-UI diagnostic to Bug Deluxe on
Cyborg and Toolbox on Moskito, loading the earned `bbb-concert-invitations-v1`
checkpoint. Both reach the numeric greeting and return to the bridge without
injected game state. The same relocated original allocator/profile-loader
replay matches every tested tail byte in each captured transition:

| Capture | Before / After / Numeric Sample | Operand | Matched Tail Bytes | Retained SCRIPT2.DIC Offset | Suffix | Empty / `20` Seed |
| --- | --- | ---: | ---: | ---: | --- | --- |
| 48, SCRIPT9 | 0145 / 0155 / 0158 | 3944 | 4,376 | 584 | `er` | 157 / 163 |
| 49, SCRIPT8 | 0135 / 0144 / 0147 | 984 | 6,136 | 1192 | `direct` | 190 / 196 |

Sample filenames are `state-NNNN.bin`; both use extent 7000. The generated
fixtures are `big_bug_bang_retained_pool_bug.json` and
`big_bug_bang_retained_pool_toolbox.json`. Provenance now requires a matching
32-byte context, since short dictionary suffixes need not be unique. Replaying
capture 46 with that stricter check produces its existing fixture byte-for-byte.
The seed pairs remain controlled executions of the original hasher, not proof
of the exact live call boundary. All three captured profile transitions are
covered by pool/hash regression tests and the original-resource loader test.
The expanded game-library suite passes 1,123 tests, with 79 ignored.

`bbb-bug-deluxe-arrival-v2` has 4,830 frames, 2,396 events, nine decoded resources,
and five source-matched SCRIPT9 sites. Its frame-2432 numeric hash has empty
scratch, `er`, and seed 157. Its 97-manifest lineage passes; slot zero at frame
4694 has SHA-256
`990faa7e2eee437efec0a6a7d2bbf39832c5b25889e2448107529a1d8e6a9682`.
V1 failed before producing a trace because it lacked the graphics development
environment and is excluded.
`bbb-numeric-arrivals-v1` reaches Sushi Deluxe and Toolbox, with five matched
sites each, 10,768 frames, 6,729 events, and twelve decoded resources. Toolbox's
frame-8343 hash has empty scratch, `direct`, and seed 190. Sushi's native hash
uses `rant` and seed 171; this run does not independently establish its DOS call
timing. Slot zero at frame 10632 has SHA-256
`dfeba5ee12804854a1e3c01f7e52741a50d930e52d26baf9297e2d4f35c2ac25`.
The intervening Lovia approach does not open Fuji's encounter, so this witness
claims only the two observed greetings, despite its three intended stops.

`bbb-fuji-deluxe-arrival-v1` separately reaches Fuji on Lovia, using the same
earned invitations save. The arrival itself speaks; the following ordinary
destination-region click contacts Fuji again, yielding encounter count two.
It has 6,990 frames, 4,041 events, eight decoded resources, and five matched
SCRIPT9 sites. Its first numeric hash at frame 2505 uses empty scratch, retained
`j'ai`, and seed 165. Slot zero at frame 6854 has SHA-256
`bf4165649f72439757f105a328bf3a2cb72b684314f0e18e97080a39c2adb55d`.

`bbb-robot-arrivals-v1` loads the earned invitations save and visits Toolcase
at Venusia, Jerry Khan at Kult, Mohi Khan at Magnus, and Shado Khan at Sat.
All four actually speak, contributing twenty SCRIPT8 sites across 12,683
frames, 6,219 events, and seventeen decoded resources. Toolcase and Mohi are
contacted twice by the normal region click; Jerry and Shado once. The initial
native numeric hashes are `GLUXX`/176, `x`/158, `nsons`/185, and `lubrifie`/203,
respectively, with empty numeric scratch. These are native observations using
the repaired retained pool, not independent DOS call-timing proofs for each
actor. Slot zero at frame 12547 has SHA-256
`8341e2fe6e1d64a67efa26d5985737388bcbe1200357a678cae8b5a01f806b6a`.

`bbb-toolbox-gifts-v1` loads `bbb-trump-painting-v1`, explicitly recording the
runtime update, and gives the already-owned energy, picture, food, and guitar
through Toolbox's displayed menus. Each item moves from aboard to Toolbox
and its gift bit clears after its response. `B41` becomes one; energy remains
at its existing cap of 1000, and population/evolution remain 20/100. The ten
matched sites include the four item replies and GIVE menu. The 6,087-frame,
1,837-event run decodes twelve resources and saves at frame 5951, SHA-256
`86e9e0f1170f17dc8f006b62ee078a5fbc0bd1a86b129aab7fbbdc108f5f8080`.
`robot-slice-audit-v1/report.json` rechecks all three leaves, 100 distinct
lineage manifests, current source/input hashes, numeric trace inputs,
rendered menu rows, transfers, and saves. All 35 displayed source sites match;
five are repeated Toolbox arrival sites. No rendering or runtime edits were
needed for these additions.

`bbb-oil-arrivals-v1` reaches Morning Oil at Ekatomb, Midnight Oil at Erazor,
and Evening Oil at Pterra from the earned invitations save. All three speak;
Morning and Evening are contacted twice, Midnight once. The run has 9,975
frames, 4,191 events, twelve decoded resources, and fifteen source-matched
SCRIPT8 sites. Slot zero at frame 9839 has SHA-256
`9e49e6379d88098dc3a029d966f45e2fa5bfbabfed4b417573638aaa7a542d6a`.
Their first numeric hashes use empty scratch and retained suffixes `r`, `nt`,
and `als`, with seeds 157, 164, and 170 at frames 2592, 6142, and 7959.

`robot-slice-audit-v1/hash_variants.py` replays the original allocation code
against captures 48/49 again, then substitutes each authored numeric operand
in a private copy of the captured word list. The unchanged original hasher
matches the actual native word bytes and initial seeds for all eleven checked
SCRIPT8/9 actors: Bug, Sushi, Fuji, Toolbox, Toolcase, Jerry, Mohi, Shado,
Morning, Midnight, and Evening. The generated `hash-variants.json` retains
capture/tail hashes and native frame references. This is controlled arithmetic
and retained-byte evidence, not an independently observed DOS call boundary
or full conversation-parity proof for each actor.

`bbb-toolbox-bionium-refuse-v1` and `bbb-toolbox-bionium-accept-v2` branch from
the published four-gift save. The owned bionium raises population 20 to 120,
evolution 100 to 130, and consumes one of three stock units. The decoder offer
appears immediately within this conversation, not only on a later visit.
Refusal leaves two credits and the decoder with Blue Wave. Acceptance spends
one credit and teleports the decoder aboard. Both encounters return the
bionium and credit inventory tokens after closing; that does not restore the
spent quantities. The displayed post-decision menu has three items in either
case, with different item identities. Refusal observes ten sites in 4,894
frames and saves at 4758, SHA-256
`66cdf8bee75c352e5b70c4d60485e323a39f70d355d55ad380d7ee086c8e2ba2`;
acceptance observes twelve sites in 4,534 frames and saves at 4398, SHA-256
`ce21291d443587d3564d9da182ee292428093d03f6561c94ebeaefc8dc5abd11`.
The premature ship-choice attempt and acceptance v1's incorrect Cancel
coordinate remain failed and excluded. `bbb-oil-toolbox-audit-v1/report.json`
validates 101 distinct lineage manifests, current source/input hashes,
matching text, menu selections, real transfers, quantities, and saved endpoints.
These three leaves add 21 sites beyond `audit-v11`; robot gifts and sales are
still not complete.

`bbb-toolbox-no-credit-v1` continues the accepted-decoder checkpoint. Giving
the remaining credit, decoder, ship, and painting through the displayed menus
spends the last credit, raises Toolbox's evolution 130 to 190 and population
120 to 320, and leaves the three objects with Toolbox. Returning the decoder
reopens its sale with the actual no-credit response at SCRIPT8 COD 1869.
The bionium stock remains two; the exhausted credit token moves to `internet`.
The run has 5,907 frames, 1,802 events, and thirteen matched sites, five new
against `audit-v12`. Slot zero at frame 5771 has SHA-256
`35c341562fa5b611ee60260dfb44a6b72fd0368dfa945e5df311514497a26f0d`.

`bbb-jerry-bionium-accept-v1` and `bbb-morning-bionium-accept-v1` branch from
the earned `bbb-concert-funded-v1` save, explicitly recording the runtime
update. Each uses one of two bionium units, raising the contacted robot's
evolution 100 to 130 and population 20 to 120, then accepts the immediate
one-credit offer. Both save with one credit and one bionium unit remaining.
Jerry's technology offer and caption actually transfer `decodeur`, leaving
`technologie` with Cyberquizz, exactly as the recovered script instructs.
Morning's decoder caption instead accompanies the real `cadeaux` transfer.
These authored inconsistencies are preserved, not corrected in the flow.
Jerry records 4,801 frames/1,636 events and saves at frame 4665, SHA-256
`b479660bacdcbe6d90db36313d296443b6b6c3544c5863f9fa94772123fad7a0`;
Morning records 4,891 frames/1,654 events and saves at frame 4755, SHA-256
`1645b2480e131ed3c92fc2aa3367db2e632bc14f2c5d3dfbdf441cb997c22703`.
Each has twelve matched sites, seven new. The three-leaf audit in
`bbb-sales-followup-audit-v1/report.json` rechecks 101 distinct lineage
manifests, current source/input hashes, text pixels, actual menu rows,
transfers, stock changes, and closed saved endpoints. It adds nineteen sites
against `audit-v12`, without rendering or claiming complete robot coverage.

`bbb-jerry-bionium-refuse-v1` and `bbb-morning-bionium-refuse-v1` capture the
other displayed choice from the same funded predecessor. Each still spends
one bionium unit but keeps both credits; the decoder/presents stay with
Cyberquizz. Each reveals ten source-matched sites, all already present in
the respective accepted path, rather than adding distinct wording. Their
closed slot-zero saves at frames 4665/4755 have SHA-256
`356d69c718326df0f7d0957150ec61ad314b2486b3a793dd50730c1460f1d17f`
and `2f9204b3d0c01f8796c30b21f33b1ab6a2bce85db487a43e18da16323931c82e`.

`bbb-shado-gifts-v2` and `bbb-evening-gifts-v2` each give the eight items
already aboard in `bbb-trump-painting-v1`: energy, picture, food, guitar,
painting, ship, bionium, and credit. All transfers and cleared gift flags
are observed; six items stay with the recipient and the two shared tokens
return aboard, with one credit and two bionium units remaining. Evolution
rises 100 to 180. Bionium initially raises population 20 to 120; subsequent
simulation grows it to 144, then relocates Shado to Rondoland or Evening to
Malusland with population twelve. The audit keeps these later events separate
from the immediate gift effects. The runs record 4,967/5,063 frames and
1,616/1,595 events, fourteen matched sites each, eighteen new sites together.
`bbb-two-robot-gifts-audit-v1/report.json` validates 94 distinct lineage
manifests, current source/input hashes, menu rows, transfers, and saves.
Slot zero at frames 4831/4927 has SHA-256
`4f2395d486acf5ec7fda9a83066c4f703ff233f847861912cad5ac11fdff5809`
and `f9ed292388c8136e96218f8741e1f8f5f065d229255c34f001a84f9a800bce72`.
The eight-gift menu closes automatically when empty. V1's unnecessary Cancel
click reopened the encounter, so those failed attempts remain excluded.

The refreshed `output/game-flows/audit-v4` accepts 265 witnesses and lists 107
excluded attempts. CB has 1,569 successful-route sites, 367 other normal-route
sites, 407 unobserved sites with already witnessed wording, 129 empty/control
sites, and 3,064 other unobserved/dynamic sites. BBB's counts are unchanged.
All 59 flow/catalog/coverage tests pass. Rendering remains disabled; these
counts still describe source sites, not distinct scenes or complete coverage.

`cb-hom-izwals-v3` branches from the earned early chapter-five arrival, while
Yoko is still aboard. The ordinary third cryobox contact opens Hom's race
menu. Five intelligence selections present all five Izwal intelligence lines;
the science submenu then presents its own topics and the retained Izwal
parent's two `info` responses. The trace contains 19 matching Hom BAS sites
and three COD introduction sites, with no attribution rejections. The slot-zero
save at boundary 6,020 has SHA-256
`9183a017f25b1df647ca73b90264ac2ef15436cf3761559e7cdebc52dc79eccb`.
The v1 launch failed before producing a trace because it lacked the Nix graphics
environment. V2 was stopped when its next `science` input was not in the newly
opened submenu; it is not a completed witness. These attempts do not establish
that the Izwal-specific science lines are globally unreachable.

`cb-honk-explanations-v1` opens the ordinary bridge Honk conversation from
`cb-corpo-delivery-v1`. Its explanations cover Blood, Bob, Honk, the Ark, Ma,
Orxx, Olga, the Big Bang, and black holes, with 34 matching BAS sites and four
chapter-two COD sites. The slot-zero save at boundary 7,409 has SHA-256
`32e382adea1dde043bc39b67e6c47fb1953afc06b9371af56cb2823d78c9f5ad`.
The targeted audit validates both new witnesses and their combined 69-manifest
lineage, with every fully revealed site matching source text. Counts here omit
the two repeated startup COD sites. No media was rendered.

Hom's subsequent legitimate contacts retain Yoko aboard:

| Witness | Matching Hom BAS sites | Save-zero boundary | Save SHA-256 |
| --- | ---: | ---: | --- |
| `cb-hom-races-v1` | 38 | 7,064 | `d28dabdbc20486d786f4b0d1f1e05abd069ad6c5ecab65c77161a0107329f572` |
| `cb-hom-planets-v1` | 36 | 7,920 | `cfdde28315801865477a67adbefe6cf6300be6b807c7e4e92f3a701a35be4f0d` |
| `cb-hom-nested-science-v1` | 15 | 5,950 | `c97bab9de51723460fdb51c9a1dc25d86c77a6cd1109eb1147f2cf38c12c753d` |

These are per-witness totals, not additive unique-site counts. The race path
covers the offered Croolis, Slimer, Sinox, Migrax, murffalo, ondoyant,
reproduction, joke, and tubular-brain topics. The next contact covers the
offered planets, politics, television, peace, war, and Izwal food. Its fifth
food choice closes Hom's presentation; the retained departure text is not
counted as a fully revealed actor line. Reentering science six times through
`race` and `izwal` then reaches all five Izwal-specific science lines. This is
an observed nested-menu route, not an injected topic selection. All three
targeted audits pass without rejected text attributions.

Honk's `cb-honk-play-v1` and `cb-honk-consultation-v1` continue from his
explanations save. They record 52 and 50 matching BAS sites respectively,
covering play, winning/losing, help, configuration, consultation, and the
number exercise. The latter records `scr=1`: only one click on `one` was sent,
not the repeated clicks that open the script-selection cheat. The saves are
at boundaries 8,879 and 8,775, SHA-256
`7daacbb8479d396f4253dd16d0a3fa51b97f0a79a68f8889f8eed552fda16c84`
and `9a105a7813e8819b756425a9ef137adaf70a7507ed015d5a890df8eeaf73b8ad`.
The three Honk witnesses cover 133 of his 140 chapter-two BAS sites. The seven
unobserved sites remain unresolved; spelling differences in the source or
repeatable preceding lines are not treated as global unreachability proofs.

`cb-jo-exit-v1` branches from Jo's earned Pterra recruitment and opens the
second of four aboard roster entries. It witnesses Honk's first reprogramming
introduction, Jo's minigame instructions, and the zero-bionium failure reply.
Escape exits the ordinary AMER overlay after two input/presented frames; the
trace verifies graphics/audio restoration and released resources. The saved
state has `vbio=0`, `compris=0`, with slot zero at boundary 11,068, SHA-256
`87c3e0cb967b44ac4a790c3c54c3366d498cb6fd3c7416f5787c8539732d4145`.
All 29 chapter-two text sites match their source, as do the two startup sites.
The native minigame exit is not an original-DOS rendering or physics proof.
All 59 flow/catalog/coverage tests pass after these additions.

`cb-hom-nested-races-v1` continues through offered parent menus for Croolis,
Slimer, Sinox, Buggol, and ondoyant topics. Its bounded revisits produce 37
matching Hom BAS sites, including the Croolis war/joke responses and Slimer
science/politics responses missed by the first topic sweep. It saves at boundary
10,358, SHA-256
`3f14f20e6b1bdb3f98a1e2778ad3fa2c2e40b5dcae7d27728ea27bababcaac86`.
`cb-honk-headers-v1` adds the chance-based winning advice and explicit
explanations prompt, bringing the four Honk witnesses to 135 of 140 chapter-two
BAS sites. Its save at boundary 4,848 has SHA-256
`a8196277d069240d327572692c1dcfb4d4008e3b83a28b08e6dfe54d958aadf0`.
Both targeted audits pass with no rejected attributions; unresolved sites are
not silently classified as unreachable.

`cb-jo-bionium-v1` loads the witnessed zero-dose save, reenters Jo normally,
and replays a CB-specific AMER flight. CB's `AMER.XDB` has SHA-256
`6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31`,
different from BBB's level. The planner requested ten doses but returned seven
after leaving through the blue box; the actual game replay independently
records 69,091 input/paced/presented minigame frames, 57 sound callbacks, normal
restoration, `vbio=7`, and `compris=1`. Its success dialogue is witnessed, and
the slot-zero save at boundary 5,188 has SHA-256
`3a2299259b9eecc6fbdd18eec4feec3ec5a2b2620c1be77e3a3c82306ff741f1`.
The 43 MiB witness contains no recorded movie or PCM. The generated flight
inputs are copied into its hashed scenario; planning alone is not the witness.
The input-plan JSON and TSV under `output/game-flows/oracles/cb-jo-input-plan-v1`
have SHA-256 `cece072df7191ab59b827853591369162c10f2d223a7ffefb7bc41f2bf3d6b88`
and `680698371484e95e80107d84a686fe593b3b2b3a3a30e200d6ff7b1ec067c6d3`.

The refreshed `output/game-flows/audit-v5` validates 276 witnesses and excludes
109 unfinished or legacy attempts. CB now has 1,569 successful-route sites,
654 other normal-route sites, 687 unobserved sites with witnessed wording,
128 empty/control sites, and 2,498 other unobserved/dynamic sites. BBB's counts
are unchanged. The audit remains explicitly incomplete and not render-ready.

`cb-yoko-topics-v1` branches from the earned Bronko recruitment save and opens
Yoko's first Rondo conversation. Its offered topics cover Izwals, Croolis,
Tromps, Migrax, ondoyants, tubular brains, art, reproduction, politics, and
science before the ordinary farewell takes the ship to the observatory. All
93 fully displayed sites match source text, including 77 Yoko BAS sites;
the eight-manifest save lineage passes. The slot-zero save is at boundary
10,524, SHA-256
`32d23b6b3c64fa33242e2587c3af26ea60bf14fdeaac0fd82d7bffb0fd67be44`.
The subsequent `cb-yoko-nested-topics-v1` attempt from that observatory save
does not reach the requested conversation and has no completed checkpoint.
It remains a failed attempt, not a witness or an unreachability proof.
`cb-yoko-nested-topics-v2` instead branches from the same earned pre-Rondo save
as the first witness. Reentering the offered parent menus reaches the Izwal
peace, Croolis war, art/reproduction, politics, and science replies. All 55
fully displayed sites match source text, including 37 Yoko BAS sites. Its
slot-zero save is at boundary 9,368, SHA-256
`d769396bc93771c0d871462dc301e9c4fb3480e7f4e4e5b126655cb0a1a88c9b`.
Together with the existing story route, Yoko now has 102 of 104 chapter-two
BAS sites witnessed. The chance-based "You talk much" line and the plural
`weapons` response remain unobserved; neither is declared unreachable.

`cb-eviscerator-topics-v1` branches from the earned chapter-three Bronko
family-news save and enters the prison using the offered code. The conversation
covers television, politics, reproduction, science, consumerism, planets,
races, peace, friends, art, war, battle, and enemies without advancing the
treasure handover. All 90 fully displayed sites match source text, including
73 Eviscerator BAS sites, and the 25-manifest lineage passes. The slot-zero
save is at boundary 11,641, SHA-256
`a82dda79e48d056d0cf1451132426c6fd482612f59b1e7ebd6acf0a1ae307efa`.
Neither branch records a movie or PCM. The 59 flow/catalog/coverage tests pass.

The refreshed `output/game-flows/audit-v6` validates 279 witnesses and excludes
110 unfinished or legacy attempts. CB has 1,569 successful-route sites, 818
other normal-route sites, 838 unobserved sites with witnessed wording, 127
empty/control sites, and 2,184 other unobserved/dynamic sites. BBB's counts are
unchanged. The generated Markdown reports now show the twenty largest
unresolved source groups, separating new/dynamic wording, known wording, and
empty/control text. These counts do not establish feasible branches, unique
clips, or durations. All 61 flow/catalog/coverage tests pass, including the
new report checks. Rendering remains disabled pending the remaining coverage
and scene-level edit decisions.

Additional normal-input witnesses after `audit-v6`:

| Witness | Matching sites | Save-zero boundary | Save SHA-256 |
| --- | ---: | ---: | --- |
| `cb-bug-catalogue-v1` | 62 | 11,819 | `98781c40538192aa96a3b5ff41e5cb9c64684087bde37f8fea0c4ba2a0b5cf2f` |
| `cb-morning-history-v1` | 33 | 6,954 | `248138118213057a422ab00ef2b47451a28ed7746e4b4940e702449bea5f8600` |
| `cb-meal-menus-v1` | 46 | 6,061 | `05c4814e9fd11df5c46165a0a63759d15dc527ca420b3af3e0ee37af8d88c5f1` |
| `cb-bronko-menu-v1` | 10 | 3,157 | `3521dbcb231e22bc536ebdc9a65be082a217fee5d255fcdeeef5c3576f01c045` |

Each passes the source-text, artifact-hash, and recursive save-lineage audit
without rejected attributions. The shop's 55 BAS sites include the offered
product descriptions, unaffordable games, and multimedia advice. No purchase
is made: `ach=0`, and the earned credit remains aboard. Morning's 21 BAS sites
cover the war, Croolis, and treasure history; all eight COD lines of his
argument with Honk are also witnessed. The five ordinary console-menu contacts
show 44 daily-menu sites before Bronko joins; the separate recruited-Bronko
branch shows his eight improved-menu sites. These are source-site totals,
including repeated startup sites in each witness, not additive scene counts.

`cb-morning-disconnect-v2` and `cb-morning-keep-honk-v1` branch from the same
earned memory-recovery save. Four normal `hiding_place` selections reach the
offered `disconnect`/`refuse` choice; the earlier three-selection attempt
does not and remains excluded. Disconnect records all eight subsequent
argument lines, while refuse records Morning's distinct refusal. Their 19
and 12 fully displayed sites pass source attribution without rejections.
The respective slot-zero saves are at boundaries 5,708 and 5,074, SHA-256
`c25a20754170a2104ef525d2767cc303ddfd3c62b688f0f589f20118a1379bca`
and `0f75fdf0e4bc999f1a6db518667fce54e6b6ce23a15555562c48b18a58d32f2c`.

The refreshed `output/game-flows/audit-v7` validates 285 witnesses, including
both branches above, and excludes 111 unfinished or legacy attempts. CB has
1,569 successful-route sites, 963 other normal-route sites, 999 unobserved
sites with witnessed wording, 127 empty/control sites, and 1,878 other
unobserved/dynamic sites. BBB's counts remain unchanged. Every accepted
save lineage is rechecked; these totals do not establish full scene coverage
or running time. No new movie or PCM is recorded, and rendering remains
disabled pending the remaining coverage and scene-level edit decisions.

`cb-izwalito-topics-v3` replays the verified new-game prefix, then explores the
offered society, happiness, food, reproduction, ecology, and stars topics on
the first Corpo visit. Its 117 fully displayed sites match source text with
no rejected attributions, including 34 Izwalito BAS sites. Slot zero is saved
at boundary 17,462, SHA-256
`734938c47c7820a88dc0f37db5d5f9e1a81a90f1512810fa725cd097fb9b0cd6`.
The first attempt used the wrong later Corpo checkpoint; the second asked for
an unavailable main-menu `talk` row. Both remain failed attempts. The successful
route selects only offered rows and does not take the secret/Rondo branch.
This addition is after `audit-v7` and is not included in that report's totals.

`cb-morning-no-perfume-v1` branches from Morning's earned recruitment before
buying perfume. It chooses `no` at Honk's switch-off request, follows the
authored recovery and TV-memory sequence, and learns Mastachok normally. All
48 displayed sites match source, including 41 Morning COD sites, and the
17-manifest save lineage passes. The slot-zero save at boundary 22,064 has
SHA-256 `e6405b6b128ee559cfb1b505b0e296335878a489c26bc62fc9c5cc9919706a89`.
This is also after `audit-v7`. Carrying perfume already selects the guard's
gift response; merely avoiding its inventory inspection does not retain the
code-topic path. The no-perfume route instead opens the direct code menu,
without the dance-password prelude.

`cb-scruter-mac-topics-v3` continues that checkpoint through all offered robot,
SCRUT, prison, Mastachok, presents, and nested topics. All 60 fully displayed
sites match the source, including 48 guard BAS sites and ten guard COD sites.
The 18-manifest lineage passes; slot zero at boundary 11,023 has SHA-256
`61ef635f4f6d760a92e391891fff7fb04e3e145f9242d0d14721af6345f9e2e8`.
`cb-scruter-mac-repeat-visits-v1` then gives the correct code at the next two
visits, explores the happy and sulking topics, and observes the fourth-visit
expulsion and `explo3.hnm`. It reveals 20 matching sites, ends with `C1=4`,
and saves at boundary 9,340, SHA-256
`fbe7e69439bf4ce07ca7b77bbbbc5a4a1b90aea82f1ed6fef5ed00cf7759b63d`.
The BAS farewell overridden by COD remains unobserved; that is not a global
unreachability finding. Both earlier guard attempts remain excluded.

`output/game-flows/audit-v8` includes these four additions after `audit-v7`:
Izwalito's topics, the no-perfume recovery, and both guard witnesses. It validates
289 witnesses and excludes 115 failed, unfinished, or legacy attempts. CB now
has 1,569 successful-route sites, 1,070 other normal-route sites, 1,009 unobserved
sites with witnessed wording, 125 empty/control sites, and 1,763 other unresolved
sites. BBB's counts remain unchanged. Source attribution and every save lineage
are rechecked, but both flows remain incomplete and `render_ready` stays false.

After that audit, `cb-bronko-topics-v1` branches from the earned Pterra save
and revisits Moskito before recruiting Bronko. It explores weapons, robots,
intelligence, energy, technology, retirement, creds, purchases, and murffalos
through their offered menus. All 34 displayed sites match source, including
28 Bronko BAS sites; its seven-manifest lineage passes. Slot zero at boundary
8,143 has SHA-256
`6bf1684f270cbc5b1630d9ef8e24e725fcea8343f8081d4ef6386ad1143b4684`.
This witness is not yet included in `audit-v8` totals.

`cb-honk-recap-chapter2-v1` branches from the earned Mastachok-perfume save and
asks Honk's available recap and explanation topics. All 37 fully displayed
sites match source, including 27 `miss2` recap sites and five `what5` sites.
Its 23-manifest lineage passes, and slot zero at boundary 6,735 has SHA-256
`ecf7259dede7433bf90afcc0f4dbc44a85c384048580bf4ffedcb871ab54c22c`.
This addition also follows `audit-v8`; it does not close all Honk branches.

`cb-bob-chapter3-topics-v2` splits Bob's black-hole, Big Bang, mission, Corpo,
flattery, and self-history topics across ordinary cryobox calls because his
chapter-three illness timer limits each contact. Its 30 displayed sites all
match source, including 24 Bob BAS sites, and the 25-manifest lineage passes.
Slot zero at boundary 9,220 has SHA-256
`857b5c142aefc1cea3ef496c61000a18759f68bb7323a92c37b9a38bb5e6cade`.
V1 exceeded the timer while asking about the mission and remains excluded.
The subsequent `cb-bob-chapter3-revelation-timeout-v1` lets the requested
revelation play until that timer interrupts it. It observes 11 revelation
sites and Bob's illness, not the whole revelation; all 16 sites and its
26-manifest lineage pass. Its save at boundary 3,973 has SHA-256
`d987cbcaee3c952a527ebe8ee47cf04e4158d28708703f031ed67c605770062f`.

The separate `cb-bob-chapter3-kanary-v3` branches before requesting revelation
and covers cottage, cloning, reports, and employees over three calls. The
company submenu persists across those recontacts; trying to select `kanary`
again stalls, as the excluded v2 shows. All 24 sites match source, including
18 company BAS sites; its 25-manifest lineage passes. Slot zero at boundary
7,470 has SHA-256
`5966b27cabfa4ecb16935204fab5a9774d250ff1f9ffb4035bb1bfa5235211b4`.
V1 started from the pending-revelation branch and never offered its requested
company topic. These additions are after `audit-v8`, not completeness claims.

`cb-bob-chapter3-raise-v1` recontacts Bob after the employee topic and witnesses
all six `augmen` raise-request lines. All nine displayed sites match source;
its predecessor lineage validates. It completes 4,109 frames and 333 events,
with seven decoded resources. This short dialogue witness does not make a new
save and is not a resumable checkpoint.

Four fresh-game identity branches, `cb-izwal-alt-hyper-v2`,
`cb-izwal-alt-predatorus-v1`, `cb-izwal-alt-mousy-v1`, and
`cb-izwal-alt-jules-v1`, follow the ordinary Corpo approach. They select the
offered identity, accept the credit normally, and revisit Izwalito for his
identity-specific callbacks. Jules also makes the next two return visits.
The first three each have 116 source-matched sites, and Jules has 126; their
union adds 47 sites and 34 distinct wordings relative to `audit-v8`.
`cb-izwal-alt-audit-v1/coverage.json` retains the detailed comparison.
These are genuine alternative identities, not changes to the script globals.

Three further fresh-game branches, `cb-izwal-alt-exterminator-v1`,
`cb-izwal-alt-tarzoom-v1`, and `cb-izwal-alt-commander-v1`, finish the same
ordinary credit-and-callback route with their respective offered identities.
They have 116, 116, and 113 source-matched sites and add 15 sites and ten new
wordings beyond the preceding identity slice. Their current script and input
hashes pass independent review. `cb-izwal-alt-audit-v2/coverage.json` records
the detail; these three witnesses were already included in `audit-v9`.
The three wrong-Venusia attempts remain incomplete and excluded. They do not
establish the wrong-delivery or spent-credit responses.

The four Mamy witnesses `bbb-optional-mamy-stroll-v2`,
`bbb-optional-mamy-recipe-no-crush-v3`, `bbb-optional-mamy-recipe-decline-v1`,
and `bbb-optional-mamy-crush-v1` begin with the earned first-visit perfume save.
They cover the second-visit stroll, both recipe answers, and both relationship
answers on later visits. Their union adds 34 SCRIPT3 `Mamy01` sites relative to
`audit-v8`: 32 dialogue sites, two control sites, and 31 distinct new wordings.
All 30 distinct predecessor/current manifests validate, with zero rejected
presentations in these four witnesses. The no-crush branch returns food and
leaves Mamy in Loviland; the decline branch retains food; the crush branch
brings her aboard through the authored response. Detailed checks and save
hashes are in `bbb-optional-mamy-audit-v1/final.json`. Three earlier attempts
remain excluded for a wrong Cancel row or unhandled menu/travel state.

Four subsequent Mamy alternatives pass the independent source and lineage
audit in `bbb-optional-mamy-aboard-audit-v1/parent-review.json`:
`bbb-optional-mamy-aboard-return-v1`, `bbb-optional-mamy-aboard-keep-v1`,
`bbb-optional-mamy-perfume-return-v1`, and `bbb-optional-mamy-decoder-gift-v1`.
They have 14, 24, ten, and six matched sites respectively, with no rejected
presentations across 34 distinct validated lineage manifests. Their union adds
31 sites and 31 new wordings beyond the first Mamy slice; 27 of those sites
were already included in `audit-v9`.
Sending Mamy back relocates her to Cyborland and makes a normal save at
boundary 4447. Keeping her aboard reaches the authored losing ending at
SCRIPT2 `0x9f14` and exits cleanly; that alternative has no resumable save.
The perfume branch declines the renewed purchase offer and witnesses the
gift's return, leaving the perfume aboard and credits unchanged. The decoder
branch transfers it to Mamy and raises her evolution from 60 to 110. Both
leave the inventory menu normally and save, at boundaries 5563 and 5312.
These are native normal-input alternatives, not DOS parity, complete Mamy
coverage, or successful-ending continuations for every saved branch.

The next four Mamy gifts, `bbb-optional-mamy-{credit,ship,food,picture}-gift-v1`,
all use items already aboard in their earned predecessors and finish with
normal saves. They contribute nine new SCRIPT3 `donn4_5C7D` sites: eight new
dialogue wordings and one control-text site, beyond `audit-v9` plus the previous
Mamy slice. Independent review in `bbb-optional-mamy-gifts-audit-v1/parent-review.json`
validates all 66 distinct lineage manifests, current source/input hashes,
offered menu rows, transfers, and source text, with zero rejected presentations.
Credit reduces the balance from nine to eight; the later return of the credit
inventory object is not a refund. Mamy returns the ship and raises her evolution
from 60 to 90. Food gives the observed +500 energy change. The picture raises
evolution from 60 to 260 and aggression from 250 to 450, quarters the current
population, and increments `B96`. Other gifts and state-dependent responses
remain open.

`output/game-flows/audit-v10` accepts 319 witnesses and excludes 133 incomplete
or legacy attempts. The successful-route lineages remain 72 CB and 99 BBB
segments. CB has 1,569 successful-route sites and 1,255 other normal-route sites;
BBB has 3,013 and 269 respectively. There are still 1,572 CB and 2,613 BBB
unobserved-wording/dynamic sites, plus unobserved duplicate-wording and control
sites. These are site counts, not unique conversations or video durations.
`render_ready` and `all_normal_branches_complete` remain false. The intermediate
Kortland v4/v5 text witnesses included in this census still do not prove numeric
audio correctness, as explained above.

`cb-izwal-alt-wrong-venusia-v5` independently replays the fresh-game Commander
Blood identity and credit request, clears the actual queued phone calls,
buys the wrong joint of meat on Venusia, and delivers it to Izwalito. Three
ordinary return visits then cover `corbis`, `corter`, and `corqua` with the
credit still spent, including the observed chance-based credit hint. All 183
displayed sites match source; 34 sites and 28 wordings are new versus
`audit-v11`. `cb-izwal-alt-audit-v3/parent-review.json` independently verifies
the fresh-game witness, all six source and scenario hashes, endpoint, and
saves. Slot zero at frame 41873 has SHA-256
`5a435aef16a87e675a55dd275c7ad6e7363b1a991696447630b6ee873a5a0061`.
The run has 42,009 frames and 2,962 events, ending with `A11=3`, `trak4=1`,
`zob1=0`, credit held by Bug Deluxe, and the purchased meat held by Izwalito.
V4 failed after the purchase because the next requested shop category was no
longer offered; it remains excluded. Recovery, Bossanova, and an ending
continuation for this branch are still open.

The fourth Mamy gift slice adds energy/bionium v2 and medicine/technology v1.
The first pair loads Jo's earned resupply save; the second loads the prior
picture-gift save. Independent `bbb-optional-mamy-gifts-audit-v2/parent-review.json`
validates 64 distinct lineage manifests, all eighteen current source hashes
per witness, offered gift menus, actual transfers and native saves, with zero
rejected presentations. It adds five new nonnumeric sites and wordings beyond
the previous slices. Energy changes from 36 to 1000. Bionium takes the coughing
branch, with Super Zen's existing flag set and `vbio=10` unchanged; it does not
cover the genetic-code branch. Medicine raises evolution 260 to 270, population
29 to 229 before later simulation ticks, and `B22` to one. Technology raises
evolution 260 to 310 and `B20` to one. Mamy stays on Loviland throughout.
All four normal saves close the conversation; energy/bionium v1 attempts remain
failed and excluded. The frozen pre-numeric-fix binary is valid for these
SCRIPT3-only conversations, not evidence for the numeric actors.

`output/game-flows/audit-v11` adds these seven verified leaves and 35 BBB sites
to the previous census. BBB now has 3,013 successful-route sites and 304 other
normal-route sites, with 2,563 unobserved-wording/dynamic sites, 442 unobserved
duplicate-wording sites, and 599 unobserved control sites. CB is unchanged.
Neither unique conversations nor complete alternative ending continuations
are inferred from those site counts; rendering and completeness remain false.

The fifth Mamy slice, nuclear/weapons v1 and crown/mummy v2, adds six new
SCRIPT3 sites and six wordings. Independent review in
`bbb-optional-mamy-gifts-audit-v3/parent-review.json` validates 81 distinct
lineage manifests, source/input hashes, displayed text, transfers, effects,
and normal saves with zero rejected presentations. Nuclear changes evolution
310 to 510, halves the current population, and adds 200 aggression, but does
not take the high-evolution reply in the same gift. Crown adds 200 energy and
aggression; `B21` is set and later cleared. Mummy adds 100 energy and divides
population 105 by 200 to zero, followed by ordinary growth to six. Weapons
adds 100 aggression and sets `B21=1`. All four leave Mamy on Loviland and
save with no active ending. Crown/mummy v1's misplaced Cancel clicks remain
failed and excluded; no stat changes were injected to reach these branches.

The sixth Mamy slice adds guitar, optics-study, ring, and writing v1. Each
adds one new source-matched wording, at SCRIPT3 COD 24803, 25423, 25527,
and 25697 respectively. Guitar and ring each add ten evolution and 300
population, lowering aggression by 150 and 200 respectively. Optics adds
twenty evolution and increments `B20`/sets `B21`; writing adds fifty evolution.
Guitar's subsequent ordinary simulation reduces population to 198 and energy
to zero; these later changes are not attributed to the gift response.
Independent review in `bbb-optional-mamy-gifts-audit-v4/parent-review.json`
rechecks 82 distinct lineage manifests, current source/input hashes, all
displayed text, transfers and effects, and the four closed slot-zero saves.
No ending is active, and each gifted item remains with Mamy on Loviland.

`bbb-optional-mamy-nuclear-repurchase-v1` continues the evolution-510 save,
visits Emasculator twice, and buys the nuclear item on encounter seven using
his actual `yes`/`no` offer. Encounter six's earlier departure is retained.
The price is one credit, leaving seven, and the nuclear item returns aboard.
`bbb-optional-mamy-nuclear-repurchase-high-v1` then gives it to Mamy again,
observing the high-evolution response at SCRIPT3 COD 24327. The immediate
effect raises evolution 510 to 710, energy 370 to 770, and `B96` by one;
population 41 and aggression 650 do not change at that gift boundary.
Later normal simulation produces population 47 and energy 788 at the save.
The two runs add six sites, five at Emasculator and one at Mamy, with no
active ending. Slot-zero saves at frames 9842/5312 have SHA-256
`5f734c09debedc464e85fcaa0f1c8c91e14705f2f9d10730fc2b722434ef1601`
and `e3c7efaa9b359d6a44b509be21f40145edd6a34d8590899eef34b044f938a4c2`.
`bbb-optional-mamy-nuclear-repurchase-audit-v1/final.json` retains the source,
menu, pixel, transfer, and effect evidence; independent parent review is in
`branch-review-v13/report.json`. This closes the previously untested ordinary
repurchase route, without calling other Mamy branches complete.

The remaining-Mamy batch captures four distinct source cases:

- `bbb-optional-mamy-nuclear-low-v1` gives the earned nuclear item at evolution
  70. SCRIPT3 COD 24164/24196 precede the source-driven `A13=1` ending, not
  the middle- or high-evolution gift responses. All seven ordered resources
  in `31explonebul` play through the final `ppit08.hnm` closure, followed by
  a clean native exit at SCRIPT2 COD 40691. The run records 4,832 frames,
  321 events, and ten matched sites; it does not fabricate a terminal save.
- `bbb-optional-mamy-strike-no-v1` declines insistence during the `A61=6`
  negotiation, then revisits for the general-strike reply. Its closed save
  has `A61=8`, `A68=1`, and all five credits retained. It records 8,009
  frames, 348 events, and 19 matched sites. Slot zero at frame 7873 has
  SHA-256 `8fb1b254188646635470ade674f4b531f26dc618de257bb5dd676e6eecb34106`.
- `bbb-optional-mamy-strike-insist-v1` takes the other displayed answer,
  triggering the strike at `A61=7`, `A68=1`, and `A65=1`, without spending
  credits. It records 6,623 frames, 310 events, and 18 matched sites. Slot
  zero at frame 6487 has SHA-256
  `956436d4d315114c642c661e31c032df463f10d0fce3265f549476edc94750e0`.
- `bbb-optional-mamy-perfume-no-credit-v1` visits from the earned second-ship
  save with no credits or perfume. Choosing the stroll exposes COD 23537,
  without acquiring perfume or changing the zero balance. It records 5,448
  frames, 300 events, and 11 matched sites. Slot zero at frame 5312 has
  SHA-256 `c1f3c4e3c7cce8d18532231fb83fd4aa24e95de258528978156c4dc13d9e71bd`.

`bbb-optional-mamy-remaining-audit-v1/final.json` and the independent
`parent-review.json` validate 83 distinct lineage manifests, current
source/input hashes, matching text, actual effects, saved endpoints, and the
ordered failure sequence. Their union adds 26 sites and 25 distinct wordings.
The pre-menu click interrupts shared strike sentence COD 21685 in both
strike witnesses, so neither claims that sentence; a separate full-line
follow-up remains necessary. The source/earned-save assessment also leaves
the other listed guards open rather than declaring them unreachable.

`output/game-flows/audit-v12` accepts 334 witnesses and excludes 137 failed,
running, or legacy attempts. It includes the Oil greetings, both decoder
decisions, the fifth Mamy slice, and the wrong-Venusia route. Successful
lineages remain 72 CB and 99 BBB segments. CB has 1,569 successful-route and
1,289 other normal-route sites; BBB has 3,013 and 331. Other unobserved/dynamic
sites remain 1,542 CB and 2,542 BBB, in addition to duplicate-wording/control
sites. Both completeness flags remain false; no new movies were rendered.

`cb-izwal-alt-bossanova-v1` continues the wrong-Venusia save, visits Venusia,
and answers the pending radio call. It reveals the seven previously unseen
Bossanova complaint sites, with nine matched sites total in 6,257 frames and
190 compact events. The credit remains with Bug Deluxe and the wrong meat
with Izwalito; this witness is not money recovery. Slot zero at frame 6121
has SHA-256 `b4433bc2dda4ab0af8f45663a8070aaab7459740c5f9f3b3eb948ddf3ef18250`.
`cb-izwal-alt-audit-v4/coverage.json` also retains two failed recovery attempts.
The second observes recovery and the correct food delivery, but reaches an
unexpectedly early Corpo4 Hita `yes`/`no` prompt instead of its planned
farewell. Without a closed saved endpoint, it remains excluded.
`branch-review-v13/report.json` independently validates the Bossanova leaf,
both nuclear leaves, and both robot refusals through 104 distinct lineage
manifests, current source/input hashes, matching text, effects, and saves.

`cb-izwal-alt-credit-recovery-v3` now closes the spent-credit recovery from
the Bossanova save. The actual meat submenu's `murffalo` back-choice returns
the credit from Bug Deluxe, without returning the wrong meat from Izwalito.
Its 5,869 frames, 249 events, and ten matched sites end in a normal save at
frame 5733, SHA-256
`ef51d894e983594f868fea595708a83c9cd8b489bd3be54a94dedadeb0429736`.
`cb-izwal-alt-recovery-closure-v1` then spends that recovered credit at Bronko,
delivers the correct meat, receives the decoder and credit rewards, and
chooses the actual Corpo4 `NO` farewell. It saves with `A1=3`, `F1=1`, and
`trak6=0`; both meat objects remain with Izwalito and the rewards are aboard.
Its 21,049 frames, 1,167 events, and 58 matched sites add three previously
unobserved wordings, SCRIPT2 COD 21969, 21991, and 22658. Slot zero at frame
20913 has SHA-256
`15f39b74773c973648d8bbaf028e0fcab08816c7315e792fce1d67d618faa5b9`.
`cb-izwal-alt-audit-v5/parent-review.json` independently rechecks the four
lineage manifests, source/input hashes, text, menus, all six item transfers,
and closed saved endpoints. The two failed recovery attempts stay excluded;
these alternatives do not establish a full ending continuation or DOS parity.

`output/game-flows/audit-v13` accepts 349 witnesses and excludes 141 failed,
running, or legacy attempts. The successful lineages remain 72 CB and 99 BBB
segments. CB has 1,569 successful-route and 1,296 other normal-route sites;
BBB has 3,013 and 380. Other unobserved/dynamic sites remain 1,535 CB and
2,515 BBB, in addition to duplicate-wording/control sites. Both completeness
flags remain false. This snapshot includes the first inventory-preserving
Blue Wave gift below, but not its ongoing later-visit continuations.

`output/game-flows/audit-v14` accepts 359 witnesses and excludes 143 failed,
running, or legacy attempts. CB has 1,569 successful-route and 1,299 other
normal-route sites; BBB has 3,013 and 423. Other unobserved/dynamic sites
remain 1,532 CB and 2,472 BBB, separate from duplicate-wording/control sites.
Both completeness flags remain false. This snapshot includes all five Blue
Wave bionium visits, the credit/ship/hint alternative, both CB recovery
leaves, and the nuclear-low and strike Mamy leaves. It predates the no-credit
perfume leaf and the ongoing retained-ring and early-ring continuations.

The earned chapter-four continuation `cb-vista-tomb-v2` visits Super Tromp,
asks about the painting, culture, and Great Yolk, enters the tomb, and accepts
Sinox's candle offer. The trace verifies Sinox's full presentation and departure
to Trashlando, `yo=1`, and the saved checkpoint. The final fifth `yolk` selection
advances the exhausted topic back into the scripted tomb transition; choosing
`bye_bye` instead leaves the encounter without that transition.

`cb-anna-tomb-v1` then wakes Anna Haf for his repaired-robot introduction and
the separate Vista assignment. His transfer to the tomb is observed. The local
contact clicks at the end of that prefix do not reopen the destination view;
they are not evidence of the subsequent tomb conversation.
`cb-anna-tomb-contact-v4` opens and closes the chart after loading to restore
the local camera, then selects Vista. The game enters the tomb and presents
Anna's three lines directly, without repeating Super Tromp's topics. Anna is
the sole known outgoing phone contact in the resulting earned save.
`cb-anna-painting-v1` answers Jerry, calls Anna at Vista, waits for her authored
timer, receives her successful-theft call, and recovers both the portrait and
Anna through two teleport choices. Both are aboard in the saved endpoint, and
Super Zen has returned to Crazystone. The curse is still active at this point.
`cb-zen-lifts-curse-v1` delivers the recovered portrait to Super Zen and plays
the curse-removal ceremony. Its ordinary saved checkpoint confirms
`maledict=0` and the painting held by Super Zen; it is a verified chapter-four
continuation, not the end of the chapter or game.
`cb-bratakas-hom-v3` explores Bratakas's planet and leisure topics, trades the
old decoder for the ondoyant picture, and follows his farewell into Hom's
hiding place. The earned D.O.R.K. diploma recruits Hom. The saved endpoint has
Hom and the picture aboard, the diploma with Hom, and the decoder with Bratakas.
The two earlier attempts stopped on unavailable or unselected topic rows;
only v3 is a completed route witness.
`cb-fifi-picture-hat-v1` gives Fifi that picture and revisits him after the
curse has been lifted. The second encounter receives the hat. The saved state
confirms the picture with Fifi, the hat aboard, and Super Zen back at Crazystone.
`cb-zen-hat-v1` gives the hat to Super Zen and hears his Masta identification.
The saved endpoint has the hat with Zen, Zen at Trashlando, and Fifi returned
to Ron for his transport request.
`cb-fifi-transport-offer-v1` accepts that request and saves with Fifi aboard.
`cb-fifi-malus-v1` asks him for Malus's coordinates, transports him there, and
recontacts him locally. Fifi's Ondoya clue is fully presented, and the saved
endpoint places him on Malus.
`cb-ondoyant-recruit-v1` follows that clue to Ondoya and accepts the request
for passage. Its saved endpoint confirms the Ondoyant aboard.
`cb-maziok-sat-v1` wakes the Ondoyant for the aboard conversation, then returns
to Maziok after the curse removal. It presents the Ekato explanation and Sat
coordinates. The saved endpoint has `ek=1` and Maziok at Trashlando.
`cb-betakam-recruit-v1` visits Attrox, accepts Beauregard's participation,
gives Betakam the Mind Scrambler, and accepts his transport. The saved endpoint
has Betakam aboard and the scrambler held by him.
`cb-betakam-sat-v1` answers Jerry's pending update, travels to Sat, deploys
Betakam from the cryobox, and recontacts him locally. His five-line account of
life on Sat is presented; the saved state places Betakam on that planet.
`cb-masta-mummy-v1` passes Rotator, enters Outrageor's base, and returns the
treasure inside the mummy. It waits for Eviscerator's surrender call and answers
Jerry's subsequent call. The saved state has the mummy with Outrageor,
Eviscerator at the machine, and Yoko dispatched to Kukaracha.
`cb-kukaracha-rescue-v1` visits the prison ship, plays the two rescue sequences,
and teleports Yoko and Maxxon aboard. Jerry's subsequent call reveals Oddland.
Both rescued passengers are aboard in the saved checkpoint.
`cb-return-through-oddland-v2` hears both passengers' thanks and returns the
Ark to Oddland. Its attempted entry leaves the camera closed; the saved
endpoint remains in SCRIPT4. `cb-final-oddland-crossing-v2` loads that earned
arrival, opens the camera, and enters through the left control at `(25,145)`.
The previous `(60,130)` coordinate overlaps chapter four's Ekato chart marker
and selects that planet instead. The successful saved checkpoint is in
SCRIPT5 with the chapter's normal passenger roster and three bionium.
`cb-rondo-homecoming-v1` teleports Yoko and Maxxon home, visits both locally,
and wakes Hom after their departure. Hom moves to Kortex. Following him and
providing the requested mouse print completes his exam briefing. The saved
endpoint has `G1=1`, Hom at Kortex, Yoko at the observatory, Maxxon on Rondo,
and Kran Dobu's call pending; the U.R.O.U.T. diploma has not yet been earned.
`cb-urout-diploma-v2` answers Kran Dobu, passes Cyberquizz's first five questions
through their visible menus, and teleports the diploma aboard. Returning to
Hom and providing the second mouse print presents the Big Bang coordinates.
The saved endpoint retains the diploma aboard, `Bof=5`, `quest=5`, and Hom at
Trashlando. The failed v1 opening-line loop is excluded; v2 explicitly records
the original-oracle-backed skipped-A9 runtime repair described above.
`cb-big-band-arrival-v2` visits the closed Venusia shop, answers Hanna Scruta,
and follows Hom's coordinates to the Big Band Club. Bug Deluxe's wedding
announcement and Bob's complete Big Bang / Big Band explanation are presented.
The normal save confirms `exp=1`, `bok=1`, and the unchanged eight-entry aboard
roster. This run observes 29 decoded video resources and 63 fully revealed
text sites. The failed v1 blank-caption attempt is excluded; v2 records the
original-backed caption repair. Bob's explanation is not the concert ending.
`cb-wedding-ring-v1` follows Tina to Migrator's first bar conversation, receives
the wedding-ring request, and wakes Ondoyant through the ordinary cryobox.
Her handover is observed, and the normal save confirms the ring aboard, Yoko
at Bigbang, Tina at bar2, and `bok=1`. Its 27 decoded resources and 40 fully
revealed sites precede the wedding concert; they do not establish its ending.
`cb-wedding-concert-v2` then follows Yoko, Fifi, Tina, and Migrator through the
remaining wedding conversation and teleports the earned ring to Migrator.
The observed `finalmen` chain has all 22 sequence requests in authored order:
18 music clips, three Bob/Honk intercuts, and `FIN.HNM`. FIN starts at native
boundary 8259; its maximum observed decoder counter is 263, and boundary 8522
has no active or draining video. The native process exits with code zero.
`--expect-cb-ending` checks the SCRIPT5 ending site, Migrator/concert ownership,
ring and Ark locations, ordered decoded-and-closed sequences, and clean exit.
Only the final passive wait may be interrupted. Wrong choices, missing/reordered
clips, an open source, or a crash fail validation. All 24 recorder tests pass.
The earlier v1 reached the same natural exit but lacked this CB-specific
completion check and remains a failed recording, not a rewritten witness.
This completes the observed main CB story route, not every distinct alternative
and not original-executable rendering/timing parity. Rendering is still deferred.
`cb-urout-failed-exam-v1` is a separate legitimate alternative from the Rondo
homecoming checkpoint. It selects a wrong visible answer for every one of the
32 questions, presents the full failure response, and returns to the bridge.
Its normal save confirms `quest=0`, `Bof=0`, and the diploma still held by
Cyberquizz. The run observes 17 decoded resources and 117 fully revealed text
sites. The later correct-answer responses need separate witnesses because five
correct answers terminate the exam; they are not implied by this failed attempt.
`cb-exam-answers-06-10-v1` delays those five correct answers until questions
6-10, then accepts the diploma. Its normal save confirms the diploma aboard,
`Bof=5`, and `quest=10`; 18 decoded resources and 53 fully revealed sites are
observed. This is a distinct answer path from the same earned Rondo checkpoint.
`cb-exam-answers-11-16-v1` and `cb-exam-answers-17-21-v1` instead recontact
Cyberquizz from the normally saved failed exam. They earn and teleport the
diploma at `quest=16` and `quest=21`, respectively, with `Bof=5`. Q11's Vista
response is observed without awarding a point, matching the authored script.
Their 63 and 78 fully revealed sites are separate route witnesses, not an
enumeration of every possible answer combination.
`cb-exam-answers-22-26-v1`, `cb-exam-answers-27-31-v1`, and
`cb-exam-answers-32-v1` complete the remaining positive answer paths. Each
normally teleports the diploma aboard and saves with `Bof=5`, at `quest=26`,
`quest=31`, and `quest=32` respectively. Their fully revealed site counts are
93, 108, and 111. Together with the earlier pass/failure routes, a join against
the hash-matching original SCRIPT5 COD/DIC catalog observes 128 of the 129 text
sites in procedures Q1-Q32, including every question and both answer responses.
The remaining site, COD `0x41B6` (the question-32 wrap-up), is not published by
either final-question route. The earlier pass/failure handlers take over once
`quest=32`; it is kept absent, not manufactured by changing the score or guards.
This bounded exam audit is not a claim that every other CB alternative is covered.
`cb-party-crew-v2` separately visits Bronko, Anna Haf, Beauregard, and the
receiver from the Big Band arrival checkpoint. Bronko's wedding exchange and
the three short responses are observed, with all four still aboard at the
normal save. The run has 24 decoded resources and 21 fully revealed sites.
The earlier v1 used the wrong cryobox hit location and is excluded.
`cb-party-planets-v1` follows with ordinary visits to Ekatomb, Corpo, Moskito,
Eden, and Kult. Daddy's family thanks, Izwalito's departure, Emasculator's shop
closure, Amigo's closed-bar conversation, and Scruter Mac's pilgrimage/history
dialogue are all observed. The saved checkpoint has Daddy, Izwalito, and
Emasculator at Trashlando, Amigo still at the bar, and Scruter Mac at Kult.
The run has 34 decoded resources and 33 fully revealed text sites.
`cb-party-bad-password-v1` visits the chapter-five Mastachok guard and selects
the displayed `galabar` answer. The refusal, laser threat, and `explo3.hnm`
sequence play before the normal bridge save. Its 11 decoded resources and
nine fully revealed sites are a separate alternative, not a prison-entry witness.
`cb-party-prison-v1` selects the correct password and follows Eviscerator's
war, treasure, and secret topics in chapter five. His Splatch request and
Honk's "not a never ending story" response are observed. The saved state has
`D1=1`, Eviscerator still in prison, and Splatch still held by Amigo. Its
19 decoded resources and 25 fully revealed sites do not establish an escape.
`cb-party-prison-return-v2` re-enters through the correct password and presents
the missing-Splatch complaints. The saved state retains `D1=1`, Eviscerator in
prison, and Splatch with Amigo. It observes 15 decoded resources and 15 fully
revealed sites. V1 left the chart open and never reached the guard; it is excluded.
`cb-early-party-crew-v1` returns to the earned chapter-five arrival and visits
Bob, Hom, and Maxxon before taking Yoko and Maxxon home. Bob's urgency dialogue,
Hom's reminder, and Maxxon's transport request are observed. After reaching
Rondo, both visible `refuse` choices are selected; the save retains all three
passengers aboard with `G1=0`. It observes 30 decoded resources and 21 fully
revealed sites, without advancing the homecoming/exam route.
`cb-party-tv-v1` opens the bridge television and advances normally through all
six chapter-five channels. The trace contains complete ordered first passes of
`match`, `ppit`, `hatetv`, `venus` (eight consecutive clips), `scrut` (six clips),
and `present`, followed by a normal close and save. It observes 43 decoded
resources. The recorded extra loops and partial second passes are not additional
anthology chapters. Decoder counters are not used as presentation durations.
`cb-investigation-tv-v1` returns to the earned SCRIPT3 customs checkpoint and
watches its first channel. The complete `microkid` sequence runs in authored
order (`oollee01`, `bbar`, `oollee10`) before a normal close and save. The saved
game hash is unchanged from the predecessor. Its duplicate loop is retained
as evidence, not another distinct scene.
`cb-curse-tv-v1` uses the earned chapter-four portrait-recovery checkpoint and
watches the curse channel through its complete `maledict.hnm` sequence. It
returns and saves with `maledict=1`; the repeated broadcast is a single distinct
scene for assembly. No curse or channel state was assigned externally.
`cb-fifi-scrambler-v1` selects `use` on the first chapter-four Ron visit and
then leaves through the displayed farewell. The authored `brouil.hnm` request
decodes and closes normally. The run saves with 11 decoded resources and 21
fully revealed sites; it is the distinct alternative to the main route's refusal.
`cb-bob-good-v2` follows Bob's chapter-four `good_` topic through four ordinary
selections and leaves normally. It presents the Ark-status and flattery lines,
with ten decoded resources and eight fully revealed sites. The BAS source's
`arch01.hnm` request does not decode on this route, and no matching imported
asset exists. This absence is recorded rather than replaced with another Ark
clip or called globally unreachable.
`cb-briefing-no-v2` repeats the fresh-game briefing alternative with the current
runtime. The observed Ark sequence remains `aarche10`, `aarche30`, `aarche40`,
each closing normally; `aarche20` is still absent. The run observes 17 resources
and 51 fully revealed sites. Static request listings alone must not add the
missing clip to that route.
`cb-bob-revelation-v2` follows the self-history topic to the offered revelation
and saves with `reve=1`. `cb-bob-revelation-return-v3` recontacts him normally,
presents the identity-revelation dialogue, leaves through the farewell, and
saves with `reve=0` and `revelat=1`. The second run observes 12 decoded resources
and 21 fully revealed sites. Its v2 predecessor attempt presented the revelation
but remained in Bob's menu and did not save; only v3 is a saved continuation.
`cb-bob-kanary-probe-v1` records a remaining nested-menu blocker. The ordinary
click selects dictionary offset 11703 (`kanary`) at boundary 2490, commits its
BAS body 5503 at 2491, then returns the control value to 1 (`talk`) at 2492.
The main menu reopens at 2494 without presenting the employee submenu. The
probe leaves normally and saves with `emplo=0` and `emp=0`; it is not employee
dialogue coverage. Read-only trace fields now retain the selector root, current
and parent controls/bodies, pending concept, and eight-entry concept history.
No gameplay behavior was changed. All 28 existing original `0x56FE`/`0x5791`
selector oracle cases pass, as do 1,115 game-library tests (78 ignored) and 40
focused flow tests. These results do not resolve the surrounding-frame behavior
or establish that the submenu is unreachable in the original game.

The follow-up `cb-bob-kanary-delayed-probe-v4` uses only a delayed ordinary
click. It reaches the chapter-four submenu and selects `employees` (dictionary
offset 12244), but returns to the main menu without the employee replies. It
saves with `emplo=0`, `emp=0`, and SHA-256
`9df77c787b2689e22ac138a725fd6ea7c37dff4b028fc4f3a2e904e7efaabe34`
at boundary 5028. Merely observing this menu is not dialogue coverage.
`re/tools/commander_bob_selector_probe.py` independently reproduces the reset
with unmodified original interpreter instructions, the actual SCRIPT4 COD/BAS,
and the earned `cb-bob-revelation-return-v3` save. It executes the `boba1` slice,
the original selection commit, and the character scan up to its action-maintenance
tail. Open and closed scene-gate cases agree: an open gate can publish the
Kanary menu, but the following COD BC at 19966 writes `talk` to both the actor
field and GS:6782. Selecting `employees` then executes the main body 4332, not
Kanary body 5503. The checked report is
`output/game-flows/oracles/cb-bob-selector-v3.json`; its independent rerun is
byte-identical. The probe reports its explicitly seeded contact/frame inputs,
checks interpreter bytes and return-stack boundaries, and allows only the
original PRNG's five code-segment data bytes to change. It neither patches a
handler nor proves a complete DOS playthrough or global unreachability. No
runtime workaround was added to expose these authored but unwitnessed replies.

`cb-bob-kanary-early-v3` instead starts from the earned chapter-two
`cb-corpo-delivery-v1` checkpoint and uses the ordinary three-entry contact
list. The actual topic here is `Kanary`, not chapter four's lowercase label.
It witnesses all three employee lines, five cottage lines, five cloning lines,
four balance-sheet lines, and the non-random Kanary summary. Honk's six-line
raise request also runs normally. The result has 6,048 observed frames, 528
retained events, 13 decoded resources, and 31 fully revealed sites. It saves
with `emplo=1`, `emp=1`, and `BO=2` at boundary 5912, SHA-256
`a779e61d10e0a5a882de6b24dfdef517cb5966bcfa58299c119980924d859a9e`.
The probabilistic Kanary line and aggressiveness-dependent replies remain
separate coverage questions, not inferred from this successful conversation.
`cb-bob-early-topics-v2` continues that save. Returning from the saved Kanary
submenu through `talk` exposes the black-hole, Big Bang, mission, Corpo,
flattery, self-history, and revelation topics. Its 35 fully revealed sites all
match their authored wording; it saves with `BO=3` and `reve=1` at boundary 6493,
SHA-256 `bcd14052afb067544f6f42e25093a3e52f10e766efce61ff6cd79e256c10cabf`.
The ordinary Ark-status request plays `AARCHE10.HNM` at boundaries 4061--4398,
reaching counter 337 (the source-derived 338-frame lower bound minus one) and
closing normally. This is the requested single clip, not the multi-part opening
Ark sequence. The run has 6,629 frames, 629 events, and 15 decoded resources.
`cb-bob-early-revelation-v1` recontacts him, fully presents the sixteen authored
revelation lines and his third-visit water dialogue, then leaves normally.
It saves with `BO=4`, `reve=0`, `revelat=1`, and `plus=1` at boundary 6454,
SHA-256 `600aeeeaa76af74a46ca824940e866ccb5a5133d2e7f751ffcde1b1fb0cd642c`.
Its 6,590 frames retain 509 events, 13 decoded resources, and 26 fully revealed
sites. A targeted source/wording/checkpoint/lineage audit accepts all sites in
both runs without any rejected attribution. These are chapter-two alternatives;
the later chapter-four revelation remains separately witnessed.
`cb-bob-early-irritation-v1` is a normally saved fourth contact, despite its
provisional probe name. It fully presents the probabilistic Kanary empire line,
five repeated cottage replies, and the fourth-visit introduction and farewell.
Its saved aggressiveness is 24, not either authored irritation threshold, so
neither irritation reply is claimed. All eleven fully shown sites pass the
source-wording audit and all eight predecessor manifests/checkpoints validate.
It records 4,761 frames, 261 events, and eleven decoded resources; save zero is
at boundary 4625 with SHA-256
`90b0b04610cdcffa683d57de1daff4b48ac7558c2f75053db912bf8795384957`.
`cb-bob-early-food-v2` continues through the fifth and sixth contacts. The
ordinary Kanary summary raises aggressiveness from 24 to 25; the sixth contact
fully presents all five food-exchange lines and the distinct threshold-25 BAS
reply at 19569 with `borb_ctr.hnm`. No state value or encounter counter was
assigned externally. All thirteen fully shown sites match their authored text,
and the nine-manifest saved lineage validates. It records 7,766 frames, 511
events, and twelve decoded resources. Save zero at boundary 7630 has SHA-256
`10b4fe7b2b11491607e1ba2371885aaaad14ea7fbec0dcf89f42dfd84bdc4820`;
Bob's saved encounter count is six and aggressiveness is 25. V1 used a missing
asset-directory path and failed before launching the game; it is not evidence.
`cb-bob-early-thresholds-v1` branches from the same earned Corpo-delivery save,
asks about the mission, Kanary, cloning, and the cottage, then recontacts Bob
twice to repeat the cottage topic. It witnesses the aggressiveness-10 and -15
BAS replies at 19477 and 19499 on subsequent contacts, with `bobh.hnm` and
`bobi.hnm`. It saves at aggressiveness 20 after contact three; that is not yet
evidence of the threshold-20 reply. All forty fully shown sites match source
wording, and all five saved-lineage manifests validate. The run records 11,455
frames, 1,005 events, and fifteen decoded resources. Save zero at boundary 11319
has SHA-256 `998923338229238b71d0bcf972231343d79d3c11291f7b681498de3c1ae57a40`.
`cb-bob-early-twenty-v1` recontacts that save and fully presents the threshold-20
reply at BAS 19537 with `bobj.hnm` before selecting another topic. It also
repeats the Ark/flattery branch but does not witness the probabilistic
embarrassment reply. All ten fully shown sites pass the source-wording audit,
and the six-manifest saved lineage validates. It records 5,731 frames, 351
events, and twelve decoded resources. Save zero at boundary 5595 has SHA-256
`522b22c03402f88d6931b34d3f14542602fda6457f87145d2f82ad088f714629`.
The separate `cb-bob-early-timeouts-v1` probe remained in the first conversation
despite its long waits. Later contact coordinates instead selected topics in
that still-open menu. It was stopped at 35,648 observed frames with only 34 of
53 actions completed, and has no valid saved checkpoint. Its retained recipe
and events describe a failed timeout experiment, not four visits or evidence
that the authored timed-departure dialogue is globally unreachable.

`bbb-honk-cave-clue-v3` reaches the cave-puzzle briefing through three ordinary
Honk calls: the first retires the one-time `objet` procedure, the second accepts
the puzzle, and the third explains the required objects. The saved result has
`A60=7`, `A53=4`, and `A80=0`. This required no runtime change. The bounded
original-executable probe `re/tools/big_bug_bang_honk_cave_probe.py` reproduces
the interruption from the earned save: the first text publishes at COD `0x240b`,
then C9 at `0x3383` clears the conversation and disables `objet`. It executes
the original interpreter and authored Honk block, not a whole DOS playthrough;
its input hashes and register-boundary observations are retained under
`output/game-flows/oracles/bbb-honk-cave-pass-v8.json`.

`bbb-zen-cave-gifts-v3` returns the optics, guitar, and one bionium to Super Zen,
solves the letter puzzle in that same encounter, and receives Izwalito's call.
Its saved endpoint has `A80=3`, `A81=6`, and Marakas aboard. The earlier v2
attempt crashed in the native growth selector after the rescue and is not a
valid predecessor. The original selector at `0x70b0` skips actors at the aboard
sentinel; two added original-executable vectors reproduce that behavior and
failed against the old implementation. The repaired selector passes all 128
growth vectors and the full game-library suite (1,115 passed, 78 ignored).
The successful replay records the binary transition explicitly; its save,
scripts, assets, and earlier lineage remain hash-checked.
`bbb-marakas-rondo-v1` then selects Marakas from the cryobox, sends him to the
unoccupied Rondoland, and visits him there. The `force` response presents his
gratitude exchange. The saved result confirms Marakas at Rondoland and `A81=7`.
`bbb-emasculator-nuclear-return-v2` completes the fifth Attrox visit, buys the
discounted nuclear pile, and gives it to Emasculator. He returns it. The saved
endpoint has four credits, the pile aboard, and `B96=2`. The failed v1 chart
attempt recontacted Marakas instead and is excluded from the continuation.
`bbb-rotator-nuclear-v1` visits Rotator three times, observes the two introductory
sequences, answers his third-visit price questions, and offers the pile. Rotator
returns it; the saved endpoint has `B96=3`, four credits, and three visits.
`bbb-optics-guitar-again-v1` purchases replacement optics from Otto Von Smile
on Foxx and another guitar from the treated Ben Zen on Tumul. Both objects are
aboard in the saved endpoint, with two credits remaining.
`bbb-gluk-second-ship-v1` returns to the established Cyberock negotiation with
those two credits. Both affirmative dialogue choices and the low-credit
exchange complete normally. The saved checkpoint has the ship aboard, `A4=0`,
and `A34=2`; no credit or inventory state was supplied externally.
`bbb-trump-colony-v1` watches Trump's origin film, returns for the migration
instructions, and gives him the optics, ship, and remaining bionium. The saved
endpoint has `B26=2`, `vbio=0`, and Tramp's newly active Magnusland colony.
`bbb-tramp-colony-v1` visits that colony, gives Tramp the replacement guitar,
and answers Trump's warning call. The saved checkpoint has `A86=1`, the guitar
with Tramp, and Super Tromp's active colony at Corpoland.
`bbb-gluxx-resupply-v1` makes two ordinary visits to Daddy to renew the existing
loan repayment, obtains writing through Mamy's second-visit answer, buys her
perfume, and buys medicine during Papy's fourth visit. The saved checkpoint
has all three items aboard, the retained technology, and eight credits.
`bbb-super-tromp-repair-v1` reaches Super Tromp at Corpoland, hears his initial
malfunction, and returns with medicine, writing, and technology. All three
ordinary gifts and the repair dialogue complete. The saved checkpoint has
`B87=2`, all three items with Super Tromp, his returned ship aboard, and eight
credits remaining.
`bbb-bernie-negotiation-v2` completes Bernie's Internet negotiation, pays the
requested credit, and chooses `don't_cheat` at the password prompt. Trump's
spy warning then completes. The saved checkpoint has `C7=5`, `B82=0`,
`A86=2`, and seven credits. The earlier v1 stopped at the password prompt and
has no new saved checkpoint; it is not used as a predecessor.
`bbb-trump-spy-v1` follows that warning to Golgoland and solves Trump's
three-letter identification puzzle. The next Internet call declines the
optional password shortcut again and hears Tequila's invitation. The saved
endpoint has `A86=3`, `A87=1`, `A73=1`, and `B95=0`; the later presidential
conversation and war have not yet occurred.
`bbb-jo-spy-resupply-v1` returns to Jo through the cryobox and replays the
`navigate:12` input plan through the normal flight. The live invocation records
102,368 presented input frames and a completed return; the subsequent dialogue
and ordinary saved checkpoint confirm `vbio=12`. This is an earned resupply,
not the offline planner's result substituted for native gameplay.
`bbb-tequila-spy-v2` visits Tequila on Goanland, offers the collected bionium,
and chooses `don't_give` at the ten-dose prompt. The authored split-dose
continuation plays, Tequila comes aboard, and the saved checkpoint confirms
`A87=7`, two bionium, seven credits, and the retained perfume. The earlier v1
failed before producing a trace because it was launched outside the graphics
development environment; it is not a route witness.
`bbb-presidential-call-v1` makes the two follow-up Internet calls, declines
the password shortcut each time, and answers Chigraxx's queued call with
`talk` and `hello`. The conversation and ensuing war activation complete;
the saved checkpoint has `B95=3`, `B24=1`, `A73=1`, seven credits, and two
bionium. It records the same skipped-A9 runtime update as CB's repaired exam.
`bbb-zen-war-treaty-v3` accepts Izwalito's free wartime treaty, travels to
Super Zen's observed Spiraland settlement, answers his visit-dependent word
game with `no`, and hands over the treaty. The complete peace dialogue and
checkpoint confirm `A73=2`, `A74=1`, `B24=0`, and seven credits. Super Zen has
then moved to Voluptland. The failed v1 overview crash and incomplete v2
missing-choice attempt are not checkpoints. V3 explicitly records the
original-backed aboard-actor map repair.
`bbb-crown-supplies-v2` buys replacement optics from Otto Von Smile at Troma
and a guitar from Ben Zen at Magnus, including Ben's fourth-visit dialogue.
The normal save confirms optics, guitar, perfume, and food aboard, five
credits, and `A74=1`. The failed v1 used Ben's pre-load Tumul location; the
live loaded state instead places him at Magnus. Super Zen is now at Cyberland,
so later navigation must again use the current simulation state.
`bbb-crown-time-door-v2` makes the follow-up Internet calls, chooses `yes` to
identify Cassandre's warning, and declines the optional password shortcut.
The four-senses clue and time-door announcement both play; the normal saved
endpoint has `C9=1`, `A74=3`, five credits, and Outrageor at Edenland.
Super Zen remains at Cyberland with eight encounters. The run observes eight
decoded resources and 56 fully revealed sites. V1 omitted the new yes/no
question and is excluded; its planned waits were not evidence of progression.
`bbb-outrageor-crown-v1` enters Eden, gives Outrageor optics, guitar, perfume,
and food through the normal gift menu, then accepts the one-credit crown offer.
The saved checkpoint confirms the crown aboard, `A79=4`, `A74=5`, four credits,
and Outrageor at Trashlando. Super Zen is still at Cyberland with eight visits.
The 21 decoded resources and 37 fully revealed sites are observed; the ring and
Super Zen's subsequent explanation are not yet implied by this checkpoint.
`bbb-zen-crown-ring-v1` reaches Super Zen at Cyberland, answers his ninth-visit
yes-or-no game, gives him the crown, and accepts his ring offer. The explanation,
illustrating sequences, returned crown, and farewell all play. Its normal save
confirms ring and crown aboard, `A74=6`, `A72=1`, `A23=1`, and four credits;
`B81=0` still awaits Honk's announcement and Bob's return. The run observes
29 decoded resources and 86 fully revealed sites.
`bbb-bob-future-v3` makes the two ordinary Honk contacts needed after loading,
then selects Bob from the actual twelve-entry cryobox list (third, after
Tequila and Scruter Jo). Honk's announcement and Bob's complete return/future
explanation play. The normal save has `B81=2`, `B10=1`, `B7=1`, Blue Wave at
Waveland, Betakam at Bonusland, and Bob and the three Zens at Trashlando.
The run observes 22 decoded resources and 26 fully revealed sites. V1 missed
Honk's object-search interruption; v2 completed its inputs but contacted Tequila
instead of Bob and reached only `B81=1`. Neither establishes this quest result.
`bbb-blue-wave-gifts-v1` meets Blue Wave on Ondoya, answers the opening riddle,
and gives the earned crown, picture, decoder, weapons, and nuclear energy.
The saved checkpoint verifies all five transfers, `B79=5`, Blue Wave's first
encounter and 1,000 energy, with the ring still aboard. The run observes
29 decoded resources and 51 fully revealed sites; awakening Blue Wave still
requires the mummy and ring sequence.

`bbb-blue-wave-bionium-first-v1` instead gives one bionium from the earned
Bob future checkpoint. `B79` rises zero to one while the stock remains two;
the shared token returns on closing the encounter. All nine original items
are aboard at the normal save, including crown, decoder, weapons, ring, and
nuclear. This is the start of an inventory-preserving alternative, not yet a
replacement for the five-gift route. The run records 7,033 frames, 967 events,
26 decoded resources, and 42 matched sites. Slot zero at frame 6897 has
SHA-256 `17a13043423cae169ac605140e3cc2ae83f0d0f4e1d06e2c551b3d6f845a2e5b`.
`bbb-blue-wave-bionium-audit-v1/first.json` rechecks 72 distinct lineage
manifests, current source/input hashes, the rendered menu, actual transfer,
stock, retained inventory, and the closed checkpoint.
The second through fifth witnesses reuse `flow_bbb_blue_wave_bionium_repeat_v1.tsv`
once each after an ordinary load of the preceding save. Each raises `B79`
by one, reaching five, while preserving all nine items, four credits, and
two bionium. Each records 5,832 frames and seven matching sites, with no new
wording from repeating the gift. `bbb-blue-wave-bionium-audit-v1/five-visits.json`
verifies all five leaves through 76 distinct lineage manifests. The fifth
slot zero at frame 5696 has SHA-256
`446833e86fedeb5c088371fd30e5b3c723fcc708c3e3391047b6952a875560c4`.
A combined same-planet repeat attempt failed during navigation and remains
excluded; its unsaved progress is not a predecessor. The successful visits
establish the gift threshold only, not the later retained-inventory story.

`bbb-blue-wave-retained-ring-v2` continues the fifth save through Betakam's
credit/mummy offer and Blue Wave's mummy/ring sequence. Blue Wave and energy
are aboard, mummy and ring are hers, and the crown, picture, decoder, weapons,
nuclear, ship, bionium, and credit remain aboard. `A26=1`, `B79=5`, credits
rise to five, and bionium remains two. The run records 17,303 frames, 1,274
events, 30 resources, and 54 matched sites, with no new wording. Its normal
slot-zero save at frame 17167 has SHA-256
`e3a385d9fcb13dc3762f50ca73ce563525b325848fc99177e2e9b1cbd9a9578f`.
`bbb-blue-wave-retained-audit-v1/ring.json` checks 77 lineage manifests,
current source/input hashes, text, inventory, and the closed endpoint.
The failed v1 composition toggled travel on at frame 1736 and back off at
7492, so its second arrival never contacted Blue Wave. The new continuation
explicitly retains enabled travel after Betakam. This is an input-script
correction, not a runtime change or an assertion that the later robot gifts
are already reachable.

`bbb-blue-wave-credit-ship-hint-v1` is a separate alternative from the third
visit. It offers credit, ship, and the ring before obtaining the mummy, then
chooses `help` in the displayed `help`/`pride` menu. Blue Wave takes all four
credits and the ship, gains ten evolution, and returns the premature ring.
`B79` rises three to four, `B47=1`, and `A26=0`; she remains on Waveland.
The complete `SQ/momi10.hnm` hint sequence closes. The run records 7,623
frames, 730 events, 17 resources, and 21 matched sites, including 18 newly
observed sites relative to audit-v13. Slot zero at frame 7487 has SHA-256
`334915efeebf9b897b45ea1bd11b454c658c516a3fa22a8e2a4cfbc30a5046d0`.
`bbb-blue-wave-hint-audit-v1/report.json` verifies 75 lineage manifests,
current source/input hashes, text, visible choices, the three transfers,
credit loss, completed hint sequence, and closed checkpoint.

`bbb-kam-mummy-v1` visits Betakam on Bonus and accepts his ordinary offer.
The saved endpoint has the mummy aboard, five credits, and the ring still
available. Its 11 decoded resources and seven fully revealed sites are a
verified continuation from the five-gift checkpoint.
`bbb-blue-wave-ring-v1` gives Blue Wave the mummy, then the ring, and accepts
her request to come aboard. The complete transformation dialogue and sequence
intercuts play. The normal save has `A26=1`, `B79=5`, Blue Wave and her energy
aboard, the picture returned, and both mummy and ring held by Blue Wave.
Its 24 decoded resources and 47 fully revealed sites precede the aboard show
and strike storyline; `A61`, `A64`, and `A96` are still zero.
`bbb-blue-wave-show-v1` selects the show, alternate camera, and extended view,
then declines another performance. All nine authored refusal-chain sequences
are observed to close. The saved endpoint has `A96=2`, `A61=1`, and Smile's
complaint queued, with 34 decoded resources and 59 fully revealed sites.
`bbb-blue-wave-refuse-v1` separately declines the initial offer and saves at
`A96=1`, `A61=1`, with the same queued caller. It observes 20 resources and
19 fully revealed sites; shared sequence material should not be rendered twice.
`bbb-blue-wave-pass-v1` declines the alternate camera after the initial show;
it saves at `A96=1`, `A61=1`, with 31 decoded resources and 46 fully revealed
sites. `bbb-blue-wave-decline-extended-v1` accepts that camera but declines the
extended view; it saves at `A96=0`, `A61=1`, with 33 resources and 51 fully
revealed sites. These are one witness per distinct refusal point, not combined
permutations of repeated shows.
`bbb-strike-calls-v1` answers Smile, Gluk, Daddy, Mamy, and Papy in their queued
order. It lets Daddy finish and accepts Papy's demand through the displayed
choices. The normal save confirms `A61=6`; the general strike has not yet
been triggered. The run observes three decoded resources and 83 fully
revealed sites across the phone conversations.
`bbb-general-strike-v1` visits Daddy on Troma, refuses the pay increase, and
answers Chigraxx's ensuing call. The complete `cliptoot.hnm` strike sequence
closes before the call. The normal save confirms `A61=7`, `A68=1`, and `A65=2`,
with eight decoded resources and 28 fully revealed sites. Cyberquizz's invitation
and the later strike resolution are not implied by this checkpoint.
`bbb-cyberquizz-invitation-v1` makes both Internet calls, affirms Honk's
existence, and answers the disguised caller. The ordinary save confirms
Cyberquizz aboard with no prior encounters, `A90=1`, and `A55=1`. The run
observes three decoded resources and 48 fully revealed sites. His aboard
conversations have not occurred at this checkpoint.
`bbb-cyberquizz-first-v1` opens his first aboard contact from that save. The
normal gift transfer completes, with Cyberquizz at one encounter and `cadeaux`
aboard. It observes eight decoded resources and 12 fully revealed sites.
`bbb-cyberquizz-visits-v1` completes visits two through five, including a normal
pointer click on the duplicate-label menu, declining destruction, and Honk's
joke. The new-race gift sets `A100=1` and `B8=1`; the saved world places Mega,
Sebasto, and Inter Paul on Ponyland at evolution 100. Its 13 decoded resources
and 44 fully revealed sites precede their first meeting.
`bbb-jo-paul-resupply-v2` replays a newly planned two-to-six-dose collection
through ordinary Jo entry. The live Amer minigame presents 55,802 frames and
returns `vbio=6`, confirmed by the normal save. No inventory was granted by
the planner. Its predecessor is the verified Cyberquizz route, not the failed
zero-start-plan attempt.
`bbb-mega-paul-first-v1` visits Ponyland, answers the displayed questions with
`or`, `other`, and `Super_Zen`, and pays the requested three bionium. His
clairvoyance explanation completes with `A100=6` and `vbio=3`. The saved
checkpoint keeps Mega Paul on Ponyland at evolution 100; it observes 25
decoded resources and 50 fully revealed sites.
`bbb-paul-writing-v2` gives Cyberquizz's present to Mega Paul, returns through
the local camera, and accepts his writing sale. The ordinary save confirms
Mega's evolution 110, writing aboard, and credits reduced from five to four.
It observes 12 decoded resources and 12 fully revealed sites. The first attempt
used the wrong cancel-row position and has no valid continuation checkpoint.
`bbb-writing-mutation-v2` declines the discounted nuclear offer and gives
that writing to Emasculator. The normal save confirms `A93=1`, `B9=1`,
Eviscerator on Waveland, and Outrageor on Island. It observes 15 resources and
11 fully revealed sites. Its later phone clicks missed the handset because
the bridge faced the wrong direction: `C11` remains zero, so this is a mutation
checkpoint, not a completed distress-call route. The reusable gift fragment
now ends before those ineffective clicks.
`bbb-mutation-calls-v1` faces the handset and answers both queued calls.
Mega Paul's report followed by Eviscerator's distress message advances
`C11` from zero through one to two. The ordinary save follows 16 fully
revealed sites and three decoded resources; no ambush choice occurs yet.
`bbb-eviscerator-arrest-v1` visits Waveland and chooses to fight. The complete
`explroug`, `diplom1v`, and `venthig` sequence requests close normally, and the
save confirms `C11=4` with Eviscerator aboard. It observes 15 decoded resources
and 18 fully revealed sites. Prison/parole choices are subsequent contacts.
`bbb-eviscerator-parole-v1` keeps him in the cryobox, refuses release once,
then grants parole on the next contact. The `crazys20`, `p_cles`, and `match04`
requests all close normally. Its ordinary save has `C11=5`, Eviscerator on
Island, and Outrageor on Waveland, with 15 decoded resources and 16 fully
revealed sites. These are sequential ordinary contacts, not injected alternatives.
`bbb-eviscerator-obey-v2` instead surrenders at the same ambush. The live route
empties credits and bionium, sets `C11=3` and `A13=2`, and reaches the complete
`28bob` failure ending at `0x9f14` with clean native exit. It observes 17
resources and 13 fully revealed sites. V1 used an exact-frame wait that is
invalid in shutdown cadence; its nonzero exit is not accepted ending evidence.
`bbb-eviscerator-commando-v1` visits the paroled Eviscerator on Island, accepts
his mission, and hears both the report and Chigraxx's response. The ordinary
save confirms `C11=9`, `A94=1`, and no ending flag (`A13=0`). It observes 18
decoded resources and 29 fully revealed sites, enabling the return to Mega Paul.
`bbb-paul-scruters-v1` returns there normally and completes the Scruter creation
scene. The save confirms `B88=1`, `B15=1`, Mac and K on Kortland, and Jo removed
from the Ark. Mac is at evolution 100 with no encounters, and his call is queued.
The run observes 18 decoded resources and 13 fully revealed sites.
`bbb-scruter-concert-offer-v2` answers Mac, then Mig Burner, and saves with
`A63=2`. Mac's first phone encounter itself enables the concert proposal.
The run observes nine decoded resources and 21 fully revealed sites. V1's
additional Kortland visit failed on numeric audio dictionary offset 2908:
SCRIPT15.DIC is only 973 bytes. The existing audio oracle proves the original
encoded-offset hashing only within supplied dictionary memory, not the contents
beyond that allocation. No decimal-text, silent-audio, or zero-padding fallback
has been substituted, and that local visit remains uncovered.
`bbb-elvis-quiz-v3` makes the investment and answers the Elvis quiz through
ordinary Internet choices. It waits for the subsequent Scruter messages and
broadcast before saving. The witnessed checkpoint has `A63=8`, `B94=1`, three
credits, and the guitar aboard, with 13 decoded resources and 86 fully revealed
sites. V1 stalled on an unnecessary second call; v2 reached the reward but sent
its save inputs during dialogue and has no checkpoint. Neither is a predecessor.
`bbb-trump-guitar-v1` visits Crazyland, declines the repeat painting offer, and
gives Trump the earned guitar. The ordinary save confirms the guitar with Trump
and `B86=1`, with 12 decoded resources and 20 fully revealed sites. The arrival
opens his encounter automatically; no extra actor click is sent during dialogue.
`bbb-tramp-guitar-v1` collects the next guitar from Internet and visits Loneland.
It presents Tramp's second-visit clip chain, then gives him the guitar and saves
with `B86=2`. The seven requested clips, including `lpm4sc1`, `venthig`, and
`ettamorf`, close normally with counters reaching the decoded source's last
frame boundary. The route observes 19 resources and 63 fully revealed sites.

The separate fresh-start witnesses `bbb-opening-complete-v1` and
`cb-opening-complete-v2` send no skip input. BBB's first `present` pass contains
all 13 authored clips in order at native boundaries 322--4375, with every
counter reaching the source's last decoded-frame boundary. CB's `CLIPTOOT`
pass reaches counter 1257 and closes at boundary 10232 after the complete
`MIND` logo. CB v1 stopped at counter 690 and is not a complete opening witness.
These passive runs continue into attract-mode repetition; retain only the first
complete opening pass, not the later loops or reload startup excerpts. The
boundaries identify recorded evidence and are not video timestamps.
`cb-microkid-tv-v1` opens the first TV channel from the earned Amigo checkpoint.
Its first complete pass is `oollee01`, `bbar`, `oollee10` at native boundaries
2192--2972, with counters 39, 702, and 39. Later channel loops and the partial
last loop are excluded from the intended video. It returns to the bridge and
saves normally; the earlier short first-channel probe is not the full broadcast.

`bbb-super-tromp-guitar-v3` collects the third guitar, buys Super Tromp's formula,
gives him the guitar and spare supplies, and saves with `B86=3`, `A64=1`, two
credits, and two bionium doses. The Ark retains only Tequila, Cyberquizz, Blue
Wave, and the credit/bionium tokens. It observes 20 resources and 77 fully
revealed sites. V1 requested a credit token that had already left the menu;
v2 clicked after the empty inventory had automatically closed, reopening the
encounter and preventing a save. Only v3 is used as a predecessor. The remaining
balances must be spent on separate visits because those tokens return between
encounters, not between gift selections.
`bbb-concert-supplies-v3` makes two further ordinary Corpo trips, buys the
formula again on each visit, and gives the remaining bionium. Its witnessed save
has `A4=0`, `vbio=0`, `B86=3`, and `A64=1`, with only Tequila, Cyberquizz, and
Blue Wave aboard. The run observes 14 resources and 19 fully revealed sites.
The two earlier attempts left the navigation chart open and never reached the
requested choice; they are failed attempts, not predecessors. V3 uses the
normal destination selector even when returning to the same planet.
`bbb-concert-invitations-v1` answers Trump and Tramp through the incoming-call
panel. The normal save confirms `A64=3`, `A68=2`, `B98=21`, and `A13=0`.
Fourteen guests are aboard; the world update places Emasculator on Malusland.
The automatically selected `47robinv` broadcast contains its 11 authored clips
in order at native boundaries 5305--5882, with every decoder counter reaching
the source's last decoded-frame boundary. The route observes 14 resources and
22 fully revealed sites. The invitations alone are not concert playback or an
ending witness.
`bbb-concert-v1` calls Honk from that checkpoint and plays all 13 `2concert`
clips in authored order at native boundaries 1629--2505. Every counter reaches
the source's last decoded-frame boundary. It returns to a normal save with
`A13=0`; this is the shared predecessor for the final reactions, not yet the
successful ending. It observes 13 resources and one fully revealed Honk site.
`bbb-cyberquizz-blow-v1` independently repeats the second and third cryobox
visits, then selects the fourth visit's visible `blow_it_up` choice. All five
`5exploplane` clips play in order at boundaries 7129--7363 and reach the source
counter boundaries. It saves after the broadcast, observing 14 resources and
20 fully revealed sites. This is a legitimate alternative to `leave_it`, not
a prepared sequence-channel capture.
`bbb-concert-reactions-v1` contacts Eviscerator, Outrageor, Rotator, Trump,
Tramp, Super Tromp, Marakas, and Izwalito through the ordinary cryobox. Eight
distinct reactions advance `A13` from zero to 24 before a normal save. It
observes 64 resources and 33 fully revealed sites. Both `12concrool` passes
match the authored ten-clip order; all three `34contromp` passes match its
eight-clip order. Every non-startup sequence counter reaches the source's last
decoded-frame boundary. Repeated broadcasts are shared material for assembly.
`bbb-concert-alternate-reactions-v1` independently loads the post-concert save
and contacts Daddy, Mamy, Papy, Smile, and Gluk aboard, then visits Emasculator
on Malusland. Its six distinct reactions produce `A13=18` and a witnessed save,
with 22 resources and 21 fully revealed sites. The direct `match02` and `star2`
clips also reach their source counter boundaries. This branch preserves the
reactions omitted by the main route's ninth-contact ending threshold.
`bbb-rotator-money-v2` loads the earned third-visit checkpoint, returns to Kult,
and selects the fourth encounter's `want` choice. It plays the complete
11-entry `39argent` broadcast at boundaries 2864--3216, including all nine
consecutive `PION` entries, then saves normally. It observes 11 resources and
five fully revealed sites. V1 was rejected before gameplay because its
predecessor used an older binary; v2 explicitly records the already-verified
runtime repairs while preserving the exact earned save and source hashes.
`bbb-concert-ending-v2` loads the eight-reaction checkpoint and contacts Tequila.
Her reaction raises `A13` from 24 to 27 and selects SCRIPT2's successful ending
at `0x9F2E`. All 15 `48finbob` entries, `BOBB` followed by `FIN1` through
`FIN14`, play in order at native boundaries 1903--4387. Every counter reaches
the source's last decoded-frame boundary, and the native process exits cleanly
at boundary 4388, interrupting only the final passive wait. The recorder now
classifies this as `bbb_success` only after checking that ordered decoded and
closed clip chain in addition to the ending assignment. All 25 focused flow
tests pass, including rejected missing, reordered, undecoded, and unclosed
ending clips. V1 independently reached the same ending before this stronger
check was added. This establishes a successful earned BBB route, not complete
alternative coverage or DOS parity.
`bbb-trump-painting-v1` takes the painting sale instead of giving the concert
guitar. Its ordinary save has two credits, with both painting and guitar still
aboard and `B86=0`. All six `23tableau` clips play in authored order at boundaries
3148--3375 and reach their source counter boundaries. The route observes 17
resources and 12 fully revealed sites.
`bbb-rotator-joke-v1` revisits the earned fourth-visit checkpoint and accepts
the fifth visit's `anyway` choice. The three joke clips (`match02`, `matchfin`,
and `ppit07`) precede the normal seven-clip explosion ending at `0x9EF3`.
All non-startup sequence counters reach the source boundaries; the process
exits cleanly at boundary 3909. It observes 23 resources and 22 fully revealed
sites. This is a distinct failure branch, not part of the successful route.
`bbb-concert-bob-clue-v1` calls Internet with the concert route's zero-credit
balance and selects `don't_cheat`. Bob's money hint preempts the later boss
menu and ends the call; the ordinary save records 16 fully revealed sites.
Attempts that waited for `no_strike` or `strike` on this same balance never
reached those choices and are rejected, not treated as boss-choice witnesses.
`bbb-concert-funded-v1` independently continues the earned third-guitar save
without spending its remaining two credits and two bionium doses. It answers
both invitation calls, watches the robot broadcast, calls Honk, and watches the
concert before saving. The combined 24 sequence entries match `47robinv` then
`2concert` exactly and reach their source counter boundaries. Its endpoint has
`A13=0`, `A64=3`, and `A68=2`. Thirteen guests are aboard; Gluk remains on
Cyberland, unlike the main route's fully supplied invitation result. This is
an alternative credit-retaining predecessor, not a replacement for the verified
successful route and its separately recorded Gluk reaction.
`bbb-concert-internet-v3` loads that credit-retaining checkpoint, selects
`don't_cheat`, and declines Ulikan's post-concert strike. The conversation
closes and saves normally with `A4=2`, `A13=0`, `A68=2`, and `A74=6`.
It presents 49 fully revealed sites, including the continuation after refusing
the strike. No new sequence movie plays on this branch. The earlier zero-credit
attempts could not reach this choice and are not predecessors.
`bbb-concert-internet-strike-v2` independently takes the visible `strike`
choice from the same earned checkpoint. It sets `A68=3`, selects the ending
at `0x9F14`, and plays all five `28bob` entries in source order at native
boundaries 2635--2901. Every counter reaches the source's last decoded-frame
boundary. The process exits cleanly at boundary 2902, interrupting only the
final passive wait. It observes eight resources and 17 fully revealed sites;
this is the distinct dismissal ending, not successful story completion.

Verified prefixes currently include CB's explanations/Honk/Bob-briefing route
into SCRIPT2, Bob's `no` response (one additional fully shown site at `0x0813`
and `bobg.hnm`), the normal Corpo jump, and Izwalito's first two conversations.
The second earns the credit, plays `OB\\pion.hnm`, reveals Moskito, and returns
to the bridge. The subsequent normal route buys Bronko's murffalo meat on
Moskito and delivers it to Izwalito for the decoder and replacement credit;
both transfers and the earned checkpoint are observed. BBB's fresh-game route reaches Daddy's Templand settlement,
Honk's six-item handover, and the writing gift that raises Daddy's evolution to
260. Its verified saved continuations earn the Internet optics/credit rewards,
create and settle Super Zen on Crazyland, and create Marakas on Spiraland.
The corrected Spiralus route buys Marakas's food, observes both item clips,
and saves with food aboard. His second visit includes the authored illustrative
transfers, then the earned optics gift raises evolution to 90 and enables
Izwalito's settlement on Vulcland. The next two Izwalito visits reach the
help/phone-number dialogue, then the third visit completes the phone exchange
and returns normally to the bridge. The treaty purchase is still gated by
population at the moment of that conversation; it is not granted by the phone
choice alone.
The following Internet calls pass the authored busy-network and fourth-call
selector before re-earning the optics. A normal Tempest visit buys Papy's
medicine and gives him those optics; `venthig.hnm` plays and the returned
world state contains Otto Von Smile's new Foxx colony.
His first four visits then play the medicine, joke, and credit scenes. The
fifth-visit `doom` alternative reaches the explosion ending assignment at
`0x9EF3` and exits cleanly. A separate perfume attempt transfers the item but
is interrupted by a war dialogue before its quest flags are set; that attempt
is not a completed perfume quest.
The fifth-visit guitar alternative completes the explosion showcase and the
gift feedback, raising Smile's evolution to 150 and population to 362. It does
not clear the observed war flag. Giving perfume on the next visit still ends
with the war interruption, so neither perfume attempt is a successful quest
checkpoint. These are native observations, not a claim that the interruption
has been matched against the original executable.
At the following Izwalito visit, population has fallen below the treaty-offer
threshold. Giving the earned energy and medicine raises energy to 1000,
population to 387, and evolution to 120. The offer then appears; accepting it
buys the treaty, and the ordinary Cancel row permits a completed save with the
treaty aboard (`bbb-izwalito-treaty-v4`).
Delivering that treaty to Smile raises evolution to 200 and population to 562,
and sets aggressiveness to zero, but does not clear the existing war flag
(`bbb-smile-treaty-v2`). Perfume remains interrupted on that branch.
Ordinary bridge waiting after loading advances other colonies, but the guitar
has disabled Smile's attack pass. The decompiled source's legacy simulation
mnemonics are reversed: `population_growth` encodes D4, which the original-code
oracle and typed runtime identify as conflict; `population_conflict` encodes D6,
the growth handler. Read the decoded operations, not these mnemonic names, when
reasoning about B54/B55. No source-byte changes or invented peace flags are used
to bypass this state. A pre-guitar wait witnesses real conflict completion and
possible immediate reacquisition, so a later conversation still needs its own
endpoint validation.
The successful continuation instead waits briefly from the fourth-visit save,
then gives the guitar while no conflict is active
(`bbb-smile-peaceful-guitar-v2`). Its endpoint has evolution 150, population 558,
and flags 21 (war bit 8 clear). The subsequent perfume visit finishes all gift
dialogue, leaves population 961 and aggressiveness 100, and makes Bug Deluxe
known (`bbb-smile-perfume-peaceful-v1`). Both conversations close normally and
have witnessed saves. This progression uses ordinary waiting and choices; it
does not alter simulation flags directly.
The resulting two Bug calls establish the rescue request
(`bbb-bug-distress-v1`). Daddy's ordinary `more`/`lots` quiz supplies explosives
(`bbb-daddy-explosives-v1`); choosing `throw` on Bug's next call transfers them
to him and brings him aboard (`bbb-bug-explosives-v1`). The Ark-exterior clips
from `20larvarc` are decoded during this continuation, not merely assigned.
His cryobox conversation then transfers technology, brings Scruter Jo aboard,
plays `pubgren1.hnm`, and sends Bug back to Trashlando
(`bbb-bug-technology-v1`). Delivering that technology to Smile on Foxxland
plays `lentille.hnm`, leaves optics aboard, raises his evolution to 250, and
unlocks the next destination group (`bbb-smile-technology-v1`). Each of these
continuations has a witnessed normal save; none is the game's ending.
Supplying earned energy and medicine on the next Smile visit, then leaving
through the actual Cancel row, completes the gift loop and enables descendant
settlement (`bbb-smile-support-v5`). Gluk subsequently occupies Cyberland.
His first normal visit there completes the ship-price and weapons-demand
conversation (`bbb-gluk-first-v2`); both continuations have new witnessed saves.
A third Marakas visit includes his nightmare dialogue and buys replacement
food, then exits through Cancel and saves with food aboard
(`bbb-marakas-food-again-v2`). Daddy's ensuing weapons trade is still being
reconstructed; the food purchase alone does not prove that trade. His fourth
visit uses the food in its cooking demonstration and returns it to Daddy before
the gift menu. The normal continuation therefore answers the recipe and
weapons questions, leaves through Cancel, refuses the first loan, and accepts
the second (`bbb-daddy-loans-v1`). The conversation closes and saves with
`A4 = 0`, `A17 = 3`, and `A43 = 1`. It has not acquired replacement weapons.
The fifth visit answers `good`, plays the exercise clip, and leaves the gift
menu to receive the scripted repayment (`bbb-daddy-repayment-v1`). Its new
checkpoint has ten credits (`A4 = 10`) and `A17 = 6`.
Buying food on Marakas's fourth visit and giving it to Daddy on his sixth
successfully obtains replacement weapons (`bbb-daddy-food-weapons-v3`). Both
visits include their scripted nightmare scenes. Leaving Daddy's gift menu also
triggers another repayment after the food purchase lowered the balance below
ten; the witnessed endpoint has weapons aboard and nineteen credits.
Gluk's next Cyberland visit accepts the Gluxx explanation and escalating ship
price (`bbb-gluk-buy-ship-v1`). The ship really transfers aboard and its object
clip decodes. The normal checkpoint has `A4 = 0`, `A27 = 2`, and `A58 = 2`;
the weapons remain aboard. This continuation records 37 fully revealed sites
with active actor attribution, rather than counting stale navigation captions.
One subsequent Daddy contact and the four-item inventory's Cancel row complete
another repayment (`bbb-daddy-post-ship-v2`). The saved endpoint retains the
ship and ten credits, with `A17 = 6`; no extra loan dialogue is required.
The first Mamy visit buys perfume at Loviland (`bbb-mamy-perfume-v1`), decodes
its item clip, and saves with both perfume and ship aboard. On Super Zen's
third visit, the route answers both word-game questions, gives those two items,
and leaves through Cancel (`bbb-zen-perfume-ship-v2`). Both holders are now
Super Zen, his evolution is 250, and Kero Zen actually settles on Malusland.
The new colony has population 14 and active flags 5 at the saved endpoint;
this is an observed simulation result, not a manually enabled descendant.
Kero's first Malusland contact then takes `copied` and the extended `yes`
explanation, presents the authored film/advertising montage, and returns to the
bridge with a new save (`bbb-kero-first-v1`). A second Papy visit buys medicine
and plays the concert-research scenes (`bbb-papy-medicine-again-v1`); medicine
and the earlier optics are both aboard at its witnessed endpoint.
Giving those optics and medicine to Kero on his second visit completes the
cloning requirement and population support (`bbb-kero-cloning-v1`). The save
has `A54 = 1`, both gifts held by Kero, and Ben Zen newly settled on Tumland
with active flags 5 and population 14. The next route visits Ben's illness
scene, then buys another dose from Papy on his third visit
(`bbb-ben-request-medicine-v1`). It saves with medicine aboard, seven credits,
`A54 = 2`, and `A38 = 0`; the treatment itself is not yet counted as complete.
The following Ben visit gives the medicine, selects `shocked`, and accepts the
guitar purchase (`bbb-ben-treatment-guitar-v1`). The witnessed save has medicine
held by Ben, guitar aboard, six credits, `A38 = 1`, and `A54 = 4`; both treatment
and purchase finish through the actual gift-menu Cancel row.
Revisiting Gluk after giving the ship to Super Zen presents his addiction
dialogue and queues the bridge call (`bbb-gluk-followup-v1`). Answering that
call completes it and saves with `A27 = 4`, `A28 = 0`, and `A29 = 0`.
The next observed visit unexpectedly reopens the ship-price menu instead of
the source's post-call dialogue. The first `yes` attempt therefore fails with
no checkpoint. Selecting the rendered `no_way` option and then `leave_him`
reaches the ten-bionium request (`bbb-gluk-bionium-request-v2`), closes normally,
and saves with `A27 = 2`, `A28 = 2`, `A29 = 1`, and zero bionium. The menu
return was investigated rather than hidden by a quest-state edit or counted as
the missing post-call scene.
The instrumented repeat (`bbb-gluk-bionium-request-v3`) confirms that the normal
load restores `A27 = 4`, then the first Gluk-contact frame changes it to `1`.
The same visible refusal route still saves with `A29 = 1`. The executable
change is explicit in the lineage; it adds read-only global tracing and includes
the previously verified BC selector repair.
The original-executable probe `big_bug_bang_gluk_skip_oracle.py` now establishes
the cause: the rejected text at COD 15709 skips two assignments and A0, but the
original token walker still enters query mode while skipping A0. Its following
C0 compares `A27` instead of assigning it. The bounded probe uses unchanged DOS
code and disc scripts with the earned save's VAR, a controlled root guard target,
and an active earlier menu. It preserves `A27 = 4`; it is not a full DOS UI run.
The native COD skip path now retains A0/A1 mode changes without executing their
guard-stack operations. The 17 original CB token-walker cases were also rerun;
the new both-dialect regression failed before the repair. All 1,115 enabled
game-library tests pass, with 78 explicit ignores.
Replaying from `bbb-gluk-followup-v1` with that repair yields
`bbb-gluk-bionium-request-v4`: the treatment montage, `yes`, and `leave_him`
finish normally with `A27 = 5`, `A28 = 2`, `A29 = 1`, zero bionium, six credits,
and the food/medicine/energy items aboard. This replaces v2/v3 as route evidence;
those older traces remain diagnostic artifacts and are not continuation parents.
The subsequent ordinary Jo cryobox entry and planned flight replay
(`bbb-jo-bionium-v3`) completes 60,563 AMER input/presentation frames and 97 sound
callbacks. Jo's success dialogue actually appears; the native save has
`vbio = 10`, `A12 = 1`, and bionium aboard. The minigame returns and releases its
resources normally. All 60,562 planned flight actions remain in the compact
31 MiB action trace, with their original completion clocks. This is an earned
native checkpoint, not an assigned score or injected inventory item.
Gluk's next visit gives him those ten bionium, answers `no` to restarting the
argument, and chooses `take_offer` (`bbb-gluk-bionium-delivery-v1`). The game
plays both `flitutr.hnm` and `explbleu.hnm`, consumes the ten bionium, and saves
with `A29 = 2`, `A31 = 1`, and `B11 = 1`. Migrator and Mig Burner are now
actually located at Loneland; their later quests remain to be played.
Another ordinary Jo visit (`bbb-jo-resupply-v1`) earns ten fresh bionium after
Gluk consumed the first supply. Migrator's first Lone visit then gives one unit
and answers `yes` to Honk's conscience question (`bbb-migrator-bionium-v1`).
The illness montage plays, and the witnessed save has `A34 = 1`, `A35 = 1`,
nine bionium, Migrator's population at five, and medicine still aboard. The
earlier inventory-placement-only anthology chapters are not substitutes for
this ordinary-playthrough evidence.
Migrator's treatment continuation (`bbb-migrator-treatment-v2`) transfers the
medicine, chooses `ship`, and closes the remaining GIVE menu. The actual reward
is Emasculator's address, not a ship: `A35 = 3`, `A37 = 1`, and `B5 = 1`, with
both medicine and ship held by Migrator. Emasculator and Rotator are at
Attroxcity. The failed v1 omitted the final Cancel click and is not a parent.
The first Attrox visit (`bbb-emasculator-first-v1`) introduces Emasculator,
chooses `name` and `male`, and witnesses the ten-credit offer with only six
credits available. The conversation closes through the eight-item GIVE menu;
no atomique is acquired or credited at this checkpoint.
Daddy's eighth ordinary visit and GIVE-menu exit
(`bbb-daddy-nuclear-budget-v1`) trigger the existing loan repayment. The saved
balance rises from six to sixteen credits, with `A17 = 6`; this is money granted
by the running script, not a prepared purchase balance.
The second Emasculator visit (`bbb-emasculator-nuclear-v1`) answers `no` to
his joke and accepts the purchase. The save has the nuclear item aboard,
six credits remaining, and Emasculator's encounter count at two. Migrator's
ship is still with him at this checkpoint.
Migrator's next visit (`bbb-migrator-nuclear-ship-v1`) chooses `no_thanks`,
gives the purchased nuclear item, and gives the guitar. The ship transfers
aboard and the save has `A45 = 2`, `B50 = 1`, `B52 = 1`, and `B96 = 1`.
Migrator has population 352 and evolution 340. Mig Burner remains at Loneland;
the settlement flag is not counted as proof of a new colony.
Emasculator's third and fourth visits (`bbb-emasculator-settlement-v2`)
give writing, energy, and two separately earned bionium units. The fourth
visit also earns optics. The saved endpoint has seven bionium remaining,
Emasculator at population 320 and evolution 350, and `B35 = 1`. Rotator still
shares Attroxcity and has not acquired the settlement participation flag;
the fragment name is not evidence that a new colony exists.
Answering Bug's pending call with `I_do` (`bbb-bug-zen-message-v1`) brings
him aboard and sets `A53 = 1`. Once this call finishes, ordinary simulation
does settle Rotator on Kultland and Mig Burner on Mastaland, both with active
participation flags 5. The cryobox clicks in that fragment have no effect
because the call rotated the bridge to the second band; his aboard explanation
is not counted yet.
The following continuation (`bbb-zen-first-telepathy-v1`) explicitly returns
to the console and selects Bug, the second of eight cryobox records. His full
future/Plato explanation plays, and he leaves for Trashlando. Super Zen's
fourth Crazyland visit then accepts the word game and completes the telepathic
montage. The saved endpoint has `A53 = 2`, `A60 = 0`, seven bionium, and
Marakas at Trashlando, enabling the disappearance investigation.
The next three Izwalito contacts choose `yes`, then `lie`, and finish the
investigation dialogue (`bbb-izwalito-missing-marakas-v1`). Answering Super
Zen's resulting call advances `A60` to four while `A53` remains two. The normal
save retains six credits and seven bionium; technology has not yet been bought.
The next local contact (`bbb-izwalito-technology-v1`) completes Izwalito's
technology sale. The witnessed save has the technology aboard, `A60 = 5`,
five credits, and seven bionium. No gift-menu exit or extra choice is required
on this particular quest conversation.
The next Super Zen visit (`bbb-zen-second-telepathy-v2`) chooses `no` in his
word game and completes both technology-assisted visions. The save has
`A53 = 4`, `A60 = 5`, and two bionium: the authored five-unit cost is observed.
The first attempt appended a Honk contact that closed before its expected
dialogue; that failed attempt is not used as a predecessor.
Earlier prefixes still require final-build replay before a complete flow claim.
Two earned-save alternatives, declining the help and denying the later call,
reach the observed Bob game-over assignment at `0x9F14` and exit cleanly.
CB's continued Izwalito route obtains Rondo's coordinates through the secret
conversation, visits Hom via Hita, and reveals Kortex. A normal Pterra visit
with the accepted code then recruits Scruter Jo aboard. Bronko's next three
visits recruit him aboard as well and place Emasculator at the factory. The
separate refusal leaves Bronko there and presents four distinct dialogue sites.
The Rondo route follows Yoko's Slimer discussion to the Ekatomb coordinates
and Maxxon's first telescope conversation. Daddy Gluxx's subsequent treatment
discussion reveals Erazor.
Otto's transplant/Ekatomb route earns the lens, and the normal observatory
selection delivers it to Maxxon. His youth-treatment and surgery alternatives
leave the lens with Otto and add ten distinct fully shown dialogue sites.
The following Magnus visit completes Morning Oil's battery request with one
goodbye selection. Answering queued calls then plays Scruter K's final warning,
Scruter Mac's first coded message, and Bug Deluxe's full Venusia commercial.
The commercial enables the supermarket destination through normal game logic.
The normal battery purchase also observes Venusia's replacement-credit offer;
the completed endpoint has both batteries and credit aboard
(`cb-venusia-batteries-v4`). Returning to Magnus delivers the batteries to
Morning Oil and recruits him aboard (`cb-morning-recruit-v1`).
On the next Venusia visit the previous purchase's goodbye completes first.
Recontacting the shop then buys Motoroil perfume with the replacement credit;
`OB\\parf1_2.hnm` plays, perfume is aboard, and credit is held by Bug Deluxe
(`cb-venusia-perfume-v3`). The shop closes itself after the department selection.
Morning's first cryobox conversation completes the authorized inspection and
closes normally (`cb-morning-wake-v1`). The next sleep-menu attempts do not
complete the repair and have no new checkpoint. The original executable loads
that exact earned save through its normal UI and reproduces the sleep menu
(`cb-morning-sleep-original-v7` and `v8`). Its timer 6 remains at 10 while
`GS:675A` retains the pending Scruter K record `0x06C2`; this is not evidence
that the native countdown is wrong. The phone-first continuation answers the
call, lets the timer expire, and selects `sleep` to resume the conversation.
The original run (`cb-morning-sleep-original-v10`) observes the pending pointer
clear, timer 6 reach zero and then become disabled, and the bridge return.
The native run (`cb-morning-sleep-after-call-v2`) reveals the garbage-recovery
dialogue, closes the conversation, and saves normally. No timer or quest-state
patch was needed. The recorder now retains the pending call and actual drawn
choice-row positions, including Cancel, to distinguish these states explicitly.
The next cryobox visits assign the recovered memories to TV two and return
Morning to the Ark (`cb-morning-memories-v3`). Opening TV and choosing channel
two actually decodes all twelve `match01` through `match12` clips. The panel
then closes normally and the route saves with Morning located at Ark.
Selecting Ark from the local Venusia menu and asking about Mastachok reveals
its chart marker and completes Morning's follow-up in the same contact
(`cb-morning-mastachok-v3`). The saved state has `B1 = 8` and `C1 = 1`.
There is no additional goodbye choice on this witnessed path.
The ensuing Mastachok visit first runs the guard's password scene even with
perfume aboard. Choosing `djerk` completes the rejection and explosion, after
which a normal recontact offers `teleport` (`cb-mastachok-perfume-v2`). That
conversation transfers the perfume to Scruter Mac and closes with `C1 = 6`.
The witnessed save retains that result, and the bridge queues Scruter K's
customs call. This route changes no inventory or quest state outside game input.
Answering that call completes the customs dialogue and the normal SCRIPT3
transition (`cb-scrut-customs-v1`). The new checkpoint is genuinely in SCRIPT3;
Bronko and Scruter Jo are aboard, Morning is on the Ark, and the next quest
stage has not been substituted by a prepared chapter state.
The following cryobox contact completes Bronko's airport lead and first
aboard conversation. Travelling to Ekatomb then presents the Gluxx children's
kidnapping report and returns to the bridge (`cb-bronko-gluxx-news-v1`), with
another normal save. This is the start of SCRIPT3's investigations, not its end.
The next Mastachok contact enters the prison through the guard's normal `code`
dialogue, follows the Eviscerator's war/treasure/secret topics, and earns the
SPLATCH request and Eden directions (`cb-eviscerator-eden-v4`). The conversation
closes and saves with `D1 = 1`, `secret = 0`, and `secret1 = 0`.

That conversation exposed a native dispatcher integration defect: BC topic
assignments updated the actor field but left the live selector on the preceding
topic. The direct-record handler already recorded the publication; the
dispatcher now applies it to the selector while preserving the parent control.
The unmodified original code's 17 direct-record and 14 selector-control vectors
were rerun successfully. They establish BC's write to `GS:6782` and its
precedence over the actor field. The new regression first reproduced the stale
selector, then passed for both dialects, including query and non-publication
controls. All 1,113 enabled game-library tests pass, with 78 explicit ignores.
The successful prison continuation records the rebuilt executable and repair
reason, and loads the same unmodified earned predecessor save.
The following Eden visit enters Purple Haze through Amigo, answers Tina's
first question with `no`, and returns to the bridge automatically
(`cb-tina-first-v2`). Its checkpoint has `E1 = 1` and a pending Kran Dobu call
after timer 10 expires. The earlier phone click preceded that call, so this
fragment does not claim to have answered it or begun the race.
The next continuation answers that pending call and visits Kraner's newly
revealed chart marker (`cb-kran-race-v1`). Kran presents the guitar wager,
reveals Troma, starts the race with `krando20.hnm`, and closes normally. Its
witnessed checkpoint retains the guitar with Kran; winning or repairing the
ship has not yet been claimed.
The route then travels to Troma, answers Kran's SOS, and returns to his newly
revealed position (`cb-kran-distress-v1`). His breakdown dialogue finishes and
the save has `panne = 1`, Morning on Ark, and the guitar still with Kran.
Selecting Ark from the local menu and teleporting Morning completes the repair;
recontacting Kran then teleports the guitar aboard (`cb-kran-repair-v1`). The
saved endpoint has `panne = 3`, Morning aboard, and guitar aboard. Morning's
separate transmitter/receiver handover has not yet been played.
Returning to Eden and selecting `teleport` gives Tina that guitar
(`cb-tina-guitar-v1`). The conversation closes with the guitar held by Tina,
who remains at the bar; her later recruitment is a separate step.
The subsequent Moskito airport visit introduces `commander_blood` to Migrator
and hears his singer request (`cb-migrator-first-v1`). He returns to rehearsing
and the conversation closes with a witnessed save.
The following Eden return recruits Tina with `TELEPORT`, then selects her fifth
row in the six-entry cryobox roster twice (`cb-tina-recruit-v2`). Both aboard
conversations finish, including her request to be dropped near the musician;
the checkpoint retains Tina aboard. The failed v1 clicked Jo's row instead and
is not used as a predecessor.
The corrected airport continuation chooses lowercase `teleport`, hears Tina's
arrival, and revisits Migrator for the rehearsal dialogue
(`cb-migrator-tina-v2`). Both actors end at Trashlando. A subsequent normal
cryobox contact with Morning (`cb-morning-keyrings-v1`) receives the transmitter
and receiver aboard and saves with `B1 = 1` and `panne = 3`.
Amigo's Eden visit follows the customer/password and chemistry topics through
four `splatch` selections, then `teleport` and `bye_bye`
(`cb-amigo-splatch-v3`). It saves with Splatch aboard and Amigo at Trashlando.
The earlier attempts either stopped at the last Splatch reply or omitted the
explicit goodbye; neither is a continuation parent.
Returning through the prison guard and teleporting Splatch to Eviscerator
(`cb-eviscerator-splatch-v1`) earns Tumul's coordinates and saves with `D1 = 2`.
Revisiting the guard then visiting Eden's bar (`cb-eviscerator-aftermath-v1`)
shows the escape aftermath and actually decodes `explo3.hnm`. The final save
has `D1 = 4`, Eviscerator and Scruter Mac at Trashlando, and Scruter K at Magnus.
The Scruter body is not yet aboard at this checkpoint.
The Magnus continuation enters the planet twice, first teleporting Anna Haf
and then recovering Scruter Mac's body (`cb-scruter-recovery-v2`). Both finish
normally and the save retains Anna and the body aboard. The second local menu
row is Ark, not a second character: the failed v1 selected that row and is not
used as a predecessor.
At Erazor, the fourth cryobox row offers Bronko's mission. Accepting `YES`
(`cb-bronko-erazor-v1`) saves with Bronko on Erazor and `brk = 1`. The surface
clicks later in that fragment did not start a contact: the destination sprite
was inactive after the cryobox conversation. They are not counted as a visit.
The next normal-load continuation (`cb-bronko-transmitter-v1`) enters Erazor,
hears Bronko's local report, answers the pending Cyberion call, then calls
Bronko from the phone list and gives him the transmitter. The save retains
the transmitter with Bronko and `trak1 = 1`; `fish` and `vari` remain zero.
The restored destination sprite is active after loading. This still needs a
continuous final-build replay and original-runtime comparison before treating
the earlier inactive-sprite behavior as normal gameplay.
Returning to Erazor, teleporting Bronko aboard, then contacting Bronko and the
receiver in the cryobox (`cb-bronko-return-v1`) completes the recovery. Its
save has `fish = 1`, `vari = 1`, Bronko aboard, and a new supply of Splatch
aboard. The receiver conversation is observed, not inferred from the timer.
The Tumul expedition (`cb-tumul-tomb-v2`) gives Scruter Mac's body to
Beauregard, follows the Patagos/Betakam/Gladis topics, gives Splatch, and
teleports the mummy and cursed Beauregard aboard. Its save has `beau = 6`,
`maledict = 1`, and both aboard; `fion` remains zero until his aboard contact.
The final Gladis reply needs one more topic click to leave the list. The failed
v1 omitted that click and never reached the expedition.
Beauregard's normal cryobox contact plays the curse sequence and sets
`fion = 1`. The same continuation (`cb-beauregard-rondo-v1`) sends Morning
to protect Yoko, revisits Rondo, and hears his telescope discovery. Morning
remains at the observatory and Jerry has not arrived yet; the pending Cyberion
call must be answered before proceeding with the investigation.
Answering Cyberion and Morning, visiting Hom with both fingerprint clicks,
then returning to Rondo (`cb-hom-jerry-v1`) completes Jerry's investigation
scene. The save has `G1 = 1`, `jerry = 1`, Jerry at the Shark, and Yoko and
Morning at Trashlando. Cyberock's examination and Hom's reward still follow.
The normal Cyberock exam (`cb-cyberquizz-dork-v1`) answers the first five
questions correctly and teleports the diploma aboard. Its save retains the
DORK item aboard and the five-answer score; later exam questions and failed
exam branches remain separate coverage work.
Returning to Hom with the diploma (`cb-hom-oddland-v1`) earns the scrambler
and completes Jerry's black-hole call, followed by Cyberion's third message.
The save has `G1 = 2`, the scrambler aboard, and the Ark's location at Oddland.
However, the observed profiles remain SCRIPT1/2/3: arrival has not triggered
SCRIPT4, and the camera reports an unsupported black-hole target. This is a
partial route checkpoint, not a verified chapter crossing.
The subsequent ordinary crossing (`cb-oddland-crossing-v2`) opens the bridge
camera, then selects the left black-hole entry control. It loads SCRIPT4 and
saves the authored new-chapter state: Fifi on Ron, Maziok on Magnu, Hom in the
cache, and the curse active. The first attempt omitted camera activation and
its inactive entry control did nothing. No runtime or quest-state change was
needed to cross.
The first Ron/Magnu route (`cb-fifi-maziok-v4`) declines Fifi's scrambler
request, then greets Maziok twice and follows his curse/medicine/sorcerer
topics. The full Crazystone coordinates are presented, `zen = 1`, and the
conversation closes to a witnessed save. The curse is still active. Earlier
attempts switched away from the temporary sorcerer topic or selected it once
too often; those failed attempts are not predecessor checkpoints.
Answering Jerry's call and visiting the Shark
(`cb-jerry-zen-painting-clue-v1`) presents the Vistar lead. The subsequent
Crazystone visit chooses `hope` and completes Super Zen's portrait request,
revealing Vista. The save has Super Zen at Trashlando, awaiting the painting;
the curse remains active and the portrait has not yet been obtained.

An earlier continuation exposed a separate native text defect: dictionary offset 1
is the word `talk` in CB SCRIPT2, but the shared subtitle assembler interpreted
it as BBB's live-number marker. The assembler now distinguishes the dialects.
`re/tools/commander_subtitle_oracle.py` executes the unmodified CB assembler
on the exact `0x6FF6` Morning line and three offset-one edge cases; all four
match the Rust regression. The existing 45 BBB signed-number/cursor cases
remain covered. All 1,112 enabled game-library tests pass (78 explicitly
ignored asset-dependent tests). The successful continuation records the new
binary hash and the repair reason while retaining its exact predecessor save.

That original comparison first exposed a DOS harness defect: INT 21h/AH=0Eh
returned a drive count without changing the current drive. The harness now
honors selection of its mounted drives, with relative-path and invalid-drive
regressions. All 41 recompiler tests pass, including 15,049 interpreter oracle
vectors and 3,465 instruction differential checks. The repaired harness really
opens the supplied `game1.sav`; the earlier probes that never loaded it are not
used as gameplay evidence. This fix does not change the native game binaries.
CB's separate earned-save alternatives include Izwalito's secret refusal and
Scruter Jo's rejected code, with `explo3.hnm` played and Jo still on Pterra.
These are route endpoints, not whole-game completion.
The detailed witnesses live under `output/game-flows`.

An earlier runtime limitation was exposed while buying Izwalito's treaty:
F7 from the GIVE menu fails with `SubtitleRevealError::EmptyText` before saving
(`bbb-izwalito-treaty-v3`). That run is rejected as a checkpoint. The route uses
the visible Cancel control instead. The shared empty-caption failure is now
repaired as described above; this exact historical F7 interaction has not been
replayed and is not claimed as verified.

### Source Progression Gates

These are source constraints for reconstructing the routes, not claims that the
routes have already been executed:

- CB SCRIPT1 can enter SCRIPT2 through the tutorial's `game` choice or Bob's
  mission briefing. The latter contains the Ark sequences absent from a route
  that chooses `game` immediately. Sequence requests still have authored skip
  conditions; their presence alone does not prove playback.
- CB SCRIPT2's normal exit requires Kortex known, `C1 == 6`, the lens held by
  Maxxon, and Scruter Jo and Bronko aboard. The resulting Scruter K call enters
  SCRIPT3. The separate `CHEAT MODE` selector is excluded.
- CB SCRIPT3 requires `fish == 1`, `fion == 1`, the scrambler held by Blood,
  Tina Burner and Amigo at Trashlando, and `jerry == 1`. Jerry's call enables the
  Oddland crossing that enters SCRIPT4.
- CB SCRIPT4 requires Betakam at Sat and Maxxon, Yoko, and Ondoyant aboard.
  Jerry's call enables the return through Oddland into SCRIPT5.
- CB SCRIPT5's concert follows the Bigbang bar conversations and ring transfer
  to Migrator. `finalmen` requires the Bigbang/concert/Migrator state and requests
  the concert clips followed by `fin.hnm`; decoding a loose ending asset is not
  evidence that these predecessors occurred.
- BBB SCRIPT1 introduces SCRIPT2. SCRIPT2 is the world/ship dispatcher; SCRIPT3
  through SCRIPT17 are actor modules, not successive chronological acts.
  Their engine-mediated return to the world must be included in the flow.
- BBB SCRIPT2 distinguishes the `A13`-controlled game-over procedures from
  `fin1`, whose source guard is Scruter Mac evolution above 900. Setting that
  value directly is not an acceptable route. Despite its procedure name,
  `fin1` selects the same `28bob` dismissal sequence as the `A13=2` failure.
  The congratulatory `48finbob` sequence instead requires `A13>25`; its
  post-concert route is the successful-ending target.

The CB tutorial exposed a production-runtime mismatch: Honk stopped after his
first line and opened the BAS `adieu` menu, whereas the original executable
continued the tutorial automatically. The native A6 dispatcher now preserves
CB's outer-loop handoff-lock write at `0x565B` (`GS:67B7`), independently of
BBB's additional VM-disable write. `re/tools/commander_vm_yield_oracle.py`
executes the unmodified original A6 and outer loop for 24 accepted/rejected
subtitle/menu cases; the corresponding Rust regression checks the same
pre-presentation-scan boundary. This repair does not establish whole-game parity.

The normal Izwalito credit transfer exposed two more omitted host operations.
Original CD at `BLOODPRG 0x6A8A` loads the item's DESCRIPT before requesting line
43; native COD and BAS now apply that descriptor after the original transfer
gates. Original scene completion at `0x9F0B` clears secondary request bit 2;
the ship-scene adapter now exchanges that shared flag with the lifecycle, as
the contact-transition adapter already did. The CD gate regressions, existing
original-executable vectors, and normal replay are complementary evidence.
`cb-izwalito-credit-fixed-v2` observes the clip, resumed character dialogue,
Moskito chart marker, and an earned save after the conversation closes.

The authoritative guards and side effects remain in `re/vm/profiles` and
`re/vm/big-bug-bang-profiles`. Neither a profile spine nor the static dialogue
catalog below proves that all normal-playthrough prerequisites are satisfied.

## Capture Storage

New native dialogue/sequence captures compact only after their full master
verification succeeds. The retained files are `master.mkv`, the report and
plan/provenance, a losslessly compressed `native-state.jsonl.gz`, and a
`storage.json` receipt. Redundant `video.mkv` and `audio.f32le` are removed only
after checking the master against the saved verification hash and checking the
trace's gzip round trip. `--keep-intermediates` opts out during debugging.
Coverage and assembly verification accept either raw or compressed traces.

Final assembly streams concatenated decoded PCM directly into the muxer instead
of first creating a whole-anthology raw PCM file. The integration test verifies
the decoded result, including mixed 46/68 ms frame spans and a one-frame chapter.

Existing captures are untouched unless compaction is explicitly applied. Start
with the read-only audit:

```sh
uv run tools/native_capture_storage.py --root output/anthology \
  --out output/game-flows/storage-audit.json
```

`--apply` performs the verified compaction; do not run it alongside an active
capture. Masters, final videos, source assets, and failed captures are retained.
The initial audit found 751 candidate chapters, 40,993,221,022 bytes of duplicate
video/PCM and 41,979,899,252 bytes of raw traces. These are candidate input sizes,
not claimed reclaimed space or a promise about compression ratio. No existing
capture was deleted by that audit.

## Static Dialogue Trees

```sh
nix develop -c cargo build --release -p commander-blood-script-compiler \
  --example dialogue_catalog
uv run tools/dialogue_catalog.py --out output/anthology/static-dialogue-en
```

This compiles the readable BloodScript files with the existing compiler, then
uses the typed COD/BAS decoders and control-flow analyzers. It does not execute
the VM, launch either game, use a display, or change saves. Default inputs are
all five CB profiles in `re/vm/profiles` and all 17 BBB profiles in
`re/vm/big-bug-bang-profiles`. BBB uses the existing English display catalogs by
default, just as the runtime does. The recovered French script remains the
authoritative source for IDs and conditions; it is not rewritten or retranslated.
`--cb-source`, `--bbb-source`, and `--analyzer` allow explicit alternate inputs.
`--bbb-english` selects the catalog directory (default
`localization/big-bug-bang/en`). `--bbb-language source` explicitly disables the
English overlay for source-language analysis.

The analyzer hashes the actual compiled COD, DIC, and optional BAS images. The exporter requires
both hashes, the profile tag, and the exact set of text-site IDs to match the
English catalog. It also checks section and choice counts, ordered live-number
operands, and inventory generators. Mismatches fail the export, rather than
silently assigning English text to different code. All 6,921 BBB text sites have
English display entries; this is catalog coverage, not a new editorial review.

The output contains an `index.md`, a hashed `catalog.json`, and per-profile:

- `source.blood`: the exact analyzed source, including named conditions.
- `graph.json`: every COD instruction, block, conditional edge, text site,
  deferred profile request, BAS selector list, and local menu-to-selector link.
- `dialogue.md`: readable lines grouped by COD procedure or BAS object/selector.
- `translation.json`: the exact bound English catalog for each BBB profile.
- `cod.dot` and, for CB, `bas.dot`: Graphviz graphs. Frame-resume edges are dashed
  and remain distinct from immediate control transfers.

Markdown and COD text/choice labels use English display fields. `graph.json`
retains the original `text`, `spoken_operands`, `choice_operands`, instructions,
and edges unchanged, with an additional per-site `display` object. Original
guard descriptions are not rewritten using translated choice labels. Translation
hashes and the selected language are included in export provenance.

Text sites retain dictionary operands, choice labels, presentation selectors,
chatter flags, conditional skips, progress/history predicates, and resume targets.
State-number and inventory operands remain symbolic. Identical strings at
different instruction offsets are distinct sites. Text-record ownership is
preserved without assuming it always identifies the audible speaker.

BAS choice links follow the first matching selector in the same object's linked
list. Shadowed duplicate selectors remain visible, and a missing local match is
explicit, not labeled unreachable. Inline TEXT choice operands are retained
separately from BAS menu-header links. COD branches retain both conditional
outcomes; conditions are not solved into feasible complete playthroughs. Cycles
stay as graph edges rather than becoming infinite DFS paths. The existing CFG's
`reachable` field is static control-flow reachability, not save-state feasibility.

The current census is **5,536 CB text sites** (3,687 COD + 1,849 BAS) and
**6,921 BBB text sites**, with 321 CB BAS selectors and 1,396 BAS menu rows.
Tests compare those sites against every authored `say`/`text_tokens` statement.
These counts include UI and repeated/conditional text, not just unique spoken
sentences. Static discovery is not video coverage or proof of runtime fidelity.
The separate BBB `SCRIPT2.BAS` artifact is not owned by the 17 active profiles
and is not included in this profile census.

The exporter publishes the directory only after every profile succeeds, retains
source/analyzer/exporter/translation hashes, and refuses to overwrite existing output.

## Build and Catalog

Requires the repository's Nix development shell, `uv`, and imported asset stores.
PyAV is pinned in the script's inline dependency declaration; `uv` manages it.

```sh
nix develop -c cargo build --release -p commander-blood-game --bins
nix develop -c uv run tools/video_anthology.py catalog \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --out output/anthology/catalog-cb.json
nix develop -c uv run tools/video_anthology.py catalog \
  --assets output/big-bug-bang/imported-assets \
  --out output/anthology/catalog-bbb.json
```

The catalog verifies every source resource against the import manifest, then
uses the typed DESCRIPT parser to associate HNM files with authored records,
captions, and roles. Categories are conversation, planetary, environment, object,
sequence, and unclassified. An asset can have several roles. Directory names are
hints, not proof of what a clip contains. No DESCRIPT reference does **not** mean
unreachable: scripts can refer to media independently. Intro, TV, credits, endings,
and story-specific labels need further authored-script classification.

## Record

```sh
nix develop -c uv run tools/video_anthology.py record \
  --jobs accuracy/anthology-pilot.json \
  --cb-assets "$HOME/.local/share/commander-blood/assets-v1" \
  --bbb-assets output/big-bug-bang/imported-assets \
  --out output/anthology/pilot --mp4
```

The checked-in pilot contains Bob's first-contact/goodbye route in CB and HONK's
initial PLAY conversation in BBB. Both include their startup/navigation prelude;
they are not edited, conversation-only cuts. Use repeatable `--job ID` options to
select jobs. `--bin-dir` selects a different build.

By default each job uses a private Xvfb display and SDL's dummy output device.
The real mixer still runs, but it does not play through the user's speakers.
`--visible` instead uses the current desktop and audio device. Every attempt has
a fresh `--write-data` directory; existing saves are never reused or changed.

Each `attempt-*` directory contains:

- `master.mkv`: final 30 fps FFV1 video plus timestamp-aligned FLAC audio.
- `viewing.mp4`: optional H.264/AAC viewing copy, not an archival source.
- `transcript.json` and `transcript.srt`: subtitles and spoken inline words.
- `chapters.ffmeta`: the route title and exact frame-based duration.
- `capture/video.mkv`, `audio.f32le`, and `timeline.jsonl`: original captured
  frames, untouched float mixer samples, and their monotonic timestamps.
- `capture/audio-timestamped.mka`: the same source PCM packets with callback
  timestamps, used by FFmpeg's asynchronous resampler to prevent audio drift.
- `capture/scenes.jsonl`: changes in profile, HNM, text, choices, and music.
- `runtime-trace.jsonl`, `scenario.tsv`, `game.log`, and `status.json`: action
  boundaries, exact route input, diagnostics, verification, and provenance.

The recorder retains native idle animation, chatter, music, progressive text,
and script waits. Recording enables real-time frame pacing even for scenarios,
which normally run accelerated. Scripted PIT ticks remain calibrated to scenario
frames. There is no artificial dialogue freeze-frame padding. The video is
sampled at 30 fps, so native frames may be repeated. Audio is measured at SDL
submission, not at the physical speaker; device latency is not calibrated.
The aligned master resamples audio to this common clock; the raw PCM remains
available for independent analysis. Final audio is padded/trimmed to the video
duration. GPU readback/encoding can still affect runtime speed on a slow machine.

Transcript timing marks a complete line at its first visible word, not individual
character reveal timing. Choice menus are retained separately in scene events.
This is not yet a speaker-attributed or static exhaustive script transcript.

## Jobs and Resume

Job manifests have `schema: 1` and a `jobs` array. Each job requires `id`, `game`
(`cb` or `bbb`), `category`, `title`, and `scenario` relative to the manifest.
Optional fields are `packed_second` (default 39), `timeout_seconds` (default 900),
`expected_resources` (DOS resource names), `expected_subtitles` (substrings), and
`expected_final_profile` (zero-based). These assertions validate a route's
observed content; they do not establish exhaustive branching coverage.

Rerunning an unchanged command verifies and reuses completed attempts. Resume
requires matching hashes of the game binary, runner, asset manifest and scenario,
matching job/options, intact output hashes, and a successful full media decode.
All source assets are revalidated. Changed inputs or damaged outputs produce a
new attempt; failed attempts remain for diagnosis. A job is complete only after
the final scripted action, its content assertions, mux, and decode checks pass.
Timeouts terminate the job process group, including its encoder.

## Assemble

Pass completed attempt directories in chapter order. Assemble the two games
separately; incompatible stream formats or corrupted captures are rejected.

```sh
nix develop -c uv run tools/video_anthology.py assemble \
  --attempt output/anthology/pilot/cb-bob-first-contact/attempt-REPLACE \
  --out output/anthology/cb-conversations.mkv
```

Repeat `--attempt` for additional chapters. Assembly stream-copies video and
losslessly re-encodes FLAC audio (to regenerate its whole-stream checksum),
offsets chapter boundaries by their exact frame durations, and writes a sibling
`.manifest.json` with source hashes. Existing output files are never overwritten.
The assembler does not deduplicate shared intros or trim routes automatically.

## Remaining Coverage Work

Use the static graph as the authored inventory for anthology planning. No click
traversal is needed to discover dialogue. The next export step is to select
finite graph segments, resolve their media and symbolic state variants, and
render alternate branches with explicit chapters. It must keep loops, deferred
profile changes, inventory, story phase, and prior-visit predicates explicit;
the graph alone does not choose one valid state for every line.

The requested full videos must use the game's presentation behavior, including
scene palettes, original subtitle fonts and reveal/hold timing, animation,
music, voices, and effects. A separate approximate compositor or silent preview
is not an acceptable substitute. The static scanner does not yet automatically
turn every graph branch into a video. Both catalogs leave unrecorded content
unrecorded rather than equating script/file presence with video coverage.

### Source-Ordered CB Contact Sweep

`tools/native_cb_contact_plans.py` walks the hashed CB COD/BAS catalog and all
65 contact procedures in profile/offset order:

```sh
uv run tools/native_cb_contact_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --out output/anthology/cb-all-contact-candidates-v2
```

The resulting `planning.json` records COD text/choice offsets for every contact,
351 simple BAS-topic candidates, and 29 procedures without a unique BAS list.
These are **not** 351 playable or verified chapters. COD story dialogue can close
a presentation before its actor's BAS menu opens; state and branch prerequisites
must be derived and tested before selecting a candidate for native capture.
SCRIPT3 Bronko's direct-contact `intelligence` candidate and SCRIPT4 Fifi's
`planet` candidate both failed the native check with "dialogue ended before all
planned choices." A SCRIPT4 Super Tromp probe navigated to Vista's `tombeau`
and selected Sinox, not Super Tromp. The source's `st1` procedure checks planet
Vista, so a second probe used the planet itself as the travel destination.
That native route verified all three simple Super Tromp BAS topics (painting,
culture, yolk) as 104.046 seconds of new capture in
`cb-super-tromp-vista-dialogue-native.mkv`. The three-chapter source batch is
`dialogue-cb-super-tromp-orbit-all-v1`. CB's combined v2 movie appends that
SCRIPT4 batch after the earlier SCRIPT1/2 dialogue captures; the earlier
standalone sequence block is not a reconstructed continuous gameplay route.
`cb-verified-anthology-v2.mkv` passed full decoded RGBA, PCM, timestamp, and
chapter verification: **78 chapters, 3,824.298 seconds, and 58,572 frames**.
The recomputed ledger `dialogue-coverage-cb-super-tromp-v1.json` has 275 CB
sites fully revealed in the native UI buffer (up from 261), with 5,254 still
uncovered. BBB remains at 787 fully revealed and 6,091 uncovered.

SCRIPT4 `bra1` begins on planet Vistar with Bratakas already there. Four
source-derived BAS topics (Vistar, planet, race, croolis) passed native capture
and complete chapter verification in `dialogue-cb-bratakas-vistar-v1`. Their
standalone movie `cb-bratakas-vistar-dialogue-native.mkv` has four chapters and
runs 174.946 seconds. The batch leaves three actor-list BAS sites unplanned;
one is the generic `talk` response and two use `leisure`, which is not offered
in this menu. The combined `cb-verified-anthology-v3.mkv` places Bratakas's
earlier SCRIPT4 procedure before Super Tromp's and passed full media checks:
82 chapters, 3,999.244 seconds.

`native_sequence_anthology.py assemble --dialogue-source-order` keeps the
authored sequence block first, then stable-sorts verified dialogue by profile
and source procedure offset where supplied, falling back to the earliest
required COD site. It checks each chapter's source-plan report hash before
sorting and records the ordering in the output manifest. This is script source
order within the captured branch set, not an inferred continuous playthrough;
the sequence records and mutually exclusive dialogue alternatives have no
single universal gameplay order.

Two SCRIPT4 `sin1` travel plans at Vista's `tombeau` now capture Sinox's candle
choice both ways. The accept and refuse plans bind COD 12056's authored words
and require their own response while excluding the other. They passed native
trace and media verification in `dialogue-cb-sinox-candle-v1`; the new batch
belongs after Super Tromp's SCRIPT4 `st1` procedure in source order. Their
standalone `cb-sinox-candle-native.mkv` passed full media verification with
two chapters and a 117.940-second duration.
The source-ordered CB assembly `cb-source-ordered-anthology-v2.mkv` passed
full media verification with 84 chapters and a 4,117.184-second duration.

BBB SCRIPT7's Betakam first-contact accept path passed as a 33.474-second
chapter in `dialogue-bbb-betakam-first-v3` and
`bbb-betakam-first-native.mkv`. COD 2182 is VM-published but not present at a
frame boundary before closure, so the plan requires publication but not a UI
draw. SCRIPT15 Scruter Mac's first-contact probe stopped at numeric chatter
dictionary position 2908. That site and chapter remain unverified; neither a
fabricated numeric voice nor a silent replacement is included.
The source-ordered BBB assembly `bbb-source-ordered-anthology-v1.mkv` passed
full media verification with 317 chapters and a 15,323.570-second duration;
the Betakam chapter occurs once in its manifest.

### Travel Audio Correction

The source-ordered CB v2 and BBB v1 assemblies above have an **audio defect**:
their offline travel entries skipped the ship-HUD music selection and retained
the startup bridge `TABLO2.VOC` stream. Their decoded PCM hashes prove only that
the captured mix survived encoding, not that the selected soundtrack was right.
Do not treat those full assemblies as audio-correct. Travel capture now applies
the planet and destination DESCRIPT selections before entry, restarts changed
music as the ship HUD does, and reports the selected `travel_music`. Dialogue
trace verification rejects a travel chapter whose selected music is not the
active navigation stream.

Freshly captured and fully verified samples are `cb-travel-audio-corrected-v3.mkv`
(Bratakas on `ITE2.VOC`, Sinox on `UMTHA2.VOC`; two chapters, 109.248 seconds)
and `bbb-travel-audio-corrected-v3.mkv` (Rotator on `TROMA.VOC`; one chapter,
74.794 seconds). The corrected CB selected-branch master is
`cb-audio-corrected-anthology-v1.mkv`: all 60 travel chapters were recaptured,
their original plans matched exactly, and the 84-chapter, 4,117.184-second
assembly passed full decoded RGBA, PCM, timestamp, and chapter verification.
Its travel chapters select six DESCRIPT music resources rather than the bridge
`TABLO2.VOC`. The old CB v2 movie remains an audio-defective historical output.
The corrected BBB selected-branch master is
`bbb-audio-corrected-anthology-v1.mkv`: all 236 travel chapters were recaptured
with `TROMA.VOC` selected for Cyberock/Cyberland, their original plans matched
exactly, and the 317-chapter, 15,323.570-second assembly passed the same full
media and chapter checks. The old BBB v1 movie remains an audio-defective
historical output. Both corrected assemblies retain the original selected
source order, not a continuous gameplay branch order or exhaustive BAS/COD
coverage. The outputs and their manifests are under `output/anthology`.
A BBB Bug Deluxe probe with the current exporter stopped at numeric chatter
dictionary position 3944; it is not a corrected chapter or evidence that every
BBB branch can already be regenerated.

With Bratakas, Super Tromp, Sinox, and Betakam included, the recomputed
`dialogue-coverage-source-ordered-v1.json` ledger has **309 CB** and **794 BBB**
sites fully revealed in native UI buffers. It retains **5,220 CB** and **6,083
BBB** uncovered sites; publication without a raster remains separate. Script7
Alphakam and Gammakam first-contact probes stayed on their numeric presentation
line through 2,500 native frames and failed the bounded endpoint check. They are
not included in the ledger or any assembled movie.

The corrected movies reuse the same selected branch sets, so these dialogue
coverage counts are unchanged. CB's 60 travel plans cover seven
planet/destination pairs. All 236 travel plans in the selected BBB master target
Cyberock/Cyberland; its greater running time is not evidence of broader world
or branch coverage.

### Offline Presentation Backend

`ModernGameServices::new_offline` now constructs the production services without
an SDL window or audio device. It uses the same GPU composition passes and the
same audio callback mixer as live playback:

- `read_offline_rgba` reads the fully composited frame after presentation. It
  rejects an unpresented or newly resized target instead of returning a blank
  frame. Main-viewport reconfiguration retains the offline target and its size.
- `render_offline_audio` consumes an explicit number of mono `f32` samples at
  `RuntimeAudioHost::output_sample_rate_hz()` (48,000 Hz). Native stream page
  refills, sound selection, and timers remain the caller's responsibility.
  Pulling samples from a live SDL host is rejected to prevent two consumers
  from advancing the same playback state.

This is a shared output backend, not a full-game export driver. It does not
choose dialogue branches, fabricate missing scene state, or change the native
schedule. The offline driver must still connect the static graph to those
services and advance the recovered clocks and stream refills at the correct
boundaries. The nominal 25 fps in normalized WebM files is not an authoritative
playback clock. No new complete anthology is claimed by these backend tests.

Tests check exact original-font pixels and palette colors for both executables
through the final GPU passes, output resizing and row padding, audio sample
equivalence with the SDL callback across stream refills and sound effects,
and device-free service initialization with real assets from both games.
These establish the tested output-backend behavior, not DOS whole-game parity.

### Offline Native Sequence Export

```sh
nix develop -c cargo run --release -p commander-blood-game \
  --bin offline-presentation -- \
  "$HOME/.local/share/commander-blood/assets-v1" opening \
  output/anthology/cb-opening
```

Use `credits` for presentation line one, `cinematic` for the complete startup
DESCRIPT sequence list, and the BBB imported asset directory for the sequel.
An optional final argument sets the frame cap (default 100,000).
The command requires FFmpeg/FFprobe and a headless wgpu adapter, but no SDL
device, window, mouse clicks, or wall-clock playback waits. Line zero is the
opening logo reel, not the subsequent scripted cinematic.

Both live and offline paths call the same native blocking presentation runner
and main lifecycle. The offline driver advances the production PIT accumulator
over each 46 ms game or 68 ms presentation wait and consumes exactly the
corresponding 48 kHz mixer samples.
Sound advances during the wait before the next completed GPU frame is exposed,
preserving the live runner's ordering. Capture includes native frame boundaries,
not the desktop host's extra render-only interpolated refreshes. It does not use
normalized WebM's 25 fps.

Each new output contains `master.mkv` (lossless full-range RGB VP9 and unmodified
float PCM), the separate `video.mkv` and `audio.f32le`, per-interval hashes and
sample offsets in `timeline.jsonl`, verified video timestamps, the source asset
manifest, private runtime files, and a provenance/accounting report. The earlier
v1/v2 logo and credits captures used lossless FFV1; the timestamped RGB encoder
was verified against the same decoded pixel stream. The final
GPU flip has zero duration at the capture endpoint; `endpoint.rgba` retains it
without inventing a hold. No audio resampling, padding, or trimming is performed.
Muxing assigns packet durations from the native timestamps and final wait;
WebM SimpleBlocks otherwise let FFmpeg infer a frame duration that can extend a
46 ms final wait to 68 ms. Both chapter export and assembly preserve the exact
endpoint, with decoded regression checks for variable and single-frame timelines.

Every imported source hash is checked before and after export. Full decoding of
the saved master must reproduce every captured pixel and audio sample exactly;
every encoded video timestamp and duration must match the native wait schedule.
Publication requires natural scene completion. A cap, codec failure, or mismatch
leaves only staging files. Existing output directories are never reused.

Verified captures on 2026-09-20: CB logos 262 frames / 17.816 s; CB credits
879 frames / 59.772 s; BBB logos 292 frames / 19.856 s; BBB credits 1,440 frames /
97.920 s. Both logo reels are silent; both credits exports contain native mixer
audio. These are standalone captures from fresh service state with the Initial
scene link, not a reconstruction of a preceding playthrough. Full dialogue,
other cinematic, travel, and branch coverage remain outstanding. Matching this native
Rust path and lossless encoding do not alone establish DOS timing/sound parity.

The `cinematic` target initializes the actual game lifecycle, executes the logo
reel during bootstrap, then captures from the first main-loop frame through the
first naturally completed DESCRIPT sequence list. It neither clicks through
scenes nor substitutes another playlist. Shutdown releases resources without
appending unrelated credits. CB's `present` sequence contains `cliptoot.hnm` and
`BLINTR.VOC`; BBB's contains thirteen clips and `CROOLRAP.VOC`. The checked
captures are 2,545 frames / 172.378 s (CB) and 1,784 frames / 120.630 s (BBB).
Both contain original-font captions; BBB uses the bound English display catalog.
`native-state.jsonl` retains each main-loop state at its pre-wait boundary.
The report also retains the selected sequence record, authored media order,
source caption bytes, displayed captions, cue frames, and fixed clock/seed.
Those cue frames belong to the game's sequence clock, not the encoded frame
number. The full library and original control-flow vectors test the shared
stepped lifecycle; exports still require their own decoded-media checks.

For a standalone authored chapter, use `sequence:RECORD`, for example
`sequence:maledict` (CB) or `sequence:48finbob` (BBB). The exporter validates the
name against the imported DESCRIPT database and rejects missing or non-sequence
records. After the initial profile is loaded, and before the panel actor opens,
it replaces only the first sequence-name slot. The unmodified native panel then
plays that record's complete ordered clip list, captions, and music through the
same lifecycle and PIT/mixer schedule. The short native bridge/panel prelude is
retained. This is explicit chapter selection, not a gameplay route or proof that
the record is reachable in that profile. The report records this distinction and
the exact selection time. No saved progress, script branch, or mouse click is used.

The databases contain 11 CB and 54 BBB sequence records, including `present`.
These are the authored multi-clip records, not all standalone HNM files or
dialogue-triggered sequences. Exporting them does not complete the dialogue,
travel, object, environment, and alternate-branch parts of the anthologies.

### Sequence Batches

```sh
nix develop -c uv run tools/native_sequence_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --out output/anthology/native-sequences/cb
nix develop -c uv run tools/native_sequence_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --out output/anthology/native-sequences/bbb
nix develop -c uv run tools/native_sequence_anthology.py assemble \
  --batch output/anthology/native-sequences/cb \
  --out output/anthology/cb-sequences.mkv
```

The batch reads the actual database with `video-catalog`, renders every sequence
record, and writes `selection.json` and `coverage.json`. Resume requires identical
source/exporter/catalog provenance and re-verifies complete chapter outputs;
changed inputs require a new output directory. Failures remain explicit and do
not prevent attempts at the remaining records. Native-state verification checks
the ordered clip occurrences, including repeated filenames, and actual caption
cue draws. Missing authored files are recorded without substituting other media.
In particular, CB's `year` references absent `SQ/PUVEN1.HNM`; this is distinct from
the existing `SQ/PUBVEN1.HNM` and follows the native unavailable-source path.

Assembly keeps every variable-rate video frame and concatenates the untouched
float PCM, with exact chapter boundaries. It stream-copies RGB VP9, then decodes
the entire assembled video/audio against each source interval's hashes and checks
every frame timestamp and chapter boundary before publishing. No frame-rate
conversion or audio resampling is used. The sibling manifest retains source and
output hashes, missing-resource dispositions, and caption coverage.

The shared authored subtitle clock must survive HNM switches. Original CB
`screen_mode_update` resets DS:0x131C at 0x7B65 only when starting a new record;
the loader and consumer increment it at 0xA18B and 0xA3F0. BBB performs the same
operations on DS:0x156A at 0x8C0C, 0xB96E, and 0xBBDA. The production player now
retains that counter across per-file queue ownership. Earlier multi-clip exports
reset it incorrectly and are superseded: lossless encoding alone did not detect
the resulting delayed or omitted captions. Per-file queue cursors still restart.

### Verified Sequence Outputs

The `output/anthology/native-sequences-v2` batches completed all 11 CB and 54 BBB
records, with no processing failures. Their assembled outputs are:

| Game | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| CB | `cb-authored-sequences-v2.mkv` | 11 | 608.352 s | 9,057 |
| BBB | `bbb-authored-sequences-v2.mkv` | 54 | 3,056.152 s | 45,485 |

The coverage reports explicitly retain CB's missing `year` resource and its two
unshown captions. One further CB cue and 15 BBB cues (including blank cues) are
timed beyond their captured sequence counters; no extra hold frames are invented
to show them. BBB has no missing authored sequence resources. These are sequence
anthologies, not the requested complete-game videos; the other content categories
and alternate dialogue branches remain to be rendered and verified.

### Native Dialogue Branches

`dialogue:PLAN.json` runs a source-bound COD/BAS branch through the same device-free
main lifecycle. It selects DIC words semantically, without pointer input. Plans
are under `accuracy/anthology-dialogue/`; that directory's README documents the
native skipped/preempted lines, radio/contact entry, and profile-setup limits.

```sh
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan accuracy/anthology-dialogue/cb-izwalito-game.json \
  --plan accuracy/anthology-dialogue/cb-izwalito-explanations.json \
  --out output/anthology/dialogue/cb
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --plan accuracy/anthology-dialogue/bbb-honk-play.json \
  --plan accuracy/anthology-dialogue/bbb-honk-instructions.json \
  --out output/anthology/dialogue/bbb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue/cb \
  --out output/anthology/cb-intro-dialogue-native.mkv
```

The batch verifies source/exporter provenance on resume, native publications and
choices, absence of pointer buttons, glyph-buffer audits, and complete decoded
RGBA/PCM/PTS equality. It records lines without full native UI reveal rather than
extending them or substituting subtitle cards. The assembly uses the sequence
assembler's lossless video copy and exact PCM concatenation.

`render --reuse-batch PATH` can reuse completed chapters from an earlier batch,
including a batch with other failed chapters. Plans, asset-manifest hashes, and
exporter hashes must match. Each selected chapter is fully decoded and its
source, trace, and media evidence compared with the saved coverage entry before
the new batch links it. Failed or altered entries cannot become successful by
being reused. The original batch and its failures are unchanged. Keep the source
capture directories: reused chapters are links, not independent copies. Resume
still verifies the linked captures. Different binaries require new captures.

The verified local `dialogue-native-v4` batch contains two CB chapters totaling
44.988 seconds and two BBB chapters totaling 75.026 seconds. Its chaptered movies
are `output/anthology/cb-intro-dialogue-native.mkv` and
`output/anthology/bbb-intro-dialogue-native.mkv`. They are introductory dialogue
branches only, not the completed full-game anthologies.

Four additional chapters render Bob's mission with both CB answers, BBB Bob's
recorded message, and BBB SCRIPT2 HONK's cryobox conversation. Contact chapters
include the native opening/closing transitions and embedded movie sequences.
No missing or skipped sequence is inserted by the exporter. These assembled
movies have passed full decoded-frame, PCM, timestamp, and chapter verification:

| Game | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| CB | `cb-bob-mission-dialogue-native.mkv` | 2 | 320.566 s | 4,776 |
| BBB | `bbb-mission-dialogue-native.mkv` | 2 | 128.068 s | 1,983 |

```sh
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan accuracy/anthology-dialogue/cb-bob-mission-yes.json \
  --plan accuracy/anthology-dialogue/cb-bob-mission-no.json \
  --out output/anthology/dialogue-contacts/cb
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets output/big-bug-bang/imported-assets \
  --plan accuracy/anthology-dialogue/bbb-bob-recorded-mission.json \
  --plan accuracy/anthology-dialogue/bbb-honk-daddy-cryobox.json \
  --out output/anthology/dialogue-contacts/bbb
```

The CB assembled movie retains its `dialogue-contacts-v2/cb` inputs, and the BBB
movie uses `dialogue-contacts-v3/bbb`. The v3 exporter is archived at
`output/anthology/dialogue-contacts-v3/bin/offline-presentation` for reproduction.
Both CB chapters reproduced with that exporter in `dialogue-contacts-v3/cb`,
matching their v2 pixels, PCM, endpoints, and runner reports. The final-build
BBB SCRIPT2 reproduction in `dialogue-final-build/bbb` matched those same fields.
Frame inspection confirmed original contact artwork and fonts and BBB English
text in sampled encoded frames; this is not visual inspection of every line.

### Static BAS Topic Plans

```sh
uv run tools/native_bas_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile 2 --procedure 7244 \
  --out output/anthology/bas-plans/cb-bob-script2
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan-set output/anthology/bas-plans/cb-bob-script2/planning.json \
  --exporter output/anthology/dialogue-bas-bob-v1/bin/offline-presentation \
  --out output/anthology/dialogue-bas-bob-v1/cb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue-bas-bob-v1/cb \
  --out output/anthology/cb-bob-bas-topics-native.mkv
```

The planner follows the static graph's first-match menu links and emits finite
paths to simple positive-history topic replies. It selects a topic once for each
successive reply and closes through an authored `bye_bye` row. Random, record,
resume, and additional-history gates, unvisited nodes, and untargeted sites remain
explicit in `planning.json`; no UI traversal is used to discover them. Graph and
contact-manifest hashes bind the planning report. Generated plans are candidates,
not proof that a given COD contact procedure reaches that BAS menu.

The plans use explicit prepared contact state, validated against the CB contact
manifest. Capture retains all changed save bytes and before/after hashes; this
is not a continuous gameplay route. Required BAS publications are independently
source-bound and cannot alias COD offsets. The normal native text, menu, hand,
animation, audio, and closing-transition lifecycle remains active.

The verified Bob SCRIPT2 batch has **nine chapters, 408.562 seconds, and 6,105
native frames**. It targeted 38 BAS sites and published 39, including an extra
nested-menu-path line. All 39 have full native UI-buffer reveal evidence. Its
assembled movie passed full decoded RGBA, PCM, timestamp, and chapter checks.
Selected encoded frames were inspected for the original artwork and fonts,
including a nested cottage-topic reply; this is not inspection of every line.
Fourteen of this actor list's 53 BAS sites remain unrecorded. A separate Bronko
contact probe ended before its planned choices and is not counted as coverage.

### Prepared Travel Topics

```sh
uv run tools/native_bas_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile 2 --procedure 23683 \
  --travel-planet Moskito --travel-destination usine \
  --out output/anthology/bas-plans/cb-bronko-travel-script2-v2
nix develop -c uv run tools/native_dialogue_anthology.py render \
  --assets "$HOME/.local/share/commander-blood/assets-v1" \
  --plan-set output/anthology/bas-plans/cb-bronko-travel-script2-v2/planning.json \
  --exporter output/anthology/dialogue-travel-bronko-v4/bin/offline-presentation \
  --out output/anthology/dialogue-travel-bronko-v4/cb
nix develop -c uv run tools/native_dialogue_anthology.py assemble \
  --batch output/anthology/dialogue-travel-bronko-v4/cb \
  --out output/anthology/cb-bronko-bas-topics-native.mkv
```

The travel planner reads the hashed COD graph's outer travel guard rather than
the contact manifest. Runtime setup validates the actor and supported predicates,
records changed save bytes and hashes, then starts the native post-HUD travel
lifecycle. Arrival, automatic actor selection, conversation, and the completed
return to the bridge remain in each chapter. This is explicitly prepared state,
not a continuous route or proof that prior gameplay reached those conditions.

The Bronko batch verified **seven chapters, 183.094 seconds, and 2,788 native
frames**. It targeted 17 BAS sites and published 21, plus three COD sites; all
24 have full native UI-buffer reveal evidence. Nine of Bronko's 30 BAS sites
remain unrecorded. Buy and war each repeat the authored goodbye once after a
native menu reopening; the explicit bounded policy does not force closure or
change timing. Earlier failed contact/travel attempts are not counted.

The assembled video passed full decoded RGBA, PCM, timestamp, and chapter checks.
The checked-in energy fixture independently reproduced the generated chapter's
487 frames, pixels, PCM, and endpoint. A sampled frame from the assembled video
was inspected for original factory artwork, font, text, and hand; this is not
visual inspection of every line. No BBB travel chapter is claimed by this batch.

Three further actor batches use the same archived Bronko v4 exporter:

| Actor | Output under `output/anthology` | Chapters | Duration | Native frames |
| --- | --- | ---: | ---: | ---: |
| Yoko | `cb-yoko-bas-topics-native.mkv` | 28 | 1,336.188 s | 20,733 |
| Daddy Gluxx | `cb-gluxx-bas-topics-native.mkv` | 5 | 408.960 s | 6,095 |
| Izwalito | `cb-izwalito-bas-topics-native.mkv` | 11 | 409.542 s | 6,430 |

Their capture batches are `dialogue-travel-yoko-v2/cb`,
`dialogue-travel-gluxx-v2/cb`, and `dialogue-travel-izwalito-v4/cb`. Yoko uses
SCRIPT2 procedure 32225 on Rondo at `pavillon`; Gluxx uses 34287 on Ekatomb;
Izwalito uses 21258 on Corpo with `--entry-menu 5915`. The latter is the BAS menu
selected by the COD `topic = "talk"` assignment, not a forced runtime setting.
Repeated topic names are distinguished by their actual menu paths.

The generated Gluxx treatment plan is replaced by the checked-in three-selection
plan to account for an unconditional intervening line. Izwalito's generic ideal
plan is replaced by both checked-in secret-answer branches. Each retains the
BAS goodbye that triggers the COD question and the later resumed-menu goodbye.
The accepted answer does not open the `know` submenu in this captured route;
those sites are still uncovered. No timing or script modification was needed.
The corrected Gluxx and Izwalito batches reuse their other verified chapters;
all earlier failed batches and diagnostic attempts remain separate.

Yoko publishes 70 BAS and six COD sites, Gluxx 15 BAS and 17 COD, and Izwalito
36 BAS and nine COD, all with full native UI-buffer reveals. Their respective
actor lists still have 34, six, and eleven BAS sites unrecorded. These 44 chapters
are additional prepared-state branches, not exhaustive actor dialogue or a
continuous playthrough. Sampled encoded frames were inspected; complete decoded
RGBA, PCM, timing, and chapter verification applies to each assembled movie.
The final-build reproduction of Izwalito's acceptance branch in
`dialogue-travel-actors-final-build/cb` matches the original capture's pixels,
PCM, endpoint, and complete runner report. Its binary is archived in the sibling
`bin` directory.

### BBB Prepared Contacts

The eleven checked-in SCRIPT2-5 prepared-contact plans produce
`output/anthology/bbb-cryobox-dialogue-native.mkv`: **266.934 seconds and 3,986
native frames**. They cover Bob's brief return, concert aftermath for Daddy,
Papy, Mamy, Marakas, Izwalito, Tequila, Eviscerator, and Outrageor, and Tequila's
outburst and ghost branches. The capture batch is
`output/anthology/dialogue-bbb-prepared-contacts-v1/bbb`; its exporter is archived
in the sibling `bin` directory.

BBB preparation reads the typed COD outer D1 guard and validates the selected
actor's action record. Only supported entry predicates are prepared; body
conditions are not promoted to entry state. Other outer contact procedures are
disabled. The report binds the COD hash and retains every changed save byte and
before/after hashes. CB continues to use its existing contact manifest. This is
explicit chapter setup, not proof of a gameplay route or contact-menu eligibility.

All 75 required COD publications occurred: 64 fully reveal in the native UI
buffer, and eleven terminal sites have no UI draw. Native CRYOGEL/CRYORAD
transitions, embedded movie sequences, English display text, fonts, timing, and
audio remain active. No artificial hold or missing-clip insertion was added.
The assembled movie passed full decoded RGBA, PCM, timestamp, and chapter checks.
Sampled encoded Bob and Tequila frames show the original artwork and subtitle
fonts with English text; this is not visual inspection of every line.
Two CB regression captures preserve prior pixels, PCM, and endpoints: Bob's
black-hole contact and Bronko's energy travel topic. These checks do not establish
original-executable parity or full-game coverage.

Ten later-profile chapters produce
`output/anthology/bbb-later-contacts-dialogue-native.mkv`: **274.948 seconds and
4,099 native frames**. They cover Rotator, Otto Von Smile, Otto Von Gluk, Trump,
Tramp, and Super Tromp after the concert; Bug Deluxe's future and farewell
conversations; and Cyberquizz's first visit and Christmas greeting. Their batch
is `dialogue-bbb-later-contacts-v1/bbb`, using the same archived prepared-contact
exporter as the preceding eleven chapters. All 74 required publications occur;
64 fully reveal in the native UI buffer, and ten terminal sites have no draw.
The movie passed full decoded RGBA, PCM, timestamp, and chapter verification.
Sampled encoded Bug Deluxe and Cyberquizz frames were inspected for native
artwork, fonts, and English text; not every line was visually inspected.

Cyberquizz's second visit is
`output/anthology/bbb-cyberquizz-second-visit-native.mkv`: **63.876 seconds and
951 native frames**, from `dialogue-bbb-encounters-v1/bbb`, whose new exporter is
archived in its sibling `bin` directory. `contact_encounter_guard: 2114` binds the
authored equality test on Cyberquizz's encounter counter within procedure 1748.
Setup stores one; native C4 entry increments it to two. Both visit plans assert
that the other branch's lines are absent. This is explicit prepared visit state,
not a replay of the first encounter. Unsupported or out-of-procedure guards and
arbitrary body assignments are rejected.

All 14 second-visit publications occur, 13 fully reveal in the UI buffer, and
terminal site 2504 has no draw. The native `PPIT07` sequence and contact closing
transition remain intact. Assembly passed full decoded media, timing, and
chapter checks; a sampled encoded second-visit frame was visually inspected.
The new exporter reproduces Cyberquizz's first visit, CB Bob's black-hole
contact, and CB Bronko's energy travel topic with identical pixels, PCM,
endpoints, and complete runner reports in `dialogue-bbb-encounter-regression`.

Cyberquizz and Bioquizz travel greetings produce
`output/anthology/bbb-quizz-travel-dialogue-native.mkv`: **66.400 seconds and
992 native frames**, from `dialogue-bbb-travel-quizz-v1/bbb`. The exporter is
archived in `dialogue-bbb-travel-probe-v2/bin`. Each chapter explicitly stages
the selected actor at Cyberock's Cyberland destination, including the native
navigation visibility flag. Cyberquizz also enables its source-bound companion
travel procedure. Native arrival, actor selection, dialogue, empty inventory
response, departure, and bridge return remain active. This is prepared chapter
state, not evidence of a normal gameplay route or canonical actor placement.

The two chapters publish twelve sites: ten fully reveal in the native UI buffer,
and terminal sites 4795 and 6046 have no draw. Assembly passed full decoded RGBA,
PCM, timestamp, and chapter checks. Encoded dialogue samples for both actors
were visually inspected; this is not an inspection of every line. The new
exporter reproduces CB Bob's black-hole contact, CB Bronko's energy travel
topic, and BBB Cyberquizz's first visit with identical pixels, PCM, endpoints,
and runner reports in `dialogue-bbb-travel-regression`.

The checked-in Bug Deluxe travel plan remains a failed probe in
`dialogue-bbb-travel-probe-v2/bbb`, not verified coverage. Numeric chatter tries
to hash dictionary offset 3944 while SCRIPT9.DIC contains only 2,612 bytes.
Original-executable boundary probes demonstrate that the hash depends on bytes
beyond that resource; the live native allocation contents are not recovered.
No zero-padding, substituted number text, or silent-audio workaround was added.

### BBB Inventory Branches

Object-backed inventory choices use their original VAR record offsets, not
dictionary words or translated labels. Optional starting inventory is explicitly
staged aboard and included in the setup's save-byte diff. The existing native
chooser, hand animation, transfer, descriptor continuation, and response scripts
remain responsible for the resulting scene. The verifier checks the actual
offered menu/item/recipient and the later ownership change. This does not prove
a normal gameplay acquisition route.

The static inventory planner binds the catalog graph, travel template, and
existing English inventory-label catalog. It requires each selected item's A6
and flat flag-guarded reaction in the native trace. It does not set item-transfer
flags or force reaction lines. Plans remain candidates until captures pass:

```sh
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --template accuracy/anthology-dialogue/bbb-cyberquizz-travel-greeting.json \
  --inventory-menu 4730 \
  --out output/anthology/plans-bbb-cyberquizz-inventory-v1
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --template accuracy/anthology-dialogue/bbb-bioquizz-travel-greeting.json \
  --inventory-menu 5981 \
  --out output/anthology/plans-bbb-bioquizz-inventory-v1
```

Each report plans 22 item branches. Nuclear reactions require additional
evolution predicates and remain deferred by this flat planner; laws and scruter have no simple
matching item-flag guard. Those are explicit gaps, not unreachable dialogue.
The checked-in technology and treaty plans exercise response requirements and
exact native identity independently of display-name encoding.

All 44 generated chapters completed with a verified native offer and ownership
transfer in each. The batches are `dialogue-bbb-inventory-v2/cyberquizz` and
`dialogue-bbb-inventory-v2/bioquizz`, with their exporter archived in the sibling
`bin` directory. Each assembled movie contains 22 chapters, **826.356 seconds
and 12,338 native frames**:

- `output/anthology/bbb-cyberquizz-inventory-dialogue-native.mkv`
- `output/anthology/bbb-bioquizz-inventory-dialogue-native.mkv`

Both passed full decoded RGBA, PCM, timestamp, and chapter checks. Each batch
publishes 29 unique COD sites: 28 fully reveal in the native UI buffer, and its
existing terminal site has no draw. Together they add 46 previously unrecorded
fully revealed sites, including the two inventory prompts and 44 reactions.
Encoded samples show the English treaty menu, native hand and actor artwork,
and guitar response. Not every encoded line was visually inspected. The earlier
string-identity prototype batch is excluded from the ledger; the final runner
uses VAR offsets, and its technology chapter preserves the prototype's pixels,
PCM, and endpoint.

Inventory chooser, transfer, and descriptor boundaries were rerun against the
original executable: 191 cases match the checked-in oracle vectors. These
boundary checks do not establish whole-conversation DOS parity. Four captures
in `dialogue-bbb-inventory-regression` preserve prior pixels, PCM, and endpoints:
CB Bob's black-hole topic, CB Bronko's energy topic, BBB HONK's instructions,
and Bioquizz's travel greeting. Runner reports also match except for the older
HONK report's legacy metadata; its native choices, publications, and timing match.

### BBB Nuclear Gifts

Six checked-in nuclear-gift plans add source-bound initial evolution guards to
the existing native inventory path. Cyberquizz guards 3669/3687/3752 and
Bioquizz guards 4920/4938/5003 derive starting values 0/101/501 using the
production signed comparison handler. The preparation records its guard,
before/after value, and save-byte diff; trace verification checks the actual
actor evolution before accepting the gift interaction. No item-transfer flag
or response line is forced.

The batch `output/anthology/dialogue-bbb-nuclear-v2/bbb` contains six verified
chapters, assembled as
`output/anthology/bbb-nuclear-gift-dialogue-native.mkv`: **224.232 seconds and
3,348 native frames**. The archived exporter is
`dialogue-bbb-nuclear-v1/bin/offline-presentation`. The initial v1 capture batch
failed only at encoding because it was launched without the Nix environment's
`ffmpeg`; it is excluded from coverage. Run captures inside `nix develop -c`.

Every chapter verifies the native item offer and ownership transfer. Middle
and high captures require their own response and exclude the other; low
captures exclude both spoken responses. The four new response sites fully
reveal in the UI buffer. Actor traces show the middle branch's population
1-to-0 and aggressiveness 0-to-50 changes, and the high branch's evolution
501-to-521 and energy 0-to-100 changes. Low captures are not evidence that no
global state changed. The authored 100 and 500 boundary gaps and signed-word
comparisons are covered by binding tests, not additional movie chapters.

The assembled movie passed full decoded RGBA, PCM, timestamp, and chapter
checks. Encoded samples of Cyberquizz's middle and Bioquizz's high response
show fully revealed English text; not every encoded frame was visually
inspected. The original executable's 1,728 shared-state arithmetic cases still
match the checked-in vectors. Three regression captures in
`dialogue-bbb-nuclear-regression` exactly preserve the prior runner, pixels,
audio, and endpoint for CB Bob's black-hole topic, Cyberquizz's technology
gift, and Bioquizz's travel greeting. These checks do not prove complete DOS
conversation parity or a normal gameplay acquisition/evolution route.

### BBB Paul Gifts

Three source-bound SCRIPT16 templates cover prepared gift visits to Mega Paul,
Sebasto Paul, and Inter Paul. They explicitly stage each actor at Cyberland,
enable the selected gift procedure, and leave native arrival, presentation,
inventory selection, transfer, and closure intact. This is not a reconstructed
route from their initial Trashlando placement. Mega Paul's separate story
procedures are not enabled by his gift template.

The flat planner produces 22 item candidates per actor. The templates are
`accuracy/anthology-dialogue/bbb-{mega,sebasto,inter}-paul-gift-visit.json`, with
inventory menus 5540, 6935, and 8328 respectively. The generated plan sets are
`output/anthology/plans-bbb-{mega,sebasto,inter}-paul-inventory-v1/planning.json`.
Nuclear responses and other evolution/story branches remain outside these
flat plan sets.

All 66 generated chapters passed native offer/transfer and lossless-media
verification in `output/anthology/dialogue-bbb-paul-inventory-v1`. The exporter
is the unchanged archived `dialogue-bbb-nuclear-v1/bin/offline-presentation`.
The three assembled outputs are:

| Movie under `output/anthology` | Chapters | Seconds | Native Frames |
| --- | ---: | ---: | ---: |
| `bbb-mega-paul-inventory-dialogue-native.mkv` | 22 | 707.632 | 10,660 |
| `bbb-sebasto-paul-inventory-dialogue-native.mkv` | 22 | 810.952 | 12,193 |
| `bbb-inter-paul-inventory-dialogue-native.mkv` | 22 | 810.952 | 12,193 |

All three assemblies passed full decoded RGBA, PCM, timestamp, and chapter
verification. The 69 captures including visits total 2,423.170 seconds and
36,455 native frames.

Together the batches publish 85 distinct COD sites: 82 fully reveal in the
native UI buffer and three terminal sites (5580, 6975, 8368) have no draw.
Encoded Mega/Inter guitar-response samples show the native artwork and fully
revealed English text. This is not a visual audit of every encoded line. The
three unstocked visit captures also passed, assembled as
`bbb-paul-gift-visits-native.mkv` (93.634 seconds, 1,409 frames) from
`dialogue-bbb-paul-gift-visits-v1/bbb`. They add chapters, not distinct response
coverage beyond the stocked branches.

The planner can also derive a gift-entry template directly from the hashed
catalog, without a hand-written visit fixture:

```sh
uv run tools/native_inventory_plans.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --profile SCRIPT16 --inventory-menu 5540 \
  --planet Cyberock --destination Cyberland \
  --out output/anthology/plans-bbb-mega-paul-derived-inventory-v1
```

This mode accepts only an outer guard containing exactly D0 travel plus a
positive actor selection bound to the menu owner and player. Its report retains
the derived template and graph hash. Extra entry predicates are rejected; actor
kind, procedure, destination type, and planet relationship are independently
checked by the native exporter. The generated plans require the menu and flat
reaction, not unrelated introductory lines. Every actual publication is still
retained in the trace and ledger.

A static scan finds 963 flat candidates across 46 inventory menus. This is not
runtime coverage: one Daddy Gluxx menu has no flat candidates, complex reactions
remain deferred, and some actors still hit the unresolved numeric-chatter
dictionary-tail issue. The graph-derived Mega Paul guitar capture in
`dialogue-bbb-derived-entry-regression/bbb` matches the hand-written template's
RGBA, PCM, endpoint, and runner behavior (apart from the different required-site
plan). It is not counted as additional dialogue coverage. Regenerating Inter
Paul's original template-based plan set is byte-for-byte unchanged.

### BBB Eviscerator and Outrageor Gifts

SCRIPT5 has 23 flat item reactions for each actor, derived from source-bound
inventory menus 5431 (Eviscerator) and 7846 (Outrageor). Their checked-in nuclear
fixtures and generated plans explicitly stage the actor at Cyberland and the
item aboard; these are prepared branches, not acquisition or encounter routes.
The plan sets are `plans-bbb-eviscerator-inventory-v1` and
`plans-bbb-outrageor-inventory-v2` under `output/anthology`.

All 46 captures passed in `dialogue-bbb-mutant-inventory-v1/eviscerator` and
`dialogue-bbb-mutant-inventory-v2/outrageor`. Both assemblies passed full
decoded RGBA, PCM, timestamp, and chapter checks:

| Movie under `output/anthology` | Chapters | Seconds | Native Frames |
| --- | ---: | ---: | ---: |
| `bbb-eviscerator-inventory-dialogue-native.mkv` | 23 | 726.174 | 10,944 |
| `bbb-outrageor-inventory-dialogue-native.mkv` | 23 | 704.994 | 10,606 |

Together they add 1,431.168 seconds and 21,550 native frames. Eviscerator
publishes 28 distinct COD sites, 27 fully revealed in the native UI buffer;
Outrageor publishes 32, 31 fully revealed. Terminal sites 5471 and 7886 have
no draw. These counts include control text, not only spoken sentences.

Outrageor's nuclear reaction writes the item's holder back to aboard at COD
6712. The native inventory menu reopens. A new `inventory_cancel` choice binds
the same source menu but names neither an item nor a dictionary word. It
requests the chooser's existing final row and retains native hand animation,
Closing/Closed states, and script completion. The exporter cannot use it when
the native chooser has no cancel row. The trace verifier checks visible cancel
glyphs, matching menu/recipient, native closure, and aboard ownership of every
offered item through closure; the coverage ledger recomputes that evidence.
The flat planner adds cancellation when a reaction writes a known inventory
holder back to the aboard sentinel, still subject to native capture verification.

In the nuclear pilot, the item is offered at 23.512 seconds, transferred at
23.966, and returned before the reopened menu at 27.980. Cancellation enters
Closing at 28.048 and Closed at 28.366, retaining the nuclear item aboard.
The original executable's 11 inventory-chooser cases and four call-order cases
still match the checked-in vectors. This is not a complete original-executable
conversation comparison.

The Eviscerator batch uses the archived `dialogue-bbb-nuclear-v1` exporter.
Outrageor uses `dialogue-bbb-inventory-cancel-v1/bin/offline-presentation`.
Three captures under `dialogue-bbb-inventory-cancel-regression` preserve the
previous RGBA, PCM, endpoint, timing, and complete runner records for CB Bob's
black-hole topic, Cyberquizz's technology gift, and Eviscerator's nuclear gift.
Encoded samples show Eviscerator's guitar response, Outrageor's nuclear response,
and the returned-item CANCEL row. Not every encoded frame was visually audited.

The initial SCRIPT6 probes in `dialogue-bbb-mutant-inventory-pilot-v1` failed:
Emasculator's actor-only story procedure reaches a name/no question before the
gift menu; Rotator's first-visit story ends before gift selection. Each has 20
flat candidates and three deferred nested reactions. Those failed probes are
not counted as coverage; the following captures retain the story procedures
and prepare source-bound later encounters.

### BBB Emasculator and Rotator Visits

Thirty SCRIPT6 fixtures record both actors' first visits, selected later story
branches, and six nested gift reactions. All explicitly stage the actor at
Cyberland. First visits retain the native initial counter. Later visits use the
new BBB-only
`travel_setup.actor_encounter_guard`: a source offset inside an enabled
actor-only story procedure, whose outer guard must have no extra predicates.
Only a single positive equality against this actor's encounter field is
accepted. The exporter writes one less than the authored operand, then native
C4 entry increments it. The report retains the original/prepared values and
save hashes; the trace verifier checks preparation and the first presentation
of the selected actor. The ledger independently recomputes this evidence.
This is prepared visit state, not a replay of earlier encounters.

The first-visit batch is `dialogue-bbb-script6-first-visits-v2/bbb`; its three
chapters total 402.006 seconds and 6,050 native frames. It publishes 79 distinct
COD sites, 77 fully revealed in the UI buffer and two terminal sites without
a draw. The later-visit batch `dialogue-bbb-script6-later-visits-v1/bbb` adds
three chapters totaling 186.708 seconds and 2,802 frames: Emasculator's second
visit with the affirmative answer, Rotator's second visit, and his fourth
visit declining the offer. It publishes 46 sites, 43 fully revealed and three
without a draw. Rotator's second-visit site 12345 is not published; preceding
A6 12329 carries skip-next=1 when not shown. Its fixture records this native
omission instead of adding text or a hold.

The inventory planner now retains validated, same-actor COD dictionary
prerequisites before appending the item choice. Emasculator's template keeps
the second-visit affirmative answer, and Rotator's keeps the fourth-visit
decline. Native story, chooser, transfer, returned-item cancellation, and
closure still execute. Both actors have 20 static flat candidates. The planner
defers technology, culture, and writing, but separate source-bound fixtures
capture one native path through each of those six reactions. Emasculator's
captured technology path publishes COD 9578. COD 9495 requires the separate
`globals.A100 > 4` branch and remains unrecorded. Laws and scruter have no
matching simple item-flag guard.

Emasculator's painting candidate (VAR 7256) fails to publish required COD
9218/9244 in both second-visit and third-visit probes. The native trace shows
the item flag cleared before either publication. The source's final chatter
has skip-next=2 before three state mutations, including that clear, but the
cause and original-game behavior have not been established. The failed batch
and separate `dialogue-bbb-script6-painting-probe-v1/bbb` remain evidence of an
unresolved gap, not global unreachability. The final Emasculator plan set uses
`--exclude-item 7256`, preserving the exclusion in its planning report and
leaving the reaction uncovered.

The final batches and their assembled outputs are:

| Movie under `output/anthology` | Batch suffix | Chapters | Seconds | Native frames |
| --- | --- | ---: | ---: | ---: |
| `bbb-script6-first-visits-native.mkv` | `dialogue-bbb-script6-first-visits-v2/bbb` | 3 | 402.006 | 6,050 |
| `bbb-script6-later-visits-native.mkv` | `dialogue-bbb-script6-later-visits-v1/bbb` | 3 | 186.708 | 2,802 |
| `bbb-script6-story-dialogue-native.mkv` | `dialogue-bbb-script6-story-branches-v3/bbb` | 18 | 1,619.826 | 24,238 |
| `bbb-emasculator-inventory-dialogue-native.mkv` | `dialogue-bbb-script6-inventory-v2/emasculator` | 19 | 1,657.214 | 24,870 |
| `bbb-rotator-inventory-dialogue-native.mkv` | `dialogue-bbb-script6-inventory-v1/rotator` | 20 | 1,288.924 | 19,453 |
| `bbb-script6-nested-gifts-native.mkv` | `dialogue-bbb-script6-nested-gifts-v1/bbb` | 6 | 472.732 | 7,115 |

Each assembled movie passed full decoded RGBA, PCM, timestamp, and chapter
verification. First-visit, later-visit, Rotator painting, and Emasculator
fifth-visit encoded samples were visually inspected; this is not an audit of
every encoded line. Rotator's fourth-visit acceptance publishes COD 13395
without a frame-boundary UI draw. Its fixture requires the publication and
records that limitation.

First visits use the archived `dialogue-bbb-inventory-cancel-v1` exporter.
Later visits and gifts use `dialogue-bbb-travel-encounters-v1/bin/offline-presentation`.
Three captures under `dialogue-bbb-travel-encounters-regression` exactly retain
the previous pixels, PCM, endpoint, and runner records for CB Bob's black-hole
topic, Cyberquizz's second visit, and Outrageor's nuclear gift. All 33 original
BBB action-dispatch oracle cases still match the checked-in vectors. These
checks do not establish complete DOS conversation parity. Unrecorded answer
combinations, alternate nested reaction states, and global story states remain open.

### Whole-Catalog Dialogue Ledger

```sh
uv run tools/native_dialogue_coverage.py \
  --catalog output/anthology/static-dialogue-en-v2 \
  --batch output/anthology/dialogue-native-v4/cb \
  --batch output/anthology/dialogue-native-v4/bbb \
  --batch output/anthology/dialogue-contacts-v2/cb \
  --batch output/anthology/dialogue-contacts-v3/bbb \
  --batch output/anthology/dialogue-bas-bob-v1/cb \
  --batch output/anthology/dialogue-travel-bronko-v4/cb \
  --batch output/anthology/dialogue-travel-yoko-v2/cb \
  --batch output/anthology/dialogue-travel-gluxx-v2/cb \
  --batch output/anthology/dialogue-travel-izwalito-v4/cb \
  --batch output/anthology/dialogue-bbb-prepared-contacts-v1/bbb \
  --batch output/anthology/dialogue-bbb-later-contacts-v1/bbb \
  --batch output/anthology/dialogue-bbb-encounters-v1/bbb \
  --batch output/anthology/dialogue-bbb-travel-quizz-v1/bbb \
  --batch output/anthology/dialogue-bbb-inventory-v2/cyberquizz \
  --batch output/anthology/dialogue-bbb-inventory-v2/bioquizz \
  --batch output/anthology/dialogue-bbb-nuclear-v2/bbb \
  --batch output/anthology/dialogue-bbb-paul-gift-visits-v1/bbb \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/mega-paul \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/sebasto-paul \
  --batch output/anthology/dialogue-bbb-paul-inventory-v1/inter-paul \
  --batch output/anthology/dialogue-bbb-mutant-inventory-v1/eviscerator \
  --batch output/anthology/dialogue-bbb-mutant-inventory-v2/outrageor \
  --batch output/anthology/dialogue-bbb-script6-first-visits-v2/bbb \
  --batch output/anthology/dialogue-bbb-script6-later-visits-v1/bbb \
  --batch output/anthology/dialogue-bbb-script6-story-branches-v3/bbb \
  --batch output/anthology/dialogue-bbb-script6-inventory-v2/emasculator \
  --batch output/anthology/dialogue-bbb-script6-inventory-v1/rotator \
  --batch output/anthology/dialogue-bbb-script6-nested-gifts-v1/bbb \
  --out output/anthology/dialogue-coverage-bbb-script6-full.json
```

The ledger binds graph, chapter, trace, and media hashes, recomputes UI evidence
from the retained trace, and joins by game/profile/source-kind/offset. BAS
evidence requires the matching BAS hash. Repeated captures do not inflate the site census. A
site absent in one branch can still be published in another; absence is never
classified as global unreachability.

Across the 326 verified dialogue chapters, CB has 261 sites fully revealed in
the native UI buffer, six published without a UI draw, one absent from the
selected branches, and 5,268 uncovered. BBB has 787 fully revealed, 41
published without a UI draw, two absent from the selected branches, and 6,091
uncovered. Native UI-buffer evidence alone does not prove encoded glyph
visibility. These counts include empty/control text sites and do
not imply that every uncovered site is a unique spoken line. Neither full-game
anthology is complete; remaining profile branches, most CB BAS, state-dependent
conversations, objects, travel, and environments still need coverage.

### Combined Verified Movies

`native_sequence_anthology.py assemble` accepts repeated `--batch` arguments
in chapter order. It rejects incomplete or mixed-game batches and duplicate
captures. The two combined outputs join the 65 verified sequence chapters with
the 326 verified dialogue chapters. Each manifest lists its ordered
`source_batches`, source chapter hashes, chapter boundaries, and whole-file
RGBA/PCM hashes.

| Movie under `output/anthology` | Chapters | Seconds | Native frames |
| --- | ---: | ---: | ---: |
| `cb-verified-anthology-v1.mkv` | 75 | 3,720.252 | 56,962 |
| `bbb-verified-anthology-v1.mkv` | 316 | 15,290.096 | 229,684 |

Both combined movies passed full decoded pixel, float PCM, timestamp, and
chapter verification. They preserve the native chapter captures in order;
they are chaptered anthologies, not continuous playthroughs. The dialogue
ledger above still records thousands of uncovered sites. CB's authored `year`
sequence names `SQ/PUVEN1.HNM`, which is absent from the imported assets; the
native capture does not fabricate it. BBB's Bug Deluxe numeric-chatter travel
branch and Emasculator's painting reaction likewise remain unresolved. These
limitations prevent either movie from being called an exhaustive game video.

The sequence and initial dialogue batch binaries are archived in `native-sequences-v2/bin`. To re-verify
or resume these batches after rebuilding, pass their `offline-presentation` and
`video-catalog` paths through `--exporter` and `--catalog-binary`. Each chapter and
assembled movie retains its own hashes and native timing evidence. Final-build
reproduction of BBB's repeated-clip `39argent` record matched the batch's pixels,
PCM, endpoint, and runner report exactly.

## Tests

```sh
uv run python -m unittest discover -s tools -p test_dialogue_catalog.py
uv run python -m unittest discover -s tools -p 'test_native_*.py'
nix develop -c cargo test --release -p commander-blood-script-compiler \
  --test dialogue_catalog
nix develop -c uv run --with av==18.1.0 python -m unittest discover \
  -s tools -p test_video_anthology.py
nix develop -c cargo test -p commander-blood-game --lib
nix develop -c env CBLOOD_REQUIRE_ACCURACY_TESTS=1 \
  cargo test --release -p commander-blood-game --lib offline
nix develop -c cargo test --release -p commander-blood-game --lib \
  runtime::offline_export::tests::offline_export_preserves_variable_intervals_pixels_and_audio \
  -- --ignored --exact
nix develop -c cargo test -p commander-blood-game --lib \
  recording::tests::lossless_writer_preserves_frames_audio_and_shared_start_offset \
  -- --ignored --exact
```

PCM packet muxing uses [PyAV audio frames](https://pyav.org/docs/stable/api/audio.html)
and [containers](https://pyav.org/docs/stable/api/container.html); clock correction
uses FFmpeg's `aresample` filter. Captured media and imported game assets are not
checked into the repository.
