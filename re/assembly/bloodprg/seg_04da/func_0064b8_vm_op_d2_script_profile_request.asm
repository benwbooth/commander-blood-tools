; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0064b8
; seg_off: 04da:1118
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_d2_script_profile_request
; label_comment: 0xD2 handler: DS:0x6780 = sign_extend(operand byte) - 1
; incoming: vm_opcode_handlers:opcode_0xd2
; byte_count: 8
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0064b8_vm_op_d2_script_profile_request.cpp
; routine_bytes_sha256: 98ab9918a5bcad942ff5f60cf107c5d65d21b16c2d8e8771cf6847f568fb13f9

0064B8:  AC                           lodsb    al, byte ptr [si]
0064B9:  98                           cwde    
0064BA:  48                           dec      ax
0064BB:  65 A3 80 67                  mov      word ptr gs:[0x6780], ax
0064BF:  C3                           ret     
