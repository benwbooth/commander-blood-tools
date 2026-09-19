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

The isolated September 19 replay did not reach this boundary before stopping.
Request the affected save and screenshot, then compare the resource sequence
and displayed background with the original executable. Keep video-local RGB
ownership intact while tracing the inherited scene colors.

## Cryobox music retained after travel to Lovia

Unresolved. `DESCRIPT.descript` assigns `VOL.VOC` to Loviland, while Lovia itself
has no music field. Confirm whether the report concerns orbit/bridge or the
surface location; then compare selected DESCRIPT music with actual playback.
No music behavior has been changed yet.
