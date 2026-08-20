; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002203
; seg_off: 008b:1353
; group: seg_008b
; provenance: input_action_handler_table_index_4
; label: input_action_request_shutdown
; label_comment: Sets the main-loop shutdown latch. No shipped key translates to this action index.
; byte_count: 6
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: eedf9d68683b58593f0a258026640f0482661296a9311a1d88da921926562ccb

002203:  C6 06 13 0B 01               mov      byte ptr [0xb13], 1
002208:  C3                           ret
