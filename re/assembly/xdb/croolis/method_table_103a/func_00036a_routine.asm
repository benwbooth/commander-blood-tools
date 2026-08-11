; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00036a
; group: method_table_103a
; provenance: alien_method_table_103a_slot_7@0x4338
; byte_count: 370
; boundary: cfg_blocks_24_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 7835f5dd49d2936d8747723768ac7008a4ed64a11b3791df33e35222bec184f8

00036A:  8B 75 16                     mov      si, word ptr [di + 0x16]
00036D:  66 C7 44 36 00 00 00 00      mov      dword ptr [si + 0x36], 0
000375:  66 C7 44 3A 00 00 00 00      mov      dword ptr [si + 0x3a], 0
00037D:  66 C7 44 3A 00 00 00 00      mov      dword ptr [si + 0x3a], 0
000385:  66 C7 44 12 00 80 00 00      mov      dword ptr [si + 0x12], 0x8000
00038D:  66 C7 44 22 00 80 00 00      mov      dword ptr [si + 0x22], 0x8000
000395:  66 C7 44 32 00 80 00 00      mov      dword ptr [si + 0x32], 0x8000
00039D:  8B DE                        mov      bx, si
00039F:  83 C6 5E                     add      si, 0x5e
0003A2:  89 1C                        mov      word ptr [si], bx
0003A4:  66 0F BF 06 2A 00            movsx    eax, word ptr [0x2a]
0003AA:  66 0F BF 1E 2C 00            movsx    ebx, word ptr [0x2c]
0003B0:  66 F7 DB                     neg      ebx
0003B3:  66 8B 4C 3E                  mov      ecx, dword ptr [si + 0x3e]
0003B7:  66 C1 F9 08                  sar      ecx, 8
0003BB:  66 BA C4 FF FF FF            mov      edx, 0xffffffc4
0003C1:  66 0F AF D1                  imul     edx, ecx
0003C5:  66 0F AF C8                  imul     ecx, eax
0003C9:  C1 E0 02                     shl      ax, 2
0003CC:  89 44 52                     mov      word ptr [si + 0x52], ax
0003CF:  89 44 50                     mov      word ptr [si + 0x50], ax
0003D2:  89 5C 4E                     mov      word ptr [si + 0x4e], bx
0003D5:  66 C1 F9 02                  sar      ecx, 2
0003D9:  66 2B 4C 36                  sub      ecx, dword ptr [si + 0x36]
0003DD:  66 2B 54 3A                  sub      edx, dword ptr [si + 0x3a]
0003E1:  66 C1 F9 10                  sar      ecx, 0x10
0003E5:  66 C1 FA 10                  sar      edx, 0x10
0003E9:  66 01 4C 42                  add      dword ptr [si + 0x42], ecx
0003ED:  66 01 54 46                  add      dword ptr [si + 0x46], edx
0003F1:  2E 8B 0E FC 02               mov      cx, word ptr cs:[0x2fc]
0003F6:  0B C9                        or       cx, cx
0003F8:  74 23                        je       0x41d
0003FA:  49                           dec      cx
0003FB:  2E 89 0E FC 02               mov      word ptr cs:[0x2fc], cx
000400:  83 E1 03                     and      cx, 3
000403:  B8 0A 00                     mov      ax, 0xa
000406:  BB 0D 00                     mov      bx, 0xd
000409:  BA 0B 00                     mov      dx, 0xb
00040C:  D3 E0                        shl      ax, cl
00040E:  D3 E3                        shl      bx, cl
000410:  D3 E2                        shl      dx, cl
000412:  A3 36 25                     mov      word ptr [0x2536], ax
000415:  89 1E 94 25                  mov      word ptr [0x2594], bx
000419:  89 16 F2 25                  mov      word ptr [0x25f2], dx
00041D:  1E                           push     ds
00041E:  2E A1 99 00                  mov      ax, word ptr cs:[0x99]
000422:  3D 80 00                     cmp      ax, 0x80
000425:  0F 87 B1 00                  ja       0x4da
000429:  BE 80 00                     mov      si, 0x80
00042C:  BA 80 00                     mov      dx, 0x80
00042F:  2E 2B 16 9B 00               sub      dx, word ptr cs:[0x9b]
000434:  2B F0                        sub      si, ax
000436:  2E A3 9B 00                  mov      word ptr cs:[0x9b], ax
00043A:  2E 8B 1E 9F 00               mov      bx, word ptr cs:[0x9f]
00043F:  02 C3                        add      al, bl
000441:  0F 88 95 00                  js       0x4da
000445:  FE CF                        dec      bh
000447:  79 04                        jns      0x44d
000449:  B7 03                        mov      bh, 3
00044B:  F6 DB                        neg      bl
00044D:  2E 89 1E 9F 00               mov      word ptr cs:[0x9f], bx
000452:  2E A3 99 00                  mov      word ptr cs:[0x99], ax
000456:  3B F2                        cmp      si, dx
000458:  0F 84 7E 00                  je       0x4da
00045C:  7C 02                        jl       0x460
00045E:  87 F2                        xchg     dx, si
000460:  A1 04 00                     mov      ax, word ptr [4]
000463:  8E D8                        mov      ds, ax
000465:  8E C0                        mov      es, ax
000467:  BB DC 04                     mov      bx, 0x4dc
00046A:  56                           push     si
00046B:  52                           push     dx
00046C:  83 EE 3F                     sub      si, 0x3f
00046F:  73 03                        jae      0x474
000471:  BE 00 00                     mov      si, 0
000474:  83 EA 3F                     sub      dx, 0x3f
000477:  73 03                        jae      0x47c
000479:  BA 00 00                     mov      dx, 0
00047C:  2B D6                        sub      dx, si
00047E:  74 23                        je       0x4a3
000480:  C1 E6 08                     shl      si, 8
000483:  83 C6 1E                     add      si, 0x1e
000486:  B9 71 00                     mov      cx, 0x71
000489:  8B 04                        mov      ax, word ptr [si]
00048B:  2E D7                        xlatb   
00048D:  86 C4                        xchg     ah, al
00048F:  2E D7                        xlatb   
000491:  86 C4                        xchg     ah, al
000493:  89 04                        mov      word ptr [si], ax
000495:  83 C6 02                     add      si, 2
000498:  E2 EF                        loop     0x489
00049A:  83 C6 1E                     add      si, 0x1e
00049D:  B9 71 00                     mov      cx, 0x71
0004A0:  4A                           dec      dx
0004A1:  75 E6                        jne      0x489
0004A3:  5A                           pop      dx
0004A4:  5E                           pop      si
0004A5:  83 FE 3F                     cmp      si, 0x3f
0004A8:  7E 03                        jle      0x4ad
0004AA:  BE 3F 00                     mov      si, 0x3f
0004AD:  83 FA 3F                     cmp      dx, 0x3f
0004B0:  7E 03                        jle      0x4b5
0004B2:  BA 3F 00                     mov      dx, 0x3f
0004B5:  2B D6                        sub      dx, si
0004B7:  74 21                        je       0x4da
0004B9:  C1 E6 08                     shl      si, 8
0004BC:  B9 0F 00                     mov      cx, 0xf
0004BF:  8B 04                        mov      ax, word ptr [si]
0004C1:  2E D7                        xlatb   
0004C3:  86 C4                        xchg     ah, al
0004C5:  2E D7                        xlatb   
0004C7:  86 C4                        xchg     ah, al
0004C9:  89 04                        mov      word ptr [si], ax
0004CB:  83 C6 02                     add      si, 2
0004CE:  E2 EF                        loop     0x4bf
0004D0:  81 C6 E2 00                  add      si, 0xe2
0004D4:  B9 0F 00                     mov      cx, 0xf
0004D7:  4A                           dec      dx
0004D8:  75 E5                        jne      0x4bf
0004DA:  1F                           pop      ds
0004DB:  C3                           ret     
