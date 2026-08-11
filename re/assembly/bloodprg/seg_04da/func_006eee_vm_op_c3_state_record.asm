; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006eee
; seg_off: 04da:1b4e
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c3_state_record
; label_comment: VM opcode 0xC3: object/line-record state op (les di,gs:0x6724 + 0xA1-skip prologue). C-range state-table family; exact field op partial || ALSO RECORDED as `vm_op_c3_record_link`: 0xC3 record-link handler; writes es:[record]={0xc3,related,1} on mode-0 success || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xc3
; byte_count: 116
; boundary: cfg_blocks_16_terminals_4
; terminal: jmp 0x6f60:3, ret:1
; direct_callees: 0x006034, 0x006462
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_006eee_vm_op_c3_state_record.cpp
; routine_bytes_sha256: d9d716778c81420b283b3f0af6d3a4a658d6034b721fd914945c5dfb7431f9e6

006EEE:  57                           push     di
006EEF:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006EF4:  32 D2                        xor      dl, dl
006EF6:  8A 04                        mov      al, byte ptr [si]
006EF8:  3C A1                        cmp      al, 0xa1
006EFA:  75 03                        jne      0x6eff
006EFC:  FE C2                        inc      dl
006EFE:  46                           inc      si
006EFF:  AD                           lodsw    ax, word ptr [si]
006F00:  8B E8                        mov      bp, ax
006F02:  E8 2F F1                     call     0x6034
006F05:  8B F8                        mov      di, ax
006F07:  AD                           lodsw    ax, word ptr [si]
006F08:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006F0E:  74 22                        je       0x6f32
006F10:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006F15:  74 15                        je       0x6f2c
006F17:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006F1B:  75 0F                        jne      0x6f2c
006F1D:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006F21:  3D C3 00                     cmp      ax, 0xc3
006F24:  75 06                        jne      0x6f2c
006F26:  0A D2                        or       dl, dl
006F28:  75 33                        jne      0x6f5d
006F2A:  EB 34                        jmp      0x6f60
006F2C:  0A D2                        or       dl, dl
006F2E:  74 2D                        je       0x6f5d
006F30:  EB 2E                        jmp      0x6f60
006F32:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006F37:  74 24                        je       0x6f5d
006F39:  8B D8                        mov      bx, ax
006F3B:  26 F6 47 02 01               test     byte ptr es:[bx + 2], 1
006F40:  74 1B                        je       0x6f5d
006F42:  26 8B 46 00                  mov      ax, word ptr es:[bp]
006F46:  3D C4 00                     cmp      ax, 0xc4
006F49:  74 12                        je       0x6f5d
006F4B:  26 C7 46 00 C3 00            mov      word ptr es:[bp], 0xc3
006F51:  26 89 5E 02                  mov      word ptr es:[bp + 2], bx
006F55:  26 C7 46 04 01 00            mov      word ptr es:[bp + 4], 1
006F5B:  EB 03                        jmp      0x6f60
006F5D:  E8 02 F5                     call     0x6462
006F60:  5F                           pop      di
006F61:  C3                           ret     
