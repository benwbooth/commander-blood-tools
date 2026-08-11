; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000b65
; group: method_table_103a
; provenance: alien_method_table_103a_slot_12@0x4402
; byte_count: 11
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 07a87d58a684bc18b00cfeeb2cd87d37018e440df7ab597ef4704fb517532d33

000B65:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B68:  83 C6 5E                     add      si, 0x5e
000B6B:  83 6C 52 0F                  sub      word ptr [si + 0x52], 0xf
000B6F:  C3                           ret     
