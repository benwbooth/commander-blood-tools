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

Unresolved. The user reports correct initial colors, followed by persistent
wrong colors after the small-character interlude. The authored `gengl` dialogue
can play `PPIT07.HNM`; another branch precedes it with `FLITUTR.HNM`. Their palette
updates differ, so reproducing the specific sequence matters. No palette change
has been made on the strength of this report alone.

The user has no save. A fresh isolated September 19 modern replay reached the
first `PPIT07.HNM` interlude and returned to Daddy with the same background
colors (entries 128 through 191). Their raw RGB SHA-256 before and after was
`52d3da1bf8e618329c36f4a39958a83bbaedfb5fd32a9388b8a28231e821bdf3`.
Artifacts are under `output/fidelity/bbb-daddy-palette-fresh-20260919`.

An earlier ordinary-play capture under
`output/big-bug-bang/english-daddy-tempest-01` shows the pale-blue background
appearing after `FLITUTR.HNM` and persisting through the next `PPIT07.HNM`.
The authored dialogue says the screen was damaged at this point. The user has
been asked whether that screenshot matches the reported appearance. Private
DOS replays have not yet reached this later boundary, so this is not proof
that the persistent color change matches the original executable. Keep
video-local RGB ownership intact while completing that comparison.

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
