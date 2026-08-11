; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007ec0
; seg_off: 071e:06e0
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_1
; label_comment: temporary label: cs:0x06d4 table entry 1
; incoming: nav_actor_subdispatch:slot_1
; byte_count: 220
; boundary: cfg_blocks_20_terminals_4
; terminal: jmp 0x7f15:1, jmp 0x7f6c:1, jmp 0x7f9b:1, ret:1
; direct_callees: 0x007e1c
; indirect_calls: 3
; cxx_source: re/borland/bloodprg/seg_071e/func_007ec0_nav_actor_handler_1.cpp
; routine_bytes_sha256: 7d6edad04293c64b59dbd348263dd512e3c4b8e16b4ac89032b7a406fc120247

007EC0:  F6 06 93 27 10               test     byte ptr [0x2793], 0x10
007EC5:  0F 84 D2 00                  je       0x7f9b
007EC9:  80 3E 93 2A 00               cmp      byte ptr [0x2a93], 0
007ECE:  0F 85 C9 00                  jne      0x7f9b
007ED2:  8A 46 00                     mov      al, byte ptr [bp]
007ED5:  A8 01                        test     al, 1
007ED7:  74 4D                        je       0x7f26
007ED9:  A8 08                        test     al, 8
007EDB:  74 28                        je       0x7f05
007EDD:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
007EE3:  C7 06 32 0A 0B 00            mov      word ptr [0xa32], 0xb
007EE9:  E8 30 FF                     call     0x7e1c
007EEC:  73 17                        jae      0x7f05
007EEE:  C7 06 68 67 C6 00            mov      word ptr [0x6768], 0xc6
007EF4:  A1 D5 27                     mov      ax, word ptr [0x27d5]
007EF7:  A3 6A 67                     mov      word ptr [0x676a], ax
007EFA:  C6 06 92 27 00               mov      byte ptr [0x2792], 0
007EFF:  C6 46 00 00                  mov      byte ptr [bp], 0
007F03:  EB 10                        jmp      0x7f15
007F05:  F6 06 8C 27 01               test     byte ptr [0x278c], 1
007F0A:  75 09                        jne      0x7f15
007F0C:  F6 06 8E 27 01               test     byte ptr [0x278e], 1
007F11:  0F 84 86 00                  je       0x7f9b
007F15:  C7 46 02 15 00               mov      word ptr [bp + 2], 0x15
007F1A:  C6 06 E4 27 01               mov      byte ptr [0x27e4], 1
007F1F:  80 0E 93 27 04               or       byte ptr [0x2793], 4
007F24:  EB 46                        jmp      0x7f6c
007F26:  1E                           push     ds
007F27:  56                           push     si
007F28:  8B 36 52 67                  mov      si, word ptr [0x6752]
007F2C:  66 8E 1E 26 67               mov      ds, word ptr [0x6726]
007F31:  8B 74 16                     mov      si, word ptr [si + 0x16]
007F34:  8B DE                        mov      bx, si
007F36:  81 3C 00 01                  cmp      word ptr [si], 0x100
007F3A:  5E                           pop      si
007F3B:  1F                           pop      ds
007F3C:  75 5D                        jne      0x7f9b
007F3E:  89 1E D5 27                  mov      word ptr [0x27d5], bx
007F42:  8A 1E E4 27                  mov      bl, byte ptr [0x27e4]
007F46:  0A 1E 8A 27                  or       bl, byte ptr [0x278a]
007F4A:  74 4F                        je       0x7f9b
007F4C:  A8 04                        test     al, 4
007F4E:  75 1C                        jne      0x7f6c
007F50:  F6 06 8E 27 01               test     byte ptr [0x278e], 1
007F55:  75 44                        jne      0x7f9b
007F57:  B8 04 00                     mov      ax, 4
007F5A:  9A 41 12 99 02               lcall    0x299, 0x1241
007F5F:  C7 46 02 15 00               mov      word ptr [bp + 2], 0x15
007F64:  B8 05 00                     mov      ax, 5
007F67:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
007F6C:  E8 AD FE                     call     0x7e1c
007F6F:  73 2A                        jae      0x7f9b
007F71:  F6 06 8E 27 01               test     byte ptr [0x278e], 1
007F76:  75 07                        jne      0x7f7f
007F78:  F6 06 8C 27 01               test     byte ptr [0x278c], 1
007F7D:  74 0E                        je       0x7f8d
007F7F:  B8 04 00                     mov      ax, 4
007F82:  9A 41 12 99 02               lcall    0x299, 0x1241
007F87:  C6 46 00 00                  mov      byte ptr [bp], 0
007F8B:  EB 0E                        jmp      0x7f9b
007F8D:  C6 46 00 01                  mov      byte ptr [bp], 1
007F91:  80 0E 93 27 04               or       byte ptr [0x2793], 4
007F96:  C7 46 02 13 00               mov      word ptr [bp + 2], 0x13
007F9B:  C3                           ret     
