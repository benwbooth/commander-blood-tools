; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x002572
; group: direct_calls
; provenance: direct_call_from_0x59b, reviewed_contiguous_renderer_owner
; byte_count: 1531
; boundary: reviewed_contiguous_owner_internal_dispatch
; terminal: ret:1
; direct_callees: 0x002b6d
; indirect_calls: 0
; routine_bytes_sha256: 51afd4217130dba15453b8a3bcd2bc417abc05067e546e292c12a227f70dcd57

002572:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002577:  FC                            cld
002578:  64 8E 06 06 00                mov      es, word ptr fs:[6]
00257D:  BB 38 0D                      mov      bx, 0xd38
002580:  89 1E CE 0B                   mov      word ptr [0xbce], bx
002584:  B9 58 02                      mov      cx, 0x258
002587:  8B F3                         mov      si, bx
002589:  83 C3 5A                      add      bx, 0x5a
00258C:  89 1C                         mov      word ptr [si], bx
00258E:  E2 F7                         loop     0x2587
002590:  C7 04 00 00                   mov      word ptr [si], 0
002594:  C7 06 4A 09 4C 09             mov      word ptr [0x94a], 0x94c
00259A:  C7 06 46 09 00 00             mov      word ptr [0x946], 0
0025A0:  B9 40 01                      mov      cx, 0x140
0025A3:  8B 3E 4A 09                   mov      di, word ptr [0x94a]
0025A7:  2B 0E 46 09                   sub      cx, word ptr [0x946]
0025AB:  33 C0                         xor      ax, ax
0025AD:  F3 AF                         repe scasw ax, word ptr es:[di]
0025AF:  0F 84 92 00                   je       0x2645
0025B3:  BE 2A 0C                      mov      si, 0xc2a
0025B6:  BB 84 0C                      mov      bx, 0xc84
0025B9:  89 1C                         mov      word ptr [si], bx
0025BB:  C7 44 2E 4A 01                mov      word ptr [si + 0x2e], 0x14a
0025C0:  66 C7 44 08 00 00 00 80       mov      dword ptr [si + 8], 0x80000000
0025C8:  66 C7 44 18 00 00 00 00       mov      dword ptr [si + 0x18], 0
0025D0:  66 C7 44 0C 00 00 00 00       mov      dword ptr [si + 0xc], 0
0025D8:  66 C7 44 1C 00 00 00 00       mov      dword ptr [si + 0x1c], 0
0025E0:  89 77 10                      mov      word ptr [bx + 0x10], si
0025E3:  C7 07 DE 0C                   mov      word ptr [bx], 0xcde
0025E7:  C7 47 2E 40 01                mov      word ptr [bx + 0x2e], 0x140
0025EC:  66 C7 47 08 00 00 00 00       mov      dword ptr [bx + 8], 0
0025F4:  66 C7 47 18 00 00 00 00       mov      dword ptr [bx + 0x18], 0
0025FC:  C7 47 0A C8 00                mov      word ptr [bx + 0xa], 0xc8
002601:  66 C7 47 18 00 00 FF 7F       mov      dword ptr [bx + 0x18], 0x7fff0000
002609:  C7 47 02 00 80                mov      word ptr [bx + 2], 0x8000
00260E:  BE DE 0C                      mov      si, 0xcde
002611:  66 C7 44 08 FF FF FF 7F       mov      dword ptr [si + 8], 0x7fffffff
002619:  66 C7 44 18 FF FF FF 7F       mov      dword ptr [si + 0x18], 0x7fffffff
002621:  89 1E EE 0C                   mov      word ptr [0xcee], bx
002625:  C7 06 0A 0D 70 26             mov      word ptr [0xd0a], 0x2670
00262B:  C7 06 0C 0D FF FF             mov      word ptr [0xd0c], 0xffff
002631:  BB 3F 01                      mov      bx, 0x13f
002634:  83 EF 02                      sub      di, 2
002637:  2B D9                         sub      bx, cx
002639:  89 3E 4A 09                   mov      word ptr [0x94a], di
00263D:  89 1E 46 09                   mov      word ptr [0x946], bx
002641:  8B 35                         mov      si, word ptr [di]
002643:  EB 6D                         jmp      0x26b2
; -- internal owner block: shared return --
002645:  C3                            ret
; -- internal owner block: active-list insertion --
002646:  56                            push     si
002647:  8B 1D                         mov      bx, word ptr [di]
002649:  89 1C                         mov      word ptr [si], bx
00264B:  89 77 10                      mov      word ptr [bx + 0x10], si
00264E:  66 8B 45 08                   mov      eax, dword ptr [di + 8]
002652:  8B 74 10                      mov      si, word ptr [si + 0x10]
002655:  81 FE 2A 0C                   cmp      si, 0xc2a
002659:  74 06                         je       0x2661
00265B:  66 3B 44 08                   cmp      eax, dword ptr [si + 8]
00265F:  7C F1                         jl       0x2652
002661:  8B 1C                         mov      bx, word ptr [si]
002663:  89 3C                         mov      word ptr [si], di
002665:  89 1D                         mov      word ptr [di], bx
002667:  89 75 10                      mov      word ptr [di + 0x10], si
00266A:  89 7F 10                      mov      word ptr [bx + 0x10], di
00266D:  5E                            pop      si
00266E:  EB 11                         jmp      0x2681
; -- internal owner block: next-column continuation --
002670:  A1 46 09                      mov      ax, word ptr [0x946]
002673:  40                            inc      ax
002674:  3D 40 01                      cmp      ax, 0x140
002677:  73 CC                         jae      0x2645
002679:  A3 46 09                      mov      word ptr [0x946], ax
00267C:  BB 2A 0C                      mov      bx, 0xc2a
00267F:  8B 37                         mov      si, word ptr [bx]
002681:  8B 3C                         mov      di, word ptr [si]
002683:  81 FF DE 0C                   cmp      di, 0xcde
002687:  74 18                         je       0x26a1
002689:  66 8B 44 08                   mov      eax, dword ptr [si + 8]
00268D:  66 8B 4C 18                   mov      ecx, dword ptr [si + 0x18]
002691:  66 3B 45 08                   cmp      eax, dword ptr [di + 8]
002695:  7F AF                         jg       0x2646
002697:  8B F7                         mov      si, di
002699:  8B 3D                         mov      di, word ptr [di]
00269B:  81 FF DE 0C                   cmp      di, 0xcde
00269F:  75 E8                         jne      0x2689
0026A1:  8B 3E 4A 09                   mov      di, word ptr [0x94a]
0026A5:  83 C7 02                      add      di, 2
0026A8:  89 3E 4A 09                   mov      word ptr [0x94a], di
0026AC:  8B 35                         mov      si, word ptr [di]
0026AE:  0B F6                         or       si, si
0026B0:  74 1C                         je       0x26ce
0026B2:  C7 05 00 00                   mov      word ptr [di], 0
0026B6:  F7 06 CE 0B FF FF             test     word ptr [0xbce], 0xffff
0026BC:  74 10                         je       0x26ce
0026BE:  64 8E 06 02 00                mov      es, word ptr fs:[2]
0026C3:  26 FF 34                      push     word ptr es:[si]
0026C6:  E8 A4 04                      call     0x2b6d
0026C9:  5E                            pop      si
0026CA:  0B F6                         or       si, si
0026CC:  75 F5                         jne      0x26c3
0026CE:  BE 2A 0C                      mov      si, 0xc2a
0026D1:  8B 04                         mov      ax, word ptr [si]
0026D3:  3D 84 0C                      cmp      ax, 0xc84
0026D6:  0F 84 A7 03                   je       0x2a81
0026DA:  BA 3A 0C                      mov      dx, 0xc3a
0026DD:  C7 44 02 01 00                mov      word ptr [si + 2], 1
0026E2:  89 54 06                      mov      word ptr [si + 6], dx
0026E5:  8B FE                         mov      di, si
0026E7:  8B EE                         mov      bp, si
0026E9:  33 DB                         xor      bx, bx
0026EB:  8B 3D                         mov      di, word ptr [di]
0026ED:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
0026F2:  75 F7                         jne      0x26eb
0026F4:  89 5C 58                      mov      word ptr [si + 0x58], bx
0026F7:  89 5D 58                      mov      word ptr [di + 0x58], bx
0026FA:  3B 5D 0A                      cmp      bx, word ptr [di + 0xa]
0026FD:  0F 8E A1 00                   jle      0x27a2
002701:  8B F7                         mov      si, di
002703:  8B EA                         mov      bp, dx
002705:  89 3E 28 0C                   mov      word ptr [0xc28], di
002709:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
00270E:  F7 D8                         neg      ax
002710:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
002714:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
002718:  66 89 45 04                   mov      dword ptr [di + 4], eax
00271C:  EB 2F                         jmp      0x274d
00271E:  BB D0 0B                      mov      bx, 0xbd0
002721:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
002726:  F7 D8                         neg      ax
002728:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
00272C:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
002730:  66 89 45 04                   mov      dword ptr [di + 4], eax
002734:  66 3B 44 04                   cmp      eax, dword ptr [si + 4]
002738:  7E 09                         jle      0x2743
00273A:  8B DE                         mov      bx, si
00273C:  8B 74 58                      mov      si, word ptr [si + 0x58]
00273F:  0B F6                         or       si, si
002741:  75 F1                         jne      0x2734
002743:  89 7F 58                      mov      word ptr [bx + 0x58], di
002746:  89 75 58                      mov      word ptr [di + 0x58], si
002749:  8B 36 28 0C                   mov      si, word ptr [0xc28]
00274D:  8B 3D                         mov      di, word ptr [di]
00274F:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
002754:  75 F7                         jne      0x274d
002756:  F7 45 0A 00 80                test     word ptr [di + 0xa], 0x8000
00275B:  75 C1                         jne      0x271e
00275D:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002763:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
002767:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
00276B:  8B 44 1A                      mov      ax, word ptr [si + 0x1a]
00276E:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
002771:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
002774:  7D 55                         jge      0x27cb
002776:  3B C2                         cmp      ax, dx
002778:  0F 8F B1 00                   jg       0x282d
00277C:  74 72                         je       0x27f0
00277E:  8D 54 10                      lea      dx, [si + 0x10]
002781:  8B 4C 1A                      mov      cx, word ptr [si + 0x1a]
002784:  3E 89 56 06                   mov      word ptr ds:[bp + 6], dx
002788:  8B EA                         mov      bp, dx
00278A:  8B 74 58                      mov      si, word ptr [si + 0x58]
00278D:  0B F6                         or       si, si
00278F:  74 11                         je       0x27a2
002791:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
002794:  7D F4                         jge      0x278a
002796:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
00279C:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
0027A0:  EB C5                         jmp      0x2767
0027A2:  81 FF 84 0C                   cmp      di, 0xc84
0027A6:  0F 84 0F 02                   je       0x29b9
0027AA:  3E C7 46 02 01 00             mov      word ptr ds:[bp + 2], 1
0027B0:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0027B4:  8B EF                         mov      bp, di
0027B6:  C7 45 58 00 00                mov      word ptr [di + 0x58], 0
0027BB:  8B F7                         mov      si, di
0027BD:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0027C3:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
0027C7:  8B 3D                         mov      di, word ptr [di]
0027C9:  EB 9C                         jmp      0x2767
0027CB:  8B 3D                         mov      di, word ptr [di]
0027CD:  EB 98                         jmp      0x2767
0027CF:  8B 5C 58                      mov      bx, word ptr [si + 0x58]
0027D2:  0B DB                         or       bx, bx
0027D4:  75 1A                         jne      0x27f0
0027D6:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0027DA:  8B EF                         mov      bp, di
0027DC:  8B F7                         mov      si, di
0027DE:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0027E4:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
0027E8:  89 5C 58                      mov      word ptr [si + 0x58], bx
0027EB:  8B 3C                         mov      di, word ptr [si]
0027ED:  E9 77 FF                      jmp      0x2767
0027F0:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0027F4:  8B 4D 0A                      mov      cx, word ptr [di + 0xa]
0027F7:  8B EF                         mov      bp, di
0027F9:  81 FF 84 0C                   cmp      di, 0xc84
0027FD:  0F 84 B8 01                   je       0x29b9
002801:  8B 74 58                      mov      si, word ptr [si + 0x58]
002804:  0B F6                         or       si, si
002806:  74 11                         je       0x2819
002808:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
00280B:  7D F4                         jge      0x2801
00280D:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002813:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
002817:  EB 14                         jmp      0x282d
002819:  89 75 58                      mov      word ptr [di + 0x58], si
00281C:  8B F7                         mov      si, di
00281E:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
002824:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
002828:  8B 3D                         mov      di, word ptr [di]
00282A:  E9 3A FF                      jmp      0x2767
00282D:  81 FF 84 0C                   cmp      di, 0xc84
002831:  0F 84 7E 01                   je       0x29b3
002835:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
002838:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
00283B:  7D 49                         jge      0x2886
00283D:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
002841:  BB D0 0B                      mov      bx, 0xbd0
002844:  89 77 58                      mov      word ptr [bx + 0x58], si
002847:  66 8B 4D 08                   mov      ecx, dword ptr [di + 8]
00284B:  66 3B 4C 18                   cmp      ecx, dword ptr [si + 0x18]
00284F:  7C 0C                         jl       0x285d
002851:  8B 74 58                      mov      si, word ptr [si + 0x58]
002854:  0B F6                         or       si, si
002856:  89 77 58                      mov      word ptr [bx + 0x58], si
002859:  75 F0                         jne      0x284b
00285B:  EB 23                         jmp      0x2880
00285D:  66 8B C1                      mov      eax, ecx
002860:  66 2B 44 08                   sub      eax, dword ptr [si + 8]
002864:  66 F7 6C 28                   imul     dword ptr [si + 0x28]
002868:  66 0F AC D0 10                shrd     eax, edx, 0x10
00286D:  66 03 44 20                   add      eax, dword ptr [si + 0x20]
002871:  66 3B 45 20                   cmp      eax, dword ptr [di + 0x20]
002875:  7D 09                         jge      0x2880
002877:  8B DE                         mov      bx, si
002879:  8B 74 58                      mov      si, word ptr [si + 0x58]
00287C:  0B F6                         or       si, si
00287E:  75 CB                         jne      0x284b
002880:  89 7F 58                      mov      word ptr [bx + 0x58], di
002883:  89 75 58                      mov      word ptr [di + 0x58], si
002886:  8B 36 28 0C                   mov      si, word ptr [0xc28]
00288A:  3B F7                         cmp      si, di
00288C:  75 0E                         jne      0x289c
00288E:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
002892:  8B EF                         mov      bp, di
002894:  C7 45 02 00 00                mov      word ptr [di + 2], 0
002899:  89 7D 04                      mov      word ptr [di + 4], di
00289C:  8B 3D                         mov      di, word ptr [di]
00289E:  E9 C6 FE                      jmp      0x2767
; -- internal owner block: four-plane renderer --
0028A1:  8B 3E 46 09                   mov      di, word ptr [0x946]
0028A5:  8B CF                         mov      cx, di
0028A7:  83 E1 03                      and      cx, 3
0028AA:  0F 85 D3 01                   jne      0x2a81
0028AE:  64 8E 06 28 00                mov      es, word ptr fs:[0x28]
0028B3:  BA C4 03                      mov      dx, 0x3c4
0028B6:  B8 02 0F                      mov      ax, 0xf02
0028B9:  C1 EF 02                      shr      di, 2
0028BC:  89 3E 48 09                   mov      word ptr [0x948], di
0028C0:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0028C5:  EF                            out      dx, ax
0028C6:  0F 88 B7 01                   js       0x2a81
0028CA:  74 51                         je       0x291d
0028CC:  EB 30                         jmp      0x28fe
0028CE:  87 DB                         xchg     bx, bx
; -- internal owner block: Mode-X renderer --
0028D0:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0028D5:  0F 88 A8 01                   js       0x2a81
0028D9:  64 8E 06 28 00                mov      es, word ptr fs:[0x28]
0028DE:  BA C4 03                      mov      dx, 0x3c4
0028E1:  8B 3E 46 09                   mov      di, word ptr [0x946]
0028E5:  B8 02 01                      mov      ax, 0x102
0028E8:  8B CF                         mov      cx, di
0028EA:  C1 EF 02                      shr      di, 2
0028ED:  83 E1 03                      and      cx, 3
0028F0:  89 3E 48 09                   mov      word ptr [0x948], di
0028F4:  D2 E4                         shl      ah, cl
0028F6:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0028FB:  EF                            out      dx, ax
0028FC:  74 1F                         je       0x291d
0028FE:  8B 3E 48 09                   mov      di, word ptr [0x948]
002902:  8B 5F 06                      mov      bx, word ptr [bx + 6]
002905:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002908:  C1 E0 04                      shl      ax, 4
00290B:  03 F8                         add      di, ax
00290D:  C1 E0 02                      shl      ax, 2
002910:  03 F8                         add      di, ax
002912:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002917:  0F 88 66 01                   js       0x2a81
00291B:  75 E1                         jne      0x28fe
00291D:  8B 77 06                      mov      si, word ptr [bx + 6]
002920:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
002923:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
002926:  2B C8                         sub      cx, ax
002928:  56                            push     si
002929:  7E 33                         jle      0x295e
00292B:  8B 77 04                      mov      si, word ptr [bx + 4]
00292E:  90                            nop
00292F:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
002932:  75 3C                         jne      0x2970
002934:  8B 44 42                      mov      ax, word ptr [si + 0x42]
002937:  8A DC                         mov      bl, ah
002939:  8B 54 44                      mov      dx, word ptr [si + 0x44]
00293C:  8A FE                         mov      bh, dh
00293E:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002941:  90                            nop
002942:  C5 74 54                      lds      si, ptr [si + 0x54]
002945:  03 C5                         add      ax, bp
002947:  8A 2F                         mov      ch, byte ptr [bx]
002949:  03 D6                         add      dx, si
00294B:  8A DC                         mov      bl, ah
00294D:  26 88 2D                      mov      byte ptr es:[di], ch
002950:  83 C7 50                      add      di, 0x50
002953:  FE C9                         dec      cl
002955:  8A FE                         mov      bh, dh
002957:  75 EC                         jne      0x2945
002959:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
00295E:  5B                            pop      bx
00295F:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002964:  74 B7                         je       0x291d
002966:  79 96                         jns      0x28fe
002968:  E9 16 01                      jmp      0x2a81
00296B:  87 DB                         xchg     bx, bx
00296D:  87 DB                         xchg     bx, bx
00296F:  90                            nop
002970:  8B 54 54                      mov      dx, word ptr [si + 0x54]
002973:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002976:  0F AF D0                      imul     dx, ax
002979:  0F AF C5                      imul     ax, bp
00297C:  03 54 44                      add      dx, word ptr [si + 0x44]
00297F:  03 44 42                      add      ax, word ptr [si + 0x42]
002982:  8A FE                         mov      bh, dh
002984:  8A DC                         mov      bl, ah
002986:  C5 74 54                      lds      si, ptr [si + 0x54]
002989:  8A 2F                         mov      ch, byte ptr [bx]
00298B:  03 C5                         add      ax, bp
00298D:  26 88 2D                      mov      byte ptr es:[di], ch
002990:  03 D6                         add      dx, si
002992:  83 C7 50                      add      di, 0x50
002995:  FE C9                         dec      cl
002997:  8A DC                         mov      bl, ah
002999:  8A FE                         mov      bh, dh
00299B:  75 EC                         jne      0x2989
00299D:  5B                            pop      bx
00299E:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
0029A3:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0029A8:  0F 84 71 FF                   je       0x291d
0029AC:  0F 89 4E FF                   jns      0x28fe
0029B0:  E9 CE 00                      jmp      0x2a81
0029B3:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0029B7:  8B EF                         mov      bp, di
0029B9:  3E C7 46 02 00 80             mov      word ptr ds:[bp + 2], 0x8000
0029BF:  BB 2A 0C                      mov      bx, 0xc2a
0029C2:  FF 26 44 09                   jmp      word ptr [0x944]
; -- internal owner block: linear renderer --
0029C6:  64 8E 06 24 00                mov      es, word ptr fs:[0x24]
0029CB:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0029D0:  0F 88 AD 00                   js       0x2a81
0029D4:  74 1E                         je       0x29f4
0029D6:  8B 3E 46 09                   mov      di, word ptr [0x946]
0029DA:  8B 5F 06                      mov      bx, word ptr [bx + 6]
0029DD:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
0029E0:  8B C8                         mov      cx, ax
0029E2:  C1 E0 06                      shl      ax, 6
0029E5:  02 E1                         add      ah, cl
0029E7:  03 F8                         add      di, ax
0029E9:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
0029EE:  0F 88 8F 00                   js       0x2a81
0029F2:  75 E2                         jne      0x29d6
0029F4:  8B 77 06                      mov      si, word ptr [bx + 6]
0029F7:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
0029FA:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
0029FD:  2B C8                         sub      cx, ax
0029FF:  56                            push     si
002A00:  7E 32                         jle      0x2a34
002A02:  8B 77 04                      mov      si, word ptr [bx + 4]
002A05:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
002A08:  75 36                         jne      0x2a40
002A0A:  8B 44 42                      mov      ax, word ptr [si + 0x42]
002A0D:  8A DC                         mov      bl, ah
002A0F:  8B 54 44                      mov      dx, word ptr [si + 0x44]
002A12:  8A FE                         mov      bh, dh
002A14:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002A17:  C5 74 54                      lds      si, ptr [si + 0x54]
; -- internal owner block: affine fill loop --
002A1A:  03 C5                         add      ax, bp
002A1C:  03 D6                         add      dx, si
002A1E:  8A 2F                         mov      ch, byte ptr [bx]
002A20:  8A DC                         mov      bl, ah
002A22:  26 88 2D                      mov      byte ptr es:[di], ch
002A25:  81 C7 40 01                   add      di, 0x140
002A29:  FE C9                         dec      cl
002A2B:  8A FE                         mov      bh, dh
002A2D:  75 EB                         jne      0x2a1a
002A2F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002A34:  5B                            pop      bx
002A35:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A3A:  74 B8                         je       0x29f4
002A3C:  79 98                         jns      0x29d6
002A3E:  EB 41                         jmp      0x2a81
002A40:  8B 54 54                      mov      dx, word ptr [si + 0x54]
002A43:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
002A46:  0F AF D0                      imul     dx, ax
002A49:  0F AF C5                      imul     ax, bp
002A4C:  03 54 44                      add      dx, word ptr [si + 0x44]
002A4F:  03 44 42                      add      ax, word ptr [si + 0x42]
002A52:  8A FE                         mov      bh, dh
002A54:  8A DC                         mov      bl, ah
002A56:  C5 74 54                      lds      si, ptr [si + 0x54]
002A59:  8A 2F                         mov      ch, byte ptr [bx]
002A5B:  03 C5                         add      ax, bp
002A5D:  26 88 2D                      mov      byte ptr es:[di], ch
002A60:  03 D6                         add      dx, si
002A62:  81 C7 40 01                   add      di, 0x140
002A66:  FE C9                         dec      cl
002A68:  8A DC                         mov      bl, ah
002A6A:  8A FE                         mov      bh, dh
002A6C:  75 EB                         jne      0x2a59
002A6E:  5B                            pop      bx
002A6F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
002A74:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
002A79:  0F 84 77 FF                   je       0x29f4
002A7D:  0F 89 55 FF                   jns      0x29d6
; -- internal owner block: record advance --
002A81:  8B 36 2A 0C                   mov      si, word ptr [0xc2a]
002A85:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002A88:  78 2D                         js       0x2ab7
002A8A:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
002A8D:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
002A90:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
002A94:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
002A98:  01 44 42                      add      word ptr [si + 0x42], ax
002A9B:  01 5C 44                      add      word ptr [si + 0x44], bx
002A9E:  66 01 4C 08                   add      dword ptr [si + 8], ecx
002AA2:  66 01 54 20                   add      dword ptr [si + 0x20], edx
002AA6:  8B DE                         mov      bx, si
002AA8:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
002AAC:  8B 37                         mov      si, word ptr [bx]
002AAE:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
002AB2:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002AB5:  79 D3                         jns      0x2a8a
002AB7:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: secondary-left transition --
002ABA:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
002ABE:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
002AC2:  66 8B 54 46                   mov      edx, dword ptr [si + 0x46]
002AC6:  66 8B 7C 4E                   mov      edi, dword ptr [si + 0x4e]
002ACA:  66 89 44 08                   mov      dword ptr [si + 8], eax
002ACE:  66 89 4C 0C                   mov      dword ptr [si + 0xc], ecx
002AD2:  66 89 54 42                   mov      dword ptr [si + 0x42], edx
002AD6:  66 89 7C 4A                   mov      dword ptr [si + 0x4a], edi
002ADA:  66 8B 44 3A                   mov      eax, dword ptr [si + 0x3a]
002ADE:  66 8B 4C 3E                   mov      ecx, dword ptr [si + 0x3e]
002AE2:  66 89 44 20                   mov      dword ptr [si + 0x20], eax
002AE6:  66 89 4C 24                   mov      dword ptr [si + 0x24], ecx
002AEA:  8B 44 30                      mov      ax, word ptr [si + 0x30]
002AED:  89 44 2E                      mov      word ptr [si + 0x2e], ax
002AF0:  C7 44 2C 4E 2B                mov      word ptr [si + 0x2c], 0x2b4e
002AF5:  8B DE                         mov      bx, si
002AF7:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
002AFB:  8B 37                         mov      si, word ptr [bx]
002AFD:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
002B01:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002B04:  79 84                         jns      0x2a8a
002B06:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: secondary-right transition --
002B09:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
002B0D:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
002B11:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
002B14:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
002B17:  66 01 4C 08                   add      dword ptr [si + 8], ecx
002B1B:  66 01 54 20                   add      dword ptr [si + 0x20], edx
002B1F:  01 44 42                      add      word ptr [si + 0x42], ax
002B22:  01 5C 44                      add      word ptr [si + 0x44], bx
002B25:  8B DE                         mov      bx, si
002B27:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
002B2B:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
002B2F:  66 89 44 18                   mov      dword ptr [si + 0x18], eax
002B33:  66 89 4C 1C                   mov      dword ptr [si + 0x1c], ecx
002B37:  8B 37                         mov      si, word ptr [bx]
002B39:  8B 47 30                      mov      ax, word ptr [bx + 0x30]
002B3C:  89 47 2E                      mov      word ptr [bx + 0x2e], ax
002B3F:  C7 47 2C 4E 2B                mov      word ptr [bx + 0x2c], 0x2b4e
002B44:  FF 4C 2E                      dec      word ptr [si + 0x2e]
002B47:  0F 89 3F FF                   jns      0x2a8a
002B4B:  FF 64 2C                      jmp      word ptr [si + 0x2c]
; -- internal owner block: record removal --
002B4E:  8B 5C 10                      mov      bx, word ptr [si + 0x10]
002B51:  8B 3C                         mov      di, word ptr [si]
002B53:  A1 CE 0B                      mov      ax, word ptr [0xbce]
002B56:  89 3F                         mov      word ptr [bx], di
002B58:  89 5D 10                      mov      word ptr [di + 0x10], bx
002B5B:  89 04                         mov      word ptr [si], ax
002B5D:  89 36 CE 0B                   mov      word ptr [0xbce], si
002B61:  8B F7                         mov      si, di
002B63:  FF 4D 2E                      dec      word ptr [di + 0x2e]
002B66:  0F 89 20 FF                   jns      0x2a8a
002B6A:  FF 64 2C                      jmp      word ptr [si + 0x2c]
