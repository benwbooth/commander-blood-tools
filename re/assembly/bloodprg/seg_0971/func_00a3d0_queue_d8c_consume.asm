; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a3d0
; seg_off: 0971:06c0
; group: seg_0971
; provenance: recursive_graph
; label: queue_d8c_consume
; label_comment: consume a variable-length entry from the gs:0xd8c queue (2 calls): les si,[0xd90] (tail); lodsw ax = entry size; [0xd9a] -= ax (dec remaining count); si += ax (advance past the entry). Dequeues one variable-length record
; byte_count: 59
; boundary: cfg_blocks_6_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a3d0_queue_d8c_consume.cpp
; routine_bytes_sha256: 4c199e1c8299b742a5d4150cc4185349f9c2b836c9c526973c792330dd904f7e

00A3D0:  C4 36 90 0D                  les      si, ptr [0xd90]
00A3D4:  26 AD                        lodsw    ax, word ptr es:[si]
00A3D6:  29 06 9A 0D                  sub      word ptr [0xd9a], ax
00A3DA:  03 F0                        add      si, ax
00A3DC:  72 06                        jb       0xa3e4
00A3DE:  3B 36 33 52                  cmp      si, word ptr [0x5233]
00A3E2:  76 08                        jbe      0xa3ec
00A3E4:  83 E8 02                     sub      ax, 2
00A3E7:  A3 90 0D                     mov      word ptr [0xd90], ax
00A3EA:  33 C0                        xor      ax, ax
00A3EC:  01 06 90 0D                  add      word ptr [0xd90], ax
00A3F0:  FF 06 1C 13                  inc      word ptr [0x131c]
00A3F4:  A1 60 0D                     mov      ax, word ptr [0xd60]
00A3F7:  40                           inc      ax
00A3F8:  3B 06 64 0D                  cmp      ax, word ptr [0xd64]
00A3FC:  76 09                        jbe      0xa407
00A3FE:  B8 01 00                     mov      ax, 1
00A401:  C7 06 64 0D FF FF            mov      word ptr [0xd64], 0xffff
00A407:  A3 60 0D                     mov      word ptr [0xd60], ax
00A40A:  C3                           ret     
