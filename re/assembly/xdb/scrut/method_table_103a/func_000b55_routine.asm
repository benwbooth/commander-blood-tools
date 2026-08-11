; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000b55
; group: method_table_103a
; provenance: alien_method_table_103a_slot_11@0x4400
; byte_count: 16
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/method_table_103a/func_000b55_routine.cpp
; routine_bytes_sha256: 8d3c03afa6218eb2a8d1038774d66e6b1df824f3f51a9a9744d099cf3ce8a5af

000B55:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B58:  83 C6 5E                     add      si, 0x5e
000B5B:  83 6C 52 0F                  sub      word ptr [si + 0x52], 0xf
000B5F:  2E 89 36 E3 1B               mov      word ptr cs:[0x1be3], si
000B64:  C3                           ret     
