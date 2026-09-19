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
