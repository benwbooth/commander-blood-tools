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
discrepancy. Determine whether the report concerns the hyperjump lever or the
camera/map control, and whether the bars are inside the 320x200 scene or outside
the aspect-fit viewport. Do not mask these issues by globally hiding layers.

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
