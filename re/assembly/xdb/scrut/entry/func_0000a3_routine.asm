; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0000a3
; group: entry
; provenance: alien_body_entry_00a3, direct_call_from_0x0
; byte_count: 391
; boundary: cfg_blocks_18_terminals_2
; terminal: jmp 0x121:1, retf:1
; direct_callees: 0x00022a, 0x000305, 0x00034b, 0x00035c, 0x0005dc, 0x000775, 0x001edd, 0x00212c, 0x0025d4
; indirect_calls: 2
; routine_bytes_sha256: 2578e41247079f68c34bd74d9552e5eaffe72eb964092451be422b7d21960c07

0000A3:  1E                           push     ds
0000A4:  2E A1 A7 33                  mov      ax, word ptr cs:[0x33a7]
0000A8:  8E D8                        mov      ds, ax
0000AA:  8E C0                        mov      es, ax
0000AC:  8E E0                        mov      fs, ax
0000AE:  C7 06 6E 22 00 00            mov      word ptr [0x226e], 0
0000B4:  E8 4E 02                     call     0x305
0000B7:  BE 6A 1F                     mov      si, 0x1f6a
0000BA:  BA C8 03                     mov      dx, 0x3c8
0000BD:  32 C0                        xor      al, al
0000BF:  EE                           out      dx, al
0000C0:  FE C2                        inc      dl
0000C2:  B9 00 03                     mov      cx, 0x300
0000C5:  F3 6E                        rep outsb dx, byte ptr [si]
0000C7:  B9 80 02                     mov      cx, 0x280
0000CA:  BA 00 04                     mov      dx, 0x400
0000CD:  E8 7B 02                     call     0x34b
0000D0:  B9 40 01                     mov      cx, 0x140
0000D3:  BA 00 02                     mov      dx, 0x200
0000D6:  E8 83 02                     call     0x35c
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
000121:  2E 8E 1E A7 33               mov      ds, word ptr cs:[0x33a7]
000126:  FC                           cld     
000127:  8E 06 28 00                  mov      es, word ptr [0x28]
00012B:  33 FF                        xor      di, di
00012D:  BA C4 03                     mov      dx, 0x3c4
000130:  B8 02 0F                     mov      ax, 0xf02
000133:  EF                           out      dx, ax
000134:  33 C0                        xor      ax, ax
000136:  B9 40 1F                     mov      cx, 0x1f40
000139:  F3 AB                        rep stosw word ptr es:[di], ax
00013B:  E8 EC 00                     call     0x22a
00013E:  E8 9C 1D                     call     0x1edd
000141:  E8 98 04                     call     0x5dc
000144:  E8 2E 06                     call     0x775
000147:  BE 08 23                     mov      si, 0x2308
00014A:  64 8B 3C                     mov      di, word ptr fs:[si]
00014D:  83 C6 02                     add      si, 2
000150:  56                           push     si
000151:  64 8B 5D 34                  mov      bx, word ptr fs:[di + 0x34]
000155:  64 89 3E 78 22               mov      word ptr fs:[0x2278], di
00015A:  64 FF 97 3A 10               call     word ptr fs:[bx + 0x103a]
00015F:  E8 CA 1F                     call     0x212c
000162:  5E                           pop      si
000163:  64 F7 04 FF FF               test     word ptr fs:[si], 0xffff
000168:  75 E0                        jne      0x14a
00016A:  64 C7 06 82 22 00 00         mov      word ptr fs:[0x2282], 0
000171:  E8 60 24                     call     0x25d4
000174:  64 A1 26 00                  mov      ax, word ptr fs:[0x26]
000178:  8B D8                        mov      bx, ax
00017A:  B0 0C                        mov      al, 0xc
00017C:  80 C7 40                     add      bh, 0x40
00017F:  BA D4 03                     mov      dx, 0x3d4
000182:  64 89 1E 26 00               mov      word ptr fs:[0x26], bx
000187:  C0 EF 04                     shr      bh, 4
00018A:  80 CF A0                     or       bh, 0xa0
00018D:  EF                           out      dx, ax
00018E:  64 89 1E 28 00               mov      word ptr fs:[0x28], bx
000193:  64 F7 06 6E 22 FF FF         test     word ptr fs:[0x226e], 0xffff
00019A:  75 6C                        jne      0x208
00019C:  0F A0                        push     fs
00019E:  1F                           pop      ds
00019F:  66 83 06 16 00 08            add      dword ptr [0x16], 8
0001A5:  A1 1E 00                     mov      ax, word ptr [0x1e]
0001A8:  48                           dec      ax
0001A9:  C7 06 1E 00 00 00            mov      word ptr [0x1e], 0
0001AF:  66 8B 16 16 00               mov      edx, dword ptr [0x16]
0001B4:  79 28                        jns      0x1de
0001B6:  66 8B C2                     mov      eax, edx
0001B9:  66 2B 06 1A 00               sub      eax, dword ptr [0x1a]
0001BE:  66 3D 58 02 00 00            cmp      eax, 0x258
0001C4:  72 26                        jb       0x1ec
0001C6:  66 81 EA E8 03 00 00         sub      edx, 0x3e8
0001CD:  64 F7 06 82 22 FF FF         test     word ptr fs:[0x2282], 0xffff
0001D4:  74 10                        je       0x1e6
0001D6:  66 8B 16 16 00               mov      edx, dword ptr [0x16]
0001DB:  B8 02 00                     mov      ax, 2
0001DE:  66 52                        push     edx
0001E0:  FF 1E 20 00                  lcall    [0x20]
0001E4:  66 5A                        pop      edx
0001E6:  66 64 89 16 1A 00            mov      dword ptr fs:[0x1a], edx
0001EC:  B4 01                        mov      ah, 1
0001EE:  CD 16                        int      0x16
0001F0:  0F 84 2D FF                  je       0x121
0001F4:  32 E4                        xor      ah, ah
0001F6:  CD 16                        int      0x16
0001F8:  2E A3 95 00                  mov      word ptr cs:[0x95], ax
0001FC:  3C 70                        cmp      al, 0x70
0001FE:  74 19                        je       0x219
000200:  3C 50                        cmp      al, 0x50
000202:  74 15                        je       0x219
000204:  3C 1B                        cmp      al, 0x1b
000206:  75 E4                        jne      0x1ec
000208:  0F A0                        push     fs
00020A:  1F                           pop      ds
00020B:  E8 F7 00                     call     0x305
00020E:  B9 B8 0B                     mov      cx, 0xbb8
000211:  BA C8 00                     mov      dx, 0xc8
000214:  E8 34 01                     call     0x34b
000217:  1F                           pop      ds
000218:  CB                           retf    
000219:  32 E4                        xor      ah, ah
00021B:  CD 16                        int      0x16
00021D:  3C 70                        cmp      al, 0x70
00021F:  0F 84 FE FE                  je       0x121
000223:  3C 50                        cmp      al, 0x50
000225:  75 F2                        jne      0x219
000227:  E9 F7 FE                     jmp      0x121
