# Accuracy Oracle (DOSBox-X)

Ground-truth capture of the **real game** so generated cutscenes can be compared
against it (goal step 3). Runs the game in DOSBox-X on an **isolated Xvfb virtual
display** — it never touches the user's desktop.

## Run

```sh
nix develop --command bash accuracy/run_oracle.sh [seconds]
```

Captures land in `accuracy/captures/frame_NN.png` (gitignored — game content).

## Compare One Generated Frame

```sh
nix develop --command python accuracy/compare_oracle.py \
  --reference accuracy/captures/frame_12.png \
  --generated "output/mp4/executed-dialogue-run - script2 - 0001 - pterra.mp4" \
  --generated-time 0
```

The comparison output lands under `accuracy/comparisons/` (gitignored):

- `reference-native.png`: DOSBox capture cropped to the game viewport and scaled
  back to native 320x200.
- `generated-native.png`: generated MP4 frame scaled to native 320x200.
- `diff-x4.png`: amplified visual difference image.
- `comparison.json`: repeatable metrics (`mean_abs`, `rmse`, `max_abs`,
  `exact_pixel_percent`, crop, input paths, and `regions`).

`regions` splits the native 320x200 frame by recovered presentation bands:
`top_bar` (`y=0..34`), `scene_band` (`y=35..164`), `hud_panel`
(`y=165..193`), and `bottom_bar` (`y=194..199`). This separates wrong
background/foreground/subtitle failures from missing ship-HUD failures.

Current `run_oracle.sh` screenshots are 800x600 host-window grabs. The compare
script therefore defaults to the measured DOSBox viewport crop
`80,100,640,480`; override with `--ref-crop x,y,w,h` if the capture setup
changes. Use `--max-mean-abs` only for a scenario known to be frame-aligned with
the generated output.

## Run Named Scenarios

```sh
nix develop --command python accuracy/compare_oracle.py \
  --scenario-file accuracy/oracle-scenarios.tsv
```

`accuracy/oracle-scenarios.tsv` is the checked-in list of repeatable comparison
targets. Blank `max_mean_abs` values record metrics as `unchecked` without making
the batch fail; fill in a threshold only after the generated frame is known to be
frame-aligned with the DOS capture.

Scenarios may set `scan_start`, `scan_end`, and `scan_step` to search a generated
MP4 window and save the best matching frame as the scenario comparison. The scan
writes `scan.json` next to `comparison.json`; this is useful for proving whether
a mismatch is just timestamp alignment or the wrong scene/presentation state.

Single comparisons can use the same scanner:

```sh
nix develop --command python accuracy/compare_oracle.py \
  --reference accuracy/captures/frame_12.png \
  --generated "output/mp4/executed-dialogue-run - script2 - 0001 - pterra.mp4" \
  --scan-generated 0:12:1
```

## Search Candidate Videos

Before a DOS capture is promoted to a thresholded scenario, find the generated
video that is closest to the reference frame:

```sh
nix develop --command python accuracy/compare_oracle.py \
  --reference accuracy/captures/frame_12.png \
  --candidate-glob "output/mp4/executed-dialogue-run*.mp4" \
  --scan-generated 0:12:2 \
  --out-dir accuracy/comparisons/frame12-candidate-search
```

Candidate search writes `candidate-search.json` with ranked matches and saves a
normal comparison for the best match under `<out-dir>/best/`. A high best
`mean_abs` after scanning means the DOS capture is probably a different scene or
presentation state, not merely a frame-offset problem.

## What works (verified 2026-06-14)

- `accuracy/dosbox.conf` mounts the CD image (`output/CMDR_BLOOD.iso`) as `D:`,
  a writable `accuracy/cdrive` as `C:`, with `svga_s3` / `cputype=auto` /
  `cycles=max`, EMS + XMS enabled (the game requires both).
- **`BLOOD.EXE` is only the installer/launcher** ("Previously stored
  configuration has not been found on drive C:" → "I want to start a new
  installation"). It expects an HD install of `BLOOD.BAT`.
- **`BLOODPRG.EXE` runs directly** (bypassing the installer) and plays the game:
  the autoexec `cd D:` + `BLOODPRG.EXE` boots straight into the **CRYO 1995 intro
  cutscene** (HNM background + animated character + the square-stroke subtitle
  font recovered in `re/REVERSE.md`). Frames change over time → real playback.
- Capture: `import -window root` against the Xvfb display grabs the DOSBox-X
  framebuffer (no window manager needed). For pixel-exact 320×200 frames, switch
  to DOSBox-X native screenshots (mapper key) in a later iteration.

## Next steps toward scene-by-scene validation

1. Drive input (menu/scene navigation) — add `xdotool` to the flake for scripted
   key/mouse events on the Xvfb display, or use DOSBox-X `autotype` for keyboard.
2. Reach the 5 target scenes (Bob_Morlock, Izwalito, a multi-character scene, a
   subtitle-only screen, a full HNM cutscene); capture frame + audio per scene.
3. Extend the comparison harness from frame metrics to scene checks: subtitle
   text/timing, voice/chatter, animation, background, and music.

A green "1" overlay appears top-left during the intro (likely a scene/debug
index) — worth checking whether `BLOODPRG.EXE` has a script/scene-select debug
mode that would make reaching specific scenes trivial.
