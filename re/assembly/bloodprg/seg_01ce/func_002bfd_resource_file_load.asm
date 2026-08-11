; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002bfd
; seg_off: 01ce:091d
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_file_load
; label_comment: resource file load: dx=si; call 0x2693 (path build); bp=gs:[0xa8e] (FindFirst file size); cx=bp. Loads a resource file by name - path build then sized read (called 7x, a core loader)
; incoming: call@0x0017ee->01ce:091d
; incoming: call@0x00182c->01ce:091d
; incoming: call@0x0018da->01ce:091d
; incoming: call@0x0019d7->01ce:091d
; incoming: call@0x009d95->01ce:091d
; incoming: call@0x00b43a->01ce:091d
; incoming: call@0x00b68b->01ce:091d
; byte_count: 339
; boundary: cfg_blocks_33_terminals_6
; terminal: jmp 0x2cee:3, jmp 0x2d21:2, retf:1
; direct_callees: 0x002693
; indirect_calls: 0
; routine_bytes_sha256: 3cc1de8ec905520a04c6ef34c7b6724a97a5143ada29276bf0219ec12bc4a055

002BFD:  1E                           push     ds
002BFE:  56                           push     si
002BFF:  06                           push     es
002C00:  57                           push     di
002C01:  52                           push     dx
002C02:  53                           push     bx
002C03:  51                           push     cx
002C04:  66 55                        push     ebp
002C06:  8B D6                        mov      dx, si
002C08:  0E                           push     cs
002C09:  E8 87 FA                     call     0x2693
002C0C:  65 8B 2E 8E 0A               mov      bp, word ptr gs:[0xa8e]
002C11:  8B CD                        mov      cx, bp
002C13:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002C19:  75 23                        jne      0x2c3e
002C1B:  06                           push     es
002C1C:  B8 00 2F                     mov      ax, 0x2f00
002C1F:  CD 21                        int      0x21
002C21:  8B F3                        mov      si, bx
002C23:  83 C6 1A                     add      si, 0x1a
002C26:  33 C9                        xor      cx, cx
002C28:  B8 00 4E                     mov      ax, 0x4e00
002C2B:  CD 21                        int      0x21
002C2D:  26 8B 2C                     mov      bp, word ptr es:[si]
002C30:  07                           pop      es
002C31:  8B CD                        mov      cx, bp
002C33:  B8 00 3D                     mov      ax, 0x3d00
002C36:  CD 21                        int      0x21
002C38:  0F 82 07 01                  jb       0x2d43
002C3C:  8B D8                        mov      bx, ax
002C3E:  8C C0                        mov      ax, es
002C40:  8E D8                        mov      ds, ax
002C42:  8B D7                        mov      dx, di
002C44:  F7 DD                        neg      bp
002C46:  03 D5                        add      dx, bp
002C48:  03 FD                        add      di, bp
002C4A:  B8 00 3F                     mov      ax, 0x3f00
002C4D:  CD 21                        int      0x21
002C4F:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002C55:  75 05                        jne      0x2c5c
002C57:  B8 00 3E                     mov      ax, 0x3e00
002C5A:  CD 21                        int      0x21
002C5C:  66 2E A1 0E 09               mov      eax, dword ptr cs:[0x90e]
002C61:  F2 AE                        repne scasb al, byte ptr es:[di]
002C63:  0B C9                        or       cx, cx
002C65:  0F 84 DA 00                  je       0x2d43
002C69:  66 26 3B 45 FF               cmp      eax, dword ptr es:[di - 1]
002C6E:  75 F1                        jne      0x2c61
002C70:  66 2E A1 13 09               mov      eax, dword ptr cs:[0x913]
002C75:  F2 AE                        repne scasb al, byte ptr es:[di]
002C77:  0B C9                        or       cx, cx
002C79:  0F 84 C6 00                  je       0x2d43
002C7D:  66 26 3B 45 FF               cmp      eax, dword ptr es:[di - 1]
002C82:  75 F1                        jne      0x2c75
002C84:  83 C7 07                     add      di, 7
002C87:  65 F6 06 53 5B 01            test     byte ptr gs:[0x5b53], 1
002C8D:  74 2E                        je       0x2cbd
002C8F:  65 C6 06 55 5B 01            mov      byte ptr gs:[0x5b55], 1
002C95:  51                           push     cx
002C96:  06                           push     es
002C97:  57                           push     di
002C98:  B9 00 03                     mov      cx, 0x300
002C9B:  65 A0 F3 24                  mov      al, byte ptr gs:[0x24f3]
002C9F:  65 0A 06 4F 27               or       al, byte ptr gs:[0x274f]
002CA4:  74 04                        je       0x2caa
002CA6:  81 E9 C0 00                  sub      cx, 0xc0
002CAA:  8B F7                        mov      si, di
002CAC:  8C E8                        mov      ax, gs
002CAE:  8E C0                        mov      es, ax
002CB0:  BF 51 52                     mov      di, 0x5251
002CB3:  AC                           lodsb    al, byte ptr [si]
002CB4:  C0 E8 02                     shr      al, 2
002CB7:  AA                           stosb    byte ptr es:[di], al
002CB8:  E2 F9                        loop     0x2cb3
002CBA:  5F                           pop      di
002CBB:  07                           pop      es
002CBC:  59                           pop      cx
002CBD:  81 C7 00 03                  add      di, 0x300
002CC1:  66 2E A1 18 09               mov      eax, dword ptr cs:[0x918]
002CC6:  F2 AE                        repne scasb al, byte ptr es:[di]
002CC8:  E3 79                        jcxz     0x2d43
002CCA:  66 26 3B 45 FF               cmp      eax, dword ptr es:[di - 1]
002CCF:  75 F5                        jne      0x2cc6
002CD1:  83 C7 07                     add      di, 7
002CD4:  8B F7                        mov      si, di
002CD6:  65 C4 3E 29 52               les      di, ptr gs:[0x5229]
002CDB:  66 33 C0                     xor      eax, eax
002CDE:  8B C8                        mov      cx, ax
002CE0:  66 BD 00 FA 00 00            mov      ebp, 0xfa00
002CE6:  65 F6 06 57 5B 01            test     byte ptr gs:[0x5b57], 1
002CEC:  74 33                        je       0x2d21
002CEE:  0B ED                        or       bp, bp
002CF0:  74 54                        je       0x2d46
002CF2:  AC                           lodsb    al, byte ptr [si]
002CF3:  0A C0                        or       al, al
002CF5:  79 16                        jns      0x2d0d
002CF7:  F6 D8                        neg      al
002CF9:  FE C0                        inc      al
002CFB:  8A C8                        mov      cl, al
002CFD:  66 2B E8                     sub      ebp, eax
002D00:  AC                           lodsb    al, byte ptr [si]
002D01:  0A C0                        or       al, al
002D03:  75 04                        jne      0x2d09
002D05:  03 F9                        add      di, cx
002D07:  EB E5                        jmp      0x2cee
002D09:  F3 AA                        rep stosb byte ptr es:[di], al
002D0B:  EB E1                        jmp      0x2cee
002D0D:  FE C0                        inc      al
002D0F:  8A C8                        mov      cl, al
002D11:  66 2B E8                     sub      ebp, eax
002D14:  AC                           lodsb    al, byte ptr [si]
002D15:  0A C0                        or       al, al
002D17:  74 03                        je       0x2d1c
002D19:  26 88 05                     mov      byte ptr es:[di], al
002D1C:  47                           inc      di
002D1D:  E2 F5                        loop     0x2d14
002D1F:  EB CD                        jmp      0x2cee
002D21:  0B ED                        or       bp, bp
002D23:  74 21                        je       0x2d46
002D25:  AC                           lodsb    al, byte ptr [si]
002D26:  0A C0                        or       al, al
002D28:  79 0E                        jns      0x2d38
002D2A:  F6 D8                        neg      al
002D2C:  FE C0                        inc      al
002D2E:  8A C8                        mov      cl, al
002D30:  66 2B E8                     sub      ebp, eax
002D33:  AC                           lodsb    al, byte ptr [si]
002D34:  F3 AA                        rep stosb byte ptr es:[di], al
002D36:  EB E9                        jmp      0x2d21
002D38:  FE C0                        inc      al
002D3A:  8A C8                        mov      cl, al
002D3C:  66 2B E8                     sub      ebp, eax
002D3F:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
002D41:  EB DE                        jmp      0x2d21
002D43:  B8 FF FF                     mov      ax, 0xffff
002D46:  66 5D                        pop      ebp
002D48:  59                           pop      cx
002D49:  5B                           pop      bx
002D4A:  5A                           pop      dx
002D4B:  5F                           pop      di
002D4C:  07                           pop      es
002D4D:  5E                           pop      si
002D4E:  1F                           pop      ds
002D4F:  CB                           retf    
