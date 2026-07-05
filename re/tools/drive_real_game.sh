#!/usr/bin/env bash
# Drive the original Commander Blood under DOSBox-X on Xvfb with synthetic input
# (xdotool), capturing frames — the foundation for the interactive runtime-sequencing
# diff against the Rust engine. Extends capture_real_game.sh (passive capture) with
# input control so the game can be navigated to specific scenes.
#
#   nix develop --command re/tools/drive_real_game.sh <game-dir> <out-dir> [display] [args]
#
# Reads an input script from stdin: one action per line, either
#   click <x> <y>       (mouse click at game-relative x,y; game area is 640x400)
#   key <keyname>       (e.g. Return, Escape, space)
#   shot <name>         (capture the game area to <out-dir>/<name>.png)
#   wait <seconds>
# The DOSBox-X window is found by its "DOSBox-X"/"BLOODPRG" title (it appears a few
# seconds after launch — the script waits for it).
set -euo pipefail

GAME_DIR="$(realpath "${1:?usage: drive_real_game.sh <game-dir> <out-dir> [display] [args]}")"
OUT_DIR="${2:?missing out-dir}"; mkdir -p "$OUT_DIR"
DISP="${3:-:73}"; GAME_ARGS="${4:-}"
export DISPLAY="$DISP" SDL_VIDEODRIVER=x11

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
trap 'kill "$XVFB_PID" "${DOSBOX_PID:-}" 2>/dev/null || true' EXIT
sleep 3
dosbox-x -set sdl output=surface \
  -c "mount c \"$GAME_DIR\"" -c 'c:' -c "BLOODPRG.EXE $GAME_ARGS" >/dev/null 2>&1 &
DOSBOX_PID=$!

# Wait for the game window (title contains DOSBox-X), up to ~20s.
WID=""
for _ in $(seq 1 20); do
  WID=$(xdotool search --name "DOSBox-X" 2>/dev/null | head -1 || true)
  [ -n "$WID" ] && break
  sleep 1
done
[ -n "$WID" ] || { echo "game window not found"; exit 1; }
echo "driving window $WID: $(xdotool getwindowname "$WID")"
xdotool windowactivate "$WID" 2>/dev/null || true

while read -r action a b; do
  case "$action" in
    click) xdotool mousemove --window "$WID" "$a" "$b"; sleep 0.3; xdotool click --window "$WID" 1 ;;
    key)   xdotool key --window "$WID" "$a" ;;
    wait)  sleep "$a" ;;
    shot)  import -window root -gravity South -crop 640x400+0+0 +repage \
             -resize 320x200\! "$OUT_DIR/$a.png" 2>/dev/null; echo "shot $OUT_DIR/$a.png" ;;
    ''|\#*) : ;;
  esac
done
