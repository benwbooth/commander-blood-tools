; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0076ba
; seg_off: 04da:231a
; group: seg_04da
; provenance: static_dispatch_table_target
; incoming: byte_parser_dispatch_74e5:byte_0x08
; byte_count: 6
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: a5af950d5dc5f3d0cd3642b67e6d3b53aacaa1f970c35510d431a4faf04ec0dd

0076BA:  AD                           lodsw    ax, word ptr [si]
0076BB:  65 A3 A5 1F                  mov      word ptr gs:[0x1fa5], ax
0076BF:  C3                           ret     
