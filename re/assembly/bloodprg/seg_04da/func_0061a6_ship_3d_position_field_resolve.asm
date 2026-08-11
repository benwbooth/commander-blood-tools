; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0061a6
; seg_off: 04da:0e06
; group: seg_04da
; provenance: recursive_graph
; label: ship_3d_position_field_resolve
; label_comment: resolves the coordinate field for an object by following selector-0x11 links, with arche fallback and kind-0x0100 match/mismatch blocks
; byte_count: 106
; boundary: cfg_blocks_13_terminals_5
; terminal: jmp 0x61ad:2, jmp 0x620d:2, ret:1
; direct_callees: 0x006023
; indirect_calls: 0
; routine_bytes_sha256: bef68d922e95de6fb8528b1283f26d7092375640a4d5c780b7db1346bfd5dbea

0061A6:  53                           push     bx
0061A7:  56                           push     si
0061A8:  66 33 C0                     xor      eax, eax
0061AB:  8B 04                        mov      ax, word ptr [si]
0061AD:  3D 00 01                     cmp      ax, 0x100
0061B0:  74 39                        je       0x61eb
0061B2:  83 F8 08                     cmp      ax, 8
0061B5:  74 28                        je       0x61df
0061B7:  83 F8 10                     cmp      ax, 0x10
0061BA:  74 23                        je       0x61df
0061BC:  3D 00 02                     cmp      ax, 0x200
0061BF:  74 1E                        je       0x61df
0061C1:  8B D8                        mov      bx, ax
0061C3:  B8 11 00                     mov      ax, 0x11
0061C6:  E8 5A FE                     call     0x6023
0061C9:  03 F0                        add      si, ax
0061CB:  8B 34                        mov      si, word ptr [si]
0061CD:  83 FE FF                     cmp      si, -1
0061D0:  75 09                        jne      0x61db
0061D2:  65 8B 36 52 67               mov      si, word ptr gs:[0x6752]
0061D7:  8B 04                        mov      ax, word ptr [si]
0061D9:  EB D2                        jmp      0x61ad
0061DB:  8B 04                        mov      ax, word ptr [si]
0061DD:  EB CE                        jmp      0x61ad
0061DF:  8B D8                        mov      bx, ax
0061E1:  B8 0B 00                     mov      ax, 0xb
0061E4:  E8 3C FE                     call     0x6023
0061E7:  03 C6                        add      ax, si
0061E9:  EB 22                        jmp      0x620d
0061EB:  8B D8                        mov      bx, ax
0061ED:  B8 0C 00                     mov      ax, 0xc
0061F0:  E8 30 FE                     call     0x6023
0061F3:  66 98                        cwde    
0061F5:  67 3B 14 30                  cmp      dx, word ptr [eax + esi]
0061F9:  75 0A                        jne      0x6205
0061FB:  B8 09 00                     mov      ax, 9
0061FE:  E8 22 FE                     call     0x6023
006201:  03 C6                        add      ax, si
006203:  EB 08                        jmp      0x620d
006205:  B8 0A 00                     mov      ax, 0xa
006208:  E8 18 FE                     call     0x6023
00620B:  03 C6                        add      ax, si
00620D:  5E                           pop      si
00620E:  5B                           pop      bx
00620F:  C3                           ret     
