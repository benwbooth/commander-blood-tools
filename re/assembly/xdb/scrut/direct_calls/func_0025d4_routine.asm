; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0025d4
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 194
; boundary: cfg_blocks_19_terminals_1
; terminal: jmp 0x2644:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/direct_calls/func_0025d4_routine.cpp
; routine_bytes_sha256: ef8eb9a19208f2e1446c47d2783b68c4e903587f2b3c8cc553c4ad4acc28c628

0025D4:  BE 08 23                     mov      si, 0x2308
0025D7:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0025DC:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
0025E1:  26 C7 06 D4 07 00 00         mov      word ptr es:[0x7d4], 0
0025E8:  64 8B 3C                     mov      di, word ptr fs:[si]
0025EB:  83 C6 02                     add      si, 2
0025EE:  56                           push     si
0025EF:  64 8B 4D 2C                  mov      cx, word ptr fs:[di + 0x2c]
0025F3:  64 8B 75 28                  mov      si, word ptr fs:[di + 0x28]
0025F7:  8B 5C 02                     mov      bx, word ptr [si + 2]
0025FA:  8B 7C 04                     mov      di, word ptr [si + 4]
0025FD:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
002600:  8B D0                        mov      dx, ax
002602:  8B 6C 06                     mov      bp, word ptr [si + 6]
002605:  23 45 12                     and      ax, word ptr [di + 0x12]
002608:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
00260C:  75 61                        jne      0x266f
00260E:  0B 55 12                     or       dx, word ptr [di + 0x12]
002611:  3E 0B 56 12                  or       dx, word ptr ds:[bp + 0x12]
002615:  79 07                        jns      0x261e
002617:  26 C7 06 D4 07 01 00         mov      word ptr es:[0x7d4], 1
00261E:  51                           push     cx
00261F:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
002622:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002625:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
002629:  3B D1                        cmp      dx, cx
00262B:  7E 0D                        jle      0x263a
00262D:  3B C1                        cmp      ax, cx
00262F:  7C 1C                        jl       0x264d
002631:  87 DD                        xchg     bp, bx
002633:  91                           xchg     cx, ax
002634:  87 FD                        xchg     bp, di
002636:  87 CA                        xchg     dx, cx
002638:  EB 0A                        jmp      0x2644
00263A:  3B C2                        cmp      ax, dx
00263C:  7E 0F                        jle      0x264d
00263E:  87 DD                        xchg     bp, bx
002640:  91                           xchg     cx, ax
002641:  87 DF                        xchg     di, bx
002643:  92                           xchg     dx, ax
002644:  89 5C 02                     mov      word ptr [si + 2], bx
002647:  89 7C 04                     mov      word ptr [si + 4], di
00264A:  89 6C 06                     mov      word ptr [si + 6], bp
00264D:  2B D0                        sub      dx, ax
00264F:  2B C8                        sub      cx, ax
002651:  81 FA F4 01                  cmp      dx, 0x1f4
002655:  73 17                        jae      0x266e
002657:  81 F9 F4 01                  cmp      cx, 0x1f4
00265B:  73 11                        jae      0x266e
00265D:  03 C0                        add      ax, ax
00265F:  BF 4E 09                     mov      di, 0x94e
002662:  78 02                        js       0x2666
002664:  03 F8                        add      di, ax
002666:  26 8B 1D                     mov      bx, word ptr es:[di]
002669:  26 89 35                     mov      word ptr es:[di], si
00266C:  89 1C                        mov      word ptr [si], bx
00266E:  59                           pop      cx
00266F:  83 C6 08                     add      si, 8
002672:  E2 83                        loop     0x25f7
002674:  5E                           pop      si
002675:  26 F7 06 D4 07 FF FF         test     word ptr es:[0x7d4], 0xffff
00267C:  74 0F                        je       0x268d
00267E:  64 8B 44 FE                  mov      ax, word ptr fs:[si - 2]
002682:  26 C7 06 D4 07 00 00         mov      word ptr es:[0x7d4], 0
002689:  64 A3 82 22                  mov      word ptr fs:[0x2282], ax
00268D:  64 F7 04 FF FF               test     word ptr fs:[si], 0xffff
002692:  0F 85 52 FF                  jne      0x25e8
