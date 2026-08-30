#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
ASSET_CACHE=${CBLOOD_ASSET_CACHE:-"$HOME/.local/share/commander-blood/assets-v1"}
ORIGINAL_ARCHIVE_ROOT=${CBLOOD_ORIGINAL_ARCHIVE_ROOT:-"$ROOT/commander-blood-audio/_tmp_iso"}

if [[ ! -d "$ASSET_CACHE" ]]; then
  printf 'Commander Blood asset cache not found: %s\n' "$ASSET_CACHE" >&2
  exit 1
fi
if [[ ! -f "$ORIGINAL_ARCHIVE_ROOT/BLOOD.DAT" ]]; then
  printf 'Original BLOOD.DAT not found: %s\n' "$ORIGINAL_ARCHIVE_ROOT/BLOOD.DAT" >&2
  exit 1
fi

find_free_display() {
  local number
  for number in $(seq 110 139); do
    if [[ ! -S "/tmp/.X11-unix/X$number" ]]; then
      printf ':%s\n' "$number"
      return 0
    fi
  done
  printf 'no free isolated X11 display found\n' >&2
  return 1
}

DISPLAY_NUMBER=${CBLOOD_FIDELITY_DISPLAY:-$(find_free_display)}
DISPLAY_SOCKET_NUMBER=${DISPLAY_NUMBER#:}
XVFB_PID=

cleanup() {
  if [[ -n "$XVFB_PID" ]]; then
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

if [[ -S "/tmp/.X11-unix/X$DISPLAY_SOCKET_NUMBER" ]]; then
  printf 'requested fidelity display is already in use: %s\n' "$DISPLAY_NUMBER" >&2
  exit 1
fi

Xvfb "$DISPLAY_NUMBER" -screen 0 1280x960x24 -nolisten tcp >/dev/null 2>&1 &
XVFB_PID=$!
for _ in $(seq 1 100); do
  [[ -S "/tmp/.X11-unix/X$DISPLAY_SOCKET_NUMBER" ]] && break
  kill -0 "$XVFB_PID" 2>/dev/null || {
    printf 'isolated Xvfb exited before becoming ready\n' >&2
    exit 1
  }
  sleep 0.05
done
[[ -S "/tmp/.X11-unix/X$DISPLAY_SOCKET_NUMBER" ]] || {
  printf 'isolated Xvfb did not become ready\n' >&2
  exit 1
}

cd "$ROOT"
unset WAYLAND_DISPLAY
export DISPLAY=$DISPLAY_NUMBER
export SDL_VIDEODRIVER=x11
export SDL_AUDIODRIVER=dummy
export WGPU_BACKEND=vulkan
export CBLOOD_ASSET_CACHE=$ASSET_CACHE
export CBLOOD_ORIGINAL_ARCHIVE_ROOT=$ORIGINAL_ARCHIVE_ROOT
export CBLOOD_REQUIRE_ACCURACY_TESTS=1

printf 'Running fail-closed Rust fidelity gate on isolated display %s\n' "$DISPLAY"
cargo test \
  -p commander-blood-formats \
  -p commander-blood-script-compiler \
  -p commander-blood-game \
  -- \
  --test-threads=1

COVERAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/cblood-production-coverage.XXXXXX")
printf 'Instrumenting startup production coverage under %s\n' "$COVERAGE_ROOT"
CARGO_TARGET_DIR="$COVERAGE_ROOT/target" \
  CARGO_INCREMENTAL=0 \
  RUSTFLAGS=-Cinstrument-coverage \
  LLVM_PROFILE_FILE="$COVERAGE_ROOT/%p-%m.profraw" \
  cargo test \
    -p commander-blood-game \
    --test startup_phone_runtime \
    production_runtime_completes_the_authored_startup_phone_call \
    -- \
    --exact \
    --test-threads=1
llvm-profdata merge -sparse "$COVERAGE_ROOT"/*.profraw \
  -o "$COVERAGE_ROOT/merged.profdata"
python3 -P re/tools/audit_rust_production_coverage.py \
  --binary "$COVERAGE_ROOT/target/debug/commander-blood" \
  --profile "$COVERAGE_ROOT/merged.profdata" \
  --scenario startup-phone-complete \
  --expected-covered re/rust-port/production-startup-covered.tsv \
  --output "$COVERAGE_ROOT/report.json" \
  --summary-only

cargo build -p commander-blood-game --bin commander-blood
python3 -P re/tools/audit_rust_port_routing.py --strict
python3 -P re/tools/test_compare_port_runtime_traces.py
python3 -P re/tools/test_audit_rust_production_coverage.py
python3 -P re/tools/test_verify_startup_phone_trace.py

printf 'Rust fidelity gate passed\n'
