; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a622
; seg_off: 0971:0912
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_read
; label_comment: read from the gs:0xd8c list (3 calls): cx=2; call 0xa664; les si,gs:[0xd8c]; consumes entries from the list. Role: list read/dequeue; exact record op partial
; byte_count: 18
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x00a664
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a622_list_d8c_read.cpp
; routine_bytes_sha256: b11b14dd5e323bd7f73ffe721f8a5ddcce7dd7e34e770391077c71deb442a6fe

00A622:  B9 02 00                     mov      cx, 2
00A625:  E8 3C 00                     call     0xa664
00A628:  72 09                        jb       0xa633
00A62A:  65 C4 36 8C 0D               les      si, ptr gs:[0xd8c]
00A62F:  26 8B 44 FE                  mov      ax, word ptr es:[si - 2]
00A633:  C3                           ret     
