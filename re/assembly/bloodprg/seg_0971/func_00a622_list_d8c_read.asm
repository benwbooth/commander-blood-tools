; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a622
; seg_off: 0971:0912
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_read
; label_comment: stage the next two-byte list entry extent through ems_paged_read 0xa664. On success ES:SI is the post-read gs:0xd8c cursor, AX is the extent word at ES:[SI-2], and carry is clear; on transport failure carry remains set and ES:SI/AX are not replaced.
; byte_count: 18
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x00a664
; indirect_calls: 0
; routine_bytes_sha256: b11b14dd5e323bd7f73ffe721f8a5ddcce7dd7e34e770391077c71deb442a6fe

00A622:  B9 02 00                     mov      cx, 2
00A625:  E8 3C 00                     call     0xa664
00A628:  72 09                        jb       0xa633
00A62A:  65 C4 36 8C 0D               les      si, ptr gs:[0xd8c]
00A62F:  26 8B 44 FE                  mov      ax, word ptr es:[si - 2]
00A633:  C3                           ret     
