# Compiler Corpus Gate

The corpus has now been compiled with archived Turbo C 2.00 and 2.01 through
DOSBox and with Open Watcom C16 1.9. None of the tested natural-C probes was an
exact machine-code match. Turbo C's stack-oriented lowering was generally a
poor fit; Open Watcom's default register convention was structurally closer but
still not an identification. See `toolchain_fingerprints/compiler_codegen.md`
for the measured results and tool hashes.

Before adding natural C routines, build a small compiler corpus and compare
codegen against recovered routines:

- Borland/Turbo C++ candidates around 1993-1995.
- Microsoft C/C++ 7.x/8.x.
- Watcom C/C++ versions capable of real-mode segmented codegen.

Compile tiny programs that exercise:

- near and far functions with zero, one, and two integer arguments;
- `__fastcall` or equivalent register-argument modes if the compiler supports
  them;
- far-pointer arguments and ES:DI/DS:SI string operations;
- carry-like boolean/status returns;
- 32-bit arithmetic in 16-bit real-mode code;
- switch/jump-table lowering;
- segment-qualified globals.

Do not accept a recovered `.c` file until its routine's calling convention and
data declarations have a matching corpus result or a documented assembly ABI
boundary.

The initial corpus lives under `re/compiler_corpus` and is checked with:

```sh
python3 re/tools/compiler_corpus.py --check
python3 re/tools/compiler_corpus.py --original-shapes
```

The sample files are codegen probes only. They are not recovered game source.
Recovered routine candidates live separately under `bloodprg/candidates` and
are checked with `python3 re/tools/source_candidates.py --check`.

For ABI-sensitive recovered functions, corpus rows should fill
`candidate_source` with the candidate C path. These probes are deliberately
smaller than the real candidate when needed, but the link makes review explicit:
the sample is a codegen experiment for that candidate, not an unrelated toy.
