; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0081fb
; seg_off: 071e:0a1b
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_4
; label_comment: temporary label: cs:0x06d4 table entry 4
; incoming: nav_actor_subdispatch:slot_4
; byte_count: 110
; boundary: cfg_blocks_9_terminals_2
; terminal: jmp 0x8268:1, ret:1
; direct_callees: 0x007e1c
; indirect_calls: 3
; routine_bytes_sha256: f414ae0fbcf0b4cc14391122fc2818f19e7033e3f44fd400f0e1394c64df9087

0081FB:  F6 06 93 27 20               test     byte ptr [0x2793], 0x20
008200:  74 66                        je       0x8268
008202:  80 4E 00 01                  or       byte ptr [bp], 1
008206:  8A 46 00                     mov      al, byte ptr [bp]
008209:  A8 04                        test     al, 4
00820B:  75 1A                        jne      0x8227
00820D:  A8 08                        test     al, 8
00820F:  74 57                        je       0x8268
008211:  8B 1E 6A 67                  mov      bx, word ptr [0x676a]
008215:  0B DB                        or       bx, bx
008217:  75 0E                        jne      0x8227
008219:  8B 1E 5A 67                  mov      bx, word ptr [0x675a]
00821D:  0B DB                        or       bx, bx
00821F:  75 06                        jne      0x8227
008221:  C6 46 00 01                  mov      byte ptr [bp], 1
008225:  EB 41                        jmp      0x8268
008227:  C7 06 32 0A 04 00            mov      word ptr [0xa32], 4
00822D:  E8 EC FB                     call     0x7e1c
008230:  73 36                        jae      0x8268
008232:  B8 02 00                     mov      ax, 2
008235:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
00823A:  A1 5A 67                     mov      ax, word ptr [0x675a]
00823D:  A3 6A 67                     mov      word ptr [0x676a], ax
008240:  C7 06 68 67 C4 00            mov      word ptr [0x6768], 0xc4
008246:  C7 06 5A 67 00 00            mov      word ptr [0x675a], 0
00824C:  C6 46 00 01                  mov      byte ptr [bp], 1
008250:  B8 04 00                     mov      ax, 4
008253:  9A 41 12 99 02               lcall    0x299, 0x1241
008258:  80 0E 93 27 04               or       byte ptr [0x2793], 4
00825D:  B8 01 00                     mov      ax, 1
008260:  BE 16 0D                     mov      si, 0xd16
008263:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
008268:  C3                           ret     
