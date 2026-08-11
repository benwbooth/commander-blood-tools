; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004cd6
; seg_off: 0299:1d46
; group: seg_0299
; provenance: static_dispatch_table_target
; label: entity_coord_accumulate
; label_comment: entity coordinate accumulate: lds si,[di+4] (entity record's +4/+6 data far ptr); add ax,[si+4]; add bx,[si+6]; add dx,[si+4]. Sums the entity's stored x/y offsets into the running screen position
; incoming: sprite_blitter_candidates:blit_3
; byte_count: 652
; boundary: cfg_blocks_83_terminals_25
; terminal: jmp 0x4d34:1, jmp 0x4d6d:1, jmp 0x4dd9:1, jmp 0x4ddf:2, jmp 0x4e17:2, jmp 0x4e28:1, jmp 0x4e49:1, jmp 0x4e69:4, jmp 0x4e95:1, jmp 0x4e9b:2, jmp 0x4ed3:2, jmp 0x4ee4:1, jmp 0x4f0b:1, jmp 0x4f35:4, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 15c6e9bd08ba4bb1285c354f4e036a89ac314ddf5015198454351fdc477baa21

004CD6:  50                           push     ax
004CD7:  53                           push     bx
004CD8:  51                           push     cx
004CD9:  52                           push     dx
004CDA:  06                           push     es
004CDB:  57                           push     di
004CDC:  1E                           push     ds
004CDD:  56                           push     si
004CDE:  55                           push     bp
004CDF:  C5 75 04                     lds      si, ptr [di + 4]
004CE2:  03 44 04                     add      ax, word ptr [si + 4]
004CE5:  03 5C 06                     add      bx, word ptr [si + 6]
004CE8:  03 54 04                     add      dx, word ptr [si + 4]
004CEB:  03 6C 06                     add      bp, word ptr [si + 6]
004CEE:  66 2E C7 06 28 17 00 00 00 00 mov      dword ptr cs:[0x1728], 0
004CF8:  8B 04                        mov      ax, word ptr [si]
004CFA:  2E A3 26 17                  mov      word ptr cs:[0x1726], ax
004CFE:  52                           push     dx
004CFF:  83 C6 08                     add      si, 8
004D02:  26 8B 4D 0E                  mov      cx, word ptr es:[di + 0xe]
004D06:  8B C3                        mov      ax, bx
004D08:  26 2B 45 1C                  sub      ax, word ptr es:[di + 0x1c]
004D0C:  79 31                        jns      0x4d3f
004D0E:  F7 D8                        neg      ax
004D10:  2B C8                        sub      cx, ax
004D12:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004D18:  75 21                        jne      0x4d3b
004D1A:  51                           push     cx
004D1B:  8B C8                        mov      cx, ax
004D1D:  32 E4                        xor      ah, ah
004D1F:  2E 8B 1E 26 17               mov      bx, word ptr cs:[0x1726]
004D24:  AC                           lodsb    al, byte ptr [si]
004D25:  0A C0                        or       al, al
004D27:  79 07                        jns      0x4d30
004D29:  F6 D8                        neg      al
004D2B:  FE C0                        inc      al
004D2D:  46                           inc      si
004D2E:  EB 04                        jmp      0x4d34
004D30:  FE C0                        inc      al
004D32:  03 F0                        add      si, ax
004D34:  2B D8                        sub      bx, ax
004D36:  75 EC                        jne      0x4d24
004D38:  E2 E5                        loop     0x4d1f
004D3A:  59                           pop      cx
004D3B:  26 8B 5D 1C                  mov      bx, word ptr es:[di + 0x1c]
004D3F:  8B C5                        mov      ax, bp
004D41:  26 2B 45 1E                  sub      ax, word ptr es:[di + 0x1e]
004D45:  78 2D                        js       0x4d74
004D47:  74 2B                        je       0x4d74
004D49:  2B C8                        sub      cx, ax
004D4B:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004D51:  74 21                        je       0x4d74
004D53:  51                           push     cx
004D54:  8B C8                        mov      cx, ax
004D56:  32 E4                        xor      ah, ah
004D58:  2E 8B 16 26 17               mov      dx, word ptr cs:[0x1726]
004D5D:  AC                           lodsb    al, byte ptr [si]
004D5E:  0A C0                        or       al, al
004D60:  79 07                        jns      0x4d69
004D62:  F6 D8                        neg      al
004D64:  FE C0                        inc      al
004D66:  46                           inc      si
004D67:  EB 04                        jmp      0x4d6d
004D69:  FE C0                        inc      al
004D6B:  03 F0                        add      si, ax
004D6D:  2B D0                        sub      dx, ax
004D6F:  75 EC                        jne      0x4d5d
004D71:  E2 E5                        loop     0x4d58
004D73:  59                           pop      cx
004D74:  26 8B 6D 0C                  mov      bp, word ptr es:[di + 0xc]
004D78:  26 8B 55 08                  mov      dx, word ptr es:[di + 8]
004D7C:  03 54 FC                     add      dx, word ptr [si - 4]
004D7F:  8B C2                        mov      ax, dx
004D81:  26 2B 45 18                  sub      ax, word ptr es:[di + 0x18]
004D85:  79 0C                        jns      0x4d93
004D87:  F7 D8                        neg      ax
004D89:  2B E8                        sub      bp, ax
004D8B:  2E A3 28 17                  mov      word ptr cs:[0x1728], ax
004D8F:  26 8B 55 18                  mov      dx, word ptr es:[di + 0x18]
004D93:  58                           pop      ax
004D94:  26 2B 45 1A                  sub      ax, word ptr es:[di + 0x1a]
004D98:  78 06                        js       0x4da0
004D9A:  2B E8                        sub      bp, ax
004D9C:  2E A3 2A 17                  mov      word ptr cs:[0x172a], ax
004DA0:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
004DA5:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004DAB:  74 03                        je       0x4db0
004DAD:  03 D9                        add      bx, cx
004DAF:  4B                           dec      bx
004DB0:  8B C3                        mov      ax, bx
004DB2:  86 C4                        xchg     ah, al
004DB4:  C1 E3 06                     shl      bx, 6
004DB7:  03 C3                        add      ax, bx
004DB9:  03 F8                        add      di, ax
004DBB:  03 FA                        add      di, dx
004DBD:  BA 40 01                     mov      dx, 0x140
004DC0:  2B D5                        sub      dx, bp
004DC2:  32 E4                        xor      ah, ah
004DC4:  2E 8B 1E DF 14               mov      bx, word ptr cs:[0x14df]
004DC9:  0A FF                        or       bh, bh
004DCB:  74 06                        je       0x4dd3
004DCD:  03 D5                        add      dx, bp
004DCF:  03 D5                        add      dx, bp
004DD1:  F7 DA                        neg      dx
004DD3:  0A DB                        or       bl, bl
004DD5:  0F 85 B5 00                  jne      0x4e8e
004DD9:  51                           push     cx
004DDA:  2E 8B 1E 28 17               mov      bx, word ptr cs:[0x1728]
004DDF:  0B DB                        or       bx, bx
004DE1:  74 32                        je       0x4e15
004DE3:  AC                           lodsb    al, byte ptr [si]
004DE4:  0A C0                        or       al, al
004DE6:  79 15                        jns      0x4dfd
004DE8:  F6 D8                        neg      al
004DEA:  FE C0                        inc      al
004DEC:  2B D8                        sub      bx, ax
004DEE:  79 0A                        jns      0x4dfa
004DF0:  F7 DB                        neg      bx
004DF2:  8B CB                        mov      cx, bx
004DF4:  8B DD                        mov      bx, bp
004DF6:  2B D9                        sub      bx, cx
004DF8:  EB 2E                        jmp      0x4e28
004DFA:  46                           inc      si
004DFB:  EB E2                        jmp      0x4ddf
004DFD:  FE C0                        inc      al
004DFF:  2B D8                        sub      bx, ax
004E01:  79 0E                        jns      0x4e11
004E03:  F7 DB                        neg      bx
004E05:  2B C3                        sub      ax, bx
004E07:  03 F0                        add      si, ax
004E09:  8B CB                        mov      cx, bx
004E0B:  8B DD                        mov      bx, bp
004E0D:  2B D9                        sub      bx, cx
004E0F:  EB 38                        jmp      0x4e49
004E11:  03 F0                        add      si, ax
004E13:  EB CA                        jmp      0x4ddf
004E15:  8B DD                        mov      bx, bp
004E17:  0B DB                        or       bx, bx
004E19:  74 49                        je       0x4e64
004E1B:  AC                           lodsb    al, byte ptr [si]
004E1C:  0A C0                        or       al, al
004E1E:  79 23                        jns      0x4e43
004E20:  F6 D8                        neg      al
004E22:  FE C0                        inc      al
004E24:  8B C8                        mov      cx, ax
004E26:  2B D8                        sub      bx, ax
004E28:  79 14                        jns      0x4e3e
004E2A:  F7 DB                        neg      bx
004E2C:  2B C3                        sub      ax, bx
004E2E:  8B C8                        mov      cx, ax
004E30:  AC                           lodsb    al, byte ptr [si]
004E31:  F3 AA                        rep stosb byte ptr es:[di], al
004E33:  8B C3                        mov      ax, bx
004E35:  2E 8B 1E 2A 17               mov      bx, word ptr cs:[0x172a]
004E3A:  2B D8                        sub      bx, ax
004E3C:  EB 2B                        jmp      0x4e69
004E3E:  AC                           lodsb    al, byte ptr [si]
004E3F:  F3 AA                        rep stosb byte ptr es:[di], al
004E41:  EB D4                        jmp      0x4e17
004E43:  FE C0                        inc      al
004E45:  8B C8                        mov      cx, ax
004E47:  2B D8                        sub      bx, ax
004E49:  79 15                        jns      0x4e60
004E4B:  F7 DB                        neg      bx
004E4D:  2B C3                        sub      ax, bx
004E4F:  8B C8                        mov      cx, ax
004E51:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
004E53:  8B C3                        mov      ax, bx
004E55:  2E 8B 1E 2A 17               mov      bx, word ptr cs:[0x172a]
004E5A:  2B D8                        sub      bx, ax
004E5C:  03 F0                        add      si, ax
004E5E:  EB 09                        jmp      0x4e69
004E60:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
004E62:  EB B3                        jmp      0x4e17
004E64:  2E 8B 1E 2A 17               mov      bx, word ptr cs:[0x172a]
004E69:  0B DB                        or       bx, bx
004E6B:  74 16                        je       0x4e83
004E6D:  AC                           lodsb    al, byte ptr [si]
004E6E:  0A C0                        or       al, al
004E70:  79 09                        jns      0x4e7b
004E72:  F6 D8                        neg      al
004E74:  FE C0                        inc      al
004E76:  2B D8                        sub      bx, ax
004E78:  46                           inc      si
004E79:  EB EE                        jmp      0x4e69
004E7B:  FE C0                        inc      al
004E7D:  2B D8                        sub      bx, ax
004E7F:  03 F0                        add      si, ax
004E81:  EB E6                        jmp      0x4e69
004E83:  03 FA                        add      di, dx
004E85:  59                           pop      cx
004E86:  49                           dec      cx
004E87:  0F 84 CD 00                  je       0x4f58
004E8B:  E9 4B FF                     jmp      0x4dd9
004E8E:  03 D5                        add      dx, bp
004E90:  03 D5                        add      dx, bp
004E92:  03 FD                        add      di, bp
004E94:  4F                           dec      di
004E95:  51                           push     cx
004E96:  2E 8B 1E 2A 17               mov      bx, word ptr cs:[0x172a]
004E9B:  0B DB                        or       bx, bx
004E9D:  74 32                        je       0x4ed1
004E9F:  AC                           lodsb    al, byte ptr [si]
004EA0:  0A C0                        or       al, al
004EA2:  79 15                        jns      0x4eb9
004EA4:  F6 D8                        neg      al
004EA6:  FE C0                        inc      al
004EA8:  2B D8                        sub      bx, ax
004EAA:  79 0A                        jns      0x4eb6
004EAC:  F7 DB                        neg      bx
004EAE:  8B CB                        mov      cx, bx
004EB0:  8B DD                        mov      bx, bp
004EB2:  2B D9                        sub      bx, cx
004EB4:  EB 2E                        jmp      0x4ee4
004EB6:  46                           inc      si
004EB7:  EB E2                        jmp      0x4e9b
004EB9:  FE C0                        inc      al
004EBB:  2B D8                        sub      bx, ax
004EBD:  79 0E                        jns      0x4ecd
004EBF:  F7 DB                        neg      bx
004EC1:  2B C3                        sub      ax, bx
004EC3:  03 F0                        add      si, ax
004EC5:  8B CB                        mov      cx, bx
004EC7:  8B DD                        mov      bx, bp
004EC9:  2B D9                        sub      bx, cx
004ECB:  EB 3E                        jmp      0x4f0b
004ECD:  03 F0                        add      si, ax
004ECF:  EB CA                        jmp      0x4e9b
004ED1:  8B DD                        mov      bx, bp
004ED3:  0B DB                        or       bx, bx
004ED5:  74 59                        je       0x4f30
004ED7:  AC                           lodsb    al, byte ptr [si]
004ED8:  0A C0                        or       al, al
004EDA:  79 29                        jns      0x4f05
004EDC:  F6 D8                        neg      al
004EDE:  FE C0                        inc      al
004EE0:  8B C8                        mov      cx, ax
004EE2:  2B D8                        sub      bx, ax
004EE4:  79 16                        jns      0x4efc
004EE6:  F7 DB                        neg      bx
004EE8:  2B C3                        sub      ax, bx
004EEA:  8B C8                        mov      cx, ax
004EEC:  AC                           lodsb    al, byte ptr [si]
004EED:  FD                           std     
004EEE:  F3 AA                        rep stosb byte ptr es:[di], al
004EF0:  FC                           cld     
004EF1:  8B C3                        mov      ax, bx
004EF3:  2E 8B 1E 28 17               mov      bx, word ptr cs:[0x1728]
004EF8:  2B D8                        sub      bx, ax
004EFA:  EB 39                        jmp      0x4f35
004EFC:  AC                           lodsb    al, byte ptr [si]
004EFD:  FD                           std     
004EFE:  F3 AA                        rep stosb byte ptr es:[di], al
004F00:  FC                           cld     
004F01:  2B F9                        sub      di, cx
004F03:  EB CE                        jmp      0x4ed3
004F05:  FE C0                        inc      al
004F07:  8B C8                        mov      cx, ax
004F09:  2B D8                        sub      bx, ax
004F0B:  79 1A                        jns      0x4f27
004F0D:  F7 DB                        neg      bx
004F0F:  2B C3                        sub      ax, bx
004F11:  8B C8                        mov      cx, ax
004F13:  AC                           lodsb    al, byte ptr [si]
004F14:  26 88 05                     mov      byte ptr es:[di], al
004F17:  4F                           dec      di
004F18:  E2 F9                        loop     0x4f13
004F1A:  8B C3                        mov      ax, bx
004F1C:  2E 8B 1E 28 17               mov      bx, word ptr cs:[0x1728]
004F21:  2B D8                        sub      bx, ax
004F23:  03 F0                        add      si, ax
004F25:  EB 0E                        jmp      0x4f35
004F27:  AC                           lodsb    al, byte ptr [si]
004F28:  26 88 05                     mov      byte ptr es:[di], al
004F2B:  4F                           dec      di
004F2C:  E2 F9                        loop     0x4f27
004F2E:  EB A3                        jmp      0x4ed3
004F30:  2E 8B 1E 28 17               mov      bx, word ptr cs:[0x1728]
004F35:  0B DB                        or       bx, bx
004F37:  74 16                        je       0x4f4f
004F39:  AC                           lodsb    al, byte ptr [si]
004F3A:  0A C0                        or       al, al
004F3C:  79 09                        jns      0x4f47
004F3E:  F6 D8                        neg      al
004F40:  FE C0                        inc      al
004F42:  2B D8                        sub      bx, ax
004F44:  46                           inc      si
004F45:  EB EE                        jmp      0x4f35
004F47:  FE C0                        inc      al
004F49:  2B D8                        sub      bx, ax
004F4B:  03 F0                        add      si, ax
004F4D:  EB E6                        jmp      0x4f35
004F4F:  03 FA                        add      di, dx
004F51:  59                           pop      cx
004F52:  49                           dec      cx
004F53:  74 03                        je       0x4f58
004F55:  E9 3D FF                     jmp      0x4e95
004F58:  5D                           pop      bp
004F59:  5E                           pop      si
004F5A:  1F                           pop      ds
004F5B:  5F                           pop      di
004F5C:  07                           pop      es
004F5D:  5A                           pop      dx
004F5E:  59                           pop      cx
004F5F:  5B                           pop      bx
004F60:  58                           pop      ax
004F61:  C3                           ret     
