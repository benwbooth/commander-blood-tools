; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00356e
; seg_off: 0299:05de
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: clipped_blit_w10
; label_comment: clipped blit (10-px span variant): cmp dx,gs:[0x523b] height clip; cx=gs:[0x5239]-0xa width clip. Wider-span bounds-checked copy
; incoming: call@0x0072f6->0299:05de
; byte_count: 194
; boundary: cfg_blocks_14_terminals_2
; terminal: jmp 0x35bc:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 92cf717e0e565459666f57ef0e2ba6b1b3aecd3e00428767537d40a0f6726d98

00356E:  66 50                        push     eax
003570:  53                           push     bx
003571:  51                           push     cx
003572:  52                           push     dx
003573:  56                           push     si
003574:  06                           push     es
003575:  57                           push     di
003576:  55                           push     bp
003577:  65 C7 06 CD 27 00 00         mov      word ptr gs:[0x27cd], 0
00357E:  65 3B 16 3B 52               cmp      dx, word ptr gs:[0x523b]
003583:  0F 87 9F 00                  ja       0x3626
003587:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
00358C:  83 E9 0A                     sub      cx, 0xa
00358F:  3B D1                        cmp      dx, cx
003591:  0F 8E 91 00                  jle      0x3626
003595:  8A E8                        mov      ch, al
003597:  65 C4 3E 19 52               les      di, ptr gs:[0x5219]
00359C:  8B C2                        mov      ax, dx
00359E:  C1 E0 04                     shl      ax, 4
0035A1:  C1 E2 06                     shl      dx, 6
0035A4:  03 C2                        add      ax, dx
0035A6:  8A CB                        mov      cl, bl
0035A8:  80 E1 03                     and      cl, 3
0035AB:  C1 EB 02                     shr      bx, 2
0035AE:  03 C3                        add      ax, bx
0035B0:  03 F8                        add      di, ax
0035B2:  BA C4 03                     mov      dx, 0x3c4
0035B5:  B0 02                        mov      al, 2
0035B7:  EE                           out      dx, al
0035B8:  42                           inc      dx
0035B9:  66 33 C0                     xor      eax, eax
0035BC:  AC                           lodsb    al, byte ptr [si]
0035BD:  0A C0                        or       al, al
0035BF:  74 65                        je       0x3626
0035C1:  BB 02 78                     mov      bx, 0x7802
0035C4:  65 D7                        xlatb   
0035C6:  67 65 8A B8 B2 78 00 00      mov      bh, byte ptr gs:[eax + 0x78b2]
0035CE:  BD 08 79                     mov      bp, 0x7908
0035D1:  C1 E0 03                     shl      ax, 3
0035D4:  03 E8                        add      bp, ax
0035D6:  57                           push     di
0035D7:  51                           push     cx
0035D8:  B0 11                        mov      al, 0x11
0035DA:  D2 C0                        rol      al, cl
0035DC:  EE                           out      dx, al
0035DD:  B1 01                        mov      cl, 1
0035DF:  57                           push     di
0035E0:  B3 08                        mov      bl, 8
0035E2:  8A 66 00                     mov      ah, byte ptr [bp]
0035E5:  D2 E4                        shl      ah, cl
0035E7:  73 03                        jae      0x35ec
0035E9:  26 88 2D                     mov      byte ptr es:[di], ch
0035EC:  C0 E4 04                     shl      ah, 4
0035EF:  73 04                        jae      0x35f5
0035F1:  26 88 6D 01                  mov      byte ptr es:[di + 1], ch
0035F5:  83 C7 50                     add      di, 0x50
0035F8:  45                           inc      bp
0035F9:  FE CB                        dec      bl
0035FB:  75 E5                        jne      0x35e2
0035FD:  5F                           pop      di
0035FE:  D0 C0                        rol      al, 1
003600:  83 D7 00                     adc      di, 0
003603:  EE                           out      dx, al
003604:  83 ED 08                     sub      bp, 8
003607:  FE C1                        inc      cl
003609:  80 F9 05                     cmp      cl, 5
00360C:  75 D1                        jne      0x35df
00360E:  59                           pop      cx
00360F:  5F                           pop      di
003610:  8A C7                        mov      al, bh
003612:  98                           cwde    
003613:  65 01 06 CD 27               add      word ptr gs:[0x27cd], ax
003618:  02 C1                        add      al, cl
00361A:  8A C8                        mov      cl, al
00361C:  80 E1 03                     and      cl, 3
00361F:  C1 E8 02                     shr      ax, 2
003622:  03 F8                        add      di, ax
003624:  EB 96                        jmp      0x35bc
003626:  5D                           pop      bp
003627:  5F                           pop      di
003628:  07                           pop      es
003629:  5E                           pop      si
00362A:  5A                           pop      dx
00362B:  59                           pop      cx
00362C:  5B                           pop      bx
00362D:  66 58                        pop      eax
00362F:  CB                           retf    
