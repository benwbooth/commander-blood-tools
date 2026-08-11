; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0069c7
; seg_off: 04da:1627
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_cd_state_gated
; label_comment: VM opcode 0xCD: state-table record op (les di,gs:0x6724, gated on gs:[0x67ad]&1, 0xA1-skip). State-table family; exact op partial || ALSO RECORDED as `vm_op_cd_record_triple`: 0xCD record-triple handler; consumes record/first/second words, optional A1 inverted compare || MERGED 2026-07-25 (#185): one handler under several names.
; incoming: vm_opcode_handlers:opcode_0xcd
; byte_count: 224
; boundary: cfg_blocks_20_terminals_3
; terminal: jmp 0x6aa4:2, ret:1
; direct_callees: 0x005fd8, 0x005ff6, 0x006023, 0x006034, 0x006462, 0x007409
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_0069c7_vm_op_cd_state_gated.cpp
; routine_bytes_sha256: fa54bc475ffdf8c8871c886f3bc527253ef8f87b83fd1459c9cebbd16c866e51

0069C7:  06                           push     es
0069C8:  57                           push     di
0069C9:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
0069CE:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
0069D4:  74 3E                        je       0x6a14
0069D6:  32 D2                        xor      dl, dl
0069D8:  8A 34                        mov      dh, byte ptr [si]
0069DA:  80 FE A1                     cmp      dh, 0xa1
0069DD:  75 03                        jne      0x69e2
0069DF:  FE C2                        inc      dl
0069E1:  46                           inc      si
0069E2:  AD                           lodsw    ax, word ptr [si]
0069E3:  8B D8                        mov      bx, ax
0069E5:  AD                           lodsw    ax, word ptr [si]
0069E6:  8B E8                        mov      bp, ax
0069E8:  AD                           lodsw    ax, word ptr [si]
0069E9:  26 81 3F CD 00               cmp      word ptr es:[bx], 0xcd
0069EE:  75 18                        jne      0x6a08
0069F0:  26 3B 6F 02                  cmp      bp, word ptr es:[bx + 2]
0069F4:  75 12                        jne      0x6a08
0069F6:  26 3B 47 04                  cmp      ax, word ptr es:[bx + 4]
0069FA:  75 0C                        jne      0x6a08
0069FC:  0A D2                        or       dl, dl
0069FE:  0F 84 A2 00                  je       0x6aa4
006A02:  E8 5D FA                     call     0x6462
006A05:  E9 9C 00                     jmp      0x6aa4
006A08:  0A D2                        or       dl, dl
006A0A:  0F 85 96 00                  jne      0x6aa4
006A0E:  E8 51 FA                     call     0x6462
006A11:  E9 90 00                     jmp      0x6aa4
006A14:  AD                           lodsw    ax, word ptr [si]
006A15:  E8 1C F6                     call     0x6034
006A18:  8B F8                        mov      di, ax
006A1A:  AD                           lodsw    ax, word ptr [si]
006A1B:  8B D0                        mov      dx, ax
006A1D:  AD                           lodsw    ax, word ptr [si]
006A1E:  8B E8                        mov      bp, ax
006A20:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006A25:  26 F6 46 02 01               test     byte ptr es:[bp + 2], 1
006A2A:  8B EA                        mov      bp, dx
006A2C:  26 F6 46 02 01               test     byte ptr es:[bp + 2], 1
006A31:  26 8B 5E 00                  mov      bx, word ptr es:[bp]
006A35:  50                           push     ax
006A36:  B8 11 00                     mov      ax, 0x11
006A39:  E8 E7 F5                     call     0x6023
006A3C:  03 E8                        add      bp, ax
006A3E:  59                           pop      cx
006A3F:  65 3B 3E 4E 67               cmp      di, word ptr gs:[0x674e]
006A44:  75 05                        jne      0x6a4b
006A46:  8B C2                        mov      ax, dx
006A48:  E8 8D F5                     call     0x5fd8
006A4B:  8B FA                        mov      di, dx
006A4D:  26 8B 1D                     mov      bx, word ptr es:[di]
006A50:  B8 11 00                     mov      ax, 0x11
006A53:  E8 CD F5                     call     0x6023
006A56:  66 98                        cwde    
006A58:  65 3B 0E 4E 67               cmp      cx, word ptr gs:[0x674e]
006A5D:  75 0C                        jne      0x6a6b
006A5F:  50                           push     ax
006A60:  B9 FF FF                     mov      cx, 0xffff
006A63:  8B C7                        mov      ax, di
006A65:  E8 8E F5                     call     0x5ff6
006A68:  58                           pop      ax
006A69:  73 39                        jae      0x6aa4
006A6B:  67 26 89 0C 38               mov      word ptr es:[eax + edi], cx
006A70:  65 F6 06 93 27 01            test     byte ptr gs:[0x2793], 1
006A76:  75 2C                        jne      0x6aa4
006A78:  65 F6 06 AA 67 02            test     byte ptr gs:[0x67aa], 2
006A7E:  75 24                        jne      0x6aa4
006A80:  81 FB 00 04                  cmp      bx, 0x400
006A84:  75 1E                        jne      0x6aa4
006A86:  83 C7 04                     add      di, 4
006A89:  0E                           push     cs
006A8A:  E8 7C 09                     call     0x7409
006A8D:  0B C0                        or       ax, ax
006A8F:  74 13                        je       0x6aa4
006A91:  65 C6 06 B2 1F 00            mov      byte ptr gs:[0x1fb2], 0
006A97:  65 80 0E AA 67 02            or       byte ptr gs:[0x67aa], 2
006A9D:  65 C7 06 88 67 2B 00         mov      word ptr gs:[0x6788], 0x2b
006AA4:  5F                           pop      di
006AA5:  07                           pop      es
006AA6:  C3                           ret     
