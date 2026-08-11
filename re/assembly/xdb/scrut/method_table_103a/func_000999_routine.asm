; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000999
; group: method_table_103a
; provenance: alien_method_table_103a_slot_6@0x43f6
; byte_count: 92
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 440aacc88cf7b7f15e7fd49e6827fa3b922943ad509d3437e5f71510515f1928

000999:  8B 75 16                     mov      si, word ptr [di + 0x16]
00099C:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
00099F:  BF 00 40                     mov      di, 0x4000
0009A2:  BD FF 7F                     mov      bp, 0x7fff
0009A5:  83 C6 5E                     add      si, 0x5e
0009A8:  A1 EC 22                     mov      ax, word ptr [0x22ec]
0009AB:  8B 1E F0 22                  mov      bx, word ptr [0x22f0]
0009AF:  8B 16 F4 22                  mov      dx, word ptr [0x22f4]
0009B3:  03 44 42                     add      ax, word ptr [si + 0x42]
0009B6:  03 5C 46                     add      bx, word ptr [si + 0x46]
0009B9:  03 54 4A                     add      dx, word ptr [si + 0x4a]
0009BC:  03 C7                        add      ax, di
0009BE:  03 DF                        add      bx, di
0009C0:  03 D7                        add      dx, di
0009C2:  23 C5                        and      ax, bp
0009C4:  23 DD                        and      bx, bp
0009C6:  23 D5                        and      dx, bp
0009C8:  2B C7                        sub      ax, di
0009CA:  2B DF                        sub      bx, di
0009CC:  2B D7                        sub      dx, di
0009CE:  2B 06 EC 22                  sub      ax, word ptr [0x22ec]
0009D2:  2B 1E F0 22                  sub      bx, word ptr [0x22f0]
0009D6:  2B 16 F4 22                  sub      dx, word ptr [0x22f4]
0009DA:  66 0F BF C0                  movsx    eax, ax
0009DE:  66 0F BF DB                  movsx    ebx, bx
0009E2:  66 0F BF D2                  movsx    edx, dx
0009E6:  66 89 44 42                  mov      dword ptr [si + 0x42], eax
0009EA:  66 89 5C 46                  mov      dword ptr [si + 0x46], ebx
0009EE:  66 89 54 4A                  mov      dword ptr [si + 0x4a], edx
0009F2:  E2 B1                        loop     0x9a5
0009F4:  C3                           ret     
