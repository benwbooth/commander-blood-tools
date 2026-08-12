; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a744
; seg_off: 0971:0a34
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_wrap_bounds_reset
; label_comment: shared tail of list_d8c_bounds_init; independently called at 0xa304 to reset the write-wrap count and both read-wrap limits while preserving AX
; incoming: call@0x00a304->0971:0a34
; byte_count: 19
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 96b2e0264123fd02c44116c1fb9246ee9c4e8dd1dc997290fcfada1d6f2e204d

00A744:  C7 06 62 0D 00 00            mov      word ptr [0xd62], 0
00A74A:  C7 06 64 0D FF FF            mov      word ptr [0xd64], 0xffff
00A750:  C7 06 66 0D FF FF            mov      word ptr [0xd66], 0xffff
00A756:  C3                           ret     
