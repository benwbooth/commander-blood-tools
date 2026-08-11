; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000b1f
; group: method_table_103a
; provenance: alien_method_table_103a_slot_12@0x42d2
; byte_count: 16
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_000b1f_routine.cpp
; routine_bytes_sha256: c74ca55adaf050bd2fcd4d0ec1d112f3a4341bf8fe309e5eeb801255cf127b59

000B1F:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B22:  2E A1 99 00                  mov      ax, word ptr cs:[0x99]
000B26:  D1 F8                        sar      ax, 1
000B28:  78 04                        js       0xb2e
000B2A:  01 84 B0 00                  add      word ptr [si + 0xb0], ax
000B2E:  C3                           ret     
