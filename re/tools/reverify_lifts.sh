#!/usr/bin/env bash
# Re-verify every auto-lifted function against FRESHLY GENERATED oracle vectors.
#
# The committed vectors in oracle_vectors/ are a fixed corpus: replaying them
# proves the lift still matches THOSE inputs. This regenerates them from scratch —
# new random register/memory states run through the real DOS bytes under Unicorn —
# so a pass is independent evidence rather than a replay. 52 functions x 250
# vectors takes a few minutes.
#
# Requires unicorn (in the flake since this script existed). Run from the repo
# root inside `nix develop`. Vectors are restored afterwards so the tree is clean;
# pass --keep to leave the fresh ones in place.
set -euo pipefail
KEEP=${1:-}
BACKUP=$(mktemp -d)
cp re/tools/oracle_vectors/*.json "$BACKUP/"
trap '[ "$KEEP" = "--keep" ] || cp "$BACKUP"/*.json re/tools/oracle_vectors/; rm -rf "$BACKUP"' EXIT

names=$(python3 - <<'PY'
import re
mod = open('src/recomp/mod.rs').read()
i = mod.index('fn auto_lifted_batch_matches_oracle')
print('\n'.join(re.findall(r'"(func_[0-9a-f]+)"', mod[i:mod.index('];', i)])))
PY
)
n=0
for name in $names; do
  addr="${name#func_}"
  PYTHONSAFEPATH=1 python3 re/tools/auto_oracle.py "$addr" ret "$name" >/dev/null 2>&1 ||
    PYTHONSAFEPATH=1 python3 re/tools/auto_oracle.py "$addr" retf "$name" >/dev/null 2>&1 ||
    { echo "could not generate vectors for $name"; exit 1; }
  n=$((n+1))
done
echo "regenerated vectors for $n functions; replaying the lifts..."
cargo test --release --lib recomp
