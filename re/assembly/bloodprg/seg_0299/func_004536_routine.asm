; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004536
; seg_off: 0299:15a6
; group: seg_0299
; provenance: static_dispatch_table_target
; incoming: sprite_blitter_candidates:blit_0
; byte_count: 384
; boundary: cfg_blocks_37_terminals_4
; terminal: jmp 0x46ac:3, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_004536_routine.cpp
; routine_bytes_sha256: 6c9b670c496e7c25303e9cb96c47892c02c51c0596dab3047e450451c23cce81

004536:  50                           push     ax
004537:  53                           push     bx
004538:  51                           push     cx
004539:  52                           push     dx
00453A:  1E                           push     ds
00453B:  56                           push     si
00453C:  06                           push     es
00453D:  57                           push     di
00453E:  55                           push     bp
00453F:  C5 75 04                     lds      si, ptr [di + 4]
004542:  03 44 04                     add      ax, word ptr [si + 4]
004545:  03 5C 06                     add      bx, word ptr [si + 6]
004548:  03 54 04                     add      dx, word ptr [si + 4]
00454B:  03 6C 06                     add      bp, word ptr [si + 6]
00454E:  FF 34                        push     word ptr [si]
004550:  52                           push     dx
004551:  26 8B 4D 0E                  mov      cx, word ptr es:[di + 0xe]
004555:  8B C3                        mov      ax, bx
004557:  26 2B 45 1C                  sub      ax, word ptr es:[di + 0x1c]
00455B:  79 14                        jns      0x4571
00455D:  F7 D8                        neg      ax
00455F:  2B C8                        sub      cx, ax
004561:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004567:  75 04                        jne      0x456d
004569:  F7 24                        mul      word ptr [si]
00456B:  03 F0                        add      si, ax
00456D:  26 8B 5D 1C                  mov      bx, word ptr es:[di + 0x1c]
004571:  8B C5                        mov      ax, bp
004573:  26 2B 45 1E                  sub      ax, word ptr es:[di + 0x1e]
004577:  78 0E                        js       0x4587
004579:  2B C8                        sub      cx, ax
00457B:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004581:  74 04                        je       0x4587
004583:  F7 24                        mul      word ptr [si]
004585:  03 F0                        add      si, ax
004587:  26 8B 6D 0C                  mov      bp, word ptr es:[di + 0xc]
00458B:  26 8B 55 08                  mov      dx, word ptr es:[di + 8]
00458F:  03 54 04                     add      dx, word ptr [si + 4]
004592:  8B C2                        mov      ax, dx
004594:  26 2B 45 18                  sub      ax, word ptr es:[di + 0x18]
004598:  79 10                        jns      0x45aa
00459A:  03 E8                        add      bp, ax
00459C:  2E F6 06 DF 14 01            test     byte ptr cs:[0x14df], 1
0045A2:  75 02                        jne      0x45a6
0045A4:  2B F0                        sub      si, ax
0045A6:  26 8B 55 18                  mov      dx, word ptr es:[di + 0x18]
0045AA:  58                           pop      ax
0045AB:  26 2B 45 1A                  sub      ax, word ptr es:[di + 0x1a]
0045AF:  78 0C                        js       0x45bd
0045B1:  2B E8                        sub      bp, ax
0045B3:  2E F6 06 DF 14 01            test     byte ptr cs:[0x14df], 1
0045B9:  74 02                        je       0x45bd
0045BB:  03 F0                        add      si, ax
0045BD:  53                           push     bx
0045BE:  33 DB                        xor      bx, bx
0045C0:  26 8A 45 01                  mov      al, byte ptr es:[di + 1]
0045C4:  24 03                        and      al, 3
0045C6:  74 0A                        je       0x45d2
0045C8:  BB 11 5F                     mov      bx, 0x5f11
0045CB:  FE C8                        dec      al
0045CD:  74 03                        je       0x45d2
0045CF:  BB 11 60                     mov      bx, 0x6011
0045D2:  65 89 1E 4B 52               mov      word ptr gs:[0x524b], bx
0045D7:  5B                           pop      bx
0045D8:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
0045DD:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
0045E3:  74 03                        je       0x45e8
0045E5:  03 D9                        add      bx, cx
0045E7:  4B                           dec      bx
0045E8:  8B C3                        mov      ax, bx
0045EA:  86 C4                        xchg     ah, al
0045EC:  C1 E3 06                     shl      bx, 6
0045EF:  03 C3                        add      ax, bx
0045F1:  03 F8                        add      di, ax
0045F3:  03 FA                        add      di, dx
0045F5:  83 C6 08                     add      si, 8
0045F8:  BA 40 01                     mov      dx, 0x140
0045FB:  2B D5                        sub      dx, bp
0045FD:  5B                           pop      bx
0045FE:  2B DD                        sub      bx, bp
004600:  2E 89 1E A4 15               mov      word ptr cs:[0x15a4], bx
004605:  2E A1 DF 14                  mov      ax, word ptr cs:[0x14df]
004609:  0A E4                        or       ah, ah
00460B:  74 06                        je       0x4613
00460D:  03 D5                        add      dx, bp
00460F:  03 D5                        add      dx, bp
004611:  F7 DA                        neg      dx
004613:  0A C0                        or       al, al
004615:  75 48                        jne      0x465f
004617:  65 8B 1E 4B 52               mov      bx, word ptr gs:[0x524b]
00461C:  0B DB                        or       bx, bx
00461E:  74 21                        je       0x4641
004620:  8A E1                        mov      ah, cl
004622:  8B CD                        mov      cx, bp
004624:  AC                           lodsb    al, byte ptr [si]
004625:  0A C0                        or       al, al
004627:  74 08                        je       0x4631
004629:  26 8A 05                     mov      al, byte ptr es:[di]
00462C:  65 D7                        xlatb   
00462E:  26 88 05                     mov      byte ptr es:[di], al
004631:  47                           inc      di
004632:  E2 F0                        loop     0x4624
004634:  03 FA                        add      di, dx
004636:  2E 03 36 A4 15               add      si, word ptr cs:[0x15a4]
00463B:  8A CC                        mov      cl, ah
00463D:  E2 E1                        loop     0x4620
00463F:  EB 6B                        jmp      0x46ac
004641:  2E 8B 1E A4 15               mov      bx, word ptr cs:[0x15a4]
004646:  8A E1                        mov      ah, cl
004648:  8B CD                        mov      cx, bp
00464A:  AC                           lodsb    al, byte ptr [si]
00464B:  0A C0                        or       al, al
00464D:  74 03                        je       0x4652
00464F:  26 88 05                     mov      byte ptr es:[di], al
004652:  47                           inc      di
004653:  E2 F5                        loop     0x464a
004655:  03 FA                        add      di, dx
004657:  03 F3                        add      si, bx
004659:  8A CC                        mov      cl, ah
00465B:  E2 E9                        loop     0x4646
00465D:  EB 4D                        jmp      0x46ac
00465F:  03 D5                        add      dx, bp
004661:  03 D5                        add      dx, bp
004663:  03 FD                        add      di, bp
004665:  4F                           dec      di
004666:  65 8B 1E 4B 52               mov      bx, word ptr gs:[0x524b]
00466B:  0B DB                        or       bx, bx
00466D:  74 21                        je       0x4690
00466F:  8A E1                        mov      ah, cl
004671:  8B CD                        mov      cx, bp
004673:  AC                           lodsb    al, byte ptr [si]
004674:  0A C0                        or       al, al
004676:  74 08                        je       0x4680
004678:  26 8A 05                     mov      al, byte ptr es:[di]
00467B:  65 D7                        xlatb   
00467D:  26 88 05                     mov      byte ptr es:[di], al
004680:  4F                           dec      di
004681:  E2 F0                        loop     0x4673
004683:  03 FA                        add      di, dx
004685:  2E 03 36 A4 15               add      si, word ptr cs:[0x15a4]
00468A:  8A CC                        mov      cl, ah
00468C:  E2 E1                        loop     0x466f
00468E:  EB 1C                        jmp      0x46ac
004690:  2E 8B 1E A4 15               mov      bx, word ptr cs:[0x15a4]
004695:  8A E1                        mov      ah, cl
004697:  8B CD                        mov      cx, bp
004699:  AC                           lodsb    al, byte ptr [si]
00469A:  0A C0                        or       al, al
00469C:  74 03                        je       0x46a1
00469E:  26 88 05                     mov      byte ptr es:[di], al
0046A1:  4F                           dec      di
0046A2:  E2 F5                        loop     0x4699
0046A4:  03 FA                        add      di, dx
0046A6:  03 F3                        add      si, bx
0046A8:  8A CC                        mov      cl, ah
0046AA:  E2 E9                        loop     0x4695
0046AC:  5D                           pop      bp
0046AD:  5F                           pop      di
0046AE:  07                           pop      es
0046AF:  5E                           pop      si
0046B0:  1F                           pop      ds
0046B1:  5A                           pop      dx
0046B2:  59                           pop      cx
0046B3:  5B                           pop      bx
0046B4:  58                           pop      ax
0046B5:  C3                           ret     
