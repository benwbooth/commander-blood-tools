; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003428
; seg_off: 0299:0498
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: graphics_plot_modex
; label_comment: SEG 0x299:0x498: mode-X pixel/span plot. Inputs bx=x, dx=y, al=colour. Bounds-check dx vs gs:[0x5239](min+0xa)/[0x523b](max). Address = screen_buffer_far_ptr(gs:0x521d) + y*80 (ax=y<<4 + dx=y<<6) + x/4 (bx>>2); plane = x&3 (cl=bl&3) selected via the VGA map-mask. Confirms mode-X layout: byte offset = y*80 + x/4, plane = x&3. The game's core rendering primitive (my engine's linear y*320+x framebuffer yields the same pixel)
; incoming: call@0x001ac6->0299:0498
; byte_count: 326
; boundary: cfg_blocks_34_terminals_6
; terminal: jmp 0x3475:1, jmp 0x34a4:1, jmp 0x34d3:1, jmp 0x3503:1, jmp 0x3532:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: cac919590f332ddff0e782a5fa2e6fb78c21f8bd2b65828051d668f8b50614c2

003428:  50                           push     ax
003429:  53                           push     bx
00342A:  51                           push     cx
00342B:  52                           push     dx
00342C:  56                           push     si
00342D:  06                           push     es
00342E:  57                           push     di
00342F:  55                           push     bp
003430:  65 C7 06 CD 27 00 00         mov      word ptr gs:[0x27cd], 0
003437:  65 3B 16 3B 52               cmp      dx, word ptr gs:[0x523b]
00343C:  0F 87 25 01                  ja       0x3565
003440:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
003445:  83 E9 0A                     sub      cx, 0xa
003448:  3B D1                        cmp      dx, cx
00344A:  0F 8E 17 01                  jle      0x3565
00344E:  8A E8                        mov      ch, al
003450:  65 C4 3E 1D 52               les      di, ptr gs:[0x521d]
003455:  8B C2                        mov      ax, dx
003457:  C1 E0 04                     shl      ax, 4
00345A:  C1 E2 06                     shl      dx, 6
00345D:  03 C2                        add      ax, dx
00345F:  8A CB                        mov      cl, bl
003461:  80 E1 03                     and      cl, 3
003464:  C1 EB 02                     shr      bx, 2
003467:  03 C3                        add      ax, bx
003469:  03 F8                        add      di, ax
00346B:  BA C4 03                     mov      dx, 0x3c4
00346E:  B0 02                        mov      al, 2
003470:  EE                           out      dx, al
003471:  42                           inc      dx
003472:  66 33 C0                     xor      eax, eax
003475:  AC                           lodsb    al, byte ptr [si]
003476:  0A C0                        or       al, al
003478:  0F 84 E9 00                  je       0x3565
00347C:  BB 62 73                     mov      bx, 0x7362
00347F:  65 D7                        xlatb   
003481:  67 65 8A B8 12 74 00 00      mov      bh, byte ptr gs:[eax + 0x7412]
003489:  BD 42 74                     mov      bp, 0x7442
00348C:  B3 14                        mov      bl, 0x14
00348E:  F6 E3                        mul      bl
003490:  03 E8                        add      bp, ax
003492:  8A E5                        mov      ah, ch
003494:  57                           push     di
003495:  51                           push     cx
003496:  B0 11                        mov      al, 0x11
003498:  D2 C0                        rol      al, cl
00349A:  EE                           out      dx, al
00349B:  B3 0A                        mov      bl, 0xa
00349D:  57                           push     di
00349E:  8B 4E 00                     mov      cx, word ptr [bp]
0034A1:  86 CD                        xchg     ch, cl
0034A3:  57                           push     di
0034A4:  0B C9                        or       cx, cx
0034A6:  79 03                        jns      0x34ab
0034A8:  26 88 25                     mov      byte ptr es:[di], ah
0034AB:  74 06                        je       0x34b3
0034AD:  C1 E1 04                     shl      cx, 4
0034B0:  47                           inc      di
0034B1:  EB F1                        jmp      0x34a4
0034B3:  5F                           pop      di
0034B4:  83 C7 50                     add      di, 0x50
0034B7:  83 C5 02                     add      bp, 2
0034BA:  FE CB                        dec      bl
0034BC:  75 E0                        jne      0x349e
0034BE:  5F                           pop      di
0034BF:  D0 C0                        rol      al, 1
0034C1:  83 D7 00                     adc      di, 0
0034C4:  EE                           out      dx, al
0034C5:  B3 0A                        mov      bl, 0xa
0034C7:  83 ED 14                     sub      bp, 0x14
0034CA:  57                           push     di
0034CB:  8B 4E 00                     mov      cx, word ptr [bp]
0034CE:  86 CD                        xchg     ch, cl
0034D0:  03 C9                        add      cx, cx
0034D2:  57                           push     di
0034D3:  0B C9                        or       cx, cx
0034D5:  79 03                        jns      0x34da
0034D7:  26 88 25                     mov      byte ptr es:[di], ah
0034DA:  74 06                        je       0x34e2
0034DC:  C1 E1 04                     shl      cx, 4
0034DF:  47                           inc      di
0034E0:  EB F1                        jmp      0x34d3
0034E2:  5F                           pop      di
0034E3:  83 C7 50                     add      di, 0x50
0034E6:  83 C5 02                     add      bp, 2
0034E9:  FE CB                        dec      bl
0034EB:  75 DE                        jne      0x34cb
0034ED:  5F                           pop      di
0034EE:  D0 C0                        rol      al, 1
0034F0:  83 D7 00                     adc      di, 0
0034F3:  EE                           out      dx, al
0034F4:  B3 0A                        mov      bl, 0xa
0034F6:  83 ED 14                     sub      bp, 0x14
0034F9:  57                           push     di
0034FA:  8B 4E 00                     mov      cx, word ptr [bp]
0034FD:  86 CD                        xchg     ch, cl
0034FF:  C1 E1 02                     shl      cx, 2
003502:  57                           push     di
003503:  0B C9                        or       cx, cx
003505:  79 03                        jns      0x350a
003507:  26 88 25                     mov      byte ptr es:[di], ah
00350A:  74 06                        je       0x3512
00350C:  C1 E1 04                     shl      cx, 4
00350F:  47                           inc      di
003510:  EB F1                        jmp      0x3503
003512:  5F                           pop      di
003513:  83 C7 50                     add      di, 0x50
003516:  83 C5 02                     add      bp, 2
003519:  FE CB                        dec      bl
00351B:  75 DD                        jne      0x34fa
00351D:  5F                           pop      di
00351E:  D0 C0                        rol      al, 1
003520:  83 D7 00                     adc      di, 0
003523:  EE                           out      dx, al
003524:  B3 0A                        mov      bl, 0xa
003526:  83 ED 14                     sub      bp, 0x14
003529:  8B 4E 00                     mov      cx, word ptr [bp]
00352C:  86 CD                        xchg     ch, cl
00352E:  C1 E1 03                     shl      cx, 3
003531:  57                           push     di
003532:  0B C9                        or       cx, cx
003534:  79 03                        jns      0x3539
003536:  26 88 25                     mov      byte ptr es:[di], ah
003539:  74 06                        je       0x3541
00353B:  C1 E1 04                     shl      cx, 4
00353E:  47                           inc      di
00353F:  EB F1                        jmp      0x3532
003541:  5F                           pop      di
003542:  83 C7 50                     add      di, 0x50
003545:  83 C5 02                     add      bp, 2
003548:  FE CB                        dec      bl
00354A:  75 DD                        jne      0x3529
00354C:  59                           pop      cx
00354D:  5F                           pop      di
00354E:  8A C7                        mov      al, bh
003550:  98                           cwde    
003551:  65 01 06 CD 27               add      word ptr gs:[0x27cd], ax
003556:  02 C1                        add      al, cl
003558:  8A C8                        mov      cl, al
00355A:  80 E1 03                     and      cl, 3
00355D:  C1 E8 02                     shr      ax, 2
003560:  03 F8                        add      di, ax
003562:  E9 10 FF                     jmp      0x3475
003565:  5D                           pop      bp
003566:  5F                           pop      di
003567:  07                           pop      es
003568:  5E                           pop      si
003569:  5A                           pop      dx
00356A:  59                           pop      cx
00356B:  5B                           pop      bx
00356C:  58                           pop      ax
00356D:  CB                           retf    
