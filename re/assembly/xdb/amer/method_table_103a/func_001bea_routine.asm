; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001bea
; group: method_table_103a
; provenance: alien_method_table_103a_slot_13@0x42d4
; byte_count: 25
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp bx:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9d298b17dbf20f335fe135f28ed40e81110af1973e9628dd3ab5d420b60e9eed

001BEA:  8B 5D 36                     mov      bx, word ptr [di + 0x36]
001BED:  0B DB                        or       bx, bx
001BEF:  74 02                        je       0x1bf3
001BF1:  FF E3                        jmp      bx
001BF3:  C7 45 36 34 1C               mov      word ptr [di + 0x36], 0x1c34
001BF8:  C7 45 38 00 00               mov      word ptr [di + 0x38], 0
001BFD:  C7 45 3A 00 00               mov      word ptr [di + 0x3a], 0
001C02:  C3                           ret     
