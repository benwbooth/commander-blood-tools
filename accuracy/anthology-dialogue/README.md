# Native Dialogue Chapters

These plans select branches of the initial COD dialogue trees. They bind the
original COD/DIC bytes and address choices by the A6 source offset and DIC word
offset, not screen coordinates. `required_frame_boundary_cod_sites` checks
retained script state, not visible pixels. The batch report separately records
glyph-buffer evidence and lines with no full reveal.

Current scope: CB SCRIPT1 Izwalito's game/explanations choices and BBB SCRIPT1
HONK's PLAY/INSTRUCTIONS choices. Other profiles, actors, dialogue branches,
objects, travel, and environment chapters are not covered by these four plans.

## Native Omissions

Do not add artificial holds or subtitle cards to fill these gaps:

| Game/branch | COD offset (decimal) | Observed disposition |
| --- | ---: | --- |
| CB game | 1206 | Published in the final frame, without a glyph-buffer draw before profile handoff. |
| CB explanations | 1552 | Published in the final frame, without a glyph-buffer draw before presentation termination. |
| BBB PLAY | 1860 | Published then replaced by profile loading within the same frame; not retained at frame end. |
| BBB INSTRUCTIONS | 2355 | Never published on this branch: the preceding A6 at 2317 arms skip-next=1 even when inactive on a subsequent VM pass. |
| BBB INSTRUCTIONS | 2371 | Published without a glyph-buffer draw before presentation termination. |

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
loads the original radio sound bank, and enters the requested typed C4 radio
presentation. Semantic word selection uses the native choice widget, including
its selected frame, hand request, sound, and close phases. It is not a gameplay
route or mouse-input simulation. Only initial-profile radio entry is currently
supported; other profiles are explicitly rejected.

The native frame/PIT/audio clocks remain active. Captures use the game's own
artwork and fonts, and BBB's existing English localization. The verifier decodes
every RGBA frame and float PCM sample, checks every timestamp, rejects pointer
button input, and records partial/full native UI glyph reveals. UI audits occur
before presentation and must not alone be described as encoded visibility.
