#!/usr/bin/env bash
# Drive the original Commander Blood under DOSBox-X on Xvfb with synthetic input
# (xdotool), capturing frames — the foundation for the interactive runtime-sequencing
# diff against the Rust engine. Extends capture_real_game.sh (passive capture) with
# input control so the game can be navigated to specific scenes.
#
#   nix develop --command re/tools/drive_real_game.sh \
#     <game-dir> <out-dir> [display] [install-parent] [executable]
#
# Reads an input script from stdin: one action per line, either
#   click <x> <y>       (mouse click at game-relative x,y; game area is 640x400)
#   key <keyname>       (e.g. Return, Escape, space)
#   shot <name>         (capture the game area to <out-dir>/<name>.png)
#   wait <seconds>
# Set DOSBOX_CYCLES (for example, `max` or `3000`) to override DOSBox-X's
# cycle rate while preserving the same launch and input sequence.
# The DOSBox-X window is found by its "DOSBox-X"/"BLOODPRG" title (it appears a few
# seconds after launch — the script waits for it).
set -euo pipefail

# <game-dir> is the CD image dir that CONTAINS BLOODPRG.EXE (e.g. output/_tmp_iso),
# mounted as D:. The installed data dir (C:\cblood) is a SEPARATE tree.
GAME_DIR="$(realpath "${1:?usage: drive_real_game.sh <cd-dir> <out-dir> [display] [install-parent] [executable]}")"
OUT_DIR="${2:?missing out-dir}"; mkdir -p "$OUT_DIR"
DISP="${3:-:73}"
# 4th arg is now the PARENT of the `cblood` install dir, mounted as C: so the game's
# write path C:\cblood\ resolves. Defaults to accuracy/cblood_install.
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALL_PARENT="${4:-$REPO_ROOT/accuracy/cblood_install}"
GAME_EXECUTABLE="${5:-BLOODPRG.EXE}"
DOSBOX_CYCLES="${DOSBOX_CYCLES:-}"
export DISPLAY="$DISP" SDL_VIDEODRIVER=x11

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
trap 'kill "$XVFB_PID" "${DOSBOX_PID:-}" 2>/dev/null || true' EXIT
sleep 3
# Reproduce BLOOD.BAT exactly. Mounting only one drive, or launching BLOODPRG with no
# arguments, leaves the game looping the ATTRACT DEMO -- it never reaches a playable
# state, so every capture and every memory dump taken that way is inert. This is the
# same defect that made re/tools/dump_dosbox_mem.py silently useless until it was fixed.
DOSBOX_ARGS=(dosbox-x -set "sdl output=surface")
if [ -n "$DOSBOX_CYCLES" ]; then
  DOSBOX_ARGS+=(-set "cpu cycles=$DOSBOX_CYCLES")
fi
"${DOSBOX_ARGS[@]}" \
  -c "mount c \"$INSTALL_PARENT\"" \
  -c "mount d \"$GAME_DIR\" -t cdrom" \
  -c 'd:' \
  -c "$GAME_EXECUTABLE AMR S162227 EMS WRIC:\\cblood\\" >/dev/null 2>&1 &
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
