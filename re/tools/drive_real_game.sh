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
#   move_relative <dx> <dy>
#   mouse_button <n>    (press/release without repositioning the captured pointer)
#   mouse_down <n>      (hold a button for later inspection or release)
#   mouse_up <n>        (release a button held by mouse_down)
#   key <keyname>       (e.g. Return, Escape, space)
#   key_down <keyname>  (hold a key for later key_up, including normal repeat)
#   key_up <keyname>    (release a key held by key_down)
#   fastforward <secs>  (hold the emulator's Alt+F12 turbo control)
#   shot <name>         (capture the game area to <out-dir>/<name>.png)
#   wait <seconds>
# Interactive runs default to DOSBox Staging's dynamic core at maximum cycles.
# Override DOSBOX_BINARY, DOSBOX_CORE, DOSBOX_CYCLES, or DOSBOX_FRAMESKIP when a
# strict DOSBox-X normal-core comparison is required.
# Set DOSBOX_TRACE_FILE to record host file reads with strace. By default only
# BLOOD.DAT is traced; DOSBOX_TRACE_PATHS accepts colon-separated host paths.
# The window lookup follows child processes so tracing does not break input.
# The emulator window appears a few seconds after launch; the script waits for it.
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
DOSBOX_BINARY="${DOSBOX_BINARY:-dosbox-staging}"
DOSBOX_CORE="${DOSBOX_CORE:-dynamic}"
DOSBOX_CYCLES="${DOSBOX_CYCLES:-max}"
DOSBOX_FRAMESKIP="${DOSBOX_FRAMESKIP:-10}"
DOSBOX_WINDOW_NAME="${DOSBOX_WINDOW_NAME:-$GAME_EXECUTABLE}"
export DISPLAY="$DISP" SDL_VIDEODRIVER=x11

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
# The emulator may run under strace, which does not forward signals to its
# tracee: killing the wrapper PID leaked a live DOSBox on every aborted run.
# Launch the whole tree in its own process group and kill the GROUP instead.
GAME_PGID=""
cleanup() {
  kill "$XVFB_PID" "${DOSBOX_PID:-}" 2>/dev/null || true
  if [ -n "$GAME_PGID" ]; then
    kill -9 -"$GAME_PGID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
sleep 3
# Reproduce BLOOD.BAT exactly. Mounting only one drive, or launching BLOODPRG with no
# arguments, leaves the game looping the ATTRACT DEMO -- it never reaches a playable
# state, so every capture and every memory dump taken that way is inert. This is the
# same defect that made re/tools/dump_dosbox_mem.py silently useless until it was fixed.
command -v "$DOSBOX_BINARY" >/dev/null || {
  echo "$DOSBOX_BINARY not found (use nix develop)"
  exit 1
}
if [[ "$(basename "$DOSBOX_BINARY")" == *staging* ]]; then
  DOSBOX_ARGS=(
    "$DOSBOX_BINARY"
    --noprimaryconf
    --nolocalconf
    --set output=surface
    --set "cycles=$DOSBOX_CYCLES"
    --set "core=$DOSBOX_CORE"
    --set "frameskip=$DOSBOX_FRAMESKIP"
    --set mouse_capture=onstart
    --set dos_mouse_immediate=true
  )
else
  DOSBOX_ARGS=(
    "$DOSBOX_BINARY"
    -set "sdl output=surface"
    -set "cpu cycles=$DOSBOX_CYCLES"
    -set "cpu core=$DOSBOX_CORE"
    -set "render frameskip=$DOSBOX_FRAMESKIP"
    -set "sdl autolock=true"
  )
fi
if [ -n "${DOSBOX_TRACE_FILE:-}" ]; then
  command -v strace >/dev/null || {
    echo "strace not found"
    exit 1
  }
  mkdir -p "$(dirname "$DOSBOX_TRACE_FILE")"
  IFS=: read -r -a DOSBOX_TRACE_PATH_ARRAY <<< \
    "${DOSBOX_TRACE_PATHS:-$GAME_DIR/BLOOD.DAT}"
  DOSBOX_TRACE_PATH_ARGS=()
  if [ -n "${DOSBOX_TRACE_ALL:-}" ]; then
    # No -P filters: log every traced syscall with its path, so the file a
    # stall or crash happens mid-load is identified directly.
    :
  else
    for trace_path in "${DOSBOX_TRACE_PATH_ARRAY[@]}"; do
      [ -n "$trace_path" ] && DOSBOX_TRACE_PATH_ARGS+=(-P "$trace_path")
    done
  fi
  DOSBOX_ARGS=(
    strace
    -f
    -yy
    # seccomp-bpf filtering keeps untraced syscalls from stopping the
    # tracee: without it, per-frame ptrace stops made the guest miss
    # keyboard input entirely on current kernels (the briefing skip
    # froze mid-presentation), while the traced evidence stays identical.
    --seccomp-bpf
    -e "trace=openat,read,lseek,_llseek,pread64"
    "${DOSBOX_TRACE_PATH_ARGS[@]}"
    -o "$DOSBOX_TRACE_FILE"
    "${DOSBOX_ARGS[@]}"
  )
fi
setsid --wait "${DOSBOX_ARGS[@]}" \
  -c "mount c \"$INSTALL_PARENT\"" \
  -c "mount d \"$GAME_DIR\" -t cdrom" \
  -c 'd:' \
  -c "$GAME_EXECUTABLE AMR S162227 EMS WRIC:\\cblood\\" >/dev/null 2>&1 &
DOSBOX_PID=$!
GAME_PGID="$(ps -o pgid= -p "$DOSBOX_PID" 2>/dev/null | tr -d ' ')"

# Wait for the game window, up to ~20s. Profilers and syscall tracers insert a
# process between this driver and DOSBox, so inspect the complete child tree.
find_process_window() {
  local -a process_queue=("$DOSBOX_PID")
  local queue_index=0
  local process_id child_id

  WID=""
  while (( queue_index < ${#process_queue[@]} )); do
    process_id="${process_queue[$queue_index]}"
    queue_index=$((queue_index + 1))
    WID=$(xdotool search --all --pid "$process_id" 2>/dev/null | head -1 || true)
    [ -n "$WID" ] && return
    while read -r child_id; do
      [ -n "$child_id" ] && process_queue+=("$child_id")
    done < <(pgrep -P "$process_id" 2>/dev/null || true)
  done
}

WID=""
for _ in $(seq 1 20); do
  find_process_window
  if [ -z "$WID" ]; then
    WID=$(xdotool search --name "$DOSBOX_WINDOW_NAME\|DOSBox-X\|DOSBox Staging" \
      2>/dev/null | head -1 || true)
  fi
  [ -n "$WID" ] && break
  sleep 1
done
[ -n "$WID" ] || { echo "game window not found"; exit 1; }
echo "driving window $WID: $(xdotool getwindowname "$WID")"
xdotool windowactivate "$WID" 2>/dev/null || true
xdotool windowfocus --sync "$WID" 2>/dev/null || true
xdotool mousemove --window "$WID" 400 300

while read -r action a b; do
  case "$action" in
    click)
      xdotool mousemove --window "$WID" "$a" "$b"
      sleep 0.3
      xdotool mousedown 1
      sleep 0.2
      xdotool mouseup 1
      ;;
    move_relative)
      xdotool mousemove_relative -- "$a" "$b"
      ;;
    move)
      # Absolute position in the 320x200 GAME space (the dual-run scenario
      # vocabulary); the window is 640x400, so scale by two.
      xdotool mousemove --window "$WID" $((a * 2)) $((b * 2))
      ;;
    mouse_button)
      xdotool mousedown "$a"
      sleep 0.2
      xdotool mouseup "$a"
      ;;
    mouse_down) xdotool mousedown "$a" ;;
    mouse_up)   xdotool mouseup "$a" ;;
    key)
      xdotool keydown --window "$WID" "$a"
      sleep 0.2
      xdotool keyup --window "$WID" "$a"
      ;;
    key_down) xdotool keydown --window "$WID" "$a" ;;
    key_up)   xdotool keyup --window "$WID" "$a" ;;
    fastforward)
      xdotool keydown --window "$WID" Alt_L
      xdotool keydown --window "$WID" F12
      sleep "$a"
      xdotool keyup --window "$WID" F12
      xdotool keyup --window "$WID" Alt_L
      ;;
    wait)  sleep "$a" ;;
    shot)  import -window "$WID" -resize 320x200\! \
             "$OUT_DIR/$a.png" 2>/dev/null; echo "shot $OUT_DIR/$a.png" ;;
    ''|\#*) : ;;
  esac
done
