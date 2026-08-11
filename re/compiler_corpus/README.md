# Compiler Corpus

This directory contains natural-C codegen probes for the bit-exact source
recovery track. The files under `samples/` are not recovered game source. They
are small programs used to test what candidate DOS compilers emit for the
calling conventions and data shapes seen in `BLOODPRG.EXE`.

Run integrity and original-routine shape checks:

```sh
python3 re/tools/compiler_corpus.py --check
python3 re/tools/compiler_corpus.py --original-shapes
```

When a compiler is available locally, create a JSON config from:

```sh
python3 re/tools/compiler_corpus.py --print-config-template
```

Then run:

```sh
python3 re/tools/compiler_corpus.py --config path/to/config.json --run
```

Do not check generated compiler output into git. The default output directory is
`re/compiler_corpus/out/`, which is ignored.
