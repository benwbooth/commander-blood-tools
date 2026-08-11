; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009510
; seg_off: 071e:1d30
; group: seg_071e
; provenance: recursive_graph
; label: presentation_mode_check
; label_comment: presentation-mode gate: ax=[0x2793]; and 0xff0f; test 2 -> branch on the presentation-active bit of [0x2793]
; byte_count: 58
; boundary: cfg_blocks_8_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: e392b2ee6954a3ddd813ed580b630c89d89ae7d30c5c2a6b645972a2e84425c4

009510:  53                           push     bx
009511:  52                           push     dx
009512:  A1 93 27                     mov      ax, word ptr [0x2793]
009515:  25 0F FF                     and      ax, 0xff0f
009518:  A9 02 00                     test     ax, 2
00951B:  75 27                        jne      0x9544
00951D:  BB 01 00                     mov      bx, 1
009520:  8B 16 95 27                  mov      dx, word ptr [0x2795]
009524:  83 FA 16                     cmp      dx, 0x16
009527:  7E 16                        jle      0x953f
009529:  81 FA 9D 00                  cmp      dx, 0x9d
00952D:  7F 10                        jg       0x953f
00952F:  03 DB                        add      bx, bx
009531:  83 FA 43                     cmp      dx, 0x43
009534:  7E 09                        jle      0x953f
009536:  03 DB                        add      bx, bx
009538:  83 FA 70                     cmp      dx, 0x70
00953B:  7E 02                        jle      0x953f
00953D:  03 DB                        add      bx, bx
00953F:  C1 E3 04                     shl      bx, 4
009542:  0B C3                        or       ax, bx
009544:  A3 93 27                     mov      word ptr [0x2793], ax
009547:  5A                           pop      dx
009548:  5B                           pop      bx
009549:  C3                           ret     
