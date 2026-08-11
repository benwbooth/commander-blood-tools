; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004471
; seg_off: 0299:14e1
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: sprite_slot_dirty_range_render
; label_comment: walks active sprite slots against dirty rects, selects blitter dispatch, and clears dirty bit
; incoming: call@0x00786e->0299:14e1
; incoming: call@0x00789a->0299:14e1
; incoming: call@0x008da0->0299:14e1
; incoming: call@0x00910c->0299:14e1
; incoming: call@0x00913d->0299:14e1
; incoming: call@0x0091fe->0299:14e1
; incoming: call@0x00957a->0299:14e1
; byte_count: 177
; boundary: cfg_blocks_11_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 757fdd10edb7c7068597766a63b7825a1700d9760826d7f037ba2f4127628e0a

004471:  66 50                        push     eax
004473:  53                           push     bx
004474:  51                           push     cx
004475:  52                           push     dx
004476:  06                           push     es
004477:  57                           push     di
004478:  1E                           push     ds
004479:  56                           push     si
00447A:  66 55                        push     ebp
00447C:  8B E8                        mov      bp, ax
00447E:  66 C1 E5 10                  shl      ebp, 0x10
004482:  8B EB                        mov      bp, bx
004484:  8C E8                        mov      ax, gs
004486:  8E D8                        mov      ds, ax
004488:  8E C0                        mov      es, ax
00448A:  BF 12 66                     mov      di, 0x6612
00448D:  8B 05                        mov      ax, word ptr [di]
00448F:  0B C0                        or       ax, ax
004491:  0F 88 81 00                  js       0x4516
004495:  8B DD                        mov      bx, bp
004497:  66 C1 ED 10                  shr      ebp, 0x10
00449B:  8B C5                        mov      ax, bp
00449D:  43                           inc      bx
00449E:  8B CB                        mov      cx, bx
0044A0:  2B C8                        sub      cx, ax
0044A2:  BF 12 62                     mov      di, 0x6212
0044A5:  C1 E3 05                     shl      bx, 5
0044A8:  03 FB                        add      di, bx
0044AA:  83 EF 20                     sub      di, 0x20
0044AD:  8B 05                        mov      ax, word ptr [di]
0044AF:  A8 01                        test     al, 1
0044B1:  74 5E                        je       0x4511
0044B3:  8B D8                        mov      bx, ax
0044B5:  D1 EB                        shr      bx, 1
0044B7:  83 E3 0E                     and      bx, 0xe
0044BA:  2E 8B 9F 92 15               mov      bx, word ptr cs:[bx + 0x1592]
0044BF:  2E 89 1E A2 15               mov      word ptr cs:[0x15a2], bx
0044C4:  C1 E8 06                     shr      ax, 6
0044C7:  2E 0F 92 06 DF 14            setb     byte ptr cs:[0x14df]
0044CD:  D1 E8                        shr      ax, 1
0044CF:  2E 0F 92 06 E0 14            setb     byte ptr cs:[0x14e0]
0044D5:  BE 12 66                     mov      si, 0x6612
0044D8:  8B 45 08                     mov      ax, word ptr [di + 8]
0044DB:  8B 5D 0A                     mov      bx, word ptr [di + 0xa]
0044DE:  8B D0                        mov      dx, ax
0044E0:  03 55 0C                     add      dx, word ptr [di + 0xc]
0044E3:  8B EB                        mov      bp, bx
0044E5:  03 6D 0E                     add      bp, word ptr [di + 0xe]
0044E8:  83 C7 18                     add      di, 0x18
0044EB:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
0044ED:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
0044EF:  83 EF 20                     sub      di, 0x20
0044F2:  3B 45 1A                     cmp      ax, word ptr [di + 0x1a]
0044F5:  7D 14                        jge      0x450b
0044F7:  3B 5D 1E                     cmp      bx, word ptr [di + 0x1e]
0044FA:  7D 0F                        jge      0x450b
0044FC:  3B 55 18                     cmp      dx, word ptr [di + 0x18]
0044FF:  7E 0A                        jle      0x450b
004501:  3B 6D 1C                     cmp      bp, word ptr [di + 0x1c]
004504:  7E 05                        jle      0x450b
004506:  2E FF 16 A2 15               call     word ptr cs:[0x15a2]
00450B:  F7 04 00 80                  test     word ptr [si], 0x8000
00450F:  74 D7                        je       0x44e8
004511:  80 25 FD                     and      byte ptr [di], 0xfd
004514:  E2 94                        loop     0x44aa
004516:  66 5D                        pop      ebp
004518:  5E                           pop      si
004519:  1F                           pop      ds
00451A:  5F                           pop      di
00451B:  07                           pop      es
00451C:  5A                           pop      dx
00451D:  59                           pop      cx
00451E:  5B                           pop      bx
00451F:  66 58                        pop      eax
004521:  CB                           retf    
