; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0005dc
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 403
; boundary: cfg_blocks_29_terminals_2
; terminal: jmp 0x73a:1, ret:1
; direct_callees: 0x002696
; indirect_calls: 0
; routine_bytes_sha256: e30b562918452a15fd9eeecf6b06ca502bd49f1223f313416b4ef2d05baf359b

0005DC:  1E                           push     ds
0005DD:  64 8B 3E 06 23               mov      di, word ptr fs:[0x2306]
0005E2:  8B 4D 20                     mov      cx, word ptr [di + 0x20]
0005E5:  8B 75 1C                     mov      si, word ptr [di + 0x1c]
0005E8:  C7 06 7E 22 0F 80            mov      word ptr [0x227e], 0x800f
0005EE:  C7 06 80 22 00 00            mov      word ptr [0x2280], 0
0005F4:  89 0E 7C 22                  mov      word ptr [0x227c], cx
0005F8:  64 8E 06 02 00               mov      es, word ptr fs:[2]
0005FD:  26 C7 44 12 0F 80            mov      word ptr es:[si + 0x12], 0x800f
000603:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
000609:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
00060F:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
000615:  66 A1 D2 22                  mov      eax, dword ptr [0x22d2]
000619:  66 0F AF C3                  imul     eax, ebx
00061D:  66 8B E8                     mov      ebp, eax
000620:  66 A1 D6 22                  mov      eax, dword ptr [0x22d6]
000624:  66 0F AF C1                  imul     eax, ecx
000628:  66 03 E8                     add      ebp, eax
00062B:  66 A1 DA 22                  mov      eax, dword ptr [0x22da]
00062F:  66 0F AF C2                  imul     eax, edx
000633:  66 03 E8                     add      ebp, eax
000636:  0F 88 99 00                  js       0x6d3
00063A:  66 C1 FD 08                  sar      ebp, 8
00063E:  0F 84 91 00                  je       0x6d3
000642:  66 A1 BA 22                  mov      eax, dword ptr [0x22ba]
000646:  66 0F AF C3                  imul     eax, ebx
00064A:  66 8B F8                     mov      edi, eax
00064D:  66 A1 BE 22                  mov      eax, dword ptr [0x22be]
000651:  66 0F AF C1                  imul     eax, ecx
000655:  66 03 F8                     add      edi, eax
000658:  66 A1 C2 22                  mov      eax, dword ptr [0x22c2]
00065C:  66 0F AF C2                  imul     eax, edx
000660:  66 03 C7                     add      eax, edi
000663:  66 50                        push     eax
000665:  66 A1 C6 22                  mov      eax, dword ptr [0x22c6]
000669:  66 0F AF C3                  imul     eax, ebx
00066D:  66 8B F8                     mov      edi, eax
000670:  66 A1 CA 22                  mov      eax, dword ptr [0x22ca]
000674:  66 0F AF C1                  imul     eax, ecx
000678:  66 03 F8                     add      edi, eax
00067B:  66 A1 CE 22                  mov      eax, dword ptr [0x22ce]
00067F:  66 0F AF C2                  imul     eax, edx
000683:  66 03 C7                     add      eax, edi
000686:  66 99                        cdq     
000688:  66 F7 FD                     idiv     ebp
00068B:  66 8B D8                     mov      ebx, eax
00068E:  66 58                        pop      eax
000690:  66 99                        cdq     
000692:  66 F7 FD                     idiv     ebp
000695:  33 C9                        xor      cx, cx
000697:  66 F7 DB                     neg      ebx
00069A:  66 03 06 70 22               add      eax, dword ptr [0x2270]
00069F:  79 02                        jns      0x6a3
0006A1:  B1 01                        mov      cl, 1
0006A3:  66 3D 40 01 00 00            cmp      eax, 0x140
0006A9:  7C 02                        jl       0x6ad
0006AB:  B1 02                        mov      cl, 2
0006AD:  66 03 1E 74 22               add      ebx, dword ptr [0x2274]
0006B2:  79 03                        jns      0x6b7
0006B4:  80 C9 04                     or       cl, 4
0006B7:  66 81 FB C8 00 00 00         cmp      ebx, 0xc8
0006BE:  7C 03                        jl       0x6c3
0006C0:  80 C9 08                     or       cl, 8
0006C3:  21 0E 7E 22                  and      word ptr [0x227e], cx
0006C7:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
0006CB:  26 89 44 0A                  mov      word ptr es:[si + 0xa], ax
0006CF:  26 89 5C 0C                  mov      word ptr es:[si + 0xc], bx
0006D3:  83 C6 14                     add      si, 0x14
0006D6:  FF 0E 7C 22                  dec      word ptr [0x227c]
0006DA:  0F 85 1F FF                  jne      0x5fd
0006DE:  F7 06 7E 22 FF FF            test     word ptr [0x227e], 0xffff
0006E4:  0F 85 85 00                  jne      0x76d
0006E8:  64 8B 3E 06 23               mov      di, word ptr fs:[0x2306]
0006ED:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0006F2:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
0006F7:  64 8B 4D 2C                  mov      cx, word ptr fs:[di + 0x2c]
0006FB:  64 8B 75 28                  mov      si, word ptr fs:[di + 0x28]
0006FF:  8B 5C 02                     mov      bx, word ptr [si + 2]
000702:  8B 7C 04                     mov      di, word ptr [si + 4]
000705:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
000708:  8B 6C 06                     mov      bp, word ptr [si + 6]
00070B:  23 45 12                     and      ax, word ptr [di + 0x12]
00070E:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
000712:  75 51                        jne      0x765
000714:  51                           push     cx
000715:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
000718:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
00071B:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00071F:  3B D1                        cmp      dx, cx
000721:  7E 0D                        jle      0x730
000723:  3B C1                        cmp      ax, cx
000725:  7C 1C                        jl       0x743
000727:  87 DD                        xchg     bp, bx
000729:  91                           xchg     cx, ax
00072A:  87 FD                        xchg     bp, di
00072C:  87 CA                        xchg     dx, cx
00072E:  EB 0A                        jmp      0x73a
000730:  3B C2                        cmp      ax, dx
000732:  7E 0F                        jle      0x743
000734:  87 DD                        xchg     bp, bx
000736:  91                           xchg     cx, ax
000737:  87 DF                        xchg     di, bx
000739:  92                           xchg     dx, ax
00073A:  89 5C 02                     mov      word ptr [si + 2], bx
00073D:  89 7C 04                     mov      word ptr [si + 4], di
000740:  89 6C 06                     mov      word ptr [si + 6], bp
000743:  2B D0                        sub      dx, ax
000745:  2B C8                        sub      cx, ax
000747:  81 FA F4 01                  cmp      dx, 0x1f4
00074B:  73 17                        jae      0x764
00074D:  81 F9 F4 01                  cmp      cx, 0x1f4
000751:  73 11                        jae      0x764
000753:  03 C0                        add      ax, ax
000755:  BF 4E 09                     mov      di, 0x94e
000758:  78 02                        js       0x75c
00075A:  03 F8                        add      di, ax
00075C:  26 8B 1D                     mov      bx, word ptr es:[di]
00075F:  26 89 35                     mov      word ptr es:[di], si
000762:  89 1C                        mov      word ptr [si], bx
000764:  59                           pop      cx
000765:  83 C6 08                     add      si, 8
000768:  E2 95                        loop     0x6ff
00076A:  E8 29 1F                     call     0x2696
00076D:  1F                           pop      ds
00076E:  C3                           ret     
