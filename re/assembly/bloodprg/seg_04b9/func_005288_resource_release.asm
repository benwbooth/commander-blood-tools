; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x005288
; seg_off: 04b9:00f8
; group: seg_04b9
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_release
; label_comment: SEG 0x4b9:0xf8: release resource handle AX if loaded. bx=AX<<3; if fs:[bx+2]&3 (loaded flag) set, call 0x529c (actual free). Resource table entry = 8 bytes {segment u16@+0, flags u16@+2 (bits0-1=loaded), ...}. Counterpart to resource_handle_resolve 0x5320
; incoming: call@0x0053b5->04b9:00f8
; byte_count: 20
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: 0x00529c
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04b9/func_005288_resource_release.cpp
; routine_bytes_sha256: 51355c79878d14bfff085e4b54683078a95a4455f3450e8d8e52b14fc5f05820

005288:  53                           push     bx
005289:  8B D8                        mov      bx, ax
00528B:  C1 E3 03                     shl      bx, 3
00528E:  64 F7 47 02 03 00            test     word ptr fs:[bx + 2], 3
005294:  74 04                        je       0x529a
005296:  0E                           push     cs
005297:  E8 02 00                     call     0x529c
00529A:  5B                           pop      bx
00529B:  CB                           retf    
