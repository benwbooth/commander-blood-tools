# Big Bug Bang live regressions

Reported September 19, 2026 during ordinary play. These reports are not a
whole-game fidelity acceptance test.

## Bridge hand jitter while moving the mouse

The native steering step consumes horizontal motion and recenters the logical
pointer as the panorama turns. Render-only host refreshes were publishing raw
mouse coordinates back into authoritative input, then projecting the hand at
those coordinates against the unchanged panorama. The next game tick could
recenter it again.

Render-only refreshes now read host coordinates without publishing game input.
While the bridge owns the pointer and its retained frame is scrolling, the
hand keeps the game-selected horizontal anchor. Vertical motion remains live;
menus and non-scrolling frames retain both live pointer axes. The recovered
steering and animation algorithms are unchanged.

Focused regression tests cover left/right scrolling with multiple intervening
mouse deltas and unrestricted movement outside scrolling. The drift test failed
with the previous coordinate selection and passed after the correction. Live
confirmation with the user's moving mouse remains pending.

## Daddy Glux background colors

The user reports correct initial colors, followed by persistent wrong colors
after the small-character interlude. The authored `gengl` dialogue can play
`PPIT07.HNM`; another branch precedes it with `FLITUTR.HNM`. The latter changes
colors used by the retained cryobox background. The old presentation path
re-resolved the entire indexed page after a clip-local color change, including
pixels that the clip had not written.

Presentation backgrounds are now resolved to RGB with their own colors when
loaded. Contact returns restore the cached `FRIGO.FD` RGB artwork: they must not
decode it again using a just-finished interlude's color context. The decoder tracks
exact write coverage, including same-index writes and transparent AD rectangles,
and may update only its own video pixels. Back-page copies, clip switches, and
retained end frames carry RGB pixels instead of reconstructing inherited artwork
from the next clip's color state. Contact darkening is an explicit RGB layer
effect on the recovered transition clock.

The original-asset regression plays `CRYOGEL`, `GLUXROUG`, `FLITUTR`, `PPIT07`,
and `GLUXROUG` with the cryobox artwork. It checks retained and displayed
background pixels at every serviced frame, verifies that the interlude really
changes decoder colors, and demonstrates that the previous full-page conversion
would recolor those pixels. A second original-asset test exercises the player
lifecycle, including `GLU00`, `GLU01`, and the contact-image reload after each
interlude. That reload was missing from the first focused test; adding it
reproduced the pale-background failure. Focused tests also cover transparent and
opaque zero, same-index writes, prepared-background handoff, external back-page
replacement, and nonaccumulating RGB darkening.

Verification: the clean library suite passes 1,086 tests, with 66 optional tests
ignored. Both original-asset regressions above pass when run separately; the
static-owner and disposition checks pass 6 and 4 tests. Both game executables
build, and the Nix release build succeeds.

The isolated release replay at
`output/fidelity/bbb-rgb-daddy-tempest-20260919-v5` returns from `FLITUTR` and the
following `PPIT07` without recoloring the background. Screens 020, 027, and 030
retain an identical 960-by-45 top-background crop, with pixel signature
`f117cc6d18066f3816c3da0048b2dc0a70c478d03d19d1e3b23187220bc402bb`.
The earlier v3/v4 diagnostic replays still reproduced the failure and are not
fix evidence. This is not a claim that every legacy rendering path has been
migrated to RGB assets.

The user has no save. A fresh isolated September 19 modern replay reached the
first `PPIT07.HNM` interlude and returned to Daddy with the same background
colors (entries 128 through 191). Their raw RGB SHA-256 before and after was
`52d3da1bf8e618329c36f4a39958a83bbaedfb5fd32a9388b8a28231e821bdf3`.
Artifacts are under `output/fidelity/bbb-daddy-palette-fresh-20260919`.

An earlier ordinary-play capture under
`output/big-bug-bang/english-daddy-tempest-01` shows the pale-blue background
appearing after `FLITUTR.HNM` and persisting through the next `PPIT07.HNM`.
The authored dialogue says the screen was damaged at this point. Private DOS
replays have not reached that later boundary, so those captures do not prove
original-executable equivalence. The correction follows the requested RGB
ownership policy rather than preserving shared-DAC recoloring of loaded artwork.

## Cryobox music retained on Loviland's surface

The user confirmed the surface view, not orbit, and has no saved checkpoint.
`DESCRIPT.descript` assigns `VOL.VOC` to Loviland. Navigation can select a new
description before the HUD consumes it, and repeated selection reports the name
as reused. `ensure_navigation_music` previously kept any playing stream without
checking whether it contained the selected track.

The runtime now retains the identity of the music actually loaded into the
shared stream. Ensuring music reloads a different selection, but does not restart
the matching playing track. Voice loads, discarded pending streams, and audio
enable-state changes invalidate that identity. Runtime traces include the loaded
name separately from the DESCRIPT selection.

The real-asset regression
`loviland_music_replaces_a_playing_track_after_description_selection` failed
before the fix because the actual stream payload still contained the preceding
music. It passes after the fix and checks the payload against normalized
`VOL.VOC`, along with uninterrupted matching playback, pending replacement,
audio disable/re-enable, and shared voice-stream replacement. This is a concrete
SDL/service handoff test, not yet an end-to-end replay of the user's travel route.

## Navigation overview crash with Daddy aboard

The September 19 game process exited while updating the sequel overview shortly
before the user selected a planet. A new original-asset regression reproduces a
failure on the first initialized SCRIPT2 frame: Daddy_Gluxx has the valid
`aboard` holder sentinel (`0xffff`) and is not a map participant. The adapter
resolved every actor's holder before applying the native roster filters, so it
incorrectly treated this state as a fatal malformed relation.

The adapter now checks actor participation and the raw state-header bit before
resolving holders, then checks holder participation before resolving positions
and opponents. This follows the native roster builders at `0x6FF2`/`0x706E`.
Eligible actors with invalid relations still fail explicitly. Navigation host
errors also retain their underlying causes instead of losing them at the generic
camera-error boundary. The original incident log lacked that underlying cause;
the asset-backed failure establishes this defect, not every possible crash cause.

Verification: 1,087 library tests pass with 67 optional tests ignored. Both
overview asset tests pass separately, covering all 17 raw profiles plus 20
initialized updates and overview rendering per profile. The initialized test
failed before the fix on Daddy's holder sentinel and also checks that forcing
that actor to participate still reports an invalid relation. All 32 original
executable overview cases were regenerated and match the checked-in vectors.
The clean Nix release builds successfully. The isolated UI replay at
`output/fidelity/bbb-overview-fix-20260919` did not reach its intended loaded
navigation state and was stopped; it is not end-to-end planet-selection proof.

## Left-click video and dialogue skipping

BBB now treats a fresh left-click during a video or non-choice dialogue as a
request to skip the current segment. This is an intentional modern-runtime
extension, not a claim about original DOS input behavior. The recovered native
right-click and Escape policies are unchanged, as is Commander Blood behavior.

The ordinary lifecycle and the separate opening/credits runner both handle the
shortcut. Skipping releases the current video source and lets its existing scene
coordinator perform completion; it does not bypass story callbacks or auto-select
dialogue responses. Text-only holds release the VM immediately. Consumed pointer
latches cannot select a newly exposed control, and holding the button cannot skip
subsequent segments. Word choices, inventory, bridge menus, and confirmation or
save/load dialogs retain their normal input. Foreground and streamed speech stop;
the selected background music is retained.

Verification: 1,089 library tests pass with 68 optional tests ignored. The new
original-asset SDL/service test passes separately for GLUXROUG, FLITUTR, PPIT07,
opening and credits streams, normal scene completion, text-state synchronization,
held-click rejection, music retention, and stopped speech. Policy tests cover all
45 presentation line IDs, choices, paused input, and the Commander Blood boundary.
All six static-owner audit checks pass. Both debug game binaries and the Nix
release build succeed. The user's already-running process is not restarted.

The isolated `bbb_click_skip_opening.tsv` replay changes from 149 recorded movie
frames with the old build (no transition before scenario shutdown) to 20 movie
frames followed by 230 game frames with the new build. Captures are under
`output/fidelity/bbb-click-skip-opening-before-20260919` and
`output/fidelity/bbb-click-skip-opening-20260919`. The additional
`output/fidelity/bbb-click-skip-story-20260919` replay consumes four in-game clicks
and advances the current clips through the normal presentation panel. It covers
startup sequences, not an exhaustive playthrough of every authored conversation.

## Navigation ball immediately closes the star chart

Opening the camera also requests BBB's simulation overlay. In early SCRIPT2 the
overlay has no participating groups. Its native `ClosedEmpty` result arms the
camera actor with flags `9`, starting another animation and closing the chart
without further input. The real-asset regression reproduced activation at frame
16 followed by an unrequested close at frame 33.

The modern runtime now dismisses an empty overlay without arming that second
animation. The base chart remains available for location selection, and the
camera actor remains clickable. This is an explicit usability fallback in the
runtime adapter; the native overview controller and its original-executable
vectors remain unchanged. Nonempty overlays keep their existing behavior.

The original-asset SDL/service regression
`sequel_navigation_ball_keeps_empty_star_chart_open` initializes authentic
SCRIPT2 and an idle bridge, then drives real ball hit-testing, actor animation,
pointer edges, and chart updates. It checks 100 frames after each of three
clicks (open, close, reopen), then selects a celestial destination and opens its
location panel. It passes with the fallback; the single-click portion fails
without it. This is an isolated runtime test, not a complete story playthrough.
All 1,089 regular library tests pass, with 69 optional tests ignored; the new
asset regression was run explicitly. The six static-audit and four disposition
checks also pass after refreshing the source hashes.

## Clip skipping swallowed startup intro dismissal

The generic click shortcut ran before the presentation panel and cleared the
primary latch. For the startup robot intro, that made each click finish one HNM
clip while preventing the panel's existing reverse-close path from receiving
input. The earlier skip tests checked individual scene completion, not dismissal
of the entire intro. The earlier startup click replay is evidence of this
regression, not proof of correct intro controls.

The shortcut now yields clicks whenever the ordinary TV panel owns input. The
panel handles startup dismissal and normal channel selection itself; standalone
opening/credits playback and dialogue outside the panel retain click-to-skip.
No recovered native input or panel state machine was changed.

`sequel_intro_panel_click_minimizes_instead_of_skipping_one_clip` opens the
authentic startup scene list, sends one click through the shortcut and panel,
and checks cancellation, all closing phases, and that no subsequent clip loads.
It fails at the swallowed-click assertion without the new ownership guard and
passes with it. All six sequel service tests, including real-media skips and
navigation close/reopen, pass explicitly. The regular library suite passes
1,089 tests with 70 optional tests ignored. Both debug and release binaries build;
six static-audit and four disposition checks pass.

The isolated `bbb_intro_panel_dismiss.tsv` replay skips the standalone opening
after 20 recorded movie frames, then clicks the startup video at game frame 160.
Before the fix it starts another clip and retains the panel through frame 300.
With the fix it enters closing phase 106, finalizes at frame 167, and remains
closed with no video resource through frame 300. The replay reaches the robot
video (`ppit09.hnm`, followed by `ppit06.hnm`) before dismissal and exits normally.
Traces and screenshots are in `output/fidelity/bbb-intro-dismiss-before-20260919`
and `output/fidelity/bbb-intro-dismiss-fixed-20260919`.

## Bridge ambience and Tempest navigation RGB handoff

The bridge loop `mu\\tablo2.voc` shares the native VOC stream with music and
standalone voice. It has no DESCRIPT music identity. The modern click shortcut
incorrectly treated absence of that identity as proof that the stream was speech,
so dismissing dialogue could stop the bridge background. Stream purpose is now
explicit: navigation music, bridge ambience, or voice. Only voice is stopped by
dialogue dismissal. The real-asset click regression covers TABLO2 continuity as
well as music preservation and standalone-voice cancellation. Trace audio now
includes stream purpose, actual playback position, and pending-start state.

The reported normal phone-ending music loss was not reproduced in the ordinary
Daddy replay: TABLO2 playback continued after the conversation. The click bug is
confirmed separately, not proof of every possible phone-ending failure.

Tempest's standalone HNM conversion is independent of preceding colors, but the
navigation frame owner restored only its indexed back buffer. Its already
imported RGB PBM page was never published. Navigation now presents that RGB page
at the same boundary as the native back-buffer restore. The authentic-asset
`sequel_tempest_navigation_presents_imported_rgb_after_palette_changes` test
failed before this handoff, then passed with every nontransparent PBM pixel
matching the imported colors across unrelated red, green, and blue game palettes.
The separate `tempest_landing_colors_do_not_depend_on_the_preceding_scene` test
compares every video frame in the landing band under two starting color states.

The full replay also exposed missing RGB population-panel styles. BBB uses 98,
252, and 96 in addition to the original title/source styles 238 and 254. Both
the glyph importer and stat-bar color lookup now support the complete set.
Previously the populated Tempest panel raised `unknown location-panel text style`
and then, after the glyph-only repair, `unknown dialogue UI color`. The font
regression checks the independently enumerated authored styles, glyph coverage,
widths, and the same RGB colors used by the bars.

The next full replay reached the travel button but failed with
`MissingInitialTarget`. At frame 9101 a C1 destination command was queued while
the VM remained paused; at the subsequent HUD reset Arche still referred to its
startup placeholder. The BBB hyperjump adapter now resumes the VM when it queues
that command. The existing real-input chart regression was extended through
destination selection and travel with an explicitly paused VM. It fails without
the resumption, and passes after checking the updated navigation target. Native
hyperjump planning and Commander Blood behavior are unchanged.

The final private-display replay of
`accuracy/scenarios/bbb_phone_then_tempest_landing.tsv` exits successfully at
frame 10909. It plays `PL\\tempet10.hnm` at frames 9104-9250, then
`PE\\gluxpla.hnm` and the destination contact scene. Screenshots 069-070 show
the landing and its destination handoff. The trace retains active bridge
ambience after the phone conversation and switches to navigation music during
landing. Evidence is in `output/fidelity/bbb-phone-landing-20260919-v4`.
This replay verifies that route's runtime handoffs, not original-executable
pixel parity or every possible phone-ending audio path.

Verification: 1,089 regular library tests passed (72 optional tests ignored),
all seven sequel service tests passed explicitly in private SDL/wgpu, the
Tempest video test passed explicitly, and all six static-audit plus four
disposition checks passed. Debug and release builds succeeded. These are
targeted runtime/RGB ownership fixes, not a completed whole-game RGB migration.

## Hand ownership after skipping an empty Tempest landing

The landing skip correctly closed the video and opened the empty-location exit
list, but the retained RGB page still hid MANU3. The display-ownership gate
recognized ship-target and dialogue menus, not the navigation exit list. An
active ship presentation therefore continued to count as video-owned even with
no video source left. The faulty gate was based on ship ownership, not whether
the video had been skipped.

The active navigation exit list now returns pointer ownership when no video is
open or draining. A pending camera approach still suppresses the hand. The
native skip, scene-completion, navigation, and click-selection logic is unchanged.

The original-asset service test
`sequel_skipped_tempest_landing_returns_the_hand_and_exit_interaction` failed
before the fix at its missing-hand assertion. It now verifies that the arrival
clip hides the hand, skipping restores it over the empty-location list, a fresh
exit click is not consumed as a video skip, and navigation closes to the bridge.

The full input replay `accuracy/scenarios/bbb_tempest_landing_skip.tsv` starts
PLAY without sending Daddy to Templand. In the old build, frame 2295 has the
Tempest clip active, the click stops it by frame 2301, and the hand remains
suppressed through frame 3652 with the exit label visible. Captures and traces
are in `output/fidelity/bbb-empty-tempest-skip-before-20260919`.

The fixed full replay exits normally at frame 4108. Its frame 2301 already
allows the hand and submits 107 MANU3 triangles. The hand remains visible over
the empty planet, and the exit click changes ship mode to inactive by frame
3658. Screenshots 023 and 029 show the hand over the planet and the subsequent
bridge view. Evidence is in
`output/fidelity/bbb-empty-tempest-skip-fixed-20260919`.

Verification: all 1,089 regular library tests pass (73 optional tests ignored),
all eight sequel service regressions pass explicitly, and all six static-audit
plus four disposition checks pass. Debug and release game binaries build.
