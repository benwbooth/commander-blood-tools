; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a914
; seg_off: 0971:0c04
; group: seg_0971
; provenance: recursive_graph
; byte_count: 424
; boundary: cfg_blocks_61_terminals_23
; terminal: jmp 0xa956:10, jmp 0xaa03:1, jmp 0xaa0e:9, jmp 0xaa1c:1, ret:2
; direct_callees: 0x00aabc
; indirect_calls: 0
; routine_bytes_sha256: b8b6aa14a6315aa039a8e09eb970cc390244b135db8cc51e7ce921ddbfec946d

00A914:  8B DE                        mov      bx, si
00A916:  8A 44 04                     mov      al, byte ptr [si + 4]
00A919:  83 C6 06                     add      si, 6
00A91C:  A8 04                        test     al, 4
00A91E:  75 02                        jne      0xa922
00A920:  A5                           movsw    word ptr es:[di], word ptr [si]
00A921:  A5                           movsw    word ptr es:[di], word ptr [si]
00A922:  50                           push     ax
00A923:  24 40                        and      al, 0x40
00A925:  02 C0                        add      al, al
00A927:  2E A2 DD 0D                  mov      byte ptr cs:[0xddd], al
00A92B:  2E A2 0D 0E                  mov      byte ptr cs:[0xe0d], al
00A92F:  8B EF                        mov      bp, di
00A931:  03 2F                        add      bp, word ptr [bx]
00A933:  57                           push     di
00A934:  8B FD                        mov      di, bp
00A936:  2B 7F 02                     sub      di, word ptr [bx + 2]
00A939:  57                           push     di
00A93A:  E8 7F 01                     call     0xaabc
00A93D:  5E                           pop      si
00A93E:  5F                           pop      di
00A93F:  58                           pop      ax
00A940:  33 C9                        xor      cx, cx
00A942:  3C 80                        cmp      al, 0x80
00A944:  72 06                        jb       0xa94c
00A946:  E9 BA 00                     jmp      0xaa03
; -- non-contiguous block: next 0x00a94c --
00A94C:  8B 17                        mov      dx, word ptr [bx]
00A94E:  43                           inc      bx
00A94F:  13 D2                        adc      dx, dx
00A951:  43                           inc      bx
00A952:  72 08                        jb       0xa95c
00A954:  26 A4                        movsb    byte ptr es:[di], byte ptr es:[si]
00A956:  03 D2                        add      dx, dx
00A958:  73 FA                        jae      0xa954
00A95A:  74 F0                        je       0xa94c
00A95C:  26 AC                        lodsb    al, byte ptr es:[si]
00A95E:  8A E0                        mov      ah, al
00A960:  03 D2                        add      dx, dx
00A962:  72 0E                        jb       0xa972
00A964:  AB                           stosw    word ptr es:[di], ax
00A965:  EB EF                        jmp      0xa956
00A967:  8B 17                        mov      dx, word ptr [bx]
00A969:  43                           inc      bx
00A96A:  13 D2                        adc      dx, dx
00A96C:  43                           inc      bx
00A96D:  72 05                        jb       0xa974
00A96F:  AB                           stosw    word ptr es:[di], ax
00A970:  EB E4                        jmp      0xa956
00A972:  74 F3                        je       0xa967
00A974:  03 D2                        add      dx, dx
00A976:  72 18                        jb       0xa990
00A978:  26 88 05                     mov      byte ptr es:[di], al
00A97B:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00A97F:  83 C7 03                     add      di, 3
00A982:  EB D2                        jmp      0xa956
00A984:  8B 17                        mov      dx, word ptr [bx]
00A986:  43                           inc      bx
00A987:  13 D2                        adc      dx, dx
00A989:  43                           inc      bx
00A98A:  72 06                        jb       0xa992
00A98C:  AB                           stosw    word ptr es:[di], ax
00A98D:  AA                           stosb    byte ptr es:[di], al
00A98E:  EB C6                        jmp      0xa956
00A990:  74 F2                        je       0xa984
00A992:  03 D2                        add      dx, dx
00A994:  72 18                        jb       0xa9ae
00A996:  26 89 05                     mov      word ptr es:[di], ax
00A999:  26 89 45 02                  mov      word ptr es:[di + 2], ax
00A99D:  83 C7 04                     add      di, 4
00A9A0:  EB B4                        jmp      0xa956
00A9A2:  8B 17                        mov      dx, word ptr [bx]
00A9A4:  43                           inc      bx
00A9A5:  13 D2                        adc      dx, dx
00A9A7:  43                           inc      bx
00A9A8:  72 06                        jb       0xa9b0
00A9AA:  AB                           stosw    word ptr es:[di], ax
00A9AB:  AB                           stosw    word ptr es:[di], ax
00A9AC:  EB A8                        jmp      0xa956
00A9AE:  74 F2                        je       0xa9a2
00A9B0:  3B FD                        cmp      di, bp
00A9B2:  73 4E                        jae      0xaa02
00A9B4:  83 F9 04                     cmp      cx, 4
00A9B7:  72 16                        jb       0xa9cf
00A9B9:  74 04                        je       0xa9bf
00A9BB:  F3 AA                        rep stosb byte ptr es:[di], al
00A9BD:  EB 97                        jmp      0xa956
00A9BF:  8A 0F                        mov      cl, byte ptr [bx]
00A9C1:  83 C1 14                     add      cx, 0x14
00A9C4:  D1 E9                        shr      cx, 1
00A9C6:  F3 AB                        rep stosw word ptr es:[di], ax
00A9C8:  13 C9                        adc      cx, cx
00A9CA:  43                           inc      bx
00A9CB:  F3 AA                        rep stosb byte ptr es:[di], al
00A9CD:  EB 87                        jmp      0xa956
00A9CF:  8A 0F                        mov      cl, byte ptr [bx]
00A9D1:  51                           push     cx
00A9D2:  C0 E9 04                     shr      cl, 4
00A9D5:  74 10                        je       0xa9e7
00A9D7:  80 C1 04                     add      cl, 4
00A9DA:  F3 AA                        rep stosb byte ptr es:[di], al
00A9DC:  43                           inc      bx
00A9DD:  59                           pop      cx
00A9DE:  80 E1 0F                     and      cl, 0xf
00A9E1:  83 C1 04                     add      cx, 4
00A9E4:  E9 6F FF                     jmp      0xa956
00A9E7:  8A 4F 01                     mov      cl, byte ptr [bx + 1]
00A9EA:  83 C1 14                     add      cx, 0x14
00A9ED:  D1 E9                        shr      cx, 1
00A9EF:  F3 AB                        rep stosw word ptr es:[di], ax
00A9F1:  13 C9                        adc      cx, cx
00A9F3:  F3 AA                        rep stosb byte ptr es:[di], al
00A9F5:  59                           pop      cx
00A9F6:  83 C3 02                     add      bx, 2
00A9F9:  80 E1 0F                     and      cl, 0xf
00A9FC:  83 C1 04                     add      cx, 4
00A9FF:  E9 54 FF                     jmp      0xa956
00AA02:  C3                           ret     
00AA03:  F9                           stc     
00AA04:  8B 17                        mov      dx, word ptr [bx]
00AA06:  43                           inc      bx
00AA07:  13 D2                        adc      dx, dx
00AA09:  43                           inc      bx
00AA0A:  72 08                        jb       0xaa14
00AA0C:  26 A4                        movsb    byte ptr es:[di], byte ptr es:[si]
00AA0E:  03 D2                        add      dx, dx
00AA10:  73 FA                        jae      0xaa0c
00AA12:  74 F0                        je       0xaa04
00AA14:  26 AC                        lodsb    al, byte ptr es:[si]
00AA16:  8A E0                        mov      ah, al
00AA18:  03 D2                        add      dx, dx
00AA1A:  72 56                        jb       0xaa72
00AA1C:  83 F9 04                     cmp      cx, 4
00AA1F:  72 16                        jb       0xaa37
00AA21:  74 04                        je       0xaa27
00AA23:  F3 AA                        rep stosb byte ptr es:[di], al
00AA25:  EB E7                        jmp      0xaa0e
00AA27:  8A 0F                        mov      cl, byte ptr [bx]
00AA29:  83 C1 14                     add      cx, 0x14
00AA2C:  D1 E9                        shr      cx, 1
00AA2E:  F3 AB                        rep stosw word ptr es:[di], ax
00AA30:  13 C9                        adc      cx, cx
00AA32:  43                           inc      bx
00AA33:  F3 AA                        rep stosb byte ptr es:[di], al
00AA35:  EB D7                        jmp      0xaa0e
00AA37:  8A 0F                        mov      cl, byte ptr [bx]
00AA39:  51                           push     cx
00AA3A:  C0 E9 04                     shr      cl, 4
00AA3D:  74 0F                        je       0xaa4e
00AA3F:  80 C1 04                     add      cl, 4
00AA42:  F3 AA                        rep stosb byte ptr es:[di], al
00AA44:  43                           inc      bx
00AA45:  59                           pop      cx
00AA46:  80 E1 0F                     and      cl, 0xf
00AA49:  83 C1 04                     add      cx, 4
00AA4C:  EB C0                        jmp      0xaa0e
00AA4E:  8A 4F 01                     mov      cl, byte ptr [bx + 1]
00AA51:  83 C1 14                     add      cx, 0x14
00AA54:  D1 E9                        shr      cx, 1
00AA56:  F3 AB                        rep stosw word ptr es:[di], ax
00AA58:  13 C9                        adc      cx, cx
00AA5A:  F3 AA                        rep stosb byte ptr es:[di], al
00AA5C:  59                           pop      cx
00AA5D:  83 C3 02                     add      bx, 2
00AA60:  80 E1 0F                     and      cl, 0xf
00AA63:  83 C1 04                     add      cx, 4
00AA66:  EB A6                        jmp      0xaa0e
00AA68:  8B 17                        mov      dx, word ptr [bx]
00AA6A:  43                           inc      bx
00AA6B:  13 D2                        adc      dx, dx
00AA6D:  43                           inc      bx
00AA6E:  72 04                        jb       0xaa74
00AA70:  EB AA                        jmp      0xaa1c
00AA72:  74 F4                        je       0xaa68
00AA74:  03 D2                        add      dx, dx
00AA76:  72 10                        jb       0xaa88
00AA78:  AB                           stosw    word ptr es:[di], ax
00AA79:  EB 93                        jmp      0xaa0e
00AA7B:  8B 17                        mov      dx, word ptr [bx]
00AA7D:  83 C3 02                     add      bx, 2
00AA80:  F9                           stc     
00AA81:  13 D2                        adc      dx, dx
00AA83:  72 05                        jb       0xaa8a
00AA85:  AB                           stosw    word ptr es:[di], ax
00AA86:  EB 86                        jmp      0xaa0e
00AA88:  74 F1                        je       0xaa7b
00AA8A:  03 D2                        add      dx, dx
00AA8C:  72 1A                        jb       0xaaa8
00AA8E:  26 88 05                     mov      byte ptr es:[di], al
00AA91:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00AA95:  83 C7 03                     add      di, 3
00AA98:  E9 73 FF                     jmp      0xaa0e
00AA9B:  8B 17                        mov      dx, word ptr [bx]
00AA9D:  43                           inc      bx
00AA9E:  13 D2                        adc      dx, dx
00AAA0:  43                           inc      bx
00AAA1:  72 07                        jb       0xaaaa
00AAA3:  AB                           stosw    word ptr es:[di], ax
00AAA4:  AA                           stosb    byte ptr es:[di], al
00AAA5:  E9 66 FF                     jmp      0xaa0e
00AAA8:  74 F1                        je       0xaa9b
00AAAA:  3B FD                        cmp      di, bp
00AAAC:  73 0D                        jae      0xaabb
00AAAE:  26 89 05                     mov      word ptr es:[di], ax
00AAB1:  26 89 45 02                  mov      word ptr es:[di + 2], ax
00AAB5:  83 C7 04                     add      di, 4
00AAB8:  E9 53 FF                     jmp      0xaa0e
00AABB:  C3                           ret     
