; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064a0
; seg_off: 04da:1100
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_d0_cond_branch
; label_comment: VM opcode 0xD0: if gs:[0x252a]&1 is CLEAR, call vm_branch 0x6462 (conditional jump on game-flag [0x252a] bit0)
; incoming: vm_opcode_handlers:opcode_0xd0
; byte_count: 12
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 2ac1b2e1c7bf0ea1e6bc90f37ae5920048dcb7fc3b67b87b5f0ca33296776c5c

0064A0:  65 F6 06 2A 25 01            test     byte ptr gs:[0x252a], 1
0064A6:  75 03                        jne      0x64ab
0064A8:  E8 B7 FF                     call     0x6462
0064AB:  C3                           ret     
