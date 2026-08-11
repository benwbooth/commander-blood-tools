; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000d4a
; seg_off: 0000:074a
; group: seg_0000
; provenance: recursive_graph
; label: mouse_set_hrange
; label_comment: mouse range: cx=ax(min), dx=bx(max); int 33h ax=7 (set horizontal cursor range). Clamps mouse X to the game's coordinate range
; byte_count: 23
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_000d4a_mouse_set_hrange.cpp
; routine_bytes_sha256: 2525634f2e7b2ba50d8c9f6fabbc0d7eca82898d7ca4eafa5bf0fe1bb15c1208

000D4A:  50                           push     ax
000D4B:  53                           push     bx
000D4C:  51                           push     cx
000D4D:  52                           push     dx
000D4E:  8B C8                        mov      cx, ax
000D50:  8B D3                        mov      dx, bx
000D52:  B8 07 00                     mov      ax, 7
000D55:  CD 33                        int      0x33
000D57:  5A                           pop      dx
000D58:  59                           pop      cx
000D59:  B8 08 00                     mov      ax, 8
000D5C:  CD 33                        int      0x33
000D5E:  5B                           pop      bx
000D5F:  58                           pop      ax
000D60:  CB                           retf    
