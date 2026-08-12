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
| `vm_record_lookup_by_threshold` | medium, `-ox`, register | 12/12 | 0.0833 | 0.8333 | 0.1667 |
| `active_object_list_build` | medium, `-ox`, register | 32/28 | 0.2188 | 0.6562 | 0.2500 |
| `ship_3d_position_distance` | medium, `-ox`, register | 88/117 | 0.0682 | 0.5341 | 0.1023 |
| `ship_3d_position_field_resolve` | medium, `-ox`, register | 45/46 | 0.1111 | 0.6667 | 0.2444 |
| `ship_3d_object_table_bit_test` | medium, `-ox`, register | 31/33 | 0.2581 | 0.7419 | 0.3548 |
| `ship_3d_nav_source_list_build` | medium, `-ox`, register | 34/51 | 0.1765 | 0.7647 | 0.2059 |
| `vm_token_special` | medium, `-ox`, register | 9/9 | 0.3333 | 1.0000 | 1.0000 |
| `vm_condition_5` | medium, `-ox`, register | 104/142 | 0.0577 | 0.5096 | 0.0769 |
| `presentation_line_step` | medium, unoptimized, register | 60/62 | 0.2167 | 0.7333 | 0.2833 |
| `segment_global_gate` | medium, `-ox`, register | 4/3 | 0.2500 | 0.5000 | 0.2500 |
| `string_equal_mixed` | huge, unoptimized, register | 16/32 | 0.4375 | 0.6250 | 0.5000 |
| `u32_sqrt_newton` | compact, unoptimized, register | 35/51 | 0.1714 | 0.7714 | 0.2286 |
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
| `vm_c9_record_clear` | compact, unoptimized, register | 26/38 | 0.0769 | 0.5769 | 0.1154 |
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
