; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a734
; seg_off: 0971:0a24
; group: seg_0971
; provenance: recursive_graph
; label: queue_d8c_enqueue
; label_comment: enqueue into the gs:0xd8c ring buffer (2 calls): [0xd8c] += ax (advance head), [0xd9a] += ax (add to count); clc. Adds an ax-byte entry to the queue head - the enqueue counterpart to queue_d8c_consume 0xa3d0
; byte_count: 10
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a734_queue_d8c_enqueue.cpp
; routine_bytes_sha256: 397508aab83c2e32beb2b763a55283030a360ddd6ed6a5c762662710512c3f09

00A734:  01 06 8C 0D                  add      word ptr [0xd8c], ax
00A738:  01 06 9A 0D                  add      word ptr [0xd9a], ax
00A73C:  F8                           clc     
00A73D:  C3                           ret     
