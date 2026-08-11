; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a117
; seg_off: 0971:0407
; group: seg_0971
; provenance: recursive_graph
; label: flag_gated_2751
; label_comment: flag-gated routine: test byte gs:[0x2751],1; if set jump, else ds=es and continue. Branches on the 0x2751 state bit (a render/update enable flag)
; byte_count: 29
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 6cd00db04e9af49e2284c0a02160bf35661d5bd7051e0c780f6e58aafc8d0489

00A117:  1E                           push     ds
00A118:  56                           push     si
00A119:  65 F6 06 51 27 01            test     byte ptr gs:[0x2751], 1
00A11F:  75 10                        jne      0xa131
00A121:  8C C1                        mov      cx, es
00A123:  8E D9                        mov      ds, cx
00A125:  BE 51 52                     mov      si, 0x5251
00A128:  BF 51 58                     mov      di, 0x5851
00A12B:  B9 60 00                     mov      cx, 0x60
00A12E:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00A131:  5E                           pop      si
00A132:  1F                           pop      ds
00A133:  C3                           ret     
