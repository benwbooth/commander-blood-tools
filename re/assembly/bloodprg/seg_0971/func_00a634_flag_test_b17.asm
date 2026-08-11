; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a634
; seg_off: 0971:0924
; group: seg_0971
; provenance: recursive_graph
; label: flag_test_b17
; label_comment: flag test: ds=ax; test byte [0xb17],1. Reads the 0xb17 state bit (a mode/enable flag) and restores ds
; byte_count: 14
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 70ed47cfe0a09240e61c1dda4ba1daf5f24019813d06e2f4d0151a221c046432

00A634:  50                           push     ax
00A635:  1E                           push     ds
00A636:  8C E8                        mov      ax, gs
00A638:  8E D8                        mov      ds, ax
00A63A:  F6 06 17 0B 01               test     byte ptr [0xb17], 1
00A63F:  1F                           pop      ds
00A640:  58                           pop      ax
00A641:  C3                           ret     
