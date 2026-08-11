; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00206c
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 1192
; boundary: cfg_blocks_28_terminals_2
; terminal: jmp 0x2407:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/direct_calls/func_00206c_routine.cpp
; routine_bytes_sha256: 684386c05fa5f8cf92643bbc57b996af068081eb76ff69ccf5278e67acb5691a

00206C:  64 8B 3E 78 22               mov      di, word ptr fs:[0x2278]
002071:  8E 06 02 00                  mov      es, word ptr [2]
002075:  8B 45 1A                     mov      ax, word ptr [di + 0x1a]
002078:  A3 7C 22                     mov      word ptr [0x227c], ax
00207B:  57                           push     di
00207C:  8B 7D 16                     mov      di, word ptr [di + 0x16]
00207F:  83 C7 5E                     add      di, 0x5e
002082:  89 3E 7A 22                  mov      word ptr [0x227a], di
002086:  8B 45 52                     mov      ax, word ptr [di + 0x52]
002089:  BB FC 0F                     mov      bx, 0xffc
00208C:  8B 75 4E                     mov      si, word ptr [di + 0x4e]
00208F:  23 C3                        and      ax, bx
002091:  8B 7D 50                     mov      di, word ptr [di + 0x50]
002094:  23 F3                        and      si, bx
002096:  23 FB                        and      di, bx
002098:  89 3E 30 00                  mov      word ptr [0x30], di
00209C:  89 36 32 00                  mov      word ptr [0x32], si
0020A0:  A3 34 00                     mov      word ptr [0x34], ax
0020A3:  03 F8                        add      di, ax
0020A5:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
0020AB:  2B F7                        sub      si, di
0020AD:  66 03 D2                     add      edx, edx
0020B0:  23 F3                        and      si, bx
0020B2:  66 F7 DA                     neg      edx
0020B5:  66 89 16 98 22               mov      dword ptr [0x2298], edx
0020BA:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
0020C0:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
0020C6:  03 F7                        add      si, di
0020C8:  03 F7                        add      si, di
0020CA:  23 F3                        and      si, bx
0020CC:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
0020D2:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
0020D8:  66 2B C1                     sub      eax, ecx
0020DB:  66 03 EA                     add      ebp, edx
0020DE:  66 D1 F8                     sar      eax, 1
0020E1:  66 D1 FD                     sar      ebp, 1
0020E4:  23 FB                        and      di, bx
0020E6:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
0020EC:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
0020F2:  66 03 C1                     add      eax, ecx
0020F5:  66 03 EA                     add      ebp, edx
0020F8:  66 A3 88 22                  mov      dword ptr [0x2288], eax
0020FC:  66 F7 D8                     neg      eax
0020FF:  66 A3 9C 22                  mov      dword ptr [0x229c], eax
002103:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
002108:  66 89 2E A0 22               mov      dword ptr [0x22a0], ebp
00210D:  8B 3E 30 00                  mov      di, word ptr [0x30]
002111:  2B 3E 34 00                  sub      di, word ptr [0x34]
002115:  8B 36 32 00                  mov      si, word ptr [0x32]
002119:  2B F7                        sub      si, di
00211B:  23 F3                        and      si, bx
00211D:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
002123:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
002129:  03 F7                        add      si, di
00212B:  03 F7                        add      si, di
00212D:  23 F3                        and      si, bx
00212F:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
002135:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
00213B:  66 2B C1                     sub      eax, ecx
00213E:  66 03 EA                     add      ebp, edx
002141:  66 D1 F8                     sar      eax, 1
002144:  66 D1 FD                     sar      ebp, 1
002147:  23 FB                        and      di, bx
002149:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
00214F:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
002155:  66 2B C8                     sub      ecx, eax
002158:  66 2B D5                     sub      edx, ebp
00215B:  66 29 0E 88 22               sub      dword ptr [0x2288], ecx
002160:  66 29 0E 9C 22               sub      dword ptr [0x229c], ecx
002165:  66 01 16 84 22               add      dword ptr [0x2284], edx
00216A:  66 29 16 A0 22               sub      dword ptr [0x22a0], edx
00216F:  8B 3E 34 00                  mov      di, word ptr [0x34]
002173:  8B 2E 32 00                  mov      bp, word ptr [0x32]
002177:  8B F7                        mov      si, di
002179:  03 FD                        add      di, bp
00217B:  2B F5                        sub      si, bp
00217D:  23 FB                        and      di, bx
00217F:  23 F3                        and      si, bx
002181:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
002187:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
00218D:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
002193:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002199:  66 03 C1                     add      eax, ecx
00219C:  66 03 EA                     add      ebp, edx
00219F:  66 F7 DD                     neg      ebp
0021A2:  66 A3 94 22                  mov      dword ptr [0x2294], eax
0021A6:  66 89 2E 90 22               mov      dword ptr [0x2290], ebp
0021AB:  8B 3E 30 00                  mov      di, word ptr [0x30]
0021AF:  8B 2E 32 00                  mov      bp, word ptr [0x32]
0021B3:  8B F7                        mov      si, di
0021B5:  03 FD                        add      di, bp
0021B7:  2B F5                        sub      si, bp
0021B9:  23 FB                        and      di, bx
0021BB:  23 F3                        and      si, bx
0021BD:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
0021C3:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
0021C9:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
0021CF:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
0021D5:  66 03 C1                     add      eax, ecx
0021D8:  66 03 EA                     add      ebp, edx
0021DB:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
0021DF:  66 89 2E 8C 22               mov      dword ptr [0x228c], ebp
0021E4:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
0021E8:  66 0F BF 5D 54               movsx    ebx, word ptr [di + 0x54]
0021ED:  0B DB                        or       bx, bx
0021EF:  74 31                        je       0x2222
0021F1:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
0021F5:  66 F7 EB                     imul     ebx
0021F8:  66 C1 F8 10                  sar      eax, 0x10
0021FC:  66 01 45 42                  add      dword ptr [di + 0x42], eax
002200:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
002204:  66 F7 EB                     imul     ebx
002207:  66 C1 F8 10                  sar      eax, 0x10
00220B:  66 83 D0 00                  adc      eax, 0
00220F:  66 01 45 46                  add      dword ptr [di + 0x46], eax
002213:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
002217:  66 F7 EB                     imul     ebx
00221A:  66 C1 F8 10                  sar      eax, 0x10
00221E:  66 01 45 4A                  add      dword ptr [di + 0x4a], eax
002222:  8B 35                        mov      si, word ptr [di]
002224:  66 0F BF 5D 42               movsx    ebx, word ptr [di + 0x42]
002229:  66 0F BF 4D 46               movsx    ecx, word ptr [di + 0x46]
00222E:  66 0F BF 55 4A               movsx    edx, word ptr [di + 0x4a]
002233:  66 8B 44 2A                  mov      eax, dword ptr [si + 0x2a]
002237:  66 0F AF C3                  imul     eax, ebx
00223B:  66 8B E8                     mov      ebp, eax
00223E:  66 8B 44 2E                  mov      eax, dword ptr [si + 0x2e]
002242:  66 0F AF C1                  imul     eax, ecx
002246:  66 03 E8                     add      ebp, eax
002249:  66 8B 44 32                  mov      eax, dword ptr [si + 0x32]
00224D:  66 0F AF C2                  imul     eax, edx
002251:  66 03 C5                     add      eax, ebp
002254:  66 03 44 3E                  add      eax, dword ptr [si + 0x3e]
002258:  66 89 45 3E                  mov      dword ptr [di + 0x3e], eax
00225C:  66 8B 44 1E                  mov      eax, dword ptr [si + 0x1e]
002260:  66 0F AF C3                  imul     eax, ebx
002264:  66 8B E8                     mov      ebp, eax
002267:  66 8B 44 22                  mov      eax, dword ptr [si + 0x22]
00226B:  66 0F AF C1                  imul     eax, ecx
00226F:  66 03 E8                     add      ebp, eax
002272:  66 8B 44 26                  mov      eax, dword ptr [si + 0x26]
002276:  66 0F AF C2                  imul     eax, edx
00227A:  66 03 C5                     add      eax, ebp
00227D:  66 03 44 3A                  add      eax, dword ptr [si + 0x3a]
002281:  66 89 45 3A                  mov      dword ptr [di + 0x3a], eax
002285:  66 8B 44 12                  mov      eax, dword ptr [si + 0x12]
002289:  66 0F AF C3                  imul     eax, ebx
00228D:  66 8B E8                     mov      ebp, eax
002290:  66 8B 44 16                  mov      eax, dword ptr [si + 0x16]
002294:  66 0F AF C1                  imul     eax, ecx
002298:  66 03 E8                     add      ebp, eax
00229B:  66 8B 44 1A                  mov      eax, dword ptr [si + 0x1a]
00229F:  66 0F AF C2                  imul     eax, edx
0022A3:  66 03 C5                     add      eax, ebp
0022A6:  66 03 44 36                  add      eax, dword ptr [si + 0x36]
0022AA:  66 89 45 36                  mov      dword ptr [di + 0x36], eax
0022AE:  8B 35                        mov      si, word ptr [di]
0022B0:  8D 74 12                     lea      si, [si + 0x12]
0022B3:  8D 7D 12                     lea      di, [di + 0x12]
0022B6:  B9 03 00                     mov      cx, 3
0022B9:  66 8B 5C 04                  mov      ebx, dword ptr [si + 4]
0022BD:  66 8B 54 08                  mov      edx, dword ptr [si + 8]
0022C1:  66 A1 84 22                  mov      eax, dword ptr [0x2284]
0022C5:  66 0F AF 04                  imul     eax, dword ptr [si]
0022C9:  66 8B E8                     mov      ebp, eax
0022CC:  66 A1 90 22                  mov      eax, dword ptr [0x2290]
0022D0:  66 0F AF C3                  imul     eax, ebx
0022D4:  66 03 E8                     add      ebp, eax
0022D7:  66 A1 9C 22                  mov      eax, dword ptr [0x229c]
0022DB:  66 0F AF C2                  imul     eax, edx
0022DF:  66 03 E8                     add      ebp, eax
0022E2:  66 C1 FD 0F                  sar      ebp, 0xf
0022E6:  66 89 2D                     mov      dword ptr [di], ebp
0022E9:  66 A1 88 22                  mov      eax, dword ptr [0x2288]
0022ED:  66 0F AF 04                  imul     eax, dword ptr [si]
0022F1:  66 8B E8                     mov      ebp, eax
0022F4:  66 A1 94 22                  mov      eax, dword ptr [0x2294]
0022F8:  66 0F AF C3                  imul     eax, ebx
0022FC:  66 03 E8                     add      ebp, eax
0022FF:  66 A1 A0 22                  mov      eax, dword ptr [0x22a0]
002303:  66 0F AF C2                  imul     eax, edx
002307:  66 03 E8                     add      ebp, eax
00230A:  66 C1 FD 0F                  sar      ebp, 0xf
00230E:  66 89 6D 04                  mov      dword ptr [di + 4], ebp
002312:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
002316:  66 0F AF 04                  imul     eax, dword ptr [si]
00231A:  66 8B E8                     mov      ebp, eax
00231D:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
002321:  66 0F AF C3                  imul     eax, ebx
002325:  66 03 E8                     add      ebp, eax
002328:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
00232C:  66 0F AF C2                  imul     eax, edx
002330:  66 03 E8                     add      ebp, eax
002333:  66 C1 FD 0F                  sar      ebp, 0xf
002337:  66 89 6D 08                  mov      dword ptr [di + 8], ebp
00233B:  83 C6 0C                     add      si, 0xc
00233E:  83 C7 0C                     add      di, 0xc
002341:  49                           dec      cx
002342:  0F 85 73 FF                  jne      0x22b9
002346:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
00234A:  8B 4D 02                     mov      cx, word ptr [di + 2]
00234D:  8B 75 06                     mov      si, word ptr [di + 6]
002350:  C7 06 7E 22 0F 80            mov      word ptr [0x227e], 0x800f
002356:  C7 06 80 22 00 00            mov      word ptr [0x2280], 0
00235C:  51                           push     cx
00235D:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
002363:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
002369:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
00236F:  66 8B 45 2A                  mov      eax, dword ptr [di + 0x2a]
002373:  66 0F AF C3                  imul     eax, ebx
002377:  66 8B E8                     mov      ebp, eax
00237A:  66 8B 45 2E                  mov      eax, dword ptr [di + 0x2e]
00237E:  66 0F AF C1                  imul     eax, ecx
002382:  66 03 E8                     add      ebp, eax
002385:  66 8B 45 32                  mov      eax, dword ptr [di + 0x32]
002389:  66 0F AF C2                  imul     eax, edx
00238D:  66 03 E8                     add      ebp, eax
002390:  66 03 6D 3E                  add      ebp, dword ptr [di + 0x3e]
002394:  66 C1 FD 08                  sar      ebp, 8
002398:  66 26 89 6C 0E               mov      dword ptr es:[si + 0xe], ebp
00239D:  66 8B 45 1E                  mov      eax, dword ptr [di + 0x1e]
0023A1:  66 0F AF C3                  imul     eax, ebx
0023A5:  66 8B E8                     mov      ebp, eax
0023A8:  66 8B 45 22                  mov      eax, dword ptr [di + 0x22]
0023AC:  66 0F AF C1                  imul     eax, ecx
0023B0:  66 03 E8                     add      ebp, eax
0023B3:  66 8B 45 26                  mov      eax, dword ptr [di + 0x26]
0023B7:  66 0F AF C2                  imul     eax, edx
0023BB:  66 03 45 3A                  add      eax, dword ptr [di + 0x3a]
0023BF:  66 03 C5                     add      eax, ebp
0023C2:  66 50                        push     eax
0023C4:  66 8B 45 12                  mov      eax, dword ptr [di + 0x12]
0023C8:  66 0F AF C3                  imul     eax, ebx
0023CC:  66 8B E8                     mov      ebp, eax
0023CF:  66 8B 45 16                  mov      eax, dword ptr [di + 0x16]
0023D3:  66 0F AF C1                  imul     eax, ecx
0023D7:  66 03 E8                     add      ebp, eax
0023DA:  66 8B 45 1A                  mov      eax, dword ptr [di + 0x1a]
0023DE:  66 0F AF C2                  imul     eax, edx
0023E2:  66 03 45 36                  add      eax, dword ptr [di + 0x36]
0023E6:  66 03 C5                     add      eax, ebp
0023E9:  33 C9                        xor      cx, cx
0023EB:  66 26 8B 6C 0E               mov      ebp, dword ptr es:[si + 0xe]
0023F0:  66 83 FD 01                  cmp      ebp, 1
0023F4:  0F 8C 0A 01                  jl       0x2502
0023F8:  66 99                        cdq     
0023FA:  66 F7 FD                     idiv     ebp
0023FD:  66 8B D8                     mov      ebx, eax
002400:  66 58                        pop      eax
002402:  66 99                        cdq     
002404:  66 F7 FD                     idiv     ebp
002407:  66 F7 D8                     neg      eax
00240A:  66 03 1E 70 22               add      ebx, dword ptr [0x2270]
00240F:  79 0B                        jns      0x241c
002411:  B1 01                        mov      cl, 1
002413:  66 83 FB A6                  cmp      ebx, -0x5a
002417:  7F 03                        jg       0x241c
002419:  BB A7 FF                     mov      bx, 0xffa7
00241C:  66 81 FB 40 01 00 00         cmp      ebx, 0x140
002423:  7C 0E                        jl       0x2433
002425:  B1 02                        mov      cl, 2
002427:  66 81 FB 9A 01 00 00         cmp      ebx, 0x19a
00242E:  7C 03                        jl       0x2433
002430:  BB 99 01                     mov      bx, 0x199
002433:  66 03 06 74 22               add      eax, dword ptr [0x2274]
002438:  79 0E                        jns      0x2448
00243A:  80 C9 04                     or       cl, 4
00243D:  66 3D 6A FF FF FF            cmp      eax, 0xffffff6a
002443:  7F 03                        jg       0x2448
002445:  B8 6B FF                     mov      ax, 0xff6b
002448:  66 3D C8 00 00 00            cmp      eax, 0xc8
00244E:  7C 0E                        jl       0x245e
002450:  80 C9 08                     or       cl, 8
002453:  66 3D 5E 01 00 00            cmp      eax, 0x15e
002459:  7C 03                        jl       0x245e
00245B:  B8 5D 01                     mov      ax, 0x15d
00245E:  21 0E 7E 22                  and      word ptr [0x227e], cx
002462:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
002466:  26 89 5C 0A                  mov      word ptr es:[si + 0xa], bx
00246A:  26 89 44 0C                  mov      word ptr es:[si + 0xc], ax
00246E:  59                           pop      cx
00246F:  83 C6 14                     add      si, 0x14
002472:  49                           dec      cx
002473:  0F 85 E5 FE                  jne      0x235c
002477:  A1 80 22                     mov      ax, word ptr [0x2280]
00247A:  F7 06 7E 22 FF FF            test     word ptr [0x227e], 0xffff
002480:  74 11                        je       0x2493
002482:  8B 4D 02                     mov      cx, word ptr [di + 2]
002485:  8B 75 06                     mov      si, word ptr [di + 6]
002488:  26 C7 44 12 FF 00            mov      word ptr es:[si + 0x12], 0xff
00248E:  83 C6 14                     add      si, 0x14
002491:  E2 F5                        loop     0x2488
002493:  FF 0E 7C 22                  dec      word ptr [0x227c]
002497:  0F 85 E4 FB                  jne      0x207f
00249B:  5F                           pop      di
00249C:  8B 4D 26                     mov      cx, word ptr [di + 0x26]
00249F:  E3 29                        jcxz     0x24ca
0024A1:  1E                           push     ds
0024A2:  64 8B 75 22                  mov      si, word ptr fs:[di + 0x22]
0024A6:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
0024AB:  8B 5C 04                     mov      bx, word ptr [si + 4]
0024AE:  66 8B 47 0A                  mov      eax, dword ptr [bx + 0xa]
0024B2:  66 8B 57 0E                  mov      edx, dword ptr [bx + 0xe]
0024B6:  8B 6F 12                     mov      bp, word ptr [bx + 0x12]
0024B9:  66 89 44 0A                  mov      dword ptr [si + 0xa], eax
0024BD:  66 89 54 0E                  mov      dword ptr [si + 0xe], edx
0024C1:  89 6C 12                     mov      word ptr [si + 0x12], bp
0024C4:  83 C6 14                     add      si, 0x14
0024C7:  E2 E2                        loop     0x24ab
0024C9:  1F                           pop      ds
0024CA:  C3                           ret     
; -- non-contiguous block: next 0x002502 --
002502:  B5 80                        mov      ch, 0x80
002504:  66 8B D8                     mov      ebx, eax
002507:  66 58                        pop      eax
002509:  66 C1 F8 0C                  sar      eax, 0xc
00250D:  66 C1 FB 0C                  sar      ebx, 0xc
002511:  E9 F3 FE                     jmp      0x2407
