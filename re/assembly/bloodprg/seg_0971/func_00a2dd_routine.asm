; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a2dd
; seg_off: 0971:05cd
; group: seg_0971
; provenance: recursive_graph
; byte_count: 21
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x00a141
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a2dd_routine.cpp
; routine_bytes_sha256: e1d6530f0db000cfc3ecb5028e10c414f83108e0cace3fd43ce4ef71fceeca7f

00A2DD:  80 0E 5F 0D 01               or       byte ptr [0xd5f], 1
00A2E2:  83 3E 9A 0D 00               cmp      word ptr [0xd9a], 0
00A2E7:  75 08                        jne      0xa2f1
00A2E9:  80 0E 5F 0D 02               or       byte ptr [0xd5f], 2
00A2EE:  E8 50 FE                     call     0xa141
00A2F1:  C3                           ret     
