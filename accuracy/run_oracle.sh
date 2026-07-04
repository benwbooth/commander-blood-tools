#!/usr/bin/env bash
# Commander Blood accuracy oracle: boot the real game in DOSBox-X on an isolated
# Xvfb virtual display and capture reference frames. Never uses the user's
# desktop display. Run from the repo root via the nix dev shell:
#
#   nix develop --command bash accuracy/run_oracle.sh [seconds]
#
# Output: accuracy/captures/frame_NN.png (host window grabs) and any DOSBox-X
# native screenshots/AVI under accuracy/captures/. Also writes
# accuracy/captures/capture-manifest.tsv with elapsed capture metadata.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
ISO="$(readlink -f "$HERE/output/CMDR_BLOOD.iso")"
DUR="${1:-40}" # total seconds to let the game run
INTERVAL="${ORACLE_CAPTURE_INTERVAL:-4}"
DISP="${ORACLE_DISPLAY:-:99}"
CAP="${ORACLE_CAPTURE_DIR:-$HERE/accuracy/captures}"
CONF="$HERE/accuracy/.dosbox.runtime.conf"
MANIFEST="${ORACLE_CAPTURE_MANIFEST:-$CAP/capture-manifest.tsv}"
INPUT_SCRIPT="${ORACLE_INPUT_SCRIPT:-}"
INPUT_DELAY="${ORACLE_INPUT_DELAY:-5}"
NATIVE_CROP_X=80
NATIVE_CROP_Y=100
NATIVE_CROP_W=640
NATIVE_CROP_H=480
NATIVE_W=320
NATIVE_H=200
DB_PID=""
INPUT_PID=""

case "$DUR" in
  ''|*[!0-9]*) echo "duration must be a positive integer number of seconds"; exit 1 ;;
esac
# INTERVAL may be fractional (e.g. 0.5) so the boot logo sequence can be
# sampled densely enough to frame-align a fast cutscene against a capture.
case "$INTERVAL" in
  ''|*[!0-9.]*|*.*.*) echo "ORACLE_CAPTURE_INTERVAL must be a positive number of seconds"; exit 1 ;;
esac
if [ "$DUR" -le 0 ]; then
  echo "duration must be positive"
  exit 1
fi
if ! awk "BEGIN{exit !($INTERVAL > 0)}"; then
  echo "ORACLE_CAPTURE_INTERVAL must be positive"
  exit 1
fi

mkdir -p "$CAP"
mkdir -p "$(dirname "$MANIFEST")"
CDRIVE="$HERE/accuracy/cdrive"
mkdir -p "$CDRIVE"
# Materialise a config with the real ISO path + writable C: + absolute captures.
sed -e "s|ISO_PATH_PLACEHOLDER|$ISO|" \
    -e "s|CDRIVE_PLACEHOLDER|$CDRIVE|" \
    -e "s|^captures .*|captures = $CAP|" \
    "$HERE/accuracy/dosbox.conf" > "$CONF"

command -v Xvfb     >/dev/null || { echo "Xvfb not found"; exit 1; }
command -v dosbox-x >/dev/null || { echo "dosbox-x not found (use nix develop)"; exit 1; }
command -v import   >/dev/null || { echo "ImageMagick 'import' not found"; exit 1; }
if [ -n "$INPUT_SCRIPT" ]; then
  [ -x "$INPUT_SCRIPT" ] || { echo "ORACLE_INPUT_SCRIPT is not executable: $INPUT_SCRIPT"; exit 1; }
  command -v xdotool >/dev/null || { echo "xdotool not found (use nix develop)"; exit 1; }
fi

echo "ISO     : $ISO"
echo "display : $DISP (isolated Xvfb)"
echo "duration: ${DUR}s"
echo "interval: ${INTERVAL}s"
echo "captures: $CAP"
echo "manifest: $MANIFEST"

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
cleanup() {
  [ -n "$INPUT_PID" ] && kill "$INPUT_PID" 2>/dev/null || true
  [ -n "$DB_PID" ] && kill "$DB_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM
sleep 2

DISPLAY="$DISP" dosbox-x -conf "$CONF" -nogui >"$CAP/dosbox.log" 2>&1 &
DB_PID=$!

if [ -n "$INPUT_SCRIPT" ]; then
  (
    sleep "$INPUT_DELAY"
    DISPLAY="$DISP" \
      ORACLE_DISPLAY="$DISP" \
      ORACLE_DOSBOX_PID="$DB_PID" \
      ORACLE_CAPTURE_DIR="$CAP" \
      "$INPUT_SCRIPT"
  ) &
  INPUT_PID=$!
fi

printf "frame\tpath\telapsed_s\tepoch_s\tdisplay\tcapture_kind\tcrop_x\tcrop_y\tcrop_w\tcrop_h\tnative_w\tnative_h\n" > "$MANIFEST"

# Grab the framebuffer every few seconds while the game runs.
n=0
elapsed=0
# Float-safe loop so ORACLE_CAPTURE_INTERVAL can be sub-second. `elapsed` is
# formatted with awk; the manifest records it verbatim (integer-valued when the
# interval is whole seconds, so existing manifests are unchanged).
while awk "BEGIN{exit !($elapsed < $DUR)}"; do
  sleep "$INTERVAL"
  elapsed="$(awk "BEGIN{printf \"%g\", $elapsed + $INTERVAL}")"
  n=$((n + 1))
  frame="frame_$(printf '%02d' $n).png"
  if DISPLAY="$DISP" import -window root "$CAP/$frame" 2>/dev/null; then
    epoch="$(date +%s)"
    frame_path="$(readlink -f "$CAP/$frame")"
    printf "%s\t%s\t%s\t%s\t%s\thost-root\t%s\t%s\t%s\t%s\t%s\t%s\n" \
      "$frame" "$frame_path" "$elapsed" "$epoch" "$DISP" \
      "$NATIVE_CROP_X" "$NATIVE_CROP_Y" "$NATIVE_CROP_W" "$NATIVE_CROP_H" \
      "$NATIVE_W" "$NATIVE_H" >> "$MANIFEST"
    echo "captured $frame at ${elapsed}s"
  fi
  kill -0 "$DB_PID" 2>/dev/null || { echo "dosbox-x exited early (see dosbox.log)"; break; }
done

echo "done; captures in $CAP"
