; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0012cc
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x43f0
; byte_count: 45
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/xdb/scrut/method_table_103a/func_0012cc_routine.cpp
; routine_bytes_sha256: 94633ef6bbccc77422b758adcf2d747e9f65f3b4524d1a791a5e447befb959ae

0012CC:  8B 75 16                     mov      si, word ptr [di + 0x16]
0012CF:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
0012D2:  83 C6 5E                     add      si, 0x5e
0012D5:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0012DA:  0F 84 CB FE                  je       0x11a9
0012DE:  78 0E                        js       0x12ee
0012E0:  2E FF 0E 72 0B               dec      word ptr cs:[0xb72]
0012E5:  79 07                        jns      0x12ee
0012E7:  2E C7 06 72 0B 07 00         mov      word ptr cs:[0xb72], 7
0012EE:  51                           push     cx
0012EF:  FF 54 0E                     call     word ptr [si + 0xe]
0012F2:  59                           pop      cx
0012F3:  83 C6 5E                     add      si, 0x5e
0012F6:  E2 F6                        loop     0x12ee
0012F8:  C3                           ret     
