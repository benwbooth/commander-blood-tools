; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0085e2
; seg_off: 071e:0e02
; group: seg_071e
; provenance: recursive_graph
; label: nav_choice_dispatch
; label_comment: navigation-choice hit-test and activation preamble before cs:0x0f29 table dispatch
; byte_count: 295
; boundary: cfg_blocks_18_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 3
; routine_bytes_sha256: 38cdc1831a3f2bdbdac9d5b2584fef467fe8dd0186079929b9eeccc514fe5581

0085E2:  53                           push     bx
0085E3:  51                           push     cx
0085E4:  52                           push     dx
0085E5:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0085EA:  0F 85 17 01                  jne      0x8705
0085EE:  A0 36 27                     mov      al, byte ptr [0x2736]
0085F1:  0A 06 37 27                  or       al, byte ptr [0x2737]
0085F5:  0A 06 9B 25                  or       al, byte ptr [0x259b]
0085F9:  0A 06 13 0B                  or       al, byte ptr [0xb13]
0085FD:  0F 85 04 01                  jne      0x8705
008601:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
008606:  0F 85 FB 00                  jne      0x8705
00860A:  8B 1E 19 2A                  mov      bx, word ptr [0x2a19]
00860E:  0B DB                        or       bx, bx
008610:  0F 85 DD 00                  jne      0x86f1
008614:  A1 95 27                     mov      ax, word ptr [0x2795]
008617:  83 F8 3C                     cmp      ax, 0x3c
00861A:  0F 8F E7 00                  jg       0x8705
00861E:  83 F8 28                     cmp      ax, 0x28
008621:  0F 8C E0 00                  jl       0x8705
008625:  9A D7 05 00 00               lcall    0, 0x5d7
00862A:  50                           push     ax
00862B:  BA C8 03                     mov      dx, 0x3c8
00862E:  B0 7B                        mov      al, 0x7b
008630:  EE                           out      dx, al
008631:  FE C2                        inc      dl
008633:  B9 05 00                     mov      cx, 5
008636:  B0 10                        mov      al, 0x10
008638:  EE                           out      dx, al
008639:  B0 0C                        mov      al, 0xc
00863B:  EE                           out      dx, al
00863C:  32 C0                        xor      al, al
00863E:  EE                           out      dx, al
00863F:  E2 F5                        loop     0x8636
008641:  58                           pop      ax
008642:  83 E8 2D                     sub      ax, 0x2d
008645:  8B C8                        mov      cx, ax
008647:  8B 1E 2A 0A                  mov      bx, word ptr [0xa2a]
00864B:  C1 E0 03                     shl      ax, 3
00864E:  F7 D8                        neg      ax
008650:  05 E8 00                     add      ax, 0xe8
008653:  83 C0 37                     add      ax, 0x37
008656:  3B D8                        cmp      bx, ax
008658:  0F 8F A9 00                  jg       0x8705
00865C:  83 E8 6E                     sub      ax, 0x6e
00865F:  0F 88 A2 00                  js       0x8705
008663:  3B D8                        cmp      bx, ax
008665:  0F 8C 9C 00                  jl       0x8705
008669:  8B C1                        mov      ax, cx
00866B:  0B C0                        or       ax, ax
00866D:  79 02                        jns      0x8671
00866F:  F7 D8                        neg      ax
008671:  83 EF 0F                     sub      di, 0xf
008674:  BB 48 00                     mov      bx, 0x48
008677:  03 D8                        add      bx, ax
008679:  B1 12                        mov      cl, 0x12
00867B:  C1 E8 02                     shr      ax, 2
00867E:  03 D8                        add      bx, ax
008680:  D0 E8                        shr      al, 1
008682:  2A C8                        sub      cl, al
008684:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
008687:  2B C3                        sub      ax, bx
008689:  78 7A                        js       0x8705
00868B:  F6 F1                        div      cl
00868D:  3C 05                        cmp      al, 5
00868F:  7D 74                        jge      0x8705
008691:  98                           cwde    
008692:  8B D8                        mov      bx, ax
008694:  BA C8 03                     mov      dx, 0x3c8
008697:  83 C0 7B                     add      ax, 0x7b
00869A:  EE                           out      dx, al
00869B:  FE C2                        inc      dl
00869D:  B0 3F                        mov      al, 0x3f
00869F:  EE                           out      dx, al
0086A0:  32 C0                        xor      al, al
0086A2:  EE                           out      dx, al
0086A3:  EE                           out      dx, al
0086A4:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
0086A9:  74 5A                        je       0x8705
0086AB:  C7 06 32 0A 05 00            mov      word ptr [0xa32], 5
0086B1:  43                           inc      bx
0086B2:  89 1E 19 2A                  mov      word ptr [0x2a19], bx
0086B6:  80 0E 93 27 0C               or       byte ptr [0x2793], 0xc
0086BB:  C7 06 9B 27 5A 00            mov      word ptr [0x279b], 0x5a
0086C1:  C6 06 65 25 01               mov      byte ptr [0x2565], 1
0086C6:  8A C3                        mov      al, bl
0086C8:  FE C8                        dec      al
0086CA:  B1 12                        mov      cl, 0x12
0086CC:  F6 E1                        mul      cl
0086CE:  83 C0 50                     add      ax, 0x50
0086D1:  A3 3F 25                     mov      word ptr [0x253f], ax
0086D4:  C6 06 DC 0A 01               mov      byte ptr [0xadc], 1
0086D9:  C7 06 C6 0A 64 00            mov      word ptr [0xac6], 0x64
0086DF:  C6 06 DD 0A 01               mov      byte ptr [0xadd], 1
0086E4:  C6 06 DA 0A 0A               mov      byte ptr [0xada], 0xa
0086E9:  B8 04 00                     mov      ax, 4
0086EC:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
0086F1:  F6 06 93 27 08               test     byte ptr [0x2793], 8
0086F6:  75 0D                        jne      0x8705
0086F8:  4B                           dec      bx
0086F9:  03 DB                        add      bx, bx
0086FB:  F6 06 65 25 01               test     byte ptr [0x2565], 1
008700:  2E FF 97 29 0F               call     word ptr cs:[bx + 0xf29]
008705:  5A                           pop      dx
008706:  59                           pop      cx
008707:  5B                           pop      bx
008708:  C3                           ret     
