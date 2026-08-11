; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000966
; group: method_table_103a
; provenance: alien_method_table_103a_slot_10@0x43fe
; byte_count: 51
; boundary: cfg_blocks_6_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: a6786d7561c37e5e6e2359d0d8bd9a28781b7f0f5196eeebf3c66d262ddb781d

000966:  8B 75 16                     mov      si, word ptr [di + 0x16]
000969:  83 C6 5E                     add      si, 0x5e
00096C:  83 44 50 40                  add      word ptr [si + 0x50], 0x40
000970:  8B 44 40                     mov      ax, word ptr [si + 0x40]
000973:  3D 64 00                     cmp      ax, 0x64
000976:  77 21                        ja       0x999
000978:  8B 44 38                     mov      ax, word ptr [si + 0x38]
00097B:  3D 64 00                     cmp      ax, 0x64
00097E:  7F 19                        jg       0x999
000980:  3D 9C FF                     cmp      ax, 0xff9c
000983:  7C 14                        jl       0x999
000985:  8B 44 3C                     mov      ax, word ptr [si + 0x3c]
000988:  3D 64 00                     cmp      ax, 0x64
00098B:  7F 0C                        jg       0x999
00098D:  3D 9C FF                     cmp      ax, 0xff9c
000990:  7C 07                        jl       0x999
000992:  64 C7 06 6E 22 01 00         mov      word ptr fs:[0x226e], 1
