; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000eb0
; seg_off: 008b:0000
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: mem_alloc_64k
; label_comment: memory alloc 64K: ax=8; ebp=0x10000 (65536 bytes); lcall 0x4b9:0 (allocator); gs:[0xa98]=ds (store returned segment). Allocates a 64KB arena and records its segment
; incoming: call@0x0006d0->008b:0000
; byte_count: 1172
; boundary: cfg_blocks_77_terminals_11
; terminal: jmp 0x1066:3, jmp 0x11a5:3, jmp 0x123b:1, jmp 0x1246:2, jmp 0xffb:1, retf:1
; direct_callees: 0x001344, 0x00147f, 0x00149b, 0x0014ca, 0x00155f, 0x001610, 0x0016a7, 0x00178b, 0x0017af, 0x0017d9, 0x001855, 0x001a93, 0x001ad3, 0x001b4b, 0x001ec1, 0x001f10, 0x001f78, 0x001fbc, 0x00210e
; indirect_calls: 37
; routine_bytes_sha256: 11c25185af4cf3fc10569a0bf64706d13f4c7ca5463cee449152d6b4279f5a00

000EB0:  1E                           push     ds
000EB1:  B8 08 00                     mov      ax, 8
000EB4:  66 BD 00 00 01 00            mov      ebp, 0x10000
000EBA:  9A 00 00 B9 04               lcall    0x4b9, 0
000EBF:  65 8C 1E 98 0A               mov      word ptr gs:[0xa98], ds
000EC4:  B8 0A 00                     mov      ax, 0xa
000EC7:  66 BD 00 00 01 00            mov      ebp, 0x10000
000ECD:  9A 00 00 B9 04               lcall    0x4b9, 0
000ED2:  66 65 8C 1E 23 52            mov      word ptr gs:[0x5223], ds
000ED8:  B8 0B 00                     mov      ax, 0xb
000EDB:  66 BD 10 00 01 00            mov      ebp, 0x10010
000EE1:  9A 00 00 B9 04               lcall    0x4b9, 0
000EE6:  65 8C 1E 2F 52               mov      word ptr gs:[0x522f], ds
000EEB:  8C D8                        mov      ax, ds
000EED:  40                           inc      ax
000EEE:  65 A3 2B 52                  mov      word ptr gs:[0x522b], ax
000EF2:  B8 0C 00                     mov      ax, 0xc
000EF5:  66 BD 00 00 01 00            mov      ebp, 0x10000
000EFB:  9A 00 00 B9 04               lcall    0x4b9, 0
000F00:  66 65 8C 1E 7E 0A            mov      word ptr gs:[0xa7e], ds
000F06:  8C D8                        mov      ax, ds
000F08:  05 40 06                     add      ax, 0x640
000F0B:  65 A3 82 0A                  mov      word ptr gs:[0xa82], ax
000F0F:  B8 09 00                     mov      ax, 9
000F12:  9A 00 00 B9 04               lcall    0x4b9, 0
000F17:  66 65 8C 1E BE 0A            mov      word ptr gs:[0xabe], ds
000F1D:  B8 64 00                     mov      ax, 0x64
000F20:  66 83 C5 10                  add      ebp, 0x10
000F24:  9A 00 00 B9 04               lcall    0x4b9, 0
000F29:  65 8C 1E B5 0B               mov      word ptr gs:[0xbb5], ds
000F2E:  8C D8                        mov      ax, ds
000F30:  05 00 08                     add      ax, 0x800
000F33:  65 A3 B9 0B                  mov      word ptr gs:[0xbb9], ax
000F37:  1F                           pop      ds
000F38:  E8 6C 07                     call     0x16a7
000F3B:  E8 21 06                     call     0x155f
000F3E:  E8 03 04                     call     0x1344
000F41:  BE 13 01                     mov      si, 0x113
000F44:  C4 3E 96 0A                  les      di, ptr [0xa96]
000F48:  9A DB 07 CE 01               lcall    0x1ce, 0x7db
000F4D:  C4 3E 2D 52                  les      di, ptr [0x522d]
000F51:  66 33 C0                     xor      eax, eax
000F54:  AB                           stosw    word ptr es:[di], ax
000F55:  40                           inc      ax
000F56:  AB                           stosw    word ptr es:[di], ax
000F57:  83 C0 03                     add      ax, 3
000F5A:  66 AB                        stosd    dword ptr es:[di], eax
000F5C:  B8 40 01                     mov      ax, 0x140
000F5F:  AB                           stosw    word ptr es:[di], ax
000F60:  B8 C8 00                     mov      ax, 0xc8
000F63:  AB                           stosw    word ptr es:[di], ax
000F64:  33 C0                        xor      ax, ax
000F66:  66 AB                        stosd    dword ptr es:[di], eax
000F68:  BA D3 00                     mov      dx, 0xd3
000F6B:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
000F70:  B8 00 3D                     mov      ax, 0x3d00
000F73:  CD 21                        int      0x21
000F75:  0F 82 62 03                  jb       0x12db
000F79:  A3 C4 0A                     mov      word ptr [0xac4], ax
000F7C:  8C E8                        mov      ax, gs
000F7E:  8E C0                        mov      es, ax
000F80:  BE FC 00                     mov      si, 0xfc
000F83:  BF ED 25                     mov      di, 0x25ed
000F86:  9A DB 07 CE 01               lcall    0x1ce, 0x7db
000F8B:  06                           push     es
000F8C:  A1 3B 0C                     mov      ax, word ptr [0xc3b]
000F8F:  50                           push     ax
000F90:  9A 9B 05 CE 01               lcall    0x1ce, 0x59b
000F95:  58                           pop      ax
000F96:  83 F8 01                     cmp      ax, 1
000F99:  74 05                        je       0xfa0
000F9B:  C6 06 DE 0A 01               mov      byte ptr [0xade], 1
000FA0:  1E                           push     ds
000FA1:  9A 90 01 B9 04               lcall    0x4b9, 0x190
000FA6:  8C D8                        mov      ax, ds
000FA8:  83 E8 10                     sub      ax, 0x10
000FAB:  1F                           pop      ds
000FAC:  C4 3E B7 0B                  les      di, ptr [0xbb7]
000FB0:  9A 00 00 1B 0B               lcall    0xb1b, 0
000FB5:  07                           pop      es
000FB6:  B8 2C 00                     mov      ax, 0x2c
000FB9:  9A 37 10 99 02               lcall    0x299, 0x1037
000FBE:  C6 06 13 0B 00               mov      byte ptr [0xb13], 0
000FC3:  C6 06 E0 27 01               mov      byte ptr [0x27e0], 1
000FC8:  C7 06 93 27 01 00            mov      word ptr [0x2793], 1
000FCE:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
000FD3:  9A 87 23 1E 07               lcall    0x71e, 0x2387
000FD8:  E8 E6 0E                     call     0x1ec1
000FDB:  BE B2 01                     mov      si, 0x1b2
000FDE:  89 36 C2 0A                  mov      word ptr [0xac2], si
000FE2:  BE FC 0C                     mov      si, 0xcfc
000FE5:  33 C0                        xor      ax, ax
000FE7:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
000FEC:  0E                           push     cs
000FED:  E8 E9 07                     call     0x17d9
000FF0:  B9 D0 02                     mov      cx, 0x2d0
000FF3:  BA 96 00                     mov      dx, 0x96
000FF6:  B8 04 00                     mov      ax, 4
000FF9:  CD 33                        int      0x33
000FFB:  C7 06 2D 0B 08 00            mov      word ptr [0xb2d], 8
001001:  C7 06 12 66 FF FF            mov      word ptr [0x6612], 0xffff
001007:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
00100D:  0E                           push     cs
00100E:  E8 FD 10                     call     0x210e
001011:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
001016:  75 39                        jne      0x1051
001018:  F7 06 93 27 08 00            test     word ptr [0x2793], 8
00101E:  75 31                        jne      0x1051
001020:  9A 0E 07 00 00               lcall    0, 0x70e
001025:  F6 06 40 0A 03               test     byte ptr [0xa40], 3
00102A:  75 11                        jne      0x103d
00102C:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
001031:  C6 06 3F 0A 00               mov      byte ptr [0xa3f], 0
001036:  C6 06 E7 27 00               mov      byte ptr [0x27e7], 0
00103B:  EB 29                        jmp      0x1066
00103D:  F6 06 40 0A 02               test     byte ptr [0xa40], 2
001042:  74 07                        je       0x104b
001044:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
001049:  EB 1B                        jmp      0x1066
00104B:  FE 0E 40 0A                  dec      byte ptr [0xa40]
00104F:  EB 15                        jmp      0x1066
001051:  A1 38 0A                     mov      ax, word ptr [0xa38]
001054:  8B C8                        mov      cx, ax
001056:  A3 2A 0A                     mov      word ptr [0xa2a], ax
001059:  A1 3A 0A                     mov      ax, word ptr [0xa3a]
00105C:  8B D0                        mov      dx, ax
00105E:  A3 2C 0A                     mov      word ptr [0xa2c], ax
001061:  B8 04 00                     mov      ax, 4
001064:  CD 33                        int      0x33
001066:  F6 06 13 0B 01               test     byte ptr [0xb13], 1
00106B:  0F 85 6C 02                  jne      0x12db
00106F:  E8 21 0A                     call     0x1a93
001072:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
001077:  75 82                        jne      0xffb
001079:  E8 40 0F                     call     0x1fbc
00107C:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
001081:  75 0B                        jne      0x108e
001083:  9A 04 02 DA 04               lcall    0x4da, 0x204
001088:  0B C0                        or       ax, ax
00108A:  0F 88 4D 02                  js       0x12db
00108E:  83 3E 80 67 FF               cmp      word ptr [0x6780], -1
001093:  74 65                        je       0x10fa
001095:  F6 06 93 27 0E               test     byte ptr [0x2793], 0xe
00109A:  75 5E                        jne      0x10fa
00109C:  A0 AC 67                     mov      al, byte ptr [0x67ac]
00109F:  0A 06 F3 24                  or       al, byte ptr [0x24f3]
0010A3:  0A 06 51 27                  or       al, byte ptr [0x2751]
0010A7:  0A 06 B0 67                  or       al, byte ptr [0x67b0]
0010AB:  0A 06 64 5E                  or       al, byte ptr [0x5e64]
0010AF:  0A 06 65 25                  or       al, byte ptr [0x2565]
0010B3:  0A 06 36 27                  or       al, byte ptr [0x2736]
0010B7:  0A 06 37 27                  or       al, byte ptr [0x2737]
0010BB:  0A 06 DA 27                  or       al, byte ptr [0x27da]
0010BF:  0A 06 92 27                  or       al, byte ptr [0x2792]
0010C3:  75 35                        jne      0x10fa
0010C5:  A1 80 67                     mov      ax, word ptr [0x6780]
0010C8:  9A 00 00 DA 04               lcall    0x4da, 0
0010CD:  0B C0                        or       ax, ax
0010CF:  0F 88 08 02                  js       0x12db
0010D3:  C7 06 80 67 FF FF            mov      word ptr [0x6780], 0xffff
0010D9:  C6 06 A8 67 01               mov      byte ptr [0x67a8], 1
0010DE:  9A 04 02 DA 04               lcall    0x4da, 0x204
0010E3:  9A BB 01 DA 04               lcall    0x4da, 0x1bb
0010E8:  E8 B0 03                     call     0x149b
0010EB:  9A B6 14 1E 07               lcall    0x71e, 0x14b6
0010F0:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
0010F5:  C6 06 DA 27 00               mov      byte ptr [0x27da], 0
0010FA:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0010FF:  75 05                        jne      0x1106
001101:  C6 06 B8 0D 01               mov      byte ptr [0xdb8], 1
001106:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
00110B:  74 50                        je       0x115d
00110D:  A0 B0 67                     mov      al, byte ptr [0x67b0]
001110:  0A 06 64 5E                  or       al, byte ptr [0x5e64]
001114:  75 0A                        jne      0x1120
001116:  C6 06 B7 67 00               mov      byte ptr [0x67b7], 0
00111B:  C6 06 BC 67 01               mov      byte ptr [0x67bc], 1
001120:  F6 06 BC 67 01               test     byte ptr [0x67bc], 1
001125:  74 1A                        je       0x1141
001127:  83 3E F8 67 00               cmp      word ptr [0x67f8], 0
00112C:  74 07                        je       0x1135
00112E:  C6 06 D7 27 01               mov      byte ptr [0x27d7], 1
001133:  EB 70                        jmp      0x11a5
001135:  C6 06 B0 67 00               mov      byte ptr [0x67b0], 0
00113A:  C6 06 64 5E 00               mov      byte ptr [0x5e64], 0
00113F:  EB 64                        jmp      0x11a5
001141:  A0 B0 67                     mov      al, byte ptr [0x67b0]
001144:  0A 06 64 5E                  or       al, byte ptr [0x5e64]
001148:  74 5B                        je       0x11a5
00114A:  C7 06 9A 67 64 5E            mov      word ptr [0x679a], 0x5e64
001150:  F6 06 64 5E 01               test     byte ptr [0x5e64], 1
001155:  75 06                        jne      0x115d
001157:  C7 06 9A 67 B0 67            mov      word ptr [0x679a], 0x67b0
00115D:  F6 06 BB 67 01               test     byte ptr [0x67bb], 1
001162:  74 41                        je       0x11a5
001164:  83 3E F8 67 00               cmp      word ptr [0x67f8], 0
001169:  74 0A                        je       0x1175
00116B:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
001170:  0F 95 06 D7 27               setne    byte ptr [0x27d7]
001175:  83 3E 35 0B 00               cmp      word ptr [0xb35], 0
00117A:  74 07                        je       0x1183
00117C:  F6 06 3F 0A 01               test     byte ptr [0xa3f], 1
001181:  74 22                        je       0x11a5
001183:  C6 06 BB 67 00               mov      byte ptr [0x67bb], 0
001188:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
00118D:  0F 95 06 BC 67               setne    byte ptr [0x67bc]
001192:  75 0C                        jne      0x11a0
001194:  C6 06 64 5E 00               mov      byte ptr [0x5e64], 0
001199:  C6 06 B0 67 00               mov      byte ptr [0x67b0], 0
00119E:  EB 05                        jmp      0x11a5
0011A0:  80 26 AA 67 FE               and      byte ptr [0x67aa], 0xfe
0011A5:  A0 AC 67                     mov      al, byte ptr [0x67ac]
0011A8:  0A 06 BC 67                  or       al, byte ptr [0x67bc]
0011AC:  0A 06 F3 24                  or       al, byte ptr [0x24f3]
0011B0:  75 0A                        jne      0x11bc
0011B2:  C6 06 64 5E 00               mov      byte ptr [0x5e64], 0
0011B7:  C6 06 B0 67 00               mov      byte ptr [0x67b0], 0
0011BC:  F6 06 AC 67 01               test     byte ptr [0x67ac], 1
0011C1:  0F 84 81 00                  je       0x1246
0011C5:  A0 4F 27                     mov      al, byte ptr [0x274f]
0011C8:  0A 06 2A 25                  or       al, byte ptr [0x252a]
0011CC:  74 78                        je       0x1246
0011CE:  F6 06 AA 67 02               test     byte ptr [0x67aa], 2
0011D3:  75 71                        jne      0x1246
0011D5:  BB 08 00                     mov      bx, 8
0011D8:  A0 AA 67                     mov      al, byte ptr [0x67aa]
0011DB:  0A 06 35 0B                  or       al, byte ptr [0xb35]
0011DF:  74 2E                        je       0x120f
0011E1:  F6 06 B3 1F 01               test     byte ptr [0x1fb3], 1
0011E6:  74 15                        je       0x11fd
0011E8:  C6 06 B3 1F 00               mov      byte ptr [0x1fb3], 0
0011ED:  C6 06 B2 1F 00               mov      byte ptr [0x1fb2], 0
0011F2:  A1 AB 1F                     mov      ax, word ptr [0x1fab]
0011F5:  83 C0 09                     add      ax, 9
0011F8:  A3 88 67                     mov      word ptr [0x6788], ax
0011FB:  EB 49                        jmp      0x1246
0011FD:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001202:  75 42                        jne      0x1246
001204:  C6 06 FA 0C 00               mov      byte ptr [0xcfa], 0
001209:  89 1E 88 67                  mov      word ptr [0x6788], bx
00120D:  EB 37                        jmp      0x1246
00120F:  3B 1E 88 67                  cmp      bx, word ptr [0x6788]
001213:  74 26                        je       0x123b
001215:  A1 AF 0D                     mov      ax, word ptr [0xdaf]
001218:  2B 06 60 0D                  sub      ax, word ptr [0xd60]
00121C:  74 28                        je       0x1246
00121E:  8B 2E 9A 67                  mov      bp, word ptr [0x679a]
001222:  0B ED                        or       bp, bp
001224:  74 06                        je       0x122c
001226:  C6 46 00 01                  mov      byte ptr [bp], 1
00122A:  EB 0F                        jmp      0x123b
00122C:  C6 06 B0 67 01               mov      byte ptr [0x67b0], 1
001231:  C7 06 4A 67 90 67            mov      word ptr [0x674a], 0x6790
001237:  8C 1E 4C 67                  mov      word ptr [0x674c], ds
00123B:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
001240:  75 04                        jne      0x1246
001242:  89 1E 88 67                  mov      word ptr [0x6788], bx
001246:  F6 06 EB 27 01               test     byte ptr [0x27eb], 1
00124B:  74 1C                        je       0x1269
00124D:  C6 06 EB 27 00               mov      byte ptr [0x27eb], 0
001252:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
001257:  BE 3D 0D                     mov      si, 0xd3d
00125A:  C6 06 30 0D 78               mov      byte ptr [0xd30], 0x78
00125F:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
001264:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
001269:  9A 00 00 1E 07               lcall    0x71e, 0
00126E:  E8 59 02                     call     0x14ca
001271:  9A A0 04 1B 0B               lcall    0xb1b, 0x4a0
001276:  9A 33 00 1B 0B               lcall    0xb1b, 0x33
00127B:  9A 00 00 9A 0A               lcall    0xa9a, 0
001280:  E8 D2 05                     call     0x1855
001283:  E8 C5 08                     call     0x1b4b
001286:  E8 4A 08                     call     0x1ad3
001289:  F6 06 AA 67 02               test     byte ptr [0x67aa], 2
00128E:  75 07                        jne      0x1297
001290:  F6 06 AA 67 01               test     byte ptr [0x67aa], 1
001295:  75 0A                        jne      0x12a1
001297:  C6 06 FA 0C 00               mov      byte ptr [0xcfa], 0
00129C:  C6 06 FB 0C 00               mov      byte ptr [0xcfb], 0
0012A1:  F6 06 B8 0D 01               test     byte ptr [0xdb8], 1
0012A6:  74 05                        je       0x12ad
0012A8:  9A 83 11 1E 07               lcall    0x71e, 0x1183
0012AD:  1E                           push     ds
0012AE:  C5 36 21 52                  lds      si, ptr [0x5221]
0012B2:  9A 3E 0F 99 02               lcall    0x299, 0xf3e
0012B7:  1F                           pop      ds
0012B8:  9A 08 1F DA 04               lcall    0x4da, 0x1f08
0012BD:  9A 15 1C 1E 07               lcall    0x71e, 0x1c15
0012C2:  E8 4B 03                     call     0x1610
0012C5:  0E                           push     cs
0012C6:  E8 AF 0C                     call     0x1f78
0012C9:  83 3E 2D 0B 00               cmp      word ptr [0xb2d], 0
0012CE:  75 F9                        jne      0x12c9
0012D0:  FA                           cli     
0012D1:  E8 DB 04                     call     0x17af
0012D4:  E8 B4 04                     call     0x178b
0012D7:  FB                           sti     
0012D8:  E9 20 FD                     jmp      0xffb
0012DB:  9A 43 02 71 09               lcall    0x971, 0x243
0012E0:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
0012E5:  E8 28 0C                     call     0x1f10
0012E8:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
0012ED:  9A E3 04 CE 01               lcall    0x1ce, 0x4e3
0012F2:  8C E8                        mov      ax, gs
0012F4:  8E C0                        mov      es, ax
0012F6:  8B 1E 47 0C                  mov      bx, word ptr [0xc47]
0012FA:  0B DB                        or       bx, bx
0012FC:  74 0B                        je       0x1309
0012FE:  B4 3E                        mov      ah, 0x3e
001300:  CD 21                        int      0x21
001302:  BA A6 00                     mov      dx, 0xa6
001305:  B4 41                        mov      ah, 0x41
001307:  CD 21                        int      0x21
001309:  8B 1E 49 0C                  mov      bx, word ptr [0xc49]
00130D:  0B DB                        or       bx, bx
00130F:  74 0B                        je       0x131c
001311:  B4 3E                        mov      ah, 0x3e
001313:  CD 21                        int      0x21
001315:  BA AE 00                     mov      dx, 0xae
001318:  B4 41                        mov      ah, 0x41
00131A:  CD 21                        int      0x21
00131C:  8B 1E 88 0A                  mov      bx, word ptr [0xa88]
001320:  0B DB                        or       bx, bx
001322:  74 0B                        je       0x132f
001324:  B4 3E                        mov      ah, 0x3e
001326:  CD 21                        int      0x21
001328:  BA CB 00                     mov      dx, 0xcb
00132B:  B4 41                        mov      ah, 0x41
00132D:  CD 21                        int      0x21
00132F:  E8 4D 01                     call     0x147f
001332:  9A 09 05 CE 01               lcall    0x1ce, 0x509
001337:  8B 1E 86 0A                  mov      bx, word ptr [0xa86]
00133B:  0B DB                        or       bx, bx
00133D:  74 04                        je       0x1343
00133F:  B4 3E                        mov      ah, 0x3e
001341:  CD 21                        int      0x21
001343:  CB                           retf    
