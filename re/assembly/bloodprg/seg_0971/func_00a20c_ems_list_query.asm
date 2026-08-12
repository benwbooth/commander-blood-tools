; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a20c
; seg_off: 0971:04fc
; group: seg_0971
; provenance: recursive_graph
; label: list_d8c_activate_ready
; label_comment: report carry clear when an entry is already active, or activate the tail entry when queued bytes contain its complete extent. A 0x6d6d link marker bypasses the extent-size check. Calls 0xa552 with AX=extent, ES:SI=payload, and BP selected from 0x0abe or 0x0da8 by resource flag 0x40.
; byte_count: 52
; boundary: cfg_blocks_8_terminals_1
; terminal: ret:1
; direct_callees: 0x00a552
; indirect_calls: 0
; routine_bytes_sha256: 8c2be585dd7b3567cf6c2e7a3472cdc57ea5657c6c2366f9f2a85681837f2f33

00A20C:  83 3E 96 0D 00               cmp      word ptr [0xd96], 0
00A211:  77 2C                        ja       0xa23f
00A213:  8B 0E 9A 0D                  mov      cx, word ptr [0xd9a]
00A217:  F9                           stc     
00A218:  E3 25                        jcxz     0xa23f
00A21A:  C4 36 90 0D                  les      si, ptr [0xd90]
00A21E:  26 AD                        lodsw    ax, word ptr es:[si]
00A220:  26 81 3C 6D 6D               cmp      word ptr es:[si], 0x6d6d
00A225:  74 04                        je       0xa22b
00A227:  3B C8                        cmp      cx, ax
00A229:  72 14                        jb       0xa23f
00A22B:  8B 2E BE 0A                  mov      bp, word ptr [0xabe]
00A22F:  F6 06 76 0D 40               test     byte ptr [0xd76], 0x40
00A234:  74 04                        je       0xa23a
00A236:  8B 2E A8 0D                  mov      bp, word ptr [0xda8]
00A23A:  E8 15 03                     call     0xa552
00A23D:  33 C0                        xor      ax, ax
00A23F:  C3                           ret     
