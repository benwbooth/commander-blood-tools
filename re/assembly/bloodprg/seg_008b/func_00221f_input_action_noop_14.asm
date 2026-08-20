; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00221f
; seg_off: 008b:136f
; group: seg_008b
; provenance: input_action_handler_table_index_14
; label: input_action_noop_14
; label_comment: F7 action; semantically empty, with compiler-style ES/DI preservation retained in the shipped body.
; byte_count: 5
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: b66e39c8fcae7b58bf2073ec5ee5f739f9291df82306d4a6068f0b1a95c2edba

00221F:  06                           push     es
002220:  57                           push     di
002221:  5F                           pop      di
002222:  07                           pop      es
002223:  C3                           ret
