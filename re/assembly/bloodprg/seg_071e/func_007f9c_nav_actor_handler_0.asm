; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007f9c
; seg_off: 071e:07bc
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_0
; label_comment: temporary label: cs:0x06d4 table entry 0
; incoming: nav_actor_subdispatch:slot_0
; byte_count: 230
; boundary: cfg_blocks_19_terminals_4
; terminal: jmp 0x800e:1, jmp 0x8081:2, ret:1
; direct_callees: 0x007e1c
; indirect_calls: 6
; cxx_source: re/borland/bloodprg/seg_071e/func_007f9c_nav_actor_handler_0.cpp
; routine_bytes_sha256: 1caceb16901003237d89814aa861ebbf5b25573214dbf0fd25688441a4805e24

007F9C:  F6 06 93 27 10               test     byte ptr [0x2793], 0x10
007FA1:  0F 84 DC 00                  je       0x8081
007FA5:  80 3E 7B 2A 00               cmp      byte ptr [0x2a7b], 0
007FAA:  0F 85 D3 00                  jne      0x8081
007FAE:  8A 46 00                     mov      al, byte ptr [bp]
007FB1:  A8 01                        test     al, 1
007FB3:  74 73                        je       0x8028
007FB5:  A8 08                        test     al, 8
007FB7:  74 55                        je       0x800e
007FB9:  C7 06 34 0A 00 00            mov      word ptr [0xa34], 0
007FBF:  C7 06 32 0A 0A 00            mov      word ptr [0xa32], 0xa
007FC5:  33 C0                        xor      ax, ax
007FC7:  9A 41 12 99 02               lcall    0x299, 0x1241
007FCC:  B8 04 00                     mov      ax, 4
007FCF:  9A 41 12 99 02               lcall    0x299, 0x1241
007FD4:  E8 45 FE                     call     0x7e1c
007FD7:  83 7E 08 01                  cmp      word ptr [bp + 8], 1
007FDB:  75 07                        jne      0x7fe4
007FDD:  C6 06 8B 27 08               mov      byte ptr [0x278b], 8
007FE2:  EB 2A                        jmp      0x800e
007FE4:  80 3E 8B 27 00               cmp      byte ptr [0x278b], 0
007FE9:  75 23                        jne      0x800e
007FEB:  C6 46 00 07                  mov      byte ptr [bp], 7
007FEF:  C7 06 68 67 C1 00            mov      word ptr [0x6768], 0xc1
007FF5:  C6 06 DA 27 01               mov      byte ptr [0x27da], 1
007FFA:  B8 04 00                     mov      ax, 4
007FFD:  9A 41 12 99 02               lcall    0x299, 0x1241
008002:  C6 06 8C 27 00               mov      byte ptr [0x278c], 0
008007:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00800C:  EB 73                        jmp      0x8081
00800E:  F6 06 8C 27 01               test     byte ptr [0x278c], 1
008013:  75 6C                        jne      0x8081
008015:  C7 46 02 14 00               mov      word ptr [bp + 2], 0x14
00801A:  C6 06 E4 27 01               mov      byte ptr [0x27e4], 1
00801F:  C6 46 00 00                  mov      byte ptr [bp], 0
008023:  80 0E 93 27 04               or       byte ptr [0x2793], 4
008028:  8B 1E 6A 67                  mov      bx, word ptr [0x676a]
00802C:  0B DB                        or       bx, bx
00802E:  74 51                        je       0x8081
008030:  8A 1E E4 27                  mov      bl, byte ptr [0x27e4]
008034:  0A 1E 8C 27                  or       bl, byte ptr [0x278c]
008038:  74 47                        je       0x8081
00803A:  A8 04                        test     al, 4
00803C:  75 15                        jne      0x8053
00803E:  B8 04 00                     mov      ax, 4
008041:  9A 41 12 99 02               lcall    0x299, 0x1241
008046:  C7 46 02 14 00               mov      word ptr [bp + 2], 0x14
00804B:  B8 05 00                     mov      ax, 5
00804E:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
008053:  E8 C6 FD                     call     0x7e1c
008056:  73 29                        jae      0x8081
008058:  F6 06 8C 27 01               test     byte ptr [0x278c], 1
00805D:  75 14                        jne      0x8073
00805F:  C7 06 6A 67 00 00            mov      word ptr [0x676a], 0
008065:  B8 04 00                     mov      ax, 4
008068:  9A 41 12 99 02               lcall    0x299, 0x1241
00806D:  C6 46 00 00                  mov      byte ptr [bp], 0
008071:  EB 0E                        jmp      0x8081
008073:  C6 46 00 01                  mov      byte ptr [bp], 1
008077:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00807C:  C7 46 02 12 00               mov      word ptr [bp + 2], 0x12
008081:  C3                           ret     
