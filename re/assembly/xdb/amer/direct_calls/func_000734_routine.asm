; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000734
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 497
; boundary: cfg_blocks_22_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: c20927be684fe47460ff868324a7f228fa927e1b00a4a61d8770bb700343e601

000734:  1E                           push     ds
000735:  2E 8E 1E 77 32               mov      ds, word ptr cs:[0x3277]
00073A:  64 8E 06 06 00               mov      es, word ptr fs:[6]
00073F:  BE BA 22                     mov      si, 0x22ba
000742:  BF 4A 0D                     mov      di, 0xd4a
000745:  B9 09 00                     mov      cx, 9
000748:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00074B:  66 8B 1E EA 22               mov      ebx, dword ptr [0x22ea]
000750:  66 8B 0E EE 22               mov      ecx, dword ptr [0x22ee]
000755:  66 8B 16 F2 22               mov      edx, dword ptr [0x22f2]
00075A:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
00075F:  66 C1 EB 0D                  shr      ebx, 0xd
000763:  66 C1 E9 0D                  shr      ecx, 0xd
000767:  66 C1 EA 0D                  shr      edx, 0xd
00076B:  89 1E 7A 0D                  mov      word ptr [0xd7a], bx
00076F:  89 0E 7E 0D                  mov      word ptr [0xd7e], cx
000773:  89 16 82 0D                  mov      word ptr [0xd82], dx
000777:  C7 06 D8 08 AF 04            mov      word ptr [0x8d8], 0x4af
00077D:  C7 06 DA 08 38 1F            mov      word ptr [0x8da], 0x1f38
000783:  C7 06 DC 08 38 25            mov      word ptr [0x8dc], 0x2538
000789:  C7 06 DE 08 38 2B            mov      word ptr [0x8de], 0x2b38
00078F:  C7 06 E0 08 38 31            mov      word ptr [0x8e0], 0x3138
000795:  66 8B 36 D4 08               mov      esi, dword ptr [0x8d4]
00079A:  8B 1E 7A 0D                  mov      bx, word ptr [0xd7a]
00079E:  8B 0E 7E 0D                  mov      cx, word ptr [0xd7e]
0007A2:  8B 16 82 0D                  mov      dx, word ptr [0xd82]
0007A6:  66 C1 CE 07                  ror      esi, 7
0007AA:  66 83 DE 00                  sbb      esi, 0
0007AE:  2B DE                        sub      bx, si
0007B0:  66 C1 CE 07                  ror      esi, 7
0007B4:  66 83 DE 00                  sbb      esi, 0
0007B8:  2B CE                        sub      cx, si
0007BA:  66 C1 CE 07                  ror      esi, 7
0007BE:  66 83 DE 00                  sbb      esi, 0
0007C2:  2B D6                        sub      dx, si
0007C4:  66 0F BF DB                  movsx    ebx, bx
0007C8:  66 0F BF C9                  movsx    ecx, cx
0007CC:  66 0F BF D2                  movsx    edx, dx
0007D0:  66 A1 62 0D                  mov      eax, dword ptr [0xd62]
0007D4:  66 0F AF C3                  imul     eax, ebx
0007D8:  66 8B E8                     mov      ebp, eax
0007DB:  66 A1 66 0D                  mov      eax, dword ptr [0xd66]
0007DF:  66 0F AF C1                  imul     eax, ecx
0007E3:  66 03 E8                     add      ebp, eax
0007E6:  66 A1 6A 0D                  mov      eax, dword ptr [0xd6a]
0007EA:  66 0F AF C2                  imul     eax, edx
0007EE:  66 03 E8                     add      ebp, eax
0007F1:  0F 88 95 00                  js       0x88a
0007F5:  66 C1 FD 08                  sar      ebp, 8
0007F9:  0F 84 8D 00                  je       0x88a
0007FD:  66 A1 56 0D                  mov      eax, dword ptr [0xd56]
000801:  66 0F AF C3                  imul     eax, ebx
000805:  66 8B F8                     mov      edi, eax
000808:  66 A1 5A 0D                  mov      eax, dword ptr [0xd5a]
00080C:  66 0F AF C1                  imul     eax, ecx
000810:  66 03 F8                     add      edi, eax
000813:  66 A1 5E 0D                  mov      eax, dword ptr [0xd5e]
000817:  66 0F AF C2                  imul     eax, edx
00081B:  66 03 C7                     add      eax, edi
00081E:  66 50                        push     eax
000820:  66 A1 4A 0D                  mov      eax, dword ptr [0xd4a]
000824:  66 0F AF C3                  imul     eax, ebx
000828:  66 8B F8                     mov      edi, eax
00082B:  66 A1 4E 0D                  mov      eax, dword ptr [0xd4e]
00082F:  66 0F AF C1                  imul     eax, ecx
000833:  66 03 F8                     add      edi, eax
000836:  66 A1 52 0D                  mov      eax, dword ptr [0xd52]
00083A:  66 0F AF C2                  imul     eax, edx
00083E:  66 03 C7                     add      eax, edi
000841:  66 99                        cdq     
000843:  66 F7 FD                     idiv     ebp
000846:  8B F8                        mov      di, ax
000848:  66 58                        pop      eax
00084A:  81 C7 A0 00                  add      di, 0xa0
00084E:  78 3A                        js       0x88a
000850:  81 FF 40 01                  cmp      di, 0x140
000854:  7D 34                        jge      0x88a
000856:  66 99                        cdq     
000858:  66 F7 FD                     idiv     ebp
00085B:  F7 D8                        neg      ax
00085D:  05 64 00                     add      ax, 0x64
000860:  78 28                        js       0x88a
000862:  3D C8 00                     cmp      ax, 0xc8
000865:  7D 23                        jge      0x88a
000867:  8B D0                        mov      dx, ax
000869:  C1 E2 06                     shl      dx, 6
00086C:  02 F0                        add      dh, al
00086E:  03 D7                        add      dx, di
000870:  83 E7 03                     and      di, 3
000873:  C1 EA 02                     shr      dx, 2
000876:  03 FF                        add      di, di
000878:  66 C1 ED 0F                  shr      ebp, 0xf
00087C:  8B 9D DA 08                  mov      bx, word ptr [di + 0x8da]
000880:  83 85 DA 08 04               add      word ptr [di + 0x8da], 4
000885:  89 17                        mov      word ptr [bx], dx
000887:  89 6F 02                     mov      word ptr [bx + 2], bp
00088A:  26 FF 0E D8 08               dec      word ptr es:[0x8d8]
00088F:  0F 89 07 FF                  jns      0x79a
000893:  64 8E 06 28 00               mov      es, word ptr fs:[0x28]
000898:  BE 38 1F                     mov      si, 0x1f38
00089B:  8B 2E DA 08                  mov      bp, word ptr [0x8da]
00089F:  BA C4 03                     mov      dx, 0x3c4
0008A2:  3B F5                        cmp      si, bp
0008A4:  73 17                        jae      0x8bd
0008A6:  B8 02 01                     mov      ax, 0x102
0008A9:  EF                           out      dx, ax
0008AA:  8B 5C 02                     mov      bx, word ptr [si + 2]
0008AD:  8B 3C                        mov      di, word ptr [si]
0008AF:  83 C6 04                     add      si, 4
0008B2:  8A 87 D4 07                  mov      al, byte ptr [bx + 0x7d4]
0008B6:  3B F5                        cmp      si, bp
0008B8:  26 88 05                     mov      byte ptr es:[di], al
0008BB:  72 ED                        jb       0x8aa
0008BD:  BE 38 25                     mov      si, 0x2538
0008C0:  8B 2E DC 08                  mov      bp, word ptr [0x8dc]
0008C4:  3B F5                        cmp      si, bp
0008C6:  B8 02 02                     mov      ax, 0x202
0008C9:  73 14                        jae      0x8df
0008CB:  EF                           out      dx, ax
0008CC:  8B 5C 02                     mov      bx, word ptr [si + 2]
0008CF:  8B 3C                        mov      di, word ptr [si]
0008D1:  83 C6 04                     add      si, 4
0008D4:  8A 87 D4 07                  mov      al, byte ptr [bx + 0x7d4]
0008D8:  3B F5                        cmp      si, bp
0008DA:  26 88 05                     mov      byte ptr es:[di], al
0008DD:  72 ED                        jb       0x8cc
0008DF:  BE 38 2B                     mov      si, 0x2b38
0008E2:  8B 2E DE 08                  mov      bp, word ptr [0x8de]
0008E6:  3B F5                        cmp      si, bp
0008E8:  B8 02 03                     mov      ax, 0x302
0008EB:  73 14                        jae      0x901
0008ED:  EF                           out      dx, ax
0008EE:  8B 5C 02                     mov      bx, word ptr [si + 2]
0008F1:  8B 3C                        mov      di, word ptr [si]
0008F3:  83 C6 04                     add      si, 4
0008F6:  8A 87 D4 07                  mov      al, byte ptr [bx + 0x7d4]
0008FA:  3B F5                        cmp      si, bp
0008FC:  26 88 05                     mov      byte ptr es:[di], al
0008FF:  72 ED                        jb       0x8ee
000901:  BE 38 31                     mov      si, 0x3138
000904:  8B 2E E0 08                  mov      bp, word ptr [0x8e0]
000908:  3B F5                        cmp      si, bp
00090A:  B8 02 04                     mov      ax, 0x402
00090D:  73 14                        jae      0x923
00090F:  EF                           out      dx, ax
000910:  8B 5C 02                     mov      bx, word ptr [si + 2]
000913:  8B 3C                        mov      di, word ptr [si]
000915:  83 C6 04                     add      si, 4
000918:  8A 87 D4 07                  mov      al, byte ptr [bx + 0x7d4]
00091C:  3B F5                        cmp      si, bp
00091E:  26 88 05                     mov      byte ptr es:[di], al
000921:  72 ED                        jb       0x910
000923:  1F                           pop      ds
000924:  C3                           ret     
