; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x002696
; group: direct_calls
; provenance: direct_call_from_0x5dc, reviewed_contiguous_renderer_owner
; byte_count: 1543
; boundary: reviewed_contiguous_owner_internal_dispatch
; terminal: ret:1
; direct_callees: 0x002c9d
; indirect_calls: 0
; routine_bytes_sha256: 1e798885570673359062749747c6271d91cb1de8f3d4ab9cbc5fa1541de9eeeb

002696:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
00269B:  FC                            cld
00269C:  64 8E 06 06 00                mov      es, word ptr fs:[6]
0026A1:  BB 3A 0D                      mov      bx, 0xd3a
0026A4:  89 1E D0 0B                   mov      word ptr [0xbd0], bx
0026A8:  B9 58 02                      mov      cx, 0x258
0026AB:  8B F3                         mov      si, bx
0026AD:  83 C3 5A                      add      bx, 0x5a
0026B0:  89 1C                         mov      word ptr [si], bx
0026B2:  E2 F7                         loop     0x26ab
0026B4:  C7 04 00 00                   mov      word ptr [si], 0
0026B8:  C7 06 4C 09 4E 09             mov      word ptr [0x94c], 0x94e
0026BE:  C7 06 48 09 00 00             mov      word ptr [0x948], 0
0026C4:  B9 40 01                      mov      cx, 0x140
0026C7:  8B 3E 4C 09                   mov      di, word ptr [0x94c]
0026CB:  2B 0E 48 09                   sub      cx, word ptr [0x948]
0026CF:  33 C0                         xor      ax, ax
0026D1:  F3 AF                         repe scasw ax, word ptr es:[di]
0026D3:  0F 84 92 00                   je       0x2769
0026D7:  BE 2C 0C                      mov      si, 0xc2c
0026DA:  BB 86 0C                      mov      bx, 0xc86
0026DD:  89 1C                         mov      word ptr [si], bx
0026DF:  C7 44 2E 4A 01                mov      word ptr [si + 0x2e], 0x14a
0026E4:  66 C7 44 08 00 00 00 80       mov      dword ptr [si + 8], 0x80000000
0026EC:  66 C7 44 18 00 00 00 00       mov      dword ptr [si + 0x18], 0
0026F4:  66 C7 44 0C 00 00 00 00       mov      dword ptr [si + 0xc], 0
0026FC:  66 C7 44 1C 00 00 00 00       mov      dword ptr [si + 0x1c], 0
002704:  89 77 10                      mov      word ptr [bx + 0x10], si
002707:  C7 07 E0 0C                   mov      word ptr [bx], 0xce0
00270B:  C7 47 2E 40 01                mov      word ptr [bx + 0x2e], 0x140
002710:  66 C7 47 08 00 00 00 00       mov      dword ptr [bx + 8], 0
002718:  66 C7 47 18 00 00 00 00       mov      dword ptr [bx + 0x18], 0
002720:  C7 47 0A C8 00                mov      word ptr [bx + 0xa], 0xc8
002725:  66 C7 47 18 00 00 FF 7F       mov      dword ptr [bx + 0x18], 0x7fff0000
00272D:  C7 47 02 00 80                mov      word ptr [bx + 2], 0x8000
002732:  BE E0 0C                      mov      si, 0xce0
002735:  66 C7 44 08 FF FF FF 7F       mov      dword ptr [si + 8], 0x7fffffff
00273D:  66 C7 44 18 FF FF FF 7F       mov      dword ptr [si + 0x18], 0x7fffffff
002745:  89 1E F0 0C                   mov      word ptr [0xcf0], bx
002749:  C7 06 0C 0D 94 27             mov      word ptr [0xd0c], 0x2794
00274F:  C7 06 0E 0D FF FF             mov      word ptr [0xd0e], 0xffff
002755:  BB 3F 01                      mov      bx, 0x13f
002758:  83 EF 02                      sub      di, 2
00275B:  2B D9                         sub      bx, cx
00275D:  89 3E 4C 09                   mov      word ptr [0x94c], di
002761:  89 1E 48 09                   mov      word ptr [0x948], bx
002765:  8B 35                         mov      si, word ptr [di]
002767:  EB 6D                         jmp      0x27d6
; -- internal owner block: shared return --
002769:  C3                            ret
; -- internal owner block: active-list insertion --
00276A:  56                            push     si
00276B:  8B 1D                         mov      bx, word ptr [di]
00276D:  89 1C                         mov      word ptr [si], bx
00276F:  89 77 10                      mov      word ptr [bx + 0x10], si
002772:  66 8B 45 08                   mov      eax, dword ptr [di + 8]
002776:  8B 74 10                      mov      si, word ptr [si + 0x10]
002779:  81 FE 2C 0C                   cmp      si, 0xc2c
00277D:  74 06                         je       0x2785
00277F:  66 3B 44 08                   cmp      eax, dword ptr [si + 8]
002783:  7C F1                         jl       0x2776
002785:  8B 1C                         mov      bx, word ptr [si]
002787:  89 3C                         mov      word ptr [si], di
002789:  89 1D                         mov      word ptr [di], bx
00278B:  89 75 10                      mov      word ptr [di + 0x10], si
00278E:  89 7F 10                      mov      word ptr [bx + 0x10], di
002791:  5E                            pop      si
002792:  EB 11                         jmp      0x27a5
; -- internal owner block: next-column continuation --
002794:  A1 48 09                      mov      ax, word ptr [0x948]
002797:  40                            inc      ax
002798:  3D 40 01                      cmp      ax, 0x140
00279B:  73 CC                         jae      0x2769
00279D:  A3 48 09                      mov      word ptr [0x948], ax
0027A0:  BB 2C 0C                      mov      bx, 0xc2c
0027A3:  8B 37                         mov      si, word ptr [bx]
0027A5:  8B 3C                         mov      di, word ptr [si]
0027A7:  81 FF E0 0C                   cmp      di, 0xce0
0027AB:  74 18                         je       0x27c5
0027AD:  66 8B 44 08                   mov      eax, dword ptr [si + 8]
0027B1:  66 8B 4C 18                   mov      ecx, dword ptr [si + 0x18]
0027B5:  66 3B 45 08                   cmp      eax, dword ptr [di + 8]
0027B9:  7F AF                         jg       0x276a
0027BB:  8B F7                         mov      si, di
0027BD:  8B 3D                         mov      di, word ptr [di]
0027BF:  81 FF E0 0C                   cmp      di, 0xce0
0027C3:  75 E8                         jne      0x27ad
0027C5:  8B 3E 4C 09                   mov      di, word ptr [0x94c]
0027C9:  83 C7 02                      add      di, 2
0027CC:  89 3E 4C 09                   mov      word ptr [0x94c], di
0027D0:  8B 35                         mov      si, word ptr [di]
0027D2:  0B F6                         or       si, si
0027D4:  74 1C                         je       0x27f2
0027D6:  C7 05 00 00                   mov      word ptr [di], 0
0027DA:  F7 06 D0 0B FF FF             test     word ptr [0xbd0], 0xffff
0027E0:  74 10                         je       0x27f2
0027E2:  64 8E 06 02 00                mov      es, word ptr fs:[2]
0027E7:  26 FF 34                      push     word ptr es:[si]
0027EA:  E8 B0 04                      call     0x2c9d
0027ED:  5E                            pop      si
0027EE:  0B F6                         or       si, si
0027F0:  75 F5                         jne      0x27e7
0027F2:  BE 2C 0C                      mov      si, 0xc2c
0027F5:  8B 04                         mov      ax, word ptr [si]
0027F7:  3D 86 0C                      cmp      ax, 0xc86
0027FA:  0F 84 B3 03                   je       0x2bb1
0027FE:  BA 3C 0C                      mov      dx, 0xc3c
002801:  C7 44 02 01 00                mov      word ptr [si + 2], 1
002806:  89 54 06                      mov      word ptr [si + 6], dx
002809:  8B FE                         mov      di, si
00280B:  8B EE                         mov      bp, si
00280D:  33 DB                         xor      bx, bx
00280F:  8B 3D                         mov      di, word ptr [di]
002811:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
002816:  75 F7                         jne      0x280f
002818:  89 5C 58                      mov      word ptr [si + 0x58], bx
00281B:  89 5D 58                      mov      word ptr [di + 0x58], bx
00281E:  3B 5D 0A                      cmp      bx, word ptr [di + 0xa]
002821:  0F 8E A1 00                   jle      0x28c6
002825:  8B F7                         mov      si, di
002827:  8B EA                         mov      bp, dx
002829:  89 3E 2A 0C                   mov      word ptr [0xc2a], di
00282D:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
002832:  F7 D8                         neg      ax
002834:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
002838:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
00283C:  66 89 45 04                   mov      dword ptr [di + 4], eax
002840:  EB 2F                         jmp      0x2871
002842:  BB D2 0B                      mov      bx, 0xbd2
002845:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
00284A:  F7 D8                         neg      ax
00284C:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
002850:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
002854:  66 89 45 04                   mov      dword ptr [di + 4], eax
002858:  66 3B 44 04                   cmp      eax, dword ptr [si + 4]
00285C:  7E 09                         jle      0x2867
00285E:  8B DE                         mov      bx, si
002860:  8B 74 58                      mov      si, word ptr [si + 0x58]
002863:  0B F6                         or       si, si
002865:  75 F1                         jne      0x2858
002867:  89 7F 58                      mov      word ptr [bx + 0x58], di
00286A:  89 75 58                      mov      word ptr [di + 0x58], si
00286D:  8B 36 2A 0C                   mov      si, word ptr [0xc2a]
002871:  8B 3D                         mov      di, word ptr [di]
002873:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
002878:  75 F7                         jne      0x2871
00287A:  F7 45 0A 00 80                test     word ptr [di + 0xa], 0x8000
00287F:  75 C1                         jne      0x2842
002881:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002887:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
00288B:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
00288F:  8B 44 1A                      mov      ax, word ptr [si + 0x1a]
002892:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
002895:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
002898:  7D 55                         jge      0x28ef
00289A:  3B C2                         cmp      ax, dx
00289C:  0F 8F B1 00                   jg       0x2951
0028A0:  74 72                         je       0x2914
0028A2:  8D 54 10                      lea      dx, [si + 0x10]
0028A5:  8B 4C 1A                      mov      cx, word ptr [si + 0x1a]
0028A8:  3E 89 56 06                   mov      word ptr ds:[bp + 6], dx
0028AC:  8B EA                         mov      bp, dx
0028AE:  8B 74 58                      mov      si, word ptr [si + 0x58]
0028B1:  0B F6                         or       si, si
0028B3:  74 11                         je       0x28c6
0028B5:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
0028B8:  7D F4                         jge      0x28ae
0028BA:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0028C0:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
0028C4:  EB C5                         jmp      0x288b
0028C6:  81 FF 86 0C                   cmp      di, 0xc86
0028CA:  0F 84 1B 02                   je       0x2ae9
0028CE:  3E C7 46 02 01 00             mov      word ptr ds:[bp + 2], 1
0028D4:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0028D8:  8B EF                         mov      bp, di
0028DA:  C7 45 58 00 00                mov      word ptr [di + 0x58], 0
0028DF:  8B F7                         mov      si, di
0028E1:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0028E7:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
0028EB:  8B 3D                         mov      di, word ptr [di]
0028ED:  EB 9C                         jmp      0x288b
0028EF:  8B 3D                         mov      di, word ptr [di]
0028F1:  EB 98                         jmp      0x288b
0028F3:  8B 5C 58                      mov      bx, word ptr [si + 0x58]
0028F6:  0B DB                         or       bx, bx
0028F8:  75 1A                         jne      0x2914
0028FA:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0028FE:  8B EF                         mov      bp, di
002900:  8B F7                         mov      si, di
002902:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002908:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
00290C:  89 5C 58                      mov      word ptr [si + 0x58], bx
00290F:  8B 3C                         mov      di, word ptr [si]
002911:  E9 77 FF                      jmp      0x288b
002914:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
002918:  8B 4D 0A                      mov      cx, word ptr [di + 0xa]
00291B:  8B EF                         mov      bp, di
00291D:  81 FF 86 0C                   cmp      di, 0xc86
002921:  0F 84 C4 01                   je       0x2ae9
002925:  8B 74 58                      mov      si, word ptr [si + 0x58]
002928:  0B F6                         or       si, si
00292A:  74 11                         je       0x293d
00292C:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
00292F:  7D F4                         jge      0x2925
002931:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002937:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
00293B:  EB 14                         jmp      0x2951
00293D:  89 75 58                      mov      word ptr [di + 0x58], si
002940:  8B F7                         mov      si, di
002942:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002948:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
00294C:  8B 3D                         mov      di, word ptr [di]
00294E:  E9 3A FF                      jmp      0x288b
002951:  81 FF 86 0C                   cmp      di, 0xc86
002955:  0F 84 8A 01                   je       0x2ae3
002959:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
00295C:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
00295F:  7D 49                         jge      0x29aa
002961:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
002965:  BB D2 0B                      mov      bx, 0xbd2
002968:  89 77 58                      mov      word ptr [bx + 0x58], si
00296B:  66 8B 4D 08                   mov      ecx, dword ptr [di + 8]
00296F:  66 3B 4C 18                   cmp      ecx, dword ptr [si + 0x18]
002973:  7C 0C                         jl       0x2981
002975:  8B 74 58                      mov      si, word ptr [si + 0x58]
002978:  0B F6                         or       si, si
00297A:  89 77 58                      mov      word ptr [bx + 0x58], si
00297D:  75 F0                         jne      0x296f
00297F:  EB 23                         jmp      0x29a4
002981:  66 8B C1                      mov      eax, ecx
002984:  66 2B 44 08                   sub      eax, dword ptr [si + 8]
002988:  66 F7 6C 28                   imul     dword ptr [si + 0x28]
00298C:  66 0F AC D0 10                shrd     eax, edx, 0x10
002991:  66 03 44 20                   add      eax, dword ptr [si + 0x20]
002995:  66 3B 45 20                   cmp      eax, dword ptr [di + 0x20]
002999:  7D 09                         jge      0x29a4
00299B:  8B DE                         mov      bx, si
00299D:  8B 74 58                      mov      si, word ptr [si + 0x58]
0029A0:  0B F6                         or       si, si
0029A2:  75 CB                         jne      0x296f
0029A4:  89 7F 58                      mov      word ptr [bx + 0x58], di
0029A7:  89 75 58                      mov      word ptr [di + 0x58], si
0029AA:  8B 36 2A 0C                   mov      si, word ptr [0xc2a]
0029AE:  3B F7                         cmp      si, di
0029B0:  75 0E                         jne      0x29c0
0029B2:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0029B6:  8B EF                         mov      bp, di
0029B8:  C7 45 02 00 00                mov      word ptr [di + 2], 0
0029BD:  89 7D 04                      mov      word ptr [di + 4], di
0029C0:  8B 3D                         mov      di, word ptr [di]
0029C2:  E9 C6 FE                      jmp      0x288b
; -- internal owner block: four-plane renderer --
0029C5:  8B 3E 48 09                   mov      di, word ptr [0x948]
0029C9:  8B CF                         mov      cx, di
0029CB:  83 E1 03                      and      cx, 3
0029CE:  0F 85 DF 01                   jne      0x2bb1
0029D2:  64 8E 06 28 00                mov      es, word ptr fs:[0x28]
0029D7:  BA C4 03                      mov      dx, 0x3c4
0029DA:  B8 02 0F                      mov      ax, 0xf02
0029DD:  C1 EF 02                      shr      di, 2
0029E0:  89 3E 4A 09                   mov      word ptr [0x94a], di
0029E4:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0029E9:  EF                            out      dx, ax
0029EA:  0F 88 C3 01                   js       0x2bb1
0029EE:  74 5D                         je       0x2a4d
0029F0:  EB 3C                         jmp      0x2a2e
0029F2:  87 DB                         xchg     bx, bx
0029F4:  87 DB                         xchg     bx, bx
0029F6:  87 DB                         xchg     bx, bx
0029F8:  87 DB                         xchg     bx, bx
0029FA:  87 DB                         xchg     bx, bx
0029FC:  87 DB                         xchg     bx, bx
0029FE:  87 DB                         xchg     bx, bx
; -- internal owner block: Mode-X renderer --
002A00:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A05:  0F 88 A8 01                   js       0x2bb1
002A09:  64 8E 06 28 00                mov      es, word ptr fs:[0x28]
002A0E:  BA C4 03                      mov      dx, 0x3c4
002A11:  8B 3E 48 09                   mov      di, word ptr [0x948]
002A15:  B8 02 01                      mov      ax, 0x102
002A18:  8B CF                         mov      cx, di
002A1A:  C1 EF 02                      shr      di, 2
002A1D:  83 E1 03                      and      cx, 3
002A20:  89 3E 4A 09                   mov      word ptr [0x94a], di
002A24:  D2 E4                         shl      ah, cl
002A26:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A2B:  EF                            out      dx, ax
002A2C:  74 1F                         je       0x2a4d
002A2E:  8B 3E 4A 09                   mov      di, word ptr [0x94a]
002A32:  8B 5F 06                      mov      bx, word ptr [bx + 6]
002A35:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002A38:  C1 E0 04                      shl      ax, 4
002A3B:  03 F8                         add      di, ax
002A3D:  C1 E0 02                      shl      ax, 2
002A40:  03 F8                         add      di, ax
002A42:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A47:  0F 88 66 01                   js       0x2bb1
002A4B:  75 E1                         jne      0x2a2e
002A4D:  8B 77 06                      mov      si, word ptr [bx + 6]
002A50:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002A53:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
002A56:  2B C8                         sub      cx, ax
002A58:  56                            push     si
002A59:  7E 33                         jle      0x2a8e
002A5B:  8B 77 04                      mov      si, word ptr [bx + 4]
002A5E:  90                            nop
002A5F:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
002A62:  75 3C                         jne      0x2aa0
002A64:  8B 44 42                      mov      ax, word ptr [si + 0x42]
002A67:  8A DC                         mov      bl, ah
002A69:  8B 54 44                      mov      dx, word ptr [si + 0x44]
002A6C:  8A FE                         mov      bh, dh
002A6E:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002A71:  90                            nop
002A72:  C5 74 54                      lds      si, ptr [si + 0x54]
002A75:  03 C5                         add      ax, bp
002A77:  8A 2F                         mov      ch, byte ptr [bx]
002A79:  03 D6                         add      dx, si
002A7B:  8A DC                         mov      bl, ah
002A7D:  26 88 2D                      mov      byte ptr es:[di], ch
002A80:  83 C7 50                      add      di, 0x50
002A83:  FE C9                         dec      cl
002A85:  8A FE                         mov      bh, dh
002A87:  75 EC                         jne      0x2a75
002A89:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002A8E:  5B                            pop      bx
002A8F:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A94:  74 B7                         je       0x2a4d
002A96:  79 96                         jns      0x2a2e
002A98:  E9 16 01                      jmp      0x2bb1
002A9B:  87 DB                         xchg     bx, bx
002A9D:  87 DB                         xchg     bx, bx
002A9F:  90                            nop
002AA0:  8B 54 54                      mov      dx, word ptr [si + 0x54]
002AA3:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002AA6:  0F AF D0                      imul     dx, ax
002AA9:  0F AF C5                      imul     ax, bp
002AAC:  03 54 44                      add      dx, word ptr [si + 0x44]
002AAF:  03 44 42                      add      ax, word ptr [si + 0x42]
002AB2:  8A FE                         mov      bh, dh
002AB4:  8A DC                         mov      bl, ah
002AB6:  C5 74 54                      lds      si, ptr [si + 0x54]
002AB9:  8A 2F                         mov      ch, byte ptr [bx]
002ABB:  03 C5                         add      ax, bp
002ABD:  26 88 2D                      mov      byte ptr es:[di], ch
002AC0:  03 D6                         add      dx, si
002AC2:  83 C7 50                      add      di, 0x50
002AC5:  FE C9                         dec      cl
002AC7:  8A DC                         mov      bl, ah
002AC9:  8A FE                         mov      bh, dh
002ACB:  75 EC                         jne      0x2ab9
002ACD:  5B                            pop      bx
002ACE:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002AD3:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002AD8:  0F 84 71 FF                   je       0x2a4d
002ADC:  0F 89 4E FF                   jns      0x2a2e
002AE0:  E9 CE 00                      jmp      0x2bb1
002AE3:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
002AE7:  8B EF                         mov      bp, di
002AE9:  3E C7 46 02 00 80             mov      word ptr ds:[bp + 2], 0x8000
002AEF:  BB 2C 0C                      mov      bx, 0xc2c
002AF2:  FF 26 46 09                   jmp      word ptr [0x946]
; -- internal owner block: linear renderer --
002AF6:  64 8E 06 24 00                mov      es, word ptr fs:[0x24]
002AFB:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002B00:  0F 88 AD 00                   js       0x2bb1
002B04:  74 1E                         je       0x2b24
002B06:  8B 3E 48 09                   mov      di, word ptr [0x948]
002B0A:  8B 5F 06                      mov      bx, word ptr [bx + 6]
002B0D:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002B10:  8B C8                         mov      cx, ax
002B12:  C1 E0 06                      shl      ax, 6
002B15:  02 E1                         add      ah, cl
002B17:  03 F8                         add      di, ax
002B19:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002B1E:  0F 88 8F 00                   js       0x2bb1
002B22:  75 E2                         jne      0x2b06
002B24:  8B 77 06                      mov      si, word ptr [bx + 6]
002B27:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002B2A:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
002B2D:  2B C8                         sub      cx, ax
002B2F:  56                            push     si
002B30:  7E 32                         jle      0x2b64
002B32:  8B 77 04                      mov      si, word ptr [bx + 4]
002B35:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
002B38:  75 36                         jne      0x2b70
002B3A:  8B 44 42                      mov      ax, word ptr [si + 0x42]
002B3D:  8A DC                         mov      bl, ah
002B3F:  8B 54 44                      mov      dx, word ptr [si + 0x44]
002B42:  8A FE                         mov      bh, dh
002B44:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002B47:  C5 74 54                      lds      si, ptr [si + 0x54]
; -- internal owner block: affine fill loop --
002B4A:  03 C5                         add      ax, bp
002B4C:  03 D6                         add      dx, si
002B4E:  8A 2F                         mov      ch, byte ptr [bx]
002B50:  8A DC                         mov      bl, ah
002B52:  26 88 2D                      mov      byte ptr es:[di], ch
002B55:  81 C7 40 01                   add      di, 0x140
002B59:  FE C9                         dec      cl
002B5B:  8A FE                         mov      bh, dh
002B5D:  75 EB                         jne      0x2b4a
002B5F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002B64:  5B                            pop      bx
002B65:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002B6A:  74 B8                         je       0x2b24
002B6C:  79 98                         jns      0x2b06
002B6E:  EB 41                         jmp      0x2bb1
002B70:  8B 54 54                      mov      dx, word ptr [si + 0x54]
002B73:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002B76:  0F AF D0                      imul     dx, ax
002B79:  0F AF C5                      imul     ax, bp
002B7C:  03 54 44                      add      dx, word ptr [si + 0x44]
002B7F:  03 44 42                      add      ax, word ptr [si + 0x42]
002B82:  8A FE                         mov      bh, dh
002B84:  8A DC                         mov      bl, ah
002B86:  C5 74 54                      lds      si, ptr [si + 0x54]
002B89:  8A 2F                         mov      ch, byte ptr [bx]
002B8B:  03 C5                         add      ax, bp
002B8D:  26 88 2D                      mov      byte ptr es:[di], ch
002B90:  03 D6                         add      dx, si
002B92:  81 C7 40 01                   add      di, 0x140
002B96:  FE C9                         dec      cl
002B98:  8A DC                         mov      bl, ah
002B9A:  8A FE                         mov      bh, dh
002B9C:  75 EB                         jne      0x2b89
002B9E:  5B                            pop      bx
002B9F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002BA4:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002BA9:  0F 84 77 FF                   je       0x2b24
002BAD:  0F 89 55 FF                   jns      0x2b06
; -- internal owner block: record advance --
002BB1:  8B 36 2C 0C                   mov      si, word ptr [0xc2c]
002BB5:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002BB8:  78 2D                         js       0x2be7
002BBA:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
002BBD:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
002BC0:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
002BC4:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
002BC8:  01 44 42                      add      word ptr [si + 0x42], ax
002BCB:  01 5C 44                      add      word ptr [si + 0x44], bx
002BCE:  66 01 4C 08                   add      dword ptr [si + 8], ecx
002BD2:  66 01 54 20                   add      dword ptr [si + 0x20], edx
002BD6:  8B DE                         mov      bx, si
002BD8:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
002BDC:  8B 37                         mov      si, word ptr [bx]
002BDE:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
002BE2:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002BE5:  79 D3                         jns      0x2bba
002BE7:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: secondary-left transition --
002BEA:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
002BEE:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
002BF2:  66 8B 54 46                   mov      edx, dword ptr [si + 0x46]
002BF6:  66 8B 7C 4E                   mov      edi, dword ptr [si + 0x4e]
002BFA:  66 89 44 08                   mov      dword ptr [si + 8], eax
002BFE:  66 89 4C 0C                   mov      dword ptr [si + 0xc], ecx
002C02:  66 89 54 42                   mov      dword ptr [si + 0x42], edx
002C06:  66 89 7C 4A                   mov      dword ptr [si + 0x4a], edi
002C0A:  66 8B 44 3A                   mov      eax, dword ptr [si + 0x3a]
002C0E:  66 8B 4C 3E                   mov      ecx, dword ptr [si + 0x3e]
002C12:  66 89 44 20                   mov      dword ptr [si + 0x20], eax
002C16:  66 89 4C 24                   mov      dword ptr [si + 0x24], ecx
002C1A:  8B 44 30                      mov      ax, word ptr [si + 0x30]
002C1D:  89 44 2E                      mov      word ptr [si + 0x2e], ax
002C20:  C7 44 2C 7E 2C                mov      word ptr [si + 0x2c], 0x2c7e
002C25:  8B DE                         mov      bx, si
002C27:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
002C2B:  8B 37                         mov      si, word ptr [bx]
002C2D:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
002C31:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002C34:  79 84                         jns      0x2bba
002C36:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: secondary-right transition --
002C39:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
002C3D:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
002C41:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
002C44:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
002C47:  66 01 4C 08                   add      dword ptr [si + 8], ecx
002C4B:  66 01 54 20                   add      dword ptr [si + 0x20], edx
002C4F:  01 44 42                      add      word ptr [si + 0x42], ax
002C52:  01 5C 44                      add      word ptr [si + 0x44], bx
002C55:  8B DE                         mov      bx, si
002C57:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
002C5B:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
002C5F:  66 89 44 18                   mov      dword ptr [si + 0x18], eax
002C63:  66 89 4C 1C                   mov      dword ptr [si + 0x1c], ecx
002C67:  8B 37                         mov      si, word ptr [bx]
002C69:  8B 47 30                      mov      ax, word ptr [bx + 0x30]
002C6C:  89 47 2E                      mov      word ptr [bx + 0x2e], ax
002C6F:  C7 47 2C 7E 2C                mov      word ptr [bx + 0x2c], 0x2c7e
002C74:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002C77:  0F 89 3F FF                   jns      0x2bba
002C7B:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: record removal --
002C7E:  8B 5C 10                      mov      bx, word ptr [si + 0x10]
002C81:  8B 3C                         mov      di, word ptr [si]
002C83:  A1 D0 0B                      mov      ax, word ptr [0xbd0]
002C86:  89 3F                         mov      word ptr [bx], di
002C88:  89 5D 10                      mov      word ptr [di + 0x10], bx
002C8B:  89 04                         mov      word ptr [si], ax
002C8D:  89 36 D0 0B                   mov      word ptr [0xbd0], si
002C91:  8B F7                         mov      si, di
002C93:  FF 4D 2E                      dec      word ptr [di + 0x2e]
002C96:  0F 89 20 FF                   jns      0x2bba
002C9A:  FF 64 2C                      jmp      word ptr [si + 0x2c]
