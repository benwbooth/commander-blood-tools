; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001286
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x42c0
; byte_count: 45
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: a83d10991b982d63cc141e841462aae99d03a8789fd69d1af2221d41c30df5a1

001286:  8B 75 16                     mov      si, word ptr [di + 0x16]
001289:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
00128C:  83 C6 5E                     add      si, 0x5e
00128F:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
001294:  0F 84 CB FE                  je       0x1163
001298:  78 0E                        js       0x12a8
00129A:  2E FF 0E 31 0B               dec      word ptr cs:[0xb31]
00129F:  79 07                        jns      0x12a8
0012A1:  2E C7 06 31 0B 07 00         mov      word ptr cs:[0xb31], 7
0012A8:  51                           push     cx
0012A9:  FF 54 0E                     call     word ptr [si + 0xe]
0012AC:  59                           pop      cx
0012AD:  83 C6 5E                     add      si, 0x5e
0012B0:  E2 F6                        loop     0x12a8
0012B2:  C3                           ret     
