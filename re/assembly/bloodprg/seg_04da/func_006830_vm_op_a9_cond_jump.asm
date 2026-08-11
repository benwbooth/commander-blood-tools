; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006830
; seg_off: 04da:1490
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a9_cond_jump
; label_comment: VM opcode 0xA9: lodsb al; if bit0 CLEAR, si=[si] (jump to operand). Conditional jump on operand bit0
; incoming: vm_opcode_handlers:opcode_0xa9
; byte_count: 28
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x684b:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 8d52c7cd9cdbe18a1703851da0ef43dd199a0145678698cf5e695eb31ab75ddf

006830:  AC                           lodsb    al, byte ptr [si]
006831:  A8 01                        test     al, 1
006833:  75 04                        jne      0x6839
006835:  8B 34                        mov      si, word ptr [si]
006837:  EB 12                        jmp      0x684b
006839:  65 C6 06 AD 67 01            mov      byte ptr gs:[0x67ad], 1
00683F:  AD                           lodsw    ax, word ptr [si]
006840:  65 A3 20 68                  mov      word ptr gs:[0x6820], ax
006844:  65 C7 06 84 68 02 00         mov      word ptr gs:[0x6884], 2
00684B:  C3                           ret     
