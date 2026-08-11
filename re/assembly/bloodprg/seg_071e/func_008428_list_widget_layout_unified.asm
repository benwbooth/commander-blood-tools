; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008428
; seg_off: 071e:0c48
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: list_widget_layout_unified
; label_comment: THE unified vertical-list widget (nav targets, concept menus, SAVE SLOTS): input si = 0xFFFF/0-terminated word-offset list; per-label widths via 0x299:0x13d into DS:0x2AB3; max-width box (min 0x64, or 0x37 with [0xADD] mode +bp init 10); PITCH 11 = add bp,0xB @0x847A (assembly source of the row pitch); save-slot substitution cmp ax,[0x2734] -> si=0x273B (the edit buffer renders in place of the slot name). Continues into centered-rect + draw + mouse/query return || NARROWER EARLIER READING `ship_3d_target_query_layout`: target-list label measurement, centered layout rectangle, draw path, and mouse/query return helper || MERGED 2026-07-25 (audit-fixes #133): one address, two names, the shorter describing a prologue or a single facet. Kept because a narrow reading records a true observation; renamed away because it is not what the routine IS.
; incoming: call@0x001aee->071e:0c48
; incoming: call@0x001b20->071e:0c48
; incoming: call@0x001b8f->071e:0c48
; incoming: call@0x001be1->071e:0c48
; incoming: call@0x00b2e8->071e:0c48
; incoming: call@0x00b318->071e:0c48
; incoming: call@0x00b3da->071e:0c48
; incoming: call@0x00b4c6->071e:0c48
; byte_count: 442
; boundary: cfg_blocks_40_terminals_4
; terminal: jmp 0x8451:1, jmp 0x854e:1, jmp 0x8565:1, retf:1
; direct_callees: none
; indirect_calls: 5
; cxx_source: re/borland/bloodprg/seg_071e/func_008428_list_widget_layout_unified.cpp
; routine_bytes_sha256: 6b881bfc221a2deebed1cfebb4a128d4b8442753f415bae68280cab3f6aa1a29

008428:  53                           push     bx
008429:  51                           push     cx
00842A:  52                           push     dx
00842B:  57                           push     di
00842C:  1E                           push     ds
00842D:  06                           push     es
00842E:  55                           push     bp
00842F:  56                           push     si
008430:  C6 06 E7 27 00               mov      byte ptr [0x27e7], 0
008435:  56                           push     si
008436:  33 ED                        xor      bp, bp
008438:  BA 64 00                     mov      dx, 0x64
00843B:  F6 06 DD 0A 01               test     byte ptr [0xadd], 1
008440:  74 06                        je       0x8448
008442:  BD 0A 00                     mov      bp, 0xa
008445:  BA 37 00                     mov      dx, 0x37
008448:  8C C3                        mov      bx, es
00844A:  8C D8                        mov      ax, ds
00844C:  8E C0                        mov      es, ax
00844E:  BF B3 2A                     mov      di, 0x2ab3
008451:  AD                           lodsw    ax, word ptr [si]
008452:  0B C0                        or       ax, ax
008454:  74 29                        je       0x847f
008456:  83 F8 FF                     cmp      ax, -1
008459:  74 24                        je       0x847f
00845B:  56                           push     si
00845C:  1E                           push     ds
00845D:  8E DB                        mov      ds, bx
00845F:  8B F0                        mov      si, ax
008461:  3B 06 34 27                  cmp      ax, word ptr [0x2734]
008465:  75 03                        jne      0x846a
008467:  BE 3B 27                     mov      si, 0x273b
00846A:  33 C0                        xor      ax, ax
00846C:  9A 3D 01 99 02               lcall    0x299, 0x13d
008471:  AB                           stosw    word ptr es:[di], ax
008472:  3B C2                        cmp      ax, dx
008474:  72 02                        jb       0x8478
008476:  8B D0                        mov      dx, ax
008478:  1F                           pop      ds
008479:  5E                           pop      si
00847A:  83 C5 0B                     add      bp, 0xb
00847D:  EB D2                        jmp      0x8451
00847F:  F6 06 DD 0A 01               test     byte ptr [0xadd], 1
008484:  74 04                        je       0x848a
008486:  B8 37 00                     mov      ax, 0x37
008489:  AB                           stosw    word ptr es:[di], ax
00848A:  F6 06 DC 0A 01               test     byte ptr [0xadc], 1
00848F:  75 0B                        jne      0x849c
008491:  8B CF                        mov      cx, di
008493:  BF B3 2A                     mov      di, 0x2ab3
008496:  2B CF                        sub      cx, di
008498:  8B C2                        mov      ax, dx
00849A:  F3 AB                        rep stosw word ptr es:[di], ax
00849C:  8E C3                        mov      es, bx
00849E:  BE AB 2A                     mov      si, 0x2aab
0084A1:  83 C2 14                     add      dx, 0x14
0084A4:  89 54 04                     mov      word ptr [si + 4], dx
0084A7:  83 C5 08                     add      bp, 8
0084AA:  89 6C 06                     mov      word ptr [si + 6], bp
0084AD:  D1 EA                        shr      dx, 1
0084AF:  2B 16 C6 0A                  sub      dx, word ptr [0xac6]
0084B3:  F7 DA                        neg      dx
0084B5:  8B DA                        mov      bx, dx
0084B7:  89 14                        mov      word ptr [si], dx
0084B9:  81 ED C8 00                  sub      bp, 0xc8
0084BD:  F7 DD                        neg      bp
0084BF:  D1 ED                        shr      bp, 1
0084C1:  8B CD                        mov      cx, bp
0084C3:  89 6C 02                     mov      word ptr [si + 2], bp
0084C6:  8B 54 04                     mov      dx, word ptr [si + 4]
0084C9:  8B 6C 06                     mov      bp, word ptr [si + 6]
0084CC:  5E                           pop      si
0084CD:  F6 06 E6 27 01               test     byte ptr [0x27e6], 1
0084D2:  0F 85 FD 00                  jne      0x85d3
0084D6:  8B FE                        mov      di, si
0084D8:  8B 36 C8 0A                  mov      si, word ptr [0xac8]
0084DC:  9A 0E 04 99 02               lcall    0x299, 0x40e
0084E1:  C6 06 C7 27 00               mov      byte ptr [0x27c7], 0
0084E6:  83 C1 04                     add      cx, 4
0084E9:  87 D1                        xchg     cx, dx
0084EB:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
0084EE:  3B C3                        cmp      ax, bx
0084F0:  7C 49                        jl       0x853b
0084F2:  03 D9                        add      bx, cx
0084F4:  3B C3                        cmp      ax, bx
0084F6:  7F 43                        jg       0x853b
0084F8:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
0084FB:  2B C2                        sub      ax, dx
0084FD:  78 3C                        js       0x853b
0084FF:  83 ED 08                     sub      bp, 8
008502:  3B C5                        cmp      ax, bp
008504:  7D 35                        jge      0x853b
008506:  B3 0B                        mov      bl, 0xb
008508:  F6 F3                        div      bl
00850A:  FE C0                        inc      al
00850C:  A2 C7 27                     mov      byte ptr [0x27c7], al
00850F:  83 3E 34 0A 06               cmp      word ptr [0xa34], 6
008514:  74 0C                        je       0x8522
008516:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
00851C:  C7 06 32 0A 06 00            mov      word ptr [0xa32], 6
008522:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
008527:  74 25                        je       0x854e
008529:  C7 06 32 0A 07 00            mov      word ptr [0xa32], 7
00852F:  A2 E7 27                     mov      byte ptr [0x27e7], al
008532:  33 C0                        xor      ax, ax
008534:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
008539:  EB 13                        jmp      0x854e
00853B:  83 3E 34 0A 01               cmp      word ptr [0xa34], 1
008540:  74 0C                        je       0x854e
008542:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
008548:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
00854E:  BD B3 2A                     mov      bp, 0x2ab3
008551:  8B 1E AF 2A                  mov      bx, word ptr [0x2aaf]
008555:  83 EB 14                     sub      bx, 0x14
008558:  8B 0E AB 2A                  mov      cx, word ptr [0x2aab]
00855C:  83 C1 0A                     add      cx, 0xa
00855F:  8C D8                        mov      ax, ds
008561:  06                           push     es
008562:  1F                           pop      ds
008563:  8E C0                        mov      es, ax
008565:  B0 E8                        mov      al, 0xe8
008567:  26 8B 35                     mov      si, word ptr es:[di]
00856A:  0B F6                        or       si, si
00856C:  74 3A                        je       0x85a8
00856E:  83 FE FF                     cmp      si, -1
008571:  74 35                        je       0x85a8
008573:  3B 36 34 27                  cmp      si, word ptr [0x2734]
008577:  75 03                        jne      0x857c
008579:  BE 3B 27                     mov      si, 0x273b
00857C:  53                           push     bx
00857D:  2B 5E 00                     sub      bx, word ptr [bp]
008580:  D1 EB                        shr      bx, 1
008582:  03 D9                        add      bx, cx
008584:  65 FE 0E C7 27               dec      byte ptr gs:[0x27c7]
008589:  75 0C                        jne      0x8597
00858B:  B0 EF                        mov      al, 0xef
00858D:  65 F6 06 3E 0A 01            test     byte ptr gs:[0xa3e], 1
008593:  74 02                        je       0x8597
008595:  B0 FE                        mov      al, 0xfe
008597:  9A 76 01 99 02               lcall    0x299, 0x176
00859C:  5B                           pop      bx
00859D:  83 C5 02                     add      bp, 2
0085A0:  83 C2 0B                     add      dx, 0xb
0085A3:  83 C7 02                     add      di, 2
0085A6:  EB BD                        jmp      0x8565
0085A8:  8C EE                        mov      si, gs
0085AA:  8E DE                        mov      ds, si
0085AC:  F6 06 DD 0A 01               test     byte ptr [0xadd], 1
0085B1:  74 20                        je       0x85d3
0085B3:  BE 74 01                     mov      si, 0x174
0085B6:  2B 5E 00                     sub      bx, word ptr [bp]
0085B9:  D1 EB                        shr      bx, 1
0085BB:  03 D9                        add      bx, cx
0085BD:  FE 0E C7 27                  dec      byte ptr [0x27c7]
0085C1:  75 0B                        jne      0x85ce
0085C3:  B0 EF                        mov      al, 0xef
0085C5:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
0085CA:  74 02                        je       0x85ce
0085CC:  B0 FE                        mov      al, 0xfe
0085CE:  9A 76 01 99 02               lcall    0x299, 0x176
0085D3:  A0 E7 27                     mov      al, byte ptr [0x27e7]
0085D6:  FE C8                        dec      al
0085D8:  98                           cwde    
0085D9:  5E                           pop      si
0085DA:  5D                           pop      bp
0085DB:  07                           pop      es
0085DC:  1F                           pop      ds
0085DD:  5F                           pop      di
0085DE:  5A                           pop      dx
0085DF:  59                           pop      cx
0085E0:  5B                           pop      bx
0085E1:  CB                           retf    
