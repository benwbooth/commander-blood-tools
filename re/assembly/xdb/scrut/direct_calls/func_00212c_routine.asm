; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00212c
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 1192
; boundary: cfg_blocks_28_terminals_2
; terminal: jmp 0x24c7:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/direct_calls/func_00212c_routine.cpp
; routine_bytes_sha256: 684386c05fa5f8cf92643bbc57b996af068081eb76ff69ccf5278e67acb5691a

00212C:  64 8B 3E 78 22               mov      di, word ptr fs:[0x2278]
002131:  8E 06 02 00                  mov      es, word ptr [2]
002135:  8B 45 1A                     mov      ax, word ptr [di + 0x1a]
002138:  A3 7C 22                     mov      word ptr [0x227c], ax
00213B:  57                           push     di
00213C:  8B 7D 16                     mov      di, word ptr [di + 0x16]
00213F:  83 C7 5E                     add      di, 0x5e
002142:  89 3E 7A 22                  mov      word ptr [0x227a], di
002146:  8B 45 52                     mov      ax, word ptr [di + 0x52]
002149:  BB FC 0F                     mov      bx, 0xffc
00214C:  8B 75 4E                     mov      si, word ptr [di + 0x4e]
00214F:  23 C3                        and      ax, bx
002151:  8B 7D 50                     mov      di, word ptr [di + 0x50]
002154:  23 F3                        and      si, bx
002156:  23 FB                        and      di, bx
002158:  89 3E 30 00                  mov      word ptr [0x30], di
00215C:  89 36 32 00                  mov      word ptr [0x32], si
002160:  A3 34 00                     mov      word ptr [0x34], ax
002163:  03 F8                        add      di, ax
002165:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
00216B:  2B F7                        sub      si, di
00216D:  66 03 D2                     add      edx, edx
002170:  23 F3                        and      si, bx
002172:  66 F7 DA                     neg      edx
002175:  66 89 16 98 22               mov      dword ptr [0x2298], edx
00217A:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
002180:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
002186:  03 F7                        add      si, di
002188:  03 F7                        add      si, di
00218A:  23 F3                        and      si, bx
00218C:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
002192:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002198:  66 2B C1                     sub      eax, ecx
00219B:  66 03 EA                     add      ebp, edx
00219E:  66 D1 F8                     sar      eax, 1
0021A1:  66 D1 FD                     sar      ebp, 1
0021A4:  23 FB                        and      di, bx
0021A6:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
0021AC:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
0021B2:  66 03 C1                     add      eax, ecx
0021B5:  66 03 EA                     add      ebp, edx
0021B8:  66 A3 88 22                  mov      dword ptr [0x2288], eax
0021BC:  66 F7 D8                     neg      eax
0021BF:  66 A3 9C 22                  mov      dword ptr [0x229c], eax
0021C3:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
0021C8:  66 89 2E A0 22               mov      dword ptr [0x22a0], ebp
0021CD:  8B 3E 30 00                  mov      di, word ptr [0x30]
0021D1:  2B 3E 34 00                  sub      di, word ptr [0x34]
0021D5:  8B 36 32 00                  mov      si, word ptr [0x32]
0021D9:  2B F7                        sub      si, di
0021DB:  23 F3                        and      si, bx
0021DD:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
0021E3:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
0021E9:  03 F7                        add      si, di
0021EB:  03 F7                        add      si, di
0021ED:  23 F3                        and      si, bx
0021EF:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
0021F5:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
0021FB:  66 2B C1                     sub      eax, ecx
0021FE:  66 03 EA                     add      ebp, edx
002201:  66 D1 F8                     sar      eax, 1
002204:  66 D1 FD                     sar      ebp, 1
002207:  23 FB                        and      di, bx
002209:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
00220F:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
002215:  66 2B C8                     sub      ecx, eax
002218:  66 2B D5                     sub      edx, ebp
00221B:  66 29 0E 88 22               sub      dword ptr [0x2288], ecx
002220:  66 29 0E 9C 22               sub      dword ptr [0x229c], ecx
002225:  66 01 16 84 22               add      dword ptr [0x2284], edx
00222A:  66 29 16 A0 22               sub      dword ptr [0x22a0], edx
00222F:  8B 3E 34 00                  mov      di, word ptr [0x34]
002233:  8B 2E 32 00                  mov      bp, word ptr [0x32]
002237:  8B F7                        mov      si, di
002239:  03 FD                        add      di, bp
00223B:  2B F5                        sub      si, bp
00223D:  23 FB                        and      di, bx
00223F:  23 F3                        and      si, bx
002241:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
002247:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
00224D:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
002253:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002259:  66 03 C1                     add      eax, ecx
00225C:  66 03 EA                     add      ebp, edx
00225F:  66 F7 DD                     neg      ebp
002262:  66 A3 94 22                  mov      dword ptr [0x2294], eax
002266:  66 89 2E 90 22               mov      dword ptr [0x2290], ebp
00226B:  8B 3E 30 00                  mov      di, word ptr [0x30]
00226F:  8B 2E 32 00                  mov      bp, word ptr [0x32]
002273:  8B F7                        mov      si, di
002275:  03 FD                        add      di, bp
002277:  2B F5                        sub      si, bp
002279:  23 FB                        and      di, bx
00227B:  23 F3                        and      si, bx
00227D:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
002283:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
002289:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
00228F:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002295:  66 03 C1                     add      eax, ecx
002298:  66 03 EA                     add      ebp, edx
00229B:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
00229F:  66 89 2E 8C 22               mov      dword ptr [0x228c], ebp
0022A4:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
0022A8:  66 0F BF 5D 54               movsx    ebx, word ptr [di + 0x54]
0022AD:  0B DB                        or       bx, bx
0022AF:  74 31                        je       0x22e2
0022B1:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
0022B5:  66 F7 EB                     imul     ebx
0022B8:  66 C1 F8 10                  sar      eax, 0x10
0022BC:  66 01 45 42                  add      dword ptr [di + 0x42], eax
0022C0:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
0022C4:  66 F7 EB                     imul     ebx
0022C7:  66 C1 F8 10                  sar      eax, 0x10
0022CB:  66 83 D0 00                  adc      eax, 0
0022CF:  66 01 45 46                  add      dword ptr [di + 0x46], eax
0022D3:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
0022D7:  66 F7 EB                     imul     ebx
0022DA:  66 C1 F8 10                  sar      eax, 0x10
0022DE:  66 01 45 4A                  add      dword ptr [di + 0x4a], eax
0022E2:  8B 35                        mov      si, word ptr [di]
0022E4:  66 0F BF 5D 42               movsx    ebx, word ptr [di + 0x42]
0022E9:  66 0F BF 4D 46               movsx    ecx, word ptr [di + 0x46]
0022EE:  66 0F BF 55 4A               movsx    edx, word ptr [di + 0x4a]
0022F3:  66 8B 44 2A                  mov      eax, dword ptr [si + 0x2a]
0022F7:  66 0F AF C3                  imul     eax, ebx
0022FB:  66 8B E8                     mov      ebp, eax
0022FE:  66 8B 44 2E                  mov      eax, dword ptr [si + 0x2e]
002302:  66 0F AF C1                  imul     eax, ecx
002306:  66 03 E8                     add      ebp, eax
002309:  66 8B 44 32                  mov      eax, dword ptr [si + 0x32]
00230D:  66 0F AF C2                  imul     eax, edx
002311:  66 03 C5                     add      eax, ebp
002314:  66 03 44 3E                  add      eax, dword ptr [si + 0x3e]
002318:  66 89 45 3E                  mov      dword ptr [di + 0x3e], eax
00231C:  66 8B 44 1E                  mov      eax, dword ptr [si + 0x1e]
002320:  66 0F AF C3                  imul     eax, ebx
002324:  66 8B E8                     mov      ebp, eax
002327:  66 8B 44 22                  mov      eax, dword ptr [si + 0x22]
00232B:  66 0F AF C1                  imul     eax, ecx
00232F:  66 03 E8                     add      ebp, eax
002332:  66 8B 44 26                  mov      eax, dword ptr [si + 0x26]
002336:  66 0F AF C2                  imul     eax, edx
00233A:  66 03 C5                     add      eax, ebp
00233D:  66 03 44 3A                  add      eax, dword ptr [si + 0x3a]
002341:  66 89 45 3A                  mov      dword ptr [di + 0x3a], eax
002345:  66 8B 44 12                  mov      eax, dword ptr [si + 0x12]
002349:  66 0F AF C3                  imul     eax, ebx
00234D:  66 8B E8                     mov      ebp, eax
002350:  66 8B 44 16                  mov      eax, dword ptr [si + 0x16]
002354:  66 0F AF C1                  imul     eax, ecx
002358:  66 03 E8                     add      ebp, eax
00235B:  66 8B 44 1A                  mov      eax, dword ptr [si + 0x1a]
00235F:  66 0F AF C2                  imul     eax, edx
002363:  66 03 C5                     add      eax, ebp
002366:  66 03 44 36                  add      eax, dword ptr [si + 0x36]
00236A:  66 89 45 36                  mov      dword ptr [di + 0x36], eax
00236E:  8B 35                        mov      si, word ptr [di]
002370:  8D 74 12                     lea      si, [si + 0x12]
002373:  8D 7D 12                     lea      di, [di + 0x12]
002376:  B9 03 00                     mov      cx, 3
002379:  66 8B 5C 04                  mov      ebx, dword ptr [si + 4]
00237D:  66 8B 54 08                  mov      edx, dword ptr [si + 8]
002381:  66 A1 84 22                  mov      eax, dword ptr [0x2284]
002385:  66 0F AF 04                  imul     eax, dword ptr [si]
002389:  66 8B E8                     mov      ebp, eax
00238C:  66 A1 90 22                  mov      eax, dword ptr [0x2290]
002390:  66 0F AF C3                  imul     eax, ebx
002394:  66 03 E8                     add      ebp, eax
002397:  66 A1 9C 22                  mov      eax, dword ptr [0x229c]
00239B:  66 0F AF C2                  imul     eax, edx
00239F:  66 03 E8                     add      ebp, eax
0023A2:  66 C1 FD 0F                  sar      ebp, 0xf
0023A6:  66 89 2D                     mov      dword ptr [di], ebp
0023A9:  66 A1 88 22                  mov      eax, dword ptr [0x2288]
0023AD:  66 0F AF 04                  imul     eax, dword ptr [si]
0023B1:  66 8B E8                     mov      ebp, eax
0023B4:  66 A1 94 22                  mov      eax, dword ptr [0x2294]
0023B8:  66 0F AF C3                  imul     eax, ebx
0023BC:  66 03 E8                     add      ebp, eax
0023BF:  66 A1 A0 22                  mov      eax, dword ptr [0x22a0]
0023C3:  66 0F AF C2                  imul     eax, edx
0023C7:  66 03 E8                     add      ebp, eax
0023CA:  66 C1 FD 0F                  sar      ebp, 0xf
0023CE:  66 89 6D 04                  mov      dword ptr [di + 4], ebp
0023D2:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
0023D6:  66 0F AF 04                  imul     eax, dword ptr [si]
0023DA:  66 8B E8                     mov      ebp, eax
0023DD:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
0023E1:  66 0F AF C3                  imul     eax, ebx
0023E5:  66 03 E8                     add      ebp, eax
0023E8:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
0023EC:  66 0F AF C2                  imul     eax, edx
0023F0:  66 03 E8                     add      ebp, eax
0023F3:  66 C1 FD 0F                  sar      ebp, 0xf
0023F7:  66 89 6D 08                  mov      dword ptr [di + 8], ebp
0023FB:  83 C6 0C                     add      si, 0xc
0023FE:  83 C7 0C                     add      di, 0xc
002401:  49                           dec      cx
002402:  0F 85 73 FF                  jne      0x2379
002406:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
00240A:  8B 4D 02                     mov      cx, word ptr [di + 2]
00240D:  8B 75 06                     mov      si, word ptr [di + 6]
002410:  C7 06 7E 22 0F 80            mov      word ptr [0x227e], 0x800f
002416:  C7 06 80 22 00 00            mov      word ptr [0x2280], 0
00241C:  51                           push     cx
00241D:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
002423:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
002429:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
00242F:  66 8B 45 2A                  mov      eax, dword ptr [di + 0x2a]
002433:  66 0F AF C3                  imul     eax, ebx
002437:  66 8B E8                     mov      ebp, eax
00243A:  66 8B 45 2E                  mov      eax, dword ptr [di + 0x2e]
00243E:  66 0F AF C1                  imul     eax, ecx
002442:  66 03 E8                     add      ebp, eax
002445:  66 8B 45 32                  mov      eax, dword ptr [di + 0x32]
002449:  66 0F AF C2                  imul     eax, edx
00244D:  66 03 E8                     add      ebp, eax
002450:  66 03 6D 3E                  add      ebp, dword ptr [di + 0x3e]
002454:  66 C1 FD 08                  sar      ebp, 8
002458:  66 26 89 6C 0E               mov      dword ptr es:[si + 0xe], ebp
00245D:  66 8B 45 1E                  mov      eax, dword ptr [di + 0x1e]
002461:  66 0F AF C3                  imul     eax, ebx
002465:  66 8B E8                     mov      ebp, eax
002468:  66 8B 45 22                  mov      eax, dword ptr [di + 0x22]
00246C:  66 0F AF C1                  imul     eax, ecx
002470:  66 03 E8                     add      ebp, eax
002473:  66 8B 45 26                  mov      eax, dword ptr [di + 0x26]
002477:  66 0F AF C2                  imul     eax, edx
00247B:  66 03 45 3A                  add      eax, dword ptr [di + 0x3a]
00247F:  66 03 C5                     add      eax, ebp
002482:  66 50                        push     eax
002484:  66 8B 45 12                  mov      eax, dword ptr [di + 0x12]
002488:  66 0F AF C3                  imul     eax, ebx
00248C:  66 8B E8                     mov      ebp, eax
00248F:  66 8B 45 16                  mov      eax, dword ptr [di + 0x16]
002493:  66 0F AF C1                  imul     eax, ecx
002497:  66 03 E8                     add      ebp, eax
00249A:  66 8B 45 1A                  mov      eax, dword ptr [di + 0x1a]
00249E:  66 0F AF C2                  imul     eax, edx
0024A2:  66 03 45 36                  add      eax, dword ptr [di + 0x36]
0024A6:  66 03 C5                     add      eax, ebp
0024A9:  33 C9                        xor      cx, cx
0024AB:  66 26 8B 6C 0E               mov      ebp, dword ptr es:[si + 0xe]
0024B0:  66 83 FD 01                  cmp      ebp, 1
0024B4:  0F 8C 0A 01                  jl       0x25c2
0024B8:  66 99                        cdq     
0024BA:  66 F7 FD                     idiv     ebp
0024BD:  66 8B D8                     mov      ebx, eax
0024C0:  66 58                        pop      eax
0024C2:  66 99                        cdq     
0024C4:  66 F7 FD                     idiv     ebp
0024C7:  66 F7 D8                     neg      eax
0024CA:  66 03 1E 70 22               add      ebx, dword ptr [0x2270]
0024CF:  79 0B                        jns      0x24dc
0024D1:  B1 01                        mov      cl, 1
0024D3:  66 83 FB A6                  cmp      ebx, -0x5a
0024D7:  7F 03                        jg       0x24dc
0024D9:  BB A7 FF                     mov      bx, 0xffa7
0024DC:  66 81 FB 40 01 00 00         cmp      ebx, 0x140
0024E3:  7C 0E                        jl       0x24f3
0024E5:  B1 02                        mov      cl, 2
0024E7:  66 81 FB 9A 01 00 00         cmp      ebx, 0x19a
0024EE:  7C 03                        jl       0x24f3
0024F0:  BB 99 01                     mov      bx, 0x199
0024F3:  66 03 06 74 22               add      eax, dword ptr [0x2274]
0024F8:  79 0E                        jns      0x2508
0024FA:  80 C9 04                     or       cl, 4
0024FD:  66 3D 6A FF FF FF            cmp      eax, 0xffffff6a
002503:  7F 03                        jg       0x2508
002505:  B8 6B FF                     mov      ax, 0xff6b
002508:  66 3D C8 00 00 00            cmp      eax, 0xc8
00250E:  7C 0E                        jl       0x251e
002510:  80 C9 08                     or       cl, 8
002513:  66 3D 5E 01 00 00            cmp      eax, 0x15e
002519:  7C 03                        jl       0x251e
00251B:  B8 5D 01                     mov      ax, 0x15d
00251E:  21 0E 7E 22                  and      word ptr [0x227e], cx
002522:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
002526:  26 89 5C 0A                  mov      word ptr es:[si + 0xa], bx
00252A:  26 89 44 0C                  mov      word ptr es:[si + 0xc], ax
00252E:  59                           pop      cx
00252F:  83 C6 14                     add      si, 0x14
002532:  49                           dec      cx
002533:  0F 85 E5 FE                  jne      0x241c
002537:  A1 80 22                     mov      ax, word ptr [0x2280]
00253A:  F7 06 7E 22 FF FF            test     word ptr [0x227e], 0xffff
002540:  74 11                        je       0x2553
002542:  8B 4D 02                     mov      cx, word ptr [di + 2]
002545:  8B 75 06                     mov      si, word ptr [di + 6]
002548:  26 C7 44 12 FF 00            mov      word ptr es:[si + 0x12], 0xff
00254E:  83 C6 14                     add      si, 0x14
002551:  E2 F5                        loop     0x2548
002553:  FF 0E 7C 22                  dec      word ptr [0x227c]
002557:  0F 85 E4 FB                  jne      0x213f
00255B:  5F                           pop      di
00255C:  8B 4D 26                     mov      cx, word ptr [di + 0x26]
00255F:  E3 29                        jcxz     0x258a
002561:  1E                           push     ds
002562:  64 8B 75 22                  mov      si, word ptr fs:[di + 0x22]
002566:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
00256B:  8B 5C 04                     mov      bx, word ptr [si + 4]
00256E:  66 8B 47 0A                  mov      eax, dword ptr [bx + 0xa]
002572:  66 8B 57 0E                  mov      edx, dword ptr [bx + 0xe]
002576:  8B 6F 12                     mov      bp, word ptr [bx + 0x12]
002579:  66 89 44 0A                  mov      dword ptr [si + 0xa], eax
00257D:  66 89 54 0E                  mov      dword ptr [si + 0xe], edx
002581:  89 6C 12                     mov      word ptr [si + 0x12], bp
002584:  83 C6 14                     add      si, 0x14
002587:  E2 E2                        loop     0x256b
002589:  1F                           pop      ds
00258A:  C3                           ret     
; -- non-contiguous block: next 0x0025c2 --
0025C2:  B5 80                        mov      ch, 0x80
0025C4:  66 8B D8                     mov      ebx, eax
0025C7:  66 58                        pop      eax
0025C9:  66 C1 F8 0C                  sar      eax, 0xc
0025CD:  66 C1 FB 0C                  sar      ebx, 0xc
0025D1:  E9 F3 FE                     jmp      0x24c7
