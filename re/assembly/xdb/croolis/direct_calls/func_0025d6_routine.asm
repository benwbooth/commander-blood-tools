; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0025d6
; group: direct_calls
; provenance: direct_call_from_0x5dc
; byte_count: 1364
; boundary: cfg_blocks_50_terminals_12
; terminal: jmp 0x2716:1, jmp 0x27b1:1, jmp 0x27cb:5, jmp 0x2891:1, jmp 0x28e4:1, jmp word ptr [0x946]:1, jmp word ptr [si + 0x2c]:1, ret:1
; direct_callees: 0x002bdd
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/direct_calls/func_0025d6_routine.cpp
; routine_bytes_sha256: e4f99db84ca2c0efce626e4333e41b776e54f041badd4d7ecbee28e1a565a90e

0025D6:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
0025DB:  FC                           cld     
0025DC:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0025E1:  BB 3A 0D                     mov      bx, 0xd3a
0025E4:  89 1E D0 0B                  mov      word ptr [0xbd0], bx
0025E8:  B9 58 02                     mov      cx, 0x258
0025EB:  8B F3                        mov      si, bx
0025ED:  83 C3 5A                     add      bx, 0x5a
0025F0:  89 1C                        mov      word ptr [si], bx
0025F2:  E2 F7                        loop     0x25eb
0025F4:  C7 04 00 00                  mov      word ptr [si], 0
0025F8:  C7 06 4C 09 4E 09            mov      word ptr [0x94c], 0x94e
0025FE:  C7 06 48 09 00 00            mov      word ptr [0x948], 0
002604:  B9 40 01                     mov      cx, 0x140
002607:  8B 3E 4C 09                  mov      di, word ptr [0x94c]
00260B:  2B 0E 48 09                  sub      cx, word ptr [0x948]
00260F:  33 C0                        xor      ax, ax
002611:  F3 AF                        repe scasw ax, word ptr es:[di]
002613:  0F 84 92 00                  je       0x26a9
002617:  BE 2C 0C                     mov      si, 0xc2c
00261A:  BB 86 0C                     mov      bx, 0xc86
00261D:  89 1C                        mov      word ptr [si], bx
00261F:  C7 44 2E 4A 01               mov      word ptr [si + 0x2e], 0x14a
002624:  66 C7 44 08 00 00 00 80      mov      dword ptr [si + 8], 0x80000000
00262C:  66 C7 44 18 00 00 00 00      mov      dword ptr [si + 0x18], 0
002634:  66 C7 44 0C 00 00 00 00      mov      dword ptr [si + 0xc], 0
00263C:  66 C7 44 1C 00 00 00 00      mov      dword ptr [si + 0x1c], 0
002644:  89 77 10                     mov      word ptr [bx + 0x10], si
002647:  C7 07 E0 0C                  mov      word ptr [bx], 0xce0
00264B:  C7 47 2E 40 01               mov      word ptr [bx + 0x2e], 0x140
002650:  66 C7 47 08 00 00 00 00      mov      dword ptr [bx + 8], 0
002658:  66 C7 47 18 00 00 00 00      mov      dword ptr [bx + 0x18], 0
002660:  C7 47 0A C8 00               mov      word ptr [bx + 0xa], 0xc8
002665:  66 C7 47 18 00 00 FF 7F      mov      dword ptr [bx + 0x18], 0x7fff0000
00266D:  C7 47 02 00 80               mov      word ptr [bx + 2], 0x8000
002672:  BE E0 0C                     mov      si, 0xce0
002675:  66 C7 44 08 FF FF FF 7F      mov      dword ptr [si + 8], 0x7fffffff
00267D:  66 C7 44 18 FF FF FF 7F      mov      dword ptr [si + 0x18], 0x7fffffff
002685:  89 1E F0 0C                  mov      word ptr [0xcf0], bx
002689:  C7 06 0C 0D D4 26            mov      word ptr [0xd0c], 0x26d4
00268F:  C7 06 0E 0D FF FF            mov      word ptr [0xd0e], 0xffff
002695:  BB 3F 01                     mov      bx, 0x13f
002698:  83 EF 02                     sub      di, 2
00269B:  2B D9                        sub      bx, cx
00269D:  89 3E 4C 09                  mov      word ptr [0x94c], di
0026A1:  89 1E 48 09                  mov      word ptr [0x948], bx
0026A5:  8B 35                        mov      si, word ptr [di]
0026A7:  EB 6D                        jmp      0x2716
0026A9:  C3                           ret     
; -- non-contiguous block: next 0x002716 --
002716:  C7 05 00 00                  mov      word ptr [di], 0
00271A:  F7 06 D0 0B FF FF            test     word ptr [0xbd0], 0xffff
002720:  74 10                        je       0x2732
002722:  64 8E 06 02 00               mov      es, word ptr fs:[2]
002727:  26 FF 34                     push     word ptr es:[si]
00272A:  E8 B0 04                     call     0x2bdd
00272D:  5E                           pop      si
00272E:  0B F6                        or       si, si
002730:  75 F5                        jne      0x2727
002732:  BE 2C 0C                     mov      si, 0xc2c
002735:  8B 04                        mov      ax, word ptr [si]
002737:  3D 86 0C                     cmp      ax, 0xc86
00273A:  0F 84 B3 03                  je       0x2af1
00273E:  BA 3C 0C                     mov      dx, 0xc3c
002741:  C7 44 02 01 00               mov      word ptr [si + 2], 1
002746:  89 54 06                     mov      word ptr [si + 6], dx
002749:  8B FE                        mov      di, si
00274B:  8B EE                        mov      bp, si
00274D:  33 DB                        xor      bx, bx
00274F:  8B 3D                        mov      di, word ptr [di]
002751:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
002756:  75 F7                        jne      0x274f
002758:  89 5C 58                     mov      word ptr [si + 0x58], bx
00275B:  89 5D 58                     mov      word ptr [di + 0x58], bx
00275E:  3B 5D 0A                     cmp      bx, word ptr [di + 0xa]
002761:  0F 8E A1 00                  jle      0x2806
002765:  8B F7                        mov      si, di
002767:  8B EA                        mov      bp, dx
002769:  89 3E 2A 0C                  mov      word ptr [0xc2a], di
00276D:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
002772:  F7 D8                        neg      ax
002774:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
002778:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
00277C:  66 89 45 04                  mov      dword ptr [di + 4], eax
002780:  EB 2F                        jmp      0x27b1
002782:  BB D2 0B                     mov      bx, 0xbd2
002785:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
00278A:  F7 D8                        neg      ax
00278C:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
002790:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
002794:  66 89 45 04                  mov      dword ptr [di + 4], eax
002798:  66 3B 44 04                  cmp      eax, dword ptr [si + 4]
00279C:  7E 09                        jle      0x27a7
00279E:  8B DE                        mov      bx, si
0027A0:  8B 74 58                     mov      si, word ptr [si + 0x58]
0027A3:  0B F6                        or       si, si
0027A5:  75 F1                        jne      0x2798
0027A7:  89 7F 58                     mov      word ptr [bx + 0x58], di
0027AA:  89 75 58                     mov      word ptr [di + 0x58], si
0027AD:  8B 36 2A 0C                  mov      si, word ptr [0xc2a]
0027B1:  8B 3D                        mov      di, word ptr [di]
0027B3:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
0027B8:  75 F7                        jne      0x27b1
0027BA:  F7 45 0A 00 80               test     word ptr [di + 0xa], 0x8000
0027BF:  75 C1                        jne      0x2782
0027C1:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
0027C7:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
0027CB:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
0027CF:  8B 44 1A                     mov      ax, word ptr [si + 0x1a]
0027D2:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
0027D5:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
0027D8:  7D 55                        jge      0x282f
0027DA:  3B C2                        cmp      ax, dx
0027DC:  0F 8F B1 00                  jg       0x2891
0027E0:  74 72                        je       0x2854
0027E2:  8D 54 10                     lea      dx, [si + 0x10]
0027E5:  8B 4C 1A                     mov      cx, word ptr [si + 0x1a]
0027E8:  3E 89 56 06                  mov      word ptr ds:[bp + 6], dx
0027EC:  8B EA                        mov      bp, dx
0027EE:  8B 74 58                     mov      si, word ptr [si + 0x58]
0027F1:  0B F6                        or       si, si
0027F3:  74 11                        je       0x2806
0027F5:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
0027F8:  7D F4                        jge      0x27ee
0027FA:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002800:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
002804:  EB C5                        jmp      0x27cb
002806:  81 FF 86 0C                  cmp      di, 0xc86
00280A:  0F 84 1B 02                  je       0x2a29
00280E:  3E C7 46 02 01 00            mov      word ptr ds:[bp + 2], 1
002814:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002818:  8B EF                        mov      bp, di
00281A:  C7 45 58 00 00               mov      word ptr [di + 0x58], 0
00281F:  8B F7                        mov      si, di
002821:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002827:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
00282B:  8B 3D                        mov      di, word ptr [di]
00282D:  EB 9C                        jmp      0x27cb
00282F:  8B 3D                        mov      di, word ptr [di]
002831:  EB 98                        jmp      0x27cb
; -- non-contiguous block: next 0x002854 --
002854:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002858:  8B 4D 0A                     mov      cx, word ptr [di + 0xa]
00285B:  8B EF                        mov      bp, di
00285D:  81 FF 86 0C                  cmp      di, 0xc86
002861:  0F 84 C4 01                  je       0x2a29
002865:  8B 74 58                     mov      si, word ptr [si + 0x58]
002868:  0B F6                        or       si, si
00286A:  74 11                        je       0x287d
00286C:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
00286F:  7D F4                        jge      0x2865
002871:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002877:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
00287B:  EB 14                        jmp      0x2891
00287D:  89 75 58                     mov      word ptr [di + 0x58], si
002880:  8B F7                        mov      si, di
002882:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002888:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
00288C:  8B 3D                        mov      di, word ptr [di]
00288E:  E9 3A FF                     jmp      0x27cb
002891:  81 FF 86 0C                  cmp      di, 0xc86
002895:  0F 84 8A 01                  je       0x2a23
002899:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
00289C:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
00289F:  7D 49                        jge      0x28ea
0028A1:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
0028A5:  BB D2 0B                     mov      bx, 0xbd2
0028A8:  89 77 58                     mov      word ptr [bx + 0x58], si
0028AB:  66 8B 4D 08                  mov      ecx, dword ptr [di + 8]
0028AF:  66 3B 4C 18                  cmp      ecx, dword ptr [si + 0x18]
0028B3:  7C 0C                        jl       0x28c1
0028B5:  8B 74 58                     mov      si, word ptr [si + 0x58]
0028B8:  0B F6                        or       si, si
0028BA:  89 77 58                     mov      word ptr [bx + 0x58], si
0028BD:  75 F0                        jne      0x28af
0028BF:  EB 23                        jmp      0x28e4
0028C1:  66 8B C1                     mov      eax, ecx
0028C4:  66 2B 44 08                  sub      eax, dword ptr [si + 8]
0028C8:  66 F7 6C 28                  imul     dword ptr [si + 0x28]
0028CC:  66 0F AC D0 10               shrd     eax, edx, 0x10
0028D1:  66 03 44 20                  add      eax, dword ptr [si + 0x20]
0028D5:  66 3B 45 20                  cmp      eax, dword ptr [di + 0x20]
0028D9:  7D 09                        jge      0x28e4
0028DB:  8B DE                        mov      bx, si
0028DD:  8B 74 58                     mov      si, word ptr [si + 0x58]
0028E0:  0B F6                        or       si, si
0028E2:  75 CB                        jne      0x28af
0028E4:  89 7F 58                     mov      word ptr [bx + 0x58], di
0028E7:  89 75 58                     mov      word ptr [di + 0x58], si
0028EA:  8B 36 2A 0C                  mov      si, word ptr [0xc2a]
0028EE:  3B F7                        cmp      si, di
0028F0:  75 0E                        jne      0x2900
0028F2:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0028F6:  8B EF                        mov      bp, di
0028F8:  C7 45 02 00 00               mov      word ptr [di + 2], 0
0028FD:  89 7D 04                     mov      word ptr [di + 4], di
002900:  8B 3D                        mov      di, word ptr [di]
002902:  E9 C6 FE                     jmp      0x27cb
; -- non-contiguous block: next 0x002a23 --
002A23:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002A27:  8B EF                        mov      bp, di
002A29:  3E C7 46 02 00 80            mov      word ptr ds:[bp + 2], 0x8000
002A2F:  BB 2C 0C                     mov      bx, 0xc2c
002A32:  FF 26 46 09                  jmp      word ptr [0x946]
; -- non-contiguous block: next 0x002af1 --
002AF1:  8B 36 2C 0C                  mov      si, word ptr [0xc2c]
002AF5:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002AF8:  78 2D                        js       0x2b27
002AFA:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
002AFD:  8B 5C 4C                     mov      bx, word ptr [si + 0x4c]
002B00:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
002B04:  66 8B 54 24                  mov      edx, dword ptr [si + 0x24]
002B08:  01 44 42                     add      word ptr [si + 0x42], ax
002B0B:  01 5C 44                     add      word ptr [si + 0x44], bx
002B0E:  66 01 4C 08                  add      dword ptr [si + 8], ecx
002B12:  66 01 54 20                  add      dword ptr [si + 0x20], edx
002B16:  8B DE                        mov      bx, si
002B18:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002B1C:  8B 37                        mov      si, word ptr [bx]
002B1E:  66 01 4F 18                  add      dword ptr [bx + 0x18], ecx
002B22:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002B25:  79 D3                        jns      0x2afa
002B27:  FF 64 2C                     jmp      word ptr [si + 0x2c]
