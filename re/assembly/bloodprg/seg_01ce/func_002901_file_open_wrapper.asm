; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002901
; seg_off: 01ce:0621
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: file_open_wrapper
; label_comment: file-open wrapper: dx=si (filename); push cs; call path_builder_gs_relative 0x2693; then DOS-open the assembled path. A resource open via the path builder
; incoming: call@0x007747->01ce:0621
; byte_count: 241
; boundary: cfg_blocks_14_terminals_3
; terminal: jmp 0x29e5:1, jmp 0x29e8:1, retf:1
; direct_callees: 0x002693
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_01ce/func_002901_file_open_wrapper.cpp
; routine_bytes_sha256: 7a06ef7c766e9919b6c8ca3a09ae979c3b659525dec95187edf2b171a94ded45

002901:  66 50                        push     eax
002903:  53                           push     bx
002904:  51                           push     cx
002905:  52                           push     dx
002906:  06                           push     es
002907:  1E                           push     ds
002908:  56                           push     si
002909:  57                           push     di
00290A:  06                           push     es
00290B:  8B D6                        mov      dx, si
00290D:  0E                           push     cs
00290E:  E8 82 FD                     call     0x2693
002911:  8B C3                        mov      ax, bx
002913:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
002919:  75 2D                        jne      0x2948
00291B:  B8 00 2F                     mov      ax, 0x2f00
00291E:  CD 21                        int      0x21
002920:  8B F3                        mov      si, bx
002922:  83 C6 1A                     add      si, 0x1a
002925:  33 C9                        xor      cx, cx
002927:  B8 00 4E                     mov      ax, 0x4e00
00292A:  CD 21                        int      0x21
00292C:  66 26 8B 04                  mov      eax, dword ptr es:[si]
002930:  66 65 A3 8E 0A               mov      dword ptr gs:[0xa8e], eax
002935:  66 65 A3 92 0A               mov      dword ptr gs:[0xa92], eax
00293A:  B8 00 3D                     mov      ax, 0x3d00
00293D:  CD 21                        int      0x21
00293F:  73 07                        jae      0x2948
002941:  B8 01 00                     mov      ax, 1
002944:  1F                           pop      ds
002945:  E9 9D 00                     jmp      0x29e5
002948:  1F                           pop      ds
002949:  65 A3 84 0A                  mov      word ptr gs:[0xa84], ax
00294D:  8B D7                        mov      dx, di
00294F:  8C E8                        mov      ax, gs
002951:  8E C0                        mov      es, ax
002953:  BE 6C 0A                     mov      si, 0xa6c
002956:  66 65 C7 06 4E 0A 00 00 00 00 mov      dword ptr gs:[0xa4e], 0
002960:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002965:  B9 00 7D                     mov      cx, 0x7d00
002968:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
00296D:  66 2B C1                     sub      eax, ecx
002970:  79 02                        jns      0x2974
002972:  03 C8                        add      cx, ax
002974:  B8 00 3F                     mov      ax, 0x3f00
002977:  CD 21                        int      0x21
002979:  66 0F B7 C0                  movzx    eax, ax
00297D:  66 65 29 06 92 0A            sub      dword ptr gs:[0xa92], eax
002983:  8B FE                        mov      di, si
002985:  1E                           push     ds
002986:  A8 01                        test     al, 1
002988:  74 02                        je       0x298c
00298A:  66 40                        inc      eax
00298C:  66 AB                        stosd    dword ptr es:[di], eax
00298E:  33 C0                        xor      ax, ax
002990:  AB                           stosw    word ptr es:[di], ax
002991:  8B C2                        mov      ax, dx
002993:  AB                           stosw    word ptr es:[di], ax
002994:  8C D8                        mov      ax, ds
002996:  AB                           stosw    word ptr es:[di], ax
002997:  65 A1 56 0A                  mov      ax, word ptr gs:[0xa56]
00299B:  AB                           stosw    word ptr es:[di], ax
00299C:  66 65 A1 4E 0A               mov      eax, dword ptr gs:[0xa4e]
0029A1:  66 AB                        stosd    dword ptr es:[di], eax
0029A3:  66 65 81 06 4E 0A 00 7D 00 00 add      dword ptr gs:[0xa4e], 0x7d00
0029AD:  8C C0                        mov      ax, es
0029AF:  8E D8                        mov      ds, ax
0029B1:  66 B8 00 0B 00 00            mov      eax, 0xb00
0029B7:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
0029BC:  1F                           pop      ds
0029BD:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
0029C2:  66 0B C0                     or       eax, eax
0029C5:  75 99                        jne      0x2960
0029C7:  65 F6 06 E2 0A 01            test     byte ptr gs:[0xae2], 1
0029CD:  75 0A                        jne      0x29d9
0029CF:  B8 00 3E                     mov      ax, 0x3e00
0029D2:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
0029D7:  CD 21                        int      0x21
0029D9:  66 65 A1 8E 0A               mov      eax, dword ptr gs:[0xa8e]
0029DE:  66 65 A3 52 0A               mov      dword ptr gs:[0xa52], eax
0029E3:  EB 03                        jmp      0x29e8
0029E5:  66 33 C0                     xor      eax, eax
0029E8:  5F                           pop      di
0029E9:  5E                           pop      si
0029EA:  1F                           pop      ds
0029EB:  07                           pop      es
0029EC:  5A                           pop      dx
0029ED:  59                           pop      cx
0029EE:  5B                           pop      bx
0029EF:  66 58                        pop      eax
0029F1:  CB                           retf    
