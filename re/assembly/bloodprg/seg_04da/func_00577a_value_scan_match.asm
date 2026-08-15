; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00577a
; seg_off: 04da:03da
; group: seg_04da
; provenance: recursive_graph
; label: value_scan_match
; label_comment: scan BAS DS:SI linked {selector,next_offset,body} nodes for AX; follow next_offset directly on mismatch and return node offset +4 on match; natural C: re/source/bloodprg/candidates/seg_04da/func_00577a_value_scan_match.c
; byte_count: 23
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x577d:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d3ffe46301e6ca63c1a8c4557ae6efb9a2ee8a75bd628d878258f36f946c827a

00577A:  56                           push     si
00577B:  8B D8                        mov      bx, ax
00577D:  AD                           lodsw    ax, word ptr [si]
00577E:  3B C3                        cmp      ax, bx
005780:  74 08                        je       0x578a
005782:  8B 34                        mov      si, word ptr [si]
005784:  0B F6                        or       si, si
005786:  74 05                        je       0x578d
005788:  EB F3                        jmp      0x577d
00578A:  83 C6 02                     add      si, 2
00578D:  8B C6                        mov      ax, si
00578F:  5E                           pop      si
005790:  C3                           ret     
