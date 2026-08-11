; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006b4c
; seg_off: 04da:17ac
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_c1_record_state
; label_comment: 0xC1 line-record state handler; raw token C1 record operand
; incoming: vm_opcode_handlers:opcode_0xc1
; byte_count: 306
; boundary: cfg_blocks_34_terminals_7
; terminal: jmp 0x6c1c:2, jmp 0x6c7a:1, jmp 0x6c7c:3, ret:1
; direct_callees: 0x006023, 0x006034, 0x0060dd, 0x006210, 0x00624b, 0x006462
; indirect_calls: 0
; routine_bytes_sha256: 47d60246bcb856e4fb2483f3358c78183bec65e07d1d14f9f2bb35142d521651

006B4C:  57                           push     di
006B4D:  65 C4 3E 24 67               les      di, ptr gs:[0x6724]
006B52:  32 D2                        xor      dl, dl
006B54:  8A 04                        mov      al, byte ptr [si]
006B56:  3C A1                        cmp      al, 0xa1
006B58:  75 03                        jne      0x6b5d
006B5A:  FE C2                        inc      dl
006B5C:  46                           inc      si
006B5D:  AD                           lodsw    ax, word ptr [si]
006B5E:  8B E8                        mov      bp, ax
006B60:  E8 D1 F4                     call     0x6034
006B63:  8B F8                        mov      di, ax
006B65:  26 8B 05                     mov      ax, word ptr es:[di]
006B68:  26 8B 4E 00                  mov      cx, word ptr es:[bp]
006B6C:  AD                           lodsw    ax, word ptr [si]
006B6D:  65 A3 36 67                  mov      word ptr gs:[0x6736], ax
006B71:  1E                           push     ds
006B72:  56                           push     si
006B73:  65 F6 06 AD 67 01            test     byte ptr gs:[0x67ad], 1
006B79:  74 53                        je       0x6bce
006B7B:  83 F8 02                     cmp      ax, 2
006B7E:  74 05                        je       0x6b85
006B80:  83 F8 01                     cmp      ax, 1
006B83:  75 27                        jne      0x6bac
006B85:  81 F9 C1 00                  cmp      cx, 0xc1
006B89:  74 21                        je       0x6bac
006B8B:  8B D8                        mov      bx, ax
006B8D:  B8 11 00                     mov      ax, 0x11
006B90:  E8 90 F4                     call     0x6023
006B93:  8B D8                        mov      bx, ax
006B95:  26 8B 29                     mov      bp, word ptr es:[bx + di]
006B98:  26 8B 5E 00                  mov      bx, word ptr es:[bp]
006B9C:  B8 13 00                     mov      ax, 0x13
006B9F:  E8 81 F4                     call     0x6023
006BA2:  0B C0                        or       ax, ax
006BA4:  74 1F                        je       0x6bc5
006BA6:  03 E8                        add      bp, ax
006BA8:  26 8B 4E 00                  mov      cx, word ptr es:[bp]
006BAC:  65 A1 36 67                  mov      ax, word ptr gs:[0x6736]
006BB0:  81 F9 C1 00                  cmp      cx, 0xc1
006BB4:  75 0F                        jne      0x6bc5
006BB6:  26 3B 46 02                  cmp      ax, word ptr es:[bp + 2]
006BBA:  75 09                        jne      0x6bc5
006BBC:  0A D2                        or       dl, dl
006BBE:  0F 85 B1 00                  jne      0x6c73
006BC2:  E9 B7 00                     jmp      0x6c7c
006BC5:  0A D2                        or       dl, dl
006BC7:  0F 84 A8 00                  je       0x6c73
006BCB:  E9 AE 00                     jmp      0x6c7c
006BCE:  26 F6 45 02 01               test     byte ptr es:[di + 2], 1
006BD3:  0F 84 9C 00                  je       0x6c73
006BD7:  8C C3                        mov      bx, es
006BD9:  8E DB                        mov      ds, bx
006BDB:  65 8B 36 36 67               mov      si, word ptr gs:[0x6736]
006BE0:  83 F8 02                     cmp      ax, 2
006BE3:  74 05                        je       0x6bea
006BE5:  83 F8 01                     cmp      ax, 1
006BE8:  75 1A                        jne      0x6c04
006BEA:  E8 F0 F4                     call     0x60dd
006BED:  0B C0                        or       ax, ax
006BEF:  74 13                        je       0x6c04
006BF1:  8B 1D                        mov      bx, word ptr [di]
006BF3:  B8 11 00                     mov      ax, 0x11
006BF6:  E8 2A F4                     call     0x6023
006BF9:  8B D8                        mov      bx, ax
006BFB:  8B 39                        mov      di, word ptr [bx + di]
006BFD:  8B 05                        mov      ax, word ptr [di]
006BFF:  83 F8 10                     cmp      ax, 0x10
006C02:  75 6F                        jne      0x6c73
006C04:  26 8B 05                     mov      ax, word ptr es:[di]
006C07:  83 F8 10                     cmp      ax, 0x10
006C0A:  75 49                        jne      0x6c55
006C0C:  55                           push     bp
006C0D:  BD 86 68                     mov      bp, 0x6886
006C10:  0E                           push     cs
006C11:  E8 37 F6                     call     0x624b
006C14:  5D                           pop      bp
006C15:  8C E8                        mov      ax, gs
006C17:  8E D8                        mov      ds, ax
006C19:  BE 86 68                     mov      si, 0x6886
006C1C:  AD                           lodsw    ax, word ptr [si]
006C1D:  83 F8 FF                     cmp      ax, -1
006C20:  74 5A                        je       0x6c7c
006C22:  8B D8                        mov      bx, ax
006C24:  26 8B 07                     mov      ax, word ptr es:[bx]
006C27:  83 F8 02                     cmp      ax, 2
006C2A:  75 0A                        jne      0x6c36
006C2C:  A1 36 67                     mov      ax, word ptr [0x6736]
006C2F:  E8 DE F5                     call     0x6210
006C32:  72 14                        jb       0x6c48
006C34:  EB E6                        jmp      0x6c1c
006C36:  83 F8 01                     cmp      ax, 1
006C39:  75 E1                        jne      0x6c1c
006C3B:  8B 1E 36 67                  mov      bx, word ptr [0x6736]
006C3F:  26 F6 47 02 02               test     byte ptr es:[bx + 2], 2
006C44:  75 02                        jne      0x6c48
006C46:  EB D4                        jmp      0x6c1c
006C48:  B8 13 00                     mov      ax, 0x13
006C4B:  BB 10 00                     mov      bx, 0x10
006C4E:  E8 D2 F3                     call     0x6023
006C51:  03 C7                        add      ax, di
006C53:  8B E8                        mov      bp, ax
006C55:  26 8B 4E 00                  mov      cx, word ptr es:[bp]
006C59:  0B C9                        or       cx, cx
006C5B:  75 16                        jne      0x6c73
006C5D:  26 C7 46 00 C1 00            mov      word ptr es:[bp], 0xc1
006C63:  65 A1 36 67                  mov      ax, word ptr gs:[0x6736]
006C67:  26 89 46 02                  mov      word ptr es:[bp + 2], ax
006C6B:  26 C7 46 04 02 00            mov      word ptr es:[bp + 4], 2
006C71:  EB 07                        jmp      0x6c7a
006C73:  5E                           pop      si
006C74:  1F                           pop      ds
006C75:  E8 EA F7                     call     0x6462
006C78:  EB 02                        jmp      0x6c7c
006C7A:  5E                           pop      si
006C7B:  1F                           pop      ds
006C7C:  5F                           pop      di
006C7D:  C3                           ret     
