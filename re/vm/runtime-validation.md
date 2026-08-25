# Compiled VM runtime validation

Runtime substitution was validated on 2026-08-14 with DOSBox-X 2026.05.02.
The unified-profile replacement was validated on 2026-08-24 by byte comparison
and isolated runtime-tree construction.

The checked-in unified profiles were compiled and compared before building a
runnable extracted-CD tree:

```sh
cargo run --bin cbvm -- compile-bundle \
  re/vm/profiles output/_tmp_iso /tmp/cb-vm-bundle-v7

cargo run --bin cbvm -- build-runtime-tree \
  re/vm/profiles output/_tmp_iso /tmp/cblood-runtime-v7
```

The bundle contains 25 exact compiled resources: ten COD/BAS images compiled
from the five self-contained BloodScript v8 profiles.
`bundle-manifest.tsv` records every size, source origin, and comparison result.
The runtime-tree builder omitted all original 25 resources while cloning assets,
then installed the compiled bundle, so the test tree cannot silently retain any
original VM resource.

The 2026-08-14 isolated oracle mounted the equivalent split-source runtime tree
as DOS drive D and ran the original executable with the recovered launch
arguments:

```sh
ORACLE_GAME_DIR=/path/to/compiled-runtime-tree \
ORACLE_CAPTURE_DIR=/tmp/cb-vm-oracle \
ORACLE_CAPTURE_INTERVAL=2 \
nix develop --command bash accuracy/run_oracle.sh 12
```

The generated DOSBox configuration contained:

```text
mount D "/path/to/compiled-runtime-tree"
BLOODPRG AMR S162227 EMS WRIC:\cblood\
```

DOSBox stayed alive for the complete run and produced six distinct captures at
2-second intervals. The final capture showed the rendered spacecraft intro.
That run proves the original DOS executable loads and runs against the recovered
25-resource bundle. The unified compiler emits those same 317,835 bytes exactly,
so it preserves the validated runtime inputs; a fresh interactive DOSBox run is
still the final behavioral gate after future source edits. The captures are
transient test artifacts, not repository inputs.
