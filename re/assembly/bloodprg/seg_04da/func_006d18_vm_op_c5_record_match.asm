; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006d18
; seg_off: 04da:1978
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c5_record_match
; label_comment: VM opcode 0xC5 FULL: les di,gs:0x6724; lodsw bp=record ptr, lodsw operand; gated on gs:[0x67ad]&1: cmp operand vs es:[bp+2] (record id/value field) AND cmp es:[bp] vs 0xC5 (record TYPE tag = the opcode value). So state-table records are TYPED - field +0 = the opcode/type that created them, +2 = id/value. Each C-range opcode (C5/C6/C7/C8) matches records of ITS OWN type (compares +0 to its own opcode). Record-match/conditional op || ALSO RECORDED as `vm_op_c5_state_record`: VM opcode 0xC5: object/line-record state op (les di,gs:0x6724 + 0xA1-skip prologue + lodsw operand). Part of the C-range state-table family; exact field operation NOT yet fully decoded (only the shared prologue confirmed) || ALSO RECORDED as `vm_op_c5_record_entry`: 0xC5 record-entry handler; writes es:[record]={0xc5,related,0} when related type is 0x0200 || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc5
; byte_count: 104
; boundary: cfg_blocks_15_terminals_4
; terminal: jmp 0x6d7e:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: dc40faa626b2b323cb35de3f0e0251c3f5ebb5cc6e98895fc8c292988e175f19

006D18:  57                           push     di
006D19:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006D1E:  32 D2                        xor      dl, dl
006D20:  8A 04                        mov      al, byte ptr [si]
006D22:  3C A1                        cmp      al, 0xa1
006D24:  75 03                        jne      0x6d29
006D26:  FE C2                        inc      dl
006D28:  46                           inc      si
006D29:  AD                           lodsw    ax, word ptr [si]
006D2A:  8B E8                        mov      bp, ax
006D2C:  AD                           lodsw    ax, word ptr [si]
006D2D:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006D33:  74 1B                        je       0x6d50
006D35:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006D39:  75 0F                        jne      0x6d4a
006D3B:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006D3F:  3D C5 00                     cmp      ax, 0xc5
006D42:  75 06                        jne      0x6d4a
006D44:  0A D2                        or       dl, dl
006D46:  75 33                        jne      0x6d7b
006D48:  EB 34                        jmp      0x6d7e
006D4A:  0A D2                        or       dl, dl
006D4C:  74 2D                        je       0x6d7b
006D4E:  EB 2E                        jmp      0x6d7e
006D50:  8B D8                        mov      bx, ax
006D52:  26 F6 47 02 01               test     byte ptr es:[bx + 2], 1
006D57:  74 22                        je       0x6d7b
006D59:  26 8B 07                     mov      ax, word ptr es:[bx]
006D5C:  3D 00 02                     cmp      ax, 0x200
006D5F:  75 1A                        jne      0x6d7b
006D61:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006D65:  0B C0                        or       ax, ax
006D67:  75 12                        jne      0x6d7b
006D69:  26 C7 46 00 C5 00            mov      word ptr es:[bp], 0xc5
006D6F:  26 89 5E 02                  mov      word ptr es:[bp + 2], bx
006D73:  26 C7 46 04 00 00            mov      word ptr es:[bp + 4], 0
006D79:  EB 03                        jmp      0x6d7e
006D7B:  E8 E4 F6                     call     0x6462
006D7E:  5F                           pop      di
006D7F:  C3                           ret     
