; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x00059b
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 403
; boundary: cfg_blocks_29_terminals_2
; terminal: jmp 0x6f9:1, ret:1
; direct_callees: 0x002572
; indirect_calls: 0
; routine_bytes_sha256: e8b8889034e477f80bc9bb2a2cc0c3877220804facb2c40371e03acac5a9e744

00059B:  1E                           push     ds
00059C:  64 8B 3E 06 23               mov      di, word ptr fs:[0x2306]
0005A1:  8B 4D 20                     mov      cx, word ptr [di + 0x20]
0005A4:  8B 75 1C                     mov      si, word ptr [di + 0x1c]
0005A7:  C7 06 7E 22 0F 80            mov      word ptr [0x227e], 0x800f
0005AD:  C7 06 80 22 00 00            mov      word ptr [0x2280], 0
0005B3:  89 0E 7C 22                  mov      word ptr [0x227c], cx
0005B7:  64 8E 06 02 00               mov      es, word ptr fs:[2]
0005BC:  26 C7 44 12 0F 80            mov      word ptr es:[si + 0x12], 0x800f
0005C2:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
0005C8:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
0005CE:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
0005D4:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
0005D8:  66 0F AF C3                  imul     eax, ebx
0005DC:  66 8B E8                     mov      ebp, eax
0005DF:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
0005E3:  66 0F AF C1                  imul     eax, ecx
0005E7:  66 03 E8                     add      ebp, eax
0005EA:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
0005EE:  66 0F AF C2                  imul     eax, edx
0005F2:  66 03 E8                     add      ebp, eax
0005F5:  0F 88 99 00                  js       0x692
0005F9:  66 C1 FD 08                  sar      ebp, 8
0005FD:  0F 84 91 00                  je       0x692
000601:  66 A1 BA 22                  mov      eax, dword ptr [0x22ba]
000605:  66 0F AF C3                  imul     eax, ebx
000609:  66 8B F8                     mov      edi, eax
00060C:  66 A1 BE 22                  mov      eax, dword ptr [0x22be]
000610:  66 0F AF C1                  imul     eax, ecx
000614:  66 03 F8                     add      edi, eax
000617:  66 A1 C2 22                  mov      eax, dword ptr [0x22c2]
00061B:  66 0F AF C2                  imul     eax, edx
00061F:  66 03 C7                     add      eax, edi
000622:  66 50                        push     eax
000624:  66 A1 C6 22                  mov      eax, dword ptr [0x22c6]
000628:  66 0F AF C3                  imul     eax, ebx
00062C:  66 8B F8                     mov      edi, eax
00062F:  66 A1 CA 22                  mov      eax, dword ptr [0x22ca]
000633:  66 0F AF C1                  imul     eax, ecx
000637:  66 03 F8                     add      edi, eax
00063A:  66 A1 CE 22                  mov      eax, dword ptr [0x22ce]
00063E:  66 0F AF C2                  imul     eax, edx
000642:  66 03 C7                     add      eax, edi
000645:  66 99                        cdq     
000647:  66 F7 FD                     idiv     ebp
00064A:  66 8B D8                     mov      ebx, eax
00064D:  66 58                        pop      eax
00064F:  66 99                        cdq     
000651:  66 F7 FD                     idiv     ebp
000654:  33 C9                        xor      cx, cx
000656:  66 F7 DB                     neg      ebx
000659:  66 03 06 70 22               add      eax, dword ptr [0x2270]
00065E:  79 02                        jns      0x662
000660:  B1 01                        mov      cl, 1
000662:  66 3D 40 01 00 00            cmp      eax, 0x140
000668:  7C 02                        jl       0x66c
00066A:  B1 02                        mov      cl, 2
00066C:  66 03 1E 74 22               add      ebx, dword ptr [0x2274]
000671:  79 03                        jns      0x676
000673:  80 C9 04                     or       cl, 4
000676:  66 81 FB C8 00 00 00         cmp      ebx, 0xc8
00067D:  7C 03                        jl       0x682
00067F:  80 C9 08                     or       cl, 8
000682:  21 0E 7E 22                  and      word ptr [0x227e], cx
000686:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
00068A:  26 89 44 0A                  mov      word ptr es:[si + 0xa], ax
00068E:  26 89 5C 0C                  mov      word ptr es:[si + 0xc], bx
000692:  83 C6 14                     add      si, 0x14
000695:  FF 0E 7C 22                  dec      word ptr [0x227c]
000699:  0F 85 1F FF                  jne      0x5bc
00069D:  F7 06 7E 22 FF FF            test     word ptr [0x227e], 0xffff
0006A3:  0F 85 85 00                  jne      0x72c
0006A7:  64 8B 3E 06 23               mov      di, word ptr fs:[0x2306]
0006AC:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0006B1:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
0006B6:  64 8B 4D 2C                  mov      cx, word ptr fs:[di + 0x2c]
0006BA:  64 8B 75 28                  mov      si, word ptr fs:[di + 0x28]
0006BE:  8B 5C 02                     mov      bx, word ptr [si + 2]
0006C1:  8B 7C 04                     mov      di, word ptr [si + 4]
0006C4:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
0006C7:  8B 6C 06                     mov      bp, word ptr [si + 6]
0006CA:  23 45 12                     and      ax, word ptr [di + 0x12]
0006CD:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
0006D1:  75 51                        jne      0x724
0006D3:  51                           push     cx
0006D4:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
0006D7:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
0006DA:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
0006DE:  3B D1                        cmp      dx, cx
0006E0:  7E 0D                        jle      0x6ef
0006E2:  3B C1                        cmp      ax, cx
0006E4:  7C 1C                        jl       0x702
0006E6:  87 DD                        xchg     bp, bx
0006E8:  91                           xchg     cx, ax
0006E9:  87 FD                        xchg     bp, di
0006EB:  87 CA                        xchg     dx, cx
0006ED:  EB 0A                        jmp      0x6f9
0006EF:  3B C2                        cmp      ax, dx
0006F1:  7E 0F                        jle      0x702
0006F3:  87 DD                        xchg     bp, bx
0006F5:  91                           xchg     cx, ax
0006F6:  87 DF                        xchg     di, bx
0006F8:  92                           xchg     dx, ax
0006F9:  89 5C 02                     mov      word ptr [si + 2], bx
0006FC:  89 7C 04                     mov      word ptr [si + 4], di
0006FF:  89 6C 06                     mov      word ptr [si + 6], bp
000702:  2B D0                        sub      dx, ax
000704:  2B C8                        sub      cx, ax
000706:  81 FA F4 01                  cmp      dx, 0x1f4
00070A:  73 17                        jae      0x723
00070C:  81 F9 F4 01                  cmp      cx, 0x1f4
000710:  73 11                        jae      0x723
000712:  03 C0                        add      ax, ax
000714:  BF 4C 09                     mov      di, 0x94c
000717:  78 02                        js       0x71b
000719:  03 F8                        add      di, ax
00071B:  26 8B 1D                     mov      bx, word ptr es:[di]
00071E:  26 89 35                     mov      word ptr es:[di], si
000721:  89 1C                        mov      word ptr [si], bx
000723:  59                           pop      cx
000724:  83 C6 08                     add      si, 8
000727:  E2 95                        loop     0x6be
000729:  E8 46 1E                     call     0x2572
00072C:  1F                           pop      ds
00072D:  C3                           ret     
