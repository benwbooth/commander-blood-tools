; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0016a4
; group: method_table_103a
; provenance: alien_method_table_103a_slot_2@0x432e, alien_method_table_103a_slot_4@0x4332
; byte_count: 121
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp word ptr [si + 0xe]:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/croolis/method_table_103a/func_0016a4_routine.cpp
; routine_bytes_sha256: 6da3c5d246e0202a486757484c491c49745638c5c36ad1fef6361d56d1346377

0016A4:  8B 75 16                     mov      si, word ptr [di + 0x16]
0016A7:  83 C6 5E                     add      si, 0x5e
0016AA:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
0016AF:  74 03                        je       0x16b4
0016B1:  FF 64 0E                     jmp      word ptr [si + 0xe]
0016B4:  64 A1 5C 10                  mov      ax, word ptr fs:[0x105c]
0016B8:  C1 C8 07                     ror      ax, 7
0016BB:  1D 00 00                     sbb      ax, 0
0016BE:  64 A3 5C 10                  mov      word ptr fs:[0x105c], ax
0016C2:  66 2E 0F BF 1E A2 16         movsx    ebx, word ptr cs:[0x16a2]
0016C9:  C7 45 36 01 00               mov      word ptr [di + 0x36], 1
0016CE:  C7 45 38 32 00               mov      word ptr [di + 0x38], 0x32
0016D3:  C7 45 3A 00 00               mov      word ptr [di + 0x3a], 0
0016D8:  66 89 5D 3C                  mov      dword ptr [di + 0x3c], ebx
0016DC:  81 C3 FA 00                  add      bx, 0xfa
0016E0:  2E 89 1E A2 16               mov      word ptr cs:[0x16a2], bx
0016E5:  C1 C8 07                     ror      ax, 7
0016E8:  1D 00 00                     sbb      ax, 0
0016EB:  89 45 42                     mov      word ptr [di + 0x42], ax
0016EE:  25 FC 0F                     and      ax, 0xffc
0016F1:  89 44 50                     mov      word ptr [si + 0x50], ax
0016F4:  C7 44 52 00 00               mov      word ptr [si + 0x52], 0
0016F9:  C7 44 54 00 00               mov      word ptr [si + 0x54], 0
0016FE:  C7 44 0E 27 17               mov      word ptr [si + 0xe], 0x1727
001703:  C7 44 56 00 00               mov      word ptr [si + 0x56], 0
001708:  C7 44 58 00 00               mov      word ptr [si + 0x58], 0
00170D:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
001710:  49                           dec      cx
001711:  83 C6 5E                     add      si, 0x5e
001714:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
001717:  89 44 56                     mov      word ptr [si + 0x56], ax
00171A:  E2 F5                        loop     0x1711
00171C:  C3                           ret     
