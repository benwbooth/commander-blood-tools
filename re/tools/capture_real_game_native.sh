#!/usr/bin/env bash
# Geometry-correct capture of the original Commander Blood under DOSBox-X on Xvfb.
#
# Supersedes capture_real_game.sh's geometry for behavioral-equivalence work: that script
# cropped 640x360 with South gravity and resized to 320x200, which BOTH mis-positioned the
# frame and changed the aspect ratio (16:9 -> 8:5), inflating any pixel diff against a clean
# decode with alignment error that is NOT decoder inaccuracy.
#
# This version forces `windowresolution=640x400` + `render aspect=false`, so DOSBox renders
# the game's 320x200 as an exact 2x (640x400) at the window top-left (0,0). We then crop
# 640x400+0+0 and resize to 320x200 -> a clean, aspect-correct, undistorted frame that is
# directly pixel-comparable to the Rust engine's / decoder's native 320x200 output.
#
#   nix develop --command re/tools/capture_real_game_native.sh <game-dir> <out-dir> [display] [args]
# Frames land as <out-dir>/nat_<t>s.png (320x200, no aspect distortion).
set -euo pipefail

GAME_DIR="$(realpath "${1:?usage: capture_real_game_native.sh <game-dir> <out-dir> [display] [args]}")"
OUT_DIR="${2:?missing out-dir}"; mkdir -p "$OUT_DIR"
DISP="${3:-:64}"; GAME_ARGS="${4:-}"
export DISPLAY="$DISP" SDL_VIDEODRIVER=x11

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
trap 'kill "$XVFB_PID" "${DOSBOX_PID:-}" 2>/dev/null || true' EXIT
sleep 3

dosbox-x -set sdl output=surface -set sdl windowresolution=640x400 -set render aspect=false \
  -set sdl showmenu=false \
  -c "mount c \"$GAME_DIR\"" -c 'c:' -c "BLOODPRG.EXE $GAME_ARGS" >/dev/null 2>&1 &
DOSBOX_PID=$!

# The 640x400 game render is CENTERED in the 800x600 Xvfb (x=(800-640)/2=80,
# y=(600-400)/2=100). Crop exactly that rect and Box-downscale 2x -> undistorted 320x200.
# NOTE: this DOSBox-X build still draws a ~14px menu bar at the very top even with
# showmenu=false; mask the top rows (e.g. -crop 320x186+0+14) before any pixel comparison.
# CAVEAT (see re/PROGRESS.md): even with correct geometry, screen-scrape pixel-diff is NOT a
# rigorous parity metric - the DOSBox SDL scaler + palette/gamma differ from a clean decode,
# and high-contrast content amplifies sub-pixel misalignment. Use for visual/qualitative
# checks; for per-pixel parity read the mode-X framebuffer from memory instead.
for t in 6 8 10 12 14 16 20 26 32; do
  sleep 2
  import -window root -crop 640x400+80+100 +repage -filter Box -resize 320x200\! \
    "$OUT_DIR/nat_${t}s.png" 2>/dev/null || true
  echo "captured $OUT_DIR/nat_${t}s.png"
done
