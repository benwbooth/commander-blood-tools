; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001b5f
; group: method_table_103a
; provenance: alien_method_table_103a_slot_8@0x42ca
; byte_count: 48
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_001b5f_routine.cpp
; routine_bytes_sha256: 946199cf60611843e6ebdcba55201fae95864e43b62c23206d65165c5169174c

001B5F:  1E                           push     ds
001B60:  8B 75 38                     mov      si, word ptr [di + 0x38]
001B63:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001B66:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001B6A:  83 C6 04                     add      si, 4
001B6D:  81 E6 FC 0F                  and      si, 0xffc
001B71:  89 75 38                     mov      word ptr [di + 0x38], si
001B74:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001B77:  2B C3                        sub      ax, bx
001B79:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001B7E:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001B82:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001B86:  01 04                        add      word ptr [si], ax
001B88:  83 C6 14                     add      si, 0x14
001B8B:  E2 F9                        loop     0x1b86
001B8D:  1F                           pop      ds
001B8E:  C3                           ret     
