; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0018d3
; group: callback_state_machine
; provenance: state_callback_store@0x0018c0
; byte_count: 107
; boundary: cfg_blocks_3_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 80758c3f56df8bc6ca5779d4813b1a9ee671aa7875e0cae20ffb03436de81de9

0018D3:  FF 4D 38                     dec      word ptr [di + 0x38]
0018D6:  78 2A                        js       0x1902
0018D8:  C7 44 54 00 00               mov      word ptr [si + 0x54], 0
0018DD:  81 44 50 80 00               add      word ptr [si + 0x50], 0x80
0018E2:  83 6C 52 75                  sub      word ptr [si + 0x52], 0x75
0018E6:  66 0F BF 45 3A               movsx    eax, word ptr [di + 0x3a]
0018EB:  66 0F BF 5D 3C               movsx    ebx, word ptr [di + 0x3c]
0018F0:  66 0F BF 4D 3E               movsx    ecx, word ptr [di + 0x3e]
0018F5:  66 01 44 42                  add      dword ptr [si + 0x42], eax
0018F9:  66 01 5C 46                  add      dword ptr [si + 0x46], ebx
0018FD:  66 01 4C 4A                  add      dword ptr [si + 0x4a], ecx
001901:  C3                           ret
001902:  C7 45 36 01 00               mov      word ptr [di + 0x36], 1
001907:  C7 45 38 20 00               mov      word ptr [di + 0x38], 0x20
00190C:  C7 44 54 00 00               mov      word ptr [si + 0x54], 0
001911:  8B 5C 50                     mov      bx, word ptr [si + 0x50]
001914:  8B 4C 52                     mov      cx, word ptr [si + 0x52]
001917:  C1 E3 04                     shl      bx, 4
00191A:  C1 E1 04                     shl      cx, 4
00191D:  C1 FB 04                     sar      bx, 4
001920:  C1 F9 04                     sar      cx, 4
001923:  89 5C 50                     mov      word ptr [si + 0x50], bx
001926:  89 4C 52                     mov      word ptr [si + 0x52], cx
001929:  F7 D9                        neg      cx
00192B:  C1 F9 05                     sar      cx, 5
00192E:  89 4D 3A                     mov      word ptr [di + 0x3a], cx
001931:  C7 44 0E 92 16               mov      word ptr [si + 0xe], 0x1692
001936:  2E C7 06 48 16 00 00         mov      word ptr cs:[0x1648], 0
00193D:  C3                           ret
