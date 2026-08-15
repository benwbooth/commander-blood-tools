# Compiler Corpus

This directory contains natural-C codegen probes for the bit-exact source
recovery track. The files under `samples/` are not recovered game source. They
are small programs used to test what candidate DOS compilers emit for the
calling conventions and data shapes seen in `BLOODPRG.EXE`.

`manifest.tsv` records the recovered routine each probe is meant to exercise.
Rows may also point at the recovered candidate file through `candidate_source`;
the checker verifies those links so probe coverage stays attached to the actual
C recovery work.

Run integrity and original-routine shape checks:

```sh
python3 re/tools/compiler_corpus.py --check
python3 re/tools/compiler_corpus.py --original-shapes
```

Two historical compiler paths have first-party runners. Run an installed Turbo
C tree through DOSBox with:

```sh
nix shell nixpkgs#dosbox-staging -c \
  python3 re/tools/compiler_corpus.py --run-turbo-c \
    --turbo-c tc201=/path/to/tc201 --flag=-mh --flag=-O --flag=-Z
```

Run native Open Watcom C16 and disassemble each generated OMF object with:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure nixpkgs#open-watcom-bin -c \
  python3 re/tools/compiler_corpus.py --run-watcom \
    --watcom watcom19=wcc --flag=-3 --flag=-ox --flag=-mh
```

Both runners accept repeated `--sample` options and store each compiler label
under `out/`. Repeating the command with different labels and flags builds a
comparison matrix. The Turbo C runner flattens repository-local quoted include
dependencies to temporary DOS 8.3 names, so wrapper probes can compile the real
candidate source without maintaining a duplicate probe body.

For another compiler, create a generic JSON runner config from:

```sh
python3 re/tools/compiler_corpus.py --print-config-template
```

Then run:

```sh
python3 re/tools/compiler_corpus.py --config path/to/config.json --run
```

After compiler listings exist, compare them against the recovered assembly
oracles:

```sh
python3 re/tools/compiler_corpus.py --compare
```

The comparator scans `re/compiler_corpus/out/<compiler>/<sample>/`, preferring
raw `*.asm` listings and falling back to `*.normalized.asm`. It emits JSON
metrics for ordered instruction matches, exact mnemonic sequences, ordered
mnemonic matches, mnemonic multiset overlap, and byte-line matches when the
listing includes bytes. These scores are evidence for a compiler/codegen shape;
they do not by themselves accept recovered source.

Exact recovered routine bytes can also be searched in archived compiler
library trees:

```sh
python3 re/tools/compiler_corpus.py \
  --scan-library tc201=/path/to/tc201/TC/LIB --min-routine-bytes 8
```

See `re/source/toolchain_fingerprints/compiler_codegen.md` for the checked
Turbo C 2.00/2.01 and Open Watcom 1.9 experiment results.

Do not check generated compiler output into git. The default output directory is
`re/compiler_corpus/out/`, which is ignored.
