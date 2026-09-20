# Native Dialogue Chapters

These plans select branches of the COD/BAS dialogue trees. They bind the
original COD/DIC bytes (and BAS bytes when used) and address choices by source
offset and DIC word offset, not screen coordinates. `required_frame_boundary_cod_sites` checks
retained script state, not visible pixels. The batch report separately records
glyph-buffer evidence and lines with no full reveal.

Current scope: CB SCRIPT1 Izwalito's game/explanations choices and Bob's mission
yes/no choices; BBB SCRIPT1 HONK's PLAY/INSTRUCTIONS choices and Bob's recorded
mission; BBB SCRIPT2 HONK's Daddy-in-the-cryobox conversation; CB SCRIPT2 Bob's
black-hole BAS topic, Bronko's energy topic, Daddy Gluxx's treatment, and both
answers to Izwalito's ideal/secret question through prepared travel. These thirteen
checked-in plans do not cover all actors, dialogue branches, objects, travel, or
environments. The static BAS planner also
produces topic plan sets from the hashed catalog without running the game.

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
binary-derived contact manifest. It validates the profile and actor, sets authored
entry predicates, and disables competing contact procedures. The report retains
the manifest hash, original-save hashes before/after, and every changed save byte.
This is explicit prepared-state chapter setup, not proof of contact-menu
eligibility or a gameplay route. It does not modify script text, timing, or media.

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
