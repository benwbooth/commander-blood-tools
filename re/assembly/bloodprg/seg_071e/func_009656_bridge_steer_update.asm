; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009656
; seg_off: 071e:1e76
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: bridge_steer_update
; label_comment: per-tick bridge view update: if seek ([0x2793]&8) ease frame toward [0x279b]/2 by half-remaining (min 1) shortest way on the 180 ring, long seeks ([0x279d]>=0x28) drag the cursor ring anchor along; else mouse-push steering: dist(frame*2, [0xa2a]/4) on the 360 arc ring, dead zone <=0x1f, view lands 0x1e arc short of the cursor ([0x97c4]/[0x97d4]); [0x2793]&4 = menu-engaged clamp mode (drags cursor back at 0x28). Falls into 0x97e4 sync + 0x97fc screen rebase. Ported: src/bridge.rs BridgeView::update_view (replays all 3 BRIDGEPROBE observations) || ALSO RECORDED as `bridge_steer_ring`: bridge view steer tracks mouse in the 1440-px RING ([0xa2a]), not screen — edge-push keeps rotating. Ported: MouseInput.dx/dy raw deltas -> BridgeView || ALSO RECORDED as `ship_3d_procedural_angle_update`: ship HUD/procedural update: angle state, mouse ring wrapping, and target-list rotation gate || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; incoming: call@0x00198f->071e:1e76
; incoming: call@0x00b18e->071e:1e76
; incoming: call@0x00b493->071e:1e76
; byte_count: 453
; boundary: cfg_blocks_58_terminals_8
; terminal: jmp 0x96a5:1, jmp 0x96f3:1, jmp 0x96fc:1, jmp 0x9717:1, jmp 0x9794:1, jmp 0x97e1:1, jmp 0x97fc:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6fe70e0d16926546ddd268e3b76389d4ac4ba7fb4aa0792ff114be219e8be55f

009656:  A1 95 27                     mov      ax, word ptr [0x2795]
009659:  8B 1E 2A 0A                  mov      bx, word ptr [0xa2a]
00965D:  F7 06 93 27 08 00            test     word ptr [0x2793], 8
009663:  0F 84 95 00                  je       0x96fc
009667:  8B 16 9B 27                  mov      dx, word ptr [0x279b]
00966B:  D1 EA                        shr      dx, 1
00966D:  3B C2                        cmp      ax, dx
00966F:  75 0D                        jne      0x967e
009671:  83 36 93 27 08               xor      word ptr [0x2793], 8
009676:  C7 06 9D 27 00 00            mov      word ptr [0x279d], 0
00967C:  EB 7E                        jmp      0x96fc
00967E:  7F 01                        jg       0x9681
009680:  92                           xchg     dx, ax
009681:  2B C2                        sub      ax, dx
009683:  83 F8 5A                     cmp      ax, 0x5a
009686:  7C 05                        jl       0x968d
009688:  2D B4 00                     sub      ax, 0xb4
00968B:  F7 D8                        neg      ax
00968D:  8B 0E 95 27                  mov      cx, word ptr [0x2795]
009691:  03 C8                        add      cx, ax
009693:  79 06                        jns      0x969b
009695:  81 C1 B4 00                  add      cx, 0xb4
009699:  EB 0A                        jmp      0x96a5
00969B:  81 F9 B4 00                  cmp      cx, 0xb4
00969F:  7C 04                        jl       0x96a5
0096A1:  81 E9 B4 00                  sub      cx, 0xb4
0096A5:  03 C9                        add      cx, cx
0096A7:  8B 16 9D 27                  mov      dx, word ptr [0x279d]
0096AB:  0B D2                        or       dx, dx
0096AD:  75 03                        jne      0x96b2
0096AF:  A3 9D 27                     mov      word ptr [0x279d], ax
0096B2:  8B D0                        mov      dx, ax
0096B4:  D1 EA                        shr      dx, 1
0096B6:  75 01                        jne      0x96b9
0096B8:  42                           inc      dx
0096B9:  C1 E0 02                     shl      ax, 2
0096BC:  3B 0E 9B 27                  cmp      cx, word ptr [0x279b]
0096C0:  C6 06 DB 27 01               mov      byte ptr [0x27db], 1
0096C5:  74 09                        je       0x96d0
0096C7:  C6 06 DB 27 00               mov      byte ptr [0x27db], 0
0096CC:  F7 DA                        neg      dx
0096CE:  F7 D8                        neg      ax
0096D0:  8B 0E 9D 27                  mov      cx, word ptr [0x279d]
0096D4:  83 F9 28                     cmp      cx, 0x28
0096D7:  7C 06                        jl       0x96df
0096D9:  03 D8                        add      bx, ax
0096DB:  01 06 38 0A                  add      word ptr [0xa38], ax
0096DF:  A1 95 27                     mov      ax, word ptr [0x2795]
0096E2:  03 C2                        add      ax, dx
0096E4:  79 05                        jns      0x96eb
0096E6:  05 B4 00                     add      ax, 0xb4
0096E9:  EB 08                        jmp      0x96f3
0096EB:  3D B4 00                     cmp      ax, 0xb4
0096EE:  7C 03                        jl       0x96f3
0096F0:  2D B4 00                     sub      ax, 0xb4
0096F3:  A3 95 27                     mov      word ptr [0x2795], ax
0096F6:  C7 06 2E 0A 00 00            mov      word ptr [0xa2e], 0
0096FC:  A1 95 27                     mov      ax, word ptr [0x2795]
0096FF:  03 C0                        add      ax, ax
009701:  81 EB A0 05                  sub      bx, 0x5a0
009705:  79 06                        jns      0x970d
009707:  81 C3 A0 05                  add      bx, 0x5a0
00970B:  EB 0A                        jmp      0x9717
00970D:  81 FB A0 05                  cmp      bx, 0x5a0
009711:  7C 04                        jl       0x9717
009713:  81 EB A0 05                  sub      bx, 0x5a0
009717:  50                           push     ax
009718:  8B CB                        mov      cx, bx
00971A:  81 C1 A0 05                  add      cx, 0x5a0
00971E:  8B 16 2C 0A                  mov      dx, word ptr [0xa2c]
009722:  B8 04 00                     mov      ax, 4
009725:  CD 33                        int      0x33
009727:  58                           pop      ax
009728:  89 1E 2A 0A                  mov      word ptr [0xa2a], bx
00972C:  C1 EB 02                     shr      bx, 2
00972F:  89 1E 97 27                  mov      word ptr [0x2797], bx
009733:  F7 06 93 27 08 00            test     word ptr [0x2793], 8
009739:  0F 85 AA 00                  jne      0x97e7
00973D:  8B E8                        mov      bp, ax
00973F:  8B D3                        mov      dx, bx
009741:  3B C2                        cmp      ax, dx
009743:  7F 01                        jg       0x9746
009745:  92                           xchg     dx, ax
009746:  2B C2                        sub      ax, dx
009748:  3D B4 00                     cmp      ax, 0xb4
00974B:  7C 05                        jl       0x9752
00974D:  2D 68 01                     sub      ax, 0x168
009750:  F7 D8                        neg      ax
009752:  83 F8 1F                     cmp      ax, 0x1f
009755:  F8                           clc     
009756:  0F 8E A2 00                  jle      0x97fc
00975A:  F7 06 93 27 04 00            test     word ptr [0x2793], 4
009760:  74 4B                        je       0x97ad
009762:  83 F8 28                     cmp      ax, 0x28
009765:  F8                           clc     
009766:  0F 8C 92 00                  jl       0x97fc
00976A:  8B D3                        mov      dx, bx
00976C:  03 D0                        add      dx, ax
00976E:  81 FA 68 01                  cmp      dx, 0x168
009772:  7C 04                        jl       0x9778
009774:  81 EA 68 01                  sub      dx, 0x168
009778:  3B D5                        cmp      dx, bp
00977A:  74 0F                        je       0x978b
00977C:  83 C5 28                     add      bp, 0x28
00977F:  81 FD 68 01                  cmp      bp, 0x168
009783:  7C 0F                        jl       0x9794
009785:  81 ED 68 01                  sub      bp, 0x168
009789:  EB 09                        jmp      0x9794
00978B:  83 ED 28                     sub      bp, 0x28
00978E:  79 04                        jns      0x9794
009790:  81 C5 68 01                  add      bp, 0x168
009794:  C1 E5 02                     shl      bp, 2
009797:  8B CD                        mov      cx, bp
009799:  89 0E 2A 0A                  mov      word ptr [0xa2a], cx
00979D:  81 C1 A0 05                  add      cx, 0x5a0
0097A1:  8B 16 2C 0A                  mov      dx, word ptr [0xa2c]
0097A5:  B8 04 00                     mov      ax, 4
0097A8:  CD 33                        int      0x33
0097AA:  F8                           clc     
0097AB:  EB 4F                        jmp      0x97fc
0097AD:  8B D3                        mov      dx, bx
0097AF:  03 D0                        add      dx, ax
0097B1:  81 FA 68 01                  cmp      dx, 0x168
0097B5:  7C 04                        jl       0x97bb
0097B7:  81 EA 68 01                  sub      dx, 0x168
0097BB:  3B D5                        cmp      dx, bp
0097BD:  74 10                        je       0x97cf
0097BF:  C6 06 DB 27 01               mov      byte ptr [0x27db], 1
0097C4:  83 EB 1E                     sub      bx, 0x1e
0097C7:  79 18                        jns      0x97e1
0097C9:  81 C3 68 01                  add      bx, 0x168
0097CD:  EB 12                        jmp      0x97e1
0097CF:  C6 06 DB 27 00               mov      byte ptr [0x27db], 0
0097D4:  83 C3 1E                     add      bx, 0x1e
0097D7:  81 FB 68 01                  cmp      bx, 0x168
0097DB:  7C 04                        jl       0x97e1
0097DD:  81 EB 68 01                  sub      bx, 0x168
0097E1:  D1 EB                        shr      bx, 1
0097E3:  89 1E 95 27                  mov      word ptr [0x2795], bx
0097E7:  A1 95 27                     mov      ax, word ptr [0x2795]
0097EA:  A3 6D 2F                     mov      word ptr [0x2f6d], ax
0097ED:  C1 E0 03                     shl      ax, 3
0097F0:  2D A0 00                     sub      ax, 0xa0
0097F3:  A3 A7 27                     mov      word ptr [0x27a7], ax
0097F6:  83 26 2A 0A F8               and      word ptr [0xa2a], 0xfff8
0097FB:  F9                           stc     
0097FC:  9C                           pushf   
0097FD:  8B 1E 2A 0A                  mov      bx, word ptr [0xa2a]
009801:  2B 1E A7 27                  sub      bx, word ptr [0x27a7]
009805:  79 04                        jns      0x980b
009807:  81 C3 A0 05                  add      bx, 0x5a0
00980B:  81 FB A0 05                  cmp      bx, 0x5a0
00980F:  7C 04                        jl       0x9815
009811:  81 EB A0 05                  sub      bx, 0x5a0
009815:  89 1E 2A 0A                  mov      word ptr [0xa2a], bx
009819:  9D                           popf    
00981A:  CB                           retf    
