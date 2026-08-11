; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001bb0
; group: method_table_103a
; provenance: alien_method_table_103a_slot_9@0x43fc
; byte_count: 51
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: adbd3507c4073fd2f2e866dc269fcb1885d1873f896c7d1861824f46492dfafe

001BB0:  1E                           push     ds
001BB1:  8B 75 38                     mov      si, word ptr [di + 0x38]
001BB4:  8B 5D 3A                     mov      bx, word ptr [di + 0x3a]
001BB7:  8B 84 36 00                  mov      ax, word ptr [si + 0x36]
001BBB:  83 C6 04                     add      si, 4
001BBE:  81 E6 FC 0F                  and      si, 0xffc
001BC2:  89 75 38                     mov      word ptr [di + 0x38], si
001BC5:  C1 F8 04                     sar      ax, 4
001BC8:  89 45 3A                     mov      word ptr [di + 0x3a], ax
001BCB:  2B C3                        sub      ax, bx
001BCD:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
001BD2:  64 8B 75 1C                  mov      si, word ptr fs:[di + 0x1c]
001BD6:  64 8B 4D 20                  mov      cx, word ptr fs:[di + 0x20]
001BDA:  01 04                        add      word ptr [si], ax
001BDC:  83 C6 14                     add      si, 0x14
001BDF:  E2 F9                        loop     0x1bda
001BE1:  1F                           pop      ds
001BE2:  C3                           ret     
