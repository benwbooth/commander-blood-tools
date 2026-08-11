; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0000a3
; group: entry
; provenance: alien_body_entry_00a3, direct_call_from_0x0
; byte_count: 384
; boundary: cfg_blocks_18_terminals_2
; terminal: jmp 0x121:1, retf:1
; direct_callees: 0x000223, 0x0002f0, 0x000336, 0x000347, 0x00059b, 0x000734, 0x001dd8, 0x002027, 0x0024cf
; indirect_calls: 2
; routine_bytes_sha256: d9ac4420d0879158c8023912cc10a07f16931b23f561ad3b8534011d54b8c47e

0000A3:  1E                           push     ds
0000A4:  2E A1 77 32                  mov      ax, word ptr cs:[0x3277]
0000A8:  8E D8                        mov      ds, ax
0000AA:  8E C0                        mov      es, ax
0000AC:  8E E0                        mov      fs, ax
0000AE:  C7 06 6E 22 00 00            mov      word ptr [0x226e], 0
0000B4:  E8 39 02                     call     0x2f0
0000B7:  BE 6A 1F                     mov      si, 0x1f6a
0000BA:  BA C8 03                     mov      dx, 0x3c8
0000BD:  32 C0                        xor      al, al
0000BF:  EE                           out      dx, al
0000C0:  FE C2                        inc      dl
0000C2:  B9 00 03                     mov      cx, 0x300
0000C5:  F3 6E                        rep outsb dx, byte ptr [si]
0000C7:  B9 80 02                     mov      cx, 0x280
0000CA:  BA 00 04                     mov      dx, 0x400
0000CD:  E8 66 02                     call     0x336
0000D0:  B9 40 01                     mov      cx, 0x140
0000D3:  BA 00 02                     mov      dx, 0x200
0000D6:  E8 6E 02                     call     0x347
0000D9:  64 C7 06 A8 22 00 00         mov      word ptr fs:[0x22a8], 0
0000E0:  64 C7 06 EC 22 5D 07         mov      word ptr fs:[0x22ec], 0x75d
0000E7:  64 C7 06 F0 22 11 FF         mov      word ptr fs:[0x22f0], 0xff11
0000EE:  64 C7 06 F4 22 C2 D9         mov      word ptr fs:[0x22f4], 0xd9c2
0000F5:  64 C7 06 F6 22 00 00         mov      word ptr fs:[0x22f6], 0
0000FC:  64 C7 06 F8 22 78 06         mov      word ptr fs:[0x22f8], 0x678
000103:  64 C7 06 FA 22 00 00         mov      word ptr fs:[0x22fa], 0
00010A:  64 C7 06 FC 22 00 00         mov      word ptr fs:[0x22fc], 0
000111:  66 64 A1 16 00               mov      eax, dword ptr fs:[0x16]
000116:  66 2D 6C 02 00 00            sub      eax, 0x26c
00011C:  66 64 A3 1A 00               mov      dword ptr fs:[0x1a], eax
000121:  2E 8E 1E 77 32               mov      ds, word ptr cs:[0x3277]
000126:  FC                           cld     
000127:  8E 06 28 00                  mov      es, word ptr [0x28]
00012B:  33 FF                        xor      di, di
00012D:  BA C4 03                     mov      dx, 0x3c4
000130:  B8 02 0F                     mov      ax, 0xf02
000133:  EF                           out      dx, ax
000134:  33 C0                        xor      ax, ax
000136:  B9 40 1F                     mov      cx, 0x1f40
000139:  F3 AB                        rep stosw word ptr es:[di], ax
00013B:  E8 E5 00                     call     0x223
00013E:  E8 97 1C                     call     0x1dd8
000141:  E8 57 04                     call     0x59b
000144:  E8 ED 05                     call     0x734
000147:  BE 08 23                     mov      si, 0x2308
00014A:  64 8B 3C                     mov      di, word ptr fs:[si]
00014D:  83 C6 02                     add      si, 2
000150:  56                           push     si
000151:  64 8B 5D 34                  mov      bx, word ptr fs:[di + 0x34]
000155:  64 89 3E 78 22               mov      word ptr fs:[0x2278], di
00015A:  64 FF 97 3A 10               call     word ptr fs:[bx + 0x103a]
00015F:  E8 C5 1E                     call     0x2027
000162:  5E                           pop      si
000163:  64 F7 04 FF FF               test     word ptr fs:[si], 0xffff
000168:  75 E0                        jne      0x14a
00016A:  E8 62 23                     call     0x24cf
00016D:  64 A1 26 00                  mov      ax, word ptr fs:[0x26]
000171:  8B D8                        mov      bx, ax
000173:  B0 0C                        mov      al, 0xc
000175:  80 C7 40                     add      bh, 0x40
000178:  BA D4 03                     mov      dx, 0x3d4
00017B:  64 89 1E 26 00               mov      word ptr fs:[0x26], bx
000180:  C0 EF 04                     shr      bh, 4
000183:  80 CF A0                     or       bh, 0xa0
000186:  EF                           out      dx, ax
000187:  64 89 1E 28 00               mov      word ptr fs:[0x28], bx
00018C:  64 F7 06 6E 22 FF FF         test     word ptr fs:[0x226e], 0xffff
000193:  75 6C                        jne      0x201
000195:  0F A0                        push     fs
000197:  1F                           pop      ds
000198:  66 83 06 16 00 08            add      dword ptr [0x16], 8
00019E:  A1 1E 00                     mov      ax, word ptr [0x1e]
0001A1:  48                           dec      ax
0001A2:  C7 06 1E 00 00 00            mov      word ptr [0x1e], 0
0001A8:  66 8B 16 16 00               mov      edx, dword ptr [0x16]
0001AD:  79 28                        jns      0x1d7
0001AF:  66 8B C2                     mov      eax, edx
0001B2:  66 2B 06 1A 00               sub      eax, dword ptr [0x1a]
0001B7:  66 3D 58 02 00 00            cmp      eax, 0x258
0001BD:  72 26                        jb       0x1e5
0001BF:  66 81 EA E8 03 00 00         sub      edx, 0x3e8
0001C6:  64 F7 06 82 22 FF FF         test     word ptr fs:[0x2282], 0xffff
0001CD:  74 10                        je       0x1df
0001CF:  66 8B 16 16 00               mov      edx, dword ptr [0x16]
0001D4:  B8 02 00                     mov      ax, 2
0001D7:  66 52                        push     edx
0001D9:  FF 1E 20 00                  lcall    [0x20]
0001DD:  66 5A                        pop      edx
0001DF:  66 64 89 16 1A 00            mov      dword ptr fs:[0x1a], edx
0001E5:  B4 01                        mov      ah, 1
0001E7:  CD 16                        int      0x16
0001E9:  0F 84 34 FF                  je       0x121
0001ED:  32 E4                        xor      ah, ah
0001EF:  CD 16                        int      0x16
0001F1:  2E A3 95 00                  mov      word ptr cs:[0x95], ax
0001F5:  3C 70                        cmp      al, 0x70
0001F7:  74 19                        je       0x212
0001F9:  3C 50                        cmp      al, 0x50
0001FB:  74 15                        je       0x212
0001FD:  3C 1B                        cmp      al, 0x1b
0001FF:  75 E4                        jne      0x1e5
000201:  0F A0                        push     fs
000203:  1F                           pop      ds
000204:  E8 E9 00                     call     0x2f0
000207:  B9 B8 0B                     mov      cx, 0xbb8
00020A:  BA C8 00                     mov      dx, 0xc8
00020D:  E8 26 01                     call     0x336
000210:  1F                           pop      ds
000211:  CB                           retf    
000212:  32 E4                        xor      ah, ah
000214:  CD 16                        int      0x16
000216:  3C 70                        cmp      al, 0x70
000218:  0F 84 05 FF                  je       0x121
00021C:  3C 50                        cmp      al, 0x50
00021E:  75 F2                        jne      0x212
000220:  E9 FE FE                     jmp      0x121
