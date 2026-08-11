; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x00164c
; group: method_table_103a
; provenance: alien_method_table_103a_slot_2@0x42be
; byte_count: 60
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp word ptr [si + 0xe]:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 76e7934fa691d1e8042a8e1ca78b50085c911d0e2da12cb828c86344cdb92ec1

00164C:  8B 75 16                     mov      si, word ptr [di + 0x16]
00164F:  83 C6 5E                     add      si, 0x5e
001652:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
001657:  74 03                        je       0x165c
001659:  FF 64 0E                     jmp      word ptr [si + 0xe]
00165C:  64 A1 5C 10                  mov      ax, word ptr fs:[0x105c]
001660:  C1 C8 07                     ror      ax, 7
001663:  1D 00 00                     sbb      ax, 0
001666:  64 A3 5C 10                  mov      word ptr fs:[0x105c], ax
00166A:  C7 45 36 01 00               mov      word ptr [di + 0x36], 1
00166F:  C7 45 38 00 00               mov      word ptr [di + 0x38], 0
001674:  89 45 40                     mov      word ptr [di + 0x40], ax
001677:  C7 44 0E 92 16               mov      word ptr [si + 0xe], 0x1692
00167C:  25 FC 0F                     and      ax, 0xffc
00167F:  89 44 50                     mov      word ptr [si + 0x50], ax
001682:  C7 44 58 14 00               mov      word ptr [si + 0x58], 0x14
001687:  C3                           ret     
