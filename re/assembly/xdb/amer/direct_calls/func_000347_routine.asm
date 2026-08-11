; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000347
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 14
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/direct_calls/func_000347_routine.cpp
; routine_bytes_sha256: 6eef96589bdec402ce6079bdeac73e81b55468ed5c9e7ed666225fd3145ffe32

000347:  89 0E 2A 00                  mov      word ptr [0x2a], cx
00034B:  89 16 2C 00                  mov      word ptr [0x2c], dx
00034F:  B8 04 00                     mov      ax, 4
000352:  CD 33                        int      0x33
000354:  C3                           ret     
