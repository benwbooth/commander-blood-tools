; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0060dd
; seg_off: 05ad:000d
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: ship_3d_position_distance
; label_comment: computes integer-sqrt distance between two runtime object coordinate records resolved through selector helpers
; incoming: call@0x006bea->0x0060dd
; byte_count: 201
; boundary: cfg_blocks_18_terminals_1
; terminal: ret:1
; direct_callees: 0x006023, 0x0061a6
; indirect_calls: 1
; routine_bytes_sha256: e40542e1f8c79e082a921cda825d24a2e9327662d33da3dc1a3a0104aedc05c6

0060DD:  66 53                        push     ebx
0060DF:  66 52                        push     edx
0060E1:  56                           push     si
0060E2:  57                           push     di
0060E3:  8B 04                        mov      ax, word ptr [si]
0060E5:  3D 00 01                     cmp      ax, 0x100
0060E8:  75 2A                        jne      0x6114
0060EA:  8B 1D                        mov      bx, word ptr [di]
0060EC:  B8 0E 00                     mov      ax, 0xe
0060EF:  E8 31 FF                     call     0x6023
0060F2:  8B D8                        mov      bx, ax
0060F4:  8B 11                        mov      dx, word ptr [bx + di]
0060F6:  BB 00 01                     mov      bx, 0x100
0060F9:  B8 0C 00                     mov      ax, 0xc
0060FC:  E8 24 FF                     call     0x6023
0060FF:  8B D8                        mov      bx, ax
006101:  B8 09 00                     mov      ax, 9
006104:  3B 10                        cmp      dx, word ptr [bx + si]
006106:  74 01                        je       0x6109
006108:  40                           inc      ax
006109:  BB 00 01                     mov      bx, 0x100
00610C:  E8 14 FF                     call     0x6023
00610F:  03 C6                        add      ax, si
006111:  50                           push     ax
006112:  EB 16                        jmp      0x612a
006114:  83 F8 40                     cmp      ax, 0x40
006117:  75 0D                        jne      0x6126
006119:  8B D8                        mov      bx, ax
00611B:  B8 0B 00                     mov      ax, 0xb
00611E:  E8 02 FF                     call     0x6023
006121:  03 C6                        add      ax, si
006123:  50                           push     ax
006124:  EB 04                        jmp      0x612a
006126:  E8 7D 00                     call     0x61a6
006129:  50                           push     ax
00612A:  8B 05                        mov      ax, word ptr [di]
00612C:  3D 00 01                     cmp      ax, 0x100
00612F:  75 29                        jne      0x615a
006131:  8B 1C                        mov      bx, word ptr [si]
006133:  B8 0E 00                     mov      ax, 0xe
006136:  E8 EA FE                     call     0x6023
006139:  8B D8                        mov      bx, ax
00613B:  8B 10                        mov      dx, word ptr [bx + si]
00613D:  BB 00 01                     mov      bx, 0x100
006140:  B8 0C 00                     mov      ax, 0xc
006143:  E8 DD FE                     call     0x6023
006146:  8B D8                        mov      bx, ax
006148:  B8 09 00                     mov      ax, 9
00614B:  3B 11                        cmp      dx, word ptr [bx + di]
00614D:  74 01                        je       0x6150
00614F:  40                           inc      ax
006150:  BB 00 01                     mov      bx, 0x100
006153:  E8 CD FE                     call     0x6023
006156:  03 F8                        add      di, ax
006158:  EB 18                        jmp      0x6172
00615A:  83 F8 40                     cmp      ax, 0x40
00615D:  75 0C                        jne      0x616b
00615F:  8B D8                        mov      bx, ax
006161:  B8 0B 00                     mov      ax, 0xb
006164:  E8 BC FE                     call     0x6023
006167:  03 F8                        add      di, ax
006169:  EB 07                        jmp      0x6172
00616B:  8B F7                        mov      si, di
00616D:  E8 36 00                     call     0x61a6
006170:  8B F8                        mov      di, ax
006172:  5E                           pop      si
006173:  66 33 C0                     xor      eax, eax
006176:  AD                           lodsw    ax, word ptr [si]
006177:  2B 05                        sub      ax, word ptr [di]
006179:  79 02                        jns      0x617d
00617B:  F7 D8                        neg      ax
00617D:  8B 1C                        mov      bx, word ptr [si]
00617F:  2B 5D 02                     sub      bx, word ptr [di + 2]
006182:  79 02                        jns      0x6186
006184:  F7 DB                        neg      bx
006186:  66 98                        cwde
006188:  66 F7 E0                     mul      eax
00618B:  66 93                        xchg     ebx, eax
00618D:  66 98                        cwde
00618F:  66 F7 E0                     mul      eax
006192:  66 03 C3                     add      eax, ebx
006195:  66 0F A4 C2 10               shld     edx, eax, 0x10
00619A:  9A 53 0B CE 01               lcall    0x1ce, 0xb53
00619F:  5F                           pop      di
0061A0:  5E                           pop      si
0061A1:  66 5A                        pop      edx
0061A3:  66 5B                        pop      ebx
0061A5:  C3                           ret
