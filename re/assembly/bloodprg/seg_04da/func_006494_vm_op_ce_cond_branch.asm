; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006494
; seg_off: 04da:10f4
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_ce_cond_branch
; label_comment: VM opcode 0xCE: if gs:[0x2793]&1 is CLEAR, call vm_branch 0x6462 (conditional jump on presentation-flag [0x2793] bit0)
; incoming: vm_opcode_handlers:opcode_0xce
; byte_count: 12
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 5f519a19f7c2bc74f446ba94134050199f080500b64edba183e3f3fd671995bf

006494:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
00649A:  75 03                        jne      0x649f
00649C:  E8 C3 FF                     call     0x6462
00649F:  C3                           ret     
