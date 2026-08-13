; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003630
; seg_off: 0299:06a0
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: subtitle_reveal_draw_wrapper
; label_comment: Mode-X subtitle reveal renderer. DS:SI points to a CR-terminated line, BX=x, and DX=y. It scans the line length, addresses full GS:[0x5219] as y*80+x/4, and compares each current text offset with GS:0x5E58. Signed-negative distance stops drawing; low distances 0/1/other select colors FF/FE/FD. Characters map through GS:0x70FA; high-bit results skip glyph writes but retain fixed eight-pixel advance. Four plane passes read paired bits from SS:0x71AA+glyph*8 under rotating masks. Natural C and raw vectors: re/source/bloodprg/candidates/seg_0299/func_003630_subtitle_reveal_draw_wrapper.c and re/tools/oracle_vectors/func_3630_natural.json
; incoming: call@0x0094ee->0299:06a0
; byte_count: 186
; boundary: cfg_blocks_16_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 92b974665ae76fd5b36a0b4e503813b7aa8cf7f601076e459ceb0c4e0a5f3a21

003630:  55                           push     bp
003631:  06                           push     es
003632:  57                           push     di
003633:  56                           push     si
003634:  50                           push     ax
003635:  53                           push     bx
003636:  51                           push     cx
003637:  52                           push     dx
003638:  8B FE                        mov      di, si
00363A:  8C D8                        mov      ax, ds
00363C:  8E C0                        mov      es, ax
00363E:  B0 0D                        mov      al, 0xd
003640:  B9 FF FF                     mov      cx, 0xffff
003643:  F2 AE                        repne scasb al, byte ptr es:[di]
003645:  F7 D9                        neg      cx
003647:  83 E9 02                     sub      cx, 2
00364A:  8A E9                        mov      ch, cl
00364C:  65 C4 3E 19 52               les      di, ptr gs:[0x5219]
003651:  8B C2                        mov      ax, dx
003653:  C1 E0 04                     shl      ax, 4
003656:  C1 E2 06                     shl      dx, 6
003659:  03 C2                        add      ax, dx
00365B:  8A CB                        mov      cl, bl
00365D:  80 E1 03                     and      cl, 3
003660:  C1 EB 02                     shr      bx, 2
003663:  03 C3                        add      ax, bx
003665:  03 F8                        add      di, ax
003667:  BA C4 03                     mov      dx, 0x3c4
00366A:  B0 02                        mov      al, 2
00366C:  EE                           out      dx, al
00366D:  B0 11                        mov      al, 0x11
00366F:  D2 E0                        shl      al, cl
003671:  42                           inc      dx
003672:  EE                           out      dx, al
003673:  65 8B 1E 58 5E               mov      bx, word ptr gs:[0x5e58]
003678:  2B DE                        sub      bx, si
00367A:  78 65                        js       0x36e1
00367C:  50                           push     ax
00367D:  51                           push     cx
00367E:  57                           push     di
00367F:  8A EB                        mov      ch, bl
003681:  8A C8                        mov      cl, al
003683:  AC                           lodsb    al, byte ptr [si]
003684:  BB FA 70                     mov      bx, 0x70fa
003687:  65 D7                        xlatb   
003689:  0A C0                        or       al, al
00368B:  78 4A                        js       0x36d7
00368D:  BD AA 71                     mov      bp, 0x71aa
003690:  98                           cwde    
003691:  C1 E0 03                     shl      ax, 3
003694:  03 E8                        add      bp, ax
003696:  B4 FF                        mov      ah, 0xff
003698:  8A C1                        mov      al, cl
00369A:  0A ED                        or       ch, ch
00369C:  74 08                        je       0x36a6
00369E:  FE CC                        dec      ah
0036A0:  FE CD                        dec      ch
0036A2:  74 02                        je       0x36a6
0036A4:  FE CC                        dec      ah
0036A6:  B1 01                        mov      cl, 1
0036A8:  B7 08                        mov      bh, 8
0036AA:  57                           push     di
0036AB:  8A 5E 00                     mov      bl, byte ptr [bp]
0036AE:  D2 E3                        shl      bl, cl
0036B0:  73 03                        jae      0x36b5
0036B2:  26 88 25                     mov      byte ptr es:[di], ah
0036B5:  C0 E3 04                     shl      bl, 4
0036B8:  73 04                        jae      0x36be
0036BA:  26 88 65 01                  mov      byte ptr es:[di + 1], ah
0036BE:  83 C7 50                     add      di, 0x50
0036C1:  45                           inc      bp
0036C2:  FE CF                        dec      bh
0036C4:  75 E5                        jne      0x36ab
0036C6:  5F                           pop      di
0036C7:  D0 C0                        rol      al, 1
0036C9:  83 D7 00                     adc      di, 0
0036CC:  EE                           out      dx, al
0036CD:  83 ED 08                     sub      bp, 8
0036D0:  FE C1                        inc      cl
0036D2:  80 F9 05                     cmp      cl, 5
0036D5:  75 D1                        jne      0x36a8
0036D7:  5F                           pop      di
0036D8:  59                           pop      cx
0036D9:  58                           pop      ax
0036DA:  83 C7 02                     add      di, 2
0036DD:  FE CD                        dec      ch
0036DF:  75 91                        jne      0x3672
0036E1:  5A                           pop      dx
0036E2:  59                           pop      cx
0036E3:  5B                           pop      bx
0036E4:  58                           pop      ax
0036E5:  5E                           pop      si
0036E6:  5F                           pop      di
0036E7:  07                           pop      es
0036E8:  5D                           pop      bp
0036E9:  CB                           retf    
