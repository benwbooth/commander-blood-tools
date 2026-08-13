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

Five later recovered functions provide a stronger positive result for Turbo C
2.01 in the medium memory model with `-O -Z`:

| routine | source operations | original bytes | Turbo OMF result |
| --- | ---: | ---: | --- |
| `0x00A73E list_d8c_bounds_init` | four direct word stores plus `ret` | 25 | exact opcode/immediate LEDATA shape; address-word FIXUPP records at offsets 2, 8, 14, and 20 |
| `0x00A744 list_d8c_wrap_bounds_reset` | three direct word stores plus `ret` | 19 | exact opcode/immediate LEDATA shape; address-word FIXUPP records at offsets 2, 8, and 14 |
| `0x00A2DD presentation_queue_finish` | two byte ORs, zero-word branch, near call, `ret` | 21 | exact LEDATA shape; global/call FIXUPP records at offsets 2, 7, 14, and 18 |
| `0x009F53 presentation_update_1fb2` | natural gated state updates plus six inline register saves/restores | 45 | exact LEDATA shape; global/call FIXUPP records at offsets 5, 11, 15, 22, 27, 33, and 38 |
| `0x008713 nav_choice_handler_0` | bit gate, word copy, word/byte constants, `ret` | 25 | exact LEDATA shape; address-word FIXUPP records at offsets 2, 8, 11, 15, and 21 |

The two list-bound initializer payloads contain zero placeholders where the original has
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

For `0x00A20C`, seven direct cases cover an existing active entry, an empty
queue, ordinary incomplete/exact/excess extents, the `0x6D6D` link-marker
bypass, both storage-segment choices, and far-pointer offset wrap. The patched
activation boundary receives exactly the original AX extent, ES:SI payload
pointer, and BP storage segment. Open Watcom 1.9 medium emits 32 instructions
and 79 bytes; Turbo C 2.01 medium emits 56 instructions, versus 18 instructions
and 52 bytes in the original. The natural far-pointer decision tree is
verified, but Boolean materialization and typed parameter passing replace the
original carry result and register-call boundary.

For `0x00A240`, twelve direct cases execute the routine's actual indirect far
callback and cover both audio-phase threshold boundaries, signed-negative
phase correction, callback-result wrap, all three software-clock fallback
gates, positive and negative tick deltas, the `0x8000` edge, zero threshold,
and the due path's deliberate second tick read. A source-level subtraction and
negation produces the original `SUB AX,4000h` / `NEG AX` pair. Open Watcom 1.9
medium emits 39 instructions and 103 bytes; Turbo C 2.01 medium emits 52
instructions, versus 31 instructions and 81 bytes in the original. Watcom
retains the three bit tests, indirect far call, 16-bit correction arithmetic,
threshold decisions, and ordered clock stores, but uses DX for the preserved
phase/delta and materializes the logical result in AX instead of returning it
through carry.

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

For `0x00A778`, four direct cases patch only the palette-parser call and prove
that the routine discards the queue head offset, retains its segment, replaces
SI with the `pl` payload offset at DS:0x0D9E, and returns the parser's ES:SI
stream result. The queue initializer at `0x00A757` is the only recovered writer
of DS:0x0D8E and always stores the queue-buffer base segment, supporting the
natural buffer-relative expression. With the proven parser and wrapper ES:SI
ABI declared through `#pragma aux`, Open Watcom 1.9 medium emits 8 instructions
and 18 bytes; Turbo C 2.01 medium emits 9 instructions, versus 4 instructions
and 12 bytes in the original. Watcom preserves AX and the returned ES:SI, but
materializes the buffer symbol's segment through AX and adds the payload offset
to its relocatable base instead of using the original `LES` plus replacement
`MOV SI` pair.

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
word, a near resource-descriptor pointer. The `0x009F8E` consumer proves that
the descriptor begins with byte flags, a mutable variant byte, and its filename
at offset two. The cases cover 16-bit table
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

For `0x009F8E`, seven direct-execution cases cover banked, embedded, and
external-file resources, along with open, initial-read, and body-read failures.
The cases patch only the path-builder, DOS, and staged-read call boundaries;
the original close, list initialization, bounds reset, descriptor lookup, and
palette routines execute unchanged. They verify the extent word is used only
for ring-wrap detection, palette data begins immediately after that word unless
the record wraps to offset zero, `0xFF` metadata padding is skipped, and both
32-bit absolute/remaining range pairs use the recovered relative offsets. Open
Watcom 1.9 medium compiles the natural body to 182 instructions and 514 bytes,
and Turbo C 2.01 medium emits 206 instructions, versus 103 instructions and 309
bytes in the original. The excess is primarily the conventional Boolean and
pointer interfaces replacing the original AX,
BX, ES:SI, and carry conventions, so this is a behaviorally verified natural C
body with unresolved assembly boundaries rather than an exact codegen match.

For `0x00A0C3`, five direct-execution cases confirm the complete palette-block
loop, including immediate termination, a nonterminating zero-count block,
multiple destination ranges, the nested `0x00A117` render-state copy, 16-bit
metric underflow, and the source/destination segment split. Open Watcom 1.9
medium targeting 8086 emits 60 instructions and 131 bytes from the natural
far-pointer form; Turbo C 2.01 medium also emits 60 instructions, versus 44
instructions and 84 bytes in the original. Both compilers preserve the logical
loop but use conventional far-pointer arguments and explicit byte copies. They
do not reproduce the original ES:SI stream input/result, DS/ES swap, LODSW,
REP MOVSB, or register-preservation boundary. This is therefore accepted as a
behaviorally verified natural C body with a confirmed assembly ABI boundary,
not as exact compiler-generated source.

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

For `0x00A622`, six direct-file cases execute the original `0x00A664` transport
and its `0x00A734` shared tail without patching either routine. They verify
failure with handle zero, ordinary and wrapping destination reads, 32-bit
source offset/remaining carry and borrow, and retries after both a short read
and a carry-set short response, proving that AX alone controls retry. On
success the routine returns the extent in AX, the post-read cursor in ES:SI,
and carry clear; both callers consume those exact results. The natural C
candidate exposes logical success plus extent/cursor output parameters. Open
Watcom 1.9 medium emits 23
instructions and 47 bytes, and Turbo C 2.01 medium emits 32 instructions,
versus six instructions and 18 bytes in the original. The algorithm is
verified, but the compact carry/AX/ES:SI result remains an assembly ABI boundary.

For `0x00A642`, six direct-file vectors execute the real `0x00A757` queue init,
`0x00A622` extent read, and `0x00A664` body read. They cover initial and body
failure, zero and `0xFFFF` wrapped body lengths, header relocation, repeated
short reads even when carry is set, and all source/queue accounting. Open
Watcom 1.9 medium emits 32 instructions and 80 bytes; Turbo C 2.01 medium emits
48 instructions. The original has a 12-instruction, 34-byte unique prefix and
then physically falls through the complete `0x00A664` body, making its indexed
span 100 instructions and 252 bytes. The natural helper composition is
behaviorally equivalent, but output parameters, logical returns, and a normal
call replace the original AX/ES:SI/carry ABI and shared-tail placement.

For `0x00A664`, nine direct cases cover all three source backends and execute
the common `0x00A734` queue tail. The EMS path maps four consecutive logical
pages to physical pages zero through three even for a zero-byte request, then
calls the far memmove at `01CE:0B93` from `source_offset & 0x3FFF`. The XMS path
builds the standard 16-byte function-0x0B move descriptor and rounds an odd
physical move length up, while source and queue accounting advances only by
the requested count. The direct-file path always seeks before reading, retries
while AX is below CX, and ignores DOS carry; a crafted carry-set AX above CX is
accepted and then masked by the common carry clear. Open Watcom 1.9 medium
emits 133 instructions and 386 bytes, and Turbo C 2.01 medium emits 174
instructions, versus 88 instructions and 218 bytes in the original. The
natural body preserves the backend selection and accounting, but the original
interrupts, XMS callback, register ABI, carry result, and shared-tail placement
remain explicit integration boundaries.

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

For `0x00AD96`, five direct cases execute both forms of this outlined local
helper from `0x00AB25`. They verify the low-byte-only row decrement, preserved
high byte, zero-to-255 underflow, 16-bit 320-byte offset wrap, CX/DI reloads,
and the last-row path that discards the helper return address and unwinds the
enclosing decoder's SP/BP/DS frame. The natural six-byte state structure keeps
the same three fields and returns a Boolean so the eventual `0x00AB25` C body
can perform a normal return. Open Watcom 1.9 medium emits 13 instructions and
27 bytes; Turbo C 2.01 medium emits 16 instructions, versus 11 instructions
and 25 bytes in the complete original span. The close size does not make the
code drop-in: the natural form materializes a Boolean instead of performing a
nonlocal stack unwind.

For the four hardware leaves at `0x000CC0`, `0x002DD3`, `0x002F90`, and
`0x002FA6`, 19 direct-execution vectors now hook the actual `INT`, `IN`, and
`OUT` instructions. They verify the saved video-mode BIOS argument, every CMOS
transaction and duplicated-byte store, the complete 768-byte palette stream
including a 64 KiB SI wrap, all 768 DAC-clear writes, and the recovered
register boundaries. The candidates use normal DOS `int86`, `inportb`, and
`outportb` facilities rather than modeled registers or memory.

Open Watcom 1.9 targeting 8086 medium model compiles the four actual candidate
files without warnings at 38, 25, 30, and 23 bytes, versus 11, 15, 22, and 21
bytes in the original routines. A source-level pragma gives the palette writer
its recovered DS:SI argument, but natural indexed C remains an 18-instruction
scalar loop rather than `REP OUTSB`. The DAC clear reaches the original
14-instruction count in both Watcom and Turbo C 2.01, but both choose BX plus
`DEC/JNE`, reload port 0x3C9, and clobber AX instead of preserving AX/CX/DX
around a CX plus `LOOP` body. The RTC candidate inlines real port operations,
but Watcom uses DX-selected ports, DS storage, and 13 instructions instead of
the immediate-port, CS-store 8-instruction original. Both compilers lower the
natural `int86` video-mode call to 17 instructions instead of the original six
with a direct `INT 10h`. These are behaviorally verified natural functions with
explicit compiler and hardware ABI boundaries, not accepted drop-in bodies.

The adjacent interrupt-wrapper slice covers `0x00093B`, `0x000986`,
`0x000B32`, `0x000D4A`, `0x000D61`, and `0x00267D`. Its 279 direct vectors
include exhaustive packed-BCD inputs and exercise RTC, MSCDEX, paired mouse,
DOS character-output, and BIOS keyboard interrupt contracts. The mouse vectors
prove that the old `mouse_set_hrange` label is incomplete: AX/BX supply the
horizontal range, CX/DX supply the vertical range, and the routine invokes
INT 33h functions 7 and 8 in order.

Open Watcom 1.9 compiles all six actual candidates without warnings. The BCD
helper's AX-input/AL-output pragma reduces it to 13 instructions/22 bytes,
versus 9/17 original, and lets the RTC caller avoid a stack argument. The RTC,
MSCDEX, and mouse candidates are 45, 53, and 69 bytes versus 21, 16, and 23
original because `int86` marshals a register structure instead of emitting the
direct interrupt and selective save/restore sequence. The natural DOS string
loop is close at 15 instructions/29 bytes versus 14/20, but calls `bdos` and
advances SI. The natural keyboard wrapper is 7 instructions/20 bytes versus
8/16, but calls and tail-jumps to `_bios_keybrd` instead of consuming BIOS ZF
around direct INT 16h instructions. These candidates retain recovered logic;
their remaining differences are explicit runtime and register-ABI boundaries.

The sprite-slot state slice covers `0x0041D1`, `0x00420D`, and `0x0042CD`
with 21 direct-execution vectors. They prove the 32-byte `GS:0x6212` record
stride, low-byte flag edits with high-byte preservation, independent position
updates, extent comparison/update ordering, and preservation of unrelated
record bytes and incoming registers. The extent vectors additionally prove that
the original loads its source-dimensions far pointer from inherited
`SS:BP+4`; the natural API makes that hidden context a typed pointer argument.

A word/byte union for the flag field materially improves generated code by
letting Open Watcom keep the full flag word in `AX` while editing `AL`. Under
8086 medium `-ox`, the actual candidates compile without warnings to 30, 50,
and 63 bytes, versus 31, 51, and 73 original bytes. The corresponding probes
contain 17/25/30 instructions versus 16/23/32. These close sizes do not erase
the remaining boundaries: generated globals use the default data segment,
Watcom follows its own volatile-register convention, and `0x0042CD` receives a
normalized `ES:SI` pointer rather than recovering the inherited `SS:BP+4`
context itself. The `-2` and `-3` targets reproduce the original immediate
five-bit shifts, but omit more of the original preservation sequence and emit
14/21/26 instructions and 27/45/58 bytes.

The adjacent range slice covers `0x004240` and `0x0043F7` with nine more direct
vectors. The first routine is now named `sprite_slot_range_mark_dirty`: its old
`range_count` label described only the `last-first+1` setup and omitted the
actual inclusive slot walk and active-byte state transition. The commit vectors
prove both the one-shot global clip snapshot and the ordinary range walk,
including the exact low-byte bit-1 plus bit-0 geometry-copy gate, full snapshot
flag-word clear, `0xffff` dirty-list sentinel, GS ownership, and complete
register preservation.

Open Watcom 1.9 8086 medium compiles the actual candidates without warnings to
43 and 105 bytes, versus 45 and 120 original. Their probes contain 24 and 50
instructions versus 27 and 51. The commit control-flow shape is especially
close, but natural aggregate/member assignment becomes four `MOVSW` or word
load/store pairs where the binary uses two dword copies. GS data placement,
packed-EBP range transport, and the original save/restore sets remain explicit
ABI boundaries.

The dirty-range renderer at `0x004471` adds six direct vectors. They prove that
the first signed-negative dirty-list edge exits immediately; otherwise slots
are visited from `BX` down through `AX`, inactive slots still lose dirty bit 1,
and active slots copy and test every rectangle using signed exclusive edges.
The vectors also corrected an earlier transcription error: the dispatch mode is
`(flags >> 2) & 7` (state bits 2..4), while bit 1 is the dirty flag. Flip flags
come from bits 5/6, and the indirect call sequence follows descending slot
order.

Open Watcom 1.9 compiles the actual candidate without warnings. Its closest
tested `-3 -ox -mm` probe has 77 instructions/190 bytes versus 75/177 original;
the 8086 and 286 probes have 88/201 and 85/197. Watcom retains the descending
pointer, signed comparisons, sentinel loop, and dispatch structure, but uses
stack locals, default-DS globals, and `REP MOVSW`. A typed function convention
does reproduce the binary's DI callback argument and preserve-all contract. The
binary instead packs the range through EBP, owns state in GS and dispatch
scratch in CS, and performs two `MOVSD` copies.

The first real dispatch target, raw-transparent blitter `0x004536`, has ten
direct framebuffer vectors. They prove direct nonzero source copies, transparent
zero skips, destination-as-index remapping through both `GS:0x5F11` and
`GS:0x6011`, signed clipping on every edge, all flip combinations, and complete
register preservation. They also lock down a non-obvious pointer dependency:
vertical clipping advances `SI`, and the horizontal setup subsequently reloads
the x-origin from `[SI+4]` at that advanced address. The natural source keeps
that mutable cursor instead of replacing it with an immutable frame-header
access.

Open Watcom compiles the actual `0x004536` candidate without warnings; after
correcting destination-x initialization, `-3 -ox -mm` emits 422 bytes versus
384 original. Standalone 8086/286/386 probes emit
161/154/157 instructions and 427/410/422 bytes versus 166/384. Their far-pointer
normalization keeps slot state and globals in DS while loading frame data in ES;
the binary enters with DS=ES=GS, retains the slot in ES/GS, and loads frame data
into DS. The original also receives its already-computed rectangle in
AX/BX/DX/BP and uses register-shaped row loops, while the natural function
derives that context from the typed slot.

RLE-transparent blitter `0x0046BC` has ten direct framebuffer vectors. They
prove direct-copy and both destination-remap tables including selector 3, zero
repeat/literal transparency, repeat and literal runs independently crossing
either clip boundary, complete encoded-row skipping in both vertical directions,
ordinary and noncanonical flips, the post-skip `[SI-4]` x-origin reload, CS and
remap scratch, source/framebuffer ownership, and complete register preservation.
The vectors remain within the same one-token clip-span contract as mode 3.

Open Watcom compiles the actual `0x0046BC` candidate without warnings; `-3 -ox
-mm` emits 673 bytes versus 1260 original. Standalone 8086/286/386 probes emit
257/249/244 instructions and 679/665/673 bytes versus 603/1260, while Turbo C
2.01 emits 297 instructions. The natural function shares one streaming decoder
and one optional-remap write loop. The binary instead duplicates leading skip,
visible decode, trailing skip, and row traversal across direct/remapped and
forward/reverse paths, using `XLATB`, `REP STOSB`, and scalar direction-specific
copies.

Raw-opaque blitter `0x004BA8` has ten direct framebuffer vectors over the
same clipping geometry. They prove that source zeroes overwrite the destination,
high-byte remap selectors leave `GS:0x524B` unchanged, and the forward path's
dword-only, dword-plus-tail, and byte-only width classes agree with the binary.
The remaining vectors cover every ordinary flip combination, signed origins,
the advanced-source x-origin reload, source/framebuffer ownership, and complete
register preservation. Two noncanonical flip-byte cases prove that clipping and
initial vertical placement test bit 0 while final traversal tests the full byte.

Open Watcom compiles the actual `0x004BA8` candidate without warnings; `-3 -ox
-mm` emits 361 bytes versus 302 original. Standalone 8086/286/386 probes emit
133/127/135 instructions and 350/340/361 bytes versus 135/302, while Turbo C
2.01 emits 181 instructions. Watcom preserves the signed clipping and computed
forward/reverse traversal, but normalizes segment ownership and emits scalar
far-pointer copies. The binary instead receives AX/BX/DX/BP bounds from its
caller and specializes forward rows with `REP MOVSD` plus `REP MOVSB`.

RLE-opaque blitter `0x004CD6` has ten direct framebuffer vectors. They prove
opaque zero writes, repeat and literal runs independently crossing either clip
boundary, complete encoded-row skipping in both vertical directions, ordinary
and noncanonical flips, the post-skip `[SI-4]` x-origin reload, CS stride/clip
scratch, source/framebuffer ownership, and complete register preservation. An
adversarial token spanning both clips and the entire visible interval exposed
an original state-machine assumption; the natural run/viewport intersection is
defined for that case, but the acceptance vectors stay within the binary's
valid-stream contract.

Open Watcom compiles the actual `0x004CD6` candidate without warnings; `-3 -ox
-mm` emits 589 bytes versus 652 original. Standalone 8086/286/386 probes emit
228/225/219 instructions and 600/599/589 bytes versus 307/652, while Turbo C
2.01 emits 320 instructions. The natural function is shorter because it shares
one run-interval decoder across directions. The binary duplicates leading-skip,
visible-copy, and trailing-skip state machines and uses `REP STOSB`, `REP MOVSB`,
and reverse scalar copies, in addition to its CS scratch and register-fed bounds.

Scaled-transparent blitter `0x004F62` has ten direct framebuffer vectors. They
prove separate zero-width and zero-height exits, one-to-one and fractional
up/down scaling, zero-key transparency, all-edge fixed-point clip advancement,
signed-negative origins, zero source dimensions, source/framebuffer ownership,
and complete register preservation. Nonzero frame offsets, arbitrary flip bytes,
remap selectors/tables, and RLE scratch remain unchanged, proving this mode does
not consume the metadata used by the other blitters.

Open Watcom compiles the actual `0x004F62` candidate without warnings; `-3 -ox
-mm` emits 407 bytes versus 312 original. Standalone 8086/286/386 probes emit
152/146/147 instructions and 409/398/407 bytes versus 128/312, while Turbo C
2.01 emits 183 instructions. The original uses inline operand-size-prefixed
32-bit `DIV`/`MUL`, splits each fixed-point accumulator into register words, and
receives draw bounds in AX/BX/DX/BP. Watcom instead calls unsigned-long runtime
helpers and derives the normalized bounds from the typed slot.

Dispatch modes 5, 6, and 7 at `0x00509A..0x00509C` are exact one-byte no-ops.
The direct oracle executes each `RET` with a synthetic near return address and
proves that only SP advances by two; all general registers including their upper
halves, segments, flags, and adjacent stack bytes are preserved. Open Watcom
`-3 -ox -mm` compiles each typed empty DI-argument callback to the exact original
`C3` byte. Turbo C 2.01 retains an unnecessary BP frame and emits four
instructions, so Watcom is the exact result for this slice.

Dirty-rectangle copy `0x00509D` has eight direct framebuffer vectors. They
prove the bit-0 gate and immediate signed-sentinel exits, exclusive rectangle
edges, byte-only, aligned and unaligned dword, leading-byte, and trailing-byte
width classes, multiple records, source/destination/list ownership, and complete
register/segment preservation. The assembly discards the offset words after
loading both far framebuffer pointers, establishing the runtime precondition
that `GS:0x5221` and `GS:0x5229` point at segment offset zero.

Open Watcom compiles the actual `0x00509D` candidate without warnings; `-3 -ox
-mm` emits 158 bytes versus 231 original. Standalone 8086/286/386 probes emit
68/64/64 instructions and 160/154/158 bytes versus 111/231, while Turbo C 2.01
emits 58 instructions. The natural function uses one scalar byte-copy loop. The
binary instead splits rows by source alignment and width remainder, then uses
four specialized combinations of `REP MOVSD` and `REP MOVSB`.

Resource-release gate `0x005288` has six patched-callee direct vectors. They
prove clear, unrelated, individual, and combined loaded-flag paths, 16-bit
`handle * 8` wrap, AX propagation, read-only handle-table access, full register
and segment preservation, and the exact push-CS/near-call stack consumed by the
callee's far return.

Open Watcom compiles the actual `0x005288` candidate without warnings; `-3 -ox
-mm` emits 11 instructions/26 bytes versus 9/20 original. Standalone 8086/286/
386 probes emit 15/11/11 instructions and 30/26/26 bytes, while Turbo C 2.01
emits 16 instructions. The natural conditional call is structurally close, but
Watcom relocates the abstract handle table through DS and emits a direct far
call; exact integration still needs original FS placement and same-segment
push-CS/near-call lowering.

Resource compactor `0x00529C` has six patched-`far_memmove` direct vectors.
They prove low-two-bit clearing, 32-bit free-byte accounting, floor(size/16)
paragraph accounting, first/middle/last resident-list removal, arbitrary signed
terminators, following-entry segment shifts, moved-size accumulation including
zero-sized followers, exact compaction pointers/data, and complete register and
segment preservation.

Open Watcom compiles the actual `0x00529C` candidate without warnings; `-3 -ox
-mm` emits 69 instructions/158 bytes versus 55/120 original. Standalone 8086/
286/386 probes emit 71/69/69 instructions and 162/158/158 bytes, while Turbo C
2.01 emits 95 instructions. The natural implementation preserves the typed
resource table, resident list, accounting, and conditional `far_memmove` logic.
Exact integration still needs the original FS/GS placement, `REPNE SCASW`
search, packed 32-bit register operations, and DS/ES segment construction at
the far-call boundary.

Resource-handle resolver `0x005320` has six direct vectors. They prove clear and
unrelated unloaded flags, each and combined loaded bits, 16-bit `handle * 8`
wrap, read-only table access, the unloaded `AX=0` plus unchanged `DS:SI` result,
the loaded `AX=1` plus `DS=segment`/`SI=0` result, unrelated-register
preservation, and the far-return boundary.

Open Watcom compiles the actual six-byte structured-result candidate without
warnings; `-3 -ox -mm` emits 32 instructions/70 bytes versus 12/28 original.
Standalone 8086/286/386 probes emit 36/32/32 instructions and 75/70/70 bytes,
while Turbo C 2.01 emits 31 instructions. Both compiler families implement a
hidden structure-return convention. That is suitable for natural integrated C,
but it cannot reproduce the binary's simultaneous status in `AX` and conditional
pointer in `DS:SI`; exact binary integration requires a narrow ABI adapter plus
FS table placement.

Resource dword getter `0x00533C` has eight deterministic direct vectors. They
prove zero, high, and wrapped handles, 16-bit `handle * 8` indexing, full dword
loads, read-only table ownership, preservation of BX and every unrelated
register/segment, the `RETF` boundary, and the architecturally defined
CF/PF/ZF/SF results left by `SHL AX,3`.

Open Watcom compiles the actual candidate without warnings; `-3 -ox -mm` emits
7 instructions/16 bytes versus 6/13 original. Standalone 8086/286/386 probes
emit 10/7/7 instructions and 19/16/16 bytes, while Turbo C 2.01 emits 10
instructions. Watcom preserves the original mnemonic set and AX input, but uses
DS table placement, shifts BX, and returns the natural 32-bit value in DX:AX.
Its C16 pragma parser rejects EAX register names, so exact integration needs a
narrow FS/EAX adapter rather than contaminating the natural field getter with
inline assembly.

VM special-slot helpers `0x005FD8` and `0x005FF6` have six removal and seven
insertion vectors. They prove absent, first/middle/last, duplicate, full-list,
and zero-owner paths; first-match removal; duplicate-before-empty insertion;
exact mutations; carry-only status; near-return boundaries; and complete
register/segment preservation. Distinct arrays placed at `SS:0x6D3E`,
`DS:0x6D3E`, and `GS:0x6D3E` prove these BP-based helpers touch SS only. Other
SI-based consumers use DS for the same array, establishing an `SS=DS` runtime
alias rather than GS ownership.

Open Watcom compiles both actual candidates without warnings; `-3 -ox -mm`
emits 21 instructions/40 bytes for removal versus 15/30 original and 29/60 for
insertion versus 21/45 original. The 8086/286/386 outputs have the same counts;
Turbo C 2.01 emits 22 and 32 instructions. The natural C preserves the exact
list algorithms, but emits DS-symbol accesses and Boolean results in AX. Exact
binary integration therefore needs a narrow carry adapter that preserves the
input AX plus the recovered `SS=DS` placement contract.

VM field-offset resolver `0x006023` has eight deterministic direct vectors.
They prove selector input in AX, a nonzero kind bitmask in BX, lowest-set-bit
selection when several bits are set, 16-bit wrapping of `selector * 16 + bit`,
GS table ownership against a DS decoy, signed-byte extension by 16-bit `CBW`
while preserving upper EAX, BX and unrelated-register preservation, the near
return boundary, and all six arithmetic flags left by the final `ADD`.

The natural candidate counts trailing zeroes by shifting a local copy of the
kind mask, which is both clearer and smaller than constructing a new bit mask
for every test. Open Watcom compiles the actual candidate without warnings;
`-3 -ox -mm` emits 20 instructions/39 bytes versus 8/17 original. Standalone
8086/286/386 probes emit 24/21/20 instructions and 42/39/39 bytes, while Turbo
C 2.01 emits 20 instructions. The AX/BX pragma reproduces the entry and return
registers, but neither compiler lowers natural C to `BSF`, and Watcom resolves
the far table through ES instead of fixed GS. Exact integration therefore needs
a narrow BSF/GS adapter, not register-state code in the natural function.

VM record owner lookup `0x006034` has nine deterministic direct vectors. They
prove that the result is the greatest directory base strictly less than AX,
including equality at the first and later entries and the binary's unconditional
pre-first read when no entry is lower. Multi-entry and wrapped-offset cases
prove the 20-byte stride and 16-bit SI arithmetic. Distinct GS and DS pointer
slots prove the far directory pointer comes from `GS:0x672C`; the vectors also
cover immutable directory data, upper-EAX and full unrelated-state
preservation, the near return, and every flag class left by `SUB SI,20`.

Replacing a tracked `previous` variable with the faithful natural sequence
"scan, then decrement once" reduces the actual Watcom candidate from 37 to 27
bytes. With the AX-only preserve-all pragma, `-3 -ox -mm` emits 12 instructions/
27 bytes versus 12/26 original; the 8086/286/386 probes are identical, while
Turbo C 2.01 emits 18 instructions. Watcom even folds the predecessor load to
`ES:[BX-4]` and leaves the final flags from a following `SUB BX,20`. The remaining
mismatch is segmented data placement: it loads a DS-held far pointer into ES:BX,
whereas the binary loads a GS-held pointer into DS:SI. The natural algorithm and
observable ABI are retained; exact register bytes need linker/segment binding or
a narrow adapter.

Active-object list builder `0x00604E` has five focused direct vectors in addition
to the earlier lifted/native sweep. They prove immediate and later early-stop
paths, low-byte-only object flag testing, unconditional `0xFFFF` termination,
16-bit directory and object address wrap, and final terminating-`CMP` flags.
Distinct GS/DS pointer and output slots establish GS ownership. A nonzero offset
in the object-block far pointer proves the binary discards that half and uses the
segment with each directory object offset as an absolute offset.

The natural candidate now expresses that segment-only access with the standard
16-bit `FP_SEG`/`MK_FP` idiom rather than incorrectly adding the far-pointer
offset. Open Watcom compiles it warning-free at all three CPU targets to 28
instructions/67 bytes versus 32/65 original; Turbo C 2.01 emits 31 instructions.
The close sizes are not an ABI match: Watcom binds globals/output through DS,
uses ES for both far inputs, and leaves AX/ES changed. The binary uses GS-owned
globals/output, FS object reads, and restores every register and segment. Exact
integration therefore needs segment binding and a preservation adapter around
the natural algorithm.

Ship 3D position resolver `0x0061A6` has eight direct vectors covering direct
kinds `0x0008`, `0x0010`, and `0x0200`, an ordinary selector-`0x11` parent,
`0xFFFF` arche fallback, both kind-`0x0100` outcomes, and a wrapping returned
offset. Distinct GS and DS copies prove that both the selector table and arche
offset are GS-owned. The kind-`0x0100` path's address-size override also exposes
a real precondition: ESI must be zero-extended because the comparison reads
`[EAX+ESI]`, not `[AX+SI]`.

The resolver is now one natural C function over a near record pointer. Open
Watcom `-3 -ox -mm` emits 46 instructions/98 bytes versus 45/106 original, with
mnemonic multiset overlap 0.8667. It binds SI/DX and returns the near pointer in
AX, but saves extra CX/DX/DI temporaries and addresses the arche global through
DS instead of GS. Turbo C 2.01 medium emits 76 instructions. This is a close
Watcom structural lowering, not an exact ABI image.

Ship 3D distance `0x0060DD` has six direct vectors covering kind-`0x0040`,
delegated direct and parent/arche resolution, kind-`0x0100` on either operand,
inherited compare state, and the signed `0x8000` delta edge. The vectors execute
the real mirrored far sqrt body and verify the binary's full EAX result: AX is
the root while the upper word remains the squared-distance high word.

Replacing far-base arithmetic and three private helpers with one natural
near-pointer function reduces Watcom medium output from 617 to 282 bytes. The
remaining result is 117 instructions versus 88 original, with mnemonic multiset
overlap 0.8523. Watcom emits two far `__I4M` calls for the long squares, while
the binary uses compact 386 `CWDE`/`MUL EAX` operations, `SHLD` to form DX:AX,
and one far sqrt call. Turbo C 2.01 medium emits 178 instructions. Exact
integration still requires GS table placement and a narrow codegen/preservation
boundary; the recovered algorithm remains plain C.

Ship 3D object-table bit test `0x006210` has eight direct vectors spanning
directory indices 0, 1, 7, 8, and 15, a wrapping 20-byte directory walk, and a
negative selector-table byte. They prove object input in AX, bitset base in
DS:SI, GS ownership of both the far directory pointer and selector table,
signed field-offset addition, 16-bit address wrap, immutable inputs, and full
state preservation. The final byte `SHL` reports the high-bit-first selection in
carry; the vectors also verify ZF/SF/PF and OF when its count is one.

The natural candidate states the same selection as a word shift followed by a
bit-`0x0100` Boolean test. Open Watcom `-3 -ox -mm` emits 33 instructions/70
bytes versus 31/59 original, with mnemonic multiset overlap 0.8387; Turbo C 2.01
medium emits 48 instructions. Watcom binds AX/SI and closely reproduces the
scan, but loads the far directory pointer through DS rather than GS and returns
the Boolean in AX. The binary instead restores AX and exposes only carry, so
exact integration requires a narrow carry-result adapter around the natural C.

Ship 3D navigation-source builder `0x00624B` has eight direct vectors covering
no children, one child, two siblings, nested depth-first output, a zero selector
offset, an inactive next entry, output-cursor wrap, and wrapped directory/object
fields. They prove GS ownership of the directory and selector table, SS
ownership of the output against a DS decoy, preservation of ES:DI and every
register/segment other than the advanced BP cursor, the far recursion/return
boundary, and flags from the outer terminating directory-kind comparison.

The natural candidate is one recursive C function over a far object pointer and
a returned near output cursor. Open Watcom `-3 -ox -mm` cannot bind BP as a
pragma-aux parameter or result, so the codegen probe binds the real ES:DI target
and substitutes BX for the cursor. It emits 51 instructions/113 bytes versus
34/72 original, with mnemonic multiset overlap 0.9118; Turbo C 2.01 medium emits
54 instructions. Exact integration needs GS data placement, the recovered
runtime SS=DS alias, and a narrow BP/BX adapter, but no register emulation is
present in the recovered algorithm.

VM token scanner `0x006293` has nine direct vectors covering immediate,
aligned, and unaligned matches; scan-cursor wrap; a word read crossing offset
`0xFFFF`; post-match addition wrap; and optional-increment wrap and signed
overflow. They prove the terminator in AX, the near cursor in DS:SI, DS ownership
against ES/GS decoys, preservation of AX and every register/segment other than
the returned SI cursor, immutable input, the near-return boundary, and flags
from the final byte comparison or increment.

Changing the pending pointer-to-pointer API to a natural pointer return exposes
the original data flow directly. Open Watcom `-3 -ox -mm` with AX/SI pragma-aux
inputs and an SI result emits the exact nine instructions and 16 bytes, with no
relocations. Turbo C 2.01 medium emits 19 instructions because it uses stack
arguments. This candidate can be linked directly without an ABI adapter or
inline assembly.

VM condition helper `0x006339` has fifteen direct vectors covering random-gate
pass and short circuit; equality, signed-greater, and inverted signed field
comparisons; both history algorithms; zero-sentinel failure; isolated mode and
copy side effects; and a combined cursor-flow case. The duplicate-history case
exposed and corrected a real pending-C bug: the binary keeps scanning all eight
history slots after a match, so duplicate slots can satisfy multiple required
hits for one operand.

The vectors also prove CX, ES:DI, and DS:SI inputs; GS ownership of the field
table and mode flags; an ES history base; presentation output through
SS:0x67F8 against GS/DS decoys; CF-only success; immutable inputs; and
CX/SI/DI/BP plus segment preservation. The natural candidate is now exactly one
C function. Open Watcom `-3 -ox -mm` binds the three input locations and emits
142 instructions/355 bytes versus 104/250 original; Turbo C 2.01 medium emits
179 instructions. Exact integration still needs fixed segment placement, the
runtime SS=GS alias, and a narrow Boolean-to-carry result adapter.

VM dictionary lookup `0x006433` has eight direct vectors covering first- and
later-entry matches, immediate inactive termination, active miss, prefix
rejection, high-byte equality, DIC offset wrap, and a 20-byte directory stride
across offset `0xFFFF`. The vectors execute the original far `string_compare`
callee and prove AX input/result, CF match status, GS ownership of both far
pointers, DS:SI and ES:DI comparator inputs, immutable source data, preservation
of every other register and segment, and the near-return boundary.

The natural candidate returns an object-offset plus matched-status structure.
Open Watcom `-3 -ox -mm` naturally places those words in AX and DX, but cannot
bind the DIC far pointer to DS:SI while retaining its medium-model DGROUP. It
emits 38 instructions/90 bytes versus 21/47 original; Turbo C 2.01 medium emits
36 instructions. Exact integration therefore needs GS data placement and
narrow DS:SI/ES:DI comparator and carry-result ABI boundaries, not additional
lookup logic.

VM branch helper `0x006462` has seven direct vectors covering the first and
second stack words, odd byte offsets, top underflow, signed overflow, and stack
effective-address wrap. They prove that GS owns the byte-count top and query
flag while the `BP`-based target load uses SS; the routine returns the new top
in AX and target script cursor in SI, preserves BP and all unrelated state,
retains flags from the 16-bit subtraction, and near-returns. Direct execution
also exposed two pending-C errors: it loaded the target after clearing query
mode and divided the byte offset by two, losing odd-offset behavior.

The corrected natural C performs the byte-granular access in binary order and
uses a Watcom pragma only to return the target in SI. Open Watcom `-3 -ox -mm`
emits 7 instructions/21 bytes versus 8/25 original; Turbo C 2.01 medium emits
10 instructions. Watcom's body is close, but it uses DS and BX, leaves AX
untouched, and lowers the subtraction to `ADD 0xFFFE`, whose carry differs on
underflow. Exact integration therefore needs fixed GS/SS placement, the
runtime segment alias, and a narrow AX/BP/flag boundary.

VM positive-word scanner `0x00647B` has ten direct vectors covering immediate
zero, minus one, and signed-minimum stops; one and multiple positive words;
unaligned and wrapping cursors; auxiliary- and overflow-flag count edges; and
complete 65,535-word `LOOP` exhaustion. They prove DS:SI input with restored SI,
restored CX, GS ownership of the count against DS/SS decoys, the terminating or
final positive word in AX, immutable input, exact `NEG`/`DEC` result flags, and
the near-return boundary.

The natural C uses a read-only near pointer, signed comparison, and explicit
`0xFFFF` count bound. Open Watcom `-3 -ox -mm` binds SI and emits 11
instructions/22 bytes versus 14/25 original; Turbo C 2.01 medium emits 16
instructions. Watcom preserves SI but stores through DS, leaves the count in AX,
and returns comparison flags rather than the binary's terminal word and final
count flags. Exact integration needs GS placement and a narrow AX/flag boundary;
the scan algorithm itself requires no assembly.

VM conditional gates `0x006494`, `0x0064A0`, and `0x0064AC` each have four
direct vectors for flag values zero, an unrelated-bit-only value, bit zero, and
all bits. The twelve vectors prove GS ownership of the distinct gate bytes,
calls through the real `0x006462` branch helper only when bit zero is clear,
branch-stack and query effects, conditional AX/SI outputs, TEST flags on the
continue path, SUB flags on the branch path, preservation, and near return.
Turbo C 2.01 medium emits the exact four-mnemonic TEST/JNE/CALL/RET shape for
the representative natural gate. Watcom `-3 -ox -mm` emits an equivalent
three-instruction conditional tail branch after the outer AX/SI clobber contract
is declared. Exact integration still needs fixed GS placement and the recovered
branch-helper ABI.

VM script-profile request `0x0064B8` has six direct vectors covering signed byte
values `0x00`, `0x01`, `0x7F`, `0x80`, and `0xFF`, plus SI wrap. They prove DS
script ownership, GS output ownership, AX sign-extension/decrement result, SI
cursor return, preserved incoming carry, all DEC-defined flags, immutable input,
preservation, and near return. Replacing the pointer-to-pointer API with a
natural pointer return exposes the binary data flow. A separate local request
value is semantically ordinary and prevents Watcom from duplicating the volatile
store. Watcom then emits 5 instructions/9 bytes versus 5/8 original, using an
equivalent MOVSX/INC pair instead of LODSB/CBW; Turbo C medium emits 16
instructions. Only fixed GS placement and opcode selection remain mismatched.

VM clear-state handler `0x0064C0` has four vectors proving that the GS byte clear
precedes the GS word clear, DS/SS decoys remain unchanged, all registers,
segments, and flags are preserved, and the routine near-returns. Turbo C 2.01
medium emits the exact three-mnemonic MOV/MOV/RET shape. Watcom emits 5
instructions/11 bytes versus 3/14 original by zeroing AX first, which changes AX
and flags. The two natural assignments are complete; exact integration needs GS
placement and favors the Turbo C lowering for this routine.

VM record-string copy `0x0064CE` has nine direct vectors covering the first and
second slots, operand zero, the `0x80`/`0x81` signed boundary, raw high bytes,
source-cursor wrap and signed overflow, and a copy extending beyond the nominal
16-byte slot. They prove that the raw slot byte is decremented before signed
extension, the signed slot is scaled by 16, DS owns the source, SS owns the
destination, the NUL is copied, one pad byte is skipped, no slot-length bound is
enforced, AX/SI/BP and final INC flags match, unrelated state is preserved, and
the routine near-returns. This exposed a real pending-C bug: converting operand
`0x80` to signed before decrement produced -129 instead of the binary's +127.

The corrected natural C uses explicit 8-bit wrap, a flat destination pointer, a
do-while NUL copy, and a returned near source cursor. Open Watcom `-3 -ox -mm`
emits 20 instructions/32 bytes versus 13/23 original and preserves the critical
DEC/CBW/SHL arithmetic; Turbo C 2.01 medium emits 31 instructions. Watcom uses
AX for the destination and leaves BP unchanged, while the binary writes through
SS:BP and returns advanced BP. Exact integration therefore needs the runtime
SS=GS alias and a narrow BP boundary, not additional string-copy logic.

VM tagged-word comparison `0x0064E5` has fourteen direct vectors covering F1
signed-greater pass, equality, failure, and overflow; F2 signed-less pass,
equality, failure, and overflow; default equality and mismatch; ignored tag high
bytes; unaligned input; and a second word read spanning the segment end before SI
wrap. They prove DS source ownership, GS comparison-word ownership, calls through
the real branch helper, branch-stack and query effects, pass-path AX/SI outputs,
failure-path AX/SI outputs, DL tag output, comparison versus SUB flags by path,
preservation, immutable input, and near return.

Replacing the pointer-to-pointer API and Boolean temporary with direct natural
cursor returns matters materially. Keeping the volatile comparison global in
each selected path lets Open Watcom `-3 -ox -mm` recover the original DL tag, AX
value, and SI cursor allocation and emit exactly 17 instructions/47 bytes versus
17/43 original; Turbo C 2.01 medium emits 33 instructions. Watcom uses MOV/ADD
cursor loads and equivalent conditional tail branches to the branch helper
instead of LODSW and CALL/shared-RET. Exact integration needs fixed GS placement
and the original call boundary, but the comparison logic and register data flow
need no assembly.

VM tagged-byte-pair comparison `0x006510` has eighteen direct vectors covering
every F1 lexicographic signed-greater path, every F2 signed-less path, default
pair equality and mismatch, strict equal boundaries, signed overflow in either
component, unaligned input, and a padding-word read spanning the segment end.
They prove the packed low/high byte order, five-byte DS:SI consumption, separate
GS comparison globals at `0x0AA8` and `0x0AAA`, the real branch-helper effects,
pass-path AX/BX/DL/SI and flags, failure-path helper outputs, preservation,
immutable input, and near return.

A natural two-byte union plus explicit high-then-low branches avoids shifts and
duplicate comparisons. Open Watcom `-3 -ox -mm` emits 27 instructions/81 bytes
versus 28/73 original, including DL tag retention, direct signed AH/AL compares,
and the same decision topology; Turbo C 2.01 medium emits 48 instructions. The
remaining data-flow difference is bounded: optimizing C skips the otherwise
unused padding load and therefore retains the pair in AX, while the binary loads
the pair into BX and padding into AX. Both VM dispatchers observe SI after the
handler and overwrite AL/BX before their next use. Exact integration still
needs fixed GS placement and the original CALL/shared-RET boundary; reproducing
the incidental padding load would require an ABI adapter or a non-natural
volatile read, so it is intentionally not hidden inside the C logic.

VM branch-stack push `0x006559` has nine direct vectors covering the first and
later stack slots, odd byte offsets, top wrap to zero and one, signed overflow,
effective-address wrap, a stack word spanning `SS:FFFF`, and a script word
spanning `DS:FFFF`. Instruction-phase checks prove that query mode is set first,
the top is grown second, the operand is loaded third, and the stack word is
written last. The vectors also prove DS/GS/SS ownership, AX operand, BP old top,
advanced SI, ADD flags, preservation, immutable input, and near return.

Replacing the word-indexed pointer-to-pointer API with a byte-pointer store and
direct cursor return fixes the odd-offset bug. A compound volatile top update
also prevents Watcom from duplicating the store. Open Watcom `-3 -ox -mm` then
emits 9 instructions/26 bytes versus 8/25 original; Turbo C 2.01 medium emits 21
instructions. Watcom uses saved BX for the old top and a memory ADD, while the
binary clobbers BP and performs ADD/store through AX; the arithmetic flags and
observable memory order match. Exact integration needs fixed GS/SS placement
and the original BP allocation, but no register or memory emulation belongs in
the recovered C.

VM branch-stack pop `0x006572` has seven direct vectors covering the empty base
top, ordinary and odd tops, underflow from zero and one, signed overflow from
`0x8000`, and the maximum top. They prove query mode clears before the top read,
top 2 performs no write, every nonempty path performs one wrapped decrement, GS
owns both globals, AX returns the old top, flags come from CMP or SUB by path,
all unrelated state is preserved, and the routine near-returns.

The natural C reads the volatile top once for comparison and return, exposes
that old top as the function result, and uses one compound volatile decrement.
Open Watcom `-3 -ox -mm` emits 7 instructions/20 bytes versus 6/22 original and
keeps the result in AX, but canonicalizes the memory subtraction to `ADD
0xFFFE`, which differs in carry and overflow on edge cases. Turbo C 2.01 medium
emits 9 instructions, preserves the original memory SUB and final flags, and
moves its saved SI local to AX before return. Exact codegen therefore needs a
narrow compiler/register-allocation decision; the recovered C logic itself is
complete and contains no synthetic flag handling.

VM random branch `0x006588` has seven direct vectors using a deterministic
`MOV AX,result; RETF` stub at the original PRNG target. This isolates the opcode
handler while the PRNG algorithm remains covered by its own 300-vector oracle.
The A2 vectors prove that modulus 0, 1, 3, 5, 7, 9, and `0xFFFF` arrive in AX,
SI is advanced at far-call entry, CS is `0x01CE`, SP reflects a far call, zero
always continues, and nonzero results including `0x8000` and `0xFFFF` invoke the
real branch helper. They also cover script wrap, odd and overflowed branch-stack
tops, query/top effects, path-specific AX/SI and flags, restored CS/SP,
preservation, and immutable input.

The natural candidate now returns the DS:SI cursor directly, and the shared
PRNG declaration records its recovered AX parameter/result ABI. Open Watcom
`-3 -ox -mm` emits exactly 6 instructions/17 bytes versus 6/14 original;
Turbo C 2.01 medium emits 20 instructions because it uses a stack argument.
Watcom emits a real far call but schedules SI advancement after it, uses TEST
instead of OR, and tail-branches to the helper. These are function-level
equivalences because the PRNG preserves SI and logical TEST/OR flags agree;
exact codegen still needs original LODSW scheduling and CALL/shared-RET control.

VM conditional block `0x006596` has twelve direct vectors covering immediate and
later token-special scans, optional zero padding, scan-bit masking, ordinary and
inverted equality/mismatch, zero-match failure, default versus resume match
selection, and target words spanning the segment end with and without an A1
prefix. They prove the scan path calls `0x006293` with AX zero, not the positive
word scanner; the selected match is read through SS:BP at `0x6762` or `0x6764`;
and failed comparisons call the real branch helper. Cursor results,
branch-stack/query effects, AX/BP/DL/SI, path flags, DI preservation, segmented
decoys, and immutable input are all checked.

Replacing the pointer-to-pointer API and Boolean expression with direct cursor
returns and explicit inverted/noninverted branches gives Open Watcom `-3 -ox
-mm` 32 instructions/68 bytes versus 29/69 original; Turbo C 2.01 medium emits
48 instructions. Watcom preserves the control structure and both helper calls,
but keeps target in saved BX and match in AX, while the binary leaves target in
AX and the selected offset in BP. Exact integration therefore needs the runtime
SS=GS alias, fixed globals, and narrow AX/BP compatibility, not a different C
algorithm.

VM script jump `0x0065DB` has six direct vectors covering ordinary, zero, odd,
and maximum targets, unaligned input, and a target word spanning `DS:FFFF`.
Instruction-phase checks prove SI is replaced directly from DS:SI before the
GS:0x67B1 byte clear, which precedes the GS:0x6764 word clear. The vectors also
prove no operand postincrement, unchanged AX and unrelated state, complete
arithmetic-flag preservation, segmented output ownership, immutable input, and
near return.

Representing the result as a near byte-stream pointer lets Open Watcom `-3 -ox
-mm` recover the SI input/result directly and emit 8 instructions/15 bytes
versus 4/16 original. It saves AX and synthesizes the two zeros with XOR, though,
so final flags differ. Turbo C 2.01 medium emits 11 instructions with stack
input and AX return, but uses immediate stores and therefore preserves flags.
The C logic is complete; exact integration needs Watcom's register ABI combined
with Turbo-like immediate stores or a narrow codegen boundary.

VM conditional-state handler `0x0065EB` has eight direct vectors proving that
the operand byte is signed: `CBW` maps `0xFF` and `0x80` to state words before
SS:0x6ADE, rather than to unsigned entries 255 and 128. The vectors also prove
that GS:0x67AD bit zero selects a one-byte query path, while its clear path
consumes a following word; state storage is through SS:BP, script input through
DS:SI, and a nonzero query calls the real branch helper. Path-specific AX, BP,
SI, flags, stack/query effects, and a word crossing `DS:FFFF` are covered.

Open Watcom `-3 -ox -mm` preserves SI as the pointer input/result but emits 18
instructions/39 bytes versus 13/33 original, using `MOVSX` and saved BX for the
index plus DS globals. Turbo C 2.01 medium emits 28 instructions under its stack
ABI; it does retain the natural source's byte load followed by `CBW`. The
logical C is complete, but exact integration still needs SS state placement and
the original AX-to-BP allocation with string loads.

VM TEXT handler `0x00660C` has eleven direct vectors over its complete four-phase
flow: control setup, display/presentation gating, accepted-token mutation, and
the shared post-output terminator scan. They cover every pre-display gate,
deterministic random rejection through the real `0x006339` helper, signed b3,
the optional-control ordering, raw menu pointers and count, subtitle punctuation
spacing, 35-column wrapping, the `0xFFFF` spoken/menu separator, all touched
segments/globals, path registers, and final flags.

The full natural candidate compiles cleanly with Open Watcom `-3 -ox -mm` to
188 instructions/515 bytes versus 138/411 original. It retains the high-level
branch topology and direct SI result but introduces a frame and locals, uses DS
for globals, normalizes far pointers, and materializes the condition helper's
logical result in AX instead of consuming its original carry result. This is a
compiler/ABI mismatch; the recovered C body has no register-state or memory
emulation layer.

Near string-length helper `0x0067A7` has eight direct vectors covering empty,
ordinary, high-byte, segment-offset wrapping, the maximum terminated length,
and the original `0xFFFF`-probe unterminated bound. The natural C retains that
bound and its `0xFFFE` sentinel explicitly. ES ownership, AX result, CX/DI and
unrelated-state preservation, immutable input, and SUB-derived flags are all
checked.

With an ES:DI argument and AX result declaration, Open Watcom `-3 -ox -mm`
emits the exact 11-instruction count in 21 bytes versus 19 original. It chooses
a scalar increment/count loop instead of `REPNE SCASB`, so return values and
the malformed-input bound match while final flags do not. Turbo C 2.01 medium
uses a stack far pointer and emits 18 instructions.

VM presentation-register handler `0x0067BA` has six direct vectors proving that
the DS:SI word load and SI advance precede the gate, only bit zero of
GS:0x67AC matters, and GS:0x6770 is written only on the active path. Unaligned
and segment-end operands, AX/SI outputs, segmented decoys, preservation, and
TEST-derived flags are covered.

Open Watcom `-3 -ox -mm` preserves SI input/result, AX operand, the bit test,
conditional store, and final flags in 7 instructions/17 bytes versus 5/14
original. It uses MOV/ADD instead of LODSW and duplicates RET around an inverted
branch. Turbo C 2.01 medium uses a stack argument and emits 16 instructions.

VM padded-string handler `0x0067C8` has ten direct vectors proving SS:0x2120
destination ownership, DS:SI NUL copy, one-byte pad consumption, case-sensitive
`fin.` prefix matching, request-bit exclusion, ship/scene bit-zero gates, and
the ordered active-line/request/presentation/actor/dialog stores. Copy and pad
offset wrap, AL-only clearing with AH preserved, SI/BP outputs, untouched state,
and path flags are included.

Open Watcom `-3 -ox -mm` emits 38 instructions/105 bytes versus the original
29/104, with mnemonic LCS 25/29 and multiset overlap 26/29. The control flow is
close, but Watcom uses DS globals, AX as the destination cursor, and saved BX/DX
instead of the original SS:BP destination and AL loop. Turbo C 2.01 medium emits
42 instructions with mnemonic LCS 26/29.

VM conditional jump handler `0x006830` has nine direct vectors covering clear
and set flag bit zero, unrelated flag bits, zero and maximum targets, unaligned
input, and target words spanning `DS:FFFF`. Instruction-phase checks prove both
paths consume the flag first; the clear path replaces SI directly from the
following word, while the set path writes query mode, consumes the target,
writes branch-stack root, and finally sets the top to 2. The vectors also prove
GS state ownership, DS script ownership, path-specific AX/SI, preservation,
immutable input, TEST-derived flags, and near return.

The direct-return natural candidate compiles without warnings under Open Watcom
`-3 -ox -mm` to 12 instructions/30 bytes versus 10/28 original. Watcom retains
the branch topology, SI input/result, store ordering, and immediate state values,
but expands LODSB/LODSW to MOV plus pointer arithmetic, duplicates RET, addresses
globals through DS, and overwrites the odd path's TEST flags with `ADD SI,2`.
Turbo C 2.01 medium emits 25 instructions. Exact integration therefore needs
fixed GS placement and the original string-load/shared-return codegen; the
natural C logic and data flow require no register or memory emulation.

VM byte-poke handler `0x00684C` has ten direct vectors covering zero, high-bit,
and maximum byte values; aligned and unaligned pointers; a target at
`DS:FFFF`; an operand word spanning the segment end; and final cursor additions
that exercise carry, zero, auxiliary carry, sign, and overflow. Two aliasing
cases prove the target pointer is resolved before the write when it points at
the source value or its own low pointer byte. The vectors also prove DS source
and target ownership, AX/BX/SI outputs, preservation, final ADD flags, and near
return.

The direct-return natural candidate compiles without warnings under Open Watcom
`-3 -ox -mm` to 6 instructions/11 bytes versus 5/9 original. Watcom expands
`LODSB` into `MOV AL,[SI]` plus `INC SI`; its target load, byte store,
`ADD SI,2`, and `RET` are byte-for-byte identical to the remaining original
instructions. The expansion does not alter final behavior because the final ADD
replaces INC's arithmetic flags. Turbo C 2.01 medium emits 20 instructions under
its stack ABI. Exact codegen requires only LODSB selection, not a different C
algorithm or an emulation layer.

VM yield handlers `0x006855` (AA) and `0x00685C` (AC) are byte-identical but
remain separate dispatch targets and separate C functions. Six direct vectors
per entry cover zero, already-set, high-bit, maximum, and alternating initial
values. They prove an unconditional write of one to GS:0x67B4, isolation from
DS/ES/FS/SS decoys, complete register preservation, preserved arithmetic,
interrupt, and direction flags, and near return.

The existing natural C body is exactly one volatile assignment for each entry.
Both Open Watcom `-3 -ox -mm` and Turbo C 2.01 medium emit the exact two-mnemonic
`MOV; RET` shape. Watcom compiles each actual candidate without warnings to 6
bytes versus 7 original; the sole structural difference is the missing GS
override because an ordinary external global is addressed through DS. Exact
integration is therefore a data-placement/linker problem, not missing C logic.

Shared VM state handler `0x006863` serves dispatch opcodes B1, B4, B5, B6, BE,
BF, and C0. Twenty direct vectors cover all six query relations, signed boundary
cases, unknown-query failure, immediate and C0/C2 record-backed RHS values,
wrapping add/subtract, assignment, and the unchanged-field write performed for
unknown set operations. They also prove parse and dereference order, GS far-base
and query/top ownership, ES record ownership, DS script ownership, SS branch
stack ownership, offset and script-segment wrap, real branch-helper effects,
path registers, final flags, and near return.

The corrected natural candidate returns either the six-byte-advanced cursor or
the branch helper's replacement cursor directly. Open Watcom `-3 -ox -mm`
compiles it without warnings to 87 instructions/198 bytes versus 69/159
original; Turbo C 2.01 medium emits 111 instructions. Watcom retains all signed
SETcc relations and the SI result, but creates a frame and integer Boolean,
keeps current in AX and markers in BL/CL instead of CX and AH/AL, and addresses
the far-base/query globals through DS rather than fixed GS. Exact integration
needs the original segmented placement and narrow register allocation, not a
different algorithm or an emulation layer.

Shared VM bit-state handler `0x006902` serves opcodes AE and B0. Fourteen direct
vectors prove that optional A1 consumes one byte and flips query polarity; the
query asks whether any masked bit is present, including partial and zero-mask
cases; and set mode uses OR without A1 or complemented AND with A1. They also
cover four- versus five-byte cursor consumption, GS far-base/query/top
ownership, ES record ownership, DS script ownership, SS branch-stack ownership,
record-offset and script-segment wrap, real branch-helper effects, path
registers, final flags, and near return.

The corrected candidate loads the far base before parsing and directly returns
either the parsed cursor or branch target. Open Watcom `-3 -ox -mm` compiles it
without warnings to 36 instructions/80 bytes versus 31/68 original; Turbo C
2.01 medium emits 54 instructions. Watcom retains the far field and SI result,
but creates a local for the base offset, represents inversion in AX and the mask
in DX, lowers query polarity through TEST/SETNE/CMP, and addresses query mode
through DS. Exact integration needs fixed GS placement and the original
frameless DL/AX/BX allocation, not an emulation layer.

Shared VM record-wildcard handler `0x006946` serves opcodes AD, AF, B2, B3,
BA, BB, and BC. Seventeen direct vectors cover ordinary and A1-inverted equality,
GS:0x674E-to-0xFFFF query substitution, BC value publication, direct writes,
owner removal when replacing an old 0xFFFF field, existing/free/full owner-list
insertion, and full-list write suppression. The vectors execute the real
directory lookup, remove, insert, and branch helpers and prove their call order,
SS slot ownership, all other segment ownership, record and script wrap, path
registers, final flags, and near return.

The new natural candidate directly returns either the parsed cursor or branch
target and reads the dispatch opcode through `script_bytes[-5]`, matching the
original post-parse SI-relative access. Open Watcom `-3 -ox -mm` compiles it
without warnings to 56 instructions/139 bytes versus 55/129 original; Turbo C
2.01 medium emits 88 instructions. The one-instruction delta is not byte
equivalence: Watcom materializes the far base in BX/DI, reallocates offset/value
to CX/DX, creates an AX Boolean for query equality, addresses globals through
DS, and consumes Boolean AX results where the original slot helpers return
carry while preserving AX. Exact integration needs segmented placement and
narrow carry adapters, not different C logic.

VM opcode-CD handler `0x0069C7` has two modes. Query mode optionally consumes
an A1 inversion prefix and matches `{0x00CD, second, third}` at the first record
offset. Set mode resolves the first operand through the threshold directory,
performs three flag-byte reads whose TEST results are not consumed, looks up
selector `0x11` twice, synchronizes the second record with the special-owner
list, writes through a signed field offset, and conditionally requests C2
presentation. Twenty direct vectors prove those decisions, the real helper
ordering and side effects, absolute offsets in the loaded record segment,
segment ownership and wrap, path registers and flags, and the C2 far-call ABI.

The one-to-one natural candidate keeps the direct cursor/branch result, signed
field update, duplicate selector lookup, full-list early return, and ordered
presentation writes. Open Watcom `-3 -ox -mm` compiles it without warnings to
86 instructions/232 bytes versus 82/224 original; Turbo C 2.01 medium emits 135
instructions. Watcom is structurally closer but introduces a four-byte frame,
reallocates the owner/record/value registers, addresses globals through DS,
uses Boolean AX for the carry-return insertion helper, and drops the three dead
TEST reads even though the C expressions use volatile lvalues. Exact integration
therefore still needs segmented placement, narrow ABI adapters, and either the
original register allocation or a deliberately assembly-shaped boundary.

VM opcode-B7 handler `0x006AA7` loads the far record base before consuming an
optional A1 inversion prefix, a word record offset, and a byte bit index. It
uses high-bit-first numbering: the byte advances by `index / 8`, while mask
`0x80 >> (index & 7)` selects the bit. Query mode branches when the bit state
equals inversion; set mode ORs the mask without A1 and clears it through a
complemented AND with A1. Fourteen direct vectors prove indices 0, 7, 8, and
255; every query and update outcome; record-base offset participation; 16-bit
record and script wrap; the real branch helper; segments, registers, flags,
preservation, and near return.

The corrected one-to-one candidate replaces the old pointer-to-pointer API
with an SI input/result and directly returns either the parsed cursor or branch
target. Open Watcom `-3 -ox -mm` compiles it without warnings to 51
instructions/113 bytes versus 43/95 original; Turbo C 2.01 medium emits 70
instructions. Watcom retains far-byte addressing and the logical shifts but
creates a two-byte frame, stores the record offset, uses AX for inversion and
materializes query truth with SETNE/CMP, addresses globals through DS, and
duplicates returns. Exact integration needs fixed segment placement and the
original narrow register allocation, not a different algorithm.

Shared VM pair handler `0x006B06` serves opcodes B8, B9, and BD. It loads the
far record base, adds the script record offset to the loaded pointer offset,
and consumes two values. Query mode compares both record words and branches on
either mismatch. Set mode writes both words, passes the effective segment
offset to the threshold directory helper, then loads GS:0x6752 as an absolute
record-segment offset and clears its `+0x16` link when it matches the resolved
owner. Ten direct vectors prove both comparison failures, writes, link match
and mismatch, nonzero mode bits, the real helpers, nonzero base offsets,
effective-address and script wrap, segmented ownership, registers, flags, and
near return. They exposed and corrected the old candidate's offset-only helper
argument and incorrectly base-relative secondary link.

The corrected one-to-one candidate computes the effective 16-bit segment
offset explicitly and directly returns the parsed cursor or branch target.
Open Watcom `-3 -ox -mm` compiles it without warnings to 37 instructions/91
bytes versus 26/70 original; Turbo C 2.01 medium emits 63 instructions. Watcom
preserves the essential far loads, pair tests/writes, helper call, and absolute
link access, but materializes the far pointer in AX/CX, keeps the effective
offset in BX, allocates the pair to AX/DX, saves CX/DX/DI, addresses globals
through DS, and duplicates returns. Exact integration needs fixed segment
placement and the original compact register allocation.

VM opcode-C5 handler `0x006D18` loads the segment from GS:0x6724 but ignores
the far pointer's offset: both script operands are absolute offsets in that
segment. Query mode optionally inverts a match of destination type C5 and value
equal to the operand. Set mode ignores inversion, requires the related record's
byte `+2` bit zero, related type `0x0200`, and an empty destination in that
order, then writes `{0x00C5, related offset, 0}`. Fourteen direct vectors prove
all query outcomes and guards, prefix handling in both modes, no partial writes,
the real branch helper, ignored base-offset decoys, record and script boundary
behavior, segmented ownership, registers, flags, and near return. They exposed
and corrected the old candidate's base-relative record addressing.

The corrected one-to-one candidate uses explicit absolute far pointers and
directly returns the parsed cursor or branch target. Open Watcom `-3 -ox -mm`
compiles it without warnings to 41 instructions/107 bytes versus 40/104
original; Turbo C 2.01 medium emits 64 instructions. Despite the close size,
Watcom omits the loaded base offset, keeps the cursor in BX and destination in
SI, allocates inversion to AX and operand to DI, materializes query truth with
CMP, addresses globals through DS, and duplicates returns. Exact integration
still needs fixed segment placement and the original BP/DL/AX/BX allocation.

VM opcode-C6 handler `0x006D80` shares C5's optional inverted typed query and
absolute record-segment addressing, but its set mode has no guards: it always
overwrites the destination with `{0x00C6, operand, 0}`, including with A1 and
when the old record is nonempty. Eleven direct vectors prove every query
outcome, empty and occupied overwrites, prefix indifference in set mode,
ordered writes, the real query-failure helper, ignored base-offset decoys,
record and script boundaries, segmented ownership, registers, flags, and near
return. They exposed and corrected the old candidate's base-relative pointer.

The corrected one-to-one candidate directly returns the parsed cursor or
branch target. Open Watcom `-3 -ox -mm` compiles it without warnings to 32
instructions/82 bytes versus 31/79 original; Turbo C 2.01 medium emits 53
instructions. Watcom keeps the same operations but uses BX for the record, DX
for the operand, AX for inversion, SET-like Boolean comparison control, DS
globals, a saved BX, and duplicate returns. Exact integration still needs fixed
segment placement and the original BP/DL/AX allocation.

VM opcode-C7 handler `0x006DCF` uses absolute destination and related offsets
in the segment loaded from GS:0x6724. Query mode optionally inverts a type-C7
and operand match. Set mode ignores inversion, requires only related byte `+2`
bit zero (the related type is not inspected), then reads destination type once
and accepts zero or C4 before writing `{0x00C7, related offset, 0}`. Fifteen
direct vectors prove every query and guard outcome, ignored related type,
single destination-type read, no partial writes, the real branch helper,
ignored base-offset decoys, boundaries, segments, registers, flags, and return.
They corrected both the old candidate's base-relative pointers and its repeated
volatile destination-type expression.

The corrected one-to-one candidate directly returns the cursor or branch
target. Open Watcom `-3 -ox -mm` compiles it without warnings to 49
instructions/123 bytes versus 39/101 original; Turbo C 2.01 medium emits 66
instructions. Watcom introduces a two-byte frame, retains segment state in a
local, reallocates destination/related/inversion, materializes query truth,
addresses globals through DS, and duplicates returns. Exact integration still
needs fixed segment placement and the original compact BP/BX/AX/DL allocation.

VM opcode-C9 handler `0x006FB9` consumes an absolute record offset in the
segment loaded from GS:0x6724. It reads the old kind, clears kind, then reads
the old related offset before clearing related and value. For old C4 records it
reads the related kind, adds the signed selector-0x13 field offset with 16-bit
wrap, resets the sequence/depth globals, and clears the reciprocal triple.
Eight direct vectors prove the exact volatile access order, both teardown
paths, positive, negative, and zero offsets, reciprocal wrap and aliasing, the
real field-offset helper, segmented ownership, registers, flags, and return.
They corrected the old candidate's base-relative pointers and premature read.

The corrected one-to-one candidate directly accepts and returns SI. Open
Watcom `-3 -ox -mm` compiles it without warnings to 29 instructions/88 bytes
versus 26/58 original; Turbo C 2.01 medium emits 56 instructions. Watcom keeps
the direct helper call, signed 16-bit addition, and ordered volatile stores,
but saves DX, materializes the record segment in DX, uses indexed BX stores
instead of DI plus STOSW, and addresses globals through DS. Exact integration
still needs fixed GS placement and the original frameless AX/BX/CX/DI
allocation.

Byte-parser handlers `0x007542`, `0x007549`, `0x007550`, and `0x007557` are
byte-identical entry points for opcodes 0x01, 0x02, 0x0F, and 0x04. Two direct
vectors per entry execute through RET and prove the exact seven-byte body,
constant overwrite of GS:0x0B16, DS and SS decoy preservation, unchanged
registers, segments, and flags, and the two-byte near-return stack advance.

Each one-to-one candidate is the same natural volatile assignment. Open Watcom
`-3 -ox -mm` compiles all four without warnings to the same two instructions
and six bytes; Turbo C 2.01 medium emits the same two mnemonics. The original's
seventh byte is the GS override, while both standalone probes address the
unresolved global through DS. The C logic and instruction shape are settled,
but direct integration requires a GS-qualified data mechanism or a minimal
one-instruction boundary because DS owns the parser stream at these entries.

Byte-parser opcode-05 handler `0x007612` copies a NUL-terminated DS:SI string,
including and consuming the terminator, to ES:0x0E18. The dispatcher has set
ES=GS before switching DS to the script segment. After the copy, the handler
sets GS:0x5E64 and then clears GS:0x5E58. Five direct vectors prove empty,
ordinary, high-byte, and segment-wrapped sources; distinct DS, ES, and GS
ownership; exact state-write order; registers, segments, flags, source
immutability, and near-return stack behavior.

The corrected candidate returns the advanced SI cursor directly and places the
destination and state symbols in one named based segment. Open Watcom
`-3 -ox -mm` compiles it without warnings to 19 instructions/38 bytes versus 8/23
original; Turbo C 2.01 medium emits 27 instructions. Watcom loads `GAME_DATA`
into ES, saves BX/DX/ES, emits a scalar indexed copy, and zero-extends the final
AL through AH before storing the timer. Exact integration needs the original
ambient ES=GS contract, DI allocation, LODSB/STOSB loop, and GS state accesses.

Byte-parser handlers `0x007629`, `0x00766F`, `0x0076C0`, and `0x0076D5`
share one 21-byte copy shape with destination offsets 0x20B8, 0x24C6, 0x2460,
and 0x247A. Each copies bytes 0x20 through 0x7F from DS:SI to ES:DI, leaves
the first low-control or high-bit byte unconsumed, and writes a NUL without
advancing DI. Eight direct vectors per entry prove both stopping classes,
boundary bytes, source wrap, DS/ES ownership, AL/SI/DI outputs, preservation,
near return, CF inherited from the stopping test, and the remaining flags from
the final `DEC SI`. They also narrow `0x00766F` from an incorrect whole-record
label to its actual name-copy boundary.

The four one-to-one candidates use direct SI results, a signed-byte guard, and
named based-segment destinations. Open Watcom `-3 -ox -mm` compiles each without
warnings to 23 instructions/42 bytes versus 11/21 original; Turbo C 2.01 medium
emits 28 instructions. Watcom retains the signed and unsigned tests, explicit
cursor decrement, and NUL store, but loads `GAME_DATA`, saves BX/DX/ES, and
uses scalar indexing instead of ambient ES plus LODSB/STOSB.

Byte-parser opcode-11 handler `0x00763E` extends the same printable-copy shape
with a GS:0x2793 bit-zero gate and a far SND-bank loader call. Eight direct
vectors prove the copy bounds, stopping-byte preservation, segment wrap and
ownership, call-versus-skip behavior, actual loader entry with AX=1 and
DS:SI=GS:0x0D06, caller restoration of parser DS:SI, path-specific outputs,
flags, and near return. The real loader body executes its early return in the
call vectors, so this is not a synthetic call stub.

The one-to-one candidate uses a direct SI result, named based-segment filename,
path, and gate globals, plus an ordinary far C call. The loader declaration now
uses its actual AX mode and near-SI path convention. Open Watcom `-3 -ox -mm`
compiles the candidate without warnings to 26 instructions/62 bytes versus
22/49 original; Turbo C 2.01 medium emits 41 instructions. Watcom retains the
signed and unsigned stops, explicit cursor decrement, NUL store, bit test,
mode value, and SI argument. A drop-in build still needs a narrow adapter to
switch DS to `GAME_DATA` around that call and restore the parser context.

Navigation-choice handlers `0x008713` and `0x008848` share a natural early-
return state update: test DS:0x2565 bit zero, copy a named source record to
DS:0x676A, store type `0x00C3` at DS:0x6768, then clear the phase byte. Six
direct vectors per routine prove inactive and active phases, exact store order,
DS ownership against GS/ES decoys, register and flag effects, and near return.
The radio handler's isolated far-call boundary additionally proves that its
loader runs after all stores with AX=1 and DS:SI=DS:0x0D16.

Turbo C 2.01 medium `-O -Z` emits the exact seven-instruction, 25-byte LEDATA
shape for `0x008713`; FIXUPP records cover the five zero address words at byte
offsets 2, 8, 11, 15, and 21. Binding those externals to DS:0x2565, 0x6754,
0x676A, 0x6768, and 0x2565 reproduces the original bytes. `0x008848` is not
exact: the near-pointer Watcom pragma supplies the correct AX/SI call arguments,
but Watcom emits 14 instructions/40 bytes versus 10/36 because it preserves ES
and splits the return; Turbo C emits 14 instructions with stack arguments.

Back-buffer row copy `0x00933A` takes x, row, and width in BX/CX/DX and loads
the source/destination far-pointer segments from GS:0x0ABC and GS:0x5229. Twelve
direct vectors prove zero, partial, full-row, and 64 KiB-wrapped copies; GS
ownership against DS/ES decoys; exact `rep movsb` entry/exit state; preservation;
and every status flag from the final offset addition. They also expose the
routine's domain assumptions: its byte-swap plus shift equals `row * 320` only
for `row <= 255`, it discards both stored far-pointer offsets, and `rep movsb`
expects DF clear. All ten recovered callers keep rows at 0..199, and the game
buffers use normalized offset-zero pointers.

The one-to-one candidate therefore keeps the natural `row * 320 + x` expression
and invokes the ordinary DOS `_fmemcpy` far-memory primitive. Open Watcom's
intrinsic form honors the BX/CX/DX pragma and compiles without warnings to 34
instructions/69 bytes versus 24/42 original. It selects `rep movsw` plus a byte
tail and emits generic based-segment loads, so it is not code-shape exact. Turbo
C 2.01 medium emits 32 instructions and a near library `_fmemcpy` call.

Presentation mode selector `0x009510` has fifteen direct vectors covering the
bit-1 bypass, signed minimum/maximum, and both sides of frame thresholds 22,
67, 112, and 157. They prove that only DS:0x2793/0x2795 are accessed, mode bits
4..7 are replaced while the high byte and unrelated low bits survive, and the
new state is returned in AX with BX/DX restored. This AX result changed the
natural signature from `void` to `uint16_t`; the known caller discards it, but
it remains part of the recovered routine contract.

Open Watcom `-3 -ox -mm` compiles that candidate without warnings to the same
25-instruction count and 59 bytes versus 58 original. It uses byte-sized
mask/test operations, constant mode assignments, and swaps the internal BX/DX
roles, so the shape is close but not exact. Turbo C 2.01 medium emits 41
instructions.

Byte-parser opcode-07 handler `0x007684` contains a stale-flag dependency that
the earlier candidate misread as an asset-id sign test. Its `CBW` sign-extends
the source byte but does not change flags; the sole proven caller's dispatch
index `ADD AX,AX` leaves SF clear for opcode 0x07. Nine vectors entering through
that real dispatcher prove the arithmetic path for IDs 0x00, 0x01, 0x02, 0x03,
0x04, 0x80, and 0xFF, including shipped 0xFF -> 0x0DB7. Three direct-entry
controls prove that an artificial incoming SF instead selects the unchanged
sign-extended value. The vectors also cover both cursor updates, printable
bounds, unconsumed low/high stopping bytes, source and destination 64 KiB
wraps, DS/ES/GS ownership, outputs, final flags, source immutability, and near
return. `DESCRIPT.DES` contains 448 opcode-07 records using IDs 1, 2, 3, 4,
and 0xFF, so the corrected high-id behavior is exercised by shipped data.

The one-to-one natural candidate models the complete reachable caller contract
with direct SI input/result, 16-bit wrapping arithmetic, and volatile
named-segment cursor pointers. Open Watcom `-3 -ox -mm` compiles it without
warnings to 35 instructions/82 bytes versus 22/54 original; Turbo C 2.01
medium emits 63 instructions. Watcom preserves the signed load, equivalent
offset arithmetic (`id*16+0x0DC7`), two 16-bit based pointers, copy bounds, and
unconsumed stop, but saves BX/DX/ES, reloads `GAME_DATA`, and uses scalar
loads/stores rather than the original ambient ES=GS and string instructions.
The out-of-contract incoming-SF branch remains a documented machine-level ABI
fact rather than an artificial flag parameter in the natural C function.

Byte-parser opcode-0B handler `0x0076EA` has the same stale-SF shape before a
larger path-selection tail. Seven real-dispatch vectors and three direct-entry
controls prove that normal execution always applies the signed-id offset
arithmetic, including 0xFF -> 0x0DB7, while only artificial SF-set entry stores
the sign-extended id unchanged. They also prove the printable copy and
unconsumed stop, source wrap, ES destination versus GS state ownership, EMS
preference over XMS, XMS fallback, no-backend path, restored parser state,
path-dependent full-EAX clearing, final flags, and near return.

The helper bodies are separately recovered routines, so the call vectors place
a single `RETF` at each runtime helper entry. This preserves the original
handler and far-call stack mechanics while isolating its boundary: the EMS path
enters `01CE:0712` with DS:SI=GS:0x2137; the XMS path enters `01CE:0621` with
that same path and ES:DI loaded from GS:0x5229. These vectors verify caller ABI
and cleanup, not the helper bodies' independent behavior.

Open Watcom `-3 -ox -mm` compiles the direct-SI, named-game-data candidate
without warnings to 44 instructions/117 bytes versus 47/106 original; Turbo C
2.01 medium emits 72 instructions. Watcom preserves the logic in fewer
instructions than the original, but its natural far calls pass the path offset
in AX and the destination in CX:BX. Drop-in linkage therefore needs two narrow
adapters to the game's DS:SI and ES:DI helper conventions. Watcom's 16-bit
`#pragma aux` also rejects EAX as a clobber name, so the original call-only
`XOR EAX,EAX` remains an explicit machine-ABI boundary.

Byte-parser opcode-0C handler `0x007754` and opcode-0D handler `0x007776`
consume different record forms through GS-owned 16-bit destination cursors.
Eight direct vectors for `0x007754` prove printable bounds, unconsumed low/high
stops, NUL placement, source and destination wrap, ES writes versus GS cursor
and count state, fixed 16-byte cursor advance, count wrap, AX/SI/DI outputs,
ADD-carried CF plus INC-derived final flags, preservation, and near return.
Seven vectors for `0x007776` prove aligned and unaligned leading-word copies,
arbitrary high string bytes, embedded and consumed NUL, both wraps, DS/ES/GS
ownership, final cursor, outputs, zero-test flags, preservation, and return.

Their one-to-one candidates now return SI directly and use volatile named-data
based pointers. The first retains a natural printable loop and fixed slot/count
updates; the second uses a typed word assignment followed by a do-while byte
copy. Open Watcom `-3 -ox -mm` compiles them without warnings to 27
instructions/60 bytes and 23/49 respectively, versus originals of 13/34 and
8/18. Turbo C 2.01 medium emits 37 and 35 instructions. Watcom preserves the
operations but saves BX/DX/ES, loads `GAME_DATA`, and uses scalar loads, stores,
and pointer additions instead of the original ambient ES=GS and compact
LODSB/STOSB/MOVSW forms.

Byte-parser opcode-0E handler `0x007788` copies bytes `0x20..0x7F` from DS:SI
to a fixed FS buffer, leaves the stopping byte unconsumed, terminates the
result, and sets a GS-owned dirty flag to exactly one. Eight direct vectors
prove both stop classes, source wrap, FS destination ownership, GS state
ownership, ES restoration, outputs, DEC-derived flags, preservation, and
return. Opcode-12 handler `0x0077A9` accepts `0x21..0x7F`, masks every accepted
byte at least `0x61` with `0xDF`, compares the transformed byte before writing,
sets the changed byte to exactly one after a mismatch, and ORs the unchanged
byte with one only if changed bit zero is clear at the stop. Eleven vectors
cover unusual inputs such as backtick and brace, preexisting even and odd
changed values, source wrap, segment ownership, outputs, flags, and return.

Their one-to-one candidates now consume and return SI directly and use named
FS/game-data objects. Open Watcom `-3 -ox -mm` compiles the actual candidates
without warnings to 26 instructions/53 bytes and 38/87 respectively, versus
originals of 16/33 and 20/52. Turbo C 2.01 medium emits 31 and 47 instructions.
Watcom preserves the natural operations, but its register saves and named
segment reloads do not reproduce the original ambient ES/GS/FS setup or string
instruction allocation.

Presentation-line helper `0x007E1C` is entered with its 24-byte record at
SS:BP and returns completion in carry. Twelve direct vectors prove its busy
early return, loaded and unloaded paths, modulo-16-bit resource-name indexing,
resource header read, forward and reverse initialization/advancement, zero and
wrapping frame edges, completion state, SS record versus DS globals and FS
names, preserved registers, final flags, and carry result. Isolated `RETF`
boundaries additionally prove the resource loader receives DS:SI at
`FS:0x0C04 + (resource_id << 4)` and ES:DI from DS:0x0A80, while the entity
setter receives AX=4, the same ES:DI, BX/CX coordinates, and the frame in BP.

The natural typed candidate now represents the filename as a real far pointer;
Watcom consequently emits a segment relocation instead of treating the FS
table as near DS data. Open Watcom `-3 -ox -mm` compiles the actual candidate
without warnings to 59 instructions/161 bytes versus 60/152 original; Turbo C
2.01 medium emits 78 instructions. The similar counts are not an ABI match:
Watcom uses AX for the record pointer, ordinary helper-call conventions, and AX
for the logical Boolean result. An attempted BP parameter pragma is rejected
by Watcom with E1122, so BP input and carry output remain narrow integration
boundaries rather than inline-assembly code in the candidate.

Byte-parser opcode-08 handler `0x0076BA` is a six-byte leaf: LODSW consumes one
little-endian word from DS:SI, a GS-qualified store writes it to offset 0x1FA5,
and RET preserves all incoming status flags. Eight direct vectors prove aligned
and unaligned loads, SI wrap from 0xFFFE, load-before-store order, distinct
DS/GS ownership against segment decoys, AX/SI outputs, complete status-flag and
register preservation, source immutability, exact bytes, and near return.

The one-to-one candidate is a post-incremented near-word dereference assigned to
a volatile named based-segment global, with the advanced cursor returned
directly. Open Watcom `-3 -ox -mm` compiles it without warnings to 10
instructions/19 bytes versus 3/6 original; Turbo C 2.01 medium emits 14
instructions. Watcom preserves the C behavior but saves DX/ES, loads the named
segment through them, and emits MOV plus ADD instead of the original ambient
GS store and LODSW. Fixed GS placement remains the only integration boundary.

Matrix-slot clear `0x00963F` uses BP without an override, so its six stores are
to SS:0x2A1B rather than the previously recorded GS segment. Each iteration
zeros only the first word and advances 24 bytes. Five direct vectors prove all
six addresses, arbitrary untouched record tails, SS ownership against DS/ES/GS
decoys, every preserved register, final ADD flags, and four-byte RETF stack
consumption. The only caller emits `PUSH CS` followed by a near `CALL`, which
constructs that far-return frame without an inter-segment call instruction.

A natural pointer-to-end loop is a better compiler formulation than the
initial array-index loop. Open Watcom `-3 -ox -mm` compiles the actual candidate
without warnings to 8 instructions/19 bytes versus 12/23 original; Turbo C
2.01 medium also emits 8 instructions. Both generated modules explicitly
assume DS:DGROUP and SS:DGROUP, making the named DS-relative object equivalent
to the original SS-relative storage in the medium-model game. Watcom naturally
preserves every register and emits the far return, but ends the loop with CMP,
so exact integration must still account for final flags from CMP instead of
the original final BP ADD.

Projection-matrix builder `0x0098B9` has another BP-default segment boundary:
the three indexed trig-pair reads are from SS:0x4F45, while the routine first
sets DS and ES from GS for the angle words, persisted six-dword term workspace
at 0x2F7D, and nine-dword matrix at 0x2F95. Twelve direct vectors exercise
zero and identity matrices, mixed signs, signed extrema, deliberate modulo-32-
bit overflow, repeated indices, and the recovered 0..180 table boundary. They
prove every persisted term and matrix value, all nine STOSD destinations,
segment isolation, full register/segment restoration, final defined SAR flags,
and RETF. DF must be clear at entry, as required by the surrounding ABI.

The corrected candidate removes two source-only helper functions so one
assembly routine again corresponds to one C function, and restores the
previously omitted 24-byte workspace side effect. Open Watcom `-3 -ox -mm`
compiles it without warnings to 248 instructions/737 bytes versus 104/343
original; Turbo C 2.01 medium emits 281 instructions. Both preserve the natural
arithmetic, but neither emits the original inline 32-bit 386 multiply sequence:
Watcom calls `__I4M` and implements each arithmetic shift as a 15-iteration
16-bit SAR/RCR loop. Its based-segment form also leaves AX and ES clobbered, so
the C declaration exposes those clobbers for a full rebuild; a drop-in binary
replacement would still require a narrow preservation boundary.

Point plotter `0x009B04` reads projected x/y/depth through SS:BP, compares
against signed DS clip bounds, and addresses a normalized ES framebuffer. Its
row calculation byte-swaps the y word to obtain y*256 and adds y*64+x. Fourteen
direct vectors cover both sides of all four clip edges, occupied-pixel
rejection, depth nibbles 0/1/15, 16-bit offset wrap, every path's defined flags,
segment isolation, full preservation, and near return. Deliberately admitted
rows -1 and 256 prove the machine formula differs from natural y*320 there;
the live-verified game viewport [0,320]x[0,200] keeps every shipped row in the
equivalent 0..255 domain.

Open Watcom `-3 -ox -mm` compiles the game-data context plus far-framebuffer
candidate without warnings to 39 instructions/98 bytes versus 30/68 original;
Turbo C 2.01 medium emits 54 instructions. Watcom performs the same signed
clipping, zero test, offset, and shade but takes the context in AX and the far
framebuffer in CX:BX, switches ES between both based objects, and accounts for
the framebuffer's offset. The original's implicit BP context, segment-only ES
framebuffer, and preserve-all boundary remain integration constraints rather
than register emulation in the C source.

Point-cloud initializer `0x009B67` sets ES from GS, starts DI at 0x2FC1, and
performs three far PRNG calls with AX=0xFFFF for each of 1,000 records. STOSW
writes x/y/z and advances six bytes; ADD DI,2 skips the fourth word. Four
scripted complete-cloud vectors verify 12,000 PRNG entries across the suite,
including AX/CX/DI/ES and far-return stack state, then verify all resulting
component stores, all scratch words, DS/SS decoys, return registers/segments,
CX=0, final ADD flags, and RETF.

The natural candidate is a typed GAME_DATA pointer-to-end loop. Open Watcom
`-3 -ox -mm` compiles it without warnings to 20 instructions/57 bytes versus
22/49 original, retaining the three AX-register far calls, ordered stores, and
eight-byte stride. It uses BX and an end-pointer CMP, saves BX/DX, and leaves AX
and ES clobbered rather than using DI/CX/LOOP and restoring AX/DI/ES. Turbo C
2.01 medium emits 35 instructions because it passes each modulus on the stack
and carries a four-byte far pointer. This is close natural codegen, but not yet
a drop-in ABI match.

Depth-scroll step `0x00B75C` gives opening bit zero precedence over closing bit
zero. It changes only the low byte of the depth word: opening then compares the
whole signed word with 0x41, while closing branches directly on the sign flag
from the byte subtraction. Seventeen direct vectors cover inactive flag high
bits, precedence, completion, progress/equality/overshoot, low-byte wrap with
AH preserved, signed high words, zero steps, and both closing clamp outcomes.
They also prove DS ownership against GS/ES/SS decoys, untouched adjacent bytes,
all-register preservation, path-specific defined flags, and near return.

Representing the local as a natural little-endian word/byte union is materially
closer than splitting and recombining a word and byte. Open Watcom `-3 -ox -mm`
compiles the actual candidate without warnings to 27 instructions/75 bytes
versus 29/76 original and emits the exact `ADD AL,[step]` and `SUB AL,[step]`
operations. It still leaves AX clobbered, emits separate returns instead of the
original AX/BX-preserving shared epilogue, uses an immediate clamp store, and
inserts `TEST AL,AL` before the closing branch, changing final flags from the
original SUB. Turbo C 2.01 medium emits 36 instructions with a stack-resident
union. The sole caller immediately invokes `0x00B6DD` and does not branch on
the returned flags, so these are integration boundaries rather than missing
game-state logic.

SND driver wrapper `0x00BB9D` saves AX/DS/ES, switches DS to GS, zeroes AX,
far-calls the external reset vector at DS:0x0CDF, and clears DS:0x0BA0 after
the callback returns. Six patched-callback vectors prove the exact AX/DS/ES
and far-return-frame state at callback entry, GS vector ownership against an
incoming-DS trap, clear-after-callback ordering, callback register and flag
effects, wrapper restoration, and RETF. The callback must preserve the active
DS until the wrapper performs its pending-byte store.

The initial no-argument candidate omitted the real AX command. Changing the
callback type to accept a 16-bit command and calling it with `0u` lets Open
Watcom `-3 -ox -mm` emit `XOR AX,AX`, the indirect far call, byte clear, and
RETF in 4 instructions/12 bytes versus 12/22 original. The generated function
assumes normal C DS and caller-clobber conventions, so it omits the explicit
DS=GS and AX/DS/ES save/restore envelope. Turbo C 2.01 medium emits 6
instructions but passes zero on the stack, which is incompatible with the
external driver's observed AX interface. A full rebuild can bind game data to
DGROUP and compile callers around Watcom's clobbers; a binary drop-in still
needs the original narrow wrapper boundary.

EMS transfer dispatcher `0x00BD09` was previously mislabeled as a timer. It
does not update GS:0x0B9F; that byte is a transfer-mode selector. Two wrapping
byte decrements partition all values into mode 0 or 0x81..0xFF for EMS page
copy `0xBD26`, mode 1 for XMS buffer setup `0xBD4E`, and modes 2..0x80 for
file-page read `0xBD8D`. All branches forward the live AX value and ES:DI
destination. Exhaustive 256-mode direct vectors prove the selected helper,
path-dependent BL, near-call frame, GS ownership against DS decoys, selector
immutability, BX restoration, final DEC flags, and near return. Both recovered
callers establish DS=GS before entry.

A natural signed-byte local with two pre-decrement tests is closer than range
comparisons. With AX and ES:DI helper pragmas, Open Watcom `-3 -ox -mm` emits
both `DEC BL` operations and the three direct near calls in 22 instructions/38
bytes versus 13/29 original. It inserts `TEST BL,BL` after each decrement,
saves an otherwise unnecessary DX copy of ES, and duplicates the epilogue.
Turbo C 2.01 medium emits 32 instructions; although it branches directly from
each decrement, its stack parameters do not reproduce the live AX/ES:DI ABI.

Centered-layout helper `0x000E62` consumes columns in AX and rows in BX. It
computes `width = columns*4+4` and `height = rows*6+4` with 16-bit wrapping,
then centers that rectangle with logical right shifts after wrapping unsigned
subtractions from 320 and 200. It calls the black-fill helper at
`0x0299:0x0CDC` and the color-15 outline helper at `0x0299:0x0BB5`; both receive
AX=color, BX=x, CX=y, DX=width, and BP=height. The routine returns the two-pixel
inset coordinate in AX/BX. Twelve direct vectors prove wrapped dimensions,
oversized unsigned centering, both complete helper call frames, the result,
register preservation, final ADD flags, and RETF.

The natural candidate returns one packed 32-bit value, with x in its low word
and y in its high word, allowing a Watcom pragma to expose the original BX:AX
result without a struct-return buffer. Open Watcom `-3 -ox -mm` emits 35
instructions/76 bytes versus 32/71 original and preserves CX/DX/BP. Its first
four helper arguments are exact, but height is pushed on the stack: Watcom
forbids BP as a modified custom-ABI register in a 16-bit small-data model.
Turbo C 2.01 medium emits 46 instructions with all parameters on the stack and
returns the packed value through DX:AX. The remaining BP argument is an ABI
integration boundary; the C arithmetic and call semantics need no emulator or
synthetic wrapper.

Object-heap access helper `0x00149B` reads its directory pointer and object-heap
segment directly through DS, which aliases game data at its sole caller. The
offset half of the heap pointer at DS:0x6724 is ignored: each directory +0x10
word is an absolute offset within segment DS:0x6726. Entry zero is processed
unconditionally; after each iteration, the next 20-byte entry is processed only
when its +0x12 kind equals one. A qualifying object has any kind bit from
0x0118 and low flag bit 0x02, and its byte at +0x14 increments with wrap. Six
direct vectors prove selector and pointer ownership, both gates, the unusual
first-entry rule, wrapping offsets, duplicate object references, byte wrap,
final CMP flags, full preservation, and near return.

The corrected candidate uses `FP_SEG`/`MK_FP` to state the segment-only heap
rule and caches that segment before the loop, matching the binary's one-time
load. Four Watcom-only inline instructions save and restore AX/ES around the
natural body because `modify exact []` does not force preservation on a
definition. Open Watcom `-3 -ox -mm` emits 30 instructions/64 bytes versus
20/47 original and retains the two TESTs, conditional INC, 20-byte stride,
terminating CMP, and full runtime register contract. It alternates ES between
directory and heap instead of loading the directory into DS, and conservatively
saves extra registers around the inline boundary. Turbo C 2.01 medium emits 29
instructions with stack-frame temporaries. These are register-placement
boundaries, not missing C logic.

Palette upload helper `0x00178B` tests bit zero of DS:0x5B55. On the dirty
path it far-calls `0000:05D7`, loads SI with the palette at DS:0x5251,
far-calls the 768-byte VGA DAC writer at `0299:0000`, and only then clears the
dirty byte plus DS:0x0A40 and DS:0x0A3E. The first helper maps to recovered
routine `0x000BD7`: GS:0x0A9E comes from BIOS Data Area word `0x40:0x63`, so
its `base+6` bit-3 polling is a calibrated VGA retrace-phase wait, not an audio
gate. Seven direct vectors prove clean and dirty values, both exact far-call
frames, DS versus GS ownership, clear-after-call ordering, the untouched
secondary mouse latch and palette bytes, path-specific SI, flags, and near
return.

Open Watcom `-3 -ox -mm` compiles the natural conditional and two calls to 14
instructions/36 bytes, the same byte count as the original 9 instructions.
Two inline instructions preserve incoming AX, which the navigation caller uses
immediately after return; a local call-clobber declaration keeps the SI palette
load after the retrace call. Watcom otherwise reuses zero in AL for the three
stores. Turbo C 2.01 medium emits 11 instructions, but stack-passes the palette
pointer and does not preserve AX. The natural logic is recovered; exact
replacement still depends on the original far-helper and fixed-DGROUP linkage.

VM patch-stream helper `0x001D74` consumes the DOS read count in AX and packed
three-byte `{u16 destination_offset, u8 value}` records from the GS:0x0ABC far
stream. `LES DI,GS:[0x671C]` is immediately followed by `MOV DI,AX`, so the
stored pointer offset is deliberately discarded: every record offset is
absolute within only the destination segment. The loop subtracts three modulo
16 bits and returns the last destination offset in AX. Four direct vectors
prove GS pointer ownership against DS/ES decoys, absolute offsets 0 and 0xFFFF,
ordered duplicate overwrites, source wrap including a word load at offset
0xFFFF, the zero-count 65,536-iteration path, full non-result preservation,
final SUB flags, and near return.

The corrected natural candidate uses a packed record and standard DOS
`FP_SEG`/`MK_FP` construction to express the segment-only destination without
emulating registers or memory. Open Watcom `-3 -ox -mm` accepts the AX
input/result pragma and emits 30 instructions/66 bytes versus 19/32 original.
It retains the word load, byte store, three-byte source step, subtract, and
loop, but uses ES for both far objects plus a stack local for the source segment
instead of the original simultaneous DS:SI and ES:DI allocation. Turbo C 2.01
medium emits 34 instructions and stack-passes the count. The remaining gap is
compiler segment/register placement, not missing patch semantics.

Mouse button helper `0x001FBC` is not a simple primary-precedence edge test. It
loads the current low byte into a mutable working value, conditionally ANDs that
value with the previous low byte for button one, and then tests button two on
the possibly changed result. Consequently current `3` with previous `2`
suppresses a primary edge, current `3` with previous `1` suppresses a secondary
edge, and current `3` with previous `0` sets only the primary latch because the
first AND clears the working value. Fifteen direct vectors prove those overlap
cases, DS ownership, a separately observable second previous-byte read, a final
full-word current-state reload, all three latch effects, AX, preservation,
flags, and near return.

The natural candidate keeps that destructive byte flow and volatile access
order directly. Open Watcom `-3 -ox -mm` accepts the AX-only result declaration
and emits 20 instructions/54 bytes versus 16/50 original. It preserves the
logical branches and stores, but emits `TEST AL,AL` after both `AND`s and
outlines the secondary stores into a shared-return branch. Turbo C 2.01 medium
emits all 16 original mnemonics within a 29-instruction stack-frame body. The
remaining mismatch is optimizer and fixed-DGROUP placement, not omitted input
logic.

Palette helper `0x00248B` disproves its old render-state label. GS:0x5251 is
the 768-byte live DAC palette used by the upload, resource palette, and blend
routines. Clearing 0x90 dwords from that address zeroes exactly the first 576
bytes: RGB entries 0 through 191. The upper 64 entries, where the UI/console
color banks live, are deliberately preserved. Four direct vectors prove the
ascending extent, surrounding bytes, GS ownership against DS/ES/FS decoys,
full register and segment preservation, final XOR flags, far return, and the
binary's inherited-direction behavior when DF is set.

The natural candidate performs one far `_fmemset` over the named 576-byte scene
palette region. Four Watcom-only push/pop instructions preserve EAX and ES;
the function declaration makes Watcom preserve every remaining register. Open
Watcom `-3 -ox -mm` emits 27 instructions/42 bytes versus 15/27 original. Its
mnemonic multiset contains every original instruction and 14 of 15 mnemonics
remain in order, but it lowers the even byte count to `REP STOSW` followed by a
zero-length residual `REP STOSB`, rather than `REP STOSD`, and adds conservative
saves. Turbo C 2.01 emits a 15-instruction wrapper around its far memset library
instead of an inline clear. Natural C assumes the normal DOS C ABI invariant
that DF is clear on entry; the direct oracle still records the original
descending write extent for completeness.

String comparator `0x0025A4` consumes its left string through DS:SI and its
right string through ES:DI, preserves every register and segment, and returns
equality only in carry. Ten direct vectors cover empty and ordinary equality,
first/middle mismatch, both prefix directions, high bytes, independent SI and
DI offset wrap, segment decoys, immutable input, full preservation, flags, and
far return. A DF-set vector also records the binary's asymmetric behavior:
`LODSB` walks the left string backward while explicit `INC DI` still walks the
right string forward.

The corrected natural candidate caches each left byte once, models the DS side
as a near pointer and the ES side as a far pointer, and returns an ordinary C
Boolean. Open Watcom `-3 -ox -mm` binds SI and ES:DI directly and emits 18
instructions/28 bytes versus 16/22 original. The compare loop and pointer
preservation are close, but Watcom emits `TEST` instead of `OR`, materializes
zero/one in AX, duplicates the far epilogue, and cannot express a carry return
that preserves incoming AX. Turbo C 2.01 medium emits 22 instructions with
stack arguments. Exact binary integration therefore needs a small
Boolean-to-carry/AX-preservation adapter; the C logic itself is complete.

Far string-length helper `0x002665` is a bounded routine rather than an
ordinary unbounded library `strlen`. It probes at most `0xFFFF` bytes through
ES:DI. A terminator can therefore yield lengths zero through `0xFFFE`; if all
`0xFFFF` probes are nonzero, the routine also returns `0xFFFE`. Nine direct
vectors cover both indistinguishable boundary outcomes, empty and high-byte
strings, ascending offset wrap, ES ownership against segment decoys, immutable
input, register and segment preservation, final `SUB` flags, and `RETF`. A
descending vector records that the binary inherits DF and walks backward,
which is outside the normal clear-DF DOS C ABI contract.

The corrected natural candidate uses a `0xFFFF`-bounded length loop and the
recovered ES:DI argument plus AX result declaration. Open Watcom `-3 -ox -mm`
compiles both the probe and actual candidate without warnings to 11
instructions/21 bytes, versus 11/19 original. It preserves DI, leaves CX
untouched, and emits the far return, but uses a scalar `CMP`/`INC` loop instead
of `REPNE SCASB`. Turbo C 2.01 medium emits 18 instructions with a stack far
pointer. The natural logic is complete for conforming C callers; reproducing
the inherited-DF path would require an assembly boundary and is not appropriate
inside this C function.

Unsigned square-root helper `0x002E33` consumes its 32-bit value in DX:AX and
uses one of four BX seeds (`0x000F`, `0x00FF`, `0x0FFF`, or `0xFFFF`). Values
with a high word at least `0xFFFE` return the input low word immediately. Every
other nonzero input iterates a 32-by-16 `DIV`, forms the carry-aware mean of the
quotient and old estimate with `ADD`/`RCR`, and returns the first candidate that
does not decrease. A 404-vector direct oracle covers all byte-sized inputs,
seed transitions, square neighbors, full-width boundaries, and deterministic
32-bit samples. It verifies every intermediate dividend, seed, quotient,
remainder, and candidate in addition to the result, preservation, final flags,
inherited DF, and far return.

The natural candidate expresses that exact seed ladder and Newton loop without
inline assembly. Its Watcom declaration binds the input to DX:AX, returns AX,
and preserves every other recovered register. Open Watcom `-3 -ox -mm`
compiles both the probe and actual candidate without warnings to 49
instructions/104 bytes versus 35/64 original; Turbo C 2.01 medium emits 51
instructions. Watcom reproduces the branch structure and carry-aware average,
but standard 32-bit C division calls `__U4D`. The assembly exploits the seed
invariant that every quotient fits 16 bits and executes one `DIV BX`, a fact C
cannot express directly. Keeping this as natural division preserves the proven
logic; a hardware-DIV intrinsic can remain a narrow later integration option.

Framebuffer band-fill siblings `0x003D7B` and `0x003DBF` consume color in AL
and differ only in selecting the display pointer at GS:0x5221 or backbuffer
pointer at GS:0x5229. Both discard the stored pointer offset, use its segment,
compute the destination with the binary's wrapping byte-swap-plus-shift row
formula, compute `(bottom - top) * 80` dwords modulo 16 bits, replicate AL over
EAX, clear DF, and execute `REP STOSD`. Ten independent direct vectors per
routine verify GS ownership, segment-only pointer use, zero and full-screen
bands, row/count wrap, height underflow, dword offset wrap, exact destination
bytes, preservation, final shift flags, CLD, and far return. This also corrects
the older single-scanline description of `0x003D7B`; it fills a row band.

The corrected one-to-one candidates use named GAME_DATA controls,
`FP_SEG`/`MK_FP` to preserve the segment-only framebuffer behavior, explicit
16-bit arithmetic, and natural dword loops. Their Watcom declarations bind the
byte-valued parameter through AX and preserve all registers. Open Watcom `-3
-ox -mm` compiles each actual candidate without warnings to 52
instructions/114 bytes versus 30/68 original; Turbo C 2.01 medium emits 77
instructions. Watcom splits each dword into two word stores and emits a scalar
loop instead of `REP STOSD`. It also relies on the normal clear-DF C ABI instead
of reproducing the binary's unconditional `CLD`, which remains a narrow
integration boundary rather than a reason to put assembly into the C logic.

Fullscreen-copy siblings `0x003E46` and `0x003E5B` consume a near source in
DS:SI, retain both the segment and offset of the display or backbuffer pointer
from GS, clear DF, and copy exactly `0x3E80` dwords (64,000 bytes) with `REP
MOVSD`. Six independent direct vectors per routine prove source and destination
ownership, nonzero offsets, separate and simultaneous 16-bit offset wrap,
exact destination extent, immutable source and sibling buffer, full register
and segment preservation, all non-DF flags, CLD, and far return.

The one-to-one candidates express the operation as ordinary `_fmemcpy`, bind
the source through SI, and retain the named GAME_DATA far pointer. Four
Watcom-only push/pop instructions preserve AX and ES because the intrinsic's C
boundary otherwise exposes those implementation registers. Open Watcom `-3
-ox -mm` compiles each actual candidate without warnings to 35 instructions/53
bytes versus 13/21 original. It uses `REP MOVSW` followed by a zero-byte `REP
MOVSB` tail and assumes the standard clear-DF C ABI. Turbo C 2.01 medium emits a
14-instruction wrapper that stack-passes the source and calls its far-memory
library. The data operation is fully represented in natural C; direct `REP
MOVSD` and unconditional CLD remain narrow integration/codegen differences.

Three sibling XDB alien slot-11 methods are independently proven state anchors:
AMER `0x000B0F`, CROOLIS `0x000B50`, and SCRUT `0x000B55`. Seven direct
raw-overlay vectors per entry verify the `DI` near-context input, the `DS`
state pointer load, 16-bit `+0x5E` pointer bias, signed-word mutation at the
original state's `+0xB0`, overlay-specific `CS` cursor publication, `SI`
pointer result, modulo wrapping, preservation, `SUB` flags, and near return.
Open Watcom `-3 -ox -mm` compiles each actual candidate without warnings to
the same five instructions and 16 bytes as the original. A named `_CODE`
segment data object produces the required `CS` override. Watcom's one
remaining semantic codegen mismatch is `ADD word,-15` instead of `SUB word,15`:
the stored word is identical, but the arithmetic flags differ. The cursor
symbol also remains a relocation that the overlay linker must place at
`0x1BC2`, `0x1B2E`, or `0x1BE3`. Turbo C 2.01 medium emits a 19-instruction
stack-argument wrapper and accesses the cursor as far data.

The three alien slot-12 methods split into two verified behaviors. AMER
`0x000B1F` and CROOLIS `0x000B60` load a signed delta from `CS:0x0099`,
arithmetic-shift it right in `AX`, and add only a nonnegative result to the
`DS` state at `+0xB0`; ten raw-overlay vectors each verify rounding, negative
suppression, wrapping, ownership, `AX`/`SI`, preservation, flags, and near
return. Watcom compiles each actual candidate without warnings to 9
instructions/20 bytes versus 6/16. It returns the half-delta in `AX`, but uses
saved `BX` instead of `SI` for state and inserts `TEST`/`JL` instead of direct
`JS` from `SAR`, changing negative-path flags. Turbo C emits 21 instructions
with a stack argument and far delta access.

SCRUT `0x000B65` instead repeats the slot-11 field subtraction without the
cursor publication. Seven vectors verify the missing `CS` write as well as
the pointer/field wraps, `SI` result, preservation, flags, and near return.
Watcom emits four instructions/12 bytes versus four/11 original: it performs
an equivalent-result `ADD word,-15` at state `+0xB0`, then adds `0x5E` to
`SI`. Memory and `SI` match, but the reordered final arithmetic flags do not.
Turbo C emits a 16-instruction stack-argument wrapper.

The AMER `0x001BEA`, CROOLIS `0x001B46`, and SCRUT `0x001BFB` slot-13
methods implement a resume-state dispatcher. Six raw-overlay vectors per
method prove that a nonzero near callback at context `+0x36` is tail-jumped
with the caller's original stack frame and `DI` context, while zero installs
the overlay-specific resume offset and clears context `+0x38/+0x3A`. Open
Watcom compiles each actual natural callback candidate without warnings to 9
instructions/22 bytes versus 8/25 original. The initializer has the correct
memory result, but generated callback dispatch uses `AX` and nested `CALL; RET`
instead of `BX` and tail `JMP`; callback-entry `SP` therefore differs. Turbo C
2.01 emits a 19-instruction stack-argument implementation. Exact overlay
integration needs a narrow tail-call adapter, while the natural C control flow
is sufficient for a consistently recompiled caller/callback set.

The byte-identical AMER `0x000347`, CROOLIS `0x00035C`, and SCRUT `0x00035C`
mouse-position helpers are recovered as two natural DS-global assignments plus
a narrow `INT 33h` hardware intrinsic. Six interrupt-hook vectors per overlay
verify ordered `DS:0x002A/0x002C` stores before the driver boundary,
`AX=4/CX=x/DX=y`, stack position, ownership, preservation, and driver result
propagation. Open Watcom compiles every actual candidate without warnings to
the original five-instruction sequence and 14-byte size. Only the two global
relocations remain for the overlay linker to bind. Turbo C 2.01 emits the same
five-operation core inside a 10-instruction stack-argument frame.

The byte-identical AMER `0x000336`, CROOLIS `0x00034B`, and SCRUT `0x00034B`
mouse-bounds helpers are recovered as two typed mouse-driver calls. The first
sets vertical bounds to `0..max_y`; the second sets horizontal bounds to
`0..max_x`. Six interrupt-hook vectors per overlay make the first driver
destroy `AX/CX/DX`, proving that the second call uses the original `CX` value
saved by the routine rather than an incidental register value. They also
verify call order, zero minima, stack state, preservation, and propagation of
the second driver's registers and flags. Open Watcom compiles every actual
candidate without warnings to 11 instructions/21 bytes versus the original
9/17. It saves `max_x` in `BX` with a function prologue/epilogue instead of the
original `PUSH CX`/`POP DX`; the natural semantics are otherwise complete.
Turbo C 2.01 emits a 12-instruction stack-argument implementation with the
same two interrupt operations.

The byte-identical AMER `0x000958`, CROOLIS `0x000999`, and SCRUT `0x000999`
slot-6 methods wrap all three position coordinates for each 94-byte alien
state. Eight raw-overlay vectors per sibling cover the positive and negative
window boundaries, large view origins, input dwords with unrelated high words,
multiple states, wrapped near-state pointers, full `DS` integrity, sign-extended
32-bit stores, scratch outputs, final subtraction flags, and near return. The
method uses only each input dword's low word, maps `position + origin` into
`[-0x4000,0x3FFF]`, then subtracts the origin and sign-extends the resulting
word. The natural unsigned modular arithmetic and `do/while` loop preserve that
logic. The count-zero vector directly proves all 65,536 original `LOOP`
iterations and compares 65,538 bytes from the `DS` base, including the two bytes
written past offset `0xFFFF` by a dword store beginning at `0xFFFE`.

Open Watcom `-3 -ox -mm -zdp` compiles the probe and all three actual candidates
without warnings to 34 instructions/97 bytes versus 31/92 original. Pegging
`DS` to DGROUP matches the overlay entry invariant and avoids inappropriate
`SS` overrides. Watcom keeps the low-word loads and stores, modular windowing,
sign extension, 94-byte traversal, and decrement loop, but uses `BX`, `CWD`,
split word stores, and `DEC/JNE`; scratch registers and final flags therefore
differ. Turbo C 2.01 medium emits 56 instructions with a stack argument.

The byte-identical AMER `0x0002F0`, CROOLIS `0x000305`, and SCRUT `0x000305`
VGA helpers are recovered as natural counted palette and framebuffer clears,
two page-global assignments, two word-sized VGA control writes, and two
status-poll loops. Four port-hook vectors per overlay prove all 769 byte output
writes, both control words, operation ordering, the exact 64,000-byte
`A000:0000` clear while preserving `A000:FA00..FFFF`, and retrace-low followed
by retrace-high synchronization across multiple input sequences. They also
verify DS ownership, full register/segment effects, `CLD`, final `TEST` flags,
and near return.

Open Watcom compiles all three actual candidates without warnings to 37
instructions/86 bytes versus the original 30/70. Port operations remain the
same narrow hardware instructions. Watcom lowers the natural far framebuffer
loop to scalar `ES` stores with `DEC/JNE`; the original uses `CLD` plus `REP
STOSW`, so scratch-register results and flags differ even though observable VGA
and memory behavior agrees. Turbo C 2.01 emits 55 instructions.

The byte-identical slot-8 trio (AMER `0x001B5F`, CROOLIS `0x001ACB`, SCRUT
`0x001B80`) advances a byte cursor by four modulo `0x1000`, subtracts the
previous signed sample from the current sample, and applies that wrapping
delta to the first word of every 20-byte object record. The slot-9 trio (AMER
`0x001B8F`, CROOLIS `0x001AFB`, SCRUT `0x001BB0`) is identical except that it
arithmetic-shifts the current signed sample right by four before storing and
differencing it. Seven raw-overlay vectors per routine cover positive and
negative values, signed overflow, cursor and object-offset wrap, and count
zero. The latter proves that the original `LOOP` executes 65,536 iterations,
which the natural unsigned `do/while (--count)` retains.

Open Watcom compiles all six actual candidates without warnings. Slot 8 is 21
instructions/51 bytes versus the original 17/48; slot 9 is 22/54 versus 18/51.
The compiler uses a natural far `ES:BX` object pointer, preserves `DX/ES`, and
lowers the loop to `DEC/JNE`. The original temporarily installs the object
segment in `DS`, traverses through `SI`, and uses `CX/LOOP`. Memory and declared
`AX` delta results agree, but scratch-register outputs and final flags differ.
Turbo C 2.01 emits 40 and 43 instructions respectively with stack arguments.

Four XDB entries are independently proven one-byte near-return methods: AMER
`0x001DD6`, CROOLIS `0x001D27`, MANU3 `0x000848`, and SCRUT `0x001DE7`. Three
direct raw-overlay vectors per entry verify that only the two-byte return word
is consumed while all registers, segments, tested flags, and following stack
bytes survive. Empty one-to-one C functions compile under both Open Watcom and
Turbo C 2.01 to a single `RET`; Watcom's object byte is exactly `C3`. These
empty functions are accepted recovered behavior rather than stubs.

Eight direct vectors prove the `0x000000` MANU3 far API coordinator. Its caller
supplies signed cursor x/y, a five-bit animation selector, and a framebuffer
window through an 8-byte `SS:BP` request. The active path advances tween state,
temporarily applies cursor-relative yaw and pitch around matrix construction,
projects geometry point `ES:0x02AC` through state `DS:0x24AE` to derive the two
screen-center dwords, then dispatches projection and face building. The matrix
also proves the true inactive jump into `0x000121`, selector masking,
framebuffer-window extremes, camera wrap/restoration, positive/zero/negative
depth, signed matrix inputs, call publication timing, registers, flags, stack,
and all segment owners.

The actual candidate compiles warning-free with Open Watcom. Medium model
`-3 -ox -mm -zdp` emits 170 instructions/510 bytes versus the original 83/289;
Turbo C 2.01 medium emits 192 instructions. Watcom calls `__U4M` nine times and
`__I4D` twice instead of retaining the binary's inline 386 arithmetic. The
natural C body is behaviorally verified, while current-`CS` input, caller
`SS:BP` request acquisition, and active `DS`/`ES`/`FS` installation remain a
narrow far-entry adapter.

The first MANU3 animation chain is recovered as three one-to-one natural C
functions. Entry `0x00017C` is a far wrapper around near selector `0x000181`;
four patched-callee vectors prove its BX argument, inner return word, outer far
return, preservation, and flags. Open Watcom `-3 -ox -mm -zdp` emits the exact
two instructions and four bytes (`CALL near`, `RETF`) from the natural wrapper.
Turbo C 2.01 medium emits seven instructions because it stack-passes the
selector and creates a BP frame.

Four patched-constructor vectors prove that `0x000181` masks the selector to
five bits, indexes a relative-word table based at DS:`0x2306`, clears the phase,
publishes the wrapped script offset, loads BX with the active-list base
`0x1032`, and tail-jumps to the constructor. Watcom emits 10 instructions/29
bytes versus 8/26 original. It retains every original mnemonic class but uses a
normal `CALL`/`RET` pair and different table-index temporaries. Turbo C emits 21
instructions with a stack argument.

Eight patched-constructor vectors prove the `0x00019B` per-frame stepper. It
loads DS from FS, honors both forms of a nonzero phase high byte, publishes the
signed integer half of each Q16 accumulator, decrements its counter, performs a
defined modular add for live records, and swap-removes expired records before
tail-jumping to the constructor with BX at the reduced active-list end. The
cases cover empty input, accumulator wrap, counter values 0, 1, `0x7FFF`, and
`0x8000`, and removal from first, middle, and last positions. Watcom emits 30
instructions/75 bytes versus the 26 listed instructions/69 unique bytes outside
the shared constructor block, and naturally retains the final tail `JMP`.
Turbo C emits 44 instructions. Watcom omits the original defensive DS-from-FS
adapter, uses different scratch registers, splits the dword add, and inserts a
counter comparison after decrement; the source is therefore a verified logical
match with a documented segment adapter, not exact code shape.

Eight direct vectors prove the complete `0x0001DF` constructor: count/phase
gates over packed 8-byte specs; 14-byte records containing counter, target,
Q16 accumulator, and Q16 step; 16-bit wrapped signed deltas; truncating signed
division; multiple records; both script and active-cursor wrap; phase advance;
and the empty-sequence camera/final-state writes. Open Watcom compiles the
natural struct loop to 65 instructions/179 bytes versus 49/145 original. It
uses a stack frame and `__I4D` helper, whereas the binary keeps values in
registers and executes 386 `CDQ`/`IDIV ECX` inline. Turbo C emits 87
instructions and calls `LDIV@`. The source-level data and control flow are
verified; the constructor remains a codegen mismatch rather than an assembly
substitute.

Seven direct vectors prove the complete `0x000270` matrix and tree-transform
loop. Each 94-byte node supplies three masked byte-offset angles into trig pairs
rooted at DS:`0x0026`, a signed radial displacement, a local position, and a
near parent offset. The routine constructs all nine Q15 rotation terms, applies
the radial adjustment including the original rounded Y term, then composes the
node's world translation and matrix with its parent. Cases cover zero, mixed,
masked, and extreme angles, both radial extremes, modular dword overflow, and a
two-node hierarchy while checking full memory, registers, flags, and segments.

Control-flow captures also prove that labeled `0x000477` is not a callable
routine: it consumes live `EAX` from `0x000473`, decrements the shared node
count, and jumps back into `0x000279`. The inventory therefore merges it into
the 729-byte `0x000270` owner. Watcom medium emits 445 instructions/1237 bytes
versus 198/729 original, and Turbo C emits 568 instructions. Watcom calls
`__U4M` for nine products while
the binary retains inline 386 multiplies. This is a verified natural-C
translation-unit recovery, not matching compiler codegen.

Fifteen direct vectors prove the complete `0x000549` entity projector. The
routine walks 94-byte state records and 20-byte geometry vertices, evaluates a
signed 3x4 fixed-point transform, stores depth shifted right by eight, rejects
nonpositive depth, divides the projected x/y values, applies every original
clip and clamp boundary, and optionally copies projected fields through linked
vertex offsets. The matrix covers multiple vertices, multiple states, the
copy tail, offset wrap, and the original count-zero behavior of 65,536 inner
iterations while checking full memory, registers, segments, stack, and flags.

The actual candidate compiles warning-free with Open Watcom. Medium model
`-3 -ox -mm -zdp` emits 230 instructions/712 bytes versus the original
104/368. Watcom calls `__U4M` for all nine 32-bit products and `__I4D` for both
divisions, whereas the binary uses inline 386 `IMUL` and `IDIV`. Turbo C 2.01
medium emits 278 instructions. The source-level records, arithmetic, traversal,
and decisions are verified; the original `ES` geometry and `DS`/`FS` active
data ownership remain a narrow integration adapter rather than natural C
codegen.

Six entry-boundary vectors and ten complete raw-overlay vectors prove the
merged `0x000D7D..0x001366` face activation and gradient routine. `0x000D93`
has no incoming call: the prelude either jumps to the shared `0x000848` return
for an empty free list or falls through with the three vertex offsets and
raster record live. The full matrix covers backface and degenerate rejection,
the vertical-first-edge special case, all three `x1`/`x2` orderings, both
negative-X clipping paths, texture/depth fixed-point equations, texture-bank
segment selection, the 90-byte record layout, free-list pop, and active-list
insertion. The oracle compares every externally retained record byte and link
against an independent arithmetic model; it caught and corrected a first-pass
double-clipping error in the natural C.

The complete candidate compiles warning-free with Open Watcom medium model
`-3 -ox -mm -zdp`. The main function is 803 instructions/2349 bytes, plus a
50-instruction/115-byte natural fixed-point multiply helper, versus the
original 424 instructions/1514 bytes. The generated size reflects stack-frame
temporaries and compiler multiply helpers; the recovered control flow and
record effects are verified. Exact integration still needs the original
`ES` geometry, raster `DS`, and directory `FS` contract installed at entry.

Seven patched-callee vectors prove the `0x000150` no-cursor frame coordinator.
It gates on the relocated data segment at CS:`0x136A`, installs that segment in
`DS`, `ES`, and `FS`, converts the SS:`0x20CE` byte-window offset to
`0xA000 + (offset >> 4)` at DS:`0x0018`, then calls tween step, matrix build,
entity projection, and face builder in that exact order. The vectors separate
all five segment owners, cover low-nibble truncation and high offsets, and
verify call return IPs, publication timing, flags, saved `DS`, and far return.
Watcom emits 15 instructions/37 bytes versus the original 17/44, while Turbo C
emits 14 instructions. The natural body retains the gate, calculation, and
call graph; exact integration still needs a narrow adapter for the `CS`/`SS`
inputs and `DS`/`ES`/`FS` installation.

Six relocated-code vectors prove the `0x000121` MANU3 initialization block.
They execute the original image at multiple nonzero `CS` values and verify
`data = CS + CS:[0x1368]`, publication at CS:`0x136A`, then three cumulative
segment additions from data offsets `0x000C/0x000E/0x0010` into
`0x0002/0x0004/0x0006`. The final work segment receives continuation `0x0AE0`
at offset `0x067E`. Cases cover data and cumulative wrap, zero and maximum
deltas, and a final zero segment while checking `FS`/`ES`, saved `DS`, flags,
memory ownership, and the shared far-return epilogue. Watcom emits 16
instructions/50 bytes versus the original 14/47 and keeps the core chained
adds and stores; Turbo C emits 36 instructions. The natural function exposes
the hidden current-`CS` value as a typed argument. Exact integration still
needs that tiny input adapter, segment installation, and the original jump
through its caller's saved-`DS` epilogue.

Six patched-sorter vectors prove the `0x0006F6` stage prelude. It loads the
geometry segment from FS:`0x0002` into `DS` and the raster/bucket segment from
FS:`0x0006` into `ES`, then falls into face bucket sort at `0x000700` without a
stack change. Zero, equal, high-bit, and maximum segment cases verify exact
segment outputs, full register/flag preservation, and memory ownership.
Natural C exposes those two segment selectors as typed arguments. Watcom emits
`MOV DX,[+6]; MOV AX,[+2]; JMP`, preserving the tail transfer and the original
10-byte length, versus two segment-register loads in the binary. Turbo C emits
6 instructions with stack arguments. Exact integration needs only the
`AX/DX` to `DS/ES` sorter-entry adapter.

Ten sorter vectors and seven complete owner vectors prove the merged
`0x000700..0x000D7C` face renderer. The initial loop walks 8-byte faces and
20-byte projected vertices, rejects triangles whose clip masks share a bit,
rotates the signed lowest-X vertex into slot zero, rejects modular spans at 400
or more, and prepends accepted faces to the corresponding column bucket. The
fallthrough renderer initializes a 200-record free list, activates faces
through `0x000D7D`, constructs depth-ordered vertical span boundaries, samples
the 256x256 texture, and draws Mode-X, four-plane, or linear framebuffer
columns. It then advances current or secondary edges, returns expired records
to the free list, insertion-sorts crossings, and continues through column 319.

The full-owner vectors cover empty initialization, negative-Y clipping,
stepped edges and texture coordinates, all three framebuffer continuations,
exact VGA plane words, both `0x000CCA` and `0x000D19` secondary-edge paths, the
`0x000D5E` removal path, and complete sweep termination. Separate control-flow
analysis proves that `0x000775`, `0x000848`, `0x000849`, and `0x000C2A` have no
call entries: they are respectively the physical renderer fallthrough, shared
return, active-list insertion back edge, and affine inner-loop head. They are
now merged into the single true owner.

The actual candidate compiles warning-free with Open Watcom medium model
`-3 -ox -mm -zdp` to 959 instructions/2876 bytes versus the original 543
instructions/1661 bytes. The generated code retains typed far geometry,
raster, texture, and framebuffer pointers plus the narrow VGA word-output
intrinsic; it contains no register machine or generic memory-access layer.
Exact integration still needs the original live geometry `DS`, raster `ES` and
later `DS`, and active-directory `FS` contract installed around the natural
function.

An exact raw-byte search of 307 recovered BLOODPRG routines of at least eight
bytes over all 20 files in each Turbo C `TC/LIB` tree found zero matches for
both versions. For example, Turbo C 2.01's `CH.LIB` `_strlen` member is a
34-byte stack-argument routine, while BLOODPRG `0x002665` is a 19-byte
register-entry helper using `ES:DI` and `repne scasb`.

Representative best Watcom result per probe, ranked by canonical instruction
LCS and then mnemonic similarity:

| probe | best configuration | original/generated instructions | instruction LCS | mnemonic LCS | byte-line LCS |
| --- | --- | ---: | ---: | ---: | ---: |
| `far_strlen` | medium, `-ox`, register | 11/11 | 0.2727 | 0.4545 | 0.2727 |
| `field_offset` | compact, `-ox`, register | 8/23 | 0.3750 | 0.7500 | 0.3750 |
| `vm_record_lookup_by_threshold` | medium, `-ox`, register | 12/12 | 0.0833 | 0.8333 | 0.1667 |
| `active_object_list_build` | medium, `-ox`, register | 32/28 | 0.2188 | 0.6562 | 0.2500 |
| `ship_3d_position_distance` | medium, `-ox`, register | 88/117 | 0.0682 | 0.5341 | 0.1023 |
| `ship_3d_position_field_resolve` | medium, `-ox`, register | 45/46 | 0.1111 | 0.6667 | 0.2444 |
| `ship_3d_object_table_bit_test` | medium, `-ox`, register | 31/33 | 0.2581 | 0.7419 | 0.3548 |
| `ship_3d_nav_source_list_build` | medium, `-ox`, register | 34/51 | 0.1765 | 0.7647 | 0.2059 |
| `vm_token_special` | medium, `-ox`, register | 9/9 | 0.3333 | 1.0000 | 1.0000 |
| `vm_condition_5` | medium, `-ox`, register | 104/142 | 0.0577 | 0.5096 | 0.0769 |
| `presentation_line_step` | medium, `-ox`, register | 60/59 | 0.1833 | 0.6500 | 0.2333 |
| `segment_global_gate` | medium, `-ox`, register | 4/3 | 0.2500 | 0.5000 | 0.2500 |
| `string_equal_mixed` | medium, `-ox`, register | 16/18 | 0.3750 | 0.6250 | 0.5000 |
| `u32_sqrt_newton` | medium, `-ox`, register | 35/49 | 0.2286 | 0.6571 | 0.2571 |
| `graphics_band_fill` | medium, `-ox`, register | 30/52 | 0.1667 | 0.7667 | 0.2000 |
| `fullscreen_copy` | medium, `-ox`, register | 13/35 | 0.5385 | 0.9231 | 0.5385 |
| `xdb_near_noop` | medium, `-ox`, register | 1/1 | 1.0000 | 1.0000 | 1.0000 |
| `xdb_anchor_state` | medium, `-ox`, register | 5/5 | 0.2000 | 0.8000 | 0.6000 |
| `xdb_apply_delta` | medium, `-ox`, register | 6/9 | 0.1667 | 0.6667 | 0.3333 |
| `xdb_lower_state` | medium, `-ox`, register | 4/4 | 0.2500 | 0.7500 | 0.7500 |
| `xdb_resume_or_init` | medium, `-ox`, register | 8/9 | 0.1250 | 0.6250 | 0.1250 |
| `xdb_mouse_position_set` | medium, `-ox`, register | 5/5 | 0.4000 | 1.0000 | 0.6000 |
| `xdb_mouse_bounds_set` | medium, `-ox`, register | 9/11 | 0.3333 | 0.8889 | 0.5556 |
| `xdb_wrap_positions` | medium, `-ox -zdp`, register | 31/34 | 0.0323 | 0.5484 | 0.0645 |
| `xdb_sample_delta` | medium, `-ox`, register | 17/21 | 0.0588 | 0.8824 | 0.0588 |
| `xdb_scaled_sample_delta` | medium, `-ox`, register | 18/22 | 0.0556 | 0.7778 | 0.0556 |
| `xdb_vga_clear_and_sync` | medium, `-ox`, register | 30/37 | 0.0333 | 0.7667 | 0.5333 |
| `xdb_manu3_api_entry` | medium, `-ox -zdp`, register | 83/170 | 0.0120 | 0.6627 | 0.0602 |
| `xdb_manu3_anim_select_entry` | medium, `-ox -zdp`, register | 2/2 | 0.5000 | 1.0000 | 0.5000 |
| `xdb_manu3_init_protocol` | medium, `-ox -zdp`, register | 14/16 | 0.0000 | 0.7857 | 0.0714 |
| `xdb_manu3_frame_step` | medium, `-ox -zdp`, register | 17/15 | 0.0588 | 0.6471 | 0.1765 |
| `xdb_manu3_anim_select` | medium, `-ox -zdp`, register | 8/10 | 0.0000 | 0.8750 | 0.1250 |
| `xdb_manu3_tween_step` | medium, `-ox -zdp`, register | 26/30 | 0.0385 | 0.6538 | 0.0769 |
| `xdb_manu3_tween_constructor` | medium, `-ox -zdp`, register | 49/65 | 0.0408 | 0.5714 | 0.0408 |
| `xdb_manu3_matrix_build` | medium, `-ox -zdp`, register | 198/445 | 0.0051 | 0.6465 | 0.0101 |
| `xdb_manu3_entity_project` | medium, `-ox -zdp`, register | 104/230 | 0.0096 | 0.5481 | 0.0192 |
| `xdb_manu3_face_builder_next` | medium, `-ox -zdp`, register | 2/3 | 0.0000 | 1.0000 | 0.0000 |
| `xdb_manu3_face_bucket_sort` | medium, `-ox -zdp`, register | 47/83 | 0.0000 | 0.6809 | 0.0426 |
| `xdb_manu3_face_activate` | medium, `-ox -zdp`, register | 6/12 | 0.0000 | 0.6667 | 0.0000 |
| `vm_branch_stack_return` | medium, `-ox`, register | 8/7 | 0.1250 | 0.7500 | 0.1250 |
| `scan_zero_word` | medium, `-ox`, register | 14/11 | 0.2143 | 0.2857 | 0.2143 |
| `vm_script_profile_request` | medium, `-ox`, register | 5/5 | 0.4000 | 0.6000 | 0.4000 |
| `vm_clear_state` | medium, `-ox`, register | 3/5 | 0.3333 | 1.0000 | 0.3333 |
| `vm_record_string_copy` | medium, `-ox`, register | 13/20 | 0.2308 | 0.6923 | 0.3846 |
| `vm_tagged_word_compare` | medium, `-ox`, register | 17/17 | 0.0588 | 0.5294 | 0.2941 |
| `vm_tagged_byte_pair_compare` | medium, `-ox`, register | 28/27 | 0.0357 | 0.7500 | 0.1071 |
| `vm_branch_stack_push` | medium, `-ox`, register | 8/9 | 0.1250 | 0.7500 | 0.1250 |
| `vm_branch_stack_pop` | medium, `-ox`, register | 6/7 | 0.1667 | 0.6667 | 0.1667 |
| `vm_random_branch` | medium, `-ox`, register | 6/6 | 0.1667 | 0.3333 | 0.1667 |
| `vm_conditional_block` | medium, `-ox`, register | 29/32 | 0.0690 | 0.6897 | 0.1034 |
| `vm_script_jump` | medium, `-ox`, register | 4/8 | 0.2500 | 1.0000 | 0.5000 |
| `vm_cond_state_array` | medium, `-ox`, register | 13/18 | 0.0769 | 0.4615 | 0.0769 |
| `strlen_b` | medium, `-ox`, register | 11/11 | 0.2727 | 0.4545 | 0.2727 |
| `vm_presentation_register_set` | medium, `-ox`, register | 5/7 | 0.2000 | 0.6000 | 0.2000 |
| `vm_load_string` | medium, `-ox`, register | 29/38 | 0.0690 | 0.8621 | 0.1034 |
| `vm_conditional_jump` | medium, `-ox`, register | 10/12 | 0.1000 | 0.7000 | 0.3000 |
| `vm_poke_byte` | medium, `-ox`, register | 5/6 | 0.2000 | 0.8000 | 0.8000 |
| `vm_yield` | medium, `-ox`, register | 2/2 | 0.5000 | 1.0000 | 0.5000 |
| `vm_shared_state` | medium, `-ox`, register | 69/87 | 0.1304 | 0.7971 | 0.2464 |
| `vm_shared_bit_state` | medium, `-ox`, register | 31/36 | 0.0323 | 0.4839 | 0.0645 |
| `vm_record_wildcard` | medium, `-ox`, register | 55/56 | 0.0545 | 0.5455 | 0.1273 |
| `vm_cd_record_triple` | medium, `-ox`, register | 82/86 | 0.0366 | 0.6341 | 0.0732 |
| `vm_b7_record_bit` | medium, `-ox`, register | 43/51 | 0.0698 | 0.5116 | 0.0930 |
| `vm_b8_record_pair` | medium, `-ox`, register | 26/37 | 0.1154 | 0.7308 | 0.1154 |
| `vm_c5_record_match` | medium, `-ox`, register | 40/41 | 0.0750 | 0.5750 | 0.1000 |
| `vm_c6_record_match` | medium, `-ox`, register | 31/32 | 0.0323 | 0.5806 | 0.0323 |
| `vm_c7_record_match` | medium, `-ox`, register | 39/49 | 0.0769 | 0.6154 | 0.1026 |
| `vm_c8_record_match` | medium, `-ox`, register | 34/32 | 0.0294 | 0.5294 | 0.0294 |
| `vm_c9_record_clear` | medium, `-ox`, register | 26/29 | 0.1538 | 0.5000 | 0.1923 |
| `byte_parser_mark_b16` | medium, `-ox`, register | 2/2 | 0.5000 | 1.0000 | 0.5000 |
| `credit_presenter_b_cryo` | medium, `-ox`, register | 8/19 | 0.1250 | 0.6250 | 0.1250 |
| `byte_parser_copy_printable` | medium, `-ox`, register | 11/23 | 0.1818 | 0.6364 | 0.2727 |
| `byte_parser_snd_bank_name_load` | medium, `-ox`, register | 22/26 | 0.0455 | 0.4091 | 0.1364 |
| `dlg_line_asset_table_fill` | medium, `-ox`, register | 22/35 | 0.0909 | 0.5455 | 0.1818 |
| `index_lookup_1fd7` | medium, `-ox`, register | 47/44 | 0.0851 | 0.4043 | 0.1064 |
| `byte_parser_copy_131a_entry` | medium, `-ox`, register | 13/27 | 0.1538 | 0.6154 | 0.2308 |
| `byte_parser_stream_0f18_append` | medium, `-ox`, register | 8/23 | 0.1250 | 0.5000 | 0.1250 |
| `fs_name_area_read` | medium, `-ox`, register | 16/26 | 0.2500 | 0.6875 | 0.3125 |
| `music_voc_name_patcher` | medium, `-ox`, register | 20/38 | 0.1000 | 0.4500 | 0.2000 |
| `nav_choice_handler_0` | medium, `-ox`, register | 7/8 | 0.1429 | 0.8571 | 0.1429 |
| `nav_choice_handler_3` | medium, `-ox`, register | 10/14 | 0.1000 | 0.8000 | 0.2000 |
| `back_buffer_copy_from` | medium, `-ox`, register | 24/34 | 0.2083 | 0.7917 | 0.2500 |
| `presentation_mode_bits_update` | medium, `-ox`, register | 25/25 | 0.2000 | 0.8800 | 0.2000 |
| `matrix_table_clear_2a1b` | medium, `-ox`, register | 12/8 | 0.0833 | 0.5000 | 0.0833 |
| `ship_3d_projection_matrix_build` | medium, `-ox`, register | 104/248 | 0.0481 | 0.5962 | 0.0577 |
| `ship_3d_plot_point` | medium, `-ox`, register | 30/39 | 0.1000 | 0.7667 | 0.1000 |
| `ship_3d_point_cloud_randomize` | medium, `-ox`, register | 22/20 | 0.0455 | 0.5909 | 0.1818 |
| `ship_3d_depth_scroll_step` | medium, `-ox`, register | 29/27 | 0.0345 | 0.6207 | 0.0690 |
| `snd_driver_call` | medium, `-ox`, register | 12/4 | 0.0833 | 0.2500 | 0.0833 |
| `ems_transfer_dispatch` | medium, `-ox`, register | 13/22 | 0.3846 | 0.6154 | 0.3846 |
| `layout_offset_calc` | medium, `-ox`, register | 32/35 | 0.1875 | 0.5938 | 0.2812 |
| `object_heap_access` | medium, `-ox`, register | 20/30 | 0.1500 | 0.9500 | 0.3000 |
| `palette_upload_if_dirty` | medium, `-ox`, register | 9/14 | 0.1111 | 0.6667 | 0.1111 |
| `vm_patch_stream_apply` | medium, `-ox`, register | 19/30 | 0.2632 | 0.7895 | 0.2632 |
| `mouse_button_edges_update` | medium, `-ox`, register | 16/20 | 0.0625 | 0.8750 | 0.2500 |
| `palette_scene_entries_clear` | medium, `-ox`, register | 15/27 | 0.3333 | 0.9333 | 0.3333 |
| `byte_parser_store_word_1fa5` | medium, `-ox`, register | 3/10 | 0.3333 | 0.6667 | 0.3333 |
| `vm_dic_lookup_result` | medium, `-ox`, register | 21/38 | 0.1429 | 0.6190 | 0.1429 |
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
