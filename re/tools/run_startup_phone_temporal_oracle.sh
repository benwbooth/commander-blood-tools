#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
ASSET_CACHE=${CBLOOD_ASSET_CACHE:-"$HOME/.local/share/commander-blood/assets-v1"}
ORIGINAL_ARCHIVE_ROOT=${CBLOOD_ORIGINAL_ARCHIVE_ROOT:-"$ROOT/commander-blood-audio/_tmp_iso"}
SCENARIO="$ROOT/accuracy/scenarios/startup_phone_name_area_timeline.tsv"
ARTIFACT_ROOT=${CBLOOD_FIDELITY_ARTIFACT_ROOT:-"$ROOT/output/fidelity"}
ORACLE_TIMEOUT_SECONDS=${CBLOOD_ORACLE_TIMEOUT_SECONDS:-600}
mkdir -p "$ARTIFACT_ROOT"
TRACE_ROOT=$(mktemp -d "$ARTIFACT_ROOT/startup-temporal.XXXXXX")
XVFB_PID=

cleanup() {
  if [[ -n "$XVFB_PID" ]]; then
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
  fi
  printf 'Startup temporal oracle artifacts: %s\n' "$TRACE_ROOT" >&2
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

git rev-parse HEAD >"$TRACE_ROOT/revision.txt"
git status --porcelain >"$TRACE_ROOT/worktree-status.txt"
git diff HEAD -- crates src re accuracy/scenarios >"$TRACE_ROOT/worktree.patch"
sha256sum \
  "$ROOT/target/debug/runtime_boot" \
  "$ROOT/target/debug/commander-blood" \
  "$ORIGINAL_ARCHIVE_ROOT/BLOODPRG.EXE" \
  "$ORIGINAL_ARCHIVE_ROOT/BLOOD.DAT" \
  "$ASSET_CACHE/manifest.json" \
  "$ROOT/Cargo.lock" "$ROOT/flake.lock" "$SCENARIO" \
  >"$TRACE_ROOT/sha256sums.txt"

mkdir -p "$TRACE_ROOT/original" "$TRACE_ROOT/c-original" "$TRACE_ROOT/rust"
VERIFYSCRIPT=$SCENARIO \
VERIFYSTATE=- \
VERIFYTRACE="$TRACE_ROOT/original/semantic-trace.jsonl" \
  timeout --kill-after=10s "${ORACLE_TIMEOUT_SECONDS}s" \
    "$ROOT/target/debug/runtime_boot" \
    --c-root "$TRACE_ROOT/c-original" \
    --d-root "$ORIGINAL_ARCHIVE_ROOT" \
    --executable BLOODPRG.EXE \
    --out "$TRACE_ROOT/original" \
    --cpu-multiplier 1 \
    2>&1 | tee "$TRACE_ROOT/original.log"

timeout --kill-after=10s "${ORACLE_TIMEOUT_SECONDS}s" \
  "$ROOT/target/debug/commander-blood" \
  --write-data "$TRACE_ROOT/rust-writable" \
  --scenario "$SCENARIO" \
  --trace "$TRACE_ROOT/rust/semantic-trace.jsonl" \
  --oracle-packed-second 39 \
  2>&1 | tee "$TRACE_ROOT/rust.log"

python3 -P "$ROOT/re/tools/compare_port_runtime_traces.py" \
  "$TRACE_ROOT/original/semantic-trace.jsonl" \
  "$TRACE_ROOT/rust/semantic-trace.jsonl" \
  --start-action 9 \
  --minimum-compared-records 47 \
  --bridge-frame-tolerance 0 \
  --require-game-frame-clock \
  --output "$TRACE_ROOT/report.json"

printf 'Startup phone temporal oracle passed; see report.json for measured coverage\n'
