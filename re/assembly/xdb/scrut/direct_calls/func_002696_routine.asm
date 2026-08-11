; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x002696
; group: direct_calls
; provenance: direct_call_from_0x5dc
; byte_count: 1364
; boundary: cfg_blocks_50_terminals_12
; terminal: jmp 0x27d6:1, jmp 0x2871:1, jmp 0x288b:5, jmp 0x2951:1, jmp 0x29a4:1, jmp word ptr [0x946]:1, jmp word ptr [si + 0x2c]:1, ret:1
; direct_callees: 0x002c9d
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/direct_calls/func_002696_routine.cpp
; routine_bytes_sha256: b1f14c151111b6dc06b028f898ad55ed00860a988c44cd418f259f0db474a8d0

002696:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
00269B:  FC                           cld     
00269C:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0026A1:  BB 3A 0D                     mov      bx, 0xd3a
0026A4:  89 1E D0 0B                  mov      word ptr [0xbd0], bx
0026A8:  B9 58 02                     mov      cx, 0x258
0026AB:  8B F3                        mov      si, bx
0026AD:  83 C3 5A                     add      bx, 0x5a
0026B0:  89 1C                        mov      word ptr [si], bx
0026B2:  E2 F7                        loop     0x26ab
0026B4:  C7 04 00 00                  mov      word ptr [si], 0
0026B8:  C7 06 4C 09 4E 09            mov      word ptr [0x94c], 0x94e
0026BE:  C7 06 48 09 00 00            mov      word ptr [0x948], 0
0026C4:  B9 40 01                     mov      cx, 0x140
0026C7:  8B 3E 4C 09                  mov      di, word ptr [0x94c]
0026CB:  2B 0E 48 09                  sub      cx, word ptr [0x948]
0026CF:  33 C0                        xor      ax, ax
0026D1:  F3 AF                        repe scasw ax, word ptr es:[di]
0026D3:  0F 84 92 00                  je       0x2769
0026D7:  BE 2C 0C                     mov      si, 0xc2c
0026DA:  BB 86 0C                     mov      bx, 0xc86
0026DD:  89 1C                        mov      word ptr [si], bx
0026DF:  C7 44 2E 4A 01               mov      word ptr [si + 0x2e], 0x14a
0026E4:  66 C7 44 08 00 00 00 80      mov      dword ptr [si + 8], 0x80000000
0026EC:  66 C7 44 18 00 00 00 00      mov      dword ptr [si + 0x18], 0
0026F4:  66 C7 44 0C 00 00 00 00      mov      dword ptr [si + 0xc], 0
0026FC:  66 C7 44 1C 00 00 00 00      mov      dword ptr [si + 0x1c], 0
002704:  89 77 10                     mov      word ptr [bx + 0x10], si
002707:  C7 07 E0 0C                  mov      word ptr [bx], 0xce0
00270B:  C7 47 2E 40 01               mov      word ptr [bx + 0x2e], 0x140
002710:  66 C7 47 08 00 00 00 00      mov      dword ptr [bx + 8], 0
002718:  66 C7 47 18 00 00 00 00      mov      dword ptr [bx + 0x18], 0
002720:  C7 47 0A C8 00               mov      word ptr [bx + 0xa], 0xc8
002725:  66 C7 47 18 00 00 FF 7F      mov      dword ptr [bx + 0x18], 0x7fff0000
00272D:  C7 47 02 00 80               mov      word ptr [bx + 2], 0x8000
002732:  BE E0 0C                     mov      si, 0xce0
002735:  66 C7 44 08 FF FF FF 7F      mov      dword ptr [si + 8], 0x7fffffff
00273D:  66 C7 44 18 FF FF FF 7F      mov      dword ptr [si + 0x18], 0x7fffffff
002745:  89 1E F0 0C                  mov      word ptr [0xcf0], bx
002749:  C7 06 0C 0D 94 27            mov      word ptr [0xd0c], 0x2794
00274F:  C7 06 0E 0D FF FF            mov      word ptr [0xd0e], 0xffff
002755:  BB 3F 01                     mov      bx, 0x13f
002758:  83 EF 02                     sub      di, 2
00275B:  2B D9                        sub      bx, cx
00275D:  89 3E 4C 09                  mov      word ptr [0x94c], di
002761:  89 1E 48 09                  mov      word ptr [0x948], bx
002765:  8B 35                        mov      si, word ptr [di]
002767:  EB 6D                        jmp      0x27d6
002769:  C3                           ret     
; -- non-contiguous block: next 0x0027d6 --
0027D6:  C7 05 00 00                  mov      word ptr [di], 0
0027DA:  F7 06 D0 0B FF FF            test     word ptr [0xbd0], 0xffff
0027E0:  74 10                        je       0x27f2
0027E2:  64 8E 06 02 00               mov      es, word ptr fs:[2]
0027E7:  26 FF 34                     push     word ptr es:[si]
0027EA:  E8 B0 04                     call     0x2c9d
0027ED:  5E                           pop      si
0027EE:  0B F6                        or       si, si
0027F0:  75 F5                        jne      0x27e7
0027F2:  BE 2C 0C                     mov      si, 0xc2c
0027F5:  8B 04                        mov      ax, word ptr [si]
0027F7:  3D 86 0C                     cmp      ax, 0xc86
0027FA:  0F 84 B3 03                  je       0x2bb1
0027FE:  BA 3C 0C                     mov      dx, 0xc3c
002801:  C7 44 02 01 00               mov      word ptr [si + 2], 1
002806:  89 54 06                     mov      word ptr [si + 6], dx
002809:  8B FE                        mov      di, si
00280B:  8B EE                        mov      bp, si
00280D:  33 DB                        xor      bx, bx
00280F:  8B 3D                        mov      di, word ptr [di]
002811:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
002816:  75 F7                        jne      0x280f
002818:  89 5C 58                     mov      word ptr [si + 0x58], bx
00281B:  89 5D 58                     mov      word ptr [di + 0x58], bx
00281E:  3B 5D 0A                     cmp      bx, word ptr [di + 0xa]
002821:  0F 8E A1 00                  jle      0x28c6
002825:  8B F7                        mov      si, di
002827:  8B EA                        mov      bp, dx
002829:  89 3E 2A 0C                  mov      word ptr [0xc2a], di
00282D:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
002832:  F7 D8                        neg      ax
002834:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
002838:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
00283C:  66 89 45 04                  mov      dword ptr [di + 4], eax
002840:  EB 2F                        jmp      0x2871
002842:  BB D2 0B                     mov      bx, 0xbd2
002845:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
00284A:  F7 D8                        neg      ax
00284C:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
002850:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
002854:  66 89 45 04                  mov      dword ptr [di + 4], eax
002858:  66 3B 44 04                  cmp      eax, dword ptr [si + 4]
00285C:  7E 09                        jle      0x2867
00285E:  8B DE                        mov      bx, si
002860:  8B 74 58                     mov      si, word ptr [si + 0x58]
002863:  0B F6                        or       si, si
002865:  75 F1                        jne      0x2858
002867:  89 7F 58                     mov      word ptr [bx + 0x58], di
00286A:  89 75 58                     mov      word ptr [di + 0x58], si
00286D:  8B 36 2A 0C                  mov      si, word ptr [0xc2a]
002871:  8B 3D                        mov      di, word ptr [di]
002873:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
002878:  75 F7                        jne      0x2871
00287A:  F7 45 0A 00 80               test     word ptr [di + 0xa], 0x8000
00287F:  75 C1                        jne      0x2842
002881:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002887:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
00288B:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00288F:  8B 44 1A                     mov      ax, word ptr [si + 0x1a]
002892:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002895:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
002898:  7D 55                        jge      0x28ef
00289A:  3B C2                        cmp      ax, dx
00289C:  0F 8F B1 00                  jg       0x2951
0028A0:  74 72                        je       0x2914
0028A2:  8D 54 10                     lea      dx, [si + 0x10]
0028A5:  8B 4C 1A                     mov      cx, word ptr [si + 0x1a]
0028A8:  3E 89 56 06                  mov      word ptr ds:[bp + 6], dx
0028AC:  8B EA                        mov      bp, dx
0028AE:  8B 74 58                     mov      si, word ptr [si + 0x58]
0028B1:  0B F6                        or       si, si
0028B3:  74 11                        je       0x28c6
0028B5:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
0028B8:  7D F4                        jge      0x28ae
0028BA:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
0028C0:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
0028C4:  EB C5                        jmp      0x288b
0028C6:  81 FF 86 0C                  cmp      di, 0xc86
0028CA:  0F 84 1B 02                  je       0x2ae9
0028CE:  3E C7 46 02 01 00            mov      word ptr ds:[bp + 2], 1
0028D4:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0028D8:  8B EF                        mov      bp, di
0028DA:  C7 45 58 00 00               mov      word ptr [di + 0x58], 0
0028DF:  8B F7                        mov      si, di
0028E1:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
0028E7:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
0028EB:  8B 3D                        mov      di, word ptr [di]
0028ED:  EB 9C                        jmp      0x288b
0028EF:  8B 3D                        mov      di, word ptr [di]
0028F1:  EB 98                        jmp      0x288b
; -- non-contiguous block: next 0x002914 --
002914:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002918:  8B 4D 0A                     mov      cx, word ptr [di + 0xa]
00291B:  8B EF                        mov      bp, di
00291D:  81 FF 86 0C                  cmp      di, 0xc86
002921:  0F 84 C4 01                  je       0x2ae9
002925:  8B 74 58                     mov      si, word ptr [si + 0x58]
002928:  0B F6                        or       si, si
00292A:  74 11                        je       0x293d
00292C:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
00292F:  7D F4                        jge      0x2925
002931:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002937:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
00293B:  EB 14                        jmp      0x2951
00293D:  89 75 58                     mov      word ptr [di + 0x58], si
002940:  8B F7                        mov      si, di
002942:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
002948:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
00294C:  8B 3D                        mov      di, word ptr [di]
00294E:  E9 3A FF                     jmp      0x288b
002951:  81 FF 86 0C                  cmp      di, 0xc86
002955:  0F 84 8A 01                  je       0x2ae3
002959:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
00295C:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
00295F:  7D 49                        jge      0x29aa
002961:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
002965:  BB D2 0B                     mov      bx, 0xbd2
002968:  89 77 58                     mov      word ptr [bx + 0x58], si
00296B:  66 8B 4D 08                  mov      ecx, dword ptr [di + 8]
00296F:  66 3B 4C 18                  cmp      ecx, dword ptr [si + 0x18]
002973:  7C 0C                        jl       0x2981
002975:  8B 74 58                     mov      si, word ptr [si + 0x58]
002978:  0B F6                        or       si, si
00297A:  89 77 58                     mov      word ptr [bx + 0x58], si
00297D:  75 F0                        jne      0x296f
00297F:  EB 23                        jmp      0x29a4
002981:  66 8B C1                     mov      eax, ecx
002984:  66 2B 44 08                  sub      eax, dword ptr [si + 8]
002988:  66 F7 6C 28                  imul     dword ptr [si + 0x28]
00298C:  66 0F AC D0 10               shrd     eax, edx, 0x10
002991:  66 03 44 20                  add      eax, dword ptr [si + 0x20]
002995:  66 3B 45 20                  cmp      eax, dword ptr [di + 0x20]
002999:  7D 09                        jge      0x29a4
00299B:  8B DE                        mov      bx, si
00299D:  8B 74 58                     mov      si, word ptr [si + 0x58]
0029A0:  0B F6                        or       si, si
0029A2:  75 CB                        jne      0x296f
0029A4:  89 7F 58                     mov      word ptr [bx + 0x58], di
0029A7:  89 75 58                     mov      word ptr [di + 0x58], si
0029AA:  8B 36 2A 0C                  mov      si, word ptr [0xc2a]
0029AE:  3B F7                        cmp      si, di
0029B0:  75 0E                        jne      0x29c0
0029B2:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0029B6:  8B EF                        mov      bp, di
0029B8:  C7 45 02 00 00               mov      word ptr [di + 2], 0
0029BD:  89 7D 04                     mov      word ptr [di + 4], di
0029C0:  8B 3D                        mov      di, word ptr [di]
0029C2:  E9 C6 FE                     jmp      0x288b
; -- non-contiguous block: next 0x002ae3 --
002AE3:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
002AE7:  8B EF                        mov      bp, di
002AE9:  3E C7 46 02 00 80            mov      word ptr ds:[bp + 2], 0x8000
002AEF:  BB 2C 0C                     mov      bx, 0xc2c
002AF2:  FF 26 46 09                  jmp      word ptr [0x946]
; -- non-contiguous block: next 0x002bb1 --
002BB1:  8B 36 2C 0C                  mov      si, word ptr [0xc2c]
002BB5:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002BB8:  78 2D                        js       0x2be7
002BBA:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
002BBD:  8B 5C 4C                     mov      bx, word ptr [si + 0x4c]
002BC0:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
002BC4:  66 8B 54 24                  mov      edx, dword ptr [si + 0x24]
002BC8:  01 44 42                     add      word ptr [si + 0x42], ax
002BCB:  01 5C 44                     add      word ptr [si + 0x44], bx
002BCE:  66 01 4C 08                  add      dword ptr [si + 8], ecx
002BD2:  66 01 54 20                  add      dword ptr [si + 0x20], edx
002BD6:  8B DE                        mov      bx, si
002BD8:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
002BDC:  8B 37                        mov      si, word ptr [bx]
002BDE:  66 01 4F 18                  add      dword ptr [bx + 0x18], ecx
002BE2:  FF 4C 2E                     dec      word ptr [si + 0x2e]
002BE5:  79 D3                        jns      0x2bba
002BE7:  FF 64 2C                     jmp      word ptr [si + 0x2c]
