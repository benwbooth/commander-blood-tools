; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00ad96
; seg_off: 0971:1086
; group: seg_0971
; provenance: recursive_graph
; label: gfx_scanline_advance
; label_comment: rendering row helper (22 call sites - most-used unlabeled utility): dec row counter [bp-6]; if 0 return via 0xada9; else di=[bp-8]+0x140 (advance one scanline = 320 bytes in the linear back-buffer), store [bp-8]=di, cx=[bp-0xa]. The per-row advance used across the render/blit loops
; byte_count: 25
; boundary: cfg_blocks_3_terminals_2
; terminal: ret:2
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00ad96_gfx_scanline_advance.cpp
; routine_bytes_sha256: feef3e2ff73a401183cbe70553f51c764629351adb6c54f7c16a81128e89d798

00AD96:  FE 4E FA                     dec      byte ptr [bp - 6]
00AD99:  74 0E                        je       0xada9
00AD9B:  8B 7E F8                     mov      di, word ptr [bp - 8]
00AD9E:  81 C7 40 01                  add      di, 0x140
00ADA2:  8B 4E F6                     mov      cx, word ptr [bp - 0xa]
00ADA5:  89 7E F8                     mov      word ptr [bp - 8], di
00ADA8:  C3                           ret     
00ADA9:  83 C4 02                     add      sp, 2
00ADAC:  C9                           leave   
00ADAD:  1F                           pop      ds
00ADAE:  C3                           ret     
