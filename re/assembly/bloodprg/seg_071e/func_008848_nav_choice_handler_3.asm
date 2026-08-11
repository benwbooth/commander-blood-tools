; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008848
; seg_off: 071e:1068
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_choice_handler_3
; label_comment: navigation-choice handler 3 (dispatch-table entry 3). PORTED: ship3d.rs run_ship_3d_nav_choice_handler_3 || MERGED 2026-07-25 (audit-fixes #130), also recorded as: static C3 record-link handler: DS:0x6756 -> DS:0x676A, clears phase, reloads radio.snd
; incoming: nav_choice_subdispatch:choice_3
; byte_count: 36
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 1042a534ceca566ad5030d96d5ed1b4173f4e95b8d6fdbef331e9ce0aee7cdc3

008848:  F6 06 65 25 01               test     byte ptr [0x2565], 1
00884D:  74 1C                        je       0x886b
00884F:  A1 56 67                     mov      ax, word ptr [0x6756]
008852:  A3 6A 67                     mov      word ptr [0x676a], ax
008855:  C7 06 68 67 C3 00            mov      word ptr [0x6768], 0xc3
00885B:  C6 06 65 25 00               mov      byte ptr [0x2565], 0
008860:  BE 16 0D                     mov      si, 0xd16
008863:  B8 01 00                     mov      ax, 1
008866:  9A 55 08 1B 0B               lcall    0xb1b, 0x855
00886B:  C3                           ret     
