; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000477
; group: manu3_labeled
; provenance: label:manu3_transform, manu3 transform routine
; label: manu3_transform
; label_comment: VERIFIED: Q15 3x3 transform, matrix 0x2250, dword vertex triples [si] -> [di], translation [si+0x36]; then walks a vertex CHAIN (mov si,[di]; lea si/di,+0x12) — 3 links per record (the skeletal chain)
; byte_count: 210
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 02a26b127e9c8e43f298876f26be497cee726feb95a0d92cccb694c04c067c9a

000477:  66 89 45 3A                  mov      dword ptr [di + 0x3a], eax
00047B:  66 8B 44 12                  mov      eax, dword ptr [si + 0x12]
00047F:  66 0F AF C3                  imul     eax, ebx
000483:  66 8B E8                     mov      ebp, eax
000486:  66 8B 44 16                  mov      eax, dword ptr [si + 0x16]
00048A:  66 0F AF C1                  imul     eax, ecx
00048E:  66 03 E8                     add      ebp, eax
000491:  66 8B 44 1A                  mov      eax, dword ptr [si + 0x1a]
000495:  66 0F AF C2                  imul     eax, edx
000499:  66 03 C5                     add      eax, ebp
00049C:  66 03 44 36                  add      eax, dword ptr [si + 0x36]
0004A0:  66 89 45 36                  mov      dword ptr [di + 0x36], eax
0004A4:  8B 35                        mov      si, word ptr [di]
0004A6:  8D 74 12                     lea      si, [si + 0x12]
0004A9:  8D 7D 12                     lea      di, [di + 0x12]
0004AC:  B9 03 00                     mov      cx, 3
0004AF:  66 8B 5C 04                  mov      ebx, dword ptr [si + 4]
0004B3:  66 8B 54 08                  mov      edx, dword ptr [si + 8]
0004B7:  66 A1 50 22                  mov      eax, dword ptr [0x2250]
0004BB:  66 0F AF 04                  imul     eax, dword ptr [si]
0004BF:  66 8B E8                     mov      ebp, eax
0004C2:  66 A1 5C 22                  mov      eax, dword ptr [0x225c]
0004C6:  66 0F AF C3                  imul     eax, ebx
0004CA:  66 03 E8                     add      ebp, eax
0004CD:  66 A1 68 22                  mov      eax, dword ptr [0x2268]
0004D1:  66 0F AF C2                  imul     eax, edx
0004D5:  66 03 E8                     add      ebp, eax
0004D8:  66 C1 FD 0F                  sar      ebp, 0xf
0004DC:  66 89 2D                     mov      dword ptr [di], ebp
0004DF:  66 A1 54 22                  mov      eax, dword ptr [0x2254]
0004E3:  66 0F AF 04                  imul     eax, dword ptr [si]
0004E7:  66 8B E8                     mov      ebp, eax
0004EA:  66 A1 60 22                  mov      eax, dword ptr [0x2260]
0004EE:  66 0F AF C3                  imul     eax, ebx
0004F2:  66 03 E8                     add      ebp, eax
0004F5:  66 A1 6C 22                  mov      eax, dword ptr [0x226c]
0004F9:  66 0F AF C2                  imul     eax, edx
0004FD:  66 03 E8                     add      ebp, eax
000500:  66 C1 FD 0F                  sar      ebp, 0xf
000504:  66 89 6D 04                  mov      dword ptr [di + 4], ebp
000508:  66 A1 58 22                  mov      eax, dword ptr [0x2258]
00050C:  66 0F AF 04                  imul     eax, dword ptr [si]
000510:  66 8B E8                     mov      ebp, eax
000513:  66 A1 64 22                  mov      eax, dword ptr [0x2264]
000517:  66 0F AF C3                  imul     eax, ebx
00051B:  66 03 E8                     add      ebp, eax
00051E:  66 A1 70 22                  mov      eax, dword ptr [0x2270]
000522:  66 0F AF C2                  imul     eax, edx
000526:  66 03 E8                     add      ebp, eax
000529:  66 C1 FD 0F                  sar      ebp, 0xf
00052D:  66 89 6D 08                  mov      dword ptr [di + 8], ebp
000531:  83 C6 0C                     add      si, 0xc
000534:  83 C7 0C                     add      di, 0xc
000537:  49                           dec      cx
000538:  0F 85 73 FF                  jne      0x4af
00053C:  FF 0E 4A 22                  dec      word ptr [0x224a]
000540:  8B 3E 48 22                  mov      di, word ptr [0x2248]
000544:  0F 85 31 FD                  jne      0x279
000548:  C3                           ret     
