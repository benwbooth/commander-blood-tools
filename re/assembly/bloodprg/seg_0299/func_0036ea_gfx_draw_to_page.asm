; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0036ea
; seg_off: 0299:075a
; group: seg_0299
; provenance: relocation_proven_far_transfer_target
; label: gfx_draw_to_page
; label_comment: draw to the VGA display page: les di,gs:[0x5219] (display page ptr); ch=dl; x offset = bx<<4. Renders directly into the visible VGA page (vs the linear back-buffer)
; incoming: call@0x000da8->0299:075a
; incoming: call@0x000dd1->0299:075a
; incoming: call@0x000ddb->0299:075a
; incoming: call@0x000e01->0299:075a
; incoming: call@0x000e0c->0299:075a
; incoming: call@0x000e2f->0299:075a
; incoming: call@0x000e3b->0299:075a
; incoming: call@0x000e4e->0299:075a
; byte_count: 139
; boundary: cfg_blocks_11_terminals_2
; terminal: jmp 0x371a:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0036ea_gfx_draw_to_page.cpp
; routine_bytes_sha256: 42fe2f622a12ff3f3395b40c56e015d55e788fe49ab111a979103561971572a3

0036EA:  50                           push     ax
0036EB:  53                           push     bx
0036EC:  51                           push     cx
0036ED:  52                           push     dx
0036EE:  06                           push     es
0036EF:  57                           push     di
0036F0:  56                           push     si
0036F1:  55                           push     bp
0036F2:  8A EA                        mov      ch, dl
0036F4:  65 C4 3E 19 52               les      di, ptr gs:[0x5219]
0036F9:  8B D3                        mov      dx, bx
0036FB:  C1 E2 04                     shl      dx, 4
0036FE:  C1 E3 06                     shl      bx, 6
003701:  03 D3                        add      dx, bx
003703:  8A C8                        mov      cl, al
003705:  80 E1 03                     and      cl, 3
003708:  C1 E8 02                     shr      ax, 2
00370B:  03 C2                        add      ax, dx
00370D:  03 F8                        add      di, ax
00370F:  BA C4 03                     mov      dx, 0x3c4
003712:  B0 02                        mov      al, 2
003714:  EE                           out      dx, al
003715:  42                           inc      dx
003716:  B0 11                        mov      al, 0x11
003718:  D2 E0                        shl      al, cl
00371A:  EE                           out      dx, al
00371B:  50                           push     ax
00371C:  57                           push     di
00371D:  8A C8                        mov      cl, al
00371F:  AC                           lodsb    al, byte ptr [si]
003720:  0A C0                        or       al, al
003722:  74 45                        je       0x3769
003724:  BB A8 6F                     mov      bx, 0x6fa8
003727:  65 D7                        xlatb   
003729:  0A C0                        or       al, al
00372B:  78 37                        js       0x3764
00372D:  BD 28 70                     mov      bp, 0x7028
003730:  98                           cwde    
003731:  8B D8                        mov      bx, ax
003733:  C1 E0 02                     shl      ax, 2
003736:  03 C3                        add      ax, bx
003738:  03 E8                        add      bp, ax
00373A:  8A C1                        mov      al, cl
00373C:  B1 01                        mov      cl, 1
00373E:  57                           push     di
00373F:  B3 05                        mov      bl, 5
003741:  8A 66 00                     mov      ah, byte ptr [bp]
003744:  D2 E4                        shl      ah, cl
003746:  73 03                        jae      0x374b
003748:  26 88 2D                     mov      byte ptr es:[di], ch
00374B:  45                           inc      bp
00374C:  83 C7 50                     add      di, 0x50
00374F:  FE CB                        dec      bl
003751:  75 EE                        jne      0x3741
003753:  5F                           pop      di
003754:  D0 C0                        rol      al, 1
003756:  83 D7 00                     adc      di, 0
003759:  EE                           out      dx, al
00375A:  83 ED 05                     sub      bp, 5
00375D:  FE C1                        inc      cl
00375F:  80 F9 05                     cmp      cl, 5
003762:  75 DA                        jne      0x373e
003764:  5F                           pop      di
003765:  58                           pop      ax
003766:  47                           inc      di
003767:  EB B1                        jmp      0x371a
003769:  83 C4 04                     add      sp, 4
00376C:  5D                           pop      bp
00376D:  5E                           pop      si
00376E:  5F                           pop      di
00376F:  07                           pop      es
003770:  5A                           pop      dx
003771:  59                           pop      cx
003772:  5B                           pop      bx
003773:  58                           pop      ax
003774:  CB                           retf    
