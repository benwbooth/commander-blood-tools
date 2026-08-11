; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a38e
; seg_off: 0971:067e
; group: seg_0971
; provenance: recursive_graph
; label: queue_d8c_wrap
; label_comment: ring-buffer pointer wrap (2 calls): si += ax; if si > [0x5233] (buffer end) wrap. Handles the gs:0xd8c queue's circular wraparound
; byte_count: 31
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a38e_queue_d8c_wrap.cpp
; routine_bytes_sha256: 84de4c19f0e44213424ba2e92b86fe66d3d582c0c42555fe59031b629b10a323

00A38E:  03 F0                        add      si, ax
00A390:  72 06                        jb       0xa398
00A392:  3B 36 33 52                  cmp      si, word ptr [0x5233]
00A396:  76 0A                        jbe      0xa3a2
00A398:  33 C9                        xor      cx, cx
00A39A:  87 0E 8C 0D                  xchg     word ptr [0xd8c], cx
00A39E:  89 0E 98 0D                  mov      word ptr [0xd98], cx
00A3A2:  83 E8 02                     sub      ax, 2
00A3A5:  A3 A0 0D                     mov      word ptr [0xda0], ax
00A3A8:  FF 06 62 0D                  inc      word ptr [0xd62]
00A3AC:  C3                           ret     
