; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001acb
; group: method_table_103a
; provenance: alien_method_table_103a_slot_8@0x433a
; byte_count: 48
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 946199cf60611843e6ebdcba55201fae95864e43b62c23206d65165c5169174c

001ACB:  1E                           push     ds
001ACC:  8B 75 38                     mov      si, word ptr [di + 0x38]
001ACF:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001AD2:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001AD6:  83 C6 04                     add      si, 4
001AD9:  81 E6 FC 0F                  and      si, 0xffc
001ADD:  89 75 38                     mov      word ptr [di + 0x38], si
001AE0:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001AE3:  2B C3                        sub      ax, bx
001AE5:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001AEA:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001AEE:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001AF2:  01 04                        add      word ptr [si], ax
001AF4:  83 C6 14                     add      si, 0x14
001AF7:  E2 F9                        loop     0x1af2
001AF9:  1F                           pop      ds
001AFA:  C3                           ret     
