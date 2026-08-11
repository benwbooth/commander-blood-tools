; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008082
; seg_off: 071e:08a2
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_5
; label_comment: temporary label: cs:0x06d4 table entry 5
; incoming: nav_actor_subdispatch:slot_5
; byte_count: 184
; boundary: cfg_blocks_17_terminals_3
; terminal: jmp 0x8127:1, jmp 0x8139:1, ret:1
; direct_callees: 0x007e1c, 0x008c96, 0x00954a
; indirect_calls: 3
; cxx_source: re/borland/bloodprg/seg_071e/func_008082_nav_actor_handler_5.cpp
; routine_bytes_sha256: d2dccfb97b5916a7adb8d2c01f3cda90ac7e142b984f489e297dec099d68eb50

008082:  F6 06 93 27 10               test     byte ptr [0x2793], 0x10
008087:  0F 84 AE 00                  je       0x8139
00808B:  F6 06 8E 27 01               test     byte ptr [0x278e], 1
008090:  75 0B                        jne      0x809d
008092:  80 4E 00 01                  or       byte ptr [bp], 1
008096:  8A 46 00                     mov      al, byte ptr [bp]
008099:  A8 08                        test     al, 8
00809B:  74 65                        je       0x8102
00809D:  8A 1E 93 2A                  mov      bl, byte ptr [0x2a93]
0080A1:  0A 1E 7B 2A                  or       bl, byte ptr [0x2a7b]
0080A5:  74 0D                        je       0x80b4
0080A7:  C6 06 8E 27 01               mov      byte ptr [0x278e], 1
0080AC:  C6 06 8C 27 00               mov      byte ptr [0x278c], 0
0080B1:  E9 85 00                     jmp      0x8139
0080B4:  33 C0                        xor      ax, ax
0080B6:  9A 41 12 99 02               lcall    0x299, 0x1241
0080BB:  C7 06 BF 27 00 00            mov      word ptr [0x27bf], 0
0080C1:  C7 06 32 0A 0A 00            mov      word ptr [0xa32], 0xa
0080C7:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
0080CC:  E8 4D FD                     call     0x7e1c
0080CF:  9C                           pushf   
0080D0:  83 7E 08 07                  cmp      word ptr [bp + 8], 7
0080D4:  75 1F                        jne      0x80f5
0080D6:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
0080DB:  75 04                        jne      0x80e1
0080DD:  0E                           push     cs
0080DE:  E8 69 14                     call     0x954a
0080E1:  50                           push     ax
0080E2:  B8 03 00                     mov      ax, 3
0080E5:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
0080EA:  58                           pop      ax
0080EB:  C6 06 8B 27 08               mov      byte ptr [0x278b], 8
0080F0:  80 0E 93 27 04               or       byte ptr [0x2793], 4
0080F5:  9D                           popf    
0080F6:  73 0A                        jae      0x8102
0080F8:  C6 06 8E 27 00               mov      byte ptr [0x278e], 0
0080FD:  B0 07                        mov      al, 7
0080FF:  88 46 00                     mov      byte ptr [bp], al
008102:  A8 02                        test     al, 2
008104:  74 33                        je       0x8139
008106:  80 36 8A 27 01               xor      byte ptr [0x278a], 1
00810B:  80 26 93 27 FB               and      byte ptr [0x2793], 0xfb
008110:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
008115:  74 07                        je       0x811e
008117:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00811C:  EB 09                        jmp      0x8127
00811E:  0E                           push     cs
00811F:  E8 74 0B                     call     0x8c96
008122:  C6 06 D9 27 01               mov      byte ptr [0x27d9], 1
008127:  C6 06 8C 27 00               mov      byte ptr [0x278c], 0
00812C:  B0 01                        mov      al, 1
00812E:  88 46 00                     mov      byte ptr [bp], al
008131:  B8 04 00                     mov      ax, 4
008134:  9A 41 12 99 02               lcall    0x299, 0x1241
008139:  C3                           ret     
