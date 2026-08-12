; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007776
; seg_off: 04da:23d6
; group: seg_04da
; provenance: static_dispatch_table_target
; label: byte_parser_stream_0f18_append
; label_comment: Byte-parser opcode 0x0D copies one leading little-endian word and then a complete NUL-terminated byte string from DS:SI through the ES destination offset held at GS:0x0F18. Both source and destination advance past the copied NUL, and the final DI offset is stored back to GS:0x0F18.
; incoming: byte_parser_dispatch_74e5:byte_0x0d
; byte_count: 18
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d93f743d34dbe42e419c9a1ca52aae856d5d941fc2b79b8bc2fe15c241b0bfdc

007776:  65 8B 3E 18 0F               mov      di, word ptr gs:[0xf18]
00777B:  A5                           movsw    word ptr es:[di], word ptr [si]
00777C:  AC                           lodsb    al, byte ptr [si]
00777D:  AA                           stosb    byte ptr es:[di], al
00777E:  0A C0                        or       al, al
007780:  75 FA                        jne      0x777c
007782:  65 89 3E 18 0F               mov      word ptr gs:[0xf18], di
007787:  C3                           ret     
