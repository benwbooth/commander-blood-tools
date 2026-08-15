# Compiled VM runtime validation

Validated on 2026-08-14 with DOSBox-X 2026.05.02.

The checked-in structured sources were compiled and compared before building a
runnable extracted-CD tree:

```sh
cargo run --bin cbvm -- compile-bundle \
  re/vm/structured output/_tmp_iso /tmp/cb-vm-bundle-eee4e105

cargo run --bin cbvm -- build-runtime-tree \
  re/vm/structured output/_tmp_iso /tmp/cblood-runtime-eee4e105
```

The bundle contains 25 exact resources: ten COD/BAS images compiled from
BloodScript and fifteen DEB/DIC/VAR companions preserved from the English CD.
`bundle-manifest.tsv` records every size and comparison result. The runtime-tree
builder omitted the original 25 resources while cloning assets, then installed
the compiled bundle, so the test tree cannot silently retain an original COD or
BAS file.

The isolated oracle then mounted that directory as DOS drive D and ran the
original executable with the recovered launch arguments:

```sh
ORACLE_GAME_DIR=/tmp/cblood-runtime-eee4e105 \
ORACLE_CAPTURE_DIR=/tmp/cb-vm-oracle-eee4e105 \
ORACLE_CAPTURE_INTERVAL=2 \
nix develop --command bash accuracy/run_oracle.sh 12
```

The generated DOSBox configuration contained:

```text
mount D "/tmp/cblood-runtime-eee4e105"
BLOODPRG AMR S162227 EMS WRIC:\cblood\
```

DOSBox stayed alive for the complete run and produced six changing captures at
2-second intervals. The final capture showed the rendered spacecraft intro.
This proves that the original DOS executable loads and runs against the bundle
emitted by the BloodScript compiler. The captures are transient test artifacts,
not repository inputs.
