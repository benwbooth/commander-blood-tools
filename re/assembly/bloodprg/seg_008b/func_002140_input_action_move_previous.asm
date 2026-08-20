; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002140
; seg_off: 008b:1290
; group: seg_008b
; provenance: input_action_handler_table_index_0
; label: input_action_move_previous
; label_comment: Moves the VM-directory selection upward, scrolling its first visible row when needed, or moves the active save-slot selection upward and copies its 16-byte name into the edit buffer.
; byte_count: 77
; boundary: cfg_blocks_9_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: fc33da9380345cdb12641f81268d3e4c98d44f78d8e94e13ec2e0f7bb1af5d8b

002140:  51                           push     cx
002141:  F6 06 A6 67 03               test     byte ptr [0x67a6], 3
002146:  74 1F                        je       0x2167
002148:  A1 9E 67                     mov      ax, word ptr [0x679e]
00214B:  83 F8 FF                     cmp      ax, -1
00214E:  75 3B                        jne      0x218b
002150:  A1 A2 67                     mov      ax, word ptr [0x67a2]
002153:  0B C0                        or       ax, ax
002155:  74 34                        je       0x218b
002157:  48                           dec      ax
002158:  3B 06 A0 67                  cmp      ax, word ptr [0x67a0]
00215C:  7D 04                        jge      0x2162
00215E:  FF 0E A0 67                  dec      word ptr [0x67a0]
002162:  A3 A2 67                     mov      word ptr [0x67a2], ax
002165:  EB 24                        jmp      0x218b
002167:  F6 06 36 27 01               test     byte ptr [0x2736], 1
00216C:  74 1D                        je       0x218b
00216E:  83 3E 32 27 00               cmp      word ptr [0x2732], 0
002173:  74 16                        je       0x218b
002175:  FF 0E 32 27                  dec      word ptr [0x2732]
002179:  83 2E 34 27 20               sub      word ptr [0x2734], 0x20
00217E:  8B 36 34 27                  mov      si, word ptr [0x2734]
002182:  B9 04 00                     mov      cx, 4
002185:  BF 3B 27                     mov      di, 0x273b
002188:  F3 66 A5                     rep movsd
00218B:  59                           pop      cx
00218C:  C3                           ret
