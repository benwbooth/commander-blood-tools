; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000b50
; group: method_table_103a
; provenance: alien_method_table_103a_slot_11@0x4340
; byte_count: 16
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/method_table_103a/func_000b50_routine.cpp
; routine_bytes_sha256: 109d245c3c4255132c8885031405d043b675d83028f3e698bf65d345ccba27cb

000B50:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B53:  83 C6 5E                     add      si, 0x5e
000B56:  83 6C 52 0F                  sub      word ptr [si + 0x52], 0xf
000B5A:  2E 89 36 2E 1B               mov      word ptr cs:[0x1b2e], si
000B5F:  C3                           ret     
