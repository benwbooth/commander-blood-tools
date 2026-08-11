; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000925
; group: method_table_103a
; provenance: alien_method_table_103a_slot_10@0x42ce
; byte_count: 51
; boundary: cfg_blocks_6_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: a6786d7561c37e5e6e2359d0d8bd9a28781b7f0f5196eeebf3c66d262ddb781d

000925:  8B 75 16                     mov      si, word ptr [di + 0x16]
000928:  83 C6 5E                     add      si, 0x5e
00092B:  83 44 50 40                  add      word ptr [si + 0x50], 0x40
00092F:  8B 44 40                     mov      ax, word ptr [si + 0x40]
000932:  3D 64 00                     cmp      ax, 0x64
000935:  77 21                        ja       0x958
000937:  8B 44 38                     mov      ax, word ptr [si + 0x38]
00093A:  3D 64 00                     cmp      ax, 0x64
00093D:  7F 19                        jg       0x958
00093F:  3D 9C FF                     cmp      ax, 0xff9c
000942:  7C 14                        jl       0x958
000944:  8B 44 3C                     mov      ax, word ptr [si + 0x3c]
000947:  3D 64 00                     cmp      ax, 0x64
00094A:  7F 0C                        jg       0x958
00094C:  3D 9C FF                     cmp      ax, 0xff9c
00094F:  7C 07                        jl       0x958
000951:  64 C7 06 6E 22 01 00         mov      word ptr fs:[0x226e], 1
