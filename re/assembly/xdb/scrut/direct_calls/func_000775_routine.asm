; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000775
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 497
; boundary: cfg_blocks_22_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 84a6560b66ea7cf94afd831458f7417f6b59a861de9b516f913628c65061d821

000775:  1E                           push     ds
000776:  2E 8E 1E A7 33               mov      ds, word ptr cs:[0x33a7]
00077B:  64 8E 06 06 00               mov      es, word ptr fs:[6]
000780:  BE BA 22                     mov      si, 0x22ba
000783:  BF 4C 0D                     mov      di, 0xd4c
000786:  B9 09 00                     mov      cx, 9
000789:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00078C:  66 8B 1E EA 22               mov      ebx, dword ptr [0x22ea]
000791:  66 8B 0E EE 22               mov      ecx, dword ptr [0x22ee]
000796:  66 8B 16 F2 22               mov      edx, dword ptr [0x22f2]
00079B:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
0007A0:  66 C1 EB 0D                  shr      ebx, 0xd
0007A4:  66 C1 E9 0D                  shr      ecx, 0xd
0007A8:  66 C1 EA 0D                  shr      edx, 0xd
0007AC:  89 1E 7C 0D                  mov      word ptr [0xd7c], bx
0007B0:  89 0E 80 0D                  mov      word ptr [0xd80], cx
0007B4:  89 16 84 0D                  mov      word ptr [0xd84], dx
0007B8:  C7 06 DA 08 AF 04            mov      word ptr [0x8da], 0x4af
0007BE:  C7 06 DC 08 3A 1F            mov      word ptr [0x8dc], 0x1f3a
0007C4:  C7 06 DE 08 3A 25            mov      word ptr [0x8de], 0x253a
0007CA:  C7 06 E0 08 3A 2B            mov      word ptr [0x8e0], 0x2b3a
0007D0:  C7 06 E2 08 3A 31            mov      word ptr [0x8e2], 0x313a
0007D6:  66 8B 36 D6 08               mov      esi, dword ptr [0x8d6]
0007DB:  8B 1E 7C 0D                  mov      bx, word ptr [0xd7c]
0007DF:  8B 0E 80 0D                  mov      cx, word ptr [0xd80]
0007E3:  8B 16 84 0D                  mov      dx, word ptr [0xd84]
0007E7:  66 C1 CE 07                  ror      esi, 7
0007EB:  66 83 DE 00                  sbb      esi, 0
0007EF:  2B DE                        sub      bx, si
0007F1:  66 C1 CE 07                  ror      esi, 7
0007F5:  66 83 DE 00                  sbb      esi, 0
0007F9:  2B CE                        sub      cx, si
0007FB:  66 C1 CE 07                  ror      esi, 7
0007FF:  66 83 DE 00                  sbb      esi, 0
000803:  2B D6                        sub      dx, si
000805:  66 0F BF DB                  movsx    ebx, bx
000809:  66 0F BF C9                  movsx    ecx, cx
00080D:  66 0F BF D2                  movsx    edx, dx
000811:  66 A1 64 0D                  mov      eax, dword ptr [0xd64]
000815:  66 0F AF C3                  imul     eax, ebx
000819:  66 8B E8                     mov      ebp, eax
00081C:  66 A1 68 0D                  mov      eax, dword ptr [0xd68]
000820:  66 0F AF C1                  imul     eax, ecx
000824:  66 03 E8                     add      ebp, eax
000827:  66 A1 6C 0D                  mov      eax, dword ptr [0xd6c]
00082B:  66 0F AF C2                  imul     eax, edx
00082F:  66 03 E8                     add      ebp, eax
000832:  0F 88 95 00                  js       0x8cb
000836:  66 C1 FD 08                  sar      ebp, 8
00083A:  0F 84 8D 00                  je       0x8cb
00083E:  66 A1 58 0D                  mov      eax, dword ptr [0xd58]
000842:  66 0F AF C3                  imul     eax, ebx
000846:  66 8B F8                     mov      edi, eax
000849:  66 A1 5C 0D                  mov      eax, dword ptr [0xd5c]
00084D:  66 0F AF C1                  imul     eax, ecx
000851:  66 03 F8                     add      edi, eax
000854:  66 A1 60 0D                  mov      eax, dword ptr [0xd60]
000858:  66 0F AF C2                  imul     eax, edx
00085C:  66 03 C7                     add      eax, edi
00085F:  66 50                        push     eax
000861:  66 A1 4C 0D                  mov      eax, dword ptr [0xd4c]
000865:  66 0F AF C3                  imul     eax, ebx
000869:  66 8B F8                     mov      edi, eax
00086C:  66 A1 50 0D                  mov      eax, dword ptr [0xd50]
000870:  66 0F AF C1                  imul     eax, ecx
000874:  66 03 F8                     add      edi, eax
000877:  66 A1 54 0D                  mov      eax, dword ptr [0xd54]
00087B:  66 0F AF C2                  imul     eax, edx
00087F:  66 03 C7                     add      eax, edi
000882:  66 99                        cdq     
000884:  66 F7 FD                     idiv     ebp
000887:  8B F8                        mov      di, ax
000889:  66 58                        pop      eax
00088B:  81 C7 A0 00                  add      di, 0xa0
00088F:  78 3A                        js       0x8cb
000891:  81 FF 40 01                  cmp      di, 0x140
000895:  7D 34                        jge      0x8cb
000897:  66 99                        cdq     
000899:  66 F7 FD                     idiv     ebp
00089C:  F7 D8                        neg      ax
00089E:  05 64 00                     add      ax, 0x64
0008A1:  78 28                        js       0x8cb
0008A3:  3D C8 00                     cmp      ax, 0xc8
0008A6:  7D 23                        jge      0x8cb
0008A8:  8B D0                        mov      dx, ax
0008AA:  C1 E2 06                     shl      dx, 6
0008AD:  02 F0                        add      dh, al
0008AF:  03 D7                        add      dx, di
0008B1:  83 E7 03                     and      di, 3
0008B4:  C1 EA 02                     shr      dx, 2
0008B7:  03 FF                        add      di, di
0008B9:  66 C1 ED 0F                  shr      ebp, 0xf
0008BD:  8B 9D DC 08                  mov      bx, word ptr [di + 0x8dc]
0008C1:  83 85 DC 08 04               add      word ptr [di + 0x8dc], 4
0008C6:  89 17                        mov      word ptr [bx], dx
0008C8:  89 6F 02                     mov      word ptr [bx + 2], bp
0008CB:  26 FF 0E DA 08               dec      word ptr es:[0x8da]
0008D0:  0F 89 07 FF                  jns      0x7db
0008D4:  64 8E 06 28 00               mov      es, word ptr fs:[0x28]
0008D9:  BE 3A 1F                     mov      si, 0x1f3a
0008DC:  8B 2E DC 08                  mov      bp, word ptr [0x8dc]
0008E0:  BA C4 03                     mov      dx, 0x3c4
0008E3:  3B F5                        cmp      si, bp
0008E5:  73 17                        jae      0x8fe
0008E7:  B8 02 01                     mov      ax, 0x102
0008EA:  EF                           out      dx, ax
0008EB:  8B 5C 02                     mov      bx, word ptr [si + 2]
0008EE:  8B 3C                        mov      di, word ptr [si]
0008F0:  83 C6 04                     add      si, 4
0008F3:  8A 87 D6 07                  mov      al, byte ptr [bx + 0x7d6]
0008F7:  3B F5                        cmp      si, bp
0008F9:  26 88 05                     mov      byte ptr es:[di], al
0008FC:  72 ED                        jb       0x8eb
0008FE:  BE 3A 25                     mov      si, 0x253a
000901:  8B 2E DE 08                  mov      bp, word ptr [0x8de]
000905:  3B F5                        cmp      si, bp
000907:  B8 02 02                     mov      ax, 0x202
00090A:  73 14                        jae      0x920
00090C:  EF                           out      dx, ax
00090D:  8B 5C 02                     mov      bx, word ptr [si + 2]
000910:  8B 3C                        mov      di, word ptr [si]
000912:  83 C6 04                     add      si, 4
000915:  8A 87 D6 07                  mov      al, byte ptr [bx + 0x7d6]
000919:  3B F5                        cmp      si, bp
00091B:  26 88 05                     mov      byte ptr es:[di], al
00091E:  72 ED                        jb       0x90d
000920:  BE 3A 2B                     mov      si, 0x2b3a
000923:  8B 2E E0 08                  mov      bp, word ptr [0x8e0]
000927:  3B F5                        cmp      si, bp
000929:  B8 02 03                     mov      ax, 0x302
00092C:  73 14                        jae      0x942
00092E:  EF                           out      dx, ax
00092F:  8B 5C 02                     mov      bx, word ptr [si + 2]
000932:  8B 3C                        mov      di, word ptr [si]
000934:  83 C6 04                     add      si, 4
000937:  8A 87 D6 07                  mov      al, byte ptr [bx + 0x7d6]
00093B:  3B F5                        cmp      si, bp
00093D:  26 88 05                     mov      byte ptr es:[di], al
000940:  72 ED                        jb       0x92f
000942:  BE 3A 31                     mov      si, 0x313a
000945:  8B 2E E2 08                  mov      bp, word ptr [0x8e2]
000949:  3B F5                        cmp      si, bp
00094B:  B8 02 04                     mov      ax, 0x402
00094E:  73 14                        jae      0x964
000950:  EF                           out      dx, ax
000951:  8B 5C 02                     mov      bx, word ptr [si + 2]
000954:  8B 3C                        mov      di, word ptr [si]
000956:  83 C6 04                     add      si, 4
000959:  8A 87 D6 07                  mov      al, byte ptr [bx + 0x7d6]
00095D:  3B F5                        cmp      si, bp
00095F:  26 88 05                     mov      byte ptr es:[di], al
000962:  72 ED                        jb       0x951
000964:  1F                           pop      ds
000965:  C3                           ret     
