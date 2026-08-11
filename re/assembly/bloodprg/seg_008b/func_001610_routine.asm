; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001610
; seg_off: 008b:0760
; group: seg_008b
; provenance: recursive_graph
; byte_count: 151
; boundary: cfg_blocks_15_terminals_5
; terminal: jmp 0x1649:1, jmp 0x165b:1, jmp 0x16a6:2, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: ba151c144bd8270408f41fc313f60d87c1400b6f4e8b11be055029634182af38

001610:  F6 06 E0 27 01               test     byte ptr [0x27e0], 1
001615:  0F 85 8D 00                  jne      0x16a6
001619:  F6 06 DF 0A 01               test     byte ptr [0xadf], 1
00161E:  0F 85 84 00                  jne      0x16a6
001622:  A1 32 0A                     mov      ax, word ptr [0xa32]
001625:  0B C0                        or       ax, ax
001627:  78 7D                        js       0x16a6
001629:  3B 06 34 0A                  cmp      ax, word ptr [0xa34]
00162D:  75 22                        jne      0x1651
00162F:  83 3E 2E 0A 00               cmp      word ptr [0xa2e], 0
001634:  EB 13                        jmp      0x1649
; -- non-contiguous block: next 0x001649 --
001649:  C7 06 32 0A 00 00            mov      word ptr [0xa32], 0
00164F:  EB 0A                        jmp      0x165b
001651:  A3 32 0A                     mov      word ptr [0xa32], ax
001654:  0B C0                        or       ax, ax
001656:  74 03                        je       0x165b
001658:  A3 34 0A                     mov      word ptr [0xa34], ax
00165B:  F6 06 2D 25 01               test     byte ptr [0x252d], 1
001660:  75 0E                        jne      0x1670
001662:  F6 06 AA 67 02               test     byte ptr [0x67aa], 2
001667:  74 07                        je       0x1670
001669:  C6 06 E7 0A 02               mov      byte ptr [0xae7], 2
00166E:  EB 36                        jmp      0x16a6
001670:  80 3E E7 0A 00               cmp      byte ptr [0xae7], 0
001675:  74 06                        je       0x167d
001677:  FE 0E E7 0A                  dec      byte ptr [0xae7]
00167B:  EB 29                        jmp      0x16a6
00167D:  A1 32 0A                     mov      ax, word ptr [0xa32]
001680:  8B 1E 2A 0A                  mov      bx, word ptr [0xa2a]
001684:  8B 0E 2C 0A                  mov      cx, word ptr [0xa2c]
001688:  BD B4 0A                     mov      bp, 0xab4
00168B:  89 5E 00                     mov      word ptr [bp], bx
00168E:  89 4E 02                     mov      word ptr [bp + 2], cx
001691:  89 46 04                     mov      word ptr [bp + 4], ax
001694:  A1 19 52                     mov      ax, word ptr [0x5219]
001697:  89 46 06                     mov      word ptr [bp + 6], ax
00169A:  1E                           push     ds
00169B:  06                           push     es
00169C:  0F A0                        push     fs
00169E:  FF 1E 96 0A                  lcall    [0xa96]
0016A2:  0F A1                        pop      fs
0016A4:  07                           pop      es
0016A5:  1F                           pop      ds
0016A6:  C3                           ret     
