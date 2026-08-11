; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0067ba
; seg_off: 04da:141a
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a7_set_if_presentation
; label_comment: VM opcode 0xA7: lodsw operand; if gs:[0x67ac]&1 (presentation active) set gs:[0x6770]=operand. Conditional set-state
; incoming: vm_opcode_handlers:opcode_0xa7
; byte_count: 14
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0067ba_vm_op_a7_set_if_presentation.cpp
; routine_bytes_sha256: ab9c9d28c806f026e04825afecc5711063785d9da85b90d16e650f12c5b87bce

0067BA:  AD                           lodsw    ax, word ptr [si]
0067BB:  65 F6 06 AC 67 01            test     byte ptr gs:[0x67ac], 1
0067C1:  74 04                        je       0x67c7
0067C3:  65 A3 70 67                  mov      word ptr gs:[0x6770], ax
0067C7:  C3                           ret     
