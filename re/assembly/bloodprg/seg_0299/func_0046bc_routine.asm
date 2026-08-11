; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0046bc
; seg_off: 0299:172c
; group: seg_0299
; provenance: static_dispatch_table_target
; incoming: sprite_blitter_candidates:blit_1
; byte_count: 1260
; boundary: cfg_blocks_179_terminals_55
; terminal: jmp 0x471a:1, jmp 0x4753:1, jmp 0x47e7:1, jmp 0x47ee:2, jmp 0x4826:3, jmp 0x4839:1, jmp 0x4852:1, jmp 0x4876:1, jmp 0x48b5:4, jmp 0x48e7:1, jmp 0x48ee:2, jmp 0x4926:3, jmp 0x4939:1, jmp 0x4954:1, jmp 0x497a:1, jmp 0x49b7:4, jmp 0x49e3:1, jmp 0x49ea:2, jmp 0x4a22:3, jmp 0x4a33:1, jmp 0x4a46:1, jmp 0x4a64:1, jmp 0x4a96:4, jmp 0x4ac3:1, jmp 0x4aca:2, jmp 0x4b02:3, jmp 0x4b13:1, jmp 0x4b28:1, jmp 0x4b48:1, jmp 0x4b7a:4, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0046bc_routine.cpp
; routine_bytes_sha256: f98b0fd105ad6b88cfa1c3a7c2e146ac425df32dc811e81064a677b5ffbc29b4

0046BC:  50                           push     ax
0046BD:  53                           push     bx
0046BE:  51                           push     cx
0046BF:  52                           push     dx
0046C0:  06                           push     es
0046C1:  57                           push     di
0046C2:  1E                           push     ds
0046C3:  56                           push     si
0046C4:  55                           push     bp
0046C5:  C5 75 04                     lds      si, ptr [di + 4]
0046C8:  03 44 04                     add      ax, word ptr [si + 4]
0046CB:  03 5C 06                     add      bx, word ptr [si + 6]
0046CE:  03 54 04                     add      dx, word ptr [si + 4]
0046D1:  03 6C 06                     add      bp, word ptr [si + 6]
0046D4:  66 2E C7 06 28 17 00 00 00 00 mov      dword ptr cs:[0x1728], 0
0046DE:  8B 04                        mov      ax, word ptr [si]
0046E0:  2E A3 26 17                  mov      word ptr cs:[0x1726], ax
0046E4:  52                           push     dx
0046E5:  83 C6 08                     add      si, 8
0046E8:  26 8B 4D 0E                  mov      cx, word ptr es:[di + 0xe]
0046EC:  8B C3                        mov      ax, bx
0046EE:  26 2B 45 1C                  sub      ax, word ptr es:[di + 0x1c]
0046F2:  79 31                        jns      0x4725
0046F4:  F7 D8                        neg      ax
0046F6:  2B C8                        sub      cx, ax
0046F8:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
0046FE:  75 21                        jne      0x4721
004700:  51                           push     cx
004701:  8B C8                        mov      cx, ax
004703:  32 E4                        xor      ah, ah
004705:  2E 8B 1E 26 17               mov      bx, word ptr cs:[0x1726]
00470A:  AC                           lodsb    al, byte ptr [si]
00470B:  0A C0                        or       al, al
00470D:  79 07                        jns      0x4716
00470F:  F6 D8                        neg      al
004711:  FE C0                        inc      al
004713:  46                           inc      si
004714:  EB 04                        jmp      0x471a
004716:  FE C0                        inc      al
004718:  03 F0                        add      si, ax
00471A:  2B D8                        sub      bx, ax
00471C:  75 EC                        jne      0x470a
00471E:  E2 E5                        loop     0x4705
004720:  59                           pop      cx
004721:  26 8B 5D 1C                  mov      bx, word ptr es:[di + 0x1c]
004725:  8B C5                        mov      ax, bp
004727:  26 2B 45 1E                  sub      ax, word ptr es:[di + 0x1e]
00472B:  78 2D                        js       0x475a
00472D:  74 2B                        je       0x475a
00472F:  2B C8                        sub      cx, ax
004731:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004737:  74 21                        je       0x475a
004739:  51                           push     cx
00473A:  8B C8                        mov      cx, ax
00473C:  32 E4                        xor      ah, ah
00473E:  2E 8B 16 26 17               mov      dx, word ptr cs:[0x1726]
004743:  AC                           lodsb    al, byte ptr [si]
004744:  0A C0                        or       al, al
004746:  79 07                        jns      0x474f
004748:  F6 D8                        neg      al
00474A:  FE C0                        inc      al
00474C:  46                           inc      si
00474D:  EB 04                        jmp      0x4753
00474F:  FE C0                        inc      al
004751:  03 F0                        add      si, ax
004753:  2B D0                        sub      dx, ax
004755:  75 EC                        jne      0x4743
004757:  E2 E5                        loop     0x473e
004759:  59                           pop      cx
00475A:  26 8B 6D 0C                  mov      bp, word ptr es:[di + 0xc]
00475E:  26 8B 55 08                  mov      dx, word ptr es:[di + 8]
004762:  03 54 FC                     add      dx, word ptr [si - 4]
004765:  8B C2                        mov      ax, dx
004767:  26 2B 45 18                  sub      ax, word ptr es:[di + 0x18]
00476B:  79 0C                        jns      0x4779
00476D:  F7 D8                        neg      ax
00476F:  2B E8                        sub      bp, ax
004771:  2E A3 28 17                  mov      word ptr cs:[0x1728], ax
004775:  26 8B 55 18                  mov      dx, word ptr es:[di + 0x18]
004779:  58                           pop      ax
00477A:  26 2B 45 1A                  sub      ax, word ptr es:[di + 0x1a]
00477E:  78 06                        js       0x4786
004780:  2B E8                        sub      bp, ax
004782:  2E A3 2A 17                  mov      word ptr cs:[0x172a], ax
004786:  53                           push     bx
004787:  33 DB                        xor      bx, bx
004789:  26 8A 45 01                  mov      al, byte ptr es:[di + 1]
00478D:  24 03                        and      al, 3
00478F:  74 0A                        je       0x479b
004791:  BB 11 5F                     mov      bx, 0x5f11
004794:  FE C8                        dec      al
004796:  74 03                        je       0x479b
004798:  BB 11 60                     mov      bx, 0x6011
00479B:  65 89 1E 4B 52               mov      word ptr gs:[0x524b], bx
0047A0:  5B                           pop      bx
0047A1:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
0047A6:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
0047AC:  74 03                        je       0x47b1
0047AE:  03 D9                        add      bx, cx
0047B0:  4B                           dec      bx
0047B1:  8B C3                        mov      ax, bx
0047B3:  86 C4                        xchg     ah, al
0047B5:  C1 E3 06                     shl      bx, 6
0047B8:  03 C3                        add      ax, bx
0047BA:  03 F8                        add      di, ax
0047BC:  03 FA                        add      di, dx
0047BE:  BA 40 01                     mov      dx, 0x140
0047C1:  2B D5                        sub      dx, bp
0047C3:  32 E4                        xor      ah, ah
0047C5:  2E 8B 1E DF 14               mov      bx, word ptr cs:[0x14df]
0047CA:  0A FF                        or       bh, bh
0047CC:  74 06                        je       0x47d4
0047CE:  03 D5                        add      dx, bp
0047D0:  03 D5                        add      dx, bp
0047D2:  F7 DA                        neg      dx
0047D4:  8A C3                        mov      al, bl
0047D6:  65 8B 1E 4B 52               mov      bx, word ptr gs:[0x524b]
0047DB:  0B DB                        or       bx, bx
0047DD:  0F 84 FC 01                  je       0x49dd
0047E1:  0A C0                        or       al, al
0047E3:  0F 85 F4 00                  jne      0x48db
0047E7:  51                           push     cx
0047E8:  52                           push     dx
0047E9:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
0047EE:  0B D2                        or       dx, dx
0047F0:  74 32                        je       0x4824
0047F2:  AC                           lodsb    al, byte ptr [si]
0047F3:  0A C0                        or       al, al
0047F5:  79 15                        jns      0x480c
0047F7:  F6 D8                        neg      al
0047F9:  FE C0                        inc      al
0047FB:  2B D0                        sub      dx, ax
0047FD:  79 0A                        jns      0x4809
0047FF:  F7 DA                        neg      dx
004801:  8B CA                        mov      cx, dx
004803:  8B D5                        mov      dx, bp
004805:  2B D1                        sub      dx, cx
004807:  EB 30                        jmp      0x4839
004809:  46                           inc      si
00480A:  EB E2                        jmp      0x47ee
00480C:  FE C0                        inc      al
00480E:  2B D0                        sub      dx, ax
004810:  79 0E                        jns      0x4820
004812:  F7 DA                        neg      dx
004814:  2B C2                        sub      ax, dx
004816:  03 F0                        add      si, ax
004818:  8B CA                        mov      cx, dx
00481A:  8B D5                        mov      dx, bp
00481C:  2B D1                        sub      dx, cx
00481E:  EB 56                        jmp      0x4876
004820:  03 F0                        add      si, ax
004822:  EB CA                        jmp      0x47ee
004824:  8B D5                        mov      dx, bp
004826:  0B D2                        or       dx, dx
004828:  0F 84 84 00                  je       0x48b0
00482C:  AC                           lodsb    al, byte ptr [si]
00482D:  0A C0                        or       al, al
00482F:  79 3F                        jns      0x4870
004831:  F6 D8                        neg      al
004833:  FE C0                        inc      al
004835:  8B C8                        mov      cx, ax
004837:  2B D0                        sub      dx, ax
004839:  79 22                        jns      0x485d
00483B:  F7 DA                        neg      dx
00483D:  2B C2                        sub      ax, dx
00483F:  8B C8                        mov      cx, ax
004841:  AC                           lodsb    al, byte ptr [si]
004842:  0A C0                        or       al, al
004844:  74 0A                        je       0x4850
004846:  26 8A 05                     mov      al, byte ptr es:[di]
004849:  65 D7                        xlatb   
00484B:  AA                           stosb    byte ptr es:[di], al
00484C:  E2 F8                        loop     0x4846
00484E:  EB 02                        jmp      0x4852
004850:  03 F9                        add      di, cx
004852:  8B C2                        mov      ax, dx
004854:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004859:  2B D0                        sub      dx, ax
00485B:  EB 58                        jmp      0x48b5
00485D:  AC                           lodsb    al, byte ptr [si]
00485E:  0A C0                        or       al, al
004860:  74 0A                        je       0x486c
004862:  26 8A 05                     mov      al, byte ptr es:[di]
004865:  65 D7                        xlatb   
004867:  AA                           stosb    byte ptr es:[di], al
004868:  E2 F8                        loop     0x4862
00486A:  EB BA                        jmp      0x4826
00486C:  03 F9                        add      di, cx
00486E:  EB B6                        jmp      0x4826
004870:  FE C0                        inc      al
004872:  8B C8                        mov      cx, ax
004874:  2B D0                        sub      dx, ax
004876:  79 23                        jns      0x489b
004878:  F7 DA                        neg      dx
00487A:  2B C2                        sub      ax, dx
00487C:  8B C8                        mov      cx, ax
00487E:  AC                           lodsb    al, byte ptr [si]
00487F:  0A C0                        or       al, al
004881:  74 08                        je       0x488b
004883:  26 8A 05                     mov      al, byte ptr es:[di]
004886:  65 D7                        xlatb   
004888:  26 88 05                     mov      byte ptr es:[di], al
00488B:  47                           inc      di
00488C:  E2 F0                        loop     0x487e
00488E:  8B C2                        mov      ax, dx
004890:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004895:  2B D0                        sub      dx, ax
004897:  03 F0                        add      si, ax
004899:  EB 1A                        jmp      0x48b5
00489B:  AC                           lodsb    al, byte ptr [si]
00489C:  0A C0                        or       al, al
00489E:  74 08                        je       0x48a8
0048A0:  26 8A 05                     mov      al, byte ptr es:[di]
0048A3:  65 D7                        xlatb   
0048A5:  26 88 05                     mov      byte ptr es:[di], al
0048A8:  47                           inc      di
0048A9:  E2 F0                        loop     0x489b
0048AB:  32 E4                        xor      ah, ah
0048AD:  E9 76 FF                     jmp      0x4826
0048B0:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
0048B5:  0B D2                        or       dx, dx
0048B7:  74 16                        je       0x48cf
0048B9:  AC                           lodsb    al, byte ptr [si]
0048BA:  0A C0                        or       al, al
0048BC:  79 09                        jns      0x48c7
0048BE:  F6 D8                        neg      al
0048C0:  FE C0                        inc      al
0048C2:  2B D0                        sub      dx, ax
0048C4:  46                           inc      si
0048C5:  EB EE                        jmp      0x48b5
0048C7:  FE C0                        inc      al
0048C9:  2B D0                        sub      dx, ax
0048CB:  03 F0                        add      si, ax
0048CD:  EB E6                        jmp      0x48b5
0048CF:  5A                           pop      dx
0048D0:  03 FA                        add      di, dx
0048D2:  59                           pop      cx
0048D3:  49                           dec      cx
0048D4:  0F 84 C6 02                  je       0x4b9e
0048D8:  E9 0C FF                     jmp      0x47e7
0048DB:  03 D5                        add      dx, bp
0048DD:  03 D5                        add      dx, bp
0048DF:  03 FD                        add      di, bp
0048E1:  4F                           dec      di
0048E2:  65 8B 1E 4B 52               mov      bx, word ptr gs:[0x524b]
0048E7:  51                           push     cx
0048E8:  52                           push     dx
0048E9:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
0048EE:  0B D2                        or       dx, dx
0048F0:  74 32                        je       0x4924
0048F2:  AC                           lodsb    al, byte ptr [si]
0048F3:  0A C0                        or       al, al
0048F5:  79 15                        jns      0x490c
0048F7:  F6 D8                        neg      al
0048F9:  FE C0                        inc      al
0048FB:  2B D0                        sub      dx, ax
0048FD:  79 0A                        jns      0x4909
0048FF:  F7 DA                        neg      dx
004901:  8B CA                        mov      cx, dx
004903:  8B D5                        mov      dx, bp
004905:  2B D1                        sub      dx, cx
004907:  EB 30                        jmp      0x4939
004909:  46                           inc      si
00490A:  EB E2                        jmp      0x48ee
00490C:  FE C0                        inc      al
00490E:  2B D0                        sub      dx, ax
004910:  79 0E                        jns      0x4920
004912:  F7 DA                        neg      dx
004914:  2B C2                        sub      ax, dx
004916:  03 F0                        add      si, ax
004918:  8B CA                        mov      cx, dx
00491A:  8B D5                        mov      dx, bp
00491C:  2B D1                        sub      dx, cx
00491E:  EB 5A                        jmp      0x497a
004920:  03 F0                        add      si, ax
004922:  EB CA                        jmp      0x48ee
004924:  8B D5                        mov      dx, bp
004926:  0B D2                        or       dx, dx
004928:  0F 84 86 00                  je       0x49b2
00492C:  AC                           lodsb    al, byte ptr [si]
00492D:  0A C0                        or       al, al
00492F:  79 43                        jns      0x4974
004931:  F6 D8                        neg      al
004933:  FE C0                        inc      al
004935:  8B C8                        mov      cx, ax
004937:  2B D0                        sub      dx, ax
004939:  79 24                        jns      0x495f
00493B:  F7 DA                        neg      dx
00493D:  2B C2                        sub      ax, dx
00493F:  8B C8                        mov      cx, ax
004941:  AC                           lodsb    al, byte ptr [si]
004942:  0A C0                        or       al, al
004944:  74 0C                        je       0x4952
004946:  FD                           std     
004947:  26 8A 05                     mov      al, byte ptr es:[di]
00494A:  65 D7                        xlatb   
00494C:  AA                           stosb    byte ptr es:[di], al
00494D:  E2 F8                        loop     0x4947
00494F:  FC                           cld     
004950:  EB 02                        jmp      0x4954
004952:  2B F9                        sub      di, cx
004954:  8B C2                        mov      ax, dx
004956:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
00495B:  2B D0                        sub      dx, ax
00495D:  EB 58                        jmp      0x49b7
00495F:  AC                           lodsb    al, byte ptr [si]
004960:  0A C0                        or       al, al
004962:  74 0C                        je       0x4970
004964:  FD                           std     
004965:  26 8A 05                     mov      al, byte ptr es:[di]
004968:  65 D7                        xlatb   
00496A:  AA                           stosb    byte ptr es:[di], al
00496B:  E2 F8                        loop     0x4965
00496D:  FC                           cld     
00496E:  EB B6                        jmp      0x4926
004970:  2B F9                        sub      di, cx
004972:  EB B2                        jmp      0x4926
004974:  FE C0                        inc      al
004976:  8B C8                        mov      cx, ax
004978:  2B D0                        sub      dx, ax
00497A:  79 23                        jns      0x499f
00497C:  F7 DA                        neg      dx
00497E:  2B C2                        sub      ax, dx
004980:  8B C8                        mov      cx, ax
004982:  AC                           lodsb    al, byte ptr [si]
004983:  0A C0                        or       al, al
004985:  74 08                        je       0x498f
004987:  26 8A 05                     mov      al, byte ptr es:[di]
00498A:  65 D7                        xlatb   
00498C:  26 88 05                     mov      byte ptr es:[di], al
00498F:  4F                           dec      di
004990:  E2 F0                        loop     0x4982
004992:  8B C2                        mov      ax, dx
004994:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
004999:  2B D0                        sub      dx, ax
00499B:  03 F0                        add      si, ax
00499D:  EB 18                        jmp      0x49b7
00499F:  AC                           lodsb    al, byte ptr [si]
0049A0:  0A C0                        or       al, al
0049A2:  74 08                        je       0x49ac
0049A4:  26 8A 05                     mov      al, byte ptr es:[di]
0049A7:  65 D7                        xlatb   
0049A9:  26 88 05                     mov      byte ptr es:[di], al
0049AC:  4F                           dec      di
0049AD:  E2 F0                        loop     0x499f
0049AF:  E9 74 FF                     jmp      0x4926
0049B2:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
0049B7:  0B D2                        or       dx, dx
0049B9:  74 16                        je       0x49d1
0049BB:  AC                           lodsb    al, byte ptr [si]
0049BC:  0A C0                        or       al, al
0049BE:  79 09                        jns      0x49c9
0049C0:  F6 D8                        neg      al
0049C2:  FE C0                        inc      al
0049C4:  2B D0                        sub      dx, ax
0049C6:  46                           inc      si
0049C7:  EB EE                        jmp      0x49b7
0049C9:  FE C0                        inc      al
0049CB:  2B D0                        sub      dx, ax
0049CD:  03 F0                        add      si, ax
0049CF:  EB E6                        jmp      0x49b7
0049D1:  5A                           pop      dx
0049D2:  03 FA                        add      di, dx
0049D4:  59                           pop      cx
0049D5:  49                           dec      cx
0049D6:  0F 84 C4 01                  je       0x4b9e
0049DA:  E9 0A FF                     jmp      0x48e7
0049DD:  0A C0                        or       al, al
0049DF:  0F 85 D9 00                  jne      0x4abc
0049E3:  51                           push     cx
0049E4:  52                           push     dx
0049E5:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
0049EA:  0B D2                        or       dx, dx
0049EC:  74 32                        je       0x4a20
0049EE:  AC                           lodsb    al, byte ptr [si]
0049EF:  0A C0                        or       al, al
0049F1:  79 15                        jns      0x4a08
0049F3:  F6 D8                        neg      al
0049F5:  FE C0                        inc      al
0049F7:  2B D0                        sub      dx, ax
0049F9:  79 0A                        jns      0x4a05
0049FB:  F7 DA                        neg      dx
0049FD:  8B CA                        mov      cx, dx
0049FF:  8B D5                        mov      dx, bp
004A01:  2B D1                        sub      dx, cx
004A03:  EB 2E                        jmp      0x4a33
004A05:  46                           inc      si
004A06:  EB E2                        jmp      0x49ea
004A08:  FE C0                        inc      al
004A0A:  2B D0                        sub      dx, ax
004A0C:  79 0E                        jns      0x4a1c
004A0E:  F7 DA                        neg      dx
004A10:  2B C2                        sub      ax, dx
004A12:  03 F0                        add      si, ax
004A14:  8B CA                        mov      cx, dx
004A16:  8B D5                        mov      dx, bp
004A18:  2B D1                        sub      dx, cx
004A1A:  EB 48                        jmp      0x4a64
004A1C:  03 F0                        add      si, ax
004A1E:  EB CA                        jmp      0x49ea
004A20:  8B D5                        mov      dx, bp
004A22:  0B D2                        or       dx, dx
004A24:  74 6B                        je       0x4a91
004A26:  AC                           lodsb    al, byte ptr [si]
004A27:  0A C0                        or       al, al
004A29:  79 33                        jns      0x4a5e
004A2B:  F6 D8                        neg      al
004A2D:  FE C0                        inc      al
004A2F:  8B C8                        mov      cx, ax
004A31:  2B D0                        sub      dx, ax
004A33:  79 1C                        jns      0x4a51
004A35:  F7 DA                        neg      dx
004A37:  2B C2                        sub      ax, dx
004A39:  8B C8                        mov      cx, ax
004A3B:  AC                           lodsb    al, byte ptr [si]
004A3C:  0A C0                        or       al, al
004A3E:  74 04                        je       0x4a44
004A40:  F3 AA                        rep stosb byte ptr es:[di], al
004A42:  EB 02                        jmp      0x4a46
004A44:  03 F9                        add      di, cx
004A46:  8B C2                        mov      ax, dx
004A48:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004A4D:  2B D0                        sub      dx, ax
004A4F:  EB 45                        jmp      0x4a96
004A51:  AC                           lodsb    al, byte ptr [si]
004A52:  0A C0                        or       al, al
004A54:  74 04                        je       0x4a5a
004A56:  F3 AA                        rep stosb byte ptr es:[di], al
004A58:  EB C8                        jmp      0x4a22
004A5A:  03 F9                        add      di, cx
004A5C:  EB C4                        jmp      0x4a22
004A5E:  FE C0                        inc      al
004A60:  8B C8                        mov      cx, ax
004A62:  2B D0                        sub      dx, ax
004A64:  79 1E                        jns      0x4a84
004A66:  F7 DA                        neg      dx
004A68:  2B C2                        sub      ax, dx
004A6A:  8B C8                        mov      cx, ax
004A6C:  AC                           lodsb    al, byte ptr [si]
004A6D:  0A C0                        or       al, al
004A6F:  74 03                        je       0x4a74
004A71:  26 88 05                     mov      byte ptr es:[di], al
004A74:  47                           inc      di
004A75:  E2 F5                        loop     0x4a6c
004A77:  8B C2                        mov      ax, dx
004A79:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004A7E:  2B D0                        sub      dx, ax
004A80:  03 F0                        add      si, ax
004A82:  EB 12                        jmp      0x4a96
004A84:  AC                           lodsb    al, byte ptr [si]
004A85:  0A C0                        or       al, al
004A87:  74 03                        je       0x4a8c
004A89:  26 88 05                     mov      byte ptr es:[di], al
004A8C:  47                           inc      di
004A8D:  E2 F5                        loop     0x4a84
004A8F:  EB 91                        jmp      0x4a22
004A91:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004A96:  0B D2                        or       dx, dx
004A98:  74 16                        je       0x4ab0
004A9A:  AC                           lodsb    al, byte ptr [si]
004A9B:  0A C0                        or       al, al
004A9D:  79 09                        jns      0x4aa8
004A9F:  F6 D8                        neg      al
004AA1:  FE C0                        inc      al
004AA3:  2B D0                        sub      dx, ax
004AA5:  46                           inc      si
004AA6:  EB EE                        jmp      0x4a96
004AA8:  FE C0                        inc      al
004AAA:  2B D0                        sub      dx, ax
004AAC:  03 F0                        add      si, ax
004AAE:  EB E6                        jmp      0x4a96
004AB0:  5A                           pop      dx
004AB1:  03 FA                        add      di, dx
004AB3:  59                           pop      cx
004AB4:  49                           dec      cx
004AB5:  0F 84 E5 00                  je       0x4b9e
004AB9:  E9 27 FF                     jmp      0x49e3
004ABC:  03 D5                        add      dx, bp
004ABE:  03 D5                        add      dx, bp
004AC0:  03 FD                        add      di, bp
004AC2:  4F                           dec      di
004AC3:  51                           push     cx
004AC4:  52                           push     dx
004AC5:  2E 8B 16 2A 17               mov      dx, word ptr cs:[0x172a]
004ACA:  0B D2                        or       dx, dx
004ACC:  74 32                        je       0x4b00
004ACE:  AC                           lodsb    al, byte ptr [si]
004ACF:  0A C0                        or       al, al
004AD1:  79 15                        jns      0x4ae8
004AD3:  F6 D8                        neg      al
004AD5:  FE C0                        inc      al
004AD7:  2B D0                        sub      dx, ax
004AD9:  79 0A                        jns      0x4ae5
004ADB:  F7 DA                        neg      dx
004ADD:  8B CA                        mov      cx, dx
004ADF:  8B D5                        mov      dx, bp
004AE1:  2B D1                        sub      dx, cx
004AE3:  EB 2E                        jmp      0x4b13
004AE5:  46                           inc      si
004AE6:  EB E2                        jmp      0x4aca
004AE8:  FE C0                        inc      al
004AEA:  2B D0                        sub      dx, ax
004AEC:  79 0E                        jns      0x4afc
004AEE:  F7 DA                        neg      dx
004AF0:  2B C2                        sub      ax, dx
004AF2:  03 F0                        add      si, ax
004AF4:  8B CA                        mov      cx, dx
004AF6:  8B D5                        mov      dx, bp
004AF8:  2B D1                        sub      dx, cx
004AFA:  EB 4C                        jmp      0x4b48
004AFC:  03 F0                        add      si, ax
004AFE:  EB CA                        jmp      0x4aca
004B00:  8B D5                        mov      dx, bp
004B02:  0B D2                        or       dx, dx
004B04:  74 6F                        je       0x4b75
004B06:  AC                           lodsb    al, byte ptr [si]
004B07:  0A C0                        or       al, al
004B09:  79 37                        jns      0x4b42
004B0B:  F6 D8                        neg      al
004B0D:  FE C0                        inc      al
004B0F:  8B C8                        mov      cx, ax
004B11:  2B D0                        sub      dx, ax
004B13:  79 1E                        jns      0x4b33
004B15:  F7 DA                        neg      dx
004B17:  2B C2                        sub      ax, dx
004B19:  8B C8                        mov      cx, ax
004B1B:  AC                           lodsb    al, byte ptr [si]
004B1C:  0A C0                        or       al, al
004B1E:  74 06                        je       0x4b26
004B20:  FD                           std     
004B21:  F3 AA                        rep stosb byte ptr es:[di], al
004B23:  FC                           cld     
004B24:  EB 02                        jmp      0x4b28
004B26:  2B F9                        sub      di, cx
004B28:  8B C2                        mov      ax, dx
004B2A:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
004B2F:  2B D0                        sub      dx, ax
004B31:  EB 47                        jmp      0x4b7a
004B33:  AC                           lodsb    al, byte ptr [si]
004B34:  0A C0                        or       al, al
004B36:  74 06                        je       0x4b3e
004B38:  FD                           std     
004B39:  F3 AA                        rep stosb byte ptr es:[di], al
004B3B:  FC                           cld     
004B3C:  EB C4                        jmp      0x4b02
004B3E:  2B F9                        sub      di, cx
004B40:  EB C0                        jmp      0x4b02
004B42:  FE C0                        inc      al
004B44:  8B C8                        mov      cx, ax
004B46:  2B D0                        sub      dx, ax
004B48:  79 1E                        jns      0x4b68
004B4A:  F7 DA                        neg      dx
004B4C:  2B C2                        sub      ax, dx
004B4E:  8B C8                        mov      cx, ax
004B50:  AC                           lodsb    al, byte ptr [si]
004B51:  0A C0                        or       al, al
004B53:  74 03                        je       0x4b58
004B55:  26 88 05                     mov      byte ptr es:[di], al
004B58:  4F                           dec      di
004B59:  E2 F5                        loop     0x4b50
004B5B:  8B C2                        mov      ax, dx
004B5D:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
004B62:  2B D0                        sub      dx, ax
004B64:  03 F0                        add      si, ax
004B66:  EB 12                        jmp      0x4b7a
004B68:  AC                           lodsb    al, byte ptr [si]
004B69:  0A C0                        or       al, al
004B6B:  74 03                        je       0x4b70
004B6D:  26 88 05                     mov      byte ptr es:[di], al
004B70:  4F                           dec      di
004B71:  E2 F5                        loop     0x4b68
004B73:  EB 8D                        jmp      0x4b02
004B75:  2E 8B 16 28 17               mov      dx, word ptr cs:[0x1728]
004B7A:  0B D2                        or       dx, dx
004B7C:  74 16                        je       0x4b94
004B7E:  AC                           lodsb    al, byte ptr [si]
004B7F:  0A C0                        or       al, al
004B81:  79 09                        jns      0x4b8c
004B83:  F6 D8                        neg      al
004B85:  FE C0                        inc      al
004B87:  2B D0                        sub      dx, ax
004B89:  46                           inc      si
004B8A:  EB EE                        jmp      0x4b7a
004B8C:  FE C0                        inc      al
004B8E:  2B D0                        sub      dx, ax
004B90:  03 F0                        add      si, ax
004B92:  EB E6                        jmp      0x4b7a
004B94:  5A                           pop      dx
004B95:  03 FA                        add      di, dx
004B97:  59                           pop      cx
004B98:  49                           dec      cx
004B99:  74 03                        je       0x4b9e
004B9B:  E9 25 FF                     jmp      0x4ac3
004B9E:  5D                           pop      bp
004B9F:  5E                           pop      si
004BA0:  1F                           pop      ds
004BA1:  5F                           pop      di
004BA2:  07                           pop      es
004BA3:  5A                           pop      dx
004BA4:  59                           pop      cx
004BA5:  5B                           pop      bx
004BA6:  58                           pop      ax
004BA7:  C3                           ret     
