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
