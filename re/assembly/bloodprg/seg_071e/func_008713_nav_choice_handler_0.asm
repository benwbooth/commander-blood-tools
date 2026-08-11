; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008713
; seg_off: 071e:0f33
; group: seg_071e
; provenance: static_dispatch_table_target
; label: nav_choice_handler_0
; label_comment: if phase bit set, defers C3 record link to named Honk object and clears phase
; incoming: nav_choice_subdispatch:choice_0
; byte_count: 25
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 0415e99bdfa96db2734d75f9db77377a62603f797a3526cd3650f7b9c96ce0df

008713:  F6 06 65 25 01               test     byte ptr [0x2565], 1
008718:  74 11                        je       0x872b
00871A:  A1 54 67                     mov      ax, word ptr [0x6754]
00871D:  A3 6A 67                     mov      word ptr [0x676a], ax
008720:  C7 06 68 67 C3 00            mov      word ptr [0x6768], 0xc3
008726:  C6 06 65 25 00               mov      byte ptr [0x2565], 0
00872B:  C3                           ret     
