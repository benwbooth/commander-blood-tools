; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001bfb
; group: method_table_103a
; provenance: alien_method_table_103a_slot_13@0x4404
; byte_count: 25
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp bx:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: b1a0b8943aaa5f76c2091d91bb198b2df18d4e3ffd989a47b3cc8bafc61b41d6

001BFB:  8B 5D 36                     mov      bx, word ptr [di + 0x36]
001BFE:  0B DB                        or       bx, bx
001C00:  74 02                        je       0x1c04
001C02:  FF E3                        jmp      bx
001C04:  C7 45 36 45 1C               mov      word ptr [di + 0x36], 0x1c45
001C09:  C7 45 38 00 00               mov      word ptr [di + 0x38], 0
001C0E:  C7 45 3A 00 00               mov      word ptr [di + 0x3a], 0
001C13:  C3                           ret     
