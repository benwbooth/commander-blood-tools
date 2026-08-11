; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001b46
; group: method_table_103a
; provenance: alien_method_table_103a_slot_13@0x4344
; byte_count: 25
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp bx:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/method_table_103a/func_001b46_routine.cpp
; routine_bytes_sha256: efadb9db7ca5a5948b626039dc5b44f96b1eb61b8ad6ebe0e2758ec249323a81

001B46:  8B 5D 36                     mov      bx, word ptr [di + 0x36]
001B49:  0B DB                        or       bx, bx
001B4B:  74 02                        je       0x1b4f
001B4D:  FF E3                        jmp      bx
001B4F:  C7 45 36 85 1B               mov      word ptr [di + 0x36], 0x1b85
001B54:  C7 45 38 00 00               mov      word ptr [di + 0x38], 0
001B59:  C7 45 3A 00 00               mov      word ptr [di + 0x3a], 0
001B5E:  C3                           ret     
