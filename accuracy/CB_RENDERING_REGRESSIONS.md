# Commander Blood rendering regressions

Commander Blood fixes take priority over further Big Bug Bang progression work.
These five reported symptoms are not all resolved.

## Password hand

The original MANU3 dispatcher at file 0x001610 uses presentation-mode,
request-delay, and selector gates. It does not suppress the hand merely because
a character idle video is playing. The additional Rust display-ownership gate
in runtime/services.rs suppressed the active Pterra identity-code chooser.

The chooser now bypasses that additional occlusion gate, while the native
dispatcher still controls whether a hand frame is generated. The Pterra
production test now requires hand geometry at the chooser and continues to
reject hand geometry over noninteractive movies. It passes with real assets.

## Retained video pixels

runtime/video.rs previously recaptured the shared front work page even when
queue service presented no frame. The palette-fade path in runtime/services.rs
also recaptured that page on every tick. Both paths could replace a retained
video image with unrelated work-buffer pixels.

Palette refresh now recolors owned pixels only. Queue service replaces them
only when it presents a frame. Two real-resource regressions cover a modified
work page during a color refresh and a timer-gated Scruter idle queue. The first
test failed before the correction. Presentation-player and video tests pass,
including the existing DOS opening-frame hashes.

This establishes a corruption-path correction, not yet proof that the reported
one-frame Scruter transition flash and overwritten black bars are both gone.
Inspect clip-switch boundaries next. The explicit ship-depth copy at 0x00B6DD
also updated only the indexed work page. Its two destination row bands are now
copied into an owned video page without replacing the center. A real-resource
regression checks both active and finished video ownership.

## Pterra map colors

Original assembly 0x0090F6 selects GS:0x5F11 for the remap built with -50 and
black at location-panel initialization. At 0x009152 it reads DS:0x0AC8; the
shipped executable's initialized word at file 0xDEE8 is also 0x5F11. The C
candidate palette_blend_remap_table_build at 0x0022E0 searches the live palette
for nearest colors, including last-index tie handling.

The modern location panel instead halved RGB channels. Imported planet artwork
now retains the native remapped RGB variant, used inside the panel's remapped
rectangle. Scaling, source-zero transparency, and clipping are unchanged.
The artwork test checks all 42 layout entries at five placements/scales against
native indexed scaling and palette remapping. The live navigation-chart and
travel-to-password scenarios pass. A screenshot of the password chooser in
`output/fidelity/production-load-pterra-ship-navigation.jsonl-1788834458686223589-2077237-0/screen-28.png`
visibly confirms the hand; this capture predates the explicit band-write fix.

## Lever and black bars

The precise reported visual symptoms still need verification. The hyperjump
handler at 0x007FB9 writes zero to DS:0x0A34, the
current MANU3 selector. Thus the Rust hand restart is not by itself a translation
discrepancy. The user confirmed the travel lever. It remains unclear whether
the reported bars are inside the 320x200 scene or outside the aspect-fit
viewport. Do not mask these issues by globally hiding layers.

The offscreen UI-composite test previously fitted 320x200 instead of the
production renderer's 4:3 display. Its helper now uses the production aspect
constants. A new GPU test poisons the entire base texture yellow and uploads
an opaque magenta overlay, then checks every pixel outside the viewport is
black with the overlay enabled and disabled. It covers 640x360, 256x384, and
640x481 surfaces, including resize and fractional viewport boundaries. All
15 renderer tests pass, including original hand, bridge, and alien bounds.
No outer-bar overwrite was reproduced. The user has been asked whether the
reported bars are inside the game image or outside its viewport; that remains
necessary context for locating any persisting overwrite.

The modern MANU3 between-tick renderer predicted an ordinary next tween step
even immediately after an explicit animation selection. Repeated native lever
selection can invalidate that prediction. It now reprojects the current pose
for such intervals, retaining pointer tracking without speculative tweening.
Ordinary no-selector intervals still interpolate. The repeated-selector
regression fails with the guard disabled and passes with it enabled; all 22
MANU3 tests, including original-binary semantic vectors, pass. The desktop
executable was rebuilt; subsequent visual verification is described below.

The user subsequently confirmed the travel lever. The rebuilt executable's
full navigation scenario passed in 229.54 seconds. Its capture is
`output/fidelity/cb-travel-lever-413e16ec.mkv`; the short pull excerpt is
`output/fidelity/cb-travel-lever-pull.mp4`. At trace frames 2136-2143 the lever
advances from 1 through 8 without skipping, with hand selector 10 throughout.
The 60-fps contact sheet `output/fidelity/cb-travel-lever-pull.png` shows held
native poses between simulation ticks during the repeated selector interval,
rather than speculative between-tick tweening. Hyperspace and bridge return
also pass. This verifies the interpolation correction on the reported control;
an exact original/live animation comparison is still absent.

The zoomed Pterra view from that build is captured at
`output/fidelity/cb-pterra-zoom-after.png`. The scenario's trace directory is
`output/fidelity/production-load-pterra-navigation.jsonl-1788837484936872489-2469590-0`.

A 150-second, 60-fps lossless X11 capture is available at
`output/fidelity/cb-scruter-continuity-20260907-01/display-attached.mkv`.
It predates the interpolation correction and includes the password chooser.
The initial contact-sheet inspection does not establish exact talking-clip
boundary parity; the reported Scruter flash remains open.

### Scruter talking-boundary isolation

The continuous-capture trace at
`output/fidelity/production-load-pterra-ship-navigation.jsonl-1788836195036749096-2273077-0/frames.jsonl`
includes `PE\\scr02.hnm` talking clips. Frames 2755-2757 contain decoded count
30, the closed-source retained page, then the next clip's first frame.
The palette hash remains `72c3fee4af276d22` across that boundary. Video ownership
stays true and submitted hand triangles stay zero. A changed final image hash
is not alone a flash: the authored final frame changes the green waveform.

Rebuilt `hnm-corpus-trace` and reran `compare_hnm_decoder_corpus.py` on the
imported `PE/SCR02.HNM`: ordinary decoding checks 31 frames, `--rect` checks
31 frames, and `--palette` checks one record, all with zero mismatches against
the original executable. Destination-history checks also report zero sensitive
frames. These are isolated original-code gates, not full runtime parity.

`scruter_talking_clip_close_preserves_the_authored_final_frame` now checks that
the player presents all 31 frames, retains the distinct final frame on close,
and preserves it and its palette after poisoning the work page and refreshing
colors. All 11 presentation-player tests pass. Remaining investigation is
runtime composition and timing, with synchronized original/live frame evidence
needed before declaring the reported flash resolved.

Subsequent 60-fps inspection of the continuous capture at 50.0-51.07 seconds
(talk to idle) and 55.2-56.27 seconds (talk to talk) shows no blank page, stray
hand, or overwritten bars. Contact sheets are `talk-boundary-50.png` and
`talk-boundary-55.png` beside the recording. The talk-to-talk boundary does
reset to the authored first pose. These captured transitions are visually clean
after the retained-buffer correction; other dialogue paths are not certified.

## Verification setup

Use SDL_VIDEODRIVER=x11 with Xvfb for screenshots. Inheriting WAYLAND_DISPLAY
can run the game on Wayland even when DISPLAY points at Xvfb, producing empty
Xvfb screenshots. Empty captures are not rendering evidence.

The final runtime suite reports 263 passed and 16 ignored. The final-band build's
X11 capture is under
`output/fidelity/production-load-pterra-ship-navigation.jsonl-1788834742610631001-2082439-0`.
`screen-18.png` visibly retains the password hand and the video's upper/lower
black bands. This is a scoped visual check, not a frame-by-frame conversation
or lever-animation parity claim.
