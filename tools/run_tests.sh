#!/usr/bin/env bash
# Run the WHOLE test suite and summarise every binary.
#
# Why this exists: `cargo test | grep '^test result' | head -3` reads as a full
# green suite and is not one. The workspace has ten test binaries; cargo prints
# them in order and ABORTS at the first failure, so a truncated grep can miss both
# the failure and everything after it. A concept-menu oracle test failed that way
# for an unknown number of sessions while the summary said "green"
# (docs/audit-fixes.md #111).
#
# Usage: tools/run_tests.sh [extra cargo args]
set -uo pipefail

out=$(nix develop --command cargo test --release --no-fail-fast "$@" 2>&1)
status=$?

echo "$out" | grep -E "^(     Running|test result:)" |
    sed 's/^     Running /\nBINARY  /; s/^test result: /  -> /'

echo
failed=$(echo "$out" | grep -cE "^test result: FAILED")
# awk, not bc: bc is not in this dev shell and its absence silently printed "?".
passed=$(echo "$out" | grep -oE "^test result: ok\. [0-9]+" | grep -oE "[0-9]+$" |
    awk '{n += $1} END {print n + 0}')
echo "SUMMARY: $passed test(s) passed across $(echo "$out" | grep -cE '^test result:') binaries; $failed binary/binaries FAILED"

# --no-fail-fast keeps going past a failing binary, so the summary covers all of
# them rather than stopping at the first.
if [ "$failed" -ne 0 ]; then
    echo
    echo "$out" | grep -A 4 "^failures:" | head -40
fi
exit $status
