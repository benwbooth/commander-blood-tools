; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a3ad
; seg_off: 0971:069d
; group: seg_0971
; provenance: recursive_graph
; label: queue_d8c_empty_check
; label_comment: queue empty/full check (2 calls): ax=[0xd8c] head, bx=[0xd90] tail; cmp -> the gs:0xd8c queue is a ring buffer, head vs tail determines empty. Confirms 0xd8c = a variable-length RING-BUFFER QUEUE (head 0xd8c, tail 0xd90, count 0xd9a, wrap [0x5233])
; byte_count: 35
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ba1b14b60f408bf920ea5e66674732334e992ec47235796073b75a72c3a76539

00A3AD:  A1 8C 0D                     mov      ax, word ptr [0xd8c]
00A3B0:  8B 1E 90 0D                  mov      bx, word ptr [0xd90]
00A3B4:  3B C3                        cmp      ax, bx
00A3B6:  73 09                        jae      0xa3c1
00A3B8:  03 C1                        add      ax, cx
00A3BA:  83 C0 12                     add      ax, 0x12
00A3BD:  3B D8                        cmp      bx, ax
00A3BF:  72 0E                        jb       0xa3cf
00A3C1:  A1 9A 0D                     mov      ax, word ptr [0xd9a]
00A3C4:  83 C0 0A                     add      ax, 0xa
00A3C7:  03 C1                        add      ax, cx
00A3C9:  72 04                        jb       0xa3cf
00A3CB:  39 06 98 0D                  cmp      word ptr [0xd98], ax
00A3CF:  C3                           ret     
