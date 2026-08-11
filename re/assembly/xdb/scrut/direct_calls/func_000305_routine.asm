; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000305
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 70
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/direct_calls/func_000305_routine.cpp
; routine_bytes_sha256: 3efa9eb40129f518dfc3dc860ca40a59ddb841eee7db8cada6ea9c15839291df

000305:  BA C8 03                     mov      dx, 0x3c8
000308:  32 C0                        xor      al, al
00030A:  EE                           out      dx, al
00030B:  FE C2                        inc      dl
00030D:  B9 00 03                     mov      cx, 0x300
000310:  EE                           out      dx, al
000311:  E2 FD                        loop     0x310
000313:  B8 0C 00                     mov      ax, 0xc
000316:  BA D4 03                     mov      dx, 0x3d4
000319:  BB 00 A0                     mov      bx, 0xa000
00031C:  C7 06 26 00 00 40            mov      word ptr [0x26], 0x4000
000322:  C7 06 28 00 00 A4            mov      word ptr [0x28], 0xa400
000328:  EF                           out      dx, ax
000329:  FC                           cld     
00032A:  BA C4 03                     mov      dx, 0x3c4
00032D:  B8 02 0F                     mov      ax, 0xf02
000330:  8E C3                        mov      es, bx
000332:  33 FF                        xor      di, di
000334:  B9 00 7D                     mov      cx, 0x7d00
000337:  EF                           out      dx, ax
000338:  66 33 C0                     xor      eax, eax
00033B:  F3 AB                        rep stosw word ptr es:[di], ax
00033D:  BA DA 03                     mov      dx, 0x3da
000340:  EC                           in       al, dx
000341:  A8 08                        test     al, 8
000343:  75 FB                        jne      0x340
000345:  EC                           in       al, dx
000346:  A8 08                        test     al, 8
000348:  74 FB                        je       0x345
00034A:  C3                           ret     
