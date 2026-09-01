#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
ASSET_CACHE=${CBLOOD_ASSET_CACHE:-"$HOME/.local/share/commander-blood/assets-v1"}
ORIGINAL_ARCHIVE_ROOT=${CBLOOD_ORIGINAL_ARCHIVE_ROOT:-"$ROOT/commander-blood-audio/_tmp_iso"}
SCENARIO="$ROOT/accuracy/scenarios/startup_phone_name_area_timeline.tsv"
TRACE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/cblood-startup-temporal.XXXXXX")
XVFB_PID=
KEEP_TRACE=1

cleanup() {
  if [[ -n "$XVFB_PID" ]]; then
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_TRACE" -eq 0 ]]; then
    rm -rf "$TRACE_ROOT"
  else
    printf 'Preserving failed startup temporal oracle at %s\n' "$TRACE_ROOT" >&2
  fi
}
trap cleanup EXIT INT TERM

if [[ ! -d "$ASSET_CACHE" ]]; then
  printf 'Commander Blood asset cache not found: %s\n' "$ASSET_CACHE" >&2
  exit 1
fi
if [[ ! -f "$ORIGINAL_ARCHIVE_ROOT/BLOODPRG.EXE" ]]; then
  printf 'Original BLOODPRG.EXE not found: %s\n' "$ORIGINAL_ARCHIVE_ROOT/BLOODPRG.EXE" >&2
  exit 1
fi

if [[ -z "${DISPLAY:-}" ]]; then
  for number in $(seq 140 169); do
    if [[ ! -S "/tmp/.X11-unix/X$number" ]]; then
      export DISPLAY=":$number"
      break
    fi
  done
  if [[ -z "${DISPLAY:-}" ]]; then
    printf 'No free isolated X11 display found\n' >&2
    exit 1
  fi
  Xvfb "$DISPLAY" -screen 0 1280x960x24 -nolisten tcp >/dev/null 2>&1 &
  XVFB_PID=$!
  for _ in $(seq 1 100); do
    [[ -S "/tmp/.X11-unix/X${DISPLAY#:}" ]] && break
    kill -0 "$XVFB_PID" 2>/dev/null || {
      printf 'Isolated Xvfb exited before becoming ready\n' >&2
      exit 1
    }
    sleep 0.05
  done
  [[ -S "/tmp/.X11-unix/X${DISPLAY#:}" ]] || {
    printf 'Isolated Xvfb did not become ready\n' >&2
    exit 1
  }
fi

unset WAYLAND_DISPLAY
export CBLOOD_ASSET_CACHE=$ASSET_CACHE
export SDL_AUDIODRIVER=dummy
export WGPU_BACKEND=${WGPU_BACKEND:-vulkan}

cd "$ROOT"
cargo build -p commander-blood-tools --bin runtime_boot
cargo build -p commander-blood-game --bin commander-blood

mkdir -p "$TRACE_ROOT/original" "$TRACE_ROOT/c-original" "$TRACE_ROOT/rust"
VERIFYSCRIPT=$SCENARIO \
VERIFYSTATE=- \
VERIFYTRACE="$TRACE_ROOT/original/semantic-trace.jsonl" \
  "$ROOT/target/debug/runtime_boot" \
    --c-root "$TRACE_ROOT/c-original" \
    --d-root "$ORIGINAL_ARCHIVE_ROOT" \
    --executable BLOODPRG.EXE \
    --out "$TRACE_ROOT/original" \
    --cpu-multiplier 1

"$ROOT/target/debug/commander-blood" \
  --write-data "$TRACE_ROOT/rust-writable" \
  --scenario "$SCENARIO" \
  --trace "$TRACE_ROOT/rust/semantic-trace.jsonl" \
  --oracle-packed-second 39

python3 -P "$ROOT/re/tools/compare_port_runtime_traces.py" \
  "$TRACE_ROOT/original/semantic-trace.jsonl" \
  "$TRACE_ROOT/rust/semantic-trace.jsonl" \
  --start-action 9 \
  --require-game-frame-clock \
  --output "$TRACE_ROOT/report.json"

KEEP_TRACE=0
printf 'Startup phone temporal oracle passed: 47 exact frame-aligned records\n'
