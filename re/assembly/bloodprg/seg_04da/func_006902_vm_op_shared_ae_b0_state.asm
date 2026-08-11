; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006902
; seg_off: 04da:1562
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_shared_ae_b0_state
; label_comment: VM opcodes 0xAE + 0xB0 (shared handler 0x6902): object/line-record state op (les di,gs:0x6724 + 0xA1-skip). State-table family; exact op partial
; incoming: vm_opcode_handlers:opcode_0xae
; incoming: vm_opcode_handlers:opcode_0xb0
; byte_count: 68
; boundary: cfg_blocks_12_terminals_4
; terminal: jmp 0x6944:3, ret:1
; direct_callees: 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 982bc049edca13d8a8c5d278e509b87447e1dc567cd991fc6c7f9cf8bb54e0e6

006902:  57                           push     di
006903:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006908:  32 D2                        xor      dl, dl
00690A:  8A 04                        mov      al, byte ptr [si]
00690C:  3C A1                        cmp      al, 0xa1
00690E:  75 03                        jne      0x6913
006910:  46                           inc      si
006911:  FE C2                        inc      dl
006913:  AD                           lodsw    ax, word ptr [si]
006914:  8B D8                        mov      bx, ax
006916:  AD                           lodsw    ax, word ptr [si]
006917:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
00691D:  74 17                        je       0x6936
00691F:  26 23 01                     and      ax, word ptr es:[bx + di]
006922:  74 09                        je       0x692d
006924:  0A D2                        or       dl, dl
006926:  74 1C                        je       0x6944
006928:  E8 37 FB                     call     0x6462
00692B:  EB 17                        jmp      0x6944
00692D:  0A D2                        or       dl, dl
00692F:  75 13                        jne      0x6944
006931:  E8 2E FB                     call     0x6462
006934:  EB 0E                        jmp      0x6944
006936:  0A D2                        or       dl, dl
006938:  75 05                        jne      0x693f
00693A:  26 09 01                     or       word ptr es:[bx + di], ax
00693D:  EB 05                        jmp      0x6944
00693F:  F7 D0                        not      ax
006941:  26 21 01                     and      word ptr es:[bx + di], ax
006944:  5F                           pop      di
006945:  C3                           ret     
