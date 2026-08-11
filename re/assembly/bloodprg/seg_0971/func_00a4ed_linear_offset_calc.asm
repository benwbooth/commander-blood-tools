; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a4ed
; seg_off: 0971:07dd
; group: seg_0971
; provenance: recursive_graph
; label: linear_offset_calc
; label_comment: linear screen offset calc (2 calls): bx*256 (xchg al,ah) + bx*64 (shl bx,6) = bx*320; = the y*320 row-offset into the linear buffer. A coordinate->offset helper
; byte_count: 101
; boundary: cfg_blocks_14_terminals_4
; terminal: jmp 0xa54f:3, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d68fc64fe931eda8ecabf762096b253b2324a77d9dcc17eb4298953f7d99fbdc

00A4ED:  50                           push     ax
00A4EE:  53                           push     bx
00A4EF:  8B EF                        mov      bp, di
00A4F1:  8B C3                        mov      ax, bx
00A4F3:  86 E0                        xchg     al, ah
00A4F5:  C1 E3 06                     shl      bx, 6
00A4F8:  03 C3                        add      ax, bx
00A4FA:  8B F8                        mov      di, ax
00A4FC:  03 FA                        add      di, dx
00A4FE:  8A D1                        mov      dl, cl
00A500:  81 FD 40 01                  cmp      bp, 0x140
00A504:  74 2B                        je       0xa531
00A506:  BB 40 01                     mov      bx, 0x140
00A509:  2B DD                        sub      bx, bp
00A50B:  80 FD FF                     cmp      ch, 0xff
00A50E:  74 0C                        je       0xa51c
00A510:  8B CD                        mov      cx, bp
00A512:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00A514:  03 FB                        add      di, bx
00A516:  FE CA                        dec      dl
00A518:  75 F6                        jne      0xa510
00A51A:  EB 33                        jmp      0xa54f
00A51C:  8B CD                        mov      cx, bp
00A51E:  AC                           lodsb    al, byte ptr [si]
00A51F:  0A C0                        or       al, al
00A521:  74 03                        je       0xa526
00A523:  26 88 05                     mov      byte ptr es:[di], al
00A526:  47                           inc      di
00A527:  E2 F5                        loop     0xa51e
00A529:  03 FB                        add      di, bx
00A52B:  FE CA                        dec      dl
00A52D:  75 ED                        jne      0xa51c
00A52F:  EB 1E                        jmp      0xa54f
00A531:  8A C2                        mov      al, dl
00A533:  32 E4                        xor      ah, ah
00A535:  F7 E5                        mul      bp
00A537:  80 FD FF                     cmp      ch, 0xff
00A53A:  74 06                        je       0xa542
00A53C:  8B C8                        mov      cx, ax
00A53E:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00A540:  EB 0D                        jmp      0xa54f
00A542:  8B C8                        mov      cx, ax
00A544:  AC                           lodsb    al, byte ptr [si]
00A545:  0A C0                        or       al, al
00A547:  74 03                        je       0xa54c
00A549:  26 88 05                     mov      byte ptr es:[di], al
00A54C:  47                           inc      di
00A54D:  E2 F5                        loop     0xa544
00A54F:  5B                           pop      bx
00A550:  58                           pop      ax
00A551:  C3                           ret     
