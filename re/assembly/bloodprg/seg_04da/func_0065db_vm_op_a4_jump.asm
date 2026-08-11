; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0065db
; seg_off: 04da:123b
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a4_jump
; label_comment: VM opcode 0xA4 JUMP: si=[si] (set script PC to the operand address); clear resume state gs:[0x67b1]=0, gs:[0x6764]=0. Unconditional jump
; incoming: vm_opcode_handlers:opcode_0xa4
; byte_count: 16
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0065db_vm_op_a4_jump.cpp
; routine_bytes_sha256: c1eca78647642bd73e303a1a203042c7e47bc8f3c2d192c1c308430a1fc742a9

0065DB:  8B 34                        mov      si, word ptr [si]
0065DD:  65 C6 06 B1 67 00            mov      byte ptr gs:[0x67b1], 0
0065E3:  65 C7 06 64 67 00 00         mov      word ptr gs:[0x6764], 0
0065EA:  C3                           ret     
