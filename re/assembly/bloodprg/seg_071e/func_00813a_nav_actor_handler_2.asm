; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00813a
; seg_off: 071e:095a
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_actor_handler_2
; label_comment: temporary label: cs:0x06d4 table entry 2
; incoming: nav_actor_subdispatch:slot_2
; byte_count: 68
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: 0x007e1c
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_071e/func_00813a_nav_actor_handler_2.cpp
; routine_bytes_sha256: c42f71c2705b768ec78d8834bbc52c0765ee76e2c1120adbe5bbf54ab0f92e70

00813A:  51                           push     cx
00813B:  F6 06 93 27 90               test     byte ptr [0x2793], 0x90
008140:  74 3A                        je       0x817c
008142:  80 4E 00 01                  or       byte ptr [bp], 1
008146:  8A 46 00                     mov      al, byte ptr [bp]
008149:  A8 08                        test     al, 8
00814B:  74 2F                        je       0x817c
00814D:  C7 06 32 0A 10 00            mov      word ptr [0xa32], 0x10
008153:  E8 C6 FC                     call     0x7e1c
008156:  73 24                        jae      0x817c
008158:  B8 05 00                     mov      ax, 5
00815B:  9A 1D 01 1B 0B               lcall    0xb1b, 0x11d
008160:  C7 06 F3 24 01 00            mov      word ptr [0x24f3], 1
008166:  BE 51 52                     mov      si, 0x5251
008169:  BF 58 5B                     mov      di, 0x5b58
00816C:  B9 90 00                     mov      cx, 0x90
00816F:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
008172:  C7 06 27 25 00 00            mov      word ptr [0x2527], 0
008178:  C6 46 00 07                  mov      byte ptr [bp], 7
00817C:  59                           pop      cx
00817D:  C3                           ret     
