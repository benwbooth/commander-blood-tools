#!/usr/bin/env bash
# Run the original Commander Blood (BLOODPRG.EXE) headlessly under DOSBox-X on a
# virtual framebuffer (Xvfb) and capture frames of its boot/attract sequence. This
# is the ground-truth "oracle" for verifying the Rust reimplementation's decoders
# and engine against the actual game — a stronger reference than the YouTube
# playthrough because it is the game itself, reproducibly, in this environment.
#
# Requires the nix devShell (provides dosbox-x, Xvfb via xorg-server, imagemagick,
# and the graphics runtime libs on LD_LIBRARY_PATH — see flake.nix). Run as:
#   nix develop --command re/tools/capture_real_game.sh <game-dir> <out-dir> [display-num] [install-parent] [executable]
# where <game-dir> holds BLOODPRG.EXE + assets (e.g. output/_tmp_iso).
#
# Why this works when winit/minifb did not: DOSBox-X uses SDL, which — like the
# engine's x11rb backend — talks to the X server directly; with the graphics libs on
# LD_LIBRARY_PATH it renders fine under Xvfb. Captured frames land as boot_<t>s.png.
set -euo pipefail

GAME_DIR="$(realpath "${1:?usage: capture_real_game.sh <game-dir> <out-dir> [display]}")"
OUT_DIR="${2:?missing out-dir}"
DISP="${3:-:83}"
mkdir -p "$OUT_DIR"

export DISPLAY="$DISP" SDL_VIDEODRIVER=x11
Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
trap 'kill "$XVFB_PID" "${DOSBOX_PID:-}" 2>/dev/null || true' EXIT
sleep 3

# Reproduce BLOOD.BAT. Launching BLOODPRG with NO ARGUMENTS and only one drive
# mounted leaves the game looping the ATTRACT DEMO -- it never reaches a playable
# state. Captures taken that way show only the intro/attract sequence, which is why
# this is worth stating: it is the THIRD copy of this launch sequence in re/tools, and
# the other two (dump_dosbox_mem.py, drive_real_game.sh) both carried the same defect.
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALL_PARENT="${4:-$REPO_ROOT/accuracy/cblood_install}"
GAME_EXECUTABLE="${5:-BLOODPRG.EXE}"
dosbox-x -set sdl output=surface \
  -c "mount c \"$INSTALL_PARENT\"" \
  -c "mount d \"$GAME_DIR\" -t cdrom" \
  -c 'd:' \
  -c "$GAME_EXECUTABLE AMR S162227 EMS WRIC:\\cblood\\" >/dev/null 2>&1 &
DOSBOX_PID=$!

# Sample the boot sequence: MINDSCAPE logo -> Microfolie's logo -> intro cutscene.
for t in 6 10 14 18 24 30 36; do
  sleep 4
  import -window root -gravity South -crop 640x360+0+0 +repage \
    -resize 320x200\! "$OUT_DIR/boot_${t}s.png" 2>/dev/null || true
  echo "captured $OUT_DIR/boot_${t}s.png"
done
