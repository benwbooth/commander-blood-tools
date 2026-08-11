; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000549
; group: manu3_labeled
; provenance: direct_call_from_0x0, direct_call_from_0x150, label:manu3_entity_project, manu3 entity projector
; label: manu3_entity_project
; label_comment: second API entry: projects PARENT-segment objects (es=[2]; i16 vectors at es:[si+4/6/8], depth out es:[si+0xE], flag es:[si+0x12]=0x8000) through the state record's 3x4 matrix (+0x12..+0x35 rows, +0x36/3A/3E translation, sar 8) — the same structure as the main EXE's 0x9B98 nav projection. State record: +2=count->0x224A, +6=object ptr
; byte_count: 368
; boundary: cfg_blocks_22_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000549_manu3_entity_project.cpp
; routine_bytes_sha256: b8b58b8148911c130bebdc2455fea987390e6c8f291d8521527f58596dbb41f7

000549:  8E 06 02 00                  mov      es, word ptr [2]
00054D:  BF 36 23                     mov      di, 0x2336
000550:  8B 0E F2 22                  mov      cx, word ptr [0x22f2]
000554:  51                           push     cx
000555:  83 C7 5E                     add      di, 0x5e
000558:  8B 4D 02                     mov      cx, word ptr [di + 2]
00055B:  8B 75 06                     mov      si, word ptr [di + 6]
00055E:  89 0E 4A 22                  mov      word ptr [0x224a], cx
000562:  C7 06 4E 22 00 00            mov      word ptr [0x224e], 0
000568:  26 C7 44 12 00 80            mov      word ptr es:[si + 0x12], 0x8000
00056E:  66 26 0F BF 5C 04            movsx    ebx, word ptr es:[si + 4]
000574:  66 26 0F BF 4C 06            movsx    ecx, word ptr es:[si + 6]
00057A:  66 26 0F BF 54 08            movsx    edx, word ptr es:[si + 8]
000580:  66 8B 45 2A                  mov      eax, dword ptr [di + 0x2a]
000584:  66 0F AF C3                  imul     eax, ebx
000588:  66 8B E8                     mov      ebp, eax
00058B:  66 8B 45 2E                  mov      eax, dword ptr [di + 0x2e]
00058F:  66 0F AF C1                  imul     eax, ecx
000593:  66 03 E8                     add      ebp, eax
000596:  66 8B 45 32                  mov      eax, dword ptr [di + 0x32]
00059A:  66 0F AF C2                  imul     eax, edx
00059E:  66 03 E8                     add      ebp, eax
0005A1:  66 03 6D 3E                  add      ebp, dword ptr [di + 0x3e]
0005A5:  66 C1 FD 08                  sar      ebp, 8
0005A9:  66 26 89 6C 0E               mov      dword ptr es:[si + 0xe], ebp
0005AE:  0F 88 C7 00                  js       0x679
0005B2:  0F 84 C3 00                  je       0x679
0005B6:  66 8B 45 1E                  mov      eax, dword ptr [di + 0x1e]
0005BA:  66 0F AF C3                  imul     eax, ebx
0005BE:  66 8B E8                     mov      ebp, eax
0005C1:  66 8B 45 22                  mov      eax, dword ptr [di + 0x22]
0005C5:  66 0F AF C1                  imul     eax, ecx
0005C9:  66 03 E8                     add      ebp, eax
0005CC:  66 8B 45 26                  mov      eax, dword ptr [di + 0x26]
0005D0:  66 0F AF C2                  imul     eax, edx
0005D4:  66 03 45 3A                  add      eax, dword ptr [di + 0x3a]
0005D8:  66 03 C5                     add      eax, ebp
0005DB:  66 50                        push     eax
0005DD:  66 8B 45 12                  mov      eax, dword ptr [di + 0x12]
0005E1:  66 0F AF C3                  imul     eax, ebx
0005E5:  66 8B E8                     mov      ebp, eax
0005E8:  66 8B 45 16                  mov      eax, dword ptr [di + 0x16]
0005EC:  66 0F AF C1                  imul     eax, ecx
0005F0:  66 03 E8                     add      ebp, eax
0005F3:  66 8B 45 1A                  mov      eax, dword ptr [di + 0x1a]
0005F7:  66 0F AF C2                  imul     eax, edx
0005FB:  66 03 45 36                  add      eax, dword ptr [di + 0x36]
0005FF:  66 03 C5                     add      eax, ebp
000602:  33 C9                        xor      cx, cx
000604:  66 26 8B 6C 0E               mov      ebp, dword ptr es:[si + 0xe]
000609:  66 99                        cdq     
00060B:  66 F7 FD                     idiv     ebp
00060E:  66 8B D8                     mov      ebx, eax
000611:  66 58                        pop      eax
000613:  66 99                        cdq     
000615:  66 F7 FD                     idiv     ebp
000618:  66 F7 D8                     neg      eax
00061B:  66 03 1E 3E 22               add      ebx, dword ptr [0x223e]
000620:  79 0B                        jns      0x62d
000622:  B1 01                        mov      cl, 1
000624:  66 83 FB D8                  cmp      ebx, -0x28
000628:  7F 03                        jg       0x62d
00062A:  BB D9 FF                     mov      bx, 0xffd9
00062D:  66 81 FB 40 01 00 00         cmp      ebx, 0x140
000634:  7C 0E                        jl       0x644
000636:  B1 02                        mov      cl, 2
000638:  66 81 FB 68 01 00 00         cmp      ebx, 0x168
00063F:  7C 03                        jl       0x644
000641:  BB 67 01                     mov      bx, 0x167
000644:  66 03 06 42 22               add      eax, dword ptr [0x2242]
000649:  79 0C                        jns      0x657
00064B:  80 C9 04                     or       cl, 4
00064E:  66 83 F8 9C                  cmp      eax, -0x64
000652:  7F 03                        jg       0x657
000654:  B8 9D FF                     mov      ax, 0xff9d
000657:  66 3D C8 00 00 00            cmp      eax, 0xc8
00065D:  7C 0E                        jl       0x66d
00065F:  80 C9 08                     or       cl, 8
000662:  66 3D 2C 01 00 00            cmp      eax, 0x12c
000668:  7C 03                        jl       0x66d
00066A:  B8 2B 01                     mov      ax, 0x12b
00066D:  26 89 4C 12                  mov      word ptr es:[si + 0x12], cx
000671:  26 89 5C 0A                  mov      word ptr es:[si + 0xa], bx
000675:  26 89 44 0C                  mov      word ptr es:[si + 0xc], ax
000679:  83 C6 14                     add      si, 0x14
00067C:  FF 0E 4A 22                  dec      word ptr [0x224a]
000680:  0F 85 E4 FE                  jne      0x568
000684:  59                           pop      cx
000685:  49                           dec      cx
000686:  0F 85 CA FE                  jne      0x554
00068A:  8B 0E FE 22                  mov      cx, word ptr [0x22fe]
00068E:  E3 28                        jcxz     0x6b8
000690:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
000695:  64 8B 3E FA 22               mov      di, word ptr fs:[0x22fa]
00069A:  8B 75 04                     mov      si, word ptr [di + 4]
00069D:  66 8B 44 0A                  mov      eax, dword ptr [si + 0xa]
0006A1:  66 8B 5C 0E                  mov      ebx, dword ptr [si + 0xe]
0006A5:  8B 54 12                     mov      dx, word ptr [si + 0x12]
0006A8:  66 89 45 0A                  mov      dword ptr [di + 0xa], eax
0006AC:  66 89 5D 0E                  mov      dword ptr [di + 0xe], ebx
0006B0:  89 55 12                     mov      word ptr [di + 0x12], dx
0006B3:  83 C7 14                     add      di, 0x14
0006B6:  E2 E2                        loop     0x69a
0006B8:  C3                           ret     
