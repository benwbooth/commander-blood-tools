; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007549
; seg_off: 04da:21a9
; group: seg_04da
; provenance: static_dispatch_table_target
; label: byte_parser_op_02_mark_b16
; label_comment: Byte-parser opcode 0x02 sets GS:0x0B16 to one and returns without changing registers or flags. It is byte-identical to the handlers for opcodes 0x01, 0x0F, and 0x04.
; incoming: byte_parser_dispatch_74e5:byte_0x02
; byte_count: 7
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: dfd2efc51f60cd6ca3547264a73e2e048fd3cd2abb1d8dc5475ddd30470c4b08

007549:  65 C6 06 16 0B 01            mov      byte ptr gs:[0xb16], 1
00754F:  C3                           ret     
