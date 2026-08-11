; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0024cf
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 163
; boundary: cfg_blocks_17_terminals_1
; terminal: jmp 0x2538:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: e784f6305eb359e3b85baaf4a5c87d0600db42cea9525f094cde2ee2ccc0bcc2

0024CF:  BE 08 23                     mov      si, 0x2308
0024D2:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0024D7:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
0024DC:  64 8B 3C                     mov      di, word ptr fs:[si]
0024DF:  83 C6 02                     add      si, 2
0024E2:  56                           push     si
0024E3:  64 8B 4D 2C                  mov      cx, word ptr fs:[di + 0x2c]
0024E7:  64 8B 75 28                  mov      si, word ptr fs:[di + 0x28]
0024EB:  8B 5C 02                     mov      bx, word ptr [si + 2]
0024EE:  8B 7C 04                     mov      di, word ptr [si + 4]
0024F1:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
0024F4:  8B D0                        mov      dx, ax
0024F6:  8B 6C 06                     mov      bp, word ptr [si + 6]
0024F9:  23 45 12                     and      ax, word ptr [di + 0x12]
0024FC:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
002500:  75 61                        jne      0x2563
002502:  0B 55 12                     or       dx, word ptr [di + 0x12]
002505:  3E 0B 56 12                  or       dx, word ptr ds:[bp + 0x12]
002509:  79 07                        jns      0x2512
00250B:  64 C7 06 82 22 01 00         mov      word ptr fs:[0x2282], 1
002512:  51                           push     cx
002513:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
002516:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
002519:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00251D:  3B D1                        cmp      dx, cx
00251F:  7E 0D                        jle      0x252e
002521:  3B C1                        cmp      ax, cx
002523:  7C 1C                        jl       0x2541
002525:  87 DD                        xchg     bp, bx
002527:  91                           xchg     cx, ax
002528:  87 FD                        xchg     bp, di
00252A:  87 CA                        xchg     dx, cx
00252C:  EB 0A                        jmp      0x2538
00252E:  3B C2                        cmp      ax, dx
002530:  7E 0F                        jle      0x2541
002532:  87 DD                        xchg     bp, bx
002534:  91                           xchg     cx, ax
002535:  87 DF                        xchg     di, bx
002537:  92                           xchg     dx, ax
002538:  89 5C 02                     mov      word ptr [si + 2], bx
00253B:  89 7C 04                     mov      word ptr [si + 4], di
00253E:  89 6C 06                     mov      word ptr [si + 6], bp
002541:  2B D0                        sub      dx, ax
002543:  2B C8                        sub      cx, ax
002545:  81 FA F4 01                  cmp      dx, 0x1f4
002549:  73 17                        jae      0x2562
00254B:  81 F9 F4 01                  cmp      cx, 0x1f4
00254F:  73 11                        jae      0x2562
002551:  03 C0                        add      ax, ax
002553:  BF 4C 09                     mov      di, 0x94c
002556:  78 02                        js       0x255a
002558:  03 F8                        add      di, ax
00255A:  26 8B 1D                     mov      bx, word ptr es:[di]
00255D:  26 89 35                     mov      word ptr es:[di], si
002560:  89 1C                        mov      word ptr [si], bx
002562:  59                           pop      cx
002563:  83 C6 08                     add      si, 8
002566:  E2 83                        loop     0x24eb
002568:  5E                           pop      si
002569:  64 F7 04 FF FF               test     word ptr fs:[si], 0xffff
00256E:  0F 85 6A FF                  jne      0x24dc
