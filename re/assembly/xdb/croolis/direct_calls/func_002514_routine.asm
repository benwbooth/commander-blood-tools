; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x002514
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 194
; boundary: cfg_blocks_19_terminals_1
; terminal: jmp 0x2584:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ef8eb9a19208f2e1446c47d2783b68c4e903587f2b3c8cc553c4ad4acc28c628

002514:  BE 08 23                     mov      si, 0x2308
002517:  64 8E 06 06 00               mov      es, word ptr fs:[6]
00251C:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
002521:  26 C7 06 D4 07 00 00         mov      word ptr es:[0x7d4], 0
002528:  64 8B 3C                     mov      di, word ptr fs:[si]
00252B:  83 C6 02                     add      si, 2
00252E:  56                           push     si
00252F:  64 8B 4D 2C                  mov      cx, word ptr fs:[di + 0x2c]
002533:  64 8B 75 28                  mov      si, word ptr fs:[di + 0x28]
002537:  8B 5C 02                     mov      bx, word ptr [si + 2]
00253A:  8B 7C 04                     mov      di, word ptr [si + 4]
00253D:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
002540:  8B D0                        mov      dx, ax
002542:  8B 6C 06                     mov      bp, word ptr [si + 6]
002545:  23 45 12                     and      ax, word ptr [di + 0x12]
002548:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
00254C:  75 61                        jne      0x25af
00254E:  0B 55 12                     or       dx, word ptr [di + 0x12]
002551:  3E 0B 56 12                  or       dx, word ptr ds:[bp + 0x12]
002555:  79 07                        jns      0x255e
002557:  26 C7 06 D4 07 01 00         mov      word ptr es:[0x7d4], 1
00255E:  51                           push     cx
00255F:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
002562:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002565:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
002569:  3B D1                        cmp      dx, cx
00256B:  7E 0D                        jle      0x257a
00256D:  3B C1                        cmp      ax, cx
00256F:  7C 1C                        jl       0x258d
002571:  87 DD                        xchg     bp, bx
002573:  91                           xchg     cx, ax
002574:  87 FD                        xchg     bp, di
002576:  87 CA                        xchg     dx, cx
002578:  EB 0A                        jmp      0x2584
00257A:  3B C2                        cmp      ax, dx
00257C:  7E 0F                        jle      0x258d
00257E:  87 DD                        xchg     bp, bx
002580:  91                           xchg     cx, ax
002581:  87 DF                        xchg     di, bx
002583:  92                           xchg     dx, ax
002584:  89 5C 02                     mov      word ptr [si + 2], bx
002587:  89 7C 04                     mov      word ptr [si + 4], di
00258A:  89 6C 06                     mov      word ptr [si + 6], bp
00258D:  2B D0                        sub      dx, ax
00258F:  2B C8                        sub      cx, ax
002591:  81 FA F4 01                  cmp      dx, 0x1f4
002595:  73 17                        jae      0x25ae
002597:  81 F9 F4 01                  cmp      cx, 0x1f4
00259B:  73 11                        jae      0x25ae
00259D:  03 C0                        add      ax, ax
00259F:  BF 4E 09                     mov      di, 0x94e
0025A2:  78 02                        js       0x25a6
0025A4:  03 F8                        add      di, ax
0025A6:  26 8B 1D                     mov      bx, word ptr es:[di]
0025A9:  26 89 35                     mov      word ptr es:[di], si
0025AC:  89 1C                        mov      word ptr [si], bx
0025AE:  59                           pop      cx
0025AF:  83 C6 08                     add      si, 8
0025B2:  E2 83                        loop     0x2537
0025B4:  5E                           pop      si
0025B5:  26 F7 06 D4 07 FF FF         test     word ptr es:[0x7d4], 0xffff
0025BC:  74 0F                        je       0x25cd
0025BE:  64 8B 44 FE                  mov      ax, word ptr fs:[si - 2]
0025C2:  26 C7 06 D4 07 00 00         mov      word ptr es:[0x7d4], 0
0025C9:  64 A3 82 22                  mov      word ptr fs:[0x2282], ax
0025CD:  64 F7 04 FF FF               test     word ptr fs:[si], 0xffff
0025D2:  0F 85 52 FF                  jne      0x2528
