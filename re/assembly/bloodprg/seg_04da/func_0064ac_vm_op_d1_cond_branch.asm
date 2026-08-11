; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064ac
; seg_off: 04da:110c
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_d1_cond_branch
; label_comment: VM opcode 0xD1: if gs:[0x274f]&1 is CLEAR, call vm_branch 0x6462. Conditional branch on game-flag [0x274f] bit0. Completes the conditional-branch opcode family: 0xCE([0x2793])/0xD0([0x252a])/0xD1([0x274f]) - each branches on a distinct game-state flag bit
; incoming: vm_opcode_handlers:opcode_0xd1
; byte_count: 12
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0064ac_vm_op_d1_cond_branch.cpp
; routine_bytes_sha256: 26276d20cbbd6e872d74f96e0ed07e246312dda8c5b5d0ab3f9fb5f5a1f54965

0064AC:  65 F6 06 4F 27 01            test     byte ptr gs:[0x274f], 1
0064B2:  75 03                        jne      0x64b7
0064B4:  E8 AB FF                     call     0x6462
0064B7:  C3                           ret     
