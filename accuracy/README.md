# Accuracy Oracle (DOSBox-X)

Ground-truth capture of the **real game** so generated cutscenes can be compared
against it (goal step 3). Runs the game in DOSBox-X on an **isolated Xvfb virtual
display** — it never touches the user's desktop.

## Run

```sh
nix develop --command bash accuracy/run_oracle.sh [seconds]
```

Captures land in `accuracy/captures/frame_NN.png` (gitignored — game content).

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
3. Build a comparison harness: our exporter's mp4 vs the captured reference —
   subtitle text/timing, voice/chatter, animation, background, music.

A green "1" overlay appears top-left during the intro (likely a scene/debug
index) — worth checking whether `BLOODPRG.EXE` has a script/scene-select debug
mode that would make reaching specific scenes trivial.
