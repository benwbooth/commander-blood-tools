; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001afb
; group: method_table_103a
; provenance: alien_method_table_103a_slot_9@0x433c
; byte_count: 51
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: adbd3507c4073fd2f2e866dc269fcb1885d1873f896c7d1861824f46492dfafe

001AFB:  1E                           push     ds
001AFC:  8B 75 38                     mov      si, word ptr [di + 0x38]
001AFF:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001B02:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001B06:  83 C6 04                     add      si, 4
001B09:  81 E6 FC 0F                  and      si, 0xffc
001B0D:  89 75 38                     mov      word ptr [di + 0x38], si
001B10:  C1 F8 04                     sar      ax, 4
001B13:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001B16:  2B C3                        sub      ax, bx
001B18:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001B1D:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001B21:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001B25:  01 04                        add      word ptr [si], ax
001B27:  83 C6 14                     add      si, 0x14
001B2A:  E2 F9                        loop     0x1b25
001B2C:  1F                           pop      ds
001B2D:  C3                           ret     
