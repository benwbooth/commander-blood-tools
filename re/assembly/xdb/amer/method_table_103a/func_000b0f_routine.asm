; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000b0f
; group: method_table_103a
; provenance: alien_method_table_103a_slot_11@0x42d0
; byte_count: 16
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_000b0f_routine.cpp
; routine_bytes_sha256: bf622e9b3898598d1a4b96727eea2ded42c454fcd7720fd868f5bdcb219858c5

000B0F:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B12:  83 C6 5E                     add      si, 0x5e
000B15:  83 6C 52 0F                  sub      word ptr [si + 0x52], 0xf
000B19:  2E 89 36 C2 1B               mov      word ptr cs:[0x1bc2], si
000B1E:  C3                           ret     
