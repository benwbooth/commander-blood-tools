; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00c005
; seg_off: 0b1b:0855
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: snd_bank_loader
; label_comment: SND bank loader/extractor at 0x0B1B:0x0855; AX=0 builds in-memory table, AX!=0 preserves table and can write son.snd
; incoming: call@0x000fe7->0b1b:0855
; incoming: call@0x007667->0b1b:0855
; incoming: call@0x008263->0b1b:0855
; incoming: call@0x0087ab->0b1b:0855
; incoming: call@0x008866->0b1b:0855
; incoming: call@0x00b5dc->0b1b:0855
; incoming: call@0x00b610->0b1b:0855
; byte_count: 481
; boundary: cfg_blocks_29_terminals_4
; terminal: jmp 0xc12d:1, jmp 0xc1cf:2, retf:1
; direct_callees: none
; indirect_calls: 4
; routine_bytes_sha256: c792ab3ce5c558301f850256089f963a2738a41bad701b7cc866a4f1629ee662

00C005:  50                           push     ax
00C006:  1E                           push     ds
00C007:  56                           push     si
00C008:  06                           push     es
00C009:  57                           push     di
00C00A:  53                           push     bx
00C00B:  66 55                        push     ebp
00C00D:  52                           push     dx
00C00E:  51                           push     cx
00C00F:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
00C015:  0F 84 C2 01                  je       0xc1db
00C019:  8C EB                        mov      bx, gs
00C01B:  8E DB                        mov      ds, bx
00C01D:  50                           push     ax
00C01E:  8B D6                        mov      dx, si
00C020:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
00C025:  F6 06 E2 0A 01               test     byte ptr [0xae2], 1
00C02A:  75 14                        jne      0xc040
00C02C:  9A EA 05 CE 01               lcall    0x1ce, 0x5ea
00C031:  66 89 2E 92 0A               mov      dword ptr [0xa92], ebp
00C036:  66 33 ED                     xor      ebp, ebp
00C039:  B8 00 3D                     mov      ax, 0x3d00
00C03C:  CD 21                        int      0x21
00C03E:  8B D8                        mov      bx, ax
00C040:  BD BF 0B                     mov      bp, 0xbbf
00C043:  BA BB 0B                     mov      dx, 0xbbb
00C046:  66 B9 04 00 00 00            mov      ecx, 4
00C04C:  66 29 0E 92 0A               sub      dword ptr [0xa92], ecx
00C051:  B4 3F                        mov      ah, 0x3f
00C053:  CD 21                        int      0x21
00C055:  8B 0E BB 0B                  mov      cx, word ptr [0xbbb]
00C059:  41                           inc      cx
00C05A:  C1 E1 02                     shl      cx, 2
00C05D:  66 29 0E 92 0A               sub      dword ptr [0xa92], ecx
00C062:  BA 1A 0F                     mov      dx, 0xf1a
00C065:  B4 3F                        mov      ah, 0x3f
00C067:  CD 21                        int      0x21
00C069:  58                           pop      ax
00C06A:  0B C0                        or       ax, ax
00C06C:  75 32                        jne      0xc0a0
00C06E:  65 8B 0E BB 0B               mov      cx, word ptr gs:[0xbbb]
00C073:  8B F2                        mov      si, dx
00C075:  66 AD                        lodsd    eax, dword ptr [si]
00C077:  89 46 00                     mov      word ptr [bp], ax
00C07A:  66 8B 14                     mov      edx, dword ptr [si]
00C07D:  66 2B D0                     sub      edx, eax
00C080:  4A                           dec      dx
00C081:  89 56 02                     mov      word ptr [bp + 2], dx
00C084:  83 C5 04                     add      bp, 4
00C087:  E2 EC                        loop     0xc075
00C089:  66 33 D2                     xor      edx, edx
00C08C:  66 8B C2                     mov      eax, edx
00C08F:  65 8B 0E 92 0A               mov      cx, word ptr gs:[0xa92]
00C094:  B4 3F                        mov      ah, 0x3f
00C096:  65 C5 16 B3 0B               lds      dx, ptr gs:[0xbb3]
00C09B:  CD 21                        int      0x21
00C09D:  E9 2F 01                     jmp      0xc1cf
00C0A0:  66 65 8B 2E 92 0A            mov      ebp, dword ptr gs:[0xa92]
00C0A6:  8C E8                        mov      ax, gs
00C0A8:  8E C0                        mov      es, ax
00C0AA:  BF 57 0C                     mov      di, 0xc57
00C0AD:  65 8B 0E BB 0B               mov      cx, word ptr gs:[0xbbb]
00C0B2:  65 89 0E 53 0C               mov      word ptr gs:[0xc53], cx
00C0B7:  41                           inc      cx
00C0B8:  8B F2                        mov      si, dx
00C0BA:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00C0BD:  65 83 3E 5C 0A FF            cmp      word ptr gs:[0xa5c], -1
00C0C3:  74 4A                        je       0xc10f
00C0C5:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00C0CA:  65 C7 06 4E 0A 00 00         mov      word ptr gs:[0xa4e], 0
00C0D1:  53                           push     bx
00C0D2:  65 8B 1E 4E 0A               mov      bx, word ptr gs:[0xa4e]
00C0D7:  32 C0                        xor      al, al
00C0D9:  B9 02 00                     mov      cx, 2
00C0DC:  65 8B 16 5C 0A               mov      dx, word ptr gs:[0xa5c]
00C0E1:  B4 44                        mov      ah, 0x44
00C0E3:  CD 67                        int      0x67
00C0E5:  43                           inc      bx
00C0E6:  FE C0                        inc      al
00C0E8:  E2 F7                        loop     0xc0e1
00C0EA:  65 89 1E 4E 0A               mov      word ptr gs:[0xa4e], bx
00C0EF:  5B                           pop      bx
00C0F0:  33 D2                        xor      dx, dx
00C0F2:  B9 00 80                     mov      cx, 0x8000
00C0F5:  66 8B C5                     mov      eax, ebp
00C0F8:  66 2B C1                     sub      eax, ecx
00C0FB:  79 02                        jns      0xc0ff
00C0FD:  03 C8                        add      cx, ax
00C0FF:  B4 3F                        mov      ah, 0x3f
00C101:  CD 21                        int      0x21
00C103:  66 0F B7 C0                  movzx    eax, ax
00C107:  66 2B E8                     sub      ebp, eax
00C10A:  75 C5                        jne      0xc0d1
00C10C:  E9 C0 00                     jmp      0xc1cf
00C10F:  65 C5 16 BC 0A               lds      dx, ptr gs:[0xabc]
00C114:  81 C2 00 7D                  add      dx, 0x7d00
00C118:  65 83 3E 5A 0A FF            cmp      word ptr gs:[0xa5a], -1
00C11E:  74 61                        je       0xc181
00C120:  BE 6C 0A                     mov      si, 0xa6c
00C123:  66 65 C7 06 4E 0A 00 00 00 00 mov      dword ptr gs:[0xa4e], 0
00C12D:  B9 00 7D                     mov      cx, 0x7d00
00C130:  66 8B C5                     mov      eax, ebp
00C133:  66 2B C1                     sub      eax, ecx
00C136:  79 02                        jns      0xc13a
00C138:  03 C8                        add      cx, ax
00C13A:  B4 3F                        mov      ah, 0x3f
00C13C:  CD 21                        int      0x21
00C13E:  50                           push     ax
00C13F:  66 0F B7 C0                  movzx    eax, ax
00C143:  1E                           push     ds
00C144:  53                           push     bx
00C145:  A8 01                        test     al, 1
00C147:  74 02                        je       0xc14b
00C149:  66 40                        inc      eax
00C14B:  8B FE                        mov      di, si
00C14D:  66 AB                        stosd    dword ptr es:[di], eax
00C14F:  33 C0                        xor      ax, ax
00C151:  AB                           stosw    word ptr es:[di], ax
00C152:  8B C2                        mov      ax, dx
00C154:  AB                           stosw    word ptr es:[di], ax
00C155:  8C D8                        mov      ax, ds
00C157:  AB                           stosw    word ptr es:[di], ax
00C158:  06                           push     es
00C159:  1F                           pop      ds
00C15A:  A1 5A 0A                     mov      ax, word ptr [0xa5a]
00C15D:  AB                           stosw    word ptr es:[di], ax
00C15E:  66 A1 4E 0A                  mov      eax, dword ptr [0xa4e]
00C162:  66 AB                        stosd    dword ptr es:[di], eax
00C164:  66 81 06 4E 0A 00 7D 00 00   add      dword ptr [0xa4e], 0x7d00
00C16D:  66 B8 00 0B 00 00            mov      eax, 0xb00
00C173:  FF 1E 4A 0A                  lcall    [0xa4a]
00C177:  5B                           pop      bx
00C178:  1F                           pop      ds
00C179:  58                           pop      ax
00C17A:  66 2B E8                     sub      ebp, eax
00C17D:  74 50                        je       0xc1cf
00C17F:  EB AC                        jmp      0xc12d
00C181:  65 A1 47 0C                  mov      ax, word ptr gs:[0xc47]
00C185:  0B C0                        or       ax, ax
00C187:  74 08                        je       0xc191
00C189:  53                           push     bx
00C18A:  8B D8                        mov      bx, ax
00C18C:  B4 3E                        mov      ah, 0x3e
00C18E:  CD 21                        int      0x21
00C190:  5B                           pop      bx
00C191:  1E                           push     ds
00C192:  52                           push     dx
00C193:  06                           push     es
00C194:  1F                           pop      ds
00C195:  33 C9                        xor      cx, cx
00C197:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
00C19C:  BA A6 00                     mov      dx, 0xa6
00C19F:  B8 00 3C                     mov      ax, 0x3c00
00C1A2:  CD 21                        int      0x21
00C1A4:  A3 47 0C                     mov      word ptr [0xc47], ax
00C1A7:  5A                           pop      dx
00C1A8:  1F                           pop      ds
00C1A9:  B9 00 7D                     mov      cx, 0x7d00
00C1AC:  66 8B C5                     mov      eax, ebp
00C1AF:  66 2B C1                     sub      eax, ecx
00C1B2:  79 02                        jns      0xc1b6
00C1B4:  03 C8                        add      cx, ax
00C1B6:  B4 3F                        mov      ah, 0x3f
00C1B8:  CD 21                        int      0x21
00C1BA:  53                           push     bx
00C1BB:  8B C8                        mov      cx, ax
00C1BD:  65 8B 1E 47 0C               mov      bx, word ptr gs:[0xc47]
00C1C2:  B4 40                        mov      ah, 0x40
00C1C4:  CD 21                        int      0x21
00C1C6:  5B                           pop      bx
00C1C7:  66 2B E9                     sub      ebp, ecx
00C1CA:  75 DD                        jne      0xc1a9
00C1CC:  66 33 C0                     xor      eax, eax
00C1CF:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
00C1D5:  75 04                        jne      0xc1db
00C1D7:  B4 3E                        mov      ah, 0x3e
00C1D9:  CD 21                        int      0x21
00C1DB:  59                           pop      cx
00C1DC:  5A                           pop      dx
00C1DD:  66 5D                        pop      ebp
00C1DF:  5B                           pop      bx
00C1E0:  5F                           pop      di
00C1E1:  07                           pop      es
00C1E2:  5E                           pop      si
00C1E3:  1F                           pop      ds
00C1E4:  58                           pop      ax
00C1E5:  CB                           retf    
