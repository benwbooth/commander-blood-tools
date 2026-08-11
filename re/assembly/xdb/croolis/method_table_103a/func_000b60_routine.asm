; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000b60
; group: method_table_103a
; provenance: alien_method_table_103a_slot_12@0x4342
; byte_count: 16
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: c74ca55adaf050bd2fcd4d0ec1d112f3a4341bf8fe309e5eeb801255cf127b59

000B60:  8B 75 16                     mov      si, word ptr [di + 0x16]
000B63:  2E A1 99 00                  mov      ax, word ptr cs:[0x99]
000B67:  D1 F8                        sar      ax, 1
000B69:  78 04                        js       0xb6f
000B6B:  01 84 B0 00                  add      word ptr [si + 0xb0], ax
000B6F:  C3                           ret     
