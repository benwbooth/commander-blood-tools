; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000336
; group: direct_calls
; provenance: direct_call_from_0xa3
; byte_count: 17
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9088c864b81d156291d0a7bcc1f0de09edfa68b14d89034733fd541a0d196efc

000336:  51                           push     cx
000337:  B8 08 00                     mov      ax, 8
00033A:  33 C9                        xor      cx, cx
00033C:  CD 33                        int      0x33
00033E:  B8 07 00                     mov      ax, 7
000341:  5A                           pop      dx
000342:  33 C9                        xor      cx, cx
000344:  CD 33                        int      0x33
000346:  C3                           ret     
