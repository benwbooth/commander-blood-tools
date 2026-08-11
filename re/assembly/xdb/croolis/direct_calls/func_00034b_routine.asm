; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00034b
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 17
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9088c864b81d156291d0a7bcc1f0de09edfa68b14d89034733fd541a0d196efc

00034B:  51                           push     cx
00034C:  B8 08 00                     mov      ax, 8
00034F:  33 C9                        xor      cx, cx
000351:  CD 33                        int      0x33
000353:  B8 07 00                     mov      ax, 7
000356:  5A                           pop      dx
000357:  33 C9                        xor      cx, cx
000359:  CD 33                        int      0x33
00035B:  C3                           ret     
