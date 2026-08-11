; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000000
; group: manu3_labeled
; provenance: label:manu3_api_entry, label:manu3_segments_resolved, manu3 external far-call API entry, overlay_entry_0
; label: manu3_api_entry
; label_comment: manu3.xdb far-call API: [bp]=cursor x/y dwords -> [0x1a]/[0x1c]; [bp+4]&0x1f = function selector (call 0x181 if set); [bp+6]>>4+0xA000 -> [0x18] caller seg. CURSOR->POSE: yaw [0x23e4] += (x-160)*2, pitch [0x23e2] += (y-100)*2 around the render call 0x270, then restored
; label: manu3_segments_resolved
; label_comment: RESOLVED: the old notes' 'ds 0x17A3/0x1C94/0x2094' are SEGMENTS — 0x166C code, 0x17A3 data (ds), 0x1B76/0x1C94/0x2094 derived work segs (ds:[2]/[4]/[6]). Live dumps in accuracy/manu3/ (ds + all three derived): 216 faces at ds:0xB18 ({next,v0,v1,v2} ptrs), vertex records in the work segs (source coords via es=[2] in projection). Mesh extraction now offline-analyzable
; byte_count: 289
; boundary: cfg_blocks_7_terminals_1
; terminal: retf:1
; direct_callees: 0x000181, 0x00019b, 0x000270, 0x000549, 0x0006f6
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000000_manu3_api_entry.cpp
; routine_bytes_sha256: 9d5ca45567f31b131e58d4532c14fe288d957a3136ce4e25e1363e28de3ac8a5

000000:  1E                           push     ds
000001:  2E 8B 0E 6A 13               mov      cx, word ptr cs:[0x136a]
000006:  0B C9                        or       cx, cx
000008:  0F 84 15 01                  je       0x121
00000C:  8E E1                        mov      fs, cx
00000E:  8E D9                        mov      ds, cx
000010:  8E C1                        mov      es, cx
000012:  66 8B 46 00                  mov      eax, dword ptr [bp]
000016:  66 A3 1A 00                  mov      dword ptr [0x1a], eax
00001A:  8B 46 06                     mov      ax, word ptr [bp + 6]
00001D:  C1 E8 04                     shr      ax, 4
000020:  80 C4 A0                     add      ah, 0xa0
000023:  A3 18 00                     mov      word ptr [0x18], ax
000026:  8B 5E 04                     mov      bx, word ptr [bp + 4]
000029:  83 E3 1F                     and      bx, 0x1f
00002C:  74 03                        je       0x31
00002E:  E8 50 01                     call     0x181
000031:  E8 67 01                     call     0x19b
000034:  FF 36 E2 23                  push     word ptr [0x23e2]
000038:  FF 36 E4 23                  push     word ptr [0x23e4]
00003C:  A1 1A 00                     mov      ax, word ptr [0x1a]
00003F:  8B 1E 1C 00                  mov      bx, word ptr [0x1c]
000043:  2D A0 00                     sub      ax, 0xa0
000046:  03 C0                        add      ax, ax
000048:  01 06 E4 23                  add      word ptr [0x23e4], ax
00004C:  83 EB 64                     sub      bx, 0x64
00004F:  03 DB                        add      bx, bx
000051:  01 1E E2 23                  add      word ptr [0x23e2], bx
000055:  E8 18 02                     call     0x270
000058:  8F 06 E4 23                  pop      word ptr [0x23e4]
00005C:  8F 06 E2 23                  pop      word ptr [0x23e2]
000060:  8E 06 02 00                  mov      es, word ptr [2]
000064:  BF AE 24                     mov      di, 0x24ae
000067:  66 26 0F BF 1E AC 02         movsx    ebx, word ptr es:[0x2ac]
00006E:  66 26 0F BF 0E AE 02         movsx    ecx, word ptr es:[0x2ae]
000075:  66 26 0F BF 2E B0 02         movsx    ebp, word ptr es:[0x2b0]
00007C:  66 8B 45 2A                  mov      eax, dword ptr [di + 0x2a]
000080:  66 0F AF C3                  imul     eax, ebx
000084:  66 8B F0                     mov      esi, eax
000087:  66 8B 45 2E                  mov      eax, dword ptr [di + 0x2e]
00008B:  66 0F AF C1                  imul     eax, ecx
00008F:  66 03 F0                     add      esi, eax
000092:  66 8B 45 32                  mov      eax, dword ptr [di + 0x32]
000096:  66 0F AF C5                  imul     eax, ebp
00009A:  66 03 F0                     add      esi, eax
00009D:  66 03 75 3E                  add      esi, dword ptr [di + 0x3e]
0000A1:  66 C1 FE 08                  sar      esi, 8
0000A5:  78 72                        js       0x119
0000A7:  74 70                        je       0x119
0000A9:  66 8B 45 1E                  mov      eax, dword ptr [di + 0x1e]
0000AD:  66 0F AF C3                  imul     eax, ebx
0000B1:  66 8B D0                     mov      edx, eax
0000B4:  66 8B 45 22                  mov      eax, dword ptr [di + 0x22]
0000B8:  66 0F AF C1                  imul     eax, ecx
0000BC:  66 03 D0                     add      edx, eax
0000BF:  66 8B 45 26                  mov      eax, dword ptr [di + 0x26]
0000C3:  66 0F AF C5                  imul     eax, ebp
0000C7:  66 03 45 3A                  add      eax, dword ptr [di + 0x3a]
0000CB:  66 03 C2                     add      eax, edx
0000CE:  66 99                        cdq     
0000D0:  66 F7 FE                     idiv     esi
0000D3:  66 0F BF 16 1C 00            movsx    edx, word ptr [0x1c]
0000D9:  66 03 D0                     add      edx, eax
0000DC:  66 89 16 42 22               mov      dword ptr [0x2242], edx
0000E1:  66 8B 45 12                  mov      eax, dword ptr [di + 0x12]
0000E5:  66 0F AF C3                  imul     eax, ebx
0000E9:  66 8B D0                     mov      edx, eax
0000EC:  66 8B 45 16                  mov      eax, dword ptr [di + 0x16]
0000F0:  66 0F AF C1                  imul     eax, ecx
0000F4:  66 03 D0                     add      edx, eax
0000F7:  66 8B 45 1A                  mov      eax, dword ptr [di + 0x1a]
0000FB:  66 0F AF C5                  imul     eax, ebp
0000FF:  66 03 45 36                  add      eax, dword ptr [di + 0x36]
000103:  66 03 C2                     add      eax, edx
000106:  66 99                        cdq     
000108:  66 0F BF 2E 1A 00            movsx    ebp, word ptr [0x1a]
00010E:  66 F7 FE                     idiv     esi
000111:  66 2B E8                     sub      ebp, eax
000114:  66 89 2E 3E 22               mov      dword ptr [0x223e], ebp
000119:  E8 2D 04                     call     0x549
00011C:  E8 D7 05                     call     0x6f6
00011F:  1F                           pop      ds
000120:  CB                           retf    
