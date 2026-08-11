; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x002572
; group: direct_calls
; provenance: direct_call_from_0x59b
; byte_count: 1352
; boundary: cfg_blocks_50_terminals_12
; terminal: jmp 0x26b2:1, jmp 0x274d:1, jmp 0x2767:5, jmp 0x282d:1, jmp 0x2880:1, jmp word ptr [0x944]:1, jmp word ptr [si + 0x2c]:1, ret:1
; direct_callees: 0x002b6d
; indirect_calls: 0
; routine_bytes_sha256: e36eed22b577ba7e098c16f6b328d5c9490ae80d6a82ec8d56134bc9b1a42ca2

002572:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
002577:  FC                           cld     
002578:  64 8E 06 06 00               mov      es, word ptr fs:[6]
00257D:  BB 38 0D                     mov      bx, 0xd38
002580:  89 1E CE 0B                  mov      word ptr [0xbce], bx
002584:  B9 58 02                     mov      cx, 0x258
002587:  8B F3                        mov      si, bx
002589:  83 C3 5A                     add      bx, 0x5a
00258C:  89 1C                        mov      word ptr [si], bx
00258E:  E2 F7                        loop     0x2587
002590:  C7 04 00 00                  mov      word ptr [si], 0
002594:  C7 06 4A 09 4C 09            mov      word ptr [0x94a], 0x94c
00259A:  C7 06 46 09 00 00            mov      word ptr [0x946], 0
0025A0:  B9 40 01                     mov      cx, 0x140
0025A3:  8B 3E 4A 09                  mov      di, word ptr [0x94a]
0025A7:  2B 0E 46 09                  sub      cx, word ptr [0x946]
0025AB:  33 C0                        xor      ax, ax
0025AD:  F3 AF                        repe scasw ax, word ptr es:[di]
0025AF:  0F 84 92 00                  je       0x2645
0025B3:  BE 2A 0C                     mov      si, 0xc2a
0025B6:  BB 84 0C                     mov      bx, 0xc84
0025B9:  89 1C                        mov      word ptr [si], bx
0025BB:  C7 44 2E 4A 01               mov      word ptr [si + 0x2e], 0x14a
0025C0:  66 C7 44 08 00 00 00 80      mov      dword ptr [si + 8], 0x80000000
0025C8:  66 C7 44 18 00 00 00 00      mov      dword ptr [si + 0x18], 0
0025D0:  66 C7 44 0C 00 00 00 00      mov      dword ptr [si + 0xc], 0
0025D8:  66 C7 44 1C 00 00 00 00      mov      dword ptr [si + 0x1c], 0
0025E0:  89 77 10                     mov      word ptr [bx + 0x10], si
0025E3:  C7 07 DE 0C                  mov      word ptr [bx], 0xcde
0025E7:  C7 47 2E 40 01               mov      word ptr [bx + 0x2e], 0x140
0025EC:  66 C7 47 08 00 00 00 00      mov      dword ptr [bx + 8], 0
0025F4:  66 C7 47 18 00 00 00 00      mov      dword ptr [bx + 0x18], 0
0025FC:  C7 47 0A C8 00               mov      word ptr [bx + 0xa], 0xc8
002601:  66 C7 47 18 00 00 FF 7F      mov      dword ptr [bx + 0x18], 0x7fff0000
002609:  C7 47 02 00 80               mov      word ptr [bx + 2], 0x8000
00260E:  BE DE 0C                     mov      si, 0xcde
002611:  66 C7 44 08 FF FF FF 7F      mov      dword ptr [si + 8], 0x7fffffff
002619:  66 C7 44 18 FF FF FF 7F      mov      dword ptr [si + 0x18], 0x7fffffff
002621:  89 1E EE 0C                  mov      word ptr [0xcee], bx
002625:  C7 06 0A 0D 70 26            mov      word ptr [0xd0a], 0x2670
00262B:  C7 06 0C 0D FF FF            mov      word ptr [0xd0c], 0xffff
002631:  BB 3F 01                     mov      bx, 0x13f
002634:  83 EF 02                     sub      di, 2
002637:  2B D9                        sub      bx, cx
002639:  89 3E 4A 09                  mov      word ptr [0x94a], di
00263D:  89 1E 46 09                  mov      word ptr [0x946], bx
002641:  8B 35                        mov      si, word ptr [di]
002643:  EB 6D                        jmp      0x26b2
002645:  C3                           ret     
; -- non-contiguous block: next 0x0026b2 --
0026B2:  C7 05 00 00                  mov      word ptr [di], 0
0026B6:  F7 06 CE 0B FF FF            test     word ptr [0xbce], 0xffff
0026BC:  74 10                        je       0x26ce
0026BE:  64 8E 06 02 00               mov      es, word ptr fs:[2]
0026C3:  26 FF 34                     push     word ptr es:[si]
0026C6:  E8 A4 04                     call     0x2b6d
0026C9:  5E                           pop      si
0026CA:  0B F6                        or       si, si
0026CC:  75 F5                        jne      0x26c3
0026CE:  BE 2A 0C                     mov      si, 0xc2a
0026D1:  8B 04                        mov      ax, word ptr [si]
0026D3:  3D 84 0C                     cmp      ax, 0xc84
0026D6:  0F 84 A7 03                  je       0x2a81
0026DA:  BA 3A 0C                     mov      dx, 0xc3a
0026DD:  C7 44 02 01 00               mov      word ptr [si + 2], 1
0026E2:  89 54 06                     mov      word ptr [si + 6], dx
0026E5:  8B FE                        mov      di, si
0026E7:  8B EE                        mov      bp, si
0026E9:  33 DB                        xor      bx, bx
0026EB:  8B 3D                        mov      di, word ptr [di]
0026ED:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
0026F2:  75 F7                        jne      0x26eb
0026F4:  89 5C 58                     mov      word ptr [si + 0x58], bx
0026F7:  89 5D 58                     mov      word ptr [di + 0x58], bx
0026FA:  3B 5D 0A                     cmp      bx, word ptr [di + 0xa]
0026FD:  0F 8E A1 00                  jle      0x27a2
002701:  8B F7                        mov      si, di
002703:  8B EA                        mov      bp, dx
002705:  89 3E 28 0C                  mov      word ptr [0xc28], di
002709:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
00270E:  F7 D8                        neg      ax
002710:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
002714:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
002718:  66 89 45 04                  mov      dword ptr [di + 4], eax
00271C:  EB 2F                        jmp      0x274d
00271E:  BB D0 0B                     mov      bx, 0xbd0
002721:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
002726:  F7 D8                        neg      ax
002728:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
00272C:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
002730:  66 89 45 04                  mov      dword ptr [di + 4], eax
002734:  66 3B 44 04                  cmp      eax, dword ptr [si + 4]
002738:  7E 09                        jle      0x2743
00273A:  8B DE                        mov      bx, si
00273C:  8B 74 58                     mov      si, word ptr [si + 0x58]
00273F:  0B F6                        or       si, si
002741:  75 F1                        jne      0x2734
002743:  89 7F 58                     mov      word ptr [bx + 0x58], di
002746:  89 75 58                     mov      word ptr [di + 0x58], si
002749:  8B 36 28 0C                  mov      si, word ptr [0xc28]
00274D:  8B 3D                        mov      di, word ptr [di]
00274F:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
002754:  75 F7                        jne      0x274d
002756:  F7 45 0A 00 80               test     word ptr [di + 0xa], 0x8000
00275B:  75 C1                        jne      0x271e
00275D:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002763:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
002767:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00276B:  8B 44 1A                     mov      ax, word ptr [si + 0x1a]
00276E:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002771:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
002774:  7D 55                        jge      0x27cb
002776:  3B C2                        cmp      ax, dx
002778:  0F 8F B1 00                  jg       0x282d
00277C:  74 72                        je       0x27f0
00277E:  8D 54 10                     lea      dx, [si + 0x10]
002781:  8B 4C 1A                     mov      cx, word ptr [si + 0x1a]
002784:  3E 89 56 06                  mov      word ptr ds:[bp + 6], dx
002788:  8B EA                        mov      bp, dx
00278A:  8B 74 58                     mov      si, word ptr [si + 0x58]
00278D:  0B F6                        or       si, si
00278F:  74 11                        je       0x27a2
002791:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
002794:  7D F4                        jge      0x278a
002796:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
00279C:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
0027A0:  EB C5                        jmp      0x2767
0027A2:  81 FF 84 0C                  cmp      di, 0xc84
0027A6:  0F 84 0F 02                  je       0x29b9
0027AA:  3E C7 46 02 01 00            mov      word ptr ds:[bp + 2], 1
0027B0:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0027B4:  8B EF                        mov      bp, di
0027B6:  C7 45 58 00 00               mov      word ptr [di + 0x58], 0
0027BB:  8B F7                        mov      si, di
0027BD:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
0027C3:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
0027C7:  8B 3D                        mov      di, word ptr [di]
0027C9:  EB 9C                        jmp      0x2767
0027CB:  8B 3D                        mov      di, word ptr [di]
0027CD:  EB 98                        jmp      0x2767
; -- non-contiguous block: next 0x0027f0 --
0027F0:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0027F4:  8B 4D 0A                     mov      cx, word ptr [di + 0xa]
0027F7:  8B EF                        mov      bp, di
0027F9:  81 FF 84 0C                  cmp      di, 0xc84
0027FD:  0F 84 B8 01                  je       0x29b9
002801:  8B 74 58                     mov      si, word ptr [si + 0x58]
002804:  0B F6                        or       si, si
002806:  74 11                        je       0x2819
002808:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
00280B:  7D F4                        jge      0x2801
00280D:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002813:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
002817:  EB 14                        jmp      0x282d
002819:  89 75 58                     mov      word ptr [di + 0x58], si
00281C:  8B F7                        mov      si, di
00281E:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002824:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
002828:  8B 3D                        mov      di, word ptr [di]
00282A:  E9 3A FF                     jmp      0x2767
00282D:  81 FF 84 0C                  cmp      di, 0xc84
002831:  0F 84 7E 01                  je       0x29b3
002835:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002838:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
00283B:  7D 49                        jge      0x2886
00283D:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
002841:  BB D0 0B                     mov      bx, 0xbd0
002844:  89 77 58                     mov      word ptr [bx + 0x58], si
002847:  66 8B 4D 08                  mov      ecx, dword ptr [di + 8]
00284B:  66 3B 4C 18                  cmp      ecx, dword ptr [si + 0x18]
00284F:  7C 0C                        jl       0x285d
002851:  8B 74 58                     mov      si, word ptr [si + 0x58]
002854:  0B F6                        or       si, si
002856:  89 77 58                     mov      word ptr [bx + 0x58], si
002859:  75 F0                        jne      0x284b
00285B:  EB 23                        jmp      0x2880
00285D:  66 8B C1                     mov      eax, ecx
002860:  66 2B 44 08                  sub      eax, dword ptr [si + 8]
002864:  66 F7 6C 28                  imul     dword ptr [si + 0x28]
002868:  66 0F AC D0 10               shrd     eax, edx, 0x10
00286D:  66 03 44 20                  add      eax, dword ptr [si + 0x20]
002871:  66 3B 45 20                  cmp      eax, dword ptr [di + 0x20]
002875:  7D 09                        jge      0x2880
002877:  8B DE                        mov      bx, si
002879:  8B 74 58                     mov      si, word ptr [si + 0x58]
00287C:  0B F6                        or       si, si
00287E:  75 CB                        jne      0x284b
002880:  89 7F 58                     mov      word ptr [bx + 0x58], di
002883:  89 75 58                     mov      word ptr [di + 0x58], si
002886:  8B 36 28 0C                  mov      si, word ptr [0xc28]
00288A:  3B F7                        cmp      si, di
00288C:  75 0E                        jne      0x289c
00288E:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002892:  8B EF                        mov      bp, di
002894:  C7 45 02 00 00               mov      word ptr [di + 2], 0
002899:  89 7D 04                     mov      word ptr [di + 4], di
00289C:  8B 3D                        mov      di, word ptr [di]
00289E:  E9 C6 FE                     jmp      0x2767
; -- non-contiguous block: next 0x0029b3 --
0029B3:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0029B7:  8B EF                        mov      bp, di
0029B9:  3E C7 46 02 00 80            mov      word ptr ds:[bp + 2], 0x8000
0029BF:  BB 2A 0C                     mov      bx, 0xc2a
0029C2:  FF 26 44 09                  jmp      word ptr [0x944]
; -- non-contiguous block: next 0x002a81 --
002A81:  8B 36 2A 0C                  mov      si, word ptr [0xc2a]
002A85:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002A88:  78 2D                        js       0x2ab7
002A8A:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
002A8D:  8B 5C 4C                     mov      bx, word ptr [si + 0x4c]
002A90:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
002A94:  66 8B 54 24                  mov      edx, dword ptr [si + 0x24]
002A98:  01 44 42                     add      word ptr [si + 0x42], ax
002A9B:  01 5C 44                     add      word ptr [si + 0x44], bx
002A9E:  66 01 4C 08                  add      dword ptr [si + 8], ecx
002AA2:  66 01 54 20                  add      dword ptr [si + 0x20], edx
002AA6:  8B DE                        mov      bx, si
002AA8:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002AAC:  8B 37                        mov      si, word ptr [bx]
002AAE:  66 01 4F 18                  add      dword ptr [bx + 0x18], ecx
002AB2:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002AB5:  79 D3                        jns      0x2a8a
002AB7:  FF 64 2C                     jmp      word ptr [si + 0x2c]
