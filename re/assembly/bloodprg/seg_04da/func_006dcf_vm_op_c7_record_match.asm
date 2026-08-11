; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006dcf
; seg_off: 04da:1a2f
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c7_record_match
; label_comment: VM opcode 0xC7: matches state-table records tagged 0xC7 (cmp es:[bp] vs 0xc7). Confirms the typed-record pattern across C5/C6/C7/C8 - all four verified || ALSO RECORDED as `vm_op_c7_state_record`: VM opcode 0xC7: object/line-record state op (state-table + 0xA1-skip prologue). C-range family; exact field op partial || ALSO RECORDED as `vm_op_c7_record_entry`: 0xC7 record-entry handler; writes es:[record]={0xc7,related,0} when related record is active || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc7
; byte_count: 101
; boundary: cfg_blocks_15_terminals_4
; terminal: jmp 0x6e32:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: f8879e57ae0f5b6dac45dc28ba181e576dff46f205e509aadcd31c0452540ea8

006DCF:  57                           push     di
006DD0:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006DD5:  32 D2                        xor      dl, dl
006DD7:  8A 04                        mov      al, byte ptr [si]
006DD9:  3C A1                        cmp      al, 0xa1
006DDB:  75 03                        jne      0x6de0
006DDD:  FE C2                        inc      dl
006DDF:  46                           inc      si
006DE0:  AD                           lodsw    ax, word ptr [si]
006DE1:  8B E8                        mov      bp, ax
006DE3:  AD                           lodsw    ax, word ptr [si]
006DE4:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006DEA:  74 1B                        je       0x6e07
006DEC:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006DF0:  75 0F                        jne      0x6e01
006DF2:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006DF6:  3D C7 00                     cmp      ax, 0xc7
006DF9:  75 06                        jne      0x6e01
006DFB:  0A D2                        or       dl, dl
006DFD:  75 30                        jne      0x6e2f
006DFF:  EB 31                        jmp      0x6e32
006E01:  0A D2                        or       dl, dl
006E03:  74 2A                        je       0x6e2f
006E05:  EB 2B                        jmp      0x6e32
006E07:  8B D8                        mov      bx, ax
006E09:  26 F6 47 02 01               test     byte ptr es:[bx + 2], 1
006E0E:  74 1F                        je       0x6e2f
006E10:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006E14:  3D C4 00                     cmp      ax, 0xc4
006E17:  74 04                        je       0x6e1d
006E19:  0B C0                        or       ax, ax
006E1B:  75 12                        jne      0x6e2f
006E1D:  26 C7 46 00 C7 00            mov      word ptr es:[bp], 0xc7
006E23:  26 89 5E 02                  mov      word ptr es:[bp + 2], bx
006E27:  26 C7 46 04 00 00            mov      word ptr es:[bp + 4], 0
006E2D:  EB 03                        jmp      0x6e32
006E2F:  E8 30 F6                     call     0x6462
006E32:  5F                           pop      di
006E33:  C3                           ret     
