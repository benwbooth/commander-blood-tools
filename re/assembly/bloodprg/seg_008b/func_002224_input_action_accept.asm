; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002224
; seg_off: 008b:1374
; group: seg_008b
; provenance: input_action_handler_table_index_6
; label: input_action_accept
; label_comment: Latches Enter and, in pointer-backed directory mode, converts the low-byte selection index to a 20-byte record offset and commits it only when the record kind is active.
; byte_count: 41
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 154e446bbe3ec08a22491b2223ebb1eb72dc45490d73b1c29c59b5cc23769ac6

002224:  1E                           push     ds
002225:  88 16 15 0B                  mov      byte ptr [0xb15], dl
002229:  F6 06 A6 67 01               test     byte ptr [0x67a6], 1
00222E:  74 1B                        je       0x224b
002230:  A1 A2 67                     mov      ax, word ptr [0x67a2]
002233:  BA 14 00                     mov      dx, 0x14
002236:  F6 E2                        mul      dl
002238:  65 8E 1E 2E 67               mov      ds, word ptr gs:[0x672e]
00223D:  8B D8                        mov      bx, ax
00223F:  8B 5F 12                     mov      bx, word ptr [bx + 0x12]
002242:  83 FB 01                     cmp      bx, 1
002245:  75 04                        jne      0x224b
002247:  65 A3 9E 67                  mov      word ptr gs:[0x679e], ax
00224B:  1F                           pop      ds
00224C:  C3                           ret
