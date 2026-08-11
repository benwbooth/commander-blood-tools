; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006d80
; seg_off: 04da:19e0
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c6_record_match
; label_comment: VM opcode 0xC6 FULL (confirmed same structure as C5): les di,gs:0x6724; lodsw bp record, lodsw operand; gated on [0x67ad]; cmp operand vs es:[bp+2] (id) AND cmp es:[bp] vs 0xC6 (self type-tag). Matches state-table records of type 0xC6. VERIFIES the C-range typed-record pattern (C5->0xc5, C6->0xc6, C8->0xc8) || ALSO RECORDED as `vm_op_c6_state_record`: VM opcode 0xC6: object/line-record state op (same state-table + 0xA1-skip prologue as C5). C-range family member; exact operation not yet fully decoded || ALSO RECORDED as `vm_op_c6_record_entry`: 0xC6 record-entry handler; writes es:[record]={0xc6,operand,0} || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc6
; byte_count: 79
; boundary: cfg_blocks_12_terminals_4
; terminal: jmp 0x6dcd:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 437c508f2f7434812389fc91b24548bbcb76c6ba75d51948597d7a7c41f4d230

006D80:  57                           push     di
006D81:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006D86:  32 D2                        xor      dl, dl
006D88:  8A 04                        mov      al, byte ptr [si]
006D8A:  3C A1                        cmp      al, 0xa1
006D8C:  75 03                        jne      0x6d91
006D8E:  FE C2                        inc      dl
006D90:  46                           inc      si
006D91:  AD                           lodsw    ax, word ptr [si]
006D92:  8B E8                        mov      bp, ax
006D94:  AD                           lodsw    ax, word ptr [si]
006D95:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006D9B:  74 1B                        je       0x6db8
006D9D:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006DA1:  75 0F                        jne      0x6db2
006DA3:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006DA7:  3D C6 00                     cmp      ax, 0xc6
006DAA:  75 06                        jne      0x6db2
006DAC:  0A D2                        or       dl, dl
006DAE:  75 1A                        jne      0x6dca
006DB0:  EB 1B                        jmp      0x6dcd
006DB2:  0A D2                        or       dl, dl
006DB4:  74 14                        je       0x6dca
006DB6:  EB 15                        jmp      0x6dcd
006DB8:  26 C7 46 00 C6 00            mov      word ptr es:[bp], 0xc6
006DBE:  26 89 46 02                  mov      word ptr es:[bp + 2], ax
006DC2:  26 C7 46 04 00 00            mov      word ptr es:[bp + 4], 0
006DC8:  EB 03                        jmp      0x6dcd
006DCA:  E8 95 F6                     call     0x6462
006DCD:  5F                           pop      di
006DCE:  C3                           ret     
