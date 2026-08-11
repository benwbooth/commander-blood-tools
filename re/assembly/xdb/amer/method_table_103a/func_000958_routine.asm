; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000958
; group: method_table_103a
; provenance: alien_method_table_103a_slot_6@0x42c6
; byte_count: 92
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/amer/method_table_103a/func_000958_routine.cpp
; routine_bytes_sha256: 440aacc88cf7b7f15e7fd49e6827fa3b922943ad509d3437e5f71510515f1928

000958:  8B 75 16                     mov      si, word ptr [di + 0x16]
00095B:  8B 4D 1A                     mov      cx, word ptr [di + 0x1a]
00095E:  BF 00 40                     mov      di, 0x4000
000961:  BD FF 7F                     mov      bp, 0x7fff
000964:  83 C6 5E                     add      si, 0x5e
000967:  A1 EC 22                     mov      ax, word ptr [0x22ec]
00096A:  8B 1E F0 22                  mov      bx, word ptr [0x22f0]
00096E:  8B 16 F4 22                  mov      dx, word ptr [0x22f4]
000972:  03 44 42                     add      ax, word ptr [si + 0x42]
000975:  03 5C 46                     add      bx, word ptr [si + 0x46]
000978:  03 54 4A                     add      dx, word ptr [si + 0x4a]
00097B:  03 C7                        add      ax, di
00097D:  03 DF                        add      bx, di
00097F:  03 D7                        add      dx, di
000981:  23 C5                        and      ax, bp
000983:  23 DD                        and      bx, bp
000985:  23 D5                        and      dx, bp
000987:  2B C7                        sub      ax, di
000989:  2B DF                        sub      bx, di
00098B:  2B D7                        sub      dx, di
00098D:  2B 06 EC 22                  sub      ax, word ptr [0x22ec]
000991:  2B 1E F0 22                  sub      bx, word ptr [0x22f0]
000995:  2B 16 F4 22                  sub      dx, word ptr [0x22f4]
000999:  66 0F BF C0                  movsx    eax, ax
00099D:  66 0F BF DB                  movsx    ebx, bx
0009A1:  66 0F BF D2                  movsx    edx, dx
0009A5:  66 89 44 42                  mov      dword ptr [si + 0x42], eax
0009A9:  66 89 5C 46                  mov      dword ptr [si + 0x46], ebx
0009AD:  66 89 54 4A                  mov      dword ptr [si + 0x4a], edx
0009B1:  E2 B1                        loop     0x964
0009B3:  C3                           ret     
