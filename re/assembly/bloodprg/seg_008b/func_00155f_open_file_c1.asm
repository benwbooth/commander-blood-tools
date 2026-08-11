; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00155f
; seg_off: 008b:06af
; group: seg_008b
; provenance: recursive_graph
; label: open_file_c1
; label_comment: resource open: lcall 0x1ce:0x509 (prep); dx=0xc1 (filename in the string seg); ax=0x3d00; int21h (DOS open). Opens a specific resource file (name at gs:0xc1)
; byte_count: 169
; boundary: cfg_blocks_7_terminals_3
; terminal: jmp 0x1605:2, ret:1
; direct_callees: none
; indirect_calls: 3
; routine_bytes_sha256: 09cfefd4f2bb1664679eba55ffebe044280f07dfc25e3cb65cd20b8d5325060c

00155F:  06                           push     es
001560:  1E                           push     ds
001561:  9A 09 05 CE 01               lcall    0x1ce, 0x509
001566:  BA C1 00                     mov      dx, 0xc1
001569:  B8 00 3D                     mov      ax, 0x3d00
00156C:  CD 21                        int      0x21
00156E:  0F 82 93 00                  jb       0x1605
001572:  A3 86 0A                     mov      word ptr [0xa86], ax
001575:  8B D8                        mov      bx, ax
001577:  C5 16 BC 0A                  lds      dx, ptr [0xabc]
00157B:  B4 3F                        mov      ah, 0x3f
00157D:  B9 FF FF                     mov      cx, 0xffff
001580:  CD 21                        int      0x21
001582:  65 83 3E 64 0A FF            cmp      word ptr gs:[0xa64], -1
001588:  74 27                        je       0x15b1
00158A:  33 C0                        xor      ax, ax
00158C:  8B F8                        mov      di, ax
00158E:  8B F0                        mov      si, ax
001590:  8B D8                        mov      bx, ax
001592:  65 8B 16 64 0A               mov      dx, word ptr gs:[0xa64]
001597:  B9 04 00                     mov      cx, 4
00159A:  B4 44                        mov      ah, 0x44
00159C:  CD 67                        int      0x67
00159E:  FE C3                        inc      bl
0015A0:  FE C0                        inc      al
0015A2:  E2 F6                        loop     0x159a
0015A4:  65 8E 06 66 0A               mov      es, word ptr gs:[0xa66]
0015A9:  B9 00 40                     mov      cx, 0x4000
0015AC:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
0015AF:  EB 54                        jmp      0x1605
0015B1:  65 83 3E 62 0A FF            cmp      word ptr gs:[0xa62], -1
0015B7:  74 2C                        je       0x15e5
0015B9:  8C EB                        mov      bx, gs
0015BB:  8E C3                        mov      es, bx
0015BD:  BF 6C 0A                     mov      di, 0xa6c
0015C0:  8B F7                        mov      si, di
0015C2:  66 B8 00 00 01 00            mov      eax, 0x10000
0015C8:  66 AB                        stosd    dword ptr es:[di], eax
0015CA:  33 C0                        xor      ax, ax
0015CC:  AB                           stosw    word ptr es:[di], ax
0015CD:  AB                           stosw    word ptr es:[di], ax
0015CE:  8C D8                        mov      ax, ds
0015D0:  AB                           stosw    word ptr es:[di], ax
0015D1:  8E DB                        mov      ds, bx
0015D3:  A1 62 0A                     mov      ax, word ptr [0xa62]
0015D6:  AB                           stosw    word ptr es:[di], ax
0015D7:  66 33 C0                     xor      eax, eax
0015DA:  66 AB                        stosd    dword ptr es:[di], eax
0015DC:  B8 00 0B                     mov      ax, 0xb00
0015DF:  FF 1E 4A 0A                  lcall    [0xa4a]
0015E3:  EB 20                        jmp      0x1605
0015E5:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
0015EA:  1E                           push     ds
0015EB:  8C E8                        mov      ax, gs
0015ED:  8E D8                        mov      ds, ax
0015EF:  BA CB 00                     mov      dx, 0xcb
0015F2:  33 C9                        xor      cx, cx
0015F4:  B4 3C                        mov      ah, 0x3c
0015F6:  CD 21                        int      0x21
0015F8:  8B D8                        mov      bx, ax
0015FA:  A3 88 0A                     mov      word ptr [0xa88], ax
0015FD:  1F                           pop      ds
0015FE:  B9 FF FF                     mov      cx, 0xffff
001601:  B4 40                        mov      ah, 0x40
001603:  CD 21                        int      0x21
001605:  1F                           pop      ds
001606:  07                           pop      es
001607:  C3                           ret     
