; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0022d0
; seg_off: 008b:1420
; group: seg_008b
; provenance: input_action_handler_table_index_8
; label: input_action_latch_text_key
; label_comment: Publishes the raw low keyboard byte for save-name editing and other active UI consumers.
; byte_count: 5
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d025dfd0d91c8157f4e869024a98523b9afaa828e4690b9fdfcab45864a64231

0022D0:  88 16 15 0B                  mov      byte ptr [0xb15], dl
0022D4:  C3                           ret
