; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001b80
; group: method_table_103a
; provenance: alien_method_table_103a_slot_8@0x43fa
; byte_count: 48
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 946199cf60611843e6ebdcba55201fae95864e43b62c23206d65165c5169174c

001B80:  1E                           push     ds
001B81:  8B 75 38                     mov      si, word ptr [di + 0x38]
001B84:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001B87:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001B8B:  83 C6 04                     add      si, 4
001B8E:  81 E6 FC 0F                  and      si, 0xffc
001B92:  89 75 38                     mov      word ptr [di + 0x38], si
001B95:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001B98:  2B C3                        sub      ax, bx
001B9A:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001B9F:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001BA3:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001BA7:  01 04                        add      word ptr [si], ax
001BA9:  83 C6 14                     add      si, 0x14
001BAC:  E2 F9                        loop     0x1ba7
001BAE:  1F                           pop      ds
001BAF:  C3                           ret     
