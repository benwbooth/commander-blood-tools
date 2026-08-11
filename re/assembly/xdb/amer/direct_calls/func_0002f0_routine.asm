; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0002f0
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 70
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3efa9eb40129f518dfc3dc860ca40a59ddb841eee7db8cada6ea9c15839291df

0002F0:  BA C8 03                     mov      dx, 0x3c8
0002F3:  32 C0                        xor      al, al
0002F5:  EE                           out      dx, al
0002F6:  FE C2                        inc      dl
0002F8:  B9 00 03                     mov      cx, 0x300
0002FB:  EE                           out      dx, al
0002FC:  E2 FD                        loop     0x2fb
0002FE:  B8 0C 00                     mov      ax, 0xc
000301:  BA D4 03                     mov      dx, 0x3d4
000304:  BB 00 A0                     mov      bx, 0xa000
000307:  C7 06 26 00 00 40            mov      word ptr [0x26], 0x4000
00030D:  C7 06 28 00 00 A4            mov      word ptr [0x28], 0xa400
000313:  EF                           out      dx, ax
000314:  FC                           cld     
000315:  BA C4 03                     mov      dx, 0x3c4
000318:  B8 02 0F                     mov      ax, 0xf02
00031B:  8E C3                        mov      es, bx
00031D:  33 FF                        xor      di, di
00031F:  B9 00 7D                     mov      cx, 0x7d00
000322:  EF                           out      dx, ax
000323:  66 33 C0                     xor      eax, eax
000326:  F3 AB                        rep stosw word ptr es:[di], ax
000328:  BA DA 03                     mov      dx, 0x3da
00032B:  EC                           in       al, dx
00032C:  A8 08                        test     al, 8
00032E:  75 FB                        jne      0x32b
000330:  EC                           in       al, dx
000331:  A8 08                        test     al, 8
000333:  74 FB                        je       0x330
000335:  C3                           ret     
