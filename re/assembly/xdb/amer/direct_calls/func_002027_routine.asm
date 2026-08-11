; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x002027
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 1192
; boundary: cfg_blocks_28_terminals_2
; terminal: jmp 0x23c2:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/direct_calls/func_002027_routine.cpp
; routine_bytes_sha256: 684386c05fa5f8cf92643bbc57b996af068081eb76ff69ccf5278e67acb5691a

002027:  64 8B 3E 78 22               mov      di, word ptr fs:[0x2278]
00202C:  8E 06 02 00                  mov      es, word ptr [2]
002030:  8B 45 1A                     mov      ax, word ptr [di + 0x1a]
002033:  A3 7C 22                     mov      word ptr [0x227c], ax
002036:  57                           push     di
002037:  8B 7D 16                     mov      di, word ptr [di + 0x16]
00203A:  83 C7 5E                     add      di, 0x5e
00203D:  89 3E 7A 22                  mov      word ptr [0x227a], di
002041:  8B 45 52                     mov      ax, word ptr [di + 0x52]
002044:  BB FC 0F                     mov      bx, 0xffc
002047:  8B 75 4E                     mov      si, word ptr [di + 0x4e]
00204A:  23 C3                        and      ax, bx
00204C:  8B 7D 50                     mov      di, word ptr [di + 0x50]
00204F:  23 F3                        and      si, bx
002051:  23 FB                        and      di, bx
002053:  89 3E 30 00                  mov      word ptr [0x30], di
002057:  89 36 32 00                  mov      word ptr [0x32], si
00205B:  A3 34 00                     mov      word ptr [0x34], ax
00205E:  03 F8                        add      di, ax
002060:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002066:  2B F7                        sub      si, di
002068:  66 03 D2                     add      edx, edx
00206B:  23 F3                        and      si, bx
00206D:  66 F7 DA                     neg      edx
002070:  66 89 16 98 22               mov      dword ptr [0x2298], edx
002075:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
00207B:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
002081:  03 F7                        add      si, di
002083:  03 F7                        add      si, di
002085:  23 F3                        and      si, bx
002087:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
00208D:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002093:  66 2B C1                     sub      eax, ecx
002096:  66 03 EA                     add      ebp, edx
002099:  66 D1 F8                     sar      eax, 1
00209C:  66 D1 FD                     sar      ebp, 1
00209F:  23 FB                        and      di, bx
0020A1:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
0020A7:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
0020AD:  66 03 C1                     add      eax, ecx
0020B0:  66 03 EA                     add      ebp, edx
0020B3:  66 A3 88 22                  mov      dword ptr [0x2288], eax
0020B7:  66 F7 D8                     neg      eax
0020BA:  66 A3 9C 22                  mov      dword ptr [0x229c], eax
0020BE:  66 89 2E 84 22               mov      dword ptr [0x2284], ebp
0020C3:  66 89 2E A0 22               mov      dword ptr [0x22a0], ebp
0020C8:  8B 3E 30 00                  mov      di, word ptr [0x30]
0020CC:  2B 3E 34 00                  sub      di, word ptr [0x34]
0020D0:  8B 36 32 00                  mov      si, word ptr [0x32]
0020D4:  2B F7                        sub      si, di
0020D6:  23 F3                        and      si, bx
0020D8:  66 0F BF 84 36 00            movsx    eax, word ptr [si + 0x36]
0020DE:  66 0F BF AC 38 00            movsx    ebp, word ptr [si + 0x38]
0020E4:  03 F7                        add      si, di
0020E6:  03 F7                        add      si, di
0020E8:  23 F3                        and      si, bx
0020EA:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
0020F0:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
0020F6:  66 2B C1                     sub      eax, ecx
0020F9:  66 03 EA                     add      ebp, edx
0020FC:  66 D1 F8                     sar      eax, 1
0020FF:  66 D1 FD                     sar      ebp, 1
002102:  23 FB                        and      di, bx
002104:  66 0F BF 8D 38 00            movsx    ecx, word ptr [di + 0x38]
00210A:  66 0F BF 95 36 00            movsx    edx, word ptr [di + 0x36]
002110:  66 2B C8                     sub      ecx, eax
002113:  66 2B D5                     sub      edx, ebp
002116:  66 29 0E 88 22               sub      dword ptr [0x2288], ecx
00211B:  66 29 0E 9C 22               sub      dword ptr [0x229c], ecx
002120:  66 01 16 84 22               add      dword ptr [0x2284], edx
002125:  66 29 16 A0 22               sub      dword ptr [0x22a0], edx
00212A:  8B 3E 34 00                  mov      di, word ptr [0x34]
00212E:  8B 2E 32 00                  mov      bp, word ptr [0x32]
002132:  8B F7                        mov      si, di
002134:  03 FD                        add      di, bp
002136:  2B F5                        sub      si, bp
002138:  23 FB                        and      di, bx
00213A:  23 F3                        and      si, bx
00213C:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
002142:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
002148:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
00214E:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002154:  66 03 C1                     add      eax, ecx
002157:  66 03 EA                     add      ebp, edx
00215A:  66 F7 DD                     neg      ebp
00215D:  66 A3 94 22                  mov      dword ptr [0x2294], eax
002161:  66 89 2E 90 22               mov      dword ptr [0x2290], ebp
002166:  8B 3E 30 00                  mov      di, word ptr [0x30]
00216A:  8B 2E 32 00                  mov      bp, word ptr [0x32]
00216E:  8B F7                        mov      si, di
002170:  03 FD                        add      di, bp
002172:  2B F5                        sub      si, bp
002174:  23 FB                        and      di, bx
002176:  23 F3                        and      si, bx
002178:  66 0F BF 85 36 00            movsx    eax, word ptr [di + 0x36]
00217E:  66 0F BF AD 38 00            movsx    ebp, word ptr [di + 0x38]
002184:  66 0F BF 8C 36 00            movsx    ecx, word ptr [si + 0x36]
00218A:  66 0F BF 94 38 00            movsx    edx, word ptr [si + 0x38]
002190:  66 03 C1                     add      eax, ecx
002193:  66 03 EA                     add      ebp, edx
002196:  66 A3 A4 22                  mov      dword ptr [0x22a4], eax
00219A:  66 89 2E 8C 22               mov      dword ptr [0x228c], ebp
00219F:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
0021A3:  66 0F BF 5D 54               movsx    ebx, word ptr [di + 0x54]
0021A8:  0B DB                        or       bx, bx
0021AA:  74 31                        je       0x21dd
0021AC:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
0021B0:  66 F7 EB                     imul     ebx
0021B3:  66 C1 F8 10                  sar      eax, 0x10
0021B7:  66 01 45 42                  add      dword ptr [di + 0x42], eax
0021BB:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
0021BF:  66 F7 EB                     imul     ebx
0021C2:  66 C1 F8 10                  sar      eax, 0x10
0021C6:  66 83 D0 00                  adc      eax, 0
0021CA:  66 01 45 46                  add      dword ptr [di + 0x46], eax
0021CE:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
0021D2:  66 F7 EB                     imul     ebx
0021D5:  66 C1 F8 10                  sar      eax, 0x10
0021D9:  66 01 45 4A                  add      dword ptr [di + 0x4a], eax
0021DD:  8B 35                        mov      si, word ptr [di]
0021DF:  66 0F BF 5D 42               movsx    ebx, word ptr [di + 0x42]
0021E4:  66 0F BF 4D 46               movsx    ecx, word ptr [di + 0x46]
0021E9:  66 0F BF 55 4A               movsx    edx, word ptr [di + 0x4a]
0021EE:  66 8B 44 2A                  mov      eax, dword ptr [si + 0x2a]
0021F2:  66 0F AF C3                  imul     eax, ebx
0021F6:  66 8B E8                     mov      ebp, eax
0021F9:  66 8B 44 2E                  mov      eax, dword ptr [si + 0x2e]
0021FD:  66 0F AF C1                  imul     eax, ecx
002201:  66 03 E8                     add      ebp, eax
002204:  66 8B 44 32                  mov      eax, dword ptr [si + 0x32]
002208:  66 0F AF C2                  imul     eax, edx
00220C:  66 03 C5                     add      eax, ebp
00220F:  66 03 44 3E                  add      eax, dword ptr [si + 0x3e]
002213:  66 89 45 3E                  mov      dword ptr [di + 0x3e], eax
002217:  66 8B 44 1E                  mov      eax, dword ptr [si + 0x1e]
00221B:  66 0F AF C3                  imul     eax, ebx
00221F:  66 8B E8                     mov      ebp, eax
002222:  66 8B 44 22                  mov      eax, dword ptr [si + 0x22]
002226:  66 0F AF C1                  imul     eax, ecx
00222A:  66 03 E8                     add      ebp, eax
00222D:  66 8B 44 26                  mov      eax, dword ptr [si + 0x26]
002231:  66 0F AF C2                  imul     eax, edx
002235:  66 03 C5                     add      eax, ebp
002238:  66 03 44 3A                  add      eax, dword ptr [si + 0x3a]
00223C:  66 89 45 3A                  mov      dword ptr [di + 0x3a], eax
002240:  66 8B 44 12                  mov      eax, dword ptr [si + 0x12]
002244:  66 0F AF C3                  imul     eax, ebx
002248:  66 8B E8                     mov      ebp, eax
00224B:  66 8B 44 16                  mov      eax, dword ptr [si + 0x16]
00224F:  66 0F AF C1                  imul     eax, ecx
002253:  66 03 E8                     add      ebp, eax
002256:  66 8B 44 1A                  mov      eax, dword ptr [si + 0x1a]
00225A:  66 0F AF C2                  imul     eax, edx
00225E:  66 03 C5                     add      eax, ebp
002261:  66 03 44 36                  add      eax, dword ptr [si + 0x36]
002265:  66 89 45 36                  mov      dword ptr [di + 0x36], eax
002269:  8B 35                        mov      si, word ptr [di]
00226B:  8D 74 12                     lea      si, [si + 0x12]
00226E:  8D 7D 12                     lea      di, [di + 0x12]
002271:  B9 03 00                     mov      cx, 3
002274:  66 8B 5C 04                  mov      ebx, dword ptr [si + 4]
002278:  66 8B 54 08                  mov      edx, dword ptr [si + 8]
00227C:  66 A1 84 22                  mov      eax, dword ptr [0x2284]
002280:  66 0F AF 04                  imul     eax, dword ptr [si]
002284:  66 8B E8                     mov      ebp, eax
002287:  66 A1 90 22                  mov      eax, dword ptr [0x2290]
00228B:  66 0F AF C3                  imul     eax, ebx
00228F:  66 03 E8                     add      ebp, eax
002292:  66 A1 9C 22                  mov      eax, dword ptr [0x229c]
002296:  66 0F AF C2                  imul     eax, edx
00229A:  66 03 E8                     add      ebp, eax
00229D:  66 C1 FD 0F                  sar      ebp, 0xf
0022A1:  66 89 2D                     mov      dword ptr [di], ebp
0022A4:  66 A1 88 22                  mov      eax, dword ptr [0x2288]
0022A8:  66 0F AF 04                  imul     eax, dword ptr [si]
0022AC:  66 8B E8                     mov      ebp, eax
0022AF:  66 A1 94 22                  mov      eax, dword ptr [0x2294]
0022B3:  66 0F AF C3                  imul     eax, ebx
0022B7:  66 03 E8                     add      ebp, eax
0022BA:  66 A1 A0 22                  mov      eax, dword ptr [0x22a0]
0022BE:  66 0F AF C2                  imul     eax, edx
0022C2:  66 03 E8                     add      ebp, eax
0022C5:  66 C1 FD 0F                  sar      ebp, 0xf
0022C9:  66 89 6D 04                  mov      dword ptr [di + 4], ebp
0022CD:  66 A1 8C 22                  mov      eax, dword ptr [0x228c]
0022D1:  66 0F AF 04                  imul     eax, dword ptr [si]
0022D5:  66 8B E8                     mov      ebp, eax
0022D8:  66 A1 98 22                  mov      eax, dword ptr [0x2298]
0022DC:  66 0F AF C3                  imul     eax, ebx
0022E0:  66 03 E8                     add      ebp, eax
0022E3:  66 A1 A4 22                  mov      eax, dword ptr [0x22a4]
0022E7:  66 0F AF C2                  imul     eax, edx
0022EB:  66 03 E8                     add      ebp, eax
0022EE:  66 C1 FD 0F                  sar      ebp, 0xf
0022F2:  66 89 6D 08                  mov      dword ptr [di + 8], ebp
0022F6:  83 C6 0C                     add      si, 0xc
0022F9:  83 C7 0C                     add      di, 0xc
0022FC:  49                           dec      cx
0022FD:  0F 85 73 FF                  jne      0x2274
002301:  8B 3E 7A 22                  mov      di, word ptr [0x227a]
002305:  8B 4D 02                     mov      cx, word ptr [di + 2]
002308:  8B 75 06                     mov      si, word ptr [di + 6]
00230B:  C7 06 7E 22 0F 80            mov      word ptr [0x227e], 0x800f
002311:  C7 06 80 22 00 00            mov      word ptr [0x2280], 0
002317:  51                           push     cx
002318:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
00231E:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
002324:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
00232A:  66 8B 45 2A                  mov      eax, dword ptr [di + 0x2a]
00232E:  66 0F AF C3                  imul     eax, ebx
002332:  66 8B E8                     mov      ebp, eax
002335:  66 8B 45 2E                  mov      eax, dword ptr [di + 0x2e]
002339:  66 0F AF C1                  imul     eax, ecx
00233D:  66 03 E8                     add      ebp, eax
002340:  66 8B 45 32                  mov      eax, dword ptr [di + 0x32]
002344:  66 0F AF C2                  imul     eax, edx
002348:  66 03 E8                     add      ebp, eax
00234B:  66 03 6D 3E                  add      ebp, dword ptr [di + 0x3e]
00234F:  66 C1 FD 08                  sar      ebp, 8
002353:  66 26 89 6C 0E               mov      dword ptr es:[si + 0xe], ebp
002358:  66 8B 45 1E                  mov      eax, dword ptr [di + 0x1e]
00235C:  66 0F AF C3                  imul     eax, ebx
002360:  66 8B E8                     mov      ebp, eax
002363:  66 8B 45 22                  mov      eax, dword ptr [di + 0x22]
002367:  66 0F AF C1                  imul     eax, ecx
00236B:  66 03 E8                     add      ebp, eax
00236E:  66 8B 45 26                  mov      eax, dword ptr [di + 0x26]
002372:  66 0F AF C2                  imul     eax, edx
002376:  66 03 45 3A                  add      eax, dword ptr [di + 0x3a]
00237A:  66 03 C5                     add      eax, ebp
00237D:  66 50                        push     eax
00237F:  66 8B 45 12                  mov      eax, dword ptr [di + 0x12]
002383:  66 0F AF C3                  imul     eax, ebx
002387:  66 8B E8                     mov      ebp, eax
00238A:  66 8B 45 16                  mov      eax, dword ptr [di + 0x16]
00238E:  66 0F AF C1                  imul     eax, ecx
002392:  66 03 E8                     add      ebp, eax
002395:  66 8B 45 1A                  mov      eax, dword ptr [di + 0x1a]
002399:  66 0F AF C2                  imul     eax, edx
00239D:  66 03 45 36                  add      eax, dword ptr [di + 0x36]
0023A1:  66 03 C5                     add      eax, ebp
0023A4:  33 C9                        xor      cx, cx
0023A6:  66 26 8B 6C 0E               mov      ebp, dword ptr es:[si + 0xe]
0023AB:  66 83 FD 01                  cmp      ebp, 1
0023AF:  0F 8C 0A 01                  jl       0x24bd
0023B3:  66 99                        cdq     
0023B5:  66 F7 FD                     idiv     ebp
0023B8:  66 8B D8                     mov      ebx, eax
0023BB:  66 58                        pop      eax
0023BD:  66 99                        cdq     
0023BF:  66 F7 FD                     idiv     ebp
0023C2:  66 F7 D8                     neg      eax
0023C5:  66 03 1E 70 22               add      ebx, dword ptr [0x2270]
0023CA:  79 0B                        jns      0x23d7
0023CC:  B1 01                        mov      cl, 1
0023CE:  66 83 FB A6                  cmp      ebx, -0x5a
0023D2:  7F 03                        jg       0x23d7
0023D4:  BB A7 FF                     mov      bx, 0xffa7
0023D7:  66 81 FB 40 01 00 00         cmp      ebx, 0x140
0023DE:  7C 0E                        jl       0x23ee
0023E0:  B1 02                        mov      cl, 2
0023E2:  66 81 FB 9A 01 00 00         cmp      ebx, 0x19a
0023E9:  7C 03                        jl       0x23ee
0023EB:  BB 99 01                     mov      bx, 0x199
0023EE:  66 03 06 74 22               add      eax, dword ptr [0x2274]
0023F3:  79 0E                        jns      0x2403
0023F5:  80 C9 04                     or       cl, 4
0023F8:  66 3D 6A FF FF FF            cmp      eax, 0xffffff6a
0023FE:  7F 03                        jg       0x2403
002400:  B8 6B FF                     mov      ax, 0xff6b
002403:  66 3D C8 00 00 00            cmp      eax, 0xc8
002409:  7C 0E                        jl       0x2419
00240B:  80 C9 08                     or       cl, 8
00240E:  66 3D 5E 01 00 00            cmp      eax, 0x15e
002414:  7C 03                        jl       0x2419
002416:  B8 5D 01                     mov      ax, 0x15d
002419:  21 0E 7E 22                  and      word ptr [0x227e], cx
00241D:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
002421:  26 89 5C 0A                  mov      word ptr es:[si + 0xa], bx
002425:  26 89 44 0C                  mov      word ptr es:[si + 0xc], ax
002429:  59                           pop      cx
00242A:  83 C6 14                     add      si, 0x14
00242D:  49                           dec      cx
00242E:  0F 85 E5 FE                  jne      0x2317
002432:  A1 80 22                     mov      ax, word ptr [0x2280]
002435:  F7 06 7E 22 FF FF            test     word ptr [0x227e], 0xffff
00243B:  74 11                        je       0x244e
00243D:  8B 4D 02                     mov      cx, word ptr [di + 2]
002440:  8B 75 06                     mov      si, word ptr [di + 6]
002443:  26 C7 44 12 FF 00            mov      word ptr es:[si + 0x12], 0xff
002449:  83 C6 14                     add      si, 0x14
00244C:  E2 F5                        loop     0x2443
00244E:  FF 0E 7C 22                  dec      word ptr [0x227c]
002452:  0F 85 E4 FB                  jne      0x203a
002456:  5F                           pop      di
002457:  8B 4D 26                     mov      cx, word ptr [di + 0x26]
00245A:  E3 29                        jcxz     0x2485
00245C:  1E                           push     ds
00245D:  64 8B 75 22                  mov      si, word ptr fs:[di + 0x22]
002461:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
002466:  8B 5C 04                     mov      bx, word ptr [si + 4]
002469:  66 8B 47 0A                  mov      eax, dword ptr [bx + 0xa]
00246D:  66 8B 57 0E                  mov      edx, dword ptr [bx + 0xe]
002471:  8B 6F 12                     mov      bp, word ptr [bx + 0x12]
002474:  66 89 44 0A                  mov      dword ptr [si + 0xa], eax
002478:  66 89 54 0E                  mov      dword ptr [si + 0xe], edx
00247C:  89 6C 12                     mov      word ptr [si + 0x12], bp
00247F:  83 C6 14                     add      si, 0x14
002482:  E2 E2                        loop     0x2466
002484:  1F                           pop      ds
002485:  C3                           ret     
; -- non-contiguous block: next 0x0024bd --
0024BD:  B5 80                        mov      ch, 0x80
0024BF:  66 8B D8                     mov      ebx, eax
0024C2:  66 58                        pop      eax
0024C4:  66 C1 F8 0C                  sar      eax, 0xc
0024C8:  66 C1 FB 0C                  sar      ebx, 0xc
0024CC:  E9 F3 FE                     jmp      0x23c2
