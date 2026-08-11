; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000270
; group: manu3_labeled
; provenance: direct_call_from_0x0, direct_call_from_0x150, label:manu3_matrix_build, manu3 matrix builder
; label: manu3_matrix_build
; label_comment: builds the Q15 rotation matrix at 0x2250..0x226F from THREE Euler angles at state+0x4E/0x50/0x52 (state block 0x2336, active record +0x5E; angles masked 0xFFC = dword offsets into the sin/cos tables, pairs at [tbl+0x26]/[tbl+0x28]); combines via angle-sum identities (add/sub + sar 1)
; byte_count: 519
; boundary: cfg_blocks_1_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000270_manu3_matrix_build.cpp
; routine_bytes_sha256: f383ad334ccd68c5dfecb155d1522cf9d09648f8dde3c8f815eb98fc0583c813

000270:  BF 36 23                     mov      di, 0x2336
000273:  A1 F2 22                     mov      ax, word ptr [0x22f2]
000276:  A3 4A 22                     mov      word ptr [0x224a], ax
000279:  83 C7 5E                     add      di, 0x5e
00027C:  89 3E 48 22                  mov      word ptr [0x2248], di
000280:  8B 45 52                     mov      ax, word ptr [di + 0x52]
000283:  BB FC 0F                     mov      bx, 0xffc
000286:  8B 75 4E                     mov      si, word ptr [di + 0x4e]
000289:  23 C3                        and      ax, bx
00028B:  8B 7D 50                     mov      di, word ptr [di + 0x50]
00028E:  23 F3                        and      si, bx
000290:  23 FB                        and      di, bx
000292:  89 3E 20 00                  mov      word ptr [0x20], di
000296:  89 36 22 00                  mov      word ptr [0x22], si
00029A:  A3 24 00                     mov      word ptr [0x24], ax
00029D:  03 F8                        add      di, ax
00029F:  66 0F BF 94 28 00            movsx    edx, word ptr [si + 0x28]
0002A5:  2B F7                        sub      si, di
0002A7:  66 03 D2                     add      edx, edx
0002AA:  23 F3                        and      si, bx
0002AC:  66 F7 DA                     neg      edx
0002AF:  66 89 16 64 22               mov      dword ptr [0x2264], edx
0002B4:  66 0F BF 84 26 00            movsx    eax, word ptr [si + 0x26]
0002BA:  66 0F BF AC 28 00            movsx    ebp, word ptr [si + 0x28]
0002C0:  03 F7                        add      si, di
0002C2:  03 F7                        add      si, di
0002C4:  23 F3                        and      si, bx
0002C6:  66 0F BF 8C 26 00            movsx    ecx, word ptr [si + 0x26]
0002CC:  66 0F BF 94 28 00            movsx    edx, word ptr [si + 0x28]
0002D2:  66 2B C1                     sub      eax, ecx
0002D5:  66 03 EA                     add      ebp, edx
0002D8:  66 D1 F8                     sar      eax, 1
0002DB:  66 D1 FD                     sar      ebp, 1
0002DE:  23 FB                        and      di, bx
0002E0:  66 0F BF 8D 28 00            movsx    ecx, word ptr [di + 0x28]
0002E6:  66 0F BF 95 26 00            movsx    edx, word ptr [di + 0x26]
0002EC:  66 03 C1                     add      eax, ecx
0002EF:  66 03 EA                     add      ebp, edx
0002F2:  66 A3 54 22                  mov      dword ptr [0x2254], eax
0002F6:  66 F7 D8                     neg      eax
0002F9:  66 A3 68 22                  mov      dword ptr [0x2268], eax
0002FD:  66 89 2E 50 22               mov      dword ptr [0x2250], ebp
000302:  66 89 2E 6C 22               mov      dword ptr [0x226c], ebp
000307:  8B 3E 20 00                  mov      di, word ptr [0x20]
00030B:  2B 3E 24 00                  sub      di, word ptr [0x24]
00030F:  8B 36 22 00                  mov      si, word ptr [0x22]
000313:  2B F7                        sub      si, di
000315:  23 F3                        and      si, bx
000317:  66 0F BF 84 26 00            movsx    eax, word ptr [si + 0x26]
00031D:  66 0F BF AC 28 00            movsx    ebp, word ptr [si + 0x28]
000323:  03 F7                        add      si, di
000325:  03 F7                        add      si, di
000327:  23 F3                        and      si, bx
000329:  66 0F BF 8C 26 00            movsx    ecx, word ptr [si + 0x26]
00032F:  66 0F BF 94 28 00            movsx    edx, word ptr [si + 0x28]
000335:  66 2B C1                     sub      eax, ecx
000338:  66 03 EA                     add      ebp, edx
00033B:  66 D1 F8                     sar      eax, 1
00033E:  66 D1 FD                     sar      ebp, 1
000341:  23 FB                        and      di, bx
000343:  66 0F BF 8D 28 00            movsx    ecx, word ptr [di + 0x28]
000349:  66 0F BF 95 26 00            movsx    edx, word ptr [di + 0x26]
00034F:  66 2B C8                     sub      ecx, eax
000352:  66 2B D5                     sub      edx, ebp
000355:  66 29 0E 54 22               sub      dword ptr [0x2254], ecx
00035A:  66 29 0E 68 22               sub      dword ptr [0x2268], ecx
00035F:  66 01 16 50 22               add      dword ptr [0x2250], edx
000364:  66 29 16 6C 22               sub      dword ptr [0x226c], edx
000369:  8B 3E 24 00                  mov      di, word ptr [0x24]
00036D:  8B 2E 22 00                  mov      bp, word ptr [0x22]
000371:  8B F7                        mov      si, di
000373:  03 FD                        add      di, bp
000375:  2B F5                        sub      si, bp
000377:  23 FB                        and      di, bx
000379:  23 F3                        and      si, bx
00037B:  66 0F BF 85 26 00            movsx    eax, word ptr [di + 0x26]
000381:  66 0F BF AD 28 00            movsx    ebp, word ptr [di + 0x28]
000387:  66 0F BF 8C 26 00            movsx    ecx, word ptr [si + 0x26]
00038D:  66 0F BF 94 28 00            movsx    edx, word ptr [si + 0x28]
000393:  66 03 C1                     add      eax, ecx
000396:  66 03 EA                     add      ebp, edx
000399:  66 F7 DD                     neg      ebp
00039C:  66 A3 60 22                  mov      dword ptr [0x2260], eax
0003A0:  66 89 2E 5C 22               mov      dword ptr [0x225c], ebp
0003A5:  8B 3E 20 00                  mov      di, word ptr [0x20]
0003A9:  8B 2E 22 00                  mov      bp, word ptr [0x22]
0003AD:  8B F7                        mov      si, di
0003AF:  03 FD                        add      di, bp
0003B1:  2B F5                        sub      si, bp
0003B3:  23 FB                        and      di, bx
0003B5:  23 F3                        and      si, bx
0003B7:  66 0F BF 85 26 00            movsx    eax, word ptr [di + 0x26]
0003BD:  66 0F BF AD 28 00            movsx    ebp, word ptr [di + 0x28]
0003C3:  66 0F BF 8C 26 00            movsx    ecx, word ptr [si + 0x26]
0003C9:  66 0F BF 94 28 00            movsx    edx, word ptr [si + 0x28]
0003CF:  66 03 C1                     add      eax, ecx
0003D2:  66 03 EA                     add      ebp, edx
0003D5:  66 A3 70 22                  mov      dword ptr [0x2270], eax
0003D9:  66 89 2E 58 22               mov      dword ptr [0x2258], ebp
0003DE:  8B 3E 48 22                  mov      di, word ptr [0x2248]
0003E2:  66 0F BF 5D 54               movsx    ebx, word ptr [di + 0x54]
0003E7:  66 A1 58 22                  mov      eax, dword ptr [0x2258]
0003EB:  66 F7 EB                     imul     ebx
0003EE:  66 C1 F8 10                  sar      eax, 0x10
0003F2:  66 01 45 42                  add      dword ptr [di + 0x42], eax
0003F6:  66 A1 64 22                  mov      eax, dword ptr [0x2264]
0003FA:  66 F7 EB                     imul     ebx
0003FD:  66 C1 F8 10                  sar      eax, 0x10
000401:  66 83 D0 00                  adc      eax, 0
000405:  66 01 45 46                  add      dword ptr [di + 0x46], eax
000409:  66 A1 70 22                  mov      eax, dword ptr [0x2270]
00040D:  66 F7 EB                     imul     ebx
000410:  66 C1 F8 10                  sar      eax, 0x10
000414:  66 01 45 4A                  add      dword ptr [di + 0x4a], eax
000418:  8B 35                        mov      si, word ptr [di]
00041A:  66 0F BF 5D 42               movsx    ebx, word ptr [di + 0x42]
00041F:  66 0F BF 4D 46               movsx    ecx, word ptr [di + 0x46]
000424:  66 0F BF 55 4A               movsx    edx, word ptr [di + 0x4a]
000429:  66 8B 44 2A                  mov      eax, dword ptr [si + 0x2a]
00042D:  66 0F AF C3                  imul     eax, ebx
000431:  66 8B E8                     mov      ebp, eax
000434:  66 8B 44 2E                  mov      eax, dword ptr [si + 0x2e]
000438:  66 0F AF C1                  imul     eax, ecx
00043C:  66 03 E8                     add      ebp, eax
00043F:  66 8B 44 32                  mov      eax, dword ptr [si + 0x32]
000443:  66 0F AF C2                  imul     eax, edx
000447:  66 03 C5                     add      eax, ebp
00044A:  66 03 44 3E                  add      eax, dword ptr [si + 0x3e]
00044E:  66 89 45 3E                  mov      dword ptr [di + 0x3e], eax
000452:  66 8B 44 1E                  mov      eax, dword ptr [si + 0x1e]
000456:  66 0F AF C3                  imul     eax, ebx
00045A:  66 8B E8                     mov      ebp, eax
00045D:  66 8B 44 22                  mov      eax, dword ptr [si + 0x22]
000461:  66 0F AF C1                  imul     eax, ecx
000465:  66 03 E8                     add      ebp, eax
000468:  66 8B 44 26                  mov      eax, dword ptr [si + 0x26]
00046C:  66 0F AF C2                  imul     eax, edx
000470:  66 03 C5                     add      eax, ebp
000473:  66 03 44 3A                  add      eax, dword ptr [si + 0x3a]
