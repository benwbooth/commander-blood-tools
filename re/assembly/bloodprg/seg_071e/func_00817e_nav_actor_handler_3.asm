; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00817e
; seg_off: 071e:099e
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_3
; label_comment: temporary label: cs:0x06d4 table entry 3
; incoming: nav_actor_subdispatch:slot_3
; byte_count: 125
; boundary: cfg_blocks_14_terminals_2
; terminal: jmp 0x81e8:1, ret:1
; direct_callees: 0x007e1c
; indirect_calls: 2
; routine_bytes_sha256: 65408f79c2031a8c3137a107deed6e6416e5ac4fcfb69de418b7bcd06668fe4b

00817E:  F6 06 93 27 40               test     byte ptr [0x2793], 0x40
008183:  74 75                        je       0x81fa
008185:  80 4E 00 01                  or       byte ptr [bp], 1
008189:  8A 46 00                     mov      al, byte ptr [bp]
00818C:  A8 08                        test     al, 8
00818E:  74 58                        je       0x81e8
008190:  C7 06 32 0A 0D 00            mov      word ptr [0xa32], 0xd
008196:  F6 06 E1 27 01               test     byte ptr [0x27e1], 1
00819B:  74 19                        je       0x81b6
00819D:  83 3E 93 2B 64               cmp      word ptr [0x2b93], 0x64
0081A2:  7D 12                        jge      0x81b6
0081A4:  C7 06 93 2B 6A 00            mov      word ptr [0x2b93], 0x6a
0081AA:  F6 06 B2 1F 01               test     byte ptr [0x1fb2], 1
0081AF:  74 05                        je       0x81b6
0081B1:  9A 43 02 71 09               lcall    0x971, 0x243
0081B6:  C6 06 3E 0A 00               mov      byte ptr [0xa3e], 0
0081BB:  C6 06 40 0A 00               mov      byte ptr [0xa40], 0
0081C0:  E8 59 FC                     call     0x7e1c
0081C3:  73 23                        jae      0x81e8
0081C5:  B8 04 00                     mov      ax, 4
0081C8:  9A 41 12 99 02               lcall    0x299, 0x1241
0081CD:  F6 06 E1 27 01               test     byte ptr [0x27e1], 1
0081D2:  74 06                        je       0x81da
0081D4:  C6 46 00 01                  mov      byte ptr [bp], 1
0081D8:  EB 0E                        jmp      0x81e8
0081DA:  C6 46 00 01                  mov      byte ptr [bp], 1
0081DE:  C6 06 E1 27 01               mov      byte ptr [0x27e1], 1
0081E3:  80 0E 93 27 04               or       byte ptr [0x2793], 4
0081E8:  F6 06 E1 27 01               test     byte ptr [0x27e1], 1
0081ED:  74 0B                        je       0x81fa
0081EF:  F6 46 00 04                  test     byte ptr [bp], 4
0081F3:  74 05                        je       0x81fa
0081F5:  C6 06 E5 27 01               mov      byte ptr [0x27e5], 1
0081FA:  C3                           ret     
