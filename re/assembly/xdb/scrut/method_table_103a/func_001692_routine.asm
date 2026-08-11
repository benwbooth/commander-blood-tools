; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001692
; group: method_table_103a
; provenance: alien_method_table_103a_slot_2@0x43ee, alien_method_table_103a_slot_4@0x43f2
; byte_count: 127
; boundary: cfg_blocks_3_terminals_2
; terminal: jmp word ptr [si + 0xe]:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 93ab5fc7e70f4185f3af1c80581ad143fe9a5835c8a1ae937b70c347bf03615f

001692:  8B 75 16                     mov      si, word ptr [di + 0x16]
001695:  83 C6 5E                     add      si, 0x5e
001698:  F7 45 36 FF FF               test     word ptr [di + 0x36], 0xffff
00169D:  74 03                        je       0x16a2
00169F:  FF 64 0E                     jmp      word ptr [si + 0xe]
0016A2:  64 A1 5C 10                  mov      ax, word ptr fs:[0x105c]
0016A6:  C1 C8 07                     ror      ax, 7
0016A9:  1D 00 00                     sbb      ax, 0
0016AC:  64 A3 5C 10                  mov      word ptr fs:[0x105c], ax
0016B0:  66 2E 0F BF 1E 90 16         movsx    ebx, word ptr cs:[0x1690]
0016B7:  C7 45 36 01 00               mov      word ptr [di + 0x36], 1
0016BC:  C7 45 38 32 00               mov      word ptr [di + 0x38], 0x32
0016C1:  66 89 5D 3A                  mov      dword ptr [di + 0x3a], ebx
0016C5:  81 C3 2C 01                  add      bx, 0x12c
0016C9:  2E 89 1E 90 16               mov      word ptr cs:[0x1690], bx
0016CE:  C1 C8 07                     ror      ax, 7
0016D1:  1D 00 00                     sbb      ax, 0
0016D4:  89 45 42                     mov      word ptr [di + 0x42], ax
0016D7:  25 FC 0F                     and      ax, 0xffc
0016DA:  89 44 50                     mov      word ptr [si + 0x50], ax
0016DD:  C7 44 52 00 00               mov      word ptr [si + 0x52], 0
0016E2:  C7 44 54 00 00               mov      word ptr [si + 0x54], 0
0016E7:  C7 44 0E 1B 17               mov      word ptr [si + 0xe], 0x171b
0016EC:  C7 44 56 00 00               mov      word ptr [si + 0x56], 0
0016F1:  C7 44 58 00 00               mov      word ptr [si + 0x58], 0
0016F6:  C7 44 5A 00 00               mov      word ptr [si + 0x5a], 0
0016FB:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
0016FE:  49                           dec      cx
0016FF:  83 C6 5E                     add      si, 0x5e
001702:  8B 44 42                     mov      ax, word ptr [si + 0x42]
001705:  8B 5C 4A                     mov      bx, word ptr [si + 0x4a]
001708:  89 44 56                     mov      word ptr [si + 0x56], ax
00170B:  89 5C 5A                     mov      word ptr [si + 0x5a], bx
00170E:  E2 EF                        loop     0x16ff
001710:  C3                           ret     
