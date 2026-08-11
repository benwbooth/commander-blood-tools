; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002612
; seg_off: 01ce:0332
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ascii_digit_parse
; label_comment: ASCII digit parse: bl=[si]; cmp bl,0x39 ('9'); jg. Parses a numeric string, testing each byte against the '0'..'9' range
; incoming: call@0x000766->01ce:0332
; byte_count: 83
; boundary: cfg_blocks_11_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_002612_ascii_digit_parse.cpp
; routine_bytes_sha256: 79aec148e4473edca687e714d782a1a810be0ea7a2b45d096a29b59bb4277366

002612:  53                           push     bx
002613:  51                           push     cx
002614:  52                           push     dx
002615:  56                           push     si
002616:  57                           push     di
002617:  33 D2                        xor      dx, dx
002619:  33 C0                        xor      ax, ax
00261B:  8A 1C                        mov      bl, byte ptr [si]
00261D:  80 FB 39                     cmp      bl, 0x39
002620:  7F 3D                        jg       0x265f
002622:  80 FB 30                     cmp      bl, 0x30
002625:  7D 0D                        jge      0x2634
002627:  46                           inc      si
002628:  80 FB 2B                     cmp      bl, 0x2b
00262B:  74 07                        je       0x2634
00262D:  80 FB 2D                     cmp      bl, 0x2d
002630:  75 2D                        jne      0x265f
002632:  F7 D2                        not      dx
002634:  BF DA 02                     mov      di, 0x2da
002637:  33 C9                        xor      cx, cx
002639:  32 FF                        xor      bh, bh
00263B:  46                           inc      si
00263C:  8A 1C                        mov      bl, byte ptr [si]
00263E:  41                           inc      cx
00263F:  80 FB 39                     cmp      bl, 0x39
002642:  7F 05                        jg       0x2649
002644:  80 FB 30                     cmp      bl, 0x30
002647:  7D F2                        jge      0x263b
002649:  4E                           dec      si
00264A:  8A 1C                        mov      bl, byte ptr [si]
00264C:  80 EB 30                     sub      bl, 0x30
00264F:  D0 E3                        shl      bl, 1
002651:  2E 03 01                     add      ax, word ptr cs:[bx + di]
002654:  83 C7 14                     add      di, 0x14
002657:  E2 F0                        loop     0x2649
002659:  0B D2                        or       dx, dx
00265B:  74 02                        je       0x265f
00265D:  F7 D8                        neg      ax
00265F:  5F                           pop      di
002660:  5E                           pop      si
002661:  5A                           pop      dx
002662:  59                           pop      cx
002663:  5B                           pop      bx
002664:  CB                           retf    
