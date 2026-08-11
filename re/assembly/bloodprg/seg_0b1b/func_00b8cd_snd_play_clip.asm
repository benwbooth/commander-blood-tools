; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b8cd
; seg_off: 0b1b:011d
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: snd_play_clip
; label_comment: SND clip player (2 calls): test gs:[0xade]&1 (sound enabled); ds=gs; plays a sound clip (AX=clip index). Matches the prior-session finding (dead_ends): the SND player at 0xb8cd, clip*4 table @DS:0x0BBF, final AX via the lcall [0xcdf] driver callback. The audio playback entry
; incoming: call@0x005d71->0b1b:011d
; incoming: call@0x007a2a->0b1b:011d
; incoming: call@0x007bf8->0b1b:011d
; incoming: call@0x007f67->0b1b:011d
; incoming: call@0x00804e->0b1b:011d
; incoming: call@0x0080e5->0b1b:011d
; incoming: call@0x00815b->0b1b:011d
; incoming: call@0x008235->0b1b:011d
; incoming: call@0x008534->0b1b:011d
; incoming: call@0x0086ec->0b1b:011d
; byte_count: 720
; boundary: cfg_blocks_43_terminals_10
; terminal: jmp 0xbb03:3, jmp 0xbb4e:1, jmp 0xbb76:1, jmp 0xbb93:4, retf:1
; direct_callees: 0x00bb9d
; indirect_calls: 8
; cxx_source: re/borland/bloodprg/seg_0b1b/func_00b8cd_snd_play_clip.cpp
; routine_bytes_sha256: 9c7e53a679ab66f26f0621eb7f70b6da0e348778f2b6183da22c1c5b202ab063

00B8CD:  1E                           push     ds
00B8CE:  56                           push     si
00B8CF:  06                           push     es
00B8D0:  57                           push     di
00B8D1:  53                           push     bx
00B8D2:  66 51                        push     ecx
00B8D4:  52                           push     dx
00B8D5:  55                           push     bp
00B8D6:  65 F6 06 DE 0A 01            test     byte ptr gs:[0xade], 1
00B8DC:  0F 84 B3 02                  je       0xbb93
00B8E0:  0F A8                        push     gs
00B8E2:  1F                           pop      ds
00B8E3:  F6 06 A0 0B 02               test     byte ptr [0xba0], 2
00B8E8:  0F 85 31 01                  jne      0xba1d
00B8EC:  0E                           push     cs
00B8ED:  E8 AD 02                     call     0xbb9d
00B8F0:  BE AB 0B                     mov      si, 0xbab
00B8F3:  0B C0                        or       ax, ax
00B8F5:  78 23                        js       0xb91a
00B8F7:  BD BF 0B                     mov      bp, 0xbbf
00B8FA:  C1 E0 02                     shl      ax, 2
00B8FD:  03 E8                        add      bp, ax
00B8FF:  A1 B5 0B                     mov      ax, word ptr [0xbb5]
00B902:  A3 AD 0B                     mov      word ptr [0xbad], ax
00B905:  8B 46 00                     mov      ax, word ptr [bp]
00B908:  A3 AB 0B                     mov      word ptr [0xbab], ax
00B90B:  8B 46 02                     mov      ax, word ptr [bp + 2]
00B90E:  A3 AF 0B                     mov      word ptr [0xbaf], ax
00B911:  33 C0                        xor      ax, ax
00B913:  FF 1E DB 0C                  lcall    [0xcdb]
00B917:  E9 79 02                     jmp      0xbb93
00B91A:  BD 57 0C                     mov      bp, 0xc57
00B91D:  C1 E0 02                     shl      ax, 2
00B920:  03 E8                        add      bp, ax
00B922:  65 83 3E 5C 0A FF            cmp      word ptr gs:[0xa5c], -1
00B928:  74 54                        je       0xb97e
00B92A:  B9 04 00                     mov      cx, 4
00B92D:  66 8B 46 00                  mov      eax, dword ptr [bp]
00B931:  8B F0                        mov      si, ax
00B933:  81 E6 FF 3F                  and      si, 0x3fff
00B937:  66 C1 E8 0E                  shr      eax, 0xe
00B93B:  8B D8                        mov      bx, ax
00B93D:  32 C0                        xor      al, al
00B93F:  65 8B 16 5C 0A               mov      dx, word ptr gs:[0xa5c]
00B944:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00B949:  B4 44                        mov      ah, 0x44
00B94B:  CD 67                        int      0x67
00B94D:  FE C0                        inc      al
00B94F:  43                           inc      bx
00B950:  E2 F7                        loop     0xb949
00B952:  66 8B 46 04                  mov      eax, dword ptr [bp + 4]
00B956:  66 2B 46 00                  sub      eax, dword ptr [bp]
00B95A:  65 C4 3E B7 0B               les      di, ptr gs:[0xbb7]
00B95F:  9A 93 0B CE 01               lcall    0x1ce, 0xb93
00B964:  0F A8                        push     gs
00B966:  1F                           pop      ds
00B967:  BE AB 0B                     mov      si, 0xbab
00B96A:  89 3E AB 0B                  mov      word ptr [0xbab], di
00B96E:  8C 06 AD 0B                  mov      word ptr [0xbad], es
00B972:  A3 AF 0B                     mov      word ptr [0xbaf], ax
00B975:  33 C0                        xor      ax, ax
00B977:  FF 1E DB 0C                  lcall    [0xcdb]
00B97B:  E9 15 02                     jmp      0xbb93
00B97E:  65 83 3E 5A 0A FF            cmp      word ptr gs:[0xa5a], -1
00B984:  74 58                        je       0xb9de
00B986:  8C E8                        mov      ax, gs
00B988:  8E D8                        mov      ds, ax
00B98A:  8E C0                        mov      es, ax
00B98C:  BF 6C 0A                     mov      di, 0xa6c
00B98F:  8B F7                        mov      si, di
00B991:  66 8B 4E 00                  mov      ecx, dword ptr [bp]
00B995:  66 8B 46 04                  mov      eax, dword ptr [bp + 4]
00B999:  66 2B C1                     sub      eax, ecx
00B99C:  50                           push     ax
00B99D:  A8 01                        test     al, 1
00B99F:  74 02                        je       0xb9a3
00B9A1:  66 40                        inc      eax
00B9A3:  66 AB                        stosd    dword ptr es:[di], eax
00B9A5:  A1 5A 0A                     mov      ax, word ptr [0xa5a]
00B9A8:  AB                           stosw    word ptr es:[di], ax
00B9A9:  66 8B C1                     mov      eax, ecx
00B9AC:  66 AB                        stosd    dword ptr es:[di], eax
00B9AE:  33 C0                        xor      ax, ax
00B9B0:  AB                           stosw    word ptr es:[di], ax
00B9B1:  66 A1 B7 0B                  mov      eax, dword ptr [0xbb7]
00B9B5:  66 AB                        stosd    dword ptr es:[di], eax
00B9B7:  66 B8 00 0B 00 00            mov      eax, 0xb00
00B9BD:  FF 1E 4A 0A                  lcall    [0xa4a]
00B9C1:  BE AB 0B                     mov      si, 0xbab
00B9C4:  66 8B 0E B7 0B               mov      ecx, dword ptr [0xbb7]
00B9C9:  66 89 0E AB 0B               mov      dword ptr [0xbab], ecx
00B9CE:  66 33 C9                     xor      ecx, ecx
00B9D1:  58                           pop      ax
00B9D2:  A3 AF 0B                     mov      word ptr [0xbaf], ax
00B9D5:  33 C0                        xor      ax, ax
00B9D7:  FF 1E DB 0C                  lcall    [0xcdb]
00B9DB:  E9 B5 01                     jmp      0xbb93
00B9DE:  65 C5 16 B7 0B               lds      dx, ptr gs:[0xbb7]
00B9E3:  65 8B 1E 47 0C               mov      bx, word ptr gs:[0xc47]
00B9E8:  8B 56 00                     mov      dx, word ptr [bp]
00B9EB:  8B 4E 02                     mov      cx, word ptr [bp + 2]
00B9EE:  B8 00 42                     mov      ax, 0x4200
00B9F1:  CD 21                        int      0x21
00B9F3:  65 8B 16 B7 0B               mov      dx, word ptr gs:[0xbb7]
00B9F8:  66 8B 4E 04                  mov      ecx, dword ptr [bp + 4]
00B9FC:  66 2B 4E 00                  sub      ecx, dword ptr [bp]
00BA00:  B4 3F                        mov      ah, 0x3f
00BA02:  CD 21                        int      0x21
00BA04:  8C DB                        mov      bx, ds
00BA06:  0F A8                        push     gs
00BA08:  1F                           pop      ds
00BA09:  89 16 AB 0B                  mov      word ptr [0xbab], dx
00BA0D:  89 1E AD 0B                  mov      word ptr [0xbad], bx
00BA11:  A3 AF 0B                     mov      word ptr [0xbaf], ax
00BA14:  33 C0                        xor      ax, ax
00BA16:  FF 1E DB 0C                  lcall    [0xcdb]
00BA1A:  E9 76 01                     jmp      0xbb93
00BA1D:  0B C0                        or       ax, ax
00BA1F:  78 18                        js       0xba39
00BA21:  BD BF 0B                     mov      bp, 0xbbf
00BA24:  C1 E0 02                     shl      ax, 2
00BA27:  03 E8                        add      bp, ax
00BA29:  C5 36 B3 0B                  lds      si, ptr [0xbb3]
00BA2D:  03 76 00                     add      si, word ptr [bp]
00BA30:  83 C6 06                     add      si, 6
00BA33:  8B 4E 02                     mov      cx, word ptr [bp + 2]
00BA36:  E9 CA 00                     jmp      0xbb03
00BA39:  BD 57 0C                     mov      bp, 0xc57
00BA3C:  C1 E0 02                     shl      ax, 2
00BA3F:  03 E8                        add      bp, ax
00BA41:  83 3E 5C 0A FF               cmp      word ptr [0xa5c], -1
00BA46:  74 38                        je       0xba80
00BA48:  B9 04 00                     mov      cx, 4
00BA4B:  66 8B 46 00                  mov      eax, dword ptr [bp]
00BA4F:  8B F0                        mov      si, ax
00BA51:  81 E6 FF 3F                  and      si, 0x3fff
00BA55:  66 C1 E8 0E                  shr      eax, 0xe
00BA59:  8B D8                        mov      bx, ax
00BA5B:  32 C0                        xor      al, al
00BA5D:  8B 16 5C 0A                  mov      dx, word ptr [0xa5c]
00BA61:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00BA66:  B4 44                        mov      ah, 0x44
00BA68:  CD 67                        int      0x67
00BA6A:  FE C0                        inc      al
00BA6C:  43                           inc      bx
00BA6D:  E2 F7                        loop     0xba66
00BA6F:  83 C6 06                     add      si, 6
00BA72:  66 8B 4E 04                  mov      ecx, dword ptr [bp + 4]
00BA76:  66 2B 4E 00                  sub      ecx, dword ptr [bp]
00BA7A:  83 E9 06                     sub      cx, 6
00BA7D:  E9 83 00                     jmp      0xbb03
00BA80:  83 3E 5A 0A FF               cmp      word ptr [0xa5a], -1
00BA85:  74 50                        je       0xbad7
00BA87:  8C E8                        mov      ax, gs
00BA89:  8E D8                        mov      ds, ax
00BA8B:  8E C0                        mov      es, ax
00BA8D:  BF 6C 0A                     mov      di, 0xa6c
00BA90:  8B F7                        mov      si, di
00BA92:  66 8B 4E 00                  mov      ecx, dword ptr [bp]
00BA96:  66 8B 46 04                  mov      eax, dword ptr [bp + 4]
00BA9A:  66 2B C1                     sub      eax, ecx
00BA9D:  50                           push     ax
00BA9E:  A8 01                        test     al, 1
00BAA0:  74 02                        je       0xbaa4
00BAA2:  66 40                        inc      eax
00BAA4:  66 AB                        stosd    dword ptr es:[di], eax
00BAA6:  A1 5A 0A                     mov      ax, word ptr [0xa5a]
00BAA9:  AB                           stosw    word ptr es:[di], ax
00BAAA:  66 8B C1                     mov      eax, ecx
00BAAD:  66 AB                        stosd    dword ptr es:[di], eax
00BAAF:  33 C0                        xor      ax, ax
00BAB1:  AB                           stosw    word ptr es:[di], ax
00BAB2:  66 A1 BC 0A                  mov      eax, dword ptr [0xabc]
00BAB6:  05 00 7D                     add      ax, 0x7d00
00BAB9:  66 AB                        stosd    dword ptr es:[di], eax
00BABB:  66 B8 00 0B 00 00            mov      eax, 0xb00
00BAC1:  FF 1E 4A 0A                  lcall    [0xa4a]
00BAC5:  59                           pop      cx
00BAC6:  66 0F B7 C9                  movzx    ecx, cx
00BACA:  83 E9 06                     sub      cx, 6
00BACD:  C5 36 BC 0A                  lds      si, ptr [0xabc]
00BAD1:  81 C6 06 7D                  add      si, 0x7d06
00BAD5:  EB 2C                        jmp      0xbb03
00BAD7:  65 C5 16 BC 0A               lds      dx, ptr gs:[0xabc]
00BADC:  65 8B 1E 47 0C               mov      bx, word ptr gs:[0xc47]
00BAE1:  8B 56 00                     mov      dx, word ptr [bp]
00BAE4:  8B 4E 02                     mov      cx, word ptr [bp + 2]
00BAE7:  B8 00 42                     mov      ax, 0x4200
00BAEA:  CD 21                        int      0x21
00BAEC:  BA 00 7D                     mov      dx, 0x7d00
00BAEF:  66 8B 4E 04                  mov      ecx, dword ptr [bp + 4]
00BAF3:  66 2B 4E 00                  sub      ecx, dword ptr [bp]
00BAF7:  B4 3F                        mov      ah, 0x3f
00BAF9:  CD 21                        int      0x21
00BAFB:  8B C8                        mov      cx, ax
00BAFD:  83 E9 06                     sub      cx, 6
00BB00:  BE 06 7D                     mov      si, 0x7d06
00BB03:  65 F6 06 A2 0B 01            test     byte ptr gs:[0xba2], 1
00BB09:  74 02                        je       0xbb0d
00BB0B:  03 C9                        add      cx, cx
00BB0D:  BD 89 0B                     mov      bp, 0xb89
00BB10:  BA 91 0B                     mov      dx, 0xb91
00BB13:  80 7E 06 03                  cmp      byte ptr [bp + 6], 3
00BB17:  74 08                        je       0xbb21
00BB19:  87 EA                        xchg     dx, bp
00BB1B:  80 7E 06 03                  cmp      byte ptr [bp + 6], 3
00BB1F:  75 72                        jne      0xbb93
00BB21:  C4 7E 00                     les      di, ptr [bp]
00BB24:  83 C7 06                     add      di, 6
00BB27:  52                           push     dx
00BB28:  65 FF 1E F3 0C               lcall    gs:[0xcf3]
00BB2D:  5A                           pop      dx
00BB2E:  83 F8 FF                     cmp      ax, -1
00BB31:  74 60                        je       0xbb93
00BB33:  2B 46 04                     sub      ax, word ptr [bp + 4]
00BB36:  79 02                        jns      0xbb3a
00BB38:  F7 D8                        neg      ax
00BB3A:  8B D9                        mov      bx, cx
00BB3C:  3B 46 04                     cmp      ax, word ptr [bp + 4]
00BB3F:  73 35                        jae      0xbb76
00BB41:  03 F8                        add      di, ax
00BB43:  2B 46 04                     sub      ax, word ptr [bp + 4]
00BB46:  F7 D8                        neg      ax
00BB48:  2B D8                        sub      bx, ax
00BB4A:  78 02                        js       0xbb4e
00BB4C:  8B C8                        mov      cx, ax
00BB4E:  49                           dec      cx
00BB4F:  74 25                        je       0xbb76
00BB51:  78 23                        js       0xbb76
00BB53:  65 F6 06 A2 0B 01            test     byte ptr gs:[0xba2], 1
00BB59:  74 12                        je       0xbb6d
00BB5B:  8A 04                        mov      al, byte ptr [si]
00BB5D:  F6 C1 01                     test     cl, 1
00BB60:  75 01                        jne      0xbb63
00BB62:  46                           inc      si
00BB63:  26 02 05                     add      al, byte ptr es:[di]
00BB66:  D0 D8                        rcr      al, 1
00BB68:  AA                           stosb    byte ptr es:[di], al
00BB69:  E2 F0                        loop     0xbb5b
00BB6B:  EB 09                        jmp      0xbb76
00BB6D:  AC                           lodsb    al, byte ptr [si]
00BB6E:  26 02 05                     add      al, byte ptr es:[di]
00BB71:  D0 D8                        rcr      al, 1
00BB73:  AA                           stosb    byte ptr es:[di], al
00BB74:  E2 F7                        loop     0xbb6d
00BB76:  0B DB                        or       bx, bx
00BB78:  78 19                        js       0xbb93
00BB7A:  74 17                        je       0xbb93
00BB7C:  8B EA                        mov      bp, dx
00BB7E:  8B CB                        mov      cx, bx
00BB80:  3B 4E 04                     cmp      cx, word ptr [bp + 4]
00BB83:  76 03                        jbe      0xbb88
00BB85:  8B 4E 04                     mov      cx, word ptr [bp + 4]
00BB88:  BB FF FF                     mov      bx, 0xffff
00BB8B:  C4 7E 00                     les      di, ptr [bp]
00BB8E:  83 C7 06                     add      di, 6
00BB91:  EB BB                        jmp      0xbb4e
00BB93:  5D                           pop      bp
00BB94:  5A                           pop      dx
00BB95:  66 59                        pop      ecx
00BB97:  5B                           pop      bx
00BB98:  5F                           pop      di
00BB99:  07                           pop      es
00BB9A:  5E                           pop      si
00BB9B:  1F                           pop      ds
00BB9C:  CB                           retf    
