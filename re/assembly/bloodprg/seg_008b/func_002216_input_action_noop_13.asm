; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002216
; seg_off: 008b:1366
; group: seg_008b
; provenance: input_action_handler_table_index_13
; label: input_action_noop_13
; label_comment: F6 action; semantically empty, with compiler-style DS/SI/ES/DI preservation retained in the shipped body.
; byte_count: 9
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ac56c7e5e73ead184b39d0dfb7ee9bd454f6a0c21529d178f44550866eb38bbf

002216:  1E                           push     ds
002217:  56                           push     si
002218:  06                           push     es
002219:  57                           push     di
00221A:  5F                           pop      di
00221B:  07                           pop      es
00221C:  5E                           pop      si
00221D:  1F                           pop      ds
00221E:  C3                           ret
