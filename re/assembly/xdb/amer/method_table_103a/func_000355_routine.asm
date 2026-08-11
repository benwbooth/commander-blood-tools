; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000355
; group: method_table_103a
; provenance: alien_method_table_103a_slot_7@0x42c8
; byte_count: 326
; boundary: cfg_blocks_22_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_000355_routine.cpp
; routine_bytes_sha256: ece70386a3be89e1fee265e7a6574ab62278cba59efa250d2bbe20bd19a17249

000355:  8B 75 16                     mov      si, word ptr [di + 0x16]
000358:  66 C7 44 36 00 00 00 00      mov      dword ptr [si + 0x36], 0
000360:  66 C7 44 3A 00 00 00 00      mov      dword ptr [si + 0x3a], 0
000368:  66 C7 44 3A 00 00 00 00      mov      dword ptr [si + 0x3a], 0
000370:  66 C7 44 12 00 80 00 00      mov      dword ptr [si + 0x12], 0x8000
000378:  66 C7 44 22 00 80 00 00      mov      dword ptr [si + 0x22], 0x8000
000380:  66 C7 44 32 00 80 00 00      mov      dword ptr [si + 0x32], 0x8000
000388:  8B DE                        mov      bx, si
00038A:  83 C6 5E                     add      si, 0x5e
00038D:  89 1C                        mov      word ptr [si], bx
00038F:  66 0F BF 06 2A 00            movsx    eax, word ptr [0x2a]
000395:  66 0F BF 1E 2C 00            movsx    ebx, word ptr [0x2c]
00039B:  66 F7 DB                     neg      ebx
00039E:  66 8B 4C 3E                  mov      ecx, dword ptr [si + 0x3e]
0003A2:  66 C1 F9 08                  sar      ecx, 8
0003A6:  66 BA C4 FF FF FF            mov      edx, 0xffffffc4
0003AC:  66 0F AF D1                  imul     edx, ecx
0003B0:  66 0F AF C8                  imul     ecx, eax
0003B4:  C1 E0 02                     shl      ax, 2
0003B7:  89 44 52                     mov      word ptr [si + 0x52], ax
0003BA:  89 44 50                     mov      word ptr [si + 0x50], ax
0003BD:  89 5C 4E                     mov      word ptr [si + 0x4e], bx
0003C0:  66 C1 F9 02                  sar      ecx, 2
0003C4:  66 2B 4C 36                  sub      ecx, dword ptr [si + 0x36]
0003C8:  66 2B 54 3A                  sub      edx, dword ptr [si + 0x3a]
0003CC:  66 C1 F9 10                  sar      ecx, 0x10
0003D0:  66 C1 FA 10                  sar      edx, 0x10
0003D4:  66 01 4C 42                  add      dword ptr [si + 0x42], ecx
0003D8:  66 01 54 46                  add      dword ptr [si + 0x46], edx
0003DC:  1E                           push     ds
0003DD:  2E A1 99 00                  mov      ax, word ptr cs:[0x99]
0003E1:  3D 80 00                     cmp      ax, 0x80
0003E4:  0F 87 B1 00                  ja       0x499
0003E8:  BE 80 00                     mov      si, 0x80
0003EB:  BA 80 00                     mov      dx, 0x80
0003EE:  2E 2B 16 9B 00               sub      dx, word ptr cs:[0x9b]
0003F3:  2B F0                        sub      si, ax
0003F5:  2E A3 9B 00                  mov      word ptr cs:[0x9b], ax
0003F9:  2E 8B 1E 9F 00               mov      bx, word ptr cs:[0x9f]
0003FE:  02 C3                        add      al, bl
000400:  0F 88 95 00                  js       0x499
000404:  FE CF                        dec      bh
000406:  79 04                        jns      0x40c
000408:  B7 03                        mov      bh, 3
00040A:  F6 DB                        neg      bl
00040C:  2E 89 1E 9F 00               mov      word ptr cs:[0x9f], bx
000411:  2E A3 99 00                  mov      word ptr cs:[0x99], ax
000415:  3B F2                        cmp      si, dx
000417:  0F 84 7E 00                  je       0x499
00041B:  7C 02                        jl       0x41f
00041D:  87 F2                        xchg     dx, si
00041F:  A1 04 00                     mov      ax, word ptr [4]
000422:  8E D8                        mov      ds, ax
000424:  8E C0                        mov      es, ax
000426:  BB 9B 04                     mov      bx, 0x49b
000429:  56                           push     si
00042A:  52                           push     dx
00042B:  83 EE 3F                     sub      si, 0x3f
00042E:  73 03                        jae      0x433
000430:  BE 00 00                     mov      si, 0
000433:  83 EA 3F                     sub      dx, 0x3f
000436:  73 03                        jae      0x43b
000438:  BA 00 00                     mov      dx, 0
00043B:  2B D6                        sub      dx, si
00043D:  74 23                        je       0x462
00043F:  C1 E6 08                     shl      si, 8
000442:  83 C6 1E                     add      si, 0x1e
000445:  B9 71 00                     mov      cx, 0x71
000448:  8B 04                        mov      ax, word ptr [si]
00044A:  2E D7                        xlatb   
00044C:  86 C4                        xchg     ah, al
00044E:  2E D7                        xlatb   
000450:  86 C4                        xchg     ah, al
000452:  89 04                        mov      word ptr [si], ax
000454:  83 C6 02                     add      si, 2
000457:  E2 EF                        loop     0x448
000459:  83 C6 1E                     add      si, 0x1e
00045C:  B9 71 00                     mov      cx, 0x71
00045F:  4A                           dec      dx
000460:  75 E6                        jne      0x448
000462:  5A                           pop      dx
000463:  5E                           pop      si
000464:  83 FE 3F                     cmp      si, 0x3f
000467:  7E 03                        jle      0x46c
000469:  BE 3F 00                     mov      si, 0x3f
00046C:  83 FA 3F                     cmp      dx, 0x3f
00046F:  7E 03                        jle      0x474
000471:  BA 3F 00                     mov      dx, 0x3f
000474:  2B D6                        sub      dx, si
000476:  74 21                        je       0x499
000478:  C1 E6 08                     shl      si, 8
00047B:  B9 0F 00                     mov      cx, 0xf
00047E:  8B 04                        mov      ax, word ptr [si]
000480:  2E D7                        xlatb   
000482:  86 C4                        xchg     ah, al
000484:  2E D7                        xlatb   
000486:  86 C4                        xchg     ah, al
000488:  89 04                        mov      word ptr [si], ax
00048A:  83 C6 02                     add      si, 2
00048D:  E2 EF                        loop     0x47e
00048F:  81 C6 E2 00                  add      si, 0xe2
000493:  B9 0F 00                     mov      cx, 0xf
000496:  4A                           dec      dx
000497:  75 E5                        jne      0x47e
000499:  1F                           pop      ds
00049A:  C3                           ret     
