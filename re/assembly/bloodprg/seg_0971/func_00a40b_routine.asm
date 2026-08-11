; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a40b
; seg_off: 0971:06fb
; group: seg_0971
; provenance: recursive_graph
; byte_count: 15
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a40b_routine.cpp
; routine_bytes_sha256: 849d11b4ac03e9e5ba5e20040766d551aeb86a1ade414058cddafd2d98ec8901

00A40B:  65 80 3E 5F 0D 00            cmp      byte ptr gs:[0xd5f], 0
00A411:  74 06                        je       0xa419
00A413:  65 80 3E 5F 0D 01            cmp      byte ptr gs:[0xd5f], 1
00A419:  C3                           ret     
