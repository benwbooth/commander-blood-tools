# Historical Compiler Codegen Matrix

This report records compiler-in-the-loop experiments against ten natural-C
probes in `re/compiler_corpus`. Generated objects and listings remain ignored;
the source corpus, runner, commands, hashes, and conclusions are checked in.

## Toolchains

| tool | version | sha256 |
| --- | --- | --- |
| Turbo C `TCC.EXE` | 2.00 | `24535a9a628b70f1daf3e2382c6911046f671baad02e890be5e7e62dbaad6c1e` |
| Turbo C `TLINK.EXE` | from 2.00 tree | `997fcac6089fa88f3d868bdaf8bd65bd44c5aa83885a1d61e73f77808bd4f8f7` |
| Turbo C `TCC.EXE` | 2.01 | `19650666dcaa03e3f68efd9beeb57821ba4ed4d84c88a52b0e989bbfc97e07ba` |
| Turbo C `TLINK.EXE` | from 2.01 tree | `821a862e71bdfea977a2fddbe58d1127047eecbfe8d1b6eb82830cd8f493c140` |
| Open Watcom `wcc` | C16 1.9 | `0f5f6270a1c3d11d39673aec2e2752ab16c21e2cd3707a4b5f91e6f42d6d3fa5` |
| Open Watcom `wdis` | 1.9 | `8e6bb6f6d9425f11c2012a774f5d8143f437e26f357d65b2346a8fe08e7cb2db` |

Open Watcom was obtained from the Nixpkgs `open-watcom-bin` package. The Turbo
C installations are local archives outside this repository.

## Matrix

Turbo C 2.00 and 2.01 were tested in small, medium, compact, large, and huge
models with `-O -Z`. Turbo C 2.01 also covered unoptimized, `-O`, and `-Z`
variants in the small and huge models. This produced 16 configurations and 160
probe comparisons. The 50 same-model Turbo C 2.00/2.01 normalized listings were
identical.

Open Watcom C16 1.9 was tested with 386 code generation (`-3`) across all five
memory models, with and without `-ox`, and with the default Watcom register
calling convention or cdecl (`-ecc`). This produced 20 configurations and 200
probe comparisons. `wdis` listings retained the generated object-code bytes.

## Results

No Watcom configuration produced an exact mnemonic sequence or exact sequence
of encoded instruction bytes for any probe. The strongest aggregate Watcom
configuration was unoptimized huge model with its default register convention.
Its main positive signal is ABI shape: simple functions naturally receive
arguments in registers and can omit a stack frame.

Turbo C produced the same four-mnemonic sequence as the trivial
`segment_global_gate` probe in 11 configurations, but only one of four
canonicalized instructions matched because the segment/global operand and call
target form differed. No other probe supplied a close instruction match. Turbo
C's `-S` output does not include encoded bytes, so byte equality was not scored
for those listings.

An exact raw-byte search of 307 recovered BLOODPRG routines of at least eight
bytes over all 20 files in each Turbo C `TC/LIB` tree found zero matches for
both versions. For example, Turbo C 2.01's `CH.LIB` `_strlen` member is a
34-byte stack-argument routine, while BLOODPRG `0x002665` is a 19-byte
register-entry helper using `ES:DI` and `repne scasb`.

Representative best Watcom result per probe, ranked by canonical instruction
LCS and then mnemonic similarity:

| probe | best configuration | original/generated instructions | instruction LCS | mnemonic LCS | byte-line LCS |
| --- | --- | ---: | ---: | ---: | ---: |
| `far_strlen` | compact, unoptimized, register | 11/15 | 0.0909 | 0.6364 | 0.0909 |
| `field_offset` | compact, `-ox`, register | 8/23 | 0.3750 | 0.7500 | 0.3750 |
| `presentation_line_step` | medium, unoptimized, register | 60/62 | 0.2167 | 0.7333 | 0.2833 |
| `segment_global_gate` | compact, unoptimized, cdecl | 4/8 | 0.2500 | 0.7500 | 0.2500 |
| `string_equal_mixed` | huge, unoptimized, register | 16/32 | 0.4375 | 0.6250 | 0.5000 |
| `u32_sqrt_newton` | compact, unoptimized, register | 35/51 | 0.1714 | 0.7714 | 0.2286 |
| `vm_branch_stack_return` | compact, `-ox`, register | 8/11 | 0.1250 | 0.8750 | 0.1250 |
| `vm_c9_record_clear` | compact, unoptimized, register | 26/38 | 0.0769 | 0.5769 | 0.1154 |
| `vm_dic_lookup_result` | compact, unoptimized, cdecl | 21/42 | 0.2381 | 0.6190 | 0.2381 |
| `vm_special_slot_insert` | huge, `-ox`, register | 21/52 | 0.1905 | 0.7619 | 0.1905 |

## Interpretation

These results reject Turbo C 2.00/2.01 as the default assumed compiler for the
tested natural-C formulations and ABI shapes. They do not prove that no code in
the executable came from Turbo C or from hand-written assembly.

Open Watcom 1.9 is a better structural lead because its default register ABI
resembles many recovered entry conventions, but it is not an exact match and is
not a period-specific compiler identification. Continue to treat the original
assembly as the oracle, test older compiler versions when available, and accept
natural C one routine at a time only after its ABI and generated code are
accounted for.

## Reproduction

Use the direct runner commands in `re/compiler_corpus/README.md`, then emit the
comparison JSON with:

```sh
python3 re/tools/compiler_corpus.py --compare
```

Repeat the Turbo library search with:

```sh
python3 re/tools/compiler_corpus.py \
  --scan-library tc20=/path/to/tc20/TC/LIB \
  --scan-library tc201=/path/to/tc201/TC/LIB \
  --min-routine-bytes 8
```
