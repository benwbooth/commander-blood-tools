# Compiler Corpus Gate

No historical C/C++ compiler is currently installed in this checkout's dev
environment. The available tools are DOSBox-X, nasm, objdump, Rust, and the
Python RE/oracle scripts.

Before adding natural C routines, build a small compiler corpus and compare
codegen against recovered routines:

- Borland/Turbo C++ candidates around 1993-1995.
- Microsoft C/C++ 7.x/8.x.
- Watcom C/C++ only if real-mode segmented codegen can be matched.

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
