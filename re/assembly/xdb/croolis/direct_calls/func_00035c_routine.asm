; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00035c
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 14
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/direct_calls/func_00035c_routine.cpp
; routine_bytes_sha256: 6eef96589bdec402ce6079bdeac73e81b55468ed5c9e7ed666225fd3145ffe32

00035C:  89 0E 2A 00                  mov      word ptr [0x2a], cx
000360:  89 16 2C 00                  mov      word ptr [0x2c], dx
000364:  B8 04 00                     mov      ax, 4
000367:  CD 33                        int      0x33
000369:  C3                           ret     
