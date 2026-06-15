#!/usr/bin/env bash
# Commander Blood accuracy oracle: boot the real game in DOSBox-X on an isolated
# Xvfb virtual display and capture reference frames. Never uses the user's
# desktop display. Run from the repo root via the nix dev shell:
#
#   nix develop --command bash accuracy/run_oracle.sh [seconds]
#
# Output: accuracy/captures/frame_NN.png (host window grabs) and any DOSBox-X
# native screenshots/AVI under accuracy/captures/.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
ISO="$(readlink -f "$HERE/output/CMDR_BLOOD.iso")"
DUR="${1:-40}"               # total seconds to let the game run
DISP=":99"
CAP="$HERE/accuracy/captures"
CONF="$HERE/accuracy/.dosbox.runtime.conf"

mkdir -p "$CAP"
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

echo "ISO     : $ISO"
echo "display : $DISP (isolated Xvfb)"
echo "duration: ${DUR}s"

Xvfb "$DISP" -screen 0 800x600x24 >/dev/null 2>&1 &
XVFB_PID=$!
cleanup() { kill "$DB_PID" "$XVFB_PID" 2>/dev/null || true; }
trap cleanup EXIT INT TERM
sleep 2

DISPLAY="$DISP" dosbox-x -conf "$CONF" -nogui >"$CAP/dosbox.log" 2>&1 &
DB_PID=$!

# Grab the framebuffer every few seconds while the game runs.
n=0
elapsed=0
while [ "$elapsed" -lt "$DUR" ]; do
  sleep 4
  elapsed=$((elapsed + 4))
  n=$((n + 1))
  if DISPLAY="$DISP" import -window root "$CAP/frame_$(printf '%02d' $n).png" 2>/dev/null; then
    echo "captured frame_$(printf '%02d' $n).png at ${elapsed}s"
  fi
  kill -0 "$DB_PID" 2>/dev/null || { echo "dosbox-x exited early (see dosbox.log)"; break; }
done

echo "done; captures in $CAP"
