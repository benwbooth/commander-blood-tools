# Historical Compiler Codegen Matrix

This report records an initial compiler-in-the-loop matrix against ten natural-C
probes in `re/compiler_corpus`, plus focused recovered-source follow-ups.
Generated objects and listings remain ignored; the source corpus, runner,
commands, hashes, and conclusions are checked in.

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

In the initial matrix, no Watcom configuration produced an exact mnemonic
sequence or exact sequence of encoded instruction bytes for any probe. The
strongest aggregate Watcom configuration was unoptimized huge model with its
default register convention. Its main positive signal is ABI shape: simple
functions naturally receive arguments in registers and can omit a stack frame.

In the initial ten-probe matrix, Turbo C produced the same four-mnemonic sequence as the trivial
`segment_global_gate` probe in 11 configurations, but only one of four
canonicalized instructions matched because the segment/global operand and call
target form differed. No other probe supplied a close instruction match. Turbo
C's `-S` output does not include encoded bytes, so byte equality was not scored
for those listings.

## Recovered-source follow-up

Four later recovered functions provide a stronger positive result for Turbo C
2.01 in the medium memory model with `-O -Z`:

| routine | source operations | original bytes | Turbo OMF result |
| --- | ---: | ---: | --- |
| `0x00A73E list_d8c_bounds_init` | four direct word stores plus `ret` | 25 | exact opcode/immediate LEDATA shape; address-word FIXUPP records at offsets 2, 8, 14, and 20 |
| `0x00A744 list_d8c_wrap_bounds_reset` | three direct word stores plus `ret` | 19 | exact opcode/immediate LEDATA shape; address-word FIXUPP records at offsets 2, 8, and 14 |
| `0x00A2DD presentation_queue_finish` | two byte ORs, zero-word branch, near call, `ret` | 21 | exact LEDATA shape; global/call FIXUPP records at offsets 2, 7, 14, and 18 |
| `0x009F53 presentation_update_1fb2` | natural gated state updates plus six inline register saves/restores | 45 | exact LEDATA shape; global/call FIXUPP records at offsets 5, 11, 15, 22, 27, 33, and 38 |

The object payloads contain zero placeholders where the original has
`0x0D60`, `0x0D62`, `0x0D64`, and `0x0D66`; the adjacent FIXUPP records cover
exactly those words. Binding the external globals to their recovered data
offsets therefore supplies the original instruction bytes. The first two entrypoints
share the tail beginning at `0x00A744`, so reproducing their overlapping linked
layout remains a separate translation-unit/linker problem.

Open Watcom 1.9 medium instead materializes zero and `0xFFFF` in AX before
storing, producing 18-byte and 15-byte routines that clobber AX and flags. It
does not match these routines. Turbo C and Watcom medium both preserve the core
signed subtract/compare order of the `0x008269` and `0x008295` hit tests, but
neither reproduces their live SI/BP pointer ABI or the latter routine's carry
return from ordinary C declarations.

The `0x0030CD` text-width probe is a useful near miss. Open Watcom 1.9 medium
`-3 -ox` emits 60 bytes and 31 instructions versus the original 57 bytes and 28
instructions, with the same two-table selection, byte lookup, word accumulation,
subtract-two result, preserved BX/CX/SI/DI, and far return. It still passes the
text pointer in AX and selector in DX, addresses tables through DS, and uses
MOVZX loads rather than the original AX selector, DS:SI text, GS-prefixed XLAT
and indexed advance read. Turbo C 2.01 medium uses stack arguments and a stack
frame, so neither result reproduces the routine boundary.

For `0x007CB4`, Turbo C 2.01 and Open Watcom 1.9 medium preserve the recovered
signed table index, big-endian row construction, set-bit-only writes, fixed
framebuffer offset, and 320-byte row advance. They emit 51 and 40 instructions
respectively, versus the original 26, and do not reproduce its compact
LODSW/XCHG/shift-until-zero/LOOP form or register effects. The natural source is
therefore behaviorally verified but remains a codegen mismatch.

For `0x00A117`, four direct-execution cases confirm the GS bit-0 gate and the
exact 384-byte copy from caller ES:0x5251 to ES:0x5851 under the C ABI's clear
direction flag. Separate DS and GS decoy buffers prove the copy is ES-local.
The cases also verify restored DS/SI, copied-path CX=0 and DI=0x59D1, the other
preserved registers and segments, and the final TEST flags. Open Watcom 1.9
medium recognizes a fixed 384-byte aggregate assignment as an inline string
copy. Its size-optimized 386 build emits guarded `REP MOVSW` in 25 bytes and
uses `PUSH DS` / `POP ES`. Its 27-byte speed build has the exact original
13-mnemonic sequence, but with different operands: it saves ES, sets ES through
AX, copies 192 words, and restores CX/DI. The original is 29 bytes and instead
temporarily sets DS from the caller's ES, uses 96 iterations of `REP MOVSD`,
preserves ES, and leaves CX/DI clobbered. Turbo C 2.01 medium calls its far
`SCOPY@` runtime. This is a verified fixed-size copy with a confirmed segment
and register ABI boundary, not an exact compiler match.

For `0x00A2DD`, six direct-execution cases confirm the unconditional queue-state
bit 0 update, the zero-count-only bit 1 update and close-helper call, preservation
of the other state bits and storage, helper-derived BX/CX effects, and final
flags. Turbo C 2.01 medium `-O -Z` emits the exact six mnemonics and 21-byte
LEDATA payload. Its address/call words are zero only at FIXUPP-covered offsets
2, 7, 14, and 18. Binding the repeated state external to DS:0x0D5F, the count
external to DS:0x0D9A, and the near helper to `0x00A141` produces the original
bytes exactly. Open Watcom 1.9 also emits 21 bytes, but reverses the branch and
tail-jumps to the close helper instead of retaining the original call/return
shape.

For `0x00A141`, seven direct-execution cases cover zero and reserved-handle
skips, successful and failed DOS closes, the clear-before-interrupt ordering,
the unconditional post-close bounds reset, DS versus GS decoys, every register
and segment effect, and the final `XOR CX,CX` flags. Open Watcom 1.9 targeting
8086 in the medium model (`-0 -ox -mm`) keeps the handle in BX, emits the direct
`INT 21h` close and bounds-reset call, and retains 10 of the original 11
mnemonics in order. The ABI-honest source emits 14 instructions and 31 bytes
versus the original 11 and 30: it saves/restores ES, uses equivalent `TEST BX,BX`
instead of `OR BX,BX`, and lowers the zero assignment through `XOR AX,AX` plus a
store. The last choice changes AL before the interrupt, so the build is not
classified as exact. An exploratory 29-byte form omitted the ES save only after
declaring ES clobbered at the C boundary; that declaration was rejected because
the original preserves ES. Turbo C 2.01 medium preserves the natural branch and
direct zero-store shape but saves SI and calls the far CRT `close` routine rather
than issuing the interrupt inline.

For `0x009F53`, eight direct-execution cases cover the inactive gate, both
redraw outcomes, low-byte versus high-byte ship flags, nonzero and zero queue
counts, the reserved-handle close path, request-bit preservation, and final
TEST/AND flags. GS decoys prove that this far entry accesses the shared game
data through DS, and every register and segment is preserved across the nested
`0x00A2DD` call. Turbo C 2.01 medium `-O -Z` emits the exact 45-byte LEDATA
payload from the natural state-machine body plus three inline PUSH and three
inline POP instructions. The seven FIXUPP records cover the gate at offsets 5
and 33, near call at 11, ship-state low byte at 15, redraw byte at 22, active
line at 27, and request byte at 38. Binding those externals to DS:0x1FB2,
0x00A2DD, DS:0x24F3, DS:0x27D8, DS:0x6788, and DS:0x67AA supplies every original
byte. The inline instructions are limited to the nonstandard AX/BX/CX
preservation envelope; the function logic itself remains natural C.

For `0x009F80`, eight direct-execution cases confirm that AX is an unsigned
index into the four-byte table at DS:0x1FB5 and BX receives the entry's first
word, a near presentation-line record pointer. The cases cover 16-bit table
offset wrap, DS selection against ES/GS decoys, preservation of AX and every
other register, and all six arithmetic flags left by the fourth ADD. Each of
the five callers immediately consumes BX; their next instructions are MOV,
TEST, AND, AND, and PUSH, so none depends on the incidental flags. Open Watcom
1.9 medium targeting 8086, with a source-level `#pragma aux` declaration for
the recovered AX-argument/BX-result ABI, emits `MOV BX,AX; SHL BX,1; SHL BX,1;
MOV BX,[table+BX]; RET`: five instructions and 11 bytes versus the original
seven instructions and 14 bytes. The function body remains the natural
`table[index].record` expression. Turbo C 2.01 medium instead passes the index
on the stack and returns the pointer in AX. The natural source is therefore a
drop-in logical match under the Watcom ABI declaration, but not an exact code
shape or an isolated Borland replacement.

For `0x00A38E`, six direct-execution boundary cases confirm the natural queue
wrap source and show that both direct callers ignore its incidental AX/SI/CX
results. Open Watcom 1.9 medium is closest at 16 instructions and 43 bytes,
versus the original 11 instructions and 31 bytes; placing byte count first
recovers its AX argument and final subtraction. The generated cursor remains in
DX/BX rather than SI, and ordinary C stores do not lower to the original
XOR/XCHG head clear. Turbo C 2.01 medium emits 21 instructions and uses stack
arguments. Keeping the iteration-count word non-volatile avoids a duplicate
Watcom store while retaining the original store-before-increment order.

For `0x00A3AD`, eight direct-execution cases establish a carry-clear queue-room
predicate and disprove the older empty-check label. Open Watcom 1.9 medium
preserves 13 of the original 14 mnemonics in order and reproduces every branch
and arithmetic operation in the natural C body. Its complete function is 26
instructions and 53 bytes versus the original 14 instructions and 35 bytes,
because it moves the request from AX, saves registers, and materializes the
Boolean result with SETBE/XOR instead of returning the final comparison flags.
Turbo C 2.01 medium emits 38 instructions with a stack argument and frame.

For `0x00A3D0`, eight direct-execution boundary cases confirm the natural
queue-consumption source, including the distinction between discarded overflow
from `tail + 2` and wrapping overflow from the following entry-size add. Turbo C
2.01 medium emits 40 instructions and Open Watcom 1.9 medium emits 32, versus
the original 20. Both retain the two unsigned wrap tests and counter rollover,
but neither emits the original LES SI/LODSW pointer advance or register effects.
An exploratory non-volatile Watcom build shortened the routine to 29
instructions but reordered shared-state accesses, so it is not accepted as the
recovered source formulation.

For `0x00A40B`, exhaustive direct execution over all 256 state-byte values
confirms that ZF is set exactly for zero and one and that all registers are
preserved. Open Watcom 1.9 and Turbo C 2.01 retain the natural source's two
comparisons, but emit eight and ten instructions respectively versus the
original four. The extra instructions materialize a C Boolean in AX; the
original sole caller consumes ZF directly, making this a confirmed flag-ABI
boundary rather than an unresolved algorithm.

For `0x00A634`, exhaustive direct execution over all 256 state-byte values
confirms that ZF is set exactly when GS:0x0B17 bit 0 is clear. It also verifies
the GS-versus-DS selection, PF/CF/SF/OF results, preservation of IF/DF, and all
register and segment effects. The sole caller immediately branches on JE and
does not consume a register result. Open Watcom 1.9 emits four instructions and
11 bytes for the core natural Boolean using `TEST`, `SETNE`, and `XOR`; Turbo C
2.01 emits a six-instruction branch sequence. Neither returns the original TEST
flags or preserves AX while materializing a Boolean, and neither ordinary data
declaration models the helper's temporary GS-to-DS selection. This is therefore
a verified logical predicate with a confirmed flag and segment ABI boundary.

For `0x00A734`, eight direct-execution cases confirm the two wrapping queue
updates, register preservation, and unconditional carry clear. Open Watcom 1.9
medium compiles the corrected void function to the exact two direct memory ADD
forms followed by RET: nine generated bytes versus ten original bytes, modulo
the two external-address relocations. The sole missing instruction is the
original CLC immediately before RET. Both callers ignore flags and registers,
so this is a confirmed one-instruction assembly ABI boundary; the prior natural
C `return 1` was not supported by caller behavior. Turbo C 2.01 instead uses a
stack argument and emits eight instructions.

For `0x00A757`, five direct-execution cases confirm the ordered far-pointer
field initialization, five cleared queue words, wrap-limit copy, adjacent-word
preservation, final AX, and far return. Open Watcom 1.9 medium emits the same 12
mnemonics in the same order and the same 33-byte length as the original. Eleven
instruction encodings match modulo external-address relocations; its equivalent
`XOR AX,AX` is encoded as `31 C0` instead of the original `33 C0`. All three
callers execute PUSH CS before a near CALL, independently confirming the
natural candidate's far-return declaration. Turbo C 2.01 emits 13 instructions
because it saves SI for the base-segment temporary.

For `0x00A7E6`, six direct-execution cases under the C ABI's clear direction
flag confirm four sequential forward word copies, deterministic overlap
behavior, 16-bit SI/DI offset wrapping, ES=DS, preserved AX/BX/CX/DX/BP, and
preserved flags. The caller at `0x00A32F` immediately
executes `MOVSB`, proving that the helper's eight-byte pointer advancement is
part of its assembly boundary and forms a nine-byte record copy there. Open
Watcom 1.9 medium with an explicit DI/SI ABI declaration emits a fixed eight-byte
structure assignment as seven instructions and 12 bytes using `REP MOVSW`.
Targeting 8086 for speed with stack checks disabled instead emits four unrolled
`MOVSW` instructions and a near return in nine bytes. That is the closest tested
natural formulation, but its four-byte `MOV AX,DS` / `MOV ES,AX` setup replaces
the original two-byte `PUSH DS` / `POP ES` pair and clobbers AX. Turbo C 2.01
medium calls its far `SCOPY@` runtime. The routine is therefore classified as a
fixed-record compiler/helper boundary with a behaviorally verified natural C
body, not as exact compiler-generated source.

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

The initial matrix rejects Turbo C 2.00/2.01 as a blanket default for those ten
tested formulations and ABI shapes. The exact medium-model initializer results
show that Turbo C 2.01 remains a viable generator for at least some recovered
translation units. They do not identify the whole executable's compiler or
exclude hand-written assembly.

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
