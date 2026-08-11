; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0012de
; group: method_table_103a
; provenance: alien_method_table_103a_slot_3@0x4330
; byte_count: 45
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/xdb/croolis/method_table_103a/func_0012de_routine.cpp
; routine_bytes_sha256: 94633ef6bbccc77422b758adcf2d747e9f65f3b4524d1a791a5e447befb959ae

0012DE:  8B 75 16                     mov      si, word ptr [di + 0x16]
0012E1:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
0012E4:  83 C6 5E                     add      si, 0x5e
0012E7:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0012EC:  0F 84 CB FE                  je       0x11bb
0012F0:  78 0E                        js       0x1300
0012F2:  2E FF 0E 72 0B               dec      word ptr cs:[0xb72]
0012F7:  79 07                        jns      0x1300
0012F9:  2E C7 06 72 0B 07 00         mov      word ptr cs:[0xb72], 7
001300:  51                           push     cx
001301:  FF 54 0E                     call     word ptr [si + 0xe]
001304:  59                           pop      cx
001305:  83 C6 5E                     add      si, 0x5e
001308:  E2 F6                        loop     0x1300
00130A:  C3                           ret     
