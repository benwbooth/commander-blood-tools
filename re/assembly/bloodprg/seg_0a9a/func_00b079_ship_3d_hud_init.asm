; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b079
; seg_off: 0a9a:00d9
; group: seg_0a9a
; provenance: recursive_graph
; label: ship_3d_hud_init
; label_comment: sets DS:0x2793 bit 3 and initializes ship HUD/procedural-3D state
; byte_count: 578
; boundary: cfg_blocks_24_terminals_2
; terminal: jmp 0xb2b3:1, ret:1
; direct_callees: 0x00b2bb, 0x00b6dd
; indirect_calls: 15
; routine_bytes_sha256: 7986e7ff48c5c6b130f918cd1ff3ad5039ea676b8eec9ea7a0546a877175497e

00B079:  50                           push     ax
00B07A:  55                           push     bp
00B07B:  51                           push     cx
00B07C:  06                           push     es
00B07D:  57                           push     di
00B07E:  56                           push     si
00B07F:  53                           push     bx
00B080:  F6 06 29 25 01               test     byte ptr [0x2529], 1
00B085:  0F 85 FC 00                  jne      0xb185
00B089:  F6 06 35 25 01               test     byte ptr [0x2535], 1
00B08E:  74 16                        je       0xb0a6
00B090:  C6 06 35 25 00               mov      byte ptr [0x2535], 0
00B095:  C6 06 E2 27 00               mov      byte ptr [0x27e2], 0
00B09A:  BE 98 5D                     mov      si, 0x5d98
00B09D:  BF 91 54                     mov      di, 0x5491
00B0A0:  B9 30 00                     mov      cx, 0x30
00B0A3:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00B0A6:  9A 67 09 8B 00               lcall    0x8b, 0x967
00B0AB:  C7 06 9B 27 00 00            mov      word ptr [0x279b], 0
00B0B1:  C7 06 95 27 B3 00            mov      word ptr [0x2795], 0xb3
00B0B7:  83 0E 93 27 08               or       word ptr [0x2793], 8
00B0BC:  C7 06 32 0A 01 00            mov      word ptr [0xa32], 1
00B0C2:  C6 06 29 25 01               mov      byte ptr [0x2529], 1
00B0C7:  9A 9D 1D DA 04               lcall    0x4da, 0x1d9d
00B0CC:  C6 06 2B 25 01               mov      byte ptr [0x252b], 1
00B0D1:  C7 06 C6 0A 50 00            mov      word ptr [0xac6], 0x50
00B0D7:  C6 06 DC 0A 01               mov      byte ptr [0xadc], 1
00B0DC:  C6 06 DD 0A 01               mov      byte ptr [0xadd], 1
00B0E1:  C6 06 DA 0A 0A               mov      byte ptr [0xada], 0xa
00B0E6:  C4 3E 24 67                  les      di, ptr [0x6724]
00B0EA:  8B 3E 52 67                  mov      di, word ptr [0x6752]
00B0EE:  9A B9 1E DA 04               lcall    0x4da, 0x1eb9
00B0F3:  26 8B 45 16                  mov      ax, word ptr es:[di + 0x16]
00B0F7:  8B 3E 0B 25                  mov      di, word ptr [0x250b]
00B0FB:  67 26 F7 00 40 01            test     word ptr es:[eax], 0x140
00B101:  75 0A                        jne      0xb10d
00B103:  8B F8                        mov      di, ax
00B105:  9A B9 1E DA 04               lcall    0x4da, 0x1eb9
00B10A:  83 C7 04                     add      di, 4
00B10D:  89 3E 1B 25                  mov      word ptr [0x251b], di
00B111:  83 2E 1B 25 04               sub      word ptr [0x251b], 4
00B116:  9A 69 20 DA 04               lcall    0x4da, 0x2069
00B11B:  C6 06 2D 25 01               mov      byte ptr [0x252d], 1
00B120:  C7 06 88 67 03 00            mov      word ptr [0x6788], 3
00B126:  A1 A5 1F                     mov      ax, word ptr [0x1fa5]
00B129:  C6 06 2E 25 01               mov      byte ptr [0x252e], 1
00B12E:  A3 A7 1F                     mov      word ptr [0x1fa7], ax
00B131:  C6 06 B2 1F 00               mov      byte ptr [0x1fb2], 0
00B136:  9A 00 00 71 09               lcall    0x971, 0
00B13B:  1E                           push     ds
00B13C:  C5 36 21 52                  lds      si, ptr [0x5221]
00B140:  9A CB 0E 99 02               lcall    0x299, 0xecb
00B145:  1F                           pop      ds
00B146:  0E                           push     cs
00B147:  E8 93 05                     call     0xb6dd
00B14A:  8C E8                        mov      ax, gs
00B14C:  8E C0                        mov      es, ax
00B14E:  B9 C0 00                     mov      cx, 0xc0
00B151:  BF 51 58                     mov      di, 0x5851
00B154:  BE 51 52                     mov      si, 0x5251
00B157:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00B15A:  66 33 C0                     xor      eax, eax
00B15D:  B9 90 00                     mov      cx, 0x90
00B160:  BF 51 55                     mov      di, 0x5551
00B163:  F3 66 AB                     rep stosd dword ptr es:[di], eax
00B166:  BE 91 54                     mov      si, 0x5491
00B169:  B9 10 00                     mov      cx, 0x10
00B16C:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00B16F:  C7 06 4F 52 00 00            mov      word ptr [0x524f], 0
00B175:  C7 06 4D 52 0A 00            mov      word ptr [0x524d], 0xa
00B17B:  C6 06 51 5B 00               mov      byte ptr [0x5b51], 0
00B180:  C6 06 52 5B C0               mov      byte ptr [0x5b52], 0xc0
00B185:  F6 06 32 25 01               test     byte ptr [0x2532], 1
00B18A:  0F 85 FA 00                  jne      0xb288
00B18E:  9A 76 1E 1E 07               lcall    0x71e, 0x1e76
00B193:  F7 06 93 27 08 00            test     word ptr [0x2793], 8
00B199:  74 29                        je       0xb1c4
00B19B:  C7 06 39 52 23 00            mov      word ptr [0x5239], 0x23
00B1A1:  C7 06 3B 52 A5 00            mov      word ptr [0x523b], 0xa5
00B1A7:  B8 CE FF                     mov      ax, 0xffce
00B1AA:  33 DB                        xor      bx, bx
00B1AC:  8B CB                        mov      cx, bx
00B1AE:  8B D3                        mov      dx, bx
00B1B0:  BF 11 5F                     mov      di, 0x5f11
00B1B3:  9A 00 00 CE 01               lcall    0x1ce, 0
00B1B8:  C7 06 39 52 00 00            mov      word ptr [0x5239], 0
00B1BE:  C7 06 3B 52 C8 00            mov      word ptr [0x523b], 0xc8
00B1C4:  C7 06 49 52 01 00            mov      word ptr [0x5249], 1
00B1CA:  B8 00 00                     mov      ax, 0
00B1CD:  BB 1F 00                     mov      bx, 0x1f
00B1D0:  9A 67 14 99 02               lcall    0x299, 0x1467
00B1D5:  BF 12 66                     mov      di, 0x6612
00B1D8:  9A 0D 21 99 02               lcall    0x299, 0x210d
00B1DD:  F6 06 64 5E 01               test     byte ptr [0x5e64], 1
00B1E2:  0F 84 CD 00                  je       0xb2b3
00B1E6:  8B 1E 58 5E                  mov      bx, word ptr [0x5e58]
00B1EA:  8A 07                        mov      al, byte ptr [bx]
00B1EC:  0A C0                        or       al, al
00B1EE:  0F 85 C1 00                  jne      0xb2b3
00B1F2:  83 3E 4F 52 64               cmp      word ptr [0x524f], 0x64
00B1F7:  75 0E                        jne      0xb207
00B1F9:  A1 4D 52                     mov      ax, word ptr [0x524d]
00B1FC:  83 F8 0A                     cmp      ax, 0xa
00B1FF:  75 06                        jne      0xb207
00B201:  C7 06 4D 52 00 00            mov      word ptr [0x524d], 0
00B207:  C6 06 B8 0D 01               mov      byte ptr [0xdb8], 1
00B20C:  E8 AC 00                     call     0xb2bb
00B20F:  0B C0                        or       ax, ax
00B211:  0F 84 9E 00                  je       0xb2b3
00B215:  83 F8 FF                     cmp      ax, -1
00B218:  74 6E                        je       0xb288
00B21A:  3B 06 1B 25                  cmp      ax, word ptr [0x251b]
00B21E:  74 13                        je       0xb233
00B220:  C4 3E 24 67                  les      di, ptr [0x6724]
00B224:  A3 1B 25                     mov      word ptr [0x251b], ax
00B227:  50                           push     ax
00B228:  8B F8                        mov      di, ax
00B22A:  83 C7 04                     add      di, 4
00B22D:  9A 69 20 DA 04               lcall    0x4da, 0x2069
00B232:  58                           pop      ax
00B233:  50                           push     ax
00B234:  F6 06 A1 0B 01               test     byte ptr [0xba1], 1
00B239:  74 26                        je       0xb261
00B23B:  66 FF 36 19 52               push     dword ptr [0x5219]
00B240:  66 A1 1D 52                  mov      eax, dword ptr [0x521d]
00B244:  66 A3 19 52                  mov      dword ptr [0x5219], eax
00B248:  66 33 C0                     xor      eax, eax
00B24B:  0E                           push     cs
00B24C:  E8 8E 04                     call     0xb6dd
00B24F:  66 8F 06 19 52               pop      dword ptr [0x5219]
00B254:  9A ED 03 1B 0B               lcall    0xb1b, 0x3ed
00B259:  BE 2D 0D                     mov      si, 0xd2d
00B25C:  9A 07 06 1B 0B               lcall    0xb1b, 0x607
00B261:  9A 03 04 1B 0B               lcall    0xb1b, 0x403
00B266:  58                           pop      ax
00B267:  C4 3E 24 67                  les      di, ptr [0x6724]
00B26B:  8B 3E 50 67                  mov      di, word ptr [0x6750]
00B26F:  83 C7 0A                     add      di, 0xa
00B272:  26 C7 05 C1 00               mov      word ptr es:[di], 0xc1
00B277:  26 89 45 02                  mov      word ptr es:[di + 2], ax
00B27B:  26 C7 45 04 00 00            mov      word ptr es:[di + 4], 0
00B281:  C6 06 2D 25 00               mov      byte ptr [0x252d], 0
00B286:  EB 2B                        jmp      0xb2b3
00B288:  C6 06 32 25 01               mov      byte ptr [0x2532], 1
00B28D:  F6 06 2F 25 01               test     byte ptr [0x252f], 1
00B292:  75 1F                        jne      0xb2b3
00B294:  C7 06 F3 24 11 00            mov      word ptr [0x24f3], 0x11
00B29A:  C6 06 2A 25 00               mov      byte ptr [0x252a], 0
00B29F:  C6 06 64 5E 00               mov      byte ptr [0x5e64], 0
00B2A4:  C6 06 2D 25 00               mov      byte ptr [0x252d], 0
00B2A9:  C6 06 D8 27 00               mov      byte ptr [0x27d8], 0
00B2AE:  C6 06 32 25 00               mov      byte ptr [0x2532], 0
00B2B3:  5B                           pop      bx
00B2B4:  5E                           pop      si
00B2B5:  5F                           pop      di
00B2B6:  07                           pop      es
00B2B7:  59                           pop      cx
00B2B8:  5D                           pop      bp
00B2B9:  58                           pop      ax
00B2BA:  C3                           ret     
