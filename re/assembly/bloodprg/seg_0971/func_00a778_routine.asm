; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a778
; seg_off: 0971:0a68
; group: seg_0971
; provenance: recursive_graph
; byte_count: 12
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: 0x00a0c3
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a778_routine.cpp
; routine_bytes_sha256: eda7d9fd962cfc6acd23ec76178fe2308e8f4e6d5a28c1cd01d0acd74fc9b92d

00A778:  C4 36 8C 0D                  les      si, ptr [0xd8c]
00A77C:  8B 36 9E 0D                  mov      si, word ptr [0xd9e]
00A780:  E8 40 F9                     call     0xa0c3
00A783:  C3                           ret     
