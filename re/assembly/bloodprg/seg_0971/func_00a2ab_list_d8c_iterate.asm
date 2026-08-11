; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a2ab
; seg_off: 0971:059b
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_iterate
; label_comment: iterate the gs:0xd8c list (3 calls): cx=[0xda0] count; jcxz skip; scans entries gated on [0xd76]. Role: per-entry iteration over the list (count [0xda0]); exact per-entry op partial
; byte_count: 43
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0xa664:1, ret:1
; direct_callees: 0x00a3ad
; indirect_calls: 0
; routine_bytes_sha256: 34fd9b5527f7f74cf515fb19444538cc03fae38879579dbea6c1b381247b205e

00A2AB:  8B 0E A0 0D                  mov      cx, word ptr [0xda0]
00A2AF:  E3 E0                        jcxz     0xa291
00A2B1:  80 3E 76 0D 00               cmp      byte ptr [0xd76], 0
00A2B6:  78 11                        js       0xa2c9
00A2B8:  A1 84 0D                     mov      ax, word ptr [0xd84]
00A2BB:  F7 D8                        neg      ax
00A2BD:  25 FF 07                     and      ax, 0x7ff
00A2C0:  80 C4 08                     add      ah, 8
00A2C3:  3B C1                        cmp      ax, cx
00A2C5:  73 02                        jae      0xa2c9
00A2C7:  8B C8                        mov      cx, ax
00A2C9:  E8 E1 00                     call     0xa3ad
00A2CC:  72 07                        jb       0xa2d5
00A2CE:  29 0E A0 0D                  sub      word ptr [0xda0], cx
00A2D2:  E9 8F 03                     jmp      0xa664
00A2D5:  C3                           ret     
