# Native Dialogue Chapters

These plans select branches of the COD/BAS dialogue trees. They bind the
original COD/DIC bytes (and BAS bytes when used) and address choices by source
offset and DIC word or VAR item-record offset, not screen coordinates. `required_frame_boundary_cod_sites` checks
retained script state, not visible pixels. The batch report separately records
glyph-buffer evidence and lines with no full reveal.

Current scope: CB SCRIPT1 Izwalito's game/explanations choices and Bob's mission
yes/no choices; BBB SCRIPT1 HONK's PLAY/INSTRUCTIONS choices and Bob's recorded
mission; BBB SCRIPT2 HONK's Daddy-in-the-cryobox conversation; CB SCRIPT2 Bob's
black-hole BAS topic, Bronko's energy topic, Daddy Gluxx's treatment, and both
answers to Izwalito's ideal/secret question through prepared travel; BBB SCRIPT2
Bob's brief return, SCRIPT3-5 concert aftermath for eight actors, and both
Tequila cryobox outburst/ghost branches; later-profile concert reactions, Bug
Deluxe's future/farewell conversations, and Cyberquizz's first/second visits and
Christmas greeting; Cyberquizz/Bioquizz travel greetings; and inventory gifts
to both quizzers. There are 46 checked-in plans, including the currently failing
Bug Deluxe travel probe. These
checked-in plans do not cover all actors, dialogue branches, objects, travel, or
environments. The static BAS and inventory planners also produce candidate plan
sets from the hashed catalog without running the game.

## Native Omissions

Do not add artificial holds or subtitle cards to fill these gaps:

| Game/branch | COD offset (decimal) | Observed disposition |
| --- | ---: | --- |
| CB game | 1206 | Published in the final frame, without a glyph-buffer draw before profile handoff. |
| CB explanations | 1552 | Published in the final frame, without a glyph-buffer draw before presentation termination. |
| BBB PLAY | 1860 | Published then replaced by profile loading within the same frame; not retained at frame end. |
| BBB INSTRUCTIONS | 2355 | Never published on this branch: the preceding A6 at 2317 arms skip-next=1 even when inactive on a subsequent VM pass. |
| BBB INSTRUCTIONS | 2371 | Published without a glyph-buffer draw before presentation termination. |
| CB Bob yes | 2067 | Not published: the selected affirmative branch skips the negative reply. |
| CB Bob both choices | 2300 | Not published: inactive A6 at 2257 skips three tokens, including this empty A6 and the A8 for `SQ/AARCHE20.HNM`. |
| CB Bob both choices | 2323, 2346, 3047 | Published without a glyph-buffer draw: two empty sequence-control sites and terminal `stop`. |
| BBB Bob | 3252 | Terminal publication without a glyph-buffer draw. |
| BBB SCRIPT2 HONK | 7461 | Terminal publication without a glyph-buffer draw. |
| BBB prepared contacts, SCRIPT2-5 | 7183; 3005, 15361, 21365; 1783, 8054, 15997, 16230, 16482; 1895, 5619 | One terminal publication per chapter without a glyph-buffer draw; offsets are profile-local. |
| BBB later contacts, SCRIPT6/9/13/14/17 | 11669; 2638, 3109; 1837, 9984; 1811, 6886, 10995; 2098, 2504, 4841 | One terminal publication per chapter without a glyph-buffer draw; includes both Cyberquizz visits. |
| BBB SCRIPT17 travel greetings | 4795, 6046 | Terminal publications without a glyph-buffer draw. |

The 2355 disposition is distinct from a publication that was not drawn. The
original `BLOOD2PG.EXE` A6/outer-loop oracle now includes five skip-next cases:
accepted, inactive, already shown, active menu, and active subtitle. The first
executes the following token; the four rejected cases skip it. The typed Rust
dispatcher agrees with all 81 oracle cases. This is boundary evidence, not a
complete DOS gameplay replay of the chapter.

BBB profile loading was separately checked against the original executable's
512 gate cases, five resets, and 17 post-load cases. Those gates do not wait for
active text. A publication observer retains old-profile events across a
same-frame load so they are not falsely classified as never executed.

## Capture Contract

The runner performs normal initialization, removes the startup sequence slots
(their original bytes remain in the report), closes the native startup panel,
then enters the requested presentation. The default `entry` is `radio`, which
loads the original radio sound bank and uses typed C4 radio entry. Semantic word
selection uses the native choice widget, including
its selected frame, hand request, sound, and close phases. It is not a gameplay
route or mouse-input simulation. `entry: "contact"` requests the typed CONTACTS
scene transition, including its opening and closing sequences. Contact chapters
must finish native scene cleanup and navigation rebuilding before capture stops.

For nonzero `initial_profile`, the runner queues the validated profile in the
native pending-profile slot and waits for the ordinary lifecycle handoff before
binding the target and starting capture. The report retains the selection time.
This is chapter setup, not evidence that prior gameplay reached this state or
that the target was eligible in the contact menu. SCRIPT2 HONK has been rendered;
later profiles may require additional authored prerequisites. Plan requirements
and choices refer only to the selected profile's COD/BAS sites, not a chain of
different profile-local conversations.

`source` on a choice defaults to `cod`. `bas` identifies an A6 text instruction;
`bas_menu` identifies the Menu body offset, not its selector-node root. Both BAS
forms require `bas_sha256`. BAS publication and frame-boundary requirements use
`required_bas_sites` and `required_frame_boundary_bas_sites`; they cannot satisfy
COD requirements at the same numeric offset. Every semantic selection still
passes through the native choice widget and must be offered at that boundary.

An optional `contact_procedure` prepares the exact CB procedure using the existing
binary-derived contact manifest. For BBB, it instead validates the typed COD
procedure's outer D1 contact guard and actor action record. Both validate the
profile and actor, set supported authored entry predicates, and disable competing
contact procedures. Unsupported BBB predicates are rejected, and outer-guard
preparation does not promote body guards to entry conditions. The report retains
the CB manifest hash or BBB COD hash, original-save hashes before/after, and
every changed save byte.
This is explicit prepared-state chapter setup, not proof of contact-menu
eligibility or a gameplay route. It does not modify script text, timing, or media.

The eleven BBB prepared-contact chapters publish all 75 required COD sites;
64 fully reveal in the native UI buffer and eleven terminate without a draw.
The native CRYOGEL/CRYORAD transitions and automatically selected embedded
sequences remain in the captures. English uses the existing display overlay;
the authored French script, waits, and media selection are unchanged. Ten later
SCRIPT6/9/13/14/17 chapters add 74 publications: 64 full UI-buffer reveals and
ten terminal sites without a draw.

An optional `contact_encounter_guard` selects an authored visit-count predicate
inside the chosen contact procedure. It must be a single, positive, immediate
equality guard on the selected actor's encounter counter. A body assignment,
another actor's field, another procedure, or a compound guard is rejected. Setup
stores one less than the source count; native C4 entry performs the increment.
The report retains the guard offset, expected count, pre-entry value, and save
byte changes. This is prepared visit state, not a replay of earlier encounters.
Cyberquizz's two visit plans require their own branch's sites and assert absence
of the other branch's sites. The second visit publishes 14 required sites, with
13 full UI-buffer reveals and one terminal publication without a draw.

Bob's checked-in black-hole plan repeats the topic selection for each of five
successive native BAS replies, then selects the authored `bye_bye` row. The
initial BAS prompt and BAS goodbye are preempted by the COD conversation; the
plan does not require or invent them. The generated nine-topic batch additionally
checks nested Kanary menu paths. Generated plans remain candidates: a Bronko
SCRIPT2 contact probe ended before its planned BAS choices, so it is retained as
a failed attempt and contributes no verified coverage. That actor's BAS menu is
not established as reachable through that contact procedure.

`entry: "travel"` instead requires `travel_setup` with an authored planet,
destination, and COD procedure offset. Setup validates that procedure's outer D0
travel guard and actor action record, prepares its supported entry predicates,
and disables competing travel procedures. Equality predicates on the encounter
counter account for the native C4 increment before the COD guard. Unsupported
predicates are rejected. The report retains all changed save bytes and hashes.
The normal post-HUD travel lifecycle performs arrival, automatic actor selection,
conversation, departure, and return to the bridge. This is prepared chapter state,
not a recorded gameplay route to the planet or proof of general reachability.

Optional `supporting_procedures` re-enables explicitly named outer D0 procedures
for the same actor. Duplicate offsets, other actors, contact procedures, and
additional unprepared entry predicates are rejected. BBB-only
`stage_actor_at_destination` sets the actor's typed location and native
navigation visibility flag after validating that the destination belongs to the
selected planet. It does not bypass native arrival or automatic actor selection.
The report retains these changes and the trace verifier checks actor placement.
This is deliberate chapter staging, not a claim about the actor's canonical
location or a normal gameplay route.

Cyberquizz and Bioquizz each publish six sites: five fully reveal in the native
UI buffer, and the terminal site has no draw. They retain the native empty
inventory response, not an invented inventory choice. Encoded dialogue samples
show the native actor artwork and English font rendering. Bug Deluxe's travel
plan is source-bound but currently fails resolving numeric chatter dictionary
offset 3944, beyond SCRIPT9.DIC's 2,612 bytes. It contributes no coverage. Do not
replace that unresolved original-memory dependency with zero padding or silence.

### Object-Backed Inventory Choices

BBB `source: "inventory"` choices use `inventory_item`, the original VAR object
record byte offset, instead of `word_offset`. For example, technology is 7352
and the treaty is 7736. Names and English display labels are not selection
identities, so DOS-encoded names do not need lossy string lookup. Validation
requires a real inventory record and an authored inventory A6 for the selected
recipient. Other actors' menus and mixed dictionary/object identities fail.

`travel_setup.stage_aboard_inventory` explicitly stages these record offsets
aboard. The normal typed roster is rebuilt from the saved holder sentinel; it
is not a forged menu list. Setup retains every save-byte change. The runner
selects only an object offered by the native chooser, then waits for its normal
selection animation, transfer, descriptor handling, reaction, and closure.
The trace verifier requires the exact offered item/menu/recipient at the
selection frame and a subsequent ownership transfer. Starting inventory is
prepared chapter state, not a recorded acquisition route.

`tools/native_inventory_plans.py` derives single-item candidates from the hashed
catalog and a choiceless travel template. Each plan requires the inventory A6
and its item-flag reaction at frame boundaries. Nested or enclosing conditions,
dynamic text, extra choice gates, and reactions without a matching flag clear
remain deferred. For SCRIPT17 it plans 22 branches per quizzer. Nuclear gifts
have additional evolution guards; laws and scruter have no simple matching
reaction guard. These gaps are retained in each planning report, not counted
as covered. The technology fixture requires its spoken reaction; the treaty
fixture also checks native record identity despite its DOS-encoded name.

All 44 generated quizzer gift chapters passed native capture and transfer
verification. Each actor's 22-chapter movie is 826.356 seconds and 12,338 native
frames. Each batch publishes 29 distinct COD sites, with 28 fully revealed in
the native UI buffer and one terminal site without a draw. Sampled encoded
menus and reactions were visually inspected; this is not a visual audit of
every line or a full-game completion claim.

Six nuclear-gift fixtures cover low, middle, and high evolution for both
quizzers. `travel_setup.actor_evolution_guard` identifies an authored guard
start in an enabled travel procedure. Preparation accepts only a complete
guard of immediate comparisons against this actor's evolution field. It uses
the production signed query evaluator to preserve a satisfying initial value
or derive the first satisfying 16-bit value. It cannot borrow a body assignment,
another actor's field, or a disabled procedure. The selected guard, before/after
value, and save-byte changes are reported; the verifier checks the initial
actor value in the native trace. Item acquisition remains explicitly staged.

The fixtures derive values 0, 101, and 501. All six passed offer/transfer and
lossless-media verification. The four middle/high response sites fully reveal;
each plan also asserts the alternate response's absence. The low plans assert
both responses absent, not that the script did nothing or that those sites
are globally unreachable. Values exactly 100 and 500 satisfy neither spoken
response guard; binding tests cover those boundaries and signed negative words.
These are prepared branch captures, not complete gameplay routes or full
original-executable conversation comparisons.

Bronko's SCRIPT2 energy fixture uses Moskito's `usine` destination and procedure
23683. The seven generated topic captures publish 21 BAS sites and three COD
sites with full native UI-buffer reveals. Nine of the actor list's 30 BAS sites
remain unrecorded. For a native menu that reopens after goodbye, `max_exit_retries`
allows a bounded repetition of only the last authored BAS `bye_bye` choice (at
most eight). Every repetition must still match the current source menu and
offered word; the exporter never forces presentation closure. The buy and war
chapters each needed one repetition, retained in their choice traces.

Generated topic titles include the menu path when different submenus reuse a
word, as in Yoko's race-specific `news` and `brain` topics. `--entry-menu` selects
the authored BAS menu body from which to plan, not a runtime menu override.
Izwalito's `cor4bis` procedure explicitly sets `topic = "talk"`, selecting menu
5915 rather than the list's first `talk1` menu. The native renderer still checks
the actual menu at every selection.

The checked-in Gluxx treatment plan includes three treatment selections: the
first advances the unconditional youth line before the two treatment replies.
One selection per statically matched reply is not a general scheduling rule.
Izwalito's ideal plans retain the BAS goodbye before the COD secret question,
then answer it and close the resumed BAS menu. Merely waiting on that menu does
not advance the question. The accepted answer does not establish coverage of
the `know` submenu; it remains unrecorded in these plans.

`expected_unpublished_cod_sites` asserts absence on a selected branch, not global
unreachability. The CB Bob chapters preserve the observed `AARCHE10`, `AARCHE30`,
and `AARCHE40` order; they do not insert `AARCHE20` or extend the empty A6. Original
CB A6, outer-loop, and string-loader boundary oracles were rechecked (35 cases);
these are not an original-executable trace of the complete Bob conversation.

The native frame/PIT/audio clocks remain active. Captures use the game's own
artwork and fonts, and BBB's existing English localization. The verifier decodes
every RGBA frame and float PCM sample, checks every timestamp, rejects pointer
button input, and records partial/full native UI glyph reveals. UI audits occur
before presentation and must not alone be described as encoded visibility.
