# Native Dialogue Chapters

These plans select branches of the COD dialogue trees. They bind the
original COD/DIC bytes and address choices by the A6 source offset and DIC word
offset, not screen coordinates. `required_frame_boundary_cod_sites` checks
retained script state, not visible pixels. The batch report separately records
glyph-buffer evidence and lines with no full reveal.

Current scope: CB SCRIPT1 Izwalito's game/explanations choices and Bob's mission
yes/no choices; BBB SCRIPT1 HONK's PLAY/INSTRUCTIONS choices and Bob's recorded
mission; BBB SCRIPT2 HONK's Daddy-in-the-cryobox conversation. These eight plans
do not cover the other actors, dialogue branches, objects, travel, or environments.

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
and choices currently refer only to the selected profile's COD sites, not BAS
or a chain of different profile-local conversations.

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
