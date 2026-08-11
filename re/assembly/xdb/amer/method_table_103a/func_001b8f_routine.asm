; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001b8f
; group: method_table_103a
; provenance: alien_method_table_103a_slot_9@0x42cc
; byte_count: 51
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_001b8f_routine.cpp
; routine_bytes_sha256: adbd3507c4073fd2f2e866dc269fcb1885d1873f896c7d1861824f46492dfafe

001B8F:  1E                           push     ds
001B90:  8B 75 38                     mov      si, word ptr [di + 0x38]
001B93:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001B96:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001B9A:  83 C6 04                     add      si, 4
001B9D:  81 E6 FC 0F                  and      si, 0xffc
001BA1:  89 75 38                     mov      word ptr [di + 0x38], si
001BA4:  C1 F8 04                     sar      ax, 4
001BA7:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001BAA:  2B C3                        sub      ax, bx
001BAC:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001BB1:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001BB5:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001BB9:  01 04                        add      word ptr [si], ax
001BBB:  83 C6 14                     add      si, 0x14
001BBE:  E2 F9                        loop     0x1bb9
001BC0:  1F                           pop      ds
001BC1:  C3                           ret     
