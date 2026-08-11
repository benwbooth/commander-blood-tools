; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003c6c
; seg_off: 0299:0cdc
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blit_coord_guard_c
; label_comment: blit coordinate guard (sibling of 0x339e): si=ax; or dx,dx; je/js 0x3d71 cull
; incoming: call@0x000e8e->0299:0cdc
; incoming: call@0x0014f2->0299:0cdc
; incoming: call@0x001e42->0299:0cdc
; incoming: call@0x007a5a->0299:0cdc
; incoming: call@0x007ade->0299:0cdc
; incoming: call@0x007c34->0299:0cdc
; incoming: call@0x007ca0->0299:0cdc
; byte_count: 271
; boundary: cfg_blocks_30_terminals_6
; terminal: jmp 0x3d67:1, jmp 0x3d71:4, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7fbccd05a4ce5360379a0050d0e0bdecbcc40f7f6e41958f18b27d866078349a

003C6C:  50                           push     ax
003C6D:  53                           push     bx
003C6E:  51                           push     cx
003C6F:  52                           push     dx
003C70:  06                           push     es
003C71:  57                           push     di
003C72:  1E                           push     ds
003C73:  55                           push     bp
003C74:  56                           push     si
003C75:  8B F0                        mov      si, ax
003C77:  0B D2                        or       dx, dx
003C79:  0F 84 F4 00                  je       0x3d71
003C7D:  0F 88 F0 00                  js       0x3d71
003C81:  0B ED                        or       bp, bp
003C83:  0F 84 EA 00                  je       0x3d71
003C87:  0F 88 E6 00                  js       0x3d71
003C8B:  8C E8                        mov      ax, gs
003C8D:  8E D8                        mov      ds, ax
003C8F:  C4 3E 21 52                  les      di, ptr [0x5221]
003C93:  8B C3                        mov      ax, bx
003C95:  2B 06 35 52                  sub      ax, word ptr [0x5235]
003C99:  79 0A                        jns      0x3ca5
003C9B:  03 D0                        add      dx, ax
003C9D:  0F 8E D0 00                  jle      0x3d71
003CA1:  8B 1E 35 52                  mov      bx, word ptr [0x5235]
003CA5:  8B C3                        mov      ax, bx
003CA7:  03 C2                        add      ax, dx
003CA9:  2B 06 37 52                  sub      ax, word ptr [0x5237]
003CAD:  7E 06                        jle      0x3cb5
003CAF:  2B D0                        sub      dx, ax
003CB1:  0F 8E BC 00                  jle      0x3d71
003CB5:  8B C1                        mov      ax, cx
003CB7:  2B 06 39 52                  sub      ax, word ptr [0x5239]
003CBB:  79 0A                        jns      0x3cc7
003CBD:  03 E8                        add      bp, ax
003CBF:  0F 8E AE 00                  jle      0x3d71
003CC3:  8B 0E 39 52                  mov      cx, word ptr [0x5239]
003CC7:  8B C1                        mov      ax, cx
003CC9:  03 C5                        add      ax, bp
003CCB:  2B 06 3B 52                  sub      ax, word ptr [0x523b]
003CCF:  7E 06                        jle      0x3cd7
003CD1:  2B E8                        sub      bp, ax
003CD3:  0F 8E 9A 00                  jle      0x3d71
003CD7:  8B C1                        mov      ax, cx
003CD9:  86 C4                        xchg     ah, al
003CDB:  C1 E1 06                     shl      cx, 6
003CDE:  03 C1                        add      ax, cx
003CE0:  03 C3                        add      ax, bx
003CE2:  03 F8                        add      di, ax
003CE4:  BB 40 01                     mov      bx, 0x140
003CE7:  2B DA                        sub      bx, dx
003CE9:  87 EB                        xchg     bx, bp
003CEB:  8B C6                        mov      ax, si
003CED:  8B CA                        mov      cx, dx
003CEF:  8B D7                        mov      dx, di
003CF1:  8A F1                        mov      dh, cl
003CF3:  81 E2 03 03                  and      dx, 0x303
003CF7:  C1 E9 02                     shr      cx, 2
003CFA:  74 6B                        je       0x3d67
003CFC:  8A F9                        mov      bh, cl
003CFE:  2A F2                        sub      dh, dl
003D00:  80 DF 00                     sbb      bh, 0
003D03:  75 07                        jne      0x3d0c
003D05:  80 E6 03                     and      dh, 3
003D08:  02 F2                        add      dh, dl
003D0A:  EB 5B                        jmp      0x3d67
003D0C:  8A E0                        mov      ah, al
003D0E:  66 C1 E0 10                  shl      eax, 0x10
003D12:  8B C6                        mov      ax, si
003D14:  8A E0                        mov      ah, al
003D16:  80 E6 03                     and      dh, 3
003D19:  74 19                        je       0x3d34
003D1B:  0A D2                        or       dl, dl
003D1D:  74 2A                        je       0x3d49
003D1F:  8A CA                        mov      cl, dl
003D21:  F3 AA                        rep stosb byte ptr es:[di], al
003D23:  8A CF                        mov      cl, bh
003D25:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003D28:  8A CE                        mov      cl, dh
003D2A:  F3 AA                        rep stosb byte ptr es:[di], al
003D2C:  03 FD                        add      di, bp
003D2E:  FE CB                        dec      bl
003D30:  75 ED                        jne      0x3d1f
003D32:  EB 3D                        jmp      0x3d71
003D34:  0A D2                        or       dl, dl
003D36:  74 22                        je       0x3d5a
003D38:  8A CA                        mov      cl, dl
003D3A:  F3 AA                        rep stosb byte ptr es:[di], al
003D3C:  8A CF                        mov      cl, bh
003D3E:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003D41:  03 FD                        add      di, bp
003D43:  FE CB                        dec      bl
003D45:  75 F1                        jne      0x3d38
003D47:  EB 28                        jmp      0x3d71
003D49:  8A CF                        mov      cl, bh
003D4B:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003D4E:  8A CE                        mov      cl, dh
003D50:  F3 AA                        rep stosb byte ptr es:[di], al
003D52:  03 FD                        add      di, bp
003D54:  FE CB                        dec      bl
003D56:  75 F1                        jne      0x3d49
003D58:  EB 17                        jmp      0x3d71
003D5A:  8A CF                        mov      cl, bh
003D5C:  F3 66 AB                     rep stosd dword ptr es:[di], eax
003D5F:  03 FD                        add      di, bp
003D61:  FE CB                        dec      bl
003D63:  75 F5                        jne      0x3d5a
003D65:  EB 0A                        jmp      0x3d71
003D67:  8A CE                        mov      cl, dh
003D69:  F3 AA                        rep stosb byte ptr es:[di], al
003D6B:  03 FD                        add      di, bp
003D6D:  FE CB                        dec      bl
003D6F:  75 F6                        jne      0x3d67
003D71:  5E                           pop      si
003D72:  5D                           pop      bp
003D73:  1F                           pop      ds
003D74:  5F                           pop      di
003D75:  07                           pop      es
003D76:  5A                           pop      dx
003D77:  59                           pop      cx
003D78:  5B                           pop      bx
003D79:  58                           pop      ax
003D7A:  CB                           retf    
